//! The Warmth inside the screen whose place holds the light: its row,
//! saying the warmth now and what holds it, and inside it the Night Light
//! and Follow the Room switches, then the warmth's figures, from what the
//! backlight program says of it (modules/machine's steered.rs).
//! Displays.qml built these rows from the same record (Warmth.js) until
//! 2026-11-14; they're served here now, so the shell only draws them
//! (Y2YBTS's rule, 17a4dc7e). A switch's Return asks this daemon, which
//! asks the machine daemon (main.rs's `light_done`).
//!
//! A time is served as a moment, `{at, as}`, which the shell words by its
//! own clock as it draws the row and as Return copies it, so the daemon
//! words the clock nowhere (ready-rows.md's Times).

use ready_rows::numbers::Numbers;
use serde_json::{Value, json};

use super::{Inside, Light, Warmth, Words, runs};
use crate::screens::{PLACE, key};

/// The switches inside the Warmth, by the vocabulary's id of each, and
/// the act the machine daemon knows it by with what that act takes.
const NIGHT_LIGHT: (&str, &str, &str) = ("night-light", "night-light", "on");
const FOLLOW_THE_ROOM: (&str, &str, &str) = ("follow-room", "follow-the-room", "follows");

impl Warmth {
    /// Whether the night under way holds the moment `now`.
    pub(super) fn dark(&self, now: u64) -> bool {
        matches!((self.sunset, self.sunrise),
            (Some(sunset), Some(sunrise)) if sunset <= now && now < sunrise)
    }
}

/// A kelvin with its thousands grouped: "4,000 K" (VKCF3Z, 2026-11-08).
pub(super) fn kelvin(value: f64, numbers: &Numbers) -> String {
    format!("{} K", numbers.number(value, 0))
}

/// What holds the warmth now, in a few words: the night light until
/// sunrise, or from sunset, either time the shell's to say, or waiting for
/// where Joel is; else the room, or the screen's own white.
fn note(warmth: &Warmth, now: u64, words: &Words) -> Value {
    let said = |id: &str| words.warmth.get(id).map_or("", String::as_str);
    match (warmth.night_light, warmth.sunset, warmth.sunrise) {
        (true, None, _) => json!(said("waits")),
        (true, _, Some(sunrise)) if warmth.dark(now) => {
            json!({"at": sunrise, "as": "moment", "said": said("until")})
        }
        (true, Some(sunset), _) => json!({"at": sunset, "as": "moment", "said": said("from")}),
        (false, ..) if warmth.follows_the_room && warmth.sees_colour => json!(said("following")),
        (false, ..) => json!(said("own")),
    }
}

/// One of the warmth's figures: its id after `warmth:`, and what it says,
/// words or a time.
struct Figure {
    id: &'static str,
    said: Value,
}

/// The figures, from what moves most: the room's white where a sensor
/// sees it, the night's sunset and sunrise, whichever comes next first,
/// so by night the sunrise leads (VKCF3Z, 2026-11-08), then the night's
/// white; each only where it's known.
fn figures(warmth: &Warmth, inside: &Inside) -> Vec<Figure> {
    let kelvins = |id, value: Option<f64>| {
        value.map(|value| Figure {
            id,
            said: json!(kelvin(value, inside.numbers)),
        })
    };
    let moment = |id, at: Option<u64>| {
        at.map(|at| Figure {
            id,
            said: json!({"at": at, "as": "moment"}),
        })
    };
    let mut sun = vec![
        moment("sunset", warmth.sunset),
        moment("sunrise", warmth.sunrise),
    ];
    if warmth.dark(inside.now) {
        sun.reverse();
    }
    let room = kelvins("room", warmth.room_white.filter(|_| warmth.sees_colour));
    std::iter::once(room)
        .chain(sun)
        .chain([kelvins("night", warmth.night_white)])
        .flatten()
        .collect()
}

/// A switch of the Warmth's, on or off, its Return asking the machine
/// daemon to flip it, the launcher staying to show it change.
fn switch(
    (id, act, takes): (&str, &str, &str),
    on: bool,
    warmth: &str,
    inside: &Inside,
    words: &Words,
) -> Value {
    let name = inside.name;
    let mut asked = json!({"ask": "light", "do": act});
    asked[takes] = json!(!on);
    json!({
        "key": key(&format!("switch:{name}:{id}")),
        "thing": format!("{PLACE}:{name}:{id}"),
        "place": PLACE,
        "kind": "switch",
        "name": (inside.named)(id),
        "glyph": words.glyphs.get(id).cloned().unwrap_or_default(),
        "hue": "machine",
        "parent": warmth,
        "depth": 2,
        "side": [{"said": if on { &words.on } else { &words.off }}],
        "held": {"on": on},
        "acts": [{"key": format!("{PLACE}:{id}"), "ask": asked}],
    })
}

/// The Warmth's row inside `inside`'s screen, then what a step into it
/// holds: the Night Light, Follow the Room where a sensor sees colour,
/// then its figures, each copying what it says; none where nothing warms
/// the screen, as when the backlight program has stopped.
#[must_use]
pub fn rows(light: &Light, inside: &Inside, words: &Words) -> Vec<Value> {
    let Some(warmth) = &light.warmth else {
        return Vec::new();
    };
    let name = inside.name;
    let said = |id: &str| words.warmth.get(id).map_or("", String::as_str);
    let title = (inside.named)("warmth");
    let figured = figures(warmth, inside);
    let facts: Vec<Value> = figured
        .iter()
        .map(|figure| {
            let mut fact = json!({"name": (inside.named)(&format!("warmth:{}", figure.id))});
            match &figure.said {
                Value::String(told) => fact["said"] = json!(told),
                time => {
                    fact["at"] = time["at"].clone();
                    fact["as"] = time["as"].clone();
                }
            }
            fact
        })
        .collect();
    let row = key(&format!("warmth:{name}"));
    let mut rows = vec![json!({
        "key": row,
        "thing": format!("{PLACE}:{name}:warmth"),
        "place": PLACE,
        "kind": "warmth",
        "name": title,
        "glyph": "warmth",
        "hue": "machine",
        "parent": inside.screen,
        "depth": 1,
        "side": [{"said": kelvin(warmth.kelvin, inside.numbers)}],
        "note": note(warmth, inside.now, words),
        // Where it's set, then what it warms, which gives way first where
        // the plate is narrow (VKCF3Z: where, then kind).
        "drawer": {
            "eyebrow": runs(&[said("via"), said("screen")]),
            "eyebrowItems": [{"said": said("via"), "keep": 2}, {"said": said("screen"), "keep": 1}],
            "eyebrowNames": true,
            "title": title,
            "facts": facts,
        },
        "acts": [{"key": "verb:more", "step": "rows"}],
    })];
    rows.push(switch(NIGHT_LIGHT, warmth.night_light, &row, inside, words));
    if warmth.sees_colour {
        rows.push(switch(
            FOLLOW_THE_ROOM,
            warmth.follows_the_room,
            &row,
            inside,
            words,
        ));
    }
    rows.extend(figured.into_iter().map(|figure| {
        let id = figure.id;
        let side = match &figure.said {
            Value::String(told) => json!([{"said": told}]),
            time => json!([time]),
        };
        json!({
            "key": key(&format!("fact:{name}:warmth-{id}")),
            "thing": format!("fact:{PLACE}:{name}:warmth-{id}"),
            "place": PLACE,
            "kind": "fact",
            "parent": row,
            "name": (inside.named)(&format!("warmth:{id}")),
            "glyph": "",
            "hue": "voice",
            "depth": 2,
            "side": side,
            "acts": [{"key": "verb:copy", "copy": figure.said}],
        })
    }));
    rows
}

#[cfg(test)]
mod tests;
