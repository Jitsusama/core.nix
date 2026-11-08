//! The daemon's socket, for whatever shows the home network as it changes.
//!
//! Whoever connects is written the model at once, as one line of JSON, and
//! another line each time it changes; a line `read` from them asks for the
//! network to be read now. While anyone is connected the daemon reads at
//! its fast pace, so the socket is how the shell says it's looking.
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

/// What a listener did, which wakes the daemon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Call {
    Joined,
    Left,
    /// It asked for the network to be read now.
    Read,
}

/// How long a write to a listener may take before it's dropped, so one
/// that stopped reading can't hold up the others or the daemon.
const WRITE_TIMEOUT: Duration = Duration::from_secs(2);

/// Everyone connected, and the line each newcomer is written first.
#[derive(Clone)]
pub struct Listeners {
    hub: Arc<Mutex<Hub>>,
}

#[derive(Default)]
struct Hub {
    line: Option<String>,
    streams: Vec<(u64, UnixStream)>,
    next_id: u64,
}

impl Listeners {
    /// Listens at `path`, replacing a socket an earlier daemon left, and
    /// tells `wake` whenever a listener joins, leaves or asks for a read.
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

    /// Whether anyone is connected.
    pub fn listening(&self) -> bool {
        !self.hub().streams.is_empty()
    }

    /// Writes the model's line to everyone, and keeps it for whoever
    /// connects next. A listener that can't take it is dropped.
    pub fn send(&self, line: &str) {
        let mut hub = self.hub();
        hub.line = Some(line.to_owned());
        hub.streams.retain_mut(|(_, stream)| {
            let written = write_line(stream, line).is_ok();
            if !written {
                let _ = stream.shutdown(Shutdown::Both);
            }
            written
        });
    }

    /// Takes a new connection: writes it the model, if there is one yet,
    /// and reads what it says until it goes.
    fn join(&self, mut stream: UnixStream, wake: Sender<Call>) {
        if stream.set_write_timeout(Some(WRITE_TIMEOUT)).is_err() {
            return;
        }
        let Ok(reading) = stream.try_clone() else {
            return;
        };
        let id = {
            let mut hub = self.hub();
            if let Some(line) = &hub.line
                && write_line(&mut stream, line).is_err()
            {
                return;
            }
            let id = hub.next_id;
            hub.next_id += 1;
            hub.streams.push((id, stream));
            id
        };
        let _ = wake.send(Call::Joined);
        let listeners = self.clone();
        thread::spawn(move || {
            for line in BufReader::new(reading).lines() {
                match line {
                    Ok(line) if line.trim() == "read" => {
                        let _ = wake.send(Call::Read);
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
            listeners.hub().streams.retain(|(other, _)| *other != id);
            let _ = wake.send(Call::Left);
        });
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
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join("socket");
        let (wake, woken) = mpsc::channel();
        (Listeners::listen(&path, wake).unwrap(), woken, path)
    }

    fn read_line(reader: &mut BufReader<UnixStream>) -> String {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        line
    }

    #[test]
    fn a_listener_is_written_the_model_on_joining_and_on_every_change() {
        let (listeners, woken, path) = listening("changes");
        listeners.send(r#"{"name":"home"}"#);

        let stream = UnixStream::connect(&path).unwrap();
        let mut reader = BufReader::new(stream);

        assert_eq!(read_line(&mut reader), "{\"name\":\"home\"}\n");
        assert_eq!(woken.recv().unwrap(), Call::Joined);
        assert!(listeners.listening());
        listeners.send(r#"{"name":"cottage"}"#);
        assert_eq!(read_line(&mut reader), "{\"name\":\"cottage\"}\n");
    }

    #[test]
    fn a_listener_asking_to_read_wakes_the_daemon_and_leaving_is_heard() {
        let (listeners, woken, path) = listening("asks");
        let mut stream = UnixStream::connect(&path).unwrap();
        assert_eq!(woken.recv().unwrap(), Call::Joined);

        stream.write_all(b"read\n").unwrap();

        assert_eq!(woken.recv().unwrap(), Call::Read);
        drop(stream);
        assert_eq!(woken.recv().unwrap(), Call::Left);
        assert!(!listeners.listening());
    }
}
