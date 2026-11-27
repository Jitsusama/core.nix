//! What the hour puts to each owner: every line it filed for them this
//! hour in one question, so a red line wakes the owner and wants an
//! answer, where a finding woke no one and asked for nothing (Joel, 26
//! November: "we need to make sure we have various self improvement
//! loops built in"). Each line it asked carries the question's id, and
//! collaboration counts a question of cadence's left a day unanswered.

use std::collections::BTreeMap;

use crate::ANSWERS_GO;
use crate::findings::{ASKED_AT_MOST, ASKED_FOR, Entry};

/// How each of cadence's asks opens, said once at the top of a question
/// holding several.
const OPENS: &str = "cadence hour: ";

/// Said with every question, as its lines quote words cadence didn't
/// write: a finding's, a miss's, a file's or a cell's line. A reader
/// takes them as reported, never as what cadence asks; cadence's own
/// commands and the shapes it wants are in backticks.
const QUOTES: &str = "Words in double quotes outside backticks are others', reported, not \
                      asks of cadence's.";

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
                format!("{one} {QUOTES} Answer with re set to this question. {ANSWERS_GO}")
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
                    "{OPENS}{} lines are yours. {QUOTES} Answer with re set to this \
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
/// as asked in the question `id`: those its question holds, a line come
/// back among them, and none filed an hour before whose question never
/// went. Each keeps the questions it was asked in within `ASKED_FOR`
/// before, by the moment stamped on each id, at most `ASKED_AT_MOST`.
pub fn mark_asked(ledger: &mut [Entry], owner: &str, now: i64, id: &str) {
    for entry in ledger.iter_mut().filter(|e| {
        e.open && super::owing::is_asked(e) && e.owner.as_deref() == Some(owner) && e.last_to == now
    }) {
        if !entry.asked.iter().any(|asked| asked == id) {
            entry.asked.push(id.to_owned());
        }
        let since = (now - ASKED_FOR) * 1000;
        entry
            .asked
            .retain(|asked| crate::mesh::stamped_at(asked).is_none_or(|ms| ms >= since));
        let over = entry.asked.len().saturating_sub(ASKED_AT_MOST);
        entry.asked.drain(..over);
    }
}

/// Undoes the hour's filing for each of `unsent`, owners whose question
/// never went, back to `before`, the ledger as the hour read it: each of
/// their lines is as it was, so the next hour files and asks it again,
/// rather than holding it open as asked by a question nobody had. The
/// lines of owners already asked stand, and so does all else the hour
/// did. A filing only changes lines in place or adds them after, so a
/// line keeps its place.
pub fn unfile(ledger: &mut Vec<Entry>, before: &[Entry], unsent: &[&str]) {
    let theirs = |e: &Entry| e.owner.as_deref().is_some_and(|o| unsent.contains(&o));
    let mut at = 0;
    ledger.retain(|e| {
        at += 1;
        at <= before.len() || !theirs(e)
    });
    for (line, was) in ledger.iter_mut().zip(before) {
        if theirs(&*line) || theirs(was) {
            line.clone_from(was);
        }
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
        // never went, so this one doesn't hold it. hour:e came back, asked
        // in questions before, and keeps those of the last two days beside
        // this, however many: a line is asked again for each new name.
        let mut ledger = vec![
            entry("hour:a", "ABC123", NOW, "drag"),
            entry("hour:b", "ABC123", NOW - 3600, "drag"),
            entry("hour:c", "DEF456", NOW, "drag"),
            entry("security:d", "ABC123", NOW, "must"),
        ];
        let mut red = entry("hour:f", "ABC123", NOW, "drag");
        red.red = true;
        ledger.push(red);
        let mut back = entry("hour:e", "ABC123", NOW, "drag");
        let stamped = |at: i64| format!("{}-CADENCE-1-a", at * 1000);
        back.asked = vec![
            stamped(NOW - 2 * crate::DAY - 1),
            stamped(NOW - crate::DAY),
            stamped(NOW - 7200),
            stamped(NOW - 3600),
            stamped(NOW - 1800),
            "unstamped".to_owned(),
        ];
        ledger.push(back);
        mark_asked(&mut ledger, "ABC123", NOW, "the-question");
        mark_asked(&mut ledger, "ABC123", NOW, "the-question");
        let asked: Vec<Vec<&str>> = ledger
            .iter()
            .map(|e| e.asked.iter().map(String::as_str).collect())
            .collect();
        let none: Vec<&str> = Vec::new();
        assert_eq!(
            asked,
            [
                vec!["the-question"],
                none.clone(),
                none.clone(),
                none,
                vec!["the-question"],
                vec![
                    "1799913600000-CADENCE-1-a",
                    "1799992800000-CADENCE-1-a",
                    "1799996400000-CADENCE-1-a",
                    "1799998200000-CADENCE-1-a",
                    "unstamped",
                    "the-question"
                ]
            ],
            "a line not filed for that owner this hour or not a drag took the id, a red \
             line or one come back didn't, or one kept an id past two days or dropped a \
             younger one"
        );
    }

    #[test]
    fn a_question_that_never_went_leaves_its_owner_s_lines_as_the_hour_found_them() {
        let mut closed = entry("hour:b", "DEF456", NOW - 7200, "drag");
        closed.open = false;
        let before = vec![
            entry("hour:a", "ABC123", NOW - 7200, "drag"),
            closed.clone(),
        ];
        // The hour asked ABC123 of a new line, and filed DEF456's line
        // again and a new one, then DEF456's question failed.
        let mut ledger = before.clone();
        ledger[0].last_to = NOW;
        ledger[1].open = true;
        ledger[1].last_to = NOW;
        ledger.push(entry("hour:c", "ABC123", NOW, "drag"));
        ledger.push(entry("hour:d", "DEF456", NOW, "drag"));
        let asked = ledger.clone();
        unfile(&mut ledger, &before, &["DEF456"]);
        assert_eq!(
            ledger,
            [asked[0].clone(), closed, asked[2].clone()],
            "an owner never asked kept the hour's filing, so their lines read as asked, \
             or an owner asked lost theirs"
        );
    }
}
