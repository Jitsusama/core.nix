//! `changed-checks keep-main`: keeps each head of origin's main where runs
//! are sent, once it has stood a while, so a run on a branch cut from it
//! evaluates its own tree alone.
//!
//! The first run against a new base paid for the base: on 5 November, 42
//! of the lent builder's 105 runs evaluated theirs before their own tree,
//! a median of 444 s each. Merges land in bursts, and a head that moves on
//! within two minutes is seldom anyone's base, so a head is kept once it
//! has stood that long: replaying that day's 101 heads, waiting two
//! minutes kept 74 and spared 29 of those 42 evaluations, where keeping
//! every head spared 38 for 101. A head is kept only where nix-offload
//! sends it, never on this machine, whose own runs are sent too.

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

/// How often main's head is read.
pub const LOOK_EVERY: Duration = Duration::from_secs(30);

/// How long a head stands before it's kept.
pub const STOOD: Duration = Duration::from_mins(2);

/// How long a head that couldn't be kept waits before it's tried again,
/// as when no builder is lent; a head that moves on is tried at once.
pub const FAILED_AGAIN: Duration = Duration::from_mins(10);

/// What the keeper has seen of main, which says when a head is kept.
#[derive(Default)]
pub struct Keeper {
    /// The head last read, and when it was first read.
    seen: Option<(String, Instant)>,
    /// The head last kept.
    kept: Option<String>,
    /// The head last tried and not kept, and when.
    failed: Option<(String, Instant)>,
}

impl Keeper {
    /// The head to keep now, given main's head as read at `now`: one that
    /// has stood `STOOD`, isn't kept yet, and didn't just fail.
    pub fn due(&mut self, head: &str, now: Instant) -> Option<String> {
        if self.seen.as_ref().is_none_or(|(seen, _)| seen != head) {
            self.seen = Some((head.to_owned(), now));
        }
        let (seen, since) = self.seen.as_ref()?;
        let stood = now.duration_since(*since) >= STOOD;
        let kept = self.kept.as_deref() == Some(head);
        let failing = self
            .failed
            .as_ref()
            .is_some_and(|(failed, at)| failed == head && now.duration_since(*at) < FAILED_AGAIN);
        (stood && !kept && !failing).then(|| seen.clone())
    }

    /// Note that `head` is kept.
    pub fn kept(&mut self, head: &str) {
        self.kept = Some(head.to_owned());
        self.failed = None;
    }

    /// Note that `head` couldn't be kept at `now`.
    pub fn failed(&mut self, head: &str, now: Instant) {
        self.failed = Some((head.to_owned(), now));
    }
}

/// Main's head at `url`, as the remote says it now.
pub fn head(url: &str) -> Result<String, String> {
    let said = git(None, &["ls-remote", "--exit-code", url, "refs/heads/main"])?;
    said.split_whitespace()
        .next()
        .map(str::to_owned)
        .ok_or_else(|| format!("{url} said nothing of its main"))
}

/// Put the keeper's own clone of `url`, in `clone`, at `head` with nothing
/// changed, and origin's main where the remote has it, as a run reads them.
pub fn check_out(url: &str, clone: &Path, head: &str) -> Result<(), String> {
    if !clone.join(".git").exists() {
        if let Some(parent) = clone.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("making {}: {error}", parent.display()))?;
        }
        let into = clone.to_string_lossy();
        git(None, &["clone", "-q", "--no-checkout", url, &into])?;
    }
    git(
        Some(clone),
        &[
            "fetch",
            "-q",
            "origin",
            "+refs/heads/main:refs/remotes/origin/main",
        ],
    )?;
    git(
        Some(clone),
        &["checkout", "-q", "--force", "--detach", head],
    )?;
    Ok(())
}

/// Run git, in `folder` when one is named, and give what it printed.
fn git(folder: Option<&Path>, arguments: &[&str]) -> Result<String, String> {
    let mut command = Command::new("git");
    if let Some(folder) = folder {
        command.current_dir(folder);
    }
    let ran = command
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
    fn a_new_head_waits_until_it_has_stood() {
        let mut keeper = Keeper::default();
        let start = Instant::now();
        assert_eq!(keeper.due("a", start), None);
        assert_eq!(keeper.due("a", start + STOOD / 2), None);
        assert_eq!(keeper.due("a", start + STOOD), Some("a".to_owned()));
    }

    #[test]
    fn a_head_that_moves_on_starts_its_wait_again() {
        let mut keeper = Keeper::default();
        let start = Instant::now();
        keeper.due("a", start);
        assert_eq!(keeper.due("b", start + STOOD), None, "b was only just seen");
        assert_eq!(keeper.due("b", start + STOOD * 2), Some("b".to_owned()));
    }

    #[test]
    fn a_kept_head_is_kept_once() {
        let mut keeper = Keeper::default();
        let start = Instant::now();
        keeper.due("a", start);
        keeper.kept("a");
        assert_eq!(keeper.due("a", start + STOOD * 3), None);
    }

    #[test]
    fn a_head_that_failed_is_tried_again_after_a_while_or_once_main_moves() {
        let mut keeper = Keeper::default();
        let start = Instant::now();
        keeper.due("a", start);
        let tried = start + STOOD;
        keeper.failed("a", tried);
        assert_eq!(keeper.due("a", tried + LOOK_EVERY), None);
        assert_eq!(keeper.due("a", tried + FAILED_AGAIN), Some("a".to_owned()));
        keeper.failed("a", tried + FAILED_AGAIN);
        let moved = tried + FAILED_AGAIN + LOOK_EVERY;
        keeper.due("b", moved);
        assert_eq!(keeper.due("b", moved + STOOD), Some("b".to_owned()));
    }
}
