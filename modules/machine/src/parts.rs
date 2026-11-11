//! The parts Heat's rows say, gathered from the sensor, zone, cooling and
//! processor records this daemon keeps ([`crate::heat`],
//! [`crate::processor`]) and the processor's own telemetry: the processor
//! and its cores, its NPU, image processor and media and display engines,
//! the disk and its other sensors, the memory, the air inside and each
//! fan, how near each is to the most it's meant to reach, and how often
//! the processor slowed itself to cool down. It's what the launcher's
//! Heat.js made of the same records, moved here so the shell holds no
//! data and builds no rows (Y2YBTS's ready-row form); its cases are the
//! ones tests/answers.qml held for Heat.js.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde_json::Value;

/// A sensor on a chip that measures many, as dell_ddv's are, by its label,
/// as the part it stands for. One labelled "CPU" or "Other" is left out:
/// the processor's own chip says the first, and the rest say nothing of
/// what they measure.
const LABELLED: [(&str, &str); 3] = [
    ("Ambient", "air"),
    ("SODIMM", "memory"),
    ("GPU", "graphics"),
];

/// The processor's own parts no hwmon chip measures, as its telemetry
/// reader says them, by the part Heat calls each.
const TELEMETRY: [(&str, &str); 4] = [
    ("npu", "npu"),
    ("imageProcessor", "image-processor"),
    ("media", "media-engine"),
    ("display", "display-engine"),
];

/// The parts in the order they're listed, the processor first as what's
/// asked most, and its own parts after its graphics.
const ORDER: [&str; 13] = [
    "processor",
    "graphics",
    "npu",
    "image-processor",
    "media-engine",
    "display-engine",
    "disk",
    "memory",
    "wifi",
    "chipset",
    "board",
    "battery",
    "air",
];

/// The parts a chip can stand for in Heat, by the part this daemon names
/// it for. A chip named for something else, as the fans' own chip or a
/// USB-C port's, measures many things or none, so its sensors are named by
/// their labels, as a chip of no part's are.
const CHIP_PARTS: [&str; 7] = [
    "processor",
    "disk",
    "wifi",
    "board",
    "chipset",
    "graphics",
    "battery",
];

/// What a part is drawn against when its chip says no most: the disk's
/// usual most, and a battery's, which wants to stay well below a
/// processor's.
fn usual_most(part: &str) -> f64 {
    match part {
        "disk" => 85.0,
        "battery" => 60.0,
        _ => 100.0,
    }
}

/// How near the most a temperature warns, where the chip says no lower
/// point of its own.
const NEAR: f64 = 10.0;
/// The least span of degrees a spark is drawn across, so a reading that
/// barely moves draws flat rather than its own noise full.
const SPAN: f64 = 10.0;
/// A fan's spark is drawn across this many turns a minute at least.
const FAN_SPAN: f64 = 500.0;
/// The temperatures a part could read, in degrees, beyond which a sensor
/// says nothing true: an unused disk sensor says 65,261.
const COLDEST: f64 = -40.0;
const HOTTEST: f64 = 150.0;

/// One temperature a chip reads, in degrees, with the limits it says.
#[derive(Clone, Debug, PartialEq)]
pub struct Temp {
    pub n: u64,
    pub label: String,
    pub value: f64,
    pub high: Option<f64>,
    pub crit: Option<f64>,
    pub low: Option<f64>,
    pub emergency: Option<f64>,
    pub alarm: bool,
    pub chip: String,
}

/// One fan a chip turns, in turns a minute, with what it says of it.
#[derive(Clone, Debug, PartialEq)]
pub struct Fan {
    pub id: String,
    pub n: u64,
    pub label: String,
    pub value: f64,
    pub max: Option<f64>,
    pub min: Option<f64>,
    pub target: Option<f64>,
    pub power: Option<f64>,
    pub mode: Option<String>,
}

/// A voltage, current or power a chip reads, in its unit.
#[derive(Clone, Debug, PartialEq)]
pub struct Figure {
    pub n: u64,
    pub label: String,
    pub value: f64,
}

/// What a chip says of electricity.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Electric {
    pub volts: Vec<Figure>,
    pub amps: Vec<Figure>,
    pub watts: Vec<Figure>,
}

/// A chip as Heat gathers it: what it stands for, its temperatures, its
/// fans and its electricity.
#[derive(Clone, Debug, PartialEq)]
pub struct Chip {
    pub chip: String,
    pub part: String,
    pub temps: Vec<Temp>,
    pub fans: Vec<Fan>,
    pub electric: Electric,
}

/// A thermal zone that stands for a part, and where the firmware acts on
/// its heat.
#[derive(Clone, Debug, PartialEq)]
pub struct Zone {
    pub part: String,
    pub kind: String,
    pub value: f64,
    pub trips: Vec<Trip>,
}

/// A point where the firmware acts on a part's heat, and the zone that
/// says it.
#[derive(Clone, Debug, PartialEq)]
pub struct Trip {
    pub kind: String,
    pub value: f64,
    pub zone: String,
}

/// A kind of cooling device, stepped as far as any of it is.
#[derive(Clone, Debug, PartialEq)]
pub struct Cooling {
    pub kind: String,
    pub count: u64,
    pub state: u64,
    pub max: u64,
}

/// One of a part's other sensors, as a disk's second one.
#[derive(Clone, Debug, PartialEq)]
pub struct Sensor {
    pub n: u64,
    pub label: String,
    pub value: f64,
}

/// A core's temperature, by its logical processor.
#[derive(Clone, Debug, PartialEq)]
pub struct Core {
    pub cpu: u64,
    pub value: f64,
}

/// A part Heat lists: its id, its whole reading and limits, its other
/// sensors and cores, and `nth`, its number among parts said twice, as
/// two disks are, or 0.
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    pub id: String,
    pub value: f64,
    pub high: Option<f64>,
    pub crit: Option<f64>,
    pub low: Option<f64>,
    pub emergency: Option<f64>,
    pub alarm: bool,
    pub chip: String,
    pub electric: Option<Electric>,
    pub sensors: Vec<Sensor>,
    pub cores: Vec<Core>,
    pub nth: usize,
}

/// The parts and the fans, as Heat lists them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Gathered {
    pub temps: Vec<Part>,
    pub fans: Vec<Fan>,
}

/// How often the processor slowed itself to cool down since the machine
/// started, the package's and each core's by its first logical processor.
#[derive(Clone, Debug, PartialEq)]
pub struct Throttled {
    pub times: u64,
    pub ms: Option<u64>,
    pub longest: Option<u64>,
    pub cores: Vec<(u64, u64)>,
    pub total: u64,
}

/// What's left of a temperature to its limit: the degrees to it or past
/// it, which limit, whether it's past, and the hue it's drawn in, "" below
/// where it warns.
#[derive(Clone, Debug, PartialEq)]
pub struct Left {
    pub by: f64,
    pub limit: &'static str,
    pub past: bool,
    pub hue: &'static str,
}

/// How near a temperature is to the most it's meant to reach, 0 to 1,
/// and whether it warns.
#[derive(Clone, Debug, PartialEq)]
pub struct Level {
    pub value: f64,
    pub warn: bool,
}

fn plausible(value: Option<f64>) -> Option<f64> {
    value.filter(|v| *v > COLDEST && *v < HOTTEST)
}

/// A limit a chip says, or none where it says one no temperature could
/// reach, or none at all.
fn limit_of(value: Option<f64>) -> Option<f64> {
    plausible(value).filter(|v| *v > 0.0)
}

fn number(value: &Value) -> Option<f64> {
    value.as_f64()
}

fn whole(value: &Value) -> Option<u64> {
    value.as_u64()
}

fn text(value: &Value) -> String {
    value.as_str().unwrap_or_default().to_owned()
}

fn list(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}

/// A name broken into runs of digits and of the rest, lowered, as the
/// launcher's Order.js reads names: "Core 10" after "Core 9".
fn pieces(name: &str) -> Vec<String> {
    let lowered = name.to_lowercase();
    let mut out: Vec<String> = Vec::new();
    for c in lowered.chars() {
        match out.last_mut() {
            Some(last) if last.chars().all(|d| d.is_ascii_digit()) == c.is_ascii_digit() => {
                last.push(c);
            }
            _ => out.push(c.to_string()),
        }
    }
    out
}

/// Two names in Order.js's order: A to Z with case alike, numbers by
/// their worth.
pub fn by_name(one: &str, other: &str) -> Ordering {
    let (a, b) = (pieces(one), pieces(other));
    for (x, y) in a.iter().zip(&b) {
        let digits = |p: &str| p.starts_with(|c: char| c.is_ascii_digit());
        let said = if digits(x) && digits(y) {
            let worth = |p: &str| p.parse::<f64>().unwrap_or(0.0);
            worth(x).total_cmp(&worth(y))
        } else {
            x.cmp(y)
        };
        if said != Ordering::Equal {
            return said;
        }
    }
    a.len().cmp(&b.len())
}

/// A chip's voltages, currents or power, from this daemon's list of them
/// and the figure each says in its unit.
fn electric_of(said: &Value, unit: &str) -> Vec<Figure> {
    list(said)
        .iter()
        .filter_map(|each| {
            Some(Figure {
                n: whole(&each["n"]).unwrap_or(0),
                label: text(&each["label"]),
                value: number(&each[unit])?,
            })
        })
        .collect()
}

/// Each chip as Heat gathers it, from the sensor records, in the order of
/// their keys. A chip of no part keeps only the sensors its labels name,
/// since the rest say nothing of what they measure; a fan whose speed
/// can't be read is left out.
pub fn chips_of(records: &[Value]) -> Vec<Chip> {
    let mut sorted: Vec<&Value> = records.iter().collect();
    sorted.sort_by(|a, b| by_name(&text(&a["key"]), &text(&b["key"])));
    sorted.into_iter().map(chip_of).collect()
}

fn chip_of(record: &Value) -> Chip {
    let named = text(&record["part"]);
    let part = if CHIP_PARTS.contains(&named.as_str()) {
        named
    } else {
        String::new()
    };
    let chip = text(&record["chip"]);
    let temps = list(&record["temperatures"])
        .iter()
        .filter_map(|each| {
            Some(Temp {
                n: whole(&each["n"]).unwrap_or(0),
                label: text(&each["label"]),
                value: plausible(number(&each["temperature_c"]))?,
                high: limit_of(number(&each["high_c"])),
                crit: limit_of(number(&each["critical_c"])),
                low: plausible(number(&each["low_c"])),
                emergency: limit_of(number(&each["emergency_c"])),
                alarm: each["alarm"] == Value::Bool(true),
                chip: chip.clone(),
            })
        })
        .filter(|each| !part.is_empty() || !labelled(&each.label).is_empty())
        .collect();
    let fans = list(&record["fans"])
        .iter()
        .filter_map(|each| {
            let label = text(&each["label"]);
            Some(Fan {
                id: fan_id(&label),
                n: whole(&each["n"]).unwrap_or(0),
                value: number(&each["rpm"])?,
                max: number(&each["max_rpm"]),
                min: number(&each["min_rpm"]),
                target: number(&each["target_rpm"]),
                power: number(&each["power_ratio"]),
                mode: each["mode"].as_str().map(str::to_owned),
                label,
            })
        })
        .collect();
    let electric = Electric {
        volts: electric_of(&record["voltages"], "volts"),
        amps: electric_of(&record["currents"], "amps"),
        watts: electric_of(&record["power"], "watts"),
    };
    Chip {
        chip,
        part,
        temps,
        fans,
        electric,
    }
}

/// What a thermal zone measures, by its type, for a machine whose hwmon
/// chips leave a part out. On a laptop with every chip they're all said
/// already, so they're read only for a part nothing else gives.
pub fn zone_part(kind: &str) -> &'static str {
    if ["x86_pkg_temp", "TCPU", "B0D4", "TCPU_PCI"].contains(&kind) {
        "processor"
    } else if kind == "acpitz" {
        "board"
    } else if kind.starts_with("iwlwifi") {
        "wifi"
    } else if kind.starts_with("pch_") {
        "chipset"
    } else {
        ""
    }
}

/// The thermal zones that stand for a part, from the zone records, their
/// trips within reach.
pub fn zones_of(records: &[Value]) -> Vec<Zone> {
    records
        .iter()
        .filter_map(|each| {
            let kind = text(&each["type"]);
            let part = zone_part(&kind);
            if part.is_empty() {
                return None;
            }
            let trips = list(&each["trips"])
                .iter()
                .filter_map(|trip| {
                    Some(Trip {
                        kind: text(&trip["type"]),
                        value: limit_of(number(&trip["temperature_c"]))?,
                        zone: kind.clone(),
                    })
                })
                .collect();
            Some(Zone {
                part: part.to_owned(),
                value: plausible(number(&each["temperature_c"]))?,
                kind,
                trips,
            })
        })
        .collect()
}

/// The points at which the firmware acts on a part's heat, from every
/// zone that stands for it, coolest first; a point two zones share is
/// said once.
pub fn trips_for(zones: &[Zone], part: &str) -> Vec<Trip> {
    let mut out: Vec<Trip> = Vec::new();
    for zone in zones.iter().filter(|zone| zone.part == part) {
        for trip in &zone.trips {
            if !out
                .iter()
                .any(|each| each.kind == trip.kind && each.value == trip.value)
            {
                out.push(trip.clone());
            }
        }
    }
    out.sort_by(|a, b| a.value.total_cmp(&b.value));
    out
}

/// Each kind of cooling device the firmware can step, by type, its state
/// the furthest any of them is stepped.
pub fn cooling_of(records: &[Value]) -> Vec<Cooling> {
    let mut out: Vec<Cooling> = records
        .iter()
        .filter_map(|each| {
            let max = whole(&each["max_state"]).filter(|most| *most > 0)?;
            Some(Cooling {
                kind: text(&each["type"]),
                count: whole(&each["count"]).unwrap_or(1),
                state: list(&each["states"])
                    .iter()
                    .filter_map(Value::as_u64)
                    .max()
                    .unwrap_or(0),
                max,
            })
        })
        .collect();
    out.sort_by(|a, b| by_name(&a.kind, &b.kind));
    out
}

/// Where a temperature starts to warn: the chip's own high point where
/// that's below its critical one, as a disk's is, else ten degrees short
/// of the critical, as a processor's high and critical are one; or
/// nowhere, with neither said.
fn warns_from(high: Option<f64>, crit: Option<f64>) -> Option<f64> {
    match (high, crit) {
        (Some(high), crit) if crit.is_none_or(|crit| high < crit) => Some(high),
        (_, Some(crit)) => Some(crit - NEAR),
        _ => None,
    }
}

/// What's left of a temperature to its limit (VKCF3Z, 2026-11-08). Past
/// the high, what's left is to the critical, where the chip says one;
/// where a chip's high is its critical, it warns ten degrees short of it.
/// None where it says neither.
pub fn left_to(value: f64, high: Option<f64>, crit: Option<f64>) -> Option<Left> {
    let warns = warns_from(high, crit);
    let distinct = high.filter(|high| crit.is_none_or(|crit| *high < crit));
    if let Some(crit) = crit.filter(|crit| value >= *crit) {
        return Some(Left {
            by: value - crit,
            limit: "critical",
            past: true,
            hue: "alert",
        });
    }
    let hue = if warns.is_some_and(|warns| value >= warns) {
        "warning"
    } else {
        ""
    };
    if let Some(crit) = crit.filter(|_| distinct.is_none_or(|high| value >= high)) {
        return Some(Left {
            by: crit - value,
            limit: "critical",
            past: false,
            hue,
        });
    }
    distinct.map(|high| Left {
        by: (value - high).abs(),
        limit: "high",
        past: value >= high,
        hue,
    })
}

/// How near a temperature is to the most it's meant to reach: its share
/// of that most, or of its part's usual most where the chip says none.
pub fn level(value: f64, high: Option<f64>, crit: Option<f64>, part: &str) -> Level {
    let most = crit.or(high).unwrap_or_else(|| usual_most(part));
    Level {
        value: (value / most).clamp(0.0, 1.0),
        warn: warns_from(high, crit).is_some_and(|from| value >= from),
    }
}

/// A spark's values, from 0 to 1, across the span its reads cover or
/// `least`, whichever is more, centred, so a steady reading draws level
/// halfway up and a climb draws as one.
pub fn spark(values: &[f64], least: f64) -> Vec<f64> {
    let most = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let fewest = values.iter().copied().fold(f64::INFINITY, f64::min);
    let span = least.max(most - fewest);
    let floor = (most + fewest) / 2.0 - span / 2.0;
    values.iter().map(|value| (value - floor) / span).collect()
}

/// A temperature's spark.
pub fn heat_spark(values: &[f64]) -> Vec<f64> {
    spark(values, SPAN)
}

/// A fan's spark.
pub fn fan_spark(values: &[f64]) -> Vec<f64> {
    spark(values, FAN_SPAN)
}

/// Each logical processor's core, from the processor record.
pub fn cpus_of(processor: &Value) -> Vec<(u64, u64)> {
    list(&processor["cores"])
        .iter()
        .filter_map(|each| Some((whole(&each["n"])?, whole(&each["core"])?)))
        .collect()
}

/// Each core's logical processor, by the core's number as its chip labels
/// it, so Heat's Core 1 is the one Machine's Cores call Core 1: the lowest
/// of the processors that share it.
pub fn core_cpus(cpus: &[(u64, u64)]) -> BTreeMap<u64, u64> {
    let mut out = BTreeMap::new();
    for &(cpu, core) in cpus {
        out.entry(core)
            .and_modify(|least: &mut u64| *least = (*least).min(cpu))
            .or_insert(cpu);
    }
    out
}

/// How often the processor has slowed itself to cool down, from the
/// processor record, or none where the machine counts none.
pub fn throttled_of(processor: &Value) -> Option<Throttled> {
    let said = &processor["throttled"];
    if !said.is_object() {
        return None;
    }
    let mut cores: Vec<(u64, u64)> = list(&said["cores"])
        .iter()
        .filter_map(|each| Some((whole(&each["cpu"])?, whole(&each["count"]).unwrap_or(0))))
        .collect();
    cores.sort_unstable();
    Some(Throttled {
        times: whole(&said["package_count"]).unwrap_or(0),
        ms: whole(&said["package_ms"]),
        longest: whole(&said["package_longest_ms"]),
        total: whole(&said["cores_count"])
            .unwrap_or_else(|| cores.iter().map(|(_, count)| count).sum()),
        cores,
    })
}

/// The part a sensor stands for by its label, or "".
pub fn labelled(label: &str) -> &'static str {
    LABELLED
        .iter()
        .find(|(said, _)| *said == label)
        .map_or("", |(_, part)| part)
}

/// Whether any chip gives a part's temperature, so a thermal zone isn't
/// read for it.
pub fn gives(chips: &[Chip], part: &str) -> bool {
    chips.iter().any(|chip| {
        !chip.temps.is_empty()
            && (chip.part == part
                || (chip.part.is_empty() && chip.temps.iter().any(|t| labelled(&t.label) == part)))
    })
}

/// A fan's id within Heat's words, by its label: "Video Fan" is
/// video-fan, and one unlabelled is fan.
pub fn fan_id(label: &str) -> String {
    let words: Vec<String> = label.split_whitespace().map(str::to_lowercase).collect();
    if words.is_empty() {
        "fan".to_owned()
    } else {
        words.join("-")
    }
}

/// A part's name, a title, as a sentence says it after "the": "Air
/// Inside" is "air inside", while a word with more capitals, as "Wi-Fi"
/// in "Wi-Fi Card", keeps them.
pub fn phrase(name: &str) -> String {
    let plain = |word: &str| {
        let mut chars = word.chars();
        chars.next().is_some_and(|c| c.is_ascii_uppercase())
            && chars.all(|c| c.is_ascii_lowercase())
    };
    name.split(' ')
        .map(|word| {
            if plain(word) {
                word.to_lowercase()
            } else {
                word.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_core(label: &str) -> Option<u64> {
    label.strip_prefix("Core ")?.parse().ok()
}

fn names_whole(label: &str) -> bool {
    ["Composite", "Package", "Tctl", "Tdie"]
        .iter()
        .any(|start| label.starts_with(start))
}

fn hottest<'a>(temps: impl Iterator<Item = &'a Temp>) -> Option<&'a Temp> {
    temps.reduce(|a, b| if a.value >= b.value { a } else { b })
}

/// The processor's own temperature among its chip's sensors: the
/// package's or the die's, else its hottest core's, else its first.
pub fn processor(temps: &[Temp]) -> Option<&Temp> {
    let whole = ["Package", "Tctl", "Tdie"];
    temps
        .iter()
        .find(|t| whole.iter().any(|start| t.label.starts_with(start)))
        .or_else(|| hottest(temps.iter().filter(|t| t.label.starts_with("Core"))))
        .or_else(|| temps.first())
}

fn entry(id: &str, whole: &Temp, sensors: Vec<Sensor>, cores: Vec<Core>) -> Part {
    Part {
        id: id.to_owned(),
        value: whole.value,
        high: whole.high,
        crit: whole.crit,
        low: whole.low,
        emergency: whole.emergency,
        alarm: whole.alarm,
        chip: whole.chip.clone(),
        electric: None,
        sensors,
        cores,
        nth: 0,
    }
}

fn reading(value: f64, high: Option<f64>, crit: Option<f64>) -> Temp {
    Temp {
        n: 0,
        label: String::new(),
        value,
        high,
        crit,
        low: None,
        emergency: None,
        alarm: false,
        chip: String::new(),
    }
}

/// The parts one chip gives: the processor's with its cores, one whole
/// reading of a chip that measures one part with its other sensors
/// beside, or each named sensor of one that measures many.
fn chip_parts(chip: &Chip, cpu_of_core: &BTreeMap<u64, u64>) -> Vec<Part> {
    if chip.temps.is_empty() {
        return Vec::new();
    }
    if chip.part == "processor" {
        let Some(whole) = processor(&chip.temps) else {
            return Vec::new();
        };
        let mut cores: Vec<Core> = chip
            .temps
            .iter()
            .filter_map(|t| {
                let n = is_core(&t.label)?;
                Some(Core {
                    cpu: cpu_of_core.get(&n).copied().unwrap_or(n),
                    value: t.value,
                })
            })
            .collect();
        cores.sort_by_key(|core| core.cpu);
        let mut part = entry("processor", whole, Vec::new(), cores);
        part.chip.clone_from(&chip.chip);
        part.electric = Some(chip.electric.clone());
        return vec![part];
    }
    if chip.part.is_empty() {
        return chip
            .temps
            .iter()
            .filter(|t| !labelled(&t.label).is_empty())
            .map(|t| entry(labelled(&t.label), t, Vec::new(), Vec::new()))
            .collect();
    }
    let found = chip.temps.iter().find(|t| names_whole(&t.label));
    let Some(whole) = found.or_else(|| hottest(chip.temps.iter())) else {
        return Vec::new();
    };
    let sensors = chip
        .temps
        .iter()
        .filter(|t| !std::ptr::eq(*t, whole))
        .map(|t| Sensor {
            n: t.n,
            label: t.label.clone(),
            value: t.value,
        })
        .collect();
    let mut part = entry(&chip.part, whole, sensors, Vec::new());
    part.electric = Some(chip.electric.clone());
    vec![part]
}

/// The parts in their order, then as the chips were listed, so two disks
/// keep theirs; a part said twice is numbered.
fn placed(mut parts: Vec<Part>) -> Vec<Part> {
    let rank = |id: &str| {
        ORDER
            .iter()
            .position(|each| *each == id)
            .unwrap_or(ORDER.len())
    };
    parts.sort_by_key(|part| rank(&part.id));
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for part in &parts {
        *counts.entry(part.id.clone()).or_default() += 1;
    }
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for part in &mut parts {
        let at = seen.entry(part.id.clone()).or_default();
        *at += 1;
        part.nth = if counts[&part.id] > 1 { *at } else { 0 };
    }
    parts
}

/// The processor's own parts, from its telemetry's temperatures, each
/// `{ part, celsius }`: those Heat lists, held to the processor's high and
/// critical points, as they sit on the same die.
fn own_parts(telemetry: &[Value], processor: Option<&Part>) -> Vec<Part> {
    telemetry
        .iter()
        .filter_map(|each| {
            let said = text(&each["part"]);
            let (_, id) = TELEMETRY.iter().find(|(named, _)| *named == said)?;
            let held = reading(
                number(&each["celsius"])?,
                processor.and_then(|p| p.high),
                processor.and_then(|p| p.crit),
            );
            Some(entry(id, &held, Vec::new(), Vec::new()))
        })
        .collect()
}

/// The parts and fans, from the chips, the zones (the first of a part no
/// chip gives added), each core's processor and the processor's
/// telemetry.
pub fn gather(
    chips: &[Chip],
    zones: &[Zone],
    cpu_of_core: &BTreeMap<u64, u64>,
    telemetry: &[Value],
) -> Gathered {
    let mut temps: Vec<Part> = chips
        .iter()
        .flat_map(|chip| chip_parts(chip, cpu_of_core))
        .collect();
    let mut added: Vec<Part> = Vec::new();
    for zone in zones {
        let taken = temps.iter().chain(&added).any(|part| part.id == zone.part);
        if !zone.part.is_empty() && !taken {
            added.push(entry(
                &zone.part,
                &reading(zone.value, None, None),
                Vec::new(),
                Vec::new(),
            ));
        }
    }
    temps.extend(added);
    let own = own_parts(telemetry, temps.iter().find(|part| part.id == "processor"));
    temps.extend(own);
    Gathered {
        temps: placed(temps),
        fans: fans(chips),
    }
}

/// A named fan, with what the chip that doesn't name it says.
fn told_of(mut made: Fan, other: Option<&Fan>) -> Fan {
    if let Some(other) = other {
        made.max = made.max.or(other.max);
        made.min = made.min.or(other.min);
        made.target = made.target.or(other.target);
        made.power = made.power.or(other.power);
        made.mode = made.mode.or_else(|| other.mode.clone());
    }
    made
}

/// Two chips can report the same fans, one naming them and one not, as
/// dell_ddv and dell_smm do: the named are the fans, and an unnamed chip
/// with as many says, fan by fan in order, their most, their aim and
/// their power. Where none are named, the first chip's unnamed fans are.
pub fn fans(chips: &[Chip]) -> Vec<Fan> {
    let all: Vec<&Fan> = chips.iter().flat_map(|chip| &chip.fans).collect();
    let mut named: Vec<&Fan> = Vec::new();
    for fan in all.iter().filter(|fan| !fan.label.is_empty()) {
        if !named.iter().any(|other| other.label == fan.label) {
            named.push(fan);
        }
    }
    let unnamed: Vec<Vec<&Fan>> = chips
        .iter()
        .map(|chip| {
            chip.fans
                .iter()
                .filter(|fan| fan.label.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|list| !list.is_empty())
        .collect();
    if named.is_empty() {
        return unnamed
            .first()
            .map_or_else(Vec::new, |list| list.iter().map(|f| (*f).clone()).collect());
    }
    let told = unnamed.iter().find(|list| list.len() == named.len());
    named
        .iter()
        .enumerate()
        .map(|(i, fan)| told_of((*fan).clone(), told.and_then(|list| list.get(i).copied())))
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn temp(n: u64, label: &str, value: f64, high: Option<f64>, crit: Option<f64>) -> Temp {
        Temp {
            n,
            label: label.to_owned(),
            value,
            high,
            crit,
            low: None,
            emergency: None,
            alarm: false,
            chip: String::new(),
        }
    }

    fn fan(n: u64, label: &str, value: f64) -> Fan {
        Fan {
            id: fan_id(label),
            n,
            label: label.to_owned(),
            value,
            max: None,
            min: None,
            target: None,
            power: None,
            mode: None,
        }
    }

    fn chip(part: &str, temps: Vec<Temp>, fans: Vec<Fan>) -> Chip {
        Chip {
            chip: String::new(),
            part: part.to_owned(),
            temps,
            fans,
            electric: Electric::default(),
        }
    }

    /// The laptop's chips, as chips_of gives them: the disk's with its
    /// hidden second sensor, the processor's, dell_ddv's named sensors
    /// and fans, dell_smm's unnamed fans telling their most and power, and
    /// the battery's.
    fn laptop() -> Vec<Chip> {
        let unnamed = |n, value, power| Fan {
            max: Some(4800.0),
            target: Some(4800.0),
            power: Some(power),
            ..fan(n, "", value)
        };
        vec![
            chip(
                "disk",
                vec![
                    temp(1, "Composite", 52.85, Some(82.85), Some(84.85)),
                    temp(2, "Sensor 1", 53.85, None, None),
                    temp(3, "Sensor 2", 68.85, None, None),
                ],
                vec![],
            ),
            chip(
                "processor",
                vec![
                    temp(1, "Package id 0", 83.0, Some(100.0), Some(100.0)),
                    temp(2, "Core 0", 82.0, Some(100.0), Some(100.0)),
                    temp(6, "Core 8", 84.0, Some(100.0), Some(100.0)),
                    temp(10, "Core 22", 79.0, Some(100.0), Some(100.0)),
                ],
                vec![],
            ),
            chip(
                "",
                vec![
                    temp(1, "CPU", 84.0, None, None),
                    temp(6, "SODIMM", 56.0, None, None),
                    temp(7, "Ambient", 45.0, None, None),
                ],
                vec![fan(1, "Video Fan", 5420.0), fan(2, "CPU Fan", 5385.0)],
            ),
            chip(
                "",
                vec![],
                vec![unnamed(1, 5420.0, 1.0), unnamed(2, 5385.0, 0.5)],
            ),
            chip("battery", vec![temp(1, "temp", 36.9, None, None)], vec![]),
        ]
    }

    fn ids(gathered: &Gathered) -> Vec<&str> {
        gathered.temps.iter().map(|part| part.id.as_str()).collect()
    }

    fn laptop_cores() -> BTreeMap<u64, u64> {
        core_cpus(&[(0, 0), (1, 22), (8, 8)])
    }

    #[test]
    fn the_parts_come_the_processor_first() {
        let parts = gather(&laptop(), &[], &laptop_cores(), &[]);
        assert_eq!(
            ids(&parts),
            ["processor", "disk", "memory", "battery", "air"]
        );
        let processor = &parts.temps[0];
        assert_eq!(
            (processor.value, processor.high, processor.crit),
            (83.0, Some(100.0), Some(100.0))
        );
        let cores: Vec<(u64, f64)> = processor.cores.iter().map(|c| (c.cpu, c.value)).collect();
        assert_eq!(cores, [(0, 82.0), (1, 79.0), (8, 84.0)]);
        let disk = &parts.temps[1];
        assert_eq!(
            (disk.value, disk.high, disk.crit),
            (52.85, Some(82.85), Some(84.85))
        );
        let others: Vec<(&str, f64)> = disk
            .sensors
            .iter()
            .map(|s| (s.label.as_str(), s.value))
            .collect();
        assert_eq!(others, [("Sensor 1", 53.85), ("Sensor 2", 68.85)]);
        assert_eq!(disk.nth, 0);
    }

    #[test]
    fn the_fans_are_named_and_told_by_the_unnamed_chip() {
        let fans = fans(&laptop());
        let said: Vec<(&str, u64, Option<f64>, Option<f64>)> = fans
            .iter()
            .map(|f| (f.id.as_str(), f.n, f.max, f.power))
            .collect();
        assert_eq!(
            said,
            [
                ("video-fan", 1, Some(4800.0), Some(1.0)),
                ("cpu-fan", 2, Some(4800.0), Some(0.5))
            ]
        );
        let unnamed: Vec<(String, u64)> = super::fans(&laptop()[3..4])
            .into_iter()
            .map(|f| (f.id, f.n))
            .collect();
        assert_eq!(unnamed, [("fan".to_owned(), 1), ("fan".to_owned(), 2)]);
        assert_eq!(super::fans(&laptop()[0..1]), []);
    }

    #[test]
    fn a_zone_stands_only_for_a_part_no_chip_gives() {
        let zone = Zone {
            part: "processor".to_owned(),
            kind: "TCPU".to_owned(),
            value: 78.0,
            trips: vec![],
        };
        let lone = gather(
            &laptop()[4..5],
            std::slice::from_ref(&zone),
            &BTreeMap::new(),
            &[],
        );
        assert_eq!(ids(&lone), ["processor", "battery"]);
        let all = gather(&laptop(), &[zone], &BTreeMap::new(), &[]);
        assert_eq!(all.temps[0].value, 83.0);
        assert_eq!(
            (gives(&laptop(), "air"), gives(&laptop(), "wifi")),
            (true, false)
        );
        let parts: Vec<&str> = [
            "x86_pkg_temp",
            "acpitz",
            "iwlwifi_1",
            "SEN1",
            "INT3400 Thermal",
        ]
        .map(zone_part)
        .to_vec();
        assert_eq!(parts, ["processor", "board", "wifi", "", ""]);
    }

    #[test]
    fn the_processors_own_parts_follow_it_held_to_its_points() {
        let own = [
            json!({ "part": "chipset", "celsius": 62 }),
            json!({ "part": "npu", "celsius": 62 }),
            json!({ "part": "display", "celsius": 60 }),
            json!({ "part": "imageProcessor", "celsius": 63 }),
            json!({ "part": "systemAgent", "celsius": 65 }),
            json!({ "part": "media", "celsius": 59 }),
        ];
        let parts = gather(&laptop(), &[], &BTreeMap::new(), &own);
        let said: Vec<(&str, f64)> = parts
            .temps
            .iter()
            .map(|p| (p.id.as_str(), p.value))
            .collect();
        assert_eq!(
            said,
            [
                ("processor", 83.0),
                ("npu", 62.0),
                ("image-processor", 63.0),
                ("media-engine", 59.0),
                ("display-engine", 60.0),
                ("disk", 52.85),
                ("memory", 56.0),
                ("battery", 36.9),
                ("air", 45.0),
            ]
        );
        assert_eq!(
            (parts.temps[1].high, parts.temps[1].crit),
            (Some(100.0), Some(100.0))
        );
        let alone = gather(&laptop()[4..5], &[], &BTreeMap::new(), &own[1..2]);
        let held: Vec<(&str, Option<f64>)> = alone
            .temps
            .iter()
            .map(|p| (p.id.as_str(), p.crit))
            .collect();
        assert_eq!(held, [("npu", None), ("battery", None)]);
        assert_eq!(gather(&laptop(), &[], &BTreeMap::new(), &[]).temps.len(), 5);
    }

    #[test]
    fn a_part_said_twice_is_numbered() {
        let disks = [laptop()[0].clone(), laptop()[0].clone()];
        let nths: Vec<usize> = gather(&disks, &[], &BTreeMap::new(), &[])
            .temps
            .iter()
            .map(|p| p.nth)
            .collect();
        assert_eq!(nths, [1, 2]);
    }

    #[test]
    fn a_level_is_its_share_of_the_most() {
        let hundred = Some(100.0);
        assert_eq!(
            level(83.0, hundred, hundred, "processor"),
            Level {
                value: 0.83,
                warn: false
            }
        );
        assert!(
            level(92.0, hundred, hundred, "processor").warn,
            "a processor near its most warns"
        );
        assert!(
            level(83.0, Some(82.85), Some(84.85), "disk").warn,
            "a disk past its high warns"
        );
        assert!(
            !level(80.0, Some(82.85), Some(84.85), "disk").warn,
            "a disk below its high doesn't"
        );
        assert_eq!(
            level(30.0, None, None, "battery"),
            Level {
                value: 0.5,
                warn: false
            }
        );
        assert_eq!(level(110.0, None, hundred, "processor").value, 1.0);
    }

    #[test]
    fn a_spark_is_drawn_across_its_span() {
        assert_eq!(heat_spark(&[83.0, 83.0]), [0.5, 0.5]);
        assert_eq!(heat_spark(&[80.0, 85.0, 90.0]), [0.0, 0.5, 1.0]);
        assert_eq!(heat_spark(&[]), Vec::<f64>::new());
        assert_eq!(fan_spark(&[5000.0, 5250.0]), [0.25, 0.75]);
    }

    #[test]
    fn throttling_is_the_packages_and_each_cores_once() {
        let held = throttled_of(&json!({ "throttled": {
            "cores": [{ "core": 22, "cpu": 1, "count": 8 }, { "core": 0, "cpu": 0, "count": 24 }],
            "cores_count": 32, "package_count": 3202, "package_ms": 19016,
            "package_longest_ms": 125
        } }));
        assert_eq!(
            held,
            Some(Throttled {
                times: 3202,
                ms: Some(19016),
                longest: Some(125),
                cores: vec![(0, 24), (1, 8)],
                total: 32,
            })
        );
        assert_eq!(throttled_of(&json!({ "throttled": null })), None);
    }

    /// The chips as this daemon holds the laptop's: the fans' own chip
    /// and a USB-C port's stand for no part, so only labels name their
    /// sensors, and an unlabelled fan is said, one unread left out.
    #[test]
    fn the_daemons_chips_come_by_key_each_for_its_part() {
        let held = chips_of(&[
            json!({ "key": "sensor:nvme", "part": "disk", "temperatures": [
                { "n": 1, "label": "Composite", "temperature_c": 52.9, "high_c": 82.9,
                    "critical_c": 84.9 },
                { "n": 2, "label": "Sensor 9", "temperature_c": 65261.8, "high_c": null }],
                "fans": [] }),
            json!({ "key": "sensor:dell_smm", "part": "fans", "temperatures": [
                { "n": 1, "label": "CPU", "temperature_c": 61 },
                { "n": 2, "label": "Ambient", "temperature_c": 40, "critical_c": -273 }],
                "fans": [
                { "n": 1, "label": null, "rpm": 2400, "max_rpm": 5900, "min_rpm": 1200,
                    "target_rpm": 2400, "power_ratio": 0.502, "mode": "automatic" },
                { "n": 2, "label": null, "rpm": null }] }),
            json!({ "key": "sensor:coretemp", "part": "processor", "chip": "coretemp",
                "temperatures": [{ "n": 1, "label": "Package id 0", "temperature_c": 79,
                    "high_c": 100, "critical_c": 100, "low_c": -273, "emergency_c": 105,
                    "alarm": true }], "fans": [],
                "voltages": [{ "n": 0, "label": null, "volts": 1.21 },
                    { "n": 1, "label": "Vcore", "volts": null }],
                "currents": [], "power": [{ "n": 1, "label": "Package", "watts": 12.5 }] }),
        ]);
        let parts: Vec<&str> = held.iter().map(|chip| chip.part.as_str()).collect();
        assert_eq!(parts, ["processor", "", "disk"]);
        assert_eq!(held[1].temps, [temp(2, "Ambient", 40.0, None, None)]);
        let processor = &held[0].temps[0];
        assert_eq!(
            (
                processor.low,
                processor.emergency,
                processor.alarm,
                processor.chip.as_str()
            ),
            (None, Some(105.0), true, "coretemp")
        );
        let figure = |n, label: &str, value| Figure {
            n,
            label: label.to_owned(),
            value,
        };
        assert_eq!(
            held[0].electric,
            Electric {
                volts: vec![figure(0, "", 1.21)],
                amps: vec![],
                watts: vec![figure(1, "Package", 12.5)],
            }
        );
        let labels: Vec<&str> = held[2].temps.iter().map(|t| t.label.as_str()).collect();
        assert_eq!(labels, ["Composite"]);
        assert_eq!(
            held[1].fans,
            [Fan {
                max: Some(5900.0),
                min: Some(1200.0),
                target: Some(2400.0),
                power: Some(0.502),
                mode: Some("automatic".to_owned()),
                ..fan(1, "", 2400.0)
            }]
        );
    }

    #[test]
    fn the_daemons_zones_keep_their_trips_within_reach() {
        let zones = zones_of(&[
            json!({ "type": "TCPU", "temperature_c": 78.5, "trips": [
                { "type": "critical", "temperature_c": 110 },
                { "type": "passive", "temperature_c": 95 },
                { "type": "active", "temperature_c": -273 }] }),
            json!({ "type": "x86_pkg_temp", "temperature_c": 79, "trips": [
                { "type": "passive", "temperature_c": 95 },
                { "type": "hot", "temperature_c": 105 }] }),
            json!({ "type": "SEN1", "temperature_c": 40 }),
            json!({ "type": "acpitz", "temperature_c": null }),
        ]);
        let said: Vec<(&str, f64, usize)> = zones
            .iter()
            .map(|z| (z.kind.as_str(), z.value, z.trips.len()))
            .collect();
        assert_eq!(said, [("TCPU", 78.5, 2), ("x86_pkg_temp", 79.0, 2)]);
        let trips: Vec<(String, f64, String)> = trips_for(&zones, "processor")
            .into_iter()
            .map(|t| (t.kind, t.value, t.zone))
            .collect();
        let trip = |kind: &str, value, zone: &str| (kind.to_owned(), value, zone.to_owned());
        assert_eq!(
            trips,
            [
                trip("passive", 95.0, "TCPU"),
                trip("hot", 105.0, "x86_pkg_temp"),
                trip("critical", 110.0, "TCPU"),
            ]
        );
        assert_eq!(trips_for(&zones, "disk"), []);
    }

    #[test]
    fn each_kind_of_cooling_is_stepped_as_far_as_any() {
        let cooling = cooling_of(&[
            json!({ "type": "Processor", "count": 16, "states": [0, 2, 1], "max_state": 3 }),
            json!({ "type": "iwlwifi", "count": 1, "states": [0], "max_state": 19 }),
            json!({ "type": "LCD", "count": 1, "states": [0], "max_state": 0 }),
        ]);
        let said: Vec<(&str, u64, u64, u64)> = cooling
            .iter()
            .map(|c| (c.kind.as_str(), c.count, c.state, c.max))
            .collect();
        assert_eq!(said, [("iwlwifi", 1, 0, 19), ("Processor", 16, 2, 3)]);
    }

    #[test]
    fn whats_left_to_the_limit() {
        let left = |by, limit, past, hue| {
            Some(Left {
                by,
                limit,
                past,
                hue,
            })
        };
        assert_eq!(
            left_to(71.0, Some(100.0), Some(110.0)),
            left(29.0, "high", false, "")
        );
        assert_eq!(
            left_to(102.0, Some(100.0), Some(110.0)),
            left(8.0, "critical", false, "warning")
        );
        assert_eq!(
            left_to(112.0, Some(100.0), Some(110.0)),
            left(2.0, "critical", true, "alert")
        );
        assert_eq!(
            left_to(85.0, Some(100.0), Some(100.0)),
            left(15.0, "critical", false, "")
        );
        assert_eq!(
            left_to(92.0, Some(100.0), Some(100.0)),
            left(8.0, "critical", false, "warning")
        );
        assert_eq!(
            left_to(83.0, Some(80.0), None),
            left(3.0, "high", true, "warning")
        );
        assert_eq!(left_to(50.0, None, None), None);
    }

    #[test]
    fn each_core_by_its_first_processor() {
        let cpus = cpus_of(&json!({ "cores": [
            { "n": 0, "core": 0 }, { "n": 1, "core": 0 }, { "n": 2, "core": null }] }));
        assert_eq!(cpus, [(0, 0), (1, 0)]);
        let firsts = core_cpus(&[(0, 0), (1, 0), (2, 4)]);
        assert_eq!(firsts, BTreeMap::from([(0, 0), (4, 2)]));
    }

    #[test]
    fn a_parts_name_in_a_sentence() {
        let said = ["Processor", "Wi-Fi Card", "Air Inside", "HDR"].map(phrase);
        assert_eq!(said, ["processor", "Wi-Fi card", "air inside", "HDR"]);
    }

    #[test]
    fn the_processors_own_temperature() {
        let core = |n, value| temp(n, &format!("Core {n}"), value, None, None);
        let temps = [
            core(0, 70.0),
            core(8, 84.0),
            temp(9, "Package id 0", 80.0, None, None),
        ];
        assert_eq!(processor(&temps).map(|t| t.n), Some(9));
        assert_eq!(
            processor(&temps[..2]).map(|t| t.label.as_str()),
            Some("Core 8")
        );
        assert_eq!(
            processor(&[temp(1, "", 1.0, None, None)]).map(|t| t.value),
            Some(1.0)
        );
    }

    #[test]
    fn names_in_order_numbers_by_worth() {
        let mut names = ["sensor:nvme", "Core 10", "core 9", "sensor:coretemp"];
        names.sort_by(|a, b| by_name(a, b));
        assert_eq!(
            names,
            ["core 9", "Core 10", "sensor:coretemp", "sensor:nvme"]
        );
    }
}
