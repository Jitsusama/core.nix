//! Each disk, `disk:<name>`: how fast it read, wrote, discarded and
//! flushed between two reads, in bytes and in requests, how busy it was,
//! how long each kind of request took and how many waited, how long tasks
//! waited on any disk, its sensors' temperatures, its health as its
//! controller tells `UDisks`, the file systems on it and their room, and
//! what it is: its model, firmware, link, size, block sizes and scheduler.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::{Root, float, per_second, rounded, since};

/// The fields of `/sys/block/<disk>/stat`, as the kernel's iostats.rst
/// names them.
const READS: usize = 0;
const READ_MERGES: usize = 1;
const SECTORS_READ: usize = 2;
const READ_MS: usize = 3;
const WRITES: usize = 4;
const WRITE_MERGES: usize = 5;
const SECTORS_WRITTEN: usize = 6;
const WRITE_MS: usize = 7;
const IN_FLIGHT: usize = 8;
const IO_MS: usize = 9;
const WEIGHTED_MS: usize = 10;
const DISCARDS: usize = 11;
const SECTORS_DISCARDED: usize = 13;
const DISCARD_MS: usize = 14;
const FLUSHES: usize = 15;
const FLUSH_MS: usize = 16;
/// The kernel counts sectors in 512 bytes, whatever the disk's own.
const SECTOR: u64 = 512;
/// Each field of the stat file by the name the kernel's stat.rst gives
/// it, for the drawers that say what each row is measured from.
const STAT_NAMES: [&str; 17] = [
    "read I/Os",
    "read merges",
    "read sectors",
    "read ticks",
    "write I/Os",
    "write merges",
    "write sectors",
    "write ticks",
    "in_flight",
    "io_ticks",
    "time_in_queue",
    "discard I/Os",
    "discard merges",
    "discard sectors",
    "discard ticks",
    "flush I/Os",
    "flush ticks",
];
/// The fields the kernel prints as 32-bit milliseconds (`%u`), which wrap:
/// the weighted time does every few days of uptime.
const WRAPPING: [usize; 6] = [READ_MS, WRITE_MS, IO_MS, WEIGHTED_MS, DISCARD_MS, FLUSH_MS];

/// How far a counter moved between two reads: a 32-bit one across its
/// wrap, and any other as none where it went backwards, as a reset does.
fn moved(field: usize, now: u64, before: u64) -> u64 {
    if WRAPPING.contains(&field) {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the kernel prints these as 32 bits, so their low 32 are all there is"
        )]
        let (now, before) = (now as u32, before as u32);
        u64::from(now.wrapping_sub(before))
    } else {
        now.saturating_sub(before)
    }
}

/// A disk's sensors' labels and limits, which are read once a sensor is
/// first seen and kept until the hwmon listing changes: on an NVMe disk
/// each limit read is a command to the drive, waking it, while the limits
/// never move (Y2YBTS's review of #2276).
#[derive(Default)]
pub struct Kept {
    hwmons: Vec<String>,
    sensors: BTreeMap<String, Value>,
}

/// The disks whose figures are a disk's: not loops, RAM, compressed RAM,
/// mapped or software RAID devices, optical drives, floppies or network
/// blocks, whose own figures are their disk's.
pub fn disks(root: &Root) -> Vec<String> {
    root.names("/sys/block")
        .into_iter()
        .filter(|name| {
            !["loop", "ram", "zram", "dm-", "md", "sr", "fd", "nbd"]
                .iter()
                .any(|prefix| name.starts_with(prefix))
        })
        .collect()
}

/// The counters of one read, each disk's by its name.
#[derive(Clone, Debug, PartialEq)]
pub struct Counters {
    pub at_ms: u64,
    pub disks: BTreeMap<String, Vec<u64>>,
    /// Milliseconds since boot, which a first read's rates are over.
    pub up_ms: u64,
}

pub fn counters(root: &Root, at_ms: u64, names: &[String]) -> Counters {
    let disks = names
        .iter()
        .filter_map(|name| {
            let stat = root.read(&format!("/sys/block/{name}/stat"))?;
            let fields: Vec<u64> = stat
                .split_whitespace()
                .filter_map(|f| f.parse().ok())
                .collect();
            (fields.len() > WEIGHTED_MS).then(|| (name.clone(), fields))
        })
        .collect();
    Counters {
        at_ms,
        disks,
        up_ms: crate::boot::up_ms(root).unwrap_or(0),
    }
}

/// The devices a disk's file systems are mounted from: the disk, each of
/// its partitions, and what's mapped over each, as an encrypted volume is.
fn devices(root: &Root, name: &str) -> Vec<String> {
    let mut found = vec![name.to_owned()];
    let folder = format!("/sys/block/{name}");
    let parts: Vec<String> = root
        .names(&folder)
        .into_iter()
        .filter(|part| part.starts_with(name))
        .collect();
    for part in parts.iter().chain(std::iter::once(&name.to_owned())) {
        if part != name {
            found.push(part.clone());
        }
        let holders = if part == name {
            format!("{folder}/holders")
        } else {
            format!("{folder}/{part}/holders")
        };
        found.extend(root.names(&holders));
    }
    found
}

/// The block device a mount's source names: `/dev/nvme0n1p1` is
/// nvme0n1p1, and `/dev/mapper/system` the mapped device named system.
fn source_device(root: &Root, source: &str) -> Option<String> {
    if let Some(mapped) = source.strip_prefix("/dev/mapper/") {
        return root
            .names("/sys/block")
            .into_iter()
            .find(|dm| root.said(&format!("/sys/block/{dm}/dm/name")).as_deref() == Some(mapped));
    }
    source.strip_prefix("/dev/").map(str::to_owned)
}

/// The room on a mounted file system, or none where it can't be asked.
fn room(root: &Root, mount: &str) -> Option<Value> {
    let path = std::ffi::CString::new(root.path(mount).to_string_lossy().as_bytes()).ok()?;
    // SAFETY: statvfs writes only into the struct it's given, which is
    // zeroed and ours, from a path that's a valid C string.
    let mut said: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(path.as_ptr(), &raw mut said) } != 0 {
        return None;
    }
    let block = said.f_frsize;
    Some(json!({
        "block_b": block,
        "size_b": said.f_blocks * block,
        "free_b": said.f_bfree * block,
        "available_b": said.f_bavail * block,
        "files": said.f_files,
        "files_free": said.f_ffree,
    }))
}

/// The file systems on a disk, each once by the device it's mounted from,
/// where it's first mounted, as / is before its other subvolumes.
fn file_systems(root: &Root, name: &str) -> Value {
    let devices = devices(root, name);
    let mountinfo = root.read("/proc/self/mountinfo").unwrap_or_default();
    let mut seen = Vec::new();
    let mut found = Vec::new();
    for line in mountinfo.lines() {
        let fields: Vec<&str> = line.split(' ').collect();
        let Some(dash) = fields.iter().position(|f| *f == "-") else {
            continue;
        };
        let (Some(mount), Some(kind), Some(source)) =
            (fields.get(4), fields.get(dash + 1), fields.get(dash + 2))
        else {
            continue;
        };
        let Some(device) = source_device(root, source) else {
            continue;
        };
        if !devices.contains(&device) || seen.contains(&device) {
            continue;
        }
        seen.push(device.clone());
        let mount = mount.replace("\\040", " ");
        let mut said = json!({"mount": mount, "type": kind, "device": device});
        if let (Some(room), Some(fields)) = (room(root, &mount), said.as_object_mut()) {
            fields.extend(room.as_object().cloned().unwrap_or_default());
        }
        found.push(said);
    }
    Value::Array(found)
}

/// A disk's sensors, as its hwmon says them: each by its label, with the
/// most it's meant to reach, where it's critical, and the least.
fn temperatures(root: &Root, device: Option<&str>, kept: &mut Kept) -> Value {
    let Some(device) = device else {
        return Value::Array(vec![]);
    };
    let hwmons = root.names("/sys/class/hwmon");
    if hwmons != kept.hwmons {
        kept.sensors.clear();
        kept.hwmons.clone_from(&hwmons);
    }
    for hwmon in hwmons {
        let folder = format!("/sys/class/hwmon/{hwmon}");
        if root.resolved(&format!("{folder}/device")).as_deref() != Some(device) {
            continue;
        }
        let sensors: Vec<Value> = root
            .names(&folder)
            .iter()
            .filter_map(|file| file.strip_suffix("_input"))
            .filter(|sensor| sensor.starts_with("temp"))
            .map(|sensor| {
                let degrees = |what: &str| {
                    root.said(&format!("{folder}/{sensor}_{what}"))
                        .and_then(|v| v.parse::<i64>().ok())
                        // An NVMe disk without a threshold says one far
                        // out of reach, 65261.85 °C, or absolute zero.
                        .filter(|milli| (-100_000..=200_000).contains(milli))
                        .and_then(|milli| i32::try_from(milli).ok())
                        .map(|milli| rounded(f64::from(milli) / 1000.0, 1))
                };
                let limits = kept
                    .sensors
                    .entry(format!("{folder}/{sensor}"))
                    .or_insert_with(|| {
                        json!({
                            "label": root.said(&format!("{folder}/{sensor}_label")).unwrap_or_else(|| sensor.to_owned()),
                            "high_c": degrees("max"),
                            "critical_c": degrees("crit"),
                            "low_c": degrees("min"),
                        })
                    });
                let mut said = limits.clone();
                said["temperature_c"] = degrees("input").into();
                said
            })
            .collect();
        return Value::Array(sensors);
    }
    Value::Array(vec![])
}

/// The PCI link a disk's controller is on, as it's running and at most.
fn link(root: &Root, controller: Option<&str>) -> Value {
    let Some(controller) = controller else {
        return Value::Null;
    };
    let pci = format!("{controller}/device");
    let at = |file: &str| root.said(&format!("{pci}/{file}"));
    let Some(speed) = at("current_link_speed") else {
        return Value::Null;
    };
    json!({
        "speed": speed,
        "width": at("current_link_width").and_then(|w| w.parse::<u64>().ok()),
        "max_speed": at("max_link_speed"),
        "max_width": at("max_link_width").and_then(|w| w.parse::<u64>().ok()),
    })
}

/// A disk's record, from this read's counters and the one before, and its
/// health as `UDisks` last said it, or null.
pub fn record(
    root: &Root,
    name: &str,
    now: &Counters,
    before: Option<&Counters>,
    health: &Value,
    kept: &mut Kept,
) -> Value {
    let folder = format!("/sys/block/{name}");
    let at = |file: &str| root.said(&format!("{folder}/{file}"));
    let number = |file: &str| at(file).and_then(|v| v.parse::<u64>().ok());
    let controller = root.resolved(&format!("{folder}/device"));
    let fields = now.disks.get(name).cloned().unwrap_or_default();
    let then = before.and_then(|b| b.disks.get(name));
    // A first read's rates are since the machine started. A disk new since
    // the read before, as one plugged in, has counted only since it came,
    // so it has no rate until its second read.
    let new = before.is_some() && then.is_none();
    let over_ms = before.map(|b| now.at_ms.saturating_sub(b.at_ms));
    let span = if new {
        None
    } else {
        over_ms.or(Some(now.up_ms))
    };
    let value = |i: usize| fields.get(i).copied();
    let base = |i: usize| then.and_then(|t| t.get(i).copied()).unwrap_or(0);
    let change = |i: usize| moved(i, value(i).unwrap_or(0), base(i));
    let rate = |i: usize, scale: u64| {
        value(i)
            .and_then(|v| per_second(v, Some(base(i)), span))
            .map(|r| rounded(r * float(scale), 1))
    };
    let latency = |done: usize, ms: usize| {
        let requests = change(done);
        (span.is_some() && requests > 0).then(|| rounded(float(change(ms)) / float(requests), 3))
    };
    let span_ms = span.unwrap_or(0);
    let busy = (span_ms > 0 && value(IO_MS).is_some())
        .then(|| rounded((float(change(IO_MS)) / float(span_ms)).min(1.0), 4));
    let queued = (span_ms > 0 && value(WEIGHTED_MS).is_some())
        .then(|| rounded(float(change(WEIGHTED_MS)) / float(span_ms), 3));
    let scheduler = at("queue/scheduler").unwrap_or_default();
    let chosen = scheduler
        .split_whitespace()
        .find_map(|s| s.strip_prefix('[')?.strip_suffix(']'))
        .map(str::to_owned);
    let requests = latency(READS, READ_MS).is_some() || latency(WRITES, WRITE_MS).is_some();
    let all = change(READS) + change(WRITES);
    let overall = (requests && all > 0)
        .then(|| rounded(float(change(READ_MS) + change(WRITE_MS)) / float(all), 3));
    json!({
        "key": format!("disk:{name}"),
        "kind": "disk",
        "name": name,
        "model": at("device/model").map(|m| m.split_whitespace().collect::<Vec<_>>().join(" ")),
        "firmware": at("device/firmware_rev"),
        "transport": at("device/transport"),
        "size_b": number("size").map(|s| s * SECTOR),
        "logical_block_b": number("queue/logical_block_size"),
        "physical_block_b": number("queue/physical_block_size"),
        "rotational": number("queue/rotational").map(|r| r == 1),
        "removable": number("removable").map(|r| r == 1),
        "scheduler": chosen,
        "schedulers": scheduler.split_whitespace().map(|s| s.trim_matches(['[', ']'])).collect::<Vec<_>>(),
        "queue_depth": number("queue/nr_requests"),
        "link": link(root, controller.as_deref()),
        "read_b_per_s": rate(SECTORS_READ, SECTOR),
        "written_b_per_s": rate(SECTORS_WRITTEN, SECTOR),
        "discarded_b_per_s": rate(SECTORS_DISCARDED, SECTOR),
        "reads_per_s": rate(READS, 1),
        "writes_per_s": rate(WRITES, 1),
        "discards_per_s": rate(DISCARDS, 1),
        "flushes_per_s": rate(FLUSHES, 1),
        "read_merges_per_s": rate(READ_MERGES, 1),
        "write_merges_per_s": rate(WRITE_MERGES, 1),
        "busy_ratio": busy,
        "latency_ms": overall,
        "read_latency_ms": latency(READS, READ_MS),
        "write_latency_ms": latency(WRITES, WRITE_MS),
        "discard_latency_ms": latency(DISCARDS, DISCARD_MS),
        "flush_latency_ms": latency(FLUSHES, FLUSH_MS),
        "queued": queued,
        "in_flight": value(IN_FLIGHT),
        "read_b": value(SECTORS_READ).map(|s| s * SECTOR),
        "written_b": value(SECTORS_WRITTEN).map(|s| s * SECTOR),
        "discarded_b": value(SECTORS_DISCARDED).map(|s| s * SECTOR),
        "stat": STAT_NAMES
            .iter()
            .zip(&fields)
            .map(|(name, value)| ((*name).to_owned(), json!(value)))
            .collect::<serde_json::Map<_, _>>(),
        "pressure": crate::pressure::parse(root.read("/proc/pressure/io").as_deref()),
        "temperatures": temperatures(root, controller.as_deref(), kept),
        "health": health,
        "file_systems": file_systems(root, name),
        "over_ms": span,
        "since": since(over_ms),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(at_ms: u64, disks: &[(&str, Vec<u64>)]) -> Counters {
        Counters {
            at_ms,
            disks: disks
                .iter()
                .map(|(name, fields)| ((*name).to_owned(), fields.clone()))
                .collect(),
            up_ms: 1_000_000,
        }
    }

    #[test]
    fn a_millisecond_field_moves_across_its_wrap() {
        let before = u64::from(u32::MAX) - 99;
        assert_eq!(
            moved(WEIGHTED_MS, 400, before),
            500,
            "100 before the wrap, 400 after"
        );
        assert_eq!(
            moved(READS, 400, before),
            0,
            "a 64-bit count going back is none"
        );
    }

    #[test]
    fn a_disk_new_since_the_last_read_has_no_rate_until_its_next() {
        let root = Root::new("/nonexistent");
        let mut kept = Kept::default();
        let fields = |reads| {
            vec![
                reads, 0, 0, reads, 0, 0, 0, 0, 0, 500, 900, 0, 0, 0, 0, 0, 0,
            ]
        };
        let first = read(1000, &[("nvme0n1", fields(10))]);
        let plugged = read(2000, &[("nvme0n1", fields(20)), ("sda", fields(5_000))]);
        let new = record(
            &root,
            "sda",
            &plugged,
            Some(&first),
            &Value::Null,
            &mut kept,
        );
        assert_eq!(
            new["reads_per_s"],
            Value::Null,
            "its lifetime isn't a second's"
        );
        assert_eq!(new["busy_ratio"], Value::Null);
        assert_eq!(new["read_latency_ms"], Value::Null);
        let next = read(3000, &[("nvme0n1", fields(30)), ("sda", fields(5_010))]);
        let known = record(&root, "sda", &next, Some(&plugged), &Value::Null, &mut kept);
        assert_eq!(known["reads_per_s"], json!(10.0));
    }
}
