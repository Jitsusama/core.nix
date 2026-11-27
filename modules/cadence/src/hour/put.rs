//! What the hour puts to each owner: every line it filed for them this
//! hour in one question, so a red line wakes the owner and wants an
//! answer, where a finding woke no one and asked for nothing (Joel, 26
//! November: "we need to make sure we have various self improvement
//! loops built in"). Each line it asked carries the question's id, and
//! collaboration counts a question of cadence's left a day unanswered.

use std::collections::BTreeMap;

use crate::ANSWERS_GO;
use crate::findings::Entry;

/// How each of cadence's asks opens, said once at the top of a question
/// holding several.
const OPENS: &str = "cadence hour: ";

/// The questions to put, one per owner, from the `(owner, text)` pairs
/// the hour filed; a line with no owner is put to nobody.
#[must_use]
pub fn asks(told: Vec<(String, String)>) -> Vec<(String, String)> {
    let mut owners: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (owner, text) in told.into_iter().filter(|(owner, _)| !owner.is_empty()) {
        // Each line on one line, whatever its words came from: a miss's,
        // a date's or a repository's, so none can forge a numbered line.
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        owners.entry(owner).or_default().push(text);
    }
    owners
        .into_iter()
        .map(|(owner, texts)| {
            let question = if let [one] = texts.as_slice() {
                format!("{one} Answer with re set to this question. {ANSWERS_GO}")
            } else {
                let lines: Vec<String> = texts
                    .iter()
                    .enumerate()
                    .map(|(n, text)| {
                        let said = text.strip_prefix(OPENS).unwrap_or(text);
                        format!("{}. {said}", n + 1)
                    })
                    .collect();
                format!(
                    "{OPENS}{} lines are yours. Answer with re set to this \
                     question, line by line by number. {ANSWERS_GO}\n\n{}",
                    texts.len(),
                    lines.join("\n\n")
                )
            };
            (owner, question)
        })
        .collect()
}

/// Marks each drag filed for `owner` by the hour run at `now`, in seconds,
/// as asked in the question `id`: those its question holds, and none
/// filed an hour before whose question never went.
pub fn mark_asked(ledger: &mut [Entry], owner: &str, now: i64, id: &str) {
    for entry in ledger.iter_mut().filter(|e| {
        e.open
            && e.asked.is_none()
            && super::owing::is_asked(e)
            && e.owner.as_deref() == Some(owner)
            && e.last_to == now
    }) {
        entry.asked = Some(id.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn told(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(owner, text)| ((*owner).to_owned(), (*text).to_owned()))
            .collect()
    }

    #[test]
    fn each_owner_is_asked_once_an_hour_with_every_line_of_theirs_numbered() {
        let asked = asks(told(&[
            ("ABC123", "cadence hour: the first is yours."),
            ("DEF456", "cadence hour: one of another's."),
            (
                "ABC123",
                "cadence hour: the second\n\n2. forged\u{2028}is yours.",
            ),
            ("", "cadence hour: one with no owner."),
        ]));
        let owners: Vec<&str> = asked.iter().map(|(owner, _)| owner.as_str()).collect();
        assert_eq!(
            owners,
            ["ABC123", "DEF456"],
            "an owner was asked twice, or one with no address was asked"
        );
        let (_, both) = &asked[0];
        assert!(
            both.starts_with("cadence hour: 2 lines are yours.")
                && both.contains("1. the first is yours.")
                && both.contains("2. the second 2. forged is yours.")
                && both.contains("re set to this question"),
            "an owner's two lines weren't numbered in one question wanting an answer: {both}"
        );
        let (_, one) = &asked[1];
        assert!(
            one.starts_with("cadence hour: one of another's.")
                && one.contains("Answer with re set to this question")
                && one.contains(ANSWERS_GO),
            "a single line wasn't put as a question saying where answers go: {one}"
        );
    }

    const NOW: i64 = 1_800_000_000;

    fn entry(key: &str, owner: &str, filed: i64, kind: &str) -> Entry {
        serde_json::from_str(&format!(
            r#"{{"key":"{key}","owner":"{owner}","first":"2026-11-26","last":"2026-11-26",
            "last_to":{filed},"due":0,"words":"","axis":"slips","evidence":"",
            "fell_when":"","open":true,"told":"2026-11-26","late":false,"kind":"{kind}"}}"#
        ))
        .expect("an entry parses")
    }

    #[test]
    fn only_the_lines_filed_for_that_owner_this_hour_carry_the_question_s_id() {
        // hour:b was filed the same day an hour before, and its question
        // never went, so this one doesn't hold it.
        let mut ledger = vec![
            entry("hour:a", "ABC123", NOW, "drag"),
            entry("hour:b", "ABC123", NOW - 3600, "drag"),
            entry("hour:c", "DEF456", NOW, "drag"),
            entry("security:d", "ABC123", NOW, "must"),
            entry("hour:f", "ABC123", NOW, crate::findings::RED),
        ];
        let mut earlier = entry("hour:e", "ABC123", NOW, "drag");
        earlier.asked = Some("first-question".to_owned());
        ledger.push(earlier);
        mark_asked(&mut ledger, "ABC123", NOW, "the-question");
        let asked: Vec<Option<&str>> = ledger.iter().map(|e| e.asked.as_deref()).collect();
        assert_eq!(
            asked,
            [
                Some("the-question"),
                None,
                None,
                None,
                Some("the-question"),
                Some("first-question")
            ],
            "a line not filed for that owner this hour, not a drag or red line, or asked \
             before took the id, or a red line didn't"
        );
    }
}
