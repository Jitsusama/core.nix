//! Each client of the socket: its lines read and answered, its watches
//! held, and its acts handed to PipeWire's loop, apart from the loop
//! itself in main.rs.

use std::io::{BufRead, BufReader, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};
use sound::acts::{Act, act, on_streams, planned};
use sound::interface::{Ask, VERSION, ask, error, kind_of};

use super::{Daemon, Doing, lock, timed};

/// How many lines a connection may have waiting to be written before
/// it's taken as no longer reading and hung up on.
const QUEUED: usize = 256;
/// How long an act waits for PipeWire's loop to take it.
const ACT_PATIENCE: Duration = Duration::from_secs(2);

/// One watch a client holds.
pub struct Watch {
    pub connection: u64,
    pub id: Value,
    kinds: Vec<String>,
    keys: Option<Vec<String>>,
    pub told: SyncSender<String>,
    pub stream: Arc<UnixStream>,
}

impl Watch {
    pub fn wants(&self, key: &str) -> bool {
        match &self.keys {
            Some(keys) => keys.iter().any(|k| k == key),
            None => kind_of(key).is_some_and(|kind| self.kinds.iter().any(|k| k == kind)),
        }
    }

    /// Whether it wants any record of this kind, by its kind or a key.
    pub fn wants_kind(&self, kind: &str) -> bool {
        match &self.keys {
            Some(keys) => keys.iter().any(|key| kind_of(key) == Some(kind)),
            None => self.kinds.iter().any(|k| k == kind),
        }
    }
}

pub fn serve(shared: &Mutex<Daemon>, stream: UnixStream, connection: u64) {
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
        let Some(answer) = answer(shared, ask(&line), connection, &told, &hangup) else {
            continue;
        };
        if told.send(answer.to_string()).is_err() {
            break;
        }
    }
    let mut daemon = lock(shared);
    daemon
        .watches
        .retain(|watch| watch.connection != connection);
    timed(&mut daemon);
}

/// The records held of these kinds or keys: by key, as the server's own
/// names order them.
fn held(daemon: &Daemon, kinds: &[String], keys: Option<&[String]>) -> Vec<Value> {
    match keys {
        Some(keys) => keys
            .iter()
            .filter_map(|key| daemon.held.get(key))
            .cloned()
            .collect(),
        None => daemon
            .held
            .iter()
            .filter(|(key, _)| kind_of(key).is_some_and(|kind| kinds.iter().any(|k| k == kind)))
            .map(|(_, record)| record.clone())
            .collect(),
    }
}

/// Works out what an act asks and has PipeWire's loop do it, saying it's
/// done or why not. What it moved comes to every watch as the server says
/// it back, so a watch's records are always the server's own.
fn acted(shared: &Mutex<Daemon>, id: &Value, name: &str, said: &Value) -> Value {
    let asked = match act(name, said) {
        Ok(asked) => asked,
        Err(refused) => return error(id, refused.code, &refused.message),
    };
    let (done, acting) = {
        let daemon = lock(shared);
        if let Some(done) = asked.alone() {
            (done, daemon.acting.clone())
        } else if let Act::Program { .. } = asked {
            match on_streams(&asked, &daemon.devices.streams) {
                Ok(done) => (done, daemon.acting.clone()),
                Err(refused) => return error(id, refused.code, &refused.message),
            }
        } else {
            let Some(device) = daemon.devices.device(asked.key()) else {
                return error(id, "no-device", &format!("no device {} here", asked.key()));
            };
            match planned(&asked, &device) {
                Ok(done) => (done, daemon.acting.clone()),
                Err(refused) => return error(id, refused.code, &refused.message),
            }
        }
    };
    let not_answering = || error(id, "not-answering", "the sound server isn't answering");
    let Some(acting) = acting else {
        return not_answering();
    };
    let (answer, answered) = mpsc::channel();
    if acting.send(Doing::Act { done, answer }).is_err() {
        return not_answering();
    }
    match answered.recv_timeout(ACT_PATIENCE) {
        Ok(Ok(())) => json!({"id": id, "done": true}),
        Ok(Err(why)) => error(id, "not-done", &why),
        Err(_) => not_answering(),
    }
}

/// How the daemon stands, as hello and health say it.
fn standing(daemon: &Daemon, id: &Value) -> Value {
    json!({
        "id": id, "version": VERSION, "domain": "sound", "state": "answering",
        "since": daemon.since,
        "sources": {"pipewire": {"state": daemon.server, "changes": daemon.changes}},
    })
}

/// What one ask is answered with, or none where it's been told already.
fn answer(
    shared: &Mutex<Daemon>,
    asked: Ask,
    connection: u64,
    told: &SyncSender<String>,
    hangup: &Arc<UnixStream>,
) -> Option<Value> {
    Some(match asked {
        Ask::Hello { id } => standing(&lock(shared), &id),
        Ask::Get { id, keys } => {
            json!({"id": id, "records": held(&lock(shared), &[], Some(&keys))})
        }
        Ask::List { id, kinds } => json!({"id": id, "records": held(&lock(shared), &kinds, None)}),
        Ask::Watch { id, kinds, keys } => {
            let mut daemon = lock(shared);
            // A watch asked again by its id replaces the one before.
            daemon
                .watches
                .retain(|watch| !(watch.connection == connection && watch.id == id));
            daemon.watches.push(Watch {
                connection,
                id: id.clone(),
                kinds: kinds.clone(),
                keys: keys.clone(),
                told: told.clone(),
                stream: Arc::clone(hangup),
            });
            timed(&mut daemon);
            // Told under the same lock a change is told under, so no
            // change comes before the answer it changes.
            let whole = json!({"id": id, "records": held(&daemon, &kinds, keys.as_deref())});
            if told.try_send(whole.to_string()).is_err() {
                let _ = hangup.shutdown(Shutdown::Both);
            }
            return None;
        }
        Ask::Unwatch { id } => {
            let mut daemon = lock(shared);
            daemon
                .watches
                .retain(|watch| !(watch.connection == connection && watch.id == id));
            timed(&mut daemon);
            json!({"id": id, "unwatched": true})
        }
        Ask::Health { id } => {
            let daemon = lock(shared);
            let mut health = standing(&daemon, &id);
            health["watches"] = json!(daemon.watches.len());
            health
        }
        Ask::Act { id, act, said } => acted(shared, &id, &act, &said),
        Ask::UnknownKind { id, kind } => {
            error(&id, "unknown-kind", &format!("no kind {kind} here"))
        }
        Ask::Unknown { id, ask } => error(&id, "unknown-ask", &format!("no ask {ask:?} here")),
    })
}
