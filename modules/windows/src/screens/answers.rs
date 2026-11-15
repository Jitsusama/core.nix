//! What a screen's part answers when it's asked by name from the top,
//! "scale" or "white point": a row for each lit screen that has the
//! part, the focused screen's first, behind `answer:<id>`, a key no row
//! is, so no place lists them and only the answer does. The launcher
//! words the answer from the first, its part's own row saying how it
//! stands, and lists the rest below it (launcher/Displays.qml).

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::{PLACE, key, standing};

/// The parts asked by name, as the launcher's words ask them
/// (`displays:<id>`'s answers). The Warmth is Screens' own, not a
/// screen's, so its answer is its own row's.
const ASKED: [&str; 11] = [
    "brightness",
    "room",
    "scale",
    "refresh",
    "resolution",
    "rotation",
    "depth",
    "colours",
    "white",
    "primaries",
    "hdr",
];

/// The key of part `id`'s row inside the screen of connector `name`:
/// the Brightness and the settings are settings, the Room Light its
/// own, and the rest what its EDID says.
fn part_key(id: &str, name: &str) -> String {
    match id {
        "room" => key(&format!("room:{name}")),
        "brightness" | "scale" | "refresh" | "resolution" | "rotation" => {
            key(&format!("setting:{name}:{id}"))
        }
        _ => key(&format!("fact:{name}:{id}")),
    }
}

/// The rows behind each part's answer, from the place's rows as `made`,
/// for the lit screens of those `shown`, the one of connector `focused`
/// first, then the rest as they're shown. Each is named as its screen's
/// row is, says what its part says, copies it, and holds its part's
/// key, which the answer's headline is read from.
pub(super) fn rows(
    made: &[Value],
    shown: &[(&String, &Value)],
    focused: Option<&str>,
) -> Vec<Value> {
    let mut lit: Vec<&str> = shown
        .iter()
        .filter(|(_, output)| standing(output) == "lit")
        .map(|(name, _)| name.as_str())
        .collect();
    lit.sort_by_key(|name| focused != Some(*name));
    let served: BTreeMap<&str, &Value> = made
        .iter()
        .filter_map(|row| row["key"].as_str().map(|at| (at, row)))
        .collect();
    let mut rows = Vec::new();
    for id in ASKED {
        let within = key(&format!("answer:{id}"));
        for name in &lit {
            let screen = served.get(key(&format!("display:{name}")).as_str());
            let (Some(screen), Some(part)) = (screen, served.get(part_key(id, name).as_str()))
            else {
                continue;
            };
            rows.push(json!({
                "key": key(&format!("answer:{id}:{name}")),
                "thing": format!("{PLACE}:answer:{id}:{name}"),
                "place": PLACE,
                "kind": "fact",
                "parent": within,
                "name": screen["name"],
                "glyph": "",
                "hue": "machine",
                "depth": 1,
                "side": part["side"],
                "held": {"output": name, "part": part["key"]},
                "acts": [{"key": "verb:copy", "copy": part["side"][0]["said"]}],
            }));
        }
    }
    rows
}
