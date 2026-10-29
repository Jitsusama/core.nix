//! What Joel asks cleared now: a folder whose builds, caches and waste he
//! wants gone without waiting for the disk to fill or for them to sit idle.
//! `disk-guard clear` leaves an ask in the guard's state, one file per ask
//! so two asks never race over one file, and the daemon takes each on its
//! next step. An ask stays until its pass ends, so whoever writes the
//! status file sees the folder as clearing until then.

use crate::state;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The folder the asks sit in, in a guard's state folder.
pub const DIR: &str = "clears";

/// One ask: the folder, and when it was asked, in wall seconds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ask {
    pub path: PathBuf,
    pub at: f64,
}

/// Asks for `path` to be cleared.
pub fn ask(state: &Path, path: &Path, now: f64) -> io::Result<()> {
    let dir = state.join(DIR);
    fs::create_dir_all(&dir)?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    let file = dir.join(format!("{nanos}.{}.json", std::process::id()));
    state::save(
        &file,
        &Ask {
            path: path.to_owned(),
            at: now,
        },
    )
}

/// Every ask waiting or under way, oldest first, each with its file. One
/// that won't read is left out, and stays for whoever can read it.
pub fn asked(state: &Path) -> io::Result<Vec<(PathBuf, Ask)>> {
    let entries = match fs::read_dir(state.join(DIR)) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        entries => entries?,
    };
    let mut asks = Vec::new();
    for entry in entries {
        let file = entry?.path();
        // A half-written ask is named with a dot, and isn't one yet.
        let named = file.file_name().and_then(|name| name.to_str());
        if named.is_none_or(|name| name.starts_with('.') || !name.ends_with(".json")) {
            continue;
        }
        if let Ok(Some(ask)) = state::load::<Ask>(&file) {
            asks.push((file, ask));
        }
    }
    asks.sort_by(|(_, a), (_, b)| a.at.total_cmp(&b.at));
    Ok(asks)
}

/// The folders being cleared, for marking holders.
pub fn clearing(state: &Path) -> Vec<PathBuf> {
    asked(state)
        .unwrap_or_default()
        .into_iter()
        .map(|(_, ask)| ask.path)
        .collect()
}

/// Ends an ask once its pass has.
pub fn done(file: &Path) -> io::Result<()> {
    match fs::remove_file(file) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        removed => removed,
    }
}
