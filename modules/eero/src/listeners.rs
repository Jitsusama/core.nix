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
//! a watch is told after each read only the rows that changed.
//!
//! Each connection has a thread that reads what it says, and one more
//! accepts them: plain threads, since a handful of listeners and a line
//! every half minute need no async runtime.

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
use crate::served::{self, Said, Vocabularies};

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
/// said under, and the act.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asked {
    pub listener: u64,
    pub id: Value,
    pub act: Act,
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
    /// read.
    pub fn listen(path: &Path, wake: Sender<Call>) -> io::Result<Listeners> {
        Listeners::listen_within(path, wake, Vocabularies::default())
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
        listeners.retain_mut(|listener| {
            let mut lines = Vec::new();
            if listener.watching {
                lines.push(told.clone());
            }
            for watch in &listener.rows {
                let (Some(was), Some(now)) =
                    (before.get(&watch.said), vocabularies.made(&watch.said))
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
        let Ok(reading) = stream.try_clone() else {
            return;
        };
        let id = {
            let mut hub = self.hub();
            let id = hub.next_id;
            hub.next_id += 1;
            hub.listeners.push(Listener {
                id,
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

fn write_line(stream: &mut UnixStream, line: &str) -> io::Result<()> {
    stream.write_all(line.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    /// A folder of the test's own.
    fn folder(name: &str) -> std::path::PathBuf {
        let folder = std::env::temp_dir().join(format!("eero-{name}-{}", std::process::id()));
        fs::create_dir_all(&folder).expect("the test's folder is made");
        folder
    }

    /// A socket in a folder of the test's own, and what wakes the daemon.
    fn listening(name: &str) -> (Listeners, mpsc::Receiver<Call>, std::path::PathBuf) {
        let path = folder(name).join("socket");
        let (wake, woken) = mpsc::channel();
        let listeners = Listeners::listen(&path, wake).expect("the socket listens");
        (listeners, woken, path)
    }

    /// A socket that serves rows in vocabularies read from a store of the
    /// test's own, holding the launcher's words for the Home Network; the
    /// path of those words, as a listener names them; and a connection.
    fn serving(name: &str) -> (Listeners, String, UnixStream, BufReader<UnixStream>) {
        let folder = folder(name);
        let store = folder.join("store");
        fs::create_dir_all(&store).expect("the test's store is made");
        let words = store.join("words.json");
        fs::write(&words, include_str!("../tests/words.json")).expect("the words are kept");
        let store = store.canonicalize().expect("the store is found");
        let path = folder.join("socket");
        let (wake, _woken) = mpsc::channel();
        let listeners = Listeners::listen_within(&path, wake, Vocabularies::within(store))
            .expect("the socket listens");
        let (stream, reader) = connected(&path);
        (listeners, words.display().to_string(), stream, reader)
    }

    /// The home network the rows' cases read, as the daemon sends it.
    fn home() -> Value {
        serde_json::from_str::<Value>(include_str!("../home-network.json"))
            .expect("the home network's fixture is JSON")
    }

    fn keys(records: &Value) -> Vec<&str> {
        records
            .as_array()
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .filter_map(|record| record["key"].as_str())
            .collect()
    }

    /// A connection, as the shell holds one, and what it reads.
    fn connected(path: &Path) -> (UnixStream, BufReader<UnixStream>) {
        let stream = UnixStream::connect(path).expect("the socket takes a connection");
        let reading = stream.try_clone().expect("the connection clones");
        (stream, BufReader::new(reading))
    }

    fn ask(stream: &mut UnixStream, said: &Value) {
        write_line(stream, &said.to_string()).expect("the ask is written");
    }

    fn read(reader: &mut BufReader<UnixStream>) -> Value {
        let mut line = String::new();
        reader.read_line(&mut line).expect("the daemon answers");
        serde_json::from_str(&line).expect("the answer is JSON")
    }

    fn record(name: &str) -> Value {
        json!({"key": KIND, "kind": KIND, "network": {"name": name}})
    }

    #[test]
    fn a_watch_is_answered_with_the_network_and_told_every_change() {
        let (listeners, woken, path) = listening("changes");
        listeners.send(&json!({"name": "home"}));
        let (mut stream, mut reader) = connected(&path);
        assert_eq!(woken.recv(), Ok(Call::Joined), "joining wakes the daemon");

        ask(
            &mut stream,
            &json!({"id": "hello", "ask": "hello", "version": 1}),
        );
        assert_eq!(
            read(&mut reader)["state"],
            "answering",
            "hello says how it stands"
        );
        ask(
            &mut stream,
            &json!({"id": "all", "ask": "watch", "kind": KIND}),
        );
        assert_eq!(
            read(&mut reader),
            json!({"id": "all", "records": [record("home")]}),
            "the watch is answered with the network whole"
        );
        listeners.send(&json!({"name": "cottage"}));

        assert_eq!(
            read(&mut reader),
            json!({"id": "all", "changed": [record("cottage")]}),
            "each change is told to the watch"
        );
        assert!(!listeners.listening(), "a watch alone isn't looking");
    }

    #[test]
    fn only_a_watch_ahead_of_the_network_reads_it_at_the_fast_pace() {
        let (listeners, woken, path) = listening("ahead");
        let (mut stream, mut reader) = connected(&path);
        assert_eq!(woken.recv(), Ok(Call::Joined), "joining wakes the daemon");

        ask(
            &mut stream,
            &json!({"id": "ahead", "ask": "watch", "kind": KIND, "keys": [KIND]}),
        );
        assert_eq!(
            read(&mut reader)["records"],
            json!([]),
            "no network is read yet"
        );
        assert_eq!(woken.recv(), Ok(Call::Looked), "looking sets the pace");
        assert!(listeners.listening(), "a watch ahead is looking");
        ask(&mut stream, &json!({"id": "ahead", "ask": "unwatch"}));

        assert_eq!(
            read(&mut reader)["unwatched"],
            true,
            "the unwatch is answered"
        );
        assert_eq!(woken.recv(), Ok(Call::Looked), "stopping sets the pace");
        assert!(!listeners.listening(), "nobody looks once it's let go");
    }

    #[test]
    fn an_ask_to_read_wakes_the_daemon_and_leaving_is_heard() {
        let (listeners, woken, path) = listening("asks");
        let (mut stream, mut reader) = connected(&path);
        assert_eq!(woken.recv(), Ok(Call::Joined), "joining wakes the daemon");

        ask(&mut stream, &json!({"id": "ask-1", "ask": "read"}));

        assert_eq!(
            read(&mut reader),
            json!({"id": "ask-1"}),
            "the ask is answered"
        );
        assert_eq!(
            woken.recv(),
            Ok(Call::Read),
            "the ask wakes the daemon to read"
        );
        drop((stream, reader));
        assert_eq!(woken.recv(), Ok(Call::Left), "leaving wakes the daemon");
        assert!(!listeners.listening(), "nobody looks once it's gone");
    }

    #[test]
    fn an_act_is_answered_once_the_daemon_has_done_it() {
        let (listeners, woken, path) = listening("acts");
        let (mut stream, mut reader) = connected(&path);
        assert_eq!(woken.recv(), Ok(Call::Joined), "joining wakes the daemon");

        ask(
            &mut stream,
            &json!({"id": "act-1", "ask": "act", "act": "sqm", "on": true}),
        );
        let Ok(Call::Act(asked)) = woken.recv() else {
            panic!("the act wakes the daemon to do it");
        };
        assert_eq!(
            asked.act,
            Act::Conferencing(true),
            "the act is the one asked"
        );
        listeners.answer_act(&asked, Err("eero is busy".into()));

        assert_eq!(
            read(&mut reader),
            json!({"id": "act-1", "error": {"code": "refused", "message": "eero is busy"}}),
            "the act is answered with what the daemon heard"
        );
        ask(
            &mut stream,
            &json!({"id": "act-2", "ask": "act", "act": "speedtest"}),
        );
        let Ok(Call::Act(asked)) = woken.recv() else {
            panic!("the act wakes the daemon to do it");
        };
        listeners.answer_act(&asked, Ok(()));
        assert_eq!(
            read(&mut reader),
            json!({"id": "act-2", "done": true}),
            "an act done is answered done"
        );
    }

    #[test]
    fn an_act_eero_has_none_of_is_answered_at_once() {
        let (_listeners, woken, path) = listening("bad-acts");
        let (mut stream, mut reader) = connected(&path);
        assert_eq!(woken.recv(), Ok(Call::Joined), "joining wakes the daemon");

        ask(
            &mut stream,
            &json!({"id": "act-1", "ask": "act", "act": "reboot"}),
        );

        assert_eq!(
            read(&mut reader)["error"]["code"],
            "bad-act",
            "an act eero hasn't is refused by the daemon itself"
        );
        assert!(
            woken.try_recv().is_err(),
            "an act refused doesn't wake the daemon"
        );
    }

    #[test]
    fn a_row_watch_is_answered_ready_and_told_only_the_rows_that_changed() {
        let (listeners, words, mut stream, mut reader) = serving("rows");
        listeners.send(&home());

        ask(
            &mut stream,
            &json!({"id": "rows", "ask": "watch", "kind": ["row", "drawer"], "place": "eero",
                "words": words}),
        );
        let answered = read(&mut reader);
        let all = keys(&answered["records"]);
        assert!(
            all.contains(&"row:eero:internet") && all.contains(&"row:eero:node:2003"),
            "the watch is answered with the place's rows, the internet's and each node's: {all:?}"
        );
        let records = answered["records"].as_array().expect("records are a list");
        assert!(
            records
                .iter()
                .filter(|record| record["key"]
                    .as_str()
                    .is_some_and(|key| key.starts_with("row:")))
                .all(|record| record["kind"] == "row"),
            "each row is a row's record"
        );
        assert!(
            all.contains(&"drawer:eero:place"),
            "the place's own drawer is served beside its rows: {all:?}"
        );
        let mut hall_off = home();
        hall_off["nodes"][0]["state"] = json!("OFFLINE");
        listeners.send(&hall_off);

        let told = read(&mut reader);
        let changed = keys(&told["changed"]);
        assert_eq!(told["id"], "rows", "the change is told to the row watch");
        assert!(
            changed.contains(&"row:eero:node:2003") && changed.len() < all.len(),
            "only the rows that read differently are told: {changed:?}"
        );
    }

    #[test]
    fn a_watch_of_some_rows_is_answered_with_those_alone() {
        let (listeners, words, mut stream, mut reader) = serving("some");
        listeners.send(&home());

        ask(
            &mut stream,
            &json!({"id": "one", "ask": "get", "kind": "row", "words": words,
                "keys": ["row:eero:internet"]}),
        );

        assert_eq!(
            keys(&read(&mut reader)["records"]),
            vec!["row:eero:internet"],
            "the rows asked for by key are answered alone"
        );
    }

    #[test]
    fn rows_are_refused_in_words_from_outside_the_store_or_of_another_kind() {
        let (_listeners, words, mut stream, mut reader) = serving("refused");
        let outside = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/words.json");

        ask(
            &mut stream,
            &json!({"id": 1, "ask": "list", "kind": "row", "words": outside}),
        );
        assert_eq!(
            read(&mut reader)["error"]["code"],
            "no-words",
            "words from outside the store are refused"
        );
        ask(&mut stream, &json!({"id": 2, "ask": "list", "kind": "row"}));
        assert_eq!(
            read(&mut reader)["error"]["code"],
            "no-words",
            "rows asked for in no words are refused"
        );
        ask(
            &mut stream,
            &json!({"id": 3, "ask": "list", "kind": ["row", "song"], "words": words}),
        );
        assert_eq!(
            read(&mut reader)["error"]["code"],
            "unknown-kind",
            "a kind this socket doesn't keep is refused"
        );
        ask(
            &mut stream,
            &json!({"id": 4, "ask": "list", "place": "wifi", "words": words}),
        );
        assert_eq!(
            read(&mut reader)["error"]["code"],
            "unknown-kind",
            "another place is refused"
        );
    }

    #[test]
    fn an_unwatched_row_watch_is_told_nothing_more() {
        let (listeners, words, mut stream, mut reader) = serving("unwatched");
        listeners.send(&home());
        ask(
            &mut stream,
            &json!({"id": "rows", "ask": "watch", "kind": "row", "words": words}),
        );
        read(&mut reader);

        ask(&mut stream, &json!({"id": "rows", "ask": "unwatch"}));
        assert_eq!(
            read(&mut reader)["unwatched"],
            true,
            "the unwatch is answered"
        );
        let mut hall_off = home();
        hall_off["nodes"][0]["state"] = json!("OFFLINE");
        listeners.send(&hall_off);
        ask(
            &mut stream,
            &json!({"id": "after", "ask": "hello", "version": 1}),
        );

        assert_eq!(
            read(&mut reader)["id"],
            "after",
            "the next line is the hello's answer, no change told before it"
        );
    }
}
