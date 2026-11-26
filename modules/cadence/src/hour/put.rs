//! What the hour puts to each owner: every line it filed for them this
//! hour in one question, so a red line wakes the owner and wants an
//! answer, where a finding woke no one and asked for nothing (Joel, 26
//! November: "we need to make sure we have various self improvement
//! loops built in"). Each line it asked carries the question's id, so a
//! reply naming it answers that line.

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
                    "{OPENS}{} lines past their targets are yours. Answer with re set to this \
                     question, line by line by number. {ANSWERS_GO}\n\n{}",
                    texts.len(),
                    lines.join("\n\n")
                )
            };
            (owner, question)
        })
        .collect()
}

/// Marks each drag first told to `owner` on `date` and not yet asked as
/// asked in the question `id`.
pub fn mark_asked(ledger: &mut [Entry], owner: &str, date: &str, id: &str) {
    for entry in ledger.iter_mut().filter(|e| {
        e.open
            && e.asked.is_none()
            && e.kind.as_deref() == Some("drag")
            && e.owner.as_deref() == Some(owner)
            && e.told.as_deref() == Some(date)
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
            ("ABC123", "cadence hour: the second is yours."),
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
            both.starts_with("cadence hour: 2 lines past their targets are yours")
                && both.contains("1. the first is yours.")
                && both.contains("2. the second is yours.")
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

    fn entry(key: &str, owner: &str, told: &str, kind: &str) -> Entry {
        serde_json::from_str(&format!(
            r#"{{"key":"{key}","owner":"{owner}","first":"{told}","last":"{told}","last_to":0,
            "due":0,"words":"","axis":"slips","evidence":"","fell_when":"","open":true,
            "told":"{told}","late":false,"kind":"{kind}"}}"#
        ))
        .expect("an entry parses")
    }

    #[test]
    fn only_the_drags_told_to_that_owner_today_carry_the_question_s_id() {
        let mut ledger = vec![
            entry("hour:a", "ABC123", "2026-11-26", "drag"),
            entry("hour:b", "ABC123", "2026-11-25", "drag"),
            entry("hour:c", "DEF456", "2026-11-26", "drag"),
            entry("security:d", "ABC123", "2026-11-26", "must"),
        ];
        let mut earlier = entry("hour:e", "ABC123", "2026-11-26", "drag");
        earlier.asked = Some("first-question".to_owned());
        ledger.push(earlier);
        mark_asked(&mut ledger, "ABC123", "2026-11-26", "the-question");
        let asked: Vec<Option<&str>> = ledger.iter().map(|e| e.asked.as_deref()).collect();
        assert_eq!(
            asked,
            [
                Some("the-question"),
                None,
                None,
                None,
                Some("first-question")
            ],
            "a line not told to that owner today, not a drag, or asked before took the id"
        );
    }
}
