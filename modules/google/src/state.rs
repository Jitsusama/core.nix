//! What the shell is told: how the account stands and what it holds, as
//! one JSON object to a line.

use serde::Serialize;

use crate::calendar::Meeting;
use crate::drive::Document;
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
             \"documents\":[],\"heard\":null,\"fetched\":null}\n"
        );
    }
}
