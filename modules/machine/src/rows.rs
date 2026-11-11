//! What every place's ready rows share, in Y2YBTS's ready-row form
//! (QEST-20261023-Y2YBTS attachments/speed/ready-rows.md): a row as it's
//! made, each placed under its parent with its depth, in the order shown,
//! and the graphs worked out whole in the steps the shell draws, so a
//! reading that leaves a drawing as it was changes nothing and the row
//! isn't sent again. Each place's rows are given their spaced order as
//! they're kept (ready-rows' `Kept`, in `main.rs`), each keeping its last,
//! so a row coming or going sends itself alone. Heat's rows ([`crate::heat_rows`])
//! are made with it, and Power's, Firmware's and Machine's own are to be.

use serde_json::{Map, Value, json};

/// How many reads a spark holds, two to each of its ten characters.
pub const GRAPHED: usize = 20;
/// The steps a bar fills in, one to each place of its twenty.
pub const BAR_STEPS: f64 = 20.0;
/// The steps a spark's or strip's reading is drawn in, four dots high.
pub const DOT_STEPS: f64 = 4.0;

/// A place's rows and its headline, each a record the daemon serves.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ready {
    pub rows: Vec<Value>,
    pub head: Option<Value>,
}

/// One row as it's made, before it's placed among its siblings: what it
/// copies is its first act, then any of its own, then the step inside
/// where it holds rows.
#[derive(Default)]
pub struct Made {
    pub id: String,
    pub kind: &'static str,
    pub name: String,
    pub glyph: String,
    pub hue: &'static str,
    pub side: Vec<Value>,
    pub copies: String,
    pub acts: Vec<Value>,
    pub graph: Option<Value>,
    pub note: String,
    /// A note that's a time, which the shell words as it says it, in
    /// place of `note`, as "as its controller said at 06:06" is.
    pub note_time: Option<Value>,
    pub drawer: Option<Value>,
    pub inside: Vec<Made>,
    /// Whether it's the step that asks Joel to be sure of an act that
    /// can't be taken back, so its Return does the act (Y2YBTS,
    /// 2026-11-11).
    pub sure: bool,
    /// Whether it's an act done with, drawn dimmed and asking nothing,
    /// such as an install that's gone in (Y2YBTS, 2026-11-11).
    pub gone: bool,
}

/// A row that says a figure and copies it.
pub fn figure(id: String, name: String, said: String, graph: Option<Value>, note: String) -> Made {
    Made {
        id,
        kind: "figure",
        name,
        side: vec![item(&said)],
        copies: said,
        graph,
        note,
        ..Made::default()
    }
}

/// A side item that says a figure.
pub fn item(said: &str) -> Value {
    json!({ "said": said })
}

/// A value kept between 0 and 1, in `steps` steps, any above nothing at
/// least one, as the shell lights it.
pub fn stepped(value: f64, steps: f64) -> f64 {
    let kept = if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, 1.0)
    };
    if kept <= 0.0 {
        0.0
    } else {
        (kept * steps).round().max(1.0) / steps
    }
}

pub fn bar(value: f64, warn: bool) -> Value {
    json!({ "shape": "bar", "values": [stepped(value, BAR_STEPS)], "warn": warn })
}

/// A strip: one value a member, as many members to a place as it takes
/// to fit twenty, each place its most.
pub fn strip(values: &[f64], warn: bool) -> Value {
    let per = values.len().div_ceil(GRAPHED).max(1);
    let drawn: Vec<f64> = values
        .chunks(per)
        .map(|chunk| stepped(chunk.iter().copied().fold(0.0, f64::max), DOT_STEPS))
        .collect();
    json!({ "shape": "strip", "values": drawn, "warn": warn })
}

/// A spark of the last reads, newest last, each already a share of its
/// span.
pub fn spark(values: &[f64]) -> Value {
    let from = values.len().saturating_sub(GRAPHED);
    let drawn: Vec<f64> = values[from..]
        .iter()
        .map(|value| stepped(*value, DOT_STEPS))
        .collect();
    json!({ "shape": "spark", "values": drawn, "warn": false })
}

/// Each made row as a record of `place`, in the order it's shown, each
/// under its parent. What it copies is its first act, which the shell
/// copies itself and never asks back (Y2YBTS, 2026-11-10).
pub fn placed(
    place: &str,
    made: Vec<Made>,
    parent: Option<&str>,
    depth: u64,
    out: &mut Vec<Value>,
) {
    placed_as(place, place, made, parent, depth, out);
}

/// As [`placed`], each row's thing named under `things` rather than its
/// place, as the Disk's rows are a place of their own, read only while
/// it's watched, though each is Machine's (`machine:disk:speed`).
pub fn placed_as(
    place: &str,
    things: &str,
    made: Vec<Made>,
    parent: Option<&str>,
    depth: u64,
    out: &mut Vec<Value>,
) {
    for row in made {
        let key = format!("row:{place}:{}", row.id);
        let mut record = Map::new();
        record.insert("key".to_owned(), Value::String(key.clone()));
        record.insert("thing".to_owned(), json!(format!("{things}:{}", row.id)));
        record.insert("place".to_owned(), json!(place));
        record.insert("kind".to_owned(), json!(row.kind));
        if let Some(parent) = parent {
            record.insert("parent".to_owned(), json!(parent));
        }
        record.insert("depth".to_owned(), json!(depth));
        record.insert("name".to_owned(), json!(row.name));
        let mut set = |field: &str, value: Value| {
            record.insert(field.to_owned(), value);
        };
        if !row.glyph.is_empty() {
            set("glyph", json!(row.glyph));
        }
        if !row.hue.is_empty() {
            set("hue", json!(row.hue));
        }
        set("side", Value::Array(row.side));
        if let Some(graph) = row.graph {
            set("graph", graph);
        }
        if let Some(time) = row.note_time {
            set("note", time);
        } else if !row.note.is_empty() {
            set("note", json!(row.note));
        }
        if let Some(drawer) = row.drawer {
            set("drawer", drawer);
        }
        if row.sure {
            set("sure", json!(true));
        }
        if row.gone {
            set("gone", json!(true));
        }
        let mut acts = Vec::new();
        if !row.copies.is_empty() {
            acts.push(json!({ "key": "verb:copy", "copy": row.copies }));
        }
        acts.extend(row.acts);
        if !row.inside.is_empty() {
            acts.push(json!({ "key": "verb:more", "step": "rows" }));
        }
        set("acts", Value::Array(acts));
        out.push(Value::Object(record));
        placed_as(
            place,
            things,
            row.inside,
            Some(key.as_str()),
            depth + 1,
            out,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_are_the_shells() {
        let stepped = [0.0, 0.001, 0.5, 0.52, 2.0, f64::NAN].map(|v| stepped(v, BAR_STEPS));
        assert_eq!(stepped, [0.0, 0.05, 0.5, 0.5, 1.0, 0.0]);
        assert_eq!(
            strip(&[0.1, 0.9, 0.3], true)["values"],
            json!([0.25, 1.0, 0.25])
        );
    }

    #[test]
    fn a_row_inside_names_its_parent_and_depth() {
        let mut made = figure(
            "a".to_owned(),
            "A".to_owned(),
            "1".to_owned(),
            None,
            String::new(),
        );
        made.inside = vec![figure(
            "a/b".to_owned(),
            "B".to_owned(),
            "2".to_owned(),
            None,
            String::new(),
        )];
        let mut out = Vec::new();
        placed("p", vec![made], None, 0, &mut out);
        assert_eq!(
            out[0]["acts"],
            json!([{ "key": "verb:copy", "copy": "1" }, { "key": "verb:more", "step": "rows" }])
        );
        assert_eq!(
            (out[1]["parent"].clone(), out[1]["depth"].clone()),
            (json!("row:p:a"), json!(1))
        );
    }
}
