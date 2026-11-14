//! The clipboard door: the desktop's clipboard, for pi's fenced commands
//! alone (src/lib.rs says why and how).
//!
//! systemd hands it its listening socket (`LISTEN_FDS`); `--socket`
//! binds one itself, as a check does. Each line it says goes to the
//! journal, through stderr. Its help and refusals are the terminal kit's
//! (docs/commands.md).

use std::env;
use std::fs;
use std::io;
use std::os::fd::FromRawFd;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use pi_clipboard_door::door::{self, Desktop};
use pi_clipboard_door::said;
use pi_clipboard_door::wire::{Answer, Why};
use terminal_kit::{Exit, Failure, Help};

/// The first socket systemd hands a service it starts.
const FIRST_PASSED_FD: i32 = 3;

/// The most clients answered at once. Each has a thread of its own, so
/// without a ceiling a fenced command opening connections without end
/// would run the door out of threads.
const MOST_CLIENTS: usize = 32;

/// A client's place among the [`MOST_CLIENTS`], given back as it goes.
struct Place(Arc<AtomicUsize>);

impl Place {
    fn take(clients: &Arc<AtomicUsize>) -> Option<Self> {
        if clients.fetch_add(1, Ordering::SeqCst) >= MOST_CLIENTS {
            clients.fetch_sub(1, Ordering::SeqCst);
            return None;
        }
        Some(Place(Arc::clone(clients)))
    }
}

impl Drop for Place {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

fn help() -> Help {
    Help::new(
        "pi-clipboard-door",
        "the desktop's clipboard, for pi's fenced commands alone",
    )
    .option(
        "--socket PATH",
        "listen at PATH, bound afresh, rather than on the socket systemd hands it",
    )
    .example(
        "systemctl --user start pi-clipboard-door.socket",
        "listens as the fence's wl-copy and wl-paste expect",
    )
    .exit(Exit::Usage, "a word it doesn't take")
    .exit(
        Exit::Unavailable,
        "systemd handed it no socket, and none was named",
    )
}

fn main() -> std::process::ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let socket = socket(&context.args)
            .map_err(|why| Failure::new(Exit::Usage, why).try_this("pi-clipboard-door --help"))?;
        let listener = listener(socket.as_deref()).map_err(|error| {
            Failure::new(Exit::Unavailable, "there's nothing to listen on")
                .because(error.to_string())
        })?;
        serve(&listener);
        Ok(())
    })
}

/// Each client answered, on a thread of its own, up to the ceiling.
fn serve(listener: &UnixListener) {
    let clients = Arc::new(AtomicUsize::new(0));
    for client in listener.incoming() {
        match client {
            Ok(mut stream) => {
                let Some(place) = Place::take(&clients) else {
                    use std::io::Write as _;
                    let _ = writeln!(stream, "{}", Answer::No(Why::Busy));
                    eprintln!(
                        "{}",
                        said::refused(pi_clipboard_door::peer::UNKNOWN, None, Why::Busy)
                    );
                    continue;
                };
                thread::spawn(move || {
                    door::serve(stream, &Desktop);
                    drop(place);
                });
            }
            Err(error) => eprintln!("pi-clipboard-door: a client couldn't be taken: {error}"),
        }
    }
}

/// The socket the command line names, if it names one.
fn socket(args: &[String]) -> Result<Option<PathBuf>, String> {
    let mut socket = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--socket" => socket = Some(PathBuf::from(args.next().ok_or("--socket takes a path")?)),
            _ => return Err(format!("it takes no {arg}")),
        }
    }
    Ok(socket)
}

/// The socket at `socket`, bound afresh, or else the one systemd passed.
fn listener(socket: Option<&Path>) -> io::Result<UnixListener> {
    if let Some(path) = socket {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        return UnixListener::bind(path);
    }
    let ours = env::var("LISTEN_PID").ok().and_then(|pid| pid.parse().ok()) == Some(process::id());
    if !ours || env::var("LISTEN_FDS").as_deref() != Ok("1") {
        return Err(io::Error::other(
            "systemd passed no socket, and no --socket was given",
        ));
    }
    // SAFETY: systemd passed this process one listening socket, at the
    // first descriptor it hands on, and nothing else here owns it.
    Ok(unsafe { UnixListener::from_raw_fd(FIRST_PASSED_FD) })
}
