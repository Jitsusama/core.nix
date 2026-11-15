//! The Brightness inside the screen whose place holds it: its row, saying
//! the share the backlight is at with a bar of it and what sets it, and
//! inside it the levels a step in offers, brightest first, each by its
//! value (VKCF3Z, 2026-11-09), the one it's at said chosen. A level's
//! Return asks this daemon, which asks the machine daemon to set it as the
//! brightness keys do (main.rs's `light_done`, modules/machine's
//! `screen_acts.rs`), so the backlight program learns it as Joel's pick.
//! Displays.qml built these rows from the machine daemon's backlight
//! record (Backlight.js's lit) until 2026-11-14; they're served here now,
//! so the shell only draws them (Y2YBTS's rule, 17a4dc7e).

use serde_json::{Value, json};

use super::{EVERY_PLACE, Inside, Light, PERCENT, Reading, Words};
use crate::screens::{PLACE, key};

/// The shares a step into Brightness offers, brightest first; none is
/// nought, since a backlight at none leaves the screen black.
const LEVELS: [u8; 10] = [100, 90, 80, 70, 60, 50, 40, 30, 20, 10];

/// The words a level is named by and its command typed by, which is the
/// key of its act too, so the : line's `:set brightness=40` fills in as
/// the shell's did.
pub const LEVEL: &str = "displays:brightness:level";

/// The Brightness's row inside `inside`'s screen, then its levels; none
/// without a backlight. `in_the_room` is whether the screen holds the
/// room's sensor, so says the backlight's steps; a monitor following the
/// panel says only what sets it.
/// Until it's read for this look, it says no share, nor a level chosen:
/// nothing for its first second, then that it isn't read yet, as the
/// screen's drawer does.
#[must_use]
pub fn rows(
    light: &Light,
    inside: &Inside,
    in_the_room: bool,
    read: Reading,
    words: &Words,
) -> Vec<Value> {
    let Some(lit) = &light.lit else {
        return Vec::new();
    };
    let name = inside.name;
    let heard = read == Reading::Read;
    let share = (lit.now / lit.most * PERCENT).round();
    let side = match read {
        Reading::Read => vec![json!({"said": format!("{share}%")})],
        Reading::Waiting => Vec::new(),
        Reading::Unread => vec![json!({"said": words.said("unread")})],
    };
    let count = |value: f64| inside.numbers.number(value, EVERY_PLACE);
    let said = |id: &str| words.brightness.get(id).map_or("", String::as_str);
    // With the lid shut nothing hears the room, and a monitor has no
    // steps of the panel's, so it only says what sets it.
    let note = match (in_the_room, heard) {
        (false, _) => said("everyFollows").to_owned(),
        (true, true) => said("followsRoom")
            .replace("{now}", &count(lit.now))
            .replace("{most}", &count(lit.most)),
        (true, false) => said("followsRoomUnread").replace("{most}", &count(lit.most)),
    };
    let bar = if heard {
        (lit.now / lit.most).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let named = (inside.named)("brightness");
    let setting = key(&format!("setting:{name}:brightness"));
    let mut rows = vec![json!({
        "key": setting,
        "thing": format!("{PLACE}:{name}:brightness"),
        "place": PLACE,
        "kind": "setting",
        "name": named,
        "glyph": "brightness",
        "hue": "machine",
        "parent": inside.screen,
        "depth": 1,
        "side": side,
        // However bright, it wants nothing, so it never warns.
        "graph": {"shape": "bar", "values": [bar], "warn": false},
        "note": note,
        "acts": [{"key": "verb:more", "step": "rows"}],
    })];
    rows.extend(LEVELS.iter().map(|each| {
        let chosen: &[&String] = if heard && (share - f64::from(*each)).abs() < f64::EPSILON {
            &[&words.chosen]
        } else {
            &[]
        };
        json!({
            "key": key(&format!("choice:{name}:brightness-{each}")),
            "thing": format!("verb:display-brightness-{each}"),
            "place": PLACE,
            "kind": "choice",
            "name": words.level.replace("{n}", &each.to_string()),
            "glyph": "brightness",
            "hue": "action",
            "parent": setting,
            "depth": 2,
            "side": chosen.iter().map(|said| json!({"said": said})).collect::<Vec<_>>(),
            "note": "",
            // Its setting's name is a hidden word, so what's typed for
            // the Brightness finds its levels from outside too.
            "held": {"keywords": named, "fill": {"n": each}},
            "acts": [{"key": LEVEL,
                "ask": {"ask": "light", "do": "brightness", "share": each}}],
        })
    }));
    rows
}

#[cfg(test)]
mod tests;
