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
        Made {
            id: "disk".to_owned(),
            kind: "disk",
            name: words.name("machine:disk", &[]),
            glyph: words.glyph("machine:disk"),
            hue: "machine",
            side: vec![item(&said)],
            copies: said,
            graph,
            note: self.since(),
            inside: ordered(words, "machine:disk", self.parts(room)),
            ..Made::default()
        }
    }

    fn parts(&self, room: Option<&Value>) -> Vec<Made> {
        let mut rows = Vec::new();
        rows.extend(room.map(|fs| self.space(fs)));
        rows.push(self.speed());
        if let Some(busy) = self.d["busy_ratio"].as_f64() {
            rows.push(self.figure(
                "disk:busy",
                percent(busy),
                Some(bar(busy, false)),
                self.since(),
            ));
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
        self.figure("disk:space", said, Some(bar(1.0 - share, false)), note)
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
            self.figure(
                "disk:reading",
                self.say.rate(self.rate("read_b_per_s")),
                spark(&self.sparks.read),
                self.requests(self.rate("reads_per_s"), self.rate("read_merges_per_s")),
            ),
            self.figure(
                "disk:writing",
                self.say.rate(self.rate("written_b_per_s")),
                spark(&self.sparks.written),
                self.requests(self.rate("writes_per_s"), self.rate("write_merges_per_s")),
            ),
            self.figure(
                "disk:discarding",
                self.say.rate(self.rate("discarded_b_per_s")),
                None,
                self.requests(self.rate("discards_per_s"), 0.0),
            ),
            self.figure(
                "disk:flushing",
                self.say.often(self.rate("flushes_per_s")),
                None,
                self.says("disk:flushing", "note", &[]),
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
                self.figure(&format!("disk:latency-{id}"), said, None, note)
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
        Some(self.figure("disk:queue", in_flight, None, note))
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
        Some(self.figure(
            "disk:temperature",
            self.degrees(value),
            Some(bar(level.value, level.warn)),
            note,
        ))
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
            self.figure(
                "disk:spare",
                self.says("disk:spare", "side", &[("n", &raw(&smart["avail_spare"]))]),
                Some(bar(spare / 100.0, spare < threshold)),
                self.says("disk:spare", "note", &[("n", &raw(&smart["spare_thresh"]))]),
            ),
            self.figure(
                "disk:written",
                self.say.bytes(field("total_data_written")),
                None,
                String::new(),
            ),
            self.figure(
                "disk:read",
                self.say.bytes(field("total_data_read")),
                None,
                String::new(),
            ),
        ];
        let cycles = raw(&smart["power_cycles"]);
        if let Some(hours) = smart["power_on_hours"].as_f64() {
            let cycles = if cycles.is_empty() { "0" } else { &cycles };
            rows.push(self.figure(
                "disk:powered",
                lasted(hours * 3600.0),
                None,
                self.says("disk:powered", "note", &[("cycles", cycles)]),
            ));
        }
        if let Some(minutes) = smart["ctrl_busy_time"].as_f64() {
            rows.push(self.figure(
                "disk:busy-for",
                lasted(minutes * 60.0),
                None,
                self.says("disk:busy-for", "note", &[]),
            ));
        }
        rows.push(self.figure(
            "disk:errors",
            raw(&smart["media_errors"]),
            None,
            self.says("disk:errors", "note", &[]),
        ));
        if smart["num_err_log_entries"].is_number() {
            rows.push(self.figure(
                "disk:error-log",
                raw(&smart["num_err_log_entries"]),
                None,
                self.says("disk:error-log", "note", &[]),
            ));
        }
        rows.push(self.figure(
            "disk:shutdowns",
            raw(&smart["unsafe_shutdowns"]),
            None,
            self.says("disk:shutdowns", "note", &[("cycles", &cycles)]),
        ));
        rows.extend(self.too_hot(smart));
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
        rows.push(self.figure(
            "disk:warnings",
            said,
            None,
            self.says("disk:warnings", "note", &[]),
        ));
        rows
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
        Some(self.figure(
            "disk:too-hot",
            lasted((warning + critical) * 60.0),
            None,
            note,
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
                Made {
                    name: mount.to_owned(),
                    ..self.figure(&format!("disk:file-system:{mount}"), said, graph, note)
                }
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
        let mut inside = vec![leaf("disk:model-name", model.to_owned(), String::new())];
        if let Some(firmware) = text(d, "firmware") {
            let note = text(&d["health"], "nvme_revision").map_or_else(String::new, |revision| {
                self.says("disk:firmware", "nvme", &[("revision", revision)])
            });
            inside.push(leaf("disk:firmware", firmware.to_owned(), note));
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
            inside.push(leaf("disk:kind", self.says("disk:kind", kind, &[]), note));
        }
        inside.extend(self.link());
        if let Some(size) = d["size_b"].as_f64().filter(|size| *size > 0.0) {
            inside.push(leaf("disk:capacity", self.say.bytes(size), String::new()));
        }
        if let Some(logical) = d["logical_block_b"].as_u64().filter(|b| *b > 0) {
            let (one, physical) = (logical.to_string(), raw(&d["physical_block_b"]));
            let said = if !physical.is_empty() && physical != "0" && physical != one {
                let fill = [("logical", one.as_str()), ("physical", physical.as_str())];
                self.says("disk:block", "both", &fill)
            } else {
                self.says("disk:block", "one", &[("n", &one)])
            };
            inside.push(leaf("disk:block", said, String::new()));
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
            inside.push(leaf("disk:scheduler", scheduler.to_owned(), note));
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
        Some(self.figure("disk:link", said, None, note))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// The words these cases say, as words.json has them.
    fn words() -> Words {
        let vocabulary = json!({ "things": [
            { "id": "machine:disk", "name": "Disk", "glyph": "disk",
                "said": { "head": "{free} free on {mount}",
                    "since-boot": "since the machine started",
                    "since-asked": "since you last asked", "since-now": "just now" },
                "parts": ["machine:disk:space", "machine:disk:filling", "machine:disk:speed",
                    "machine:disk:pressure", "machine:disk:busy", "machine:disk:latency",
                    "machine:disk:queue", "machine:disk:temperature", "machine:disk:health",
                    "machine:disk:file-systems", "machine:disk:model"] },
            { "id": "machine:disk:space", "name": "Space",
                "said": { "free": "{free} free", "on": "on {mount}, of {size}" } },
            { "id": "machine:disk:speed", "name": "Speed",
                "said": { "iops": "{n} requests/s", "merged": "{n} merged/s" } },
            { "id": "machine:disk:reading", "name": "Reading" },
            { "id": "machine:disk:writing", "name": "Writing" },
            { "id": "machine:disk:busy", "name": "Busy" },
            { "id": "machine:disk:pressure", "name": "Pressure",
                "said": { "side": "{pct}% held up" } },
            { "id": "machine:disk:latency", "name": "Latency",
                "said": { "idle": "none asked of it", "side-idle": "idle", "ms": "{n} ms",
                    "note": "each request, on average" } },
            { "id": "machine:disk:latency-reading", "name": "Reading" },
            { "id": "machine:disk:queue", "name": "Backlog",
                "said": { "queued": "{queued} on average", "depth": "{depth} at most" } },
            { "id": "machine:disk:temperature", "name": "Temperature",
                "said": { "degrees": "{n} \u{b0}C", "most": "at most {degrees}" } },
            { "id": "machine:disk:health", "name": "Health",
                "said": { "side": "{n}% worn", "asked": "as its controller said at {time}",
                    "failing": "its controller says it's failing",
                    "told": "as its controller says" } },
            { "id": "machine:disk:spare", "name": "Spare",
                "said": { "side": "{n}%", "note": "it warns below {n}%" } },
            { "id": "machine:disk:shutdowns", "name": "Unsafe Shutdowns",
                "said": { "note": "of {cycles} times it was powered" } },
            { "id": "machine:disk:warnings", "name": "Warnings",
                "said": { "none": "none", "spare": "spare running out" } },
            { "id": "machine:disk:file-systems", "name": "File Systems",
                "said": { "room": "{free} free of {size}", "reserved": "{reserved} kept back",
                    "files": "{free} of {files} files free" } },
            { "id": "machine:disk:model", "name": "Model" },
            { "id": "machine:disk:link", "name": "PCIe",
                "said": { "side": "{speed} \u{d7}{width}", "full": "as fast as it goes",
                    "slower": "of {most} at most" } },
            { "id": "machine:disk:block", "name": "Block Size",
                "said": { "one": "{n} B",
                    "both": "{logical} B logical \u{b7} {physical} B physical" } }] });
        Words::new(&vocabulary, ",", ".")
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
}
