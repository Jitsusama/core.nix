//! Names the checks a change touches, and builds only those.
//!
//! A check whose derivation is the same as at the base proves nothing new,
//! since its result is already what it was. This evaluates every check's
//! derivation at the base, the merge base with origin's main unless told
//! otherwise, and in the working tree, and names each check added, changed
//! or removed, and each that no longer evaluates. With `--build` it builds
//! those added and changed, from the derivations it evaluated. With `--keep-base` it only evaluates the base
//! and keeps its derivations, comparing nothing, so a run after each merge
//! leaves every later comparison against that commit one evaluation to pay.
//!
//! Usage: `changed-checks [--base REF] [--system SYSTEM] [--build] [--no-vm] [--keep-base]`
//!
//! The working tree is read as Nix reads a git checkout, so a new file
//! counts only once git knows it (`git add -N`). A change that only some
//! checks read, such as a module's, touches only those; one every check
//! reads, such as `flake.lock`, touches them all.
//!
//! The checks are evaluated a few dozen at a time, in a nix eval each, a VM
//! test counting four, two evaluations at once (`checks.rs` says why),
//! rather than all in one, which held every check at once and reached 24 GB
//! on a machine many sessions share; each evaluation's memory goes when it
//! ends. A commit's derivations never change, so the base's are
//! kept under `~/.cache/changed-checks`, and only the working tree is
//! evaluated again. A run that wants a commit another run is evaluating
//! waits for it rather than evaluating it too, and a working tree git
//! calls clean is kept by its commit the same way. git and nix are the person's own, with their settings,
//! so they're found on the path rather than pinned.
//!
//! Each run that reaches its base adds a line to `changed-checks/runs.jsonl`
//! under XDG's state folder, saying what each evaluation and the build took
//! (`runs.rs` says what it holds).

mod checks;
mod pressure;
mod runs;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Instant;

use checks::Derivations;
use runs::Run;

/// What a run asked for is refused with, as sysexits.h numbers it.
const USAGE: u8 = 64;

const USED: &str =
    "Usage: changed-checks [--base REF] [--system SYSTEM] [--build] [--no-vm] [--keep-base]";

/// What a run was asked for.
struct Options {
    /// What the change is compared with, from its merge base.
    base: String,
    /// The checks' system, or this machine's when none is named.
    system: Option<String>,
    /// Whether to build the checks added and changed.
    build: bool,
    /// Whether the VM tests are among those built.
    vm: bool,
    /// Whether to keep the base's derivations and compare nothing.
    keep_base: bool,
}

impl Options {
    /// Read the command line's arguments, or say what's wrong with them.
    fn parse(mut arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut options = Self {
            base: "refs/remotes/origin/main".to_owned(),
            system: None,
            build: false,
            vm: true,
            keep_base: false,
        };
        while let Some(argument) = arguments.next() {
            let mut value = || {
                arguments
                    .next()
                    .ok_or_else(|| format!("{argument} needs a value"))
            };
            match argument.as_str() {
                "--base" => options.base = value()?,
                "--system" => options.system = Some(value()?),
                "--build" => options.build = true,
                "--no-vm" => options.vm = false,
                "--keep-base" => options.keep_base = true,
                _ => return Err(format!("{argument} isn't something it takes")),
            }
        }
        Ok(options)
    }
}

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    if arguments.iter().any(|argument| argument == "--help") {
        println!("{USED}");
        return ExitCode::SUCCESS;
    }
    let options = match Options::parse(arguments.into_iter()) {
        Ok(options) => options,
        Err(why) => {
            eprintln!("changed-checks: {why}\n{USED}");
            return ExitCode::from(USAGE);
        }
    };
    match changed(&options) {
        Ok(code) => code,
        Err(why) => {
            eprintln!("changed-checks: {why}");
            ExitCode::FAILURE
        }
    }
}

/// Name the checks the working tree's change touches, and build them if asked.
fn changed(options: &Options) -> Result<ExitCode, String> {
    let root = run("git", &["rev-parse", "--show-toplevel"])?;
    let Ok(base) = run("git", &["-C", &root, "merge-base", "HEAD", &options.base]) else {
        eprintln!(
            "changed-checks: {} shares no history with HEAD",
            options.base
        );
        return Ok(ExitCode::from(USAGE));
    };
    let system = match &options.system {
        Some(system) => system.clone(),
        None => run(
            "nix",
            &[
                "eval",
                "--raw",
                "--impure",
                "--expr",
                "builtins.currentSystem",
            ],
        )?,
    };
    let flake = format!("git+file://{root}");
    let untracked = run(
        "git",
        &["-C", &root, "ls-files", "--others", "--exclude-standard"],
    )?;
    if !untracked.is_empty() {
        eprintln!("changed-checks: Nix doesn't read files git doesn't know:\n{untracked}");
    }
    let mut noted = Run::new(&base, &system);
    let compared = compare(options, &flake, &base, &system, &mut noted);
    let log = state().join("changed-checks/runs.jsonl");
    if let Err(why) = noted.record(&log) {
        eprintln!("changed-checks: couldn't log the run: {why}");
    }
    compared
}

/// Compare the working tree's checks with the base's, noting in the run
/// what each step took, and build what changed if asked.
fn compare(
    options: &Options,
    flake: &str,
    base: &str,
    system: &str,
    noted: &mut Run,
) -> Result<ExitCode, String> {
    let short = base.get(..12).unwrap_or(base);
    let kept = |noted: &mut Run| {
        checks::kept(&checks::keeping(&cache(), base, system), || {
            derivations(&format!("{flake}?rev={base}"), system, "base", noted)
        })
    };
    if options.keep_base {
        kept(noted)?;
        eprintln!("kept {system}'s checks at {short}");
        return Ok(ExitCode::SUCCESS);
    }
    eprintln!("comparing {system}'s checks at {short} with the working tree");
    let before = kept(noted)?;
    // A working tree git calls clean is its commit, whose derivations never
    // change, so they're kept by it as a base's are, and a run again on the
    // same commit, or one whose commit is a base kept already, evaluates
    // nothing.
    let after = match clean_commit()? {
        Some(commit) => checks::kept(&checks::keeping(&cache(), &commit, system), || {
            derivations(flake, system, "tree", noted)
        })?,
        None => derivations(flake, system, "tree", noted)?,
    };
    let comparison = checks::compare(&before, &after);
    let mut named = 0;
    for (kind, names) in comparison.kinds() {
        for name in names {
            println!("{kind:<8} {name}");
            named += 1;
        }
    }
    noted.named(named);
    if !comparison.broken.is_empty() {
        return Ok(ExitCode::FAILURE);
    }
    let targets = comparison.to_build(options.vm);
    if !options.build || targets.is_empty() {
        return Ok(ExitCode::SUCCESS);
    }
    let began = Instant::now();
    // Each check is built from the derivation just evaluated, so no
    // evaluator runs beside the builds: one holds its whole heap until its
    // last build ends, 1.4 to 4 GiB through a VM test. A derivation the
    // store has collected since is built from the flake, as before.
    let built = Command::new("nix")
        .args(["build", "--no-link", "--keep-going", "-L"])
        .args(where_to_build())
        .args(
            targets
                .iter()
                .map(|name| match after.get(*name).and_then(Option::as_deref) {
                    Some(derivation) if Path::new(derivation).exists() => format!("{derivation}^*"),
                    _ => format!("{flake}#checks.{system}.{name}"),
                }),
        )
        .status()
        .map_err(|error| format!("running nix: {error}"))?;
    noted.built(targets.len(), began.elapsed(), built.success());
    Ok(if built.success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Where nix-offload says the machine's builders: `NIX_OFFLOAD_DIR`, or
/// /run/nix-offload. In it, machines-with-standing names the standing
/// builders beside any lent one, for a build that asks for them, and store
/// the lent builder's store, while one stands.
fn offload_dir() -> PathBuf {
    env::var_os("NIX_OFFLOAD_DIR").map_or_else(|| PathBuf::from("/run/nix-offload"), PathBuf::from)
}

/// Where the checks are built. While a builder is lent, they're sent to
/// its store whole: evaluated here, with only their derivations and
/// sources crossing, and every input built or fetched there. The build
/// hook's way would build their inputs here and copy them up, and the
/// connection's upload can be slow. Otherwise they're built here, with the
/// standing builders asked for where the machine has them. Those are
/// never offered to every build, since they may be machines only public
/// code may reach; core.nix's checks are public code with public inputs
/// alone, which `stands-alone` holds, so they may go there.
fn where_to_build() -> Vec<String> {
    let dir = offload_dir();
    let with_standing = dir.join("machines-with-standing");
    let store = fs::read_to_string(dir.join("store")).unwrap_or_default();
    if let Some(store) = store
        .lines()
        .next()
        .filter(|store| !store.trim().is_empty())
    {
        eprintln!("changed-checks: building on the lent builder, {store}");
        vec![
            "--eval-store".to_owned(),
            "auto".to_owned(),
            "--store".to_owned(),
            store.trim().to_owned(),
        ]
    } else if with_standing.exists() {
        vec![
            "--builders".to_owned(),
            format!("@{}", with_standing.display()),
        ]
    } else {
        Vec::new()
    }
}

/// The working tree's commit, when nothing in it differs from the commit
/// in what Nix reads: no file git tracks, or has been told of, changed.
fn clean_commit() -> Result<Option<String>, String> {
    let changed = run("git", &["status", "--porcelain", "--untracked-files=no"])?;
    if !changed.is_empty() {
        return Ok(None);
    }
    run("git", &["rev-parse", "HEAD"]).map(Some)
}

/// Each evaluation goes past nix's evaluation cache. Evaluated through
/// `--apply`, a result can't be kept there, so the cache saved nothing (one
/// chunk took 115 s with it and 108 to 127 s without), while two evaluations
/// at once, or two runs, found its database busy.
const UNCACHED: &str = "--no-eval-cache";

/// Evaluate each of a flake's checks for a system to its derivation's path,
/// noting in the run what each chunk of them took, as the side named.
fn derivations(
    flake: &str,
    system: &str,
    side: &'static str,
    noted: &mut Run,
) -> Result<Derivations, String> {
    let mut evaluation = Run::evaluating(side);
    let checks = format!("{flake}#checks.{system}");
    let listed = run(
        "nix",
        &[
            "eval",
            "--json",
            &checks,
            "--apply",
            "builtins.attrNames",
            UNCACHED,
        ],
    )?;
    let names: Vec<String> = serde_json::from_str(&listed)
        .map_err(|error| format!("nix named the checks oddly: {error}"))?;
    let chunks = checks::chunks(&names);
    // A further chunk begins only while memory isn't stalling: two at once
    // while it was full stalled Joel's launcher on 29 October.
    let evaluated = checks::each_at_once(
        &chunks,
        checks::AT_ONCE,
        || !pressure::memory_stalled(),
        |chunk| {
            let selecting = checks::selecting(chunk)?;
            let began = Instant::now();
            let evaluated = run(
                "nix",
                &["eval", "--json", &checks, "--apply", &selecting, UNCACHED],
            )?;
            let took = began.elapsed();
            let derivations: Derivations = serde_json::from_str(&evaluated)
                .map_err(|error| format!("nix gave the checks' derivations oddly: {error}"))?;
            Ok::<_, String>((chunk.len(), took, derivations))
        },
    )?;
    let mut found = Derivations::new();
    for (checks, took, derivations) in evaluated {
        evaluation.chunk(checks, took);
        found.extend(derivations);
    }
    noted.evaluated(evaluation);
    Ok(found)
}

/// Run a program on the path, and give what it printed, trimmed; what it
/// says of itself goes to this one's standard error.
fn run(program: &str, arguments: &[&str]) -> Result<String, String> {
    let ran = Command::new(program)
        .args(arguments)
        .stderr(std::process::Stdio::inherit())
        .output()
        .map_err(|error| format!("running {program}: {error}"))?;
    if !ran.status.success() {
        return Err(format!(
            "{program} {} failed: {}",
            arguments.join(" "),
            ran.status
        ));
    }
    String::from_utf8(ran.stdout)
        .map(|said| said.trim().to_owned())
        .map_err(|_| format!("{program} {} printed what isn't text", arguments.join(" ")))
}

/// Give the cache folder: XDG's, or `~/.cache` without one.
fn cache() -> PathBuf {
    xdg("XDG_CACHE_HOME", ".cache")
}

/// Give the state folder: XDG's, or `~/.local/state` without one.
fn state() -> PathBuf {
    xdg("XDG_STATE_HOME", ".local/state")
}

/// Give the folder XDG's variable names, or the one under home without it.
fn xdg(variable: &str, under_home: &str) -> PathBuf {
    env::var_os(variable)
        .filter(|folder| !folder.is_empty())
        .map_or_else(
            || PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(under_home),
            PathBuf::from,
        )
}
