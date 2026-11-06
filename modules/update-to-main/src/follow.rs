//! The asker that follows main: an account's service that asks for an
//! update as soon as main moves, so a merge doesn't wait for someone to
//! ask. It reads where each repository's branch is with `git ls-remote`,
//! which costs the host one small answer, and asks when one moved since its
//! last update ended. A hosted main's folder can't be read that cheaply, so
//! it also asks every so often, which its `changed` command answers
//! without a fetch while the folder is where it was.
//!
//! Everything that may stop a switch stays the daemon's to judge: a call,
//! a hold, a low battery, a daemon whose restart needs a hold, and asks
//! that come while an update runs join it. The asker only chooses when to
//! ask: again soon after a refusal for being busy, not again at heads that
//! failed until they move or a while has passed, and not at all after a
//! refusal for a held daemon until the machine's last ending changes,
//! since only a person's ask, with a hold sent, gets that one through.

use crate::config::Config;
use crate::follows::Followed;
use crate::line::Line;
use crate::update::NEEDS_HOLD;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// How often the branches are read.
pub const LOOK_EVERY: Duration = Duration::from_secs(30);

/// How soon an ask refused for being busy, such as during a call, is made
/// again.
const BUSY_AGAIN: Duration = Duration::from_secs(60);

/// How long heads whose update failed wait before they're asked for
/// again, in case what failed was the builders and not the commit.
const FAILED_AGAIN: Duration = Duration::from_secs(30 * 60);

/// The key main's own head is kept under among the inputs', which no
/// flake input can be named.
const MAIN: &str = "";

/// What the asker knows from its last ask.
#[derive(Debug, Default)]
pub struct Follower {
    /// When it last asked.
    asked: Option<Instant>,
    /// The heads its last ask was made at.
    last: Option<Followed>,
    /// The heads whose update failed, and when.
    failed: Option<(Followed, Instant)>,
    /// Whether the last ask was refused for being busy.
    busy: bool,
    /// The machine's last ending when an ask was refused for a held
    /// daemon, so it waits until that changes.
    held: Option<String>,
}

impl Follower {
    /// Whether to ask now, at these heads, with the machine's last ending
    /// as its file holds it, given how often a hosted main is asked for.
    pub fn should_ask(
        &mut self,
        heads: &Followed,
        ended: &str,
        now: Instant,
        every: Duration,
    ) -> bool {
        if let Some(when_held) = &self.held {
            if when_held == ended {
                return false;
            }
            self.held = None;
        }
        let Some(asked) = self.asked else {
            return true;
        };
        let since = now.duration_since(asked);
        if let Some((failed, when)) = &self.failed
            && failed == heads
        {
            return now.duration_since(*when) >= FAILED_AGAIN;
        }
        self.last.as_ref() != Some(heads) || since >= every || (self.busy && since >= BUSY_AGAIN)
    }

    /// Keeps how an ask made at `heads` ended, with the machine's last
    /// ending as its file holds it afterwards. Says why it now waits for a
    /// person, when it does.
    pub fn asked(
        &mut self,
        heads: Followed,
        ending: &Line,
        ended: &str,
        now: Instant,
    ) -> Option<String> {
        self.asked = Some(now);
        self.busy = false;
        self.last = Some(heads.clone());
        match ending {
            Line::Done { .. } | Line::Unchanged { .. } => {
                self.failed = None;
                None
            }
            Line::Failed { .. } => {
                self.failed = Some((heads, now));
                None
            }
            Line::Refused { why, .. } if why.starts_with(NEEDS_HOLD) => {
                self.held = Some(ended.to_owned());
                Some(why.clone())
            }
            _ => {
                self.busy = true;
                None
            }
        }
    }
}

/// Where main and each followed input's branch are, as their hosts say.
/// A hosted main isn't read, since its host wants a token and its folder
/// moves far less often than its head.
pub fn heads(config: &Config) -> Result<Followed, String> {
    let mut heads = Followed::new();
    if config.hosted.is_none() {
        heads.insert(
            MAIN.to_owned(),
            remote_head(config, &config.repository, &config.branch)?,
        );
    }
    for (name, follow) in &config.follows {
        heads.insert(
            name.clone(),
            remote_head(config, &follow.repository, &follow.branch)?,
        );
    }
    Ok(heads)
}

fn remote_head(config: &Config, repository: &str, branch: &str) -> Result<String, String> {
    let output = Command::new(&config.tools.git)
        .args([
            "ls-remote",
            "--exit-code",
            repository,
            &format!("refs/heads/{branch}"),
        ])
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| format!("{}: {e}", config.tools.git.display()))?;
    let said = String::from_utf8_lossy(&output.stdout);
    match said.split_whitespace().next() {
        Some(commit) if output.status.success() => Ok(commit.to_owned()),
        _ => Err(format!("{repository} names no {branch}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EVERY: Duration = Duration::from_secs(600);

    fn at(commit: &str) -> Followed {
        Followed::from([("core".to_owned(), commit.to_owned())])
    }

    fn done() -> Line {
        Line::Unchanged {
            commit: "c".to_owned(),
            followed: Followed::new(),
        }
    }

    fn refused(why: &str) -> Line {
        Line::Refused {
            commit: None,
            why: why.to_owned(),
        }
    }

    #[test]
    fn it_asks_once_when_it_starts_and_again_only_when_a_head_moves() {
        let mut f = Follower::default();
        let t = Instant::now();
        assert!(f.should_ask(&at("a"), "", t, EVERY));
        f.asked(at("a"), &done(), "", t);
        assert!(!f.should_ask(&at("a"), "", t + LOOK_EVERY, EVERY));
        assert!(f.should_ask(&at("b"), "", t + LOOK_EVERY, EVERY));
    }

    #[test]
    fn a_main_it_cant_read_is_asked_for_every_so_often() {
        let mut f = Follower::default();
        let t = Instant::now();
        f.asked(at("a"), &done(), "", t);
        assert!(!f.should_ask(&at("a"), "", t + EVERY / 2, EVERY));
        assert!(f.should_ask(&at("a"), "", t + EVERY, EVERY));
    }

    #[test]
    fn a_busy_machine_is_asked_again_soon() {
        let mut f = Follower::default();
        let t = Instant::now();
        assert_eq!(f.asked(at("a"), &refused("a call is on"), "", t), None);
        assert!(!f.should_ask(&at("a"), "", t + LOOK_EVERY, EVERY));
        assert!(f.should_ask(&at("a"), "", t + BUSY_AGAIN, EVERY));
    }

    #[test]
    fn heads_that_failed_wait_until_they_move_or_a_while_passes() {
        let mut f = Follower::default();
        let t = Instant::now();
        let failed = Line::Failed {
            commit: None,
            why: "it didn't evaluate".to_owned(),
            lines: Vec::new(),
        };
        f.asked(at("a"), &failed, "", t);
        assert!(!f.should_ask(&at("a"), "", t + EVERY, EVERY));
        assert!(f.should_ask(&at("a"), "", t + FAILED_AGAIN, EVERY));
        assert!(f.should_ask(&at("b"), "", t + LOOK_EVERY, EVERY));
    }

    #[test]
    fn a_held_daemon_stops_it_until_the_machines_last_ending_changes() {
        let mut f = Follower::default();
        let t = Instant::now();
        let why = format!("{NEEDS_HOLD} nix-daemon.service, which waits for a hold");
        assert_eq!(f.asked(at("a"), &refused(&why), "refused", t), Some(why));
        assert!(!f.should_ask(&at("b"), "refused", t + FAILED_AGAIN, EVERY));
        // A person's ask, with a hold sent, ended some other way.
        assert!(f.should_ask(&at("b"), "done", t + FAILED_AGAIN, EVERY));
    }
}
