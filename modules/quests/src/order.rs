//! The one order Quests keeps with nothing typed, as the launcher's
//! `Order.js` kept it for things returned to (`returned`): those open
//! first, by their priority and then their rank; then each by when it was
//! last touched, the newest first, one never touched after; then A to Z,
//! numbers by their value so Part 9 comes before Part 10, case alike.
//! Each title is broken into its pieces once per making, where the shell
//! broke both apart at each of a sort's ten thousand comparisons.

use std::cmp::Ordering;

use crate::quest::Quest;

/// A title's runs of digits and of the rest, the rest in lower case.
pub fn pieces(name: &str) -> Vec<String> {
    let mut pieces: Vec<String> = Vec::new();
    let mut digits = false;
    for letter in name.to_lowercase().chars() {
        let digit = letter.is_ascii_digit();
        match pieces.last_mut() {
            Some(last) if digit == digits => last.push(letter),
            _ => pieces.push(letter.to_string()),
        }
        digits = digit;
    }
    pieces
}

/// A to Z by pieces: two runs of digits by their value, the rest as
/// written, a name that is the start of another first.
fn by_pieces(one: &[String], other: &[String]) -> Ordering {
    for (a, b) in one.iter().zip(other) {
        let numbers = a.starts_with(|c: char| c.is_ascii_digit())
            && b.starts_with(|c: char| c.is_ascii_digit());
        let said = if numbers {
            let value = |run: &str| run.parse::<f64>().unwrap_or(f64::INFINITY);
            value(a).total_cmp(&value(b))
        } else {
            a.cmp(b)
        };
        if said.is_ne() {
            return said;
        }
    }
    one.len().cmp(&other.len())
}

/// When each was touched, the newest first, one with none after them all.
fn by_updated(one: &str, other: &str) -> Ordering {
    match (one.is_empty(), other.is_empty()) {
        (false, false) => other.cmp(one),
        (empty, other_empty) => empty.cmp(&other_empty),
    }
}

/// The quests in Quests' order, those alike in it by their id.
pub fn sorted<'a>(quests: &[Quest<'a>]) -> Vec<Quest<'a>> {
    let mut keyed: Vec<(Quest<'a>, Vec<String>)> = quests
        .iter()
        .map(|quest| (*quest, pieces(quest.name())))
        .collect();
    keyed.sort_by(|(a, a_pieces), (b, b_pieces)| {
        b.open()
            .cmp(&a.open())
            .then_with(|| {
                if a.open() && b.open() {
                    a.bucket()
                        .cmp(&b.bucket())
                        .then_with(|| a.rank().total_cmp(&b.rank()))
                } else {
                    Ordering::Equal
                }
            })
            .then_with(|| by_updated(a.updated(), b.updated()))
            .then_with(|| by_pieces(a_pieces, b_pieces))
            .then_with(|| a.id().cmp(b.id()))
    });
    keyed.into_iter().map(|(quest, _)| quest).collect()
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    fn ids(records: &[Value]) -> Vec<&str> {
        let quests: Vec<Quest> = records.iter().map(Quest).collect();
        sorted(&quests).into_iter().map(Quest::id).collect()
    }

    #[test]
    fn those_open_come_first_by_priority_then_rank() {
        let records = [
            json!({"id": "ended", "status": "concluded", "updated": "2026-11-01"}),
            json!({"id": "queued", "status": "active", "priority": "queued", "rank": 1}),
            json!({"id": "second", "status": "active", "priority": "driving", "rank": 2}),
            json!({"id": "first", "status": "active", "priority": "driving", "rank": 1}),
            json!({"id": "unranked", "status": "active", "priority": "driving"}),
        ];
        assert_eq!(
            ids(&records),
            ["first", "second", "unranked", "queued", "ended"]
        );
    }

    #[test]
    fn the_rest_by_when_touched_the_newest_first_then_a_to_z() {
        let records = [
            json!({"id": "a", "title": "Part 10", "status": "retired", "updated": "2026-01-01"}),
            json!({"id": "b", "title": "part 9", "status": "retired", "updated": "2026-01-01"}),
            json!({"id": "c", "title": "Never", "status": "retired"}),
            json!({"id": "d", "title": "Newest", "status": "concluded",
                "updated": "2026-10-21T09:00:00.000Z"}),
        ];
        assert_eq!(ids(&records), ["d", "b", "a", "c"]);
    }

    #[test]
    fn a_title_breaks_into_runs_of_digits_and_the_rest() {
        assert_eq!(pieces("Tab 10b"), ["tab ", "10", "b"]);
        assert!(pieces("").is_empty(), "nothing has no pieces");
    }
}
