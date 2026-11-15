//! The Room Light inside the screen whose place holds the light: the
//! room's light as the screen's sensor hears it, measured, with a bar
//! from dark to daylight, which Return copies. Displays.qml built this row
//! from the machine daemon's sensor record (Backlight.js, Power.js's
//! roomLevel) until 2026-11-14; it's served here now, so the shell only
//! draws it (Y2YBTS's rule, 17a4dc7e).

use serde_json::{Value, json};

use super::{Inside, Light, Reading, Words, reading};
use crate::screens::{PLACE, key};

/// The lux the bar is full at: daylight, which a sensor indoors seldom
/// hears, as Power.js's roomLevel took it.
const DAYLIGHT: f64 = 10_000.0;

/// What the sensor hears as a share of the bar, on a logarithm's scale,
/// so a dim room and a bright one each move it as the eye sees them: none
/// for the dark, all for daylight or more.
fn level(lux: f64) -> f64 {
    if lux <= 0.0 {
        return 0.0;
    }
    ((lux + 1.0).log10() / (DAYLIGHT + 1.0).log10()).min(1.0)
}

/// The Room Light's row inside `inside`'s screen, while the room's sensor
/// is there to hear it: what it hears once read for this look, which
/// Return copies, or, while it's read, an ellipsis for its first second
/// and that it isn't read after, copying nothing, since those words
/// pasted are no use anywhere (goes-places, 2026-11-10).
#[must_use]
pub fn row(light: &Light, inside: &Inside, read: Reading, words: &Words) -> Option<Value> {
    let sensor = light.sensor.as_ref()?;
    let name = inside.name;
    let heard = sensor.lux.filter(|_| read == Reading::Read);
    let said = match (heard, read) {
        (Some(lux), _) => reading(lux, inside.numbers),
        (None, Reading::Waiting) => "\u{2026}".to_owned(),
        (None, _) => words.said("unread").to_owned(),
    };
    let acts = if heard.is_some() {
        json!([{"key": "verb:copy", "copy": said}])
    } else {
        json!([])
    };
    Some(json!({
        "key": key(&format!("room:{name}")),
        "thing": format!("{PLACE}:{name}:room"),
        "place": PLACE,
        "kind": "fact",
        "parent": inside.screen,
        "name": (inside.named)("room"),
        "glyph": "",
        "hue": "machine",
        "depth": 1,
        "side": [{"said": said}],
        "graph": {"shape": "bar", "values": [heard.map_or(0.0, level)], "warn": false},
        "note": words.room,
        "acts": acts,
    }))
}

#[cfg(test)]
mod tests;
