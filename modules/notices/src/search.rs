//! A search of every notice, answered here so the shell matches none
//! itself (Joel, 2026-11-10: "the daemons need to be the data layer, and
//! they need to serve the launcher"): what's typed in, the keys of the
//! notices' rows it finds out, in the order the launcher lists them. Each
//! is found by its name, else by anything it said, as the launcher's
//! PastNotices.qml found them, and ranked by how well, then newest first.
//! Matching in Qt's engine was most of what a key spent on notices, 64 of
//! the 89 ms "ca" took on Joel's own (2026-11-11).

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::body;
use crate::fuzzy::{Prepared, Words};
use crate::notice::Notice;
use crate::row;

/// One notice's texts made ready to match, kept while what it said is the
/// same, so a key matches and makes nothing ready.
#[derive(Debug)]
struct Ready {
    summary: String,
    body: String,
    name: Prepared,
    said: Prepared,
}

impl Ready {
    fn of(notice: &Notice) -> Ready {
        let (name, said) = row::searched(notice, &body::read(&notice.body));
        Ready {
            summary: notice.summary.clone(),
            body: notice.body.clone(),
            name: Prepared::of(&name),
            said: Prepared::of(&spaced(&said)),
        }
    }

    fn holds(&self, notice: &Notice) -> bool {
        self.summary == notice.summary && self.body == notice.body
    }
}

/// A text with each run of spacing one space, as the launcher's `also`
/// was what a notice said.
fn spaced(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut spacing = false;
    for c in text.chars() {
        let space = c.is_whitespace() || c == '\u{feff}';
        if !(space && spacing) {
            out.push(if space { ' ' } else { c });
        }
        spacing = space;
    }
    out
}

/// Every notice's texts made ready, by id.
#[derive(Debug, Default)]
pub struct Searched {
    ready: BTreeMap<u32, Ready>,
}

impl Searched {
    /// The keys of the rows of `notices`, in the order they're held, that
    /// what's `typed` finds, best first and newest first among equals,
    /// with their grade, as the launcher's searches answer one. Every
    /// notice found is graded 0, as the launcher's matching grades every
    /// match now.
    pub fn answer(&mut self, notices: &[Notice], typed: &str) -> Value {
        let words = Words::of(typed);
        let mut ready = std::mem::take(&mut self.ready);
        let mut found: Vec<(i32, i64, u32)> = Vec::new();
        for notice in notices {
            let one = match ready.remove(&notice.id) {
                Some(kept) if kept.holds(notice) => kept,
                _ => Ready::of(notice),
            };
            if let Some(score) = one.name.score(&words).or_else(|| one.said.score(&words)) {
                found.push((score, notice.at, notice.id));
            }
            self.ready.insert(notice.id, one);
        }
        found.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
        let keys: Vec<String> = found
            .iter()
            .map(|(_, _, id)| format!("row:{}:{id}", row::PLACE))
            .collect();
        json!({ "keys": keys, "grade": if keys.is_empty() { 3 } else { 0 }, "words": typed })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notice(id: u32, summary: &str, body: &str) -> Notice {
        serde_json::from_value(
            json!({ "id": id, "at": 1_000 * i64::from(id), "app": "Slack",
            "summary": summary, "body": body }),
        )
        .expect("a notice")
    }

    fn keys(answer: &Value) -> Vec<&str> {
        answer["keys"]
            .as_array()
            .map(|keys| keys.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default()
    }

    #[test]
    fn a_notice_is_found_by_its_name_else_by_what_it_said_best_then_newest_first() {
        let notices = [
            notice(3, "Carla shared a document", "about the cabinet"),
            notice(2, "Bart replied", "in the cabinet thread"),
            notice(1, "Cab", "starts in five minutes"),
        ];
        let mut searched = Searched::default();
        let answer = searched.answer(&notices, "cab");
        assert_eq!(
            keys(&answer),
            ["row:notices:1", "row:notices:3", "row:notices:2"],
            "its name whole first, then the newer of two that said it alike: {answer}"
        );
        assert_eq!(answer["grade"], 0, "a match is graded 0: {answer}");
        assert_eq!(
            keys(&searched.answer(&notices, "bart cab")),
            ["row:notices:2"],
            "every word has to land, in its name or in what it said"
        );
        let none = searched.answer(&notices, "zebra");
        assert!(keys(&none).is_empty(), "nothing found: {none}");
        assert_eq!(none["grade"], 3, "and graded as nothing found: {none}");
    }

    #[test]
    fn a_notice_said_again_is_matched_as_it_says_now_and_one_gone_is_let_go() {
        let mut searched = Searched::default();
        searched.answer(&[notice(1, "Build", "it failed")], "fail");
        let again = searched.answer(&[notice(1, "Build", "it passed")], "fail");
        assert!(
            keys(&again).is_empty(),
            "what it said before isn't found: {again}"
        );
        searched.answer(&[], "pass");
        assert!(searched.ready.is_empty(), "a notice gone is let go");
    }

    #[test]
    fn what_it_said_is_matched_with_its_spacing_one_space() {
        assert_eq!(
            spaced("a  b\n\tc "),
            "a b c ",
            "runs of spacing are one space"
        );
    }
}
