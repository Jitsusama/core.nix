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
mod tests;
