//! What the machine's sensors say: each hwmon chip's temperatures, fans,
//! voltages, currents and power (`sensor:<chip>`), each thermal zone and
//! its trip points (`zone:<type>`), each kind of cooling device and how
//! hard it's working (`cooling:<type>`), and how often the processor has
//! slowed itself to cool down. What a chip says of itself that never
//! changes, its labels and limits, is read once and kept until the chips
//! listed change, since some chips answer every read through the firmware,
//! which is slow.

use std::collections::{BTreeMap, HashMap};

use serde_json::{Value, json};

use crate::{Root, rounded};

const HWMON: &str = "/sys/class/hwmon";
const THERMAL: &str = "/sys/class/thermal";

/// A chip's files that never change, kept by path, and the chips they
/// were kept for.
#[derive(Default)]
pub struct Kept {
    chips: Vec<String>,
    files: HashMap<String, Option<String>>,
}

impl Kept {
    /// What a file that never changes says, read the first time it's asked.
    fn said(&mut self, root: &Root, path: &str) -> Option<String> {
        self.files
            .entry(path.to_owned())
            .or_insert_with(|| root.said(path))
            .clone()
    }

    /// Forgets what was kept once the chips listed change, since the
    /// number an unplugged chip leaves can go to another.
    fn chips(&mut self, root: &Root) -> Vec<String> {
        let chips = root.names(HWMON);
        if chips != self.chips {
            self.files.clear();
            self.chips.clone_from(&chips);
        }
        chips
    }
}

/// What a chip measures, by the name the kernel gives it, or "" for one
/// that measures many, whose sensors are named by their labels.
pub fn part(name: &str) -> &'static str {
    let starts = |prefixes: &[&str]| prefixes.iter().any(|p| name.starts_with(p));
    if ["coretemp", "k10temp", "zenpower"].contains(&name) {
        "processor"
    } else if starts(&["nvme", "drivetemp"]) {
        "disk"
    } else if starts(&["iwlwifi", "mt79", "ath1"]) {
        "wifi"
    } else if name == "acpitz" {
        "board"
    } else if starts(&["pch_"]) {
        "chipset"
    } else if ["amdgpu", "xe", "i915", "nouveau"].contains(&name) {
        "graphics"
    } else if starts(&["BAT"]) || name.contains("battery") {
        "battery"
    } else if starts(&["ucsi_source_psy"]) {
        "port"
    } else if name == "AC" || name == "ADP1" {
        "charger"
    } else if name.starts_with("dell_smm") || name.contains("fan") {
        "fans"
    } else {
        ""
    }
}

/// A reading in thousandths, as the kernel gives degrees, volts and amps,
/// or none where it's out of reach of any real one: an NVMe disk without
/// a threshold says 65261.85 °C, an unset trip point absolute zero.
fn thousandths(text: Option<String>, reach: std::ops::RangeInclusive<i64>) -> Option<f64> {
    let milli = text?.parse::<i64>().ok().filter(|m| reach.contains(m))?;
    i32::try_from(milli).ok().map(|m| f64::from(m) / 1000.0)
}

const DEGREES: std::ops::RangeInclusive<i64> = -100_000..=200_000;

/// Each sensor of a kind a chip's folder lists, by its number.
fn numbered(files: &[String], kind: &str) -> Vec<u32> {
    let mut found: Vec<u32> = files
        .iter()
        .filter_map(|file| {
            file.strip_prefix(kind)?
                .strip_suffix("_input")?
                .parse()
                .ok()
        })
        .collect();
    found.sort_unstable();
    found
}

/// Each sensor's label, by its kind and number, as its chip says it.
type Labels = [(String, u32, Option<String>)];

fn label_in(labels: &Labels, kind: &str, n: u32) -> Option<String> {
    labels
        .iter()
        .find(|(k, m, _)| k == kind && *m == n)
        .and_then(|(_, _, l)| l.clone())
}

/// A chip's sensors of one electric kind (`in`, `curr` or `power`), each
/// figure in `unit`, scaled from the thousandths or millionths the chip
/// says it in.
fn electric(
    root: &Root,
    folder: &str,
    files: &[String],
    labels: &Labels,
    (kind, unit, scale): (&str, &str, f64),
) -> Vec<Value> {
    let value = |file: String| {
        root.said(&format!("{folder}/{file}"))
            .and_then(|v| v.parse::<i64>().ok())
            .and_then(|v| i32::try_from(v).ok())
            .map(|v| rounded(f64::from(v) / scale, 3))
    };
    numbered(files, kind)
        .into_iter()
        .map(|n| {
            let mut out = serde_json::Map::new();
            out.insert("n".into(), json!(n));
            out.insert("label".into(), json!(label_in(labels, kind, n)));
            out.insert(unit.into(), value(format!("{kind}{n}_input")).into());
            out.insert(
                format!("min_{unit}"),
                value(format!("{kind}{n}_min")).into(),
            );
            out.insert(
                format!("max_{unit}"),
                value(format!("{kind}{n}_max")).into(),
            );
            Value::Object(out)
        })
        .collect()
}

/// One chip's record, its sensors' moving figures read now and their
/// labels and limits as kept.
fn chip(root: &Root, kept: &mut Kept, folder: &str, key: &str, name: &str) -> Value {
    let files = root.names(folder);
    let lists = |file: &str| files.iter().any(|f| f == file);
    // A sensor its chip doesn't label is null, never a name made up for
    // it, so the shell can tell a fan two chips both report by the one
    // that names it.
    let mut label = |kind: &str, n: u32| kept.said(root, &format!("{folder}/{kind}{n}_label"));
    let labels: Vec<(String, u32, Option<String>)> = ["temp", "fan", "in", "curr", "power"]
        .iter()
        .flat_map(|kind| numbered(&files, kind).into_iter().map(move |n| (*kind, n)))
        .map(|(kind, n)| (kind.to_owned(), n, label(kind, n)))
        .collect();
    let label_of = |kind: &str, n: u32| label_in(&labels, kind, n);
    let now = |file: String| root.said(&format!("{folder}/{file}"));
    let mut fixed = |file: String| kept.said(root, &format!("{folder}/{file}"));
    let temperatures: Vec<Value> = numbered(&files, "temp")
        .into_iter()
        .map(|n| {
            let limit = |what: &str, fixed: &mut dyn FnMut(String) -> Option<String>| {
                thousandths(fixed(format!("temp{n}_{what}")), DEGREES).map(|c| rounded(c, 1))
            };
            let alarms: Vec<(String, Option<String>)> =
                ["alarm", "crit_alarm", "max_alarm", "min_alarm"]
                    .iter()
                    .map(|what| format!("temp{n}_{what}"))
                    .filter(|file| lists(file))
                    .map(|file| (file.clone(), now(file)))
                    .collect();
            let alarm = alarms
                .iter()
                .map(|(_, said)| said.as_deref() == Some("1"))
                .reduce(|a, b| a || b);
            let input = now(format!("temp{n}_input"));
            let degrees = thousandths(input.clone(), DEGREES);
            let files = temp_files(n, input, label_of("temp", n), &mut fixed, alarms);
            json!({
                "files": files,
                "n": n,
                "label": label_of("temp", n),
                "temperature_c": degrees.map(|c| rounded(c, 1)),
                "low_c": limit("min", &mut fixed),
                "high_c": limit("max", &mut fixed),
                "critical_c": limit("crit", &mut fixed),
                "emergency_c": limit("emergency", &mut fixed),
                "alarm": alarm,
            })
        })
        .filter(|sensor| !sensor["temperature_c"].is_null())
        .collect();
    let fans: Vec<Value> = numbered(&files, "fan")
        .into_iter()
        .map(|n| {
            let rpm = |file: String| now(file).and_then(|v| v.parse::<u64>().ok());
            let most = fixed(format!("fan{n}_max")).and_then(|v| v.parse::<u64>().ok());
            let least = fixed(format!("fan{n}_min")).and_then(|v| v.parse::<u64>().ok());
            let power = lists(&format!("pwm{n}"))
                .then(|| now(format!("pwm{n}")).and_then(|v| v.parse::<u32>().ok()))
                .flatten()
                .map(|p| rounded(f64::from(p.min(255)) / 255.0, 3));
            // pwm_enable says 1 where the fan is set by hand, 2 where the
            // chip sets it itself, 0 where it's at full.
            let mode = now(format!("pwm{n}_enable")).map(|m| match m.as_str() {
                "0" => "full",
                "1" => "manual",
                _ => "automatic",
            });
            json!({
                "files": fan_files(n, &now, &mut fixed, &lists),
                "n": n,
                "label": label_of("fan", n),
                "rpm": rpm(format!("fan{n}_input")),
                "min_rpm": least.filter(|r| *r > 0),
                "max_rpm": most.filter(|r| *r > 0),
                "target_rpm": rpm(format!("fan{n}_target")).filter(|r| *r > 0),
                "power_ratio": power,
                "mode": mode,
            })
        })
        .collect();
    let electrics = |kind| electric(root, folder, &files, &labels, kind);
    json!({
        "key": key,
        "kind": "sensor",
        "chip": name,
        "folder": folder,
        "part": part(name),
        "temperatures": temperatures,
        "fans": fans,
        "voltages": electrics(("in", "volts", 1000.0)),
        "currents": electrics(("curr", "amps", 1000.0)),
        "power": electrics(("power", "watts", 1_000_000.0)),
    })
}

/// The files behind a temperature as they say it, for its rows' drawers:
/// what it reads, its label, its limits (as kept), then its alarms.
fn temp_files(
    n: u32,
    input: Option<String>,
    label: Option<String>,
    fixed: &mut dyn FnMut(String) -> Option<String>,
    alarms: Vec<(String, Option<String>)>,
) -> Value {
    let mut files = vec![
        (format!("temp{n}_input"), input),
        (format!("temp{n}_label"), label),
    ];
    for what in ["min", "max", "crit", "emergency"] {
        let file = format!("temp{n}_{what}");
        files.push((file.clone(), fixed(file)));
    }
    files.extend(alarms);
    said_files(files)
}

/// The files behind a fan as they say it: its speed and where it's aimed
/// read now, its least and most as kept, then how it's driven.
fn fan_files(
    n: u32,
    now: &dyn Fn(String) -> Option<String>,
    fixed: &mut dyn FnMut(String) -> Option<String>,
    lists: &dyn Fn(&str) -> bool,
) -> Value {
    let mut files: Vec<(String, Option<String>)> = Vec::new();
    for what in ["input", "min", "max", "target"] {
        let file = format!("fan{n}_{what}");
        let said = if matches!(what, "input" | "target") {
            now(file.clone())
        } else {
            fixed(file.clone())
        };
        files.push((file, said));
    }
    for file in [format!("pwm{n}"), format!("pwm{n}_enable")] {
        let said = lists(&file).then(|| now(file.clone())).flatten();
        files.push((file, said));
    }
    said_files(files)
}

/// Files as `[name, said]` pairs, in the order read, those that said
/// nothing left out.
fn said_files(files: Vec<(String, Option<String>)>) -> Value {
    files
        .into_iter()
        .filter_map(|(name, said)| Some(json!([name, said?])))
        .collect()
}

/// Every chip's record, a second chip of a name numbered after the first.
pub fn sensors(root: &Root, kept: &mut Kept) -> Vec<Value> {
    let mut seen: HashMap<String, u32> = HashMap::new();
    let mut records = Vec::new();
    for hwmon in kept.chips(root) {
        let folder = format!("{HWMON}/{hwmon}");
        let Some(name) = kept.said(root, &format!("{folder}/name")) else {
            continue;
        };
        let count = seen.entry(name.clone()).or_insert(0);
        *count += 1;
        let key = if *count == 1 {
            format!("sensor:{name}")
        } else {
            format!("sensor:{name}:{count}")
        };
        records.push(chip(root, kept, &folder, &key, &name));
    }
    records
}

/// Every thermal zone's record: its temperature, how it's governed, and
/// the trip points where it acts, an unset one left out.
pub fn zones(root: &Root) -> Vec<Value> {
    let mut seen: HashMap<String, u32> = HashMap::new();
    let mut records = Vec::new();
    for zone in root.names(THERMAL) {
        if !zone.starts_with("thermal_zone") {
            continue;
        }
        let folder = format!("{THERMAL}/{zone}");
        let Some(kind) = root.said(&format!("{folder}/type")) else {
            continue;
        };
        let count = seen.entry(kind.clone()).or_insert(0);
        *count += 1;
        let key = if *count == 1 {
            format!("zone:{kind}")
        } else {
            format!("zone:{kind}:{count}")
        };
        let files = root.names(&folder);
        let mut trips: Vec<(u32, Value)> = files
            .iter()
            .filter_map(|file| {
                file.strip_prefix("trip_point_")?
                    .strip_suffix("_temp")?
                    .parse::<u32>()
                    .ok()
            })
            .filter_map(|n| {
                let at = |what: &str| root.said(&format!("{folder}/trip_point_{n}_{what}"));
                let degrees = thousandths(at("temp"), DEGREES)?;
                Some((
                    n,
                    json!({
                        "type": at("type"),
                        "temperature_c": rounded(degrees, 1),
                        "hysteresis_c": thousandths(at("hyst"), DEGREES).map(|c| rounded(c, 1)),
                    }),
                ))
            })
            .collect();
        trips.sort_by_key(|(n, _)| *n);
        let temp = root.said(&format!("{folder}/temp"));
        let degrees = thousandths(temp.clone(), DEGREES);
        records.push(json!({
            "key": key,
            "kind": "zone",
            "folder": folder,
            "files": said_files(vec![("temp".to_owned(), temp)]),
            "type": kind,
            "temperature_c": degrees.map(|c| rounded(c, 1)),
            "policy": root.said(&format!("{folder}/policy")),
            "mode": root.said(&format!("{folder}/mode")),
            "trips": trips.into_iter().map(|(_, trip)| trip).collect::<Vec<_>>(),
        }));
    }
    records
}

/// Each kind of cooling device, alike ones together: how many there are,
/// each one's state now, and the most any can reach.
pub fn cooling(root: &Root) -> Vec<Value> {
    let mut kinds: BTreeMap<String, Vec<(u64, Option<u64>)>> = BTreeMap::new();
    let mut devices: Vec<(u32, String)> = root
        .names(THERMAL)
        .into_iter()
        .filter_map(|name| {
            let n = name.strip_prefix("cooling_device")?.parse().ok()?;
            Some((n, name))
        })
        .collect();
    devices.sort_by_key(|(n, _)| *n);
    for (_, device) in devices {
        let folder = format!("{THERMAL}/{device}");
        let Some(kind) = root.said(&format!("{folder}/type")) else {
            continue;
        };
        let state = root.number(&format!("{folder}/cur_state")).unwrap_or(0);
        kinds
            .entry(kind)
            .or_default()
            .push((state, root.number(&format!("{folder}/max_state"))));
    }
    kinds
        .into_iter()
        .map(|(kind, each)| {
            json!({
                "key": format!("cooling:{kind}"),
                "kind": "cooling",
                "type": kind,
                "count": each.len(),
                "states": each.iter().map(|(state, _)| state).collect::<Vec<_>>(),
                "max_state": each.iter().filter_map(|(_, most)| *most).max(),
            })
        })
        .collect()
}

/// How often the processor has slowed itself to cool down since the
/// machine started: each core's count once, from its first processor, and
/// the package's from the processor that saw most; null where the kernel
/// doesn't count them.
pub fn throttled(root: &Root, cpus: &[(u32, Option<u32>)]) -> Value {
    let at = |n: u32, file: &str| {
        root.number(&format!(
            "/sys/devices/system/cpu/cpu{n}/thermal_throttle/{file}"
        ))
    };
    let Some(first) = cpus.first() else {
        return Value::Null;
    };
    if at(first.0, "package_throttle_count").is_none() {
        return Value::Null;
    }
    let mut cores: BTreeMap<u32, u32> = BTreeMap::new();
    for (n, core) in cpus {
        let core = core.unwrap_or(*n);
        let entry = cores.entry(core).or_insert(*n);
        *entry = (*entry).min(*n);
    }
    let sum = |file: &str| -> u64 { cores.values().filter_map(|n| at(*n, file)).sum() };
    let most = |file: &str| -> Option<u64> { cores.values().filter_map(|n| at(*n, file)).max() };
    let package = cpus
        .iter()
        .max_by_key(|(n, _)| at(*n, "package_throttle_count").unwrap_or(0))
        .map_or(first.0, |(n, _)| *n);
    // Each core by its first processor, as each processor sharing a core
    // counts the same slowing, so Heat can say which core slowed most.
    let each: Vec<Value> = cores
        .iter()
        .map(|(core, n)| {
            json!({
                "core": core,
                "cpu": n,
                "count": at(*n, "core_throttle_count"),
                "ms": at(*n, "core_throttle_total_time_ms"),
                "longest_ms": at(*n, "core_throttle_max_time_ms"),
            })
        })
        .collect();
    json!({
        "cores": each,
        "cores_count": sum("core_throttle_count"),
        "cores_ms": sum("core_throttle_total_time_ms"),
        "cores_longest_ms": most("core_throttle_max_time_ms"),
        "package_cpu": package,
        "package_count": at(package, "package_throttle_count"),
        "package_ms": at(package, "package_throttle_total_time_ms"),
        "package_longest_ms": at(package, "package_throttle_max_time_ms"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chip_is_named_for_what_it_measures() {
        assert_eq!(part("coretemp"), "processor");
        assert_eq!(part("nvme"), "disk");
        assert_eq!(part("iwlwifi_1"), "wifi");
        assert_eq!(part("BAT0"), "battery");
        assert_eq!(part("ucsi_source_psy_USBC000:001"), "port");
        assert_eq!(part("dell_smm"), "fans");
        assert_eq!(part("dell_ddv"), "");
    }

    /// A moving file is read once a pass, its drawer's files the figures'
    /// own reads: `dell_smm` answers each read through the firmware, about
    /// 2 ms a file on Joel's laptop, and Heat reads every second it's
    /// shown (2026-11-13).
    #[test]
    fn each_moving_file_is_read_once_a_pass() {
        let folder = std::env::temp_dir().join(format!("machine-heat-{}", std::process::id()));
        let chip = folder.join("sys/class/hwmon/hwmon0");
        std::fs::create_dir_all(&chip).expect("the chip's folder is made");
        let said = [
            ("name", "dell_smm"),
            ("temp1_input", "47000"),
            ("temp1_alarm", "0"),
            ("fan1_input", "2400"),
            ("fan1_target", "2500"),
            ("fan1_min", "0"),
            ("fan1_max", "5000"),
            ("pwm1", "128"),
            ("pwm1_enable", "2"),
        ];
        for (file, says) in said {
            std::fs::write(chip.join(file), says).expect("the chip's file is written");
        }
        let root = Root::new(&folder);
        let records = sensors(&root, &mut Kept::default());
        std::fs::remove_dir_all(&folder).ok();
        let moving = [
            "temp1_input",
            "temp1_alarm",
            "fan1_input",
            "fan1_target",
            "pwm1",
            "pwm1_enable",
        ];
        let not_once: Vec<(&str, usize)> = moving
            .iter()
            .map(|file| (*file, root.reads_of(&format!("{HWMON}/hwmon0/{file}"))))
            .filter(|(_, reads)| *reads != 1)
            .collect();
        assert!(
            not_once.is_empty(),
            "each moving file is read once a pass, these as often as said: {not_once:?}"
        );
        assert_eq!(
            records[0]["fans"][0]["files"],
            json!([
                ["fan1_input", "2400"],
                ["fan1_min", "0"],
                ["fan1_max", "5000"],
                ["fan1_target", "2500"],
                ["pwm1", "128"],
                ["pwm1_enable", "2"],
            ]),
            "the fan's drawer names its files as read"
        );
    }

    #[test]
    fn a_reading_out_of_reach_is_none() {
        assert_eq!(thousandths(Some("65261850".into()), DEGREES), None);
        assert_eq!(thousandths(Some("-274000".into()), DEGREES), None);
        assert_eq!(thousandths(Some("47000".into()), DEGREES), Some(47.0));
    }
}
