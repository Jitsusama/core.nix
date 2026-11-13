//! A search of every notice, answered here so the shell matches none
//! itself (Joel, 2026-11-10: "the daemons need to be the data layer, and
//! they need to serve the launcher"): what's typed in, the keys of the
//! notices' rows it finds out, in the order the launcher lists them. Each
//! is found by its name, else by anything it said, as the launcher's
//! PastNotices.qml found them, and ranked by how well, then newest first,
//! matched by the daemons' one matcher, Fuzzy.js's (ready-rows' fuzzy).
//! Matching in Qt's engine was most of what a key spent on notices, 64 of
//! the 89 ms "ca" took on Joel's own (2026-11-11). A found notice whose
//! row the notices place doesn't serve comes with its row, as ready-rows.md
//! says a search's answer does, so the place need serve only a page: all
//! 1,000 of Joel's rows were held for searches alone, about 11 MB of the
//! shell's heap kept live in the lab (2026-11-12). Only those come, and
//! only for the first of those found, since one letter finds nearly every
//! notice, about 1.5 MB to send a key.

use std::collections::BTreeMap;

use ready_rows::fuzzy::{Item, Typed};
use serde_json::{Value, json};

use crate::body;
use crate::notice::Notice;
use crate::row;

/// How many of the notices an answer lists, first first, come with their
/// row where the place doesn't serve it: about three screens of the
/// launcher's list, some 72 KB. Every found one came before, 1.46 MB for
/// "s" and 577 KB for "st" on Joel's 1,000 notices (2026-11-12), which
/// the shell parsed on its main thread, reading notices 103 to 116 ms
/// after his keys. One found further down still lists where the place
/// serves its row, as the newest hundred are.
const FOUND_ROWS: usize = 30;

/// How many of the notices an answer lists at most, first first: six
/// screens of the launcher's list, past anything read down to. The shell
/// makes a copy of every row listed, and every one found was listed
/// before: Joel's "s" found 971 of his 1,000, and an opening's five short
/// searches made 933 rows, as "s" took 747 ms to his screen (2026-11-12).
const FOUND_LISTED: usize = 60;

/// One notice's texts made ready to match, kept while what it said is the
/// same, so a key matches and makes nothing ready.
struct Ready {
    summary: String,
    body: String,
    name: Item,
    said: Item,
}

/// Said by what it was made from, since a matcher's ready text says
/// nothing of its own.
impl std::fmt::Debug for Ready {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ready")
            .field("summary", &self.summary)
            .field("body", &self.body)
            .finish_non_exhaustive()
    }
}

impl Ready {
    fn of(notice: &Notice) -> Ready {
        let (name, said) = row::searched(notice, &body::read(&notice.body));
        Ready {
            summary: notice.summary.clone(),
            body: notice.body.clone(),
            name: Item::of(&name),
            said: Item::of(&spaced(&said)),
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
    /// what's `typed` finds, best first and newest first among equals, the
    /// first `FOUND_LISTED` of them, with their grade, as the launcher's searches answer one. Every
    /// notice found is graded 0, as the launcher's matching grades every
    /// match now. Beside them, by key, the row of each found that's among
    /// `unserved`, the notices place's rows its watch doesn't serve, and
    /// how many were `found` in all, so the launcher can say when more were
    /// found than it lists.
    pub fn answer(
        &mut self,
        notices: &[Notice],
        typed: &str,
        unserved: &BTreeMap<String, Value>,
    ) -> Value {
        let words = Typed::of(typed);
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
            .take(FOUND_LISTED)
            .map(|(_, _, id)| format!("row:{}:{id}", row::PLACE))
            .collect();
        let rows: serde_json::Map<String, Value> = keys
            .iter()
            .take(FOUND_ROWS)
            .filter_map(|key| Some((key.clone(), unserved.get(key)?.clone())))
            .collect();
        json!({
            "keys": keys, "rows": rows, "found": found.len(),
            "grade": if keys.is_empty() { 3 } else { 0 }, "words": typed,
        })
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

    fn none() -> BTreeMap<String, Value> {
        BTreeMap::new()
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
        let answer = searched.answer(&notices, "cab", &none());
        assert_eq!(
            keys(&answer),
            ["row:notices:1", "row:notices:3", "row:notices:2"],
            "its name whole first, then the newer of two that said it alike: {answer}"
        );
        assert_eq!(answer["grade"], 0, "a match is graded 0: {answer}");
        assert_eq!(
            keys(&searched.answer(&notices, "bart cab", &none())),
            ["row:notices:2"],
            "every word has to land, in its name or in what it said"
        );
        let nothing = searched.answer(&notices, "zebra", &none());
        assert!(keys(&nothing).is_empty(), "nothing found: {nothing}");
        assert_eq!(
            nothing["grade"], 3,
            "and graded as nothing found: {nothing}"
        );
        assert_eq!(nothing["found"], 0, "and says none was: {nothing}");
    }

    #[test]
    fn a_notice_found_comes_with_its_row_only_where_the_watch_never_served_it() {
        let notices = [
            notice(3, "Cabinet delivered", ""),
            notice(2, "Bart replied", "about the cab"),
            notice(1, "Lunch", "at noon"),
        ];
        let unserved: BTreeMap<String, Value> = [2, 1]
            .into_iter()
            .map(|id| {
                (
                    format!("row:notices:{id}"),
                    json!({ "name": format!("row {id}") }),
                )
            })
            .collect();
        let mut searched = Searched::default();
        let answer = searched.answer(&notices, "cab", &unserved);
        assert_eq!(
            answer["rows"],
            json!({ "row:notices:2": { "name": "row 2" } }),
            "the unserved one found comes, and not the served or the unfound: {answer}"
        );
        let all_served = searched.answer(&notices, "cab", &none());
        assert_eq!(
            all_served["rows"],
            json!({}),
            "with every row served, none comes: {all_served}"
        );
    }

    #[test]
    fn an_answer_lists_six_screens_with_the_rows_of_its_first_page_alone() {
        let found = u32::try_from(FOUND_LISTED).expect("six screens fit") + 10;
        let notices: Vec<Notice> = (1..=found).map(|id| notice(id, "Standup", "")).collect();
        let unserved: BTreeMap<String, Value> = (1..=found)
            .map(|id| (format!("row:notices:{id}"), json!({ "id": id })))
            .collect();
        let mut searched = Searched::default();
        let answer = searched.answer(&notices, "s", &unserved);
        let listed = keys(&answer);
        let newest: Vec<String> = (11..=found)
            .rev()
            .map(|id| format!("row:notices:{id}"))
            .collect();
        assert_eq!(
            listed, newest,
            "six screens of those found are listed, newest first"
        );
        assert_eq!(
            answer["found"], found,
            "and it says how many were found, listed or not"
        );
        let rows = answer["rows"].as_object().cloned().unwrap_or_default();
        assert_eq!(
            rows.len(),
            FOUND_ROWS,
            "only a page of rows comes: {} of {}",
            rows.len(),
            listed.len()
        );
        assert!(
            listed[..FOUND_ROWS]
                .iter()
                .all(|key| rows.contains_key(*key)),
            "and they're the first listed, best and newest: {:?}",
            rows.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_notice_said_again_is_matched_as_it_says_now_and_one_gone_is_let_go() {
        let mut searched = Searched::default();
        searched.answer(&[notice(1, "Build", "it failed")], "fail", &none());
        let again = searched.answer(&[notice(1, "Build", "it passed")], "fail", &none());
        assert!(
            keys(&again).is_empty(),
            "what it said before isn't found: {again}"
        );
        searched.answer(&[], "pass", &none());
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
