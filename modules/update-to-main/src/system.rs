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

/// Builds `flake`, its inputs overridden by `overrides`, keeping it from
/// being collected until the next build, and gives the system's path once
/// it's one this machine's kind can run. Building elsewhere, everything but
/// the light pieces is built on the builders first.
pub fn build(config: &Config, flake: &str, overrides: &[String]) -> Result<PathBuf, Failure> {
    if config.elsewhere {
        build_elsewhere(config, flake, overrides)?;
    }
    let mut nix = nix(config);
    nix.args(["build", "--print-out-paths", "--out-link"])
        .arg(config.building_link())
        .args(overrides);
    // What's left is built here with no builders, so a piece built from a
    // kept one can't go to the daemon's.
    if config.elsewhere {
        nix.args(["--builders", ""]);
    }
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

/// Nix, with the commands and flakes it's asked for.
fn nix(config: &Config) -> std::process::Command {
    let mut nix = run::command(&config.tools.nix, &config.state);
    nix.args(["--extra-experimental-features", "nix-command flakes"]);
    nix
}

/// Builds every piece of the system that nixpkgs doesn't mark light on the
/// machine's builders, with no build here allowed, so it fails rather than
/// compile here when none takes one, unless `here` lets it build them here
/// small. A builder may take only pieces that ask for what it has, as one
/// for big builds does, so the rest is built here then too. The light pieces, such as a
/// written file or a folder of links, are left for the build here, as
/// sending each one away would cost more than making it. Kept pieces go
/// first and only to their own builders. A piece built from one would
/// carry it to whichever builder took it, so it stays for the build here,
/// where the kept one now is: such pieces are a system's links, units and
/// wrappers, not what compiles.
fn build_elsewhere(config: &Config, flake: &str, overrides: &[String]) -> Result<(), Failure> {
    let mut listing = nix(config);
    listing
        .args(["build", "--dry-run", "--no-link"])
        .args(overrides)
        .arg(flake);
    let listed = listing
        .output()
        .map_err(|e| Failure::new(format!("listing what the system needs: {e}")))?;
    let said = String::from_utf8_lossy(&listed.stderr);
    if !listed.status.success() {
        return Err(Failure {
            why: format!("listing what the system needs failed ({})", listed.status),
            lines: run::tail(&said),
        });
    }
    let needed = to_build(&said);
    if needed.is_empty() {
        return Ok(());
    }
    let mut show = nix(config);
    show.args(["derivation", "show"]);
    // What the needed pieces are built from is read only when some pieces
    // are kept, since it's the whole system's graph.
    if !config.kept.names.is_empty() {
        show.arg("--recursive");
    }
    let shown = run::output(show, "reading what the system needs", &needed)?;
    let (kept, rest) = heavy(&pieces(&shown)?, &needed, &config.kept.names);
    if !kept.is_empty() {
        build_kept(config, &kept)?;
    }
    if !rest.is_empty() {
        let builders = config.builders.as_deref();
        if let Err(apart) = away(
            config,
            &rest,
            builders,
            "building the system on the machine's builders",
        ) {
            // The builders stay, so a piece only they take still goes to them.
            here(config, &rest, apart, builders, "building the rest here")?;
        }
    }
    Ok(())
}

/// Builds pieces on builders, the daemon's when none are given, with no
/// build here allowed.
fn away(
    config: &Config,
    pieces: &[String],
    builders: Option<&str>,
    what: &str,
) -> Result<(), Failure> {
    let mut away = nix(config);
    away.args(["build", "--no-link", "--max-jobs", "0"]);
    if let Some(builders) = builders {
        away.args(["--builders", builders]);
    }
    run::output(away, what, outputs(pieces)).map(|_| ())
}

/// Every output of each piece, as Nix builds them.
fn outputs(pieces: &[String]) -> impl Iterator<Item = String> + '_ {
    pieces.iter().map(|drv| format!("{drv}^*"))
}

/// Builds the kept pieces on their own builders, or, when those don't and
/// the machine may, here, small and only while it's quiet.
fn build_kept(config: &Config, kept: &[String]) -> Result<(), Failure> {
    let apart = match &config.kept.builders {
        Some(builders) => match away(
            config,
            kept,
            Some(builders),
            "building the kept pieces on their own builders",
        ) {
            Ok(()) => return Ok(()),
            Err(failure) => failure,
        },
        None => Failure {
            why: format!(
                "{} pieces may go only to builders of their own, and none are named",
                kept.len()
            ),
            lines: kept.iter().take(run::TAIL).cloned().collect(),
        },
    };
    // With no builders at all, so a kept piece goes to none of the daemon's.
    here(
        config,
        kept,
        apart,
        Some(""),
        "building the kept pieces here",
    )
}

/// Builds pieces their builders didn't here, small, when `here` lets the
/// machine and only while it's quiet; otherwise gives why they weren't.
fn here(
    config: &Config,
    pieces: &[String],
    apart: Failure,
    builders: Option<&str>,
    what: &str,
) -> Result<(), Failure> {
    let Some(here) = &config.here else {
        return Err(apart);
    };
    let busy = match load() {
        Some(load) if load < here.below_load => None,
        Some(load) => Some(format!(
            "the load here is {load:.1}, not below {}",
            here.below_load
        )),
        None => Some("the load here can't be read".to_owned()),
    };
    if let Some(busy) = busy {
        return Err(Failure {
            why: format!("{}, and {busy}, so they wait", apart.why),
            lines: apart.lines,
        });
    }
    let mut local = nix(config);
    local.args(["build", "--no-link"]);
    if let Some(builders) = builders {
        local.args(["--builders", builders]);
    }
    local.args(["--max-jobs", &here.max_jobs.to_string()]);
    local.args(["--cores", &here.cores.to_string()]);
    run::output(local, what, outputs(pieces)).map(|_| ())
}

/// The machine's load averaged over the last minute, when it can be read.
fn load() -> Option<f64> {
    let mut loads = [0.0_f64; 1];
    // SAFETY: getloadavg writes at most as many samples as it's given room
    // for, here one.
    let read = unsafe { libc::getloadavg(loads.as_mut_ptr(), 1) };
    (read == 1).then_some(loads[0])
}

/// The derivations a dry run says it would build, by their paths.
fn to_build(said: &str) -> Vec<String> {
    let mut building = false;
    let mut needed = Vec::new();
    for line in said.lines() {
        let line = line.trim();
        if line.ends_with("will be built:") {
            building = true;
        } else if line.contains("will be fetched") {
            building = false;
        } else if building
            && line.starts_with('/')
            && Path::new(line).extension().is_some_and(|e| e == "drv")
        {
            needed.push(line.to_owned());
        }
    }
    needed
}

/// What `nix derivation show` says of one derivation.
struct Piece {
    names: Vec<String>,
    light: bool,
    /// The derivations it's built from, by their paths.
    inputs: Vec<String>,
}

/// The derivations `nix derivation show` gave, by their paths. Newer Nix
/// keys them by name under `derivations` and lists what each is built from
/// under `inputs.drvs`; older keys them by path at the top, under
/// `inputDrvs`.
fn pieces(shown: &str) -> Result<BTreeMap<String, Piece>, Failure> {
    let shown: serde_json::Value = serde_json::from_str(shown)
        .map_err(|e| Failure::new(format!("reading what the system needs: {e}")))?;
    let derivations = shown.get("derivations").unwrap_or(&shown);
    let derivations = derivations
        .as_object()
        .ok_or_else(|| Failure::new("reading what the system needs: no derivations"))?;
    Ok(derivations
        .iter()
        .map(|(key, d)| {
            let names = [&d["name"], &d["env"]["pname"]]
                .iter()
                .filter_map(|n| n.as_str())
                .map(str::to_owned)
                .collect();
            let inputs = d["inputs"]["drvs"]
                .as_object()
                .or_else(|| d["inputDrvs"].as_object())
                .map(|drvs| drvs.keys().map(|k| stored(k)).collect())
                .unwrap_or_default();
            let piece = Piece {
                names,
                light: d["env"]["preferLocalBuild"].as_str() == Some("1"),
                inputs,
            };
            (stored(key), piece)
        })
        .collect())
}

/// A derivation's path, given by its path or its name in the store.
fn stored(key: &str) -> String {
    if key.starts_with('/') {
        key.to_owned()
    } else {
        format!("/nix/store/{key}")
    }
}

/// Of the needed derivations, the ones not marked to build where they're
/// asked for: first those `kept` names, then those built from none of them.
/// The ones built from a kept one are in neither.
fn heavy(
    pieces: &BTreeMap<String, Piece>,
    needed: &[String],
    kept: &[String],
) -> (Vec<String>, Vec<String>) {
    let mut known = BTreeMap::new();
    let mut held = Vec::new();
    let mut rest = Vec::new();
    for drv in needed {
        if pieces.get(drv).is_none_or(|p| p.light) {
            continue;
        }
        if pieces.get(drv).is_some_and(|p| named(p, kept)) {
            held.push(drv.clone());
        } else if !is_kept(drv, pieces, kept, &mut known) {
            rest.push(drv.clone());
        }
    }
    (held, rest)
}

/// Whether a derivation is named by `kept` or built from one that is,
/// remembering each answer in `known`, since the system's pieces share
/// most of what they're built from.
fn is_kept(
    drv: &str,
    pieces: &BTreeMap<String, Piece>,
    kept: &[String],
    known: &mut BTreeMap<String, bool>,
) -> bool {
    if kept.is_empty() {
        return false;
    }
    if let Some(answer) = known.get(drv) {
        return *answer;
    }
    let answer = pieces.get(drv).is_some_and(|piece| {
        named(piece, kept)
            || piece
                .inputs
                .iter()
                .any(|input| is_kept(input, pieces, kept, known))
    });
    known.insert(drv.to_owned(), answer);
    answer
}

/// Whether one of `kept`'s patterns names the piece.
fn named(piece: &Piece, kept: &[String]) -> bool {
    piece
        .names
        .iter()
        .any(|name| kept.iter().any(|pattern| matches(pattern, name)))
}

/// Whether `name` is what `pattern` says, where `*` stands for any run of
/// characters, none included.
fn matches(pattern: &str, name: &str) -> bool {
    match pattern.split_once('*') {
        None => pattern == name,
        Some((first, after)) => {
            let Some(left) = name.strip_prefix(first) else {
                return false;
            };
            (0..=left.len())
                .filter(|&at| left.is_char_boundary(at))
                .any(|at| matches(after, &left[at..]))
        }
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
    fn what_a_dry_run_would_build_is_read_and_what_it_would_fetch_isnt() {
        let said = "these 2 derivations will be built:\n  /nix/store/a-one.drv\n  \
                    /nix/store/b-two.drv\nthis path will be fetched (1.0 MiB download):\n  \
                    /nix/store/c-three\nwarning: something\n";
        assert_eq!(
            to_build(said),
            ["/nix/store/a-one.drv", "/nix/store/b-two.drv"]
        );
        assert!(to_build("this path will be fetched:\n  /nix/store/c.drv\n").is_empty());
    }

    fn every(shown: &str) -> Vec<String> {
        pieces(shown).unwrap().into_keys().collect()
    }

    #[test]
    fn only_what_nixpkgs_marks_light_stays_here() {
        let newer = r#"{"derivations": {
            "a-light.drv": {"env": {"preferLocalBuild": "1"}},
            "b-heavy.drv": {"env": {}},
            "c-heavy.drv": {"env": {"preferLocalBuild": ""}}}}"#;
        assert_eq!(
            heavy(&pieces(newer).unwrap(), &every(newer), &[]),
            (
                vec![],
                vec![
                    "/nix/store/b-heavy.drv".to_owned(),
                    "/nix/store/c-heavy.drv".to_owned()
                ]
            )
        );
        let older = r#"{"/nix/store/d-heavy.drv": {"env": {}}}"#;
        assert_eq!(
            heavy(&pieces(older).unwrap(), &every(older), &[]).1,
            ["/nix/store/d-heavy.drv"]
        );
    }

    #[test]
    fn a_kept_piece_goes_apart_and_whatever_is_built_from_one_nowhere() {
        // The secret piece is built already, so only what's built from it
        // is needed, and a second one is needed too; what's built from the
        // first, even at one remove, goes to no builder, and a piece built
        // from none goes to the daemon's.
        let newer = r#"{"derivations": {
            "s-secret-1.0.drv": {"name": "secret-1.0", "env": {}},
            "w-wrapper.drv": {"name": "wrapper", "env": {},
                "inputs": {"drvs": {"s-secret-1.0.drv": {}}}},
            "x-outer.drv": {"name": "outer", "env": {},
                "inputs": {"drvs": {"w-wrapper.drv": {}, "o-open.drv": {}}}},
            "o-open.drv": {"name": "open", "env": {}},
            "t-secret-tool.drv": {"name": "secret-tool-2", "env": {}},
            "l-links.drv": {"name": "links", "env": {"preferLocalBuild": "1"},
                "inputs": {"drvs": {"x-outer.drv": {}}}}}}"#;
        let needed: Vec<String> = [
            "w-wrapper.drv",
            "x-outer.drv",
            "o-open.drv",
            "t-secret-tool.drv",
            "l-links.drv",
        ]
        .iter()
        .map(|k| stored(k))
        .collect();
        let (kept, rest) = heavy(&pieces(newer).unwrap(), &needed, &["secret*".to_owned()]);
        assert_eq!(kept, ["/nix/store/t-secret-tool.drv"]);
        assert_eq!(rest, ["/nix/store/o-open.drv"]);
        // Older Nix lists what a piece is built from by path, and a
        // pattern may name a piece's pname alone.
        let older = r#"{
            "/nix/store/s-secret.drv": {"name": "other", "env": {"pname": "secret"}},
            "/nix/store/w-wrapper.drv": {"name": "wrapper", "env": {},
                "inputDrvs": {"/nix/store/s-secret.drv": ["out"]}}}"#;
        let needed = ["/nix/store/s-secret.drv", "/nix/store/w-wrapper.drv"].map(str::to_owned);
        let (kept, rest) = heavy(&pieces(older).unwrap(), &needed, &["secret".to_owned()]);
        assert_eq!(kept, ["/nix/store/s-secret.drv"]);
        assert!(rest.is_empty());
    }

    #[test]
    fn a_pattern_stands_for_any_run_where_it_says_so() {
        assert!(matches("aquifer-link*", "aquifer-link-0.1.0"));
        assert!(matches("aquifer-link*", "aquifer-link"));
        assert!(matches("*-far", "aquifer-link-far"));
        assert!(matches("a*b*c", "a-x-b-y-c"));
        assert!(!matches("aquifer-link*", "an-aquifer-link"));
        assert!(!matches("vault", "vault-people"));
        assert!(!matches("a*c", "a-b"));
    }

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
