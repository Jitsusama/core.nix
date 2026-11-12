//! The processor, `processor:0`: how busy it was between two reads and
//! where its time went, each core's share, clock and temperature, its
//! clock's range, driver and policy, its load, how often it switched
//! tasks, took interrupts and started processes, how long tasks waited for
//! it, its NPU's load, and what it is: its model, family, microcode and
//! caches.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::{Root, cpu_list, float, per_second, rounded, since};

/// The fields of a processor's line in `/proc/stat`, in `USER_HZ` ticks.
pub const TIMES: [&str; 10] = [
    "user",
    "nice",
    "system",
    "idle",
    "iowait",
    "irq",
    "softirq",
    "steal",
    "guest",
    "guest_nice",
];
/// The first eight count all its time; guest time is in user time
/// already, and guest niced in niced.
const COUNTED: usize = 8;
const IDLE: usize = 3;
const IOWAIT: usize = 4;

/// One logical processor, as the kernel numbers it.
#[derive(Clone, Debug, PartialEq)]
pub struct Cpu {
    pub n: u32,
    /// The physical core it runs on, as coretemp names its sensor.
    pub core: Option<u32>,
    /// "performance", "efficient" or "low-power" on a processor of more
    /// than one kind, else none.
    pub kind: Option<&'static str>,
    pub min_khz: Option<u64>,
    pub max_khz: Option<u64>,
    pub base_khz: Option<u64>,
}

/// What doesn't change while the machine runs, read once.
#[derive(Clone, Debug, PartialEq)]
pub struct Fixed {
    pub model: Option<String>,
    pub vendor: Option<String>,
    pub family: Option<u64>,
    pub model_number: Option<u64>,
    pub stepping: Option<u64>,
    pub microcode: Option<String>,
    pub caches: Value,
    pub cpus: Vec<Cpu>,
    pub driver: Option<String>,
    /// Whether it runs inside another machine, as its flags say, so time
    /// stolen by that machine can be other than none.
    pub guest: bool,
}

/// The counters a rate is measured between, from one read.
#[derive(Clone, Debug, PartialEq)]
pub struct Counters {
    pub at_ms: u64,
    pub whole: Vec<u64>,
    pub cores: BTreeMap<u32, Vec<u64>>,
    pub switches: Option<u64>,
    pub interrupts: Option<u64>,
    pub softirqs: Option<u64>,
    pub forks: Option<u64>,
    pub npu_busy_us: Option<u64>,
}

const NPU: &str = "/sys/class/accel/accel0/device";

/// How soon after a read another measures nothing: the kernel counts a
/// processor's time in ticks of 10 ms, so two reads a few apart see no
/// tick pass, and a share of nothing is no figure. Read so soon, the
/// record held from the last read stands, where it would have said the
/// processor's busy share was unknown and its row gone until the next
/// (a watch's first read landing beside the second's tick).
const SOONEST_MS: u64 = 250;

/// Whether a read at `at_ms` comes too soon after `before` to measure.
#[must_use]
pub fn too_soon(before: Option<&Counters>, at_ms: u64) -> bool {
    before.is_some_and(|before| at_ms.saturating_sub(before.at_ms) < SOONEST_MS)
}

/// A processor's model as `/proc/cpuinfo` names it, without its trade
/// marks or the clock some add: "Intel(R) Core(TM) Ultra X7 358H" is an
/// Intel Core Ultra X7 358H.
pub fn model(name: &str) -> String {
    let unmarked = name.split('@').next().unwrap_or(name);
    unmarked
        .replace("(R)", " ")
        .replace("(TM)", " ")
        .replace("(tm)", " ")
        .split_whitespace()
        .filter(|word| *word != "CPU")
        .collect::<Vec<_>>()
        .join(" ")
}

/// The first processor's field in `/proc/cpuinfo`.
fn info<'a>(cpuinfo: &'a str, field: &str) -> Option<&'a str> {
    cpuinfo
        .lines()
        .take_while(|line| !line.trim().is_empty())
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            (name.trim() == field).then(|| value.trim())
        })
}

/// What's fixed about the processors.
pub fn fixed(root: &Root) -> Fixed {
    let cpuinfo = root.read("/proc/cpuinfo").unwrap_or_default();
    let numbers: Vec<u32> = root
        .names("/sys/devices/system/cpu")
        .iter()
        .filter_map(|name| name.strip_prefix("cpu")?.parse().ok())
        .collect();
    let mut numbers = numbers;
    numbers.sort_unstable();
    let freq =
        |n: u32, file: &str| root.number(&format!("/sys/devices/system/cpu/cpu{n}/cpufreq/{file}"));
    let mut cpus: Vec<Cpu> = numbers
        .iter()
        .map(|&n| Cpu {
            n,
            core: root
                .number(&format!("/sys/devices/system/cpu/cpu{n}/topology/core_id"))
                .and_then(|id| u32::try_from(id).ok()),
            kind: None,
            min_khz: freq(n, "cpuinfo_min_freq"),
            max_khz: freq(n, "cpuinfo_max_freq"),
            base_khz: freq(n, "base_frequency"),
        })
        .collect();
    kinds(
        &mut cpus,
        &cpu_list(&root.said("/sys/devices/cpu_core/cpus").unwrap_or_default()),
    );
    let number = |field| info(&cpuinfo, field).and_then(|v| v.parse().ok());
    Fixed {
        model: info(&cpuinfo, "model name").map(model),
        vendor: info(&cpuinfo, "vendor_id").map(str::to_owned),
        family: number("cpu family"),
        model_number: number("model"),
        stepping: number("stepping"),
        microcode: info(&cpuinfo, "microcode").map(str::to_owned),
        caches: caches(root, &numbers),
        guest: info(&cpuinfo, "flags")
            .is_some_and(|flags| flags.split_whitespace().any(|flag| flag == "hypervisor")),
        driver: numbers.first().and_then(|n| {
            root.said(&format!(
                "/sys/devices/system/cpu/cpu{n}/cpufreq/scaling_driver"
            ))
        }),
        cpus,
    }
}

/// Each core's kind, on a processor of more than one: those the kernel
/// calls cores are for performance, and of its smaller ones, those that
/// clock highest are efficient and the rest low-power.
fn kinds(cpus: &mut [Cpu], performance: &[u32]) {
    if performance.is_empty() {
        return;
    }
    let fastest = cpus
        .iter()
        .filter(|cpu| !performance.contains(&cpu.n))
        .filter_map(|cpu| cpu.max_khz)
        .max()
        .unwrap_or(0);
    for cpu in cpus {
        cpu.kind = Some(if performance.contains(&cpu.n) {
            "performance"
        } else if cpu.max_khz.unwrap_or(0) >= fastest {
            "efficient"
        } else {
            "low-power"
        });
    }
}

/// The caches, those alike as one: each level and type of a size, its
/// ways and line, how many there are, the CPUs they serve between them,
/// and how many CPUs share each, smallest level first. A processor of
/// two kinds has two of a level: twelve 48 KiB L1 data caches and four
/// of 32 KiB, say.
fn caches(root: &Root, numbers: &[u32]) -> Value {
    let mut found: BTreeMap<(u64, String, String), Value> = BTreeMap::new();
    for n in numbers {
        let folder = format!("/sys/devices/system/cpu/cpu{n}/cache");
        for index in root
            .names(&folder)
            .iter()
            .filter(|name| name.starts_with("index"))
        {
            let at = |file: &str| root.said(&format!("{folder}/{index}/{file}"));
            let (Some(level), Some(kind)) =
                (at("level").and_then(|l| l.parse::<u64>().ok()), at("type"))
            else {
                continue;
            };
            let shared = at("shared_cpu_list").unwrap_or_default();
            let id = at("id").unwrap_or_else(|| shared.clone());
            found.entry((level, kind.clone(), id)).or_insert_with(|| {
                json!({
                    "level": level,
                    "type": kind.to_lowercase(),
                    "size_b": at("size").and_then(|size| bytes(&size)),
                    "ways": at("ways_of_associativity").and_then(|w| w.parse::<u64>().ok()),
                    "line_b": at("coherency_line_size").and_then(|w| w.parse::<u64>().ok()),
                    "cpus": cpu_list(&shared),
                })
            });
        }
    }
    let mut alike: Vec<Value> = Vec::new();
    for cache in found.into_values() {
        let cpus = cache["cpus"].as_array().cloned().unwrap_or_default();
        let same = alike.iter_mut().find(|one| {
            ["level", "type", "size_b", "ways", "line_b"]
                .iter()
                .all(|field| one[*field] == cache[*field])
                && one["shared_by"] == json!(cpus.len())
        });
        if let Some(one) = same {
            one["count"] = json!(one["count"].as_u64().unwrap_or(0) + 1);
            if let Some(all) = one["cpus"].as_array_mut() {
                all.extend(cpus);
                all.sort_by_key(Value::as_u64);
            }
        } else {
            let mut one = cache;
            one["count"] = json!(1);
            one["shared_by"] = json!(cpus.len());
            alike.push(one);
        }
    }
    Value::Array(alike)
}

/// A size as sysfs writes a cache's, "48K" or "18432K", in bytes.
fn bytes(size: &str) -> Option<u64> {
    let (digits, unit) = size.split_at(
        size.find(|c: char| !c.is_ascii_digit())
            .unwrap_or(size.len()),
    );
    let scale = match unit.trim() {
        "" => 1,
        "K" => 1024,
        "M" => 1024 * 1024,
        _ => return None,
    };
    digits.parse::<u64>().ok().map(|n| n * scale)
}

/// The counters of one read.
pub fn counters(root: &Root, at_ms: u64) -> Counters {
    let stat = root.read("/proc/stat").unwrap_or_default();
    let mut whole = Vec::new();
    let mut cores = BTreeMap::new();
    let named = |name: &str| -> Option<u64> {
        stat.lines().find_map(|line| {
            line.strip_prefix(name)?
                .split_whitespace()
                .next()?
                .parse()
                .ok()
        })
    };
    let switches = named("ctxt ");
    let interrupts = named("intr ");
    let softirqs = named("softirq ");
    let forks = named("processes ");
    for line in stat.lines() {
        let Some(rest) = line.strip_prefix("cpu") else {
            continue;
        };
        let (which, fields) = rest.split_at(rest.find(' ').unwrap_or(rest.len()));
        let fields: Vec<u64> = fields
            .split_whitespace()
            .filter_map(|f| f.parse().ok())
            .collect();
        if which.is_empty() {
            whole = fields;
        } else if let Ok(n) = which.parse::<u32>() {
            cores.insert(n, fields);
        }
    }
    Counters {
        at_ms,
        whole,
        cores,
        switches,
        interrupts,
        softirqs,
        forks,
        npu_busy_us: root.number(&format!("{NPU}/npu_busy_time_us")),
    }
}

/// Where a processor's time went between two reads, each field's share,
/// and how busy it was: all but idle and waiting on a disk.
fn spent(now: &[u64], before: Option<&Vec<u64>>) -> (Value, Value) {
    let change = |i: usize| {
        now.get(i)
            .copied()
            .unwrap_or(0)
            .saturating_sub(before.and_then(|b| b.get(i).copied()).unwrap_or(0))
    };
    let all: u64 = (0..COUNTED).map(change).sum();
    if all == 0 {
        return (Value::Null, Value::Null);
    }
    let share = |ticks: u64| rounded(float(ticks) / float(all), 4);
    let time: serde_json::Map<String, Value> = TIMES
        .iter()
        .enumerate()
        .filter(|(i, _)| *i < now.len())
        .map(|(i, name)| ((*name).to_owned(), share(change(i))))
        .collect();
    let busy = all - change(IDLE) - change(IOWAIT);
    (share(busy), Value::Object(time))
}

/// The temperatures of the processor's sensors, as coretemp or k10temp
/// say them: the whole package's, with the most it's meant to reach and
/// where it's critical, and each core's by its core id.
fn temperatures(root: &Root) -> (Value, BTreeMap<u32, f64>) {
    let mut cores = BTreeMap::new();
    for hwmon in root.names("/sys/class/hwmon") {
        let folder = format!("/sys/class/hwmon/{hwmon}");
        let name = root.said(&format!("{folder}/name")).unwrap_or_default();
        if !matches!(name.as_str(), "coretemp" | "k10temp" | "zenpower") {
            continue;
        }
        let mut whole = Value::Null;
        for file in root.names(&folder).iter().filter(|f| f.ends_with("_label")) {
            let sensor = file.trim_end_matches("_label");
            let label = root.said(&format!("{folder}/{file}")).unwrap_or_default();
            let degrees = |what: &str| {
                root.said(&format!("{folder}/{sensor}_{what}"))
                    .and_then(|v| v.parse::<i32>().ok())
                    .map(|milli| rounded(f64::from(milli) / 1000.0, 1))
            };
            if let Some(core) = label.strip_prefix("Core ").and_then(|n| n.parse().ok()) {
                if let Some(Value::Number(value)) = degrees("input") {
                    cores.insert(core, value.as_f64().unwrap_or_default());
                }
            } else if label.starts_with("Package") || label == "Tctl" || label == "Tdie" {
                whole = json!({
                    "temperature_c": degrees("input"),
                    "high_c": degrees("max"),
                    "critical_c": degrees("crit"),
                });
            }
        }
        if whole.is_null()
            && let Some(hottest) = cores.values().copied().reduce(f64::max)
        {
            whole =
                json!({"temperature_c": rounded(hottest, 1), "high_c": null, "critical_c": null});
        }
        return (whole, cores);
    }
    (Value::Null, cores)
}

/// The NPU's load over `span` milliseconds, its busy time and its clock,
/// or none on a machine without one.
fn npu(root: &Root, now: &Counters, before: Option<&Counters>, span: Option<u64>) -> Option<Value> {
    let max = root.number(&format!("{NPU}/npu_max_frequency_mhz"))?;
    let then = before.map_or(Some(0), |b| b.npu_busy_us);
    let busy = match (now.npu_busy_us, then, span) {
        (Some(now), Some(then), Some(over)) if over > 0 && now >= then => {
            rounded((float(now - then) / (float(over) * 1000.0)).min(1.0), 4)
        }
        _ => Value::Null,
    };
    Some(json!({
        "busy_ratio": busy,
        "busy_us": now.npu_busy_us,
        "clock_mhz": root.number(&format!("{NPU}/npu_current_frequency_mhz")),
        "max_mhz": max,
    }))
}

/// The processor's record, from this read's counters and the one before.
pub fn record(root: &Root, fixed: &Fixed, now: &Counters, before: Option<&Counters>) -> Value {
    let over_ms = before.map(|b| now.at_ms.saturating_sub(b.at_ms));
    let (busy, time) = spent(&now.whole, before.map(|b| &b.whole));
    let (heat, core_heat) = temperatures(root);
    let cpu =
        |n: u32, file: &str| root.said(&format!("/sys/devices/system/cpu/cpu{n}/cpufreq/{file}"));
    let cores: Vec<Value> = fixed
        .cpus
        .iter()
        .map(|each| {
            let counted = now.cores.get(&each.n);
            let (busy, time) = counted.map_or((Value::Null, Value::Null), |fields| {
                spent(fields, before.and_then(|b| b.cores.get(&each.n)))
            });
            let online = counted.is_some();
            json!({
                "n": each.n,
                "core": each.core,
                "kind": each.kind,
                "online": online,
                "busy_ratio": busy,
                "time": time,
                "clock_khz": if online { cpu(each.n, "scaling_cur_freq").and_then(|c| c.parse::<u64>().ok()) } else { None },
                "min_khz": each.min_khz,
                "max_khz": each.max_khz,
                "base_khz": each.base_khz,
                "temperature_c": each.core.and_then(|core| core_heat.get(&core)).copied(),
            })
        })
        .collect();
    let first = fixed.cpus.first().map_or(0, |cpu| cpu.n);
    let loadavg = root.read("/proc/loadavg").unwrap_or_default();
    let load: Vec<&str> = loadavg.split_whitespace().collect();
    let (runnable, tasks) = load
        .get(3)
        .and_then(|tasks| tasks.split_once('/'))
        .map_or((None, None), |(r, t)| {
            (r.parse::<u64>().ok(), t.parse::<u64>().ok())
        });
    let stat = root.read("/proc/stat").unwrap_or_default();
    let gauge = |name: &str| -> Option<u64> {
        stat.lines().find_map(|line| {
            line.strip_prefix(name)?
                .split_whitespace()
                .next()?
                .parse()
                .ok()
        })
    };
    // Which file said whether it boosts, and what it said, for the
    // drawer that says it.
    let turbo_read = [("intel_pstate/no_turbo", 0), ("cpufreq/boost", 1)]
        .into_iter()
        .find_map(|(file, on)| {
            let said = root.number(&format!("/sys/devices/system/cpu/{file}"))?;
            Some((file, said, said == on))
        });
    let turbo = turbo_read.map(|(_, _, on)| on);
    // A first read's rates are since the machine started.
    let span = over_ms.or_else(|| crate::boot::up_ms(root));
    let rate = |now: Option<u64>, before: Option<u64>| {
        per_second(now?, before, span).map(|r| rounded(r, 1))
    };
    let npu = npu(root, now, before, span);
    json!({
        "key": "processor:0",
        "kind": "processor",
        "model": fixed.model,
        "vendor": fixed.vendor,
        "family": fixed.family,
        "model_number": fixed.model_number,
        "stepping": fixed.stepping,
        "microcode": fixed.microcode,
        "caches": fixed.caches,
        "busy_ratio": busy,
        "time": time,
        "cores": cores,
        "driver": fixed.driver,
        "guest": fixed.guest,
        "governor": cpu(first, "scaling_governor"),
        "preference": cpu(first, "energy_performance_preference"),
        "turbo": turbo,
        "turbo_read": turbo_read.map(|(file, said, _)| json!([file, said])),
        "load": load.iter().take(3).filter_map(|l| l.parse::<f64>().ok()).collect::<Vec<_>>(),
        "runnable": runnable,
        "tasks": tasks,
        "running": gauge("procs_running "),
        "blocked": gauge("procs_blocked "),
        "switches_per_s": rate(now.switches, before.and_then(|b| b.switches)),
        "interrupts_per_s": rate(now.interrupts, before.and_then(|b| b.interrupts)),
        "softirqs_per_s": rate(now.softirqs, before.and_then(|b| b.softirqs)),
        "forks_per_s": rate(now.forks, before.and_then(|b| b.forks)),
        // The counts since startup the rates are measured from, by the
        // names /proc/stat gives them.
        "counts": { "ctxt": now.switches, "intr": now.interrupts,
            "softirq": now.softirqs, "processes": now.forks },
        "pressure": crate::pressure::parse(root.read("/proc/pressure/cpu").as_deref()),
        "temperature": heat,
        "throttled": crate::heat::throttled(
            root,
            &fixed.cpus.iter().map(|each| (each.n, each.core)).collect::<Vec<_>>(),
        ),
        "npu": npu,
        "over_ms": span,
        "since": since(over_ms),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_read_within_a_few_ticks_of_the_last_measures_nothing() {
        let before = Counters {
            at_ms: 10_000,
            whole: vec![],
            cores: BTreeMap::new(),
            switches: None,
            interrupts: None,
            softirqs: None,
            forks: None,
            npu_busy_us: None,
        };
        assert!(!too_soon(None, 10_000), "a first read always measures");
        assert!(too_soon(Some(&before), 10_004), "no tick has passed");
        assert!(
            too_soon(Some(&before), 10_249),
            "too few ticks to say a share"
        );
        assert!(
            !too_soon(Some(&before), 10_250),
            "a quarter second is enough"
        );
    }

    #[test]
    fn a_model_loses_its_marks() {
        assert_eq!(
            model("Intel(R) Core(TM) Ultra X7 358H"),
            "Intel Core Ultra X7 358H"
        );
        assert_eq!(
            model("Intel(R) Core(TM) i7-8650U CPU @ 1.90GHz"),
            "Intel Core i7-8650U"
        );
    }

    #[test]
    fn a_cache_size_is_in_bytes() {
        assert_eq!(bytes("48K"), Some(49152));
        assert_eq!(bytes("18432K"), Some(18_874_368));
        assert_eq!(bytes("2M"), Some(2_097_152));
    }

    #[test]
    fn time_spent_is_each_fields_share_and_busy_leaves_out_waiting() {
        let before = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let now = vec![40, 0, 10, 40, 10, 0, 0, 0, 5, 0];
        let (busy, time) = spent(&now, Some(&before));
        assert_eq!(busy, json!(0.5));
        assert_eq!(time["user"], json!(0.4));
        assert_eq!(time["iowait"], json!(0.1));
        assert_eq!(time["guest"], json!(0.05));
    }

    #[test]
    fn small_cores_that_clock_highest_are_efficient() {
        let cpu = |n, max| Cpu {
            n,
            core: None,
            kind: None,
            min_khz: None,
            max_khz: Some(max),
            base_khz: None,
        };
        let mut cpus = vec![cpu(0, 4_700_000), cpu(4, 3_700_000), cpu(12, 3_300_000)];
        kinds(&mut cpus, &[0]);
        let said: Vec<_> = cpus.iter().map(|c| c.kind).collect();
        assert_eq!(
            said,
            [Some("performance"), Some("efficient"), Some("low-power")]
        );
    }
}
