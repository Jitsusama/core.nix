//! A lit screen's settings, served as the rows a step into its row
//! offers, as Displays.qml made them until 11 November 2026: Scale,
//! Refresh, Variable Refresh, Resolution and Rotation, each saying how
//! it's set, and each setting's choices a step into it, by their value
//! alone, the one it's at said chosen and the mode the screen prefers
//! noted so (VKCF3Z, 2026-11-09). A choice is an act this daemon does on
//! niri (acts.rs's `Set`), and what the : line's :setl would type for it
//! is held, so its command fills in as the shell's did. Variable Refresh
//! is a switch, its Return flipping it. A setting with one choice or none
//! has nothing to step into, so it only says how it's set.

use serde_json::{Value, json};

use super::acts::{Act, Set};
use super::{APART, MILLIHERTZ, PLACE, TRANSFORMS, Words, hertz, key, mode_of, scale};

/// The scales offered, as multiples, as Screens.js offered them.
const SCALES: [f64; 7] = [1.0, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0];

/// Two scales closer than this are one.
const SAME_SCALE: f64 = 0.01;

/// One choice of a setting, as its row is made.
struct Choice {
    /// Its own part of its key, as `scale-1.5`, which the shell keyed it
    /// by too.
    id: String,
    name: String,
    chosen: bool,
    note: String,
    /// niri's action on the screen's output that sets it.
    action: Value,
    /// What finds it besides its name and its setting's, as a scale's
    /// share, "150%".
    keywords: String,
    /// How the : line's :setl types it, as "1.5".
    typed: String,
}

/// The settings' rows of the lit screen `name`, each inside its screen's
/// row, each choice inside its setting's, in the order a step in offers
/// them.
#[must_use]
pub fn rows(name: &str, output: &Value, words: &Words) -> Vec<Value> {
    let screen = key(&format!("display:{name}"));
    let logical = &output["logical"];
    let now = logical["scale"].as_f64().unwrap_or(1.0);
    let mut rows = setting(
        name,
        &screen,
        "scale",
        &scale(now),
        SCALES
            .iter()
            .map(|each| Choice {
                id: format!("scale-{}", multiple(*each)),
                name: scale(*each),
                chosen: (each - now).abs() < SAME_SCALE,
                note: String::new(),
                action: json!({"Scale": {"scale": {"Specific": each}}}),
                keywords: format!("{}%", (each * 100.0).round()),
                typed: multiple(*each),
            })
            .collect(),
        words,
    );
    if let Some(mode) = mode_of(output) {
        let rate = mode["refresh_rate"].as_u64().unwrap_or(0);
        rows.extend(setting(
            name,
            &screen,
            "refresh",
            &hertz(rate),
            refreshes(output, mode, words),
            words,
        ));
        if output["vrr_supported"] == true {
            rows.push(variable(name, &screen, output, words));
        }
        rows.extend(setting(
            name,
            &screen,
            "resolution",
            &format!("{} \u{d7} {}", mode["width"], mode["height"]),
            sizes(output, mode, words),
            words,
        ));
    }
    let turned = logical["transform"].as_str().unwrap_or("Normal");
    rows.extend(setting(
        name,
        &screen,
        "rotation",
        words.told.turned.get(turned).map_or("", String::as_str),
        TRANSFORMS
            .iter()
            .map(|turn| Choice {
                id: format!("turn-{turn}"),
                name: words.told.turned[turn].clone(),
                chosen: turned == *turn,
                note: String::new(),
                action: json!({"Transform": {"transform": turn}}),
                keywords: String::new(),
                typed: words.told.typed[turn].clone(),
            })
            .collect(),
        words,
    ));
    rows
}

/// A multiple as the : line types it and as a scale's choice is keyed:
/// "1", "1.25".
fn multiple(each: f64) -> String {
    let said = format!("{each:.2}");
    said.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// A setting's option on the : line, its name lowercased and joined, as
/// Vim's are: Scale's scale, Variable Refresh's variablerefresh.
fn option(named: &str) -> String {
    named.to_lowercase().split(' ').collect()
}

/// A setting's row, saying how it's set, then its choices' rows, which a
/// step into it offers where there are two at least.
fn setting(
    name: &str,
    screen: &str,
    id: &str,
    side: &str,
    choices: Vec<Choice>,
    words: &Words,
) -> Vec<Value> {
    let named = words.told.part(id);
    let at = key(&format!("setting:{name}:{id}"));
    let offers = choices.len() > 1;
    let (glyph, acts) = if offers {
        (id, json!([{"key": "verb:more", "step": "rows"}]))
    } else {
        ("", json!([]))
    };
    let mut rows = vec![json!({
        "key": at,
        "thing": format!("{PLACE}:{name}:{id}"),
        "place": PLACE,
        "kind": "setting",
        "name": named,
        "glyph": glyph,
        "hue": "machine",
        "parent": screen,
        "depth": 1,
        "side": [{"said": side}],
        "acts": acts,
    })];
    if !offers {
        return rows;
    }
    rows.extend(choices.into_iter().map(|choice| {
        let set = Set {
            output: name.to_owned(),
            action: choice.action,
            said: choice.name.clone(),
            done: "set".to_owned(),
        };
        let side: &[&String] = if choice.chosen { &[&words.chosen] } else { &[] };
        let keywords = [named.as_str(), choice.keywords.as_str()].join(" ");
        json!({
            "key": key(&format!("choice:{name}:{}", choice.id)),
            "thing": format!("verb:display-{}", choice.id),
            "place": PLACE,
            "kind": "choice",
            "name": choice.name,
            "glyph": id,
            "hue": "action",
            "parent": at,
            "depth": 2,
            "side": side.iter().map(|said| json!({"said": said})).collect::<Vec<_>>(),
            "note": choice.note,
            // Its setting's name finds it from outside, as "scale 1.5".
            "held": {
                "keywords": keywords.trim(),
                "fill": {"setting": option(&named), "value": choice.typed},
            },
            "acts": [{"key": "verb:display-set", "ask": Act::Set(set).ask()}],
        })
    }));
    rows
}

/// What a mode's choice notes after its value: "preferred" where the
/// EDID prefers it, "custom" where it's a custom mode niri runs.
fn noted(preferred: bool, custom: bool, words: &Words) -> String {
    [
        (preferred, &words.told.preferred),
        (custom, &words.told.custom),
    ]
    .iter()
    .filter(|(said, _)| *said)
    .map(|(_, word)| word.as_str())
    .collect::<Vec<_>>()
    .join(APART)
}

/// Refresh's choices: each rate the mode it runs is offered at, at its
/// size, said by its value with every digit niri gives, so 59.94 Hz and
/// 60 Hz are two, and keyed by the mode's place in niri's list, the one
/// key a mode has where two share a size and a rate.
fn refreshes(output: &Value, now: &Value, words: &Words) -> Vec<Choice> {
    let custom = output["is_custom_mode"] == true;
    output["modes"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .filter(|(_, mode)| mode["width"] == now["width"] && mode["height"] == now["height"])
        .map(|(at, mode)| {
            let rate = mode["refresh_rate"].as_u64().unwrap_or(0);
            let chosen = mode == now;
            let said = hertz(rate);
            Choice {
                id: format!("mode-{at}"),
                typed: said.trim_end_matches(" Hz").to_owned(),
                name: said,
                chosen,
                note: noted(mode["is_preferred"] == true, chosen && custom, words),
                action: json!({"Mode": {"mode": {"Specific": {
                    "width": mode["width"],
                    "height": mode["height"],
                    "refresh": rate_of(rate),
                }}}}),
                keywords: String::new(),
            }
        })
        .collect()
}

/// A rate in millihertz as niri's Mode action takes it, in hertz. A rate
/// is a few hundred thousand millihertz at most, so one past a u32 is no
/// rate, and none.
fn rate_of(millihertz: u64) -> f64 {
    let whole = |count: u64| f64::from(u32::try_from(count).unwrap_or(0));
    whole(millihertz) / whole(MILLIHERTZ)
}

/// Resolution's choices: each size once, in niri's order, preferred where
/// any of its modes is, niri choosing the rate.
fn sizes(output: &Value, now: &Value, words: &Words) -> Vec<Choice> {
    let custom = output["is_custom_mode"] == true;
    let mut found: Vec<(&Value, &Value, bool)> = Vec::new();
    for mode in output["modes"].as_array().into_iter().flatten() {
        let preferred = mode["is_preferred"] == true;
        match found
            .iter_mut()
            .find(|(width, height, _)| **width == mode["width"] && **height == mode["height"])
        {
            Some(size) => size.2 |= preferred,
            None => found.push((&mode["width"], &mode["height"], preferred)),
        }
    }
    found
        .into_iter()
        .map(|(width, height, preferred)| {
            let chosen = *width == now["width"] && *height == now["height"];
            Choice {
                id: format!("size-{width}x{height}"),
                name: format!("{width} \u{d7} {height}"),
                chosen,
                note: noted(preferred, chosen && custom, words),
                action: json!({"Mode": {"mode": {"Specific": {
                    "width": width,
                    "height": height,
                    "refresh": null,
                }}}}),
                keywords: String::new(),
                typed: format!("{width}x{height}"),
            }
        })
        .collect()
}

/// Variable Refresh, a switch keeping its name as it flips, saying on or
/// off, Return flipping it. niri's on-demand variable refresh, on only
/// while a window rule asks, would make it a setting of three; nothing
/// here sets that, so it's on or off.
fn variable(name: &str, screen: &str, output: &Value, words: &Words) -> Value {
    let on = output["vrr_enabled"] == true;
    let named = words.told.part("variable");
    let side = if on {
        &words.told.variable_on
    } else {
        &words.told.variable_off
    };
    let set = Set {
        output: name.to_owned(),
        action: json!({"Vrr": {"vrr": {"vrr": !on, "on_demand": false}}}),
        said: named.clone(),
        done: if on { "off" } else { "on" }.to_owned(),
    };
    json!({
        "key": key(&format!("setting:{name}:variable")),
        "thing": format!("{PLACE}:{name}:variable"),
        "place": PLACE,
        "kind": "switch",
        "name": named,
        // Its own mark, as a switch wears its thing's: one mark, one
        // meaning, so never Refresh's.
        "glyph": "variableRefresh",
        "hue": "machine",
        "parent": screen,
        "depth": 1,
        "side": [{"said": side}],
        "held": {"on": on},
        "acts": [{"key": "displays:variable", "ask": Act::Set(set).ask()}],
    })
}

#[cfg(test)]
mod tests;
