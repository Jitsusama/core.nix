//! The shell's socket: each shell that connects is sent the whole state at
//! once, then the whole state again each time it changes, one line each.
//! What a shell writes, a line at a time, is handed on as it comes
//! (ask.rs reads it).
//!
//! The same line is left in a file beside the socket, which the shell
//! watches for what it shows: a file that isn't there is no account, said
//! without the shell trying a socket nobody listens on. It's replaced
//! whole each time, through a new file renamed over it, so it's never read
//! half written. The new file is made in a `.writing` folder beside it:
//! the shell watches the folder the file is in, and a file made and
//! written there woke it three times a minute more than the rename alone
//! (2026-10-31).
//!
//! A shell that stops reading is let go after a second rather than holding
//! everyone else up; it connects again when it restarts.

use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

#[derive(Default)]
struct Shared {
    line: String,
    shells: Vec<UnixStream>,
}

/// The socket, what it last said, and the file it leaves it in.
#[derive(Clone)]
pub struct Socket {
    shared: Arc<Mutex<Shared>>,
    told: Arc<PathBuf>,
}

impl Socket {
    /// Listens at `path`, replacing a socket an earlier run left there,
    /// and starts sending `first` to each shell that connects, leaving it
    /// in `told` too. What the shells write is sent on `asks`, a line each,
    /// beside whatever else asks this program.
    pub fn listen(
        path: &Path,
        told: &Path,
        first: String,
        asks: Sender<String>,
    ) -> io::Result<Socket> {
        if path.exists() {
            fs::remove_file(path)?;
        }
        let listener = UnixListener::bind(path)?;
        leave(told, &first)?;
        let socket = Socket {
            shared: Arc::new(Mutex::new(Shared {
                line: first,
                shells: Vec::new(),
            })),
            told: Arc::new(told.to_owned()),
        };
        let accepting = socket.clone();
        thread::spawn(move || {
            for shell in listener.incoming().flatten() {
                accepting.welcome(shell, &asks);
            }
        });
        Ok(socket)
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
        // A file that can't be written leaves the shell what it last read,
        // and the next change tries again.
        if let Err(error) = leave(&self.told, &line) {
            eprintln!("<4>couldn't write {}: {error}", self.told.display());
        }
        shared.line = line;
    }
}

/// Leaves `line` in the file `told`, replacing it whole, written first in
/// the `.writing` folder beside it, where nobody watches.
fn leave(told: &Path, line: &str) -> io::Result<()> {
    let (Some(folder), Some(name)) = (told.parent(), told.file_name()) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no file to leave it in",
        ));
    };
    let writing = folder.join(".writing");
    fs::create_dir_all(&writing)?;
    let mut new = name.to_owned();
    new.push(".new");
    let new = writing.join(new);
    fs::write(&new, line)?;
    fs::rename(&new, told)
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::{self, Receiver};

    use super::*;

    /// Listens as the program does, with what the shells write on a
    /// channel of the test's own.
    fn listening(path: &Path, told: &Path) -> (Socket, Receiver<String>) {
        let (asks, asked) = mpsc::channel();
        (
            Socket::listen(path, told, "first\n".into(), asks).unwrap(),
            asked,
        )
    }

    fn folder(test: &str) -> std::path::PathBuf {
        let folder = std::env::temp_dir().join(format!("google-{test}-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn what_a_shell_writes_is_handed_on_a_line_at_a_time() {
        let folder = folder("asks");
        let path = folder.join("google.sock");
        let told = folder.join("google.json");
        let (_socket, asked) = listening(&path, &told);
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
        let told = folder.join("google.json");
        let (socket, _asked) = listening(&path, &told);
        let mut lines = BufReader::new(UnixStream::connect(&path).unwrap()).lines();
        assert_eq!(lines.next().unwrap().unwrap(), "first");
        // A shell is welcomed under the lock that saying takes, so once it
        // has heard the first line, a change can't pass it by.
        socket.say("first\n".into());
        socket.say("second\n".into());
        assert_eq!(lines.next().unwrap().unwrap(), "second");
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn the_file_beside_it_holds_the_state_whole_and_each_change() {
        let folder = folder("told");
        let path = folder.join("google.sock");
        let told = folder.join("google.json");
        fs::write(&told, "left by an earlier run\n").unwrap();
        let (socket, _asked) = listening(&path, &told);
        assert_eq!(fs::read_to_string(&told).unwrap(), "first\n");
        socket.say("second\n".into());
        assert_eq!(fs::read_to_string(&told).unwrap(), "second\n");
        let left: Vec<_> = fs::read_dir(&folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(left.len(), 3, "{left:?}");
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn the_file_is_written_where_nobody_watches_then_renamed_in() {
        let folder = folder("aside");
        let path = folder.join("google.sock");
        let told = folder.join("google.json");
        let (socket, _asked) = listening(&path, &told);
        socket.say("second\n".into());
        let mut left: Vec<_> = fs::read_dir(&folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        left.sort();
        assert_eq!(left, [".writing", "google.json", "google.sock"]);
        assert_eq!(fs::read_dir(folder.join(".writing")).unwrap().count(), 0);
        std::fs::remove_dir_all(&folder).unwrap();
    }
}
