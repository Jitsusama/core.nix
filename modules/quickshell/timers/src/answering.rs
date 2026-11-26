//! The answers over the socket: each connection's asks read a line at a
//! time, a watch answered whole before any change is told it, and the
//! records a get or a watch names.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::sync::mpsc::{self, Sender};

use crate::ringing::{notice, withdraw};
use crate::{Daemon, Shared, Watch, changes, lock};
use serde_json::{Value, json};
use timers::{Ask, KIND, Refusal, STOPWATCH, Stopwatch, Timer, VERSION, ask, error, now, watched};

/// Answers one client until it goes.
pub(crate) fn serve(shared: &Shared, stream: UnixStream, connection: u64) {
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
pub(crate) fn answer(
    shared: &Shared,
    asked: Ask,
    connection: u64,
    told: &Sender<String>,
) -> Option<Value> {
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
