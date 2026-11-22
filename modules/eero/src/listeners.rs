//! The daemon's socket, for whatever shows the home network as it changes.
//!
//! It speaks the daemon interface every daemon of the shell's speaks
//! (RSCH-20261029-0E8050, version 1), so the shell holds it open as it
//! holds the others (Daemon.qml), from the moment it starts, rather than
//! only while the Home Network is open and reading the record's file the
//! rest of the time. The home network is one record, kind and key
//! `home-network`, the model whole under `network`. A watch is answered
//! with it and told each change; a watch ahead of that key is someone
//! looking, so while one holds, the daemon reads at its fast pace; an ask
//! `read` asks for the network to be read now, and an ask `act` for
//! something done to it (acts.rs), which the daemon answers once eero has
//! done it, so a refusal is said in its place.
//!
//! It also serves the Home Network's rows ready to draw, as every daemon
//! serves its place's (served.rs): a get, list or watch of kind `row` or
//! place `eero`, in the launcher's words, is answered with the rows, and
//! a watch is told after each read only the rows that changed, and after
//! net's record says a different worst, which the Map row says.
//!
//! Each connection has a thread that reads what it says, one more
//! accepts them, and one follows net: plain threads, since a handful of listeners and a line
//! every half minute need no async runtime.

use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, BufReader, ErrorKind, Write};
use std::net::Shutdown;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};

use crate::acts::Act;
use crate::model::HomeNetwork;
use crate::served::{self, Mapped, Said, Vocabularies};

/// What a listener did, which wakes the daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Call {
    Joined,
    Left,
    /// It began or stopped looking, which sets the daemon's pace.
    Looked,
    /// It asked for the network to be read now.
    Read,
    /// It asked for something done to the network, which the daemon
    /// answers (`Listeners::answer_act`) once eero has done it.
    Act(Asked),
}

/// An act a listener asked for: who asked, the ask's id its answer is
/// said under, the act, and the process that asked, so each act is logged
/// by who did it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asked {
    pub listener: u64,
    pub id: Value,
    pub act: Act,
    pub peer: Option<Peer>,
}

/// The process at the other end of a listener's connection, as the kernel
/// says it, which no listener can tell otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Peer {
    pub pid: i32,
    pub uid: u32,
}

impl Peer {
    /// Who connected on `stream`, or none where the kernel won't say.
    fn of(stream: &UnixStream) -> Option<Peer> {
        let said = rustix::net::sockopt::socket_peercred(stream).ok()?;
        Some(Peer {
            pid: said.pid.as_raw_nonzero().get(),
            uid: said.uid.as_raw(),
        })
    }

    /// Whether this is the daemon's own account, the only one it answers:
    /// what it tells holds the home network's devices, and what it's asked
    /// can change it.
    fn is_ours(self) -> bool {
        self.uid == rustix::process::getuid().as_raw()
    }
}

/// The kind of the one record, and its key.
pub const KIND: &str = "home-network";

/// How long a write to a listener may take before it's dropped, so one
/// that stopped reading can't hold up the others or the daemon.
const WRITE_TIMEOUT: Duration = Duration::from_secs(2);

/// Everyone connected, and the record each newcomer is answered with.
#[derive(Clone)]
pub struct Listeners {
    hub: Arc<Mutex<Hub>>,
}

#[derive(Default)]
struct Hub {
    record: Option<Value>,
    /// The network as last read, which rows are made from.
    network: Option<HomeNetwork>,
    vocabularies: Vocabularies,
    listeners: Vec<Listener>,
    next_id: u64,
}

/// A watch of the rows: its id, the vocabulary they're said in, and the
/// keys it wants, or every row.
struct RowWatch {
    id: Value,
    said: Said,
    keys: Option<Vec<String>>,
}

impl RowWatch {
    fn wants(&self, key: &str) -> bool {
        self.keys
            .as_ref()
            .is_none_or(|keys| keys.iter().any(|wanted| wanted == key))
    }
}

struct Listener {
    id: u64,
    stream: UnixStream,
    /// Who connected, read once as it joined.
    peer: Option<Peer>,
    /// Whether it watches the record, so it's told each change.
    watching: bool,
    /// Whether it watches the record ahead, which is looking at it.
    looking: bool,
    /// Its watches of the rows.
    rows: Vec<RowWatch>,
    /// The words and number settings it last asked for rows in, which
    /// an ask naming none takes.
    words: Option<String>,
    numbers: Option<String>,
}

impl Listeners {
    /// Listens at `path`, replacing a socket an earlier daemon left, and
    /// tells `wake` whenever a listener joins, leaves, looks or asks for a
    /// read. Its rows offer the network map's page, `map`, if any.
    pub fn listen(path: &Path, wake: Sender<Call>, map: Option<Mapped>) -> io::Result<Listeners> {
        Listeners::listen_within(path, wake, Vocabularies::default().mapped(map))
    }

    /// Listens as `listen` does, reading vocabularies as `vocabularies`
    /// allows: from the Nix store, but in tests.
    fn listen_within(
        path: &Path,
        wake: Sender<Call>,
        vocabularies: Vocabularies,
    ) -> io::Result<Listeners> {
        match fs::remove_file(path) {
            Err(e) if e.kind() != ErrorKind::NotFound => return Err(e),
            _ => {}
        }
        let socket = UnixListener::bind(path)?;
        let listeners = Listeners {
            hub: Arc::new(Mutex::new(Hub {
                vocabularies,
                ..Hub::default()
            })),
        };
        let accepting = listeners.clone();
        thread::spawn(move || {
            for stream in socket.incoming() {
                match stream {
                    Ok(stream) => accepting.join(stream, wake.clone()),
                    // A refusal such as running out of file handles comes
                    // back at once, so pausing keeps it from spinning.
                    Err(e) => {
                        eprintln!("eero: couldn't take a listener: {e}");
                        thread::sleep(Duration::from_secs(1));
                    }
                }
            }
        });
        Ok(listeners)
    }

    /// Whether anyone is looking at the home network.
    pub fn listening(&self) -> bool {
        self.hub().listeners.iter().any(|listener| listener.looking)
    }

    /// Tells everyone watching the model, and keeps it for whoever watches
    /// next; makes the rows again from it, telling each watch of them the
    /// rows that changed. A listener that can't take it is dropped.
    pub fn send(&self, network: &Value) {
        let record = json!({"key": KIND, "kind": KIND, "network": network});
        let told = json!({"id": "all", "changed": [record]}).to_string();
        let mut hub = self.hub();
        let Hub {
            record: kept,
            network: model,
            vocabularies,
            listeners,
            ..
        } = &mut *hub;
        *kept = Some(record);
        // What the daemon sends is the model it read, so it reads back; a
        // network that didn't would make no rows rather than wrong ones.
        *model = serde_json::from_value(network.clone()).ok();
        vocabularies.keep(|said| {
            listeners
                .iter()
                .any(|listener| listener.rows.iter().any(|watch| &watch.said == said))
        });
        let before = model
            .as_ref()
            .map(|model| vocabularies.make(model))
            .unwrap_or_default();
        tell(listeners, vocabularies, &before, Some(&told));
    }

    /// Makes the rows again when net's record says a different worst than
    /// they were made with, while eero's network stays the same, telling
    /// each watch of them the rows that changed; whether it did. The
    /// daemon looks each `NET_LOOK` (daemon.rs), so the Map row is never
    /// further behind net than that.
    pub fn follow_net(&self) -> bool {
        let mut hub = self.hub();
        let Hub {
            network: model,
            vocabularies,
            listeners,
            ..
        } = &mut *hub;
        let Some(model) = model.as_ref() else {
            return false;
        };
        if !vocabularies.net_moved() {
            return false;
        }
        let before = vocabularies.make(model);
        tell(listeners, vocabularies, &before, None);
        true
    }

    /// Looks each `every` for net's record saying a different worst, on a
    /// thread of its own, so the Map row follows net between eero's reads.
    pub fn follow_net_every(&self, every: Duration) {
        let following = self.clone();
        thread::spawn(move || {
            loop {
                thread::sleep(every);
                following.follow_net();
            }
        });
    }

    /// Answers an act a listener asked for, done (`Ok`) or refused with
    /// why, as every daemon answers one: `done`, or an error the shell
    /// says in its own words. One gone since it asked hears nothing.
    pub fn answer_act(&self, asked: &Asked, done: Result<(), String>) {
        let answer = match done {
            Ok(()) => json!({"id": asked.id, "done": true}),
            Err(why) => json!({"id": asked.id, "error": {"code": "refused", "message": why}}),
        };
        let mut hub = self.hub();
        let Some(listener) = hub
            .listeners
            .iter_mut()
            .find(|listener| listener.id == asked.listener)
        else {
            return;
        };
        if write_line(&mut listener.stream, &answer.to_string()).is_err() {
            let _ = listener.stream.shutdown(Shutdown::Both);
        }
    }

    /// Takes a new connection and answers what it asks until it goes.
    fn join(&self, stream: UnixStream, wake: Sender<Call>) {
        if stream.set_write_timeout(Some(WRITE_TIMEOUT)).is_err() {
            return;
        }
        let peer = Peer::of(&stream);
        if !peer.is_some_and(Peer::is_ours) {
            eprintln!(
                "eero: refused a listener of uid {:?}, which isn't the daemon's",
                peer.map(|peer| peer.uid)
            );
            let _ = stream.shutdown(Shutdown::Both);
            return;
        }
        let Ok(reading) = stream.try_clone() else {
            return;
        };
        let id = {
            let mut hub = self.hub();
            let id = hub.next_id;
            hub.next_id += 1;
            hub.listeners.push(Listener {
                id,
                peer,
                stream,
                watching: false,
                looking: false,
                rows: Vec::new(),
                words: None,
                numbers: None,
            });
            id
        };
        let _ = wake.send(Call::Joined);
        let listeners = self.clone();
        thread::spawn(move || {
            for line in BufReader::new(reading).lines() {
                let Ok(line) = line else {
                    break;
                };
                if let Some(call) = listeners.answer(id, &line) {
                    let _ = wake.send(call);
                }
            }
            listeners
                .hub()
                .listeners
                .retain(|listener| listener.id != id);
            let _ = wake.send(Call::Left);
        });
    }

    /// Answers one line from listener `id`, saying what it asks of the
    /// daemon, if anything. The answer is written under the hub's lock, so
    /// a watch's answer comes before any change told after it.
    fn answer(&self, id: u64, line: &str) -> Option<Call> {
        let asked: Value = serde_json::from_str(line).ok()?;
        let said = asked.get("id").cloned().unwrap_or(Value::Null);
        let mut hub = self.hub();
        let Hub {
            record,
            network,
            vocabularies,
            listeners,
            ..
        } = &mut *hub;
        let records: Vec<Value> = record.iter().cloned().collect();
        let listener = listeners.iter_mut().find(|listener| listener.id == id)?;
        let ask = asked["ask"].as_str();
        let (answer, call) = match ask {
            Some("get" | "list" | "watch") if served::asks_rows(&asked) => (
                rows_asked(listener, vocabularies, network.as_ref(), &asked, &said),
                None,
            ),
            Some("hello") => (
                json!({"id": said, "state": "answering", "version": 1}),
                None,
            ),
            Some("watch") if asked.get("keys").is_some() => {
                let keys = asked["keys"].as_array().map_or(&[][..], Vec::as_slice);
                let was = listener.looking;
                listener.looking = keys.iter().any(|key| key == KIND);
                let looked = (listener.looking != was).then_some(Call::Looked);
                let ahead = if listener.looking {
                    records
                } else {
                    Vec::new()
                };
                (json!({"id": said, "records": ahead}), looked)
            }
            Some("watch") => {
                listener.watching = true;
                (json!({"id": said, "records": records}), None)
            }
            Some("unwatch") if listener.rows.iter().any(|watch| watch.id == said) => {
                listener.rows.retain(|watch| watch.id != said);
                (json!({"id": said, "unwatched": true}), None)
            }
            Some("unwatch") => {
                let looked = listener.looking.then_some(Call::Looked);
                if said == "ahead" {
                    listener.looking = false;
                    (json!({"id": said, "unwatched": true}), looked)
                } else {
                    listener.watching = false;
                    (json!({"id": said, "unwatched": true}), None)
                }
            }
            Some("read") => (json!({"id": said}), Some(Call::Read)),
            // Answered once it's done, by the daemon, rather than here.
            Some("act") => match Act::asked(&asked) {
                Ok(act) => {
                    return Some(Call::Act(Asked {
                        listener: id,
                        id: said,
                        act,
                        peer: listener.peer,
                    }));
                }
                Err(why) => (
                    json!({"id": said, "error": {"code": "bad-act", "message": why}}),
                    None,
                ),
            },
            _ => (
                json!({"id": said, "error": "the eero daemon doesn't know that ask"}),
                None,
            ),
        };
        if write_line(&mut listener.stream, &answer.to_string()).is_err() {
            let _ = listener.stream.shutdown(Shutdown::Both);
        }
        call
    }

    /// The hub, even if a thread panicked holding it: what it holds is
    /// whole after every step, so it's still good to use.
    fn hub(&self) -> MutexGuard<'_, Hub> {
        self.hub.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Answers a get, list or watch of the rows: the rows in the words and
/// number settings it names, or those it last named, among the keys it
/// names, if any. A watch is kept, by its id, to be told what changes.
fn rows_asked(
    listener: &mut Listener,
    vocabularies: &mut Vocabularies,
    network: Option<&HomeNetwork>,
    asked: &Value,
    id: &Value,
) -> Value {
    if let Some(refused) = served::unkept(asked) {
        return served::error(id, "unknown-kind", &refused);
    }
    let named = |field: &str| {
        asked[field]
            .as_str()
            .filter(|path| !path.is_empty())
            .map(str::to_owned)
    };
    let Some(words) = named("words").or_else(|| listener.words.clone()) else {
        return served::error(id, "no-words", "say the launcher's words.json as words");
    };
    let numbers = named("numbers").or_else(|| listener.numbers.clone());
    let said = match vocabularies.rows_in(&words, numbers.as_deref(), network) {
        Ok(said) => said,
        Err(why) => return served::error(id, "no-words", &why),
    };
    listener.words = Some(words);
    listener.numbers = numbers;
    let keys: Option<Vec<String>> = asked["keys"].as_array().map(|keys| {
        keys.iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect()
    });
    let watch = RowWatch {
        id: id.clone(),
        said,
        keys,
    };
    let records: Vec<&Value> = vocabularies
        .made(&watch.said)
        .into_iter()
        .flatten()
        .filter(|(key, _)| watch.wants(key))
        .map(|(_, row)| row)
        .collect();
    let answer = json!({"id": id, "records": records});
    if asked["ask"] == "watch" {
        listener.rows.retain(|kept| kept.id != *id);
        listener.rows.push(watch);
    }
    answer
}

/// Tells each listener `told` of the model, if it watches it, and each
/// watch of the rows those that changed since `before`, dropping any
/// listener that can't take it.
fn tell(
    listeners: &mut Vec<Listener>,
    vocabularies: &Vocabularies,
    before: &HashMap<Said, served::Made>,
    told: Option<&str>,
) {
    listeners.retain_mut(|listener| {
        let mut lines = Vec::new();
        if let Some(told) = told.filter(|_| listener.watching) {
            lines.push(told.to_owned());
        }
        for watch in &listener.rows {
            let (Some(was), Some(now)) = (before.get(&watch.said), vocabularies.made(&watch.said))
            else {
                continue;
            };
            let (changed, gone) = served::between(was, now, |key| watch.wants(key));
            if changed.is_empty() && gone.is_empty() {
                continue;
            }
            let mut said = json!({"id": watch.id, "changed": changed});
            if !gone.is_empty() {
                said["gone"] = json!(gone);
            }
            lines.push(said.to_string());
        }
        let written = lines
            .iter()
            .all(|line| write_line(&mut listener.stream, line).is_ok());
        if !written {
            let _ = listener.stream.shutdown(Shutdown::Both);
        }
        written
    });
}

fn write_line(stream: &mut UnixStream, line: &str) -> io::Result<()> {
    stream.write_all(line.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()
}

#[cfg(test)]
#[path = "listeners_tests.rs"]
mod tests;
