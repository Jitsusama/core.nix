//! The intake: one OTLP/JSON export request a line, on a Unix socket.
//!
//! Each sender gets a thread of its own, which sleeps until it writes, so
//! nothing here wakes while nobody sends. The socket answers nothing, so a
//! sender never waits for the store.

use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::otlp;
use crate::pressure;
use crate::store::Store;

/// The longest line taken. A sender batches a second's spans into a line,
/// which is kilobytes; past this it's taken to be lost, and its connection
/// is closed rather than held in memory.
const LONGEST: u64 = 4 << 20;

/// Takes spans on `socket` into `store` until the process is stopped,
/// with the waits the kernel counts below `root`.
///
/// # Errors
///
/// When the socket can't be made.
pub fn serve(socket: &Path, store: Store, root: &Path) -> Result<(), String> {
    let store = Arc::new(Mutex::new(store));
    // Read before the socket is made, so a sender that finds the socket
    // knows the first reading is taken.
    let cgroup = fs::read_to_string("/proc/self/cgroup").unwrap_or_default();
    let watching = pressure::Watching::start(pressure::sources(root, &cgroup));
    {
        let store = Arc::clone(&store);
        thread::spawn(move || watching.run(&store));
    }
    if let Some(dir) = socket.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("making {}: {e}", dir.display()))?;
    }
    // A socket left by a collector that stopped is taken over; anything
    // else in its place is left alone.
    if fs::symlink_metadata(socket).is_ok_and(|meta| meta.file_type().is_socket()) {
        let _ = fs::remove_file(socket);
    }
    let listener = UnixListener::bind(socket)
        .map_err(|e| format!("listening on {}: {e}", socket.display()))?;
    fs::set_permissions(socket, fs::Permissions::from_mode(0o600))
        .map_err(|e| format!("keeping {} to its account: {e}", socket.display()))?;
    for stream in listener.incoming().flatten() {
        let store = Arc::clone(&store);
        thread::spawn(move || take(stream, &store));
    }
    Ok(())
}

fn take(stream: UnixStream, store: &Mutex<Store>) {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        match (&mut reader).take(LONGEST).read_line(&mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) if !line.ends_with('\n') && line.len() as u64 >= LONGEST => {
                eprintln!("dropped a sender whose line passed {LONGEST} bytes");
                return;
            }
            Ok(_) => {}
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match otlp::spans(line) {
            Ok(spans) => {
                let mut store = store
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if let Err(e) = store.add(&spans, now()) {
                    eprintln!("{e}");
                }
            }
            Err(e) => eprintln!("dropped a request: {e}"),
        }
    }
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|since| i64::try_from(since.as_nanos()).ok())
        .unwrap_or(0)
}
