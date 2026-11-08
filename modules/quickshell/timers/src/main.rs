//! timers SOCKET KEPT WORDS
//!
//! Joel's timers, kept and rung whether or not the shell is up to show
//! them, answered over SOCKET in the shell's daemon interface (lib.rs).
//! A `watch` says every timer, then each change as it happens; `start`,
//! `pause`, `resume`, `again`, `add` and `end` change one, each answered
//! with the timer as it now stands, or gone. Every change is kept at KEPT
//! at once, so a restart of the shell, of this daemon or of the machine
//! loses none.
//!
//! One clock waits for the soonest end: a timerfd on the time of day, set
//! to that moment, so a suspend or the clock being set never makes it
//! ring late or early, and nothing wakes while nothing runs. A timer
//! whose end passed rings as a notice over D-Bus, in the words WORDS
//! gives, critical so it stays until put away, with Again among its acts.
//! Putting the card away ends the timer; Again sets it going afresh. While
//! no notifications server answers, as while the shell starts again, the
//! notice waits and is said once one does.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::os::fd::{FromRawFd, OwnedFd};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use rustix::event::{EventfdFlags, PollFd, PollFlags, eventfd, poll};
use rustix::time::{
    Itimerspec, TimerfdClockId, TimerfdFlags, TimerfdTimerFlags, Timespec, timerfd_create,
    timerfd_settime,
};
use serde_json::{Value, json};
use timers::{Ask, Held, KIND, Refusal, Standing, Timer, VERSION, Words, ask, error, now};
use zbus::blocking::{Connection, MessageIterator};
use zbus::zvariant::Value as Variant;
use zbus::{MatchRule, message};

/// How long a notice waits before it's said again when no notifications
/// server took it.
const AGAIN: Duration = Duration::from_secs(3);

/// How long a call over D-Bus waits for its answer, since one otherwise
/// waits for ever, and a wedged server would hold every notice behind it.
const PATIENCE: Duration = Duration::from_secs(2);

const NOTIFICATIONS: &str = "org.freedesktop.Notifications";
const NOTIFICATIONS_PATH: &str = "/org/freedesktop/Notifications";

/// The act a notice carries to set its timer going again.
const AGAIN_ACT: &str = "again";

/// Why a notice closed, as the notifications specification numbers it:
/// put away by Joel.
const DISMISSED: u32 = 2;

/// One watch a client holds.
struct Watch {
    connection: u64,
    id: Value,
    told: Sender<String>,
}

/// Everything the threads share.
struct Daemon {
    held: Held,
    kept: PathBuf,
    watches: Vec<Watch>,
    /// The notice shown for each rung timer, by the id the server gave it.
    notices: HashMap<u32, String>,
    since: u64,
}

struct Shared {
    daemon: Mutex<Daemon>,
    /// Written to whenever an end may have moved, to wake the clock.
    wake: OwnedFd,
    /// What the notices thread is to say or take back.
    notices: Mutex<Sender<Tell>>,
}

/// What the notices thread does.
enum Tell {
    /// Rings the timer with this key, unless it has rung already.
    Ring(String),
    /// Takes back the notice with this id.
    Withdraw(u32),
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(socket), Some(kept), Some(words)) = (args.next(), args.next(), args.next()) else {
        eprintln!("usage: timers SOCKET KEPT WORDS");
        std::process::exit(64);
    };
    let words: Words = match std::fs::read_to_string(&words)
        .map_err(|why| why.to_string())
        .and_then(|text| serde_json::from_str(&text).map_err(|why| why.to_string()))
    {
        Ok(words) => words,
        Err(why) => {
            eprintln!("<3>couldn't read the words: {why}");
            std::process::exit(78);
        }
    };
    let socket = PathBuf::from(socket);
    let listener = match listen(&socket) {
        Ok(listener) => listener,
        Err(why) => {
            eprintln!("<3>couldn't listen on {}: {why}", socket.display());
            std::process::exit(71);
        }
    };
    let clock = match timerfd_create(
        TimerfdClockId::Realtime,
        TimerfdFlags::CLOEXEC | TimerfdFlags::NONBLOCK,
    ) {
        Ok(clock) => clock,
        Err(why) => {
            eprintln!("<3>couldn't make a clock: {why}");
            std::process::exit(71);
        }
    };
    let Ok(wake) = eventfd(0, EventfdFlags::CLOEXEC | EventfdFlags::NONBLOCK) else {
        eprintln!("<3>couldn't make the clock's wake");
        std::process::exit(71);
    };
    let kept = PathBuf::from(kept);
    let (tell, told) = mpsc::channel();
    let shared = Arc::new(Shared {
        daemon: Mutex::new(Daemon {
            held: Held::read(&kept),
            kept,
            watches: Vec::new(),
            notices: HashMap::new(),
            since: now(),
        }),
        wake,
        notices: Mutex::new(tell),
    });
    {
        // What ended while the daemon was down rings now, late, and what
        // rang but was never said is said.
        let mut daemon = lock(&shared);
        let rung = daemon.held.due(now());
        let changed: Vec<Value> = rung
            .iter()
            .filter_map(|key| daemon.held.get(key).map(Timer::json))
            .collect();
        changes(&shared, &mut daemon, &changed, &[]);
        for key in daemon.held.untold() {
            say(&shared, Tell::Ring(key));
        }
    }
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || keep_time(&shared, &clock));
    }
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || notify(&shared, &words, &told));
    }
    for (connection, stream) in (1u64..).zip(listener.incoming()) {
        let Ok(stream) = stream else { continue };
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || serve(&shared, stream, connection));
    }
}

/// The socket systemd holds for it (timers.socket), so the shell never
/// finds it missing while the daemon starts or starts again; or, run by
/// hand, its own, in a folder only Joel reads, replacing one left behind.
fn listen(socket: &std::path::Path) -> std::io::Result<UnixListener> {
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

fn lock(shared: &Shared) -> MutexGuard<'_, Daemon> {
    shared
        .daemon
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Hands the notices thread something to do.
fn say(shared: &Shared, tell: Tell) {
    let _ = shared
        .notices
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .send(tell);
}

/// After a change: kept at once, told to every watch, and the clock woken,
/// since the soonest end may have moved.
fn changes(shared: &Shared, daemon: &mut Daemon, changed: &[Value], gone: &[String]) {
    if changed.is_empty() && gone.is_empty() {
        return;
    }
    if let Err(why) = daemon.held.write(&daemon.kept) {
        eprintln!(
            "<3>couldn't keep the timers at {}: {why}",
            daemon.kept.display()
        );
    }
    daemon.watches.retain(|watch| {
        let line = json!({"id": watch.id, "changed": changed, "gone": gone}).to_string();
        watch.told.send(line).is_ok()
    });
    let _ = rustix::io::write(&shared.wake, &1u64.to_ne_bytes());
}

/// Waits for the soonest end and rings what's due, for as long as the
/// daemon runs.
fn keep_time(shared: &Shared, clock: &OwnedFd) {
    loop {
        let soonest = lock(shared).held.next_end();
        // Set to the time of day itself, so a suspend or the clock being
        // set is still woken at the end; and cancelled by the clock being
        // set, so the end is looked at again then.
        let at = soonest.map_or(
            Timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            |ends| Timespec {
                tv_sec: i64::try_from(ends / 1000).unwrap_or(i64::MAX),
                tv_nsec: i64::try_from(ends % 1000 * 1_000_000).unwrap_or(0),
            },
        );
        let armed = timerfd_settime(
            clock,
            TimerfdTimerFlags::ABSTIME | TimerfdTimerFlags::CANCEL_ON_SET,
            &Itimerspec {
                it_interval: Timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                },
                it_value: at,
            },
        );
        if let Err(why) = armed {
            eprintln!("<3>couldn't set the clock: {why}");
            std::thread::sleep(AGAIN);
            continue;
        }
        let mut waits = [
            PollFd::new(clock, PollFlags::IN),
            PollFd::new(&shared.wake, PollFlags::IN),
        ];
        if poll(&mut waits, None).is_err() {
            continue;
        }
        // Each read only empties what woke it: the clock's count, an
        // ECANCELED when the clock was set, or the wake's count.
        let mut count = [0u8; 8];
        let _ = rustix::io::read(clock, &mut count);
        let _ = rustix::io::read(&shared.wake, &mut count);
        let mut daemon = lock(shared);
        let rung = daemon.held.due(now());
        let changed: Vec<Value> = rung
            .iter()
            .filter_map(|key| daemon.held.get(key).map(Timer::json))
            .collect();
        changes(shared, &mut daemon, &changed, &[]);
        drop(daemon);
        for key in rung {
            say(shared, Tell::Ring(key));
        }
    }
}

/// Says each ring as a notice, and takes back each the timer no longer
/// needs, holding them until a notifications server answers.
fn notify(shared: &Arc<Shared>, words: &Words, told: &Receiver<Tell>) {
    let mut connection: Option<Connection> = None;
    let mut waiting: Vec<Tell> = Vec::new();
    loop {
        let next = if waiting.is_empty() {
            told.recv().map_err(|_| RecvTimeoutError::Disconnected)
        } else {
            told.recv_timeout(AGAIN)
        };
        match next {
            Ok(tell) => waiting.push(tell),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        if connection.is_none() {
            connection = connect(shared);
        }
        let Some(bus) = &connection else { continue };
        let mut still = Vec::new();
        for tell in waiting.drain(..) {
            let done = match &tell {
                Tell::Ring(key) => ring(shared, bus, words, key),
                Tell::Withdraw(id) => bus
                    .call_method(
                        Some(NOTIFICATIONS),
                        NOTIFICATIONS_PATH,
                        Some(NOTIFICATIONS),
                        "CloseNotification",
                        id,
                    )
                    .map(|_| ()),
            };
            if let Err(why) = done {
                eprintln!("<4>the notifications server didn't answer: {why}");
                still.push(tell);
            }
        }
        waiting = still;
    }
}

/// The session bus, with a thread hearing what Joel does with each notice.
fn connect(shared: &Arc<Shared>) -> Option<Connection> {
    let bus = zbus::blocking::connection::Builder::session()
        .and_then(|builder| builder.method_timeout(PATIENCE).build())
        .map_err(|why| eprintln!("<4>couldn't reach the session bus: {why}"))
        .ok()?;
    let heard = bus.clone();
    let shared = Arc::clone(shared);
    std::thread::spawn(move || hear(&shared, &heard));
    Some(bus)
}

/// Says one rung timer's notice, once.
fn ring(shared: &Shared, bus: &Connection, words: &Words, key: &str) -> zbus::Result<()> {
    let (summary, body) = {
        let daemon = lock(shared);
        match daemon.held.get(key) {
            Some(timer) if matches!(timer.standing, Standing::Rung { .. }) && !timer.told => {
                words.notice(timer)
            }
            _ => return Ok(()),
        }
    };
    let hints = HashMap::from([
        ("urgency", Variant::U8(2)),
        ("category", Variant::from("x-jitsusama.timer")),
    ]);
    let reply = bus.call_method(
        Some(NOTIFICATIONS),
        NOTIFICATIONS_PATH,
        Some(NOTIFICATIONS),
        "Notify",
        &(
            words.app.as_str(),
            0u32,
            "",
            summary.as_str(),
            body.as_str(),
            vec![AGAIN_ACT, words.again.as_str()],
            hints,
            0i32,
        ),
    )?;
    let id: u32 = reply.body().deserialize()?;
    let mut daemon = lock(shared);
    let ringing = daemon.held.timers.iter_mut().find(|timer| timer.key == key);
    if let Some(timer) = ringing
        && matches!(timer.standing, Standing::Rung { .. })
    {
        timer.told = true;
        daemon.notices.insert(id, key.to_owned());
        if let Err(why) = daemon.held.write(&daemon.kept) {
            eprintln!("<3>couldn't keep the timers: {why}");
        }
    } else {
        // Ended or started again while it was being said.
        drop(daemon);
        say(shared, Tell::Withdraw(id));
    }
    Ok(())
}

/// Hears each notice's act and its closing until the bus goes: Again sets
/// its timer going afresh, and putting it away ends a timer still rung.
fn hear(shared: &Shared, bus: &Connection) {
    let rule = || -> zbus::Result<MatchRule<'static>> {
        Ok(MatchRule::builder()
            .msg_type(message::Type::Signal)
            .interface(NOTIFICATIONS)?
            .path(NOTIFICATIONS_PATH)?
            .build())
    };
    let Ok(rule) = rule() else {
        return;
    };
    let Ok(messages) = MessageIterator::for_match_rule(rule, bus, None) else {
        return;
    };
    for message in messages {
        let Ok(message) = message else { break };
        let header = message.header();
        let member = header.member().map(|member| member.as_str().to_owned());
        match member.as_deref() {
            Some("ActionInvoked") => {
                let Ok((id, act)) = message.body().deserialize::<(u32, String)>() else {
                    continue;
                };
                let mut daemon = lock(shared);
                let Some(key) = daemon.notices.get(&id).cloned() else {
                    continue;
                };
                if act == AGAIN_ACT {
                    daemon.notices.remove(&id);
                    let changed: Vec<Value> = daemon
                        .held
                        .again(&key, now())
                        .map(Timer::json)
                        .into_iter()
                        .collect();
                    changes(shared, &mut daemon, &changed, &[]);
                }
            }
            Some("NotificationClosed") => {
                let Ok((id, why)) = message.body().deserialize::<(u32, u32)>() else {
                    continue;
                };
                let mut daemon = lock(shared);
                let Some(key) = daemon.notices.remove(&id) else {
                    continue;
                };
                let rung = daemon
                    .held
                    .get(&key)
                    .is_some_and(|timer| matches!(timer.standing, Standing::Rung { .. }));
                if why == DISMISSED && rung && daemon.held.end(&key).is_ok() {
                    changes(shared, &mut daemon, &[], &[key]);
                }
            }
            _ => {}
        }
    }
}

/// Answers one client until it goes.
fn serve(shared: &Shared, stream: UnixStream, connection: u64) {
    let Ok(reading) = stream.try_clone() else {
        return;
    };
    let (told, lines) = mpsc::channel::<String>();
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
        let answer = answer(shared, ask(&line), connection, &told);
        if told.send(answer.to_string()).is_err() {
            break;
        }
    }
    lock(shared)
        .watches
        .retain(|watch| watch.connection != connection);
}

/// What one ask is answered with.
fn answer(shared: &Shared, asked: Ask, connection: u64, told: &Sender<String>) -> Value {
    let mut daemon = lock(shared);
    let at = now();
    match asked {
        Ask::Hello { id } => json!({
            "id": id, "version": VERSION, "domain": "timers", "state": "answering",
            "since": daemon.since, "sources": {},
        }),
        Ask::Watch { id } => {
            // A watch asked again by its id replaces the one before.
            daemon
                .watches
                .retain(|watch| !(watch.connection == connection && watch.id == id));
            daemon.watches.push(Watch {
                connection,
                id: id.clone(),
                told: told.clone(),
            });
            let records: Vec<Value> = daemon.held.timers.iter().map(Timer::json).collect();
            json!({"id": id, "records": records})
        }
        Ask::Unwatch { id } => {
            daemon
                .watches
                .retain(|watch| !(watch.connection == connection && watch.id == id));
            json!({"id": id, "unwatched": true})
        }
        Ask::Get { id, keys } => {
            let records: Vec<Value> = keys
                .iter()
                .filter_map(|key| daemon.held.get(key))
                .map(Timer::json)
                .collect();
            json!({"id": id, "records": records})
        }
        Ask::Start { id, name, span } => {
            let started = daemon.held.start(&name, span, at).map(Timer::json);
            changed(shared, &mut daemon, &id, started)
        }
        Ask::Pause { id, key } => {
            let paused = daemon.held.pause(&key, at).map(Timer::json);
            changed(shared, &mut daemon, &id, paused)
        }
        Ask::Resume { id, key } => {
            let resumed = daemon.held.resume(&key, at).map(Timer::json);
            changed(shared, &mut daemon, &id, resumed)
        }
        Ask::Again { id, key } => {
            withdraw(shared, &mut daemon, &key);
            let again = daemon.held.again(&key, at).map(Timer::json);
            changed(shared, &mut daemon, &id, again)
        }
        Ask::Add { id, key, span } => {
            withdraw(shared, &mut daemon, &key);
            let added = daemon.held.add(&key, span, at).map(Timer::json);
            changed(shared, &mut daemon, &id, added)
        }
        Ask::End { id, key } => match daemon.held.end(&key) {
            Ok(_) => {
                withdraw(shared, &mut daemon, &key);
                let gone = [key];
                changes(shared, &mut daemon, &[], &gone);
                json!({"id": id, "gone": gone})
            }
            Err(refusal) => refusal.json(&id),
        },
        Ask::UnknownKind { id, kind } => error(
            &id,
            "unknown-kind",
            &format!("this daemon keeps only {KIND}, not {kind}"),
        ),
        Ask::Malformed { id, why } => error(&id, "malformed", why),
        Ask::Unknown { id, ask } => error(&id, "unknown-ask", &format!("no ask {ask:?}")),
    }
}

/// The answer to an ask that changed one timer, told to every watch too,
/// or why it changed nothing.
fn changed(
    shared: &Shared,
    daemon: &mut Daemon,
    id: &Value,
    outcome: Result<Value, Refusal>,
) -> Value {
    match outcome {
        Ok(record) => {
            changes(shared, daemon, std::slice::from_ref(&record), &[]);
            json!({"id": id, "records": [record]})
        }
        Err(refusal) => refusal.json(id),
    }
}

/// Takes back a timer's notice, should it have one up.
fn withdraw(shared: &Shared, daemon: &mut Daemon, key: &str) {
    let shown: Vec<u32> = daemon
        .notices
        .iter()
        .filter(|(_, held)| held.as_str() == key)
        .map(|(id, _)| *id)
        .collect();
    for id in shown {
        daemon.notices.remove(&id);
        say(shared, Tell::Withdraw(id));
    }
}
