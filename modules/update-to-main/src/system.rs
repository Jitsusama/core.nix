//! Building the machine's system, switching to it, telling whether the
//! machine is whole afterwards, and going back to the system it left when
//! it isn't. A switch is the system's own: NixOS's switch-to-configuration
//! and nix-darwin's activate, after the profile points at it, as
//! nixos-rebuild and darwin-rebuild do.

use crate::batches;
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

/// How long the machine may be behind main before what its builders
/// didn't build is built here whatever the CPU pressure, still small. On 8
/// November a switch Joel waited on failed twice when the builders went
/// away, and each time the pieces left waited on pressure that sessions'
/// work kept over 30%, though they were small enough to build in minutes.
const OVERDUE: Duration = Duration::from_mins(20);

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
    } else if let Some(batches) = &config.batches {
        batches::build(config, batches, flake, overrides)?;
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
    let started = Instant::now();
    let printed = run::output(nix, "building the system", [flake]);
    eprintln!("building the system: {}s", started.elapsed().as_secs());
    let printed = printed?;
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

/// Nix, with the commands and flakes it's asked for, and root's git first
/// on its path: Nix runs git to fetch a `git+file` input, as main and what
/// it follows are, and the system's own path may hold none.
pub(crate) fn nix(config: &Config) -> std::process::Command {
    let mut nix = run::command(&config.tools.nix, &config.state);
    if let Some(git) = config.tools.git.parent() {
        let rest = std::env::split_paths(run::PATH);
        let path = std::env::join_paths(std::iter::once(git.to_path_buf()).chain(rest))
            .unwrap_or_else(|_| run::PATH.into());
        nix.env("PATH", path);
    }
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
    let needed = needed(config, flake, overrides)?;
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
    let pieces = pieces(&shown)?;
    let (kept, rest) = heavy(&pieces, &needed, &config.kept.names);
    eprintln!(
        "the system needs {} pieces built: {} kept apart, {} on the builders, the rest here",
        needed.len(),
        kept.len(),
        rest.len()
    );
    if !kept.is_empty() {
        build_kept(config, &kept)?;
    }
    if !rest.is_empty() {
        let builders = config.builders.as_deref();
        // A big piece, such as the kernel, goes to the builders before
        // anything is built here when the rest may build here, since built
        // here as another piece's input it held the machine's CPUs for over
        // an hour on 8 November while the lent builders were down.
        let (big, small): (Vec<String>, Vec<String>) = rest
            .iter()
            .cloned()
            .partition(|drv| pieces.get(drv).is_some_and(|p| p.big));
        let rest = if config.here.is_some() && !big.is_empty() {
            beside_the_big(config, &pieces, &big, small, builders)?
        } else {
            rest
        };
        if rest.is_empty() {
            return Ok(());
        }
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

/// The derivations the system needs built, by their paths, as a dry run
/// lists them, each kept from being collected until the next build lists
/// its own.
pub(crate) fn needed(
    config: &Config,
    flake: &str,
    overrides: &[String],
) -> Result<Vec<String>, Failure> {
    let mut listing = nix(config);
    listing
        .args(["build", "--dry-run", "--no-link"])
        .args(overrides);
    // Nix says what it would build on its error stream.
    let said = run::said(listing, "listing what the system needs", [flake])?;
    let needed = to_build(&said);
    keep(config, &needed);
    Ok(needed)
}

/// Keeps `drvs`, and what each is built from, from being collected, in
/// place of those the last build kept. The dry run writes each derivation
/// with nothing holding it, and the build of the pieces kept apart can run
/// for many minutes before anything else builds: on 11 November the disk
/// guard's collection took the launcher's derivation 13 minutes into one,
/// and the switch failed when its build came to it. A derivation's own
/// path, with no outputs named, is linked as it is, with nothing built. A
/// derivation left unkept only risks that again, so a failure is said and
/// the build goes on.
fn keep(config: &Config, drvs: &[String]) {
    let folder = config.evaluated_folder();
    let kept = match fs::remove_dir_all(&folder) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error),
        _ => fs::create_dir_all(&folder),
    };
    if let Err(error) = kept {
        eprintln!(
            "keeping what the system needs: {}: {error}",
            folder.display()
        );
        return;
    }
    if drvs.is_empty() {
        return;
    }
    let mut link = nix(config);
    link.args(["build", "--out-link"]).arg(folder.join("drv"));
    if let Err(failure) = run::output(link, "keeping what the system needs", drvs) {
        eprintln!("{}", failure.why);
    }
}

/// Builds the big pieces on the builders and, beside them, the small ones
/// none of them is built from, giving what's left to build: the small
/// pieces built from a big one, and those beside it if they didn't build.
/// Neither may build here, so a builder that is down leaves the kernel for
/// another rather than this machine. On 8 November a switch built the
/// kernel alone for 94 minutes, then OpenVINO and ONNX Runtime, which
/// need nothing of it, for an hour more after.
fn beside_the_big(
    config: &Config,
    pieces: &BTreeMap<String, Piece>,
    big: &[String],
    small: Vec<String>,
    builders: Option<&str>,
) -> Result<Vec<String>, Failure> {
    let mut known = BTreeMap::new();
    let (after, beside): (Vec<String>, Vec<String>) = small
        .into_iter()
        .partition(|drv| built_from_big(drv, pieces, &mut known));
    let update = thread::current().id();
    let (big_built, beside_built) = thread::scope(|scope| {
        let alongside = (!beside.is_empty()).then(|| {
            scope.spawn(|| {
                run::helps(update);
                away(
                    config,
                    &beside,
                    builders,
                    "building the small pieces beside the big on the machine's builders",
                )
            })
        });
        let big_built = away(
            config,
            big,
            builders,
            "building the big pieces on the machine's builders",
        );
        let beside_built = alongside.map(|building| {
            let helper = building.thread().id();
            // Nothing more is built once a big piece fails, so what builds
            // beside it is stopped rather than waited on.
            if big_built.is_err() {
                run::stop(helper);
            }
            let built = building
                .join()
                .unwrap_or_else(|_| Err(Failure::new("building beside the big pieces panicked")));
            run::helped(update, helper);
            built
        });
        (big_built, beside_built)
    });
    big_built.map_err(|failed| Failure {
        why: format!("{}, which never build here", failed.why),
        lines: failed.lines,
    })?;
    Ok(match beside_built {
        Some(Err(failed)) => {
            eprintln!("{}", gone_on(&failed));
            beside.into_iter().chain(after).collect()
        }
        _ => after,
    })
}

/// What the journal says of the small pieces beside the big when they
/// failed: why, and the lines that tell it, since the update goes on and
/// nothing else says them. On 11 November that step failed after 169
/// seconds with no reason in the journal, and its 328 pieces then took 12
/// minutes more.
fn gone_on(failed: &Failure) -> String {
    std::iter::once(format!(
        "{}; they're built after the big pieces instead",
        failed.why
    ))
    .chain(failed.lines.iter().map(|line| format!("  {line}")))
    .collect::<Vec<_>>()
    .join("\n")
}

/// Whether a derivation asks for `big-parallel` or is built from one that
/// does, remembering each answer in `known` as `is_kept` does.
fn built_from_big(
    drv: &str,
    pieces: &BTreeMap<String, Piece>,
    known: &mut BTreeMap<String, bool>,
) -> bool {
    if let Some(answer) = known.get(drv) {
        return *answer;
    }
    let answer = pieces.get(drv).is_some_and(|piece| {
        piece.big
            || piece
                .inputs
                .iter()
                .any(|input| built_from_big(input, pieces, known))
    });
    known.insert(drv.to_owned(), answer);
    answer
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
    told(config, away, what, pieces)
}

/// Builds pieces with `build`, saying in the journal how long it took and
/// where each piece was built, since what Nix says of a build that works is
/// otherwise dropped, and a slow switch then can't say what it waited on.
/// The build is stopped once it has said nothing for the config's
/// `silent_for`, so an update waiting on a builder that doesn't answer
/// goes on to build here or fails saying why within five minutes, rather
/// than wait as one did for 20 minutes on 8 November.
pub(crate) fn told(
    config: &Config,
    mut build: std::process::Command,
    what: &str,
    pieces: &[String],
) -> Result<(), Failure> {
    let started = Instant::now();
    // Each piece's log as it builds is what shows the build is moving.
    build.arg("-L");
    let silent = Duration::from_secs(config.silent_for);
    let ran = run::built(build, what, outputs(pieces), silent);
    let took = started.elapsed();
    match &ran {
        Ok(said) => eprintln!("{}", account(what, pieces.len(), took, said)),
        Err(_) => eprintln!(
            "{what}: {} pieces, failed after {}s",
            pieces.len(),
            took.as_secs()
        ),
    }
    ran.map(|_| ())
}

/// How many pieces a build was asked for, how long it took, and where Nix
/// says it built each, the first few by name.
fn account(what: &str, pieces: usize, took: Duration, said: &str) -> String {
    const NAMED: usize = 12;
    let built: Vec<String> = said.lines().filter_map(built_where).collect();
    let took = took.as_secs();
    let named = built[..built.len().min(NAMED)].join(", ");
    match (built.len(), built.len().saturating_sub(NAMED)) {
        (0, _) => format!("{what}: {pieces} pieces in {took}s"),
        (all, 0) => format!("{what}: {pieces} pieces in {took}s, {all} built: {named}"),
        (all, more) => {
            format!("{what}: {pieces} pieces in {took}s, {all} built: {named} and {more} more")
        }
    }
}

/// The derivation's name and its builder from a line in which Nix says it's
/// building one: `building '/nix/store/<hash>-<name>.drv' on '<builder>'...`,
/// or with no builder when it builds here.
fn built_where(line: &str) -> Option<String> {
    let rest = line.trim().strip_prefix("building '")?;
    let (path, rest) = rest.split_once('\'')?;
    let file = path.rsplit('/').next()?;
    let name = file.split_once('-').map_or(file, |(_, name)| name);
    let name = name.strip_suffix(".drv").unwrap_or(name);
    let place = rest
        .strip_prefix(" on '")
        .and_then(|on| on.split_once('\''))
        .map_or("here", |(builder, _)| builder);
    Some(format!("{name} on {place}"))
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

/// Why the builders didn't build what now builds here, with the last lines
/// they said.
fn fell_back(apart: &Failure) -> String {
    let mut note = format!("{}, so they build here, after:", apart.why);
    for line in &apart.lines {
        note.push_str("\n  ");
        note.push_str(line);
    }
    note
}

/// Builds pieces their builders didn't here, small, when `here` lets the
/// machine and only while it's quiet or once it has been behind main for
/// `OVERDUE`; otherwise gives why they weren't.
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
    let overdue = behind_for(config).filter(|behind| *behind >= OVERDUE);
    if let Some(behind) = overdue {
        eprintln!(
            "the machine has been behind main for {} minutes, so the rest builds here \
             whatever the CPU pressure",
            behind.as_secs() / 60
        );
    }
    let busy = match cpu_pressure(&config.cpu_pressure) {
        _ if overdue.is_some() => None,
        Some(waited) if waited < here.busy_at => None,
        Some(waited) => Some(format!(
            "CPU pressure here is {waited:.1}%, not below {}%",
            here.busy_at
        )),
        None => Some("CPU pressure here can't be read".to_owned()),
    };
    if let Some(busy) = busy {
        return Err(Failure {
            why: format!("{}, and {busy}, so they wait", apart.why),
            lines: apart.lines,
        });
    }
    // The build here may well succeed, and then nothing else would say why
    // the builders didn't: on 9 November a kept build lost 391s to a cause
    // no line kept.
    eprintln!("{}", fell_back(&apart));
    let mut local = nix(config);
    local.args(["build", "--no-link"]);
    if let Some(builders) = builders {
        local.args(["--builders", builders]);
    }
    local.args(["--max-jobs", &here.max_jobs.to_string()]);
    local.args(["--cores", &here.cores.to_string()]);
    told(config, local, what, pieces)
}

/// How long the machine has been behind main, from when an update first
/// built a main it doesn't run.
fn behind_for(config: &Config) -> Option<Duration> {
    fs::metadata(config.behind_file())
        .and_then(|meta| meta.modified())
        .ok()?
        .elapsed()
        .ok()
}

/// The share of the last minute that some runnable task waited for a CPU,
/// from Linux's `some ... avg60=` line, when it can be read. This rather
/// than the load average, which counts every runnable task and every one
/// waiting on a disk: on 7 November, with a hundred sessions open, it
/// held at 34 to 38 all evening, so the one small piece no builder took
/// waited rather than build. changed-checks gates on the same line.
fn cpu_pressure(file: &Path) -> Option<f64> {
    let text = fs::read_to_string(file).ok()?;
    let some = text.lines().find(|line| line.starts_with("some "))?;
    some.split_whitespace()
        .find_map(|field| field.strip_prefix("avg60="))?
        .parse()
        .ok()
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
pub(crate) struct Piece {
    names: Vec<String>,
    light: bool,
    /// Whether it asks for `big-parallel`, as nixpkgs' kernel does.
    big: bool,
    /// The derivations it's built from, by their paths.
    pub(crate) inputs: Vec<String>,
}

/// The derivations `nix derivation show` gave, by their paths. Newer Nix
/// keys them by name under `derivations` and lists what each is built from
/// under `inputs.drvs`; older keys them by path at the top, under
/// `inputDrvs`.
pub(crate) fn pieces(shown: &str) -> Result<BTreeMap<String, Piece>, Failure> {
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
                light: is_light(d),
                big: is_big(d),
                inputs,
            };
            (stored(key), piece)
        })
        .collect())
}

/// Whether nixpkgs marks a derivation to build where it's asked for: in
/// its environment as `"1"`, or, when its attributes are structured, as
/// writeText's are, as `true` among them. Newer Nix shows those under
/// `structuredAttrs`; older keeps them as JSON text in `env.__json`.
fn is_light(derivation: &serde_json::Value) -> bool {
    derivation["env"]["preferLocalBuild"].as_str() == Some("1")
        || structured(derivation)["preferLocalBuild"].as_bool() == Some(true)
}

/// Whether a derivation asks for `big-parallel`, in its environment as
/// words or in its structured attributes as a list.
fn is_big(derivation: &serde_json::Value) -> bool {
    const BIG: &str = "big-parallel";
    let asked = derivation["env"]["requiredSystemFeatures"]
        .as_str()
        .is_some_and(|words| words.split_whitespace().any(|word| word == BIG));
    asked
        || structured(derivation)["requiredSystemFeatures"]
            .as_array()
            .is_some_and(|features| features.iter().any(|f| f.as_str() == Some(BIG)))
}

/// A derivation's structured attributes, from `__json` in older Nix's
/// environment or `structuredAttrs` in newer's, or null when it has none.
fn structured(derivation: &serde_json::Value) -> serde_json::Value {
    derivation["env"]["__json"]
        .as_str()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok())
        .unwrap_or_else(|| derivation["structuredAttrs"].clone())
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

/// Points the profile at `system` and switches the machine to it live.
///
/// On NixOS a new kernel, initrd or kernel parameters can only come with a
/// boot, so the system runs now on the kernel the machine booted, as
/// NixOS's own switch does: its modules are still loaded from the booted
/// system, and the boot loader starts the new kernel next time. The switch
/// says so as the restart it needs.
pub fn switch(config: &Config, system: &Path) -> Result<Switched, Failure> {
    let previous = current(config);
    set_profile(config, system)?;
    match config.kind {
        Kind::Nixos => {
            let booted = &config.booted;
            let waits = booted
                .exists()
                .then(|| for_next_boot(booted, system))
                .flatten();
            // It says what it stopped, started and restarted on its error
            // stream.
            let said = run::said(
                run::command(&system.join("bin/switch-to-configuration"), &config.state),
                "switching",
                ["switch"],
            )?;
            Ok(Switched {
                applied: Applied::Now,
                restart: waits,
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

/// What only the next boot brings in switching from NixOS system `a` to
/// `b`, as the restart it needs, naming the kernel, initrd or kernel
/// parameters that differ; none when they boot the same.
fn for_next_boot(a: &Path, b: &Path) -> Option<String> {
    let same = |f: &str| fs::canonicalize(a.join(f)).ok() == fs::canonicalize(b.join(f)).ok();
    let params = |s: &Path| fs::read_to_string(s.join("kernel-params")).ok();
    let changed: Vec<&str> = [
        (!same("kernel")).then_some("kernel"),
        (!same("initrd")).then_some("initrd"),
        (params(a) != params(b)).then_some("kernel parameters"),
    ]
    .into_iter()
    .flatten()
    .collect();
    match changed.as_slice() {
        [] => None,
        [one] => Some(format!("its {one} changed")),
        [first @ .., last] => Some(format!("its {} and {last} changed", first.join(", "))),
    }
}

/// The units switch-to-configuration says it restarted, or stopped and
/// started again, as it does a changed service unless that says otherwise.
pub fn restarted_units(said: &str) -> Vec<String> {
    let units = |prefix: &str| -> Vec<String> {
        said.lines()
            .filter_map(|l| l.strip_prefix(prefix))
            .flat_map(|units| units.split(", "))
            .map(|u| u.trim().to_owned())
            .filter(|u| !u.is_empty())
            .collect()
    };
    let started = units("starting the following units: ");
    let mut restarted = units("restarting the following units: ");
    let again: Vec<String> = units("stopping the following units: ")
        .into_iter()
        .filter(|u| started.contains(u) && !restarted.contains(u))
        .collect();
    restarted.extend(again);
    restarted
}

/// Whether a daemon's definition differs between two systems, or only one
/// has it, so switching from `old` to `new` restarts it: a systemd unit's
/// file and its drop-ins on NixOS, a launchd plist on a Mac.
pub fn daemon_changed(kind: Kind, old: &Path, new: &Path, name: &str) -> bool {
    match kind {
        Kind::Nixos => {
            let unit = |system: &Path| {
                let units = system.join("etc/systemd/system");
                (
                    fs::read_to_string(units.join(name)).ok(),
                    folder(&units.join(format!("{name}.d"))),
                )
            };
            unit(old) != unit(new)
        }
        Kind::Darwin => {
            let plist = |system: &Path| {
                fs::read_to_string(system.join(format!("Library/LaunchDaemons/{name}.plist"))).ok()
            };
            plist(old) != plist(new)
        }
    }
}

/// Whether switching from `old` to `new` restarts an account's unit:
/// home-manager restarts one whose file or drop-ins changed, or that only
/// one system has, unless the new file says `X-RestartIfChanged=false`.
/// Each system's `home-manager-<account>.service` names the generation
/// whose home holds the unit's file.
pub fn account_unit_restarts(old: &Path, new: &Path, account: &str, name: &str) -> bool {
    let unit = |system: &Path| {
        let Some(generation) = home_generation(system, account) else {
            return (None, BTreeMap::new());
        };
        let units = generation.join("home-files/.config/systemd/user");
        (
            fs::read_to_string(units.join(name)).ok(),
            folder(&units.join(format!("{name}.d"))),
        )
    };
    let (before, after) = (unit(old), unit(new));
    let kept = after
        .0
        .as_deref()
        .is_some_and(|file| file.lines().any(|l| l.trim() == "X-RestartIfChanged=false"));
    before != after && !kept
}

/// The home-manager generation a NixOS system activates for an account, as
/// its `home-manager-<account>.service` gives it to the last word it runs.
fn home_generation(system: &Path, account: &str) -> Option<PathBuf> {
    let unit = system.join(format!("etc/systemd/system/home-manager-{account}.service"));
    fs::read_to_string(unit)
        .ok()?
        .lines()
        .find_map(|l| l.strip_prefix("ExecStart="))?
        .split_whitespace()
        .last()
        .map(PathBuf::from)
}

/// Whether a path in a system leads somewhere else in another, or only one
/// has it, as a NixOS system's `kernel` does when it boots another kernel.
pub fn path_changed(old: &Path, new: &Path, path: &str) -> bool {
    fs::canonicalize(old.join(path)).ok() != fs::canonicalize(new.join(path)).ok()
}

/// Each file in a folder by its name, with what it holds.
fn folder(path: &Path) -> BTreeMap<String, Option<String>> {
    let Ok(entries) = fs::read_dir(path) else {
        return BTreeMap::new();
    };
    entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            Some((name, fs::read_to_string(e.path()).ok()))
        })
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
        assert_eq!(
            to_build("this path will be fetched:\n  /nix/store/c.drv\n"),
            [] as [std::string::String; 0]
        );
    }

    #[test]
    fn a_build_that_falls_back_here_says_why_the_builders_didnt() {
        let apart = Failure {
            why: "building the kept pieces on their own builders failed (exit status: 1)"
                .to_owned(),
            lines: vec![
                "error: path '/nix/store/a-defmt' is required".to_owned(),
                "copying path '/nix/store/a-defmt'".to_owned(),
            ],
        };
        assert_eq!(
            fell_back(&apart),
            "building the kept pieces on their own builders failed (exit status: 1), so \
             they build here, after:\n  error: path '/nix/store/a-defmt' is required\n  \
             copying path '/nix/store/a-defmt'"
        );
    }

    fn every(shown: &str) -> Vec<String> {
        pieces(shown).unwrap().into_keys().collect()
    }

    #[test]
    fn a_build_says_how_long_it_took_and_where_each_piece_was_built() {
        let said = "these 2 derivations will be built:\n  /nix/store/a-reader.drv\n  \
                    /nix/store/b-links.drv\n\
                    building '/nix/store/0abc-reader-0.1.0.drv' on 'ssh-ng://far'...\n\
                    copying path '/nix/store/0def-reader-0.1.0' from 'ssh-ng://far'...\n\
                    building '/nix/store/0ghi-links.drv'...\n";
        assert_eq!(
            account(
                "building on the builders",
                2,
                Duration::from_secs(734),
                said
            ),
            "building on the builders: 2 pieces in 734s, 2 built: \
             reader-0.1.0 on ssh-ng://far, links on here"
        );
        assert_eq!(
            account("building here", 3, Duration::from_secs(5), ""),
            "building here: 3 pieces in 5s",
            "a build that built nothing says only its time"
        );
    }

    #[test]
    fn a_long_build_names_its_first_pieces_and_counts_the_rest() {
        let said = (0..14)
            .map(|n| format!("building '/nix/store/0h-p{n}.drv'..."))
            .collect::<Vec<_>>()
            .join("\n");
        let line = account("building here", 14, Duration::from_secs(1), &said);
        assert!(line.ends_with("p11 on here and 2 more"), "{line}");
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
    fn a_piece_with_structured_attributes_marked_light_stays_here() {
        // As nixpkgs' writeText writes a unit file now: its attributes are
        // JSON, so the mark is a boolean there and its env holds only `out`.
        let shown = r#"{"derivations": {
            "a-unit.drv": {"env": {"out": "/1rz4g4z"},
                "structuredAttrs": {"preferLocalBuild": true}},
            "b-heavy.drv": {"env": {"out": "/1rz4g4z"},
                "structuredAttrs": {"preferLocalBuild": false}},
            "c-older-unit.drv": {"env": {"out": "/1rz4g4z",
                "__json": "{\"preferLocalBuild\":true}"}}}}"#;
        assert_eq!(
            heavy(&pieces(shown).unwrap(), &every(shown), &[]).1,
            ["/nix/store/b-heavy.drv"],
            "a light piece with structured attributes is left for the build here"
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
        assert_eq!(rest, [] as [std::string::String; 0]);
    }

    #[test]
    fn small_pieces_that_failed_beside_the_big_say_why_before_going_on() {
        let said = gone_on(&Failure {
            why: "building the small pieces beside the big failed".to_owned(),
            lines: vec!["error: unable to download 'https://x': TLS".to_owned()],
        });
        assert_eq!(
            said,
            "building the small pieces beside the big failed; they're built after the big \
             pieces instead\n  error: unable to download 'https://x': TLS",
            "a failure beside the big didn't say why, or dropped its lines"
        );
    }

    #[test]
    fn what_is_built_from_a_big_piece_even_at_one_remove_waits_for_it() {
        // The kernel's modules are built from it, and the initrd from them;
        // OpenVINO needs nothing of it, so it builds beside the kernel.
        let shown = r#"{"derivations": {
            "k-linux.drv": {"name": "linux",
                "env": {"requiredSystemFeatures": "big-parallel"}},
            "m-modules.drv": {"name": "modules", "env": {},
                "inputs": {"drvs": {"k-linux.drv": {}}}},
            "i-initrd.drv": {"name": "initrd", "env": {},
                "inputs": {"drvs": {"m-modules.drv": {}, "z-zlib.drv": {}}}},
            "v-openvino.drv": {"name": "openvino", "env": {},
                "inputs": {"drvs": {"z-zlib.drv": {}}}}}}"#;
        let pieces = pieces(shown).unwrap();
        let mut known = BTreeMap::new();
        let waits = |drv: &str, known: &mut BTreeMap<String, bool>| {
            built_from_big(&stored(drv), &pieces, known)
        };
        assert!(waits("i-initrd.drv", &mut known));
        assert!(waits("m-modules.drv", &mut known));
        assert!(!waits("v-openvino.drv", &mut known));
        // What nothing shows, as a piece built already, is built from
        // nothing big.
        assert!(!waits("z-zlib.drv", &mut known));
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
        // A changed service is stopped and started again unless it says
        // otherwise, which is a restart as much as one restarted is.
        let said = "stopping the following units: a.service, c.service\n\
                    restarting the following units: nix-daemon.service, sshd.service\n\
                    starting the following units: b.service, c.service\n";
        assert_eq!(
            restarted_units(said),
            ["nix-daemon.service", "sshd.service", "c.service"]
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

    #[test]
    fn the_next_boot_says_which_of_its_pieces_changed() {
        // A system's kernel and initrd are links into the store, so the
        // same one is the same file however it's reached.
        let store = tempfile::tempdir().unwrap();
        for piece in ["one", "two"] {
            fs::write(store.path().join(piece), piece).unwrap();
        }
        let booted = tempfile::tempdir().unwrap();
        let new = tempfile::tempdir().unwrap();
        let boots = |system: &Path, kernel: &str, initrd: &str, params: &str| {
            for (link, piece) in [("kernel", kernel), ("initrd", initrd)] {
                let _ = fs::remove_file(system.join(link));
                std::os::unix::fs::symlink(store.path().join(piece), system.join(link)).unwrap();
            }
            fs::write(system.join("kernel-params"), params).unwrap();
        };
        let (a, b) = (booted.path(), new.path());
        boots(a, "one", "one", "quiet");
        boots(b, "one", "one", "quiet");
        assert_eq!(for_next_boot(a, b), None);
        boots(b, "one", "two", "quiet");
        assert_eq!(for_next_boot(a, b).as_deref(), Some("its initrd changed"));
        boots(b, "two", "two", "quiet");
        assert_eq!(
            for_next_boot(a, b).as_deref(),
            Some("its kernel and initrd changed")
        );
        boots(b, "two", "two", "loud");
        assert_eq!(
            for_next_boot(a, b).as_deref(),
            Some("its kernel, initrd and kernel parameters changed")
        );
    }

    fn daemon(system: &Path, label: &str, text: &str) {
        let dir = system.join("Library/LaunchDaemons");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(format!("{label}.plist")), text).unwrap();
    }
}
