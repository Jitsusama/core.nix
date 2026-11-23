//! Where the Slack reader hears Slack: Slack's own API with Joel's
//! sign-in, or a tool gateway that reads Slack for him.
//!
//! A round asks only two things of either, a search for the newest
//! messages and what the message starting a thread says, so the reader
//! judges the same whichever it hears through. What either couldn't be
//! asked is said as the reader counts it: a sign-in refused, none to give,
//! or a Slack that couldn't be asked this round.

use serde_json::{Value, json};
use slack_people::{Account, Slack};

use crate::deaf::{self, Deafness, Round};
use crate::gateway;
use crate::mcp::{Called, Gateway};
use crate::slack::{self, Message};

/// How many of the newest messages each search asks for.
pub const COUNT: u8 = 20;

/// What a reader hears Slack through.
pub trait Ears {
    /// The newest messages `query` finds, oldest first; else whether the
    /// sign-in was refused, missing, or Slack couldn't be asked now. An
    /// answer cut short is read only when it reaches back to `back_to`, a
    /// Slack timestamp, so nothing it could find since then goes unread.
    ///
    /// # Errors
    ///
    /// What the round counts as: see [`Round`].
    fn search(&mut self, query: &str, back_to: &str) -> Result<Vec<Message>, Round>;

    /// What the message starting the thread `thread` in `channel` says,
    /// or none when it can't be read.
    fn root(&mut self, channel: &str, thread: &str) -> Option<String>;
}

/// Slack's own API, signed in as Joel.
pub struct Api<'a> {
    pub slack: &'a Slack,
    pub account: &'a Account,
}

impl Ears for Api<'_> {
    /// Slack's own answers are never cut short, so how far back one
    /// reaches is never asked.
    fn search(&mut self, query: &str, _back_to: &str) -> Result<Vec<Message>, Round> {
        let count = COUNT.to_string();
        let asked = [
            ("query", query),
            ("count", count.as_str()),
            ("sort", "timestamp"),
            ("sort_dir", "desc"),
            ("highlight", "false"),
        ];
        let said = match self.slack.read(self.account, "search.messages", &asked) {
            Ok(said) => said,
            Err(why) => {
                eprintln!("<4>couldn't search Slack: {why:?}");
                return Err(Round::Unasked);
            }
        };
        slack::found(&said).map_err(|why| {
            eprintln!("<4>Slack refused a search: {why}");
            if deaf::refuses_the_sign_in(&why) {
                Round::Deaf(Deafness::Refused)
            } else {
                Round::Unasked
            }
        })
    }

    fn root(&mut self, channel: &str, thread: &str) -> Option<String> {
        let asked = [("channel", channel), ("ts", thread), ("limit", "1")];
        match self
            .slack
            .read(self.account, "conversations.replies", &asked)
        {
            Ok(said) => slack::root(&said),
            Err(why) => {
                eprintln!("<4>couldn't read a thread's first message: {why:?}");
                None
            }
        }
    }
}

/// A tool gateway reading Slack for Joel, its answers worded for a model
/// (src/gateway.rs reads them).
pub struct Through {
    pub gateway: Gateway,
}

impl Through {
    /// Joel's own Slack id, as the gateway says who it reads as.
    ///
    /// # Errors
    ///
    /// What the round counts as, when it can't say.
    pub fn me(&mut self) -> Result<String, Round> {
        let said = self.called("slack_who_am_i", &json!({}))?;
        let id = inner(&said, "result")
            .lines()
            .find_map(|line| line.strip_prefix("User ID:"))
            .map(|id| id.trim().to_owned())
            .filter(|id| !id.is_empty());
        id.ok_or_else(|| {
            eprintln!("<4>the gateway didn't say whose Slack it reads");
            Round::Deaf(Deafness::Unreadable)
        })
    }

    /// The text of the gateway's answer to a search for the `limit` newest
    /// messages `query` finds, in its detailed form, bots' included.
    fn newest(&mut self, query: &str, limit: usize) -> Result<String, Round> {
        let said = self.called(
            "slack_search_public_and_private",
            &json!({
                "query": query,
                "limit": limit,
                "sort": "timestamp",
                "sort_dir": "desc",
                "include_bots": true,
                "response_format": "detailed",
            }),
        )?;
        Ok(inner(&said, "results"))
    }

    /// What `tool` says, or what the round counts it as: a failure naming
    /// one of Slack's own refusals of a sign-in is one.
    fn called(&mut self, tool: &str, arguments: &Value) -> Result<String, Round> {
        self.gateway.call(tool, arguments).map_err(|called| {
            let round = match &called {
                Called::Refused => Round::Deaf(Deafness::Refused),
                Called::NoSignIn(_) => Round::Deaf(Deafness::NoSignIn),
                Called::Unreadable(_) => Round::Deaf(Deafness::Unreadable),
                Called::Failed(why)
                    if why
                        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                        .any(deaf::refuses_the_sign_in) =>
                {
                    Round::Deaf(Deafness::Refused)
                }
                Called::Unreachable(_) | Called::Failed(_) => Round::Unasked,
            };
            eprintln!("<4>the gateway's {tool} said nothing: {called:?}");
            round
        })
    }
}

impl Ears for Through {
    fn search(&mut self, query: &str, back_to: &str) -> Result<Vec<Message>, Round> {
        // An answer it can't read is no quiet Slack: it isn't heard, and
        // since its cause lasts, it's said as deaf rather than let go.
        let unread = |why: String| {
            eprintln!("<4>the gateway's search for {query:?} can't be read: {why}");
            Round::Deaf(Deafness::Unreadable)
        };
        let results = self.newest(query, usize::from(COUNT))?;
        let Some(gave) = gateway::cut_to(&results) else {
            return gateway::found(&results).map_err(unread);
        };
        // Asked again for as many as it gave, an answer whole to that
        // count is one a typed result can't pass for.
        let results = self.newest(query, gave)?;
        let found = gateway::found(&results).map_err(|why| {
            unread(format!(
                "cut to {gave} of {COUNT}, then asked for {gave}, {why}"
            ))
        })?;
        let oldest = found.first().map_or("", |message| message.ts.as_str());
        if slack::micros(oldest).is_none_or(|oldest| Some(oldest) > slack::micros(back_to)) {
            return Err(unread(format!(
                "cut to the newest {}, back to {oldest:?}, it leaves unread what was said \
                 after {back_to} and before then",
                found.len()
            )));
        }
        eprintln!(
            "<5>the gateway cut its answer for {query:?} to {gave} of {COUNT}, so it read the \
             newest {gave}, back to {oldest}, which reaches the last read, at {back_to}"
        );
        Ok(found)
    }

    fn root(&mut self, channel: &str, thread: &str) -> Option<String> {
        let said = self
            .called(
                "slack_read_thread",
                &json!({ "channel_id": channel, "message_ts": thread, "limit": 1 }),
            )
            .ok()?;
        crate::gateway::root(&inner(&said, "messages"))
    }
}

/// The text a gateway's tool wraps under `key` in a JSON object, or its
/// whole answer when it isn't wrapped.
fn inner(said: &str, key: &str) -> String {
    serde_json::from_str::<Value>(said)
        .ok()
        .and_then(|value| value.get(key).and_then(Value::as_str).map(str::to_owned))
        .unwrap_or_else(|| said.to_owned())
}
