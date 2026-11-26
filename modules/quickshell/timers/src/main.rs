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
//!
//! Here are its start, the clock and what's kept; `ringing` says the
//! notices and hears their acts, and `answering` answers the socket.

mod answering;
mod ringing;

use std::os::fd::OwnedFd;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use crate::answering::serve;
use crate::ringing::{aloud, notify, rung};
use rustix::event::{EventfdFlags, PollFd, PollFlags, eventfd, poll};
use rustix::time::{
    Itimerspec, TimerfdClockId, TimerfdFlags, TimerfdTimerFlags, Timespec, timerfd_create,
    timerfd_settime,
};
use serde_json::{Value, json};
use terminal_kit::{Exit, Failure, Help};
use timers::{Held, KIND, Notice, Timer, Words, now, watched};

/// How long a notice waits before it's said again when no notifications
/// server took it.
const AGAIN: Duration = Duration::from_secs(3);

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
    /// Says the timer's ring again once the bus is dialled again: its card
    /// in its own place, should it still be up, since what Joel did with
    /// it meanwhile went unheard, or afresh, should it be gone.
    Resay(String),
    /// Takes back this notice, should its server still be there.
    Withdraw(Notice),
    /// A listener of the bus dialled this many times over stopped, as one
    /// does when the bus goes, so nothing hears Joel's presses on it.
    Deaf(u64),
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
    let listener = daemon_serve::serve::listen(socket).map_err(|why| {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::answering::answer;
    use timers::{STOPWATCH, ask};

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
