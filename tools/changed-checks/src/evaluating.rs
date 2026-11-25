//! Evaluating a flake's checks to their derivations, a chunk at a time,
//! always pure and past nix's evaluation cache, and counting them to
//! choose the system whose checks are compared.

use std::env;
use std::time::Instant;

use crate::checks::{self, Derivations};
use crate::runs::{Evaluation, Run};
use crate::{HERE, pressure, run};

/// Each evaluation goes past nix's evaluation cache. Evaluated through
/// `--apply`, a result can't be kept there, so the cache saved nothing (one
/// chunk took 115 s with it and 108 to 127 s without), while two evaluations
/// at once, or two runs, found its database busy.
pub const UNCACHED: &str = "--no-eval-cache";

/// Every evaluation is pure, and no flake's own config is taken. A fenced
/// session's run goes to a lent builder with a flake it chose, which may
/// read only what the commit holds there (ISXEG8, 11 November): an impure
/// evaluation reads the machine's files and environment, and a flake's
/// `nixConfig`, once taken, could turn purity off again.
pub const PURE: [&str; 6] = [
    "--option",
    "pure-eval",
    "true",
    "--option",
    "accept-flake-config",
    "false",
];

/// Nix's words `asked`, then those that keep its evaluation [`PURE`].
pub fn pure<'a>(asked: &[&'a str]) -> Vec<&'a str> {
    asked.iter().copied().chain(PURE).collect()
}

/// Evaluate each of a flake's checks for a system to its derivation's path,
/// noting in the run what each chunk of them took, as the side named.
pub fn derivations(
    flake: &str,
    system: &str,
    side: &'static str,
    noted: &mut Run,
) -> Result<Derivations, String> {
    let mut evaluation = Run::evaluating(side);
    let found = evaluated(flake, system, &names(flake, system)?, &mut evaluation)?;
    noted.evaluated(evaluation);
    Ok(found)
}

/// The names of a flake's checks for a system.
pub fn names(flake: &str, system: &str) -> Result<Vec<String>, String> {
    let checks = format!("{flake}#checks.{system}");
    let naming: [&str; 6] = [
        "eval",
        "--json",
        &checks,
        "--apply",
        "builtins.attrNames",
        UNCACHED,
    ];
    let listed = run("nix", &pure(&naming))?;
    serde_json::from_str(&listed).map_err(|error| format!("nix named the checks oddly: {error}"))
}

/// Evaluate these of a flake's checks for a system to their derivations'
/// paths, noting in the evaluation what each chunk of them took.
pub fn evaluated(
    flake: &str,
    system: &str,
    names: &[String],
    evaluation: &mut Evaluation,
) -> Result<Derivations, String> {
    // As many go at once as a builder made for the run has room for, and two
    // on the person's own machine (`checks::at_once` says why).
    let at_once = checks::at_once_here(env::var_os(HERE).is_some());
    evaluated_by(flake, system, names, evaluation, at_once)
}

/// Evaluate as `evaluated` does, `at_once` chunks at a time.
pub fn evaluated_by(
    flake: &str,
    system: &str,
    names: &[String],
    evaluation: &mut Evaluation,
    at_once: usize,
) -> Result<Derivations, String> {
    let checks = format!("{flake}#checks.{system}");
    let chunks = checks::chunks(names);
    // The VM tests' chunks begin first, as they take longest, and the answers
    // stay in the names' order. A further chunk begins only while memory
    // isn't stalling: two at once while it was full stalled Joel's launcher
    // on 29 October.
    let evaluated = checks::each_at_once_in(
        &chunks,
        &checks::begin_order(&chunks),
        at_once,
        || !pressure::memory_stalled(),
        |chunk| {
            let selecting = checks::selecting(chunk)?;
            let began = Instant::now();
            let asked: [&str; 6] = ["eval", "--json", &checks, "--apply", &selecting, UNCACHED];
            let evaluated = run("nix", &pure(&asked))?;
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
    Ok(found)
}

/// The system whose checks are the most, `checks::busiest` says why. Only
/// the names are counted, which took 1.1 s and 158 MB for core's 580 checks
/// on 30 October, against 4.8 GB for evaluating them. They're counted
/// through the flake's own `checks`, as the rest is evaluated, since
/// `builtins.getFlake` on a flake not yet locked needs `--impure`.
pub fn busiest_system(flake: &str) -> Result<String, String> {
    let here = this_system();
    let checks = format!("{flake}#checks");
    let counting = "builtins.mapAttrs (_: checks: builtins.length (builtins.attrNames checks))";
    let counted = run(
        "nix",
        &pure(&["eval", "--json", &checks, "--apply", counting]),
    )?;
    let counts = serde_json::from_str(&counted)
        .map_err(|error| format!("reading how many checks each system has: {error}"))?;
    Ok(checks::busiest(&counts, &here))
}

/// The Nix system this program was built for, which is this machine's:
/// read without asking Nix, whose evaluation of `builtins.currentSystem`
/// waited 30 minutes at the jobs door on 10 November before a run it began
/// could be sent anywhere.
fn this_system() -> String {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        os => os,
    };
    format!("{}-{os}", std::env::consts::ARCH)
}
