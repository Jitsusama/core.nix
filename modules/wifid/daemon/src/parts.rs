//! A network's insides, made ready while the launcher looks at it: its
//! drawer, and the rows a step into it shows (a card's signal, speed and
//! traffic, how it's protected, and each access point heard), each worded
//! from the launcher's vocabulary and said in the account's numbers, so
//! the shell only draws them (QEST-20261023-Y2YBTS attachments/speed/
//! ready-rows.md). They're made only while a watch has the network's row
//! ahead, as the kernel's station is read only then (face.rs), and a
//! part's key is its network's with its own after it, so two networks
//! looked at once never share one.
//!
//! Long because a network's insides are some sixty parts, each worded
//! from words it must find, and its cases stay beside the private makers
//! they read.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Display;

use ready_rows::numbers::Numbers;
use ready_rows::time::{self, Form};
use serde_json::{Map, Value, json};

use crate::detail::{Detail, Joined, Point, Station, Way};
use crate::radio::{Band, Management};
use crate::rows::{self, PLACE};

/// The signal a bar is empty and full at, in dBm, as the shell's own
/// level says (Interfaces.js).
const FAINTEST: f64 = -90.0;
const LOUDEST: f64 = -30.0;
/// The reads a spark holds, and the steps a bar and a spark's dot are
/// drawn in (ready-rows.md).
const GRAPHED: usize = 20;
const BAR_STEPS: f64 = 20.0;
const DOT_STEPS: f64 = 4.0;
/// An access point's time unit, in milliseconds.
const TIME_UNIT: f64 = 1.024;
/// How many letters a note has room for beside the footer's keys.
const ROOM: usize = 40;
/// How far apart two reads of when a card joined may be and still be the
/// one joining, in milliseconds: the kernel counts in whole seconds, so
/// each poll's sum wanders by one.
const SAME_JOINING: i64 = 5_000;
/// An access point heard within this many seconds was heard just now.
const JUST_NOW: u32 = 10;
/// A second and a minute, in milliseconds.
const SECOND: u32 = 1_000;
const MINUTE: u32 = 60_000;
/// Between parts said together.
const BETWEEN: &str = " \u{b7} ";
/// SI's own symbols for a size, as every size the launcher says reads
/// (FileFacts.weighed), with the space kept that never breaks.
const SIZES: [(f64, &str); 4] = [(1e12, "TB"), (1e9, "GB"), (1e6, "MB"), (1e3, "kB")];
const KEPT: char = '\u{a0}';

/// Every word the parts are said in, by thing (after `wifi:`) and word, a
/// word of "" its name, so a vocabulary that lost one is said as it's
/// handed over rather than shown as a blank.
const NEEDED: &[(&str, &str)] = &[
    ("figures", "dbm"),
    ("figures", "db"),
    ("figures", "mbps"),
    ("figures", "mhz"),
    ("figures", "ms"),
    ("figures", "s"),
    ("figures", "us"),
    ("figures", "tu"),
    ("figures", "percent"),
    ("figures", "wifi"),
    ("figures", "mcs"),
    ("figures", "in"),
    ("figures", "out"),
    ("figures", "on"),
    ("figures", "off"),
    ("figures", "short"),
    ("figures", "long"),
    ("figures", "unread"),
    ("figures", "ghz2"),
    ("figures", "ghz5"),
    ("figures", "ghz6"),
    ("network", "bars"),
    ("network", "password"),
    ("network", "account"),
    ("card", ""),
    ("signal", ""),
    ("signal", "shown"),
    ("antenna", ""),
    ("average", ""),
    ("beacons", ""),
    ("beacons", "note"),
    ("interval", ""),
    ("interval", "note"),
    ("dtim", ""),
    ("dtim", "every"),
    ("dtim", "one"),
    ("dtim", "note"),
    ("dtim", "asleep"),
    ("margin", ""),
    ("margin", "noise"),
    ("power", ""),
    ("power", "note"),
    ("speed", ""),
    ("speed", "expected"),
    ("in", ""),
    ("out", ""),
    ("expected", ""),
    ("expected", "note"),
    ("generation", ""),
    ("generation", "older"),
    ("generation", "stream"),
    ("generation", "streams"),
    ("coding", ""),
    ("coding", "note"),
    ("streams", ""),
    ("streams", "note"),
    ("guard", ""),
    ("guard", "note"),
    ("dcm", ""),
    ("dcm", "note"),
    ("width", ""),
    ("channel", ""),
    ("preamble", ""),
    ("preamble", "note"),
    ("slot", ""),
    ("slot", "note"),
    ("wmm", ""),
    ("wmm", "note"),
    ("retries", ""),
    ("retries", "none"),
    ("retries", "some"),
    ("retries", "note"),
    ("carried", ""),
    ("carried", "note"),
    ("carried", "packet"),
    ("carried", "packets"),
    ("received", ""),
    ("sent", ""),
    ("idle", ""),
    ("idle", "note"),
    ("retried", ""),
    ("retried", "note"),
    ("failed", ""),
    ("failed", "note"),
    ("missed", ""),
    ("missed", "note"),
    ("dropped", ""),
    ("dropped", "note"),
    ("protection", ""),
    ("protection", "none"),
    ("protection", "open"),
    ("protection", "guarded"),
    ("protection", "strong"),
    ("protection", "good"),
    ("protection", "broken"),
    ("protection", "weak"),
    ("standard", ""),
    ("proof", ""),
    ("proof", "note"),
    ("proof", "or"),
    ("cipher", ""),
    ("cipher", "note"),
    ("group", ""),
    ("group", "note"),
    ("frames", ""),
    ("frames", "off"),
    ("frames", "optional"),
    ("frames", "required"),
    ("frames", "mine"),
    ("frames", "notMine"),
    ("frames", "note"),
    ("connected", ""),
    ("heard", ""),
    ("heard", "each"),
    ("heard", "bands"),
    ("heard", "ghz2"),
    ("heard", "ghz5"),
    ("heard", "ghz6"),
    ("point", ""),
    ("point", "side"),
    ("point", "connected"),
    ("point", "fastest"),
    ("estimate", ""),
    ("estimate", "side"),
    ("estimate", "note"),
    ("busy", ""),
    ("busy", "note"),
    ("stations", ""),
    ("stations", "note"),
    ("heard-ago", ""),
    ("heard-ago", "now"),
    ("heard-ago", "ago"),
    ("fastest", ""),
    ("bssid", ""),
    ("drawer", "eyebrow"),
    ("drawer", "kind"),
    ("drawer", "legacy"),
    ("drawer", "over"),
    ("drawer", "since"),
];

/// The words a network's insides are said in: Wi-Fi's things in the
/// launcher's vocabulary, by id after `wifi:`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Words {
    things: HashMap<String, Value>,
}

impl Words {
    /// Wi-Fi's things from the whole vocabulary, or the first word one
    /// lacks.
    pub fn read(vocabulary: &Value) -> Result<Words, String> {
        let things: HashMap<String, Value> = vocabulary["things"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|thing| {
                let id = thing["id"].as_str()?.strip_prefix("wifi:")?;
                Some((id.to_owned(), thing.clone()))
            })
            .collect();
        for (id, word) in NEEDED {
            let thing = things
                .get(*id)
                .ok_or_else(|| format!("the vocabulary has no wifi:{id}"))?;
            let said = if word.is_empty() {
                &thing["name"]
            } else {
                &thing["said"][word]
            };
            if !said.is_string() {
                let what = if word.is_empty() { "name" } else { *word };
                return Err(format!("the vocabulary's wifi:{id} has no {what}"));
            }
        }
        Ok(Words { things })
    }

    fn name(&self, id: &str, fill: &[(&str, &str)]) -> String {
        filled(self.word(id, &["name"]), fill)
    }

    fn said(&self, id: &str, word: &str, fill: &[(&str, &str)]) -> String {
        filled(self.word(id, &["said", word]), fill)
    }

    fn word(&self, id: &str, at: &[&str]) -> &str {
        self.things
            .get(id)
            .map(|thing| at.iter().fold(thing, |within, field| &within[field]))
            .and_then(Value::as_str)
            .unwrap_or("")
    }
}

/// Words with each `{name}` in them filled.
fn filled(words: &str, fill: &[(&str, &str)]) -> String {
    fill.iter().fold(words.to_owned(), |said, (name, value)| {
        said.replace(&format!("{{{name}}}"), value)
    })
}

/// What's kept of a network while it's looked at, so its parts hold still
/// as its readings change: its access points in the order first shown,
/// and when each card on it joined.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Look {
    order: Vec<String>,
    since: BTreeMap<String, i64>,
}

impl Look {
    /// Follows what's read of the network now: an access point first
    /// heard goes after those shown, the fastest of the new first, and a
    /// card keeps the time it joined while each read of it says much the
    /// same, so its row isn't told again for a second's wander.
    pub fn follow(&mut self, detail: &Detail, now: i64) {
        let heard = |bssid: &String| detail.points.iter().any(|point| &point.bssid == bssid);
        self.order.retain(heard);
        let mut fresh: Vec<&Point> = detail
            .points
            .iter()
            .filter(|point| !self.order.contains(&point.bssid))
            .collect();
        fresh.sort_by(|a, b| b.estimate.cmp(&a.estimate).then(b.signal.cmp(&a.signal)));
        self.order
            .extend(fresh.into_iter().map(|point| point.bssid.clone()));
        let mut since = BTreeMap::new();
        for link in &detail.joined {
            let Some(seconds) = link.station.as_ref().and_then(|station| station.connected) else {
                continue;
            };
            let at = now - i64::from(seconds) * i64::from(SECOND);
            let kept = match self.since.get(&link.card) {
                Some(was) if (at - was).abs() <= SAME_JOINING => *was,
                _ => at,
            };
            since.insert(link.card.clone(), kept);
        }
        self.since = since;
    }
}

/// A network's insides: its drawer, while a card's on it, and every row
/// a step in shows, each under its parent, the network's row first.
#[derive(Debug, Default, PartialEq)]
pub struct Insides {
    pub drawer: Option<Value>,
    pub rows: Vec<Value>,
}

/// What's read of a network now, to make its insides from.
pub struct Read<'a> {
    pub detail: &'a Detail,
    /// Whether a card's on it, as its row says.
    pub connected: bool,
    pub look: &'a Look,
    /// The signal each card's read since it was looked at, in dBm, the
    /// newest last.
    pub seen: &'a BTreeMap<String, Vec<i32>>,
    pub now: i64,
}

/// A network's insides, worded and numbered as the account says. A
/// network joined waits for its card's link, so its parts arrive
/// together, in their order, rather than the link's below the rest.
pub fn insides(read: &Read, words: &Words, numbers: &Numbers) -> Insides {
    let detail = read.detail;
    if detail.points.is_empty() || (read.connected && detail.joined.is_empty()) {
        return Insides::default();
    }
    let say = Say {
        words,
        numbers,
        read,
    };
    let mut made = Vec::new();
    if let [link] = detail.joined.as_slice() {
        made.extend(say.link_parts(link));
    } else {
        for (at, link) in detail.joined.iter().enumerate() {
            let mut inside = say.link_parts(link);
            inside.extend(say.connected(&link.card));
            let side = link
                .signal
                .map(|signal| say.dbm(signal))
                .unwrap_or_default();
            let mut card = holds(format!("card-{}", link.card), &side, inside);
            card.name = words.name("card", &[("n", &(at + 1).to_string())]);
            made.push(card);
        }
    }
    made.push(say.guard());
    made.push(say.heard());
    // How long it's been on comes last, where it lands whenever the
    // kernel's read.
    if let [link] = detail.joined.as_slice() {
        made.extend(say.connected(&link.card));
    }
    let parent = rows::key(&format!("network:{}", detail.name));
    let mut out = Vec::new();
    placed(made, &parent, 1, &mut out);
    Insides {
        drawer: if read.connected { say.drawer() } else { None },
        rows: out,
    }
}

/// One part as it's made, before it's placed under its parent: what it
/// copies is its first act, and a part holding more steps in.
#[derive(Default)]
struct Made {
    id: String,
    kind: &'static str,
    name: String,
    glyph: String,
    side: Vec<Value>,
    copies: String,
    graph: Option<Value>,
    note: Value,
    inside: Vec<Made>,
}

impl Made {
    fn named(mut self, name: String) -> Made {
        self.name = name;
        self
    }

    /// Its note, none where it's said as nothing.
    fn noted(mut self, note: impl Into<String>) -> Made {
        let note = note.into();
        self.note = if note.is_empty() {
            Value::Null
        } else {
            json!(note)
        };
        self
    }

    fn graphed(mut self, graph: Value) -> Made {
        self.graph = Some(graph);
        self
    }

    fn copying(mut self, copies: impl Into<String>) -> Made {
        self.copies = copies.into();
        self
    }
}

/// A part saying a figure, which it copies, as a figure wears no glyph.
/// Its name is its id's until `named` says otherwise.
fn fact(id: impl Into<String>, side: &str) -> Made {
    Made {
        id: id.into(),
        kind: "figure",
        side: said(side),
        copies: side.to_owned(),
        ..Made::default()
    }
}

/// A part holding more, which copies its figure and steps in, wearing
/// Wi-Fi's glyph.
fn holds(id: impl Into<String>, side: &str, inside: Vec<Made>) -> Made {
    Made {
        id: id.into(),
        kind: "holder",
        glyph: "wireless".to_owned(),
        side: said(side),
        copies: side.to_owned(),
        inside,
        ..Made::default()
    }
}

/// A side of one item saying `side`, or none when it says nothing.
fn said(side: &str) -> Vec<Value> {
    if side.is_empty() {
        Vec::new()
    } else {
        vec![json!({ "said": side })]
    }
}

/// Each part as a record of Wi-Fi's place, in the order shown, each under
/// its parent and keyed after it.
fn placed(made: Vec<Made>, parent: &str, depth: u64, out: &mut Vec<Value>) {
    for part in made {
        let key = format!("{parent}:{}", part.id);
        let mut record = Map::new();
        let mut set = |field: &str, value: Value| {
            record.insert(field.to_owned(), value);
        };
        set("key", json!(key));
        set("thing", json!(key.strip_prefix("row:").unwrap_or(&key)));
        set("place", json!(PLACE));
        set("kind", json!(part.kind));
        set("parent", json!(parent));
        set("depth", json!(depth));
        set("name", json!(part.name));
        if !part.glyph.is_empty() {
            set("glyph", json!(part.glyph));
        }
        set("hue", json!("machine"));
        set("side", Value::Array(part.side));
        if let Some(graph) = part.graph {
            set("graph", graph);
        }
        if !part.note.is_null() {
            set("note", part.note);
        }
        let mut acts = Vec::new();
        if !part.copies.is_empty() {
            acts.push(json!({ "key": "verb:copy", "copy": part.copies }));
        }
        if !part.inside.is_empty() {
            acts.push(json!({ "key": "verb:more", "step": "rows" }));
        }
        set("acts", Value::Array(acts));
        out.push(Value::Object(record));
        placed(part.inside, &key, depth + 1, out);
    }
}

/// A signal from 0 to 1, as the shell's bars light it.
fn level(dbm: f64) -> f64 {
    ((dbm - FAINTEST) / (LOUDEST - FAINTEST)).clamp(0.0, 1.0)
}

/// A value kept between 0 and 1 in `steps` steps, any above nothing at
/// least one, as the shell lights it.
fn stepped(value: f64, steps: f64) -> f64 {
    let kept = if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, 1.0)
    };
    if kept <= 0.0 {
        0.0
    } else {
        (kept * steps).round().max(1.0) / steps
    }
}

fn bar(value: f64) -> Value {
    json!({ "shape": "bar", "values": [stepped(value, BAR_STEPS)], "warn": false })
}

/// The signal's last reads, newest last.
fn spark(dbm: &[i32]) -> Value {
    let from = dbm.len().saturating_sub(GRAPHED);
    let drawn: Vec<f64> = dbm[from..]
        .iter()
        .map(|value| stepped(level(f64::from(*value)), DOT_STEPS))
        .collect();
    json!({ "shape": "spark", "values": drawn, "warn": false })
}

/// The first of these, then as many more as fit, said together.
fn fitting(parts: &[String]) -> String {
    let mut said = String::new();
    for part in parts.iter().filter(|part| !part.is_empty()) {
        let longer = if said.is_empty() {
            part.clone()
        } else {
            format!("{said}{BETWEEN}{part}")
        };
        if !said.is_empty() && longer.chars().count() > ROOM {
            break;
        }
        said = longer;
    }
    said
}

/// Parts said together, those saying nothing left out.
fn dotted(parts: &[String]) -> String {
    parts
        .iter()
        .filter(|part| !part.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(BETWEEN)
}

/// How hard a network's protection is to break, by its standard as wifid
/// names it: WPA3 strong, WPA2 good, WEP broken, and anything else weak.
fn strength(standard: &str) -> &'static str {
    if standard.contains("WPA3") {
        "strong"
    } else if standard.contains("WPA2") {
        "good"
    } else if standard.contains("WEP") {
        "broken"
    } else {
        "weak"
    }
}

/// A width as wpa_supplicant says it, "20 MHz (no HT)" or "80+80 MHz": the
/// width's figure, and whether the radio's older than Wi-Fi 4, which its
/// generation says rather than "(no HT)" (VKCF3Z, 2026-11-09). What isn't
/// a width stays as it was said, with no figure.
fn width_of(said: &str) -> (Option<String>, bool, String) {
    let trimmed = said.trim();
    let Some((figure, after)) = trimmed.split_once("MHz") else {
        return (None, false, trimmed.to_owned());
    };
    let figure = figure.trim();
    let digits = |part: &str| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit());
    if !figure.split('+').all(digits) || figure.split('+').count() > 2 {
        return (None, false, trimmed.to_owned());
    }
    (
        Some(figure.to_owned()),
        after.contains("no HT"),
        String::new(),
    )
}

/// How a network's parts are said: its words, the account's numbers, and
/// what's read of it now.
struct Say<'a> {
    words: &'a Words,
    numbers: &'a Numbers,
    read: &'a Read<'a>,
}

impl Say<'_> {
    fn name(&self, id: &str) -> String {
        self.words.name(id, &[])
    }

    fn said(&self, id: &str, word: &str) -> String {
        self.words.said(id, word, &[])
    }

    /// A figure in its unit, from wifi:figures.
    fn figure(&self, unit: &str, value: impl Display) -> String {
        self.words
            .said("figures", unit, &[("n", &value.to_string())])
    }

    fn dbm(&self, value: impl Display) -> String {
        self.figure("dbm", value)
    }

    fn count(&self, value: impl Into<f64>) -> String {
        self.numbers.number(value.into(), 0)
    }

    fn tenths(&self, value: f64) -> String {
        self.numbers.number(value, 1)
    }

    /// A figure each way: once when both ways agree, "MCS 10 in ·
    /// MCS 9 out" when they don't, one way's when only it's known, and
    /// nothing when neither is.
    fn each_way<T: PartialEq + Copy>(
        &self,
        inward: Option<T>,
        outward: Option<T>,
        say: impl Fn(T) -> String,
    ) -> String {
        let way = |word: &str, value: T| self.words.said("figures", word, &[("said", &say(value))]);
        match (inward, outward) {
            (None, None) => String::new(),
            (None, Some(outward)) => way("out", outward),
            (Some(inward), None) => way("in", inward),
            (Some(inward), Some(outward)) if inward == outward => say(inward),
            (Some(inward), Some(outward)) => {
                format!("{}{BETWEEN}{}", way("in", inward), way("out", outward))
            }
        }
    }

    fn band(&self, band: Band) -> String {
        self.said("figures", band_word(band))
    }

    fn band_of(&self, mhz: u32) -> String {
        self.band(band_of(mhz))
    }

    /// A size in SI units at three significant digits: 12 B, 940 kB,
    /// 2.41 MB, as every size the launcher says reads.
    fn weighed(&self, bytes: u64) -> String {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a size said in three digits needs no more than a double holds"
        )]
        let bytes_said = bytes as f64;
        let Some(at) = SIZES.iter().position(|(size, _)| bytes_said >= *size) else {
            return format!("{bytes}{KEPT}B");
        };
        let (size, unit) = SIZES[at];
        let scaled: f64 = format!("{:.2e}", bytes_said / size)
            .parse()
            .unwrap_or(bytes_said / size);
        // 999.6 kB would round to 1000 kB: it's 1.00 MB instead.
        if scaled >= 1000.0 && at > 0 {
            let (_, larger) = SIZES[at - 1];
            return format!(
                "{}{KEPT}{larger}",
                self.numbers.significant(scaled / 1000.0)
            );
        }
        format!("{}{KEPT}{unit}", self.numbers.significant(scaled))
    }

    /// The rates each way, rounded Mb/s: "120 Mb/s in · 86 Mb/s out",
    /// once where they agree, nothing until the receiving one is read.
    fn speeds(&self, link: &Joined) -> String {
        match (link.receive.mbps, link.send.mbps) {
            (None, _) => String::new(),
            (Some(rx), Some(tx)) if tx != rx => format!(
                "{}{BETWEEN}{}",
                self.words
                    .said("figures", "in", &[("said", &self.figure("mbps", rx))]),
                self.words
                    .said("figures", "out", &[("said", &self.figure("mbps", tx))]),
            ),
            (Some(rx), _) => self.figure("mbps", rx),
        }
    }

    /// A width as ours: its figure in MHz, or as the driver said it.
    fn width(&self, said: Option<&str>) -> (String, bool) {
        let (figure, legacy, as_said) = width_of(said.unwrap_or(""));
        match figure {
            Some(figure) => (self.figure("mhz", figure), legacy),
            None => (as_said, legacy),
        }
    }

    fn seen(&self, card: &str) -> &[i32] {
        self.read.seen.get(card).map_or(&[][..], Vec::as_slice)
    }

    /// A card's link: its Signal, Speed and Traffic, each as the driver
    /// last read it.
    fn link_parts(&self, link: &Joined) -> Vec<Made> {
        let seen = self.seen(&link.card);
        let side = link
            .signal
            .map_or_else(|| self.said("figures", "unread"), |signal| self.dbm(signal));
        let note = link.margin.map_or_else(
            || self.said("signal", "shown"),
            |margin| {
                self.words
                    .said("drawer", "over", &[("margin", &margin.to_string())])
            },
        );
        let mut signal = holds("signal", &side, self.signal_parts(link))
            .named(self.name("signal"))
            .noted(note);
        if !seen.is_empty() {
            signal = signal.graphed(spark(seen));
        }
        let rates = self.speeds(link);
        let generation = link.receive.generation.or(link.send.generation);
        let note = fitting(&[
            generation.map_or_else(String::new, |each| self.figure("wifi", each)),
            self.width(link.width.as_deref()).0,
            link.expected.map_or_else(String::new, |expected| {
                self.words
                    .said("speed", "expected", &[("n", &expected.to_string())])
            }),
        ]);
        let side = if rates.is_empty() {
            self.said("figures", "unread")
        } else {
            rates
        };
        let speed = holds("speed", &side, self.speed_parts(link))
            .named(self.name("speed"))
            .noted(note);
        let mut parts = vec![signal, speed];
        let carried = self.each_way(link.receive.bytes, link.send.bytes, |bytes| {
            self.weighed(bytes)
        });
        if !carried.is_empty() {
            parts.push(
                holds("carried", &carried, self.carried_parts(link))
                    .named(self.name("carried"))
                    .noted(self.said("carried", "note")),
            );
        }
        parts
    }

    /// What the card's carried each way since it connected, and how long
    /// since a frame came or went.
    fn carried_parts(&self, link: &Joined) -> Vec<Made> {
        let mut parts = Vec::new();
        for (id, way) in [("received", &link.receive), ("sent", &link.send)] {
            let Some(bytes) = way.bytes else { continue };
            let note = way.packets.map_or_else(String::new, |packets| {
                let word = if packets == 1 { "packet" } else { "packets" };
                self.words
                    .said("carried", word, &[("count", &self.count(packets))])
            });
            parts.push(
                fact(id, &self.weighed(bytes))
                    .named(self.name(id))
                    .copying(bytes.to_string())
                    .noted(note),
            );
        }
        if let Some(idle) = link.idle {
            let mut part = fact("idle", "")
                .named(self.name("idle"))
                .copying(self.figure("ms", idle))
                .noted(self.said("idle", "note"));
            part.side = self.idle(idle);
            parts.push(part);
        }
        parts
    }

    /// How long since a frame came or went: "1 ms", "2.4 s", or as long
    /// as it's lasted, for a card idle a minute or more.
    fn idle(&self, milliseconds: u32) -> Vec<Value> {
        if milliseconds < SECOND {
            return said(&self.figure("ms", milliseconds));
        }
        if milliseconds < MINUTE {
            let seconds = f64::from(milliseconds) / f64::from(SECOND);
            let places = usize::from(seconds < 10.0);
            return said(&self.figure("s", self.numbers.number(seconds, places)));
        }
        vec![time::at(
            self.read.now - i64::from(milliseconds),
            Form::Lasted,
        )]
    }

    /// How long the card's been on the network, as the kernel says: how
    /// long beside it, since when below.
    fn connected(&self, card: &str) -> Option<Made> {
        let at = *self.read.look.since.get(card)?;
        let mut part = fact("connected", "")
            .named(self.name("connected"))
            .copying("");
        part.side = vec![time::at(at, Form::Lasted)];
        part.note = time::said(at, Form::Moment, &self.said("drawer", "since"));
        Some(part)
    }

    /// The signal a step in: at each antenna, on average, of the access
    /// point's beacons, how far over the noise, and what the card sends
    /// at.
    fn signal_parts(&self, link: &Joined) -> Vec<Made> {
        let station = link.station.clone().unwrap_or_default();
        let mut parts: Vec<Made> = station
            .chains
            .iter()
            .enumerate()
            .map(|(at, value)| {
                fact(format!("antenna-{at}"), &self.dbm(value))
                    .named(self.words.name("antenna", &[("n", &(at + 1).to_string())]))
                    .graphed(bar(level(f64::from(*value))))
            })
            .collect();
        if let Some(average) = link.average {
            parts.push(
                fact("average", &self.dbm(average))
                    .named(self.name("average"))
                    .graphed(bar(level(f64::from(average)))),
            );
        }
        if let Some(beacons) = station.beacons {
            parts.push(
                fact("beacons", &self.dbm(beacons))
                    .named(self.name("beacons"))
                    .graphed(bar(level(f64::from(beacons))))
                    .noted(self.said("beacons", "note")),
            );
        }
        // How often the beacons come, in the access point's time units of
        // 1.024 ms each, and one in how many says what it holds for a card
        // asleep, which is as long as the card may sleep.
        let beacon = station.interval;
        if let Some(beacon) = beacon {
            let ms = self.tenths(f64::from(beacon) * TIME_UNIT);
            parts.push(
                fact("interval", &self.figure("ms", ms))
                    .named(self.name("interval"))
                    .copying(self.figure("tu", beacon))
                    .noted(
                        self.words
                            .said("interval", "note", &[("n", &beacon.to_string())]),
                    ),
            );
        }
        if let Some(dtim) = station.dtim {
            let side = if dtim == 1 {
                self.said("dtim", "every")
            } else {
                self.words.said("dtim", "one", &[("n", &dtim.to_string())])
            };
            let note = beacon.map_or_else(
                || self.said("dtim", "asleep"),
                |beacon| {
                    let every = f64::from(beacon) * TIME_UNIT * f64::from(dtim);
                    self.words
                        .said("dtim", "note", &[("ms", &self.tenths(every))])
                },
            );
            parts.push(
                fact("dtim", &side)
                    .named(self.name("dtim"))
                    .copying(dtim.to_string())
                    .noted(note),
            );
        }
        if let Some(margin) = link.margin {
            let note = link.noise.map_or_else(String::new, |noise| {
                self.words
                    .said("margin", "noise", &[("n", &noise.to_string())])
            });
            parts.push(
                fact("margin", &self.figure("db", margin))
                    .named(self.name("margin"))
                    .noted(note),
            );
        }
        if let Some(power) = station.power {
            parts.push(
                fact("power", &self.dbm(self.numbers.number(power, 1)))
                    .named(self.name("power"))
                    .noted(self.said("power", "note")),
            );
        }
        parts
    }

    /// The speed a step in: each way, the Wi-Fi it's spoken in, its
    /// coding, width and channel, how frames are framed, and what had to
    /// be sent again.
    /// The channel a card's on and the band its frequency is in, with the
    /// frequency itself in its note.
    fn channel_part(&self, frequency: Option<u32>, channel: Option<u16>) -> Option<Made> {
        if frequency.is_none() && channel.is_none() {
            return None;
        }
        let side = [
            channel.map_or_else(String::new, |channel| channel.to_string()),
            frequency.map_or_else(String::new, |mhz| self.band_of(mhz)),
        ]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
        let note = frequency.map_or_else(String::new, |mhz| self.figure("mhz", self.count(mhz)));
        Some(
            fact("channel", &side)
                .named(self.name("channel"))
                .noted(note),
        )
    }

    /// How frames are framed and wait for the air: the preamble each
    /// opens with, the slot time between tries, and whether traffic is
    /// sorted by kind (WMM).
    fn framing_parts(&self, station: &Station) -> Vec<Made> {
        [
            (
                "preamble",
                station.short_preamble.map(|short| self.short(short)),
            ),
            ("slot", station.short_slot.map(|short| self.short(short))),
            ("wmm", station.wmm.map(|on| self.on(on))),
        ]
        .into_iter()
        .filter_map(|(id, side)| {
            let side = side?;
            Some(
                fact(id, &side)
                    .named(self.name(id))
                    .noted(self.said(id, "note")),
            )
        })
        .collect()
    }

    fn speed_parts(&self, link: &Joined) -> Vec<Made> {
        let station = link.station.clone().unwrap_or_default();
        let (receive, send) = (&link.receive, &link.send);
        let mut parts = Vec::new();
        for (id, rate) in [("in", receive.mbps), ("out", send.mbps)] {
            if let Some(rate) = rate {
                parts.push(fact(id, &self.figure("mbps", rate)).named(self.name(id)));
            }
        }
        if let Some(expected) = link.expected {
            parts.push(
                fact("expected", &self.figure("mbps", expected))
                    .named(self.name("expected"))
                    .noted(self.said("expected", "note")),
            );
        }
        if let Some(generation) = receive.generation.or(send.generation) {
            let side = self.each_way(receive.generation, send.generation, |each| {
                self.figure("wifi", each)
            });
            parts.push(
                fact("generation", &side)
                    .named(self.name("generation"))
                    .noted(self.said("generation", &generation.to_string())),
            );
        }
        // The coding, streams, guard interval and width, each said once
        // where both ways agree and each way where they don't.
        let mut each = |id: &str, side: String| {
            if !side.is_empty() {
                let note = if id == "width" {
                    String::new()
                } else {
                    self.said(id, "note")
                };
                parts.push(fact(id, &side).named(self.name(id)).noted(note));
            }
        };
        each(
            "coding",
            self.each_way(receive.mcs, send.mcs, |mcs| self.figure("mcs", mcs)),
        );
        each(
            "streams",
            self.each_way(receive.streams, send.streams, |streams: u32| {
                streams.to_string()
            }),
        );
        each(
            "guard",
            self.each_way(receive.guard, send.guard, |guard| {
                self.figure("us", self.tenths(guard))
            }),
        );
        each(
            "dcm",
            self.each_way(receive.dcm, send.dcm, |on| self.on(on)),
        );
        if receive.width.is_some() || send.width.is_some() {
            each(
                "width",
                self.each_way(receive.width, send.width, |width| self.figure("mhz", width)),
            );
        } else if let Some(width) = link.width.as_deref().filter(|width| !width.is_empty()) {
            each("width", width.to_owned());
        }
        parts.extend(self.channel_part(link.frequency, station.channel));
        parts.extend(self.framing_parts(&station));
        if let Some(retries) = station.retries {
            parts.push(
                holds(
                    "retries",
                    &self.retried(retries, station.failed),
                    self.retry_parts(&station),
                )
                .named(self.name("retries"))
                .noted(self.said("retries", "note")),
            );
        }
        parts
    }

    fn on(&self, on: bool) -> String {
        self.said("figures", if on { "on" } else { "off" })
    }

    fn short(&self, short: bool) -> String {
        self.said("figures", if short { "short" } else { "long" })
    }

    /// What had to be sent again or never was since the card joined:
    /// "none", or "12 sent again · 1 never sent".
    fn retried(&self, retries: u32, failed: Option<u32>) -> String {
        let never = failed.unwrap_or(0);
        if retries + never == 0 {
            return self.said("retries", "none");
        }
        self.words.said(
            "retries",
            "some",
            &[
                ("retries", &self.count(retries)),
                ("failed", &self.count(never)),
            ],
        )
    }

    /// What had to be sent again or never was, the beacons missed and
    /// what was dropped coming in, since the card joined.
    fn retry_parts(&self, station: &Station) -> Vec<Made> {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a count of frames said in whole digits needs no more than a double holds"
        )]
        let dropped = station.dropped.map(|dropped| dropped as f64);
        [
            ("retried", station.retries.map(f64::from)),
            ("failed", station.failed.map(f64::from)),
            ("missed", station.missed.map(f64::from)),
            ("dropped", dropped),
        ]
        .into_iter()
        .filter_map(|(id, count)| {
            Some(
                fact(id, &self.count(count?))
                    .named(self.name(id))
                    .noted(self.said(id, "note")),
            )
        })
        .collect()
    }

    /// How it's protected, said for what it means: who may join and how
    /// hard it is to break; a step in, the standard, how a card proves it
    /// may join, the encryption, and whether the frames that join and
    /// leave are protected.
    fn guard(&self) -> Made {
        let detail = self.read.detail;
        let Some(protection) = detail
            .points
            .first()
            .and_then(|point| point.protection.as_ref())
        else {
            return fact("protection", &self.said("protection", "none"))
                .named(self.name("protection"))
                .copying("")
                .noted(self.said("protection", "open"));
        };
        let account = protection.ways.iter().any(|way| {
            let way = way.to_lowercase();
            way.contains("eap") || way.contains("8021x")
        });
        let who = self.said("network", if account { "account" } else { "password" });
        let side = self.words.said(
            "protection",
            "guarded",
            &[
                ("who", &who),
                (
                    "strength",
                    &self.said("protection", strength(&protection.standard)),
                ),
            ],
        );
        let or = format!(" {} ", self.said("proof", "or"));
        let mut inside = vec![
            fact("standard", &protection.standard).named(self.name("standard")),
            fact("proof", &protection.ways.join(&or))
                .named(self.name("proof"))
                .noted(self.said("proof", "note")),
        ];
        if !protection.ciphers.is_empty() {
            inside.push(
                fact("cipher", &protection.ciphers.join(&or))
                    .named(self.name("cipher"))
                    .noted(self.said("cipher", "note")),
            );
        }
        if !protection.group.is_empty() {
            inside.push(
                fact("group", &protection.group)
                    .named(self.name("group"))
                    .noted(self.said("group", "note")),
            );
        }
        let mine = detail
            .joined
            .first()
            .and_then(|link| link.station.as_ref())
            .and_then(|station| station.protected);
        let note = match mine {
            Some(true) => self.said("frames", "mine"),
            Some(false) => self.said("frames", "notMine"),
            None => self.said("frames", "note"),
        };
        let frames = match protection.management {
            Management::Off => "off",
            Management::Optional => "optional",
            Management::Required => "required",
        };
        inside.push(
            fact("frames", &self.said("frames", frames))
                .named(self.name("frames"))
                .noted(note),
        );
        holds("protection", &side, inside)
            .named(self.name("protection"))
            .noted(protection.standard.clone())
    }

    /// The access points heard, by band: "2 on 2.4 · 1 on 5 GHz".
    fn heard_of(&self) -> String {
        let bands = &self.read.detail.bands;
        if bands.is_empty() {
            return String::new();
        }
        let each: Vec<String> = bands
            .iter()
            .map(|count| {
                self.words.said(
                    "heard",
                    "each",
                    &[
                        ("n", &count.points.to_string()),
                        ("band", &self.said("heard", band_word(count.band))),
                    ],
                )
            })
            .collect();
        self.words
            .said("heard", "bands", &[("bands", &each.join(BETWEEN))])
    }

    /// How many access points are heard, and in which bands; stepped
    /// into, each of them in the order first shown.
    fn heard(&self) -> Made {
        let detail = self.read.detail;
        let points = self
            .read
            .look
            .order
            .iter()
            .filter_map(|bssid| detail.points.iter().find(|point| &point.bssid == bssid))
            .map(|point| self.point(point))
            .collect();
        let mut heard = holds("heard", &self.heard_of(), points).named(self.name("heard"));
        "accessPoint".clone_into(&mut heard.glyph);
        heard
    }

    fn fastest(&self, point: &Point) -> bool {
        let detail = self.read.detail;
        detail.points.len() > 1
            && detail
                .best
                .as_ref()
                .is_some_and(|best| best.bssid == point.bssid)
    }

    /// One access point, named by its band and channel: its signal, as a
    /// bar too, and about how fast a card would get on with it, and
    /// whether it's the one joined or the fastest; in the note, for the
    /// fastest, why. A step in, its parts.
    fn point(&self, point: &Point) -> Made {
        let fastest = self.fastest(point);
        let mut side = self.words.said(
            "point",
            "side",
            &[
                ("signal", &point.signal.to_string()),
                ("estimate", &point.estimate.to_string()),
            ],
        );
        if point.joined {
            side = format!("{side}{BETWEEN}{}", self.said("point", "connected"));
        } else if fastest {
            side = format!("{side}{BETWEEN}{}", self.said("point", "fastest"));
        }
        let note = if fastest {
            self.read
                .detail
                .best
                .as_ref()
                .map(|best| fitting(&best.reasons))
                .unwrap_or_default()
        } else {
            String::new()
        };
        let name = self.words.name(
            "point",
            &[
                ("band", &self.band(point.band)),
                ("channel", &point.channel.to_string()),
            ],
        );
        let glyph = format!(
            "{}{}",
            if point.protection.is_some() {
                "lockedSignal"
            } else {
                "signal"
            },
            point.bars.min(4)
        );
        Made {
            id: format!("point-{}", point.bssid),
            kind: "point",
            name,
            glyph,
            side: said(&side),
            graph: Some(bar(level(f64::from(point.signal)))),
            inside: self.point_parts(point, fastest),
            ..Made::default()
        }
        .noted(note)
    }

    /// An access point's parts: its signal, about how fast a card would
    /// get on with it, its width and Wi-Fi, how busy it is and how many
    /// are on it, when it was last heard, why it's the fastest, and its
    /// hardware address.
    fn point_parts(&self, point: &Point, fastest: bool) -> Vec<Made> {
        let streams = if point.streams == 1 {
            "stream"
        } else {
            "streams"
        };
        let generation = point.generation.map_or_else(
            || self.said("generation", "older"),
            |generation| self.figure("wifi", generation),
        );
        let mut parts = vec![
            fact("signal", &self.dbm(point.signal))
                .named(self.name("signal"))
                .graphed(bar(level(f64::from(point.signal))))
                .noted(
                    self.words
                        .said("network", "bars", &[("n", &point.bars.to_string())]),
                ),
            fact(
                "estimate",
                &self
                    .words
                    .said("estimate", "side", &[("n", &point.estimate.to_string())]),
            )
            .named(self.name("estimate"))
            .noted(self.said("estimate", "note")),
            fact("width", &self.figure("mhz", point.width)).named(self.name("width")),
            fact("generation", &generation)
                .named(self.name("generation"))
                .noted(self.words.said(
                    "generation",
                    streams,
                    &[("n", &point.streams.to_string())],
                )),
        ];
        if let Some(load) = &point.load {
            parts.push(
                fact("busy", &self.figure("percent", load.busy))
                    .named(self.name("busy"))
                    .graphed(bar(f64::from(load.busy) / 100.0))
                    .noted(self.said("busy", "note")),
            );
            parts.push(
                fact("stations", &self.count(load.stations))
                    .named(self.name("stations"))
                    .noted(self.said("stations", "note")),
            );
        }
        let ago = if point.age < JUST_NOW {
            self.said("heard-ago", "now")
        } else {
            self.words
                .said("heard-ago", "ago", &[("n", &point.age.to_string())])
        };
        parts.push(
            fact("heard-ago", &ago)
                .named(self.name("heard-ago"))
                .copying(""),
        );
        let reasons = self
            .read
            .detail
            .best
            .as_ref()
            .map(|best| best.reasons.join(", "))
            .unwrap_or_default();
        if fastest && !reasons.is_empty() {
            parts.push(fact("fastest", &reasons).named(self.name("fastest")));
        }
        parts.push(fact("bssid", &point.bssid).named(self.name("bssid")));
        parts
    }

    /// The network connected's drawer: the band it's on and what it is
    /// above its name; what moves first, its signal, speed, traffic and
    /// retries; then how the link is made; then the access points heard
    /// and how it's protected; how long it's been connected; and the
    /// access point's hardware address last, an identifier (VKCF3Z,
    /// 2026-11-07: what moves most first, identifiers last). A fact it
    /// doesn't read is left out, never said as unknown.
    /// How a card's link is sent each way, as the drawer says it: the
    /// generation (or that it's older than Wi-Fi 4), the coding, the
    /// streams and the width.
    fn radio_said(&self, link: &Joined) -> [(&'static str, String); 4] {
        let (width, legacy) = self.width(link.width.as_deref());
        let ways = |pick: fn(&Way) -> Option<u32>| (pick(&link.receive), pick(&link.send));
        let (generation_in, generation_out) = (link.receive.generation, link.send.generation);
        let mut generation = self.each_way(generation_in, generation_out, |each| {
            self.figure("wifi", each)
        });
        if generation.is_empty() && legacy {
            generation = self.said("drawer", "legacy");
        }
        let (mcs_in, mcs_out) = ways(|way| way.mcs);
        let (streams_in, streams_out) = ways(|way| way.streams);
        let width = if link.receive.width.is_some() || link.send.width.is_some() {
            self.each_way(link.receive.width, link.send.width, |each| {
                self.figure("mhz", each)
            })
        } else {
            width
        };
        [
            ("generation", generation),
            (
                "coding",
                self.each_way(mcs_in, mcs_out, |mcs| self.figure("mcs", mcs)),
            ),
            (
                "streams",
                self.each_way(streams_in, streams_out, |streams: u32| streams.to_string()),
            ),
            ("width", width),
        ]
    }

    fn drawer(&self) -> Option<Value> {
        let detail = self.read.detail;
        let link = detail.joined.first()?;
        let station = link.station.clone().unwrap_or_default();
        let point = detail.points.iter().find(|point| point.joined);
        let frequency = link.frequency;
        let channel = station.channel.or(point.map(|point| point.channel));
        let protection = detail
            .points
            .first()
            .and_then(|point| point.protection.as_ref());
        let margin = link.margin.map_or_else(String::new, |margin| {
            self.words
                .said("drawer", "over", &[("margin", &margin.to_string())])
        });
        let signal = link.signal.map_or_else(String::new, |signal| {
            self.dbm(self.numbers.whole(f64::from(signal)))
        });
        let said = |id: &str, said: String| json!({ "name": self.name(id), "said": said });
        let mut facts = vec![
            said("signal", dotted(&[signal, margin])),
            said("speed", self.speeds(link)),
            said(
                "expected",
                link.expected
                    .map_or_else(String::new, |expected| self.figure("mbps", expected)),
            ),
            said(
                "carried",
                self.each_way(link.receive.bytes, link.send.bytes, |bytes| {
                    self.weighed(bytes)
                }),
            ),
            said(
                "retries",
                station
                    .retries
                    .map_or_else(String::new, |retries| self.retried(retries, station.failed)),
            ),
        ];
        facts.extend(
            self.radio_said(link)
                .into_iter()
                .map(|(id, figure)| said(id, figure)),
        );
        facts.extend([
            said(
                "channel",
                dotted(&[
                    channel.map_or_else(String::new, |channel| channel.to_string()),
                    frequency.map_or_else(String::new, |mhz| self.figure("mhz", self.count(mhz))),
                ]),
            ),
            said("heard", self.heard_of()),
            said(
                "protection",
                protection.map_or_else(
                    || self.said("protection", "none"),
                    |protection| self.said("protection", strength(&protection.standard)),
                ),
            ),
            said(
                "standard",
                protection
                    .map(|protection| protection.standard.clone())
                    .unwrap_or_default(),
            ),
        ]);
        facts.retain(|fact| {
            fact["said"]
                .as_str()
                .is_some_and(|said| !said.trim().is_empty())
        });
        // How long it's been connected, and since when, as times the
        // shell words, so the drawer isn't told again each minute.
        if let Some(at) = self.read.look.since.get(&link.card) {
            facts.push(json!({
                "name": self.name("connected"),
                "said": time::at(*at, Form::Lasted),
                "qualifier": time::said(*at, Form::Moment, &self.said("drawer", "since")),
            }));
        }
        let bssid = link
            .bssid
            .clone()
            .or_else(|| point.map(|point| point.bssid.clone()))
            .unwrap_or_default();
        if !bssid.is_empty() {
            facts.push(json!({ "name": self.name("bssid"), "said": bssid, "identifies": true }));
        }
        let band = frequency
            .map(|mhz| self.band_of(mhz))
            .or_else(|| point.map(|point| self.band(point.band)));
        let eyebrow = band.map_or_else(
            || self.said("drawer", "kind"),
            |band| self.words.said("drawer", "eyebrow", &[("band", &band)]),
        );
        Some(json!({ "eyebrow": eyebrow, "title": detail.name, "facts": facts }))
    }
}

fn band_word(band: Band) -> &'static str {
    match band {
        Band::Ghz2 => "ghz2",
        Band::Ghz5 => "ghz5",
        Band::Ghz6 => "ghz6",
    }
}

fn band_of(mhz: u32) -> Band {
    if mhz >= 5925 {
        Band::Ghz6
    } else if mhz >= 5000 {
        Band::Ghz5
    } else {
        Band::Ghz2
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::detail::{Best, Count};
    use crate::radio::{Load, Protection};

    /// The words as the launcher says them, where a case reads them, and
    /// every other word its own id, so a case shows which was used.
    const SAID: &[(&str, &str, &str)] = &[
        ("figures", "dbm", "{n} dBm"),
        ("figures", "db", "{n} dB"),
        ("figures", "mbps", "{n} Mb/s"),
        ("figures", "mhz", "{n} MHz"),
        ("figures", "ms", "{n} ms"),
        ("figures", "s", "{n} s"),
        ("figures", "wifi", "Wi-Fi {n}"),
        ("figures", "mcs", "MCS {n}"),
        ("figures", "in", "{said} in"),
        ("figures", "out", "{said} out"),
        ("figures", "ghz2", "2.4 GHz"),
        ("figures", "ghz5", "5 GHz"),
        ("network", "password", "a password"),
        ("protection", "guarded", "{who}, {strength}"),
        ("protection", "good", "good"),
        ("heard", "each", "{n} on {band}"),
        ("heard", "bands", "{bands} GHz"),
        ("heard", "ghz2", "2.4"),
        ("heard", "ghz5", "5"),
        ("point", "side", "{signal} dBm \u{b7} ~{estimate} Mb/s"),
        ("point", "connected", "connected"),
        ("card", "", "Card {n}"),
        ("antenna", "", "Antenna {n}"),
        ("point", "", "{band}, Channel {channel}"),
        ("drawer", "eyebrow", "on {band} \u{b7} network"),
        ("drawer", "over", "{margin} dB over the noise"),
        ("drawer", "since", "since {time}"),
        (
            "retries",
            "some",
            "{retries} sent again \u{b7} {failed} never sent",
        ),
    ];

    /// Wi-Fi's things holding every word the parts need, but the
    /// network's, which the list's own words hold too.
    pub(crate) fn things() -> Vec<Value> {
        let mut things: BTreeMap<&str, Value> = BTreeMap::new();
        for (id, word) in NEEDED {
            let said = SAID
                .iter()
                .find(|(is, at, _)| is == id && at == word)
                .map_or_else(
                    || {
                        if word.is_empty() {
                            (*id).to_owned()
                        } else {
                            format!("{id}.{word}")
                        }
                    },
                    |(_, _, said)| (*said).to_owned(),
                );
            let thing = things
                .entry(*id)
                .or_insert_with(|| json!({ "id": format!("wifi:{id}") }));
            if word.is_empty() {
                thing["name"] = json!(said);
            } else {
                thing["said"][word] = json!(said);
            }
        }
        things.into_values().collect()
    }

    fn words() -> Words {
        Words::read(&json!({ "things": things() })).expect("every word is there")
    }

    fn point(bssid: &str, signal: i16, estimate: u32, joined: bool) -> Point {
        Point {
            bssid: bssid.into(),
            signal,
            bars: 3,
            band: Band::Ghz5,
            channel: 36,
            frequency: 5180,
            width: 80,
            generation: Some(6),
            streams: 2,
            load: Some(Load {
                stations: 4,
                busy: 30,
            }),
            protection: Some(Protection {
                standard: "WPA2".into(),
                ways: vec!["wpa-psk".into()],
                ciphers: vec!["CCMP".into()],
                group: "CCMP".into(),
                management: Management::Optional,
            }),
            age: 2,
            estimate,
            joined,
        }
    }

    fn joined(card: &str) -> Joined {
        Joined {
            card: card.into(),
            bssid: Some("aa:aa".into()),
            signal: Some(-52),
            average: Some(-54),
            noise: Some(-95),
            margin: Some(43),
            receive: Way {
                mbps: Some(866),
                generation: Some(6),
                mcs: Some(9),
                bytes: Some(2_410_000),
                packets: Some(1),
                ..Way::default()
            },
            send: Way {
                mbps: Some(780),
                generation: Some(6),
                mcs: Some(8),
                bytes: Some(940_000),
                ..Way::default()
            },
            expected: Some(600),
            idle: Some(2_400),
            width: Some("80 MHz".into()),
            frequency: Some(5180),
            station: Some(Station {
                chains: vec![-50, -55],
                retries: Some(12),
                failed: Some(1),
                connected: Some(600),
                ..Station::default()
            }),
        }
    }

    /// Lantern, heard at two access points in 5 GHz, wlan0 on the first.
    fn lantern() -> Detail {
        Detail {
            name: "Lantern".into(),
            points: vec![
                point("aa:aa", -52, 400, true),
                point("bb:bb", -60, 500, false),
            ],
            bands: vec![Count {
                band: Band::Ghz5,
                points: 2,
            }],
            joined: vec![joined("wlan0")],
            best: Some(Best {
                bssid: "bb:bb".into(),
                estimate: 500,
                reasons: vec!["the fastest of 2".into()],
            }),
        }
    }

    const NOW: i64 = 1_762_790_400_000;

    fn insides_of(detail: &Detail, connected: bool, seen: &BTreeMap<String, Vec<i32>>) -> Insides {
        let mut look = Look::default();
        look.follow(detail, NOW);
        let read = Read {
            detail,
            connected,
            look: &look,
            seen,
            now: NOW,
        };
        insides(&read, &words(), &Numbers::default())
    }

    fn row<'a>(insides: &'a Insides, key: &str) -> &'a Value {
        insides
            .rows
            .iter()
            .find(|row| row["key"] == key)
            .unwrap_or_else(|| panic!("no row {key}"))
    }

    const LANTERN: &str = "row:wifi:network:Lantern";

    #[test]
    fn a_network_joined_holds_its_cards_link_its_guard_and_its_access_points() {
        let insides = insides_of(&lantern(), true, &BTreeMap::new());
        let under: Vec<&str> = insides
            .rows
            .iter()
            .filter(|row| row["parent"] == LANTERN)
            .map(|row| row["key"].as_str().unwrap_or_default())
            .collect();
        let key = |id: &str| format!("{LANTERN}:{id}");
        assert_eq!(
            under,
            [
                "signal",
                "speed",
                "carried",
                "protection",
                "heard",
                "connected"
            ]
            .map(key),
            "the link's parts, how it's protected and what's heard, then how long"
        );
        let signal = row(&insides, &key("signal"));
        assert_eq!(signal["depth"], 1, "a step in is one deep");
        assert_eq!(signal["thing"], "wifi:network:Lantern:signal");
        assert_eq!(signal["side"], json!([{ "said": "-52 dBm" }]));
        assert_eq!(signal["note"], "43 dB over the noise");
        assert_eq!(
            signal["acts"],
            json!([
                { "key": "verb:copy", "copy": "-52 dBm" },
                { "key": "verb:more", "step": "rows" },
            ]),
            "a part holding more copies its figure and steps in"
        );
        let antenna = row(&insides, &key("signal:antenna-1"));
        assert_eq!(antenna["depth"], 2, "a part's own parts are a step further");
        assert_eq!(antenna["name"], "Antenna 2");
        assert_eq!(
            antenna["graph"]["values"],
            json!([0.6]),
            "-55 dBm lights 12 of 20"
        );
        let speed = row(&insides, &key("speed"));
        assert_eq!(speed["side"][0]["said"], "866 Mb/s in \u{b7} 780 Mb/s out");
        assert_eq!(speed["note"], "Wi-Fi 6 \u{b7} 80 MHz \u{b7} speed.expected");
        assert_eq!(
            row(&insides, &key("speed:coding"))["side"][0]["said"],
            "MCS 9 in \u{b7} MCS 8 out"
        );
        assert_eq!(
            row(&insides, &key("speed:retries"))["side"][0]["said"],
            "12 sent again \u{b7} 1 never sent"
        );
        let carried = row(&insides, &key("carried"));
        assert_eq!(
            carried["side"][0]["said"],
            "2.41\u{a0}MB in \u{b7} 940\u{a0}kB out"
        );
        assert_eq!(
            row(&insides, &key("carried:idle"))["side"][0]["said"],
            "2.4 s"
        );
        let connected = row(&insides, &key("connected"));
        let joined_at = NOW - 600_000;
        assert_eq!(
            connected["side"],
            json!([{ "at": joined_at, "as": "lasted" }]),
            "how long it's been on is a time the shell words"
        );
        assert_eq!(
            connected["note"],
            json!({ "at": joined_at, "as": "moment", "said": "since {time}" })
        );
    }

    #[test]
    fn access_points_keep_the_order_first_shown_the_fastest_new_first() {
        let mut detail = lantern();
        let mut look = Look::default();
        look.follow(&detail, NOW);
        assert_eq!(look.order, ["bb:bb", "aa:aa"], "the fastest first");
        detail.points[0].estimate = 900;
        detail.points.push(point("cc:cc", -40, 1_000, false));
        look.follow(&detail, NOW);
        assert_eq!(
            look.order,
            ["bb:bb", "aa:aa", "cc:cc"],
            "a row stays where it is as its readings change, one heard later at the end"
        );
        detail.points.remove(1);
        look.follow(&detail, NOW);
        assert_eq!(look.order, ["aa:aa", "cc:cc"], "one no longer heard goes");
    }

    #[test]
    fn a_cards_joining_holds_still_through_a_seconds_wander() {
        let mut detail = lantern();
        let mut look = Look::default();
        look.follow(&detail, NOW);
        let first = look.since["wlan0"];
        look.follow(&detail, NOW + 2_900);
        assert_eq!(
            look.since["wlan0"], first,
            "a poll later reads the same joining"
        );
        if let Some(station) = detail.joined[0].station.as_mut() {
            station.connected = Some(1);
        }
        look.follow(&detail, NOW + 2_900);
        assert_eq!(
            look.since["wlan0"],
            NOW + 1_900,
            "a card joined again is joined since then"
        );
    }

    #[test]
    fn the_access_points_are_said_by_band_and_each_by_its_channel() {
        let insides = insides_of(&lantern(), true, &BTreeMap::new());
        let heard = row(&insides, &format!("{LANTERN}:heard"));
        assert_eq!(heard["side"][0]["said"], "2 on 5 GHz");
        assert_eq!(heard["glyph"], "accessPoint");
        let fastest = row(&insides, &format!("{LANTERN}:heard:point-bb:bb"));
        assert_eq!(fastest["name"], "5 GHz, Channel 36");
        assert_eq!(
            fastest["side"][0]["said"],
            "-60 dBm \u{b7} ~500 Mb/s \u{b7} point.fastest"
        );
        assert_eq!(fastest["note"], "the fastest of 2");
        assert_eq!(fastest["glyph"], "lockedSignal3");
        let on = row(&insides, &format!("{LANTERN}:heard:point-aa:aa"));
        assert_eq!(
            on["side"][0]["said"],
            "-52 dBm \u{b7} ~400 Mb/s \u{b7} connected"
        );
        assert_eq!(
            row(&insides, &format!("{LANTERN}:heard:point-bb:bb:bssid"))["side"][0]["said"],
            "bb:bb"
        );
    }

    #[test]
    fn a_network_joined_has_its_drawer_moving_figures_first_its_address_last() {
        let insides = insides_of(&lantern(), true, &BTreeMap::new());
        let drawer = insides.drawer.expect("a drawer while a card's on it");
        assert_eq!(drawer["eyebrow"], "on 5 GHz \u{b7} network");
        assert_eq!(drawer["title"], "Lantern");
        let facts = drawer["facts"].as_array().expect("facts");
        let named: Vec<&str> = facts
            .iter()
            .map(|fact| fact["name"].as_str().unwrap_or_default())
            .collect();
        assert_eq!(
            named,
            [
                "signal",
                "speed",
                "expected",
                "carried",
                "retries",
                "generation",
                "coding",
                "width",
                "channel",
                "heard",
                "protection",
                "standard",
                "connected",
                "bssid",
            ],
            "each fact read, none it doesn't (streams)"
        );
        assert_eq!(
            facts[0]["said"],
            "\u{2212}52 dBm \u{b7} 43 dB over the noise"
        );
        assert_eq!(facts[8]["said"], "36 \u{b7} 5,180 MHz");
        assert_eq!(
            facts[12],
            json!({
                "name": "connected",
                "said": { "at": NOW - 600_000, "as": "lasted" },
                "qualifier": { "at": NOW - 600_000, "as": "moment", "said": "since {time}" },
            })
        );
        assert_eq!(facts[13]["identifies"], true, "its address identifies it");
    }

    #[test]
    fn a_network_only_heard_has_its_guard_and_access_points_and_no_drawer() {
        let mut detail = lantern();
        detail.joined.clear();
        for point in &mut detail.points {
            point.joined = false;
        }
        let insides = insides_of(&detail, false, &BTreeMap::new());
        assert_eq!(insides.drawer, None);
        let under: Vec<&str> = insides
            .rows
            .iter()
            .filter(|row| row["parent"] == LANTERN)
            .map(|row| row["key"].as_str().unwrap_or_default())
            .collect();
        assert_eq!(
            under,
            ["protection", "heard"].map(|id| format!("{LANTERN}:{id}"))
        );
        assert_eq!(
            row(&insides, &format!("{LANTERN}:protection"))["side"][0]["said"],
            "a password, good"
        );
    }

    #[test]
    fn a_network_joined_waits_for_its_cards_link() {
        let mut detail = lantern();
        detail.joined.clear();
        assert_eq!(
            insides_of(&detail, true, &BTreeMap::new()),
            Insides::default()
        );
    }

    #[test]
    fn several_cards_each_hold_their_own_link() {
        let mut detail = lantern();
        detail.joined.push(joined("wlan1"));
        let insides = insides_of(&detail, true, &BTreeMap::new());
        let card = row(&insides, &format!("{LANTERN}:card-wlan1"));
        assert_eq!(card["name"], "Card 2");
        assert_eq!(card["side"][0]["said"], "-52 dBm");
        assert!(
            insides
                .rows
                .iter()
                .any(|row| row["key"] == format!("{LANTERN}:card-wlan1:connected")),
            "each card says how long it's been on"
        );
    }

    #[test]
    fn the_signal_read_while_looked_at_is_its_spark() {
        let seen = BTreeMap::from([("wlan0".to_owned(), vec![-90, -60, -30])]);
        let insides = insides_of(&lantern(), true, &seen);
        let signal = row(&insides, &format!("{LANTERN}:signal"));
        assert_eq!(
            signal["graph"],
            json!({ "shape": "spark", "values": [0.0, 0.5, 1.0], "warn": false })
        );
    }

    #[test]
    fn a_size_is_three_figures_in_its_unit() {
        let say = |bytes| {
            let detail = lantern();
            let look = Look::default();
            let seen = BTreeMap::new();
            let read = Read {
                detail: &detail,
                connected: true,
                look: &look,
                seen: &seen,
                now: NOW,
            };
            let words = words();
            let numbers = Numbers::default();
            Say {
                words: &words,
                numbers: &numbers,
                read: &read,
            }
            .weighed(bytes)
        };
        assert_eq!(
            [12, 940_000, 2_414_000, 999_600, 19_300_000_000].map(say),
            [
                "12\u{a0}B",
                "940\u{a0}kB",
                "2.41\u{a0}MB",
                "1.00\u{a0}MB",
                "19.3\u{a0}GB"
            ]
        );
    }

    #[test]
    fn a_width_is_ours_and_no_ht_is_older_than_wifi_4() {
        assert_eq!(
            width_of("80 MHz"),
            (Some("80".into()), false, String::new())
        );
        assert_eq!(
            width_of("20 MHz (no HT)"),
            (Some("20".into()), true, String::new())
        );
        assert_eq!(
            width_of("80+80 MHz"),
            (Some("80+80".into()), false, String::new())
        );
        assert_eq!(width_of("wide"), (None, false, "wide".into()));
    }

    #[test]
    fn a_note_says_what_fits() {
        let parts = [
            "Wi-Fi 6",
            "80 MHz",
            "600 Mb/s expected",
            "and a good deal more",
        ]
        .map(str::to_owned);
        assert_eq!(
            fitting(&parts),
            "Wi-Fi 6 \u{b7} 80 MHz \u{b7} 600 Mb/s expected"
        );
    }

    #[test]
    fn a_vocabulary_without_a_word_says_which() {
        let mut things = things();
        for thing in &mut things {
            if thing["id"] == "wifi:dtim" {
                thing["said"] = json!({});
            }
        }
        assert_eq!(
            Words::read(&json!({ "things": things })),
            Err("the vocabulary's wifi:dtim has no every".into())
        );
    }

    /// The launcher's own words hold every word a network's insides are
    /// said in, so one renamed there fails here rather than blanking a
    /// part (run with them by wifid-reads-the-launchers-words).
    #[test]
    fn the_launchers_vocabulary_has_every_parts_words() {
        let Ok(path) = std::env::var("WIFID_TEST_WORDS") else {
            return;
        };
        let text = std::fs::read_to_string(path).expect("the vocabulary reads");
        let launchers: Value = serde_json::from_str(&text).expect("the vocabulary is JSON");
        let words = Words::read(&launchers).expect("every word is there");
        let detail = lantern();
        let mut look = Look::default();
        look.follow(&detail, NOW);
        let seen = BTreeMap::new();
        let read = Read {
            detail: &detail,
            connected: true,
            look: &look,
            seen: &seen,
            now: NOW,
        };
        let insides = insides(&read, &words, &Numbers::default());
        assert_eq!(
            row(&insides, &format!("{LANTERN}:heard"))["side"][0]["said"],
            "2 on 5 GHz"
        );
        assert_eq!(
            insides.drawer.expect("a drawer")["eyebrow"],
            "on 5 GHz \u{b7} network"
        );
    }
}
