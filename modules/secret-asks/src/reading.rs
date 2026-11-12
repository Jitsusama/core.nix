//! What each answered ask says to the journal, so the machine's health
//! can count how often Joel is asked and how he answers: fields alone, as
//! journald's native protocol carries them, never the value, the command
//! or the why.
//!
//! Asks are only worth their interruption while they stay rare and are
//! read, so four readings come of these lines: how many he's shown in a
//! day, yeses given faster than a line can be read, the same command
//! asked again, and asks a minted token would have spared him.

use std::os::unix::net::UnixDatagram;
use std::path::Path;

use serde_json::json;
use sha2::{Digest, Sha256};

use crate::ask::Ask;

/// Where journald takes native lines.
pub const JOURNAL: &str = "/run/systemd/journal/socket";

/// How an ask ended, as its line says it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Joel said yes, and the value went to the command.
    Released,
    /// Joel said no.
    Refused,
    /// The line changed after it was drawn, so it was drawn again.
    Changed,
    /// A yes came while the screen was locked.
    Locked,
    /// Its time ran out unanswered.
    Expired,
    /// Refused before it was drawn, for an item or a command the daemon
    /// doesn't serve, so Joel never saw it.
    Unseen,
}

impl Outcome {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Outcome::Released => "released",
            Outcome::Refused => "refused",
            Outcome::Changed => "changed",
            Outcome::Locked => "locked",
            Outcome::Expired => "expired",
            Outcome::Unseen => "unseen",
        }
    }
}

/// SHA-256 hex of some text, so two asks for the same item or command
/// can be told apart from others without either being written.
fn hashed(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .fold(String::new(), |mut hex, byte| {
            use std::fmt::Write;
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

/// A quest's id as a field may carry it: an agent writes the ask, so
/// anything but a plain id is said as unknown rather than let in, where
/// a line break would start a field of its own.
fn plain(id: Option<&str>) -> &str {
    id.filter(|id| {
        !id.is_empty()
            && id.len() <= 64
            && id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    })
    .unwrap_or("unknown")
}

/// The line for one answered ask, in journald's native form: one
/// `FIELD=value` a line, every value of a single line.
///
/// `shown_ms` is how long the line had been on screen when Joel
/// answered, as the launcher says, where it said.
#[must_use]
pub fn line(ask: &Ask, outcome: Outcome, shown_ms: Option<i64>) -> String {
    let argv = json!(ask.argv).to_string();
    let quest = plain(ask.quest.as_ref().map(|(id, _)| id.as_str()));
    let id = plain(Some(&ask.id));
    let mut fields = vec![
        ("SYSLOG_IDENTIFIER", "secret-asks".to_owned()),
        ("PRIORITY", "6".to_owned()),
        ("MESSAGE", format!("{id} {}", outcome.name())),
        ("ASK_ID", id.to_owned()),
        ("ASK_OUTCOME", outcome.name().to_owned()),
        ("ASK_ITEM_HASH", hashed(&ask.item)),
        ("ASK_ARGV_HASH", hashed(&argv)),
        ("ASK_QUEST", quest.to_owned()),
        // Whether a minted token could have served instead is the asking
        // tool's to say; the experiment's checker can't tell.
        ("ASK_AVOIDABLE", "unknown".to_owned()),
    ];
    if let Some(shown) = shown_ms {
        fields.push(("ASK_SHOWN_MS", shown.to_string()));
    }
    fields
        .iter()
        .fold(String::new(), |mut said, (field, value)| {
            use std::fmt::Write;
            let _ = writeln!(said, "{field}={value}");
            said
        })
}

/// Sends a line to journald at `journal`, saying on stderr where it can't,
/// so a reading lost is seen rather than silent.
pub fn send(journal: &Path, line: &str) {
    let sent = UnixDatagram::unbound().and_then(|socket| socket.send_to(line.as_bytes(), journal));
    if let Err(why) = sent {
        eprintln!("<4>secret-asks: can't tell the journal: {why}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(quest: Option<&str>) -> Ask {
        Ask {
            id: "3f0c9e2a7b5d41e8a6c2f9d01b7e4a35".to_owned(),
            asked_ms: 1,
            session: "s".to_owned(),
            quest: quest.map(|id| (id.to_owned(), "Title".to_owned())),
            item: "op://Private/Release Test/password".to_owned(),
            why: "a why that must not be written".to_owned(),
            argv: vec!["secret-asks".to_owned(), "received".to_owned()],
            tree: "/tree".to_owned(),
            given: "TOKEN".to_owned(),
            minutes: 1,
        }
    }

    #[test]
    fn says_fields_alone() {
        let said = line(&ask(Some("QEST-1")), Outcome::Released, Some(812));
        assert!(
            said.contains("ASK_OUTCOME=released\n"),
            "the outcome is said: {said}"
        );
        assert!(
            said.contains("ASK_SHOWN_MS=812\n"),
            "how long it showed is said: {said}"
        );
        assert!(
            said.contains("ASK_QUEST=QEST-1\n"),
            "the quest is said: {said}"
        );
        assert!(
            said.contains(&format!(
                "ASK_ITEM_HASH={}\n",
                hashed("op://Private/Release Test/password")
            )),
            "the item is said by its hash: {said}"
        );
        for kept in [
            "op://",
            "Release Test",
            "received",
            "a why",
            "/tree",
            "TOKEN",
        ] {
            assert!(!said.contains(kept), "{kept} stays out of the line: {said}");
        }
    }

    #[test]
    fn keeps_a_planted_field_out() {
        let said = line(
            &ask(Some("QEST-1\nASK_OUTCOME=refused")),
            Outcome::Released,
            None,
        );
        assert!(
            said.contains("ASK_QUEST=unknown\n"),
            "a quest with a line break is unknown: {said}"
        );
        assert_eq!(
            said.matches("ASK_OUTCOME=").count(),
            1,
            "no second outcome is planted: {said}"
        );
        assert!(
            !said.contains("ASK_SHOWN_MS"),
            "no time shown is said where none came: {said}"
        );
    }
}
