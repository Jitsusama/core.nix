//! `health follow`, the daemon: it follows the journals from the start of
//! the day, tallies what they say, and keeps the day's file current every
//! half minute, so `health` and `cadence day` read it without asking
//! anything that runs. Each half minute it also looks at the lid, which no
//! journal tells of when it's wrong.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use jiff::tz::TimeZone;
use jiff::{Timestamp, Zoned};

use crate::agents;
use crate::busy;
use crate::day::{Day, Kind};
use crate::defunct;
use crate::disk;
use crate::feel;
use crate::figures;
use crate::journal::Mark;
use crate::lid::{self, Looked};
use crate::memory;
use crate::reports;
use crate::sessions;
use crate::source::{Followers, Source};
use crate::trials;
use crate::typing;

use spell::Spell;

/// How often a changed day is written out and the lid looked at: often
/// enough that what `health` shows is a moment old, seldom enough to cost
/// nothing.
const EVERY: Duration = Duration::from_secs(30);

// Health's own faults are failures, and a mended write a notice, named once for a test to pin.
const FAILS: fn(&str) = crate::source::fail;
const NOTICES: fn(&str) = crate::source::notice;

/// How long health runs before a follower's end ends it, for systemd or
/// launchd to start it again with every follower and the day read again,
/// which hears what went unheard meanwhile that day, short of a start
/// that falls past midnight, which reads only the new day. Followers that
/// end at every start then cost a replay of the day every ten minutes,
/// rather than one every few seconds. The faults health saw itself, such
/// as a lid reading wrong, are kept through each start (`Day::resume`).
pub const SETTLED: Duration = Duration::from_mins(10);

/// How long health tries to write a day it can't before it ends to start
/// again, for a write that more room won't fix, such as a folder it may
/// no longer write, so it shows in health's restarts and costs a replay
/// an hour rather than keeping one day for ever.
const GIVES_UP: Duration = Duration::from_hours(1);

/// The day being tallied, and where it's written.
pub struct Follower {
    source: Source,
    day: Day,
    changed: bool,
    // The last write failed, so a new day's line leaves the next try to
    // the tick rather than serializing the whole day again for each.
    write_failed: bool,
    // The days before, oldest first, held until written, so the new day's
    // lines and ticks go on in it; a sleep, which stops `GIVES_UP`'s
    // clock, can pass more than one midnight.
    held: Vec<Day>,
    into: PathBuf,
    zone: TimeZone,
    ends: BTreeMap<String, i64>,
    lid: Option<Looked>,
    pains: Option<PathBuf>,
    typing: Option<PathBuf>,
    trials: Option<trials::Reader>,
    sessions: Option<sessions::Reader>,
    proc: Option<PathBuf>,
    asks_ps: bool,
    presences: Option<PathBuf>,
    cgroups: Option<(PathBuf, String)>,
    guards: Vec<PathBuf>,
    peers: Option<PathBuf>,
    /// How long each thing whose calls go unheard has gone so
    /// (follow/peers.rs).
    unheard_since: BTreeMap<String, peers::Stretch>,
    reports: Vec<PathBuf>,
    reported: BTreeSet<PathBuf>,
}

/// How much of the sessions' logs is read a tick, so a first start on a
/// day of a gigabyte of them reads it over minutes rather than all at once.
const SESSIONS_A_TICK: u64 = 32 << 20;

/// Where each pi session says it's there and what it's doing, under the
/// account's home, as the harness's mesh keeps it.
const PRESENCES: &str = ".local/state/pi/agentic-harness.pi/mesh-workflow/presence";

/// What the day calls a disk guard that stopped.
const GUARD_STOPPED: &str = "the disk guard stopped saying it's there";

/// What the day calls a lid that's wrong.
const LID_WRONG: &str = "the lid reads shut while the laptop's screen is on and its keys are typed";

impl Follower {
    /// A follower starting `date` empty, writing into `into`. It starts
    /// empty each time it runs, since the journals are read again from the
    /// day's start.
    #[must_use]
    pub fn new(date: &str, into: &Path, zone: TimeZone) -> Self {
        Self {
            source: Source::Journal,
            day: Day::new(date),
            changed: false,
            write_failed: false,
            held: Vec::new(),
            into: into.to_owned(),
            zone,
            ends: BTreeMap::new(),
            lid: None,
            pains: None,
            typing: None,
            trials: None,
            sessions: None,
            proc: None,
            asks_ps: false,
            presences: None,
            cgroups: None,
            guards: Vec::new(),
            peers: None,
            unheard_since: BTreeMap::new(),
            reports: Vec::new(),
            reported: BTreeSet::new(),
        }
    }

    /// Read the lines `source` writes, rather than the journal's.
    #[must_use]
    pub fn reading(self, source: Source) -> Self {
        Self { source, ..self }
    }

    /// Read the disks from the guards' status files in `statuses`.
    #[must_use]
    pub fn guarding(self, statuses: Vec<PathBuf>) -> Self {
        Self {
            guards: statuses,
            ..self
        }
    }

    /// Keep what the guards say of the disks at `at`. A guard found
    /// stopped counts once as a service's fault, and again only after it
    /// beats again, a start between them or not.
    pub fn guard(&mut self, at: i64) {
        if self.guards.is_empty() || date_of(at, &self.zone) != self.day.date {
            return;
        }
        let look = disk::look(&self.guards, at);
        let before = self.day.disks.clone();
        self.day.disks.saw(at, &look);
        self.changed |= before != self.day.disks;
        for stopped in &look.stopped {
            if !self.day.seeing.guards.contains(stopped) {
                self.day
                    .fault("disk-guard", Kind::Services, GUARD_STOPPED, at);
                self.changed = true;
            }
        }
        if self.day.seeing.guards != look.stopped {
            self.day.seeing.guards = look.stopped;
            self.changed = true;
        }
    }

    /// Weigh whose work kept the processors busy, from the groups beneath
    /// `cgroups`, the account's under its `manager`, named for their
    /// quests by the presences being counted.
    #[must_use]
    pub fn weighing(self, cgroups: &Path, manager: &str) -> Self {
        Self {
            cgroups: Some((cgroups.to_owned(), manager.to_owned())),
            ..self
        }
    }

    /// Every five minutes, keep what each group used since the last look
    /// in the hour of `at`.
    pub fn weigh(&mut self, at: i64) {
        let (Some((cgroups, manager)), Some(presences), Some(proc)) =
            (&self.cgroups, &self.presences, &self.proc)
        else {
            return;
        };
        if date_of(at, &self.zone) != self.day.date || !self.day.busy.due(at) {
            return;
        }
        let hour = Timestamp::from_microsecond(at)
            .map(|at| at.to_zoned(self.zone.clone()).hour())
            .unwrap_or_default();
        let groups = busy::look(cgroups, manager, presences, proc);
        self.day.busy.saw(at, &format!("{hour:02}"), groups);
        self.changed = true;
    }

    /// Look for the agents that died among the sessions' presences.
    #[must_use]
    pub fn counting_the_dead(self, presences: &Path) -> Self {
        Self {
            presences: Some(presences.to_owned()),
            ..self
        }
    }

    /// Keep the agents gone without parking at `at`, by their presences
    /// and the processes in the `proc` being minded, or that `ps` shows.
    pub fn count_the_dead(&mut self, at: i64) {
        let Some(presences) = &self.presences else {
            return;
        };
        if self.proc.is_none() && !self.asks_ps {
            return;
        }
        if date_of(at, &self.zone) != self.day.date {
            return;
        }
        let looked = match &self.proc {
            Some(proc) => agents::look(presences, proc),
            None => agents::asked(presences, at),
        };
        let (zone, date) = (&self.zone, &self.day.date);
        self.day
            .agents
            .saw(at, looked, |died| date_of(died, zone) == *date);
        self.changed = true;
    }

    /// Look at the watched programs' memory in `proc`.
    #[must_use]
    pub fn minding(self, proc: &Path) -> Self {
        Self {
            proc: Some(proc.to_owned()),
            ..self
        }
    }

    /// Ask `ps` what the watched programs hold, where there's no `proc`.
    #[must_use]
    pub fn minding_by_ps(self) -> Self {
        Self {
            asks_ps: true,
            ..self
        }
    }

    /// Keep what each watched program's copies hold now, at `at`, and the
    /// parent of the most defunct children.
    pub fn mind(&mut self, at: i64) {
        if self.proc.is_none() && !self.asks_ps {
            return;
        }
        if date_of(at, &self.zone) != self.day.date {
            return;
        }
        // Every watched program is kept each look, none running when none
        // was seen, so one that stopped doesn't go on showing its last.
        let mut seen = self
            .proc
            .as_deref()
            .map_or_else(memory::asked, memory::look);
        for name in memory::WATCHED {
            let copies = seen.remove(name).unwrap_or_default();
            self.day
                .memory
                .entry(name.to_owned())
                .or_default()
                .saw(at, &copies);
        }
        let worst = self
            .proc
            .as_deref()
            .map_or_else(defunct::asked, defunct::look);
        self.day.defunct.saw(at, worst.as_ref());
        self.changed = true;
    }

    /// Read the agents' sessions' logs, a folder each place they ran,
    /// under `sessions`.
    #[must_use]
    pub fn hearing(self, sessions: &Path) -> Self {
        Self {
            sessions: Some(sessions::Reader::new(sessions)),
            ..self
        }
    }

    /// Carry on from the day this run's last one kept, which the journals
    /// say again but the sessions' logs, read on from where it stopped,
    /// don't.
    #[must_use]
    pub fn resuming(mut self, kept: Day) -> Self {
        self.day.resume(kept);
        self
    }

    /// Read on in each session's log written to today, `budget` bytes in
    /// all at most, and count what the day's lines said.
    pub fn hear(&mut self, budget: u64) {
        let Some(reader) = &mut self.sessions else {
            return;
        };
        let (zone, day) = (&self.zone, &mut self.day);
        let date = day.date.clone();
        let today = |at: i64| date_of(at, zone) == date;
        let mut read = std::mem::take(&mut day.read);
        let any = reader.read_on(&mut read, budget, &today, |place, heard| {
            day.heard(place, &heard);
        });
        day.read = read;
        self.changed |= any;
    }

    /// Count the crashes a Mac reports in `folders`.
    #[must_use]
    pub fn reporting(self, folders: Vec<PathBuf>) -> Self {
        Self {
            reports: folders,
            ..self
        }
    }

    /// Count each new report's crash that's of the day. Each run reads
    /// the day's reports again, as it reads the day's lines again.
    pub fn read_reports(&mut self) {
        for crash in reports::look(&self.reports, &mut self.reported) {
            if date_of(crash.at, &self.zone) == self.day.date {
                self.day.take(&crash);
                self.changed = true;
            }
        }
    }

    /// Read how using the machine felt from the pain-points detector's
    /// days in `pains`.
    #[must_use]
    pub fn feeling(self, pains: &Path) -> Self {
        Self {
            pains: Some(pains.to_owned()),
            ..self
        }
    }

    /// Take the day's pains as the detector ranks them now; it rewrites
    /// its file every ten minutes.
    pub fn feel(&mut self) {
        let Some(pains) = &self.pains else {
            return;
        };
        let feel = feel::read(pains, &self.day.date);
        if feel != self.day.feel {
            self.day.feel = feel;
            self.changed = true;
        }
    }

    /// Read typing into pi from pi-keys' samples in `samples`.
    #[must_use]
    pub fn timing_typing(self, samples: &Path) -> Self {
        Self {
            typing: Some(samples.to_owned()),
            ..self
        }
    }

    /// Take the day's samples of typing into pi; pi-keys adds one a few
    /// times a day.
    pub fn time_typing(&mut self) {
        let Some(samples) = &self.typing else {
            return;
        };
        let typing = typing::read(samples, &self.day.date, &self.zone);
        if typing != self.day.typing {
            self.day.typing = typing;
            self.changed = true;
        }
    }

    /// Pool every key's time to the screen from the launcher's trials in
    /// each folder of `shells`.
    #[must_use]
    pub fn pooling_keys(self, shells: &Path) -> Self {
        Self {
            trials: Some(trials::Reader::new(shells)),
            ..self
        }
    }

    /// Pool the keys of the day's openings the shell kept since the last
    /// look; it adds one as the launcher closes.
    pub fn pool_keys(&mut self) {
        let Some(reader) = &mut self.trials else {
            return;
        };
        let day = &mut self.day;
        self.changed |= reader.pool(&mut day.launcher, &day.date, &self.zone);
    }

    /// Judge a look at the lid, taken at `at`. A lid gone wrong counts once
    /// as a device's fault, and again only after it reads open or the
    /// screen goes dark, a start between them or not.
    pub fn look_at_lid(&mut self, now: Looked, at: i64) {
        if lid::wrong(&now, self.lid.as_ref()) && !self.day.seeing.lid {
            self.day.seeing.lid = true;
            self.changed = true;
            if date_of(at, &self.zone) == self.day.date {
                self.day.fault("lid", Kind::Devices, LID_WRONG, at);
            }
        }
        if (!now.shut || !now.screen_on) && self.day.seeing.lid {
            self.day.seeing.lid = false;
            self.changed = true;
        }
        self.lid = Some(now);
    }

    /// Know when each boot before this one ended, so a shutdown seen
    /// asked for is timed to its end.
    #[must_use]
    pub fn knowing(self, ends: BTreeMap<String, i64>) -> Self {
        Self { ends, ..self }
    }

    /// Take one line of the source's. A new day's line writes the old day,
    /// held for `write` to try again where it can't be, and starts the new,
    /// which sees nothing gone wrong yet, so a fault still seen counts once
    /// in it too. Of a day already done, only stops, starts and reloads are
    /// taken, for the new day's restarts (`Launches::before`).
    ///
    /// # Errors
    ///
    /// When the finished day can't be written, its line taken all the
    /// same. After a failed write, only `write` tries again.
    pub fn take(&mut self, line: &str) -> io::Result<()> {
        let Some(entry) = self.source.parse(line) else {
            return Ok(());
        };
        let date = date_of(entry.at, &self.zone);
        let mut written = Ok(());
        if date > self.day.date {
            if !self.write_failed {
                written = self.write();
            }
            // A restart across midnight is the new day's, so its stop
            // and the reloads before it carry on, as a start of health
            // reads the restarts' followers from just before the day
            // (`journal::reading`).
            let read = self.day.read.clone();
            let launches = self.day.launches.carried(start_of(entry.at, &self.zone));
            let done = std::mem::replace(&mut self.day, Day::new(&date));
            if self.write_failed {
                self.held.push(done);
            }
            self.day.read = read;
            self.day.launches = launches;
        } else if date < self.day.date {
            // That day is done, but a stop or reload of it, heard late or
            // read as the day leads in, may still be a restart's in this.
            self.day.launches.before(&entry);
            self.changed = true;
            return Ok(());
        }
        self.day.take(&entry);
        if entry.mark == Some(Mark::Stopping)
            && let Some(&end) = self.ends.get(&entry.boot)
        {
            self.day.ended(&entry.boot, end);
        }
        self.changed = true;
        written
    }

    /// Write each day held, and the day if anything changed since it was
    /// last written, each tried whatever the others do, so one that can't
    /// be written keeps none of the rest from the disk.
    ///
    /// # Errors
    ///
    /// The first that can't be written's, the rest kept for the next try.
    pub fn write(&mut self) -> io::Result<()> {
        let mut first = Ok(());
        let mut keep = |written: io::Result<()>| {
            let failed = written.is_err();
            if first.is_ok() {
                first = written;
            }
            failed
        };
        let into = &self.into;
        self.held.retain(|day| keep(write_day(day, into)));
        if self.changed && !keep(write_day(&self.day, into)) {
            self.changed = false;
        }
        self.write_failed = first.is_err();
        first
    }
}

/// The day's file in `into`.
#[must_use]
pub fn file_of(into: &Path, date: &str) -> PathBuf {
    into.join(format!("{date}.json"))
}

/// When each boot before this one ended, from `journalctl --list-boots -o
/// json`; this boot, the last listed, hasn't.
#[must_use]
pub fn ends_of(boots: &str) -> BTreeMap<String, i64> {
    let Ok(serde_json::Value::Array(boots)) = serde_json::from_str(boots) else {
        return BTreeMap::new();
    };
    let before = boots.len().saturating_sub(1);
    boots
        .iter()
        .take(before)
        .filter_map(|boot| {
            Some((
                boot["boot_id"].as_str()?.to_owned(),
                boot["last_entry"].as_i64()?,
            ))
        })
        .collect()
}

/// Write a day whole, through a file beside it, so a reader never sees half
/// of one, and taken away on a failure, where it would hold a full disk's
/// room. The error names the day, since one held from before fails alone.
fn write_day(day: &Day, into: &Path) -> io::Result<()> {
    let part = into.join(format!(".{}.json.part", day.date));
    let written = fs::create_dir_all(into)
        .and_then(|()| figures::written(day).map_err(io::Error::other))
        .and_then(|json| fs::write(&part, json))
        .and_then(|()| fs::rename(&part, file_of(into, &day.date)));
    if written.is_err() {
        let _ = fs::remove_file(&part);
    }
    written.map_err(|error| io::Error::new(error.kind(), format!("{}'s file: {error}", day.date)))
}

/// The date, `YYYY-MM-DD`, of a moment in microseconds since the epoch, in
/// the zone given.
#[must_use]
pub fn date_of(at: i64, zone: &TimeZone) -> String {
    Timestamp::from_microsecond(at)
        .map(|at| at.to_zoned(zone.clone()).date().to_string())
        .unwrap_or_default()
}

/// The start of the day a moment in microseconds since the epoch falls in,
/// in the zone given, in microseconds: the moment itself when it can't be
/// placed, as for a stamp out of jiff's range.
fn start_of(at: i64, zone: &TimeZone) -> i64 {
    Timestamp::from_microsecond(at)
        .and_then(|moment| moment.to_zoned(zone.clone()).start_of_day())
        .map_or(at, |start| start.timestamp().as_microsecond())
}

/// Follow the machine's own source into `into` until a follower ends,
/// which only a fault does (`run`); then the day is written and systemd or
/// launchd starts it again, with every follower.
///
/// # Errors
///
/// When a follower has ended or its followers couldn't start, no sooner
/// than `SETTLED` after health started and once the day is written, or
/// when the day can't be written as health ends.
pub fn follow(into: &Path) -> io::Result<()> {
    let source = Source::here();
    let zone = TimeZone::system();
    // One moment for the day resumed and the day the journals replay, so a
    // start just before midnight can't resume one and replay the next.
    let now = Zoned::now().with_time_zone(zone.clone());
    let today = date_of(now.timestamp().as_microsecond(), &zone);
    let mut follower = Follower::new(&today, into, zone.clone())
        .reading(source)
        .knowing(source.ends()?);
    // The detector and pi-keys keep their days beside health's, in the
    // account's state.
    if let Some(state) = into.parent() {
        follower = follower
            .feeling(&state.join("pain-points"))
            .timing_typing(&state.join("pi-keys"))
            .pooling_keys(&state.join(trials::SHELLS));
    }
    // systemd gives every account's service its runtime folder, where the
    // account's guard keeps its status. A Mac has none; there it's TMPDIR,
    // the temporary folder launchd gives each agent, the guard's included.
    // Linux's TMPDIR is somewhere else, so it's never read there, as the
    // guard never reads it.
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .or_else(|| {
            cfg!(target_os = "macos")
                .then(|| std::env::var_os("TMPDIR"))
                .flatten()
        })
        .map(PathBuf::from);
    follower = follower.guarding(disk::statuses(runtime.as_deref(), source.guard_status()));
    // Every account's runtime folder sits beside this one's, where each
    // account's mycelium node keeps its sockets; only this account's can
    // be read, so only its node is asked.
    if let Some(runtimes) = runtime.as_deref().and_then(Path::parent) {
        follower = follower.hearing_peers(runtimes.to_owned());
    }
    if let Some(processes) = source.processes() {
        follower = follower.minding(processes);
    } else if source.asks_ps() {
        follower = follower.minding_by_ps();
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    follower = follower.reporting(source.reports(home.as_deref()));
    if let Some(home) = home {
        follower = follower
            .hearing(&home.join(".pi/agent/sessions"))
            .counting_the_dead(&home.join(PRESENCES));
    }
    let manager = source
        .processes()
        .and_then(|proc| fs::read_to_string(proc.join("self/cgroup")).ok())
        .and_then(|cgroup| busy::manager_of(&cgroup));
    if let (Some(cgroups), Some(manager)) = (source.cgroups(), manager) {
        follower = follower.weighing(cgroups, &manager);
    }
    if let Some(kept) = fs::read(file_of(into, &today))
        .ok()
        .and_then(|kept| serde_json::from_slice(&kept).ok())
    {
        follower = follower.resuming(kept);
    }
    let (lines, read) = mpsc::channel();
    // Dropped however this returns, ending every follower still running.
    let followers = Followers::default();
    let ended = ended_by(source.start(&now, &lines, &followers), &lines, FAILS);
    drop(lines);
    run(
        &mut follower,
        &read,
        Instant::now,
        thread::sleep,
        lid::look,
        &FAILS,
        &NOTICES,
    )?;
    Err(ended)
}

/// What health ends on once `run` returns, given how its followers
/// `started`. A start that failed is said through `say` at once, and
/// taken as a follower's end on `lines`, since one that did start may
/// still hold its sender, so a start that fails at every one costs a
/// replay of the day every `SETTLED` rather than one every few seconds.
fn ended_by(
    started: io::Result<()>,
    lines: &Sender<Option<String>>,
    say: impl Fn(&str),
) -> io::Error {
    match started {
        Ok(()) => io::Error::other("a follower ended, or the day's replay failed"),
        Err(error) => {
            // On a journal, only the day's start not being told fails
            // here, so the words don't say a follower did.
            say(&format!(
                "health: can't start its followers: {error}, {}",
                crate::source::starting_again()
            ));
            let _ = lines.send(None);
            io::Error::other(format!("its followers couldn't start: {error}"))
        }
    }
}

/// Take the followers' lines until one has ended, which its `None` says,
/// and health has run `SETTLED` by `now`, and every half minute, however
/// busy the journals are, look at the lid and write the day. Each
/// follows for as long as health runs, so an end is a fault, and health
/// starting again follows every one again.
///
/// Health ends no sooner than `SETTLED` from its start, however its
/// followers end, so the day isn't read again every few seconds where
/// they end at every start: until then whatever is still heard is taken,
/// and once nothing can be, health `wait`s out the rest, still looking
/// every half minute. What went unheard that day is heard from the day's
/// replay once health starts again.
///
/// A day that can't be written, as on a full disk, is said through `say`,
/// and once written again through `notice`, as `Spell` decides. Health
/// tries every half minute for as long as `GIVES_UP`, as a start frees no
/// room and, past midnight, would lose the old day; a new day starts as
/// ever meanwhile, holding the old, so what's heard and seen goes into it.
///
/// # Errors
///
/// When the day can't be written as health ends, or it gave up.
fn run(
    follower: &mut Follower,
    read: &Receiver<Option<String>>,
    mut now: impl FnMut() -> Instant,
    mut wait: impl FnMut(Duration),
    mut look: impl FnMut() -> Looked,
    say: &dyn Fn(&str),
    notice: &dyn Fn(&str),
) -> io::Result<()> {
    let started = now();
    let mut ticked = started;
    let mut ended = false;
    let mut hearing = true;
    let mut spell = Spell::default();
    let mut gave_up;
    loop {
        let at = now();
        let ran = at.saturating_duration_since(started);
        gave_up = spell.gave_up(at);
        if ((ended && !spell.is_on()) || gave_up) && ran >= SETTLED {
            break;
        }
        let left = EVERY.saturating_sub(at.saturating_duration_since(ticked));
        if hearing {
            match read.recv_timeout(left) {
                Ok(Some(line)) => {
                    // Only a new day's line writes, and only a write that
                    // failed says so, so a line taken without one leaves
                    // the spell as it was.
                    if let Err(error) = follower.take(&line) {
                        spell.tried(Some(&error), at, &follower.into, say, notice);
                    }
                }
                Ok(None) => ended = true,
                Err(RecvTimeoutError::Timeout) => {}
                // Every sender is gone, so nothing more can be heard.
                Err(RecvTimeoutError::Disconnected) => {
                    hearing = false;
                    ended = true;
                }
            }
        } else {
            // Past `SETTLED`, only a day still unwritten keeps health
            // here, and the next try is the next tick's, so it never
            // waits nothing and spins.
            let settles = SETTLED.saturating_sub(ran);
            wait(if settles.is_zero() {
                left
            } else {
                left.min(settles)
            });
        }
        let at = now();
        if at.saturating_duration_since(ticked) >= EVERY {
            ticked = at;
            let wall = Timestamp::now().as_microsecond();
            follower.look_at_lid(look(), wall);
            follower.mind(wall);
            follower.count_the_dead(wall);
            follower.weigh(wall);
            follower.guard(wall);
            follower.hear_peers(wall);
            follower.read_reports();
            follower.feel();
            follower.time_typing();
            follower.pool_keys();
            follower.hear(SESSIONS_A_TICK);
            let written = follower.write();
            spell.tried(written.err().as_ref(), at, &follower.into, say, notice);
        }
    }
    let written = follower.write();
    let lost = spell::losing(&follower.held);
    // A start that gave up says so, whatever its last try came to.
    if gave_up {
        return Err(spell.given_up(&follower.into, &lost));
    }
    written.map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("the day couldn't be written as health ended{lost}: {error}"),
        )
    })
}

// A spell of failed writes, and what health says of it, in a file of
// its own so the follower reads whole.
mod spell;

// How the follower hears the person's other machines' calls, in a file of
// its own, follow/peers.rs.
mod peers;

// The tests keep a file of their own, follow/tests.rs, so the follower
// reads whole.
#[cfg(test)]
mod tests;
