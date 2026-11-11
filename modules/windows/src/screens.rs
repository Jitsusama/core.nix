//! The launcher's Screens, served ready: each screen niri drives as a
//! row, the built-in one first and the rest by connector, and Turn Every
//! Screen Off while one is lit, in the launcher's words (launcher's
//! words.json). niri sends no event for a screen itself, so its outputs
//! are asked again as the stream begins and at each workspace change, as
//! they change when a screen comes or goes (Displays.qml asked the same).
//!
//! Each row carries its drawer, what niri says of the screen past its
//! row (`drawer`), and holds what the shell's own acts need of it
//! (`held`: its connector, how it stands and niri's whole record of it),
//! since its settings and every act on it are the shell's until they
//! move here, and the shell asks niri nothing of its own. The drawer's
//! light and what the screen's EDID says join it here next; until then
//! the shell still lays its own. A change
//! the shell makes to a screen says nothing in niri's stream, so it asks
//! this daemon to look again (main.rs's `look`).

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use ready_rows::numbers::Numbers;
use serde_json::{Value, json};

/// The place the launcher shows them in.
pub const PLACE: &str = "displays";

/// How long niri may take to say its outputs before they're left as
/// they were; it answers in microseconds, so only a niri stuck says
/// nothing.
const ASKING: Duration = Duration::from_secs(1);

/// A refresh rate is said in millihertz.
const MILLIHERTZ: u64 = 1000;

const MILLIMETRES_AN_INCH: f64 = 25.4;

/// Two items of one value, as a figure's runs are set apart.
const APART: &str = " \u{b7} ";

/// The facts a drawer names, each by its part in the vocabulary
/// (`displays:<id>`).
const FACTS: [&str; 10] = [
    "mode", "refresh", "variable", "logical", "position", "rotation", "modes", "density", "serial",
    "size",
];

/// niri's eight transforms, as Rotation says them.
const TRANSFORMS: [&str; 8] = [
    "Normal",
    "90",
    "180",
    "270",
    "Flipped",
    "Flipped90",
    "Flipped180",
    "Flipped270",
];

/// What the rows are said in: the vocabulary's words for a screen.
#[derive(Clone, Debug, PartialEq)]
pub struct Words {
    built_in: String,
    dark: String,
    off: String,
    focused: String,
    hub_on: String,
    scale_at: String,
    sleep: Sleep,
    told: Told,
    numbers: Numbers,
}

/// What a screen's drawer is said in: each fact's name, and the words
/// its values take.
#[derive(Clone, Debug, PartialEq)]
struct Told {
    named: BTreeMap<&'static str, String>,
    turned: BTreeMap<&'static str, String>,
    built_in: String,
    screen: String,
    preferred: String,
    custom: String,
    preferred_mode: String,
    at: String,
    unsupported: String,
}

/// Turn Every Screen Off, as the vocabulary names it.
#[derive(Clone, Debug, PartialEq)]
struct Sleep {
    name: String,
    glyph: String,
    note: String,
}

impl Words {
    /// The words a screen's row is said in, and what the vocabulary
    /// lacks. A word it lacks is said as its id, and a glyph it lacks is
    /// none, so Screens keeps its rows and what they do rather than
    /// showing none for want of a word, as the sound daemon does.
    pub fn read(vocabulary: &Value) -> (Words, Vec<String>) {
        let things: BTreeMap<&str, &Value> = vocabulary["things"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|thing| Some((thing["id"].as_str()?, thing)))
            .collect();
        let lacked = std::cell::RefCell::new(Vec::new());
        let found = |id: &str, path: &[&str]| -> Option<String> {
            let mut at = *things.get(id)?;
            for step in path {
                at = &at[*step];
            }
            at.as_str().map(str::to_owned)
        };
        let word = |id: &str, path: &[&str], unsaid: &str| {
            found(id, path).unwrap_or_else(|| {
                lacked.borrow_mut().push(format!("{id} {}", path.join(".")));
                unsaid.to_owned()
            })
        };
        let said = |key: &str| word("displays", &["said", key], key);
        let words = Words {
            built_in: word("displays:built-in", &["name"], "displays:built-in"),
            dark: said("dark"),
            off: said("off"),
            focused: said("focused"),
            hub_on: said("hubOn"),
            scale_at: said("scaleAt"),
            sleep: Sleep {
                name: word("displays:sleep", &["name"], "displays:sleep"),
                glyph: word("displays:sleep", &["glyph"], ""),
                note: word("displays:sleep", &["said", "note"], "note"),
            },
            told: Told {
                named: FACTS
                    .iter()
                    .map(|id| {
                        let part = format!("displays:{id}");
                        (*id, word(&part, &["name"], &part))
                    })
                    .collect(),
                turned: TRANSFORMS
                    .iter()
                    .map(|turn| (*turn, word("displays:rotation", &["said", turn], turn)))
                    .collect(),
                built_in: said("built-in"),
                screen: said("screen"),
                preferred: word("displays:modes", &["said", "preferred"], "preferred"),
                custom: word("displays:modes", &["said", "custom"], "custom"),
                preferred_mode: word(
                    "displays:modes",
                    &["said", "preferred-mode"],
                    "preferred-mode",
                ),
                at: word("displays:modes", &["said", "at"], "at"),
                unsupported: word("displays:variable", &["said", "unsupported"], "unsupported"),
            },
            numbers: Numbers::default(),
        };
        (words, lacked.into_inner())
    }

    /// The words with the account's number settings, which a screen's
    /// density is said in.
    #[must_use]
    pub fn numbered(self, numbers: Numbers) -> Words {
        Words { numbers, ..self }
    }
}

/// niri's outputs as it says them now, by connector, or none where it
/// didn't answer in time.
pub fn asked(niri: &Path) -> Option<BTreeMap<String, Value>> {
    let mut stream = UnixStream::connect(niri).ok()?;
    stream.set_read_timeout(Some(ASKING)).ok()?;
    writeln!(stream, "\"Outputs\"").ok()?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).ok()?;
    outputs(&serde_json::from_str(&line).ok()?)
}

/// The outputs in niri's answer, by connector.
pub fn outputs(answer: &Value) -> Option<BTreeMap<String, Value>> {
    let outputs = answer["Ok"]["Outputs"].as_object()?;
    Some(
        outputs
            .iter()
            .map(|(name, output)| (name.clone(), output.clone()))
            .collect(),
    )
}

/// How a screen stands: off where niri lays nothing out on it, dark
/// where it's powered off by name but still laid out, else lit.
fn standing(output: &Value) -> &'static str {
    if output["logical"].is_null() {
        "off"
    } else if output["powered_off"] == true {
        "dark"
    } else {
        "lit"
    }
}

fn built_in(name: &str) -> bool {
    name.starts_with("eDP")
}

/// Its maker and model, leaving out a model that's only a number, or
/// its connector where niri says neither.
fn named(name: &str, output: &Value, words: &Words) -> String {
    if built_in(name) {
        return words.built_in.clone();
    }
    let model = output["model"].as_str().unwrap_or("");
    let numbered = model
        .strip_prefix("0x")
        .or_else(|| model.strip_prefix("0X"))
        .is_some_and(|digits| !digits.is_empty() && digits.chars().all(|c| c.is_ascii_hexdigit()));
    let make = output["make"].as_str().unwrap_or("");
    let said = format!("{make} {}", if numbered { "" } else { model });
    let said = said.trim();
    if said.is_empty() {
        name.to_owned()
    } else {
        said.to_owned()
    }
}

/// A refresh rate in millihertz, as hertz with every digit niri gives:
/// "60 Hz", "59.951 Hz".
pub fn hertz(millihertz: u64) -> String {
    let whole = millihertz / MILLIHERTZ;
    let rest = millihertz % MILLIHERTZ;
    if rest == 0 {
        return format!("{whole} Hz");
    }
    let decimals = format!("{rest:03}");
    format!("{whole}.{} Hz", decimals.trim_end_matches('0'))
}

/// A scale as the theme says one, a multiple to two places at most:
/// "2\u{d7}", "1.25\u{d7}".
pub fn scale(multiple: f64) -> String {
    let said = format!("{multiple:.2}");
    let said = said.trim_end_matches('0').trim_end_matches('.');
    format!("{said}\u{d7}")
}

/// A size in pixels or millimetres, as niri gives one: "2560 \u{d7} 1600".
fn size(width: &Value, height: &Value) -> String {
    format!("{width} \u{d7} {height}")
}

/// The mode a screen runs, or none where it runs none.
fn mode_of(output: &Value) -> Option<&Value> {
    output["current_mode"]
        .as_u64()
        .and_then(|at| output["modes"].get(usize::try_from(at).ok()?))
}

/// Each value said, a run apart, the empty ones left out.
fn runs(values: &[&str]) -> String {
    values
        .iter()
        .filter(|value| !value.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(APART)
}

/// A screen's drawer, as Displays.qml laid it from niri: its connector and
/// what it is over its name, both as written, since a connector is a name
/// from the world (VKCF3Z, 2026-11-07); then what it runs past its row,
/// where it sits and what it offers and is, its size from niri and its
/// serial last, since it identifies the screen. A screen that's off runs
/// nothing and sits nowhere, so says only what it offers and is. A
/// whole refresh, its size and its scale are its row's to say, so aren't
/// said again, and variable refresh is its switch's, so only a screen
/// without one says so.
fn drawer(name: &str, output: &Value, words: &Words) -> Value {
    let told = &words.told;
    let mut facts = Vec::new();
    let mut add = |id: &str, said: String| {
        if !said.is_empty() {
            facts.push(json!({"name": told.named[id], "said": said}));
        }
    };
    let now = mode_of(output);
    let logical = &output["logical"];
    if let Some(now) = now.filter(|_| !logical.is_null()) {
        let preferred = if now["is_preferred"] == true {
            told.preferred.as_str()
        } else {
            ""
        };
        let custom = if output["is_custom_mode"] == true {
            told.custom.as_str()
        } else {
            ""
        };
        add("mode", runs(&[preferred, custom]));
        let rate = now["refresh_rate"].as_u64().unwrap_or(0);
        add(
            "refresh",
            if rate % MILLIHERTZ == 0 {
                String::new()
            } else {
                hertz(rate)
            },
        );
        add(
            "variable",
            if output["vrr_supported"] == true {
                String::new()
            } else {
                told.unsupported.clone()
            },
        );
        add("logical", size(&logical["width"], &logical["height"]));
        add("position", format!("{}, {}", logical["x"], logical["y"]));
        let turn = logical["transform"].as_str().unwrap_or("");
        add(
            "rotation",
            told.turned
                .get(turn)
                .map_or_else(|| turn.to_owned(), Clone::clone),
        );
    }
    let modes = output["modes"].as_array().map_or(&[][..], Vec::as_slice);
    let preferred = modes.iter().find(|mode| mode["is_preferred"] == true);
    if !modes.is_empty() {
        let prefers = preferred.map_or_else(String::new, |mode| {
            let at = told
                .at
                .replace("{size}", &size(&mode["width"], &mode["height"]))
                .replace("{rate}", &hertz(mode["refresh_rate"].as_u64().unwrap_or(0)));
            told.preferred_mode.replace("{mode}", &at)
        });
        add(
            "modes",
            runs(&[modes.len().to_string().as_str(), prefers.as_str()]),
        );
    }
    let physical = |at: usize| output["physical_size"][at].as_f64().unwrap_or(0.0);
    let across = now
        .or(preferred)
        .and_then(|mode| mode["width"].as_f64())
        .unwrap_or(0.0);
    if across > 0.0 && physical(0) > 0.0 {
        let density = across / (physical(0) / MILLIMETRES_AN_INCH);
        add(
            "density",
            format!("{} ppi", words.numbers.significant(density)),
        );
    }
    if physical(0) > 0.0 && physical(1) > 0.0 {
        let measured = &output["physical_size"];
        add("size", format!("{} mm", size(&measured[0], &measured[1])));
    }
    let serial = match &output["serial"] {
        Value::Null => String::new(),
        Value::String(said) => said.clone(),
        other => other.to_string(),
    };
    if !serial.is_empty() {
        facts.push(json!({"name": told.named["serial"], "said": serial, "identifies": true}));
    }
    let what = if built_in(name) {
        told.built_in.as_str()
    } else {
        told.screen.as_str()
    };
    json!({
        "eyebrow": runs(&[name, what]),
        "eyebrowNames": true,
        "title": named(name, output, words),
        "facts": facts,
    })
}

/// What a lit screen runs: its size, its refresh rate and its scale.
fn described(output: &Value, words: &Words) -> String {
    let mode = mode_of(output);
    let (Some(mode), Some(multiple)) = (mode, output["logical"]["scale"].as_f64()) else {
        return words.off.clone();
    };
    format!(
        "{}{APART}{}{APART}{}",
        size(&mode["width"], &mode["height"]),
        hertz(mode["refresh_rate"].as_u64().unwrap_or(0)),
        words.scale_at.replace("{scale}", &scale(multiple))
    )
}

/// A row's key in the place.
pub fn key(id: &str) -> String {
    format!("row:{PLACE}:{id}")
}

/// The screens' rows, the built-in one first and the rest by connector,
/// then Turn Every Screen Off while any is lit. `focused` is the
/// connector of the workspace in front, said beside its screen only
/// where more than one is lit.
pub fn rows(outputs: &BTreeMap<String, Value>, focused: Option<&str>, words: &Words) -> Vec<Value> {
    let mut shown: Vec<(&String, &Value)> = outputs.iter().collect();
    shown.sort_by_key(|(name, _)| (!built_in(name), name.as_str()));
    let lit = shown
        .iter()
        .filter(|(_, output)| standing(output) == "lit")
        .count();
    let mut rows: Vec<Value> = shown
        .iter()
        .map(|(name, output)| {
            let stands = standing(output);
            let side = match stands {
                "dark" => Some(&words.dark),
                "off" => Some(&words.off),
                _ if lit > 1 && focused == Some(name.as_str()) => Some(&words.focused),
                _ => None,
            };
            let note = if stands == "dark" {
                words.hub_on.clone()
            } else {
                described(output, words)
            };
            json!({
                "key": key(&format!("display:{name}")),
                "thing": format!("display:{name}"),
                "place": PLACE,
                "kind": "screen",
                "name": named(name, output, words),
                // A screen, built in or not, wears the screen's mark, its
                // drawer saying it's built in (VKCF3Z, 2026-11-08).
                "glyph": "display",
                "hue": "machine",
                "depth": 0,
                "side": side.into_iter().map(|said| json!({"said": said})).collect::<Vec<_>>(),
                "note": note,
                "drawer": drawer(name, output, words),
                "held": {"output": name, "standing": stands, "builtIn": built_in(name),
                    "niri": output},
            })
        })
        .collect();
    if lit > 0 {
        rows.push(json!({
            "key": key("sleep"),
            "thing": "displays:sleep",
            "place": PLACE,
            "kind": "action",
            "name": words.sleep.name,
            "glyph": words.sleep.glyph,
            "hue": "action",
            "depth": 0,
            "side": [],
            "note": words.sleep.note,
        }));
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The launcher's words for its Screens, in the file the socket's
    /// tests hand the daemon too.
    fn vocabulary() -> Value {
        serde_json::from_str(include_str!("../tests/screen-words.json"))
            .expect("the test's words are JSON")
    }

    fn words() -> Words {
        let (words, lacked) = Words::read(&vocabulary());
        assert!(lacked.is_empty(), "the test's words lack {lacked:?}");
        words
    }

    /// A screen as niri says one: lit at its first mode, unless `logical`
    /// is null.
    fn output(make: &str, model: &str, logical: &Value) -> Value {
        json!({
            "make": make, "model": model, "current_mode": 0,
            "modes": [{"width": 2560, "height": 1600, "refresh_rate": 59_951}],
            "logical": logical,
        })
    }

    fn lit(scale: f64) -> Value {
        json!({"x": 0, "y": 0, "scale": scale, "transform": "Normal"})
    }

    fn desk() -> BTreeMap<String, Value> {
        BTreeMap::from([
            (
                "HDMI-A-1".to_owned(),
                output("Dell Inc.", "U2723QE", &lit(1.5)),
            ),
            ("eDP-1".to_owned(), output("BOE", "0x0BCA", &lit(1.75))),
            ("DP-2".to_owned(), output("LG", "0x5B11", &Value::Null)),
        ])
    }

    fn names(rows: &[Value]) -> Vec<&str> {
        rows.iter().filter_map(|row| row["name"].as_str()).collect()
    }

    #[test]
    fn the_built_in_screen_leads_then_each_by_its_connector_then_turning_them_off() {
        let rows = rows(&desk(), Some("eDP-1"), &words());
        assert_eq!(
            names(&rows),
            [
                "Built-in Screen",
                "LG",
                "Dell Inc. U2723QE",
                "Turn Every Screen Off"
            ],
            "the built-in first, the rest by connector, a numbered model left out"
        );
        assert_eq!(rows[0]["key"], "row:displays:display:eDP-1");
        assert_eq!(rows[0]["thing"], "display:eDP-1");
        assert_eq!(rows[0]["glyph"], "display");
        assert_eq!(
            rows[0]["note"],
            "2560 \u{d7} 1600 \u{b7} 59.951 Hz \u{b7} scale 1.75\u{d7}"
        );
        assert_eq!(rows[0]["held"]["output"], "eDP-1");
        assert_eq!(
            rows[0]["held"]["niri"],
            desk()["eDP-1"],
            "the shell's acts read niri's whole record of the screen"
        );
        assert_eq!(rows[3]["key"], "row:displays:sleep");
        assert_eq!(rows[3]["note"], "a key wakes them \u{b7} hubs stay on");
    }

    #[test]
    fn a_screen_says_how_it_stands_and_the_focused_one_only_beside_another_lit() {
        let rows = rows(&desk(), Some("eDP-1"), &words());
        assert_eq!(rows[0]["side"], json!([{"said": "focused"}]));
        assert_eq!(rows[1]["side"], json!([{"said": "off"}]));
        assert_eq!(rows[1]["note"], "off", "an unlaid screen runs nothing");
        assert_eq!(rows[2]["side"], json!([]));
        let alone = BTreeMap::from([("eDP-1".to_owned(), output("BOE", "", &lit(2.0)))]);
        let rows = super::rows(&alone, Some("eDP-1"), &words());
        assert_eq!(
            rows[0]["side"],
            json!([]),
            "the only screen lit isn't said focused"
        );
    }

    #[test]
    fn a_screen_dark_by_name_says_its_hub_is_on() {
        let mut dark = output("Dell Inc.", "U2723QE", &lit(1.0));
        dark["powered_off"] = json!(true);
        let outputs = BTreeMap::from([("DP-1".to_owned(), dark)]);
        let rows = rows(&outputs, None, &words());
        assert_eq!(rows[0]["side"], json!([{"said": "dark"}]));
        assert_eq!(rows[0]["note"], "hub on");
        assert_eq!(rows.len(), 1, "with none lit, none is turned off");
    }

    #[test]
    fn a_screen_naming_nothing_is_its_connector() {
        let outputs = BTreeMap::from([("DP-3".to_owned(), output("", "", &lit(1.0)))]);
        assert_eq!(names(&rows(&outputs, None, &words()))[0], "DP-3");
    }

    #[test]
    fn a_rate_keeps_every_digit_and_a_scale_two_places() {
        assert_eq!(hertz(60_000), "60 Hz");
        assert_eq!(hertz(59_951), "59.951 Hz");
        assert_eq!(hertz(143_900), "143.9 Hz");
        assert_eq!(scale(2.0), "2\u{d7}");
        assert_eq!(scale(1.25), "1.25\u{d7}");
        assert_eq!(scale(1.333_333), "1.33\u{d7}");
    }

    /// The built-in panel as niri says it, from tests/plate.qml's cases.
    fn panel() -> Value {
        json!({
            "make": "LG Display", "model": "0x07C6", "serial": null,
            "physical_size": [300, 190],
            "modes": [
                {"width": 2880, "height": 1800, "refresh_rate": 120_001, "is_preferred": true},
                {"width": 2880, "height": 1800, "refresh_rate": 60_000, "is_preferred": true}
            ],
            "current_mode": 0, "is_custom_mode": false,
            "vrr_supported": true, "vrr_enabled": false,
            "logical": {"x": 560, "y": 1440, "width": 1440, "height": 900, "scale": 2.0,
                "transform": "Normal"}
        })
    }

    /// A drawer's facts, each name=said, a semicolon apart.
    fn said(drawer: &Value) -> String {
        drawer["facts"]
            .as_array()
            .expect("the drawer's facts")
            .iter()
            .map(|fact| format!("{}={}", fact["name"].as_str().unwrap_or(""), fact["said"]))
            .collect::<Vec<_>>()
            .join("; ")
            .replace('"', "")
    }

    #[test]
    fn a_screen_on_says_in_its_drawer_what_it_runs_past_its_row_where_it_sits_and_offers() {
        let drawer = drawer("eDP-1", &panel(), &words());
        assert_eq!(drawer["eyebrow"], "eDP-1 \u{b7} built in");
        assert_eq!(
            drawer["eyebrowNames"], true,
            "a connector isn't set as written"
        );
        assert_eq!(
            drawer["title"], "Built-in Screen",
            "its title isn't its row's name"
        );
        assert_eq!(
            said(&drawer),
            "Screen Mode=preferred; Refresh=120.001 Hz; Logical Size=1440 \u{d7} 900; \
             Position=560, 1440; Rotation=upright; \
             Screen Modes=2 \u{b7} preferred 2880 \u{d7} 1800 at 120.001 Hz; \
             Density=244 ppi; Size=300 \u{d7} 190 mm"
        );
    }

    #[test]
    fn a_drawer_says_custom_and_its_serial_last_as_what_identifies_it() {
        let mut on = panel();
        on["is_custom_mode"] = json!(true);
        on["serial"] = json!("0x1234");
        on["modes"][0]["is_preferred"] = json!(false);
        let drawer = drawer("DP-1", &on, &words());
        assert_eq!(drawer["eyebrow"], "DP-1 \u{b7} screen");
        assert!(
            said(&drawer).starts_with("Screen Mode=custom; Refresh=120.001 Hz; Logical Size="),
            "a custom mode isn't said so: {}",
            said(&drawer)
        );
        let facts = drawer["facts"].as_array().expect("facts");
        assert_eq!(
            facts.last(),
            Some(&json!({"name": "Serial", "said": "0x1234", "identifies": true})),
            "the serial doesn't end the drawer, identifying the screen"
        );
    }

    #[test]
    fn a_whole_refresh_is_the_rows_and_only_a_lack_of_variable_refresh_the_drawers() {
        let mut on = panel();
        on["modes"][0]["refresh_rate"] = json!(60_000);
        on["modes"][0]["is_preferred"] = json!(false);
        assert!(
            said(&drawer("eDP-1", &on, &words())).starts_with("Logical Size="),
            "a whole refresh or a plain mode is said again: {}",
            said(&drawer("eDP-1", &on, &words()))
        );
        on["vrr_supported"] = json!(false);
        assert!(
            said(&drawer("eDP-1", &on, &words())).starts_with("Variable Refresh=unsupported; "),
            "a screen without variable refresh doesn't say so: {}",
            said(&drawer("eDP-1", &on, &words()))
        );
    }

    #[test]
    fn a_screen_off_says_only_what_it_offers_and_is_in_the_accounts_numbers() {
        let mut off = panel();
        off["logical"] = Value::Null;
        off["current_mode"] = Value::Null;
        off["physical_size"] = json!([3000, 0]);
        let words = words().numbered(Numbers::new(".", ","));
        assert_eq!(
            said(&drawer("eDP-1", &off, &words)),
            "Screen Modes=2 \u{b7} preferred 2880 \u{d7} 1800 at 120.001 Hz; Density=24,4 ppi",
            "a screen off says what it runs, or a size niri half knows"
        );
    }

    #[test]
    fn each_screens_row_carries_its_drawer() {
        let rows = rows(&desk(), None, &words());
        assert_eq!(rows[0]["drawer"]["title"], "Built-in Screen");
        assert_eq!(rows[2]["drawer"]["title"], "Dell Inc. U2723QE");
        assert_eq!(
            rows[3].get("drawer"),
            None,
            "Turn Every Screen Off has a drawer"
        );
    }

    #[test]
    fn niris_answer_holds_its_outputs_by_connector() {
        let answer = json!({"Ok": {"Outputs": {"eDP-1": {"name": "eDP-1"}}}});
        let outputs = outputs(&answer).expect("outputs");
        assert_eq!(outputs.keys().collect::<Vec<_>>(), ["eDP-1"]);
        assert_eq!(super::outputs(&json!({"Err": "no"})), None);
    }

    #[test]
    fn a_vocabulary_without_a_word_says_it_by_its_id_and_which() {
        let mut vocabulary = vocabulary();
        vocabulary["things"][0]["said"]
            .as_object_mut()
            .expect("said")
            .remove("hubOn");
        let (words, lacked) = Words::read(&vocabulary);
        assert_eq!(words.hub_on, "hubOn", "a word lacked isn't said by its id");
        assert_eq!(
            words.dark,
            self::words().dark,
            "a word held isn't said as held"
        );
        assert_eq!(lacked, vec!["displays said.hubOn".to_owned()]);
    }

    #[test]
    fn no_vocabulary_still_words_every_row_by_its_ids() {
        let (words, lacked) = Words::read(&Value::Null);
        assert_eq!(
            words.built_in, "displays:built-in",
            "the built-in isn't named by its id"
        );
        assert_eq!(
            words.sleep.name, "displays:sleep",
            "Sleep isn't named by its id"
        );
        assert_eq!(words.sleep.glyph, "", "a glyph lacked isn't none");
        assert_eq!(
            lacked.len(),
            34,
            "not every word lacked was said: {lacked:?}"
        );
        let drawer = drawer("eDP-1", &panel(), &words);
        assert_eq!(
            drawer["facts"][0]["name"], "displays:mode",
            "a fact isn't named by its id"
        );
        assert_eq!(
            drawer["facts"][4]["said"], "Normal",
            "a turn isn't said as niri says it"
        );
    }

    /// The launcher's own words.json, where the build hands it in.
    #[test]
    fn the_launchers_vocabulary_has_every_screens_words() {
        let Some(path) = option_env!("WINDOWS_TEST_WORDS") else {
            return;
        };
        let read = std::fs::read_to_string(path).expect("the launcher's words read");
        let vocabulary: Value = serde_json::from_str(&read).expect("the words are JSON");
        let (_, lacked) = Words::read(&vocabulary);
        assert!(lacked.is_empty(), "the launcher's words lack {lacked:?}");
    }
}
