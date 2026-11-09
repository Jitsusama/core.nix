//! The to-do daemon: what clears each of Joel's to-dos by itself. Every
//! session that needs him writes its quest's to-dos to `mission/waiting`
//! through `mesh_wait`, each saying how it will be known done and ready: a
//! check to run, a signal the machine gives, or only his word. This runs
//! the checks and hears the signals, and writes what it found to
//! `mission/verdicts.json`, which it alone writes: the harness settles a
//! to-do found done and marks one found ready, and For You, through the
//! mycelium node, says each as it stands.
//!
//! Usage: `to-do WAITING VERDICTS SIGNINS CORE [--root DIR] [--follower
//! UNIT] [--git GIT] [--journalctl JOURNALCTL]`. WAITING is the folder of
//! to-dos, SIGNINS the folder whose files each service's sign-in touches,
//! CORE where core's bare clone is kept. ROOT, `/` by default, is where
//! `/proc/stat`, `/sys` and the system profile are read, so a test can
//! stand in for the machine; UNIT is update-to-main's follower, a user
//! unit, `none` to read no journal. GIT and JOURNALCTL are the daemon's
//! own, so the checks keep the account's PATH as a session has it.
//!
//! It wakes only for a change: a to-do file, a sign-in, the system
//! profile, a file a to-do waits on, a journal line, a check falling
//! due, or a check's run ending. A device has no event here, so while a to-do waits on one the
//! devices are read each ten seconds. A to-do found done stays done,
//! through a restart too, until it leaves its file, so a check that
//! passed once isn't run again to be taken back.

mod bounded;
mod judge;
mod pace;
mod sources;
mod todo;
mod verdicts;

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use inotify::{Inotify, WatchDescriptor, WatchMask, Watches};

use judge::Facts;
use pace::{Pace, Run};
use todo::{Knowing, Signal, ToDo};
use verdicts::{For, State, Verdict};

/// How often the devices are read while a to-do waits on one, in seconds.
const DEVICES_EVERY: i64 = 10;

/// What wakes the daemon: a change to judge again, which runs only a new
/// to-do's check, as a to-do file or a file waited on does; a signal, a
/// sign-in or a switch, which may change what every check finds, so each
/// runs again; or a journal line, which is a signal when it says a
/// switch. A file's folder can be as busy as the runtime folder, so its
/// changes never hurry a check. A check's run, as it ends, is judged at
/// once.
enum Woken {
    Changed,
    Signal,
    Journal(String),
    Ran {
        key: String,
        check: String,
        run: Run,
    },
}

struct Paths {
    waiting: PathBuf,
    verdicts: PathBuf,
    signins: PathBuf,
    root: PathBuf,
}

struct Daemon {
    paths: Paths,
    core: sources::Core,
    watches: Watches,
    /// The folders watched for every to-do, never let go.
    fixed: Vec<WatchDescriptor>,
    /// The folders watched toward each file a to-do waits on, let go once
    /// none does.
    folders: HashMap<PathBuf, WatchDescriptor>,
    facts: Facts,
    follower: sources::Follower,
    /// Each check's runs, by `<key>:done` or `<key>:ready`.
    paces: HashMap<String, Pace>,
    /// When each to-do first reached the state it's in.
    since: HashMap<String, (State, i64)>,
    /// The core the clone was last fetched for, so it's fetched once a
    /// switch.
    fetched: Option<String>,
    /// When fetching it next may be tried, after one failed.
    fetch_again: Option<i64>,
    /// What was last found of each to-do, a done one kept as it was.
    found: BTreeMap<String, Verdict>,
    written: Option<String>,
    /// Where each check's run is told as it ends.
    tell: Sender<Woken>,
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .unwrap_or_default()
}

fn signal_words(signal: &Signal) -> String {
    match signal {
        Signal::Boot => "boot".into(),
        Signal::Switch(None) => "switch".into(),
        Signal::Switch(Some(commit)) => format!("switch:{commit}"),
        Signal::Signin(service) => format!("signin:{service}"),
        Signal::Device(name) => format!("device:{name}"),
        Signal::File(path) => format!("file:{}", path.display()),
    }
}

impl Daemon {
    fn profile(&self) -> PathBuf {
        self.paths.root.join("nix/var/nix/profiles/system")
    }

    /// Reads again what the to-dos wait on, each only while one does.
    fn read_facts(&mut self, todos: &[ToDo]) {
        let signals: Vec<&Signal> = todos
            .iter()
            .flat_map(|t| [&t.done_when, &t.ready_when])
            .filter_map(|k| match k {
                Some(Knowing::Signal(signal)) => Some(signal),
                _ => None,
            })
            .collect();
        let stat = fs::read_to_string(self.paths.root.join("proc/stat")).unwrap_or_default();
        self.facts.boot = sources::boot(&stat);
        self.facts.profile = sources::changed(&self.profile());
        self.facts.switches.clone_from(&self.follower.switches);
        self.facts.core.clone_from(&self.follower.core);
        self.facts.signins = sources::signins(&self.paths.signins);
        self.facts.devices = if signals.iter().any(|s| matches!(s, Signal::Device(_))) {
            sources::devices(&self.paths.root.join("sys"))
        } else {
            Vec::new()
        };
        self.facts.files.clear();
        let mut wanted = Vec::new();
        for signal in &signals {
            if let Signal::File(path) = signal {
                wanted.extend(self.watch_toward(path));
                if let Some(at) = sources::changed(path) {
                    self.facts.files.insert(path.clone(), at);
                }
            }
        }
        let gone: Vec<PathBuf> = self
            .folders
            .keys()
            .filter(|folder| !wanted.contains(folder))
            .cloned()
            .collect();
        for folder in gone {
            if let Some(watch) = self.folders.remove(&folder) {
                let _ = self.watches.remove(watch);
            }
        }
        let commits: Vec<&str> = signals
            .iter()
            .filter_map(|s| match s {
                Signal::Switch(Some(commit)) => Some(commit.as_str()),
                _ => None,
            })
            .collect();
        let at = now();
        let may = self.fetch_again.is_none_or(|again| at >= again);
        if commits.is_empty() {
            self.fetch_again = None;
        } else if self.fetched != self.follower.core && may {
            let mut wanted = commits.clone();
            wanted.extend(self.follower.core.as_deref());
            match self.core.fetch_for(&wanted) {
                Ok(()) => {
                    self.fetched.clone_from(&self.follower.core);
                    self.fetch_again = None;
                }
                Err(error) => {
                    // Tried again only after the longest wait a check
                    // has, rather than on every wake.
                    self.fetch_again = Some(now() + pace::LONGEST);
                    eprintln!("<4>core's history couldn't be fetched: {error}");
                }
            }
        }
    }

    /// Watches the nearest folder toward `path` that's there, so its
    /// coming is heard however deep it is, answering which. The watch is
    /// added to whatever else watches that folder, and one of the folders
    /// every to-do needs is never let go.
    fn watch_toward(&mut self, path: &Path) -> Option<PathBuf> {
        let mask = WatchMask::CREATE
            | WatchMask::MOVED_TO
            | WatchMask::CLOSE_WRITE
            | WatchMask::DELETE
            | WatchMask::MOVED_FROM
            | WatchMask::ATTRIB
            | WatchMask::MASK_ADD;
        let mut at = path.parent();
        while let Some(folder) = at {
            if folder.is_dir() {
                if !self.folders.contains_key(folder)
                    && let Ok(watch) = self.watches.add(folder, mask)
                    && !self.fixed.contains(&watch)
                {
                    self.folders.insert(folder.to_owned(), watch);
                }
                return Some(folder.to_owned());
            }
            at = folder.parent();
        }
        None
    }

    /// Runs every check that's due at once, each on a thread of its own,
    /// so one slow check holds up no other. A to-do found done, or whose
    /// owner is in the strict sandbox, runs none.
    fn run_due(&mut self, todos: &[ToDo], at: i64) {
        let mut keys = Vec::new();
        for todo in todos.iter().filter(|t| t.judged() && !t.strict) {
            if self
                .found
                .get(&todo.key)
                .is_some_and(|v| v.state == State::Done)
            {
                continue;
            }
            let ready = todo.ready_when.as_ref().filter(|_| !todo.ready);
            for (knowing, of) in [(todo.done_when.as_ref(), For::Done), (ready, For::Ready)] {
                let Some(Knowing::Check(check)) = knowing else {
                    continue;
                };
                let key = format!("{}:{}", todo.key, of.name());
                self.paces
                    .entry(key.clone())
                    .and_modify(|p| {
                        if p.check != *check {
                            *p = Pace::new(check, at);
                        }
                    })
                    .or_insert_with(|| Pace::new(check, at));
                keys.push(key);
            }
        }
        for key in keys {
            let Some(pace) = self.paces.get_mut(&key).filter(|p| p.due(at)) else {
                continue;
            };
            pace.start();
            let check = pace.check.clone();
            let tell = self.tell.clone();
            thread::spawn(move || {
                let run = pace::run(&check);
                let _ = tell.send(Woken::Ran { key, check, run });
            });
        }
    }

    /// Takes in a check's run as it ends, unless its to-do has gone or its
    /// check was changed while it ran.
    fn took(&mut self, key: &str, check: &str, run: Run) {
        if let Some(pace) = self.paces.get_mut(key).filter(|p| p.check == check) {
            pace.ran(run, now());
        }
    }

    /// Judges `knowing` for `todo`, a check by its last run.
    fn judged(
        &mut self,
        todo: &ToDo,
        knowing: &Knowing,
        of: For,
        at: i64,
    ) -> Option<(bool, serde_json::Value)> {
        match knowing {
            Knowing::Person => None,
            Knowing::Check(check) if todo.strict => Some((false, verdicts::refused(of, check))),
            Knowing::Check(check) => {
                let key = format!("{}:{}", todo.key, of.name());
                let pace = self
                    .paces
                    .entry(key)
                    .or_insert_with(|| Pace::new(check, at));
                // Nothing is said of a check until its first run ends.
                let last = pace.last.as_ref()?;
                Some((last.passed(), verdicts::by_check(of, pace)))
            }
            Knowing::Signal(signal) => {
                let found = judge::heard(signal, todo.raised, &self.facts, &self.core);
                let how = verdicts::by_signal(of, &signal_words(signal), &found);
                Some((found.is_ok(), how))
            }
        }
    }

    /// Judges every to-do, and writes the verdicts if they changed.
    /// Answers when it next must wake, while a check or a device waits.
    fn tick(&mut self) -> Option<i64> {
        let todos = todo::in_folder(&self.paths.waiting);
        self.read_facts(&todos);
        let at = now();
        self.run_due(&todos, at);
        let mut found = BTreeMap::new();
        for todo in todos.iter().filter(|t| t.judged()) {
            if let Some(kept) = self.found.get(&todo.key).filter(|v| v.state == State::Done) {
                found.insert(todo.key.clone(), kept.clone());
                continue;
            }
            let done = match todo.done_when.clone() {
                Some(knowing) => self.judged(todo, &knowing, For::Done, at),
                None => None,
            };
            let verdict = if let Some((true, how)) = done {
                (State::Done, how)
            } else {
                let ready = match todo.ready_when.clone().filter(|_| !todo.ready) {
                    Some(knowing) => self.judged(todo, &knowing, For::Ready, at),
                    None => None,
                };
                match (ready, done) {
                    (Some((true, how)), _) => (State::Ready, how),
                    (Some((false, how)), _) | (None, Some((false, how))) => (State::Waiting, how),
                    _ => continue,
                }
            };
            let (state, how) = verdict;
            let since = match self.since.get(&todo.key) {
                Some((was, since)) if *was == state => *since,
                _ => at,
            };
            self.since.insert(todo.key.clone(), (state, since));
            found.insert(
                todo.key.clone(),
                Verdict {
                    state,
                    at: since,
                    how,
                },
            );
        }
        // A to-do gone from its file takes its runs and its verdict.
        // One whose check hasn't yet run once keeps its run going.
        self.since.retain(|key, _| found.contains_key(key));
        self.paces.retain(|key, _| {
            key.rsplit_once(':')
                .is_some_and(|(key, _)| todos.iter().any(|t| t.key == key))
        });
        let text = verdicts::file(&found);
        if self.written.as_deref() != Some(text.as_str()) {
            match verdicts::write_whole(&self.paths.verdicts, &text) {
                Ok(()) => self.written = Some(text),
                Err(error) => eprintln!("<3>the verdicts couldn't be written: {error}"),
            }
        }
        self.found = found;
        let found = &self.found;
        let checks = self
            .paces
            .iter()
            .filter(|(key, _)| {
                key.rsplit_once(':')
                    .is_none_or(|(todo, _)| found.get(todo).is_none_or(|v| v.state != State::Done))
            })
            .filter_map(|(_, p)| p.next());
        let devices = (!self.facts.devices.is_empty()
            || todos.iter().any(|t| {
                [&t.done_when, &t.ready_when]
                    .iter()
                    .any(|k| matches!(k, Some(Knowing::Signal(Signal::Device(_)))))
            }))
        .then_some(at + DEVICES_EVERY);
        checks.chain(devices).chain(self.fetch_again).min()
    }

    /// A signal may change what a check finds, so each runs again.
    fn hurry(&mut self) {
        let at = now();
        for pace in self.paces.values_mut() {
            pace.hurry(at);
        }
    }
}

/// Reads the follower's journal, a line at a time, into `woken`.
fn follow_journal(journalctl: PathBuf, unit: String, woken: Sender<Woken>) {
    thread::spawn(move || {
        // The follower is a user unit, so it's in the account's journal.
        let child = Command::new(journalctl)
            .args([
                "--user",
                "-u",
                &unit,
                "-o",
                "short-unix",
                "--no-pager",
                "-f",
                "-n",
                "all",
            ])
            .args(["--since", "30 days ago"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        let Ok(mut child) = child else {
            eprintln!("<4>the follower's journal couldn't be read");
            return;
        };
        if let Some(out) = child.stdout.take() {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                if woken.send(Woken::Journal(line)).is_err() {
                    break;
                }
            }
        }
        let _ = child.wait();
    });
}

/// Hears every change inotify says, into `woken`: one in `signals`, the
/// sign-ins' and the system profiles' folders, apart from every other.
fn hear(mut inotify: Inotify, signals: Vec<WatchDescriptor>, woken: Sender<Woken>) {
    thread::spawn(move || {
        let mut buffer = [0; 4096];
        while let Ok(events) = inotify.read_events_blocking(&mut buffer) {
            let mut signal = false;
            let mut changed = false;
            for event in events {
                if signals.contains(&event.wd) {
                    signal = true;
                } else {
                    changed = true;
                }
            }
            let told = [(changed, Woken::Changed), (signal, Woken::Signal)]
                .into_iter()
                .filter(|(said, _)| *said)
                .try_for_each(|(_, woke)| woken.send(woke));
            if told.is_err() {
                break;
            }
        }
    });
}

/// The programs the daemon runs itself, apart from the checks' PATH.
struct Tools {
    git: PathBuf,
    journalctl: PathBuf,
}

fn run(paths: Paths, core: PathBuf, follower: Option<String>, tools: Tools) -> io::Result<()> {
    fs::create_dir_all(&paths.waiting)?;
    fs::create_dir_all(&paths.signins)?;
    let inotify = Inotify::init()?;
    let mut watches = inotify.watches();
    let mask = WatchMask::CLOSE_WRITE
        | WatchMask::MOVED_TO
        | WatchMask::MOVED_FROM
        | WatchMask::DELETE
        | WatchMask::CREATE
        | WatchMask::ATTRIB;
    let waiting = watches.add(&paths.waiting, mask)?;
    let mut signals = vec![watches.add(&paths.signins, mask)?];
    let profiles = paths.root.join("nix/var/nix/profiles");
    if profiles.is_dir() {
        signals.push(watches.add(&profiles, WatchMask::CREATE | WatchMask::MOVED_TO)?);
    }
    let mut fixed = signals.clone();
    fixed.push(waiting);
    let (tell, woken): (Sender<Woken>, Receiver<Woken>) = mpsc::channel();
    hear(inotify, signals, tell.clone());
    if let Some(unit) = follower {
        follow_journal(tools.journalctl, unit, tell.clone());
    }
    // What was found done before a restart stays done.
    let found = fs::read_to_string(&paths.verdicts)
        .map(|text| verdicts::done_in(&text))
        .unwrap_or_default();
    let mut daemon = Daemon {
        paths,
        core: sources::Core {
            folder: core,
            git: tools.git,
        },
        watches,
        fixed,
        folders: HashMap::new(),
        facts: Facts::default(),
        follower: sources::Follower::default(),
        paces: HashMap::new(),
        since: HashMap::new(),
        fetched: None,
        fetch_again: None,
        found,
        written: None,
        tell,
    };
    let mut next = daemon.tick();
    loop {
        let wait = next.map(|at| Duration::from_secs(u64::try_from(at - now()).unwrap_or(0)));
        let first = match wait {
            Some(wait) => woken.recv_timeout(wait),
            None => woken.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        let first = match first {
            Ok(woke) => Some(woke),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => {
                return Err(io::Error::other("nothing is heard any more"));
            }
        };
        // Everything already said is taken in before judging once.
        let mut signalled = false;
        for woke in first.into_iter().chain(woken.try_iter()) {
            signalled |= match woke {
                Woken::Journal(line) => daemon.follower.read(&line),
                Woken::Signal => true,
                Woken::Changed => false,
                Woken::Ran { key, check, run } => {
                    daemon.took(&key, &check, run);
                    false
                }
            };
        }
        if signalled {
            daemon.hurry();
        }
        next = daemon.tick();
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut positional = Vec::new();
    let mut root = PathBuf::from("/");
    let mut follower = Some("update-to-main-follows".to_owned());
    let mut tools = Tools {
        git: PathBuf::from("git"),
        journalctl: PathBuf::from("journalctl"),
    };
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--root" => root = rest.next().map_or(root, PathBuf::from),
            "--git" => tools.git = rest.next().map_or(tools.git, PathBuf::from),
            "--journalctl" => {
                tools.journalctl = rest.next().map_or(tools.journalctl, PathBuf::from);
            }
            "--follower" => {
                follower = rest.next().filter(|u| *u != "none").cloned();
            }
            _ => positional.push(PathBuf::from(arg)),
        }
    }
    let [waiting, verdicts, signins, core] = positional.as_slice() else {
        eprintln!(
            "usage: to-do WAITING VERDICTS SIGNINS CORE [--root DIR] [--follower UNIT|none] \
             [--git GIT] [--journalctl JOURNALCTL]"
        );
        return ExitCode::from(64);
    };
    let paths = Paths {
        waiting: waiting.clone(),
        verdicts: verdicts.clone(),
        signins: signins.clone(),
        root,
    };
    match run(paths, core.clone(), follower, tools) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("<3>{error}");
            ExitCode::FAILURE
        }
    }
}
