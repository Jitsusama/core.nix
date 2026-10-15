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
/// How long a refresh waits for each feed to say something back.
const REFRESH: Duration = Duration::from_secs(10);
/// How long the heartbeat goes on without a pass that worked: past this,
/// the shell is let see a watch that isn't working as one that stopped.
const FAILING: Duration = Duration::from_secs(2 * 300 + 60);

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

/// Tells the loop whenever pi's logs change: inotify on the sessions
/// directory and each directory in it, adding one as it appears, and on
/// each directory an agent's log is in.
#[cfg(target_os = "linux")]
fn watch(config: Config, wake: Sender<Wake>) {
    use rustix::fs::inotify::{self, CreateFlags, WatchFlags};
    std::thread::spawn(move || {
        let sessions = config.sessions.clone();
        let Ok(fd) = inotify::init(CreateFlags::CLOEXEC) else {
            return poll(config, wake);
        };
        let flags = WatchFlags::MODIFY
            | WatchFlags::CREATE
            | WatchFlags::MOVED_TO
            | WatchFlags::CLOSE_WRITE;
        let mut watched: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
        let mut looked: Option<Instant> = None;
        // A directory counts as watched only once inotify has it. The
        // agents' patterns are followed again at most once a pass, since
        // pi's own logs change many times a minute.
        let add = |watched: &mut std::collections::HashSet<PathBuf>,
                   looked: &mut Option<Instant>| {
            let mut paths: Vec<PathBuf> = std::iter::once(sessions.clone())
                .chain(
                    std::fs::read_dir(&sessions)
                        .into_iter()
                        .flatten()
                        .flatten()
                        .map(|entry| entry.path())
                        .filter(|path| path.is_dir()),
                )
                .collect();
            if looked.is_none_or(|at| at.elapsed() >= BUSY) {
                *looked = Some(Instant::now());
                paths.extend(crate::ingest::agents_directories(&config));
            }
            for path in paths {
                if !watched.contains(&path) && inotify::add_watch(&fd, &path, flags).is_ok() {
                    watched.insert(path);
                }
            }
        };
        add(&mut watched, &mut looked);
        // No sessions directory yet, as before pi's first run: polling
        // notices it appear.
        if !watched.contains(&sessions) {
            return poll(config, wake);
        }
        let mut buffer = [0u8; 8192];
        loop {
            match rustix::io::read(&fd, &mut buffer) {
                Ok(_) => {
                    add(&mut watched, &mut looked);
                    if wake.send(Wake::Moved).is_err() {
                        return;
                    }
                }
                Err(rustix::io::Errno::INTR) => {}
                Err(_) => return poll(config, wake),
            }
        }
    });
}

#[cfg(not(target_os = "linux"))]
fn watch(config: Config, wake: Sender<Wake>) {
    std::thread::spawn(move || poll(config, wake));
}

/// Where inotify can't be had: the logs' newest change, every five seconds.
fn poll(config: Config, wake: Sender<Wake>) {
    let newest = || {
        crate::ingest::found(&config)
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
    loop {
        let logs = crate::ingest::found(config);
        let pass = crate::ingest::scan(history, &logs, crate::ingest::Reading::of(config))?;
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
) -> Result<status::Status, String> {
    let picture = Picture::read(history, config, now)?;
    let raised = crate::detect::all(&picture, history, config)?;
    let findings = crate::findings::settle(history, raised, now)?;
    let shown = status::status(&picture, &findings);
    writer.write(
        &config.runtime()?,
        &shown,
        &crate::collection::collection(&picture, &findings, config),
    )?;
    // Telling is best effort: the shell has its figures whether or not a
    // notice could be given, as with no notification server yet at login.
    if config.notices.enable
        && let Err(error) = tell(history, config, desk, &picture, &findings, now)
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
    let changes = notices::plan(history, findings, config.notices.budget, now)?;
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
        )?;
        std::thread::sleep(HEARTBEAT);
    }
    Ok(())
}

pub fn run(config: Config) -> Result<(), String> {
    let _lock = lock(&config.state)?;
    config.runtime()?;
    hold(&config)?;
    let history = History::open(&config.history())?;
    let (wake, woken): (Sender<Wake>, Receiver<Wake>) = mpsc::channel();
    listen(&config.socket(), wake.clone())?;
    watch(config.clone(), wake.clone());
    {
        let wake = wake.clone();
        std::thread::spawn(move || {
            while wake.send(Wake::Tick).is_ok() {
                std::thread::sleep(HEARTBEAT);
            }
        });
    }
    harness(&history, &config);
    // A log that can't be read is said and passed over, never a reason to
    // stop: the loop tries again each pass.
    if let Err(error) = catch_up(&history, &config) {
        eprintln!("reading pi's logs: {error}");
    }
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
            backoff: None,
            started: Instant::now(),
        })
        .collect();
    let mut pace = Pace::Idle;
    let mut last_pass = Instant::now() - IDLE;
    let mut last_harness = Instant::now();
    let mut dirty = true;
    let mut waiting: Vec<Sender<String>> = Vec::new();
    // The feeds a refresh still waits to hear from, and until when.
    let mut owed: Vec<String> = Vec::new();
    let mut owed_until = Instant::now();
    // The status last written, when its heartbeat was, and when a pass
    // last worked.
    let mut last_status: Option<(status::Status, Instant, Instant)> = None;
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
                // Without it, a feed carries on from where it says it read.
                let kept = feed::kept_line(&history, &feed.config.name)
                    .map_err(|error| eprintln!("feed {}: {error}", feed.config.name))
                    .ok();
                feed.start(&wake, pace, kept);
            }
        }
        if last_harness.elapsed() >= HARNESS {
            harness(&history, &config);
            last_harness = Instant::now();
            dirty = true;
        }
        let answered = owed.is_empty() || Instant::now() >= owed_until;
        let due = (dirty && last_pass.elapsed() >= BUSY)
            || last_pass.elapsed() >= IDLE
            || (!waiting.is_empty() && answered);
        if due {
            if let Err(error) = catch_up(&history, &config) {
                eprintln!("reading pi's logs: {error}");
            }
            match pass(&history, &config, &mut writer, &mut desk, now) {
                Ok(shown) => last_status = Some((shown, Instant::now(), Instant::now())),
                Err(error) => eprintln!("a pass failed: {error}"),
            }
            last_pass = Instant::now();
            dirty = false;
            owed.clear();
            for reply in waiting.drain(..) {
                let _ = reply.send("done".into());
            }
        } else if let Some((status, at, worked)) = &mut last_status
            && at.elapsed() >= HEARTBEAT
            && worked.elapsed() < FAILING
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
        let until_answered = if waiting.is_empty() || owed.is_empty() {
            IDLE
        } else {
            owed_until.saturating_duration_since(Instant::now())
        };
        let timeout = until_pass
            .min(until_feed)
            .min(until_answered)
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
                Wake::Line(name, line) => {
                    owed.retain(|owing| *owing != name);
                    match feed::parse(&line) {
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
                    }
                }
                Wake::Ended(name) => {
                    if let Some(feed) = feeds.iter_mut().find(|feed| feed.config.name == name) {
                        eprintln!("feed {name} stopped; starting it again in a while");
                        feed.stopped();
                    }
                }
                Wake::Moved => dirty = true,
                Wake::Asked(what, reply) => match what.as_str() {
                    "refresh" => {
                        // Answered once each running feed has said
                        // something back, or a little while has passed.
                        for feed in &mut feeds {
                            if feed.child.is_some() {
                                feed.say(&feed::refresh_line());
                                owed.push(feed.config.name.clone());
                            }
                        }
                        owed_until = Instant::now() + REFRESH;
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

#[cfg(test)]
mod tests {
    use super::*;

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
