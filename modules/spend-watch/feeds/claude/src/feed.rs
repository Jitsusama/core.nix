//! One read of the plan: its sign-in, the usage endpoint's answer, and the
//! records that say them. Every read says the source's health as it is, so
//! spend-watch sees a failure as it happens and a recovery at once.

use std::path::PathBuf;
use std::time::Duration;

use jiff::{SignedDuration, Timestamp};

use crate::login::{self, Login, Plan};
use crate::records::{self, Cap, Health};
use crate::usage::{self, Window};

/// The share of a window each cap warns at.
const WARN: f64 = 0.8;

/// How long to wait when the endpoint asks for a pause without saying how
/// long.
const PAUSE: Duration = Duration::from_secs(300);

/// What the plan's usage is asked of: the real endpoint, or a recorded one.
pub trait Endpoint {
    /// The answer to a request for the plan's usage with this access token.
    fn usage(&mut self, token: &str) -> Result<Answer, Unreached>;
}

/// What the endpoint answered, and how long it took.
#[derive(Debug, Clone)]
pub struct Answer {
    pub status: u16,
    pub body: String,
    /// How long it asked to be left alone, with a 429.
    pub retry_after: Option<Duration>,
    pub millis: u64,
}

/// Why the endpoint couldn't be asked, and how long trying took.
#[derive(Debug, Clone)]
pub struct Unreached {
    pub why: String,
    pub millis: u64,
}

/// One read's records, each a line of the contract, and how it went.
#[derive(Debug)]
pub struct Read {
    pub lines: Vec<String>,
    pub outcome: Outcome,
}

/// How a read went, for when the next is due.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The endpoint said how the plan stands.
    Answered,
    /// There was nothing to ask with: no sign-in, or an expired one.
    Unasked,
    /// The sign-in or the endpoint failed.
    Failed,
    /// The endpoint, asked, said not to ask again before then.
    Paused(Timestamp),
    /// A pause the endpoint asked for still held, so it wasn't asked.
    Waiting(Timestamp),
}

/// The plan's feed, reading the sign-in kept in one file.
pub struct Feed {
    credentials: PathBuf,
    paused: Option<Timestamp>,
}

impl Feed {
    /// A feed for the sign-in in `credentials`, Claude Code's
    /// `.credentials.json` in its config folder.
    pub fn new(credentials: PathBuf) -> Feed {
        Feed {
            credentials,
            paused: None,
        }
    }

    /// What the feed says once, as it starts: the sign-in it offers.
    pub fn introduction(&self) -> Vec<String> {
        vec![records::sign_in()]
    }

    /// Reads the sign-in and, if there's one to ask with, the plan's usage,
    /// as of `now`. While the endpoint has asked for a pause it isn't asked,
    /// and the read says so.
    pub fn read(&mut self, endpoint: &mut impl Endpoint, now: Timestamp) -> Read {
        if let Some(until) = self.paused.filter(|until| *until > now) {
            return Read {
                outcome: Outcome::Waiting(until),
                ..paused(until, now, Vec::new())
            };
        }
        self.paused = None;
        match login::read(&self.credentials, now) {
            Ok(Login::Signed { token, plan }) => self.ask(endpoint, &token, &plan, now),
            Ok(Login::Missing) => unasked(
                Health::Optional,
                "no Claude plan is signed in for pi-claude",
            ),
            // Only Claude Code renews its sign-in, and only while it runs, so
            // an expired one is a plan not in use, whose windows can't move
            // until it is: nothing is wrong, and the last caps stand.
            Ok(Login::Expired) => unasked(
                Health::Fine,
                "the plan hasn't been used since its sign-in expired; \
                 Claude Code renews it the next time it runs",
            ),
            Ok(Login::Unscoped) => failed(
                Vec::new(),
                "the sign-in can't read the plan's usage, which needs the user:profile scope",
            ),
            Err(why) => failed(Vec::new(), &why),
        }
    }

    fn ask(
        &mut self,
        endpoint: &mut impl Endpoint,
        token: &str,
        plan: &Plan,
        now: Timestamp,
    ) -> Read {
        let answer = match endpoint.usage(token) {
            Ok(answer) => answer,
            Err(unreached) => {
                let cost = records::read(0, unreached.millis);
                let why = format!("couldn't reach Claude's usage endpoint: {}", unreached.why);
                return failed(vec![cost], &why);
            }
        };
        let cost = vec![records::read(answer.body.len(), answer.millis)];
        match answer.status {
            200 => said(&answer.body, plan, now, cost),
            401 => Read {
                lines: [records::health(
                    Health::SignedOut,
                    "Claude refused the plan's sign-in; sign in again",
                )]
                .into_iter()
                .chain(cost)
                .collect(),
                outcome: Outcome::Failed,
            },
            429 => {
                let wait = SignedDuration::try_from(answer.retry_after.unwrap_or(PAUSE))
                    .unwrap_or(SignedDuration::from_secs(300));
                let until = now + wait;
                self.paused = Some(until);
                paused(until, now, cost)
            }
            status => failed(cost, &format!("Claude's usage endpoint answered {status}")),
        }
    }
}

/// The caps an answer says, or why it says nothing usable.
fn said(body: &str, plan: &Plan, now: Timestamp, cost: Vec<String>) -> Read {
    let usage = match usage::parse(body) {
        Ok(usage) => usage,
        Err(why) => return failed(cost, &format!("Claude's usage endpoint: {why}")),
    };
    // A window whose reset has passed has rolled, so its figure is no
    // longer how it stands.
    let current = |window: Option<Window>| {
        window.filter(|window| window.resets.is_none_or(|resets| resets > now))
    };
    let title = plan.title();
    // Each allowance is named for its window's length in words, in Title
    // Case, as the contract has a window named; the plan's name is its
    // budget's label, and a figure says the window short itself.
    let caps: Vec<String> = [
        ("five-hours", "Five Hours", 5, current(usage.five_hours)),
        ("week", "Week", 168, current(usage.week)),
    ]
    .into_iter()
    .filter_map(|(name, label, window_hours, window)| {
        let window = window?;
        Some(records::cap(&Cap {
            name,
            label: label.to_owned(),
            window_hours,
            used: window.used,
            resets: window.resets,
            warn: WARN,
            at: now,
        }))
    })
    .collect();
    if caps.is_empty() {
        return failed(
            cost,
            "Claude's usage endpoint said nothing usable of the five hours or the week",
        );
    }
    Read {
        lines: caps
            .into_iter()
            .chain([records::health(
                Health::Fine,
                &format!("signed in to {title}"),
            )])
            .chain(cost)
            .collect(),
        outcome: Outcome::Answered,
    }
}

fn unasked(health: Health, message: &str) -> Read {
    Read {
        lines: vec![records::health(health, message)],
        outcome: Outcome::Unasked,
    }
}

fn failed(cost: Vec<String>, why: &str) -> Read {
    Read {
        lines: [records::health(Health::Error, why)]
            .into_iter()
            .chain(cost)
            .collect(),
        outcome: Outcome::Failed,
    }
}

fn paused(until: Timestamp, now: Timestamp, cost: Vec<String>) -> Read {
    let minutes = until
        .duration_since(now)
        .as_secs()
        .unsigned_abs()
        .div_ceil(60)
        .max(1);
    let after = if minutes == 1 {
        "a minute".into()
    } else {
        format!("{minutes} minutes")
    };
    let message = format!("Claude asked for a pause in reading the plan's usage; back in {after}");
    Read {
        lines: [records::health(Health::Stalled, &message)]
            .into_iter()
            .chain(cost)
            .collect(),
        outcome: Outcome::Paused(until),
    }
}
