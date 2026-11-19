//! How far the safe word's Slack reader has read Slack, kept in
//! judged.json beside its log: which messages it has judged, and since
//! when, so a restart neither judges a message twice nor misses one said
//! while it was down, as far back as a search's twenty newest reach; and
//! when Slack was last read whole, which `pineapple-slack heard` says
//! (src/bin/pineapple-slack.rs). It goes by the messages rather than by
//! the newest one's time, since Slack's search can find a message a while
//! after a newer one. GitHub's readers keep theirs apart
//! (src/progress.rs): they count from seconds, where Slack names a
//! message by its own timestamp.

use std::path::Path;

use serde_json::{Value, json};

use crate::slack::{Message, micros};

/// The file it's kept in, beside the reader's log.
pub const FILE: &str = "judged.json";

/// How many judged messages are kept: far more than two searches' twenty
/// newest, so one of those is never forgotten while it's still found.
pub const KEPT: usize = 1000;

/// Which messages it has judged: those since `since`, by their keys,
/// newest last.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Judged {
    /// From when messages are judged, as a Slack timestamp.
    pub since: String,
    /// The keys of those judged, newest last.
    pub keys: Vec<String>,
}

impl Judged {
    /// Nothing judged yet, from `seconds` since the epoch.
    #[must_use]
    pub fn starting(seconds: u64) -> Judged {
        Judged {
            since: format!("{seconds}.000000"),
            keys: Vec::new(),
        }
    }

    /// Whether a message is yet to be judged.
    #[must_use]
    pub fn is_new(&self, message: &Message) -> bool {
        micros(&message.ts) > micros(&self.since) && !self.keys.contains(&message.key())
    }

    /// That `message` is judged, forgetting the oldest past [`KEPT`].
    pub fn judged(&mut self, message: &Message) {
        self.keys.push(message.key());
        let over = self.keys.len().saturating_sub(KEPT);
        self.keys.drain(..over);
    }
}

/// Why a reader starts from `fresh` rather than from what was kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Afresh {
    /// Nothing was kept: a first start.
    Nothing,
    /// What was kept couldn't be read, so none of it is used.
    Unreadable,
    /// What was kept starts later than `fresh`, or at no time at all, so
    /// it would miss what was said since; the keys judged are kept. A
    /// clock set back since it was kept leaves one.
    Ahead,
}

/// How far Slack has been read, as kept in `state`, and why it starts
/// from `fresh`, in seconds, when it does.
#[must_use]
pub fn read(state: &Path, fresh: u64) -> (Judged, Option<Afresh>) {
    let afresh = |why| (Judged::starting(fresh), Some(why));
    let Ok(text) = std::fs::read_to_string(state.join(FILE)) else {
        return afresh(Afresh::Nothing);
    };
    let Some((since, keys)) = serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|kept| Some((kept.get("since")?.as_str()?.to_owned(), kept)))
    else {
        return afresh(Afresh::Unreadable);
    };
    let keys = keys
        .get("keys")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let starting = Judged::starting(fresh);
    match micros(&since) {
        Some(at) if Some(at) <= micros(&starting.since) => (Judged { since, keys }, None),
        _ => (
            Judged {
                since: starting.since,
                keys,
            },
            Some(Afresh::Ahead),
        ),
    }
}

/// When Slack was last read whole, in milliseconds, so a quiet reader is
/// told from one that isn't reading at all; none when it hasn't been.
#[must_use]
pub fn read_at(state: &Path) -> Option<u64> {
    kept(state)?.get("read_at").and_then(Value::as_u64)
}

/// Keeps `judged` whole in `state`, with `at`, in milliseconds, as when
/// Slack was read, through a file renamed into place, so no reader sees
/// half. Written every round, a few hundred bytes each 30 s.
///
/// # Errors
///
/// Why it couldn't be kept.
pub fn write(state: &Path, judged: &Judged, at: u64) -> std::io::Result<()> {
    let kept = json!({ "since": judged.since, "keys": judged.keys, "read_at": at });
    let path = state.join(FILE);
    let written = path.with_extension("new");
    std::fs::create_dir_all(state)?;
    std::fs::write(&written, kept.to_string())?;
    std::fs::rename(&written, &path)
}

fn kept(state: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(state.join(FILE)).ok()?;
    serde_json::from_str(&text).ok()
}
