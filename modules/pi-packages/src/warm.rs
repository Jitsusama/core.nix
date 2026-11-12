//! Warming each live pi's jiti cache for a version before it is swapped
//! in, so no pi pays for the transpiling at its next start.
//!
//! jiti keys what it transpiled by the source's path (jiti.rs), and each
//! version sits at a path of its own, so the first start of every pi
//! after a swap transpiled every file again: on 12 November that took a
//! start 15.3 s, and the next one 1.4 s, with the hosts and every
//! reloading session doing it at once after each of 31 swaps.
//!
//! The warm runs the pis themselves, one at a time: each node and `cli.js`
//! a live pi said it runs (loaded.rs), in RPC mode with nothing to read,
//! which loads every extension and ends. It is given the new version and
//! each other package at its `current`, its own temporary folder, so the
//! entries land where that pi reads them, and scratch state, so it joins
//! no mesh and touches no session. Running pi itself rather than jiti
//! keeps the names and options exactly as pi gives them. A warm that
//! fails or overruns is said, and the swap goes ahead: it only costs the
//! start it would have saved.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::json;

use crate::config::{Config, Package};
use crate::loaded::{self, Pi};
use crate::version::Place;

/// The longest one pi's warm may take. A cold one took 15.3 s on a
/// loaded machine on 12 November; past this it is stopped and the swap
/// goes ahead.
pub const LONGEST: Duration = Duration::from_secs(180);

/// How often a running warm is looked at.
const LOOK: Duration = Duration::from_millis(100);

/// Each pi a live process said it is, across every package, once each.
#[must_use]
pub fn pis(config: &Config) -> Vec<Pi> {
    let mut pis: Vec<Pi> = config
        .packages
        .iter()
        .flat_map(|package| loaded::read(&Place::of(config, package)).pis().to_vec())
        .collect();
    pis.sort();
    pis.dedup();
    pis
}

/// Warm each of `pis` for `version` of `package`, one at a time, saying
/// why each that failed did.
#[must_use]
pub fn warm(
    config: &Config,
    package: &Package,
    version: &Path,
    pis: &[Pi],
    env: &[(String, String)],
) -> Vec<String> {
    let place = Place::of(config, package);
    let packages = packages_with(config, package, version);
    pis.iter()
        .filter_map(|pi| {
            warm_one(&place, &packages, pi, env)
                .err()
                .map(|why| format!("{}: {why}", pi.bundle.display()))
        })
        .collect()
}

/// Every package as a pi will load it after the swap: `package` at
/// `version`, each other at where its `current` leads.
fn packages_with(config: &Config, package: &Package, version: &Path) -> Vec<PathBuf> {
    config
        .packages
        .iter()
        .filter_map(|each| {
            if each.key == package.key {
                fs::canonicalize(version).ok()
            } else {
                fs::canonicalize(Place::of(config, each).current()).ok()
            }
        })
        .collect()
}

fn warm_one(
    place: &Place,
    packages: &[PathBuf],
    pi: &Pi,
    env: &[(String, String)],
) -> Result<(), String> {
    let scratch = place.dir.join(".warming");
    let _ = fs::remove_dir_all(&scratch);
    let made = prepare(&scratch, packages);
    let ran = made.and_then(|()| run(&scratch, pi, env));
    // Nothing of the scratch state is wanted once the pi has ended.
    let _ = fs::remove_dir_all(&scratch);
    ran
}

/// The scratch state a warm runs in: an agent folder whose settings name
/// the packages, and empty state, data, cache, config and runtime folders.
fn prepare(scratch: &Path, packages: &[PathBuf]) -> Result<(), String> {
    let shown = |error: std::io::Error| format!("{}: {error}", scratch.display());
    for folder in ["agent", "state", "data", "cache", "config", "runtime"] {
        fs::create_dir_all(scratch.join(folder)).map_err(shown)?;
    }
    fs::set_permissions(scratch.join("runtime"), fs::Permissions::from_mode(0o700))
        .map_err(shown)?;
    let settings = json!({ "packages": packages });
    fs::write(scratch.join("agent/settings.json"), settings.to_string()).map_err(shown)
}

fn run(scratch: &Path, pi: &Pi, env: &[(String, String)]) -> Result<(), String> {
    let scratched = [
        ("PI_CODING_AGENT_DIR", "agent"),
        ("XDG_STATE_HOME", "state"),
        ("XDG_DATA_HOME", "data"),
        ("XDG_CACHE_HOME", "cache"),
        ("XDG_CONFIG_HOME", "config"),
        ("XDG_RUNTIME_DIR", "runtime"),
    ];
    let mut child = Command::new(&pi.node)
        .arg(pi.bundle.join("cli.js"))
        .args(["--mode", "rpc", "--no-session"])
        // Not the temporary folder itself: jiti ignores TMPDIR when it is
        // the working folder.
        .current_dir(scratch)
        .env_clear()
        .envs(
            env.iter()
                .filter(|(key, _)| {
                    key != "TMPDIR" && !scratched.iter().any(|(name, _)| name == key)
                })
                .map(|(key, value)| (key, value)),
        )
        .envs(
            scratched
                .iter()
                .map(|(key, folder)| (key, scratch.join(folder))),
        )
        .env("TMPDIR", &pi.tmp)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("{}: {error}", pi.node.display()))?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => return Err(format!("pi ended with {status}")),
            Ok(None) if started.elapsed() > LONGEST => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "pi ran past {} s and was stopped",
                    LONGEST.as_secs()
                ));
            }
            Ok(None) => thread::sleep(LOOK),
            Err(error) => return Err(format!("pi could not be waited for: {error}")),
        }
    }
}
