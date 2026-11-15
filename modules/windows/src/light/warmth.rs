//! The Warmth, Screens' own beside the screens, with the lid open or
//! shut, since the night light warms every screen (VKCF3Z, 2026-11-15):
//! its row, saying the warmth now and what holds it, and inside it the
//! Night Light, then Follow the Room while a screen in the room is lit,
//! since only the panel has the room's light to follow, then the
//! warmth's figures, from what the backlight program says of it
//! (modules/machine's steered.rs).
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

use super::{Light, Warmth, Words, runs};
use crate::screens::{PLACE, key};

/// What the Warmth's rows are made with: a part's name as the vocabulary
/// names it (`displays:<id>`), the numbers' grouping, the time now, in
/// milliseconds since the epoch, which says whether the night is under
/// way, and whether a lit screen holds the room's sensor, so the room is
/// there to follow.
pub struct Warmed<'a> {
    pub named: &'a dyn Fn(&str) -> String,
    pub numbers: &'a Numbers,
    pub now: u64,
    pub room: bool,
}

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
/// where Joel is; else the room, where there's one to follow, or the
/// screen's own white.
fn note(warmth: &Warmth, warmed: &Warmed, words: &Words) -> Value {
    let now = warmed.now;
    let said = |id: &str| words.warmth.get(id).map_or("", String::as_str);
    match (warmth.night_light, warmth.sunset, warmth.sunrise) {
        (true, None, _) => json!(said("waits")),
        (true, _, Some(sunrise)) if warmth.dark(now) => {
            json!({"at": sunrise, "as": "moment", "said": said("until")})
        }
        (true, Some(sunset), _) => json!({"at": sunset, "as": "moment", "said": said("from")}),
        (false, ..) if warmth.follows_the_room && sees_the_room(warmth, warmed) => {
            json!(said("following"))
        }
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
fn figures(warmth: &Warmth, warmed: &Warmed) -> Vec<Figure> {
    let kelvins = |id, value: Option<f64>| {
        value.map(|value| Figure {
            id,
            said: json!(kelvin(value, warmed.numbers)),
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
    if warmth.dark(warmed.now) {
        sun.reverse();
    }
    let room = kelvins(
        "room",
        warmth.room_white.filter(|_| sees_the_room(warmth, warmed)),
    );
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
    warmed: &Warmed,
    words: &Words,
) -> Value {
    let mut asked = json!({"ask": "light", "do": act});
    asked[takes] = json!(!on);
    json!({
        "key": key(&format!("switch:{id}")),
        "thing": format!("{PLACE}:{id}"),
        "place": PLACE,
        "kind": "switch",
        "name": (warmed.named)(id),
        "glyph": words.glyphs.get(id).cloned().unwrap_or_default(),
        "hue": "machine",
        "parent": warmth,
        "depth": 1,
        "side": [{"said": if on { &words.on } else { &words.off }}],
        "held": {"on": on},
        "acts": [{"key": format!("{PLACE}:{id}"), "ask": asked}],
    })
}

/// Whether the room's light is there for the warmth to follow: a sensor
/// that sees colour, in a lit screen's room.
fn sees_the_room(warmth: &Warmth, warmed: &Warmed) -> bool {
    warmth.sees_colour && warmed.room
}

/// The Warmth's row, Screens' own, then what a step into it holds: the
/// Night Light, Follow the Room where the room's light is there to
/// follow, then its figures, each copying what it says; none where
/// nothing warms the screens, as when the backlight program has stopped.
#[must_use]
pub fn rows(light: &Light, warmed: &Warmed, words: &Words) -> Vec<Value> {
    let Some(warmth) = &light.warmth else {
        return Vec::new();
    };
    let said = |id: &str| words.warmth.get(id).map_or("", String::as_str);
    let title = (warmed.named)("warmth");
    let figured = figures(warmth, warmed);
    let facts: Vec<Value> = figured
        .iter()
        .map(|figure| {
            let mut fact = json!({"name": (warmed.named)(&format!("warmth:{}", figure.id))});
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
    let row = key("warmth");
    let mut rows = vec![json!({
        "key": row,
        "thing": format!("{PLACE}:warmth"),
        "place": PLACE,
        "kind": "warmth",
        "name": title,
        "glyph": "warmth",
        "hue": "machine",
        "depth": 0,
        "side": [{"said": kelvin(warmth.kelvin, warmed.numbers)}],
        "note": note(warmth, warmed, words),
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
    rows.push(switch(NIGHT_LIGHT, warmth.night_light, &row, warmed, words));
    if sees_the_room(warmth, warmed) {
        rows.push(switch(
            FOLLOW_THE_ROOM,
            warmth.follows_the_room,
            &row,
            warmed,
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
            "key": key(&format!("fact:warmth-{id}")),
            "thing": format!("fact:{PLACE}:warmth-{id}"),
            "place": PLACE,
            "kind": "fact",
            "parent": row,
            "name": (warmed.named)(&format!("warmth:{id}")),
            "glyph": "",
            "hue": "voice",
            "depth": 1,
            "side": side,
            "acts": [{"key": "verb:copy", "copy": figure.said}],
        })
    }));
    rows
}

#[cfg(test)]
mod tests;
