//! How a command ends when it can't do what it was asked: one exit code
//! from the table every command shares, and the error said in three parts,
//! what failed, why and what to do, as text for a person or as JSON for a
//! machine (docs/commands.md, Errors and Exit Codes).

use std::fmt;

use serde_json::{Value, json};

use crate::ink::Pen;
use crate::layout::{self, Room};

/// The exit codes, and only these: past 0 and 1 they're sysexits.h's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Exit {
    /// Done, or nothing was wrong; a question's yes.
    Done,
    /// It tried and failed; a question's no.
    Failed,
    /// Asked wrongly: an unknown word or flag, a value it can't take.
    Usage,
    /// What it was given to read isn't what it reads.
    BadInput,
    /// The thing named isn't there.
    NotFound,
    /// What it needs isn't answering.
    Unavailable,
    /// It broke in a way it shouldn't.
    Bug,
    /// Not now: a lock, a busy builder, the network.
    TryAgain,
    /// Not done on purpose: a guard, a permission, an answer of no.
    Refused,
    /// Its configuration is wrong.
    Config,
    /// Interrupted by Ctrl+C.
    Interrupted,
}

impl Exit {
    /// Every code, in the table's order.
    pub const ALL: [Self; 11] = [
        Self::Done,
        Self::Failed,
        Self::Usage,
        Self::BadInput,
        Self::NotFound,
        Self::Unavailable,
        Self::Bug,
        Self::TryAgain,
        Self::Refused,
        Self::Config,
        Self::Interrupted,
    ];

    /// The number the process exits with.
    pub fn code(self) -> u8 {
        match self {
            Self::Done => 0,
            Self::Failed => 1,
            Self::Usage => 64,
            Self::BadInput => 65,
            Self::NotFound => 66,
            Self::Unavailable => 69,
            Self::Bug => 70,
            Self::TryAgain => 75,
            Self::Refused => 77,
            Self::Config => 78,
            Self::Interrupted => 130,
        }
    }

    /// The word an error's JSON names it by.
    pub fn kind(self) -> &'static str {
        match self {
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Usage => "usage",
            Self::BadInput => "bad-input",
            Self::NotFound => "not-found",
            Self::Unavailable => "unavailable",
            Self::Bug => "bug",
            Self::TryAgain => "try-again",
            Self::Refused => "refused",
            Self::Config => "config",
            Self::Interrupted => "interrupted",
        }
    }

    /// What it means, as help's EXIT group says it where a command gives
    /// no words of its own.
    pub fn means(self) -> &'static str {
        match self {
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Usage => "asked wrongly",
            Self::BadInput => "can't read what it was given",
            Self::NotFound => "not there",
            Self::Unavailable => "not answering",
            Self::Bug => "broke",
            Self::TryAgain => "not now",
            Self::Refused => "refused",
            Self::Config => "configured wrongly",
            Self::Interrupted => "interrupted",
        }
    }

    /// The exit a code means, where it's one of the table's.
    pub fn of(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|exit| exit.code() == code)
    }
}

impl From<Exit> for std::process::ExitCode {
    fn from(exit: Exit) -> Self {
        Self::from(exit.code())
    }
}

/// An error in its three parts, and the exit it ends with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub exit: Exit,
    /// What failed, in one line, naming the thing.
    pub what: String,
    /// Why, as the system said it, made plain where it can be.
    pub why: Option<String>,
    /// A command to type, or the one thing to check.
    pub then: Option<String>,
    /// The causes under `why`, said only with `--verbose`.
    pub chain: Vec<String>,
}

impl Failure {
    /// A failure that ends with `exit`, saying what failed.
    pub fn new(exit: Exit, what: impl Into<String>) -> Self {
        Self {
            exit,
            what: what.into(),
            why: None,
            then: None,
            chain: Vec::new(),
        }
    }

    /// The same, saying why.
    #[must_use]
    pub fn because(mut self, why: impl Into<String>) -> Self {
        self.why = Some(why.into());
        self
    }

    /// The same, saying what to do: a command to type, after "try".
    #[must_use]
    pub fn try_this(mut self, then: impl Into<String>) -> Self {
        self.then = Some(then.into());
        self
    }

    /// The same, with a cause under why, said only when asked to be
    /// verbose.
    #[must_use]
    pub fn caused_by(mut self, cause: impl Into<String>) -> Self {
        self.chain.push(cause.into());
        self
    }

    /// As a person reads it on stderr: the failed mark in alert and what
    /// failed, then why in muted and what to try, each two cells in, each
    /// wrapped under its own words.
    pub fn text(&self, pen: &Pen, room: Room, verbose: bool) -> String {
        let inset = layout::MARK;
        let mut lines = vec![format!(
            "{}{}",
            pen.marked("failed", "alert"),
            pen.role("text", &layout::hung(&self.what, inset, room))
        )];
        let under = |words: &str, role: &str| {
            format!(
                "{}{}",
                " ".repeat(inset),
                pen.role(role, &layout::hung(words, inset, room))
            )
        };
        if let Some(why) = &self.why {
            lines.push(under(why, "muted"));
        }
        if verbose {
            lines.extend(self.chain.iter().map(|cause| under(cause, "muted")));
        }
        if let Some(then) = &self.then {
            // A command to type is never wrapped, so it copies whole.
            lines.push(format!(
                "{}{}  {}",
                " ".repeat(inset),
                pen.role("muted", "try"),
                pen.role("text", then)
            ));
        }
        lines.join("\n") + "\n"
    }

    /// As a machine reads it on stderr under `json` and `jsonl`.
    pub fn json(&self, verbose: bool) -> Value {
        let mut value = json!({
            "schema": "error/1",
            "kind": self.exit.kind(),
            "exit": self.exit.code(),
            "what": self.what,
            "why": self.why,
            "try": self.then,
        });
        if verbose {
            value["chain"] = json!(self.chain);
        }
        value
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.why {
            Some(why) => write!(formatter, "{}: {why}", self.what),
            None => write!(formatter, "{}", self.what),
        }
    }
}

impl std::error::Error for Failure {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ink::{Ink, Pen};

    fn unreachable() -> Failure {
        Failure::new(Exit::TryAgain, "couldn't reach the builder")
            .because("connection refused after 10 s")
            .try_this("nix-offload there . checks.x86_64-linux.example-nixos")
            .caused_by("tcp 10.0.0.2:22")
    }

    #[test]
    fn every_code_is_its_tables() {
        let codes: Vec<u8> = Exit::ALL.iter().map(|exit| exit.code()).collect();
        assert_eq!(codes, [0, 1, 64, 65, 66, 69, 70, 75, 77, 78, 130]);
        assert_eq!(Exit::of(75), Some(Exit::TryAgain));
        assert_eq!(Exit::of(2), None);
    }

    #[test]
    fn a_failure_reads_in_three_parts() {
        assert_eq!(
            unreachable().text(&Pen::plain(), Room::Unbounded, false),
            "\u{2715} couldn't reach the builder\n  \
             connection refused after 10 s\n  \
             try  nix-offload there . checks.x86_64-linux.example-nixos\n"
        );
    }

    #[test]
    fn its_causes_are_said_only_when_verbose() {
        let text = unreachable().text(&Pen::plain(), Room::Unbounded, true);
        assert!(text.contains("\n  tcp 10.0.0.2:22\n"), "{text}");
    }

    #[test]
    fn a_long_why_wraps_under_itself() {
        let failure = Failure::new(Exit::Failed, "lost").because("one two three four five");
        assert_eq!(
            failure.text(&Pen::plain(), Room::Cells(12), false),
            "\u{2715} lost\n  one two\n  three four\n  five\n"
        );
    }

    #[test]
    fn its_mark_wears_the_alert() {
        let pen = Pen::new(Ink::sixteen(), None);
        let text = Failure::new(Exit::Failed, "lost").text(&pen, Room::Unbounded, false);
        assert!(
            text.starts_with("\u{1b}[31m\u{2715}\u{1b}[0m lost"),
            "{text:?}"
        );
    }

    #[test]
    fn as_json_it_names_its_kind_and_exit() {
        assert_eq!(
            unreachable().json(false),
            json!({"schema": "error/1", "kind": "try-again", "exit": 75,
                   "what": "couldn't reach the builder",
                   "why": "connection refused after 10 s",
                   "try": "nix-offload there . checks.x86_64-linux.example-nixos"})
        );
        assert_eq!(
            unreachable().json(true)["chain"],
            json!(["tcp 10.0.0.2:22"])
        );
        assert_eq!(
            Failure::new(Exit::Failed, "lost").json(false)["why"],
            Value::Null
        );
    }
}
