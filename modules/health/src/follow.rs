//! `health follow`, the daemon: it follows the journals from the start of
//! the day, tallies what they say, and keeps the day's file current every
//! half minute, so `health` and `cadence day` read it without asking
//! anything that runs. Each half minute it also looks at the lid, which no
//! journal tells of when it's wrong.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use jiff::Timestamp;
use jiff::tz::TimeZone;

use crate::agents;
use crate::busy;
use crate::day::{Day, Kind};
use crate::disk;
use crate::feel;
use crate::figures;
use crate::journal::Mark;
use crate::lid::{self, Looked};
use crate::memory;
use crate::sessions;
use crate::source::Source;
use crate::trials;
use crate::typing;

/// How often a changed day is written out and the lid looked at: often
/// enough that what `health` shows is a moment old, seldom enough to cost
/// nothing.
const EVERY: Duration = Duration::from_secs(30);

/// The day being tallied, and where it's written.
pub struct Follower {
    source: Source,
    day: Day,
    changed: bool,
    into: PathBuf,
    zone: TimeZone,
    ends: BTreeMap<String, i64>,
    lid: Option<Looked>,
    lid_wrong: bool,
    pains: Option<PathBuf>,
    typing: Option<PathBuf>,
    trials: Option<trials::Reader>,
    sessions: Option<sessions::Reader>,
    proc: Option<PathBuf>,
    presences: Option<PathBuf>,
    cgroups: Option<(PathBuf, String)>,
    guards: Vec<PathBuf>,
    guards_stopped: Vec<PathBuf>,
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
            into: into.to_owned(),
            zone,
            ends: BTreeMap::new(),
            lid: None,
            lid_wrong: false,
            pains: None,
            typing: None,
            trials: None,
            sessions: None,
            proc: None,
            presences: None,
            cgroups: None,
            guards: Vec::new(),
            guards_stopped: Vec::new(),
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
    /// beats again.
    pub fn guard(&mut self, at: i64) {
        if self.guards.is_empty() || date_of(at, &self.zone) != self.day.date {
            return;
        }
        let look = disk::look(&self.guards, at);
        let before = self.day.disks.clone();
        self.day.disks.saw(at, &look);
        self.changed |= before != self.day.disks;
        for stopped in &look.stopped {
            if !self.guards_stopped.contains(stopped) {
                self.day
                    .fault("disk-guard", Kind::Services, GUARD_STOPPED, at);
                self.changed = true;
            }
        }
        self.guards_stopped = look.stopped;
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
    /// and the processes in the `proc` being minded.
    pub fn count_the_dead(&mut self, at: i64) {
        let (Some(presences), Some(proc)) = (&self.presences, &self.proc) else {
            return;
        };
        if date_of(at, &self.zone) != self.day.date {
            return;
        }
        let gone = agents::look(presences, proc);
        let (zone, date) = (&self.zone, &self.day.date);
        self.day
            .agents
            .saw(at, gone, |died| date_of(died, zone) == *date);
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

    /// Keep what each watched program's copies hold now, at `at`.
    pub fn mind(&mut self, at: i64) {
        let Some(proc) = &self.proc else {
            return;
        };
        if date_of(at, &self.zone) != self.day.date {
            return;
        }
        // Every watched program is kept each look, none running when none
        // was seen, so one that stopped doesn't go on showing its last.
        let mut seen = memory::look(proc);
        for name in memory::WATCHED {
            let copies = seen.remove(name).unwrap_or_default();
            self.day
                .memory
                .entry(name.to_owned())
                .or_default()
                .saw(at, &copies);
        }
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
        self.changed |= reader.pool(&mut day.launcher.keys, &day.date, &self.zone);
    }

    /// Judge a look at the lid, taken at `at`. A lid gone wrong counts once
    /// as a device's fault, and again only after it reads open or the
    /// screen goes dark.
    pub fn look_at_lid(&mut self, now: Looked, at: i64) {
        if lid::wrong(&now, self.lid.as_ref()) && !self.lid_wrong {
            self.lid_wrong = true;
            if date_of(at, &self.zone) == self.day.date {
                self.day.fault("lid", Kind::Devices, LID_WRONG, at);
                self.changed = true;
            }
        }
        if !now.shut || !now.screen_on {
            self.lid_wrong = false;
        }
        self.lid = Some(now);
    }

    /// Know when each boot before this one ended, so a shutdown seen
    /// asked for is timed to its end.
    #[must_use]
    pub fn knowing(self, ends: BTreeMap<String, i64>) -> Self {
        Self { ends, ..self }
    }

    /// Take one line of the source's. A line of a new day writes
    /// the old one and starts the new; one of a day already written is
    /// left out, since that day is done.
    ///
    /// # Errors
    ///
    /// When the finished day can't be written.
    pub fn take(&mut self, line: &str) -> io::Result<()> {
        let Some(entry) = self.source.parse(line) else {
            return Ok(());
        };
        let date = date_of(entry.at, &self.zone);
        if date > self.day.date {
            self.write()?;
            let read = std::mem::take(&mut self.day.read);
            self.day = Day::new(&date);
            self.day.read = read;
        } else if date < self.day.date {
            return Ok(());
        }
        self.day.take(&entry);
        if entry.mark == Some(Mark::Stopping)
            && let Some(&end) = self.ends.get(&entry.boot)
        {
            self.day.ended(&entry.boot, end);
        }
        self.changed = true;
        Ok(())
    }

    /// Write the day if anything changed since it was last written.
    ///
    /// # Errors
    ///
    /// When it can't be written.
    pub fn write(&mut self) -> io::Result<()> {
        if !self.changed {
            return Ok(());
        }
        write_day(&self.day, &self.into)?;
        self.changed = false;
        Ok(())
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
/// of one.
fn write_day(day: &Day, into: &Path) -> io::Result<()> {
    fs::create_dir_all(into)?;
    let file = file_of(into, &day.date);
    let part = into.join(format!(".{}.json.part", day.date));
    fs::write(&part, figures::written(day).map_err(io::Error::other)?)?;
    fs::rename(part, file)
}

/// The date, `YYYY-MM-DD`, of a moment in microseconds since the epoch, in
/// the zone given.
#[must_use]
pub fn date_of(at: i64, zone: &TimeZone) -> String {
    Timestamp::from_microsecond(at)
        .map(|at| at.to_zoned(zone.clone()).date().to_string())
        .unwrap_or_default()
}

/// Follow the machine's own source into `into` until every follower ends,
/// which only a fault does; then the day is written and systemd or launchd
/// starts it again.
///
/// # Errors
///
/// When a follower can't start or the day can't be written.
pub fn follow(into: &Path) -> io::Result<()> {
    let source = Source::here();
    let zone = TimeZone::system();
    let today = date_of(Timestamp::now().as_microsecond(), &zone);
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
    // systemd gives every account's service its runtime folder, and launchd
    // its agent a temporary folder of its own, where the account's guard
    // keeps its status.
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .or_else(|| std::env::var_os("TMPDIR"))
        .map(PathBuf::from);
    follower = follower.guarding(disk::statuses(runtime.as_deref(), source.guard_status()));
    if let Some(processes) = source.processes() {
        follower = follower.minding(processes);
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
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
    // Counted as they start, so a follower added is waited on too.
    let running = source.start(&zone, &lines)?;
    drop(lines);
    run(&mut follower, &read, running, Instant::now, lid::look)?;
    Err(io::Error::other("the followers ended"))
}

/// Take the followers' lines until `running` of them end, and every half
/// minute by `now`, however busy the journals are, look at the lid and
/// write the day.
fn run(
    follower: &mut Follower,
    read: &Receiver<Option<String>>,
    mut running: usize,
    mut now: impl FnMut() -> Instant,
    mut look: impl FnMut() -> Looked,
) -> io::Result<()> {
    let mut ticked = now();
    while running > 0 {
        let left = EVERY.saturating_sub(now().saturating_duration_since(ticked));
        match read.recv_timeout(left) {
            Ok(Some(line)) => follower.take(&line)?,
            Ok(None) => running -= 1,
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
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
            follower.feel();
            follower.time_typing();
            follower.pool_keys();
            follower.hear(SESSIONS_A_TICK);
            follower.write()?;
        }
    }
    follower.write()
}

// The tests keep a file of their own, follow/tests.rs, so the follower
// reads whole.
#[cfg(test)]
mod tests;
