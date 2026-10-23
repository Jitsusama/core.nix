//! Linking the Nix store's identical files together with `nix-store
//! --optimise`, which Nix does safely: each file is linked to its twin
//! through a temporary link renamed over it, and a stopped run unwinds,
//! keeping what it linked. A run reads every file not yet linked, which
//! on a store never optimised is all of it, so it goes in a scope of its
//! own: its page cache and its table of linked files stay out of the
//! guard's memory and out of everyone else's cache.

use crate::commands::{self, Job, Ran};
use regex_lite::Regex;
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::path::Path;
use std::sync::LazyLock;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

/// A run that finished: when it began, in wall seconds, and what Nix said
/// it freed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Deduped {
    pub at: f64,
    pub freed: u64,
}

/// How long after a run that freed little the next one waits.
const BACKED_OFF: f64 = 30.0 * 86400.0;

/// How long a run may take. One on a store never optimised reads all of
/// it, at idle priority behind whatever else runs.
const TIMEOUT: Duration = Duration::from_hours(12);

/// How long a stopped run has to unwind before it's killed.
const STOP_GRACE: Duration = Duration::from_secs(30);

/// Its scope's limits. Nix keeps a set of every inode it has linked, some
/// 40 bytes a file in libstdc++, so about 400 MB for a store of nine
/// million files (a sample's estimate, not a measured run). Past the soft
/// limit the kernel takes back its page cache first; past the hard one
/// the run is killed, never the guard. Its weights put it behind
/// everything else for the processor, and for the disk where the
/// scheduler weighs at all.
const SCOPE: [&str; 4] = [
    "MemoryHigh=1G",
    "MemoryMax=2G",
    "CPUWeight=idle",
    "IOWeight=1",
];

/// Nix's last words on a finished run, sizes shown to a tenth of a unit.
static FREED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?m)^(?P<size>\d+(?:\.\d+)?) (?P<unit>[KMGTPE])iB freed by hard-linking \d+ files$",
    )
    .unwrap()
});

/// What Nix said a finished run freed, from what it wrote.
pub fn freed(said: &str) -> Option<u64> {
    let found = FREED.captures_iter(said).last()?;
    let size: f64 = found["size"].parse().ok()?;
    let power = i32::try_from("KMGTPE".find(&found["unit"])?).ok()? + 1;
    Some(crate::units::whole((size * 1024f64.powi(power)).round()))
}

/// Whether a run is due, `every` seconds after the last one, or a month
/// after one that freed under 1% of the filesystem's `total`.
pub fn due(last: Option<&Deduped>, now: f64, every: f64, total: u64) -> bool {
    let Some(last) = last else {
        return true;
    };
    let wait = if last.freed < total / 100 {
        every.max(BACKED_OFF)
    } else {
        every
    };
    now - last.at >= wait
}

/// Links the store's identical files, confined by `systemd_run` to a
/// scope where there is one, with `nix-store` from beside `nix`, until
/// `stop` is set: what Nix said it freed, or nothing for a run stopped
/// before it finished.
pub fn optimise(
    systemd_run: Option<&Path>,
    nix: &Path,
    stop: &AtomicBool,
) -> Result<Option<u64>, String> {
    let mut argv: Vec<OsString> = Vec::new();
    if let Some(systemd_run) = systemd_run {
        argv.push(systemd_run.into());
        argv.extend(
            [
                "--scope",
                "--quiet",
                "--collect",
                "--description=disk-guard: linking the Nix store's identical files",
            ]
            .map(OsString::from),
        );
        for property in SCOPE {
            argv.extend(["-p".into(), property.into()]);
        }
        argv.push("--".into());
    }
    argv.extend([nix.with_file_name("nix-store").into(), "--optimise".into()]);
    let job = Job {
        argv,
        cwd: None,
        env: Vec::new(),
        timeout: TIMEOUT,
        limit: 1 << 20,
        idle: true,
    };
    match commands::stoppable(&job, stop, STOP_GRACE) {
        Err(error) => Err(format!(
            "starting {}: {error}",
            Path::new(&job.argv[0]).display()
        )),
        Ok(Ran::Stopped) => Ok(None),
        Ok(Ran::TimedOut) => Err(format!("nix-store --optimise ran past {TIMEOUT:?}")),
        Ok(Ran::Finished(output)) => {
            let said = String::from_utf8_lossy(&output.stderr);
            if let (Some(0), Some(bytes)) = (output.status, freed(&said)) {
                Ok(Some(bytes))
            } else {
                let last = said.lines().rev().find(|line| !line.trim().is_empty());
                Err(format!(
                    "nix-store --optimise failed: {}",
                    last.unwrap_or("it said nothing")
                ))
            }
        }
    }
}
