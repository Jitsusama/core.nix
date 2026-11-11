//! The launcher's Screens, served ready: each screen niri drives as a
//! row, the built-in one first and the rest by connector, and Turn Every
//! Screen Off while one is lit, in the launcher's words (launcher's
//! words.json). niri sends no event for a screen itself, so its outputs
//! are asked again as the stream begins and at each workspace change, as
//! they change when a screen comes or goes (Displays.qml asked the same).
//!
//! Each row holds what the shell's own acts need of its screen (`held`:
//! its connector, how it stands and niri's whole record of it), since
//! its settings, its drawer and every act on it are the shell's until
//! they move here, and the shell asks niri nothing of its own. A change
//! the shell makes to a screen says nothing in niri's stream, so it asks
//! this daemon to look again (main.rs's `look`).

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};

/// The place the launcher shows them in.
pub const PLACE: &str = "displays";

/// How long niri may take to say its outputs before they're left as
/// they were; it answers in microseconds, so only a niri stuck says
/// nothing.
const ASKING: Duration = Duration::from_secs(1);

/// A refresh rate is said in millihertz.
const MILLIHERTZ: u64 = 1000;

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
}

/// Turn Every Screen Off, as the vocabulary names it.
#[derive(Clone, Debug, PartialEq)]
struct Sleep {
    name: String,
    glyph: String,
    note: String,
}

impl Words {
    /// The words a screen's row is said in, or which the vocabulary
    /// lacks.
    pub fn read(vocabulary: &Value) -> Result<Words, String> {
        let things: BTreeMap<&str, &Value> = vocabulary["things"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|thing| Some((thing["id"].as_str()?, thing)))
            .collect();
        let thing = |id: &str| {
            things
                .get(id)
                .copied()
                .ok_or_else(|| format!("the vocabulary has no {id}"))
        };
        let word = |id: &str, path: &[&str]| -> Result<String, String> {
            let mut at = thing(id)?;
            for step in path {
                at = &at[*step];
            }
            at.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("the vocabulary's {id} has no {}", path.join(".")))
        };
        let said = |key: &str| word("displays", &["said", key]);
        Ok(Words {
            built_in: word("displays:built-in", &["name"])?,
            dark: said("dark")?,
            off: said("off")?,
            focused: said("focused")?,
            hub_on: said("hubOn")?,
            scale_at: said("scaleAt")?,
            sleep: Sleep {
                name: word("displays:sleep", &["name"])?,
                glyph: word("displays:sleep", &["glyph"])?,
                note: word("displays:sleep", &["said", "note"])?,
            },
        })
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

/// What a lit screen runs: its size, its refresh rate and its scale.
fn described(output: &Value, words: &Words) -> String {
    let mode = output["current_mode"]
        .as_u64()
        .and_then(|at| output["modes"].get(usize::try_from(at).ok()?));
    let (Some(mode), Some(multiple)) = (mode, output["logical"]["scale"].as_f64()) else {
        return words.off.clone();
    };
    format!(
        "{} \u{d7} {} \u{b7} {} \u{b7} {}",
        mode["width"],
        mode["height"],
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
        Words::read(&vocabulary()).expect("the words read")
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
        assert_eq!(rows[3]["note"], "a key wakes them");
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

    #[test]
    fn niris_answer_holds_its_outputs_by_connector() {
        let answer = json!({"Ok": {"Outputs": {"eDP-1": {"name": "eDP-1"}}}});
        let outputs = outputs(&answer).expect("outputs");
        assert_eq!(outputs.keys().collect::<Vec<_>>(), ["eDP-1"]);
        assert_eq!(super::outputs(&json!({"Err": "no"})), None);
    }

    #[test]
    fn a_vocabulary_without_a_word_says_which() {
        let mut vocabulary = vocabulary();
        vocabulary["things"][0]["said"]
            .as_object_mut()
            .expect("said")
            .remove("hubOn");
        assert_eq!(
            Words::read(&vocabulary),
            Err("the vocabulary's displays has no said.hubOn".to_owned())
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
        if let Err(why) = Words::read(&vocabulary) {
            panic!("the launcher's words lack one: {why}");
        }
    }
}
