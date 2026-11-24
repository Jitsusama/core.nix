//! The daemon: one per account, held by a lock on its state directory. It
//! reads pi's logs as they grow, told by inotify, and the harness's runs
//! and quests every few minutes; it runs each feed, hands its records to
//! the history and restarts it when it stops; and after anything new, at
//! most every thirty seconds and at least every five minutes, it works
//! out how things stand, settles its findings, writes the shell's files
//! and gives notices. Between, a heartbeat every thirty seconds says it
//! still runs. A socket in the runtime directory answers `refresh`, and
//! tells the shell the collection as it changes (`interface`), taking its
//! acts on it (`acts`).

use crate::acts::{self, Act, Doer};
use crate::config::{Config, FeedConfig};
use crate::feed::{self, Pace};
use crate::findings::{Finding, Severity};
use crate::history::History;
use crate::ingest::Walks;
use crate::interface::Hub;
use crate::notices::{self, Buses, Desk};
use crate::picture::Picture;
use crate::status::{self, Writer};
use crate::time::{self, Zone};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// What wakes the loop.
enum Wake {
    Line(String, String),
    Ended(String),
    Moved,
    /// The folders the watch heard written in, `/` when it may have missed
    /// some.
    Touched(Vec<PathBuf>),
    Asked(String, Sender<String>),
    Tick,
}

/// The fewest seconds between passes, while things keep changing.
const BUSY: Duration = Duration::from_secs(30);
/// The most, while nothing does: the clocks and the day move regardless.
const IDLE: Duration = Duration::from_secs(300);
const HEARTBEAT: Duration = Duration::from_secs(30);
/// Activity in pi's logs within this long counts as working.
const ACTIVE: f64 = 600.0;
const HARNESS: Duration = Duration::from_secs(300);
/// How long a refresh waits for each feed to say something back.
const REFRESH: Duration = Duration::from_secs(10);
/// How long the heartbeat goes on without a pass that worked: past this,
/// the shell is let see a watch that isn't working as one that stopped.
const FAILING: Duration = Duration::from_mins(2 * 5 + 1);

struct Feed {
    config: FeedConfig,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    /// When it may start again, after stopping.
    next: Instant,
    /// The last wait before starting it again, if it has stopped before.
    backoff: Option<Duration>,
    started: Instant,
}

impl Feed {
    fn start(&mut self, wake: &Sender<Wake>, pace: Pace, kept: Option<String>) {
        // An attempt that can't even spawn counts as a quick stop.
        self.started = Instant::now();
        let mut command = Command::new(&self.config.command[0]);
        command
            .args(&self.config.command[1..])
            .envs(&self.config.environment)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        match command.spawn() {
            Ok(mut child) => {
                let stdout = child.stdout.take();
                self.stdin = child.stdin.take();
                // A feed that never reads mustn't stop the daemon at a full
                // pipe: a line it can't take now is dropped.
                if let Some(stdin) = &self.stdin {
                    let _ = rustix::fs::fcntl_setfl(
                        stdin,
                        rustix::fs::fcntl_getfl(stdin).unwrap_or(rustix::fs::OFlags::empty())
                            | rustix::fs::OFlags::NONBLOCK,
                    );
                }
                self.child = Some(child);
                let name = self.config.name.clone();
                let wake = wake.clone();
                if let Some(stdout) = stdout {
                    std::thread::spawn(move || {
                        for line in BufReader::new(stdout).lines() {
                            let Ok(line) = line else { break };
                            if wake.send(Wake::Line(name.clone(), line)).is_err() {
                                return;
                            }
                        }
                        let _ = wake.send(Wake::Ended(name));
                    });
                }
                self.say(&feed::pace_line(pace));
                if let Some(kept) = kept {
                    self.say(&kept);
                }
            }
            Err(error) => {
                eprintln!(
                    "feed {}: can't start {}: {error}",
                    self.config.name, self.config.command[0]
                );
                self.stopped();
            }
        }
    }

    fn say(&mut self, line: &str) {
        let Some(stdin) = &mut self.stdin else {
            return;
        };
        match stdin.write_all(format!("{line}\n").as_bytes()) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                eprintln!(
                    "feed {}: isn't reading; a line was dropped",
                    self.config.name
                );
            }
            Err(_) => self.stdin = None,
        }
    }

    /// Reaped, and due to start again after its backoff.
    fn stopped(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.stdin = None;
        let wait = backoff(self.backoff, self.started.elapsed());
        self.backoff = Some(wait);
        self.next = Instant::now() + wait;
    }
}

/// How long to wait before starting a feed again: five seconds after one
/// that ran a minute or more, else twice the last wait, to five minutes.
fn backoff(last: Option<Duration>, ran: Duration) -> Duration {
    match last {
        Some(last) if ran < Duration::from_secs(60) => (last * 2).min(Duration::from_secs(300)),
        _ => Duration::from_secs(5),
    }
}

/// Held by whatever writes the history from the sources, so two never
/// read the same log at once.
pub fn lock(state: &Path) -> Result<std::fs::File, String> {
    std::fs::create_dir_all(state)
        .map_err(|error| format!("making {}: {error}", state.display()))?;
    let path = state.join("lock");
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .map_err(|error| format!("opening {}: {error}", path.display()))?;
    rustix::fs::flock(&file, rustix::fs::FlockOperation::NonBlockingLockExclusive)
        .map_err(|_| format!("another spend-watch holds {}", path.display()))?;
    Ok(file)
}

/// The watch's inotify instance and the folder each of its watches
/// follows, shared so a pass watches the folders it found as it finds
/// them, not when the watch next hears something.
#[cfg(target_os = "linux")]
struct Watches {
    fd: std::os::fd::OwnedFd,
    watched: Mutex<std::collections::HashMap<i32, PathBuf>>,
    /// Whether the watch still reads what it's told, so a watch fallen
    /// back to polling takes no more folders nothing will hear.
    heard: std::sync::atomic::AtomicBool,
    /// Whether a watch the kernel refused has been said, so the journal
    /// hears it once rather than each pass.
    refusal_said: std::sync::atomic::AtomicBool,
}

#[cfg(target_os = "linux")]
impl Watches {
    fn new(fd: std::os::fd::OwnedFd) -> Watches {
        Watches {
            fd,
            watched: Mutex::default(),
            heard: std::sync::atomic::AtomicBool::new(true),
            refusal_said: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// What each watch listens for.
    fn flags() -> rustix::fs::inotify::WatchFlags {
        use rustix::fs::inotify::WatchFlags;
        WatchFlags::MODIFY | WatchFlags::CREATE | WatchFlags::MOVED_TO | WatchFlags::CLOSE_WRITE
    }

    /// Watches each of `paths` not yet watched, handing back those the
    /// kernel refused, which no write there will wake. A folder counts as
    /// watched only once inotify has it. The likeliest refusal is the
    /// account's watches spent, which other programs share, so it's said
    /// once; a folder gone since the pass found it isn't a refusal worth
    /// saying.
    fn follow(&self, paths: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
        use std::sync::atomic::Ordering;
        // Asked under the lock, so a pass racing `give_up` either adds
        // its watches before they're handed back or adds none.
        let mut watched = self.held();
        if !self.heard.load(Ordering::Relaxed) {
            return Vec::new();
        }
        let known: std::collections::HashSet<PathBuf> = watched.values().cloned().collect();
        let mut refused = Vec::new();
        for path in paths.into_iter().filter(|path| !known.contains(path)) {
            match rustix::fs::inotify::add_watch(&self.fd, &path, Watches::flags()) {
                Ok(watch) => {
                    watched.insert(watch, path);
                }
                Err(error) => {
                    if error != rustix::io::Errno::NOENT
                        && !self.refusal_said.swap(true, Ordering::Relaxed)
                    {
                        eprintln!(
                            "the watch can't follow {}: {error}; folders like it are walked \
                             each pass instead",
                            path.display()
                        );
                    }
                    refused.push(path);
                }
            }
        }
        refused
    }

    /// Watches every folder followed afresh, after an overflow may have
    /// dropped the word that one went. The kernel hands a folder still
    /// watched its own number back, so nothing heard meanwhile is lost,
    /// and one made again by its name gets a new number; one gone is let
    /// go. The lock is held throughout, so no event is looked up halfway.
    fn renew(&self) {
        let mut watched = self.held();
        let paths: Vec<PathBuf> = watched.drain().map(|(_, path)| path).collect();
        for path in paths {
            if let Ok(watch) = rustix::fs::inotify::add_watch(&self.fd, &path, Watches::flags()) {
                watched.insert(watch, path);
            }
        }
    }

    /// Stops taking folders, as the watch falls back to polling, and
    /// hands the kernel back the watches it held, which the account's
    /// other programs draw on too.
    fn give_up(&self) {
        self.heard
            .store(false, std::sync::atomic::Ordering::Relaxed);
        for (watch, _) in self.held().drain() {
            // A watch the kernel already dropped, as its folder went, is
            // refused here, and there's nothing left to hand back.
            let _ = rustix::fs::inotify::remove_watch(&self.fd, watch);
        }
    }

    /// Whether `path` is watched.
    fn follows(&self, path: &Path) -> bool {
        self.held().values().any(|watched| watched == path)
    }

    /// The watches by number, whole though a thread panicked holding them.
    fn held(&self) -> std::sync::MutexGuard<'_, std::collections::HashMap<i32, PathBuf>> {
        self.watched
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// Where inotify can't be had there is nothing to share.
#[cfg(not(target_os = "linux"))]
struct Watches;

#[cfg(not(target_os = "linux"))]
impl Watches {
    fn follow(&self, _paths: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
        Vec::new()
    }
}

/// Tells the loop whenever pi's logs change, and in which folders:
/// inotify on the sessions directory and each directory in it, adding one
/// as it appears, and on the folders each pass follows through the
/// `Watches` handed back, where an agent's log or another machine's is.
#[cfg(target_os = "linux")]
fn watch(config: Config, wake: Sender<Wake>) -> Option<Arc<Watches>> {
    use rustix::fs::inotify::{self, CreateFlags};
    // Each machine's logs are watched as this machine's are, a new project
    // folder at once, so its first log is read as it lands rather than on
    // the next quiet pass. Its folder is made here so there is one to
    // watch before anything is brought, and this machine's too, as before
    // pi's first run, rather than polling until the daemon is restarted.
    // One that can't be made is said with why, and the daemon polls.
    if let Err(error) = private_folder(&config.sessions) {
        eprintln!(
            "pi's sessions folder {} can't be made: {error}",
            config.sessions.display()
        );
        std::thread::spawn(move || poll(&config, &wake));
        return None;
    }
    let roots: Vec<PathBuf> = std::iter::once(config.sessions.clone())
        .chain(config.machines.iter().map(|machine| {
            // Unmade, it's found by the passes as it appears, a little
            // later than a watch would.
            let _ = private_folder(&machine.sessions);
            machine.sessions.clone()
        }))
        .collect();
    let watches = match inotify::init(CreateFlags::CLOEXEC) {
        Ok(fd) => Some(Watches::new(fd)),
        Err(error) => {
            eprintln!("inotify can't be had: {error}");
            None
        }
    };
    if let Some(watches) = &watches {
        watches.follow(projects(&roots));
    }
    // A sessions folder that couldn't be watched: polling notices its
    // logs, and the passes are handed no watch to feed.
    let Some(watches) = watches.filter(|watches| watches.follows(&config.sessions)) else {
        std::thread::spawn(move || poll(&config, &wake));
        return None;
    };
    let watches = Arc::new(watches);
    let shared = Arc::clone(&watches);
    std::thread::spawn(move || {
        let mut buffer = [std::mem::MaybeUninit::<u8>::uninit(); 8192];
        let mut events = inotify::Reader::new(&shared.fd, &mut buffer);
        loop {
            let Some(touched) = written_in(&mut events, &shared) else {
                shared.give_up();
                return poll(&config, &wake);
            };
            shared.follow(projects(&roots));
            if wake.send(Wake::Touched(touched)).is_err() {
                return;
            }
        }
    });
    Some(watches)
}

/// Makes `path` and each folder missing above it readable by this account
/// alone, as pi's own folder is kept, since a log can hold a token; one
/// already there is left as it is.
#[cfg(target_os = "linux")]
fn private_folder(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
}

/// The roots and each project folder in them, looked at again after each
/// batch heard, so a new project's first log is read at once.
#[cfg(target_os = "linux")]
fn projects(roots: &[PathBuf]) -> Vec<PathBuf> {
    roots
        .iter()
        .flat_map(|root| {
            std::iter::once(root.clone()).chain(
                std::fs::read_dir(root)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .map(|entry| entry.path())
                    .filter(|path| path.is_dir()),
            )
        })
        .collect()
}

/// What the watch has heard since it last asked: each watched folder
/// written in, `/` where it may have missed some. A watch the kernel
/// dropped, as its folder went, is forgotten, so the folder made again by
/// that name is watched afresh once a pass finds it, and an overflow
/// renews them all. None once it can't read.
#[cfg(target_os = "linux")]
fn written_in<Fd: std::os::fd::AsFd>(
    events: &mut rustix::fs::inotify::Reader<'_, Fd>,
    watches: &Watches,
) -> Option<Vec<PathBuf>> {
    use rustix::fs::inotify::ReadFlags;
    let mut touched = Vec::new();
    loop {
        match events.next() {
            Ok(event) if event.events().contains(ReadFlags::QUEUE_OVERFLOW) => {
                eprintln!("the watch missed what was written; walking every folder again");
                watches.renew();
                touched.push(PathBuf::from("/"));
            }
            Ok(event) if event.events().contains(ReadFlags::IGNORED) => {
                touched.extend(watches.held().remove(&event.wd()));
            }
            Ok(event) => touched.extend(watches.held().get(&event.wd()).cloned()),
            Err(rustix::io::Errno::INTR) => continue,
            Err(_) => return None,
        }
        if events.is_buffer_empty() {
            break;
        }
    }
    touched.sort();
    touched.dedup();
    Some(touched)
}

#[cfg(not(target_os = "linux"))]
fn watch(config: Config, wake: Sender<Wake>) -> Option<Arc<Watches>> {
    std::thread::spawn(move || poll(&config, &wake));
    None
}

/// Where inotify can't be had: the logs' newest change, every five
/// seconds. Each look is a whole walk, as every pass was before settled
/// folders, so it's said once in the journal.
fn poll(config: &Config, wake: &Sender<Wake>) {
    eprintln!("no watch on pi's logs; walking every folder each five seconds instead");
    let newest = || {
        crate::ingest::found(config)
            .iter()
            .filter_map(|found| std::fs::metadata(&found.path).ok())
            .map(|meta| (meta.len(), meta.modified().ok()))
            .fold((0u64, None), |(size, at), (len, modified)| {
                (size + len, at.max(modified))
            })
    };
    let mut last = newest();
    loop {
        std::thread::sleep(Duration::from_secs(5));
        let now = newest();
        if now != last {
            last = now;
            if wake.send(Wake::Moved).is_err() {
                return;
            }
        }
    }
}

fn listen(socket: &Path, wake: Sender<Wake>, hub: Arc<Hub>) -> Result<(), String> {
    let _ = std::fs::remove_file(socket);
    if let Some(dir) = socket.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|error| format!("making {}: {error}", dir.display()))?;
    }
    let listener = UnixListener::bind(socket)
        .map_err(|error| format!("listening on {}: {error}", socket.display()))?;
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let wake = wake.clone();
            let hub = Arc::clone(&hub);
            std::thread::spawn(move || answer(&stream, &wake, &hub));
        }
    });
    Ok(())
}

/// Answers one word, as the command line asks, or serves the daemon
/// interface to a connection whose first line is one of its asks, kept
/// open for as long as it is.
fn answer(stream: &UnixStream, wake: &Sender<Wake>, hub: &Hub) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut reader = BufReader::new(stream);
    let mut asked = String::new();
    if reader.read_line(&mut asked).is_err() {
        return;
    }
    if asked.trim_start().starts_with('{') {
        let _ = stream.set_read_timeout(None);
        if let Ok(writing) = stream.try_clone() {
            hub.serve(asked.trim(), reader, writing);
        }
        return;
    }
    let (reply, replied) = mpsc::channel();
    if wake
        .send(Wake::Asked(asked.trim().to_owned(), reply))
        .is_err()
    {
        return;
    }
    if let Ok(said) = replied.recv_timeout(Duration::from_secs(120)) {
        let mut stream = stream;
        let _ = writeln!(stream, "{said}");
    }
}

/// Asks a running daemon, for the command line.
pub fn ask(socket: &Path, what: &str) -> Result<String, String> {
    let mut stream =
        UnixStream::connect(socket).map_err(|_| "spend-watch isn't running".to_owned())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(130)))
        .map_err(|error| error.to_string())?;
    writeln!(stream, "{what}").map_err(|error| error.to_string())?;
    let mut said = String::new();
    BufReader::new(&stream)
        .read_line(&mut said)
        .map_err(|error| error.to_string())?;
    Ok(said.trim().to_owned())
}

/// Reads the harness's runs and quests into the history.
pub fn harness(history: &History, config: &Config) {
    match crate::runs::read(&config.runs) {
        Ok(runs) => {
            // Replaced in one transaction, so a report never sees none.
            if let Err(error) = history.within(|| crate::runs::store(history, &runs)) {
                eprintln!("keeping runs: {error}");
            }
        }
        Err(error) => eprintln!("reading runs: {error}"),
    }
    match crate::quests::read(&config.quests) {
        Ok(quests) => {
            if let Err(error) = history.within(|| crate::quests::store(history, &quests)) {
                eprintln!("keeping quests: {error}");
            }
        }
        Err(error) => eprintln!("{error}"),
    }
}

/// Reads what pi's logs added, all of it, a slice at a time.
pub fn catch_up(history: &History, config: &Config) -> Result<(), String> {
    catch_up_kept(history, config, &mut Walks::default()).map(|_| ())
}

/// Reads what pi's logs added, as `catch_up`, going into no folder
/// `walks` has settled, then settling what's gone quiet. Answers the
/// logs the last walk found.
fn catch_up_kept(
    history: &History,
    config: &Config,
    walks: &mut Walks,
) -> Result<Vec<crate::ingest::Found>, String> {
    loop {
        let logs = crate::ingest::found_kept(config, walks);
        let pass = crate::ingest::scan(history, &logs, crate::ingest::Reading::of(config))?;
        if !pass.behind {
            walks.settle(history, std::time::SystemTime::now());
            return Ok(logs);
        }
    }
}

/// Reads what pi's logs added and tells the watch where they are. A
/// folder the watch can't follow leaves its run unsettled, walked each
/// pass as before, since nothing would say a log was made there.
fn read_logs(history: &History, config: &Config, walks: &mut Walks, watches: Option<&Watches>) {
    match catch_up_kept(history, config, walks) {
        Ok(logs) => {
            if let Some(watches) = watches {
                for folder in watches.follow(crate::ingest::followed_directories(config, &logs)) {
                    walks.touched(&folder);
                }
            }
        }
        Err(error) => eprintln!("reading pi's logs: {error}"),
    }
}

/// What of `findings` the margin and notices hear, while the tokens'
/// warning watches the cap whose findings' keys start `watched`. That cap
/// speaks only through the warning, so its own findings stay out: one
/// warning, said once. What the provider's meter says beside it is only a
/// note, which asks for no look: on 31 October the meter sat five hours
/// past its budget with nothing refused, while the cap's count refused.
/// The collection keeps them all.
fn heard(findings: &[Finding], watched: Option<&str>) -> Vec<Finding> {
    findings
        .iter()
        .filter(|finding| !watched.is_some_and(|key| finding.key.starts_with(key)))
        .cloned()
        .map(|mut finding| {
            if watched.is_some() && finding.detector == "said" {
                finding.severity = finding.severity.min(Severity::Note);
            }
            finding
        })
        .collect()
}

/// One whole pass: how things stand, what's found, what the shell shows
/// and what's said. `kept` carries what the last pass read that can't
/// have moved since.
pub fn pass(
    history: &History,
    config: &Config,
    writer: &mut Writer,
    desk: &mut dyn Desk,
    kept: &mut crate::picture::Kept,
    hub: &Hub,
    now: f64,
) -> Result<status::Status, String> {
    let mut picture = Picture::read_keeping(history, config, now, kept)?;
    let raised = crate::detect::all_keeping(&picture, history, config, &mut kept.minutes)?;
    let findings = crate::findings::settle(history, &raised, now)?;
    let standing = crate::standing::standing(&picture, config, None);
    // Keeping what was said over an allowance is best effort too: the
    // figures are written whether or not it could be.
    match crate::over::keep(history, &picture, &standing, &findings, &config.reports()) {
        Ok(crossings) => picture.crossings = crossings,
        Err(error) => eprintln!("keeping what was said over an allowance: {error}"),
    }
    let live = crate::tokens::Rate::Live;
    let inputs = crate::tokens::read(history, &picture.budgets, now, live)?.map(|mut inputs| {
        if let Some(feed) = config
            .feeds
            .iter()
            .find(|feed| feed.name == inputs.cap.feed)
        {
            inputs.counter.clone_from(&feed.label);
        }
        inputs
    });
    let tokens = inputs
        .as_ref()
        .and_then(|inputs| crate::tokens::outlook(inputs, writer.tokens, &picture.zone, now));
    writer.tokens = tokens.as_ref().map(|tokens| tokens.level);
    let watched = inputs.as_ref().map(crate::tokens::key);
    let said = heard(&findings, watched.as_deref());
    let mut shown = status::status(&picture, &said);
    shown.tokens = tokens;
    let collection = crate::collection::collection(&picture, &findings, config);
    writer.write(&config.runtime()?, &shown, &collection, Some(&standing))?;
    hub.publish(&collection, Some(&standing));
    hub.offer(&shown.actions);
    // Telling is best effort: the shell has its figures whether or not a
    // notice could be given, as with no notification server yet at login.
    if config.notices.enable
        && let Err(error) = tell(history, config, desk, &picture, &said, now)
    {
        eprintln!("giving notices: {error}");
    }
    if config.weekly_report
        && let Err(error) = weekly(config, &picture, &findings)
    {
        eprintln!("writing the weekly report: {error}");
    }
    Ok(shown)
}

fn tell(
    history: &History,
    config: &Config,
    desk: &mut dyn Desk,
    picture: &Picture,
    findings: &[crate::findings::Finding],
    now: f64,
) -> Result<(), String> {
    let changes = notices::plan(history, findings, config.notices.per_week, now)?;
    notices::apply(history, desk, changes, now)?;
    if config.notices.daily
        && let Some(line) = notices::daily(history, picture, now)?
        && desk.unlocked()
    {
        notices::apply(history, desk, vec![line], now)?;
    }
    Ok(())
}

/// Each Monday from six, last week's report, kept with the rest.
fn weekly(
    config: &Config,
    picture: &Picture,
    findings: &[crate::findings::Finding],
) -> Result<(), String> {
    let zone = &picture.zone;
    let today = zone.day(picture.now);
    if Zone::weekday(today) != 0 || zone.local_hour(picture.now) < 6 {
        return Ok(());
    }
    let path = config
        .reports()
        .join(format!("week-of-{}.md", Zone::iso_day(today - 7)));
    if path.exists() {
        return Ok(());
    }
    let mut text = crate::report::render(
        &crate::report::summary(picture, findings),
        crate::report::Format::Markdown,
    );
    for cut in [
        crate::report::Cut::Quest,
        crate::report::Cut::Model,
        crate::report::Cut::Agent,
        crate::report::Cut::Day,
    ] {
        text.push('\n');
        let report = crate::report::cut(picture, crate::report::Period::LastWeek, cut, 12);
        text.push_str(&crate::report::render(
            &report,
            crate::report::Format::Markdown,
        ));
    }
    text.push('\n');
    text.push_str(&crate::report::render(
        &crate::report::waste(picture),
        crate::report::Format::Markdown,
    ));
    status::write_atomic(&path, &text)
}

/// While a newer spend-watch keeps the history, as when an older system
/// is booted again, says so where the shell looks and leaves it be,
/// looking again at each heartbeat: the newer one replaces this when it's
/// run, or the history is put back to a shape this one reads. Stopping
/// instead would only be started again every few seconds by systemd.
fn hold(config: &Config) -> Result<(), String> {
    let path = config.history();
    let mut writer = Writer::new();
    let mut told = false;
    while let Some(shape) = History::newer(&path) {
        if !told {
            eprintln!(
                "{} is kept by a newer spend-watch, at shape {shape} where this one \
                 reads up to {}, so it's left as it is until that one runs",
                path.display(),
                crate::history::SCHEMA
            );
            told = true;
        }
        writer.write(
            &config.runtime()?,
            &status::held(time::now()),
            &crate::collection::held(shape),
            None,
        )?;
        std::thread::sleep(HEARTBEAT);
    }
    Ok(())
}

/// The socket's hub, holding what the last run wrote until this run's
/// first pass, which waits on reading pi's logs.
/// What does the shell's acts: going to a session as the command line
/// does, starting a program through niri, and marking a finding expected
/// in a history of its own, as the command line does, with a refresh so
/// the next pass shows it.
fn doer(config: &Config, wake: Sender<Wake>) -> Doer {
    let registry = config.registry.clone();
    let history = config.history();
    Box::new(move |act| match act {
        Act::Go(session) => Ok(crate::go::go(&registry, &session)?),
        Act::Launch(command) => acts::launch(&command),
        Act::Expect(key) => {
            let held = History::open(&history)?;
            crate::findings::mark(&held, &key, time::now())?;
            // Nobody waits on the refresh's answer: the act is done.
            let (reply, _) = mpsc::channel();
            let _ = wake.send(Wake::Asked("refresh".into(), reply));
            Ok(())
        }
    })
}

fn hub(config: &Config) -> Result<Arc<Hub>, String> {
    let since = u64::try_from(jiff::Timestamp::now().as_millisecond()).unwrap_or_default();
    // A vocabulary that can't be read leaves each word its id, as the
    // launcher says one it doesn't hold, rather than serving no rows.
    let vocabulary = config
        .vocabulary
        .as_ref()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .unwrap_or_default();
    let words = crate::ready::Words::from_vocabulary(&vocabulary).unwrap_or_default();
    let refusals = crate::interface::refusals(&vocabulary);
    let hub = Arc::new(Hub::new(since).worded(words).refusing(refusals));
    let runtime = config.runtime()?;
    if let Ok(written) = std::fs::read_to_string(runtime.join("collection.json")) {
        hub.seed(&written);
    }
    if let Ok(written) = std::fs::read_to_string(runtime.join("budgets.json")) {
        hub.seed_budgets(&written);
    }
    Ok(hub)
}

/// Serves what the runtime directory holds, `collection.json` and
/// `budgets.json` as the daemon last wrote them, on its socket as the
/// daemon tells them, ready rows and all, and nothing more: no pass, and
/// every act refused as not answering. It's for fixtures: a check or a
/// virtual machine writes the files to stand in for the daemon, and the
/// shell reads them over the same wire (Y2YBTS, 2026-11-11). Either file
/// written again is told as a pass's changes are, so a fixture changes
/// what's shown by writing it.
pub fn serve(config: &Config) -> Result<(), String> {
    // The files are watched before they're first read, so one written
    // between the two is heard rather than lost, as a fixture writing them
    // just as serve starts would otherwise leave it showing nothing.
    let runtime = config.runtime()?;
    std::fs::create_dir_all(&runtime)
        .map_err(|error| format!("making {}: {error}", runtime.display()))?;
    let (wake, inbox) = mpsc::channel();
    rewrites(&runtime, wake.clone());
    let hub = hub(config)?;
    hub.answering();
    listen(&config.socket(), wake, Arc::clone(&hub))?;
    for woken in inbox {
        match woken {
            // A file written again is told as a pass would tell it, so a
            // fixture can change what's shown without starting this again.
            Wake::Moved => hub.rewritten(
                &written(&runtime, "collection.json"),
                &written(&runtime, "budgets.json"),
            ),
            // A word the command line asks, as refresh, has no pass to
            // wait on.
            Wake::Asked(_, reply) => {
                let _ = reply.send("spend-watch is serving only what was written".into());
            }
            _ => {}
        }
    }
    Ok(())
}

/// Tells the loop whenever a file in the runtime directory is written, put
/// in place or removed, for `serve`. Where it can't hear them it says so,
/// once, so a fixture's rewrite gone unheard says why rather than leaving
/// a check to wait out its timeout.
#[cfg(target_os = "linux")]
fn rewrites(runtime: &std::path::Path, wake: Sender<Wake>) {
    use rustix::fs::inotify::{self, CreateFlags, WatchFlags};
    let folder = runtime.display().to_string();
    let fd = match inotify::init(CreateFlags::CLOEXEC) {
        Ok(fd) => fd,
        Err(error) => {
            eprintln!("serve: can't hear {folder} rewritten: {error}");
            return;
        }
    };
    let flags = WatchFlags::CLOSE_WRITE | WatchFlags::MOVED_TO | WatchFlags::DELETE;
    if let Err(error) = inotify::add_watch(&fd, runtime, flags) {
        eprintln!("serve: can't hear {folder} rewritten: {error}");
        return;
    }
    std::thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        loop {
            match rustix::io::read(&fd, &mut buffer) {
                Ok(_) => {
                    if wake.send(Wake::Moved).is_err() {
                        return;
                    }
                }
                Err(rustix::io::Errno::INTR) => {}
                Err(error) => {
                    eprintln!("serve: no longer hears {folder} rewritten: {error}");
                    return;
                }
            }
        }
    });
}

/// A file of the runtime directory as it's written, or nothing.
fn written(runtime: &std::path::Path, name: &str) -> String {
    std::fs::read_to_string(runtime.join(name)).unwrap_or_default()
}

/// Where inotify can't be had, `serve` tells only what it read first.
#[cfg(not(target_os = "linux"))]
fn rewrites(_runtime: &std::path::Path, _wake: Sender<Wake>) {}

/// The history at `path`, said in the journal as it's brought up from an
/// older shape: that holds it for minutes on a large one with nothing
/// else to show, and a restart meanwhile only starts the move again.
fn opened(path: &Path) -> Result<History, String> {
    let Some(shape) = History::older(path) else {
        return History::open(path);
    };
    let to = crate::history::SCHEMA;
    eprintln!(
        "bringing the history from shape {shape} to {to}, a few minutes on a large one; \
         a restart starts it again"
    );
    let began = Instant::now();
    let history = History::open(path)?;
    eprintln!(
        "the history is in shape {to}, after {:.0} s",
        began.elapsed().as_secs_f64()
    );
    Ok(history)
}

pub fn run(config: &Config) -> Result<(), String> {
    let _lock = lock(&config.state)?;
    config.runtime()?;
    hold(config)?;
    let history = opened(&config.history())?;
    let (wake, inbox): (Sender<Wake>, Receiver<Wake>) = mpsc::channel();
    let hub = hub(config)?;
    hub.doing(doer(config, wake.clone()));
    listen(&config.socket(), wake.clone(), Arc::clone(&hub))?;
    let watches = watch(config.clone(), wake.clone());
    tick(wake.clone());
    harness(&history, config);
    // A log that can't be read is said and passed over, never a reason to
    // stop: the loop tries again each pass.
    let mut walks = Walks::default();
    read_logs(&history, config, &mut walks, watches.as_deref());
    let mut writer = Writer::new();
    let mut desk = Buses::default();
    let mut kept = crate::picture::Kept::default();
    let mut feeds = feeds(config);
    let mut pace = Pace::Idle;
    // The first pass is due at once, as though the last were an idle wait
    // ago. The monotonic clock reaches far enough back that it always is.
    let mut last_pass = Instant::now()
        .checked_sub(IDLE)
        .unwrap_or_else(Instant::now);
    let mut last_harness = Instant::now();
    let mut dirty = true;
    let mut refresh = Refresh {
        waiting: Vec::new(),
        owed: Vec::new(),
        until: Instant::now(),
    };
    // The status last written, when its heartbeat was, and when a pass
    // last worked.
    let mut last_status: Option<(status::Status, Instant, Instant)> = None;
    loop {
        let now = time::now();
        pace = repace(&history, &mut feeds, pace, now);
        start_due(&history, &mut feeds, &wake, pace);
        if last_harness.elapsed() >= HARNESS {
            harness(&history, config);
            last_harness = Instant::now();
            dirty = true;
        }
        let answered = refresh.owed.is_empty() || Instant::now() >= refresh.until;
        let due = (dirty && last_pass.elapsed() >= BUSY)
            || last_pass.elapsed() >= IDLE
            || (!refresh.waiting.is_empty() && answered);
        if due {
            read_logs(&history, config, &mut walks, watches.as_deref());
            match pass(
                &history,
                config,
                &mut writer,
                &mut desk,
                &mut kept,
                &hub,
                now,
            ) {
                Ok(shown) => last_status = Some((shown, Instant::now(), Instant::now())),
                Err(error) => eprintln!("a pass failed: {error}"),
            }
            last_pass = Instant::now();
            dirty = false;
            refresh.owed.clear();
            for reply in refresh.waiting.drain(..) {
                let _ = reply.send("done".into());
            }
        } else if let Some((status, at, worked)) = &mut last_status
            && at.elapsed() >= HEARTBEAT
            && worked.elapsed() < FAILING
        {
            beat(config, status, now);
            *at = Instant::now();
        }
        let timeout = wait_for(&feeds, &refresh, dirty, last_pass);
        let first = match inbox.recv_timeout(timeout) {
            Ok(first) => Some(first),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("the daemon's threads are gone".into());
            }
        };
        // Everything waiting, at once, so a feed's backfill is one transaction.
        let mut arrived: Vec<Wake> = first.into_iter().collect();
        arrived.extend(inbox.try_iter());
        let stored_at = time::now();
        let began = history.begin().is_ok();
        for wake in arrived {
            if take(
                wake,
                &history,
                &mut feeds,
                &mut refresh,
                &mut walks,
                stored_at,
            ) {
                dirty = true;
            }
        }
        if began && let Err(error) = history.commit() {
            eprintln!("keeping what the feeds said: {error}");
        }
    }
}

/// Starts each feed that isn't running and is due to.
fn start_due(history: &History, feeds: &mut [Feed], wake: &Sender<Wake>, pace: Pace) {
    for feed in feeds {
        if feed.child.is_none() && Instant::now() >= feed.next {
            // Without it, a feed carries on from where it says it read.
            let kept = feed::kept_line(history, &feed.config.name)
                .map_err(|error| eprintln!("feed {}: {error}", feed.config.name))
                .ok();
            feed.start(wake, pace, kept);
        }
    }
}

/// Wakes the loop every heartbeat, until it's gone.
fn tick(wake: Sender<Wake>) {
    std::thread::spawn(move || {
        while wake.send(Wake::Tick).is_ok() {
            std::thread::sleep(HEARTBEAT);
        }
    });
}

/// Each feed the config names, not yet started and free to start now.
fn feeds(config: &Config) -> Vec<Feed> {
    config
        .feeds
        .iter()
        .map(|feed| Feed {
            config: feed.clone(),
            child: None,
            stdin: None,
            next: Instant::now(),
            backoff: None,
            started: Instant::now(),
        })
        .collect()
}

/// Writes the status last written again with a new heartbeat, so the shell
/// sees the watch still runs between passes.
fn beat(config: &Config, status: &mut status::Status, now: f64) {
    status.heartbeat = now;
    let text = serde_json::to_string(&*status).unwrap_or_default();
    let path = config.runtime().unwrap_or_default().join("status.json");
    if let Err(error) = status::write_atomic(&path, &text) {
        eprintln!("{error}");
    }
}

/// Who a refresh owes an answer: those waiting on it, and the feeds it
/// still waits to hear from, until when.
struct Refresh {
    waiting: Vec<Sender<String>>,
    owed: Vec<String>,
    until: Instant,
}

/// The pace pi's logs set, working while they moved lately, told to every
/// feed when it changes.
fn repace(history: &History, feeds: &mut [Feed], pace: Pace, now: f64) -> Pace {
    let logs_at: Option<f64> = history
        .db
        .query_row("SELECT MAX(at) FROM turns", [], |row| row.get(0))
        .ok()
        .flatten();
    let paced = if logs_at.is_some_and(|at| now - at < ACTIVE) {
        Pace::Active
    } else {
        Pace::Idle
    };
    if paced != pace {
        for feed in feeds {
            feed.say(&feed::pace_line(paced));
        }
    }
    paced
}

/// How long to wait for something, but never past when a pass or a feed
/// is due, or a refresh has waited long enough.
fn wait_for(feeds: &[Feed], refresh: &Refresh, dirty: bool, last_pass: Instant) -> Duration {
    let until_pass = if dirty {
        BUSY.saturating_sub(last_pass.elapsed())
    } else {
        IDLE.saturating_sub(last_pass.elapsed())
    };
    let until_feed = feeds
        .iter()
        .filter(|feed| feed.child.is_none())
        .map(|feed| feed.next.saturating_duration_since(Instant::now()))
        .min()
        .unwrap_or(IDLE);
    let until_answered = if refresh.waiting.is_empty() || refresh.owed.is_empty() {
        IDLE
    } else {
        refresh.until.saturating_duration_since(Instant::now())
    };
    until_pass
        .min(until_feed)
        .min(until_answered)
        .min(HEARTBEAT)
        .max(Duration::from_millis(100))
}

/// Takes one thing that woke the loop, saying whether it changes what the
/// next pass shows: a feed's record, or pi's logs moving.
fn take(
    wake: Wake,
    history: &History,
    feeds: &mut [Feed],
    refresh: &mut Refresh,
    walks: &mut Walks,
    stored_at: f64,
) -> bool {
    match wake {
        Wake::Line(name, line) => {
            refresh.owed.retain(|owing| *owing != name);
            match feed::parse(&line) {
                Ok(record) => {
                    if let Err(error) = feed::store(history, &name, &record, stored_at) {
                        eprintln!("feed {name}: keeping a record: {error}");
                    }
                    true
                }
                Err(error) => {
                    eprintln!(
                        "feed {name}: {error}: {}",
                        line.chars().take(200).collect::<String>()
                    );
                    false
                }
            }
        }
        Wake::Ended(name) => {
            if let Some(feed) = feeds.iter_mut().find(|feed| feed.config.name == name) {
                eprintln!("feed {name} stopped; starting it again in a while");
                feed.stopped();
            }
            false
        }
        // Polling can't say where, and walks every folder itself anyway.
        Wake::Moved => {
            walks.touched(Path::new("/"));
            true
        }
        Wake::Touched(folders) => {
            for folder in &folders {
                walks.touched(folder);
            }
            true
        }
        Wake::Asked(what, reply) => {
            match what.as_str() {
                "refresh" => {
                    // Answered once each running feed has said something
                    // back, or a little while has passed.
                    for feed in feeds {
                        if feed.child.is_some() {
                            feed.say(&feed::refresh_line());
                            refresh.owed.push(feed.config.name.clone());
                        }
                    }
                    refresh.until = Instant::now() + REFRESH;
                    refresh.waiting.push(reply);
                }
                "ping" => {
                    let _ = reply.send("here".into());
                }
                other => {
                    let _ = reply.send(format!("not something spend-watch is asked: {other}"));
                }
            }
            false
        }
        Wake::Tick => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The watch says which folder it heard written in, and forgets a
    // folder gone, so one made again by that name is watched afresh.
    #[cfg(target_os = "linux")]
    #[test]
    fn the_watch_says_where_it_heard_and_forgets_a_folder_gone() {
        use rustix::fs::inotify::{self, CreateFlags};
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("session");
        std::fs::create_dir(&folder).unwrap();
        let watches =
            Watches::new(inotify::init(CreateFlags::CLOEXEC | CreateFlags::NONBLOCK).unwrap());
        watches.follow([folder.clone()]);
        assert!(watches.follows(&folder), "the folder wasn't watched");
        std::fs::write(folder.join("2026_a.jsonl"), "{}\n").unwrap();
        let mut buffer = [std::mem::MaybeUninit::<u8>::uninit(); 8192];
        let mut events = inotify::Reader::new(&watches.fd, &mut buffer);
        assert_eq!(
            written_in(&mut events, &watches),
            Some(vec![folder.clone()])
        );
        std::fs::remove_dir_all(&folder).unwrap();
        // The kernel's word that the watch went comes once the folder is
        // freed, so it's waited for a little.
        for _ in 0..100 {
            if !watches.follows(&folder) {
                break;
            }
            let _ = written_in(&mut events, &watches);
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(!watches.follows(&folder), "a folder gone is still watched");
        std::fs::create_dir(&folder).unwrap();
        watches.follow([folder.clone()]);
        assert!(
            watches.follows(&folder),
            "a folder made again isn't watched"
        );
        std::fs::write(folder.join("2026_b.jsonl"), "{}\n").unwrap();
        assert_eq!(
            heard_all(&mut events, &watches),
            [folder],
            "the new watch wasn't heard"
        );
    }

    /// Everything the watch has heard until there's nothing left to read.
    #[cfg(target_os = "linux")]
    fn heard_all<Fd: std::os::fd::AsFd>(
        events: &mut rustix::fs::inotify::Reader<'_, Fd>,
        watches: &Watches,
    ) -> Vec<PathBuf> {
        let mut all = Vec::new();
        while let Some(touched) = written_in(events, watches) {
            all.extend(touched);
        }
        all.sort();
        all.dedup();
        all
    }

    // An overflow says it may have missed anything, and the watches it
    // renews still hear.
    #[cfg(target_os = "linux")]
    #[test]
    fn an_overflow_walks_every_folder_and_keeps_the_watches() {
        use rustix::fs::inotify::{self, CreateFlags};
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("session");
        std::fs::create_dir(&folder).unwrap();
        let watches =
            Watches::new(inotify::init(CreateFlags::CLOEXEC | CreateFlags::NONBLOCK).unwrap());
        watches.follow([folder.clone()]);
        let queued: usize = std::fs::read_to_string("/proc/sys/fs/inotify/max_queued_events")
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        // Each file made is two events, its making and its closing.
        for number in 0..=queued / 2 {
            std::fs::File::create(folder.join(number.to_string())).unwrap();
        }
        let mut buffer = [std::mem::MaybeUninit::<u8>::uninit(); 8192];
        let mut events = inotify::Reader::new(&watches.fd, &mut buffer);
        let heard = heard_all(&mut events, &watches);
        assert!(
            heard.contains(&PathBuf::from("/")),
            "no overflow was said: {heard:?}"
        );
        assert!(watches.follows(&folder), "the overflow lost the watch");
        std::fs::write(folder.join("late"), "").unwrap();
        assert_eq!(
            heard_all(&mut events, &watches),
            [folder],
            "the renewed watch wasn't heard"
        );
    }

    // A folder gone and made again by its name while its word went
    // unread is watched again once the watches are renewed.
    #[cfg(target_os = "linux")]
    #[test]
    fn renewing_watches_a_folder_made_again() {
        use rustix::fs::inotify::{self, CreateFlags};
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("session");
        std::fs::create_dir(&folder).unwrap();
        let watches =
            Watches::new(inotify::init(CreateFlags::CLOEXEC | CreateFlags::NONBLOCK).unwrap());
        watches.follow([folder.clone()]);
        std::fs::remove_dir(&folder).unwrap();
        std::fs::create_dir(&folder).unwrap();
        watches.renew();
        let mut buffer = [std::mem::MaybeUninit::<u8>::uninit(); 8192];
        let mut events = inotify::Reader::new(&watches.fd, &mut buffer);
        // The old watch's word that it went is read first, so what's
        // heard after can only come through the renewed one.
        heard_all(&mut events, &watches);
        std::fs::write(folder.join("2026_a.jsonl"), "{}\n").unwrap();
        assert_eq!(
            heard_all(&mut events, &watches),
            std::slice::from_ref(&folder),
            "the folder made again wasn't heard"
        );
        assert!(
            watches.follows(&folder),
            "the folder made again isn't watched"
        );
    }

    // A pass hands the watch the folders it found, with no event needed.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_pass_hands_the_watch_its_folders() {
        use rustix::fs::inotify::{self, CreateFlags};
        let dir = tempfile::tempdir().unwrap();
        let session = dir.path().join("runs/r1/reviewers/a/session");
        std::fs::create_dir_all(&session).unwrap();
        let header =
            r#"{"type":"session","id":"s1","timestamp":"2026-10-14T10:00:00Z","cwd":"/x"}"#;
        std::fs::write(session.join("2026_a.jsonl"), format!("{header}\n")).unwrap();
        let root = dir.path().display();
        let config: Config = serde_json::from_value(serde_json::json!({
            "sessions": format!("{root}/none"), "runs": "/r", "registry": "/g",
            "quests": "/q", "state": "/t",
            "agents": [{"pattern": format!("{root}/runs/*/reviewers/*/session/*.jsonl"),
                "kind": "review"}],
        }))
        .unwrap();
        let watches =
            Watches::new(inotify::init(CreateFlags::CLOEXEC | CreateFlags::NONBLOCK).unwrap());
        read_logs(
            &History::memory(),
            &config,
            &mut Walks::default(),
            Some(&watches),
        );
        assert!(watches.follows(&session), "the pass's folder isn't watched");
        let held: Vec<i32> = watches.held().keys().copied().collect();
        watches.give_up();
        for watch in held {
            assert!(
                inotify::remove_watch(&watches.fd, watch).is_err(),
                "a watch given up wasn't handed back"
            );
        }
        watches.follow([dir.path().to_path_buf()]);
        assert!(
            !watches.follows(dir.path()),
            "a watch given up took a folder"
        );
    }

    // A poll's change can't say where, so it walks every settled folder
    // again; a heard one walks only the folder it was heard in.
    #[test]
    fn a_poll_s_change_unsettles_every_folder_and_a_heard_one_its_own() {
        let history = History::memory();
        let mut refresh = Refresh {
            waiting: Vec::new(),
            owed: Vec::new(),
            until: Instant::now(),
        };
        let pattern = "/r/*/s/*.jsonl";
        let mut walks = Walks::settled_at(pattern, &["/r/a", "/r/b"]);
        let heard = Wake::Touched(vec![PathBuf::from("/r/b/s")]);
        assert!(
            take(heard, &history, &mut [], &mut refresh, &mut walks, 0.0),
            "a heard write didn't mark the logs"
        );
        assert_eq!(
            walks.settled_tops(),
            [PathBuf::from("/r/a")],
            "a heard write didn't unsettle its own run alone"
        );
        let overflow = Wake::Touched(vec![PathBuf::from("/")]);
        take(overflow, &history, &mut [], &mut refresh, &mut walks, 0.0);
        assert!(
            walks.settled_tops().is_empty(),
            "an overflow left a run settled"
        );
        let mut walks = Walks::settled_at(pattern, &["/r/a", "/r/b"]);
        assert!(
            take(
                Wake::Moved,
                &history,
                &mut [],
                &mut refresh,
                &mut walks,
                0.0
            ),
            "a poll's change didn't mark the logs"
        );
        assert!(
            walks.settled_tops().is_empty(),
            "a poll's change left a run settled"
        );
    }

    // A folder the kernel won't watch is handed back, for the pass to
    // leave its run unsettled; one it takes isn't.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_folder_the_kernel_refuses_is_handed_back() {
        use rustix::fs::inotify::{self, CreateFlags};
        let dir = tempfile::tempdir().unwrap();
        let watches =
            Watches::new(inotify::init(CreateFlags::CLOEXEC | CreateFlags::NONBLOCK).unwrap());
        // A name past the kernel's 255 bytes is refused whoever runs it.
        let refused = dir.path().join("n".repeat(300));
        assert_eq!(
            watches.follow([refused.clone()]),
            [refused],
            "a refusal wasn't handed back"
        );
        assert!(
            watches.follow([dir.path().to_path_buf()]).is_empty(),
            "a watch taken was handed back"
        );
    }

    // A sessions folder not yet made is made and watched; one that can't
    // be made leaves the daemon polling, with no watch for its passes to
    // feed.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_sessions_folder_is_made_or_polled_with_no_watch() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let config = |sessions: PathBuf| -> Config {
            serde_json::from_value(serde_json::json!({
                "sessions": sessions, "runs": "/r", "registry": "/g",
                "quests": "/q", "state": "/t",
            }))
            .unwrap()
        };
        // A folder can't be made under a file.
        let file = dir.path().join("file");
        std::fs::write(&file, "").unwrap();
        let (wake, _heard) = mpsc::channel();
        assert!(
            watch(config(file.join("sessions")), wake).is_none(),
            "a watch nobody reads was handed back"
        );
        let sessions = dir.path().join("agent/sessions");
        let (wake, _heard) = mpsc::channel();
        assert!(
            watch(config(sessions.clone()), wake).is_some(),
            "a sessions folder not yet made wasn't made and watched"
        );
        // A log can hold a token, so pi's folders are this account's alone.
        for folder in [&sessions, &dir.path().join("agent")] {
            let mode = std::fs::metadata(folder).unwrap().permissions().mode() & 0o777;
            assert_eq!(
                mode,
                0o700,
                "{} was made readable by others",
                folder.display()
            );
        }
    }

    fn found(key: &str, detector: &str) -> Finding {
        Finding {
            key: key.into(),
            detector: detector.into(),
            severity: Severity::Warning,
            title: String::new(),
            detail: String::new(),
            dollars: 0.0,
            surprise: 1.0,
            badge: String::new(),
            note: String::new(),
            session: None,
            quest: None,
            action: None,
            cap: false,
            started: 0.0,
            marked: false,
        }
    }

    // On 31 October the provider's meter sat five hours past its budget
    // with nothing refused, while the cap's own count was what refused.
    #[test]
    fn a_watched_cap_speaks_alone_and_the_meter_beside_it_only_notes() {
        let findings = [
            found("cap:proxy:week:near", "caps"),
            found("said:0123", "said"),
            found("burn:x", "burn"),
        ];
        let kept: Vec<(String, Severity)> = heard(&findings, Some("cap:proxy:week:"))
            .into_iter()
            .map(|finding| (finding.key, finding.severity))
            .collect();
        assert_eq!(
            kept,
            [
                ("said:0123".to_owned(), Severity::Note),
                ("burn:x".to_owned(), Severity::Warning)
            ]
        );
        // With no cap watched, the meter is all there is to go on.
        assert_eq!(heard(&findings, None)[1].severity, Severity::Warning);
    }

    // Five seconds first, as the contract says, doubling while a feed
    // keeps stopping quickly, and five again after one that ran a while.
    #[test]
    fn a_feed_that_keeps_failing_waits_longer_each_time() {
        let quick = Duration::from_secs(1);
        let first = backoff(None, quick);
        assert_eq!(first, Duration::from_secs(5));
        let second = backoff(Some(first), quick);
        assert_eq!(second, Duration::from_secs(10));
        let most = Duration::from_secs(300);
        assert_eq!(backoff(Some(most), quick), most);
        assert_eq!(backoff(Some(second), Duration::from_secs(600)), first);
    }
}
