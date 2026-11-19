//! Whether the safe word's Slack reader can hear Slack, said to Joel where
//! Slack isn't needed. Once [`ROUNDS`] rounds in a row read nothing for a
//! reason he can fix (Slack refused the sign-in, or none can be read), it
//! writes a file of what waits on him, which For You shows as a row and
//! says as a notice (the shell's WaitingOnYou.qml); a round that reads
//! Slack again takes it away. Its phone alert goes through Slack, so that
//! can't say this: on 17 Nov Slack refused the reader for 25 minutes, and
//! only its journal said so.
//!
//! A round Slack couldn't be asked in, its network down or its rate
//! spent, counts neither way: nothing there is Joel's to do, and the
//! journal says it.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// How many rounds in a row read nothing before Joel is told: one is a
/// blip a sign-in renewed mid-round makes, two at the reader's 30 seconds
/// is a minute of stops unheard.
pub const ROUNDS: u32 = 2;

/// Whether Slack's error word refuses the sign-in itself: the slack
/// crate's own list, so this reader counts the words slack-unreads does.
pub use slack_people::refuses_the_sign_in;

/// Why a round read nothing that Joel can fix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Deafness {
    /// Slack refused the sign-in.
    Refused,
    /// No sign-in could be read: signed out, the keyring locked, or one
    /// that doesn't say who Joel is.
    NoSignIn,
}

/// What a round of the reader came to, as far as hearing goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Round {
    /// Both searches answered.
    Heard,
    /// Nothing was read, for a reason Joel can fix.
    Deaf(Deafness),
    /// Slack couldn't be asked: counted neither way.
    Unasked,
}

/// What changes in the file of what waits on Joel.
#[derive(Debug, PartialEq)]
pub enum Change {
    /// Write it, whole.
    Tell(Value),
    /// Take it away.
    Untell,
    /// Leave it.
    Keep,
}

/// The rounds read nothing in a row, and whether Joel was told, and why.
#[derive(Debug)]
pub struct Deaf {
    rounds: u32,
    told: Option<Deafness>,
    /// When this time of not hearing was first told, in milliseconds,
    /// which names its row, so each time is a row For You hasn't seen.
    since: Option<u64>,
    /// Whether a file this program didn't write may still stand, as one
    /// left by the reader before a restart may.
    left: bool,
}

impl Default for Deaf {
    fn default() -> Self {
        Deaf::new()
    }
}

impl Deaf {
    /// Not deaf yet, and what an earlier reader told still to be taken
    /// away once Slack is heard.
    #[must_use]
    pub fn new() -> Deaf {
        Deaf {
            rounds: 0,
            told: None,
            since: None,
            left: true,
        }
    }

    /// That `change` couldn't be made, so the next round makes it again:
    /// a row not written is told again, one not taken away is taken
    /// away once Slack is heard.
    pub fn failed(&mut self, change: &Change) {
        match change {
            // What an earlier tell wrote may still stand, so it's taken
            // away once Slack is heard even if no tell lands before.
            Change::Tell(_) => {
                self.told = None;
                self.left = true;
            }
            Change::Untell => self.left = true,
            Change::Keep => {}
        }
    }

    /// What `round`, at `at` in milliseconds, changes. Joel is told once
    /// [`ROUNDS`] in a row read nothing, again only when why changes, and
    /// untold once a round reads Slack.
    pub fn heard(&mut self, round: Round, at: u64) -> Change {
        match round {
            Round::Heard => {
                self.rounds = 0;
                self.since = None;
                let left = std::mem::take(&mut self.left);
                if self.told.take().is_some() || left {
                    Change::Untell
                } else {
                    Change::Keep
                }
            }
            Round::Deaf(why) => {
                self.rounds = self.rounds.saturating_add(1);
                if self.rounds < ROUNDS || self.told == Some(why) {
                    return Change::Keep;
                }
                self.told = Some(why);
                self.left = false;
                let since = *self.since.get_or_insert(at);
                Change::Tell(waits(why, since, at))
            }
            Round::Unasked => Change::Keep,
        }
    }
}

/// What a deaf reader asks of Joel, in the shape every quest's file of
/// what waits on him takes (`mesh_wait`'s), as slack-unreads' sign-in is:
/// one item he can do now, known done by the signal `signin:slack` given
/// after `since`, when this time of not hearing was first told. The row is
/// named for `since`: For You says an item it has seen once, and keeps
/// one Joel marked done gone, by name, so a later time needs its own.
#[must_use]
pub fn waits(why: Deafness, since: u64, at: u64) -> Value {
    let (said, first) = match why {
        Deafness::Refused => ("Slack refused the safe word's sign-in", None),
        Deafness::NoSignIn => (
            "The safe word has no Slack sign-in it can read, signed out or with the keyring \
             locked",
            Some("Unlock your keyring if it asks"),
        ),
    };
    let steps: Vec<&str> = first
        .into_iter()
        .chain([
            "Press Sign In on Slack in Programs",
            "Sign in to Slack in the browser it opens",
            "Or type /slack-auth in a pi session",
        ])
        .collect();
    json!({
        "v": 2,
        "updated": slack_people::messages::iso(at),
        "quest": "pineapple",
        "items": [{
            "id": format!("pineapple-slack-deaf-{since}"),
            "title": "The Safe Word Can't Hear Slack",
            "why": format!(
                "{said}, so a stop said in Slack isn't heard; one said elsewhere still is"
            ),
            "raisedAt": slack_people::messages::iso(since),
            "minutes": 1,
            "ready": true,
            "doneWhen": {"signal": "signin:slack"},
            "steps": steps,
        }]
    })
}

/// Where For You's row is written, and a name passed over for it.
#[derive(Debug, PartialEq, Eq)]
pub struct Waiting {
    /// The file the row goes in.
    pub file: PathBuf,
    /// A name given that isn't a whole path, and so names nothing.
    pub refused: Option<PathBuf>,
}

/// Where For You's row goes: `named`, as the unit names the file its stop
/// removes (home.nix), when that's a whole path; else For You's own under
/// `state_home`. A name that isn't whole is kept, for the journal to say.
#[must_use]
pub fn waiting(named: Option<&OsStr>, state_home: &Path) -> Waiting {
    match named.map(PathBuf::from) {
        Some(named) if named.is_absolute() => Waiting {
            file: named,
            refused: None,
        },
        refused => Waiting {
            file: state_home.join("mission/waiting/pineapple.json"),
            refused,
        },
    }
}

/// Makes `change` to `file`: written in a `.writing` folder beside it
/// and renamed over it, so no reader sees half, or taken away.
///
/// # Errors
///
/// Why it couldn't be written or taken away.
pub fn make(file: &Path, change: &Change) -> std::io::Result<()> {
    match change {
        Change::Tell(waits) => {
            let aside = file
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(".writing");
            std::fs::create_dir_all(&aside)?;
            let writing = aside.join(file.file_name().unwrap_or_default());
            std::fs::write(&writing, waits.to_string())?;
            std::fs::rename(&writing, file)
        }
        Change::Untell => match std::fs::remove_file(file) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            done => done,
        },
        Change::Keep => Ok(()),
    }
}
