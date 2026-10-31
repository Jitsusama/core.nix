//! Building the machine's system, switching to it, telling whether the
//! machine is whole afterwards, and going back to the system it left when
//! it isn't. A switch is the system's own: NixOS's switch-to-configuration
//! and nix-darwin's activate, after the profile points at it, as
//! nixos-rebuild and darwin-rebuild do.

use crate::config::{Config, Kind};
use crate::line::Applied;
use crate::run::{self, Failure};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

/// How long between tries of the health commands.
const HEALTH_EVERY: Duration = Duration::from_secs(2);

/// What a switch did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Switched {
    pub applied: Applied,
    pub restart: Option<String>,
    pub restarted: Vec<String>,
}

/// Builds `flake`, keeping it from being collected until the next build,
/// and gives the system's path once it's one this machine's kind can run.
pub fn build(config: &Config, flake: &str) -> Result<PathBuf, Failure> {
    let mut nix = run::command(&config.tools.nix, &config.state);
    nix.args([
        "--extra-experimental-features",
        "nix-command flakes",
        "build",
        "--print-out-paths",
        "--out-link",
    ])
    .arg(config.building_link());
    let printed = run::output(nix, "building the system", [flake])?;
    let system = PathBuf::from(printed.lines().last().unwrap_or_default());
    let needs: &[&str] = match config.kind {
        Kind::Nixos => &["bin/switch-to-configuration", "nixos-version"],
        Kind::Darwin => &["activate", "darwin-version"],
    };
    if needs.iter().all(|n| system.join(n).exists()) {
        Ok(system)
    } else {
        Err(Failure::new(format!(
            "{} isn't a system this machine can switch to",
            system.display()
        )))
    }
}

/// The system the profile points at, which the machine runs or boots.
pub fn current(config: &Config) -> Option<PathBuf> {
    fs::canonicalize(&config.profile).ok()
}

/// Points the profile at `system` and switches the machine to it: live,
/// unless on NixOS its kernel differs from the booted one, when it's only
/// made the system the machine starts.
pub fn switch(config: &Config, system: &Path) -> Result<Switched, Failure> {
    let previous = current(config);
    set_profile(config, system)?;
    match config.kind {
        Kind::Nixos => {
            let booted = &config.booted;
            let next_boot = booted.exists() && !same_boot(booted, system);
            let mode = if next_boot { "boot" } else { "switch" };
            let said = run::output(
                run::command(&system.join("bin/switch-to-configuration"), &config.state),
                "switching",
                [mode],
            )?;
            Ok(Switched {
                applied: if next_boot {
                    Applied::NextBoot
                } else {
                    Applied::Now
                },
                restart: next_boot.then(|| "its kernel changed".to_owned()),
                restarted: restarted_units(&said),
            })
        }
        Kind::Darwin => {
            let restarted = previous
                .as_deref()
                .map(|p| changed_daemons(p, system))
                .unwrap_or_default();
            run::output(
                run::command(&system.join("activate"), &config.state),
                "switching",
                std::iter::empty::<&str>(),
            )?;
            Ok(Switched {
                applied: Applied::Now,
                restart: None,
                restarted,
            })
        }
    }
}

/// Switches back to `system`, the one the machine left.
pub fn go_back(config: &Config, system: &Path) -> Result<(), Failure> {
    set_profile(config, system)?;
    let program = match config.kind {
        Kind::Nixos => system.join("bin/switch-to-configuration"),
        Kind::Darwin => system.join("activate"),
    };
    let args: &[&str] = match config.kind {
        Kind::Nixos => &["switch"],
        Kind::Darwin => &[],
    };
    run::output(
        run::command(&program, &config.state),
        "switching back",
        args,
    )
    .map(|_| ())
}

/// Runs the health commands until each has succeeded once in a row of
/// tries, or `health_within` seconds pass.
pub fn healthy(config: &Config) -> Result<(), Failure> {
    let deadline = Instant::now() + Duration::from_secs(config.health_within);
    loop {
        let mut last = None;
        for check in &config.health {
            let Some((program, args)) = check.split_first() else {
                continue;
            };
            if let Err(failure) = run::output(
                run::command(Path::new(program), &config.state),
                &format!("the health check `{}`", check.join(" ")),
                args,
            ) {
                last = Some(failure);
                break;
            }
        }
        match last {
            None => return Ok(()),
            Some(failure) if Instant::now() >= deadline => return Err(failure),
            Some(_) => thread::sleep(HEALTH_EVERY),
        }
    }
}

fn set_profile(config: &Config, system: &Path) -> Result<(), Failure> {
    let mut nix_env = run::command(&config.tools.nix_env, &config.state);
    nix_env.arg("--profile").arg(&config.profile).arg("--set");
    run::output(nix_env, "pointing the profile at the system", [system]).map(|_| ())
}

/// Whether two NixOS systems boot the same kernel, initrd and parameters,
/// so one can be switched to live from the other.
fn same_boot(a: &Path, b: &Path) -> bool {
    ["kernel", "initrd"]
        .iter()
        .all(|f| fs::canonicalize(a.join(f)).ok() == fs::canonicalize(b.join(f)).ok())
        && fs::read_to_string(a.join("kernel-params")).ok()
            == fs::read_to_string(b.join("kernel-params")).ok()
}

/// The units switch-to-configuration says it restarted.
pub fn restarted_units(said: &str) -> Vec<String> {
    said.lines()
        .filter_map(|l| l.strip_prefix("restarting the following units: "))
        .flat_map(|units| units.split(", "))
        .map(|u| u.trim().to_owned())
        .filter(|u| !u.is_empty())
        .collect()
}

/// The labels of the launchd daemons whose definition differs between two
/// nix-darwin systems, or that only the new one has: those activate
/// restarts, such as the Linux VM when its guest changed.
pub fn changed_daemons(old: &Path, new: &Path) -> Vec<String> {
    let old = daemons(old);
    daemons(new)
        .into_iter()
        .filter(|(label, plist)| old.get(label) != Some(plist))
        .map(|(label, _)| label)
        .collect()
}

fn daemons(system: &Path) -> BTreeMap<String, Option<String>> {
    let Ok(entries) = fs::read_dir(system.join("Library/LaunchDaemons")) else {
        return BTreeMap::new();
    };
    entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let label = name.strip_suffix(".plist")?.to_owned();
            Some((label, fs::read_to_string(e.path()).ok()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_units_a_switch_restarted_are_read_from_what_it_said() {
        let said = "stopping the following units: a.service\n\
                    restarting the following units: nix-daemon.service, sshd.service\n\
                    starting the following units: b.service\n";
        assert_eq!(
            restarted_units(said),
            ["nix-daemon.service", "sshd.service"]
        );
    }

    #[test]
    fn a_mac_daemon_whose_definition_changed_is_one_activate_restarts() {
        let old = tempfile::tempdir().unwrap();
        let new = tempfile::tempdir().unwrap();
        daemon(old.path(), "org.nixos.linux-builder-vz", "guest one");
        daemon(new.path(), "org.nixos.linux-builder-vz", "guest two");
        daemon(old.path(), "org.nixos.nix-daemon", "same");
        daemon(new.path(), "org.nixos.nix-daemon", "same");
        daemon(new.path(), "org.jitsusama.new", "new");
        assert_eq!(
            changed_daemons(old.path(), new.path()),
            ["org.jitsusama.new", "org.nixos.linux-builder-vz"]
        );
    }

    fn daemon(system: &Path, label: &str, text: &str) {
        let dir = system.join("Library/LaunchDaemons");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(format!("{label}.plist")), text).unwrap();
    }
}
