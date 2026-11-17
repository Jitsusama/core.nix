//! A daemon of the launcher's daemon interface, started for a test, and a
//! client asking it over its socket, one line of JSON each way, for each
//! daemon's tests, which name this crate in their Cargo.toml as a
//! dev-dependency. The helpers were copied from one daemon's tests into
//! the next; kept here, a fix to one reaches every test that asks a
//! daemon.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use serde_json::Value;

/// How long a test waits for the daemon's socket, or for an answer.
const PATIENCE: Duration = Duration::from_secs(10);

/// A daemon serving its socket, stopped and its folder taken away once
/// it's dropped.
pub struct Daemon {
    child: Child,
    socket: PathBuf,
    folder: PathBuf,
}

impl Daemon {
    /// Starts `command`, which serves `socket`, once the socket's there;
    /// `folder` holds what the test made for it. A daemon that ends before
    /// its socket comes says how it ended at once, rather than once the
    /// test's patience is spent.
    pub fn start(command: &mut Command, socket: PathBuf, folder: PathBuf) -> Daemon {
        let child = command.spawn().expect("the daemon starts");
        let mut daemon = Daemon {
            child,
            socket,
            folder,
        };
        let deadline = Instant::now() + PATIENCE;
        while !daemon.socket.exists() {
            if let Ok(Some(ended)) = daemon.child.try_wait() {
                panic!("the daemon ended before its socket came: {ended}");
            }
            assert!(Instant::now() < deadline, "the socket never came");
            std::thread::sleep(Duration::from_millis(20));
        }
        daemon
    }

    /// The folder the test made for the daemon.
    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// A client of its own, asking over the socket.
    pub fn client(&self) -> Client {
        Client::connect(&self.socket)
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.folder);
    }
}

/// One client of a daemon, which asks and hears what a watch pushes.
pub struct Client {
    stream: UnixStream,
    lines: BufReader<UnixStream>,
}

impl Client {
    /// A client of the daemon serving `socket`, for a test that starts
    /// and stops the daemon itself, as one restarting it over the same
    /// folder does, which a `Daemon` would take away.
    ///
    /// A socket on the disk may not be listened on yet, since a daemon
    /// binds it a moment before it listens, so one refusing, or gone as
    /// it's made again, is asked again within the test's patience.
    pub fn connect(socket: &Path) -> Client {
        let deadline = Instant::now() + PATIENCE;
        let stream = loop {
            match UnixStream::connect(socket) {
                Err(why)
                    if matches!(
                        why.kind(),
                        std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::NotFound
                    ) && Instant::now() < deadline =>
                {
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(why) => panic!(
                    "{} never answered within the test's patience: {why}",
                    socket.display()
                ),
                Ok(stream) => break stream,
            }
        };
        stream
            .set_read_timeout(Some(PATIENCE))
            .expect("the socket takes a timeout");
        Client {
            lines: BufReader::new(stream.try_clone().expect("the socket is cloned")),
            stream,
        }
    }

    /// Sends an ask without waiting on its answer, for a test that reads
    /// what comes back itself, as one hearing a push that may come first.
    pub fn send(&mut self, asked: &Value) {
        writeln!(self.stream, "{asked}").expect("the ask is sent");
    }

    /// The answer to an ask, passing over what a watch pushed meanwhile:
    /// its changes, and how the sources stand, which a watch is told by
    /// its own id alone. An unwatch is asked by the watch's id, so taking
    /// such a push for its answer would go on before the watch had gone.
    pub fn ask(&mut self, asked: &Value) -> Value {
        self.ask_hearing(asked, |_| {})
    }

    /// The answer to an ask, as `ask` gives it, handing `told` each line
    /// said before it rather than passing over them. A daemon can hear
    /// its own write and tell a watch the change an ask made before it
    /// answers the ask, so a test waiting for that change after takes it
    /// in here.
    pub fn ask_hearing(&mut self, asked: &Value, mut told: impl FnMut(&Value)) -> Value {
        self.send(asked);
        loop {
            let said = self.heard();
            if said["id"] == asked["id"] && !pushed(&said) {
                return said;
            }
            told(&said);
        }
    }

    /// The next line the daemon says, whatever it is.
    pub fn heard(&mut self) -> Value {
        let mut line = String::new();
        self.lines
            .read_line(&mut line)
            .expect("the daemon answers in time");
        serde_json::from_str(&line).expect("the daemon says JSON")
    }

    /// The next change a watch is told of the record with this key.
    pub fn changed(&mut self, key: &str) -> Value {
        loop {
            let said = self.heard();
            let found = said["changed"]
                .as_array()
                .and_then(|records| records.iter().find(|record| record["key"] == key));
            if let Some(record) = found {
                return record.clone();
            }
        }
    }

    /// The next change saying keys went.
    pub fn gone(&mut self) -> Vec<Value> {
        loop {
            let said = self.heard();
            if let Some(gone) = said["gone"].as_array() {
                return gone.clone();
            }
        }
    }
}

/// Whether a line is a watch's push rather than an answer: a change, or
/// how the sources stand, said with nothing but the watch's id.
fn pushed(said: &Value) -> bool {
    let standing = said
        .as_object()
        .is_some_and(|line| line.len() == 2 && line.contains_key("sources"));
    said.get("changed").is_some() || standing
}

/// Waits until `done`, failing with `why` past the test's patience.
pub fn until(why: &str, done: impl Fn() -> bool) {
    let deadline = Instant::now() + PATIENCE;
    while !done() {
        assert!(Instant::now() < deadline, "{why}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;

    /// A daemon's socket is on the disk from its bind, a moment before it
    /// listens, so a test asking in between is refused: the case a daemon
    /// starting under load gives, made here by a socket no one listens on
    /// until a moment later.
    #[test]
    fn a_socket_not_listened_on_yet_is_waited_for() {
        let folder = std::env::temp_dir().join(format!(
            "daemon-client-not-listened-on-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("the folder is made");
        let socket = folder.join("daemon.sock");
        drop(UnixListener::bind(&socket).expect("the socket is bound"));
        assert_eq!(
            UnixStream::connect(&socket)
                .map(drop)
                .map_err(|why| why.kind()),
            Err(std::io::ErrorKind::ConnectionRefused),
            "the socket isn't refusing, so the case asks nothing"
        );
        let listening = {
            let socket = socket.clone();
            std::thread::spawn(move || {
                // Refused this long, then gone this long, as a daemon
                // making its socket again leaves it, so the client meets
                // both before it's answered.
                std::thread::sleep(Duration::from_millis(300));
                std::fs::remove_file(&socket).expect("the old socket goes");
                std::thread::sleep(Duration::from_millis(150));
                let listener = UnixListener::bind(&socket).expect("the socket is listened on");
                let (mut stream, _) = listener.accept().expect("the client is taken");
                writeln!(stream, "{{\"id\": 1}}").expect("the answer is sent");
            })
        };
        let mut client = Client::connect(&socket);
        assert_eq!(client.heard()["id"], 1, "the client heard no answer");
        listening.join().expect("the listener ends");
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// What a watch is told before an ask's answer is handed on, a change
    /// under the watch's own id among it, and the answer is the line that
    /// isn't a push.
    #[test]
    fn what_a_watch_is_told_before_an_answer_is_handed_on() {
        let folder =
            std::env::temp_dir().join(format!("daemon-client-told-first-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("the folder is made");
        let socket = folder.join("daemon.sock");
        let listener = UnixListener::bind(&socket).expect("the socket is bound");
        let answering = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("the client is taken");
            let mut asked = String::new();
            BufReader::new(stream.try_clone().expect("the socket is cloned"))
                .read_line(&mut asked)
                .expect("the ask is read");
            let mut stream = stream;
            for line in [
                r#"{"id":2,"changed":[{"key":"a"}]}"#,
                r#"{"id":1,"changed":[{"key":"b"}]}"#,
                r#"{"id":2,"sources":{}}"#,
                r#"{"id":2,"done":true}"#,
            ] {
                writeln!(stream, "{line}").expect("the line is sent");
            }
        });
        let mut client = Client::connect(&socket);
        let mut told = Vec::new();
        let answer = client.ask_hearing(&serde_json::json!({"id": 2, "ask": "act"}), |said| {
            told.push(said.clone());
        });
        assert_eq!(
            answer,
            serde_json::json!({"id": 2, "done": true}),
            "the answer"
        );
        assert_eq!(
            told.len(),
            3,
            "each line before the answer is handed on: {told:?}"
        );
        answering.join().expect("the daemon ends");
        let _ = std::fs::remove_dir_all(&folder);
    }
}
