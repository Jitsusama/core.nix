//! Where the day's lines come from: systemd's journals on Linux, the
//! unified log on a Mac. The follower takes one source and asks it for its
//! lines, how to read each, when each boot before this one ended, and where
//! the processes and the system's disk guard are to look at; nothing else in
//! health knows which system it runs on.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::io::{self, BufRead as _, BufReader, Write as _};
use std::mem;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, ExitStatus, Output, Stdio};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};
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
    /// is `now`, their lines sent on, each that follows for as long as
    /// health runs sending a `None` if it ends. The journal's day so far
    /// is sent in the order it was written, then each follower follows on
    /// (`replay.rs`).
    ///
    /// # Errors
    ///
    /// When a Mac's log can't start, or the day's start can't be told. A
    /// journal's history or follower that can't start is said, and sends
    /// every follower's end (`replayed_by`). Each follower is kept in
    /// `followers`, to end as they do.
    pub fn start(
        self,
        now: &Zoned,
        lines: &Sender<Option<String>>,
        followers: &Followers,
    ) -> io::Result<()> {
        let children = &followers.children;
        let asked = match self {
            Self::Journal => return replayed(now, lines, children),
            Self::UnifiedLog => {
                let midnight = format!("{} 00:00:00", now.date());
                let now = now.strftime("%Y-%m-%d %H:%M:%S").to_string();
                unified::commands(&midnight, &now)
            }
        };
        for command in asked {
            let name = named(&command);
            spawn(command, lines.clone(), children)
                .map_err(|error| io::Error::new(error.kind(), format!("{name}: {error}")))?;
        }
        Ok(())
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
/// its history ended, each sending a `None` if it ends.
///
/// Every history ends at the journal's last line as the replay begins,
/// where journald numbered it (`replayed_by`), or reads to its own end
/// where that can't be read (`last_of`). Lines written since are heard
/// once their followers follow on, each at its own pace, as every line
/// after the replay is: a switch writes its reload before the restarts
/// it asks for.
fn replayed(now: &Zoned, lines: &Sender<Option<String>>, children: &Children) -> io::Result<()> {
    let since = day_of(now)?;
    replayed_by(
        journal::newest(),
        move |asked| journal::reading(asked, since),
        move |asked, after| journal::following(asked, after, since),
        lines,
        children,
    );
    Ok(())
}

/// `replayed`, with the journalctls it asks made by `newest`, `reading`
/// from what each history asks, and `following` from that and where its
/// history ended, so a test can point them at a journal of its own. Says
/// how many followers it starts, each sending a `None` if it ends.
///
/// All of it runs on the replay's own thread, so health's tick starts
/// however long journalctl takes. `newest` answers before any history
/// starts, so each reads past the line it named, and none can end before
/// it, short of a line written meanwhile. One that can't start sends every
/// follower's end, so health ends once it has run `follow::SETTLED`, for
/// systemd to start it again.
fn replayed_by(
    newest: Command,
    reading: impl Fn(&[String]) -> Command + Send + 'static,
    following: impl Fn(&[String], Option<&str>) -> Command + Send + 'static,
    lines: &Sender<Option<String>>,
    children: &Children,
) -> usize {
    let count = journal::followed().len();
    let lines = lines.clone();
    let children = children.clone();
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
                    &children,
                );
            }
            Err(error) => fail(&format!(
                "health: the day's replay couldn't start: {error}, {}",
                starting_again()
            )),
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

/// The replay, whose followers each send a `None` if they end.
///
/// A follower that can't follow on after the replay, or a replay that
/// fails any other way, a panic included, sends every follower's end
/// instead, so health ends as when they all have rather than counting
/// the day without their lines; an end sent twice says nothing more.
fn replaying<A>(
    histories: Vec<(History, A)>,
    last: Option<&replay::Numbered>,
    follow_on: impl Fn(&A, Option<&str>) -> Command,
    lines: &Sender<Option<String>>,
    children: &Children,
) {
    let mut ends = Ends {
        lines: lines.clone(),
        left: histories.len(),
    };
    match replay(histories, last, follow_on, lines, children) {
        Ok(()) => ends.left = 0,
        Err(error) => {
            fail(&format!(
                "health: a follower couldn't follow on from the day's replay: {error}, {}",
                starting_again()
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
    children: &Children,
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
        spawn(command, lines.clone(), children)
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

/// The followers health started, ended as `follow` returns: each still
/// running is killed and waited on once this is dropped, as a `History`
/// is, and none starts after. Once one follower has ended, health ends,
/// and the others, following a journal no line comes to, would otherwise
/// wait for their next line, outliving it where no service manager ends
/// them with it, as when `health follow` is run by hand. A signal that
/// ends health unwinds nothing, so a signal that asks health to stop is
/// caught and ends them first (`ender`, `signals.rs`).
#[derive(Default)]
pub struct Followers {
    children: Children,
}

impl Followers {
    /// What ends every follower still running, and starts none after, from
    /// another thread, as the one waiting for a signal that ends health.
    pub fn ender(&self) -> impl FnOnce() + Send + 'static {
        let children = self.children.clone();
        move || children.end()
    }

    /// A follower started on `command`, its output unread, kept until the
    /// followers are ended, for a test: its process's id.
    #[cfg(test)]
    pub(crate) fn following(&self, mut command: Command) -> u32 {
        let started = self
            .children
            .start(&mut command)
            .expect("the follower starts");
        started.expect("the followers not ended").0
    }
}

impl Drop for Followers {
    fn drop(&mut self) {
        self.children.end();
    }
}

/// The children of `Followers`, shared with each follower's thread.
#[derive(Clone, Default)]
struct Children(Arc<Mutex<Running>>);

/// What `Children` holds: each follower still running, by its process's
/// id, and whether they've been ended.
#[derive(Default)]
struct Running {
    children: BTreeMap<u32, Child>,
    ended: bool,
}

impl Children {
    fn running(&self) -> MutexGuard<'_, Running> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Start `command`, its output piped, kept until its thread takes it
    /// back: its process's id and output, or none once they've been
    /// ended, when it isn't started. It starts under the lock, so health's
    /// end can't come between its start and its keeping, where health's
    /// exit would leave it unkept and running.
    fn start(&self, command: &mut Command) -> io::Result<Option<(u32, ChildStdout)>> {
        let mut running = self.running();
        if running.ended {
            return Ok(None);
        }
        let mut child = command.stdout(Stdio::piped()).spawn()?;
        let out = child.stdout.take();
        let id = child.id();
        // Kept even without its output, so health's end still ends it.
        running.children.insert(id, child);
        out.map(|out| Some((id, out)))
            .ok_or_else(|| io::Error::other("no output"))
    }

    /// The child whose process is `id`, for its thread to wait on once
    /// its output ends, or none once they've been ended.
    fn take(&self, id: u32) -> Option<Child> {
        self.running().children.remove(&id)
    }

    /// Kill and wait on each child still running, and start none after.
    fn end(&self) {
        let children = {
            let mut running = self.running();
            running.ended = true;
            mem::take(&mut running.children)
        };
        for mut child in children.into_values() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Start `command`, its output's lines sent on, and a `None` when it ends
/// if it `follows`, or if it failed, so health ends to start it again.
/// It's kept in `children` until then.
///
/// Health says so when one ends, unless it ends whole by design and did
/// (`follower_ended`).
fn spawn(command: Command, lines: Sender<Option<String>>, children: &Children) -> io::Result<()> {
    let following = follows(&command);
    spawn_saying(command, lines, following, warn, children)
}

/// `spawn`, with what it says of its end said through `say`, before its
/// `None`, sent if it `follows` for as long as health runs, or if it
/// didn't end whole, so health starts again and reads it again. One that
/// `children` ended, as health ends, says nothing and sends nothing, and
/// one asked once they're ended isn't started.
fn spawn_saying(
    mut command: Command,
    lines: Sender<Option<String>>,
    follows: bool,
    say: impl Fn(&str) + Send + 'static,
    children: &Children,
) -> io::Result<()> {
    let name = named(&command);
    let Some((id, out)) = children.start(&mut command)? else {
        return Ok(());
    };
    let children = children.clone();
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
        let Some(mut child) = children.take(id) else {
            return;
        };
        let waited = child.wait();
        // One the stop that ends health ended too says nothing either.
        if crate::signals::stopping() {
            return;
        }
        let whole = matches!(&waited, Ok(status) if status.success());
        if let Some(said) = follower_ended(&name, waited, follows) {
            say(&said);
        }
        if follows || !whole {
            let _ = lines.send(None);
        }
    });
    Ok(())
}

/// Whether `command` follows for as long as health runs: every follower,
/// a journal's or a Mac's `log stream`, but a Mac's `log show`, which
/// reads the day and ends whole by design on every start.
fn follows(command: &Command) -> bool {
    command.get_program() != OsStr::new(unified::LOG)
        || command.get_args().next() != Some(OsStr::new("show"))
}

/// What health says of the follower named `name` that ended as `waited`
/// says, or nothing of one that doesn't follow for as long as health runs,
/// such as a Mac's `log show`, when it ended whole.
///
/// The end of one that `follows`, or the failure of one that doesn't,
/// ends health, for systemd or launchd to start it again with every
/// follower, no sooner than `follow::SETTLED` after its start
/// (`follow::run`), so it's said with when (`starting_again`), a whole
/// end too: journalctl ends whole on a SIGTERM or SIGINT (systemd 262, 18
/// November).
fn follower_ended(name: &str, waited: io::Result<ExitStatus>, follows: bool) -> Option<String> {
    let again = starting_again();
    match waited {
        Ok(status) if !follows && status.success() => None,
        Ok(status) if !follows => Some(format!("health: {name} ended, {status}, {again}")),
        Err(error) if !follows => Some(format!(
            "health: {name} can't be waited on: {error}, {again}"
        )),
        Ok(status) => Some(format!(
            "health: the follower asking {name} ended, {status}, {again}"
        )),
        Err(error) => Some(format!(
            "health: the follower asking {name} can't be waited on: {error}, {again}"
        )),
    }
}

/// What follows a follower's end, or a failure health ends on, said so
/// it isn't taken for health being stuck or well.
pub(crate) fn starting_again() -> String {
    format!(
        "so health starts again with every follower, no sooner than {} minutes after it \
         started, and reads the day again",
        follow::SETTLED.as_secs() / 60
    )
}

/// Say `words` on stderr as a warning (`as_marked`).
fn warn(words: &str) {
    say_marked(words, WARNING);
}

/// Say `words` on stderr as a failure (`as_marked`): what health ends
/// on, its own error line saying only that a follower ended or couldn't
/// start, and a day it can't write, which it runs on through for a time.
pub(crate) fn fail(words: &str) {
    say_marked(words, ERROR);
}

/// Say `words` on stderr as a notice (`as_marked`), which health's
/// warnings follower doesn't count: what's well again, such as a day
/// written once more after a spell it couldn't be.
pub(crate) fn notice(words: &str) {
    say_marked(words, NOTICE);
}

/// sd-daemon(3)'s mark for a notice.
pub(crate) const NOTICE: u8 = 5;

// A notice is below a warning, or health's warnings follower, which reads
// `-p warning`, would count each as one of health's own.
const _: () = assert!(
    NOTICE > WARNING,
    "a notice isn't below a warning, so health's warnings follower would count it"
);

/// sd-daemon(3)'s mark for a warning.
const WARNING: u8 = 4;

/// sd-daemon(3)'s mark for an error.
pub(crate) const ERROR: u8 = 3;

fn say_marked(words: &str, mark: u8) {
    static JOURNAL: OnceLock<bool> = OnceLock::new();
    // Kept for the tests to read which mark each saying went at, since
    // a warning said as a failure, or a failure as a warning, passes
    // otherwise.
    #[cfg(test)]
    tests::hear(words, mark);
    let journal = *JOURNAL.get_or_init(|| terminal_kit::Environment::here().journal);
    // A Mac's stderr is a log file, which a full disk can't take, and
    // `eprintln!` would panic on it, ending health every few seconds.
    let _ = writeln!(io::stderr().lock(), "{}", as_marked(words, journal, mark));
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

// The follower's tests read the marks it says at too.
#[cfg(test)]
pub(crate) mod tests;
