//! How often Joel is asked for a secret and how he answers, from the line
//! secret-asks writes for each answered ask: fields alone, never the value
//! or the command. An ask is worth its interruption only while asks stay
//! rare and are read, so four figures come of them: those shown, yeses
//! sooner than a line can be read, the same item and command asked again,
//! and those a minted token would have spared him.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// The identifier secret-asks writes its lines under.
pub const IDENTIFIER: &str = "secret-asks";

/// A yes sooner than this after the ask was drawn wasn't read.
const READ_IN_MS: i64 = 1_000;

/// One answered ask, as its line says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ask {
    /// released, refused, changed, locked, expired or unseen.
    pub outcome: String,
    /// How long it was on screen when he answered, where the launcher said.
    pub shown_ms: Option<i64>,
    /// The item's hash, so the same ask is known again unwritten.
    pub item: String,
    /// The command's hash.
    pub argv: String,
    /// Whether a minted token could have served instead: yes, no or
    /// unknown, as the asking tool says.
    pub avoidable: String,
}

/// The ask a secret-asks line says, from its fields by name, or none when
/// it says no outcome.
pub fn of(text: impl Fn(&str) -> Option<String>) -> Option<Ask> {
    Some(Ask {
        outcome: text("ASK_OUTCOME")?,
        shown_ms: text("ASK_SHOWN_MS").and_then(|shown| shown.parse().ok()),
        item: text("ASK_ITEM_HASH").unwrap_or_default(),
        argv: text("ASK_ARGV_HASH").unwrap_or_default(),
        avoidable: text("ASK_AVOIDABLE").unwrap_or_default(),
    })
}

/// The day's asks.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Asks {
    /// Asks drawn for him: all but those refused unseen.
    pub shown: u64,
    /// Yeses given sooner than a line can be read.
    pub unread_yeses: u64,
    /// Asks for an item and command asked for already in the day.
    pub again: u64,
    /// Asks the asking tool says a minted token would have spared.
    pub avoidable: u64,
    /// Each item and command asked for in the day, by their hashes.
    pub asked: BTreeSet<(String, String)>,
}

impl Asks {
    /// Count one answered ask.
    pub fn took(&mut self, ask: &Ask) {
        if ask.outcome == "unseen" {
            return;
        }
        self.shown += 1;
        if ask.outcome == "released" && ask.shown_ms.is_some_and(|shown| shown < READ_IN_MS) {
            self.unread_yeses += 1;
        }
        if !self.asked.insert((ask.item.clone(), ask.argv.clone())) {
            self.again += 1;
        }
        if ask.avoidable == "yes" {
            self.avoidable += 1;
        }
    }

    /// The share of asks shown that a token would have spared, in percent.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "counts of a day's asks")]
    pub fn avoidable_percent(&self) -> f64 {
        if self.shown == 0 {
            0.0
        } else {
            self.avoidable as f64 * 100.0 / self.shown as f64
        }
    }

    /// The day's asks in a line, when any were shown.
    #[must_use]
    pub fn said(&self) -> Option<String> {
        (self.shown > 0).then(|| {
            format!(
                "{} shown, {} said yes to before it could be read, {} asked again",
                self.shown, self.unread_yeses, self.again
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::day::Day;
    use crate::journal;

    fn ask(outcome: &str, shown_ms: Option<i64>, item: &str, avoidable: &str) -> Ask {
        Ask {
            outcome: outcome.into(),
            shown_ms,
            item: item.into(),
            argv: "argv".into(),
            avoidable: avoidable.into(),
        }
    }

    #[test]
    fn asks_are_counted_as_shown_read_and_asked_again() {
        let mut asks = Asks::default();
        asks.took(&ask("released", Some(400), "a", "unknown"));
        asks.took(&ask("released", Some(2_500), "a", "yes"));
        asks.took(&ask("refused", None, "b", "no"));
        asks.took(&ask("unseen", None, "c", "yes"));
        assert_eq!(
            (asks.shown, asks.unread_yeses, asks.again, asks.avoidable),
            (3, 1, 1, 1),
            "an unseen ask was shown, or a quick yes or a second ask went uncounted"
        );
        assert!(
            (asks.avoidable_percent() - 100.0 / 3.0).abs() < 1e-9,
            "the avoidable share is {}",
            asks.avoidable_percent()
        );
        assert_eq!(
            asks.said().as_deref(),
            Some("3 shown, 1 said yes to before it could be read, 1 asked again"),
            "the day's asks in words"
        );
    }

    #[test]
    fn an_ask_said_as_information_is_taken_into_the_day() {
        let line = r#"{"__REALTIME_TIMESTAMP":"1","PRIORITY":"6",
            "SYSLOG_IDENTIFIER":"secret-asks","MESSAGE":"k3 released",
            "ASK_OUTCOME":"released","ASK_SHOWN_MS":"300","ASK_ITEM_HASH":"i",
            "ASK_ARGV_HASH":"v","ASK_AVOIDABLE":"unknown"}"#
            .replace('\n', "");
        let entry = journal::parse(&line).expect("an entry");
        let mut day = Day::new("1970-01-01");
        day.take(&entry);
        assert_eq!(
            (day.asks.shown, day.asks.unread_yeses),
            (1, 1),
            "the ask's line was left out as information, or its yes went unread"
        );
        assert!(day.things.is_empty(), "an ask isn't a thing's warning");
    }
}
