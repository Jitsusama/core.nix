//! Names the checks a change touches, and builds only those.
//!
//! A check whose derivation is the same as at the base proves nothing new,
//! since its result is already what it was. This evaluates every check's
//! derivation at the base, the merge base with origin's main unless told
//! otherwise, and in the working tree, and names each check added, changed
//! or removed, and each that no longer evaluates. With `--build` it builds
//! those added and changed.
//!
//! Usage: `changed-checks [--base REF] [--system SYSTEM] [--build] [--no-vm]`
//!
//! The working tree is read as Nix reads a git checkout, so a new file
//! counts only once git knows it (`git add -N`). A change that only some
//! checks read, such as a module's, touches only those; one every check
//! reads, such as `flake.lock`, touches them all.
//!
//! The checks are evaluated a few dozen at a time, in a nix eval each, a VM
//! test counting four (`checks.rs` says why), rather than all in one, which
//! held every check at once and reached 24 GB on a machine many sessions
//! share; each evaluation's memory goes when it ends. A commit's derivations never change, so the base's are
//! kept under `~/.cache/changed-checks`, and only the working tree is
//! evaluated again. git and nix are the person's own, with their settings,
//! so they're found on the path rather than pinned.

mod checks;

use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

use checks::Derivations;

/// What a run asked for is refused with, as sysexits.h numbers it.
const USAGE: u8 = 64;

const USED: &str = "Usage: changed-checks [--base REF] [--system SYSTEM] [--build] [--no-vm]";

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
}

impl Options {
    /// Read the command line's arguments, or say what's wrong with them.
    fn parse(mut arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut options = Self {
            base: "refs/remotes/origin/main".to_owned(),
            system: None,
            build: false,
            vm: true,
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
    let short = base.get(..12).unwrap_or(&base);
    eprintln!("comparing {system}'s checks at {short} with the working tree");
    let before = checks::kept(&checks::keeping(&cache(), &base, &system), || {
        derivations(&format!("{flake}?rev={base}"), &system)
    })?;
    let comparison = checks::compare(&before, &derivations(&flake, &system)?);
    for (kind, names) in comparison.kinds() {
        for name in names {
            println!("{kind:<8} {name}");
        }
    }
    if !comparison.broken.is_empty() {
        return Ok(ExitCode::FAILURE);
    }
    let targets = comparison.to_build(options.vm);
    if !options.build || targets.is_empty() {
        return Ok(ExitCode::SUCCESS);
    }
    // A failing build is reported by its status, as nix reports it.
    let built = Command::new("nix")
        .args(["build", "--no-link", "--keep-going", "-L"])
        .args(
            targets
                .iter()
                .map(|name| format!("{flake}#checks.{system}.{name}")),
        )
        .status()
        .map_err(|error| format!("running nix: {error}"))?;
    Ok(if built.success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Evaluate each of a flake's checks for a system to its derivation's path.
fn derivations(flake: &str, system: &str) -> Result<Derivations, String> {
    let checks = format!("{flake}#checks.{system}");
    let listed = run(
        "nix",
        &["eval", "--json", &checks, "--apply", "builtins.attrNames"],
    )?;
    let names: Vec<String> = serde_json::from_str(&listed)
        .map_err(|error| format!("nix named the checks oddly: {error}"))?;
    let mut found = Derivations::new();
    for chunk in checks::chunks(&names) {
        let evaluated = run(
            "nix",
            &[
                "eval",
                "--json",
                &checks,
                "--apply",
                &checks::selecting(chunk)?,
            ],
        )?;
        let chunk: Derivations = serde_json::from_str(&evaluated)
            .map_err(|error| format!("nix gave the checks' derivations oddly: {error}"))?;
        found.extend(chunk);
    }
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
    env::var_os("XDG_CACHE_HOME")
        .filter(|cache| !cache.is_empty())
        .map_or_else(
            || PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".cache"),
            PathBuf::from,
        )
}
