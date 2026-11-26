//! A timer's ring: its notice said over D-Bus to whichever notifications
//! server answers, kept with the server that took it, said again to one
//! that takes a lost one's place, withdrawn as the timer ends, and heard
//! aloud once near its end; and the cards' acts and dismissals heard
//! back.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

use crate::{AGAIN, Daemon, Shared, Tell, changes, lock, say};
use serde_json::Value;
use timers::{Notice, Standing, Timer, Words, now, on_bus, unsaid};
use zbus::blocking::{Connection, MessageIterator};
use zbus::zvariant::Value as Variant;
use zbus::{MatchRule, message};

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

/// Plays the ring once, for every timer that came due together within a
/// minute of its end (`Held::heard`), in a
/// thread of its own, since asking whether a call is on takes a couple of
/// seconds and no notice waits on it.
pub(crate) fn aloud(shared: &Shared) {
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
pub(crate) fn notify(shared: &Arc<Shared>, words: &Words, told: &Receiver<Tell>) {
    // The bus, with its id as connect() asked it, which every notice said
    // on it is known by.
    let mut connection: Option<(Connection, String)> = None;
    // How many times the bus was dialled, so a listener of one since
    // given up that stops is told apart from the one listening now.
    let mut dialled = 0;
    // Whether the bus was lost, and whether it turned the daemon away,
    // each said once as it starts and once as it ends, not each try.
    let mut lost = false;
    let mut refused = false;
    let mut waiting: Vec<Tell> = Vec::new();
    loop {
        let next = if waiting.is_empty() && !lost {
            told.recv().map_err(|_| RecvTimeoutError::Disconnected)
        } else {
            told.recv_timeout(AGAIN)
        };
        match next {
            Ok(Tell::Deaf(stopped)) => {
                if stopped == dialled && connection.take().is_some() {
                    eprintln!("<4>stopped hearing the session bus, so it's dialled again");
                    lost = true;
                }
            }
            Ok(tell) => waiting.push(tell),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        if connection.is_none() && (lost || !waiting.is_empty()) {
            match connect(shared, dialled + 1) {
                Ok(bus) => {
                    dialled += 1;
                    if lost || refused {
                        eprintln!("<6>hears the session bus again, so what waited is said");
                    }
                    (lost, refused) = (false, false);
                    connection = Some(bus);
                    // Every rung timer, since what was said before may
                    // have gone with a server; each says itself again
                    // only where it isn't up.
                    waiting.extend(rung(shared).into_iter().map(Tell::Resay));
                }
                Err(why) if !refused => {
                    eprintln!("<4>{why}, so rings wait until it can");
                    refused = true;
                }
                Err(_) => {}
            }
        }
        let Some((bus, id)) = &connection else {
            continue;
        };
        let mut still = Vec::new();
        for tell in waiting.drain(..) {
            let done = match &tell {
                Tell::Ring(key) => ring(shared, (bus, id), words, key, false),
                Tell::Resay(key) => ring(shared, (bus, id), words, key, true),
                Tell::Withdraw(notice) => withdrawn((bus, id), notice),
                Tell::Deaf(_) => Ok(()),
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
/// Both are listening before it's returned, since its first notice is
/// said at once: a rule added in its own thread came after the notice
/// under load, and a press on the card that came sooner went unheard. A
/// rule the bus turns away, as one answering late under load does, gives
/// no bus, so the notices wait and it's tried again: a notice shown with
/// nothing hearing it would keep an Again that did nothing for the rest of
/// the daemon's life.
/// Each listener, as it stops, says so with `dialled`, the count of this
/// connection, for the bus to be dialled again.
fn connect(shared: &Arc<Shared>, dialled: u64) -> Result<(Connection, String), String> {
    let bus = zbus::blocking::connection::Builder::session()
        .and_then(|builder| builder.method_timeout(PATIENCE).build())
        .map_err(|why| format!("couldn't reach the session bus: {why}"))?;
    let listening = |rule: zbus::Result<MatchRule<'static>>, what: &str| {
        rule.and_then(|rule| MessageIterator::for_match_rule(rule, &bus, None))
            .map_err(|why| format!("couldn't hear {what} on the session bus: {why}"))
    };
    let acted = listening(acts(), "what Joel does with a notice")?;
    let followed = listening(servers(), "which server takes the notices")?;
    let id = bus_id(&bus).map_err(|why| format!("couldn't ask the session bus its id: {why}"))?;
    let heard = Arc::clone(shared);
    let heard_on = id.clone();
    std::thread::spawn(move || {
        hear(&heard, acted, &heard_on);
        say(&heard, Tell::Deaf(dialled));
    });
    let shared = Arc::clone(shared);
    std::thread::spawn(move || {
        follow(&shared, followed);
        say(&shared, Tell::Deaf(dialled));
    });
    Ok((bus, id))
}

/// The notifications server changing hands.
fn servers() -> zbus::Result<MatchRule<'static>> {
    Ok(MatchRule::builder()
        .msg_type(message::Type::Signal)
        .sender(BUS)?
        .interface(BUS)?
        .member("NameOwnerChanged")?
        .add_arg(NOTIFICATIONS)?
        .build())
}

/// What any notifications server says of its notices.
fn acts() -> zbus::Result<MatchRule<'static>> {
    Ok(MatchRule::builder()
        .msg_type(message::Type::Signal)
        .interface(NOTIFICATIONS)?
        .path(NOTIFICATIONS_PATH)?
        .build())
}

/// The keys of every rung timer.
pub(crate) fn rung(shared: &Shared) -> Vec<String> {
    lock(shared)
        .held
        .timers
        .iter()
        .filter(|timer| matches!(timer.standing, Standing::Rung { .. }))
        .map(|timer| timer.key.clone())
        .collect()
}

/// The bus's own id, which no other bus has: one begun again, as after a
/// login, names its connections from the start, so a name alone could be
/// a server gone with the last bus.
fn bus_id(bus: &Connection) -> zbus::Result<String> {
    bus.call_method(Some(BUS), BUS_PATH, Some(BUS), "GetId", &())?
        .body()
        .deserialize()
}

/// The notifications server there is now, as `Notice::server` holds it:
/// the bus's id, as `connect()` asked it, and the server's name on it.
fn server((bus, id): (&Connection, &str)) -> zbus::Result<String> {
    let name: String = bus
        .call_method(
            Some(BUS),
            BUS_PATH,
            Some(BUS),
            "GetNameOwner",
            &NOTIFICATIONS,
        )?
        .body()
        .deserialize()?;
    Ok(on_bus(id, &name))
}

/// Takes a notice back, from the server that gave it alone, since its id
/// may name another notice on a server after it.
fn withdrawn((bus, id): (&Connection, &str), notice: &Notice) -> zbus::Result<()> {
    if server((bus, id)).ok().as_deref() != Some(notice.server.as_str()) {
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
/// unless it's up there already; `again`, in the place of the one it gave,
/// since a press or putting it away while the bus wasn't heard is lost.
/// Only a card that server gave is replaced, as the id is only its own.
fn ring(
    shared: &Shared,
    (bus, id): (&Connection, &str),
    words: &Words,
    key: &str,
    again: bool,
) -> zbus::Result<()> {
    let server = server((bus, id))?;
    let (summary, body, replaces) = {
        let daemon = lock(shared);
        match daemon.held.get(key) {
            Some(timer) if unsaid(timer, &server) => {
                let (summary, body) = words.notice(timer);
                (summary, body, 0)
            }
            Some(timer) if again && matches!(timer.standing, Standing::Rung { .. }) => {
                let (summary, body) = words.notice(timer);
                (
                    summary,
                    body,
                    timer.notice.as_ref().map_or(0, |notice| notice.id),
                )
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
            replaces,
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
fn follow(shared: &Shared, messages: MessageIterator) {
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
fn hear(shared: &Shared, messages: MessageIterator, bus: &str) {
    for message in messages {
        let Ok(message) = message else { break };
        let header = message.header();
        let member = header.member().map(|member| member.as_str().to_owned());
        // Which server said it, on which bus, since a notice's id is only
        // its own.
        let sender = on_bus(
            bus,
            header
                .sender()
                .map(zbus::names::UniqueName::as_str)
                .unwrap_or_default(),
        );
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

/// A timer's notice, should it have one up.
pub(crate) fn notice(daemon: &Daemon, key: &str) -> Option<Notice> {
    daemon.held.get(key).and_then(|timer| timer.notice.clone())
}

/// Takes back a timer's notice once what was asked of it was done.
pub(crate) fn withdraw(shared: &Shared, shown: Option<Notice>, done: bool) {
    if let Some(notice) = shown
        && done
    {
        say(shared, Tell::Withdraw(notice));
    }
}
