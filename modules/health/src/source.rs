//! Where the day's lines come from: systemd's journals on Linux, the
//! unified log on a Mac. The follower takes one source and asks it for its
//! lines, how to read each, when each boot before this one ended, and where
//! the processes and the system's disk guard are to look at; nothing else in
//! health knows which system it runs on.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::io::{self, BufRead as _, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, ExitStatus, Output, Stdio};
use std::sync::OnceLock;
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
    /// When a Mac's log can't start, or the day's start can't be told. A
    /// journal's history or follower that can't start is said, and ends
    /// every count (`replayed_by`).
    pub fn start(self, now: &Zoned, lines: &Sender<Option<String>>) -> io::Result<usize> {
        let asked = match self {
            Self::Journal => return replayed(now, lines),
            Self::UnifiedLog => {
                let midnight = format!("{} 00:00:00", now.date());
                let now = now.strftime("%Y-%m-%d %H:%M:%S").to_string();
                unified::commands(&midnight, &now)
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
/// Every history ends at the journal's last line as the replay begins,
/// where journald numbered it (`replayed_by`), or reads to its own end
/// where that can't be read (`last_of`). Lines written since are heard
/// once their followers follow on, each at its own pace, as every line
/// after the replay is: a switch writes its reload before the restarts
/// it asks for.
fn replayed(now: &Zoned, lines: &Sender<Option<String>>) -> io::Result<usize> {
    let since = day_of(now)?;
    Ok(replayed_by(
        journal::newest(),
        move |asked| journal::reading(asked, since),
        move |asked, after| journal::following(asked, after, since),
        lines,
    ))
}

/// `replayed`, with the journalctls it asks made by `newest`, `reading`
/// from what each history asks, and `following` from that and where its
/// history ended, so a test can point them at a journal of its own. Says
/// how many `None`s will come.
///
/// All of it runs on the replay's own thread, so health's tick starts
/// however long journalctl takes. `newest` answers before any history
/// starts, so each reads past the line it named, and none can end before
/// it, short of a line written meanwhile. One that can't start ends every
/// count, so health ends, for systemd to start it again.
fn replayed_by(
    newest: Command,
    reading: impl Fn(&[String]) -> Command + Send + 'static,
    following: impl Fn(&[String], Option<&str>) -> Command + Send + 'static,
    lines: &Sender<Option<String>>,
) -> usize {
    let count = journal::followed().len() + 1;
    let lines = lines.clone();
    thread::spawn(move || {
        let mut ends = Ends {
            lines: lines.clone(),
            left: count,
        };
        let started = last_written(newest).and_then(|last| {
            let histories = journal::followed()
                .into_iter()
                .map(|asked| Ok((History::of(reading(&asked))?, asked)))
                .collect::<io::Result<Vec<_>>>()?;
            Ok((last, histories))
        });
        match started {
            Ok((last, histories)) => {
                ends.left = 0;
                replaying(
                    histories,
                    last.as_ref(),
                    |asked: &Vec<String>, after| following(asked, after),
                    &lines,
                );
            }
            Err(error) => fail(&format!("health: the day's replay couldn't start: {error}")),
        }
    });
    count
}

/// Where `newest` says journald numbered the journal's last line
/// (`last_of`).
///
/// # Errors
///
/// When it can't start, as when a history can't.
fn last_written(newest: Command) -> io::Result<Option<replay::Numbered>> {
    let name = named(&newest);
    Ok(last_of(&name, asking(newest)?.wait_with_output(), warn))
}

/// `newest` started, its stderr health's own.
///
/// # Errors
///
/// When it can't start, said with its name.
fn asking(mut newest: Command) -> io::Result<Child> {
    let name = named(&newest);
    newest
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| io::Error::new(error.kind(), format!("{name}: {error}")))
}

/// Where journald numbered the journal's last line, as `newest`, named
/// `name`, said it, or nothing of a journal with none.
///
/// One that ends in failure, or whose line no number is read of, gives
/// none too, said through `say` (on stderr, `last_written`): the replay
/// goes on, its histories each read to whatever end it reaches, with no
/// end in common, so lines written meanwhile may be heard out of the
/// order they were written, rather than health not starting at all. A
/// line of a counter no history's lines carry gives no end in common
/// too, unsaid: the system's journal and the account's share one
/// (`journal::newest`).
fn last_of(name: &str, said: io::Result<Output>, say: impl Fn(&str)) -> Option<replay::Numbered> {
    let said = match said {
        Ok(said) => said,
        Err(error) => {
            say(&format!(
                "health: {name} couldn't be read: {error}, so lines written as the day is read \
                 again may be heard out of order"
            ));
            return None;
        }
    };
    if !said.status.success() {
        say(&format!(
            "health: {name} ended, {}, so lines written as the day is read again may be heard \
             out of order",
            said.status
        ));
        return None;
    }
    let line = String::from_utf8_lossy(&said.stdout)
        .lines()
        .last()
        .map(str::to_owned)?;
    let last = replay::numbering(&line);
    if last.is_none() {
        say(&format!(
            "health: {name} said a line journald didn't number, so lines written as the \
             day is read again may be heard out of order"
        ));
    }
    last
}

/// The replay, then one `None` for it, as `replayed` counts: `replay`'s
/// followers each send their own.
///
/// A follower that can't follow on after the replay, or a replay that
/// fails any other way, a panic included, ends every follower's count
/// instead, so health ends as when they all have rather than
/// counting the day without their lines; a count ended twice is never
/// read.
fn replaying<A>(
    histories: Vec<(History, A)>,
    last: Option<&replay::Numbered>,
    follow_on: impl Fn(&A, Option<&str>) -> Command,
    lines: &Sender<Option<String>>,
) {
    let mut ends = Ends {
        lines: lines.clone(),
        left: histories.len() + 1,
    };
    match replay(histories, last, follow_on, lines) {
        Ok(()) => ends.left = 1,
        Err(error) => {
            fail(&format!(
                "health: a follower couldn't follow on from the day's replay: {error}"
            ));
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

/// Where every history of the day it is `now` reads from, in seconds since
/// 1970: the day's start. Worked out once, so every history and every
/// follower whose history was empty reads the same day, however long the
/// replay takes or whenever midnight passes.
fn day_of(now: &Zoned) -> io::Result<i64> {
    let start = now.start_of_day().map_err(io::Error::other)?;
    Ok(start.timestamp().as_second())
}

/// Send `histories`' lines on in the order they were written, each to
/// the journal's `last` line as the replay began (`replay::merged`), then
/// start each one's follower, made by `follow_on` from what the history
/// asked and its last cursor, each sending a `None` when it ends. Starts
/// none once nothing hears the lines.
///
/// # Errors
///
/// When a follower can't start.
fn replay<A>(
    histories: Vec<(History, A)>,
    last: Option<&replay::Numbered>,
    follow_on: impl Fn(&A, Option<&str>) -> Command,
    lines: &Sender<Option<String>>,
) -> io::Result<()> {
    let (histories, asked): (Vec<_>, Vec<_>) = histories.into_iter().unzip();
    let mut heard = true;
    let ends = replay::merged(histories, last, |line| {
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
    const EVERY: [&str; 7] = [
        "--output",
        "--no-pager",
        "--since",
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
                warn(&format!(
                    "health: the day's history asking {} ended early, {status}",
                    self.name
                ));
                false
            }
            Err(error) => {
                warn(&format!(
                    "health: the day's history asking {} can't be waited on: {error}",
                    self.name
                ));
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
                warn(&format!(
                    "health: the day's history asking {} couldn't be read on: {error}",
                    self.name
                ));
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
///
/// Health says so when one ends, unless it `ends_whole` by design and
/// did (`follower_ended`).
fn spawn(command: Command, lines: Sender<Option<String>>) -> io::Result<()> {
    spawn_saying(command, lines, warn)
}

/// `spawn`, with what it says of its end said through `say`, before its
/// `None`.
fn spawn_saying(
    mut command: Command,
    lines: Sender<Option<String>>,
    say: impl Fn(&str) + Send + 'static,
) -> io::Result<()> {
    let name = named(&command);
    let ends_whole = ends_whole(&command);
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
        if let Some(said) = follower_ended(&name, child.wait(), ends_whole) {
            say(&said);
        }
        let _ = lines.send(None);
    });
    Ok(())
}

/// How health is started again where systemd runs it, on Linux: a
/// follower's end doesn't end health while another still follows, so
/// nothing else does it until a switch that changes health, the next
/// boot, or the last follower's end, which systemd starts it again on.
/// A Mac's agent is started again as launchd names it, which isn't told
/// here.
const STARTS_AGAIN: &str = if cfg!(target_os = "linux") {
    ", as systemctl --user restart health does"
} else {
    ""
};

/// Whether `command` ends whole by design: only a Mac's `log show`, which
/// reads the day and ends so on every start. Every other follower, a
/// journal's or a Mac's `log stream`, follows for as long as health runs.
fn ends_whole(command: &Command) -> bool {
    command.get_program() == OsStr::new(unified::LOG)
        && command.get_args().next() == Some(OsStr::new("show"))
}

/// What health says of the follower named `name` that ended as `waited`
/// says, or nothing of one that `ends_whole` by design and did.
///
/// Health runs on until every follower has ended, so the lines one asked
/// for would otherwise go unheard until health next starts, with nothing
/// said. So any end of a follower that follows for as long as health runs
/// is said, a whole one too: journalctl ends whole on a SIGTERM or SIGINT
/// (systemd 262, 18 November). Of one that ends whole by design, only a
/// failure is.
fn follower_ended(name: &str, waited: io::Result<ExitStatus>, ends_whole: bool) -> Option<String> {
    match waited {
        Ok(status) if ends_whole && status.success() => None,
        Ok(status) => Some(format!(
            "health: the follower asking {name} ended, {status}, so what it asks goes unheard \
             until health starts again{STARTS_AGAIN}"
        )),
        Err(error) => Some(format!(
            "health: the follower asking {name} can't be waited on: {error}"
        )),
    }
}

/// Say `words` on stderr as a warning (`as_marked`).
fn warn(words: &str) {
    say_marked(words, WARNING);
}

/// Say `words` on stderr as a failure (`as_marked`): what health ends
/// on, its own error line saying only that the followers ended.
fn fail(words: &str) {
    say_marked(words, ERROR);
}

/// sd-daemon(3)'s mark for a warning.
const WARNING: u8 = 4;

/// sd-daemon(3)'s mark for an error.
const ERROR: u8 = 3;

fn say_marked(words: &str, mark: u8) {
    static JOURNAL: OnceLock<bool> = OnceLock::new();
    let journal = *JOURNAL.get_or_init(|| terminal_kit::Environment::here().journal);
    eprintln!("{}", as_marked(words, journal, mark));
}

/// `words` as the journal reads them at `mark`, where stderr is the
/// `journal`'s stream: each line opened with sd-daemon(3)'s `<mark>`, so
/// `journalctl -p warning` finds a warning and `-p err` a failure, as the
/// kit marks each line of a failure (docs/commands.md). Health's warnings
/// follower hears them, so its own day counts health's warnings as any
/// program's, a follower gone quiet among them, each once, as a restart's
/// replay counts the day again from the journal. The warnings follower's
/// own end is counted only from health's next start, since it was the
/// one to hear it. Elsewhere, as a Mac's log file, they're as they are.
fn as_marked(words: &str, journal: bool, mark: u8) -> String {
    if !journal {
        return words.to_owned();
    }
    words
        .lines()
        .map(|line| format!("<{mark}>{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests;
