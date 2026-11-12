//! Heat's rows, ready for the launcher, in Y2YBTS's ready-row form
//! (QEST-20261023-Y2YBTS attachments/speed/ready-rows.md): each part by
//! its name first, then what it reads (the Processor, how often it's
//! Throttled, its NPU, Image Processor, Media Engine and Display Engine,
//! the Disk, the Memory, the Wi-Fi card, the Battery, the Air inside and
//! each fan), the rows a step inside each, and Heat's headline. Each
//! temperature is drawn against the most its chip says it may reach,
//! beside what's left to its limit, with its Last Reads a step in; a fan
//! against what it's rated for. They're what the launcher's `Heat.qml` made
//! of the same records, moved here so the shell words, sorts and builds
//! nothing (Joel, 2026-11-10: "the daemons need to be the data layer").
//!
//! A row is `row:heat:<id>`; one inside another names it as its `parent`
//! and says its `depth`, and each comes in the order it's shown. Its
//! graph is worked out whole and sent in the steps the shell draws, so a
//! reading that leaves the drawing as it was changes nothing and the row
//! isn't sent again. The Last Reads are kept only between reads made
//! while Heat is watched ([`Heat::forget`] drops them), so a shut launcher
//! costs nothing; a new watch shows what's kept so far, one read at first.
//!
//! Long because every part's row, its drawer and the rows inside it change
//! together as Heat's one design does, and its cases, a quarter of it, stay
//! beside the private pieces they prove.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::parts::{self, Fan, Files, Gathered, Left, Part, Throttled, Trip, Zone};
pub use crate::rows::Ready;
use crate::rows::{GRAPHED, Made, bar, figure, item, spark, strip};
use crate::words::Words;

/// The place the launcher shows these rows in.
const PLACE: &str = "heat";
/// Reads nearer than this to the last a spark took are left out of it.
const SAMPLED_MS: u64 = 1000;
/// Reads further apart than this start a spark afresh.
const STALE_MS: u64 = 5000;
/// The firmware's points that act on heat, in the order it acts.
const TRIPS: [&str; 4] = ["passive", "active", "hot", "critical"];

/// What Heat's rows are made from: the daemon's sensor, zone, cooling and
/// processor records, and the processor's telemetry's temperatures.
#[derive(Clone, Copy, Debug, Default)]
pub struct Held<'a> {
    pub sensors: &'a [Value],
    pub zones: &'a [Value],
    pub cooling: &'a [Value],
    pub processor: Option<&'a Value>,
    pub telemetry: &'a [Value],
}

/// The reads kept between one and the next while Heat is watched: each
/// spark's, by its part, and the read before's time and throttling.
#[derive(Clone, Debug, Default)]
pub struct Heat {
    series: BTreeMap<String, Vec<f64>>,
    before: Option<(u64, Option<u64>)>,
}

/// What a read says, as the rows are made from it.
struct Read {
    parts: Gathered,
    throttled: Option<Throttled>,
    zones: Vec<Zone>,
    cooling: Vec<parts::Cooling>,
}

impl Heat {
    /// Drops every kept read, as nothing watches Heat any more.
    pub fn forget(&mut self) {
        self.series.clear();
        self.before = None;
    }

    /// Heat's rows and headline from what's held, read `now` (in ms), its
    /// sparks taking this read.
    pub fn read(&mut self, held: Held, now: u64, words: &Words) -> Ready {
        let read = gathered(held);
        self.push(&read, now);
        let say = Say {
            words,
            series: &self.series,
        };
        let temps = &read.parts.temps;
        let fans = &read.parts.fans;
        if temps.is_empty() && fans.is_empty() {
            return Ready::default();
        }
        let mut made = Vec::new();
        for part in temps {
            made.push(say.temp_row(part, &read));
            if part.id == "processor"
                && let Some(throttled) = &read.throttled
            {
                made.push(say.throttled_row(throttled));
            }
        }
        if let Some(throttled) = read
            .throttled
            .as_ref()
            .filter(|_| !temps.iter().any(|p| p.id == "processor"))
        {
            made.push(say.throttled_row(throttled));
        }
        made.extend(fans.iter().map(|fan| say.fan_row(fan)));
        let mut rows = Vec::new();
        crate::rows::placed(PLACE, made, None, 0, &mut rows);
        Ready {
            rows,
            head: Some(say.head(temps, fans)),
        }
    }

    /// Adds this read to every spark, starting them afresh after a gap:
    /// each temperature and fan as it reads, and the throttling as how
    /// many times a second it slowed since the read before.
    fn push(&mut self, read: &Read, now: u64) {
        // Read again sooner, as a read whose rates aren't of now is, the
        // sparks keep their one read a second, as the shell's did.
        if self
            .before
            .is_some_and(|(at, _)| now.saturating_sub(at) < SAMPLED_MS)
        {
            return;
        }
        let times = read.throttled.as_ref().map(|said| said.times);
        let before = self
            .before
            .filter(|(at, _)| now.saturating_sub(*at) <= STALE_MS);
        let kept = std::mem::take(&mut self.series);
        let kept = if before.is_some() {
            kept
        } else {
            BTreeMap::new()
        };
        let mut added = |name: String, value: f64| {
            let mut values = kept.get(&name).cloned().unwrap_or_default();
            values.push(value);
            let from = values.len().saturating_sub(GRAPHED);
            self.series.insert(name, values.split_off(from));
        };
        for part in &read.parts.temps {
            added(temp_key(part), part.value);
        }
        for fan in &read.parts.fans {
            added(fan_key(fan), fan.value);
        }
        if let (Some(now_times), Some((at, Some(then)))) = (times, before) {
            #[expect(
                clippy::cast_precision_loss,
                reason = "a gap of seconds is far below 2^52 ms"
            )]
            let seconds = now.saturating_sub(at).max(1) as f64 / 1000.0;
            #[expect(
                clippy::cast_precision_loss,
                reason = "a second's throttling is far below 2^52"
            )]
            let rate = now_times.saturating_sub(then) as f64 / seconds;
            added("throttled".to_owned(), rate);
        }
        self.before = Some((now, times));
    }
}

fn gathered(held: Held) -> Read {
    let chips = parts::chips_of(held.sensors);
    let zones = parts::zones_of(held.zones);
    let unread: Vec<Zone> = zones
        .iter()
        .filter(|zone| !parts::gives(&chips, &zone.part))
        .cloned()
        .collect();
    let cpus = held.processor.map(parts::cpus_of).unwrap_or_default();
    Read {
        parts: parts::gather(&chips, &unread, &parts::core_cpus(&cpus), held.telemetry),
        throttled: held.processor.and_then(parts::throttled_of),
        zones,
        cooling: parts::cooling_of(held.cooling),
    }
}

fn temp_key(part: &Part) -> String {
    format!("{}:{}", part.id, part.nth)
}

fn fan_key(fan: &Fan) -> String {
    format!("fan:{}:{}", fan.id, fan.n)
}

/// A temperature's bar, against its most, warning near it.
fn level(value: f64, part: &Part) -> Value {
    let said = parts::level(value, part.high, part.crit, &part.id);
    bar(said.value, said.warn)
}

/// Whether a file is what a reading says now: hwmon's `_input`, a zone's
/// `temp`, or the processor's telemetry's part and degrees.
fn read_now(name: &str) -> bool {
    name.ends_with("_input") || matches!(name, "temp" | "part" | "celsius")
}

/// Whether a file is a fan's power, `pwm<n>` alone.
fn pwm(name: &str) -> bool {
    name.starts_with("pwm") && !name.contains('_')
}

/// Where the kernel counts a logical processor's slowing to cool down.
fn throttle_folder(cpu: u64) -> String {
    format!("/sys/devices/system/cpu/cpu{cpu}/thermal_throttle")
}

/// Counts as the kernel keeps them, by its names, each said as a number.
fn counted(folder: String, kept: &[(&str, Option<u64>)]) -> Files {
    Files {
        folder,
        said: kept
            .iter()
            .filter_map(|(name, count)| Some(((*name).to_owned(), count?.to_string())))
            .collect(),
    }
}

fn hottest<T>(list: &[T], value: impl Fn(&T) -> f64) -> Option<&T> {
    list.iter()
        .reduce(|a, b| if value(a) >= value(b) { a } else { b })
}

/// What the rows are said with: the words, and each spark's reads.
struct Say<'a> {
    words: &'a Words,
    series: &'a BTreeMap<String, Vec<f64>>,
}

impl Say<'_> {
    /// A part's words, by the phrase's name: Heat's own where `part` is "".
    fn says(&self, part: &str, phrase: &str, fill: &[(&str, &str)]) -> String {
        let id = if part.is_empty() {
            PLACE.to_owned()
        } else {
            format!("heat:{part}")
        };
        self.words.said(&id, phrase, fill)
    }

    fn name(&self, id: &str, fill: &[(&str, &str)]) -> String {
        self.words.name(&format!("heat:{id}"), fill)
    }

    fn measured(&self, value: f64) -> String {
        self.words.significant(value)
    }

    fn degrees(&self, value: f64) -> String {
        format!("{} \u{b0}C", self.measured(value))
    }

    fn rpm(&self, value: f64) -> String {
        format!("{} rpm", self.words.number(value, 0))
    }

    #[expect(clippy::cast_precision_loss, reason = "a count is far below 2^52")]
    fn times(&self, count: u64) -> String {
        match count {
            0 => self.says("throttled", "never", &[]),
            1 => self.says("throttled", "once", &[]),
            _ => self.says(
                "throttled",
                "times",
                &[("count", &self.words.number(count as f64, 0))],
            ),
        }
    }

    /// A length of time in milliseconds, as a clock reads it at a glance.
    #[expect(
        clippy::cast_precision_loss,
        reason = "a length in ms is far below 2^52"
    )]
    fn lasted(&self, ms: u64) -> String {
        let at = ms as f64;
        let number = |value: f64, places| self.words.number(value, places);
        match ms {
            0..1000 => format!("{} ms", number(at, 0)),
            1000..120_000 => format!("{} s", number(at / 1000.0, usize::from(ms < 10_000))),
            120_000..7_200_000 => format!("{} min", number(at / 60_000.0, 0)),
            _ => format!("{} h", number(at / 3_600_000.0, 1)),
        }
    }

    /// A part's name, numbered when there are two of it, as two disks are.
    fn part_name(&self, part: &Part) -> String {
        let name = self.name(&part.id, &[]);
        if part.nth > 1 {
            format!("{name} {}", part.nth)
        } else {
            name
        }
    }

    fn fan_name(&self, fan: &Fan) -> String {
        if fan.id == "fan" {
            return self.name("fan", &[("n", &fan.n.to_string())]);
        }
        let id = format!(
            "heat:{}",
            fan.label
                .to_lowercase()
                .split(' ')
                .collect::<Vec<_>>()
                .join("-")
        );
        if self.words.has(&id) {
            self.words.name(&id, &[])
        } else {
            capitalised(&fan.label)
        }
    }

    /// A disk's other sensor, by its label: "Sensor 2".
    fn sensor_name(&self, label: &str) -> String {
        match label
            .strip_prefix("Sensor ")
            .filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
        {
            Some(n) => self.name("sensor", &[("n", n)]),
            None => capitalised(label),
        }
    }

    /// What's left to a limit, or how far past it: "29.0 °C below high".
    fn limit_said(&self, left: &Left) -> String {
        let phrase = if left.past { "past" } else { "below" };
        self.says(left.limit, phrase, &[("by", &self.measured(left.by))])
    }

    /// Its Last Reads, while shown, as a spark and its range.
    fn lately(
        &self,
        id: String,
        values: &[f64],
        said: impl Fn(f64) -> String,
        drawn: &[f64],
    ) -> Made {
        let least = values.iter().copied().fold(f64::INFINITY, f64::min);
        let most = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let span = if least.round().total_cmp(&most.round()).is_eq() {
            said(most)
        } else {
            self.says(
                "lately",
                "range",
                &[("least", &self.measured(least)), ("most", &said(most))],
            )
        };
        let note = if values.len() == 1 {
            self.says("lately", "one", &[])
        } else {
            self.says("lately", "many", &[("count", &values.len().to_string())])
        };
        figure(id, self.name("lately", &[]), span, Some(spark(drawn)), note)
    }

    /// A part's drawer: its eyebrow, the limits its side counts down to,
    /// where the firmware acts on its heat, how far it's stepping the
    /// processor's cooling, what its chip says of electricity, and the
    /// Linux Name of the chip reading it and the Zones beside it (VKCF3Z,
    /// 2026-11-08). None where it has none of these.
    fn temp_drawer(&self, part: &Part, read: &Read) -> Option<Value> {
        let mut facts = Vec::new();
        let fact = |name: String, said: String| json!({ "name": name, "said": said });
        if let Some(high) = part.high {
            facts.push(fact(self.name("high", &[]), self.degrees(high)));
        }
        if let Some(crit) = part.crit {
            facts.push(fact(self.name("critical", &[]), self.degrees(crit)));
        }
        let tripped: Vec<Trip> = parts::trips_for(&read.zones, &part.id)
            .into_iter()
            .filter(|trip| TRIPS.contains(&trip.kind.as_str()))
            .collect();
        for trip in &tripped {
            facts.push(fact(
                self.name(&format!("trip:{}", trip.kind), &[]),
                self.degrees(trip.value),
            ));
        }
        if part.id == "processor" {
            for cooling in &read.cooling {
                let of = [
                    ("state", cooling.state.to_string()),
                    ("most", cooling.max.to_string()),
                ];
                let said = self.says(
                    "cooling",
                    "of",
                    &of.each_ref().map(|(k, v)| (*k, v.as_str())),
                );
                let qualifier = if cooling.count > 1 {
                    format!("{} \u{d7}{}", cooling.kind, cooling.count)
                } else {
                    cooling.kind.clone()
                };
                facts.push(json!({ "name": self.name("cooling", &[]), "said": said,
                    "qualifier": qualifier }));
            }
        }
        if let Some(electric) = &part.electric {
            for (list, id, unit) in [
                (&electric.volts, "voltage", "V"),
                (&electric.amps, "current", "A"),
                (&electric.watts, "drawing", "W"),
            ] {
                for each in list {
                    let label = if !each.label.is_empty() {
                        each.label.clone()
                    } else if list.len() > 1 {
                        format!("{} {}", self.name(id, &[]), each.n)
                    } else {
                        self.name(id, &[])
                    };
                    facts.push(fact(label, self.electric(each.value, unit)));
                }
            }
        }
        let mut zoned: Vec<&str> = Vec::new();
        for trip in &tripped {
            if !zoned.contains(&trip.zone.as_str()) {
                zoned.push(&trip.zone);
            }
        }
        if !part.chip.is_empty() {
            facts.push(
                json!({ "name": self.name("chip", &[]), "said": part.chip, "identifies": true }),
            );
        }
        if !zoned.is_empty() {
            facts.push(
                json!({ "name": self.name("zone", &[]), "said": zoned.join(", "),
                "identifies": true }),
            );
        }
        if facts.is_empty() {
            return None;
        }
        let eyebrow = self.says(
            "chip",
            if part.chip.is_empty() {
                "zone"
            } else {
                "sensor"
            },
            &[],
        );
        // The eyebrow names where it's read, so it's drawn as a name is
        // (Drawer.qml's eyebrowNames), as Heat.qml drew it (Y2YBTS,
        // 2026-11-11).
        let title = self.part_name(part);
        Some(json!({ "eyebrow": eyebrow, "eyebrowNames": true, "title": title, "facts": facts }))
    }

    /// A row's drawer saying the files Linux keeps behind it, by their own
    /// names and as they say them, a number grouped as Memory's are, then
    /// the folder each is in (#2204's bar: the kernel's own, never
    /// smoothed over). A row none of whose files is known is left as it is.
    fn keeps(&self, made: Made, files: &[&Files], wanted: impl Fn(&str) -> bool) -> Made {
        let mut facts = Vec::new();
        for each in files {
            let kept = each.only(&wanted);
            if kept.is_empty() {
                continue;
            }
            facts.extend(
                kept.into_iter()
                    .map(|(name, said)| json!({ "name": name, "said": self.raw(&said) })),
            );
            facts.push(json!({ "name": self.words.name("machine:read-from", &[]),
                "said": each.folder, "identifies": true }));
        }
        if facts.is_empty() {
            return made;
        }
        let drawer = json!({ "eyebrow": self.words.said("machine:read-from", PLACE, &[]),
            "eyebrowNames": true, "title": made.name, "facts": facts });
        Made {
            drawer: Some(drawer),
            ..made
        }
    }

    /// What a file says, a whole number grouped: "68,850".
    #[expect(
        clippy::cast_precision_loss,
        reason = "a sensor's file says far below 2^52"
    )]
    fn raw(&self, said: &str) -> String {
        said.parse::<i64>()
            .map_or_else(|_| said.to_owned(), |n| self.words.number(n as f64, 0))
    }

    /// A holder's drawer, what it holds without stepping in, as Machine's
    /// holders say it.
    fn holding(&self, made: Made) -> Made {
        let say = crate::machine_rows::Say { words: self.words };
        Made {
            drawer: Some(say.holding(PLACE, &made.name, &made.inside)),
            ..made
        }
    }

    /// A voltage, current or power in the unit that fits it: a figure
    /// under one in its unit is said in thousandths, "3.00 mV", rather
    /// than "0.00300 V"; one that rounds up to one is said whole.
    fn electric(&self, value: f64, unit: &str) -> String {
        let rounded: f64 = format!("{value:.2e}").parse().unwrap_or(value);
        let small = value != 0.0 && rounded.abs() < 1.0;
        let figure = self.measured(if small { value * 1000.0 } else { value });
        format!("{figure} {}{unit}", if small { "m" } else { "" })
    }

    /// Where a part of it reads hotter than the whole, as a core or a
    /// disk's second sensor can, so its note says so first; else "".
    fn hotter(&self, part: &Part) -> String {
        let whole = part.value.round();
        if let Some(core) =
            hottest(&part.cores, |core| core.value).filter(|core| core.value.round() > whole)
        {
            return self.says(
                "cores",
                "hottest",
                &[
                    ("n", &core.cpu.to_string()),
                    ("degrees", &self.degrees(core.value)),
                ],
            );
        }
        if let Some(sensor) =
            hottest(&part.sensors, |s| s.value).filter(|s| s.value.round() > whole)
        {
            return self.says(
                "sensor",
                "hotter",
                &[
                    ("sensor", &self.sensor_name(&sensor.label).to_lowercase()),
                    ("degrees", &self.degrees(sensor.value)),
                ],
            );
        }
        String::new()
    }

    fn temp_row(&self, part: &Part, read: &Read) -> Made {
        // Beside it, what's left to its limit, which alone wears the
        // warning past the high and the alert past the critical (VKCF3Z,
        // 2026-11-08).
        let degrees = self.degrees(part.value);
        let mut side = vec![json!({ "said": degrees, "keep": 3 })];
        if let Some(left) = parts::left_to(part.value, part.high, part.crit) {
            let mut said = json!({ "said": self.limit_said(&left), "keep": 1 });
            if !left.hue.is_empty() {
                said["hue"] = json!(left.hue);
            }
            side.push(said);
        }
        Made {
            id: temp_key(part),
            kind: "part",
            name: self.part_name(part),
            glyph: self.words.glyph(PLACE),
            side,
            copies: degrees,
            graph: Some(level(part.value, part)),
            note: self.hotter(part),
            drawer: self.temp_drawer(part, read),
            inside: self.temp_parts(part),
            ..Made::default()
        }
    }

    fn temp_parts(&self, part: &Part) -> Vec<Made> {
        let key = temp_key(part);
        let values = self
            .series
            .get(&key)
            .cloned()
            .unwrap_or_else(|| vec![part.value]);
        let at = |what: &str| format!("{key}/{what}");
        let lately = self.lately(
            at("lately"),
            &values,
            |v| self.degrees(v),
            &parts::heat_spark(&values),
        );
        let mut rows = vec![self.keeps(lately, &[&part.files], read_now)];
        rows.extend(self.cores_row(part, &key));
        for sensor in &part.sensors {
            let said = self.degrees(sensor.value);
            let name = self.sensor_name(&sensor.label);
            let made = figure(
                at(&format!("sensor/{}", sensor.n)),
                name,
                said,
                Some(level(sensor.value, part)),
                String::new(),
            );
            rows.push(self.keeps(made, &[&sensor.files], |name| {
                name.ends_with("_input") || name.ends_with("_label")
            }));
        }
        rows.extend(self.limits(part, &key));
        rows
    }

    /// A processor's cores, a step in from it, each a step in again.
    fn cores_row(&self, part: &Part, key: &str) -> Option<Made> {
        let at = |what: &str| format!("{key}/{what}");
        hottest(&part.cores, |core| core.value).map(|core| {
            let levels: Vec<parts::Level> = part
                .cores
                .iter()
                .map(|each| parts::level(each.value, part.high, part.crit, &part.id))
                .collect();
            let count = part.cores.len().to_string();
            self.holding(Made {
                id: at("cores"),
                kind: "holder",
                name: self.name("cores", &[]),
                glyph: self.words.glyph(PLACE),
                side: vec![item(&count)],
                copies: count,
                graph: Some(strip(
                    &levels.iter().map(|each| each.value).collect::<Vec<_>>(),
                    levels.iter().any(|each| each.warn),
                )),
                note: self.says(
                    "cores",
                    "hottest",
                    &[
                        ("n", &core.cpu.to_string()),
                        ("degrees", &self.degrees(core.value)),
                    ],
                ),
                inside: part
                    .cores
                    .iter()
                    .map(|each| {
                        let name = self.name("core", &[("n", &each.cpu.to_string())]);
                        let said = self.degrees(each.value);
                        let made = figure(
                            at(&format!("cores/{}", each.cpu)),
                            name,
                            said,
                            Some(level(each.value, part)),
                            String::new(),
                        );
                        self.keeps(made, &[&each.files], |name| {
                            name.ends_with("_input") || name.ends_with("_label")
                        })
                    })
                    .collect(),
                ..Made::default()
            })
        })
    }

    /// A part's limits and its chip's warning, each a step in from it.
    fn limits(&self, part: &Part, key: &str) -> Vec<Made> {
        let at = |what: &str| format!("{key}/{what}");
        let mut rows = Vec::new();
        // Each limit's drawer is the file the chip says it in.
        let point = |id: &str, value: f64| {
            let file = match id {
                "high" => "_max",
                "critical" => "_crit",
                "low" => "_min",
                _ => "_emergency",
            };
            let made = figure(
                at(id),
                self.name(id, &[]),
                self.degrees(value),
                None,
                self.says(id, "note", &[]),
            );
            self.keeps(made, &[&part.files], |name| name.ends_with(file))
        };
        if let Some(high) = part
            .high
            .filter(|high| part.crit.is_none_or(|crit| *high < crit))
        {
            rows.push(point("high", high));
        }
        if let Some(crit) = part.crit {
            rows.push(point("critical", crit));
        }
        if let Some(emergency) = part.emergency {
            rows.push(point("emergency", emergency));
        }
        if let Some(low) = part.low {
            rows.push(point("low", low));
        }
        if part.alarm {
            let raised = self.says("alarm", "raised", &[]);
            let mut alarm = figure(
                at("alarm"),
                self.name("alarm", &[]),
                raised,
                None,
                self.says("alarm", "note", &[]),
            );
            alarm.hue = "warning";
            rows.push(self.keeps(alarm, &[&part.files], |name| name.ends_with("alarm")));
        }
        rows
    }

    fn throttled_row(&self, said: &Throttled) -> Made {
        let drawn = self.series.get("throttled").cloned().unwrap_or_default();
        let most = drawn.iter().copied().fold(1.0, f64::max);
        let shares: Vec<f64> = drawn.iter().map(|value| value / most).collect();
        let times = self.times(said.times);
        let note = match said.ms.filter(|ms| *ms > 0) {
            Some(ms) => self.says("throttled", "for", &[("time", &self.lasted(ms))]),
            None => self.says("throttled", "since", &[]),
        };
        let made = Made {
            id: "throttled".to_owned(),
            kind: "throttled",
            name: self.name("throttled", &[]),
            glyph: self.words.glyph(PLACE),
            side: vec![item(&times)],
            copies: times,
            graph: Some(spark(&shares)),
            note,
            inside: self.throttled_parts(said),
            ..Made::default()
        };
        self.package_keeps(made, said)
    }

    /// A row of the package's slowing, with the counts the kernel keeps
    /// of it in its drawer.
    fn package_keeps(&self, made: Made, said: &Throttled) -> Made {
        let Some(cpu) = said.package_cpu else {
            return made;
        };
        let files = counted(
            throttle_folder(cpu),
            &[
                ("package_throttle_count", Some(said.times)),
                ("package_throttle_total_time_ms", said.ms),
                ("package_throttle_max_time_ms", said.longest),
            ],
        );
        self.keeps(made, &[&files], |_| true)
    }

    #[expect(clippy::cast_precision_loss, reason = "a count is far below 2^52")]
    fn throttled_parts(&self, said: &Throttled) -> Vec<Made> {
        let most = said
            .cores
            .iter()
            .map(|(_, count)| *count)
            .max()
            .unwrap_or(0)
            .max(1) as f64;
        let note: Vec<String> = [
            said.ms
                .filter(|ms| *ms > 0)
                .map(|ms| self.says("package", "for", &[("time", &self.lasted(ms))])),
            said.longest
                .filter(|ms| *ms > 0)
                .map(|ms| self.says("package", "longest", &[("time", &self.lasted(ms))])),
        ]
        .into_iter()
        .flatten()
        .collect();
        let times = self.times(said.times);
        let package = figure(
            "throttled/package".to_owned(),
            self.name("package", &[]),
            times,
            None,
            note.join(", "),
        );
        let mut rows = vec![self.package_keeps(package, said)];
        if said.cores.is_empty() {
            return rows;
        }
        let busiest = said
            .cores
            .iter()
            .reduce(|a, b| if a.1 >= b.1 { a } else { b });
        let note = match busiest.filter(|(_, count)| *count > 0) {
            Some((cpu, count)) => self.says(
                "throttled-cores",
                "most",
                &[("n", &cpu.to_string()), ("times", &self.times(*count))],
            ),
            None => self.says("throttled-cores", "alone", &[]),
        };
        let total = self.times(said.total);
        rows.push(
            self.holding(Made {
                id: "throttled/cores".to_owned(),
                kind: "holder",
                name: self.name("throttled-cores", &[]),
                glyph: self.words.glyph(PLACE),
                side: vec![item(&total)],
                copies: total,
                graph: Some(strip(
                    &said
                        .cores
                        .iter()
                        .map(|(_, count)| *count as f64 / most)
                        .collect::<Vec<_>>(),
                    false,
                )),
                note,
                inside: said
                    .cores
                    .iter()
                    .map(|(cpu, count)| {
                        let name = self.name("throttled-core", &[("n", &cpu.to_string())]);
                        let made = figure(
                            format!("throttled/cores/{cpu}"),
                            name,
                            self.times(*count),
                            Some(bar(*count as f64 / most, false)),
                            String::new(),
                        );
                        let (ms, longest) = said.core_ms.get(cpu).copied().unwrap_or_default();
                        let files = counted(
                            throttle_folder(*cpu),
                            &[
                                ("core_throttle_count", Some(*count)),
                                ("core_throttle_total_time_ms", ms),
                                ("core_throttle_max_time_ms", longest),
                            ],
                        );
                        self.keeps(made, &[&files], |_| true)
                    })
                    .collect(),
                ..Made::default()
            }),
        );
        rows
    }

    /// How hard a fan is driven, as its note says it, or "".
    fn powered(&self, fan: &Fan) -> String {
        match fan.power {
            None => String::new(),
            Some(power) if power >= 1.0 => self.says("fan", "full", &[]),
            Some(power) => self.says("fan", "share", &[("pct", &percent(power))]),
        }
    }

    fn fan_row(&self, fan: &Fan) -> Made {
        let side = if fan.value > 0.0 {
            self.rpm(fan.value)
        } else {
            self.says("fan", "stopped", &[])
        };
        let rated = fan
            .max
            .map(|max| self.says("fan", "rated", &[("speed", &self.rpm(max))]));
        let note: Vec<String> = [rated, Some(self.powered(fan))]
            .into_iter()
            .flatten()
            .filter(|said| !said.is_empty())
            .collect();
        let made = Made {
            id: fan_key(fan),
            kind: "fan",
            name: self.fan_name(fan),
            glyph: "fan".to_owned(),
            side: vec![item(&side)],
            copies: side,
            graph: fan.max.map(|max| bar(fan.value / max, false)),
            note: note.join(", "),
            inside: self.fan_parts(fan),
            ..Made::default()
        };
        self.keeps(made, &fan.files.iter().collect::<Vec<_>>(), |_| true)
    }

    fn fan_parts(&self, fan: &Fan) -> Vec<Made> {
        let key = fan_key(fan);
        let values = self
            .series
            .get(&key)
            .cloned()
            .unwrap_or_else(|| vec![fan.value]);
        let at = |what: &str| format!("{key}/{what}");
        let files: Vec<&Files> = fan.files.iter().collect();
        let keeps = |made: Made, wanted: fn(&str) -> bool| self.keeps(made, &files, wanted);
        let lately = self.lately(
            at("lately"),
            &values,
            |v| self.rpm(v),
            &parts::fan_spark(&values),
        );
        // A fan told by two chips keeps its own reading first, so its Last
        // Reads say only its own chip's.
        let own: Vec<&Files> = files.iter().take(1).copied().collect();
        let mut rows = vec![self.keeps(lately, &own, read_now)];
        if let Some(power) = fan.power {
            let pct = format!("{}%", percent(power));
            let made = figure(
                at("power"),
                self.name("power", &[]),
                pct,
                Some(bar(power, false)),
                self.says("power", "note", &[]),
            );
            rows.push(keeps(made, pwm));
        }
        if let Some(target) = fan.target {
            let made = figure(
                at("aiming"),
                self.name("aiming", &[]),
                self.rpm(target),
                None,
                self.says("aiming", "note", &[]),
            );
            rows.push(keeps(made, |name| name.ends_with("_target")));
        }
        if let Some(min) = fan.min {
            let made = figure(
                at("least"),
                self.name("least", &[]),
                self.rpm(min),
                None,
                self.says("least", "note", &[]),
            );
            rows.push(keeps(made, |name| name.ends_with("_min")));
        }
        let control = fan
            .mode
            .as_deref()
            .map(|mode| self.says("control", mode, &[]))
            .unwrap_or_default();
        if !control.is_empty() {
            let made = figure(
                at("control"),
                self.name("control", &[]),
                control,
                None,
                self.says("control", "note", &[]),
            );
            rows.push(keeps(made, |name| name.ends_with("_enable")));
        }
        rows
    }

    /// How many fans turn, as the headline says it.
    fn turnings(&self, count: usize) -> String {
        match count {
            0 => self.says("", "stopped", &[]),
            1 => self.says("", "turning", &[]),
            _ => self.says("", "turnings", &[("count", &count.to_string())]),
        }
    }

    /// Heat's headline, `head:heat`: the hottest part and the fans.
    fn head(&self, temps: &[Part], fans: &[Fan]) -> Value {
        let hot = hottest(temps, |part| part.value);
        let mut said = Vec::new();
        if let Some(hot) = hot {
            said.push(self.says(
                "",
                "hottest",
                &[
                    ("degrees", &self.degrees(hot.value)),
                    ("part", &parts::phrase(&self.part_name(hot))),
                ],
            ));
        }
        if !fans.is_empty() {
            said.push(self.turnings(fans.iter().filter(|fan| fan.value > 0.0).count()));
        }
        let mut head =
            json!({ "key": format!("head:{PLACE}"), "place": PLACE, "said": said.join(", ") });
        if let Some(hot) = hot {
            head["graph"] = level(hot.value, hot);
        }
        head
    }
}

/// A share as a whole percentage, a half rounded up as the shell did.
fn percent(share: f64) -> String {
    format!("{:.0}", (share * 100.0).round())
}

fn capitalised(label: &str) -> String {
    let mut chars = label.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The words these cases say: the launcher's own (`Words::launchers`).
    fn words() -> Words {
        Words::launchers("", "")
    }

    fn sensors(processor: f64, core: f64) -> Vec<Value> {
        vec![
            json!({ "key": "sensor:coretemp", "part": "processor", "chip": "coretemp",
                "folder": "/sys/class/hwmon/hwmon4", "temperatures": [
                    { "n": 1, "label": "Package id 0", "temperature_c": processor,
                        "high_c": 100, "critical_c": 110,
                        "files": [["temp1_input", "71000"], ["temp1_label", "Package id 0"],
                            ["temp1_max", "100000"], ["temp1_crit", "110000"]] },
                    { "n": 2, "label": "Core 0", "temperature_c": core, "high_c": 100,
                        "critical_c": 110,
                        "files": [["temp2_input", "79000"], ["temp2_label", "Core 0"]] },
                    { "n": 3, "label": "Core 4", "temperature_c": 60, "high_c": 100,
                        "critical_c": 110,
                        "files": [["temp3_input", "60000"], ["temp3_label", "Core 4"]] }],
                "fans": [] }),
            json!({ "key": "sensor:nvme", "part": "disk", "folder": "/sys/class/hwmon/hwmon1",
                "temperatures": [
                { "n": 1, "label": "Composite", "temperature_c": 52.9, "high_c": 82.9,
                    "critical_c": 84.9, "files": [["temp1_input", "52850"],
                        ["temp1_max", "82850"], ["temp1_crit", "84850"]] },
                { "n": 2, "label": "Sensor 2", "temperature_c": 68.9,
                    "files": [["temp2_input", "68850"], ["temp2_label", "Sensor 2"]] }],
                "fans": [] }),
            json!({ "key": "sensor:dell_smm", "part": "fans",
                "folder": "/sys/class/hwmon/hwmon6", "temperatures": [], "fans": [
                { "n": 1, "label": null, "rpm": 2400, "max_rpm": 4800, "power_ratio": 0.5,
                    "mode": "automatic", "files": [["fan1_input", "2400"],
                        ["fan1_max", "4800"], ["pwm1", "128"], ["pwm1_enable", "2"]] },
                { "n": 2, "label": null, "rpm": 0, "max_rpm": 4800,
                    "files": [["fan2_input", "0"], ["fan2_max", "4800"]] }] }),
        ]
    }

    fn processor() -> Value {
        json!({ "cores": [{ "n": 0, "core": 0 }, { "n": 2, "core": 4 }],
            "throttled": { "cores": [{ "core": 0, "cpu": 0, "count": 24, "ms": 310 },
                { "core": 4, "cpu": 2, "count": 8, "ms": 90, "longest_ms": 12 }],
                "cores_count": 32, "package_cpu": 0, "package_count": 3202,
                "package_ms": 19_016, "package_longest_ms": 125 } })
    }

    fn zones() -> Vec<Value> {
        vec![json!({ "type": "TCPU", "temperature_c": 78.5,
            "trips": [{ "type": "passive", "temperature_c": 95 }] })]
    }

    fn read(heat: &mut Heat, processor_c: f64, core_c: f64, now: u64) -> Ready {
        let sensors = sensors(processor_c, core_c);
        let zones = zones();
        let cpu = processor();
        let held = Held {
            sensors: &sensors,
            zones: &zones,
            processor: Some(&cpu),
            ..Held::default()
        };
        heat.read(held, now, &words())
    }

    fn row<'a>(ready: &'a Ready, id: &str) -> &'a Value {
        let key = format!("row:heat:{id}");
        let found = ready.rows.iter().find(|row| row["key"] == key.as_str());
        found.unwrap_or_else(|| panic!("no row {key}"))
    }

    fn ids<'a>(ready: &'a Ready, parent: Option<&str>) -> Vec<&'a str> {
        let parent = parent.map(|id| format!("row:heat:{id}"));
        ready
            .rows
            .iter()
            .filter(|row| row["parent"].as_str() == parent.as_deref())
            .map(|row| {
                row["key"]
                    .as_str()
                    .unwrap_or_default()
                    .trim_start_matches("row:heat:")
            })
            .collect()
    }

    /// The parts first, throttling right after the processor, then the
    /// fans; each a step in from its own, shown after it.
    #[test]
    fn each_part_then_its_rows_a_step_inside() {
        let ready = read(&mut Heat::default(), 71.0, 79.0, 1000);
        assert_eq!(
            ids(&ready, None),
            [
                "processor:0",
                "throttled",
                "disk:0",
                "fan:fan:1",
                "fan:fan:2"
            ]
        );
        let inside = ids(&ready, Some("processor:0"));
        assert_eq!(
            inside,
            [
                "processor:0/lately",
                "processor:0/cores",
                "processor:0/high",
                "processor:0/critical"
            ]
        );
        let core = row(&ready, "processor:0/cores/0");
        assert_eq!(
            (core["depth"].clone(), core["name"].clone()),
            (json!(2), json!("Core 0"))
        );
        let shown: Vec<&str> = ready
            .rows
            .iter()
            .filter_map(|row| row["key"].as_str())
            .collect();
        let at = |key: &str| shown.iter().position(|one| *one == key);
        assert!(
            at("row:heat:processor:0") < at("row:heat:processor:0/cores")
                && at("row:heat:processor:0/cores") < at("row:heat:processor:0/cores/0")
                && at("row:heat:processor:0/critical") < at("row:heat:throttled"),
            "each row inside shown after its own and before the next part: {shown:?}"
        );
    }

    /// What's left to its limit beside it, a hotter core in its note, and
    /// its drawer's limits, the firmware's trip and its Linux Name.
    #[test]
    fn a_part_says_its_heat_and_its_limits() {
        let ready = read(&mut Heat::default(), 71.0, 79.0, 1000);
        let part = row(&ready, "processor:0");
        assert_eq!(
            part["side"],
            json!([{ "said": "71.0 \u{b0}C", "keep": 3 },
                { "said": "29.0 \u{b0}C below high", "keep": 1 }])
        );
        assert_eq!(part["note"], "the hottest, core 0 at 79.0 \u{b0}C");
        assert_eq!(
            part["graph"],
            json!({ "shape": "bar", "values": [0.65], "warn": false })
        );
        let facts: Vec<&str> = part["drawer"]["facts"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|fact| fact["name"].as_str().unwrap_or_default())
            .collect();
        assert_eq!(
            facts,
            ["High", "Critical", "Throttles At", "Linux Name", "Zone"]
        );
        assert_eq!(part["drawer"]["eyebrow"], "on this machine \u{b7} sensor");
        assert_eq!(
            part["drawer"]["eyebrowNames"], true,
            "its eyebrow is drawn as a name"
        );
        assert_eq!(
            row(&ready, "disk:0")["note"],
            "sensor 2 hotter at 68.9 \u{b0}C"
        );
        let acts = &part["acts"];
        assert_eq!(
            acts[0],
            json!({ "key": "verb:copy", "copy": "71.0 \u{b0}C" })
        );
        assert_eq!(acts[1]["key"], "verb:more");
    }

    #[test]
    fn throttling_and_the_fans_are_said() {
        let ready = read(&mut Heat::default(), 71.0, 79.0, 1000);
        let throttled = row(&ready, "throttled");
        assert_eq!(throttled["side"], json!([{ "said": "3,202 times" }]));
        assert_eq!(
            throttled["note"],
            "for 19 s in all, since the machine started"
        );
        assert_eq!(
            row(&ready, "throttled/cores")["note"],
            "the most, core 0, 24 times"
        );
        let fan = row(&ready, "fan:fan:1");
        assert_eq!(fan["side"], json!([{ "said": "2,400 rpm" }]));
        assert_eq!(fan["note"], "rated for 4,800 rpm, at 50% power");
        assert_eq!(
            row(&ready, "fan:fan:2")["side"],
            json!([{ "said": "stopped" }])
        );
        let head = ready.head.as_ref().map(|head| head["said"].clone());
        assert_eq!(
            head,
            Some(json!("71.0 \u{b0}C at the processor, a fan turning"))
        );
    }

    /// A jitter of under a degree leaves each drawing as it was, so the
    /// bar and the cores' strip aren't sent again (Y2YBTS, 2026-11-10).
    #[test]
    fn a_jitter_under_a_step_leaves_the_drawing() {
        let mut heat = Heat::default();
        let before = read(&mut heat, 71.0, 79.0, 1000);
        let after = read(&mut heat, 71.6, 79.4, 2000);
        for id in ["processor:0", "processor:0/cores", "processor:0/cores/0"] {
            assert_eq!(
                row(&before, id)["graph"],
                row(&after, id)["graph"],
                "{id}'s drawing"
            );
        }
        assert_eq!(
            before.head.map(|h| h["graph"].clone()),
            after.head.map(|h| h["graph"].clone())
        );
    }

    /// Every row has a drawer giving a little more than its line (Joel,
    /// 2026-11-11): a holder what it holds, and every other the files
    /// Linux keeps behind it, as they say it, and the folder they're in.
    #[test]
    fn every_heat_row_has_a_drawer_saying_more() {
        let ready = read(&mut Heat::default(), 71.0, 79.0, 1000);
        let bare: Vec<&str> = ready
            .rows
            .iter()
            .filter(|row| row["drawer"].is_null())
            .filter_map(|row| row["key"].as_str())
            .collect();
        assert!(bare.is_empty(), "rows without a drawer: {bare:?}");
        let facts = |id: &str| -> Vec<String> {
            let drawer = &row(&ready, id)["drawer"];
            drawer["facts"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|fact| format!("{}={}", text(&fact["name"]), text(&fact["said"])))
                .collect()
        };
        let coretemp = "Read From=/sys/class/hwmon/hwmon4";
        let cases: [(&str, &[&str]); 10] = [
            ("processor:0/lately", &["temp1_input=71,000", coretemp]),
            ("processor:0/high", &["temp1_max=100,000", coretemp]),
            ("processor:0/critical", &["temp1_crit=110,000", coretemp]),
            (
                "processor:0/cores",
                &["Core 0=79.0 \u{b0}C", "Core 2=60.0 \u{b0}C"],
            ),
            (
                "processor:0/cores/2",
                &["temp3_input=60,000", "temp3_label=Core 4", coretemp],
            ),
            (
                "disk:0/sensor/2",
                &[
                    "temp2_input=68,850",
                    "temp2_label=Sensor 2",
                    "Read From=/sys/class/hwmon/hwmon1",
                ],
            ),
            (
                "throttled",
                &[
                    "package_throttle_count=3,202",
                    "package_throttle_total_time_ms=19,016",
                    "package_throttle_max_time_ms=125",
                    "Read From=/sys/devices/system/cpu/cpu0/thermal_throttle",
                ],
            ),
            (
                "throttled/cores/2",
                &[
                    "core_throttle_count=8",
                    "core_throttle_total_time_ms=90",
                    "core_throttle_max_time_ms=12",
                    "Read From=/sys/devices/system/cpu/cpu2/thermal_throttle",
                ],
            ),
            (
                "fan:fan:1",
                &[
                    "fan1_input=2,400",
                    "fan1_max=4,800",
                    "pwm1=128",
                    "pwm1_enable=2",
                    "Read From=/sys/class/hwmon/hwmon6",
                ],
            ),
            (
                "fan:fan:1/control",
                &["pwm1_enable=2", "Read From=/sys/class/hwmon/hwmon6"],
            ),
        ];
        for (id, expected) in cases {
            assert_eq!(facts(id), expected, "{id}'s drawer");
        }
        let lately = &row(&ready, "processor:0/lately")["drawer"];
        assert_eq!(lately["eyebrow"], "on this machine \u{b7} heat");
        assert_eq!(lately["title"], "Last Reads");
    }

    fn text(value: &Value) -> String {
        value.as_str().unwrap_or_default().to_owned()
    }

    /// The Last Reads gather while watched, start afresh after a gap, and
    /// go with the last watch.
    #[test]
    fn the_last_reads_are_kept_only_while_watched() {
        const ONE: &str = "one read so far, one a second while it's shown";
        let mut heat = Heat::default();
        let lately = |ready: &Ready| row(ready, "processor:0/lately")["note"].clone();
        let first = read(&mut heat, 71.0, 79.0, 1000);
        assert_eq!(lately(&first), ONE);
        read(&mut heat, 72.0, 79.0, 2000);
        // Read again a fifth of a second on, as a read not yet of now is,
        // the spark takes nothing.
        assert_eq!(
            lately(&read(&mut heat, 90.0, 79.0, 2200)),
            "the last 2 reads, one a second while it's shown"
        );
        let third = read(&mut heat, 75.0, 79.0, 3000);
        assert_eq!(
            lately(&third),
            "the last 3 reads, one a second while it's shown"
        );
        assert_eq!(
            row(&third, "processor:0/lately")["side"],
            json!([{ "said": "71.0 to 75.0 \u{b0}C" }])
        );
        let stale = read(&mut heat, 75.0, 79.0, 3000 + STALE_MS + 1);
        assert_eq!(lately(&stale), ONE);
        heat.forget();
        assert_eq!(lately(&read(&mut heat, 75.0, 79.0, 9000)), ONE);
    }
}
