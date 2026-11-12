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
//! every head spared 38 for 101. By 12 November merges came faster, and
//! a keep took 3 to 8 minutes, so the wait cost more than it saved: of 23
//! runs that paid for their base from 09:00 to noon, most came 50 to 335 s
//! after it landed, before a keep begun two minutes on could finish, and 4
//! were on heads that moved on within the two minutes. A head is now kept
//! by the second look that finds it, 30 s on, which still passes over a
//! burst's middle heads, since a keep holds the loop and the next look
//! after it reads only the newest. A head is kept only where nix-offload
//! sends it, never on this machine, whose own runs are sent too, and only
//! to a builder already lent, never one asked for: a store file left
//! naming a builder whose tunnel had gone asked for one at 00:11 on 6
//! November, before nix-offload took `--never-ask`. A kept
//! head is sent again when another builder is lent, since one lent afresh
//! starts with nothing kept, as one lent that evening did; never on a
//! timer, since a builder is given back after 15 idle minutes and sending
//! on one would keep it lent all day.

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

/// How often main's head is read.
pub const LOOK_EVERY: Duration = Duration::from_secs(30);

/// How long a head stands before it's kept.
pub const STOOD: Duration = LOOK_EVERY;

/// How long a head that couldn't be kept waits before it's sent again:
/// longer than a lent builder's 15 idle minutes, so a keep that fails
/// every time never holds one lent. A head that moves on is sent once it
/// stands, and one that couldn't be kept once another builder is lent.
pub const FAILED_AGAIN: Duration = Duration::from_mins(20);

/// What the keeper has seen of main, which says when a head is sent.
#[derive(Default)]
pub struct Keeper {
    /// The head last read, and when it was first read.
    seen: Option<(String, Instant)>,
    /// The head last kept, and the builder it was kept on.
    kept: Option<(String, String)>,
    /// The head last sent and not kept, the builder it was sent to, and when.
    failed: Option<(String, String, Instant)>,
}

impl Keeper {
    /// The head to send now, given main's head and the lent builder as read
    /// at `now`: one that has stood `STOOD`, with a builder lent that
    /// hasn't kept it, and that didn't just fail there.
    pub fn due(&mut self, head: &str, builder: Option<&str>, now: Instant) -> Option<String> {
        if self.seen.as_ref().is_none_or(|(seen, _)| seen != head) {
            self.seen = Some((head.to_owned(), now));
        }
        let builder = builder?;
        let (seen, since) = self.seen.as_ref()?;
        let stood = now.duration_since(*since) >= STOOD;
        let kept = self
            .kept
            .as_ref()
            .is_some_and(|(kept, on)| kept == head && on == builder);
        let failing = self.failed.as_ref().is_some_and(|(failed, on, at)| {
            failed == head && on == builder && now.duration_since(*at) < FAILED_AGAIN
        });
        (stood && !kept && !failing).then(|| seen.clone())
    }

    /// Note that `head` is kept on `builder`.
    pub fn kept(&mut self, head: &str, builder: &str) {
        self.kept = Some((head.to_owned(), builder.to_owned()));
        self.failed = None;
    }

    /// Note that `head` couldn't be kept on `builder` at `now`.
    pub fn failed(&mut self, head: &str, builder: &str, now: Instant) {
        self.failed = Some((head.to_owned(), builder.to_owned(), now));
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

    const ONE: Option<&str> = Some("one");

    #[test]
    fn a_new_head_waits_until_it_has_stood() {
        let mut keeper = Keeper::default();
        let start = Instant::now();
        assert_eq!(keeper.due("a", ONE, start), None);
        assert_eq!(keeper.due("a", ONE, start + STOOD / 2), None);
        assert_eq!(keeper.due("a", ONE, start + STOOD), Some("a".to_owned()));
    }

    #[test]
    fn a_head_is_kept_by_the_second_look_at_it() {
        let mut keeper = Keeper::default();
        let start = Instant::now();
        keeper.due("a", ONE, start);
        assert_eq!(
            keeper.due("a", ONE, start + LOOK_EVERY),
            Some("a".to_owned()),
            "a head still there a look later waited longer, though most runs that paid for \
             their base came within five minutes of it landing"
        );
    }

    #[test]
    fn a_head_that_moves_on_starts_its_wait_again() {
        let mut keeper = Keeper::default();
        let start = Instant::now();
        keeper.due("a", ONE, start);
        assert_eq!(
            keeper.due("b", ONE, start + STOOD),
            None,
            "b was only just seen"
        );
        assert_eq!(
            keeper.due("b", ONE, start + STOOD * 2),
            Some("b".to_owned())
        );
    }

    #[test]
    fn nothing_is_sent_with_no_builder_lent() {
        let mut keeper = Keeper::default();
        let start = Instant::now();
        keeper.due("a", None, start);
        assert_eq!(keeper.due("a", None, start + STOOD), None);
        assert_eq!(keeper.due("a", ONE, start + STOOD), Some("a".to_owned()));
    }

    #[test]
    fn a_kept_head_is_sent_again_only_to_another_builder() {
        let mut keeper = Keeper::default();
        let start = Instant::now();
        keeper.due("a", ONE, start);
        keeper.kept("a", "one");
        assert_eq!(keeper.due("a", ONE, start + STOOD * 30), None);
        assert_eq!(
            keeper.due("a", Some("two"), start + STOOD * 30),
            Some("a".to_owned())
        );
    }

    #[test]
    fn a_head_that_failed_waits_past_a_builders_idle_spell_unless_another_is_lent() {
        let mut keeper = Keeper::default();
        let start = Instant::now();
        keeper.due("a", ONE, start);
        let tried = start + STOOD;
        keeper.failed("a", "one", tried);
        assert!(FAILED_AGAIN > Duration::from_mins(15));
        assert_eq!(keeper.due("a", ONE, tried + LOOK_EVERY), None);
        assert_eq!(
            keeper.due("a", Some("two"), tried + LOOK_EVERY),
            Some("a".to_owned())
        );
        assert_eq!(
            keeper.due("a", ONE, tried + FAILED_AGAIN),
            Some("a".to_owned())
        );
    }

    #[test]
    fn a_head_that_moves_on_is_sent_once_it_stands_whatever_failed_before() {
        let mut keeper = Keeper::default();
        let start = Instant::now();
        keeper.due("a", ONE, start);
        keeper.failed("a", "one", start + STOOD);
        let moved = start + STOOD + LOOK_EVERY;
        keeper.due("b", ONE, moved);
        assert_eq!(keeper.due("b", ONE, moved + STOOD), Some("b".to_owned()));
    }
}
