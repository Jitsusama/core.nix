//! `health follow`, the daemon: it follows the journals from the start of
//! the day, tallies what they say, and keeps the day's file current every
//! half minute, so `health` and `cadence day` read it without asking
//! anything that runs.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, BufRead as _, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

use jiff::Timestamp;
use jiff::tz::TimeZone;

use crate::day::Day;
use crate::journal::{
    self, LAUNCHER_READY, Mark, RESTART_SCHEDULED, SHUTTING_DOWN, STARTUP_FINISHED, UNIT_STARTED,
};

/// How often a changed day is written out: often enough that what `health`
/// shows is a moment old, seldom enough to cost nothing.
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
}

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
        }
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
            self.day = Day::new(&date);
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
    fs::write(&part, serde_json::to_vec(day).map_err(io::Error::other)?)?;
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
    let mut running = 4;
    while running > 0 {
        match read.recv_timeout(EVERY) {
            Ok(Some(line)) => follower.take(&line)?,
            Ok(None) => running -= 1,
            Err(RecvTimeoutError::Timeout) => follower.write()?,
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    follower.write()?;
    Err(io::Error::other("the journal followers ended"))
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
}
