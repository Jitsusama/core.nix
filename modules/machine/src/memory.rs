//! The memory, `memory:main`: every field `/proc/meminfo` keeps, in bytes,
//! how long tasks waited for memory, how often pages faulted, swapped and
//! paged between two reads, how many programs the kernel killed for
//! memory since boot, and the modules the machine read at boot.

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use crate::{Root, float, per_second, rounded, since};

/// The counters a rate is measured between, from one read of
/// `/proc/vmstat`.
#[derive(Clone, Debug, PartialEq)]
pub struct Counters {
    pub at_ms: u64,
    pub vmstat: BTreeMap<String, u64>,
}

pub fn counters(root: &Root, at_ms: u64) -> Counters {
    let vmstat = root
        .read("/proc/vmstat")
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let (name, value) = line.split_once(' ')?;
            Some((name.to_owned(), value.trim().parse().ok()?))
        })
        .collect();
    Counters { at_ms, vmstat }
}

/// `/proc/meminfo`'s fields by the kernel's names: those it counts in KiB
/// in bytes, and the huge pages it counts by number apart.
pub fn meminfo(text: &str) -> (Map<String, Value>, Map<String, Value>) {
    let mut sized = Map::new();
    let mut counted = Map::new();
    for line in text.lines() {
        let Some((name, rest)) = line.split_once(':') else {
            continue;
        };
        let mut fields = rest.split_whitespace();
        let Some(value) = fields.next().and_then(|v| v.parse::<u64>().ok()) else {
            continue;
        };
        if fields.next() == Some("kB") {
            sized.insert(name.to_owned(), json!(value * 1024));
        } else {
            counted.insert(name.to_owned(), json!(value));
        }
    }
    (sized, counted)
}

/// The counts in `/proc/vmstat` Memory's rates are measured from.
const SINCE_STARTUP: [&str; 9] = [
    "pgpgin",
    "pgpgout",
    "pgfault",
    "pgmajfault",
    "pswpin",
    "pswpout",
    "zswpin",
    "zswpout",
    "oom_kill",
];

/// The memory's record, from this read's counters and the one before.
/// `page_b` is the machine's page size, which swapping counts in.
pub fn record(root: &Root, now: &Counters, before: Option<&Counters>, page_b: u64) -> Value {
    let over_ms = before.map(|b| now.at_ms.saturating_sub(b.at_ms));
    // A first read's rates are since the machine started.
    let span = over_ms.or_else(|| crate::boot::up_ms(root));
    let (info, huge) = meminfo(&root.read("/proc/meminfo").unwrap_or_default());
    let field = |name: &str| info.get(name).and_then(Value::as_u64);
    let used = match (field("MemTotal"), field("MemAvailable")) {
        (Some(total), Some(available)) => Some(total.saturating_sub(available)),
        _ => None,
    };
    let counter = |name: &str| now.vmstat.get(name).copied();
    let rate = |name: &str, scale: u64| {
        let now = counter(name)?;
        per_second(now, before.and_then(|b| b.vmstat.get(name).copied()), span)
            .map(|r| rounded(r * float(scale), 1))
    };
    let modules = root
        .read("/run/memory-modules.json")
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|said| said.get("modules").cloned())
        .unwrap_or(Value::Null);
    // The counts since startup the rates are measured from, by their own
    // names, for the drawers that say them.
    let kept: serde_json::Map<String, Value> = SINCE_STARTUP
        .iter()
        .filter_map(|name| Some(((*name).to_owned(), json!(counter(name)?))))
        .collect();
    json!({
        "key": "memory:main",
        "kind": "memory",
        "total_b": field("MemTotal"),
        "available_b": field("MemAvailable"),
        "used_b": used,
        "info_b": info,
        "huge_pages": huge,
        "pressure": crate::pressure::parse(root.read("/proc/pressure/memory").as_deref()),
        "faults_per_s": rate("pgfault", 1),
        "major_faults_per_s": rate("pgmajfault", 1),
        "swapped_in_b_per_s": rate("pswpin", page_b),
        "swapped_out_b_per_s": rate("pswpout", page_b),
        "compressed_in_b_per_s": rate("zswpin", page_b),
        "compressed_out_b_per_s": rate("zswpout", page_b),
        "paged_in_b_per_s": rate("pgpgin", 1024),
        "paged_out_b_per_s": rate("pgpgout", 1024),
        "oom_kills": counter("oom_kill"),
        "vmstat": kept,
        "modules": modules,
        "over_ms": span,
        "since": since(over_ms),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_are_bytes_and_huge_pages_are_counted() {
        let (sized, counted) = meminfo(
            "MemTotal:       65432100 kB\nActive(anon):     1000 kB\nHugePages_Total:       0\n",
        );
        assert_eq!(sized["MemTotal"], json!(65_432_100u64 * 1024));
        assert_eq!(sized["Active(anon)"], json!(1_024_000));
        assert_eq!(counted["HugePages_Total"], json!(0));
    }
}
