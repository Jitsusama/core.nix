//! The Disk's own rows, ready for the launcher in Y2YBTS's ready-row form
//! (QEST-20261023-Y2YBTS attachments/speed/ready-rows.md): the room on /
//! as its headline, and a step in, what it moves, how busy and how slow
//! it is, what waited on it, how hot and how worn it is, its file systems
//! and what it is, in the order its words list them. They're what the
//! launcher's `Machine.qml` made of the machine's disk record, moved here
//! so the shell words, sorts and builds nothing (Joel, 2026-11-10: "the
//! daemons need to be the data layer").
//!
//! They're a place of their own, `disk`, read only while it's watched,
//! since each read asks an NVMe disk its temperature, a command to the
//! drive that nothing else Machine shows needs; each row is still
//! Machine's thing (`machine:disk:speed`).
//!
//! What the disk guards say of the room and of what fills it is still
//! the shell's, from each guard's own status (`Disk.qml`), placed among
//! these by the same words' order, until the guards' status is served as
//! rows here too.
//!
//! When the disk's controller last said how it is travels as a time,
//! which the shell words, so no row changes as a minute passes.
//!
//! Long because every part of a disk, from each rate to each count its
//! controller keeps, is a row with its words, and its cases stay beside
//! the pieces they prove.

use std::time::{Duration, Instant};

use ready_rows::time::{self, Form};
use serde_json::Value;

use crate::machine_rows::{Say, items, number, ordered, percent};
use crate::parts;
use crate::power_rows::{lasted, shares};
use crate::rows::{GRAPHED, Made, Ready, bar, item};
use crate::words::Words;

/// The place the launcher shows these rows in.
pub const PLACE: &str = "disk";
/// The things they are, Machine's.
const THINGS: &str = "machine";
/// A rate's spark is drawn against a megabyte a second at least, so a
/// disk all but idle draws as all but empty.
const FLOOR: f64 = 1e6;
/// Reads further apart than this start the sparks afresh.
const STALE: Duration = Duration::from_secs(5);
/// NVMe counts its temperature thresholds in kelvin.
const KELVIN: f64 = 273.15;
/// What an NVMe disk's controller warns of, by the bits of its critical
/// warning as `UDisks` names them (NVMe base specification, SMART /
/// Health Information), each with the id its words say it by.
const WARNINGS: [(&str, &str); 6] = [
    ("spare", "spare"),
    ("temperature", "temperature"),
    ("degraded", "degraded"),
    ("readonly", "read-only"),
    ("volatile_mem", "volatile-memory"),
    ("pmr_readonly", "persistent-memory-read-only"),
];
/// The latencies Latency holds, by their part's id and the record's
/// field.
const LATENCIES: [(&str, &str); 4] = [
    ("reading", "read_latency_ms"),
    ("writing", "write_latency_ms"),
    ("discarding", "discard_latency_ms"),
    ("flushing", "flush_latency_ms"),
];

/// The reads the Disk's sparks keep while its rows are watched: what it
/// read, wrote, and both, a second each, with when they were last taken.
#[derive(Clone, Debug, Default)]
pub struct Disk {
    at: Option<Instant>,
    read: Vec<f64>,
    written: Vec<f64>,
    moved: Vec<f64>,
}

impl Disk {
    /// Drops every kept read, as nothing watches the Disk any more.
    pub fn forget(&mut self) {
        *self = Disk::default();
    }

    /// The Disk's rows from the disk records held, the machine's own being
    /// the biggest, as a laptop's one disk is; `read_at` is when they were
    /// read, and the sparks take that read once. With no disk there are
    /// none, and the shell holds the place.
    pub fn read(&mut self, disks: &[Value], read_at: Option<Instant>, words: &Words) -> Ready {
        let Some(disk) = biggest(disks) else {
            return Ready::default();
        };
        self.push(disk, read_at);
        let rows = Rows {
            say: Say { words },
            d: disk,
            sparks: self,
        };
        let mut placed = Vec::new();
        crate::rows::placed_as(PLACE, THINGS, vec![rows.row()], None, 0, &mut placed);
        Ready {
            rows: placed,
            head: None,
        }
    }

    /// Adds this read's rates to the sparks, once a read and only where
    /// they're of now, starting them afresh after a gap.
    fn push(&mut self, disk: &Value, read_at: Option<Instant>) {
        let Some(at) = read_at else {
            return;
        };
        if disk["since"] != "now" || self.at == Some(at) {
            return;
        }
        let fresh = self
            .at
            .is_some_and(|last| at.saturating_duration_since(last) <= STALE);
        let (read, written) = (
            number(&disk["read_b_per_s"]),
            number(&disk["written_b_per_s"]),
        );
        for (kept, value) in [
            (&mut self.read, read),
            (&mut self.written, written),
            (&mut self.moved, read + written),
        ] {
            if !fresh {
                kept.clear();
            }
            kept.push(value);
            let from = kept.len().saturating_sub(GRAPHED);
            kept.drain(..from);
        }
        self.at = Some(at);
    }
}

/// The machine's disk, of the disk records: the biggest, the first of
/// those as big.
fn biggest(disks: &[Value]) -> Option<&Value> {
    disks
        .iter()
        .fold(None, |most: Option<&Value>, disk| match most {
            Some(most) if number(&most["size_b"]) >= number(&disk["size_b"]) => Some(most),
            _ => Some(disk),
        })
}

/// A count as its record says it, "3", or nothing where it says none.
fn raw(value: &Value) -> String {
    match value {
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        _ => String::new(),
    }
}

/// What a disk's controller warns of, each by its words' id, or by its
/// own name where they have none.
fn warnings(critical: &Value) -> Vec<String> {
    critical
        .as_array()
        .map(|bits| {
            bits.iter()
                .map(|bit| {
                    let named = raw(bit);
                    WARNINGS
                        .iter()
                        .find(|(said, _)| *said == named)
                        .map_or(named, |(_, id)| (*id).to_owned())
                })
                .collect()
        })
        .unwrap_or_default()
}

/// A string field that says something.
fn text<'a>(record: &'a Value, field: &str) -> Option<&'a str> {
    record[field].as_str().filter(|said| !said.is_empty())
}

/// The disk's rows, as its words say them.
struct Rows<'a> {
    say: Say<'a>,
    d: &'a Value,
    sparks: &'a Disk,
}

impl Rows<'_> {
    fn says(&self, id: &str, phrase: &str, fill: &[(&str, &str)]) -> String {
        self.say.says(id, phrase, fill)
    }

    fn figure(&self, id: &str, said: String, graph: Option<Value>, note: String) -> Made {
        self.say.figure(id, said, graph, note)
    }

    fn holder(&self, made: Made, inside: Vec<Made>) -> Made {
        self.say.holder(made, "disk", inside)
    }

    /// A row's drawer saying what Linux keeps behind it, by its names.
    fn linux<N: Into<String>>(
        &self,
        made: Made,
        from: &str,
        kept: impl IntoIterator<Item = (N, String)>,
    ) -> Made {
        let kept = kept.into_iter().map(|(name, said)| (name.into(), said));
        self.say.as_linux_keeps(made, "disk", from, kept.collect())
    }

    /// A value as its keeper said it: a count whole, a list by commas.
    fn kept(&self, value: &Value) -> String {
        match value {
            Value::Number(n) => n
                .as_f64()
                .map_or_else(String::new, |n| self.say.words.number(n, 0)),
            Value::String(said) => said.clone(),
            Value::Bool(said) => u8::from(*said).to_string(),
            Value::Array(each) => {
                let each: Vec<String> = each.iter().map(|one| self.kept(one)).collect();
                each.join(", ")
            }
            _ => String::new(),
        }
    }

    /// Where the kernel keeps the disk's files.
    fn folder(&self) -> String {
        format!("/sys/block/{}", self.d["name"].as_str().unwrap_or_default())
    }

    /// A row measured from the stat file, saying its fields.
    fn stat(&self, made: Made, names: &[&str]) -> Made {
        let kept = names
            .iter()
            .map(|name| (*name, self.kept(&self.d["stat"][*name])));
        self.linux(made, &format!("{}/stat", self.folder()), kept)
    }

    /// A row of the disk's own files, each by its path under the folder.
    fn files(&self, made: Made, kept: &[(&str, String)]) -> Made {
        self.linux(made, &self.folder(), kept.iter().cloned())
    }

    /// A health row saying what its controller told `UDisks`.
    fn smart(&self, made: Made, names: &[&str]) -> Made {
        let smart = &self.d["health"];
        let kept = names.iter().map(|name| (*name, self.kept(&smart[*name])));
        self.linux(made, "UDisks2 SmartGetAttributes", kept)
    }

    /// A file system's row saying where it's mounted from and its room,
    /// in blocks, as statvfs counts them.
    fn mounted(&self, made: Made, fs: &Value) -> Made {
        let block = fs["block_b"].as_u64().filter(|block| *block > 0);
        let blocks = |field: &str| {
            block
                .zip(fs[field].as_u64())
                .map_or_else(String::new, |(block, bytes)| {
                    self.kept(&Value::from(bytes / block))
                })
        };
        let kept = [
            ("device", self.kept(&fs["device"])),
            ("type", self.kept(&fs["type"])),
            (
                "f_frsize",
                block.map_or_else(String::new, |b| self.kept(&Value::from(b))),
            ),
            ("f_blocks", blocks("size_b")),
            ("f_bfree", blocks("free_b")),
            ("f_bavail", blocks("available_b")),
            ("f_files", self.kept(&fs["files"])),
            ("f_ffree", self.kept(&fs["files_free"])),
        ];
        self.linux(made, "/proc/self/mountinfo \u{b7} statvfs(2)", kept)
    }

    fn rate(&self, field: &str) -> f64 {
        number(&self.d[field])
    }

    /// What its rates are measured over.
    fn since(&self) -> String {
        let since = match self.d["since"].as_str() {
            Some("boot") => "since-boot",
            Some("asked") => "since-asked",
            _ => "since-now",
        };
        self.says("disk", since, &[])
    }

    fn degrees(&self, value: f64) -> String {
        let n = self.say.words.number(value, 0);
        self.says("disk:temperature", "degrees", &[("n", &n)])
    }

    fn ms(&self, value: f64) -> String {
        let n = self.say.words.significant(value);
        self.says("disk:latency", "ms", &[("n", &n)])
    }

    /// The root file system, or the first, where it says its room.
    fn room(&self) -> Option<&Value> {
        let all = self.d["file_systems"].as_array()?;
        let root = all
            .iter()
            .find(|fs| fs["mount"] == "/")
            .or_else(|| all.first())?;
        (number(&root["size_b"]) > 0.0).then_some(root)
    }

    /// The disk's row: the room on / beside it, or where no file system
    /// says one, what it moves, and its parts a step in.
    fn row(&self) -> Made {
        let words = self.say.words;
        let room = self.room();
        let (said, graph) = match room {
            Some(fs) => {
                let (free, size) = (number(&fs["available_b"]), number(&fs["size_b"]));
                let (said, mount) = (
                    self.say.bytes(free),
                    fs["mount"].as_str().unwrap_or_default(),
                );
                (
                    self.says("disk", "head", &[("free", &said), ("mount", mount)]),
                    Some(bar(1.0 - free / size, false)),
                )
            }
            None => (
                self.say
                    .rate(self.rate("read_b_per_s") + self.rate("written_b_per_s")),
                None,
            ),
        };
        let name = words.name("machine:disk", &[]);
        let inside = ordered(words, "machine:disk", self.parts(room));
        Made {
            id: "disk".to_owned(),
            kind: "disk",
            glyph: words.glyph("machine:disk"),
            hue: "machine",
            side: vec![item(&said)],
            copies: said,
            graph,
            note: self.since(),
            drawer: Some(self.say.holding("disk", &name, &inside)),
            name,
            inside,
            ..Made::default()
        }
    }

    fn parts(&self, room: Option<&Value>) -> Vec<Made> {
        let mut rows = Vec::new();
        rows.extend(room.map(|fs| self.space(fs)));
        rows.push(self.speed());
        if let Some(busy) = self.d["busy_ratio"].as_f64() {
            let made = self.figure(
                "disk:busy",
                percent(busy),
                Some(bar(busy, false)),
                self.since(),
            );
            rows.push(self.stat(made, &["io_ticks"]));
        }
        rows.push(self.latency());
        rows.extend(self.queue());
        let pressure = &self.d["pressure"];
        if pressure["some"].is_object() {
            rows.push(self.say.pressured("disk:pressure", pressure, true));
        }
        rows.extend(self.temperature());
        rows.extend(self.health());
        rows.extend(self.file_systems());
        rows.extend(self.model());
        rows
    }

    /// Its room, where no disk guard says it.
    fn space(&self, fs: &Value) -> Made {
        let (free, size) = (number(&fs["available_b"]), number(&fs["size_b"]));
        let free_said = self.say.bytes(free);
        let said = self.says("disk:space", "free", &[("free", &free_said)]);
        let size = self.say.bytes(size);
        let mount = fs["mount"].as_str().unwrap_or_default();
        let note = self.says("disk:space", "on", &[("mount", mount), ("size", &size)]);
        let share = free / number(&fs["size_b"]);
        let made = self.figure("disk:space", said, Some(bar(1.0 - share, false)), note);
        self.mounted(made, fs)
    }

    /// Each kind's requests and those merged into others, each a second,
    /// a part at nothing giving way (VKCF3Z, 2026-11-07).
    fn requests(&self, requests: f64, merged: f64) -> String {
        let count = |n: f64| self.say.words.number(n, usize::from(n < 10.0));
        let each = |n: f64, phrase: &str| {
            if n > 0.0 {
                self.says("disk:speed", phrase, &[("n", &count(n))])
            } else {
                String::new()
            }
        };
        items(&[each(requests, "iops"), each(merged, "merged")])
    }

    /// What it moves, each way a spark of the last reads.
    fn speed(&self) -> Made {
        let spark = |values: &[f64]| Some(shares(values, FLOOR));
        let inside = vec![
            self.stat(
                self.figure(
                    "disk:reading",
                    self.say.rate(self.rate("read_b_per_s")),
                    spark(&self.sparks.read),
                    self.requests(self.rate("reads_per_s"), self.rate("read_merges_per_s")),
                ),
                &["read I/Os", "read merges", "read sectors"],
            ),
            self.stat(
                self.figure(
                    "disk:writing",
                    self.say.rate(self.rate("written_b_per_s")),
                    spark(&self.sparks.written),
                    self.requests(self.rate("writes_per_s"), self.rate("write_merges_per_s")),
                ),
                &["write I/Os", "write merges", "write sectors"],
            ),
            self.stat(
                self.figure(
                    "disk:discarding",
                    self.say.rate(self.rate("discarded_b_per_s")),
                    None,
                    self.requests(self.rate("discards_per_s"), 0.0),
                ),
                &["discard I/Os", "discard sectors"],
            ),
            self.stat(
                self.figure(
                    "disk:flushing",
                    self.say.often(self.rate("flushes_per_s")),
                    None,
                    self.says("disk:flushing", "note", &[]),
                ),
                &["flush I/Os"],
            ),
        ];
        let moved = self.rate("read_b_per_s") + self.rate("written_b_per_s");
        let made = self.figure(
            "disk:speed",
            self.say.rate(moved),
            spark(&self.sparks.moved),
            self.since(),
        );
        self.holder(made, inside)
    }

    /// How long each request waited, on average and by its kind, or idle
    /// where none was asked of it.
    fn latency(&self) -> Made {
        let idle = || self.says("disk:latency", "side-idle", &[]);
        let inside = LATENCIES
            .iter()
            .map(|(id, field)| {
                let value = self.d[*field].as_f64();
                let note = match value {
                    Some(_) => self.since(),
                    None => self.says("disk:latency", "idle", &[]),
                };
                let said = value.map_or_else(idle, |value| self.ms(value));
                let made = self.figure(&format!("disk:latency-{id}"), said, None, note);
                // Each kind's requests and the milliseconds they took.
                let kind = match *id {
                    "reading" => "read",
                    "writing" => "write",
                    "discarding" => "discard",
                    _ => "flush",
                };
                let (done, took) = (format!("{kind} I/Os"), format!("{kind} ticks"));
                self.stat(made, &[done.as_str(), took.as_str()])
            })
            .collect();
        let overall = self.d["latency_ms"].as_f64();
        let note = overall.map_or_else(String::new, |_| self.says("disk:latency", "note", &[]));
        let said = overall.map_or_else(idle, |value| self.ms(value));
        self.holder(self.figure("disk:latency", said, None, note), inside)
    }

    /// What's in flight, with how many wait on average and how many it
    /// takes at most.
    fn queue(&self) -> Option<Made> {
        let in_flight = raw(&self.d["in_flight"]);
        if in_flight.is_empty() {
            return None;
        }
        let queued = number(&self.d["queued"]);
        let depth = self.d["queue_depth"].as_u64().filter(|depth| *depth > 0);
        let note = items(&[
            if queued > 0.0 {
                let queued = self.say.words.significant(queued);
                self.says("disk:queue", "queued", &[("queued", &queued)])
            } else {
                String::new()
            },
            depth.map_or_else(String::new, |depth| {
                let depth = depth.to_string();
                self.says("disk:queue", "depth", &[("depth", &depth)])
            }),
        ]);
        let made = self.figure("disk:queue", in_flight, None, note);
        Some(self.stat(made, &["in_flight", "time_in_queue"]))
    }

    /// How hot it is, by its composite sensor or its first, drawn as Heat
    /// draws it, against the most it's meant to reach and warning near
    /// it; its detail is Heat's alone.
    fn temperature(&self) -> Option<Made> {
        let sensors = self.d["temperatures"].as_array()?;
        let composite = sensors
            .iter()
            .find(|sensor| sensor["label"] == "Composite")
            .or_else(|| sensors.first())?;
        let value = composite["temperature_c"].as_f64()?;
        let (high, critical) = (
            composite["high_c"].as_f64().filter(|c| *c > 0.0),
            composite["critical_c"].as_f64().filter(|c| *c > 0.0),
        );
        let level = parts::level(value, high, critical, "disk");
        let note = critical.or(high).map_or_else(String::new, |most| {
            let degrees = self.degrees(most);
            self.says("disk:temperature", "most", &[("degrees", &degrees)])
        });
        let made = self.figure(
            "disk:temperature",
            self.degrees(value),
            Some(bar(level.value, level.warn)),
            note,
        );
        // In thousandths of a degree, as hwmon writes them.
        let milli = |value: Option<f64>| {
            value.map_or_else(String::new, |v| self.say.words.number(v * 1000.0, 0))
        };
        let kept = [
            ("label", self.kept(&composite["label"])),
            ("temp_input", milli(Some(value))),
            ("temp_max", milli(high)),
            ("temp_crit", milli(critical)),
        ];
        Some(self.linux(made, "/sys/class/hwmon/*/temp*_input", kept))
    }

    /// How worn it is, as its controller tells `UDisks`, with when it
    /// last said so, and its counts a step in.
    fn health(&self) -> Option<Made> {
        let smart = &self.d["health"];
        let used = smart["percent_used"].as_f64()?;
        let failing = number(&smart["avail_spare"]) < number(&smart["spare_thresh"])
            || number(&smart["media_errors"]) > 0.0
            || !warnings(&smart["critical_warning"]).is_empty();
        let worn = raw(&smart["percent_used"]);
        let mut made = self.figure(
            "disk:health",
            self.says("disk:health", "side", &[("n", &worn)]),
            Some(bar(used / 100.0, failing)),
            String::new(),
        );
        match smart["updated_s"].as_i64() {
            _ if failing => made.note = self.says("disk:health", "failing", &[]),
            Some(at) => {
                let mut told = time::at(at.saturating_mul(1000), Form::Moment);
                told["said"] = Value::String(self.says("disk:health", "asked", &[]));
                made.note_time = Some(told);
            }
            None => made.note = self.says("disk:health", "told", &[]),
        }
        Some(self.holder(made, self.health_parts(smart)))
    }

    fn health_parts(&self, smart: &Value) -> Vec<Made> {
        let field = |name: &str| number(&smart[name]);
        let (spare, threshold) = (field("avail_spare"), field("spare_thresh"));
        let mut rows = vec![
            self.smart(
                self.figure(
                    "disk:spare",
                    self.says("disk:spare", "side", &[("n", &raw(&smart["avail_spare"]))]),
                    Some(bar(spare / 100.0, spare < threshold)),
                    self.says("disk:spare", "note", &[("n", &raw(&smart["spare_thresh"]))]),
                ),
                &["avail_spare", "spare_thresh"],
            ),
            self.smart(
                self.figure(
                    "disk:written",
                    self.say.bytes(field("total_data_written")),
                    None,
                    String::new(),
                ),
                &["total_data_written"],
            ),
            self.smart(
                self.figure(
                    "disk:read",
                    self.say.bytes(field("total_data_read")),
                    None,
                    String::new(),
                ),
                &["total_data_read"],
            ),
        ];
        let cycles = raw(&smart["power_cycles"]);
        if let Some(hours) = smart["power_on_hours"].as_f64() {
            let cycles = if cycles.is_empty() { "0" } else { &cycles };
            let made = self.figure(
                "disk:powered",
                lasted(hours * 3600.0),
                None,
                self.says("disk:powered", "note", &[("cycles", cycles)]),
            );
            rows.push(self.smart(made, &["power_on_hours", "power_cycles"]));
        }
        if let Some(minutes) = smart["ctrl_busy_time"].as_f64() {
            let made = self.figure(
                "disk:busy-for",
                lasted(minutes * 60.0),
                None,
                self.says("disk:busy-for", "note", &[]),
            );
            rows.push(self.smart(made, &["ctrl_busy_time"]));
        }
        let made = self.figure(
            "disk:errors",
            raw(&smart["media_errors"]),
            None,
            self.says("disk:errors", "note", &[]),
        );
        rows.push(self.smart(made, &["media_errors"]));
        if smart["num_err_log_entries"].is_number() {
            let made = self.figure(
                "disk:error-log",
                raw(&smart["num_err_log_entries"]),
                None,
                self.says("disk:error-log", "note", &[]),
            );
            rows.push(self.smart(made, &["num_err_log_entries"]));
        }
        let made = self.figure(
            "disk:shutdowns",
            raw(&smart["unsafe_shutdowns"]),
            None,
            self.says("disk:shutdowns", "note", &[("cycles", &cycles)]),
        );
        rows.push(self.smart(made, &["unsafe_shutdowns", "power_cycles"]));
        rows.extend(self.too_hot(smart));
        rows.push(self.warned(smart));
        rows
    }

    /// What its controller warns of, or that it warns of nothing.
    fn warned(&self, smart: &Value) -> Made {
        let warned = warnings(&smart["critical_warning"]);
        let said = if warned.is_empty() {
            self.says("disk:warnings", "none", &[])
        } else {
            let each: Vec<String> = warned
                .iter()
                .map(|id| {
                    let said = self.says("disk:warnings", id, &[]);
                    if said.is_empty() { id.clone() } else { said }
                })
                .collect();
            items(&each)
        };
        let made = self.figure(
            "disk:warnings",
            said,
            None,
            self.says("disk:warnings", "note", &[]),
        );
        // None warned of is still what it said, so the drawer says so.
        let warned = self.kept(&smart["critical_warning"]);
        let warned = if warned.is_empty() {
            "0".to_owned()
        } else {
            warned
        };
        self.linux(
            made,
            "UDisks2 SmartCriticalWarning",
            [("critical_warning", warned)],
        )
    }

    /// How long it ran past where it warns of heat, and past where it's
    /// critical, as its controller counted them in minutes.
    fn too_hot(&self, smart: &Value) -> Option<Made> {
        let warning = smart["warning_temp_time"].as_f64()?;
        let critical = number(&smart["critical_temp_time"]);
        let (warns, criticals) = (number(&smart["wctemp"]), number(&smart["cctemp"]));
        let note = items(&[
            if warns > 0.0 {
                let warning = self.degrees(warns - KELVIN);
                self.says("disk:too-hot", "warning", &[("warning", &warning)])
            } else {
                String::new()
            },
            if criticals > 0.0 && critical > 0.0 {
                let degrees = self.degrees(criticals - KELVIN);
                let lasted = lasted(critical * 60.0);
                let fill = [("critical", degrees.as_str()), ("lasted", lasted.as_str())];
                self.says("disk:too-hot", "critical", &fill)
            } else {
                String::new()
            },
        ]);
        let made = self.figure(
            "disk:too-hot",
            lasted((warning + critical) * 60.0),
            None,
            note,
        );
        Some(self.smart(
            made,
            &[
                "warning_temp_time",
                "critical_temp_time",
                "wctemp",
                "cctemp",
            ],
        ))
    }

    /// Each file system on it, by where it's mounted, with its room and
    /// its files, as the kernel says them.
    fn file_systems(&self) -> Option<Made> {
        let systems = self.d["file_systems"]
            .as_array()
            .filter(|systems| !systems.is_empty())?;
        let inside = systems
            .iter()
            .map(|fs| {
                let mount = fs["mount"].as_str().unwrap_or_default();
                let kind = fs["type"].as_str().unwrap_or_default().to_owned();
                let (size, available) = (number(&fs["size_b"]), number(&fs["available_b"]));
                let (free, files) = (number(&fs["free_b"]), number(&fs["files"]));
                let said = if size > 0.0 {
                    let (free, size) = (self.say.bytes(available), self.say.bytes(size));
                    self.says(
                        "disk:file-systems",
                        "room",
                        &[("free", &free), ("size", &size)],
                    )
                } else {
                    kind.clone()
                };
                let note = items(&[
                    kind,
                    if free > available {
                        let reserved = self.say.bytes(free - available);
                        self.says("disk:file-systems", "reserved", &[("reserved", &reserved)])
                    } else {
                        String::new()
                    },
                    if files > 0.0 {
                        let words = self.say.words;
                        let (free, files) = (
                            words.number(number(&fs["files_free"]), 0),
                            words.number(files, 0),
                        );
                        let fill = [("free", free.as_str()), ("files", files.as_str())];
                        self.says("disk:file-systems", "files", &fill)
                    } else {
                        String::new()
                    },
                    fs["device"].as_str().unwrap_or_default().to_owned(),
                ]);
                let graph = (size > 0.0).then(|| bar(1.0 - available / size, false));
                let made = Made {
                    name: mount.to_owned(),
                    ..self.figure(&format!("disk:file-system:{mount}"), said, graph, note)
                };
                self.mounted(made, fs)
            })
            .collect();
        let mounts: Vec<String> = systems
            .iter()
            .map(|fs| fs["mount"].as_str().unwrap_or_default().to_owned())
            .collect();
        let made = self.figure(
            "disk:file-systems",
            systems.len().to_string(),
            None,
            items(&mounts),
        );
        Some(self.holder(made, inside))
    }

    /// What the disk is: its name, its firmware, its kind, the link it's
    /// on, its size, its blocks and how its requests are ordered.
    fn model(&self) -> Option<Made> {
        let d = self.d;
        let model = text(d, "model")?;
        let leaf = |id: &str, said: String, note: String| self.figure(id, said, None, note);
        let mut inside = vec![self.files(
            leaf("disk:model-name", model.to_owned(), String::new()),
            &[("device/model", model.to_owned())],
        )];
        if let Some(firmware) = text(d, "firmware") {
            let note = text(&d["health"], "nvme_revision").map_or_else(String::new, |revision| {
                self.says("disk:firmware", "nvme", &[("revision", revision)])
            });
            let made = leaf("disk:firmware", firmware.to_owned(), note);
            inside.push(self.files(made, &[("device/firmware_rev", firmware.to_owned())]));
        }
        if let Some(rotational) = d["rotational"].as_bool() {
            let kind = if rotational { "spinning" } else { "solid" };
            let fixed = if d["removable"] == true {
                "removable"
            } else {
                "fixed"
            };
            let note = items(&[
                text(d, "transport").unwrap_or_default().to_owned(),
                self.says("disk:kind", fixed, &[]),
            ]);
            let made = leaf("disk:kind", self.says("disk:kind", kind, &[]), note);
            let kept = [
                ("queue/rotational", self.kept(&d["rotational"])),
                ("removable", self.kept(&d["removable"])),
                ("device/transport", self.kept(&d["transport"])),
            ];
            inside.push(self.files(made, &kept));
        }
        inside.extend(self.link());
        if let Some(size) = d["size_b"].as_f64().filter(|size| *size > 0.0) {
            let made = leaf("disk:capacity", self.say.bytes(size), String::new());
            // In 512-byte sectors, whatever its blocks, as the kernel counts.
            let sectors = d["size_b"]
                .as_u64()
                .map_or(Value::Null, |b| Value::from(b / 512));
            inside.push(self.files(made, &[("size", self.kept(&sectors))]));
        }
        if let Some(logical) = d["logical_block_b"].as_u64().filter(|b| *b > 0) {
            let (one, physical) = (logical.to_string(), raw(&d["physical_block_b"]));
            let said = if !physical.is_empty() && physical != "0" && physical != one {
                let fill = [("logical", one.as_str()), ("physical", physical.as_str())];
                self.says("disk:block", "both", &fill)
            } else {
                self.says("disk:block", "one", &[("n", &one)])
            };
            let made = leaf("disk:block", said, String::new());
            let kept = [
                ("queue/logical_block_size", one),
                ("queue/physical_block_size", physical),
            ];
            inside.push(self.files(made, &kept));
        }
        if let Some(scheduler) = text(d, "scheduler") {
            let others: Vec<String> = d["schedulers"]
                .as_array()
                .map(|all| {
                    all.iter()
                        .filter_map(Value::as_str)
                        .filter(|one| *one != scheduler)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default();
            let note = if others.is_empty() {
                String::new()
            } else {
                self.says("disk:scheduler", "also", &[("all", &items(&others))])
            };
            let made = leaf("disk:scheduler", scheduler.to_owned(), note);
            // The file's own line, the one in use in brackets.
            let line: Vec<String> = d["schedulers"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(|one| {
                    if one == scheduler {
                        format!("[{one}]")
                    } else {
                        one.to_owned()
                    }
                })
                .collect();
            inside.push(self.files(made, &[("queue/scheduler", line.join(" "))]));
        }
        let made = self.figure("disk:model", model.to_owned(), None, String::new());
        Some(self.holder(made, inside))
    }

    /// The link it's on, as fast and as wide as it runs, and how much
    /// more it could where it runs slower than it goes.
    fn link(&self) -> Option<Made> {
        let link = &self.d["link"];
        let speed = text(link, "speed")?;
        let width = raw(&link["width"]);
        let said = self.says("disk:link", "side", &[("speed", speed), ("width", &width)]);
        let most = text(link, "max_speed");
        let most_width = raw(&link["max_width"]);
        let note = match most {
            Some(most) if most != speed || most_width != width => {
                let most = self.says(
                    "disk:link",
                    "side",
                    &[("speed", most), ("width", &most_width)],
                );
                self.says("disk:link", "slower", &[("most", &most)])
            }
            _ => self.says("disk:link", "full", &[]),
        };
        let made = self.figure("disk:link", said, None, note);
        let kept = [
            ("current_link_speed", speed.to_owned()),
            ("current_link_width", width),
            ("max_link_speed", most.unwrap_or_default().to_owned()),
            ("max_link_width", most_width),
        ];
        Some(self.linux(made, &format!("{}/device/device", self.folder()), kept))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// The words these cases say: the launcher's own (`Words::launchers`).
    fn words() -> Words {
        Words::launchers(",", ".")
    }

    fn disk() -> Value {
        json!({
            "key": "disk:nvme0n1", "kind": "disk", "name": "nvme0n1",
            "model": "Fast Disk 1TB", "firmware": "1.0", "size_b": 1_000_000_000_000_u64,
            "logical_block_b": 512, "physical_block_b": 4096, "rotational": false,
            "removable": false, "transport": "nvme", "scheduler": "none",
            "schedulers": ["none", "mq-deadline"],
            "link": { "speed": "16.0 GT/s PCIe", "width": 4, "max_speed": "16.0 GT/s PCIe",
                "max_width": 4 },
            "read_b_per_s": 2_000_000.0, "written_b_per_s": 500_000.0,
            "reads_per_s": 12.0, "writes_per_s": 3.0, "read_merges_per_s": 0.0,
            "busy_ratio": 0.25, "latency_ms": 0.42, "read_latency_ms": 0.3,
            "in_flight": 1, "queued": 0.5, "queue_depth": 1023,
            "pressure": { "some": { "avg10_pct": 1.0, "avg60_pct": 0.5, "avg300_pct": 0.2 } },
            "temperatures": [{ "label": "Composite", "temperature_c": 41.0,
                "high_c": 80.0, "critical_c": 85.0 }],
            "health": { "percent_used": 3, "avail_spare": 100, "spare_thresh": 10,
                "media_errors": 0, "unsafe_shutdowns": 7, "power_cycles": 120,
                "critical_warning": [], "updated_s": 1_700_000_000 },
            "file_systems": [{ "mount": "/", "type": "btrfs", "device": "/dev/nvme0n1p2",
                "size_b": 900_000_000_000_u64, "available_b": 300_000_000_000_u64,
                "free_b": 300_000_000_000_u64, "files": 0, "files_free": 0 }],
            "since": "now"
        })
    }

    fn rows(disks: &[Value]) -> Vec<Value> {
        Disk::default()
            .read(disks, Some(Instant::now()), &words())
            .rows
    }

    fn row<'a>(rows: &'a [Value], id: &str) -> &'a Value {
        let key = format!("row:disk:{id}");
        rows.iter()
            .find(|row| row["key"] == key.as_str())
            .unwrap_or_else(|| panic!("no {key} among {rows:?}"))
    }

    fn side(row: &Value) -> Value {
        row["side"][0]["said"].clone()
    }

    #[test]
    fn the_disks_rows_are_worded_and_in_its_words_order() {
        let said = rows(&[disk()]);
        let top = row(&said, "disk");
        assert_eq!(top["thing"], json!("machine:disk"), "{top}");
        assert_eq!(side(top), json!("300 GB free on /"));
        assert_eq!(top["note"], json!("just now"));
        let parts: Vec<&str> = said
            .iter()
            .filter(|each| each["parent"] == "row:disk:disk")
            .filter_map(|each| each["name"].as_str())
            .collect();
        assert_eq!(
            parts,
            [
                "Space",
                "Speed",
                "Pressure",
                "Busy",
                "Latency",
                "Backlog",
                "Temperature",
                "Health",
                "File Systems",
                "Model"
            ],
            "the parts are out of their words' order"
        );
        assert_eq!(side(row(&said, "disk:speed")), json!("2.50 MB/s"));
        assert_eq!(
            row(&said, "disk:reading")["note"],
            json!("12 requests/s"),
            "a part at nothing should give way"
        );
        assert_eq!(side(row(&said, "disk:latency")), json!("0.420 ms"));
        assert_eq!(
            row(&said, "disk:latency-writing")["note"],
            json!("none asked of it")
        );
        assert_eq!(
            row(&said, "disk:queue")["note"],
            json!("0.500 on average \u{b7} 1023 at most")
        );
        assert_eq!(
            row(&said, "disk:temperature")["note"],
            json!("at most 85 \u{b0}C")
        );
        assert_eq!(
            side(row(&said, "disk:block")),
            json!("512 B logical \u{b7} 4096 B physical")
        );
        assert_eq!(row(&said, "disk:link")["note"], json!("as fast as it goes"));
    }

    #[test]
    fn its_health_says_when_its_controller_said_it_as_a_time() {
        let said = rows(&[disk()]);
        let health = row(&said, "disk:health");
        assert_eq!(side(health), json!("3% worn"));
        assert_eq!(
            health["note"],
            json!({ "at": 1_700_000_000_000_i64, "as": "moment",
                "said": "as its controller said at {time}" }),
            "the time should travel for the shell to word"
        );
        let mut failing = disk();
        failing["health"]["critical_warning"] = json!(["spare"]);
        let said = rows(&[failing]);
        assert_eq!(
            row(&said, "disk:health")["note"],
            json!("its controller says it's failing")
        );
        assert_eq!(
            side(row(&said, "disk:warnings")),
            json!("spare running out")
        );
    }

    #[test]
    fn the_sparks_take_each_read_of_now_once() {
        let mut kept = Disk::default();
        let at = Instant::now();
        kept.read(&[disk()], Some(at), &words());
        kept.read(&[disk()], Some(at), &words());
        assert_eq!(kept.moved, [2_500_000.0], "a read was taken twice");
        let mut booted = disk();
        booted["since"] = json!("boot");
        kept.read(&[booted], Some(at + Duration::from_secs(1)), &words());
        assert_eq!(kept.moved.len(), 1, "a rate since boot was taken");
        kept.read(&[disk()], Some(at + STALE * 2), &words());
        assert_eq!(kept.moved, [2_500_000.0], "a gap didn't start afresh");
    }

    #[test]
    fn the_biggest_disk_is_the_machines_and_none_makes_no_rows() {
        let mut small = disk();
        small["key"] = json!("disk:sda");
        small["size_b"] = json!(1_000_000_u64);
        small["file_systems"] = json!([]);
        let said = rows(&[small, disk()]);
        assert_eq!(side(row(&said, "disk")), json!("300 GB free on /"));
        assert!(rows(&[]).is_empty(), "no disk should make no rows");
    }

    /// Every row the Disk serves has a drawer giving a little more than
    /// its line (Joel, 2026-11-11 22:00): a row holding more, what each a
    /// step in says; any other, what Linux keeps behind it.
    #[test]
    fn every_disk_row_has_a_drawer_saying_more() {
        let mut d = disk();
        d["stat"] = json!({ "read I/Os": 1234, "read merges": 56, "read sectors": 789_000,
            "read ticks": 400, "write I/Os": 321, "write merges": 7, "write sectors": 9_000,
            "write ticks": 90, "in_flight": 1, "io_ticks": 600, "time_in_queue": 500,
            "discard I/Os": 2, "discard merges": 0, "discard sectors": 4_096,
            "discard ticks": 3, "flush I/Os": 40, "flush ticks": 8 });
        d["file_systems"][0]["block_b"] = json!(4096);
        d["health"]["total_data_written"] = json!(5_000_000_000_u64);
        d["health"]["total_data_read"] = json!(9_000_000_000_u64);
        d["health"]["power_on_hours"] = json!(1200);
        let said = rows(&[d]);
        let bare: Vec<&str> = said
            .iter()
            .filter(|each| {
                let drawer = &each["drawer"];
                let facts = drawer["facts"].as_array().is_some_and(|f| !f.is_empty());
                !(drawer["title"].is_string() && (facts || drawer["body"].is_string()))
            })
            .filter_map(|each| each["key"].as_str())
            .collect();
        assert!(
            bare.is_empty(),
            "rows whose drawer says nothing more: {bare:?}"
        );
        let facts = |id: &str| -> Vec<String> {
            row(&said, id)["drawer"]["facts"]
                .as_array()
                .unwrap()
                .iter()
                .map(|fact| {
                    let name = fact["name"].as_str().unwrap_or_default();
                    format!("{name}={}", fact["said"].as_str().unwrap_or_default())
                })
                .collect()
        };
        assert_eq!(
            facts("disk:reading"),
            [
                "read I/Os=1,234",
                "read merges=56",
                "read sectors=789,000",
                "Read From=/sys/block/nvme0n1/stat"
            ],
            "a rate says the counts it's measured from, by the kernel's names"
        );
        assert_eq!(
            facts("disk:scheduler"),
            [
                "queue/scheduler=[none] mq-deadline",
                "Read From=/sys/block/nvme0n1"
            ],
            "what it is says the file that says it, as it says it"
        );
        assert_eq!(
            facts("disk:spare"),
            [
                "avail_spare=100",
                "spare_thresh=10",
                "Read From=UDisks2 SmartGetAttributes"
            ],
            "its health says what its controller told UDisks"
        );
        assert_eq!(
            facts("disk:file-system:/"),
            [
                "device=/dev/nvme0n1p2",
                "type=btrfs",
                "f_frsize=4,096",
                "f_blocks=219,726,562",
                "f_bfree=73,242,187",
                "f_bavail=73,242,187",
                "f_files=0",
                "f_ffree=0",
                "Read From=/proc/self/mountinfo \u{b7} statvfs(2)"
            ],
            "a file system says its mount and its room as statvfs counts it"
        );
    }
}
