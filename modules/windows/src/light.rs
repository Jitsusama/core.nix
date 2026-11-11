//! The light a screen's drawer says, from what the machine daemon serves
//! of it (modules/machine's light.rs and steered.rs): the backlight's
//! steps, the room's sensor, the level the backlight program settled and
//! its curve, and the warmth. Displays.qml said these from the same
//! records (Backlight.js, Warmth.js); they're said here now, so the shell
//! only draws them.
//!
//! The machine daemon reads the light once a second while it's watched,
//! so it's watched only while a screen's row is looked ahead at, as the
//! shell does once a screen is selected or stepped into (main.rs's
//! `follow_light`); the drawer says it only then.

use std::collections::BTreeMap;
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::time::Duration;

use ready_rows::numbers::Numbers;
use serde_json::{Value, json};

/// The machine daemon's kinds a screen's light is said from.
pub const KINDS: [&str; 4] = ["backlight", "light", "brightness", "warmth"];

/// How long the light may go unread before the drawer says so.
pub const WAITED: Duration = Duration::from_secs(1);

/// The id the light is watched by on the machine daemon's socket.
const WATCH: &str = "light";

const PERCENT: f64 = 100.0;

/// The backlight program moves the level only for a gap this wide or
/// wider (its steer.rs's SLIGHT), so a narrower one is where it rests.
const SLIGHT: f64 = 0.03;

/// Lux are said in lux, and from a thousand in klx; a reading whose three
/// digits round to a thousand reads "1.00 klx" rather than "1,000 lx".
const KILO: f64 = 1000.0;
const ROUNDS_TO_KILO: f64 = 999.5;

/// Places enough for any count a sensor or backlight says, trailing
/// zeros dropped, as Numbers.js's format took by default.
const EVERY_PLACE: usize = 10;

/// Two items of one value, as a figure's runs are set apart.
const APART: &str = " \u{b7} ";

/// Whether what's held of the light was read for this look: `Read`;
/// `Waiting`, its first second, said as nothing, since the machine
/// daemon's answer nearly always beats it and no word flashes past; or
/// `Unread`, after that, said as not read yet (VKCF3Z, 2026-11-10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reading {
    Read,
    Waiting,
    Unread,
}

/// The machine daemon's records of the screen's light, by key, kept
/// between looks so a drawer keeps its place while the next is read.
#[derive(Debug)]
pub struct Held {
    records: BTreeMap<String, Value>,
    /// Whether a watch's keys name a screen's row, so the light is
    /// watched and said.
    pub looking: bool,
    pub reading: Reading,
    /// The connection to the machine daemon while it's watched, hung up
    /// on once nothing looks.
    pub stream: Option<UnixStream>,
}

impl Default for Held {
    fn default() -> Held {
        Held {
            records: BTreeMap::new(),
            looking: false,
            reading: Reading::Waiting,
            stream: None,
        }
    }
}

impl Held {
    /// The watch the machine daemon is asked, as a line.
    #[must_use]
    pub fn watch() -> String {
        json!({"id": WATCH, "ask": "watch", "kind": KINDS}).to_string()
    }

    /// The light as what's held says it.
    #[must_use]
    pub fn light(&self) -> Light {
        Light::of(self.records.values())
    }

    /// A screen came to be looked at, or none is any more: the light is
    /// said not yet read as a look begins, and the machine daemon hung up
    /// on as the last ends. Whether that changed anything.
    pub fn look(&mut self, looking: bool) -> bool {
        if self.looking == looking {
            return false;
        }
        self.looking = looking;
        if looking {
            self.reading = Reading::Waiting;
        } else if let Some(stream) = self.stream.take() {
            let _ = stream.shutdown(Shutdown::Both);
        }
        true
    }

    /// A wait lapsed, or the connection `ended` while a screen is looked
    /// at: the light still unread is said as not read yet, and all of it
    /// once the machine daemon stops answering, since what's held is old.
    pub fn lapsed(&mut self, ended: bool) {
        if ended || self.reading == Reading::Waiting {
            self.reading = Reading::Unread;
        }
    }

    /// How long a connection may hear nothing before the drawer is said
    /// again: the rest of the light's first second while it's unread, or
    /// until the night light's next turn; none where nothing waits.
    /// `heard` is how long the connection has been heard, `now` the time.
    #[must_use]
    pub fn wait(&self, heard: Duration, now: u64) -> Option<Duration> {
        let waited = (self.reading == Reading::Waiting).then(|| WAITED.saturating_sub(heard));
        let turned = self
            .light()
            .next_turn(now)
            .map(|at| Duration::from_millis(at.saturating_sub(now)));
        // A socket refuses a wait of none, so the least is a millisecond.
        waited
            .into_iter()
            .chain(turned)
            .min()
            .map(|wait| wait.max(Duration::from_millis(1)))
    }

    /// One line of the machine daemon's: a whole answer replaces what's
    /// held, and a change replaces its records and takes out those gone;
    /// either way the light is read now. Whether it was the light's.
    pub fn heard(&mut self, line: &str) -> bool {
        let Ok(said) = serde_json::from_str::<Value>(line) else {
            eprintln!("<4>the machine daemon said what isn't JSON: {line}");
            return false;
        };
        if said["id"] != WATCH {
            return false;
        }
        if let Some(why) = said.get("error") {
            eprintln!("<4>the machine daemon refused to say the light: {why}");
            return false;
        }
        let keyed = |record: &Value| record["key"].as_str().map(str::to_owned);
        if let Some(records) = said["records"].as_array() {
            self.records = records
                .iter()
                .filter_map(|record| Some((keyed(record)?, record.clone())))
                .collect();
        }
        for record in said["changed"].as_array().into_iter().flatten() {
            let Some(key) = keyed(record) else { continue };
            if record["gone"] == true {
                self.records.remove(&key);
            } else {
                self.records.insert(key, record.clone());
            }
        }
        for key in said["gone"].as_array().into_iter().flatten() {
            if let Some(key) = key.as_str() {
                self.records.remove(key);
            }
        }
        self.reading = Reading::Read;
        true
    }
}

/// The words the light is said in, from the launcher's vocabulary.
#[derive(Clone, Debug, PartialEq)]
pub struct Words {
    /// Screens' own words (`displays`'s said), by key.
    said: BTreeMap<String, String>,
    /// Warmth's (`displays:warmth`'s said), by key.
    warmth: BTreeMap<String, String>,
}

/// The words of Screens' the light says, each said as its key where the
/// vocabulary lacks it.
const SAID: [&str; 7] = [
    "steps",
    "unread",
    "the-room",
    "on-its-way",
    "no-light",
    "at",
    "perCount",
];

/// The words of Warmth's the light says.
const WARMTH: [&str; 5] = ["until", "from", "waits", "following", "own"];

impl Words {
    /// The light's words, `word` looking one up by its thing's id and
    /// path, as screens.rs's Words does, and `screens` Screens' own said,
    /// which also holds how each backlight is stepped and driven.
    #[must_use]
    pub fn read(word: &dyn Fn(&str, &[&str], &str) -> String, screens: &Value) -> Words {
        let mut said: BTreeMap<String, String> = screens
            .as_object()
            .into_iter()
            .flatten()
            .filter(|(key, _)| key.starts_with("stepping-") || key.starts_with("driven-"))
            .filter_map(|(key, value)| Some((key.clone(), value.as_str()?.to_owned())))
            .collect();
        for key in SAID {
            said.insert(key.to_owned(), word("displays", &["said", key], key));
        }
        let warmth = WARMTH
            .iter()
            .map(|key| {
                let said = word("displays:warmth", &["said", key], key);
                ((*key).to_owned(), said)
            })
            .collect();
        Words { said, warmth }
    }

    fn said(&self, key: &str) -> &str {
        self.said.get(key).map_or("", String::as_str)
    }
}

/// The screen's backlight: its steps now and most, the steps the hardware
/// reads back where they differ, how its steps map to light, what drives
/// it, its device and the screen it lights.
#[derive(Clone, Debug, PartialEq)]
struct Lit {
    now: f64,
    most: f64,
    reads: Option<f64>,
    stepping: Option<String>,
    driven: Option<String>,
    device: String,
    connector: Option<String>,
}

/// The room's light sensor: the lux it hears, its raw count, the lux a
/// count is, its offset and its device.
#[derive(Clone, Debug, PartialEq)]
struct Sensor {
    lux: Option<f64>,
    raw: Option<f64>,
    per_count: f64,
    offset: f64,
    device: String,
}

/// The level the backlight program last settled, the lux it heard and
/// what its curve asks for them, and whether the curve sets the level.
#[derive(Clone, Debug, PartialEq)]
struct Settled {
    level: f64,
    lux: Option<f64>,
    asks: Option<f64>,
    following: bool,
}

/// What holds the warmth: the kelvin now, whether it follows the room and
/// a sensor sees colour, and the night light, with the night's sunset and
/// sunrise in milliseconds since the epoch.
#[derive(Clone, Debug, PartialEq)]
struct Warmth {
    kelvin: f64,
    follows_the_room: bool,
    sees_colour: bool,
    night_light: bool,
    sunset: Option<u64>,
    sunrise: Option<u64>,
}

/// The light as the machine daemon's records say it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Light {
    lit: Option<Lit>,
    sensor: Option<Sensor>,
    settled: Option<Settled>,
    /// The curve's points, dimmest first: its lux and level.
    curve: Vec<(f64, f64)>,
    warmth: Option<Warmth>,
}

fn finite(value: &Value) -> Option<f64> {
    value.as_f64().filter(|value| value.is_finite())
}

fn text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}

/// The first of a kind's records that holds, by its device's name.
fn first<'a>(
    records: &[&'a Value],
    kind: &str,
    holds: impl Fn(&Value) -> bool,
) -> Option<&'a Value> {
    let mut found: Vec<&Value> = records
        .iter()
        .copied()
        .filter(|record| record["kind"] == kind && holds(record))
        .collect();
    found.sort_by_key(|record| record["device"].as_str().unwrap_or("").to_lowercase());
    found.first().copied()
}

impl Light {
    /// The light the machine daemon's records say: the first backlight
    /// that says the most it can be and the first sensor, by device name,
    /// the built-in screen's settled level and curve, and its warmth.
    #[must_use]
    pub fn of<'a>(records: impl IntoIterator<Item = &'a Value>) -> Light {
        let records: Vec<&Value> = records.into_iter().collect();
        let lit = first(&records, "backlight", |record| {
            finite(&record["max_brightness"]).is_some_and(|most| most > 0.0)
                && finite(&record["brightness"]).is_some()
        })
        .map(|found| {
            let now = finite(&found["brightness"]).unwrap_or(0.0);
            Lit {
                now,
                most: finite(&found["max_brightness"]).unwrap_or(1.0),
                reads: finite(&found["actual_brightness"])
                    .filter(|reads| (reads - now).abs() > f64::EPSILON),
                stepping: text(&found["scale"]),
                driven: text(&found["type"]),
                device: text(&found["device"]).unwrap_or_default(),
                connector: text(&found["connector"]),
            }
        });
        let sensor = first(&records, "light", |_| true).map(|found| Sensor {
            lux: finite(&found["lux"]),
            raw: finite(&found["raw"]),
            per_count: finite(&found["scale"]).unwrap_or(0.0),
            offset: finite(&found["offset"]).unwrap_or(0.0),
            device: text(&found["device"]).unwrap_or_default(),
        });
        let brightness = records.iter().find(|record| record["kind"] == "brightness");
        let settled = brightness.and_then(|found| {
            Some(Settled {
                level: finite(&found["level_ratio"])?,
                lux: finite(&found["lux"]),
                asks: finite(&found["asks_ratio"]),
                following: found["following"].as_bool()?,
            })
        });
        let curve = brightness
            .and_then(|found| found["curve"].as_array())
            .into_iter()
            .flatten()
            .filter_map(|point| Some((finite(&point["lux"])?, finite(&point["level_ratio"])?)))
            .collect();
        let warmth = records
            .iter()
            .find(|record| record["kind"] == "warmth")
            .and_then(|found| {
                Some(Warmth {
                    kelvin: finite(&found["kelvin"])?,
                    follows_the_room: found["follows_the_room"].as_bool()?,
                    sees_colour: found["sees_colour"] == true,
                    night_light: found["night_light"] == true,
                    sunset: found["sunset_ms"].as_u64(),
                    sunrise: found["sunrise_ms"].as_u64(),
                })
            });
        Light {
            lit,
            sensor,
            settled,
            curve,
            warmth,
        }
    }

    /// The screen the backlight lights, by connector, where it says one.
    #[must_use]
    pub fn connector(&self) -> Option<&str> {
        self.lit.as_ref()?.connector.as_deref()
    }

    /// When what holds the warmth next changes by the clock alone, the
    /// night light's sunset or sunrise after `now`, so the drawer is said
    /// again then; none where the night light is off.
    #[must_use]
    pub fn next_turn(&self, now: u64) -> Option<u64> {
        let warmth = self.warmth.as_ref().filter(|warmth| warmth.night_light)?;
        [warmth.sunset, warmth.sunrise]
            .into_iter()
            .flatten()
            .filter(|at| *at > now)
            .min()
    }
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

/// A level as a share of the whole, rounded as Brightness says it.
fn share(level: f64) -> String {
    format!("{}%", (level * PERCENT).round())
}

/// The room's light as it's measured, to three significant digits with
/// its zeros kept: "28.5 lx", "5.20 lx", "1.24 klx" (VKCF3Z, 2026-11-08).
fn reading(lux: f64, numbers: &Numbers) -> String {
    if lux >= ROUNDS_TO_KILO {
        format!("{} klx", numbers.significant(lux / KILO))
    } else {
        format!("{} lx", numbers.significant(lux))
    }
}

/// A curve's lux, a point the program chose rather than one it measured,
/// so a name: every digit and none added, in a reading's units so the two
/// compare: "50 lx", "1.5 klx" (VKCF3Z, 2026-11-08).
fn chosen(lux: f64, numbers: &Numbers) -> String {
    if lux >= KILO {
        format!("{} klx", numbers.number(lux / KILO, EVERY_PLACE))
    } else {
        format!("{} lx", numbers.number(lux, EVERY_PLACE))
    }
}

/// What the Room Light says: the room as the sensor hears it now, or as
/// the program last heard it; or, where it heard no light, that none is,
/// so the drawer says why Follows is gone (VKCF3Z, 2026-11-08). "" where
/// nothing says anything.
fn room(settled: Option<&Settled>, heard: Option<f64>, words: &Words, numbers: &Numbers) -> String {
    if let Some(heard) = heard {
        return reading(heard, numbers);
    }
    match settled {
        None => String::new(),
        Some(settled) => settled.lux.map_or_else(
            || words.said("no-light").to_owned(),
            |lux| reading(lux, numbers),
        ),
    }
}

/// What sets the level: the room while the curve follows it, and what it
/// asks while the level is still stepping there; "" while no light is
/// heard or the lid is shut, where nothing does.
fn follows(settled: Option<&Settled>, words: &Words) -> String {
    let Some(settled) = settled.filter(|settled| settled.following) else {
        return String::new();
    };
    let stepping = settled
        .asks
        .filter(|asks| (asks - settled.level).abs() >= SLIGHT)
        .map_or_else(String::new, |asks| {
            words.said("on-its-way").replace("{share}", &share(asks))
        });
    runs(&[words.said("the-room"), &stepping])
}

/// The Warmth: its kelvin, then what holds it, as its row's note says it:
/// the night light until sunrise or from sunset, said by the shell's clock
/// as a time, or waiting for where Joel is; else the room, or the
/// screen's own white.
fn warmth(warmth: &Warmth, name: &str, now: u64, words: &Words, numbers: &Numbers) -> Value {
    let kelvin = format!("{} K", numbers.number(warmth.kelvin, 0));
    let said = |key: &str| words.warmth.get(key).map_or("", String::as_str);
    let timed = |key: &str, at: u64| {
        json!({"name": name, "at": at, "as": "moment",
            "said": runs(&[&kelvin, said(key)])})
    };
    match (warmth.night_light, warmth.sunset, warmth.sunrise) {
        (true, None, _) => json!({"name": name, "said": runs(&[&kelvin, said("waits")])}),
        (true, Some(sunset), Some(sunrise)) if sunset <= now && now < sunrise => {
            timed("until", sunrise)
        }
        (true, Some(sunset), _) => timed("from", sunset),
        (false, _, _) => {
            let holds = if warmth.follows_the_room && warmth.sees_colour {
                "following"
            } else {
                "own"
            };
            json!({"name": name, "said": runs(&[&kelvin, said(holds)])})
        }
    }
}

/// The light as it's said while a screen is looked at: what's held of it,
/// whether that was read for this look, and the time now, in milliseconds
/// since the epoch, which the warmth's night light is said by.
pub struct Seen<'a> {
    pub light: &'a Light,
    pub reading: Reading,
    pub now: u64,
}

/// A screen the light is said in: whether it holds the room's sensor, as
/// the built-in screen's lid does and the only screen there is, so says
/// its steps and the room's light too.
pub struct Screen<'a> {
    pub in_the_room: bool,
    /// A fact's name, as the vocabulary names the part (`displays:<id>`).
    pub named: &'a dyn Fn(&str) -> String,
    pub numbers: &'a Numbers,
}

/// The light's facts in the drawer of the screen whose place holds the
/// Brightness, in the order Displays.qml laid them: Brightness with its
/// steps, what the kernel says of them, the Room light and what the
/// sensor counts, Follows, Warmth, the curve, then the backlight's and
/// the sensor's devices, which identify them. A monitor following the
/// panel says only the Brightness. None without a backlight.
#[must_use]
pub fn facts(seen: &Seen, screen: &Screen, words: &Words) -> Vec<Value> {
    let (light, reading) = (seen.light, seen.reading);
    let Some(lit) = &light.lit else {
        return Vec::new();
    };
    let named = screen.named;
    let numbers = screen.numbers;
    let count = |value: f64| numbers.number(value, EVERY_PLACE);
    let read = reading == Reading::Read;
    let brightness = match reading {
        Reading::Waiting => String::new(),
        Reading::Unread => words.said("unread").to_owned(),
        // A hundred steps are its share again.
        Reading::Read if screen.in_the_room && (lit.most - PERCENT).abs() > f64::EPSILON => {
            let steps = words
                .said("steps")
                .replace("{now}", &count(lit.now))
                .replace("{most}", &count(lit.most));
            runs(&[&share(lit.now / lit.most), &steps])
        }
        Reading::Read => share(lit.now / lit.most),
    };
    let mut facts = vec![json!({
        "name": named("brightness"),
        "said": brightness,
        "figure": {"shape": "stroke", "values": [if read { lit.now / lit.most } else { 0.0 }],
            "warn": false},
    })];
    if !screen.in_the_room {
        return facts;
    }
    let mut add = |id: &str, said: String| {
        if !said.is_empty() {
            facts.push(json!({"name": named(id), "said": said}));
        }
    };
    let word = |kind: &str, key: &Option<String>| {
        key.as_ref()
            .map_or("", |key| words.said(&format!("{kind}-{key}")))
            .to_owned()
    };
    add("reads", lit.reads.map_or_else(String::new, count));
    add("stepping", word("stepping", &lit.stepping));
    add("driven-by", word("driven", &lit.driven));
    let heard = light
        .sensor
        .as_ref()
        .filter(|_| read)
        .and_then(|sensor| sensor.lux);
    add("room", room(light.settled.as_ref(), heard, words, numbers));
    if let Some(sensor) = &light.sensor {
        add(
            "raw",
            sensor.raw.filter(|_| read).map_or_else(String::new, count),
        );
        add(
            "per-count",
            words
                .said("perCount")
                .replace("{n}", &count(sensor.per_count)),
        );
        add(
            "offset",
            if sensor.offset.abs() > 0.0 {
                count(sensor.offset)
            } else {
                String::new()
            },
        );
    }
    add("follows", follows(light.settled.as_ref(), words));
    if let Some(warmed) = &light.warmth {
        facts.push(warmth(warmed, &named("warmth"), seen.now, words, numbers));
    }
    let curve: Vec<String> = light
        .curve
        .iter()
        .map(|(lux, level)| {
            words
                .said("at")
                .replace("{share}", &share(*level))
                .replace("{lux}", &chosen(*lux, numbers))
        })
        .collect();
    if !curve.is_empty() {
        facts.push(json!({"name": named("curve"), "said": curve.join(APART)}));
    }
    facts.push(json!({"name": named("device"), "said": lit.device, "identifies": true}));
    if let Some(sensor) = &light.sensor {
        facts.push(json!({"name": named("sensor"), "said": sensor.device, "identifies": true}));
    }
    facts
}

#[cfg(test)]
mod tests;
