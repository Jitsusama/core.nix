//! Building the checks a run named: which still need building, and
//! building them on the lent builder while one stands, or here.

use std::collections::BTreeSet;
use std::env;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use crate::checks::Derivations;
use crate::evaluating::PURE;
use crate::quarantine::{self, Held};
use crate::there::{self, Sent};
use crate::{offload_dir, run, state};

/// Set on the far side's run to the checks that passed at its commit
/// already, which it builds none of again (`passed_at` says how).
const PASSED: &str = "CHANGED_CHECKS_PASSED";

/// Those of the checks a comparison moved that `passed` doesn't say passed
/// at the commit already, saying of each it leaves out, when building, that
/// it isn't built again.
pub fn unpassed(
    moved: &[&str],
    passed: &[String],
    commit: Option<&str>,
    building: bool,
) -> Vec<String> {
    moved
        .iter()
        .copied()
        .filter(|name| {
            let already = passed.iter().any(|check| check == name);
            if already && building {
                let short = commit.and_then(|commit| commit.get(..12));
                eprintln!(
                    "changed-checks: {name} passed at {} already, so it isn't built again",
                    short.unwrap_or_default()
                );
            }
            !already
        })
        .map(str::to_owned)
        .collect()
}

/// A build of the checks a run named: how many, how long it took, and how
/// it ended.
pub struct Built {
    pub checks: usize,
    pub took: Duration,
    pub sent: Sent,
}

/// Build `targets`, each that main quarantines alone after the rest, from
/// the derivations `after` evaluated for them. A quarantined check's
/// failure blocks nothing unless its entry names subtests and its log
/// names a failure outside them, or none at all.
pub fn built(
    flake: &str,
    system: &str,
    held: &[Held],
    targets: &[String],
    after: &Derivations,
) -> Result<Built, String> {
    let began = Instant::now();
    let (quarantined, blocking): (Vec<&str>, Vec<&str>) = targets
        .iter()
        .map(String::as_str)
        .partition(|name| quarantine::find(held, name).is_some());
    let mut sent = Sent {
        passed: true,
        there: false,
        bytes: None,
        failed: BTreeSet::new(),
    };
    if !blocking.is_empty() {
        sent = build(flake, system, &blocking, after)?;
    }
    // Each is built alone, so a failure is known for the check it was.
    for name in quarantined {
        let alone = build(flake, system, &[name], after)?;
        if let Some(entry) = quarantine::find(held, name).filter(|_| !alone.passed) {
            if entry.excuses(&alone.failed) {
                let why = entry.said();
                eprintln!("changed-checks: {name} failed, but {why}, so it blocks nothing");
            } else {
                let unlisted: Vec<&str> = alone
                    .failed
                    .iter()
                    .filter(|subtest| !entry.subtests.contains(*subtest))
                    .map(String::as_str)
                    .collect();
                let which = if unlisted.is_empty() {
                    "no test its log names".to_owned()
                } else {
                    unlisted.join(", ")
                };
                eprintln!(
                    "changed-checks: {name} failed in {which}, which main's quarantine doesn't \
                     excuse, so it blocks"
                );
                sent.passed = false;
            }
        }
        sent.there |= alone.there;
        // Each build there sent its own, so the run sent them all.
        sent.bytes = match (sent.bytes, alone.bytes) {
            (Some(before), Some(now)) => Some(before + now),
            (before, now) => before.or(now),
        };
    }
    Ok(Built {
        checks: targets.len(),
        took: began.elapsed(),
        sent,
    })
}

/// Build `targets` on the lent builder while one stands, and otherwise
/// here, each from the derivation `after` evaluated for it.
pub fn build(
    flake: &str,
    system: &str,
    targets: &[&str],
    after: &Derivations,
) -> Result<Sent, String> {
    if there::lent(&offload_dir()) {
        let journal = state().join("nix-offload/there.jsonl");
        match published().and_then(|fetched| there::build(&fetched, system, targets, &journal)) {
            Ok(sent) => return Ok(sent),
            Err(why) => eprintln!("changed-checks: building here, since {why}"),
        }
    }
    // Each check is built from the derivation just evaluated, so no
    // evaluator runs beside the builds: one holds its whole heap until its
    // last build ends, 1.4 to 4 GiB through a VM test. A derivation the
    // store has collected since is built from the flake, as before.
    let (built, failed) =
        quarantine::heard(
            Command::new("nix")
                .args(["build", "--no-link", "--keep-going", "-L"])
                .args(PURE)
                .args(standing())
                .args(targets.iter().map(
                    |name| match after.get(*name).and_then(Option::as_deref) {
                        Some(derivation) if Path::new(derivation).exists() => {
                            format!("{derivation}^*")
                        }
                        _ => format!("{flake}#checks.{system}.{name}"),
                    },
                )),
        )
        .map_err(|error| format!("running nix: {error}"))?;
    Ok(Sent {
        passed: built.success(),
        there: false,
        bytes: None,
        failed,
    })
}

/// What a build here asks for: the standing builders, where the machine
/// has them. Those are never offered to every build, since they may be
/// machines only public code may reach; core.nix's checks are public code
/// with public inputs alone, which `stands-alone` holds, so they may go
/// there.
pub fn standing() -> Vec<String> {
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
pub fn published() -> Result<String, String> {
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

/// Which checks, by name, `CHANGED_CHECKS_PASSED` says passed at `commit`,
/// the working tree's when it's clean: its commit, then each name. A
/// check that passed at a commit passes there again, so nix-offload, which
/// keeps what passed wherever it was built, says so when it sends a run;
/// told of another commit, or of a tree with changes, it names none.
pub fn passed_at(commit: Option<&str>) -> Vec<String> {
    let told = env::var(PASSED).unwrap_or_default();
    let mut words = told.split_whitespace();
    match (words.next(), commit) {
        (Some(told), Some(commit)) if told == commit => words.map(str::to_owned).collect(),
        _ => Vec::new(),
    }
}

/// The working tree's commit, when nothing in it differs from the commit
/// in what Nix reads: no file git tracks, or has been told of, changed.
pub fn clean_commit() -> Result<Option<String>, String> {
    let changed = run("git", &["status", "--porcelain", "--untracked-files=no"])?;
    if !changed.is_empty() {
        return Ok(None);
    }
    run("git", &["rev-parse", "HEAD"]).map(Some)
}
