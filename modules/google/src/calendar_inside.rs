//! What's inside a meeting's row, served as rows of its own (ready-rows.md:
//! a row whose acts step into rows has the rows whose `parent` is its key
//! as its verbs), so the launcher words and builds none of them: each of
//! the meeting's acts in the one order every surface lists them, a row
//! whose Return does it; Guests and Attachments a row each that steps in
//! to who and what is on it; and Cancel a row that steps in to the one row
//! asking Joel to be sure. Return on the meeting itself joins its call from
//! ten minutes before it starts until it ends, and shows its details in the
//! drawer otherwise, so the row is served again as that turns, which the
//! daemon's look each minute does, rather than the shell reading the clock
//! to choose.
//!
//! Two of its acts are still the shell's, each named for what takes it
//! next: Move the Meeting's later times and one typed there, which need
//! Calendar.qml's clock and its reading of a typed time (the google daemon
//! answers them on ask next), and Open the Transcript, which only
//! call-transcripts knows (it serves that row next). Move's row is served
//! here stepping into rows, with `kind: "move"`, so the shell makes only
//! what's inside it; the transcript's isn't served.

use serde_json::{Value, json};

use crate::calendar::Meeting;
use crate::calendar_words::Words;
use crate::compose;
use crate::rows::opens;

/// How soon before its start Return on a meeting joins its call rather
/// than showing it: ten minutes (Calendar.js's `TAKES_OVER`).
const TAKES_OVER: i64 = 600_000;

/// The act every row with rows inside it steps in by.
fn more() -> Value {
    json!({"key": "verb:more", "step": "rows"})
}

/// A meeting row's own acts at `now`: Return joining its call while it's
/// on, from ten minutes before, or else showing its details in the drawer,
/// then the step into its acts.
pub(crate) fn returns(meeting: &Meeting, now: i64) -> Vec<Value> {
    let on = !meeting.all_day && now >= meeting.start - TAKES_OVER && now < meeting.end;
    let first = match meeting.link.as_deref().filter(|_| on) {
        Some(link) => opens("verb:calendar:join", link),
        None => json!({"key": "verb:details", "step": "drawer"}),
    };
    vec![first, more()]
}

/// The meeting row a row inside belongs to: its key, thing and place,
/// and the drawer it names.
pub(crate) struct Within<'a> {
    pub key: &'a str,
    pub thing: &'a str,
    pub place: &'a str,
    pub drawer: &'a str,
}

/// Every row inside the meeting's row, made from its acts as
/// `calendar_rows` lists them.
pub(crate) fn inside(
    within: &Within,
    meeting: &Meeting,
    acts: &[Value],
    words: &Words,
) -> Vec<Value> {
    let mut rows = Vec::new();
    for act in acts {
        let Some(verb) = act["key"].as_str() else {
            continue;
        };
        let id = verb.trim_start_matches("verb:calendar:");
        let mut row = acted(within, within.key, id, verb, 1, words);
        match verb {
            "verb:calendar:see" => {
                row["acts"] = json!([{"key": verb, "step": "drawer"}]);
                row["drawer"] = json!(within.drawer);
            }
            // Open the Transcript is call-transcripts' to serve.
            "verb:calendar:transcript" => continue,
            "verb:calendar:yes" | "verb:calendar:maybe" | "verb:calendar:no" => {
                row["acts"] = json!([{"key": verb, "ask": act["ask"]}]);
                if act["given"] == true {
                    row["side"] = json!([{"said": words.word("yourAnswer")}]);
                }
            }
            // What the shell's later times need to say, till they're served.
            "verb:calendar:move" => {
                row["acts"] = json!([more()]);
                row["kind"] = json!("move");
                row["moves"] = json!({"sure": act["sure"], "telling": act["telling"]});
            }
            "verb:calendar:guests" | "verb:calendar:attachments" => {
                row["acts"] = json!([more()]);
                row["side"] = json!([{"said": act["count"].to_string()}]);
                let key = row["key"].as_str().unwrap_or_default().to_owned();
                rows.push(row);
                if verb == "verb:calendar:guests" {
                    rows.extend(guests(within, &key, meeting, words));
                } else {
                    rows.extend(attachments(within, &key, meeting, words));
                }
                continue;
            }
            "verb:calendar:cancel" => {
                row["acts"] = json!([more()]);
                let key = row["key"].as_str().unwrap_or_default().to_owned();
                rows.push(row);
                let mut sure = acted(within, &key, "cancel:sure", verb, 2, words);
                sure["name"] = act["sure"].clone();
                sure["sure"] = json!(true);
                sure["hue"] = json!("alert");
                sure["acts"] = json!([{"key": verb, "ask": act["ask"]}]);
                rows.push(sure);
                continue;
            }
            "verb:calendar:link" => {
                row["acts"] = json!([{"key": verb, "copy": act["copy"]}]);
                row["side"] = json!([{"said": act["side"]}]);
            }
            _ => row["acts"] = json!([act]),
        }
        rows.push(row);
    }
    rows
}

/// A row inside `parent`, `id` after the meeting's own, doing the act
/// `verb`, named and marked as the vocabulary names it.
fn acted(within: &Within, parent: &str, id: &str, verb: &str, depth: u8, words: &Words) -> Value {
    let (name, glyph) = words.act(verb);
    json!({
        "key": format!("{}:{id}", within.key),
        "thing": format!("{}:{id}", within.thing),
        "place": within.place,
        "kind": "act",
        "parent": parent,
        "depth": depth,
        "name": name,
        "glyph": glyph,
        "hue": "action",
        "side": [],
    })
}

/// Who is on it besides Joel, each a row inside Guests: who holds it,
/// unless he does, then each guest, by name or else address, how they
/// answered at its side; Return writing to them in Google's window and a
/// second act copying their address. Each address is one row, as Guests
/// counts them (`guests_counted`), and one without an address none.
fn guests(within: &Within, parent: &str, meeting: &Meeting, words: &Words) -> Vec<Value> {
    let holder = meeting
        .organizer
        .as_ref()
        .filter(|_| !meeting.holds)
        .map(|person| (person.name.as_str(), person.email.as_deref(), None));
    let asked = meeting.guests.iter().map(|guest| {
        (
            guest.name.as_str(),
            guest.email.as_deref(),
            Some(guest.answer),
        )
    });
    let mut seen: Vec<String> = Vec::new();
    let mut rows = Vec::new();
    for (name, email, answer) in holder.into_iter().chain(asked) {
        let Some(email) = email else {
            continue;
        };
        let by = email.to_lowercase();
        if seen.contains(&by) {
            continue;
        }
        seen.push(by.clone());
        let writing = compose::writing(&[email], &meeting.title, &[]);
        let side: Vec<Value> = answer
            .map(|answer| json!({"said": words.answer(answer)}))
            .into_iter()
            .collect();
        rows.push(json!({
            "key": format!("{parent}:{by}"),
            "thing": format!("{}:guest:{by}", within.thing),
            "place": within.place,
            "kind": "guest",
            "parent": parent,
            "depth": 2,
            "name": if name.is_empty() { email } else { name },
            "glyph": "person",
            "hue": "person",
            "side": side,
            "acts": [
                opens("verb:person:mail", &writing),
                {"key": "verb:calendar:address", "copy": email},
            ],
        }));
    }
    rows
}

/// The files on it, each a row inside Attachments opening it in Google's
/// window.
fn attachments(within: &Within, parent: &str, meeting: &Meeting, words: &Words) -> Vec<Value> {
    let (_, glyph) = words.act("calendar:attachment");
    meeting
        .attachments
        .iter()
        .enumerate()
        .map(|(at, file)| {
            json!({
                "key": format!("{parent}:{at}"),
                "thing": format!("{}:attachment:{}", within.thing, file.link),
                "place": within.place,
                "kind": "attachment",
                "parent": parent,
                "depth": 2,
                "name": file.title,
                "glyph": glyph,
                "hue": "program",
                "side": [],
                "acts": [opens("calendar:attachment", &file.link)],
            })
        })
        .collect()
}
