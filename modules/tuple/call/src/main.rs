//! tuple-call SOCKET TUPLE
//!
//! The call Joel's in, held for the shell and anything else that asks,
//! over SOCKET in the shell's daemon interface (../../daemon-serve serves
//! it, as it serves every daemon of the shell): one record,
//! `call:current`, said whole to each watch as it changes (lib.rs says
//! what it holds). TUPLE is the tuple command.
//!
//! How the call stands comes from Tuple's own stream of state, `tuple
//! --format json state follow --raw` (stream.rs). Any tuple command
//! starts Tuple's daemon when none is running, so the stream is followed
//! only while the daemon answers on its socket, `tuple-0` in the runtime
//! folder, looked for every five seconds and as soon as Tuple's log says
//! its daemon started; a stopped Tuple stays stopped. A follow that keeps
//! ending at once, as it does while nobody is signed in, is tried less
//! and less often, up to once a minute.
//!
//! A ring and a missed call come from Tuple's log, `tuple/0/log.txt` in
//! the data folder, where the XDG spec puts it (log.rs). It's followed as
//! it's written, through inotify on its folder, from its last lines as
//! this starts, so a ring already going counts; Tuple's daemon writes it
//! afresh each time it starts, and the new one is read from its start.
//!
//! The call's acts are asks on the same socket, each done through Tuple's
//! own command on a thread of its own, so a share waiting on the portal
//! holds up nothing else: join a link, leave, mute or unmute, share a
//! screen, answer a ring, and open Joel's room, whose link `tuple new`
//! prints and this keeps, in `tuple-call/room` in the state folder, for
//! the record to say from then on; until then, the record says the newest
//! of Joel's own rooms, as Tuple's state lists them.
//!
//! Each call that ends and each missed is kept in Recent (recent.rs), with
//! who rang whom where a ring answered or Joel's room opened says, and
//! in `tuple-call/recent.jsonl` in the state folder as it changes, read
//! back as this starts, so Joel's last calls outlive a restart.

use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant, SystemTime};

use daemon_serve::asks::Domain;
use daemon_serve::serve::{self, Own};
use inotify::{EventMask, Inotify, WatchMask};
use serde_json::{Value, json};

use tuple_call::Held;
use tuple_call::ask::{self, Act, refused};

/// The daemon as its clients know it: Tuple's, keeping the one call.
const DOMAIN: Domain = Domain {
    name: "tuple",
    kinds: &[ask::KIND],
    sole: &[],
    named_only: &[],
};
use tuple_call::recent::Recent;
use tuple_call::stream::{self, Line};

/// What follows the tuple command for its stream.
const STATE: [&str; 5] = ["--format", "json", "state", "follow", "--raw"];

/// How often Tuple's daemon is looked for while it isn't answering.
const LOOKS_AGAIN: Duration = Duration::from_secs(5);
/// The longest wait once Tuple's follow keeps ending at once.
const WAITS_AT_MOST: Duration = Duration::from_secs(60);
/// A follow that lasted this long was following, so the wait starts over.
const FOLLOWED_FOR: Duration = Duration::from_secs(60);

/// How many of the log's last lines are read as this starts.
const READ_BACK: usize = 400;

/// How Tuple stands, as hello and health say it.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Standing {
    /// Its daemon isn't answering, as when it's stopped: no call.
    Starting,
    Answering,
    /// Nobody is signed in to Tuple.
    SignedOut,
    /// Its stream couldn't be followed, and why.
    Refused(String),
}

impl Standing {
    fn word(&self) -> &'static str {
        match self {
            Standing::Starting => "starting",
            Standing::Answering => "answering",
            Standing::SignedOut => "signed-out",
            Standing::Refused(_) => "refused",
        }
    }
}

/// What the threads hold of the call.
struct Daemon {
    held: Held,
    standing: Standing,
    /// The record as last said, less when.
    said: Value,
    /// When it last changed, in milliseconds since the epoch.
    heard: i64,
    /// Recent as its file last kept it, and the file.
    saved: Recent,
    recent: PathBuf,
}

/// Everything the threads share: the call, woken as it changes, and
/// what serves it, handed each change.
struct Calls {
    daemon: Mutex<Daemon>,
    moved: Condvar,
    /// Set once, as it starts, since what serves it answers the acts,
    /// which change the call.
    serving: OnceLock<Arc<serve::Shared>>,
}

type Shared = Arc<Calls>;

fn now() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_millis()).unwrap_or(i64::MAX)
        })
}

/// A folder the XDG spec names by a variable, else its default in home.
fn folder(variable: &str, default: &str) -> PathBuf {
    std::env::var_os(variable)
        .filter(|value| !value.is_empty())
        .map_or_else(
            || PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(default),
            PathBuf::from,
        )
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(socket), Some(tuple)) = (args.next(), args.next()) else {
        eprintln!("usage: tuple-call SOCKET TUPLE");
        std::process::exit(64);
    };
    let listener = match serve::listen(Path::new(&socket)) {
        Ok(listener) => listener,
        Err(why) => {
            eprintln!("<3>couldn't listen on {socket}: {why}");
            std::process::exit(71);
        }
    };
    let runtime = folder("XDG_RUNTIME_DIR", "");
    let tuples = runtime.join("tuple-0");
    let log = folder("XDG_DATA_HOME", ".local/share").join("tuple/0");
    let state = folder("XDG_STATE_HOME", ".local/state").join("tuple-call");
    let room = state.join("room");
    let recent = state.join("recent.jsonl");
    let kept = std::fs::read_to_string(&room)
        .ok()
        .map(|text| ask::link(&text).to_owned())
        .filter(|link| !link.is_empty());
    let began = now();
    let mut held = Held::new(began, kept);
    held.recent = Recent::read(&std::fs::read_to_string(&recent).unwrap_or_default());
    let saved = held.recent.clone();
    let said = held.record();
    let shared: Shared = Arc::new(Calls {
        daemon: Mutex::new(Daemon {
            held,
            standing: Standing::Starting,
            said,
            heard: began,
            saved,
            recent,
        }),
        moved: Condvar::new(),
        serving: OnceLock::new(),
    });
    let doing = Doing {
        tuple: tuple.clone(),
        room,
    };
    let serving = serve::Shared::new(DOMAIN, acts(&shared, doing));
    serving.slow(ask::ACTS);
    let _ = shared.serving.set(Arc::clone(&serving));
    tell(&shared, &lock(&shared));
    let (wake, woken) = mpsc::channel();
    {
        let shared = Arc::clone(&shared);
        let tuple = tuple.clone();
        std::thread::spawn(move || follow_tuple(&shared, &tuple, &tuples, &woken));
    }
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || follow_log(&shared, &log, &wake));
    }
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || end_rings(&shared));
    }
    // The socket systemd holds for it (tuple-call.socket), taken as
    // daemon-serve takes every daemon's, served for as long as this runs.
    serve::accept(serving, listener);
    loop {
        std::thread::park();
    }
}

fn lock(shared: &Shared) -> MutexGuard<'_, Daemon> {
    shared
        .daemon
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The record as last said, with when.
fn record(daemon: &Daemon) -> Value {
    let mut record = daemon.said.clone();
    record["heard"] = json!(daemon.heard);
    record
}

/// Tells every watch of the record, once it's no longer as last said, and
/// wakes whatever waits on a change. Recent is kept in its file first,
/// once it's no longer as the file holds it.
fn changed(shared: &Shared, daemon: &mut Daemon) {
    if daemon.held.recent != daemon.saved {
        match write(&daemon.recent, &daemon.held.recent.lines()) {
            Ok(()) => daemon.saved = daemon.held.recent.clone(),
            Err(why) => eprintln!("<4>couldn't keep the calls had lately: {why}"),
        }
    }
    let record_now = daemon.held.record();
    if record_now != daemon.said {
        daemon.said = record_now;
        daemon.heard = now();
        tell(shared, daemon);
    }
    shared.moved.notify_all();
}

/// Hands what serves the shell the record and how Tuple stands, which it
/// tells each watch once either changes.
fn tell(shared: &Shared, daemon: &Daemon) {
    if let Some(serving) = shared.serving.get() {
        serving.change(vec![record(daemon)], sources(&daemon.standing));
    }
}

/// How Tuple stands, as the interface says a source does: by its word,
/// and why, once it's refused.
fn sources(standing: &Standing) -> Value {
    match standing {
        Standing::Refused(why) => json!({"tuple": {"state": "refused", "reason": why}}),
        other => json!({"tuple": other.word()}),
    }
}

/// Says how Tuple stands, telling every watch once that changes, so a
/// client hears a sign-in go or come as Tuple says so.
fn stand(shared: &Shared, daemon: &mut Daemon, standing: Standing) {
    if daemon.standing == standing {
        return;
    }
    if let Standing::Refused(why) = &standing {
        eprintln!("<4>{why}");
    }
    daemon.standing = standing;
    tell(shared, daemon);
}

/// Follows Tuple's stream whenever its daemon answers on `socket`.
fn follow_tuple(shared: &Shared, tuple: &str, socket: &Path, woken: &Receiver<()>) {
    let mut wait = LOOKS_AGAIN;
    loop {
        // Tuple starting before this look is seen by it; only a start
        // after it wakes the wait below.
        while woken.try_recv().is_ok() {}
        // A stopped daemon leaves its socket's file behind, so only a
        // connection tells.
        if UnixStream::connect(socket).is_ok() {
            let began = Instant::now();
            let why = follow(shared, tuple);
            // A stream that ends as Tuple stops is Tuple stopped, not
            // its stream refused.
            let standing = if UnixStream::connect(socket).is_ok() {
                Standing::Refused(why)
            } else {
                Standing::Starting
            };
            let mut daemon = lock(shared);
            daemon.held.follow(None, now());
            stand(shared, &mut daemon, standing);
            changed(shared, &mut daemon);
            wait = if began.elapsed() >= FOLLOWED_FOR {
                LOOKS_AGAIN
            } else {
                (wait * 2).min(WAITS_AT_MOST)
            };
        } else {
            let mut daemon = lock(shared);
            daemon.held.follow(None, now());
            stand(shared, &mut daemon, Standing::Starting);
            changed(shared, &mut daemon);
            wait = LOOKS_AGAIN;
        }
        match woken.recv_timeout(wait) {
            Ok(()) => wait = LOOKS_AGAIN,
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => std::thread::sleep(wait),
        }
    }
}

/// Follows Tuple's stream until it ends, saying why it did.
fn follow(shared: &Shared, tuple: &str) -> String {
    let mut child = match Command::new(tuple)
        .args(STATE)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(why) => return format!("couldn't start {tuple}: {why}"),
    };
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return "Tuple's stream had no output".to_owned();
    };
    // The first line holds every contact, over two megabytes.
    let mut input = BufReader::with_capacity(1 << 20, stdout);
    let mut line = Vec::new();
    loop {
        line.clear();
        match input.read_until(b'\n', &mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(why) if why.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
        let text = String::from_utf8_lossy(&line);
        match stream::read(text.trim_end()) {
            Line::State {
                contacts,
                signed_in,
                room,
                call,
            } => {
                let mut daemon = lock(shared);
                daemon.held.contacts = contacts;
                daemon.held.personal = room;
                if let Some(call) = call {
                    daemon.held.follow(call, now());
                }
                let standing = if signed_in {
                    Standing::Answering
                } else {
                    Standing::SignedOut
                };
                // The record goes first, so whoever hears that Tuple
                // answers already holds what its state said.
                changed(shared, &mut daemon);
                stand(shared, &mut daemon, standing);
            }
            Line::Call(call) => {
                let mut daemon = lock(shared);
                daemon.held.follow(call, now());
                changed(shared, &mut daemon);
            }
            Line::Other => {}
        }
    }
    drop(input);
    let _ = child.kill();
    match child.wait() {
        Ok(status) => format!("Tuple's stream ended: {status}"),
        Err(why) => format!("Tuple's stream ended: {why}"),
    }
}

/// Ends each ring that has rung its minute, as missed.
fn end_rings(shared: &Shared) {
    let mut daemon = lock(shared);
    loop {
        let Some(ends) = daemon.held.log.ends() else {
            daemon = shared
                .moved
                .wait(daemon)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            continue;
        };
        let left = ends - now();
        if left <= 0 {
            let missed = daemon.held.log.tick(now());
            daemon.held.miss(missed);
            changed(shared, &mut daemon);
            continue;
        }
        let left = Duration::from_millis(u64::try_from(left).unwrap_or(0));
        daemon = shared
            .moved
            .wait_timeout(daemon, left)
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .0;
    }
}

/// Takes lines the log said, waking the stream's follow when one says
/// Tuple's daemon started.
fn took(shared: &Shared, lines: &[String], wake: &Sender<()>) {
    if lines.is_empty() {
        return;
    }
    let mut daemon = lock(shared);
    let now = now();
    for line in lines {
        let heard = daemon.held.log.heard(line, now);
        daemon.held.miss(heard.missed);
        daemon.held.answer(heard.answered);
        if heard.started {
            let _ = wake.send(());
        }
    }
    changed(shared, &mut daemon);
}

/// Tuple's log as it's written: complete lines from where it was last
/// read, a part line kept for the rest of it.
#[derive(Default)]
struct Reading {
    file: Option<File>,
    at: u64,
    part: Vec<u8>,
}

impl Reading {
    /// Opens the log afresh, from its start, or only its last lines when
    /// `back` says so.
    fn open(&mut self, path: &Path, back: bool) -> Vec<String> {
        *self = Reading {
            file: File::open(path).ok(),
            ..Reading::default()
        };
        let lines = self.more();
        if back {
            let skip = lines.len().saturating_sub(READ_BACK);
            return lines.into_iter().skip(skip).collect();
        }
        lines
    }

    /// The lines written since it was last read. A log that's shorter
    /// than that was written afresh, and is read from its start.
    fn more(&mut self) -> Vec<String> {
        let Some(file) = &mut self.file else {
            return Vec::new();
        };
        if file.metadata().is_ok_and(|meta| meta.len() < self.at) {
            self.at = 0;
            self.part.clear();
        }
        if file.seek(SeekFrom::Start(self.at)).is_err() {
            return Vec::new();
        }
        let mut read = Vec::new();
        let Ok(count) = file.read_to_end(&mut read) else {
            return Vec::new();
        };
        self.at += count as u64;
        self.part.extend_from_slice(&read);
        let Some(last) = self.part.iter().rposition(|&byte| byte == b'\n') else {
            return Vec::new();
        };
        let whole: Vec<u8> = self.part.drain(..=last).collect();
        String::from_utf8_lossy(&whole)
            .lines()
            .map(str::to_owned)
            .collect()
    }
}

/// Follows Tuple's log in `folder`, for as long as this runs.
fn follow_log(shared: &Shared, folder: &Path, wake: &Sender<()>) {
    let path = folder.join("log.txt");
    let mut back = true;
    let mut buffer = vec![0u8; 4096];
    loop {
        let Ok(mut inotify) = Inotify::init() else {
            eprintln!("<3>couldn't watch Tuple's log");
            return;
        };
        // Tuple makes its folder as it first starts.
        if inotify
            .watches()
            .add(
                folder,
                WatchMask::MODIFY
                    | WatchMask::CREATE
                    | WatchMask::MOVED_TO
                    | WatchMask::DELETE_SELF
                    | WatchMask::MOVE_SELF,
            )
            .is_err()
        {
            std::thread::sleep(LOOKS_AGAIN);
            continue;
        }
        let mut reading = Reading::default();
        took(shared, &reading.open(&path, back), wake);
        back = false;
        'watching: loop {
            let Ok(events) = inotify.read_events_blocking(&mut buffer) else {
                break;
            };
            let mut lines = Vec::new();
            for event in events {
                if event
                    .mask
                    .intersects(EventMask::DELETE_SELF | EventMask::MOVE_SELF | EventMask::IGNORED)
                {
                    took(shared, &lines, wake);
                    break 'watching;
                }
                if event.name.is_none_or(|name| name != "log.txt") {
                    continue;
                }
                if event
                    .mask
                    .intersects(EventMask::CREATE | EventMask::MOVED_TO)
                {
                    lines.extend(reading.open(&path, false));
                } else {
                    lines.extend(reading.more());
                }
            }
            took(shared, &lines, wake);
        }
    }
}

/// What the acts need: the tuple command, and where the room is kept.
struct Doing {
    tuple: String,
    room: PathBuf,
}

/// What answers the call's acts, each on a thread of its own (ask's
/// `ACTS`), which daemon-serve asks as a client asks one; it answers
/// none of the interface's own asks, which daemon-serve answers.
fn acts(shared: &Shared, doing: Doing) -> Own {
    let shared = Arc::clone(shared);
    Box::new(move |said| ask::act(said).map(|asked| act(&shared, &doing, &asked)))
}

/// Does one of the call's acts through Tuple's command, answering once
/// it's done.
fn act(shared: &Shared, doing: &Doing, asked: &Act) -> Value {
    let arguments: Vec<&str> = match asked {
        Act::Join { link } if link.is_empty() => return refused("that isn't a call's link"),
        Act::Join { link } => vec!["call", "join", link],
        Act::Leave => vec!["call", "leave"],
        Act::Mute { muted: true } => vec!["call", "mute"],
        Act::Mute { muted: false } => vec!["call", "unmute"],
        Act::Share => vec!["share"],
        Act::Accept => vec!["call", "accept"],
        Act::Open => return open(shared, doing),
    };
    let output = Command::new(&doing.tuple)
        .args(&arguments)
        .stdin(Stdio::null())
        .output();
    match output {
        Ok(output) if output.status.success() => {
            // Once Tuple says it's done, which it does as well when he
            // already was, that's how Joel stands, ahead of its stream.
            if let Act::Mute { muted } = asked {
                let mut daemon = lock(shared);
                if let Some(live) = &mut daemon.held.live {
                    live.joel.muted = *muted;
                }
                changed(shared, &mut daemon);
            }
            json!({"done": true})
        }
        Ok(output) => {
            let said = String::from_utf8_lossy(&output.stderr);
            let why = said.lines().rev().find(|line| !line.trim().is_empty());
            refused(
                why.map_or_else(|| format!("tuple ended: {}", output.status), str::to_owned)
                    .as_str(),
            )
        }
        Err(why) => refused(&format!("couldn't run tuple: {why}")),
    }
}

/// Opens Joel's room with `tuple new`, answering with the link it prints
/// as it does, and keeping it.
fn open(shared: &Shared, doing: &Doing) -> Value {
    let mut child = match Command::new(&doing.tuple)
        .arg("new")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(why) => return refused(&format!("couldn't run tuple: {why}")),
    };
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return refused("tuple new printed nothing");
    };
    let mut lines = BufReader::new(stdout).lines();
    let found = lines
        .by_ref()
        .map_while(Result::ok)
        .map(|line| ask::link(&line).to_owned())
        .find(|link| !link.is_empty());
    // Whatever else it prints is read and let go while it runs on.
    std::thread::spawn(move || {
        lines.for_each(drop);
        let _ = child.wait();
    });
    let Some(link) = found else {
        return refused("tuple new printed no link");
    };
    keep(&doing.room, &link);
    let mut daemon = lock(shared);
    daemon.held.room = Some(link.clone());
    // `tuple new` starts a call in Joel's room, so whoever comes to it
    // came to his call.
    daemon.held.began(String::new(), now());
    changed(shared, &mut daemon);
    json!({"done": true, "link": link})
}

/// Keeps the room's link.
fn keep(room: &Path, link: &str) {
    if let Err(why) = write(room, &format!("{link}\n")) {
        eprintln!("<4>couldn't keep the room's link: {why}");
    }
}

/// Writes a file of the state folder beside it and moves it into place,
/// so it's never read half written.
fn write(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let new = path.with_extension("new");
    std::fs::write(&new, text)?;
    std::fs::rename(&new, path)
}
