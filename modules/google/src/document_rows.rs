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

use serde_json::{Value, json};

use crate::drive::{Document, Is};

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

/// Every thing whose words a document's row is said in.
const THINGS: [&str; 4] = [
    "docs:document",
    "sheets:spreadsheet",
    "slides:presentation",
    "drive:file",
];

/// The words documents' rows are said in, from the launcher's vocabulary.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Words {
    /// Each thing's name, glyph and untitled name, by its id and state.
    named: Vec<Named>,
    /// "in {place}".
    within: String,
    /// "{drive} (shared drive)".
    on_drive: String,
    /// Where a file is, by Drive's places' names.
    mine: String,
    shared_with_me: String,
    drives: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Named {
    thing: String,
    state: String,
    name: String,
    glyph: String,
    untitled: String,
}

impl Words {
    /// Documents' words from the whole vocabulary, or the first it lacks.
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
        let text = |within: &Value, at: &[&str], id: &str| -> Result<String, String> {
            at.iter()
                .fold(within, |within, field| &within[field])
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("the vocabulary's {id} has no {}", at.join(".")))
        };
        let mut named = Vec::new();
        for id in THINGS {
            let held = thing(id)?;
            let name = text(held, &["name"], id)?;
            let glyph = text(held, &["glyph"], id)?;
            let untitled = text(held, &["states", "untitled", "name"], id)?;
            named.push(Named {
                thing: id.to_owned(),
                state: String::new(),
                name,
                glyph: glyph.clone(),
                untitled: untitled.clone(),
            });
            for (state, said) in held["states"].as_object().into_iter().flatten() {
                if state == "untitled" {
                    continue;
                }
                named.push(Named {
                    thing: id.to_owned(),
                    state: state.clone(),
                    name: text(said, &["name"], id)?,
                    glyph: said["glyph"].as_str().unwrap_or(&glyph).to_owned(),
                    untitled: untitled.clone(),
                });
            }
        }
        let file = thing("drive:file")?;
        Ok(Words {
            named,
            within: text(file, &["said", "in"], "drive:file")?,
            on_drive: text(file, &["said", "onDrive"], "drive:file")?,
            mine: text(thing("drive:mine")?, &["name"], "drive:mine")?,
            shared_with_me: text(
                thing("drive:place:shared")?,
                &["name"],
                "drive:place:shared",
            )?,
            drives: text(
                thing("drive:place:drives")?,
                &["name"],
                "drive:place:drives",
            )?,
        })
    }

    fn of(&self, kind: &Kind) -> Named {
        self.named
            .iter()
            .find(|named| named.thing == kind.thing && named.state == kind.state)
            .cloned()
            .unwrap_or_default()
    }
}

/// Each document Joel looked at lately, as its row, in the order held:
/// Docs', then Sheets', then Slides', each kind's last looked at first, as
/// the program keeps them (state.rs). `drives` are the shared drives, by
/// which a file on one says where it is.
pub fn documents(held: &[Document], words: &Words) -> Vec<Value> {
    held.iter()
        .enumerate()
        .map(|(order, document)| row(document, "documents", order, words))
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
/// was, so a document is one row in Docs' Recent and Drive's alike.
fn row(document: &Document, place: &str, order: usize, words: &Words) -> Value {
    let kind = kind(document.kind);
    let named = words.of(&kind);
    let side: Vec<Value> = document
        .viewed
        .map(|at| json!({"at": at, "as": "opened", "keep": 1, "stays": true}))
        .into_iter()
        .collect();
    json!({
        "key": format!("row:{place}:{}", document.id),
        "row": format!("gdoc:{}", document.id),
        "place": place,
        "kind": document.kind,
        "order": order,
        "name": title(document, words),
        "glyph": named.glyph,
        "hue": "program",
        "depth": 0,
        "side": side,
        "drawer": format!("drawer:{place}:{}", document.id),
        "acts": [
            {"key": kind.open, "ask": {"open": {"link": document.link}}},
            {"key": kind.link, "ask": {"copy": document.link}},
        ],
        "held": {"link": document.link, "viewed": document.viewed, "mime": document.mime},
    })
}

/// Its drawer: what changes most first, then what describes it, then
/// what identifies it, as every drawer's facts are (VKCF3Z, 2026-11-08);
/// where it is and its kind as its eyebrow, giving way from its end; and
/// what its owner wrote of it as its words.
fn drawer(document: &Document, title: &str, drives: &[Document], words: &Words) -> Value {
    let when = |label: &str, at: Option<i64>| {
        at.map(|at| json!({"label": label, "at": at, "as": "moment"}))
    };
    let said = |label: &str, value: Option<&String>| {
        value
            .filter(|value| !value.is_empty())
            .map(|value| json!({"label": label, "value": value}))
    };
    let facts: Vec<Value> = [
        when("Changed", document.changed),
        said("Changed By", document.changed_by.as_ref()),
        said("Owner", document.owner.as_ref()),
        when("You Changed", document.changed_by_me),
        when("Created", document.created),
        when("Shared", document.shared_with_me),
        said("Shared By", document.shared_by.as_ref()),
        document
            .size
            .map(|size| json!({"label": "Size", "value": weighed(size)})),
        (!document.id.is_empty())
            .then(|| json!({"label": "ID", "value": document.id, "identifies": true})),
    ]
    .into_iter()
    .flatten()
    .collect();
    let mut eyebrow: Vec<String> = vec![place(document, drives, words)];
    eyebrow.extend(kind_of(document, words));
    eyebrow.retain(|part| !part.is_empty());
    let count = eyebrow.len();
    let items: Vec<Value> = eyebrow
        .iter()
        .enumerate()
        .map(|(at, part)| json!({"said": part, "keep": count - at}))
        .collect();
    json!({
        "title": title,
        "facts": facts,
        "eyebrow": eyebrow.join(" \u{b7} "),
        "eyebrowItems": items,
        "eyebrowNames": true,
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
            |drive| words.on_drive.replace("{drive}", &drive.title),
        ),
        None if document.shared_with_me.is_some() => words.shared_with_me.clone(),
        None => words.mine.clone(),
    };
    words.within.replace("{place}", &place)
}

/// Its kind as the eyebrow says it after where it is: a file by its
/// ending ("PDF"), with what a picture measures and how long a video
/// runs; one of Google's other kinds by Drive's word for its type
/// ("Form"); the rest by their name.
fn kind_of(document: &Document, words: &Words) -> Vec<String> {
    const GOOGLES: &str = "application/vnd.google-apps.";
    let named = words.of(&kind(document.kind)).name;
    let kind = match document.kind {
        Is::File => ending(&document.title).unwrap_or(named),
        Is::Other => document
            .mime
            .strip_prefix(GOOGLES)
            .filter(|other| !other.is_empty())
            .map_or(named, capitalised),
        _ => named,
    };
    let measured = match (document.across, document.down) {
        (Some(across), Some(down)) if across > 0 && down > 0 => {
            format!("{across} \u{d7} {down}")
        }
        _ => String::new(),
    };
    let lasts = document.lasts.map(lasting).unwrap_or_default();
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

fn capitalised(word: &str) -> String {
    let mut letters = word.chars();
    letters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(letters).collect()
    })
}

/// A length in milliseconds as a player says one: 3:41, or 1:02:03.
fn lasting(milliseconds: u64) -> String {
    let seconds = (milliseconds + 500) / 1000;
    let (hours, minutes, rest) = (seconds / 3600, seconds % 3600 / 60, seconds % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{rest:02}")
    } else {
        format!("{minutes}:{rest:02}")
    }
}

/// A size in decimal units, as Mail.js's weighed says it, with a figure
/// after the point only below ten.
fn weighed(bytes: u64) -> String {
    const UNITS: [(f64, &str); 3] = [(1e9, "GB"), (1e6, "MB"), (1e3, "kB")];
    #[expect(clippy::cast_precision_loss, reason = "a size said to three figures")]
    let bytes = bytes as f64;
    UNITS.iter().find(|(unit, _)| bytes >= *unit).map_or_else(
        || format!("{bytes} B"),
        |(unit, name)| {
            let scaled = bytes / unit;
            if scaled < 10.0 {
                format!("{scaled:.1} {name}")
            } else {
                format!("{scaled:.0} {name}")
            }
        },
    )
}

#[cfg(test)]
#[path = "document_rows_tests.rs"]
mod tests;
