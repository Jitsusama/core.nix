//! The net under main: merges land on their own checks, VM tests aside,
//! and the net runs the VM tests each one changed after it lands.
//!
//! `merge-net watch` follows origin's main from its own checkout. For each
//! new head it asks changed-checks which checks moved since the last head
//! whose VM tests all passed, and runs each VM test among them through
//! `nix-offload there`, which sends it to the VM builder, two at once. A
//! test that fails twice at a commit fails for real: the net finds which
//! merge broke it, holds switching on it (`hold.json`), reverts it through
//! a PR merged on its own checks, and writes a finding for its owner. One
//! that fails and then passes goes on the quarantine list
//! (`quarantine.json`) instead, and nothing is reverted for it. It starts
//! nothing while the machine is held or paused (`holds.rs`, `state.rs`).
//! `watch.rs` says what a round does; `state.rs` what it keeps, and where.
//!
//! `merge-net needs`, on a branch, says whether its change runs VM tests
//! before it merges, rather than after on the net, and which: a change to
//! what can leave a machine unable to boot, unlock or sign in, which a
//! revert after the fact comes too late for (`must-run.txt`).
//!
//! `merge-net status` says where the net is.
//!
//! `watch --branch` follows another branch than main, and lands its reverts
//! there, to prove the net on a throwaway branch; `MERGE_NET_STATE` then
//! names a state folder of its own, so its hold stops no switch.
//!
//! Usage: `merge-net watch [--from COMMIT] [--branch NAME] [--once] | merge-net needs [--base REF]
//! | merge-net status`

mod holds;
mod rules;
mod state;
mod watch;
mod world;

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

use state::{State, short};
use watch::Round;
use world::Real;

const USED: &str = "Usage: merge-net watch [--from COMMIT] [--branch NAME] [--once] | \
                    merge-net needs [--base REF] | merge-net status";

/// What a run asked for is refused with, as sysexits.h numbers it.
const USAGE: u8 = 64;

/// Where the net watches unless `MERGE_NET_REPOSITORY` names another.
const REPOSITORY: &str = "https://github.com/Jitsusama/core.nix";

/// The system whose checks the net runs.
const SYSTEM: &str = "x86_64-linux";

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let words: Vec<&str> = arguments.iter().map(String::as_str).collect();
    let done = match words.as_slice() {
        ["watch", rest @ ..] => watching(rest),
        ["needs"] => needs("refs/remotes/origin/main"),
        ["needs", "--base", base] => needs(base),
        ["status"] => status(),
        ["--help"] => {
            println!("{USED}");
            Ok(ExitCode::SUCCESS)
        }
        _ => {
            eprintln!("{USED}");
            return ExitCode::from(USAGE);
        }
    };
    done.unwrap_or_else(|why| {
        eprintln!("merge-net: {why}");
        ExitCode::FAILURE
    })
}

/// Follow main, a round at a time, until stopped, or once with `--once`.
fn watching(rest: &[&str]) -> Result<ExitCode, String> {
    let (mut from, mut once, mut branch) = (None, false, "main".to_owned());
    let mut words = rest.iter();
    while let Some(word) = words.next() {
        match *word {
            "--from" => from = Some((*words.next().ok_or("--from needs a commit")?).to_owned()),
            "--branch" => branch = (*words.next().ok_or("--branch needs a name")?).to_owned(),
            "--once" => once = true,
            other => return Err(format!("{other} isn't something watch takes\n{USED}")),
        }
    }
    let state = State::from_env();
    state.make()?;
    let world = Real {
        state: state.root.clone(),
        url: env::var("MERGE_NET_REPOSITORY").unwrap_or_else(|_| REPOSITORY.to_owned()),
        system: SYSTEM.to_owned(),
        booted: holds::booted().unwrap_or(0),
        branch,
    };
    // With nothing passed yet, the first round starts where it's told, or
    // at main's head as it is now, which it takes as passing.
    let start = match (state.heads().0, from) {
        (Some(passed), _) => passed,
        (None, Some(from)) => from,
        (None, None) => watch::World::head(&world)?,
    };
    state.say(&format!(
        "watching {}'s {} from {}",
        world.url,
        world.branch,
        short(&start)
    ));
    loop {
        let round = watch::round(&world, &state, &start);
        let rest = match &round {
            Ok(Round::Passed(_) | Round::Failed(_)) => 0,
            Ok(Round::Idle | Round::Unfinished(_)) => 60,
            Err(why) => {
                state.say(why);
                120
            }
        };
        if once {
            return Ok(if matches!(round, Ok(Round::Passed(_) | Round::Idle)) {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            });
        }
        std::thread::sleep(std::time::Duration::from_secs(rest));
    }
}

/// Say whether the branch's change runs VM tests before merging, and which.
fn needs(base: &str) -> Result<ExitCode, String> {
    let merge_base = git(&["merge-base", "HEAD", base])?;
    let mut files: Vec<String> = git(&["diff", "--name-only", &merge_base])?
        .lines()
        .chain(git(&["ls-files", "--others", "--exclude-standard"])?.lines())
        .map(str::to_owned)
        .collect();
    files.sort();
    files.dedup();
    let known = known_vm_tests();
    let found = rules::needs(&rules::areas(), &files, known.as_deref());
    if found.touched.is_empty() {
        println!("no: nothing here needs its VM tests before merging; the net runs them after");
        return Ok(ExitCode::SUCCESS);
    }
    println!("yes: run these VM tests before merging, since the change touches");
    for (area, hits) in &found.touched {
        println!("  {area}: {}", hits.join(" "));
    }
    for test in &found.tests {
        println!("{test}");
    }
    if known.is_none() {
        println!("(the names are patterns: no list of the checks was found to match them against)");
    }
    Ok(ExitCode::SUCCESS)
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

/// Say where the net is: the head it last passed, what it looked at last,
/// any hold, and how many findings it has written.
fn status() -> Result<ExitCode, String> {
    let state = State::from_env();
    let (passed, seen) = state.heads();
    let name = |head: Option<String>| {
        head.map_or_else(|| "none".to_owned(), |head| short(&head).to_owned())
    };
    println!("passed  {}", name(passed));
    println!("seen    {}", name(seen));
    match state.hold() {
        Some(hold) => println!("hold    {hold}"),
        None => println!("hold    none"),
    }
    let findings = fs::read_dir(state.root.join("findings")).map_or(0, Iterator::count);
    println!(
        "findings {findings}, in {}",
        state.root.join("findings").display()
    );
    if state.paused() {
        println!("paused");
    }
    Ok(ExitCode::SUCCESS)
}

fn git(arguments: &[&str]) -> Result<String, String> {
    let ran = Command::new("git")
        .args(arguments)
        .output()
        .map_err(|error| format!("running git: {error}"))?;
    if !ran.status.success() {
        return Err(format!(
            "git {} failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&ran.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&ran.stdout).trim().to_owned())
}
