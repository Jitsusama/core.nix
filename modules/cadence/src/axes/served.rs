//! The axes as the launcher's Spec place shows them, in Y2YBTS's ready-row
//! form (QEST-20261023-Y2YBTS attachments/speed/ready-rows.md): a row an
//! axis in cadence's order, a row a reading under it with his side's
//! first, and the place's headline. Everything Spec says is worked out
//! here, so the shell only draws it (Y2YBTS, 2026-11-14, 489bd79e).
//!
//! Only his side's readings count toward what's out of spec, since no
//! axis is in spec on a stand-in (Joel, 2026-11-11 21:39), and colour goes
//! on a mark, never on text (the ink ruling, df110a81): a row with one of
//! his out wears the failed mark, a stand-in the pending one. A reading's
//! owner is served by address, so Return opens the owner's quest in
//! Agents, a step the shell takes (6632bb36).

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::words::Words;
use super::{Axis, Bar, Reading};

/// The place the rows are served in.
pub const PLACE: &str = "spec";

/// What Spec shows: its rows in the order shown, without the order Kept
/// gives them, and its headline beside them.
#[derive(Debug, Clone, PartialEq)]
pub struct Made {
    pub rows: Vec<Value>,
    pub head: Value,
}

/// Spec's rows and headline from `axes`, each owner named by the title of
/// its quest in `titles` (by address), or said to have none, since an
/// address on his screen names nothing he knows (VKCF3Z, 694b965f).
#[must_use]
pub fn made(axes: &[Axis], titles: &BTreeMap<String, String>, words: &Words) -> Made {
    let mut rows = Vec::new();
    let mut out = 0;
    for axis in axes {
        out += axis.readings.iter().filter(|r| r.his && r.out()).count();
        rows.push(axis_row(axis, words));
        let mut readings: Vec<(usize, &Reading)> = axis.readings.iter().enumerate().collect();
        readings.sort_by_key(|(_, reading)| !reading.his);
        for (n, reading) in readings {
            rows.push(reading_row(axis, n, reading, titles, words));
        }
    }
    let count = out.to_string();
    let mut head = json!({
        "key": format!("head:{PLACE}"),
        "place": PLACE,
        // It counts readings, not axes, and says so: "14 out of spec"
        // beside ten axes read as 14 of 10 (VKCF3Z, 694b965f).
        "said": if out == 0 {
            words.said("spec:head", "none", &[])
        } else if out == 1 {
            words.said("spec:head", "one", &[])
        } else {
            words.said("spec:head", "head", &[("count", &count)])
        },
    });
    if out > 0 {
        head["glyph"] = json!("failed");
    }
    Made { rows, head }
}

/// An axis's slug, as its key names it: "time to done" is time-to-done.
fn slug(axis: &Axis) -> String {
    axis.name.replace(' ', "-")
}

/// An axis's name as a row shows it, in Title Case: "Time to Done".
fn titled(name: &str) -> String {
    const SMALL: [&str; 6] = ["to", "of", "and", "a", "the", "in"];
    name.split(' ')
        .enumerate()
        .map(|(n, word)| {
            if n > 0 && SMALL.contains(&word) {
                return word.to_owned();
            }
            let mut letters = word.chars();
            letters.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(letters).collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// How many of `readings` are out, of how many are read and shown.
fn tally<'a>(readings: impl Iterator<Item = &'a Reading>) -> (usize, usize) {
    readings.fold((0, 0), |(out, of), reading| {
        let read = reading.now.is_some() && reading.held.is_none();
        (out + usize::from(reading.out()), of + usize::from(read))
    })
}

fn axis_row(axis: &Axis, words: &Words) -> Value {
    let (yours_out, yours) = tally(axis.readings.iter().filter(|r| r.his));
    let (ours_out, ours) = tally(axis.readings.iter().filter(|r| !r.his));
    let unread = yours + ours == 0;
    let (said, glyph) = if yours_out > 0 {
        let count = yours_out.to_string();
        (
            words.said("spec:axis", "out", &[("count", &count)]),
            "failed",
        )
    } else if unread {
        (words.said("spec:axis", "unread", &[]), "")
    } else if axis.readings.iter().all(|r| r.his && r.now.is_some()) {
        (words.said("spec:axis", "in", &[]), "")
    } else {
        (words.said("spec:axis", "stand-ins", &[]), "pending")
    };
    let of = |out: usize, of: usize| {
        let (out, of) = (out.to_string(), of.to_string());
        words.said("spec:axis:drawer", "of", &[("out", &out), ("of", &of)])
    };
    let name = titled(axis.name);
    let mut row = json!({
        "key": format!("row:{PLACE}:{}", slug(axis)),
        "thing": format!("axis:{}", slug(axis)),
        "place": PLACE,
        "kind": "axis",
        "depth": 0,
        "name": name,
        "side": [{"said": said}],
        "acts": [{"key": "verb:more", "step": "rows"}],
        "drawer": {
            "eyebrow": words.said("spec:axis:drawer", "eyebrow", &[("axis", axis.name)]),
            "title": name,
            "facts": [
                {"name": words.name("spec:yours-out"), "said": of(yours_out, yours)},
                {"name": words.name("spec:ours-out"), "said": of(ours_out, ours)},
            ],
        },
    });
    if !glyph.is_empty() {
        row["glyph"] = json!(glyph);
    }
    row
}

/// A figure as the nerd facts give it: to three places at most, trailing
/// zeros dropped, ungrouped, so 4.1402777 h reads 4.14 h.
fn raw(figure: f64, unit: &str) -> String {
    let fixed = format!("{figure:.3}");
    let trimmed = fixed.trim_end_matches('0').trim_end_matches('.');
    format!("{trimmed}{unit}")
}

fn reading_row(
    axis: &Axis,
    n: usize,
    reading: &Reading,
    titles: &BTreeMap<String, String>,
    words: &Words,
) -> Value {
    let owner = titles
        .get(reading.owner)
        .cloned()
        .unwrap_or_else(|| words.said("spec:reading:owner", "untitled", &[]));
    let on = if reading.his { "yours" } else { "ours" };
    let mut facts = vec![json!({"name": words.name("spec:reading:owner"), "said": owner})];
    if !reading.from.is_empty() {
        facts.push(json!({"name": words.name("spec:reading:from"), "said": reading.from}));
    }
    facts.push(json!({
        "name": words.name("spec:reading:on"),
        "said": words.said("spec:reading:on", on, &[]),
    }));
    let mut nerd = vec![json!({"label": "owner", "value": reading.owner})];
    if let Some(figure) = reading.now.filter(|_| reading.held.is_none()) {
        nerd.push(json!({"label": "figure", "value": raw(figure, reading.unit)}));
    }
    if let Some(Bar::AtMost(bar) | Bar::AtLeast(bar)) = reading.bar {
        nerd.push(json!({"label": "bar", "value": raw(bar, reading.unit)}));
    }
    let said = reading.words();
    let mut drawer = json!({
        "eyebrow": words.said("spec:axis:drawer", "eyebrow", &[("axis", axis.name)]),
        "title": said,
        "facts": facts,
        "nerd": nerd,
    });
    if let Some(detail) = &reading.detail {
        drawer["body"] = json!(detail);
    }
    let parent = format!("row:{PLACE}:{}", slug(axis));
    let mut row = json!({
        "key": format!("{parent}:{n}"),
        "thing": format!("axis:{}:{n}", slug(axis)),
        "place": PLACE,
        "kind": "reading",
        "parent": parent,
        "depth": 1,
        "name": said,
        "side": [{"said": owner}],
        "owner": reading.owner,
        "acts": [],
        "drawer": drawer,
    });
    if reading.his && reading.out() {
        row["glyph"] = json!("failed");
    } else if !reading.his {
        row["glyph"] = json!("pending");
    }
    row
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The launcher's own words for Spec in tests/words.json, which become
    /// its place file with Spec.qml; the-daemons-test-the-launchers-words
    /// keeps the two the same from then.
    fn words() -> Words {
        let text = include_str!("../../tests/words.json");
        Words::new(&serde_json::from_str(text).expect("the test's words are JSON"))
    }

    fn axes() -> Vec<Axis> {
        vec![
            Axis {
                name: "time to done",
                findings: Vec::new(),
                readings: vec![
                    Reading::new("VM wait {}", Some(3.0), " min", "5FWIUS")
                        .bar(Bar::AtMost(2.0))
                        .from("the builders' queue"),
                    Reading::new("{} asks past their date", Some(2.0), "", "WCJQSG")
                        .bar(Bar::AtMost(0.0))
                        .from("the asks ledger")
                        .detail(Some("4 owners".into()))
                        .his(),
                ],
            },
            Axis {
                name: "scale",
                findings: Vec::new(),
                readings: vec![
                    Reading::new("{} pi windows", Some(10.0), "", "OSPHT4")
                        .bar(Bar::AtMost(12.0))
                        .his(),
                ],
            },
            Axis {
                name: "security",
                findings: Vec::new(),
                readings: vec![
                    Reading::new("{} holes", Some(1.0), "", "ISXEG8").bar(Bar::AtMost(0.0)),
                ],
            },
            Axis {
                name: "polish",
                findings: Vec::new(),
                readings: vec![Reading::new("{} slow keys", None, "", "Y2YBTS").his()],
            },
        ]
    }

    fn titles() -> BTreeMap<String, String> {
        BTreeMap::from([("WCJQSG".to_owned(), "Joel's Orchestration".to_owned())])
    }

    fn row<'a>(made: &'a Made, key: &str) -> &'a Value {
        made.rows
            .iter()
            .find(|row| row["key"] == key)
            .unwrap_or_else(|| panic!("no row {key} among {:?}", made.rows))
    }

    #[test]
    fn the_headline_counts_only_his_readings_out() {
        let made = made(&axes(), &titles(), &words());
        assert_eq!(made.head["key"], "head:spec");
        assert_eq!(made.head["said"], "1 reading out of spec");
        assert_eq!(made.head["glyph"], "failed");
        assert!(
            made.head.get("hue").is_none(),
            "the headline's text takes no hue"
        );
    }

    #[test]
    fn the_headline_says_it_counts_readings_not_axes() {
        let mut axes = axes();
        axes[1].readings[0] = Reading::new("{} pi windows", Some(14.0), "", "OSPHT4")
            .bar(Bar::AtMost(12.0))
            .his();
        let made = made(&axes, &titles(), &words());
        assert_eq!(made.head["said"], "2 readings out of spec");
    }

    #[test]
    fn a_spec_with_none_of_his_out_says_so_and_wears_no_mark() {
        let mut axes = axes();
        axes.retain(|axis| axis.name != "time to done");
        let made = made(&axes, &titles(), &words());
        assert_eq!(made.head["said"], "all in spec");
        assert!(
            made.head.get("glyph").is_none(),
            "no mark while none of his is out"
        );
    }

    #[test]
    fn each_axis_says_where_it_stands_marking_only_his_out() {
        let made = made(&axes(), &titles(), &words());
        let standing = |key: &str| {
            let row = row(&made, key);
            (row["side"][0]["said"].clone(), row["glyph"].clone())
        };
        assert_eq!(
            standing("row:spec:time-to-done"),
            (json!("1 out"), json!("failed"))
        );
        assert_eq!(standing("row:spec:scale"), (json!("in spec"), Value::Null));
        assert_eq!(
            standing("row:spec:security"),
            (json!("read on stand-ins"), json!("pending"))
        );
        assert_eq!(
            standing("row:spec:polish"),
            (json!("not read yet"), Value::Null)
        );
        assert_eq!(row(&made, "row:spec:time-to-done")["name"], "Time to Done");
    }

    #[test]
    fn an_axis_drawer_counts_his_and_ours_apart() {
        let made = made(&axes(), &titles(), &words());
        let drawer = &row(&made, "row:spec:time-to-done")["drawer"];
        assert_eq!(drawer["eyebrow"], "from cadence · time to done");
        assert_eq!(
            drawer["facts"],
            json!([
                {"name": "Your Use Out", "said": "1 of 1"},
                {"name": "Stand-ins Out", "said": "1 of 1"},
            ])
        );
    }

    #[test]
    fn his_readings_come_first_under_their_axis_keeping_their_keys() {
        let made = made(&axes(), &titles(), &words());
        let keys: Vec<&str> = made.rows.iter().filter_map(|r| r["key"].as_str()).collect();
        assert_eq!(
            keys[..3],
            [
                "row:spec:time-to-done",
                "row:spec:time-to-done:1",
                "row:spec:time-to-done:0"
            ]
        );
        let his = row(&made, "row:spec:time-to-done:1");
        assert_eq!(his["parent"], "row:spec:time-to-done");
        assert_eq!(his["depth"], 1);
        assert_eq!(his["name"], "2 asks past their date, at most 0");
        assert_eq!(his["glyph"], "failed");
        assert_eq!(row(&made, "row:spec:time-to-done:0")["glyph"], "pending");
    }

    #[test]
    fn a_reading_serves_its_owner_by_address_and_names_their_quest() {
        let made = made(&axes(), &titles(), &words());
        let his = row(&made, "row:spec:time-to-done:1");
        assert_eq!(his["owner"], "WCJQSG");
        assert_eq!(his["side"], json!([{"said": "Joel's Orchestration"}]));
        assert_eq!(
            his["drawer"]["facts"],
            json!([
                {"name": "Owner", "said": "Joel's Orchestration"},
                {"name": "Read From", "said": "the asks ledger"},
                {"name": "Read On", "said": "your own use"},
            ])
        );
        assert_eq!(his["drawer"]["body"], "4 owners");
        assert_eq!(raw(4.140_277_7, " h"), "4.14 h");
        assert_eq!(raw(20.0, " min"), "20 min");
        let untitled = row(&made, "row:spec:time-to-done:0");
        assert_eq!(
            untitled["side"],
            json!([{"said": "a session with no quest"}])
        );
        assert_eq!(
            untitled["drawer"]["facts"][0]["said"],
            "a session with no quest"
        );
        assert_eq!(untitled["drawer"]["facts"][2]["said"], "a stand-in");
        assert_eq!(
            untitled["owner"], "5FWIUS",
            "Return still finds it by address"
        );
    }
}
