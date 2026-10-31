//! Names the checks a change touches, and builds only those.
//!
//! A check whose derivation is the same as at the base proves nothing new,
//! since its result is already what it was. This evaluates every check's
//! derivation at the base, the merge base with origin's main unless told
//! otherwise, and in the working tree, and names each check added, changed
//! or removed, and each that no longer evaluates, then builds those added
//! and changed, from the derivations it evaluated: all but the VM tests,
//! and with `--build` those too unless `--no-vm` says not. A named check
//! proves nothing until it's built, and core's main once broke on a change
//! that named checks without building them. `--name-only` builds nothing.
//! With no `--system` it compares the system with the most checks, this
//! machine's on a tie, since a repository may hold some on one system
//! alone, and a run on a Mac passed over core's Linux-only checks.
//! With `--keep-base` it only evaluates the base
//! and keeps its derivations, comparing nothing, so a run after each merge
//! leaves every later comparison against that commit one evaluation to pay.
//!
//! Usage: `changed-checks [--base REF] [--system SYSTEM] [--build] [--no-vm] [--name-only]
//! [--keep-base] [--here]`
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
//! The whole run goes to another machine where nix-offload can send it
//! (`nix-offload there --changed`): the lent builder, or a machine that
//! evaluates with its builds sent on. Each evaluation here held about
//! 4.8 GB and several cores of the person's machine. The far side runs
//! the commit's own changed-checks with the same arguments and marked so
//! it never sends itself on, a `--base` given as its merge base's commit.
//! With `--here`, a base whose merge base origin's main doesn't hold,
//! which the far side wouldn't have, edits not yet committed, which it
//! would pass over, or nowhere to send it, it runs here. While nix-offload lends a builder, a build here is sent
//! to it as a command naming a commit origin has, and the builder fetches
//! that commit, evaluates the checks and builds them itself (`there.rs`
//! says why).
//!
//! Each run that reaches its base adds a line to `changed-checks/runs.jsonl`
//! under XDG's state folder, saying what each evaluation and the build took
//! (`runs.rs` says what it holds).

mod checks;
mod pressure;
mod runs;
mod there;

use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Instant;

use checks::Derivations;
use runs::Run;

/// What a run asked for is refused with, as sysexits.h numbers it.
const USAGE: u8 = 64;

const USED: &str = "Usage: changed-checks [--base REF] [--system SYSTEM] [--build] [--no-vm] \
                    [--name-only] [--keep-base] [--here]";

/// The base a run compares with unless told otherwise.
const ORIGIN_MAIN: &str = "refs/remotes/origin/main";

/// What `nix-offload there --changed` exits with when nowhere else took the
/// run, so it runs here.
const NOT_TAKEN: i32 = 75;

/// Set on the far side's run, which runs there and never sends itself on.
const HERE: &str = "CHANGED_CHECKS_HERE";

/// What a run was asked for.
#[expect(
    clippy::struct_excessive_bools,
    reason = "each is a flag of its own on the command line, set apart from the rest"
)]
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
    /// Whether to run here even where another machine could.
    here: bool,
    /// The arguments as given, for a run sent elsewhere.
    given: Vec<String>,
}

impl Options {
    /// Read the command line's arguments, or say what's wrong with them.
    fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let given: Vec<String> = arguments.collect();
        let mut arguments = given.clone().into_iter();
        let mut options = Self {
            base: ORIGIN_MAIN.to_owned(),
            system: None,
            build: true,
            vm: false,
            keep_base: false,
            here: false,
            given: Vec::new(),
        };
        let (mut whole, mut no_vm, mut name_only) = (false, false, false);
        while let Some(argument) = arguments.next() {
            let mut value = || {
                arguments
                    .next()
                    .ok_or_else(|| format!("{argument} needs a value"))
            };
            match argument.as_str() {
                "--base" => options.base = value()?,
                "--system" => options.system = Some(value()?),
                "--build" => whole = true,
                "--no-vm" => no_vm = true,
                "--name-only" => name_only = true,
                "--keep-base" => options.keep_base = true,
                "--here" => options.here = true,
                _ => return Err(format!("{argument} isn't something it takes")),
            }
        }
        if whole && name_only {
            return Err("--build and --name-only ask for opposite things".to_owned());
        }
        options.build = !name_only;
        options.vm = whole && !no_vm;
        options.given = given;
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
    let flake = format!("git+file://{root}");
    let system = match &options.system {
        Some(system) => system.clone(),
        None => busiest_system(&flake)?,
    };
    let untracked = run(
        "git",
        &["-C", &root, "ls-files", "--others", "--exclude-standard"],
    )?;
    if !untracked.is_empty() {
        eprintln!("changed-checks: Nix doesn't read files git doesn't know:\n{untracked}");
    }
    if let Some(code) = elsewhere(options, &root, &base, &system)? {
        return Ok(code);
    }
    let mut noted = Run::new(&base, &system);
    let compared = compare(options, &flake, &base, &system, &mut noted);
    let log = state().join("changed-checks/runs.jsonl");
    if let Err(why) = noted.record(&log) {
        eprintln!("changed-checks: couldn't log the run: {why}");
    }
    compared
}

/// The system whose checks are the most, `checks::busiest` says why. Only
/// the names are counted, which took 1.1 s and 158 MB for core's 580 checks
/// on 30 October, against 4.8 GB for evaluating them.
fn busiest_system(flake: &str) -> Result<String, String> {
    let here = run(
        "nix",
        &[
            "eval",
            "--raw",
            "--impure",
            "--expr",
            "builtins.currentSystem",
        ],
    )?;
    let counting = format!(
        "builtins.mapAttrs (_: checks: builtins.length (builtins.attrNames checks)) \
         ((builtins.getFlake \"{flake}\").checks or {{ }})"
    );
    let counted = run("nix", &["eval", "--json", "--impure", "--expr", &counting])?;
    let counts = serde_json::from_str(&counted)
        .map_err(|error| format!("reading how many checks each system has: {error}"))?;
    Ok(checks::busiest(&counts, &here))
}

/// The run sent to another machine by `nix-offload there --changed`, its
/// exit, or none when it runs here: asked to, already the far side's, a
/// base whose merge base origin's main doesn't hold, a nix-offload that
/// doesn't take it, edits not yet in a commit, or nowhere that took it.
fn elsewhere(
    options: &Options,
    root: &str,
    base: &str,
    system: &str,
) -> Result<Option<ExitCode>, String> {
    if options.here || env::var_os(HERE).is_some() {
        return Ok(None);
    }
    // The far side fetches origin's main, so it has any commit main holds,
    // and the merge base is what the comparison starts from either way.
    let holds = Command::new("git")
        .args(["-C", root, "merge-base", "--is-ancestor", base, ORIGIN_MAIN])
        .status()
        .is_ok_and(|status| status.success());
    if options.base != ORIGIN_MAIN && !holds {
        eprintln!(
            "changed-checks: origin's main doesn't hold {}, so the run stays on this machine",
            base.get(..12).unwrap_or(base)
        );
        return Ok(None);
    }
    let takes = Command::new("nix-offload")
        .args(["there", "--takes"])
        .stderr(std::process::Stdio::null())
        .output();
    let takes_it = takes.is_ok_and(|takes| {
        takes.status.success()
            && String::from_utf8_lossy(&takes.stdout)
                .lines()
                .any(|word| word == "changed")
    });
    if !takes_it {
        return Ok(None);
    }
    // The far side checks a commit, so edits not yet in one would be passed
    // over for the last commit's checks.
    if clean_commit()?.is_none() {
        eprintln!(
            "changed-checks: uncommitted changes stay on this machine; commit to send the run"
        );
        return Ok(None);
    }
    // A base is given as its merge base's commit, which a ref named only
    // here, such as a branch, wouldn't be on the far side.
    let mut given = Vec::new();
    let mut words = options.given.iter();
    while let Some(word) = words.next() {
        given.push(word.clone());
        if word == "--base" {
            words.next();
            given.push(base.to_owned());
        }
    }
    // The far side may be another system, which would name its own.
    if options.system.is_none() {
        given.extend(["--system".to_owned(), system.to_owned()]);
    }
    eprintln!("changed-checks: sending the run elsewhere; --here runs it on this machine");
    let status = Command::new("nix-offload")
        .args(["there", "--changed"])
        .args(&given)
        .current_dir(root)
        .status()
        .map_err(|error| format!("running nix-offload: {error}"))?;
    Ok(match status.code() {
        Some(NOT_TAKEN) => None,
        Some(code) => Some(ExitCode::from(u8::try_from(code).unwrap_or(1))),
        None => Some(ExitCode::FAILURE),
    })
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
    if there::lent(&offload_dir()) {
        let journal = state().join("nix-offload/there.jsonl");
        match published().and_then(|fetched| there::build(&fetched, system, &targets, &journal)) {
            Ok(sent) => {
                noted.built(targets.len(), began.elapsed(), sent.passed);
                if sent.there {
                    noted.lent(sent.bytes);
                }
                return Ok(if sent.passed {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                });
            }
            Err(why) => eprintln!("changed-checks: building here, since {why}"),
        }
    }
    // Each check is built from the derivation just evaluated, so no
    // evaluator runs beside the builds: one holds its whole heap until its
    // last build ends, 1.4 to 4 GiB through a VM test. A derivation the
    // store has collected since is built from the flake, as before.
    let built = Command::new("nix")
        .args(["build", "--no-link", "--keep-going", "-L"])
        .args(standing())
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

/// What a build here asks for: the standing builders, where the machine
/// has them. Those are never offered to every build, since they may be
/// machines only public code may reach; core.nix's checks are public code
/// with public inputs alone, which `stands-alone` holds, so they may go
/// there.
fn standing() -> Vec<String> {
    let with_standing = offload_dir().join("machines-with-standing");
    if with_standing.exists() {
        vec![
            "--builders".to_owned(),
            format!("@{}", with_standing.display()),
        ]
    } else {
        Vec::new()
    }
}

/// The flake a builder can fetch the working tree from by itself: its
/// commit, from origin, when the tree is that commit and origin has it.
/// Otherwise why not, since then only this machine has what's built.
fn published() -> Result<String, String> {
    let Some(commit) = clean_commit()? else {
        return Err("the working tree differs from its commit".to_owned());
    };
    let short = commit.get(..12).unwrap_or(&commit);
    let on = run(
        "git",
        &["branch", "-r", "--contains", &commit, "--list", "origin/*"],
    )?;
    if on.is_empty() {
        return Err(format!(
            "origin doesn't have {short}; push it to build there"
        ));
    }
    let url = run("git", &["remote", "get-url", "origin"])?;
    Ok(there::fetched(&url, &commit))
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
