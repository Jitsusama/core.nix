//! Whether a reply a decision asks for goes out in public, by the rules
//! Joel set on 13 November ("make sure these notices never spam; we must
//! be insanely sure they are valuable"):
//!
//! - Only a stop's what-stopped and a resume's joel-asked are ever said in
//!   public; [`crate::decide`] already makes every other answer an alert.
//! - Each goes once per thread while its stop stands. A repeat is held
//!   back, and Joel's alert counts it.
//! - None goes to Joel himself, a bot, quoted or fenced text, an edit, or
//!   a message older than half an hour when it's read, which covers
//!   catching up after the reader was down. A stop still stops; only Joel
//!   hears of it.
//! - No more than three go out in any day, counted back from now.
//! - A switch turns replies off alone, leaving stops and alerts as they
//!   are.
//!
//! Every would-be reply is given the rule that let it through or held it,
//! which the log keeps, so two days of shadow show exactly what would have
//! been said and why.

use std::fmt;

use crate::decide::Reply;

/// How old a message may be when it's read, in seconds, and still be
/// answered.
pub const FRESH: u64 = 30 * 60;

/// How many replies may go out in a day.
pub const DAILY: usize = 3;

/// A day, in seconds, the span `DAILY` counts over.
pub const DAY: u64 = 24 * 60 * 60;

/// What a reply would answer, as the gate weighs it.
#[derive(Clone, Debug)]
pub struct Asked<'a> {
    /// The reply the decision asks for.
    pub reply: Reply,
    /// The thread it would go in, by a key naming the thread.
    pub thread: &'a str,
    /// Whether Joel said it.
    pub by_joel: bool,
    /// Whether a bot or an app said it.
    pub by_bot: bool,
    /// Whether it was edited after it was first said.
    pub edited: bool,
    /// How long ago it was said when it was read, in seconds.
    pub age: u64,
}

/// A reply that went out, or in shadow would have.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sent {
    /// When, in seconds since the epoch.
    pub at: u64,
    /// The thread it went in.
    pub thread: String,
    /// Which reply it was.
    pub reply: Reply,
}

/// What the gate says of a decision's reply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gate {
    /// The decision asks for no reply.
    Unasked,
    /// It goes out, by the rule named.
    Through(&'static str),
    /// It is held back, by the rule named, and only Joel hears.
    Held(&'static str),
}

impl Gate {
    /// Whether the reply goes out.
    #[must_use]
    pub fn goes(self) -> bool {
        matches!(self, Self::Through(_))
    }
}

/// Whether the reply `asked` wants goes out at `now`, given the replies
/// already sent and whether replies are switched off.
#[must_use]
pub fn gate(asked: &Asked<'_>, sent: &[Sent], now: u64, off: bool) -> Gate {
    if asked.reply == Reply::Nothing {
        return Gate::Unasked;
    }
    if asked.by_joel {
        return Gate::Held("by-joel");
    }
    if asked.by_bot {
        return Gate::Held("by-a-bot");
    }
    if asked.edited {
        return Gate::Held("an-edit");
    }
    if asked.age > FRESH {
        return Gate::Held("too-old");
    }
    // The reader leaves out the replies of stops Joel resumed
    // (crate::resume::cleared), so a thread is answered once of each
    // while its stop stands.
    if sent
        .iter()
        .any(|one| one.thread == asked.thread && one.reply == asked.reply)
    {
        return Gate::Held("repeat-in-thread");
    }
    if off {
        return Gate::Held("switched-off");
    }
    let today = sent
        .iter()
        .filter(|one| now.saturating_sub(one.at) < DAY)
        .count();
    if today >= DAILY {
        return Gate::Held("three-today");
    }
    Gate::Through("first-in-thread")
}

impl fmt::Display for Gate {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unasked => out.write_str("unasked"),
            Self::Through(rule) => write!(out, "through:{rule}"),
            Self::Held(rule) => write!(out, "held:{rule}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_800_000_000;

    fn asked(reply: Reply) -> Asked<'static> {
        Asked {
            reply,
            thread: "slack:C1:1.0",
            by_joel: false,
            by_bot: false,
            edited: false,
            age: 60,
        }
    }

    fn sent(at: u64, thread: &str) -> Sent {
        Sent {
            at,
            thread: thread.to_owned(),
            reply: Reply::WhatStopped,
        }
    }

    #[test]
    fn a_first_stop_in_a_fresh_thread_is_answered() {
        let said = gate(&asked(Reply::WhatStopped), &[], NOW, false);
        assert_eq!(said, Gate::Through("first-in-thread"), "it goes out");
        assert!(said.goes(), "and says so");
    }

    #[test]
    fn nothing_asked_is_nothing_gated() {
        let said = gate(&asked(Reply::Nothing), &[], NOW, false);
        assert_eq!(said, Gate::Unasked, "no reply is asked for");
    }

    #[test]
    fn each_rule_holds_its_reply_back() {
        let mut by_joel = asked(Reply::WhatStopped);
        by_joel.by_joel = true;
        let mut by_bot = asked(Reply::WhatStopped);
        by_bot.by_bot = true;
        let mut edited = asked(Reply::WhatStopped);
        edited.edited = true;
        let mut old = asked(Reply::WhatStopped);
        old.age = FRESH + 1;
        let cases = [
            (by_joel, "by-joel"),
            (by_bot, "by-a-bot"),
            (edited, "an-edit"),
            (old, "too-old"),
        ];
        for (one, rule) in cases {
            assert_eq!(
                gate(&one, &[], NOW, false),
                Gate::Held(rule),
                "{rule} holds it back"
            );
        }
    }

    #[test]
    fn a_thread_is_answered_once_of_each() {
        let before = [sent(NOW - 120, "slack:C1:1.0")];
        assert_eq!(
            gate(&asked(Reply::WhatStopped), &before, NOW, false),
            Gate::Held("repeat-in-thread"),
            "a second stop in the thread gets no reply"
        );
        assert!(
            gate(&asked(Reply::JoelAsked), &before, NOW, false).goes(),
            "a resume asked for there is answered once of its own"
        );
    }

    #[test]
    fn three_a_day_then_none() {
        let three = [
            sent(NOW - DAY + 60, "slack:C2:1.0"),
            sent(NOW - 600, "slack:C3:1.0"),
            sent(NOW - 60, "slack:C4:1.0"),
        ];
        assert_eq!(
            gate(&asked(Reply::WhatStopped), &three, NOW, false),
            Gate::Held("three-today"),
            "a fourth in a day is held back"
        );
        let a_day_on = NOW + 120;
        assert!(
            gate(&asked(Reply::WhatStopped), &three, a_day_on, false).goes(),
            "once the oldest is a day old, one more may go"
        );
    }

    #[test]
    fn the_switch_turns_replies_off_alone() {
        assert_eq!(
            gate(&asked(Reply::WhatStopped), &[], NOW, true),
            Gate::Held("switched-off"),
            "a switched-off reply is held back"
        );
    }

    #[test]
    fn a_gate_reads_as_its_rule() {
        assert_eq!(
            Gate::Through("first-in-thread").to_string(),
            "through:first-in-thread"
        );
        assert_eq!(Gate::Held("too-old").to_string(), "held:too-old");
        assert_eq!(Gate::Unasked.to_string(), "unasked");
    }
}
