//! The shell's socket: each shell that connects is sent the whole state at
//! once, then the whole state again each time it changes, one line each.
//! What a shell writes, a line at a time, is handed on as it comes
//! (ask.rs reads it).
//!
//! A shell that stops reading is let go after a second rather than holding
//! everyone else up; it connects again when it restarts.

use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

#[derive(Default)]
struct Shared {
    line: String,
    shells: Vec<UnixStream>,
}

/// The socket, and what it last said.
#[derive(Clone)]
pub struct Socket {
    shared: Arc<Mutex<Shared>>,
}

impl Socket {
    /// Listens at `path`, replacing a socket an earlier run left there,
    /// and starts sending `first` to each shell that connects. What the
    /// shells write comes out of the receiver, a line each.
    pub fn listen(path: &Path, first: String) -> io::Result<(Socket, Receiver<String>)> {
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        let listener = UnixListener::bind(path)?;
        let socket = Socket {
            shared: Arc::new(Mutex::new(Shared {
                line: first,
                shells: Vec::new(),
            })),
        };
        let (asks, asked) = mpsc::channel();
        let accepting = socket.clone();
        thread::spawn(move || {
            for shell in listener.incoming().flatten() {
                accepting.welcome(shell, &asks);
            }
        });
        Ok((socket, asked))
    }

    fn welcome(&self, mut shell: UnixStream, asks: &Sender<String>) {
        if shell
            .set_write_timeout(Some(Duration::from_secs(1)))
            .is_err()
        {
            return;
        }
        if let Ok(reading) = shell.try_clone() {
            let asks = asks.clone();
            thread::spawn(move || {
                for line in BufReader::new(reading).lines() {
                    let Ok(line) = line else { return };
                    if asks.send(line).is_err() {
                        return;
                    }
                }
            });
        }
        let mut shared = self.shared.lock().unwrap_or_else(PoisonError::into_inner);
        if shell.write_all(shared.line.as_bytes()).is_ok() {
            shared.shells.push(shell);
        }
    }

    /// Says `line` to every shell, if it isn't what was said last.
    pub fn say(&self, line: String) {
        let mut shared = self.shared.lock().unwrap_or_else(PoisonError::into_inner);
        if shared.line == line {
            return;
        }
        shared
            .shells
            .retain_mut(|shell| shell.write_all(line.as_bytes()).is_ok());
        shared.line = line;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(test: &str) -> std::path::PathBuf {
        let folder = std::env::temp_dir().join(format!("google-{test}-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn what_a_shell_writes_is_handed_on_a_line_at_a_time() {
        let folder = folder("asks");
        let path = folder.join("google.sock");
        let (_socket, asked) = Socket::listen(&path, "first\n".into()).unwrap();
        let mut shell = UnixStream::connect(&path).unwrap();
        shell.write_all(b"one\ntwo\n").unwrap();
        let timeout = Duration::from_secs(5);
        assert_eq!(asked.recv_timeout(timeout).unwrap(), "one");
        assert_eq!(asked.recv_timeout(timeout).unwrap(), "two");
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_shell_hears_the_state_then_each_change_once() {
        let folder = folder("socket");
        let path = folder.join("google.sock");
        let (socket, _asked) = Socket::listen(&path, "first\n".into()).unwrap();
        let mut lines = BufReader::new(UnixStream::connect(&path).unwrap()).lines();
        assert_eq!(lines.next().unwrap().unwrap(), "first");
        // A shell is welcomed under the lock that saying takes, so once it
        // has heard the first line, a change can't pass it by.
        socket.say("first\n".into());
        socket.say("second\n".into());
        assert_eq!(lines.next().unwrap().unwrap(), "second");
        std::fs::remove_dir_all(&folder).unwrap();
    }
}
