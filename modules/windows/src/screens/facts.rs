//! What desk-facts reads of each screen's EDID (modules/desk-facts), said
//! in the launcher's words: in the screen's drawer, and as the rows
//! inside its row that a step in offers after its settings and the ways
//! it turns off, each copying what it says. desk-facts writes it to a
//! file this daemon follows (main.rs's `follow_desk`), so the shell
//! reads no file for it.

use ready_rows::numbers::Numbers;
use serde_json::{Value, json};

use super::{APART, PLACE, Words, key};

/// The facts desk-facts gives that are asked of a screen most, which lead
/// the rest, its white point and primaries beside its colours (VKCF3Z,
/// 2026-11-09).
const ASKED_FIRST: [&str; 5] = ["depth", "colours", "white", "primaries", "hdr"];

/// The places a chromaticity's coordinate is said to: ten bits a
/// coordinate hold one to about a thousandth, so "0.060" says all an
/// EDID knows.
const COORDINATE_PLACES: usize = 3;

/// The figures desk-facts writes for a fact to say, or none where there
/// aren't `count` of them.
fn figures(given: &str, count: usize) -> Option<Vec<f64>> {
    let all: Vec<f64> = given
        .split_whitespace()
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    (all.len() == count && all.iter().all(|figure| figure.is_finite())).then_some(all)
}

/// A point on the CIE 1931 chart in a part's words, each coordinate named
/// so nobody has to remember the order (VKCF3Z, 2026-11-09), to three
/// places, pointed as the account says: "x 0.313 y 0.329". A coordinate
/// halfway between rounds up, as the shell's toFixed did.
fn xy(said: &str, x: f64, y: f64, numbers: &Numbers) -> String {
    let scale = 10f64.powi(3);
    let placed = |value: f64| numbers.fixed((value * scale).round() / scale, COORDINATE_PLACES);
    said.replace("{x}", &placed(x)).replace("{y}", &placed(y))
}

/// What desk-facts says of a screen from its EDID, each named first as a
/// part is, with its id: Colour Depth "10 bit", Colours, its white point
/// and primaries, HDR, then who made it and when, in desk-facts' order.
/// A monitor's brightness is the panel's, which its light says, so it
/// isn't said again, and a white point or primaries whose figures aren't
/// whole say nothing.
pub(super) fn edid(facts: &Value, words: &Words) -> Vec<(String, Value)> {
    let told = &words.told;
    let said: Vec<(String, String)> = facts
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|fact| {
            let id = fact["name"].as_str()?.to_lowercase();
            let value = fact["value"].as_str().unwrap_or("");
            let value = match id.as_str() {
                "brightness" => return None,
                "white" => figures(value, 3).map_or_else(String::new, |at| {
                    format!(
                        "{} K{APART}{}",
                        words.numbers.significant(at[0]),
                        xy(&told.white, at[1], at[2], &words.numbers)
                    )
                }),
                "primaries" => figures(value, 6).map_or_else(String::new, |at| {
                    told.primaries
                        .iter()
                        .zip(at.chunks(2))
                        .map(|(said, point)| xy(said, point[0], point[1], &words.numbers))
                        .collect::<Vec<_>>()
                        .join(APART)
                }),
                _ => value.to_owned(),
            };
            (!value.is_empty()).then_some((id, value))
        })
        .collect();
    let (first, rest): (Vec<_>, Vec<_>) = said
        .into_iter()
        .partition(|(id, _)| ASKED_FIRST.contains(&id.as_str()));
    first
        .into_iter()
        .chain(rest)
        .map(|(id, value)| {
            let fact = json!({"name": told.part(&id), "said": value});
            (id, fact)
        })
        .collect()
}

/// Whether a monitor's brightness follows the panel's, as desk-facts says
/// by the brightness it read back from it, so its place can hold the
/// Brightness when no other does.
pub(super) fn follows(facts: &Value) -> bool {
    facts.as_array().into_iter().flatten().any(|fact| {
        fact["name"]
            .as_str()
            .is_some_and(|said| said.eq_ignore_ascii_case("brightness"))
    })
}

/// What the EDID of the screen of connector `name` says, as rows inside
/// its row (`screen`, its key), in `edid`'s order, each copying what it
/// says, as the shell's own facts did.
pub(super) fn rows(name: &str, screen: &str, facts: &Value, words: &Words) -> Vec<Value> {
    edid(facts, words)
        .into_iter()
        .map(|(id, fact)| {
            let said = fact["said"].as_str().unwrap_or_default();
            json!({
                "key": key(&format!("fact:{name}:{id}")),
                // The key the shell gave the fact while it made it.
                "thing": format!("fact:{PLACE}:{name}:{id}"),
                "place": PLACE,
                "kind": "fact",
                "parent": screen,
                "name": fact["name"],
                "glyph": "",
                "hue": "voice",
                "depth": 1,
                "side": [{"said": said}],
                "acts": [{"key": "verb:copy", "copy": said}],
            })
        })
        .collect()
}
