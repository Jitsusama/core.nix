//! The launcher's Screens, served ready: each screen niri drives as a
//! row, the built-in one first and the rest by connector, and Turn Every
//! Screen Off while one is lit, in the launcher's words (launcher's
//! words.json). niri sends no event for a screen itself, so its outputs
//! are asked again as the stream begins and at each workspace change, as
//! they change when a screen comes or goes (Displays.qml asked the same).
//!
//! Each row carries its drawer, what niri says of the screen past its
//! row and what desk-facts reads of its EDID (`drawer`), and holds what
//! the shell's own acts need of it (`held`: its connector, how it stands,
//! niri's whole record of it, what the : line takes and whether its
//! brightness follows the panel's). Inside it are what a step in offers:
//! a lit one's settings (settings.rs), the ways it turns off or on, and
//! what its EDID says (facts.rs). Its Return, focusing it or turning it
//! on, the ways it turns off and Turn Every Screen Off's are served as
//! acts this daemon does on niri (acts.rs). While a screen is looked at, the
//! drawer of the one whose place holds the Brightness leads with the
//! light (light.rs). A change to a screen says nothing in niri's stream,
//! so each act looks again, and the shell asks this daemon to look after
//! a change of its own (main.rs's `look`).

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use ready_rows::numbers::Numbers;
use serde_json::{Value, json};

use crate::light::{self, Seen};

pub mod acts;
mod facts;
pub mod line;
pub mod settings;
mod tells;
pub mod vocabulary;
use acts::Act;

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

/// The primaries desk-facts gives figures for, in its order.
const PRIMARIES: [&str; 3] = ["red", "green", "blue"];

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
    /// A screen niri wouldn't turn on, and why: "{thing} not turned on
    /// · {why}".
    not_on: String,
    /// A screen niri wouldn't turn off, and why.
    not_off: String,
    /// A screen niri wouldn't unplug, and why.
    not_unplugged: String,
    /// A setting's choice niri wouldn't set, and why: "{thing} not set
    /// · {why}".
    not_set: String,
    /// Beside the choice a setting is at.
    chosen: String,
    /// A setting's value the : line typed that the screen doesn't offer,
    /// none typed, and what it offers: "{value} not set · {screen}
    /// offers {offers}", its figures "{first} to {last}".
    not_offered: String,
    none_given: String,
    offers_range: String,
    /// A screen gone before niri came to it: "{name} is unplugged".
    is_unplugged: String,
    /// Beside the last lit screen's Turn It Off: a key wakes it.
    wakes: String,
    /// Beside Unplug It: its windows move to the other screens.
    moves: String,
    /// Turn It On, Turn It Off and Unplug It, by their act's key.
    ways: BTreeMap<&'static str, acts::Way>,
    sleep: Sleep,
    told: Told,
    /// What a setting's drawer and Turn Every Screen Off's say.
    tells: tells::Tells,
    pub(crate) light: light::Words,
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
    /// How Variable Refresh stands, on or off.
    variable_on: String,
    variable_off: String,
    /// Each of niri's transforms as the : line's :setl rotation= takes
    /// it, "upright" for Normal.
    typed: BTreeMap<&'static str, String>,
    /// Every part of Screens the vocabulary names, by its id after
    /// `displays:`, which desk-facts' facts are named by.
    parts: BTreeMap<String, String>,
    white: String,
    primaries: [String; 3],
}

impl Told {
    /// A fact desk-facts names, as the vocabulary names its part, or its
    /// own name with a capital where the vocabulary has none, as the
    /// shell's partNamed said it: "hot-plug detection" is
    /// "Hot-plug detection".
    fn part(&self, id: &str) -> String {
        let key = id.split(' ').collect::<Vec<_>>().join("-");
        self.parts.get(&key).cloned().unwrap_or_else(|| {
            let mut letters = id.chars();
            letters.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(letters).collect()
            })
        })
    }
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
            not_on: said("not-on"),
            not_off: said("not-off"),
            not_unplugged: said("not-unplugged"),
            not_set: said("not-set"),
            chosen: said("chosen"),
            not_offered: said("not-offered"),
            none_given: said("none-given"),
            offers_range: said("offers-range"),
            is_unplugged: said("isUnplugged"),
            wakes: word("verb:display-off", &["said", "wakes"], "wakes"),
            moves: word("verb:display-unplug", &["said", "moves"], "moves"),
            ways: acts::Way::read(&word),
            tells: tells::Tells::read(&word),
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
                variable_on: word("displays:variable", &["said", "on"], "on"),
                variable_off: word("displays:variable", &["said", "off"], "off"),
                typed: TRANSFORMS
                    .iter()
                    .map(|turn| {
                        let typed = format!("typed-{turn}");
                        (*turn, word("displays:rotation", &["said", &typed], &typed))
                    })
                    .collect(),
                parts: things
                    .iter()
                    .filter_map(|(id, thing)| {
                        let part = id.strip_prefix("displays:")?;
                        Some((part.to_owned(), thing["name"].as_str()?.to_owned()))
                    })
                    .collect(),
                white: word("displays:white", &["said", "point"], "point"),
                primaries: PRIMARIES
                    .map(|colour| word("displays:primaries", &["said", colour], colour)),
            },
            light: light::Words::read(
                &word,
                things
                    .get("displays")
                    .map_or(&Value::Null, |screens| &screens["said"]),
            ),
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

/// The file desk-facts keeps what each screen's EDID says in, in its
/// folder in the runtime directory (modules/desk-facts).
pub const DESK: &str = "displays.json";

/// What desk-facts says of each screen, by connector, from its
/// displays.json; none where it says nothing readable, as when it's gone.
pub fn desk(said: &str) -> BTreeMap<String, Value> {
    serde_json::from_str::<Value>(said)
        .ok()
        .and_then(|said| said.as_object().cloned())
        .map(|screens| screens.into_iter().collect())
        .unwrap_or_default()
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

/// Whether no screen but the one of `name` is lit, so once it's dark a
/// key lights every screen again.
#[must_use]
pub fn last_lit(outputs: &BTreeMap<String, Value>, name: &str) -> bool {
    !outputs
        .iter()
        .any(|(other, output)| other != name && standing(output) == "lit")
}

/// A screen's row's name, by its connector, from niri's outputs; its
/// connector where niri has none of it.
#[must_use]
pub fn named_of(outputs: &BTreeMap<String, Value>, name: &str, words: &Words) -> String {
    outputs
        .get(name)
        .map_or_else(|| name.to_owned(), |output| named(name, output, words))
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
/// from the world (VKCF3Z, 2026-11-07); then what moves most, its light
/// (`lit`, light.rs's facts) where it holds it; then what it runs past its row,
/// where it sits and what it offers and is; then what its EDID says, as
/// desk-facts gives it (`desk`), its size from niri where that says none;
/// and its serial last, niri's or the EDID's, since it identifies the
/// screen. A screen that's off runs
/// nothing and sits nowhere, so says only what it offers and is. A
/// whole refresh, its size and its scale are its row's to say, so aren't
/// said again, and variable refresh is its switch's, so only a screen
/// without one says so.
fn drawer(name: &str, output: &Value, desk: &Value, lit: Vec<Value>, words: &Words) -> Value {
    let told = &words.told;
    let mut facts = lit;
    facts.extend(niris(output, words));
    let physical = |at: usize| output["physical_size"][at].as_f64().unwrap_or(0.0);
    let mut serials = vec![match &output["serial"] {
        Value::Null => String::new(),
        Value::String(said) => said.clone(),
        other => other.to_string(),
    }];
    let edid = facts::edid(desk, words);
    let sized = edid.iter().any(|(id, _)| id == "size");
    for (id, fact) in edid {
        if id == "serial" {
            serials.push(fact["said"].as_str().unwrap_or("").to_owned());
        } else {
            facts.push(fact);
        }
    }
    if !sized && physical(0) > 0.0 && physical(1) > 0.0 {
        let measured = &output["physical_size"];
        facts.push(json!({
            "name": told.named["size"],
            "said": format!("{} mm", size(&measured[0], &measured[1]))
        }));
    }
    if let Some(serial) = serials.into_iter().find(|serial| !serial.is_empty()) {
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

/// What niri says of a screen in its drawer, up to its density: the
/// mode it runs and where it sits while it's on, then the modes it offers
/// and how dense it is.
fn niris(output: &Value, words: &Words) -> Vec<Value> {
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
    facts
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

/// Whether the room's light is heard on a screen: the built-in screen's
/// lid holds the sensor, and so does the only screen there is, as in a
/// virtual machine's.
fn in_the_room(name: &str, screens: usize) -> bool {
    built_in(name) || screens == 1
}

/// The screen whose place holds the Brightness: the one the backlight
/// says it lights, else the built-in one while it's on or the only screen
/// there is, else the first lit monitor that follows the panel, as
/// desk-facts says by the brightness it read back from it. Only one holds
/// it, since every screen shows the same level; none where none is on.
fn backlit<'a>(
    shown: &[(&'a String, &Value)],
    desk: &BTreeMap<String, Value>,
    lights: Option<&str>,
) -> Option<&'a str> {
    let on: Vec<&String> = shown
        .iter()
        .filter(|(_, output)| !output["logical"].is_null())
        .map(|(name, _)| *name)
        .collect();
    let follows = |name: &str| desk.get(name).is_some_and(facts::follows);
    on.iter()
        .find(|name| Some(name.as_str()) == lights)
        .or_else(|| on.iter().find(|name| in_the_room(name, shown.len())))
        .or_else(|| on.iter().find(|name| follows(name)))
        .map(|name| name.as_str())
}

/// A row's key in the place.
pub fn key(id: &str) -> String {
    format!("row:{PLACE}:{id}")
}

/// Whether a watch's keys name a screen's row, as the shell's watch ahead
/// does while a screen is selected or stepped into, so its light is read.
#[must_use]
pub fn looks_at(keys: Option<&[String]>) -> bool {
    let screen = key("display:");
    keys.into_iter()
        .flatten()
        .any(|key| key.starts_with(&screen))
}

/// The screens' rows, the built-in one first and the rest by connector,
/// then Turn Every Screen Off while any is lit. `focused` is the
/// connector of the workspace in front, said beside its screen only
/// where more than one is lit. `desk` is what desk-facts says of each
/// screen's EDID, by connector, and `seen` the light while a screen is
/// looked at, which the drawer of the one holding it says. `each` is
/// whether niri darkens one screen by name (`NIRI_POWERS_OFF_EACH_OUTPUT`),
/// which the ways a screen turns off follow (acts.rs).
pub fn rows(
    outputs: &BTreeMap<String, Value>,
    desk: &BTreeMap<String, Value>,
    focused: Option<&str>,
    each: bool,
    seen: Option<&Seen>,
    words: &Words,
) -> Vec<Value> {
    let mut shown: Vec<(&String, &Value)> = outputs.iter().collect();
    shown.sort_by_key(|(name, _)| (!built_in(name), name.as_str()));
    let holds = seen.and_then(|seen| backlit(&shown, desk, seen.light.connector()));
    let part = |id: &str| words.told.part(id);
    let lit_by = |name: &str| match seen.filter(|_| holds == Some(name)) {
        Some(seen) => light::facts(
            seen,
            &light::Screen {
                in_the_room: in_the_room(name, shown.len()),
                named: &part,
                numbers: &words.numbers,
            },
            &words.light,
        ),
        None => Vec::new(),
    };
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
            // What the : line's :setl takes, on a lit screen alone.
            let line = if stands == "lit" && output["logical"].is_object() {
                line::of(output, words)
            } else {
                json!({})
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
                "drawer": drawer(name, output, desk.get(name.as_str()).unwrap_or(&Value::Null),
                    lit_by(name), words),
                "held": {"output": name, "standing": stands, "builtIn": built_in(name),
                    "niri": output, "line": line,
                    "follows": desk.get(name.as_str()).is_some_and(facts::follows)},
                "acts": acts::of(name, stands, lit == 1, each, words),
            })
        })
        .collect();
    // What a step into each screen offers, inside its row: a lit one's
    // settings, then the ways it turns off or on, then what its EDID says.
    for (name, output) in &shown {
        let stands = standing(output);
        if stands == "lit" && output["logical"].is_object() {
            rows.extend(settings::rows(name, output, words));
        }
        let screen = key(&format!("display:{name}"));
        rows.extend(acts::ways(name, stands, lit == 1, each, &screen, words));
        let desk = desk.get(name.as_str()).unwrap_or(&Value::Null);
        rows.extend(facts::rows(name, &screen, desk, words));
    }
    if lit > 0 {
        let turned_off: Vec<String> = shown
            .iter()
            .filter(|(_, output)| standing(output) == "lit")
            .map(|(name, output)| named(name, output, words))
            .collect();
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
            "drawer": tells::sleep(&turned_off, words),
            "acts": [{"key": "displays:sleep", "ask": Act::Sleep.ask()}],
        }));
    }
    rows
}

#[cfg(test)]
mod tests;
