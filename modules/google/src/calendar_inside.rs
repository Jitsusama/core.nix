//! What's inside a meeting's row, served as rows of its own (ready-rows.md:
//! a row whose acts step into rows has the rows whose `parent` is its key
//! as its verbs), so the launcher words and builds none of them: each of
//! the meeting's acts in the one order every surface lists them, a row
//! whose Return does it; Show in Calendar a row that steps in to what's
//! known of it, each a value Return copies, then Open in Calendar; Guests
//! and Attachments a row each that steps in to who and what is on it; and
//! Cancel a row that steps in to the one row asking Joel to be sure.
//! Return on the meeting itself joins its call from ten minutes before it
//! starts until it ends, and otherwise steps in to its Show in Calendar
//! (an act's `into`), listed first among its acts, its drawer opening by
//! resting as every drawer does. The row is served again as that turns,
//! which the daemon's look each minute does, rather than the shell
//! reading the clock to choose.
//!
//! Move the Meeting steps in to the later times it offers, each a row
//! saying its day and times and the meeting it would clash with, holding
//! the one row asking Joel to be sure, whose Return moves it; a time typed
//! inside Move is a search within Move's row (`calendar_typed`), Move's row
//! served with `kind: "move"` for the shell to find. Open the Transcript is
//! a row only call-transcripts can make, since only it knows whether a
//! call was written down, so it's served lent (ready-rows.md's lends,
//! `lentOnly`): call-transcripts' row where it lends one, and none where
//! it doesn't.

use serde_json::{Value, json};

use jiff::tz::TimeZone;
use jiff::{Span, Timestamp};

use crate::calendar::{Answer, Meeting};
use crate::calendar_rows::{dated_between, dated_fully};
use crate::calendar_words::{LATER, Later, Words};
use crate::compose;
use crate::rows::{asks, fill, opens};

/// Milliseconds in a minute, as the meetings' times count.
const MINUTE: i64 = 60_000;

/// How soon before its start Return on a meeting joins its call rather
/// than showing it: ten minutes (Calendar.js's `TAKES_OVER`).
const TAKES_OVER: i64 = 600_000;

/// The act every row with rows inside it steps in by.
fn more() -> Value {
    json!({"key": "verb:more", "step": "rows"})
}

/// The acts of the meeting row `key` at `now`: Return joining its call
/// while it's on, from ten minutes before, or else stepping into its Show
/// in Calendar, listed first, as the shell did (VKCF3Z and Y2YBTS,
/// 2026-11-12), then the step into its acts. Its drawer opens by resting.
pub(crate) fn returns(meeting: &Meeting, now: i64, key: &str) -> Vec<Value> {
    let on = !meeting.all_day && now >= meeting.start - TAKES_OVER && now < meeting.end;
    let first = match meeting.link.as_deref().filter(|_| on) {
        Some(link) => opens("verb:calendar:join", link),
        None => json!({"key": "verb:calendar:see", "step": "rows", "into": format!("{key}:see")}),
    };
    vec![first, more()]
}

/// The meeting row a row inside belongs to: its key, thing and place,
/// the drawer it names, and that drawer as served, whose facts Show in
/// Calendar lists.
pub(crate) struct Within<'a> {
    pub key: &'a str,
    pub thing: &'a str,
    pub place: &'a str,
    pub drawer: &'a str,
    pub details: &'a Value,
    /// The zone Joel's clock reads, which a day later keeps the time on.
    pub zone: &'a TimeZone,
    /// Joel's meetings, which a time Move offers may clash with.
    pub others: &'a [Meeting],
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
                row["acts"] = json!([more()]);
                row["drawer"] = json!(within.drawer);
                let key = row["key"].as_str().unwrap_or_default().to_owned();
                rows.push(row);
                rows.extend(facts(within, &key, meeting, words));
                continue;
            }
            // Call-transcripts' row of the meeting's call where it lends one, and none
            // otherwise, so its acts here are never drawn.
            "verb:calendar:transcript" => {
                row["lends"] = json!(format!("transcript:{}", meeting.id));
                row["lentOnly"] = json!(true);
                row["acts"] = json!([]);
            }
            "verb:calendar:yes" | "verb:calendar:maybe" | "verb:calendar:no" => {
                row["acts"] = json!([{"key": verb, "ask": act["ask"]}]);
                if act["given"] == true {
                    row["side"] = json!([{"said": words.word("yourAnswer")}]);
                }
            }
            "verb:calendar:move" => {
                row["acts"] = json!([more()]);
                // Found by the shell, which reads a time typed inside it.
                row["kind"] = json!("move");
                let key = row["key"].as_str().unwrap_or_default().to_owned();
                rows.push(row);
                let telling = act["telling"].as_str().unwrap_or_default();
                rows.extend(later(within, &key, meeting, telling, words));
                continue;
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

/// The later times Move offers, each a row inside Move named for how far
/// it takes the meeting, lasting as long, its day and times below and at
/// its side the first of Joel's other meetings it would overlap; each
/// holding the one row asking Joel to be sure, which names who Google
/// tells and whose Return moves it (Calendar.js's `later` and `clashes`).
/// Each is keyed by when it would start, so a row holds still as the
/// meeting is served again.
fn later(
    within: &Within,
    parent: &str,
    meeting: &Meeting,
    telling: &str,
    words: &Words,
) -> Vec<Value> {
    let lasts = meeting.end - meeting.start;
    let mut rows = Vec::new();
    for ((_, by), name) in LATER.iter().zip(&words.move_later) {
        let Some(start) = shifted(meeting.start, *by, within.zone) else {
            continue;
        };
        let to = Moved {
            start,
            end: start + lasts,
            name: Some(name),
        };
        rows.extend(moved(within, parent, meeting, telling, &to, words));
    }
    rows
}

/// A time typed inside Move, from `start` to `end`, as the daemon answers
/// a search within it (ready-rows.md's `within`): named by its day and
/// times, and made as a later time is, keyed by when it starts, so the
/// same time typed on is the same row.
pub(crate) fn typed(
    within: &Within,
    parent: &str,
    meeting: &Meeting,
    telling: &str,
    (start, end): (i64, i64),
    words: &Words,
) -> Vec<Value> {
    let to = Moved {
        start,
        end,
        name: None,
    };
    moved(within, parent, meeting, telling, &to, words)
}

/// A time Move takes the meeting to, and its name, or none to be named by
/// its day and times.
struct Moved<'a> {
    start: i64,
    end: i64,
    name: Option<&'a str>,
}

/// The row of a time Move offers, inside `parent`, its day and times
/// below unless that's its name, at its side the first of Joel's other
/// meetings it would overlap, and the one row inside it asking Joel to be
/// sure, which names who Google tells and whose Return moves it; none for
/// a time no clock says.
fn moved(
    within: &Within,
    parent: &str,
    meeting: &Meeting,
    telling: &str,
    to: &Moved,
    words: &Words,
) -> Vec<Value> {
    let (start, end) = (to.start, to.end);
    let Ok(iso) = Timestamp::from_millisecond(start) else {
        return Vec::new();
    };
    let key = format!("{parent}:{iso}");
    let dated = dated_between(meeting, start, end, within.zone, words);
    let (name, note) = match to.name {
        Some(name) => (name, dated.as_str()),
        None => (dated.as_str(), ""),
    };
    let side: Vec<Value> = clash(meeting, start, end, within.others)
        .map(|other| {
            let title = if other.title.is_empty() {
                words.untitled.as_str()
            } else {
                other.title.as_str()
            };
            json!({"said": fill(&words.move_clashes, &[("title", title)])})
        })
        .into_iter()
        .collect();
    let (_, glyph) = words.act("verb:calendar:move");
    let time = json!({
        "key": &key,
        "thing": format!("{}:move:{iso}", within.thing),
        "place": within.place,
        "kind": "time",
        "parent": parent,
        "depth": 2,
        "name": name,
        "glyph": glyph,
        "hue": "action",
        "side": side,
        "note": note,
        "acts": [more()],
    });
    let to = fill(
        &words.move_to,
        &[("sure", &words.move_sure), ("when", &dated)],
    );
    let sure = json!({
        "key": format!("{key}:sure"),
        "thing": format!("{}:move:{iso}:sure", within.thing),
        "place": within.place,
        "kind": "act",
        "parent": &key,
        "depth": 3,
        "name": fill(
            words.fact("telling", "sure"),
            &[("sure", &to), ("telling", telling)],
        ),
        "glyph": glyph,
        "hue": "alert",
        "sure": true,
        "side": [],
        "acts": [asks(
            "verb:calendar:move",
            &json!({"move": {"id": meeting.id, "start": start, "end": end}}),
        )],
    });
    vec![time, sure]
}

/// A meeting's start moved by minutes, or by days at the same time on
/// Joel's clock, which a change of the clocks between doesn't move.
fn shifted(start: i64, by: Later, zone: &TimeZone) -> Option<i64> {
    match by {
        Later::Minutes(minutes) => Some(start + minutes * MINUTE),
        Later::Days(days) => {
            let at = Timestamp::from_millisecond(start)
                .ok()?
                .to_zoned(zone.clone());
            let moved = at.checked_add(Span::new().days(days)).ok()?;
            Some(moved.timestamp().as_millisecond())
        }
    }
}

/// The first of Joel's other meetings a meeting moved to run from `start`
/// to `end` would overlap: not one all day, nor one he said no to.
fn clash<'a>(
    meeting: &Meeting,
    start: i64,
    end: i64,
    others: &'a [Meeting],
) -> Option<&'a Meeting> {
    others.iter().find(|other| {
        other.id != meeting.id
            && !other.all_day
            && other.answer != Some(Answer::No)
            && other.start < end
            && other.end > start
    })
}

/// What's known of it, each a row inside Show in Calendar named by what
/// it says, which Return copies, with what it is at its side: its day and
/// times, where, Joel's answer, its call, who holds it and each guest with
/// how they answered, copying an address where one is known; what it's
/// about on one line, copied whole; then Open in Calendar, its page in
/// Google's window, where it's changed (Calendar.qml's `facts`, as the
/// shell showed them).
fn facts(within: &Within, parent: &str, meeting: &Meeting, words: &Words) -> Vec<Value> {
    let value = |id: &str, name: &str, copy: &str, side: &str| {
        json!({
            "key": format!("{parent}:{id}"),
            "thing": format!("{}:fact:{id}", within.thing),
            "place": within.place,
            "kind": "fact",
            "parent": parent,
            "depth": 2,
            "name": name,
            "hue": "voice",
            "side": [{"said": side}],
            "acts": [{"key": "verb:copy", "copy": copy}],
        })
    };
    let dated = dated_fully(meeting, within.zone, words);
    let mut rows = vec![value("when", &dated, &dated, words.word("whenSide"))];
    if let Some(place) = meeting.place.as_deref().filter(|place| !place.is_empty()) {
        rows.push(value("where", place, place, words.word("whereSide")));
    }
    if let Some(answer) = meeting.answer {
        let said = words.answer(answer);
        rows.push(value("answer", said, said, words.word("yourAnswer")));
    }
    if let Some(link) = &meeting.link {
        rows.push(value("call", link, link, words.word("callSide")));
    }
    if let Some(holder) = &meeting.organizer {
        let copy = holder.email.as_deref().unwrap_or(&holder.name);
        rows.push(value(
            "holder",
            &holder.name,
            copy,
            words.word("holderSide"),
        ));
    }
    for (at, guest) in meeting.guests.iter().enumerate() {
        let copy = guest.email.as_deref().unwrap_or(&guest.name);
        let id = format!("guest:{at}");
        rows.push(value(&id, &guest.name, copy, words.answer(guest.answer)));
    }
    if let Some(about) = meeting
        .about
        .as_deref()
        .filter(|about| !about.trim().is_empty())
    {
        let line = about.split_whitespace().collect::<Vec<_>>().join(" ");
        rows.push(value("about", &line, about, words.word("aboutSide")));
    }
    if let Some(page) = &meeting.page {
        let verb = "verb:calendar:open";
        let mut open = acted(within, parent, "see:open", verb, 2, words);
        open["acts"] = json!([opens(verb, page)]);
        rows.push(open);
    }
    rows
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
        let by = email.trim().to_lowercase();
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
            // Who they are is People's to say, where it knows them, its row
            // stepping through to them (ready-rows.md's lends).
            "lends": format!("person:{by}"),
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
/// window, a document of Docs, Sheets or Slides the document itself, so
/// Documents' row for it shows where Documents holds it (ready-rows.md's
/// lends).
fn attachments(within: &Within, parent: &str, meeting: &Meeting, words: &Words) -> Vec<Value> {
    let (_, glyph) = words.act("calendar:attachment");
    meeting
        .attachments
        .iter()
        .enumerate()
        .map(|(at, file)| {
            let thing = match (&file.id, file.kind) {
                (Some(id), Some(_)) => format!("gdoc:{id}"),
                _ => format!("{}:attachment:{}", within.thing, file.link),
            };
            let mut row = json!({
                "key": format!("{parent}:{at}"),
                "thing": &thing,
                "place": within.place,
                "kind": "attachment",
                "parent": parent,
                "depth": 2,
                "name": file.title,
                "glyph": glyph,
                "hue": "program",
                "side": [],
                "acts": [opens("calendar:attachment", &file.link)],
            });
            if thing.starts_with("gdoc:") {
                row["lends"] = json!(thing);
            }
            row
        })
        .collect()
}
