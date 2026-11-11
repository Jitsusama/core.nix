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
//! `read` asks for the network to be read now.
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

/// What a listener did, which wakes the daemon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Call {
    Joined,
    Left,
    /// It began or stopped looking, which sets the daemon's pace.
    Looked,
    /// It asked for the network to be read now.
    Read,
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
    listeners: Vec<Listener>,
    next_id: u64,
}

struct Listener {
    id: u64,
    stream: UnixStream,
    /// Whether it watches the record, so it's told each change.
    watching: bool,
    /// Whether it watches the record ahead, which is looking at it.
    looking: bool,
}

impl Listeners {
    /// Listens at `path`, replacing a socket an earlier daemon left, and
    /// tells `wake` whenever a listener joins, leaves, looks or asks for a
    /// read.
    pub fn listen(path: &Path, wake: Sender<Call>) -> io::Result<Listeners> {
        match fs::remove_file(path) {
            Err(e) if e.kind() != ErrorKind::NotFound => return Err(e),
            _ => {}
        }
        let socket = UnixListener::bind(path)?;
        let listeners = Listeners {
            hub: Arc::new(Mutex::new(Hub::default())),
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
    /// next. A listener that can't take it is dropped.
    pub fn send(&self, network: &Value) {
        let record = json!({"key": KIND, "kind": KIND, "network": network});
        let told = json!({"id": "all", "changed": [record]}).to_string();
        let mut hub = self.hub();
        hub.record = Some(record);
        hub.listeners.retain_mut(|listener| {
            let written = !listener.watching || write_line(&mut listener.stream, &told).is_ok();
            if !written {
                let _ = listener.stream.shutdown(Shutdown::Both);
            }
            written
        });
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
        let records: Vec<Value> = hub.record.iter().cloned().collect();
        let listener = hub
            .listeners
            .iter_mut()
            .find(|listener| listener.id == id)?;
        let (answer, call) = match asked["ask"].as_str() {
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

fn write_line(stream: &mut UnixStream, line: &str) -> io::Result<()> {
    stream.write_all(line.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    /// A socket in a folder of the test's own, and what wakes the daemon.
    fn listening(name: &str) -> (Listeners, mpsc::Receiver<Call>, std::path::PathBuf) {
        let folder = std::env::temp_dir().join(format!("eero-{name}-{}", std::process::id()));
        fs::create_dir_all(&folder).expect("the test's folder is made");
        let path = folder.join("socket");
        let (wake, woken) = mpsc::channel();
        let listeners = Listeners::listen(&path, wake).expect("the socket listens");
        (listeners, woken, path)
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
}
