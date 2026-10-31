//! What an asker hears: one JSON line per step, then one ending. Every
//! ending names the commit the machine runs after it, so whoever keeps the
//! last answer knows it even after a rollback.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "step", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Line {
    /// Reading main from the repository.
    Fetching,
    /// Building the machine's system from this commit.
    Building { commit: String },
    /// Switching the machine to the system built from this commit.
    Switching { commit: String },
    /// The machine runs this commit, or boots it next.
    Done {
        commit: String,
        applied: Applied,
        /// Why the machine needs a restart before the commit is all
        /// running, when it does.
        restart: Option<String>,
        /// What the switch restarted on its own, such as a Mac's Linux VM,
        /// so an asker knows what of theirs ended.
        restarted: Vec<String>,
    },
    /// Main is the commit the machine last switched to; nothing was built.
    Unchanged { commit: String },
    /// It went wrong, and the machine is back on the commit it ran.
    Failed {
        commit: Option<String>,
        why: String,
        lines: Vec<String>,
    },
    /// It wasn't started; nothing was built or switched.
    Refused { commit: Option<String>, why: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Applied {
    /// The running system is the new one.
    Now,
    /// The new system is the one the machine starts; nothing of it runs
    /// until it does.
    NextBoot,
}

impl Line {
    /// Whether this line ends an answer.
    pub fn ends(&self) -> bool {
        !matches!(
            self,
            Line::Fetching | Line::Building { .. } | Line::Switching { .. }
        )
    }

    /// The line as it goes on the socket, without its newline.
    pub fn json(&self) -> String {
        serde_json::to_string(self).expect("a line always serializes")
    }
}
