//! Documents' rows, ready for the launcher: each document Joel looked at
//! lately as the row Documents and each Google program's Recent show it,
//! worded, ordered and with its acts, as Drive.qml's row, Drive.js's
//! details and DrivePlaces.js's drawn make it in the shell today. A
//! `row:documents:<id>` record's `row` is the launcher's own key for it,
//! `gdoc:<id>`, which Docs' Recent and Drive's share; its `kind` is
//! Google's word for it, by which each program takes its own. Its drawer
//! is a record of its own, `drawer:documents:<id>`, watched by key while
//! it shows, so a row carries nothing a drawer alone says. Times travel as
//! `{at, as}` for the shell to word against its clock (the quest's
//! google-rows.md says each form), so no row changes each minute.

use std::collections::BTreeMap;

use ready_rows::numbers::Numbers;
use serde_json::{Value, json};

use crate::drive::{Document, Is};
use crate::rows::fill;

/// What a drawer's eyebrow joins its parts with, as every run of figures
/// is joined (docs/design.md).
const BETWEEN: &str = " \u{b7} ";

/// Each kind's words: the vocabulary's thing it is, and its two acts.
struct Kind {
    thing: &'static str,
    /// The thing's state it's said as, or "" for the thing itself.
    state: &'static str,
    open: &'static str,
    link: &'static str,
}

fn kind(is: Is) -> Kind {
    let drive = |state| Kind {
        thing: "drive:file",
        state,
        open: "verb:drive:open",
        link: "verb:drive:link",
    };
    match is {
        Is::Document => Kind {
            thing: "docs:document",
            state: "",
            open: "verb:docs:open",
            link: "verb:docs:link",
        },
        Is::Spreadsheet => Kind {
            thing: "sheets:spreadsheet",
            state: "",
            open: "verb:sheets:open",
            link: "verb:sheets:link",
        },
        Is::Presentation => Kind {
            thing: "slides:presentation",
            state: "",
            open: "verb:slides:open",
            link: "verb:slides:link",
        },
        Is::Folder => drive("folder"),
        Is::Drive => drive("drive"),
        Is::File | Is::Other => drive(""),
    }
}

/// Each kind of document's thing and state in the vocabulary, every one
/// `kind` names, which `read` requires each of.
const KINDS: [(&str, &str); 6] = [
    ("docs:document", ""),
    ("sheets:spreadsheet", ""),
    ("slides:presentation", ""),
    ("drive:file", ""),
    ("drive:file", "folder"),
    ("drive:file", "drive"),
];

/// Google's other kinds by Drive's word for their type, each a state of
/// `drive:file`; a kind not here is said as a file.
const GOOGLES_OTHERS: [&str; 6] = ["form", "drawing", "map", "site", "script", "vid"];

/// Each fact a drawer names, `drive:fact:<fact>` in the vocabulary.
const FACTS: [&str; 9] = [
    "changed", "changer", "owner", "yours", "created", "shared", "sharer", "size", "id",
];

/// What `drive:file` says, each one this file says.
const SAID: [&str; 9] = [
    "in",
    "onDrive",
    "measured",
    "lasts",
    "lastsHours",
    "bytes",
    "kilobytes",
    "megabytes",
    "gigabytes",
];

/// The words documents' rows are said in, from the launcher's vocabulary,
/// every one: nothing Joel reads is written here but what Drive says of a
/// file, its figures and the mark that joins a run, which
/// `every_word_a_document_shows_is_the_vocabularys` holds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Words {
    /// Each kind's name, glyph and untitled name, by its thing and state.
    named: BTreeMap<(String, String), Named>,
    /// Google's other kinds' names, by Drive's word for their type.
    others: BTreeMap<String, String>,
    /// Each fact's name, by the fact.
    facts: BTreeMap<String, String>,
    /// What `drive:file` says, by its key.
    said: BTreeMap<String, String>,
    /// Where a file is, by Drive's places' names.
    mine: String,
    shared_with_me: String,
    drives: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Named {
    name: String,
    glyph: String,
    untitled: String,
}

impl Words {
    /// Documents' words from the whole vocabulary, or the first it lacks,
    /// so a vocabulary that lost one is refused whole rather than shown
    /// with blanks.
    pub fn read(vocabulary: &Value) -> Result<Words, String> {
        let things = vocabulary["things"]
            .as_array()
            .ok_or("the vocabulary has no things")?;
        let thing = |id: &str| {
            things
                .iter()
                .find(|thing| thing["id"] == id)
                .ok_or_else(|| format!("the vocabulary has no {id}"))
        };
        let text = |id: &str, at: &[&str]| -> Result<String, String> {
            at.iter()
                .fold(thing(id)?, |within, field| &within[field])
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("the vocabulary's {id} has no {}", at.join(".")))
        };
        let mut named = BTreeMap::new();
        for (id, state) in KINDS {
            let untitled = text(id, &["states", "untitled", "name"])?;
            let glyph = text(id, &["glyph"])?;
            let (name, glyph) = if state.is_empty() {
                (text(id, &["name"])?, glyph)
            } else {
                let glyph = text(id, &["states", state, "glyph"]).unwrap_or(glyph);
                (text(id, &["states", state, "name"])?, glyph)
            };
            let key = (id.to_owned(), state.to_owned());
            named.insert(
                key,
                Named {
                    name,
                    glyph,
                    untitled,
                },
            );
        }
        let others = GOOGLES_OTHERS
            .iter()
            .map(|other| {
                let name = text("drive:file", &["states", other, "name"])?;
                Ok(((*other).to_owned(), name))
            })
            .collect::<Result<_, String>>()?;
        let facts = FACTS
            .iter()
            .map(|fact| {
                Ok((
                    (*fact).to_owned(),
                    text(&format!("drive:fact:{fact}"), &["name"])?,
                ))
            })
            .collect::<Result<_, String>>()?;
        let said = SAID
            .iter()
            .map(|key| Ok(((*key).to_owned(), text("drive:file", &["said", key])?)))
            .collect::<Result<_, String>>()?;
        Ok(Words {
            named,
            others,
            facts,
            said,
            mine: text("drive:mine", &["name"])?,
            shared_with_me: text("drive:place:shared", &["name"])?,
            drives: text("drive:place:drives", &["name"])?,
        })
    }

    /// A kind's words, which `read` made sure of.
    fn of(&self, kind: &Kind) -> Named {
        self.named
            .get(&(kind.thing.to_owned(), kind.state.to_owned()))
            .cloned()
            .unwrap_or_default()
    }

    fn fact(&self, fact: &str) -> &str {
        self.facts.get(fact).map_or("", String::as_str)
    }

    fn word(&self, key: &str) -> &str {
        self.said.get(key).map_or("", String::as_str)
    }
}

/// Each document Joel looked at lately, as its row, in the order held:
/// Docs', then Sheets', then Slides', each kind's last looked at first, as
/// the program keeps them (state.rs). `drives` are the shared drives, by
/// which a file on one says where it is.
pub fn documents(held: &[Document], words: &Words) -> Vec<Value> {
    held.iter()
        .map(|document| row(document, "documents", words))
        .collect()
}

/// Each document's drawer, by the key its row's drawer is watched by.
/// `drives` are the shared drives, by which a file on one says where it
/// is.
pub fn drawers(held: &[Document], drives: &[Document], words: &Words) -> Vec<(String, Value)> {
    held.iter()
        .map(|document| {
            let key = format!("drawer:documents:{}", document.id);
            (
                key,
                drawer(document, &title(document, words), drives, words),
            )
        })
        .collect()
}

fn title(document: &Document, words: &Words) -> String {
    if document.title.is_empty() {
        words.of(&kind(document.kind)).untitled
    } else {
        document.title.clone()
    }
}

/// A Drive file's one row, keyed by Drive's id as a document's always
/// was, so a document is one row in Docs' Recent and Drive's alike. Its
/// `order` is the daemon's, spaced as its place's rows were last told
/// (the ready-rows crate's `Kept`).
fn row(document: &Document, place: &str, words: &Words) -> Value {
    let kind = kind(document.kind);
    let named = words.of(&kind);
    let side: Vec<Value> = document
        .viewed
        .map(|at| json!({"at": at, "as": "opened", "keep": 1}))
        .into_iter()
        .collect();
    json!({
        "key": format!("row:{place}:{}", document.id),
        "thing": format!("gdoc:{}", document.id),
        "place": place,
        "kind": document.kind,
        "name": title(document, words),
        "glyph": named.glyph,
        "hue": "program",
        "depth": 0,
        "side": side,
        "drawer": format!("drawer:{place}:{}", document.id),
        "acts": [
            {"key": kind.open, "open": document.link},
            {"key": kind.link, "copy": document.link},
        ],
        "held": {"link": document.link, "viewed": document.viewed, "mime": document.mime},
    })
}

/// Its drawer: what changes most first, then what describes it, then
/// what identifies it, as every drawer's facts are (VKCF3Z, 2026-11-08),
/// each ready-rows' `{name, said, identifies}` or `{name, at, as}`; where
/// it is and its kind as its eyebrow; and what its owner wrote of it as
/// its words.
fn drawer(document: &Document, title: &str, drives: &[Document], words: &Words) -> Value {
    let when = |fact: &str, at: Option<i64>| {
        at.map(|at| json!({"name": words.fact(fact), "at": at, "as": "moment"}))
    };
    let said = |fact: &str, value: Option<&String>| {
        value
            .filter(|value| !value.is_empty())
            .map(|value| json!({"name": words.fact(fact), "said": value}))
    };
    let facts: Vec<Value> = [
        when("changed", document.changed),
        said("changer", document.changed_by.as_ref()),
        said("owner", document.owner.as_ref()),
        when("yours", document.changed_by_me),
        when("created", document.created),
        when("shared", document.shared_with_me),
        said("sharer", document.shared_by.as_ref()),
        document
            .size
            .map(|size| json!({"name": words.fact("size"), "said": weighed(size, words)})),
        (!document.id.is_empty())
            .then(|| json!({"name": words.fact("id"), "said": document.id, "identifies": true})),
    ]
    .into_iter()
    .flatten()
    .collect();
    let mut eyebrow: Vec<String> = vec![place(document, drives, words)];
    eyebrow.extend(kind_of(document, words));
    eyebrow.retain(|part| !part.is_empty());
    json!({
        "eyebrow": eyebrow.join(BETWEEN),
        "title": title,
        "facts": facts,
        "body": document.about.clone().unwrap_or_default(),
    })
}

/// Where it is, "in My Drive", "in Shared with Me" or "in Design (shared
/// drive)", or nothing for a shared drive itself, which its title names.
fn place(document: &Document, drives: &[Document], words: &Words) -> String {
    if document.kind == Is::Drive {
        return String::new();
    }
    let place = match &document.drive {
        Some(id) => drives.iter().find(|drive| &drive.id == id).map_or_else(
            || words.drives.clone(),
            |drive| fill(words.word("onDrive"), &[("drive", &drive.title)]),
        ),
        None if document.shared_with_me.is_some() => words.shared_with_me.clone(),
        None => words.mine.clone(),
    };
    fill(words.word("in"), &[("place", &place)])
}

/// Its kind as the eyebrow says it after where it is: a file by its
/// ending ("PDF"), with what a picture measures and how long a video
/// runs; one of Google's other kinds by Drive's word for its type
/// ("Form"), or else as a file; the rest by their name.
fn kind_of(document: &Document, words: &Words) -> Vec<String> {
    const GOOGLES: &str = "application/vnd.google-apps.";
    let named = words.of(&kind(document.kind)).name;
    let kind = match document.kind {
        Is::File => ending(&document.title).unwrap_or(named),
        Is::Other => document
            .mime
            .strip_prefix(GOOGLES)
            .and_then(|other| words.others.get(other))
            .cloned()
            .unwrap_or(named),
        _ => named,
    };
    let measured = match (document.across, document.down) {
        (Some(across), Some(down)) if across > 0 && down > 0 => fill(
            words.word("measured"),
            &[("across", &across.to_string()), ("down", &down.to_string())],
        ),
        _ => String::new(),
    };
    let lasts = document
        .lasts
        .map(|lasts| lasting(lasts, words))
        .unwrap_or_default();
    [kind, measured, lasts]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect()
}

/// A file's ending as its kind, "PDF" of "plan.pdf", or none.
fn ending(name: &str) -> Option<String> {
    let at = name.rfind('.')?;
    (at > 0 && at < name.len() - 1).then(|| name[at + 1..].to_uppercase())
}

/// A length in milliseconds as a player says one: 3:41, or 1:02:03.
fn lasting(milliseconds: u64, words: &Words) -> String {
    let seconds = (milliseconds + 500) / 1000;
    let (hours, minutes, rest) = (seconds / 3600, seconds % 3600 / 60, seconds % 60);
    let rest = format!("{rest:02}");
    if hours > 0 {
        let minutes = format!("{minutes:02}");
        let hours = hours.to_string();
        fill(
            words.word("lastsHours"),
            &[("hours", &hours), ("minutes", &minutes), ("seconds", &rest)],
        )
    } else {
        fill(
            words.word("lasts"),
            &[("minutes", &minutes.to_string()), ("seconds", &rest)],
        )
    }
}

/// A size in decimal units, to three significant figures as the launcher
/// says every measure (FileFacts.js's weighed): 5.60 MB, 48.0 MB, 312 kB.
/// Its units are the vocabulary's and its figure the ready-rows crate's,
/// grouped and pointed as the launcher's own sizes are, with the comma
/// and the full stop.
fn weighed(bytes: u64, words: &Words) -> String {
    const UNITS: [(f64, &str); 3] = [(1e9, "gigabytes"), (1e6, "megabytes"), (1e3, "kilobytes")];
    // What rounds to a thousand of one unit, to three figures.
    const ROUNDS_UP: f64 = 999.5;
    let numbers = Numbers::default();
    #[expect(clippy::cast_precision_loss, reason = "a size said to three figures")]
    let bytes = bytes as f64;
    let Some(at) = UNITS.iter().position(|(unit, _)| bytes >= *unit) else {
        return fill(words.word("bytes"), &[("count", &numbers.whole(bytes))]);
    };
    let scaled = bytes / UNITS[at].0;
    // 999.6 kB would read 1,000 kB: it's 1.00 MB instead.
    let (scaled, unit) = match at.checked_sub(1) {
        Some(up) if scaled >= ROUNDS_UP => (scaled / 1e3, UNITS[up].1),
        _ => (scaled, UNITS[at].1),
    };
    fill(words.word(unit), &[("count", &numbers.significant(scaled))])
}

#[cfg(test)]
#[path = "document_rows_tests.rs"]
mod tests;
