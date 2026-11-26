//! A red candidate whose every failed check is no rider's lands rather
//! than splits. On 26 November main's own formatting red (Presented.js,
//! past biome's 500 lines after #6727) split candidate a8f33ede, and the
//! train was stopped by hand so it wouldn't send riders back to run alone
//! for it; on 26 November too #6524 touched no margin-cat file and its gate
//! failed the-margin-cat-follows-the-keys-on-niri, which had failed now and
//! then on main.
//!
//! Each check the gate's words say failed is built once more, at the
//! commit the candidate was cut on, on a lent builder, so like is compared
//! with like; a fresh main tests something else, and the train cuts again
//! when main moves anyway. One that fails there too is main's own red,
//! not the riders'. One that passes there is the riders' unless it is a
//! VM test whose files, as `owners` reads them, no rider touches: a VM
//! test runs a whole machine, and fails now and then by itself, where any
//! other check builds the same from the same files, so a pass at the base
//! says the riders broke it. A check the reading can't place, a failed
//! derivation no check names, a build at the base no builder took, and any
//! change to flake.nix or flake.lock, which every test reads, count as the
//! riders', so the train splits on them as before. Each check excused is
//! told to its owner as a finding, whether or not the rest of the red is.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::{Value, json};

use super::gated::{Gated, words_of};
use crate::state::{State, now, short};

/// Whose a failed check's red is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Blame {
    /// The riders broke it, so the train splits as before.
    Riders,
    /// It passed at the base and no rider touches what it reads.
    Flaky,
    /// It fails at the base too.
    Main,
}

/// How a red is judged: what each part asks of the world.
pub(super) struct Judging<'a> {
    /// The check each derivation name builds, for names no check has.
    pub named: &'a dyn Fn(&[String]) -> BTreeMap<String, String>,
    /// Which of the checks no rider touches.
    pub untouched: &'a dyn Fn(&[String]) -> BTreeSet<String>,
    /// How the check built at the base.
    pub at_base: &'a mut dyn FnMut(&str) -> Gated,
}

/// The names of the derivations whose builders failed, as the gate's
/// `words` say: `treefmt-check` for nix's "Cannot build
/// '/nix/store/<hash>-treefmt-check.drv'" followed by its reason, "builder
/// failed". One that couldn't build for a dependency names no check of its
/// own: its dependency's line does.
pub(super) fn failed_derivations(words: &str) -> Vec<String> {
    let mut failed = Vec::new();
    let mut lines = words.lines().peekable();
    while let Some(line) = lines.next() {
        let Some(path) = line
            .split_once("Cannot build '/nix/store/")
            .and_then(|(_, rest)| rest.split_once(".drv'"))
            .map(|(path, _)| path)
        else {
            continue;
        };
        let builder_failed = lines
            .peek()
            .is_some_and(|next| next.trim_start().starts_with("Reason: builder failed"));
        let name = path.split_once('-').map_or(path, |(_, name)| name);
        if builder_failed && !failed.iter().any(|known| known == name) {
            failed.push(name.to_owned());
        }
    }
    failed
}

/// The checks the gate's `words` name as added or changed, which it built,
/// a row each of changed-checks' table: its kind, then its name.
pub(super) fn built(words: &str) -> Vec<String> {
    words
        .lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let kind = words.next()?;
            let name = words.next()?;
            matches!(kind, "added" | "changed").then(|| name.to_owned())
        })
        .collect()
}

/// What changed-checks says when a check the riders touched no longer
/// evaluates, which no derivation's failure names, so it is theirs.
const NO_LONGER_EVALUATES: &str = "a check the change touched no longer evaluates";

/// A VM test's derivation is named for its run: `vm-test-run-vm-pi`.
const VM_RUN: &str = "vm-test-run-";

/// The check each of the `failed` derivations builds, of the checks
/// `built`: the one of its name, the VM test it runs, or the one `named`
/// says, asked only of names neither gives; none when one of them names
/// no check, since what broke can't then be told.
pub(super) fn checks_of(
    failed: &[String],
    built: &[String],
    named: &dyn Fn(&[String]) -> BTreeMap<String, String>,
) -> Option<Vec<String>> {
    let by_name = |name: &str| {
        let check = name.strip_prefix(VM_RUN).unwrap_or(name);
        built
            .iter()
            .any(|one| one == check)
            .then(|| check.to_owned())
    };
    let unplaced: Vec<String> = failed
        .iter()
        .filter(|name| by_name(name).is_none())
        .cloned()
        .collect();
    let others: Vec<String> = built
        .iter()
        .filter(|check| {
            !failed
                .iter()
                .any(|name| by_name(name).as_deref() == Some(check))
        })
        .cloned()
        .collect();
    let named = if unplaced.is_empty() {
        BTreeMap::new()
    } else {
        named(&others)
    };
    failed
        .iter()
        .map(|name| by_name(name).or_else(|| named.get(name).cloned()))
        .collect()
}

/// Whose the red of a check is, built at the base as `at_base` says, and
/// `untouched` when no rider touches what it reads.
pub(super) fn blame(at_base: &Gated, untouched: bool) -> Blame {
    match at_base {
        Gated::Red => Blame::Main,
        Gated::Passed if untouched => Blame::Flaky,
        Gated::Passed | Gated::Unsent => Blame::Riders,
    }
}

/// Each check the candidate failed, as its gate's `words` say, with whose
/// red it is; none when the words name no failed check, or one no check
/// builds, so the red is the riders' as before. Once one is the riders',
/// the red is, so those after it are counted theirs unbuilt: a rider that
/// broke the shell failed 25 niri checks at once on 26 November.
pub(super) fn judge(words: &str, judging: &mut Judging<'_>) -> Option<Vec<(String, Blame)>> {
    let failed = failed_derivations(words);
    if failed.is_empty() || words.contains(NO_LONGER_EVALUATES) {
        return None;
    }
    let checks = checks_of(&failed, &built(words), judging.named)?;
    let untouched = (judging.untouched)(&checks);
    let mut riders = false;
    Some(
        checks
            .into_iter()
            .map(|check| {
                if riders {
                    return (check, Blame::Riders);
                }
                let at_base = (judging.at_base)(&check);
                let blamed = blame(&at_base, untouched.contains(&check));
                riders = blamed == Blame::Riders;
                (check, blamed)
            })
            .collect(),
    )
}

/// Whether a red whose failed checks are `judged` so is excused: some
/// failed, and none of them is the riders'.
pub(super) fn excused(judged: &[(String, Blame)]) -> bool {
    !judged.is_empty() && judged.iter().all(|(_, blamed)| *blamed != Blame::Riders)
}

/// What every VM test reads, so a change to one moves them all.
const EVERY_TEST_READS: [&str; 2] = ["flake.nix", "flake.lock"];

/// Which of `checks` no file of `files` touches, read from the checkout at
/// `root`: the VM tests `owners` finds, owning none of them. None while the
/// change moves what every test reads, or while the reading fails.
pub(super) fn untouched(root: &Path, files: &[String], checks: &[String]) -> BTreeSet<String> {
    if files
        .iter()
        .any(|file| EVERY_TEST_READS.contains(&file.as_str()))
    {
        return BTreeSet::new();
    }
    let read = |file: &str| fs::read_to_string(root.join(file)).ok();
    let Some(known) = read("checks.nix").map(|text| {
        crate::owners::every_vm_test(&text, read)
            .into_iter()
            .map(|test| test.name)
            .collect::<BTreeSet<_>>()
    }) else {
        return BTreeSet::new();
    };
    let Some(owning) = crate::owners::in_tree(root, files) else {
        return BTreeSet::new();
    };
    checks
        .iter()
        .filter(|check| known.contains(*check))
        .filter(|check| !owning.tests.contains(check) && !owning.unread.contains(check))
        .cloned()
        .collect()
}

/// The candidate a red is judged for: where it was cut, and its words.
struct Red<'a> {
    /// The train's checkout, at the candidate.
    checkout: &'a Path,
    /// The commit the candidate was cut on.
    base: &'a str,
    /// The candidate.
    candidate: &'a str,
}

/// After the `candidate` cut on `base` in the train's `checkout` failed
/// its gate, whose words are in `log`, under the net's state folder
/// `root`: whether every check it failed is main's or a flake's, each
/// built again at its base on a lent builder from the repository at `url`,
/// so it lands rather than splits. Each check excused is told to its owner
/// as a finding.
pub(super) fn excuses(
    root: &Path,
    url: &str,
    checkout: &Path,
    base: &str,
    candidate: &str,
    log: &Path,
) -> bool {
    let red = &Red {
        checkout,
        base,
        candidate,
    };
    let owner = |check: &str, failed: &[String]| owner_in(root, url, base, check, failed);
    let words = words_of(log);
    let files: Vec<String> = super::run_git(
        red.checkout,
        &["diff", "--name-only", red.base, red.candidate],
    )
    .map(|said| said.lines().map(str::to_owned).collect())
    .unwrap_or_default();
    let flake = format!("github:{}/{}", super::slug_of(url), red.base);
    let mut at_base = |check: &str| rebuilt(root, &flake, red, check);
    let mut judging = Judging {
        named: &|checks| named_at(red.checkout, &flake, checks),
        // A diff that can't be read touches everything.
        untouched: &|checks| {
            if files.is_empty() {
                BTreeSet::new()
            } else {
                untouched(red.checkout, &files, checks)
            }
        },
        at_base: &mut at_base,
    };
    let Some(judged) = judge(&words, &mut judging) else {
        return false;
    };
    let failed: Vec<String> = judged.iter().map(|(check, _)| check.clone()).collect();
    let state = State::at(root.to_path_buf());
    for (check, blamed) in &judged {
        let kind = match blamed {
            Blame::Riders => continue,
            Blame::Flaky => "flaky",
            Blame::Main => "failed-before",
        };
        let finding = json!({
            "kind": kind,
            "test": check,
            "commit": red.base,
            "candidate": red.candidate,
            "owner_pr": owner(check, &failed),
            "log": base_log(root, red.candidate, check).display().to_string(),
            "at": now(),
        });
        if let Err(why) = state.find(&finding) {
            println!("merge-net: the finding for {check} wasn't written: {why}");
        }
    }
    let said = judged
        .iter()
        .map(|(check, blamed)| format!("{check} {}", said(*blamed)))
        .collect::<Vec<_>>()
        .join("; ");
    let shown = short(red.candidate);
    if excused(&judged) {
        println!(
            "merge-net: the candidate {shown} failed only checks that aren't its riders': \
             {said}, so it lands"
        );
        true
    } else {
        println!("merge-net: the candidate {shown} failed changed-checks: {said}");
        false
    }
}

/// The PR that owns `check`, failed beside `failed`, as the net's watch
/// reads an owner: the newest merge from the head it last passed to the
/// candidate's `base` that moved the check, at the repository `url`, under
/// the state folder `root`; none where it passed nothing yet, or finds no
/// such merge.
fn owner_in(root: &Path, url: &str, base: &str, check: &str, failed: &[String]) -> Option<u64> {
    let passed = State::at(root.to_path_buf()).heads().0?;
    let world = crate::world::Real {
        state: root.to_path_buf(),
        url: url.to_owned(),
        system: crate::SYSTEM.to_owned(),
        booted: 0,
        branch: "main".to_owned(),
        moving: std::sync::Mutex::new(()),
        dropped: std::sync::Mutex::default(),
        follow: None,
    };
    crate::watch::owner_of(&world, &passed, base, check, failed)
}

/// How the train says whose a red is.
fn said(blamed: Blame) -> &'static str {
    match blamed {
        Blame::Riders => "is its riders'",
        Blame::Flaky => "passed at the base, and no rider touches it, so it's a flake",
        Blame::Main => "fails at the base too, so it's main's",
    }
}

/// Where the build of `check` at a candidate's base writes its words.
fn base_log(root: &Path, candidate: &str, check: &str) -> std::path::PathBuf {
    super::folder(root).join(format!("{candidate}-{check}-at-base.log"))
}

/// Build `check` once more at `red`'s base, the flake `flake`, on a lent
/// builder only: how it ended.
fn rebuilt(root: &Path, flake: &str, red: &Red<'_>, check: &str) -> Gated {
    let log = base_log(root, red.candidate, check);
    let Ok(words) = fs::File::create(&log) else {
        return Gated::Unsent;
    };
    let Ok(errors) = words.try_clone() else {
        return Gated::Unsent;
    };
    println!(
        "merge-net: the train builds {check} again at {}, its words in {}",
        short(red.base),
        log.display()
    );
    let ran = Command::new("nix-offload")
        .args([
            "there",
            "--lent-only",
            "-L",
            flake,
            &format!("checks.{}.{check}", crate::SYSTEM),
        ])
        .current_dir(red.checkout)
        .env_remove("SSH_AUTH_SOCK")
        .stdout(words)
        .stderr(errors)
        .status();
    ran.map_or(Gated::Unsent, |status| {
        at_base(status.code(), &words_of(&log))
    })
}

/// How a check built at the base ended, from its exit `code` and `words`:
/// red only where a builder failed, since then main itself fails it. Any
/// other failure, an evaluation's or a fetch's, says nothing of main's
/// check, and a check the riders add isn't at the base to evaluate, so it
/// reads as untaken and the red stays the riders'.
pub(super) fn at_base(code: Option<i32>, words: &str) -> Gated {
    match code {
        Some(0) => Gated::Passed,
        _ if !failed_derivations(words).is_empty() => Gated::Red,
        _ => Gated::Unsent,
    }
}

/// The check each derivation name of `checks` builds at `flake`, read on
/// a lent builder; nothing where it can't be read.
fn named_at(checkout: &Path, flake: &str, checks: &[String]) -> BTreeMap<String, String> {
    let names: Vec<String> = checks
        .iter()
        .map(|check| Value::from(check.as_str()).to_string())
        .collect();
    let apply = format!(
        "checks: builtins.listToAttrs (map (check: \
         {{ name = checks.${{check}}.name; value = check; }}) [ {} ])",
        names.join(" ")
    );
    let attribute = format!("{flake}#checks.{}", crate::SYSTEM);
    Command::new("nix-offload")
        .args(crate::world::eval_args(
            &attribute,
            &["--json", "--apply", &apply],
        ))
        .current_dir(checkout)
        .output()
        .ok()
        .filter(|ran| ran.status.success())
        .and_then(|ran| serde_json::from_slice::<BTreeMap<String, String>>(&ran.stdout).ok())
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "flaky/tests.rs"]
mod tests;
