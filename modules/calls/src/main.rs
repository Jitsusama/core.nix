//! Says whether a call is on, for whatever mustn't cut one, such as an
//! update with nobody at it.
//!
//! ```text
//! a-call-is-on [RUNTIMES]
//! ```
//!
//! It exits 0 while a call is on, 1 while none is, and 2 when it can't
//! tell. Each account with PipeWire running has its socket in its runtime
//! folder under RUNTIMES, /run/user unless named, and pw-dump reads each
//! account's graph as that account, since PipeWire is its own. A program
//! recording there counts once it still records two seconds later, as
//! src/lib.rs counts a call.

use calls::{Recording, STARTS_AFTER};
use serde_json::Value;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread;

/// The socket a runtime folder holds while its account's PipeWire runs.
const SOCKET: &str = "pipewire-0";

fn main() -> ExitCode {
    let runtimes = std::env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from("/run/user"), PathBuf::from);
    match on(&runtimes) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(why) => {
            eprintln!("a-call-is-on: {why}");
            ExitCode::from(2)
        }
    }
}

/// Whether a call is on in any account's PipeWire under `runtimes`.
fn on(runtimes: &Path) -> Result<bool, String> {
    let folders = with_pipewire(runtimes)?;
    let first = folders
        .iter()
        .map(|folder| recording(folder))
        .collect::<Result<Vec<_>, _>>()?;
    if !first.iter().any(Recording::any) {
        return Ok(false);
    }
    thread::sleep(STARTS_AFTER);
    for (folder, before) in folders.iter().zip(&first) {
        if before.any() && before.goes_on_in(&recording(folder)?) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The runtime folders under `runtimes` whose PipeWire runs; none when
/// there are no runtimes at all, as with nobody logged in.
fn with_pipewire(runtimes: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = match fs::read_dir(runtimes) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("{}: {e}", runtimes.display())),
    };
    let mut folders: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|folder| folder.join(SOCKET).exists())
        .collect();
    folders.sort();
    Ok(folders)
}

/// What records in the PipeWire whose socket is in `folder`, read as the
/// account the folder is.
fn recording(folder: &Path) -> Result<Recording, String> {
    let owner = fs::metadata(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let mut dump = Command::new("pw-dump");
    dump.arg("--no-colors")
        .env("XDG_RUNTIME_DIR", folder)
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    // Only root may become another account; anyone else reads their own.
    if fs::metadata("/proc/self").is_ok_and(|me| me.uid() != owner.uid()) {
        dump.uid(owner.uid()).gid(owner.gid());
    }
    let out = dump
        .output()
        .map_err(|e| format!("pw-dump for {}: {e}", folder.display()))?;
    if !out.status.success() {
        return Err(format!(
            "pw-dump for {} ended {}",
            folder.display(),
            out.status
        ));
    }
    let graph: Value = serde_json::from_slice(&out.stdout)
        .map_err(|e| format!("pw-dump for {} printed no graph: {e}", folder.display()))?;
    let mut recording = Recording::default();
    recording.take(&graph);
    Ok(recording)
}
