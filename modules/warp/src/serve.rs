//! `warp serve SOCKET`: WARP's link held for the shell over the socket
//! systemd holds for it, in version 1 of the shell's daemon interface
//! (RSCH-20261029-0E8050), so the launcher spawns nothing to know it or to
//! switch it (Joel, 2026-11-10: "CLI execs is not performant").
//!
//! The link is one record, `warp:link`: what `warp --json` prints, the
//! record or `{ error, said }`, under `said`, with when it was read. It's
//! read only while someone watches it, every two seconds as WARP's figures
//! move, once more at once after a switch, and at once each time WARP's
//! status changes, as a warp-cli held listening tells it, so a link that
//! drops or degrades is said as it does rather than up to two seconds on.
//! Nothing is asked of warp-cli while the launcher shows something else. A watch is answered
//! with what's held only while it's that fresh, and otherwise with
//! nothing until the read it sets off, so an hour-old link is never shown
//! as now. A get reads afresh where what's held is older. `act` switches
//! WARP, `connect` or `disconnect`, as `warp connect` does, and answers
//! with what that printed.
//!
//! How WARP stands is a second kind, `warp-state`, and its one record,
//! `warp:state`: its status and reason, as words under `said`, or
//! `{ error }` while warp-cli can't say, with when that began where it was
//! seen to. It's what the listening warp-cli tells, so the margin, which
//! watches it all the time to say when the work network ails, sets off
//! none of the link's reads; where nothing can listen, as while WARP's
//! service is down, the status is asked once, and again five seconds on.
//!
//! Each connection is served on a thread of its own, its lines written by
//! another, so a client that stops reading holds no other back; one with
//! more than a few hundred lines unread is hung up on.

use std::io::{BufRead, BufReader, Write};
use std::net::Shutdown;
use std::os::fd::FromRawFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::ask::{Ask, Listening, Said, Unasked};

/// The interface's version this speaks.
const VERSION: u64 = 1;
/// The link's kind, and the one record of it.
const KIND: &str = "warp";
const KEY: &str = "warp:link";
/// How WARP stands, and the one record of it.
const STATE_KIND: &str = "warp-state";
const STATE_KEY: &str = "warp:state";
/// How often the link is read while watched, and how old what's held may
/// be and still answer for now: WARP's figures move by the second, and
/// warp-cli's five answers take about 80 ms together.
const FRESH: Duration = Duration::from_secs(2);
/// How long a status stream that ended by itself, as one from a warp-cli
/// that can't listen does, waits before it's asked for again: such a
/// warp-cli is then run twice a minute while WARP is shown rather than in
/// a loop, and the two-second reads still say each change meanwhile.
const LISTEN_AGAIN: Duration = Duration::from_secs(30);
/// How long a status asked where nothing could listen is held before a
/// listener is tried again, while WARP's service isn't answering, so it's
/// said back within seconds of answering again.
const STATE_AGAIN: Duration = Duration::from_secs(5);
/// How many lines a connection may have waiting to be written before
/// it's taken as no longer reading and hung up on.
const QUEUED: usize = 256;
/// How long to wait after a connection couldn't be taken, as when the
/// process is out of descriptors, so the loop doesn't spin a core.
const ACCEPT_BACKOFF: Duration = Duration::from_millis(100);

/// Which of the two kinds an ask names.
#[derive(Clone, Copy)]
struct Kinds {
    link: bool,
    state: bool,
}

/// One watch a client holds.
struct Watch {
    connection: u64,
    id: Value,
    kinds: Kinds,
    told: SyncSender<String>,
    /// Its connection, to hang up on once it stops reading.
    stream: Arc<UnixStream>,
}

struct Held {
    /// The link as last read, and when.
    link: Option<(Value, Instant)>,
    /// How WARP stands, as last told or asked while anyone watched.
    state: Option<Value>,
    watches: Vec<Watch>,
    /// Whether it's to be read at once, rather than when it's next due.
    now: bool,
    /// How to stop the warp-cli listening to WARP's status, while one is.
    listening: Option<Box<dyn FnOnce() + Send>>,
    /// When it started, in milliseconds since the epoch.
    since: u64,
}

impl Held {
    /// The link held, while it's fresh enough to say for now.
    fn fresh(&self) -> Option<&Value> {
        self.link
            .as_ref()
            .filter(|(_, at)| at.elapsed() < FRESH)
            .map(|(link, _)| link)
    }

    /// Whether anyone watches the link, which only then is read.
    fn reads(&self) -> bool {
        self.watches.iter().any(|watch| watch.kinds.link)
    }

    /// Tells each watch of a kind a record changed, and hangs up on any
    /// connection whose lines aren't being read, dropping its watches,
    /// rather than telling it less than it asked for.
    fn tell(&mut self, record: &Value, wants: impl Fn(Kinds) -> bool) {
        let mut stuck = Vec::new();
        for watch in self.watches.iter().filter(|watch| wants(watch.kinds)) {
            let told = json!({"id": watch.id, "changed": [record]});
            if watch.told.try_send(told.to_string()).is_err() {
                stuck.push(watch.connection);
                let _ = watch.stream.shutdown(Shutdown::Both);
            }
        }
        self.watches
            .retain(|watch| !stuck.contains(&watch.connection));
    }
}

/// What every connection shares with the reading loop.
struct Shared<A> {
    held: Mutex<Held>,
    woken: Condvar,
    /// Held while warp-cli is asked, so two reads never run at once.
    reading: Mutex<()>,
    cli: A,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

/// What `warp` prints for these words, as JSON.
fn printed(words: &[&str], cli: &impl Ask) -> Value {
    let args: Vec<String> = words
        .iter()
        .map(|word| (*word).to_owned())
        .chain(["--json".to_owned()])
        .collect();
    serde_json::from_str(&crate::run(&args, cli).out).unwrap_or(Value::Null)
}

/// Serves the link on this socket for as long as the program runs.
pub fn serve(socket: &Path, cli: impl Ask + Send + 'static) -> std::io::Result<()> {
    let listener = listen(socket)?;
    let shared = Arc::new(Shared {
        held: Mutex::new(Held {
            link: None,
            state: None,
            watches: Vec::new(),
            now: false,
            listening: None,
            since: now_ms(),
        }),
        woken: Condvar::new(),
        reading: Mutex::new(()),
        cli,
    });
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || read_loop(&shared));
    }
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || listen_loop(&shared));
    }
    for (connection, stream) in (1u64..).zip(listener.incoming()) {
        let Ok(stream) = stream else {
            std::thread::sleep(ACCEPT_BACKOFF);
            continue;
        };
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || answer_each(&shared, stream, connection));
    }
    Ok(())
}

/// The socket systemd holds for it (warp.socket), so the shell never
/// finds it missing while the daemon starts or starts again; or, run by
/// hand, its own, in a folder only Joel reads, replacing one left behind.
fn listen(socket: &Path) -> std::io::Result<UnixListener> {
    if let Some(listener) = handed() {
        return Ok(listener);
    }
    if let Some(folder) = socket.parent() {
        std::fs::create_dir_all(folder)?;
        std::fs::set_permissions(folder, std::fs::Permissions::from_mode(0o700))?;
    }
    let _ = std::fs::remove_file(socket);
    UnixListener::bind(socket)
}

/// The listening socket systemd hands this process, as the first
/// descriptor after the standard three, when it says so (`sd_listen_fds`).
fn handed() -> Option<UnixListener> {
    let pid: u32 = std::env::var("LISTEN_PID").ok()?.parse().ok()?;
    let count: u32 = std::env::var("LISTEN_FDS").ok()?.parse().ok()?;
    if pid != std::process::id() || count < 1 {
        return None;
    }
    // SAFETY: systemd hands descriptor 3 to this process alone, named by
    // LISTEN_PID, and nothing here has opened or taken it before.
    Some(unsafe { UnixListener::from_raw_fd(3) })
}

/// Reads the link while anyone watches it, as each read comes due or at
/// once when asked to, and sleeps while nobody does; once nobody watches
/// either kind, it stops the warp-cli listening for them, and lets go of
/// how WARP stood, which nothing then keeps current.
fn read_loop<A: Ask>(shared: &Shared<A>) {
    loop {
        let mut held = lock(&shared.held);
        while !held.reads() && !held.now {
            if held.watches.is_empty() {
                held.state = None;
                if let Some(stop) = held.listening.take() {
                    drop(held);
                    stop();
                    held = lock(&shared.held);
                    continue;
                }
            }
            held = shared
                .woken
                .wait(held)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        let age = held
            .link
            .as_ref()
            .map_or(Duration::MAX, |(_, at)| at.elapsed());
        if !held.now && age < FRESH {
            drop(
                shared
                    .woken
                    .wait_timeout(held, FRESH.saturating_sub(age))
                    .unwrap_or_else(std::sync::PoisonError::into_inner),
            );
            continue;
        }
        held.now = false;
        drop(held);
        read(shared);
    }
}

/// Holds a warp-cli listening to WARP's status while anyone watches,
/// holds how WARP stands as each line tells it, and has the link read at
/// once on each change, while it's watched, rather than when the next
/// read comes due; `read_loop` stops it once nobody watches. Where none
/// can listen, or one ends by itself, the status is asked instead.
fn listen_loop<A: Ask>(shared: &Shared<A>) {
    loop {
        {
            let mut held = lock(&shared.held);
            while held.watches.is_empty() {
                held = shared
                    .woken
                    .wait(held)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
        }
        let Some(Listening { lines, stop }) = shared.cli.listen() else {
            ask_instead(shared);
            continue;
        };
        {
            let mut held = lock(&shared.held);
            // The last watch may have gone while it started.
            if held.watches.is_empty() {
                drop(held);
                stop();
                continue;
            }
            held.listening = Some(stop);
        }
        let mut last = None;
        for line in lines.lines() {
            let Ok(line) = line else { break };
            if let Some((status, reason)) = told(&line) {
                stand(shared, &json!({"status": status, "reason": reason}));
            }
            if changed(&line, &mut last) {
                let mut held = lock(&shared.held);
                held.now = held.reads();
                shared.woken.notify_all();
            }
        }
        // Still held, it ended by itself rather than being stopped.
        let ended = lock(&shared.held).listening.take();
        if let Some(stop) = ended {
            stop();
            ask_instead(shared);
        }
    }
}

/// Asks how WARP stands where no warp-cli is listening, and holds it, then
/// waits before one is tried again: five seconds while WARP's service
/// isn't answering, so it's said back soon after it is, and half a
/// minute otherwise, as where warp-cli can't listen at all, while the
/// link's two-second reads carry on for whoever watches it.
fn ask_instead<A: Ask>(shared: &Shared<A>) {
    let said = standing(shared.cli.ask(&["status"]));
    let failed = said.get("error").is_some();
    if !lock(&shared.held).watches.is_empty() {
        stand(shared, &said);
    }
    std::thread::sleep(if failed { STATE_AGAIN } else { LISTEN_AGAIN });
}

/// How WARP stands as one status ask says it, in the words a listening
/// warp-cli tells: its status and reason, or the error code warp-cli
/// gives, as `FailedToConnectToDaemon` is while its service is down.
fn standing(said: Result<Said, Unasked>) -> Value {
    match said {
        Ok(said) => {
            let printed: Value = serde_json::from_str(&said.text).unwrap_or(Value::Null);
            match stands(&printed) {
                Some((status, reason)) if said.ok => json!({"status": status, "reason": reason}),
                _ => json!({"error": printed["code"].as_str().unwrap_or("unreadable")}),
            }
        }
        Err(Unasked::Missing(_)) => json!({"error": "missing"}),
        Err(Unasked::Stuck(_)) => json!({"error": "stuck"}),
    }
}

/// Holds how WARP stands, and tells each watch of it, when it's other
/// than what's held. When it began is said only where it was seen to
/// begin, since a state first heard may have stood for hours.
fn stand<A: Ask>(shared: &Shared<A>, said: &Value) {
    let mut held = lock(&shared.held);
    if held
        .state
        .as_ref()
        .is_some_and(|state| state["said"] == *said)
    {
        return;
    }
    let since = held.state.as_ref().map(|_| now_ms());
    let state = json!({
        "key": STATE_KEY,
        "kind": STATE_KIND,
        "read_at_ms": now_ms(),
        "since_ms": since,
        "said": said,
    });
    held.tell(&state, |kinds| kinds.state);
    held.state = Some(state);
}

/// The status and reason a line of warp-cli's tells, a reason being a
/// word, or a word holding figures, as `NetworkDegraded` is; nothing for a
/// line of anything else, as its DNS logs are.
fn told(line: &str) -> Option<(String, String)> {
    stands(&serde_json::from_str::<Value>(line).ok()?)
}

fn stands(said: &Value) -> Option<(String, String)> {
    let status = said["status"].as_str()?;
    let reason = match &said["reason"] {
        Value::String(word) => word.clone(),
        Value::Object(named) => named.keys().next().cloned().unwrap_or_default(),
        _ => String::new(),
    };
    Some((status.to_owned(), reason))
}

/// Whether a line a listening warp-cli wrote tells a status or a reason
/// other than the last it told, which it keeps as the last. The first
/// says only how WARP stands, which a watch's own read says; a reason's
/// figures moving are the two-second reads' to say; and a line of
/// anything else, as its DNS logs are, sets nothing off.
fn changed(line: &str, last: &mut Option<(String, String)>) -> bool {
    let Some(now) = told(line) else {
        return false;
    };
    let now = Some(now);
    let first = last.is_none();
    let moved = *last != now;
    *last = now;
    moved && !first
}

/// Reads the link afresh, holds it, and tells every watch of it.
fn read<A: Ask>(shared: &Shared<A>) {
    let reading = lock(&shared.reading);
    let link = json!({
        "key": KEY,
        "kind": KIND,
        "read_at_ms": now_ms(),
        "said": printed(&[], &shared.cli),
    });
    let mut held = lock(&shared.held);
    drop(reading);
    held.tell(&link, |kinds| kinds.link);
    held.link = Some((link, Instant::now()));
}

fn answer_each<A: Ask>(shared: &Shared<A>, stream: UnixStream, connection: u64) {
    let Ok(reading) = stream.try_clone() else {
        return;
    };
    let Ok(hangup) = stream.try_clone().map(Arc::new) else {
        return;
    };
    let (told, lines) = mpsc::sync_channel::<String>(QUEUED);
    let mut writing = stream;
    std::thread::spawn(move || {
        for line in lines {
            if writeln!(writing, "{line}").is_err() {
                break;
            }
        }
    });
    for line in BufReader::new(reading).lines() {
        let Ok(line) = line else { break };
        let said: Value = serde_json::from_str(&line).unwrap_or(Value::Null);
        if !answer(shared, &said, connection, &told, &hangup) {
            break;
        }
    }
    lock(&shared.held)
        .watches
        .retain(|watch| watch.connection != connection);
    // So the reading loop, which may sleep with no link to read, stops
    // the listener once this was the last watch.
    shared.woken.notify_all();
}

/// An error answer, by the interface's codes.
fn error(id: &Value, code: &str, message: &str) -> Value {
    json!({"id": id, "error": {"code": code, "message": message}})
}

/// The kinds an ask names, both where it names none, or nothing where it
/// names one not kept here; an ask naming keys asks only theirs.
fn kinds_named(said: &Value) -> Option<Kinds> {
    let named: Vec<&str> = match &said["kind"] {
        Value::Null => vec![KIND, STATE_KIND],
        Value::String(kind) => vec![kind.as_str()],
        Value::Array(kinds) => kinds
            .iter()
            .map(|kind| kind.as_str().unwrap_or(""))
            .collect(),
        _ => return None,
    };
    if !named.iter().all(|kind| [KIND, STATE_KIND].contains(kind)) {
        return None;
    }
    let keyed = |key: &str| {
        said["keys"]
            .as_array()
            .is_none_or(|keys| keys.iter().any(|asked| asked == key))
    };
    Some(Kinds {
        link: named.contains(&KIND) && keyed(KEY),
        state: named.contains(&STATE_KIND) && keyed(STATE_KEY),
    })
}

/// What's held of these kinds: the link while it's fresh, and how WARP
/// stands while anyone watches it.
fn held_of(held: &Held, kinds: Kinds) -> Vec<Value> {
    let link = held.fresh().filter(|_| kinds.link);
    let state = held.state.as_ref().filter(|_| kinds.state);
    link.into_iter().chain(state).cloned().collect()
}

/// Answers one ask, and says whether its answer could be queued; one
/// that can't is hung up on, as a change would hang up on it.
fn answer<A: Ask>(
    shared: &Shared<A>,
    said: &Value,
    connection: u64,
    told: &SyncSender<String>,
    hangup: &Arc<UnixStream>,
) -> bool {
    let id = said.get("id").cloned().unwrap_or(Value::Null);
    let asked = said["ask"].as_str().unwrap_or_default();
    // Switching and a get past what's fresh ask warp-cli, so they're done
    // before the lock is taken, and no other ask waits on them.
    let switched = match (asked, said["act"].as_str()) {
        ("act", Some(act @ ("connect" | "disconnect"))) => Some(printed(&[act], &shared.cli)),
        _ => None,
    };
    let kinds = kinds_named(said);
    if asked == "get"
        && kinds.is_some_and(|kinds| kinds.link)
        && lock(&shared.held).fresh().is_none()
    {
        read(shared);
    }
    let mut held = lock(&shared.held);
    let answer = match (asked, kinds) {
        ("hello" | "health", _) => json!({
            "id": id, "version": VERSION, "domain": KIND, "state": "answering",
            "since": held.since, "sources": {}, "watches": held.watches.len(),
        }),
        (_, None) => error(
            &id,
            "unknown-kind",
            &format!("no kind {} here", said["kind"]),
        ),
        ("get" | "list", Some(kinds)) => {
            json!({"id": id, "records": held_of(&held, kinds)})
        }
        ("watch", Some(kinds)) => {
            let records = held_of(&held, kinds);
            // Only the link is read for a watch; how WARP stands is the
            // listener's to tell, once the watch wakes it.
            if kinds.link && held.fresh().is_none() {
                held.now = true;
            }
            // A watch asked again by its id replaces the one before.
            held.watches
                .retain(|watch| !(watch.connection == connection && watch.id == id));
            held.watches.push(Watch {
                connection,
                id: id.clone(),
                kinds,
                told: told.clone(),
                stream: Arc::clone(hangup),
            });
            shared.woken.notify_all();
            json!({"id": id, "records": records})
        }
        ("unwatch", Some(_)) => {
            held.watches
                .retain(|watch| !(watch.connection == connection && watch.id == id));
            shared.woken.notify_all();
            json!({"id": id, "unwatched": true})
        }
        ("act", Some(_)) => match switched {
            Some(switched) => {
                // How the link then goes is read at once, and told after
                // this answer, since the read waits on this lock.
                held.now = true;
                shared.woken.notify_all();
                json!({"id": id, "done": true, "switched": switched})
            }
            None => error(&id, "unknown-act", "WARP's acts are connect and disconnect"),
        },
        _ => error(&id, "unknown-ask", &format!("no ask {asked:?} here")),
    };
    // Queued while the lock is held, so a change can't be queued before
    // the answer it follows.
    told.try_send(answer.to_string()).is_ok()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use super::*;
    use crate::contract::{self, Contract};

    /// warp-cli as the contract answers it, but for a link that goes down
    /// once it's asked to disconnect or the test takes it down, listening
    /// once, where the test gives it lines to tell.
    struct Switching {
        cli: Contract,
        up: Arc<AtomicBool>,
        listened: Mutex<Option<std::io::PipeReader>>,
        /// How many questions other than the status it was asked.
        asked: Arc<AtomicUsize>,
    }

    impl Ask for Switching {
        fn listen(&self) -> Option<Listening> {
            let lines = lock(&self.listened).take()?;
            Some(Listening {
                lines: Box::new(BufReader::new(lines)),
                stop: Box::new(|| {}),
            })
        }

        fn ask(&self, question: &[&str]) -> Result<Said, Unasked> {
            match question {
                ["status"] => Ok(Said {
                    ok: true,
                    text: if self.up.load(Ordering::SeqCst) {
                        r#"{"status": "Connected", "reason": "NetworkHealthy"}"#
                    } else {
                        r#"{"status": "Disconnected", "reason": "Manual"}"#
                    }
                    .into(),
                }),
                ["disconnect"] => {
                    self.up.store(false, Ordering::SeqCst);
                    Ok(Said {
                        ok: true,
                        text: "Success\n".into(),
                    })
                }
                _ => {
                    self.asked.fetch_add(1, Ordering::SeqCst);
                    self.cli.ask(question)
                }
            }
        }
    }

    struct Client {
        lines: BufReader<UnixStream>,
        stream: UnixStream,
    }

    impl Client {
        fn new(socket: &Path) -> Self {
            let started = Instant::now();
            let stream = loop {
                match UnixStream::connect(socket) {
                    Ok(stream) => break stream,
                    Err(why) => {
                        assert!(
                            started.elapsed() < Duration::from_secs(10),
                            "the socket never answered: {why}"
                        );
                        // The daemon binds its socket on a thread of its
                        // own; a moment is long enough for it.
                        std::thread::sleep(Duration::from_millis(10));
                    }
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .expect("a timeout can be set");
            Self {
                lines: BufReader::new(stream.try_clone().expect("it clones")),
                stream,
            }
        }

        fn say(&mut self, said: &Value) {
            writeln!(self.stream, "{said}").expect("the daemon reads");
        }

        fn heard(&mut self) -> Value {
            let mut line = String::new();
            self.lines.read_line(&mut line).expect("the daemon says");
            serde_json::from_str(&line).expect("a line is JSON")
        }
    }

    fn serving(name: &str) -> std::path::PathBuf {
        serving_listened(name, None, Arc::new(AtomicBool::new(true)))
    }

    fn serving_listened(
        name: &str,
        listened: Option<std::io::PipeReader>,
        up: Arc<AtomicBool>,
    ) -> std::path::PathBuf {
        serving_counted(name, listened, up, Arc::new(AtomicUsize::new(0)))
    }

    fn serving_counted(
        name: &str,
        listened: Option<std::io::PipeReader>,
        up: Arc<AtomicBool>,
        asked: Arc<AtomicUsize>,
    ) -> std::path::PathBuf {
        let socket = std::env::temp_dir().join(format!("warp-{name}-{}.sock", std::process::id()));
        let cli = Switching {
            cli: Contract::whole(),
            up,
            listened: Mutex::new(listened),
            asked,
        };
        let at = socket.clone();
        std::thread::spawn(move || serve(&at, cli));
        socket
    }

    #[test]
    fn a_state_watch_hears_how_warp_stands_from_the_listener_alone() {
        let (lines, mut tell) = std::io::pipe().expect("a pipe opens");
        let asked = Arc::new(AtomicUsize::new(0));
        let socket = serving_counted(
            "state",
            Some(lines),
            Arc::new(AtomicBool::new(true)),
            Arc::clone(&asked),
        );
        let mut client = Client::new(&socket);
        client.say(&json!({"id": "all", "ask": "watch", "kind": "warp-state"}));
        assert_eq!(
            client.heard(),
            json!({"id": "all", "records": []}),
            "nothing heard yet is said as nothing"
        );
        let told = contract::file("listen-status.jsonl");
        let told: Vec<&str> = told.lines().collect();
        writeln!(tell, "{}", told[0]).expect("the daemon listens");
        let first = client.heard();
        let state = &first["changed"][0];
        assert_eq!(state["key"], "warp:state");
        assert_eq!(state["kind"], "warp-state");
        assert_eq!(
            state["said"],
            json!({"status": "Connected", "reason": "NetworkHealthy"})
        );
        assert_eq!(
            state["since_ms"],
            Value::Null,
            "a state heard first may have stood for hours, so it says no since"
        );
        writeln!(tell, "{}", told[1]).expect("the daemon listens");
        let degraded = client.heard();
        assert_eq!(
            degraded["changed"][0]["said"],
            json!({"status": "Connected", "reason": "NetworkDegraded"})
        );
        assert!(
            degraded["changed"][0]["since_ms"].is_u64(),
            "a state seen to begin says when: {degraded}"
        );
        // Its figures moving tell nothing, so the next heard is the drop.
        writeln!(tell, "{}", told[2]).expect("the daemon listens");
        writeln!(tell, "{}", told[3]).expect("the daemon listens");
        assert_eq!(
            client.heard()["changed"][0]["said"],
            json!({"status": "Disconnected", "reason": "ManualDisconnection"})
        );
        assert_eq!(
            asked.load(Ordering::SeqCst),
            0,
            "a state watch sets off none of the link's reads"
        );
    }

    #[test]
    fn a_status_asked_says_how_warp_stands_or_why_it_cant() {
        let asked = |ok, text: &str| {
            standing(Ok(Said {
                ok,
                text: text.into(),
            }))
        };
        assert_eq!(
            asked(true, &contract::file("status.degraded.json")),
            json!({"status": "Connected", "reason": "NetworkDegraded"})
        );
        assert_eq!(
            asked(false, &contract::file("daemon-down.json")),
            json!({"error": "FailedToConnectToDaemon"}),
            "WARP's service down is said by warp-cli's own code"
        );
        assert_eq!(asked(true, ""), json!({"error": "unreadable"}));
        assert_eq!(
            standing(Err(Unasked::Missing("warp-cli".into()))),
            json!({"error": "missing"})
        );
    }

    #[test]
    fn a_state_watch_is_told_the_status_asked_where_nothing_listens() {
        let socket = serving_listened("unlistened", None, Arc::new(AtomicBool::new(false)));
        let mut client = Client::new(&socket);
        client.say(&json!({"id": "all", "ask": "watch", "kind": ["warp-state"]}));
        assert_eq!(client.heard()["records"], json!([]));
        assert_eq!(
            client.heard()["changed"][0]["said"],
            json!({"status": "Disconnected", "reason": "Manual"})
        );
    }

    #[test]
    fn only_a_status_or_reason_other_than_the_last_sets_a_read_off() {
        let told = contract::file("listen-status.jsonl");
        let told: Vec<&str> = told.lines().collect();
        let mut last = None;
        assert!(
            !changed(told[0], &mut last),
            "the first says only how it stands"
        );
        assert!(!changed(told[0], &mut last), "the same again is no change");
        assert!(changed(told[1], &mut last), "a link degrading is a change");
        assert!(
            !changed(told[2], &mut last),
            "a degraded link's figures moving are the two-second reads'"
        );
        assert!(
            !changed("resolved example.com", &mut last),
            "a DNS log sets nothing off"
        );
        assert!(changed(told[3], &mut last), "a link going down is a change");
    }

    #[test]
    fn a_change_warp_tells_is_read_at_once() {
        let (lines, mut tell) = std::io::pipe().expect("a pipe opens");
        let up = Arc::new(AtomicBool::new(true));
        let mut client = Client::new(&serving_listened("listened", Some(lines), Arc::clone(&up)));
        client.say(&json!({"id": "all", "ask": "watch", "kind": "warp"}));
        assert_eq!(client.heard()["records"], json!([]));
        let first = client.heard();
        assert_eq!(first["changed"][0]["said"]["status"]["state"], "Connected");
        let told = contract::file("listen-status.jsonl");
        let told: Vec<&str> = told.lines().collect();
        writeln!(tell, "{}", told[0]).expect("the daemon listens");
        up.store(false, Ordering::SeqCst);
        writeln!(tell, "{}", told[3]).expect("the daemon listens");
        let next = client.heard();
        assert_eq!(
            next["changed"][0]["said"]["status"]["state"],
            "Disconnected"
        );
        let read_at = |told: &Value| told["changed"][0]["read_at_ms"].as_u64().unwrap_or(0);
        let between = read_at(&next).saturating_sub(read_at(&first));
        assert!(
            between < 1000,
            "the link is read as WARP tells it went down, not when the next read comes due \
             two seconds on, but {between} ms after the first"
        );
    }

    #[test]
    fn a_watch_hears_the_link_as_read_and_again_after_a_switch() {
        let mut client = Client::new(&serving("watch"));
        client.say(&json!({"id": "hello", "ask": "hello", "version": 1}));
        assert_eq!(client.heard()["domain"], "warp");
        client.say(&json!({"id": "all", "ask": "watch", "kind": "warp"}));
        assert_eq!(
            client.heard(),
            json!({"id": "all", "records": []}),
            "nothing read yet is said as nothing, not as an old link"
        );
        let first = client.heard();
        assert_eq!(first["changed"][0]["key"], "warp:link");
        assert_eq!(first["changed"][0]["said"]["status"]["state"], "Connected");
        client.say(&json!({"id": 2, "ask": "act", "act": "disconnect"}));
        // A read come due while switching may land before the answer.
        let answer = loop {
            let heard = client.heard();
            if heard["id"] == 2 {
                break heard;
            }
        };
        assert_eq!(answer["switched"]["act"], "disconnect");
        assert_eq!(answer["switched"]["done"], true);
        assert_eq!(
            client.heard()["changed"][0]["said"]["status"]["state"],
            "Disconnected",
            "the link is read again as the switch is answered"
        );
    }

    #[test]
    fn an_ask_kind_or_act_it_doesnt_know_is_refused() {
        let mut client = Client::new(&serving("refused"));
        client.say(&json!({"id": 1, "ask": "watch", "kind": "disk"}));
        assert_eq!(client.heard()["error"]["code"], "unknown-kind");
        client.say(&json!({"id": 2, "ask": "act", "act": "reconnect"}));
        assert_eq!(client.heard()["error"]["code"], "unknown-act");
        client.say(&json!({"id": 3, "ask": "overclock"}));
        assert_eq!(client.heard()["error"]["code"], "unknown-ask");
    }

    #[test]
    fn a_get_reads_afresh_what_isnt_held_fresh() {
        let mut client = Client::new(&serving("get"));
        client.say(&json!({"id": 1, "ask": "get", "keys": ["warp:link"]}));
        let got = client.heard();
        assert_eq!(got["id"], 1);
        assert_eq!(got["records"][0]["said"]["status"]["state"], "Connected");
    }
}
