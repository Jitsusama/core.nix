//! Where the day's lines come from: systemd's journals on Linux, the
//! unified log on a Mac. The follower takes one source and asks it for its
//! lines, how to read each, when each boot before this one ended, and where
//! the processes and the system's disk guard are to look at; nothing else in
//! health knows which system it runs on.

use std::collections::BTreeMap;
use std::io::{self, BufRead as _, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::mpsc::Sender;
use std::thread;

use jiff::Zoned;

use crate::follow;
use crate::journal::{self, Entry};
use crate::replay;
use crate::reports;
use crate::unified;

/// What the day's lines are read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// systemd's journals, through journalctl.
    Journal,
    /// A Mac's unified log, through `log`.
    UnifiedLog,
}

impl Source {
    /// The source of the system health runs on.
    #[must_use]
    pub const fn here() -> Self {
        if cfg!(target_os = "macos") {
            Self::UnifiedLog
        } else {
            Self::Journal
        }
    }

    /// The entry a line of this source's holds, or none.
    #[must_use]
    pub fn parse(self, line: &str) -> Option<Entry> {
        match self {
            Self::Journal => journal::parse(line),
            Self::UnifiedLog => unified::parse(line),
        }
    }

    /// Start each of this source's followers from the start of the day it
    /// is `now`, their lines sent on, saying how many will end. The
    /// journal's day so far is sent in the order it was written, then
    /// each follower follows on (`replay.rs`).
    ///
    /// # Errors
    ///
    /// When one can't start.
    pub fn start(self, now: &Zoned, lines: &Sender<Option<String>>) -> io::Result<usize> {
        let asked = match self {
            Self::Journal => return replayed(now, lines),
            Self::UnifiedLog => {
                let midnight = format!("{} 00:00:00", now.date());
                let now = now.strftime("%Y-%m-%d %H:%M:%S").to_string();
                unified::followed(&midnight, &now)
                    .iter()
                    .map(|asked| {
                        let mut log = Command::new(unified::LOG);
                        log.args(asked);
                        log
                    })
                    .collect::<Vec<_>>()
            }
        };
        let count = asked.len();
        for command in asked {
            spawn(command, lines.clone())?;
        }
        Ok(count)
    }

    /// When each boot before this one ended. A Mac's log marks no boot's
    /// stages, so there none is timed.
    ///
    /// # Errors
    ///
    /// When journalctl can't be asked.
    pub fn ends(self) -> io::Result<BTreeMap<String, i64>> {
        match self {
            Self::Journal => {
                let boots = Command::new("journalctl")
                    .args(["--list-boots", "--output=json", "--no-pager"])
                    .output()?;
                Ok(follow::ends_of(&String::from_utf8_lossy(&boots.stdout)))
            }
            Self::UnifiedLog => Ok(BTreeMap::new()),
        }
    }

    /// Where each process can be looked at, which only Linux has: a Mac's
    /// memory is asked of `ps` instead, and the agents that died there
    /// aren't read yet.
    #[must_use]
    pub fn processes(self) -> Option<&'static Path> {
        match self {
            Self::Journal => Some(Path::new("/proc")),
            Self::UnifiedLog => None,
        }
    }

    /// Whether the watched programs' memory is asked of `ps`, as on a Mac,
    /// which has no `/proc` to look in.
    #[must_use]
    pub const fn asks_ps(self) -> bool {
        matches!(self, Self::UnifiedLog)
    }

    /// Where each group of processes says what processor time it used,
    /// which only Linux's cgroups say.
    #[must_use]
    pub fn cgroups(self) -> Option<&'static Path> {
        match self {
            Self::Journal => Some(Path::new("/sys/fs/cgroup")),
            Self::UnifiedLog => None,
        }
    }

    /// Where crash reports are written, under the account's `home`, which
    /// only a Mac has: Linux's core dumps are said in the journal.
    #[must_use]
    pub fn reports(self, home: Option<&Path>) -> Vec<PathBuf> {
        match self {
            Self::Journal => Vec::new(),
            Self::UnifiedLog => reports::folders(home),
        }
    }

    /// Where the system's disk guard says how the disks stand.
    #[must_use]
    pub fn guard_status(self) -> PathBuf {
        match self {
            Self::Journal => PathBuf::from("/run/disk-guard/status.json"),
            Self::UnifiedLog => PathBuf::from("/var/run/disk-guard/status.json"),
        }
    }
}

/// Read each journal follower's day so far, the day it is `now`, and send
/// it on in the order it was written, then start each following from where
/// its history ended, a `None` sent for each that ends and one for the
/// replay. Says how many `None`s will come.
///
/// Lines written while the histories are merged are heard once their
/// followers follow on, each at its own pace, as every line after the
/// replay is: a switch writes its reload before the restarts it asks for.
fn replayed(now: &Zoned, lines: &Sender<Option<String>>) -> io::Result<usize> {
    let (since, until) = day_of(now)?;
    let histories = journal::followed()
        .into_iter()
        .map(|asked| Ok((History::of(journal::reading(&asked, since, until))?, asked)))
        .collect::<io::Result<Vec<_>>>()?;
    let count = histories.len() + 1;
    let lines = lines.clone();
    thread::spawn(move || {
        replaying(
            histories,
            |asked: &Vec<String>, after| journal::following(asked, after, since),
            &lines,
        );
    });
    Ok(count)
}

/// The replay, then one `None` for it, as `replayed` counts: `replay`'s
/// followers each send their own.
///
/// A follower that couldn't start failed health's start, for systemd to
/// start it again. One that can't follow on after the replay, or a replay
/// that fails any other way, a panic included, ends every follower's
/// count instead, so health ends as when they all have rather than
/// counting the day without their lines; a count ended twice is never
/// read.
fn replaying<A>(
    histories: Vec<(History, A)>,
    follow_on: impl Fn(&A, Option<&str>) -> Command,
    lines: &Sender<Option<String>>,
) {
    let mut ends = Ends {
        lines: lines.clone(),
        left: histories.len() + 1,
    };
    match replay(histories, follow_on, lines) {
        Ok(()) => ends.left = 1,
        Err(error) => {
            eprintln!("health: a follower couldn't follow on from the day's replay: {error}");
        }
    }
}

/// The `None`s still owed when the replay ends, sent however it ends.
struct Ends {
    lines: Sender<Option<String>>,
    left: usize,
}

impl Drop for Ends {
    fn drop(&mut self) {
        for _ in 0..self.left {
            let _ = self.lines.send(None);
        }
    }
}

/// Where every history of the day it is `now` reads from and to, in
/// seconds since 1970: the day's start, and `now`. Worked out once, so
/// every history and every follower whose history was empty reads the same
/// day, however long the replay takes or whenever midnight passes.
fn day_of(now: &Zoned) -> io::Result<(i64, i64)> {
    let start = now.start_of_day().map_err(io::Error::other)?;
    Ok((start.timestamp().as_second(), now.timestamp().as_second()))
}

/// Send `histories`' lines on in the order they were written, then start
/// each one's follower, made by `follow_on` from what the history asked
/// and its last cursor, each sending a `None` when it ends. Starts none
/// once nothing hears the lines.
///
/// # Errors
///
/// When a follower can't start.
fn replay<A>(
    histories: Vec<(History, A)>,
    follow_on: impl Fn(&A, Option<&str>) -> Command,
    lines: &Sender<Option<String>>,
) -> io::Result<()> {
    let (histories, asked): (Vec<_>, Vec<_>) = histories.into_iter().unzip();
    let mut heard = true;
    let ends = replay::merged(histories, |line| {
        heard = lines.send(Some(line)).is_ok();
        heard
    });
    if !heard {
        return Ok(());
    }
    for (asked, end) in asked.iter().zip(&ends) {
        let command = follow_on(asked, end.as_deref());
        let name = named(&command);
        spawn(command, lines.clone())
            .map_err(|error| io::Error::new(error.kind(), format!("{name}: {error}")))?;
    }
    Ok(())
}

/// What tells `command`, a history or a follower, from the others, for
/// what health says of it: what it asks, without the arguments every
/// one of them gives, each matched by its name before any `=`.
fn named(command: &Command) -> String {
    const EVERY: [&str; 8] = [
        "--output",
        "--no-pager",
        "--since",
        "--until",
        "--boot",
        "--follow",
        "--after-cursor",
        "--output-fields",
    ];
    let asked: Vec<String> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .filter(|arg| !EVERY.contains(&arg.split('=').next().unwrap_or_default()))
        .collect();
    format!(
        "{} {}",
        command.get_program().to_string_lossy(),
        asked.join(" ")
    )
    .trim_end()
    .to_owned()
}

/// A command's output read line by line until it ends, for the replay.
struct History {
    child: Child,
    out: BufReader<ChildStdout>,
    /// How many lines it has given.
    read: usize,
    /// Whether it asked `--grep`, which ends as grep does.
    greps: bool,
    /// What it asks, for what health says of it.
    name: String,
}

impl History {
    /// Start `command`, its output to be read.
    fn of(mut command: Command) -> io::Result<Self> {
        let greps = command
            .get_args()
            .any(|arg| arg.to_string_lossy().starts_with("--grep="));
        let name = named(&command);
        let mut child = command
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|error| io::Error::new(error.kind(), format!("{name}: {error}")))?;
        let out = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("no output"))?;
        Ok(Self {
            child,
            out: BufReader::new(out),
            read: 0,
            greps,
            name,
        })
    }

    /// Say so when the history ended other than whole: its follower still
    /// follows on from its last line, but the rest of its day comes at
    /// that follower's pace rather than in the order it was written.
    /// Says whether it was whole.
    fn ended(&mut self) -> bool {
        match self.child.wait() {
            Ok(status) if whole(status.success(), status.code(), self.read, self.greps) => true,
            Ok(status) => {
                eprintln!(
                    "health: the day's history asking {} ended early, {status}",
                    self.name
                );
                false
            }
            Err(error) => {
                eprintln!(
                    "health: the day's history asking {} can't be waited on: {error}",
                    self.name
                );
                false
            }
        }
    }
}

/// Whether a history that `succeeded`, or else ended with `code`, having
/// given `read` lines and asked `--grep` if it `greps`, was whole.
/// journalctl ends as grep does when its `--grep` matched nothing, with 1
/// and no lines, which is a whole history on a quiet day: in JSON it
/// writes nothing then, where plain output writes "-- No entries --"
/// (systemd 262, 18 November). journalctl ends with 1 on any other
/// failure too, so a history that didn't ask `--grep` and ended with 1
/// ended early.
fn whole(succeeded: bool, code: Option<i32>, read: usize, greps: bool) -> bool {
    succeeded || (greps && read == 0 && code == Some(1))
}

impl Iterator for History {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        let mut line = Vec::new();
        // A line that isn't UTF-8 is read as far as it goes, as `spawn`
        // reads one.
        match self.out.read_until(b'\n', &mut line) {
            Ok(read) if read > 0 => {
                self.read += 1;
                Some(String::from_utf8_lossy(&line).into_owned())
            }
            Ok(_) => {
                let _ = self.ended();
                None
            }
            Err(error) => {
                eprintln!(
                    "health: the day's history asking {} couldn't be read on: {error}",
                    self.name
                );
                None
            }
        }
    }
}

impl Drop for History {
    /// A history the replay stopped reading is ended, and every one is
    /// waited on, so none is left behind.
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Start `command`, its output's lines sent on, and a `None` when it ends.
fn spawn(mut command: Command, lines: Sender<Option<String>>) -> io::Result<()> {
    let mut child = command.stdout(Stdio::piped()).spawn()?;
    let out = child
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
        let _ = child.wait();
        let _ = lines.send(None);
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::panic::{self, AssertUnwindSafe};
    use std::sync::mpsc;
    use std::time::Duration;

    use jiff::Timestamp;
    use jiff::tz::{self, TimeZone};

    use super::*;

    /// A shell running `script`: the build's own where it gives one, which
    /// a build's sandbox has where it has no /bin/sh.
    fn sh(script: &str) -> Command {
        let shell = std::env::var("STAND_IN_SHELL").unwrap_or_else(|_| "/bin/sh".to_owned());
        let mut sh = Command::new(shell);
        sh.args(["-c", script]);
        sh
    }

    /// A command writing `lines`, one to a line. printf given none still
    /// writes an empty line, so for none it's a command that writes
    /// nothing.
    fn writing(lines: &[String]) -> Command {
        if lines.is_empty() {
            return sh(":");
        }
        let quoted: Vec<String> = lines.iter().map(|line| format!("'{line}'")).collect();
        sh(&format!("printf '%s\\n' {}", quoted.join(" ")))
    }

    fn numbered(number: u64, cursor: &str) -> String {
        format!(
            r#"{{"__CURSOR":"{cursor}","__SEQNUM_ID":"s","__SEQNUM":"{number}","__REALTIME_TIMESTAMP":"{number}"}}"#
        )
    }

    fn history(lines: &[String]) -> History {
        History::of(writing(lines)).expect("the shell starts")
    }

    #[test]
    fn a_history_is_read_to_its_last_line() {
        let history = History::of(sh("printf 'a\\nb'")).expect("the shell starts");
        assert_eq!(
            history.collect::<Vec<_>>(),
            ["a\n", "b"],
            "a history lost a line, or its last"
        );
    }

    #[test]
    fn a_history_the_replay_stops_reading_is_ended_and_waited_on() {
        // The shell says who it is, then writes far longer than the test
        // waits, and on once its pipe closes, so only a kill ends it in
        // time. A failed test leaves it to end by itself, its errors unsaid.
        // A history that only waited would hold its thread, and one that
        // didn't wait would leave it behind.
        let (dropped, done) = mpsc::channel();
        let reading = thread::spawn(move || {
            let writes = "i=0; while [ $i -lt 10000000 ]; do echo y; i=$((i+1)); done";
            let mut history = History::of(sh(&format!(
                "echo $$; exec 2>/dev/null; trap '' PIPE; {writes}"
            )))
            .expect("the shell starts");
            let pid = history.next().unwrap_or_default().trim().to_owned();
            drop(history);
            let _ = dropped.send(());
            pid
        });
        assert!(
            done.recv_timeout(Duration::from_secs(10)).is_ok(),
            "a history dropped was waited on without being ended"
        );
        let pid = reading.join().expect("the history was read");
        if cfg!(target_os = "linux") {
            assert!(
                !Path::new(&format!("/proc/{pid}")).exists(),
                "a history dropped was left behind, as {pid}"
            );
        }
    }

    #[test]
    fn the_replay_sends_the_day_in_order_then_each_follows_on_from_its_own_end() {
        let histories = vec![
            (history(&[numbered(1, "a1"), numbered(3, "a3")]), 0),
            (history(&[numbered(2, "b2")]), 1),
            (history(&[]), 2),
        ];
        let (send, heard) = mpsc::channel();
        replay(
            histories,
            |at, after| writing(&[format!("{at} after {}", after.unwrap_or("nothing"))]),
            &send,
        )
        .expect("every follower starts");
        drop(send);
        let heard: Vec<Option<String>> = heard.iter().collect();
        let day: Vec<&str> = heard
            .iter()
            .take(3)
            .flatten()
            .filter_map(|line| line.split('"').nth(3))
            .collect();
        assert_eq!(
            day,
            ["a1", "b2", "a3"],
            "the day wasn't sent first, in order"
        );
        let mut followed: Vec<&str> = heard.iter().skip(3).flatten().map(String::as_str).collect();
        followed.sort_unstable();
        assert_eq!(
            followed,
            ["0 after a3\n", "1 after b2\n", "2 after nothing\n"],
            "a follower didn't follow on from its own history's end"
        );
        assert_eq!(
            heard.iter().filter(|line| line.is_none()).count(),
            3,
            "a follower ended without a word, or twice"
        );
    }

    #[test]
    fn the_replay_starts_no_follower_once_nothing_hears_it() {
        let (send, heard) = mpsc::channel();
        drop(heard);
        let made = Cell::new(0);
        let replayed = replay(
            vec![(history(&[numbered(1, "a1")]), ())],
            |(), _| {
                made.set(made.get() + 1);
                writing(&[])
            },
            &send,
        );
        assert!(
            replayed.is_ok() && made.get() == 0,
            "a follower was started with nothing to hear it"
        );
    }

    #[test]
    fn a_follower_that_cant_follow_on_is_an_error_naming_it() {
        let (send, _heard) = mpsc::channel();
        let replayed = replay(
            vec![(history(&[numbered(1, "a1")]), ())],
            |(), _| {
                let mut follower = Command::new("/nonexistent/journalctl");
                follower.args(["--follow", "-k"]);
                follower
            },
            &send,
        );
        assert!(
            replayed.is_err_and(|error| error
                .to_string()
                .starts_with("/nonexistent/journalctl -k: ")),
            "a follower that couldn't start was let pass, or wasn't named"
        );
    }

    #[test]
    fn a_history_and_its_follower_are_named_by_what_they_ask() {
        // One asked only shares a shared argument's start, and is kept.
        let asked = [
            "-p".to_owned(),
            "warning".to_owned(),
            "--output-x".to_owned(),
        ];
        let name = "journalctl -p warning --output-x".to_owned();
        assert_eq!(
            (
                named(&journal::reading(&asked, 1, 2)),
                named(&journal::following(&asked, Some("s=1"), 1)),
                named(&journal::following(&asked, None, 1)),
            ),
            (name.clone(), name.clone(), name),
            "a history or follower was named by what every one asks"
        );
    }

    #[test]
    fn every_history_reads_from_the_days_start_to_now() {
        // 21:00 on 17 November, five hours behind UTC, where it's already
        // the 18th.
        let now = Timestamp::from_second(1_794_967_200)
            .expect("a time")
            .to_zoned(TimeZone::fixed(tz::offset(-5)));
        assert_eq!(
            day_of(&now).expect("the day has a start"),
            (1_794_891_600, 1_794_967_200),
            "the replay didn't read its own day, from its midnight to now"
        );
    }

    /// Every `None` `replaying` and its followers send, once all are done,
    /// however the replay ends.
    fn ends_of(follow_on: impl Fn(&usize, Option<&str>) -> Command) -> usize {
        let histories = vec![
            (history(&[numbered(1, "a1")]), 0),
            (history(&[numbered(2, "b2")]), 1),
        ];
        let (send, heard) = mpsc::channel();
        let _ = panic::catch_unwind(AssertUnwindSafe(|| {
            replaying(histories, follow_on, &send);
        }));
        drop(send);
        heard.iter().filter(Option::is_none).count()
    }

    #[test]
    fn a_whole_replay_ends_once_for_each_follower_and_once_for_itself() {
        assert_eq!(
            ends_of(|_, _| writing(&[])),
            3,
            "a replay's ends didn't match what it counts"
        );
    }

    #[test]
    fn a_replay_whose_follower_cant_follow_on_ends_every_count() {
        // The first follower ends at once, and says so as well; a real one
        // follows for good, so the replay's own three must come whole.
        let ends = ends_of(|at, _| {
            if *at == 0 {
                writing(&[])
            } else {
                Command::new("/nonexistent/journalctl")
            }
        });
        assert_eq!(
            ends, 4,
            "health would wait for good, with {ends} of 3 ends and the follower's"
        );
    }

    #[test]
    fn a_replay_that_panics_ends_every_count() {
        assert_eq!(
            ends_of(|_, _| panic!("a stand-in replay's panic")),
            3,
            "a replay that panicked would leave health waiting for good"
        );
    }

    #[test]
    fn a_history_that_finds_nothing_is_whole_only_if_it_asked_grep() {
        assert!(
            whole(true, Some(0), 5, false),
            "a history that succeeded wasn't whole"
        );
        assert!(
            whole(false, Some(1), 0, true),
            "a quiet day's grep that found nothing wasn't whole"
        );
        assert!(
            !whole(false, Some(1), 1, true),
            "a grep that failed after a line was taken as whole"
        );
        assert!(
            !whole(false, Some(1), 0, false),
            "a history that didn't ask grep and failed was taken as whole"
        );
        assert!(
            !whole(false, Some(2), 0, true) && !whole(false, None, 0, true),
            "a grep that failed or was killed before a line was taken as whole"
        );
    }

    #[test]
    fn a_history_ending_in_failure_is_whole_only_as_a_grep_that_found_nothing() {
        let mut greps = sh("exit 1");
        greps.arg("--grep=x");
        let mut asked = History::of(greps).expect("the shell starts");
        assert_eq!(
            asked.next(),
            None,
            "a history that wrote nothing gave a line"
        );
        assert!(asked.ended(), "a grep that found nothing ended early");
        let mut failed = History::of(sh("exit 1")).expect("the shell starts");
        assert_eq!(
            failed.next(),
            None,
            "a history that wrote nothing gave a line"
        );
        assert!(!failed.ended(), "a history that failed was taken as whole");
        assert_eq!(
            failed.name,
            format!("{} -c exit 1", sh("").get_program().to_string_lossy()),
            "a history wasn't named by what it asked, for what health says of it"
        );
    }

    #[test]
    fn a_history_that_cant_start_is_an_error_naming_it() {
        let mut history = Command::new("/nonexistent/journalctl");
        history.args(["--since=@1", "-k"]);
        assert!(
            History::of(history).is_err_and(|error| error
                .to_string()
                .starts_with("/nonexistent/journalctl -k: ")),
            "a history that couldn't start wasn't named"
        );
    }

    #[test]
    fn every_history_and_follower_has_a_name_of_its_own() {
        let followed = journal::followed();
        let forms: [fn(&[String]) -> Command; 3] = [
            |asked| journal::reading(asked, 1, 2),
            |asked| journal::following(asked, Some("s=1"), 1),
            |asked| journal::following(asked, None, 1),
        ];
        for command in forms {
            let mut names: Vec<String> = followed
                .iter()
                .map(|asked| named(&command(asked)))
                .collect();
            names.sort_unstable();
            names.dedup();
            assert_eq!(
                names.len(),
                followed.len(),
                "two histories or followers share a name: {names:?}"
            );
        }
    }

    #[test]
    fn each_source_reads_its_own_lines() {
        let journal = r#"{"__REALTIME_TIMESTAMP":"1","PRIORITY":"3","MESSAGE":"m"}"#;
        let unified = r#"{"timestamp":"2026-11-12 05:55:36.0-0500","messageType":"Error","eventType":"logEvent","processImagePath":"/usr/libexec/d","eventMessage":"m"}"#;
        assert!(
            Source::Journal.parse(journal).is_some() && Source::Journal.parse(unified).is_none(),
            "the journal reads the journal's lines alone"
        );
        assert!(
            Source::UnifiedLog.parse(unified).is_some()
                && Source::UnifiedLog.parse(journal).is_none(),
            "the unified log reads its own lines alone"
        );
    }
}
