//! `health follow`, the daemon: it follows the journals from the start of
//! the day, tallies what they say, and keeps the day's file current every
//! half minute, so `health` and `cadence day` read it without asking
//! anything that runs.

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
use crate::journal::{self, RESTART_SCHEDULED};

/// How often a changed day is written out: often enough that what `health`
/// shows is a moment old, seldom enough to cost nothing.
const EVERY: Duration = Duration::from_secs(30);

/// The fields each entry is read by; journalctl always adds its time.
const FIELDS: &str = "MESSAGE,PRIORITY,SYSLOG_IDENTIFIER,_COMM,USER_UNIT,UNIT,_SYSTEMD_UNIT,\
                      _SYSTEMD_USER_UNIT,COREDUMP_COMM,COREDUMP_CGROUP,MESSAGE_ID,_TRANSPORT";

/// The day being tallied, and where it's written.
pub struct Follower {
    day: Day,
    changed: bool,
    into: PathBuf,
    zone: TimeZone,
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
        }
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

/// Follow the journals into `into` until both followers end, which only a
/// fault does; then the day is written and systemd starts it again.
///
/// # Errors
///
/// When a follower can't start or the day can't be written.
pub fn follow(into: &Path) -> io::Result<()> {
    let zone = TimeZone::system();
    let today = date_of(Timestamp::now().as_microsecond(), &zone);
    let mut follower = Follower::new(&today, into, zone);
    let (lines, read) = mpsc::channel();
    // Warnings and graver, and the restarts systemd schedules, which it
    // says only as information.
    spawn(&["-p", "warning"], lines.clone())?;
    spawn(&[&format!("MESSAGE_ID={RESTART_SCHEDULED}")], lines)?;
    let mut running = 2;
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
}
