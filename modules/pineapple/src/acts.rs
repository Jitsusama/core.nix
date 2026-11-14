//! Joel's presses on the rows src/tell.rs makes, taken back from the
//! launcher: `{"ask": <verb>, "key": <row's key>}`. A press acts only on a
//! row served now, and only with a verb that row offers, so a row gone
//! or changed since it was drawn is refused rather than acted on, and
//! nothing in the ask but the row's key and verb is believed: what a
//! press does comes from the row as it stands on disk. Who may press is
//! src/peer.rs's to say, before this is asked.
//!
//! - `resume` marks a stop resumed by Joel, as `pineapple resume` does.
//! - `widen` makes a stop reach everything, keeping what it reached.
//! - `stop` makes an alert's saying a stop of its own, reaching what a
//!   stop said there would have, as if the safe word had been said.
//! - `approve` resumes what someone else's resume reaches.
//! - `dismiss` only hides the row, until something new is said there.
//!
//! Every press is written to acted.jsonl, which hides what it answered.

use std::fs;
use std::io;
use std::path::Path;

use serde_json::{Value, json};

use crate::record;
use crate::resume;
use crate::tell;
use crate::vocabulary::Words;

/// The asks a press makes.
pub const VERBS: [&str; 5] = ["resume", "widen", "stop", "approve", "dismiss"];

/// Who every press is written down as made by.
const BY: &str = "joel";

/// Why a press was refused, by the interface's error code and a message.
#[derive(Debug, PartialEq, Eq)]
pub struct Refused {
    /// The error's code.
    pub code: &'static str,
    /// What went wrong, for the journal and the launcher.
    pub message: String,
}

impl Refused {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// The answer the socket gives for it.
    #[must_use]
    pub fn answer(&self) -> Value {
        json!({"error": {"code": self.code, "message": self.message}})
    }
}

/// Takes the press `said` under the state folder `state` at `now`
/// (milliseconds since the epoch), answering what it did.
///
/// # Errors
///
/// A press on no row served now, with a verb its row doesn't offer, or
/// one whose records couldn't be written.
pub fn press(state: &Path, said: &Value, now: u64, words: &Words) -> Result<Value, Refused> {
    let verb = said["ask"].as_str().unwrap_or_default();
    let key = said["key"].as_str().unwrap_or_default();
    let rows = tell::rows(&tell::kept(state), now, words);
    let row = rows
        .iter()
        .find(|row| row["key"] == key && row["shadow"] == false)
        .ok_or_else(|| Refused::new("gone", format!("no row {key:?} is offered now")))?;
    let offered = row["acts"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|act| act["ask"]["ask"] == verb);
    if !offered {
        return Err(Refused::new(
            "not-offered",
            format!("{key:?} doesn't offer {verb:?}"),
        ));
    }
    let stops = state.join("stops");
    let failed = |error: io::Error| Refused::new("failed", error.to_string());
    let done = match verb {
        "resume" => {
            let stop = row["stop"].as_str().unwrap_or_default();
            json!({"resumed": resume::resume(&stops, stop, now, BY).map_err(failed)?})
        }
        "widen" => {
            let stop = row["stop"].as_str().unwrap_or_default();
            widen(&stops, stop, now).map_err(failed)?;
            json!({"widened": stop})
        }
        "stop" => {
            let id = row["saying"].as_str().unwrap_or_default();
            record::keep(&stops, id, &stopped(row, now)).map_err(failed)?;
            json!({"stopped": id})
        }
        "approve" => json!({"resumed": approve(&stops, row, now).map_err(failed)?}),
        _ => json!({"dismissed": key}),
    };
    record::append(
        &state.join("acted.jsonl"),
        &json!({"at": now, "act": verb, "key": key, "by": BY}),
    )
    .map_err(failed)?;
    Ok(done)
}

/// A stop of Joel's making from an alert's row: its saying, reaching what
/// a stop said there would have.
fn stopped(row: &Value, now: u64) -> Value {
    json!({
        "id": row["saying"],
        "shadow": false,
        "said_by": row["said_by"],
        "where": row["link"],
        "words": row["words"],
        "said_at": row["said_at"],
        "heard_at": now,
        "by_joel": false,
        "stopped_by": BY,
        "reaches": row["reaches"],
        "fell_back": row["fell_back"],
        "resumed": Value::Null,
    })
}

/// Resumes what a resume asked for reaches: each reach its row names, and
/// every stop where that's everything.
fn approve(stops: &Path, row: &Value, now: u64) -> io::Result<Vec<String>> {
    let mut resumed = Vec::new();
    for reach in row["reaches"].as_array().into_iter().flatten() {
        let scope = match reach.as_str().unwrap_or_default() {
            "everything" => "all",
            reach => reach,
        };
        resumed.extend(resume::resume(stops, scope, now, BY)?);
    }
    Ok(resumed)
}

/// Makes the stop `id` reach everything, keeping what it reached before
/// and when it was widened, written whole and renamed into place.
fn widen(stops: &Path, id: &str, now: u64) -> io::Result<()> {
    let stop = resume::standing(stops)
        .into_iter()
        .find(|stop| stop.id == id)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "that stop no longer stands"))?;
    let mut record = stop.record;
    if let Some(fields) = record.as_object_mut() {
        let was = fields.get("reaches").cloned().unwrap_or(Value::Null);
        fields.insert("was_reaching".to_owned(), was);
        fields.insert("reaches".to_owned(), json!(["everything"]));
        fields.insert("widened".to_owned(), json!({"at": now, "by": BY}));
    }
    let written = stop.file.with_extension("new");
    fs::write(&written, format!("{record:#}\n"))?;
    fs::rename(&written, &stop.file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn state(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("pineapple-acts-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("stops")).expect("a scratch state folder");
        dir
    }

    fn stop(dir: &Path, id: &str, reaches: &[&str], shadow: bool) {
        let record = json!({"id": id, "shadow": shadow, "reaches": reaches, "heard_at": 1,
            "fell_back": false, "by_joel": false, "resumed": null});
        fs::write(
            dir.join("stops").join(format!("{id}.json")),
            record.to_string(),
        )
        .expect("a stop is written");
    }

    fn decision(dir: &Path, line: &Value) {
        record::append(&dir.join("decisions.jsonl"), line).expect("a decision is written");
    }

    fn record_of(dir: &Path, id: &str) -> Value {
        let file = dir.join("stops").join(format!("{id}.json"));
        serde_json::from_str(&fs::read_to_string(file).expect("kept")).expect("json")
    }

    fn ask(verb: &str, key: &str) -> Value {
        json!({"ask": verb, "key": key})
    }

    #[test]
    fn resume_and_widen_act_on_the_stop_their_row_names() {
        let dir = state("stop");
        stop(&dir, "s1", &["pr:example/shop#12"], false);
        let words = Words::default();
        press(&dir, &ask("widen", "pineapple:s1"), 5, &words).expect("widens");
        let widened = record_of(&dir, "s1");
        assert_eq!(
            widened["reaches"],
            json!(["everything"]),
            "it reaches everything"
        );
        assert_eq!(
            widened["was_reaching"],
            json!(["pr:example/shop#12"]),
            "keeping what it reached"
        );
        assert_eq!(
            press(&dir, &ask("widen", "pineapple:s1"), 6, &words),
            Err(Refused::new(
                "not-offered",
                "\"pineapple:s1\" doesn't offer \"widen\""
            )),
            "a stop reaching everything can't be widened again"
        );
        press(&dir, &ask("resume", "pineapple:s1"), 7, &words).expect("resumes");
        assert_eq!(
            record_of(&dir, "s1")["resumed"],
            json!({"at": 7, "by": "joel"}),
            "resumed by Joel"
        );
        assert_eq!(
            press(&dir, &ask("resume", "pineapple:s1"), 8, &words)
                .expect_err("refused")
                .code,
            "gone",
            "a resumed stop's row is gone"
        );
    }

    #[test]
    fn a_shadow_stop_or_a_made_up_key_is_never_acted_on() {
        let dir = state("shadow");
        stop(&dir, "s1", &["everything"], true);
        let words = Words::default();
        for key in ["pineapple:s1", "pineapple:nothing"] {
            assert_eq!(
                press(&dir, &ask("resume", key), 5, &words)
                    .expect_err("refused")
                    .code,
                "gone",
                "{key} isn't offered"
            );
        }
        assert!(
            record_of(&dir, "s1")["resumed"].is_null(),
            "the shadow stop is untouched"
        );
    }

    #[test]
    fn stop_on_a_waved_joke_makes_a_stop_reaching_what_it_would_have() {
        let dir = state("joke");
        decision(
            &dir,
            &json!({"key": "slack:C1:2.0", "thread": "slack:C1:1.0", "verdict": "waved:joke",
                "heard_at": 10, "by_joel": false, "shadow": false, "text": "pineapple override jk",
                "link": "https://example.slack.com/archives/C1/p2", "author": "U0OTHER001",
                "reaches": [], "would_reach": ["pr:example/shop#12"], "would_fall_back": false}),
        );
        let key = "pineapple:joke:slack:C1:1.0";
        let words = Words::default();
        press(&dir, &ask("stop", key), 20, &words).expect("stops");
        let made = resume::standing(&dir.join("stops"));
        assert_eq!(made.len(), 1, "one stop made");
        assert_eq!(
            made[0].record["reaches"],
            json!(["pr:example/shop#12"]),
            "reaching what a stop said there would"
        );
        assert_eq!(
            made[0].record["shadow"], false,
            "and live, since its alert was"
        );
        let rows = tell::rows(&tell::kept(&dir), 30, &words);
        let keys: Vec<&str> = rows.iter().filter_map(|row| row["key"].as_str()).collect();
        assert_eq!(
            keys,
            ["pineapple:slack:C1:2.0"],
            "the alert is gone and the stop is offered in its place"
        );
    }

    #[test]
    fn approve_resumes_what_someone_elses_resume_reaches() {
        let dir = state("approve");
        stop(&dir, "s1", &["quest:QEST-A"], false);
        stop(&dir, "s2", &["quest:QEST-B"], false);
        decision(
            &dir,
            &json!({"key": "slack:C1:3.0", "thread": "t", "verdict": "resume-asked",
                "heard_at": 10, "by_joel": false, "shadow": false, "reaches": ["quest:QEST-A"]}),
        );
        let answer = press(
            &dir,
            &ask("approve", "pineapple:resume:t"),
            20,
            &Words::default(),
        )
        .expect("approves");
        assert_eq!(answer["resumed"], json!(["s1"]), "only the stop it names");
        assert!(
            record_of(&dir, "s2")["resumed"].is_null(),
            "the other still stands"
        );
    }
}
