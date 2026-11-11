//! The guide's commands, a file for each of the journey's places they
//! run in: host.rs on the machine writing a stick, stick.rs on the
//! stick's console and installed.rs on the machine once it's installed.
//! main.rs says which runs, and keeps those that run anywhere. Only the
//! program builds these; the library they drive is the rest of src/.

pub mod host;
pub mod installed;
pub mod stick;

use std::fs::{File, OpenOptions};
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use guide::{Fault, Secret};

pub fn reported(result: Result<(), Fault>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(fault) => {
            eprintln!("{fault}");
            ExitCode::FAILURE
        }
    }
}

/// A passphrase, as the next line of standard input.
pub fn passphrase() -> Result<Secret, Fault> {
    let mut line = zeroize::Zeroizing::new(String::new());
    std::io::stdin()
        .lock()
        .read_line(&mut line)
        .map_err(|why| Fault::unreadable("the passphrase on standard input", why))?;
    let passphrase = line.trim_end_matches(['\n', '\r']);
    if passphrase.chars().count() < 12 {
        return Err(Fault::Blocked {
            step: "write-stick",
            why: "a passphrase has to be at least 12 characters".into(),
        });
    }
    Ok(Secret::new(passphrase.to_string()))
}

/// The person's own state for the guide, made readable only by them.
pub fn own_state() -> Option<PathBuf> {
    // A service's own, which systemd names for it.
    if let Some(state) = std::env::var_os("STATE_DIRECTORY") {
        return Some(PathBuf::from(state));
    }
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| Path::new(&home).join(".local/state")))?;
    Some(state.join("guide"))
}

/// Where the writer keeps its log: the person's own state, readable only by
/// them.
pub fn writer_log() -> Option<File> {
    let directory = own_state()?;
    std::os::unix::fs::DirBuilderExt::mode(std::fs::DirBuilder::new().recursive(true), 0o700)
        .create(&directory)
        .ok()?;
    std::os::unix::fs::OpenOptionsExt::mode(OpenOptions::new().create(true).append(true), 0o600)
        .open(directory.join("log"))
        .ok()
}
