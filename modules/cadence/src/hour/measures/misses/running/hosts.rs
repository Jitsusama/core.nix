//! Each pi host as systemd and its process say it runs: the runtime it
//! loads its pi from, when it started, and whether it was stopped.

use std::path::Path;

use super::Listed;
use crate::command;

/// The runtime each pi host loads its pi from and when it started, each
/// `None` where it couldn't be read, and whether it was stopped. A host is
/// the main process of a unit core's host module names
/// `pi-host-REALM-KIND`; the conversations it starts are read only for its
/// program, in `conversations`. A host stopped comes back on the
/// runtime its unit names, so it's judged by that rather than left out.
/// One failed or restarting may never load that runtime, as when it
/// can't start on it, so what it loads can't be told.
pub(super) fn hosts() -> Option<Vec<Listed>> {
    let units = command(
        "systemctl",
        &[
            "--user",
            "list-units",
            "--type=service",
            "--all",
            "--no-legend",
            "--plain",
            "pi-host-*",
        ],
    )
    .ok()?;
    Some(
        units
            .lines()
            .filter_map(listed_host)
            .map(|(unit, stopped)| {
                let shown = command(
                    "systemctl",
                    &[
                        "--user",
                        "show",
                        "--timestamp=unix",
                        "-p",
                        "MainPID",
                        "-p",
                        "ExecMainStartTimestamp",
                        "-p",
                        "ActiveEnterTimestamp",
                        "-p",
                        "Environment",
                        unit,
                    ],
                )
                .unwrap_or_default();
                let (pid, started) = shown_host(&shown);
                match pid {
                    Some(pid) => {
                        let runtime = std::fs::read(format!("/proc/{pid}/environ"))
                            .ok()
                            .and_then(|environ| runtime(&environ));
                        (runtime, started, false)
                    }
                    None if stopped => (unit_runtime(&shown), None, true),
                    None => (None, None, false),
                }
            })
            .collect(),
    )
}

/// The host unit a line of `systemctl list-units --all` names, and whether
/// it was stopped, unless it's one gone from the config and stopped: a host
/// still running after its unit's file went still loads its runtime, so
/// it's read.
pub(super) fn listed_host(line: &str) -> Option<(&str, bool)> {
    let mut fields = line.split_whitespace();
    let unit = fields.next().filter(|unit| host_unit(unit))?;
    let (load, active) = (fields.next(), fields.next());
    let stopped = active == Some("inactive");
    (load == Some("loaded") || !stopped).then_some((unit, stopped))
}

/// A host unit's main process and when it started, from `systemctl show`:
/// when that process began, before it opened its program through
/// `current`, rather than when it said it was ready, which a host does
/// only once its socket listens, minutes later under load.
pub(super) fn shown_host(shown: &str) -> (Option<u32>, Option<i64>) {
    let field = |name: &str| {
        shown
            .lines()
            .find_map(|line| line.strip_prefix(name)?.strip_prefix('='))
    };
    let pid = field("MainPID")
        .and_then(|pid| pid.trim().parse().ok())
        .filter(|&pid| pid != 0);
    let stamp = |name: &str| {
        field(name)
            .and_then(|at| at.trim().strip_prefix('@')?.parse().ok())
            .filter(|&at| at > 0)
    };
    let started = stamp("ExecMainStartTimestamp").or_else(|| stamp("ActiveEnterTimestamp"));
    (pid, started)
}

/// Whether `unit` is a pi host's, `pi-host-REALM-KIND.service`, and not one
/// of the units beside them, such as `pi-host-down@`.
pub(super) fn host_unit(unit: &str) -> bool {
    unit.strip_prefix("pi-host-")
        .and_then(|rest| rest.strip_suffix(".service"))
        .is_some_and(|named| !named.contains('@') && named.contains('-'))
}

/// The runtime a pi host loads its pi from, as the store path its
/// `PI_PACKAGE_DIR` lies in: a host's node may come from a newer runtime
/// while it still loads the older one's pi, as the background host did from
/// 20:24 on 19 November.
pub(super) fn runtime(environ: &[u8]) -> Option<std::path::PathBuf> {
    let dir = environ
        .split(|&b| b == 0)
        .find_map(|var| var.strip_prefix(b"PI_PACKAGE_DIR="))?;
    store_root(std::str::from_utf8(dir).ok()?)
}

/// The runtime a host's unit starts it on, from its `Environment` as
/// `systemctl show` gives it, for a host with no process to read.
pub(super) fn unit_runtime(shown: &str) -> Option<std::path::PathBuf> {
    let environment = shown
        .lines()
        .find_map(|line| line.strip_prefix("Environment="))?;
    let dir = environment
        .split_whitespace()
        .find_map(|var| var.trim_matches('"').strip_prefix("PI_PACKAGE_DIR="))?;
    store_root(dir)
}

/// The store path `dir` lies in, whatever it's named: what makes it a host's
/// runtime is the copies of its patches it keeps, which `applied` reads.
fn store_root(dir: &str) -> Option<std::path::PathBuf> {
    let dir = Path::new(dir);
    // `..` would name one store path while pi loads from another.
    if dir
        .components()
        .any(|part| part == std::path::Component::ParentDir)
    {
        return None;
    }
    let name = dir
        .strip_prefix("/nix/store")
        .ok()?
        .components()
        .next()?
        .as_os_str()
        .to_str()?;
    let (hash, _) = name.split_once('-')?;
    (hash.len() == 32).then(|| Path::new("/nix/store").join(name))
}
