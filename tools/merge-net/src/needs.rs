//! `merge-net needs`, on a branch: whether its change runs VM tests before
//! it merges, rather than after on the net, and which. It names each VM
//! test whose area the change touches (`rules.rs`) and each that owns one
//! of its files (`owners.rs`), and keeps only those whose derivation moved
//! from main, read from a run that passed at a clean head where one did
//! (`gated.rs`) and asked of changed-checks otherwise.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use crate::state::{self, short};
use crate::{SYSTEM, comments, gated, git, owners, ready, rules, world};

/// Say whether the branch's change runs VM tests before merging, and which.
pub fn needs(base: &str) -> Result<ExitCode, String> {
    let began = state::now();
    let head = git(&["rev-parse", "HEAD"])?;
    let merge_base = git(&["merge-base", "HEAD", base])?;
    let mut files: Vec<String> = git(&["diff", "--name-only", &merge_base])?
        .lines()
        .chain(git(&["ls-files", "--others", "--exclude-standard"])?.lines())
        .map(str::to_owned)
        .collect();
    files.sort();
    files.dedup();
    let files = comments::reaching(files, |file| git(&["diff", &merge_base, "--", file]).ok());
    let areas = rules::areas();
    let known = known_vm_tests();
    let root = git(&["rev-parse", "--show-toplevel"])?;
    let owning = owners::in_tree(Path::new(&root), &files).unwrap_or_else(|| {
        println!("(checks.nix or flake.nix didn't read, so no VM test is named for its files)");
        Vec::new()
    });
    if owning.is_empty()
        && rules::needs(&areas, &files, known.as_deref())
            .touched
            .is_empty()
    {
        println!("no: nothing here needs its VM tests before merging; the net runs them after");
        gated::keep(&head, began, gated::Asked::Nothing);
        return Ok(ExitCode::SUCCESS);
    }
    // Only a test whose derivation moved from main can give a result main
    // hasn't, so changed-checks is asked which moved; it costs an
    // evaluation, which only a change touching an area or a VM test's
    // files pays, and none whose gate already passed at its head.
    let clean = git(&["status", "--porcelain"]).is_ok_and(|status| status.is_empty());
    let came_before = |commit: &str| {
        Command::new("git")
            .args(["merge-base", "--is-ancestor", commit, &merge_base])
            .status()
            .is_ok_and(|status| status.success())
    };
    let (said, asked) = gated::moved(
        &ready::record(&head),
        clean,
        &merge_base,
        &came_before,
        || moved_from(base),
    );
    gated::keep(&head, began, asked);
    if asked == gated::Asked::Gate {
        println!(
            "(a run that passed at {} named what moved, so none was evaluated)",
            short(&head)
        );
    }
    if said.is_none() {
        println!("(changed-checks couldn't say which VM tests moved, so every one named runs)");
    }
    let found = rules::first(&areas, &files, said.as_deref(), known.as_deref());
    let moved = said.as_deref().map(rules::moved);
    let owning: Vec<String> = owning
        .into_iter()
        .filter(|test| moved.as_ref().is_none_or(|moved| moved.contains(test)))
        .collect();
    // The list is read from the base, main, so a branch can't quarantine
    // its own failure.
    let list = git(&["show", &format!("{base}:{}", world::QUARANTINE)])
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    if found.tests.is_empty() && found.there.is_empty() && owning.is_empty() {
        println!(
            "no: none of the VM tests the change touches moved from {base}, so each would \
             give main's result; the net runs the rest after"
        );
        return Ok(ExitCode::SUCCESS);
    }
    println!("yes:");
    said_owners(&owning, base, &list);
    if found.touched.is_empty() {
        return Ok(ExitCode::SUCCESS);
    }
    println!("the change touches");
    for (area, hits) in &found.touched {
        println!("  {area}: {}", hits.join(" "));
    }
    for area in &found.there {
        println!(
            "send one VM test through the changed {area}, end to end: run one VM test's \
             subtests with run-subtests --there, with the change's own {area} first on PATH"
        );
    }
    if found.tests.is_empty() {
        println!("no VM test it runs moved from {base}, so none runs whole first");
        return Ok(ExitCode::SUCCESS);
    }
    println!("run these VM tests whole before merging, since they moved from {base}:");
    for test in &found.tests {
        println!("{}", rules::named(test, &list));
    }
    if known.is_none() {
        println!("(the names are patterns: no list of the checks was found to match them against)");
    }
    Ok(ExitCode::SUCCESS)
}

/// Name the VM tests that own a file the change touches, each of which is
/// the one written to catch it, so it runs before the merge rather than
/// on main.
fn said_owners(owning: &[String], base: &str, list: &serde_json::Value) {
    if owning.is_empty() {
        return;
    }
    println!(
        "run the subtests that reach what it touched with run-subtests --there, in each VM \
         test that owns one of its files and moved from {base}:"
    );
    for test in owning {
        println!("  {}", rules::named(test, list));
    }
}

/// What the changed-checks it was built with (the profile has none) says
/// moved between `base` and the working copy, or nothing when it couldn't compare.
fn moved_from(base: &str) -> Option<String> {
    let ran = Command::new(option_env!("MERGE_NET_CHANGED_CHECKS").unwrap_or("changed-checks"))
        .args(rules::changed_checks_asks(base, SYSTEM))
        .stderr(Stdio::inherit())
        .output()
        .ok()?;
    let said = String::from_utf8_lossy(&ran.stdout).into_owned();
    // It fails when a check no longer evaluates, and names it.
    (ran.status.success() || said.lines().any(|line| line.starts_with("broken"))).then_some(said)
}

/// The VM tests core has, from the list each new main writes
/// (`~/.local/state/check-names/core.json`), where there is one, since
/// evaluating the flake to name them waits on the machine's memory.
fn known_vm_tests() -> Option<Vec<String>> {
    let path = PathBuf::from(env::var_os("HOME")?).join(".local/state/check-names/core.json");
    let names: serde_json::Value = serde_json::from_str(&fs::read_to_string(path).ok()?).ok()?;
    let tests: Vec<String> = names["checks"][SYSTEM]
        .as_array()?
        .iter()
        .filter_map(|name| name.as_str())
        .filter(|name| name.starts_with("vm-"))
        .map(str::to_owned)
        .collect();
    (!tests.is_empty()).then_some(tests)
}
