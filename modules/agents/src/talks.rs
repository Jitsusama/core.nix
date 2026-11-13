//! The talks Joel has had with a session, as rows inside its step in
//! Agents' top level, newest first, after its acts and the sessions under
//! it: his words, and beside them how it went in one word that changes in
//! place (delivered, reading, replying, replied, not delivered), the reply
//! in the talk's own drawer, and a step into You Said, How It Went and
//! What Came of It, each copied by Return. They were the shell's
//! (Orchestrators.qml's talksOf) while it laid Agents' tree, and the
//! served top level lost them (Y2YBTS, 2026-11-13), so they're folded
//! here from the node's `talk` records as the shell folded them.

use std::collections::HashMap;

use serde_json::{Value, json};

use crate::row::Words;

/// One talk folded from its lines, as the harness's talk.ts writes them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Talk {
    pub id: String,
    /// When it was delivered, as its line says it, so the newest sorts last
    /// as text.
    pub at: String,
    /// What Joel said.
    pub words: String,
    /// The last stage it reached.
    pub stage: String,
    /// What it's read after, while it's reading.
    pub after: String,
    pub reply: String,
    /// Why it wasn't delivered.
    pub why: String,
    /// What came of it.
    pub came: String,
}

/// One talk record's lines folded into what its rows say.
pub fn folded(record: &Value) -> Talk {
    let mut talk = Talk {
        id: text(&record["id"]),
        ..Talk::default()
    };
    for line in record["lines"].as_array().into_iter().flatten() {
        let stage = line["stage"].as_str().unwrap_or("");
        match stage {
            "delivered" => {
                if line["at"].is_string() {
                    talk.at = text(&line["at"]);
                }
                talk.words = text(&line["text"]);
            }
            "reading" => talk.after = text(&line["after"]),
            "replying" | "replied" => {
                if line["text"].is_string() {
                    talk.reply = text(&line["text"]);
                }
            }
            "failed" => talk.why = text(&line["why"]),
            // What came of it is said beside how it went, never as it.
            "came" => {
                talk.came = text(&line["what"]);
                continue;
            }
            _ => {}
        }
        if !stage.is_empty() {
            stage.clone_into(&mut talk.stage);
        }
    }
    talk
}

/// Each session's talks that said something, by its address in capitals,
/// the newest first.
pub fn by_address<'a>(records: impl Iterator<Item = &'a Value>) -> HashMap<String, Vec<Talk>> {
    let mut by: HashMap<String, Vec<Talk>> = HashMap::new();
    for record in records {
        let talk = folded(record);
        if talk.words.is_empty() {
            continue;
        }
        by.entry(text(&record["address"]).to_uppercase())
            .or_default()
            .push(talk);
    }
    for talks in by.values_mut() {
        talks.sort_by(|a, b| b.at.cmp(&a.at));
    }
    by
}

/// How a talk went, in one word, with what it's read after while it reads.
fn went(talk: &Talk, words: &Words) -> String {
    let said = words.talk_stages.word(&talk.stage);
    let said = if said.is_empty() { &talk.stage } else { said };
    if talk.stage == "reading" && !talk.after.is_empty() {
        format!("{said}, {}", talk.after)
    } else {
        said.to_owned()
    }
}

/// The mark a stage wears, hollow to full, as the queue's do.
fn mark(stage: &str) -> &'static str {
    match stage {
        "reading" | "replying" => "active",
        "replied" => "complete",
        "failed" => "failed",
        _ => "pending",
    }
}

/// The rows of a session's talks inside the row keyed `parent`, `depth`
/// steps in, each a thing stepping into its parts, then those parts.
pub fn rows(talks: &[Talk], parent: &str, place: &str, depth: usize, words: &Words) -> Vec<Value> {
    let mut rows = Vec::new();
    for talk in talks {
        let key = format!("{parent}:talk:{}", talk.id);
        let how = went(talk, words);
        let reads = if talk.reply.is_empty() {
            &talk.why
        } else {
            &talk.reply
        };
        rows.push(json!({
            "key": key, "thing": format!("talk:{}", talk.id), "place": place,
            "parent": parent, "kind": "talk", "depth": depth, "name": talk.words,
            "glyph": mark(&talk.stage), "hue": "voice", "side": [{"said": how}],
            "reads": reads, "drawer": {"title": talk.words, "body": reads},
            "acts": [{"key": "verb:more", "step": "rows"}],
        }));
        let part = |id: &str, side: &str, copy: &str| {
            json!({
                "key": format!("{key}:{id}"), "thing": format!("talk:{}:{id}", talk.id),
                "place": place, "parent": key, "kind": "fact", "depth": depth + 1,
                "name": words.talk_parts.word(id), "hue": "voice", "side": [{"said": side}],
                "acts": [{"key": "verb:copy", "copy": copy}],
            })
        };
        rows.push(part("yours", &talk.words, &talk.words));
        rows.push(part("went", &how, reads));
        if !talk.came.is_empty() {
            rows.push(part("came", &talk.came, &talk.came));
        }
    }
    rows
}

fn text(value: &Value) -> String {
    value.as_str().unwrap_or("").to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::said::LAUNCHERS;

    fn record(address: &str, id: &str, lines: &Value) -> Value {
        json!({"address": address, "id": id, "lines": lines})
    }

    fn served(talks: &[Talk]) -> Vec<Value> {
        let words = Words::from_vocabulary(LAUNCHERS).expect("the launcher's words read");
        rows(talks, "row:agents-top:LIGHT", "agents-top", 1, &words)
    }

    #[test]
    fn a_talk_is_folded_from_its_lines_as_the_harness_writes_them() {
        let talk = folded(&record(
            "light",
            "t1",
            &json!([
                {"at": "2026-10-26T04:00:00Z", "stage": "delivered", "text": "keep watch"},
                {"stage": "reading", "after": "a turn"},
                {"stage": "replying", "text": "Watch"},
                {"at": "2026-10-26T04:00:05Z", "stage": "replied", "text": "Watching."},
                {"stage": "came", "what": "a lamp lit"},
                {"stage": "a stage to come"}
            ]),
        ));
        assert_eq!(
            talk,
            Talk {
                id: "t1".into(),
                at: "2026-10-26T04:00:00Z".into(),
                words: "keep watch".into(),
                stage: "a stage to come".into(),
                after: "a turn".into(),
                reply: "Watching.".into(),
                why: String::new(),
                came: "a lamp lit".into(),
            },
            "each line in turn, what came beside how it went"
        );
    }

    #[test]
    fn each_sessions_talks_are_its_newest_first_and_only_those_that_said_something() {
        let said = |at: &str| json!([{"at": at, "stage": "delivered", "text": "words"}]);
        let records = [
            record("light", "old", &said("2026-10-26T01:00:00Z")),
            record("LIGHT", "new", &said("2026-10-26T02:00:00Z")),
            record(
                "light",
                "empty",
                &json!([{"stage": "delivered", "text": ""}]),
            ),
            record("reef", "r", &said("2026-10-26T03:00:00Z")),
        ];
        let by = by_address(records.iter());
        let ids = |address: &str| -> Vec<String> {
            by[address].iter().map(|talk| talk.id.clone()).collect()
        };
        assert_eq!(
            ids("LIGHT"),
            ["new", "old"],
            "by address in capitals, newest first"
        );
        assert_eq!(ids("REEF"), ["r"], "each session its own");
    }

    #[test]
    fn a_talk_says_how_it_went_its_reply_in_its_drawer_and_steps_into_its_parts() {
        let talk = folded(&record(
            "light",
            "t1",
            &json!([
                {"at": "2026-10-26T04:00:00Z", "stage": "delivered", "text": "keep watch tonight"},
                {"stage": "replied", "text": "Watching. All quiet."}
            ]),
        ));
        let rows = served(&[talk]);
        let thing = &rows[0];
        assert_eq!(
            (
                &thing["parent"],
                &thing["name"],
                &thing["side"],
                &thing["glyph"]
            ),
            (
                &json!("row:agents-top:LIGHT"),
                &json!("keep watch tonight"),
                &json!([{"said": "replied"}]),
                &json!("complete")
            ),
            "inside its session, his words and how it went: {thing}"
        );
        assert_eq!(
            thing["drawer"],
            json!({"title": "keep watch tonight", "body": "Watching. All quiet."}),
            "its reply in its own drawer"
        );
        assert_eq!(thing["acts"][0]["step"], "rows", "it steps into its parts");
        let parts: Vec<(&str, &str, &Value)> = rows[1..]
            .iter()
            .map(|part| {
                (
                    part["name"].as_str().unwrap_or(""),
                    part["parent"].as_str().unwrap_or(""),
                    &part["acts"][0]["copy"],
                )
            })
            .collect();
        let key = "row:agents-top:LIGHT:talk:t1";
        assert_eq!(
            parts,
            [
                ("You Said", key, &json!("keep watch tonight")),
                ("How It Went", key, &json!("Watching. All quiet."))
            ],
            "his words and how it went, each copied, nothing came of it yet"
        );
    }

    #[test]
    fn a_talk_not_delivered_says_why_and_one_reading_says_after_what() {
        let failed = Talk {
            id: "f".into(),
            words: "go".into(),
            stage: "failed".into(),
            why: "its inbox is gone".into(),
            ..Talk::default()
        };
        let reading = Talk {
            id: "r".into(),
            words: "wait".into(),
            stage: "reading".into(),
            after: "this turn".into(),
            ..Talk::default()
        };
        let rows = served(&[failed, reading]);
        assert_eq!(
            (&rows[0]["side"], &rows[0]["drawer"]["body"]),
            (
                &json!([{"said": "not delivered"}]),
                &json!("its inbox is gone")
            ),
            "not delivered, and why"
        );
        assert_eq!(
            rows[3]["side"],
            json!([{"said": "reading, this turn"}]),
            "read after what it's on"
        );
    }
}
