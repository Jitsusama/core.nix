//! What the shell is told: how the account stands and what it holds, as
//! one JSON object to a line.

use serde::Serialize;

use crate::calendar::Meeting;
use crate::drive::{Document, Drive};
use crate::files::Fetched;
use crate::mail::Mail;

/// How the account stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Standing {
    /// Nobody signed in to Google on this machine: the shell shows nothing.
    #[default]
    None,
    /// Google refused the account's sign-in; signing in again from pi
    /// mends it.
    SignedOut,
    /// Google couldn't be reached last time; what's held is from `heard`.
    Unreachable,
    /// What's held is what Google said at `heard`.
    Ready,
}

/// Nothing known yet, as at the start, is the same as no account.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct State {
    pub account: Standing,
    pub meetings: Vec<Meeting>,
    /// The mail that wants Joel, newest first.
    pub mail: Vec<Mail>,
    /// How many in the inbox are unread, or none until Gmail says.
    pub unread: Option<u64>,
    /// The documents Joel looked at lately, each kind's last first.
    pub documents: Vec<Document>,
    /// When Google last answered, in milliseconds since the epoch.
    pub heard: Option<i64>,
    /// The file mail carries that was last asked for, as it stands.
    pub fetched: Option<Fetched>,
    /// Making a Meet meeting, as it stands.
    pub meet: Meet,
    /// The Drive folders listed and the files downloaded lately.
    pub drive: Drive,
}

/// Making a Meet meeting to copy its link: whether the sign-in lets this
/// program, and the one last asked for.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Meet {
    /// Whether Joel's sign-in grants Meet's scope, which one from before
    /// it was asked for doesn't, until he signs in again from pi.
    pub granted: bool,
    pub made: Option<Made>,
}

/// The meeting last asked to be made, which the shell waits on: being
/// made, then its link or that it couldn't be.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Made {
    /// When it was asked, in milliseconds since the epoch, which tells
    /// one ask from the next.
    pub asked: i64,
    /// The link that joins it, once it's made.
    pub link: Option<String>,
    /// Whether Google wouldn't make it, or couldn't be reached.
    pub failed: bool,
}

impl State {
    pub fn line(&self) -> String {
        let mut line = serde_json::to_string(self).unwrap_or_else(|_| "{}".into());
        line.push('\n');
        line
    }

    /// The account gone or signed out: nothing it held is kept, since it
    /// may not be Joel's to see any more.
    pub fn without(standing: Standing) -> State {
        State {
            account: standing,
            ..State::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_state_is_one_line_of_json() {
        let line = State::without(Standing::SignedOut).line();
        assert_eq!(
            line,
            "{\"account\":\"signedOut\",\"meetings\":[],\"mail\":[],\"unread\":null,\
             \"documents\":[],\"heard\":null,\"fetched\":null,\
             \"meet\":{\"granted\":false,\"made\":null},\
             \"drive\":{\"listings\":[],\"downloads\":[]}}\n"
        );
    }
}
