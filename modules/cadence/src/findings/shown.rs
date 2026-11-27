//! What a line the hour holds to a fix shown has heard of its fix, and how
//! the hour closed it. A figure back in alone closes no such line, since a
//! number gone quiet isn't a fix shown: its owner names the fix, with
//! `cadence owe --done` or a reply naming the change, or says it needs
//! none, and the line closes at an hour that finds it back in. One back a
//! week with no word closes, counted apart, so the ledger can't grow for
//! good; a fall again reopens it under its own key.

use serde::{Deserialize, Serialize};

use crate::DAY;

/// How long a change a reply names may be merged but not running here
/// before the line counts as back with its fix unnamed: getting it live is
/// the switch's job, so it holds the line, never late, only this long.
pub const LIVE_WITHIN: i64 = 2 * DAY;

/// What a line's owner has said of its fix, and how the hour closed it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shown {
    /// The latest word of its fix, from its owner or whoever it was put to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub word: Option<Word>,
    /// How the hour closed it, and when, so the card counts the week's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended: Option<Ended>,
}

impl Shown {
    /// Whether nothing has been said or closed, so the ledger keeps nothing.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.word.is_none() && self.ended.is_none()
    }

    /// Whether the line waits only on a change its owner named going live
    /// here at `now`, in seconds: merged, not yet running, and named within
    /// `LIVE_WITHIN`, so it never goes late.
    #[must_use]
    pub fn awaits_switch(&self, now: i64) -> bool {
        self.word.as_ref().is_some_and(|word| {
            matches!(
                word.said,
                Said::Change {
                    live: Live::Merged,
                    ..
                }
            ) && now < word.at + LIVE_WITHIN
        })
    }
}

/// A word of a line's fix, and when it was said, in seconds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Word {
    pub said: Said,
    pub at: i64,
}

/// What a line's owner said of its fix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Said {
    /// A fix named with `cadence owe --done`, taken at its owner's word.
    Fix(String),
    /// A reply naming the change that fixed it, and how that change stands
    /// here, judged each hour.
    Change { text: String, live: Live },
    /// A reply saying it needs no fix, and why.
    Unneeded(String),
}

/// How the changes a reply names stand here.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Live {
    /// None has merged in a repository cadence reads, or its main went
    /// unread.
    #[default]
    No,
    /// One has merged, but none runs here yet.
    Merged,
    /// One runs here.
    Runs,
}

/// How the hour closed a line, and when, in seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ended {
    pub how: How,
    pub at: i64,
}

/// What a line was back in with when the hour closed it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum How {
    /// A fix named, by `--done` or a change that runs here.
    Fixed,
    /// Its owner's word that it needed none.
    Unneeded,
    /// No word in the week it was back.
    Unexplained,
}
