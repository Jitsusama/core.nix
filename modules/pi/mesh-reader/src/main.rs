//! The mesh reader: each folder of the mesh's read once for every pi
//! process that asks (src/lib.rs says why and how).
//!
//! systemd hands it its listening socket (`LISTEN_FDS`); `--socket`
//! binds one itself, as a check does. It reads only under each `--root`.
//! What it can't do goes to the journal, through stderr. Its help and
//! refusals are the terminal kit's (docs/commands.md).

use std::env;
use std::fs;
use std::io::{self, Write as _};
use std::os::fd::FromRawFd;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use pi_mesh_reader::reader::{self, BACKSTOP, Reader};
use pi_mesh_reader::wire::Line;
use terminal_kit::{Exit, Failure, Help};

/// The first socket systemd hands a service it starts.
const FIRST_PASSED_FD: i32 = 3;

/// The most clients answered at once. Each pi process follows a folder or
/// two, and each client takes two threads, so a process opening
/// connections without end would otherwise run it out of threads.
const MOST_CLIENTS: usize = 512;

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

/// What its command line names.
struct Asked {
    socket: Option<PathBuf>,
    roots: Vec<PathBuf>,
}

fn help() -> Help {
    Help::new(
        "pi-mesh-reader",
        "reads each folder of the mesh's once for every pi process that asks",
    )
    .option(
        "--root PATH",
        "read folders under PATH alone; given again, under any of them",
    )
    .option(
        "--socket PATH",
        "listen at PATH, bound afresh, rather than on the socket systemd hands it",
    )
    .example(
        "systemctl --user start pi-mesh-reader.socket",
        "listens where pi processes look for it",
    )
    .exit(Exit::Usage, "a word it doesn't take, or no root")
    .exit(
        Exit::Unavailable,
        "systemd handed it no socket, and none was named",
    )
}

fn main() -> std::process::ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let asked = asked(&context.args)
            .map_err(|why| Failure::new(Exit::Usage, why).try_this("pi-mesh-reader --help"))?;
        let listener = listener(asked.socket.as_deref()).map_err(|error| {
            Failure::new(Exit::Unavailable, "there's nothing to listen on")
                .because(error.to_string())
        })?;
        serve(&listener, &Reader::new(asked.roots, BACKSTOP));
        Ok(())
    })
}

/// Each client answered, on a thread of its own, up to the ceiling.
fn serve(listener: &UnixListener, reader: &Arc<Reader>) {
    let clients = Arc::new(AtomicUsize::new(0));
    for client in listener.incoming() {
        match client {
            Ok(mut stream) => {
                let Some(place) = Place::take(&clients) else {
                    let why = "it answers no more clients now".to_string();
                    let _ = writeln!(stream, "{}", Line::No { why }.text());
                    continue;
                };
                let reader = Arc::clone(reader);
                let started = thread::Builder::new().name("client".into()).spawn(move || {
                    reader::serve(&reader, &stream);
                    drop(place);
                });
                if let Err(error) = started {
                    eprintln!("pi-mesh-reader: a client couldn't be answered: {error}");
                }
            }
            Err(error) => eprintln!("pi-mesh-reader: a client couldn't be taken: {error}"),
        }
    }
}

/// The socket and roots the command line names.
fn asked(args: &[String]) -> Result<Asked, String> {
    let mut asked = Asked {
        socket: None,
        roots: Vec::new(),
    };
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--socket" => {
                asked.socket = Some(PathBuf::from(args.next().ok_or("--socket takes a path")?));
            }
            "--root" => asked
                .roots
                .push(PathBuf::from(args.next().ok_or("--root takes a path")?)),
            _ => return Err(format!("it takes no {arg}")),
        }
    }
    if asked.roots.is_empty() {
        return Err("it reads nothing without a --root".into());
    }
    Ok(asked)
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
