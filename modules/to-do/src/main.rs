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
//! UNIT]`. WAITING is the folder of to-dos, SIGNINS the folder whose files
//! each service's sign-in touches, CORE where core's bare clone is kept.
//! ROOT, `/` by default, is where `/proc/stat`, `/sys` and the system
//! profile are read, so a test can stand in for the machine; UNIT is
//! update-to-main's follower, `none` to read no journal.
//!
//! It wakes only for a change: a to-do file, a sign-in, the system
//! profile, a file a to-do waits on, a journal line, or a check falling
//! due. A device has no event here, so while a to-do waits on one the
//! devices are read each ten seconds.

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
use pace::Pace;
use todo::{Knowing, Signal, ToDo};
use verdicts::{For, State, Verdict};

/// How often the devices are read while a to-do waits on one, in seconds.
const DEVICES_EVERY: i64 = 10;

/// What wakes the daemon: a change to judge again, which runs only a new
/// to-do's check, as a to-do file or a file waited on does; a signal, a
/// sign-in or a switch, which may change what every check finds, so each
/// runs again; or a journal line, which is a signal when it says a
/// switch. A file's folder can be as busy as the runtime folder, so its
/// changes never hurry a check.
enum Woken {
    Changed,
    Signal,
    Journal(String),
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
    facts: Facts,
    follower: sources::Follower,
    /// Each check's runs, by `<key>:done` or `<key>:ready`.
    paces: HashMap<String, Pace>,
    /// When each to-do first reached the state it's in.
    since: HashMap<String, (State, i64)>,
    /// The core the clone was last fetched for, so it's fetched once a
    /// switch.
    fetched: Option<String>,
    written: Option<String>,
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
        for signal in &signals {
            if let Signal::File(path) = signal {
                self.watch_toward(path);
                if let Some(at) = sources::changed(path) {
                    self.facts.files.insert(path.clone(), at);
                }
            }
        }
        let commits: Vec<&str> = signals
            .iter()
            .filter_map(|s| match s {
                Signal::Switch(Some(commit)) => Some(commit.as_str()),
                _ => None,
            })
            .collect();
        if !commits.is_empty() && self.fetched != self.follower.core {
            let mut wanted = commits.clone();
            wanted.extend(self.follower.core.as_deref());
            match self.core.fetch_for(&wanted) {
                Ok(()) => self.fetched.clone_from(&self.follower.core),
                Err(error) => eprintln!("<4>core's history couldn't be fetched: {error}"),
            }
        }
    }

    /// Watches the nearest folder toward `path` that's there, so its
    /// coming is heard however deep it is.
    fn watch_toward(&mut self, path: &Path) {
        let mask = WatchMask::CREATE
            | WatchMask::MOVED_TO
            | WatchMask::CLOSE_WRITE
            | WatchMask::DELETE
            | WatchMask::MOVED_FROM
            | WatchMask::ATTRIB;
        let mut at = path.parent();
        while let Some(folder) = at {
            if folder.is_dir() {
                let _ = self.watches.add(folder, mask);
                return;
            }
            at = folder.parent();
        }
    }

    /// Judges `knowing` for `todo`, running its check if it's due.
    fn judged(
        &mut self,
        todo: &ToDo,
        knowing: &Knowing,
        of: For,
        at: i64,
    ) -> Option<(bool, serde_json::Value)> {
        match knowing {
            Knowing::Person => None,
            Knowing::Check(check) => {
                let key = format!("{}:{}", todo.key, of.name());
                let pace = self
                    .paces
                    .entry(key)
                    .and_modify(|p| {
                        if p.check != *check {
                            *p = Pace::new(check, at);
                        }
                    })
                    .or_insert_with(|| Pace::new(check, at));
                if pace.due(at) {
                    let run = pace::run(check);
                    pace.ran(run, now());
                }
                let passed = pace.last.as_ref().is_some_and(pace::Run::passed);
                Some((passed, verdicts::by_check(of, pace)))
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
        let mut found = BTreeMap::new();
        for todo in todos.iter().filter(|t| t.judged()) {
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
        self.since.retain(|key, _| found.contains_key(key));
        self.paces.retain(|key, _| {
            key.rsplit_once(':')
                .is_some_and(|(todo, _)| found.contains_key(todo))
        });
        let text = verdicts::file(&found);
        if self.written.as_deref() != Some(text.as_str()) {
            match verdicts::write_whole(&self.paths.verdicts, &text) {
                Ok(()) => self.written = Some(text),
                Err(error) => eprintln!("<3>the verdicts couldn't be written: {error}"),
            }
        }
        let checks = self
            .paces
            .iter()
            .filter(|(key, _)| {
                key.rsplit_once(':')
                    .and_then(|(todo, _)| found.get(todo))
                    .is_some_and(|v| v.state != State::Done)
            })
            .map(|(_, p)| p.next_at);
        let devices = (!self.facts.devices.is_empty()
            || todos.iter().any(|t| {
                [&t.done_when, &t.ready_when]
                    .iter()
                    .any(|k| matches!(k, Some(Knowing::Signal(Signal::Device(_)))))
            }))
        .then_some(at + DEVICES_EVERY);
        checks.chain(devices).min()
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
fn follow_journal(unit: String, woken: Sender<Woken>) {
    thread::spawn(move || {
        let child = Command::new("journalctl")
            .args([
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

fn run(paths: Paths, core: PathBuf, follower: Option<String>) -> io::Result<()> {
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
    watches.add(&paths.waiting, mask)?;
    let mut signals = vec![watches.add(&paths.signins, mask)?];
    let profiles = paths.root.join("nix/var/nix/profiles");
    if profiles.is_dir() {
        signals.push(watches.add(&profiles, WatchMask::CREATE | WatchMask::MOVED_TO)?);
    }
    let (tell, woken): (Sender<Woken>, Receiver<Woken>) = mpsc::channel();
    hear(inotify, signals, tell.clone());
    if let Some(unit) = follower {
        follow_journal(unit, tell);
    } else {
        drop(tell);
    }
    let mut daemon = Daemon {
        paths,
        core: sources::Core { folder: core },
        watches,
        facts: Facts::default(),
        follower: sources::Follower::default(),
        paces: HashMap::new(),
        since: HashMap::new(),
        fetched: None,
        written: None,
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
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--root" => root = rest.next().map_or(root, PathBuf::from),
            "--follower" => {
                follower = rest.next().filter(|u| *u != "none").cloned();
            }
            _ => positional.push(PathBuf::from(arg)),
        }
    }
    let [waiting, verdicts, signins, core] = positional.as_slice() else {
        eprintln!("usage: to-do WAITING VERDICTS SIGNINS CORE [--root DIR] [--follower UNIT|none]");
        return ExitCode::from(64);
    };
    let paths = Paths {
        waiting: waiting.clone(),
        verdicts: verdicts.clone(),
        signins: signins.clone(),
        root,
    };
    match run(paths, core.clone(), follower) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("<3>{error}");
            ExitCode::FAILURE
        }
    }
}
