//! A call call-transcripts wrote down, as People draws it in the Last Met
//! of whoever it was with: one thing between them, `kind` met, its ready
//! row the one the launcher's Transcripts shows of it (Transcripts.qml),
//! named, said and acted on alike. Its name is who it was with as Tuple
//! said them, else their address, since a source names nobody through
//! People; a meeting's call is its title.

use serde_json::{Value, json};

use ready_rows::time::{self, Form};

use crate::styled::styled;
use crate::when::When;

/// The launcher's words a call is said in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Words {
    /// "Call With {who}".
    untitled: String,
    /// "Call at {time}".
    unknown: String,
    glyph: String,
    meeting: String,
    person: String,
    call: String,
    writing: String,
    on: String,
    under: String,
    minutes: String,
    hours: String,
    hours_minutes: String,
    length: String,
    with: String,
}

impl Words {
    /// A call's words from the whole vocabulary.
    ///
    /// # Errors
    ///
    /// The first word the vocabulary's `transcript` things lack.
    pub fn read(vocabulary: &Value) -> Result<Words, String> {
        let said = |field: &str| word(vocabulary, "transcript", &["said", field]);
        Ok(Words {
            untitled: word(vocabulary, "transcript", &["states", "untitled", "name"])?,
            unknown: word(vocabulary, "transcript", &["states", "unknown", "name"])?,
            glyph: word(vocabulary, "transcript", &["glyph"])?,
            meeting: said("meeting")?,
            person: said("person")?,
            call: said("call")?,
            writing: said("writing")?,
            on: said("on")?,
            under: said("under")?,
            minutes: said("minutes")?,
            hours: said("hours")?,
            hours_minutes: said("hoursMinutes")?,
            length: word(vocabulary, "transcript:length", &["name"])?,
            with: word(vocabulary, "transcript:with", &["name"])?,
        })
    }
}

/// A thing's word in the vocabulary, by its id and the fields to it.
fn word(vocabulary: &Value, id: &str, at: &[&str]) -> Result<String, String> {
    let thing = vocabulary["things"]
        .as_array()
        .and_then(|things| things.iter().find(|thing| thing["id"] == id))
        .ok_or_else(|| format!("the vocabulary has no {id}"))?;
    at.iter()
        .fold(thing, |within, field| &within[field])
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("the vocabulary's {id} has no {}", at.join(".")))
}

/// How a moment reads as a calendar and a clock, the machine's or, in
/// the cases, the world's.
pub type Clock = fn(i64) -> When;

/// Whether a call was with someone at this address, in any case.
pub fn with(call: &Value, address: &str) -> bool {
    text(call, "person").eq_ignore_ascii_case(address) && !address.is_empty()
}

/// A call as the thing between Joel and whoever it was with, its row
/// ready. `this_year` is the year it is, so its drawer says another's.
pub fn met(call: &Value, words: &Words, clock: Clock, this_year: i32) -> Value {
    let started = call["started"].as_i64().unwrap_or(0);
    let name = titled(call, words, clock);
    json!({
        "at": started,
        "key": format!("transcript:{}", text(call, "key")),
        "kind": "met",
        "one": true,
        "mine": false,
        "row": {
            "name": name,
            "glyph": words.glyph,
            "hue": "program",
            "side": [{"said": side(call, words)}],
            "note": note(call, words),
            "keywords": [text(call, "person")],
            "acts": acts(call),
            "drawer": drawer(call, &name, words, &clock(started).dated(this_year)),
        },
    })
}

/// A field's text, or "" where it's none.
fn text<'a>(call: &'a Value, field: &str) -> &'a str {
    call[field].as_str().unwrap_or("")
}

/// Who it was with as Tuple said them, else their address, or "" for a
/// meeting's call or one with nobody named.
fn who(call: &Value) -> &str {
    if call["event"].is_string() {
        return "";
    }
    let title = text(call, "title");
    if title.is_empty() {
        text(call, "person")
    } else {
        title
    }
}

/// Its name: a meeting's title; for a call with someone and no meeting,
/// who was on it, "Call With Sara Park"; failing both, when it began,
/// "Call at 14:00", since Joel remembers a call by who was on it.
fn titled(call: &Value, words: &Words, clock: Clock) -> String {
    let title = text(call, "title");
    if call["event"].is_string() && !title.is_empty() {
        return title.to_owned();
    }
    let on = who(call);
    if on.is_empty() {
        let started = call["started"].as_i64().unwrap_or(0);
        words.unknown.replace("{time}", &clock(started).clock())
    } else {
        words.untitled.replace("{who}", on)
    }
}

/// What it was, in a word or two: a meeting's call, a call with someone,
/// or a call that was neither.
fn side(call: &Value, words: &Words) -> String {
    if call["event"].is_string() {
        words.meeting.clone()
    } else if call["person"].is_string() {
        words.person.clone()
    } else {
        words.call.clone()
    }
}

/// When it was and how long, "yesterday 14:00 · 32 min", or, while it's
/// on, that it's being written down, or when it began and that nothing
/// is.
fn note(call: &Value, words: &Words) -> Value {
    if call["transcribing"].as_bool() == Some(true) {
        return json!(words.writing);
    }
    let started = call["started"].as_i64().unwrap_or(0);
    let lasted = call["ended"]
        .as_i64()
        .map_or_else(|| words.on.clone(), |ended| length(started, ended, words));
    json!([time::at(started, Form::Moment), {"said": lasted}])
}

/// How long it went on: "under a minute", "32 min", "1 h", "1 h 5 min".
fn length(started: i64, ended: i64, words: &Words) -> String {
    let minutes = (ended - started).max(0) / 60_000;
    let filled = |said: &str, hours: i64, minutes: i64| {
        said.replacen("{hours}", &hours.to_string(), 1).replacen(
            "{minutes}",
            &minutes.to_string(),
            1,
        )
    };
    match minutes {
        0 => words.under.clone(),
        1..60 => filled(&words.minutes, 0, minutes),
        _ if minutes % 60 == 0 => filled(&words.hours, minutes / 60, 0),
        _ => filled(&words.hours_minutes, minutes / 60, minutes % 60),
    }
}

/// Its acts, in the one order every surface lists them: Open the
/// Transcript, once there's one, Open Its Folder, then, last as copying
/// always is, Copy the Path, the transcript's, or its folder's while
/// there's none.
fn acts(call: &Value) -> Value {
    let transcript = text(call, "transcript");
    let folder = text(call, "folder");
    let mut acts = vec![];
    if !transcript.is_empty() {
        acts.push(json!({"key": "verb:transcript:open", "open": transcript}));
    }
    acts.push(json!({"key": "verb:transcript:folder", "open": folder}));
    let path = if transcript.is_empty() {
        folder
    } else {
        transcript
    };
    acts.push(json!({"key": "verb:transcript:copy", "copy": path}));
    Value::Array(acts)
}

/// Its drawer: when it was over its title, how long it went on and who
/// it was with, then its summary when one's written, else how its
/// transcript opens, both set as the markdown they are, as prose.
fn drawer(call: &Value, name: &str, words: &Words, dated: &str) -> Value {
    let started = call["started"].as_i64().unwrap_or(0);
    let mut facts = vec![];
    if let Some(ended) = call["ended"].as_i64() {
        facts.push(json!({"name": words.length, "said": length(started, ended, words)}));
    }
    let person = text(call, "person");
    if !person.is_empty() {
        let named = who(call);
        let with = if named.is_empty() { person } else { named };
        facts.push(json!({"name": words.with, "said": with}));
    }
    let summary = text(call, "summary");
    let body = if summary.is_empty() {
        text(call, "opening")
    } else {
        summary
    };
    json!({
        "eyebrow": dated,
        "title": name,
        "facts": facts,
        "body": styled(body),
        "styled": true,
        "prose": true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::when::utc;

    const MINUTE: i64 = 60_000;
    // Tuesday 3 November 2026, 14:00 in the world's time.
    const AT: i64 = 1_793_714_400_000;

    fn words() -> Words {
        // The launcher's own, joined from each place's file, which the
        // build puts beside the crate's folder (package.nix).
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../quickshell/launcher/words.json"
        ))
        .expect("the launcher's words are put beside the crate");
        Words::read(&serde_json::from_str(&text).expect("the words are JSON"))
            .expect("the launcher's words say a call")
    }

    fn call(fields: &Value) -> Value {
        let mut call = json!({
            "key": "call:e1", "kind": "call", "title": "Planning", "started": AT,
            "ended": AT + 32 * MINUTE, "event": "e1", "person": null, "folder": "/calls/a",
            "transcript": "/calls/a/transcript.md", "opening": null, "summary": null,
            "transcribing": false
        });
        for (field, value) in fields.as_object().into_iter().flatten() {
            call[field] = value.clone();
        }
        call
    }

    fn row(fields: &Value) -> Value {
        met(&call(fields), &words(), utc, 2026)["row"].clone()
    }

    #[test]
    fn a_meetings_call_is_named_by_the_meeting_and_said_as_one() {
        let thing = met(&call(&json!({})), &words(), utc, 2026);
        assert_eq!(
            (&thing["at"], &thing["key"], &thing["kind"], &thing["one"]),
            (
                &json!(AT),
                &json!("transcript:call:e1"),
                &json!("met"),
                &json!(true)
            ),
            "one thing they met at, when it began"
        );
        let row = &thing["row"];
        assert_eq!(row["name"], "Planning", "named by its meeting");
        assert_eq!(
            row["side"],
            json!([{"said": "meeting"}]),
            "a meeting's call"
        );
        assert_eq!(
            row["note"],
            json!([{"at": AT, "as": "moment"}, {"said": "32 min"}]),
            "when it began, worded by the shell, and how long"
        );
        assert_eq!(
            row["drawer"],
            json!({
                "eyebrow": "Tuesday 3 November \u{b7} 14:00",
                "title": "Planning",
                "facts": [{"name": "Length", "said": "32 min"}],
                "body": "",
                "styled": true,
                "prose": true
            }),
            "its drawer, with nobody named"
        );
    }

    #[test]
    fn a_call_with_someone_is_named_by_who_tuple_said_else_their_address() {
        let named = row(&json!({"event": null, "title": "Ada Lovelace",
            "person": "ada@example.com"}));
        assert_eq!(named["name"], "Call With Ada Lovelace", "by Tuple's name");
        assert_eq!(
            named["side"],
            json!([{"said": "call"}]),
            "a call with someone"
        );
        assert_eq!(
            named["drawer"]["facts"][1],
            json!({"name": "With", "said": "Ada Lovelace"}),
            "who it was with"
        );
        assert_eq!(
            named["keywords"],
            json!(["ada@example.com"]),
            "found by address"
        );
        let bare = row(&json!({"event": null, "title": "", "person": "ada@example.com"}));
        assert_eq!(bare["name"], "Call With ada@example.com", "else by address");
        let nobody = row(&json!({"event": null, "title": ""}));
        assert_eq!(nobody["name"], "Call at 14:00", "else by when it began");
        assert_eq!(nobody["side"], json!([{"said": "call"}]), "a call alone");
    }

    #[test]
    fn a_calls_length_and_its_state_are_its_note() {
        let lasting =
            |minutes: i64| row(&json!({"ended": AT + minutes * MINUTE}))["note"][1].clone();
        assert_eq!(lasting(0), json!({"said": "under a minute"}));
        assert_eq!(lasting(5), json!({"said": "5 min"}));
        assert_eq!(lasting(60), json!({"said": "1 h"}));
        assert_eq!(lasting(65), json!({"said": "1 h 5 min"}));
        assert_eq!(
            row(&json!({"ended": null}))["note"],
            json!([{"at": AT, "as": "moment"}, {"said": "not written down"}]),
            "on still, nothing written"
        );
        assert_eq!(
            row(&json!({"ended": null, "transcribing": true}))["note"],
            "being written down",
            "being written"
        );
    }

    #[test]
    fn a_calls_acts_open_its_transcript_then_its_folder_then_copy_a_path() {
        assert_eq!(
            row(&json!({}))["acts"],
            json!([
                {"key": "verb:transcript:open", "open": "/calls/a/transcript.md"},
                {"key": "verb:transcript:folder", "open": "/calls/a"},
                {"key": "verb:transcript:copy", "copy": "/calls/a/transcript.md"}
            ]),
            "with a transcript"
        );
        assert_eq!(
            row(&json!({"transcript": null}))["acts"],
            json!([
                {"key": "verb:transcript:folder", "open": "/calls/a"},
                {"key": "verb:transcript:copy", "copy": "/calls/a"}
            ]),
            "with none written yet"
        );
    }

    #[test]
    fn a_calls_drawer_holds_its_summary_over_how_it_opens() {
        let summed = row(&json!({"summary": "- Ship it", "opening": "**Joel**, 0:00: Hi"}));
        assert_eq!(summed["drawer"]["body"], "\u{2022} Ship it", "its summary");
        let opened = row(&json!({"opening": "**Ada**, 0:00: Hi"}));
        assert_eq!(
            opened["drawer"]["body"], "<b>Ada</b>, 0:00: Hi",
            "else how it opens"
        );
        let old = met(
            &call(&json!({"started": 1_767_171_600_000_i64})),
            &words(),
            utc,
            2026,
        );
        assert_eq!(
            old["row"]["drawer"]["eyebrow"], "Wednesday 31 December 2025 \u{b7} 09:00",
            "another year's, with its year"
        );
    }

    #[test]
    fn a_call_is_with_whoever_its_address_names_in_any_case() {
        let call = call(&json!({"person": "Ada@Example.com"}));
        assert!(with(&call, "ada@example.com"), "by address, in any case");
        assert!(!with(&call, "ben@example.com"), "not with another");
        assert!(!with(&json!({}), ""), "nor with no address");
    }
}
