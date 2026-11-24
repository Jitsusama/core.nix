//! A trial rebase, started by `needs` once a change's gate has passed at a
//! head main has since moved files of: the head merged onto main as one
//! commit, run through changed-checks on the lent builders while its
//! author is still reviewing. Its pass is kept as any gate's is, under the
//! trial commit, which carries the head's newest message and makes the
//! same change from main, so `ready` finds it as it finds any earlier
//! commit of the change (`earlier::judged`) and needs no run of its own
//! when main hasn't moved those files again.
//!
//! On 24 November the train carried changes that had passed their gates
//! but shared a file with what main moved; each waited on a whole run of
//! changed-checks after `ready`, which a run started at gate green has
//! already finished by then.
//!
//! Nothing is pushed and no branch moves: the trial commit lives only in
//! the asker's object store and a throwaway clone beside the net's state,
//! taken away when the run ends, its outcome kept as `<head>.json`. At
//! most [`AT_ONCE`] go at a time, and a head already tried on the same
//! main isn't tried again.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use serde_json::json;

use crate::state::short;

/// How many trials go at once, so a burst of gates can't fill the lenders
/// with runs no one has asked to merge yet.
pub const AT_ONCE: usize = 3;

/// What a unit running a trial is described as, before its head.
const DESCRIBED: &str = "merge-net tries a rebase of ";

/// The words a trial ends its log with, as it ended.
const PASSED: &str = "merge-net: the trial rebase passed changed-checks at ";
const FAILED: &str = "merge-net: the trial rebase ended without passing at ";

/// What `needs` does for a head that passed but shares files with main.
#[derive(Debug, PartialEq, Eq)]
pub enum Wanted {
    /// Start its trial.
    Start,
    /// Its trial goes already.
    Runs,
    /// [`AT_ONCE`] trials go already, so this one waits for `ready`.
    Full,
    /// It was tried on this main already, whatever came of it.
    Tried,
}

/// Whether to start the trial of `head` on `main`, with `going` the heads
/// whose trials run now and `tried` the main its last trial was on.
#[must_use]
pub fn wanted(head: &str, main: &str, going: &[String], tried: Option<&str>) -> Wanted {
    if going.iter().any(|running| running == head) {
        Wanted::Runs
    } else if tried == Some(main) {
        Wanted::Tried
    } else if going.len() >= AT_ONCE {
        Wanted::Full
    } else {
        Wanted::Start
    }
}

/// The user unit the trial of `head` runs as.
#[must_use]
pub fn unit(head: &str) -> String {
    format!("merge-net-trial-{}.service", short(head))
}

/// The head a unit described as `description` tries.
#[must_use]
pub fn trying(description: &str) -> Option<&str> {
    description.strip_prefix(DESCRIBED).map(str::trim)
}

/// Where the trials keep their clones and outcomes, under the net's state.
#[must_use]
pub fn folder(root: &Path) -> PathBuf {
    root.join("train").join("trials")
}

/// Git run with its words and what goes to its input, giving what it
/// printed or why it failed.
pub type Git<'a> = dyn Fn(&[&str], Option<&str>) -> Result<String, String> + 'a;

/// The commit making `head`'s change on `main`, as merging it would make
/// it, carrying `head`'s newest message so `ready` finds its pass among
/// the change's, with `git` running git and giving what it printed (its
/// second word what goes to its input).
///
/// # Errors
///
/// When `head` conflicts with `main`, or git fails.
pub fn rebased(git: &Git, main: &str, head: &str) -> Result<String, String> {
    let tree = git(&["merge-tree", "--write-tree", main, head], None)
        .map_err(|_| format!("{} conflicts with main at {}", short(head), short(main)))?;
    let tree = tree.lines().next().unwrap_or_default().to_owned();
    let message = git(&["log", "-1", "--format=%B", head], None)?;
    git(
        &["commit-tree", &tree, "-p", main, "-F", "-"],
        Some(&message),
    )
}

/// What a trial's outcome file holds.
#[must_use]
pub fn outcome(head: &str, main: &str, trial: &str, code: Option<i32>, at: u64) -> String {
    json!({"head": head, "main": main, "trial": trial, "rc": code, "at": at}).to_string()
}

/// The main the last trial of `head` kept in `folder` ran on.
#[must_use]
pub fn tried_on(folder: &Path, head: &str) -> Option<String> {
    let kept = fs::read_to_string(folder.join(format!("{head}.json"))).ok()?;
    let read: serde_json::Value = serde_json::from_str(&kept).ok()?;
    read.get("main")?.as_str().map(str::to_owned)
}

/// Start the trial of `head` on `main` from the repository the asker is
/// in, unless [`wanted`] says otherwise, and say what was done.
#[must_use]
pub fn offer(head: &str, main: &str) -> Option<String> {
    let trials = folder(&crate::state::State::from_env().root);
    let going = going();
    let done = wanted(head, main, &going, tried_on(&trials, head).as_deref());
    let log = trials.join(format!("{head}.log"));
    match done {
        Wanted::Start if start(&trials, head, main, &log) => Some(format!(
            "merge-net started a trial rebase of {} onto main at {}, its words going to {}: \
             once it passes, ready reads it rather than running changed-checks again",
            short(head),
            short(main),
            log.display()
        )),
        Wanted::Runs => Some(format!(
            "merge-net's trial rebase of {} still runs, its words going to {}",
            short(head),
            log.display()
        )),
        Wanted::Full => Some(format!(
            "{AT_ONCE} trial rebases go already, so {} gets none; ready runs it as before",
            short(head)
        )),
        Wanted::Start | Wanted::Tried => None,
    }
}

/// The heads whose trials run now, read from their units.
fn going() -> Vec<String> {
    let listed = systemctl(&[
        "list-units",
        "--plain",
        "--no-legend",
        "--state=active,activating",
        "merge-net-trial-*",
    ]);
    listed
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter_map(|unit| {
            let shown = systemctl(&["show", unit, "--property=Description"]);
            let said = shown.trim().strip_prefix("Description=")?;
            trying(said).map(str::to_owned)
        })
        .collect()
}

/// Start the unit trying `head` on `main`, writing to `log`: whether
/// systemd took it.
fn start(trials: &Path, head: &str, main: &str, log: &Path) -> bool {
    let Ok(top) = crate::git(&["rev-parse", "--show-toplevel"]) else {
        return false;
    };
    if fs::create_dir_all(trials).is_err() {
        return false;
    }
    let cgroup = fs::read_to_string("/proc/self/cgroup").unwrap_or_default();
    let slice = crate::rerun::slice_of(&cgroup).unwrap_or("app.slice");
    let path = std::env::var("PATH").unwrap_or_default();
    let Ok(merge_net) = std::env::current_exe() else {
        return false;
    };
    Command::new("systemd-run")
        .args([
            "--user",
            "--quiet",
            "--collect",
            "--no-block",
            &format!("--unit={}", unit(head)),
            &format!("--description={DESCRIBED}{head}"),
            &format!("--slice={slice}"),
            &format!("--working-directory={top}"),
            &format!("--setenv=PATH={path}"),
            "--property=UnsetEnvironment=SSH_AUTH_SOCK",
            "--property=RuntimeMaxSec=7200",
            &format!("--property=StandardOutput=truncate:{}", log.display()),
            "--property=StandardError=inherit",
        ])
        .arg(&merge_net)
        .args(["trial", head, main])
        .status()
        .is_ok_and(|status| status.success())
}

/// Try `head` on `main`, as the unit `needs` started does, in the
/// repository it runs in: make the trial commit, run changed-checks at it
/// in a clone of its own, and keep how it ended.
///
/// # Errors
///
/// When the trial commit or its clone can't be made.
pub fn run(head: &str, main: &str) -> Result<ExitCode, String> {
    let trials = folder(&crate::state::State::from_env().root);
    let trial = rebased(&real_git, main, head)?;
    let common = crate::git(&["rev-parse", "--path-format=absolute", "--git-common-dir"])?;
    let clone = trials.join(head);
    let _ = fs::remove_dir_all(&clone);
    let in_clone = |arguments: &[&str]| {
        let mut asked = vec!["-C", clone.to_str().unwrap_or_default()];
        asked.extend_from_slice(arguments);
        real_git(&asked, None)
    };
    let into = clone.to_string_lossy();
    real_git(
        &[
            "clone",
            "-q",
            "--shared",
            "--no-checkout",
            "--no-tags",
            &common,
            &into,
        ],
        None,
    )?;
    in_clone(&["update-ref", "refs/remotes/origin/main", main])?;
    in_clone(&["checkout", "-q", "--detach", &trial])?;
    println!(
        "merge-net: trying {} on main at {} as {}",
        short(head),
        short(main),
        short(&trial)
    );
    let ended = Command::new("nix-offload")
        .args(["there", "--lent-only", "--changed"])
        .current_dir(&clone)
        .status()
        .map_err(|error| format!("running nix-offload: {error}"))?;
    let _ = fs::remove_dir_all(&clone);
    let kept = outcome(head, main, &trial, ended.code(), crate::state::now());
    fs::write(trials.join(format!("{head}.json")), kept)
        .map_err(|error| format!("keeping the trial's outcome: {error}"))?;
    let ending = if ended.success() { PASSED } else { FAILED };
    println!("{ending}{trial}");
    Ok(if ended.success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Run git with `arguments`, `input` on its input, and give what it
/// printed, trimmed.
fn real_git(arguments: &[&str], input: Option<&str>) -> Result<String, String> {
    let mut asked = Command::new("git");
    asked
        .args(arguments)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = asked
        .spawn()
        .map_err(|error| format!("running git: {error}"))?;
    if let (Some(words), Some(mut stdin)) = (input, child.stdin.take()) {
        use std::io::Write as _;
        stdin
            .write_all(words.as_bytes())
            .map_err(|error| format!("writing to git: {error}"))?;
    }
    let ran = child
        .wait_with_output()
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

/// What `systemctl --user` says, asked `arguments`, or nothing.
fn systemctl(arguments: &[&str]) -> String {
    Command::new("systemctl")
        .arg("--user")
        .args(arguments)
        .output()
        .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
