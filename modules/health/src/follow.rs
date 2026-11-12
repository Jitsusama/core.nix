//! `health follow`, the daemon: it follows the journals from the start of
//! the day, tallies what they say, and keeps the day's file current every
//! half minute, so `health` and `cadence day` read it without asking
//! anything that runs. Each half minute it also looks at the lid, which no
//! journal tells of when it's wrong.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, BufRead as _, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use jiff::Timestamp;
use jiff::tz::TimeZone;

use crate::agents;
use crate::day::{Day, Kind};
use crate::feel;
use crate::figures;
use crate::journal::{
    self, LAUNCHER_READY, Mark, RESTART_SCHEDULED, SHUTTING_DOWN, STARTUP_FINISHED, UNIT_STARTED,
};
use crate::lid::{self, Looked};
use crate::memory;
use crate::sessions;

/// How often a changed day is written out and the lid looked at: often
/// enough that what `health` shows is a moment old, seldom enough to cost
/// nothing.
const EVERY: Duration = Duration::from_secs(30);

/// The fields each entry is read by; journalctl always adds its time.
const FIELDS: &str = "MESSAGE,PRIORITY,SYSLOG_IDENTIFIER,_COMM,USER_UNIT,UNIT,_SYSTEMD_UNIT,\
                      _SYSTEMD_USER_UNIT,COREDUMP_COMM,COREDUMP_CGROUP,MESSAGE_ID,_TRANSPORT,\
                      KERNEL_USEC,INITRD_USEC,USERSPACE_USEC";

/// The day being tallied, and where it's written.
pub struct Follower {
    day: Day,
    changed: bool,
    into: PathBuf,
    zone: TimeZone,
    ends: BTreeMap<String, i64>,
    lid: Option<Looked>,
    lid_wrong: bool,
    pains: Option<PathBuf>,
    sessions: Option<sessions::Reader>,
    proc: Option<PathBuf>,
    presences: Option<PathBuf>,
}

/// How much of the sessions' logs is read a tick, so a first start on a
/// day of a gigabyte of them reads it over minutes rather than all at once.
const SESSIONS_A_TICK: u64 = 32 << 20;

/// Where each pi session says it's there and what it's doing, under the
/// account's home, as the harness's mesh keeps it.
const PRESENCES: &str = ".local/state/pi/agentic-harness.pi/mesh-workflow/presence";

/// What the day calls a lid that's wrong.
const LID_WRONG: &str = "the lid reads shut while the laptop's screen is on and its keys are typed";

impl Follower {
    /// A follower starting `date` empty, writing into `into`. It starts
    /// empty each time it runs, since the journals are read again from the
    /// day's start.
    #[must_use]
    pub fn new(date: &str, into: &Path, zone: TimeZone) -> Self {
        Self {
            day: Day::new(date),
            changed: false,
            into: into.to_owned(),
            zone,
            ends: BTreeMap::new(),
            lid: None,
            lid_wrong: false,
            pains: None,
            sessions: None,
            proc: None,
            presences: None,
        }
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

    /// Take one line of `journalctl -o json`. A line of a new day writes
    /// the old one and starts the new; one of a day already written is
    /// left out, since that day is done.
    ///
    /// # Errors
    ///
    /// When the finished day can't be written.
    pub fn take(&mut self, line: &str) -> io::Result<()> {
        let Some(entry) = journal::parse(line) else {
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

/// Follow the journals into `into` until every follower ends, which only a
/// fault does; then the day is written and systemd starts it again.
///
/// # Errors
///
/// When a follower can't start or the day can't be written.
pub fn follow(into: &Path) -> io::Result<()> {
    let zone = TimeZone::system();
    let today = date_of(Timestamp::now().as_microsecond(), &zone);
    let boots = Command::new("journalctl")
        .args(["--list-boots", "--output=json", "--no-pager"])
        .output()?;
    let mut follower =
        Follower::new(&today, into, zone).knowing(ends_of(&String::from_utf8_lossy(&boots.stdout)));
    // The detector keeps its days beside health's, in the account's state.
    if let Some(state) = into.parent() {
        follower = follower.feeling(&state.join("pain-points"));
    }
    follower = follower.minding(Path::new("/proc"));
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        follower = follower
            .hearing(&home.join(".pi/agent/sessions"))
            .counting_the_dead(&home.join(PRESENCES));
    }
    if let Some(kept) = fs::read(file_of(into, &today))
        .ok()
        .and_then(|kept| serde_json::from_slice(&kept).ok())
    {
        follower = follower.resuming(kept);
    }
    let (lines, read) = mpsc::channel();
    // Warnings and graver.
    spawn(&["-p", "warning"], lines.clone())?;
    // What systemd and logind say only as information: a restart
    // scheduled, the machine started or stopping, and a unit started,
    // which the day keeps only for the disk's password.
    spawn(
        &[
            &format!("MESSAGE_ID={RESTART_SCHEDULED}"),
            &format!("MESSAGE_ID={STARTUP_FINISHED}"),
            &format!("MESSAGE_ID={SHUTTING_DOWN}"),
            "+",
            "_PID=1",
            &format!("MESSAGE_ID={UNIT_STARTED}"),
        ],
        lines.clone(),
    )?;
    // The launcher ready, after each start of the shell.
    spawn(
        &[
            "SYSLOG_IDENTIFIER=quickshell",
            &format!("--grep={LAUNCHER_READY}"),
        ],
        lines.clone(),
    )?;
    // The kernel's notices and information, which is where it says a USB
    // device dropped or a PCIe link corrected itself; the day keeps only
    // those and leaves the rest, about a thousand lines an hour.
    spawn(&["-k", "-p", "notice..info"], lines)?;
    run(&mut follower, &read, 4, Instant::now, lid::look)?;
    Err(io::Error::other("the journal followers ended"))
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
            follower.feel();
            follower.hear(SESSIONS_A_TICK);
            follower.write()?;
        }
    }
    follower.write()
}

/// Start one journalctl following from the day's start, its lines sent on,
/// and a `None` when it ends.
fn spawn(asked: &[&str], lines: Sender<Option<String>>) -> io::Result<()> {
    let mut journalctl = Command::new("journalctl")
        .args(["--follow", "--output=json", "--no-pager", "--since=today"])
        .arg(format!("--output-fields={FIELDS}"))
        .args(asked)
        .stdout(Stdio::piped())
        .spawn()?;
    let out = journalctl
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("no output"))?;
    thread::spawn(move || {
        let mut out = BufReader::new(out);
        let mut line = Vec::new();
        // A line that isn't UTF-8 is read as far as it goes; journalctl
        // writes such a message as bytes, so it's rare.
        while out.read_until(b'\n', &mut line).is_ok_and(|read| read > 0) {
            if lines
                .send(Some(String::from_utf8_lossy(&line).into_owned()))
                .is_err()
            {
                break;
            }
            line.clear();
        }
        let _ = journalctl.wait();
        let _ = lines.send(None);
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A warning line at a moment in microseconds.
    fn warning(at: i64) -> String {
        format!(
            r#"{{"__REALTIME_TIMESTAMP":"{at}","PRIORITY":"4","SYSLOG_IDENTIFIER":"people","MESSAGE":"slow"}}"#
        )
    }

    /// 2026-11-12 at noon, UTC, in microseconds.
    const NOON: i64 = 1_794_484_800_000_000;
    const DAY: i64 = 86_400_000_000;

    fn read(into: &Path, date: &str) -> Day {
        let file = fs::read(file_of(into, date)).expect("the day was written");
        serde_json::from_slice(&file).expect("the day's file is a day")
    }

    #[test]
    fn a_day_is_written_only_once_something_changed() {
        let into = tempfile::tempdir().expect("a folder");
        let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC);
        follower.write().expect("nothing to write");
        assert!(
            !file_of(into.path(), "2026-11-12").exists(),
            "an untouched day isn't written"
        );
        follower.take(&warning(NOON)).expect("a line");
        follower.write().expect("the day is written");
        assert_eq!(
            read(into.path(), "2026-11-12").things["people"].lines,
            1,
            "its line counted"
        );
        assert!(
            fs::read_dir(into.path()).expect("the folder").count() == 1,
            "nothing but the day is left behind"
        );
    }

    #[test]
    fn a_new_day_writes_the_old_and_starts_empty() {
        let into = tempfile::tempdir().expect("a folder");
        let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC);
        follower.take(&warning(NOON)).expect("a line");
        follower.take(&warning(NOON + 1)).expect("a line");
        follower
            .take(&warning(NOON + DAY))
            .expect("the next day's line");
        assert_eq!(
            read(into.path(), "2026-11-12").things["people"].lines,
            2,
            "the old day whole"
        );
        follower.take(&warning(NOON)).expect("a late line");
        follower.write().expect("the new day is written");
        assert_eq!(
            read(into.path(), "2026-11-13").things["people"].lines,
            1,
            "the new day starts empty, and a late line of a day done is left out"
        );
        assert_eq!(
            read(into.path(), "2026-11-12").things["people"].lines,
            2,
            "the old day stays"
        );
    }

    #[test]
    fn a_line_that_isnt_an_entry_changes_nothing() {
        let into = tempfile::tempdir().expect("a folder");
        let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC);
        follower.take("-- No entries --").expect("words");
        follower.write().expect("nothing to write");
        assert!(
            !file_of(into.path(), "2026-11-12").exists(),
            "nothing was taken"
        );
    }

    #[test]
    fn a_moment_is_dated_in_its_zone() {
        assert_eq!(
            date_of(NOON, &TimeZone::UTC),
            "2026-11-12",
            "noon is that day"
        );
        let west = TimeZone::fixed(jiff::tz::offset(-5));
        assert_eq!(
            date_of(NOON - 8 * 3_600_000_000, &west),
            "2026-11-11",
            "04:00 UTC is still the day before five hours west"
        );
    }

    #[test]
    fn a_shutdown_seen_is_timed_to_its_boots_end() {
        let boots = r#"[{"index":-1,"boot_id":"old","first_entry":1,"last_entry":40000000},
            {"index":0,"boot_id":"now","first_entry":41000000,"last_entry":99000000}]"#;
        let ends = ends_of(boots);
        assert_eq!(
            ends.into_iter().collect::<Vec<_>>(),
            vec![("old".to_owned(), 40_000_000)],
            "only the boots before this one have ended"
        );
        let into = tempfile::tempdir().expect("a folder");
        let mut follower =
            Follower::new("1970-01-01", into.path(), TimeZone::UTC).knowing(ends_of(boots));
        follower
            .take(&format!(
                r#"{{"__REALTIME_TIMESTAMP":"10000000","_BOOT_ID":"old","MESSAGE_ID":"{SHUTTING_DOWN}","MESSAGE":"System is rebooting."}}"#
            ))
            .expect("taken");
        assert_eq!(
            follower.day.boots["old"].shutdown(),
            Some(30_000_000),
            "from logind's line to the boot's last entry"
        );
    }

    #[test]
    fn a_busy_journal_still_ticks_every_half_minute() {
        let into = tempfile::tempdir().expect("a folder");
        let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC);
        let (lines, read) = mpsc::channel();
        for second in 0..9 {
            lines
                .send(Some(warning(NOON + second)))
                .expect("a line sent");
        }
        lines.send(None).expect("its end sent");
        // Each look at the clock finds ten seconds gone, so the lines come
        // far faster than the half minute a quiet journal would wait.
        let start = Instant::now();
        let mut asked = 0;
        let now = || {
            asked += 1;
            start + Duration::from_secs(10 * asked)
        };
        let mut looks = 0;
        run(&mut follower, &read, 1, now, || {
            looks += 1;
            Looked::default()
        })
        .expect("the run");
        assert!(
            looks >= 3,
            "looked {looks} times over ten lines a hundred seconds apart"
        );
    }

    #[test]
    fn a_lid_gone_wrong_is_a_devices_fault_once() {
        let into = tempfile::tempdir().expect("a folder");
        let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC);
        let used = |keys| Looked {
            shut: true,
            screen_on: true,
            keys,
        };
        for keys in 1..5 {
            follower.look_at_lid(used(keys), NOON);
        }
        let open = Looked {
            shut: false,
            ..used(5)
        };
        follower.look_at_lid(open, NOON);
        follower.look_at_lid(used(6), NOON);
        follower.look_at_lid(used(7), NOON);
        follower.write().expect("the day is written");
        let lid = &read(into.path(), "2026-11-12").things["lid"];
        assert_eq!(lid.kind, Some(Kind::Devices), "a lid is a device");
        assert_eq!(lid.errors, 2, "once a time it went wrong: {lid:?}");
        let said: Vec<&str> = lid
            .signatures
            .values()
            .map(|said| said.example.as_str())
            .collect();
        assert_eq!(said, [LID_WRONG], "said the way Joel reads it");
    }

    #[test]
    fn the_days_pains_are_kept_as_the_detector_ranks_them() {
        let into = tempfile::tempdir().expect("a folder");
        let pains = tempfile::tempdir().expect("the detector's folder");
        let mut follower =
            Follower::new("2026-11-12", into.path(), TimeZone::UTC).feeling(pains.path());
        follower.feel();
        follower.write().expect("nothing to write");
        assert!(
            !file_of(into.path(), "2026-11-12").exists(),
            "no pains yet is no change"
        );
        fs::write(
            pains.path().join("2026-11-12.jsonl"),
            "{\"trying\":\"sound\",\"rank\":1}\n",
        )
        .expect("the detector's day");
        follower.feel();
        follower.write().expect("the day is written");
        let feel = read(into.path(), "2026-11-12").feel;
        assert_eq!(feel.len(), 1, "the pain kept: {feel:?}");
        assert_eq!(feel[0].trying, "sound", "as the detector said it");
    }

    #[test]
    fn memory_is_minded_and_kept_through_a_restart() {
        let into = tempfile::tempdir().expect("a folder");
        let proc = tempfile::tempdir().expect("a stand-in /proc");
        fs::write(proc.path().join("uptime"), "100.0 1.0\n").expect("uptime");
        let pi = proc.path().join("7");
        fs::create_dir_all(&pi).expect("a pi");
        fs::write(pi.join("cmdline"), "pi\0").expect("its command");
        fs::write(pi.join("status"), "VmRSS:\t 2048 kB\nVmSwap:\t 0 kB\n").expect("status");
        let fields = "S 1 1 1 0 -1 0 0 0 0 0 0 0 0 0 20 0 1 0 0 0 0";
        fs::write(pi.join("stat"), format!("7 (pi) {fields}\n")).expect("stat");
        let mut follower =
            Follower::new("2026-11-12", into.path(), TimeZone::UTC).minding(proc.path());
        follower.mind(NOON);
        fs::remove_dir_all(&pi).expect("the pi ends");
        follower.mind(NOON + 1);
        follower.write().expect("the day is written");
        let day = read(into.path(), "2026-11-12");
        let held = &day.memory["pi"];
        assert_eq!(held.copies, 0, "none running at the last look: {held:?}");
        assert_eq!(
            (held.most_kib, held.most_at),
            (2_048, NOON),
            "the day's most"
        );

        let mut again = Follower::new("2026-11-12", into.path(), TimeZone::UTC)
            .minding(proc.path())
            .resuming(day);
        again.mind(NOON + 2);
        again.write().expect("the day is written again");
        assert_eq!(
            read(into.path(), "2026-11-12").memory["pi"].most_kib,
            2_048,
            "a restart keeps the day's most"
        );
    }

    #[test]
    fn a_death_of_the_day_is_kept_though_its_presence_goes() {
        let into = tempfile::tempdir().expect("a folder");
        let proc = tempfile::tempdir().expect("a stand-in /proc");
        let presences = tempfile::tempdir().expect("the presences");
        let presence = |session: &str, at: &str| {
            let said = format!(
                r#"{{"session":"{session}","state":"busy","at":"{at}","owner":{{"pid":9,"startTicks":1}}}}"#
            );
            fs::write(presences.path().join(format!("{session}.json")), said).expect("a presence");
        };
        presence("yesterday", "2026-11-11T23:00:00Z");
        presence("today", "2026-11-12T11:00:00Z");
        let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC)
            .minding(proc.path())
            .counting_the_dead(presences.path());
        follower.count_the_dead(NOON);
        fs::remove_file(presences.path().join("today.json")).expect("revived");
        follower.count_the_dead(NOON + 1);
        follower.write().expect("the day is written");
        let agents = read(into.path(), "2026-11-12").agents;
        assert_eq!(
            agents.gone.keys().collect::<Vec<_>>(),
            ["yesterday"],
            "gone at the last look"
        );
        assert_eq!(
            agents.died.keys().collect::<Vec<_>>(),
            ["today"],
            "the day's death, kept after its presence went"
        );
    }

    /// A failed bash call at a moment in microseconds, as pi logs it.
    fn failed(at: i64) -> String {
        format!(
            r#"{{"type":"message","message":{{"role":"toolResult","toolName":"bash","isError":true,"timestamp":{},"content":[{{"type":"text","text":"error: no"}}]}}}}"#,
            at / 1_000
        )
    }

    #[test]
    fn the_sessions_logs_are_read_on_and_never_twice() {
        let into = tempfile::tempdir().expect("a folder");
        let sessions = tempfile::tempdir().expect("the sessions' folder");
        let folder = sessions.path().join("--home-joel-src-core.nix--");
        fs::create_dir_all(&folder).expect("a place");
        let log = folder.join("a.jsonl");
        // A line from the day before, then one of the day and half a line;
        // the day is today's, since only a log written to today is read.
        let today = date_of(Timestamp::now().as_microsecond(), &TimeZone::UTC);
        let noon = |at: i64| at + (Timestamp::now().as_microsecond() / DAY - NOON / DAY) * DAY;
        fs::write(
            &log,
            format!(
                "{}\n{}\n{{\"type",
                failed(noon(NOON - DAY)),
                failed(noon(NOON))
            ),
        )
        .expect("a log of today");
        let mut follower =
            Follower::new(&today, into.path(), TimeZone::UTC).hearing(sessions.path());
        follower.hear(1 << 20);
        follower.hear(1 << 20);
        follower.write().expect("the day is written");
        let day = read(into.path(), &today);
        let bash = &day.things["pi bash"];
        assert_eq!(bash.kind, Some(Kind::Tools), "a tool's");
        assert_eq!(bash.errors, 1, "the day's line, once: {bash:?}");
        let kept = day.read[&log.to_string_lossy().into_owned()];
        let written = fs::read(&log).expect("the log");
        let whole = written
            .iter()
            .rposition(|&byte| byte == b'\n')
            .expect("a line")
            + 1;
        assert_eq!(kept, whole as u64, "read to its last whole line");

        let mut again = Follower::new(&today, into.path(), TimeZone::UTC)
            .hearing(sessions.path())
            .resuming(day);
        again.hear(1 << 20);
        again.write().expect("the day is written again");
        assert_eq!(
            read(into.path(), &today).things["pi bash"].errors,
            1,
            "a restart reads on from where the last run stopped"
        );
    }
}
