//! timers SOCKET KEPT WORDS [PLAYER SOUND CALL]
//!
//! Joel's timers, kept and rung whether or not the shell is up to show
//! them, answered over SOCKET in the shell's daemon interface (lib.rs).
//! A `watch` says every timer, then each change as it happens; `start`,
//! `pause`, `resume`, `again`, `add` and `end` change one, each answered
//! with the timer as it now stands, or gone. The one stopwatch is a record
//! of its own kind, `stopwatch`, which `run`, `stop`, `lap` and `reset`
//! change; it rings nothing, so only what's kept and told is its own.
//! A watch hears the kinds it asked for. Every change is kept at KEPT
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
//! notice waits and is said once one does. Each notice is kept on its
//! timer with the server that took it, so a daemon started again still
//! knows its card, and a card lost with a server that went, as the
//! shell's does on a switch, is said again to the one that takes its place.
//!
//! Given PLAYER, SOUND and CALL, a timer coming due within a minute of its
//! end is heard too, once, however often its notice is said; one found
//! long past it, after a reboot or a suspend, is only said, late. PLAYER
//! (PipeWire's pw-play) plays
//! SOUND, at a third of its volume while CALL (a-call-is-on) says a call
//! is on, so a ring never drowns one out.
//!
//! Its unit alone starts it, in the terminal kit's shape, as
//! `timers --help` says (`help()`).

use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::{BufRead, BufReader, Write};
use std::os::fd::{FromRawFd, OwnedFd};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use rustix::event::{EventfdFlags, PollFd, PollFlags, eventfd, poll};
use rustix::time::{
    Itimerspec, TimerfdClockId, TimerfdFlags, TimerfdTimerFlags, Timespec, timerfd_create,
    timerfd_settime,
};
use serde_json::{Value, json};
use terminal_kit::{Exit, Failure, Help};
use timers::{
    Ask, Held, KIND, Notice, Refusal, STOPWATCH, Standing, Stopwatch, Timer, VERSION, Words, ask,
    error, now, unsaid, watched,
};
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
const BUS: &str = "org.freedesktop.DBus";
const BUS_PATH: &str = "/org/freedesktop/DBus";

/// How loud a ring plays while a call is on, as pw-play takes it.
const QUIETLY: &str = "--volume=0.3";

/// The act a notice carries to set its timer going again.
const AGAIN_ACT: &str = "again";

/// Why a notice closed, as the notifications specification numbers it:
/// put away by Joel.
const DISMISSED: u32 = 2;

/// One watch a client holds.
struct Watch {
    connection: u64,
    id: Value,
    /// The kinds it asked for, every kind when none.
    kinds: Vec<String>,
    told: Sender<String>,
}

/// Everything the threads share.
struct Daemon {
    held: Held,
    kept: PathBuf,
    watches: Vec<Watch>,
    since: u64,
}

struct Shared {
    daemon: Mutex<Daemon>,
    /// Written to whenever an end may have moved, to wake the clock.
    wake: OwnedFd,
    /// What the notices thread is to say or take back.
    notices: Mutex<Sender<Tell>>,
    /// How a timer coming due is heard, when it's heard at all.
    aloud: Option<Arc<Aloud>>,
}

/// What plays a ring aloud, and what says whether to play it quietly.
struct Aloud {
    player: PathBuf,
    sound: PathBuf,
    call: PathBuf,
}

/// What the notices thread does.
enum Tell {
    /// Rings the timer with this key, unless its ring is up already.
    Ring(String),
    /// Takes back this notice, should its server still be there.
    Withdraw(Notice),
}

/// What `timers --help` says: how its unit runs it, and how it ends.
fn help() -> Help {
    Help::new("timers", "keeps Joel's timers and rings each as a notice")
        .example(
            "timers SOCKET KEPT WORDS",
            "as its unit runs it, rung silently",
        )
        .example(
            "timers SOCKET KEPT WORDS PLAYER SOUND CALL",
            "each rung aloud too, quieter in a call",
        )
        .argument("<socket>", "where the shell asks, unless systemd hands one")
        .argument("<kept>", "the file its timers are kept in")
        .argument("<words>", "the launcher's words, which its notices say")
        .argument("[player]", "what plays a ring, pw-play")
        .argument("[sound]", "the ring it plays")
        .argument("[call]", "a-call-is-on, which quietens it")
        .exit(Exit::Failed, "its socket or its clock couldn't be made")
        .exit(Exit::Config, "its words couldn't be read")
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        // The kit catches a panic for as long as its body runs, which is
        // the daemon's whole life, so its threads say theirs as Rust does,
        // to the journal, rather than into the kit's keeping.
        drop(std::panic::take_hook());
        let asked = asked(&context.args)?;
        keep(&asked.socket, asked.kept, &asked.words, asked.aloud)
    })
}

/// What its unit hands it.
struct Asked {
    socket: PathBuf,
    kept: PathBuf,
    words: PathBuf,
    aloud: Option<Arc<Aloud>>,
}

/// Its socket, its file, its words, and how a ring is heard, from what
/// its unit hands it.
fn asked(args: &[String]) -> Result<Asked, Failure> {
    let path = |at: usize| PathBuf::from(&args[at]);
    let aloud = match args.len() {
        3 => None,
        6 => Some(Arc::new(Aloud {
            player: path(3),
            sound: path(4),
            call: path(5),
        })),
        _ => {
            return Err(Failure::new(
                Exit::Usage,
                "timers takes a socket, its file and its words, then a player, a ring and \
                 a-call-is-on, or nothing more",
            )
            .try_this("timers --help"));
        }
    };
    Ok(Asked {
        socket: path(0),
        kept: path(1),
        words: path(2),
        aloud,
    })
}

/// Keeps the timers and answers the socket for as long as it runs.
fn keep(
    socket: &std::path::Path,
    kept: PathBuf,
    words: &std::path::Path,
    sound: Option<Arc<Aloud>>,
) -> Result<(), Failure> {
    let words: Words = std::fs::read_to_string(words)
        .map_err(|why| why.to_string())
        .and_then(|text| serde_json::from_str(&text).map_err(|why| why.to_string()))
        .map_err(|why| {
            Failure::new(Exit::Config, "its words couldn't be read")
                .because(format!("{}: {why}", words.display()))
        })?;
    let listener = listen(socket).map_err(|why| {
        Failure::new(
            Exit::Failed,
            format!("it couldn't listen on {}", socket.display()),
        )
        .because(why.to_string())
    })?;
    let made = |what: &str| {
        let what = what.to_owned();
        move |why: rustix::io::Errno| {
            Failure::new(Exit::Failed, format!("it couldn't make {what}")).because(why.to_string())
        }
    };
    let clock = timerfd_create(
        TimerfdClockId::Realtime,
        TimerfdFlags::CLOEXEC | TimerfdFlags::NONBLOCK,
    )
    .map_err(made("a clock"))?;
    let wake = eventfd(0, EventfdFlags::CLOEXEC | EventfdFlags::NONBLOCK)
        .map_err(made("the clock's wake"))?;
    let (tell, told) = mpsc::channel();
    let shared = Arc::new(Shared {
        daemon: Mutex::new(Daemon {
            held: Held::read(&kept),
            kept,
            watches: Vec::new(),
            since: now(),
        }),
        wake,
        notices: Mutex::new(tell),
        aloud: sound,
    });
    {
        // What ended while the daemon was down rings now, late, and every
        // ring is said where it isn't up, once the notifications server
        // is reached, which the first of them sets going.
        let mut daemon = lock(&shared);
        let at = now();
        let heard = daemon.held.heard(at);
        let due = daemon.held.due(at);
        let changed: Vec<Value> = due
            .iter()
            .filter_map(|key| daemon.held.get(key).map(Timer::json))
            .collect();
        changes(&shared, &mut daemon, &changed, &[]);
        drop(daemon);
        if heard {
            aloud(&shared);
        }
        for key in rung(&shared) {
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
    Ok(())
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
        let kind_of = |record: &&Value| record["kind"].as_str().unwrap_or(KIND).to_owned();
        let theirs: Vec<&Value> = changed
            .iter()
            .filter(|record| watched(&watch.kinds, &kind_of(record)))
            .collect();
        // Only timers go, so only a watch of timers hears it.
        let gone: &[String] = if watched(&watch.kinds, KIND) {
            gone
        } else {
            &[]
        };
        if theirs.is_empty() && gone.is_empty() {
            return true;
        }
        let line = json!({"id": watch.id, "changed": theirs, "gone": gone}).to_string();
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
        let at = now();
        let heard = daemon.held.heard(at);
        let rung = daemon.held.due(at);
        let changed: Vec<Value> = rung
            .iter()
            .filter_map(|key| daemon.held.get(key).map(Timer::json))
            .collect();
        changes(shared, &mut daemon, &changed, &[]);
        drop(daemon);
        if heard {
            aloud(shared);
        }
        for key in rung {
            say(shared, Tell::Ring(key));
        }
    }
}

/// Plays the ring once, for every timer that came due together within a
/// minute of its end (`Held::heard`), in a
/// thread of its own, since asking whether a call is on takes a couple of
/// seconds and no notice waits on it.
fn aloud(shared: &Shared) {
    let Some(sound) = shared.aloud.clone() else {
        return;
    };
    std::thread::spawn(move || {
        let quietly = Command::new(&sound.call)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        let mut player = Command::new(&sound.player);
        if quietly {
            player.arg(QUIETLY);
        }
        match player
            .arg(&sound.sound)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .status()
        {
            Ok(status) if status.success() => {}
            Ok(status) => eprintln!("<4>the ring's player ended {status}"),
            Err(why) => eprintln!("<4>couldn't play the ring: {why}"),
        }
    });
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
            if connection.is_some() {
                // Every rung timer, since what was said before may have
                // gone with a server; each says itself again only where
                // it isn't up.
                waiting.extend(rung(shared).into_iter().map(Tell::Ring));
            }
        }
        let Some(bus) = &connection else { continue };
        let mut still = Vec::new();
        for tell in waiting.drain(..) {
            let done = match &tell {
                Tell::Ring(key) => ring(shared, bus, words, key),
                Tell::Withdraw(notice) => withdrawn(bus, notice),
            };
            if let Err(why) = done {
                eprintln!("<4>the notifications server didn't answer: {why}");
                still.push(tell);
            }
        }
        waiting = still;
    }
}

/// The session bus, with threads hearing what Joel does with each notice
/// and whether another notifications server takes the place of the last.
fn connect(shared: &Arc<Shared>) -> Option<Connection> {
    let bus = zbus::blocking::connection::Builder::session()
        .and_then(|builder| builder.method_timeout(PATIENCE).build())
        .map_err(|why| eprintln!("<4>couldn't reach the session bus: {why}"))
        .ok()?;
    {
        let heard = bus.clone();
        let shared = Arc::clone(shared);
        std::thread::spawn(move || hear(&shared, &heard));
    }
    {
        let heard = bus.clone();
        let shared = Arc::clone(shared);
        std::thread::spawn(move || follow(&shared, &heard));
    }
    Some(bus)
}

/// The keys of every rung timer.
fn rung(shared: &Shared) -> Vec<String> {
    lock(shared)
        .held
        .timers
        .iter()
        .filter(|timer| matches!(timer.standing, Standing::Rung { .. }))
        .map(|timer| timer.key.clone())
        .collect()
}

/// The bus name of the notifications server there is now.
fn server(bus: &Connection) -> zbus::Result<String> {
    bus.call_method(
        Some(BUS),
        BUS_PATH,
        Some(BUS),
        "GetNameOwner",
        &NOTIFICATIONS,
    )?
    .body()
    .deserialize()
}

/// Takes a notice back, from the server that gave it alone, since its id
/// may name another notice on a server after it.
fn withdrawn(bus: &Connection, notice: &Notice) -> zbus::Result<()> {
    if server(bus).ok().as_deref() != Some(notice.server.as_str()) {
        return Ok(());
    }
    bus.call_method(
        Some(NOTIFICATIONS),
        NOTIFICATIONS_PATH,
        Some(NOTIFICATIONS),
        "CloseNotification",
        &notice.id,
    )
    .map(|_| ())
}

/// Says one rung timer's notice to the notifications server there is now,
/// unless it's up there already.
fn ring(shared: &Shared, bus: &Connection, words: &Words, key: &str) -> zbus::Result<()> {
    let server = server(bus)?;
    let (summary, body) = {
        let daemon = lock(shared);
        match daemon.held.get(key) {
            Some(timer) if unsaid(timer, &server) => words.notice(timer),
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
    let notice = Notice { id, server };
    if let Some(timer) = ringing
        && matches!(timer.standing, Standing::Rung { .. })
    {
        timer.notice = Some(notice);
        if let Err(why) = daemon.held.write(&daemon.kept) {
            eprintln!("<3>couldn't keep the timers: {why}");
        }
    } else {
        // Ended or started again while it was being said.
        drop(daemon);
        say(shared, Tell::Withdraw(notice));
    }
    Ok(())
}

/// Hears the notifications server change hands until the bus goes: one
/// that takes its place has none of the last one's cards, so every rung
/// timer is said to it.
fn follow(shared: &Shared, bus: &Connection) {
    let rule = || -> zbus::Result<MatchRule<'static>> {
        Ok(MatchRule::builder()
            .msg_type(message::Type::Signal)
            .sender(BUS)?
            .interface(BUS)?
            .member("NameOwnerChanged")?
            .add_arg(NOTIFICATIONS)?
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
        let Ok((_, _, owner)) = message.body().deserialize::<(String, String, String)>() else {
            continue;
        };
        if owner.is_empty() {
            continue;
        }
        let mut said = String::new();
        for key in rung(shared) {
            let _ = write!(said, " {key}");
            say(shared, Tell::Ring(key));
        }
        if !said.is_empty() {
            eprintln!("<6>{owner} took the notices, so these are said again:{said}");
        }
    }
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
        // Which server said it, since a notice's id is only its own.
        let sender = header
            .sender()
            .map(|sender| sender.as_str().to_owned())
            .unwrap_or_default();
        match member.as_deref() {
            Some("ActionInvoked") => {
                let Ok((id, act)) = message.body().deserialize::<(u32, String)>() else {
                    continue;
                };
                let mut daemon = lock(shared);
                let Some(key) = daemon
                    .held
                    .noticed(id, &sender)
                    .map(|timer| timer.key.clone())
                else {
                    continue;
                };
                if act == AGAIN_ACT {
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
                // Only Joel putting a rung one's card away ends it; taken
                // back or let go any other way, it stays rung.
                let Some(key) = daemon
                    .held
                    .noticed(id, &sender)
                    .map(|timer| timer.key.clone())
                else {
                    continue;
                };
                if why == DISMISSED && daemon.held.end(&key).is_ok() {
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
        let Some(answer) = answer(shared, ask(&line), connection, &told) else {
            continue;
        };
        if told.send(answer.to_string()).is_err() {
            break;
        }
    }
    lock(shared)
        .watches
        .retain(|watch| watch.connection != connection);
}

/// What one ask is answered with, or nothing where it's already told, as a
/// watch's answer is.
fn answer(shared: &Shared, asked: Ask, connection: u64, told: &Sender<String>) -> Option<Value> {
    let mut daemon = lock(shared);
    let at = now();
    Some(match asked {
        Ask::Hello { id } => json!({
            "id": id, "version": VERSION, "domain": "timers", "state": "answering",
            "since": daemon.since, "sources": {},
        }),
        Ask::Watch { id, kinds } => {
            watch(&mut daemon, connection, told, id, kinds);
            return None;
        }
        Ask::Unwatch { id } => {
            daemon
                .watches
                .retain(|watch| !(watch.connection == connection && watch.id == id));
            json!({"id": id, "unwatched": true})
        }
        Ask::Get { id, keys } => got(&daemon, &id, &keys),
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
            let shown = notice(&daemon, &key);
            let again = daemon.held.again(&key, at).map(Timer::json);
            withdraw(shared, shown, again.is_ok());
            changed(shared, &mut daemon, &id, again)
        }
        Ask::Add { id, key, span } => {
            let shown = notice(&daemon, &key);
            let added = daemon.held.add(&key, span, at).map(Timer::json);
            withdraw(shared, shown, added.is_ok());
            changed(shared, &mut daemon, &id, added)
        }
        Ask::End { id, key } => match daemon.held.end(&key) {
            Ok(ended) => {
                withdraw(shared, ended.notice, true);
                let gone = [key];
                changes(shared, &mut daemon, &[], &gone);
                json!({"id": id, "gone": gone})
            }
            Err(refusal) => refusal.json(&id),
        },
        Ask::Run { id } => {
            let ran = daemon.held.stopwatch.run(at).map(Stopwatch::json);
            changed(shared, &mut daemon, &id, ran)
        }
        Ask::Stop { id } => {
            let stopped = daemon.held.stopwatch.stop(at).map(Stopwatch::json);
            changed(shared, &mut daemon, &id, stopped)
        }
        Ask::Lap { id } => {
            let lapped = daemon.held.stopwatch.lap(at).map(Stopwatch::json);
            changed(shared, &mut daemon, &id, lapped)
        }
        Ask::Reset { id } => {
            let reset = daemon.held.stopwatch.reset().json();
            changed(shared, &mut daemon, &id, Ok(reset))
        }
        Ask::UnknownKind { id, kind } => error(
            &id,
            "unknown-kind",
            &format!("this daemon keeps only {KIND} and {STOPWATCH}, not {kind}"),
        ),
        Ask::Malformed { id, why } => error(&id, "malformed", why),
        Ask::Unknown { id, ask } => error(&id, "unknown-ask", &format!("no ask {ask:?}")),
    })
}

/// A watch begun: every record of the kinds it asked for, and every change
/// to them told it from now on. A watch asked again by its id replaces the
/// one before.
fn watch(
    daemon: &mut Daemon,
    connection: u64,
    told: &Sender<String>,
    id: Value,
    kinds: Vec<String>,
) {
    daemon
        .watches
        .retain(|watch| !(watch.connection == connection && watch.id == id));
    let mut records: Vec<Value> = Vec::new();
    if watched(&kinds, KIND) {
        records.extend(daemon.held.timers.iter().map(Timer::json));
    }
    if watched(&kinds, STOPWATCH) {
        records.push(daemon.held.stopwatch.json());
    }
    // It's told under the lock that holds the watch, so a timer that rings
    // once the lock is let go is told after it. Told after the lock was let
    // go, a ring could be said first, and the whole answer after it would
    // put back what the ring moved on from. Where it can't be told, the
    // connection's writer has gone, and so has the connection.
    let _ = told.send(json!({"id": id, "records": records}).to_string());
    daemon.watches.push(Watch {
        connection,
        id,
        kinds,
        told: told.clone(),
    });
}

/// The records asked for by key, the stopwatch's included, passing over a
/// key nothing holds.
fn got(daemon: &Daemon, id: &Value, keys: &[String]) -> Value {
    let records: Vec<Value> = keys
        .iter()
        .filter_map(|key| {
            if key == STOPWATCH {
                Some(daemon.held.stopwatch.json())
            } else {
                daemon.held.get(key).map(Timer::json)
            }
        })
        .collect();
    json!({"id": id, "records": records})
}

/// The answer to an ask that changed one record, told to every watch too,
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

/// A timer's notice, should it have one up.
fn notice(daemon: &Daemon, key: &str) -> Option<Notice> {
    daemon.held.get(key).and_then(|timer| timer.notice.clone())
}

/// Takes back a timer's notice once what was asked of it was done.
fn withdraw(shared: &Shared, shown: Option<Notice>, done: bool) {
    if let Some(notice) = shown
        && done
    {
        say(shared, Tell::Withdraw(notice));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_watch_is_answered_whole_before_any_change_told_it() {
        let wake = eventfd(0, EventfdFlags::CLOEXEC | EventfdFlags::NONBLOCK).expect("a wake");
        let (tell, _notices) = mpsc::channel();
        let kept = std::env::temp_dir().join(format!("timers-{}.json", std::process::id()));
        let shared = Shared {
            daemon: Mutex::new(Daemon {
                held: Held::default(),
                kept: kept.clone(),
                watches: Vec::new(),
                since: now(),
            }),
            wake,
            notices: Mutex::new(tell),
            aloud: None,
        };
        let (told, lines) = mpsc::channel::<String>();
        let asked = ask(r#"{"id":"all","ask":"watch","kind":["stopwatch"]}"#);
        let first = answer(&shared, asked, 1, &told);
        // A change made once the answer's lock is let go, as a ring is.
        let ran = json!({"key": STOPWATCH, "kind": STOPWATCH, "running": true});
        changes(&shared, &mut lock(&shared), &[ran], &[]);
        if let Some(first) = first {
            told.send(first.to_string()).expect("told");
        }
        let said: Vec<Value> = lines
            .try_iter()
            .map(|line| serde_json::from_str(&line).expect("json"))
            .collect();
        let _ = std::fs::remove_file(&kept);
        assert_eq!(said.len(), 2, "{said:?}");
        assert!(said[0]["records"].is_array(), "{said:?}");
        assert_eq!(said[1]["changed"][0]["running"], true, "{said:?}");
    }
}
