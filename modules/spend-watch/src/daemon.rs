//! The daemon: one per account, held by a lock on its state directory. It
//! reads pi's logs as they grow, told by inotify, and the harness's runs
//! and quests every few minutes; it runs each feed, hands its records to
//! the history and restarts it when it stops; and after anything new, at
//! most every thirty seconds and at least every five minutes, it works
//! out how things stand, settles its findings, writes the shell's files
//! and gives notices. Between, a heartbeat every thirty seconds says it
//! still runs. A socket in the runtime directory answers `refresh`.

use crate::config::{Config, FeedConfig};
use crate::feed::{self, Pace};
use crate::history::History;
use crate::notices::{self, Buses, Desk};
use crate::picture::Picture;
use crate::status::{self, Writer};
use crate::time::{self, Zone};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

/// What wakes the loop.
enum Wake {
    Line(String, String),
    Ended(String),
    Moved,
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

struct Feed {
    config: FeedConfig,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    /// When it may start again, after stopping.
    next: Instant,
    backoff: Duration,
    started: Instant,
}

impl Feed {
    fn start(&mut self, wake: &Sender<Wake>, pace: Pace) {
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
                self.child = Some(child);
                self.started = Instant::now();
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
        if let Some(stdin) = &mut self.stdin
            && writeln!(stdin, "{line}")
                .and_then(|()| stdin.flush())
                .is_err()
        {
            self.stdin = None;
        }
    }

    /// Reaped, and due to start again after its backoff: doubled each time
    /// it stops within a minute of starting, from five seconds to five
    /// minutes.
    fn stopped(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.stdin = None;
        self.backoff = if self.started.elapsed() < Duration::from_secs(60) {
            (self.backoff * 2).clamp(Duration::from_secs(5), Duration::from_secs(300))
        } else {
            Duration::from_secs(5)
        };
        self.next = Instant::now() + self.backoff;
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

/// Tells the loop whenever pi's logs change: inotify on the sessions
/// directory and each directory in it, adding one as it appears.
#[cfg(target_os = "linux")]
fn watch(sessions: PathBuf, wake: Sender<Wake>) {
    use rustix::fs::inotify::{self, CreateFlags, WatchFlags};
    std::thread::spawn(move || {
        let Ok(fd) = inotify::init(CreateFlags::CLOEXEC) else {
            return poll(sessions, wake);
        };
        let flags = WatchFlags::MODIFY
            | WatchFlags::CREATE
            | WatchFlags::MOVED_TO
            | WatchFlags::CLOSE_WRITE;
        let mut watched: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
        let add = |watched: &mut std::collections::HashSet<PathBuf>| {
            if watched.insert(sessions.clone()) {
                let _ = inotify::add_watch(&fd, &sessions, flags);
            }
            for entry in std::fs::read_dir(&sessions).into_iter().flatten().flatten() {
                let path = entry.path();
                if path.is_dir() && watched.insert(path.clone()) {
                    let _ = inotify::add_watch(&fd, &path, flags);
                }
            }
        };
        add(&mut watched);
        let mut buffer = [0u8; 8192];
        loop {
            match rustix::io::read(&fd, &mut buffer) {
                Ok(_) => {
                    add(&mut watched);
                    if wake.send(Wake::Moved).is_err() {
                        return;
                    }
                }
                Err(rustix::io::Errno::INTR) => {}
                Err(_) => return poll(sessions, wake),
            }
        }
    });
}

#[cfg(not(target_os = "linux"))]
fn watch(sessions: PathBuf, wake: Sender<Wake>) {
    std::thread::spawn(move || poll(sessions, wake));
}

/// Where inotify can't be had: the logs' newest change, every five seconds.
fn poll(sessions: PathBuf, wake: Sender<Wake>) {
    let newest = || {
        crate::ingest::logs(&sessions)
            .iter()
            .filter_map(|path| std::fs::metadata(path).ok())
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

fn listen(socket: &Path, wake: Sender<Wake>) -> Result<(), String> {
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
            std::thread::spawn(move || answer(stream, wake));
        }
    });
    Ok(())
}

fn answer(stream: UnixStream, wake: Sender<Wake>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut reader = BufReader::new(&stream);
    let mut asked = String::new();
    if reader.read_line(&mut asked).is_err() {
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
        let mut stream = &stream;
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
            if let Err(error) = crate::runs::store(history, &runs) {
                eprintln!("keeping runs: {error}");
            }
        }
        Err(error) => eprintln!("reading runs: {error}"),
    }
    let quests = crate::quests::read(&config.quests);
    if let Err(error) = crate::quests::store(history, &quests) {
        eprintln!("keeping quests: {error}");
    }
}

/// Reads what pi's logs added, all of it, a slice at a time.
pub fn catch_up(history: &History, config: &Config) -> Result<(), String> {
    loop {
        let pass = crate::ingest::scan(history, &config.sessions, &config.refusals)?;
        if !pass.behind {
            return Ok(());
        }
    }
}

/// One whole pass: how things stand, what's found, what the shell shows
/// and what's said.
pub fn pass(
    history: &History,
    config: &Config,
    writer: &mut Writer,
    desk: &mut dyn Desk,
    now: f64,
) -> Result<Picture, String> {
    let picture = Picture::read(history, config, now)?;
    let raised = crate::detect::all(&picture, history, config)?;
    let findings = crate::findings::settle(history, raised, now)?;
    writer.write(
        &config.runtime()?,
        &status::status(&picture, &findings),
        &status::collection(&picture, &findings, config),
    )?;
    if config.notices.enable {
        let changes = notices::plan(history, &findings, config.notices.budget, now)?;
        notices::apply(history, desk, changes, now)?;
        if config.notices.daily
            && let Some(line) = notices::daily(history, &picture, now)?
            && desk.unlocked()
        {
            notices::apply(history, desk, vec![line], now)?;
        }
    }
    if config.weekly_report {
        weekly(config, &picture, &findings)?;
    }
    Ok(picture)
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

pub fn run(config: Config) -> Result<(), String> {
    let _lock = lock(&config.state)?;
    config.runtime()?;
    let history = History::open(&config.history())?;
    let (wake, woken): (Sender<Wake>, Receiver<Wake>) = mpsc::channel();
    listen(&config.socket(), wake.clone())?;
    watch(config.sessions.clone(), wake.clone());
    {
        let wake = wake.clone();
        std::thread::spawn(move || {
            while wake.send(Wake::Tick).is_ok() {
                std::thread::sleep(HEARTBEAT);
            }
        });
    }
    harness(&history, &config);
    catch_up(&history, &config)?;
    let mut writer = Writer::new();
    let mut desk = Buses::default();
    let mut feeds: Vec<Feed> = config
        .feeds
        .iter()
        .map(|feed| Feed {
            config: feed.clone(),
            child: None,
            stdin: None,
            next: Instant::now(),
            backoff: Duration::from_secs(5),
            started: Instant::now(),
        })
        .collect();
    let mut pace = Pace::Idle;
    let mut last_pass = Instant::now() - IDLE;
    let mut last_harness = Instant::now();
    let mut dirty = true;
    let mut waiting: Vec<Sender<String>> = Vec::new();
    let mut last_status: Option<(status::Status, Instant)> = None;
    loop {
        let now = time::now();
        // Working is pi's logs moving lately.
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
            pace = paced;
            for feed in &mut feeds {
                feed.say(&feed::pace_line(pace));
            }
        }
        for feed in &mut feeds {
            if feed.child.is_none() && Instant::now() >= feed.next {
                feed.start(&wake, pace);
            }
        }
        if last_harness.elapsed() >= HARNESS {
            harness(&history, &config);
            last_harness = Instant::now();
            dirty = true;
        }
        let due = (dirty && last_pass.elapsed() >= BUSY)
            || last_pass.elapsed() >= IDLE
            || !waiting.is_empty();
        if due {
            if let Err(error) = catch_up(&history, &config) {
                eprintln!("reading pi's logs: {error}");
            }
            match pass(&history, &config, &mut writer, &mut desk, now) {
                Ok(picture) => {
                    let findings = crate::findings::standing(&history).unwrap_or_default();
                    last_status = Some((status::status(&picture, &findings), Instant::now()));
                }
                Err(error) => eprintln!("a pass failed: {error}"),
            }
            last_pass = Instant::now();
            dirty = false;
            for reply in waiting.drain(..) {
                let _ = reply.send("done".into());
            }
        } else if let Some((status, at)) = &mut last_status
            && at.elapsed() >= HEARTBEAT
        {
            status.heartbeat = now;
            let text = serde_json::to_string(&*status).unwrap_or_default();
            let path = config.runtime().unwrap_or_default().join("status.json");
            if let Err(error) = status::write_atomic(&path, &text) {
                eprintln!("{error}");
            }
            *at = Instant::now();
        }
        // Wait for something, but never past when a pass or a feed is due.
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
        let timeout = until_pass
            .min(until_feed)
            .min(HEARTBEAT)
            .max(Duration::from_millis(100));
        let first = match woken.recv_timeout(timeout) {
            Ok(first) => Some(first),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("the daemon's threads are gone".into());
            }
        };
        // Everything waiting, at once, so a feed's backfill is one transaction.
        let mut woke: Vec<Wake> = first.into_iter().collect();
        woke.extend(woken.try_iter());
        let mut lines = 0;
        let stored_at = time::now();
        let began = history.begin().is_ok();
        for wake in woke {
            match wake {
                Wake::Line(name, line) => match feed::parse(&line) {
                    Ok(record) => {
                        if let Err(error) = feed::store(&history, &name, &record, stored_at) {
                            eprintln!("feed {name}: keeping a record: {error}");
                        }
                        lines += 1;
                    }
                    Err(error) => eprintln!(
                        "feed {name}: {error}: {}",
                        line.chars().take(200).collect::<String>()
                    ),
                },
                Wake::Ended(name) => {
                    if let Some(feed) = feeds.iter_mut().find(|feed| feed.config.name == name) {
                        eprintln!("feed {name} stopped; starting it again in a while");
                        feed.stopped();
                    }
                }
                Wake::Moved => dirty = true,
                Wake::Asked(what, reply) => match what.as_str() {
                    "refresh" => {
                        for feed in &mut feeds {
                            feed.say(&feed::refresh_line());
                        }
                        waiting.push(reply);
                    }
                    "ping" => {
                        let _ = reply.send("here".into());
                    }
                    other => {
                        let _ = reply.send(format!("not something spend-watch is asked: {other}"));
                    }
                },
                Wake::Tick => {}
            }
        }
        if began && let Err(error) = history.commit() {
            eprintln!("keeping what the feeds said: {error}");
        }
        if lines > 0 {
            dirty = true;
        }
    }
}
