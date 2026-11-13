//! A search of every quest, open and ended, answered here so the shell
//! need hold only the open ones (Y2YBTS's ruling, 2026-11-13): what's
//! typed in, the keys of the quests it finds out, open before ended, then
//! best first, then in Quests' order, as Quests.qml ranked them. Each is
//! found by its name or by its id and kind, matched by the daemons' one
//! matcher, Fuzzy.js's (ready-rows' fuzzy). An ended quest's row isn't
//! served, so it comes with the answer, as ready-rows.md says a search's
//! answer does: held whole, Joel's 1,007 ended quests were most of the
//! 1.6 MB of rows every collection of the shell's heap walked.

use ready_rows::fuzzy::{Item, Typed};
use serde_json::{Map, Value, json};

use crate::served::{Made, PLACE};

/// How many quests an answer lists at most, first first: six screens of
/// the launcher's list, past anything read down to, as notices' search
/// lists. One letter finds most of Joel's 1,237, and the shell makes a
/// row of each listed, an ended one from the answer.
pub const FOUND_LISTED: usize = 60;

/// One quest made ready to match, once a making: its key, whether it's
/// open, its name and its id and kind, and an ended one's row.
struct Ready {
    key: String,
    open: bool,
    name: Item,
    also: Item,
    row: Option<Value>,
}

/// Every quest made ready to match, in Quests' order, open before ended.
#[derive(Default)]
pub struct Searched {
    ready: Vec<Ready>,
}

impl Searched {
    /// Every quest of a making, ready to match.
    #[must_use]
    pub fn of(made: &Made) -> Searched {
        let ready = |row: &Value, open: bool| Ready {
            key: text(&row["key"]).to_owned(),
            open,
            name: Item::of(text(&row["name"])),
            also: Item::of(text(&row["held"]["also"])),
            row: (!open).then(|| row.clone()),
        };
        let open = made.open.iter().map(|row| ready(row, true));
        let ended = made.ended.iter().map(|row| ready(row, false));
        Searched {
            ready: open.chain(ended).collect(),
        }
    }

    /// The answer to a search ask, without its id: of Quests, what its
    /// `words` find, at most `most` where it says so, else
    /// [`FOUND_LISTED`], with their grade, as the launcher's searches
    /// answer one, every quest found graded 0, as its matching grades
    /// every match now. Beside them, by key, the row of each ended one
    /// listed, and how many were found in all.
    #[must_use]
    pub fn answer(&self, said: &Value) -> Value {
        let words = text(&said["words"]);
        let place = said["place"].as_str().unwrap_or(PLACE);
        if place != PLACE {
            return json!({
                "words": words,
                "error": {"code": "unknown-place", "message": format!("no place {place:?} here")},
            });
        }
        let most = said["most"]
            .as_u64()
            .and_then(|most| usize::try_from(most).ok())
            .filter(|most| *most > 0)
            .unwrap_or(FOUND_LISTED);
        let typed = Typed::of(words);
        let mut found: Vec<(bool, i32, &Ready)> = self
            .ready
            .iter()
            .filter_map(|one| {
                let score = one.name.score(&typed).max(one.also.score(&typed))?;
                Some((one.open, score, one))
            })
            .collect();
        // A stable sort, so those alike keep Quests' order.
        found.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
        let listed = &found[..found.len().min(most)];
        let keys: Vec<&str> = listed.iter().map(|(_, _, one)| one.key.as_str()).collect();
        let rows: Map<String, Value> = listed
            .iter()
            .filter_map(|(_, _, one)| Some((one.key.clone(), one.row.clone()?)))
            .collect();
        json!({
            "keys": keys, "rows": rows, "found": found.len(),
            "grade": if keys.is_empty() { 3 } else { 0 }, "words": words,
        })
    }
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, name: &str, open: bool) -> Value {
        json!({"key": format!("row:quests:{id}"), "name": name, "unfinished": open,
            "held": {"also": format!("{id} quest")}})
    }

    fn searched() -> Searched {
        Searched::of(&Made {
            open: vec![
                row("QEST-20261001-LAPTOP", "Bootstrap the laptop", true),
                row("QEST-20261003-PROMPT", "Retune the prompt", true),
            ],
            ended: vec![
                row("QEST-20250105-BANNER", "Tune the banner", false),
                row("QEST-20250106-THEMES", "The themes", false),
            ],
            said: String::new(),
        })
    }

    fn asked(words: &str) -> Value {
        searched().answer(&json!({"ask": "search", "place": "quests", "words": words}))
    }

    #[test]
    fn an_ended_quest_is_found_by_its_name_and_comes_with_its_row() {
        let answer = asked("banner");
        assert_eq!(answer["keys"], json!(["row:quests:QEST-20250105-BANNER"]));
        assert_eq!(answer["grade"], 0);
        assert_eq!(answer["found"], 1);
        assert_eq!(
            answer["rows"]["row:quests:QEST-20250105-BANNER"]["name"], "Tune the banner",
            "its row, since the watch never served it: {answer}"
        );
    }

    #[test]
    fn the_open_come_first_and_only_the_ended_bring_their_rows() {
        // Each answers "the" alike, so each keeps Quests' order.
        let answer = asked("the");
        assert_eq!(
            answer["keys"],
            json!([
                "row:quests:QEST-20261001-LAPTOP",
                "row:quests:QEST-20261003-PROMPT",
                "row:quests:QEST-20250105-BANNER",
                "row:quests:QEST-20250106-THEMES"
            ]),
            "open first, then Quests' order among the alike: {answer}"
        );
        let rows: Vec<&String> = answer["rows"]
            .as_object()
            .map(|rows| rows.keys().collect())
            .unwrap_or_default();
        assert_eq!(
            rows,
            [
                "row:quests:QEST-20250105-BANNER",
                "row:quests:QEST-20250106-THEMES"
            ],
            "the served open ones come without theirs"
        );
    }

    #[test]
    fn the_open_found_lead_the_ended_and_then_the_best_found_comes_first() {
        let searched = Searched::of(&Made {
            open: vec![row("QEST-20261101-AGAIN", "Tuned again", true)],
            ended: vec![
                row("QEST-20250101-TUNEDUP", "Tuned up", false),
                row("QEST-20250102-TUNE", "Tune", false),
            ],
            said: String::new(),
        });
        // "tune" only begins "Tuned", but is "Tune" whole, which ranks it.
        let answer = searched.answer(&json!({"place": "quests", "words": "tune"}));
        assert_eq!(
            answer["keys"],
            json!([
                "row:quests:QEST-20261101-AGAIN",
                "row:quests:QEST-20250102-TUNE",
                "row:quests:QEST-20250101-TUNEDUP"
            ]),
            "the open one first, though Tune answers better, which leads the ended though \
             Quests lists it second: {answer}"
        );
    }

    #[test]
    fn a_quest_is_found_by_its_id_and_nothing_found_is_graded_so() {
        let answer = asked("20261003");
        assert_eq!(answer["keys"], json!(["row:quests:QEST-20261003-PROMPT"]));
        let none = asked("zzz");
        assert_eq!((&none["keys"], &none["grade"]), (&json!([]), &json!(3)));
    }

    #[test]
    fn an_answer_lists_no_more_than_asked_but_says_how_many_it_found() {
        let answer = searched().answer(&json!({"place": "quests", "words": "the", "most": 1}));
        assert_eq!(answer["keys"], json!(["row:quests:QEST-20261001-LAPTOP"]));
        assert_eq!(answer["found"], 4);
        assert_eq!(
            answer["rows"],
            json!({}),
            "the ended ones past it bring nothing"
        );
    }

    #[test]
    fn another_place_is_refused() {
        let answer = searched().answer(&json!({"place": "agents", "words": "a"}));
        assert_eq!(answer["error"]["code"], "unknown-place", "{answer}");
    }
}
