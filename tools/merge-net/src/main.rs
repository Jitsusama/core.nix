//! The net under main: merges land on their own checks, VM tests aside,
//! and the net runs every check each one changed after it lands.
//!
//! `merge-net watch` follows origin's main from its own checkout. For each
//! new head it asks changed-checks which checks moved since the last head
//! whose checks all passed, and runs each of them, the VM tests last,
//! through `nix-offload there`, which sends it to the builders, twelve at
//! once, as bulk where nix-offload takes it, so sessions' runs go first. A
//! test that fails twice at a commit fails for real: the net finds which
//! merge broke it, holds switching on it (`hold.json`), reverts it through
//! a PR merged on its own checks, and writes a finding for its owner. A
//! held test runs first at each new head, and its pass lifts the hold. One
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
//! `watch --follow PATH` names the installed merge-net: once that is
//! another build than the one running, the watch ends between rounds, for
//! its unit to start the new one, rather than a switch stopping it
//! mid-round and losing every run in flight.
//!
//! Usage: `merge-net watch [--from COMMIT] [--branch NAME] [--once] [--follow PATH]
//! | merge-net needs [--base REF] | merge-net status`

mod holds;
mod rules;
mod state;
mod watch;
mod world;

use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};

use state::{State, short};
use watch::Round;
use world::Real;

const USED: &str = "Usage: merge-net watch [--from COMMIT] [--branch NAME] [--once] \
                    [--follow PATH] | merge-net needs [--base REF] | merge-net status";

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
        ["status"] => Ok(status()),
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
    let mut follow: Option<PathBuf> = None;
    let mut words = rest.iter();
    while let Some(word) = words.next() {
        match *word {
            "--from" => from = Some((*words.next().ok_or("--from needs a commit")?).to_owned()),
            "--branch" => (*words.next().ok_or("--branch needs a name")?).clone_into(&mut branch),
            "--once" => once = true,
            "--follow" => {
                follow = Some(PathBuf::from(*words.next().ok_or("--follow needs a path")?));
            }
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
    // at main's head as it is now, which it takes as passing. It's written
    // down at once, so a first round cut short, by a hold or a restart,
    // starts from the same place when it runs again.
    let start = match (state.heads().0, from) {
        (Some(passed), _) => passed,
        (None, Some(from)) => from,
        (None, None) => watch::World::head(&world)?,
    };
    if state.heads().0.is_none() {
        state.set_heads(&start, &start)?;
    }
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
            Ok(Round::Idle) => 60,
            Ok(Round::Unfinished(head)) => {
                state.say(&format!(
                    "{} isn't done: a test wasn't sent, so it runs again",
                    short(head)
                ));
                60
            }
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
        if let Some(installed) = &follow
            && replaced(fs::canonicalize(installed), env::current_exe())
        {
            state.say("another merge-net is installed, so this one ends between rounds");
            return Ok(ExitCode::SUCCESS);
        }
        std::thread::sleep(std::time::Duration::from_secs(rest));
    }
}

/// Whether the merge-net installed is another build than the one running.
/// On 7 November three switches in 25 minutes each restarted the net with
/// a new build, and a round of about 20 minutes ran from 16:21 past 17:15.
/// A running build no longer on disk has been replaced; an installed one
/// that can't be read says nothing.
fn replaced(installed: io::Result<PathBuf>, running: io::Result<PathBuf>) -> bool {
    let Ok(installed) = installed else {
        return false;
    };
    !running
        .and_then(fs::canonicalize)
        .is_ok_and(|running| running == installed)
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
    let areas = rules::areas();
    let known = known_vm_tests();
    if rules::needs(&areas, &files, known.as_deref())
        .touched
        .is_empty()
    {
        println!("no: nothing here needs its VM tests before merging; the net runs them after");
        return Ok(ExitCode::SUCCESS);
    }
    // Only a test whose derivation moved from main can give a result main
    // hasn't, so changed-checks is asked which moved; it costs an
    // evaluation, which only a change touching an area pays.
    let said = moved_from(base);
    if said.is_none() {
        println!("(changed-checks couldn't say which VM tests moved, so every one named runs)");
    }
    let found = rules::first(&areas, &files, said.as_deref(), known.as_deref());
    let touched = found
        .touched
        .iter()
        .map(|(area, _)| area.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    if found.tests.is_empty() && found.there.is_empty() {
        println!(
            "no: the change touches {touched}, but none of the VM tests it runs moved from \
             {base}, so each would give main's result; the net runs the rest after"
        );
        return Ok(ExitCode::SUCCESS);
    }
    println!("yes: the change touches");
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
    // The list is read from the base, main, so a branch can't quarantine
    // its own failure.
    let list = git(&["show", &format!("{base}:{}", world::QUARANTINE)])
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    for test in &found.tests {
        println!("{}", rules::named(test, &list));
    }
    if known.is_none() {
        println!("(the names are patterns: no list of the checks was found to match them against)");
    }
    Ok(ExitCode::SUCCESS)
}

/// What `changed-checks --name-only` says moved between `base` and the
/// working copy, or nothing when it couldn't compare.
fn moved_from(base: &str) -> Option<String> {
    let ran = Command::new("changed-checks")
        .args(["--base", base, "--name-only", "--system", SYSTEM])
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

/// Say where the net is: the head it last passed, what it looked at last,
/// any hold, and how many findings it has written.
fn status() -> ExitCode {
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
    ExitCode::SUCCESS
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_watch_ends_once_another_build_is_installed() {
        let dir = env::temp_dir().join(format!("merge-net-follow-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let (old, new) = (dir.join("old"), dir.join("new"));
        fs::write(&old, "").unwrap();
        fs::write(&new, "").unwrap();
        let installed = || fs::canonicalize(&new);
        assert!(
            !replaced(installed(), Ok(new.clone())),
            "the build installed runs"
        );
        assert!(
            replaced(installed(), Ok(old.clone())),
            "an older build runs"
        );
        assert!(
            replaced(installed(), Ok(dir.join("collected"))),
            "the running build is gone from disk"
        );
        assert!(
            !replaced(fs::canonicalize(dir.join("unread")), Ok(old)),
            "nothing can be read of what's installed"
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
