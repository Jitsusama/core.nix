//! Machine's own rows, ready for the launcher, in Y2YBTS's ready-row form
//! (QEST-20261023-Y2YBTS attachments/speed/ready-rows.md): the Memory,
//! with each of its parts a step in, in the order its words list them,
//! and the Uptime, with what it spent idle and asleep, when it started and
//! the Linux it runs. They're what the launcher's `Machine.qml` and
//! `Machine.js` made of the same memory, boot and using records, moved
//! here so the shell words, sorts and builds nothing (Joel, 2026-11-10:
//! "the daemons need to be the data layer"). The Processor's are made
//! in `processor_rows.rs` with the pieces here, and the Disk's in
//! `disk_rows.rs`.
//!
//! Nothing here reads the wall clock: how long the machine has been up
//! and when it started travel as times (`lasted` and `moment`), which
//! the shell words, so no row changes as a minute passes. What share of
//! that it slept is a figure, worked out against the clock since boot as
//! each read is made, and moves only as its percent does.
//!
//! The programs using the memory most are read only while the launcher
//! asks for them, each process's file every second, so Using It says
//! them only while they were read just now, and holds its place saying
//! it's still reading otherwise.
//!
//! Long because every part the memory has, from its pressure to each
//! count the kernel keeps, is a row of its own with its words, and its
//! cases stay beside the private pieces they prove.

use ready_rows::time::{self, Form};
use serde_json::{Value, json};

use crate::rows::{Made, Ready, bar, figure, item};
use crate::telemetry::Heard;
use crate::words::Words;

/// The place the launcher shows these rows in.
pub const PLACE: &str = "machine";
/// A pressure past this share of the last ten seconds warns.
const PRESSED_PCT: f64 = 10.0;
/// A measure takes the next unit a thousand at a time, rounding into it
/// from here, so it never reads a thousand of its unit.
const STEP: f64 = 1000.0;
const ROUNDS_UP: f64 = 999.5;
const BYTES: [&str; 6] = ["B", "kB", "MB", "GB", "TB", "PB"];
/// The parts of a run said in one, as "DDR5 · 2 modules".
const DOT: &str = " \u{b7} ";
/// The most rows a step in a holder's drawer says, the rest counted.
const HOLDS: usize = 8;
/// The memory's parts, by the fields `/proc/meminfo` keeps them in.
const HELD_FROM: [(&str, &[&str]); 6] = [
    ("programs", &["AnonPages", "Active(anon)", "Inactive(anon)"]),
    (
        "files",
        &["Cached", "Shmem", "Active(file)", "Inactive(file)"],
    ),
    ("shared", &["Shmem"]),
    (
        "kernel",
        &[
            "Slab",
            "KernelStack",
            "PageTables",
            "SecPageTables",
            "Percpu",
            "VmallocUsed",
        ],
    ),
    ("buffers", &["Buffers"]),
    ("unwritten", &["Dirty", "Writeback"]),
];

/// What Machine's rows are made of: the processor's record, the
/// memory's, this boot's, the programs using each most, where they
/// were read just now, and the processor's telemetry, while it's asked.
#[derive(Clone, Copy, Default)]
pub struct Held<'a> {
    pub processor: Option<&'a Value>,
    pub memory: Option<&'a Value>,
    pub boot: Option<&'a Value>,
    pub using: Option<&'a [Value]>,
    pub telemetry: Option<&'a Heard>,
}

/// Machine's rows from what's held, `now` being the milliseconds since
/// boot, asleep included. A thing whose record isn't held yet has no row,
/// and the shell holds its place.
pub fn read(held: Held, now: u64, words: &Words) -> Ready {
    let say = Say { words };
    let mut made = Vec::new();
    let processor = held
        .processor
        .and_then(|p| crate::processor_rows::row(p, held.using, words));
    // What the telemetry says goes among the processor's other parts, in
    // their words' order.
    made.extend(processor.map(|mut cpu| {
        if let Some(heard) = held.telemetry {
            let mut inside = std::mem::take(&mut cpu.inside);
            inside.extend(crate::telemetry_rows::rows(&say, heard));
            cpu.inside = ordered(words, "machine:cpu", inside);
        }
        cpu
    }));
    if let Some(memory) = held
        .memory
        .filter(|m| m["total_b"].as_f64().is_some_and(|t| t > 0.0))
    {
        made.push(say.memory(memory, held.using));
    }
    if let Some(boot) = held.boot.filter(|b| b["booted_at_s"].is_u64()) {
        made.push(say.uptime(boot, now));
    }
    let mut rows = Vec::new();
    crate::rows::placed(PLACE, made, None, 0, &mut rows);
    Ready { rows, head: None }
}

/// A figure's number, or none.
pub(crate) fn number(value: &Value) -> f64 {
    value.as_f64().unwrap_or(0.0)
}

/// A share as a whole percentage, "42%".
pub(crate) fn percent(share: f64) -> String {
    format!("{:.0}%", (share * 100.0).round())
}

/// A thing's parts in the order its words list them (VKCF3Z, 2026-11-07:
/// what moves most first, then what describes, the model last), each
/// part where its figure is made where the words don't list it.
pub(crate) fn ordered(words: &Words, thing: &str, mut rows: Vec<Made>) -> Vec<Made> {
    let order = words.parts(thing);
    let place = |row: &Made| {
        let id = format!("machine:{}", row.id);
        order
            .iter()
            .position(|part| *part == id)
            .unwrap_or(order.len())
    };
    rows.sort_by_key(place);
    rows
}

/// The rows in what's said, as Machine's own words say them.
pub(crate) struct Say<'a> {
    pub(crate) words: &'a Words,
}

impl Say<'_> {
    pub(crate) fn name(&self, id: &str) -> String {
        self.words.name(&format!("machine:{id}"), &[])
    }

    pub(crate) fn says(&self, id: &str, phrase: &str, fill: &[(&str, &str)]) -> String {
        self.words.said(&format!("machine:{id}"), phrase, fill)
    }

    /// A measure to three digits and its unit, a thousand a step: bytes
    /// whole below a kilobyte, "915 MB", "1.52 GB" (Machine.js's
    /// `measure`).
    pub(crate) fn measure(&self, value: f64, units: &[&str]) -> String {
        let mut scaled = value;
        let mut at = 0;
        while scaled.abs() >= ROUNDS_UP && at < units.len() - 1 {
            scaled /= STEP;
            at += 1;
        }
        let said = if at == 0 && units[0] == BYTES[0] {
            self.words.number(scaled.round(), 0)
        } else {
            self.words.significant(scaled)
        };
        format!("{said} {}", units[at])
    }

    pub(crate) fn bytes(&self, count: f64) -> String {
        self.measure(count, &BYTES)
    }

    pub(crate) fn rate(&self, count: f64) -> String {
        format!("{}/s", self.bytes(count))
    }

    /// A count a second, whole once it's past ten.
    pub(crate) fn often(&self, count: f64) -> String {
        format!("{}/s", self.words.number(count, usize::from(count < 10.0)))
    }

    /// Two rates running opposite ways, each named, or none while both
    /// are still.
    fn both(&self, inward: &Value, outward: &Value) -> String {
        let (inward, outward) = (number(inward), number(outward));
        if inward == 0.0 && outward == 0.0 {
            return self.says("memory", "still", &[]);
        }
        let (inward, outward) = (self.rate(inward), self.rate(outward));
        self.says("memory", "moving", &[("in", &inward), ("out", &outward)])
    }

    /// A figure of Machine's, named by its words.
    pub(crate) fn figure(
        &self,
        id: &str,
        said: String,
        graph: Option<Value>,
        note: String,
    ) -> Made {
        Made {
            hue: "machine",
            ..figure(id.to_owned(), self.name(id), said, graph, note)
        }
    }

    /// A figure holding more, a step in, wearing its thing's glyph, its
    /// drawer saying what's inside where it has none of its own.
    pub(crate) fn holder(&self, mut made: Made, glyph: &str, inside: Vec<Made>) -> Made {
        if made.drawer.is_none() {
            made.drawer = Some(self.holding(glyph, &made.name, &inside));
        }
        Made {
            glyph: self.words.glyph(&format!("machine:{glyph}")),
            inside,
            ..made
        }
    }

    /// A holder's drawer, so one key shows what's inside without stepping
    /// in, as a place's row does (`PlaceRow.js`): each row a step in and
    /// what it says, the first few, then how many more. A row saying a
    /// time, which the shell words, is left to its step.
    pub(crate) fn holding(&self, thing: &str, title: &str, inside: &[Made]) -> Value {
        let mut facts: Vec<Value> = inside
            .iter()
            .filter_map(|row| {
                let said = row.side.first()?["said"].as_str()?;
                (!said.is_empty()).then(|| json!({ "name": row.name, "said": said }))
            })
            .collect();
        let more = facts.len().saturating_sub(HOLDS);
        facts.truncate(HOLDS);
        let mut drawer = json!({ "eyebrow": self.says("read-from", thing, &[]),
            "eyebrowNames": true, "title": title, "facts": facts });
        if more > 0 {
            let n = more.to_string();
            drawer["body"] = json!(self.says("read-from", "more", &[("n", &n)]));
        }
        drawer
    }

    /// A row's drawer saying what Linux keeps behind it, by its own names
    /// and as it says them, then the file it's read from (#2204's bar:
    /// the kernel's own, never smoothed over).
    pub(crate) fn as_linux_keeps(
        &self,
        made: Made,
        thing: &str,
        from: &str,
        kept: Vec<(String, String)>,
    ) -> Made {
        let mut facts: Vec<Value> = kept
            .into_iter()
            .filter(|(_, said)| !said.is_empty())
            .map(|(name, said)| json!({ "name": name, "said": said }))
            .collect();
        facts.push(json!({ "name": self.name("read-from"), "said": from, "identifies": true }));
        let drawer = json!({ "eyebrow": self.says("read-from", thing, &[]),
            "eyebrowNames": true, "title": made.name, "facts": facts });
        Made {
            drawer: Some(drawer),
            ..made
        }
    }

    /// A count of bytes as `/proc/meminfo` says it, "16,123,456 kB".
    fn kb(&self, bytes: f64) -> String {
        format!("{} kB", self.words.number((bytes / 1024.0).round(), 0))
    }

    /// A memory row's drawer from `/proc/meminfo`'s fields, those the
    /// kernel counts by number (the huge pages) as numbers.
    fn meminfo(&self, made: Made, m: &Value, fields: &[&str]) -> Made {
        let kept = fields
            .iter()
            .filter_map(|field| {
                let said = if let Some(bytes) = m["info_b"][*field].as_f64() {
                    self.kb(bytes)
                } else {
                    self.words.number(m["huge_pages"][*field].as_f64()?, 0)
                };
                Some(((*field).to_owned(), said))
            })
            .collect();
        self.as_linux_keeps(made, "memory", "/proc/meminfo", kept)
    }

    /// A rate's drawer from the counts since startup in `/proc/vmstat` it's
    /// measured from.
    fn vmstat(&self, made: Made, m: &Value, fields: &[&str]) -> Made {
        let kept = fields
            .iter()
            .filter_map(|field| {
                let count = m["vmstat"][*field].as_f64()?;
                Some(((*field).to_owned(), self.words.number(count, 0)))
            })
            .collect();
        self.as_linux_keeps(made, "memory", "/proc/vmstat", kept)
    }

    /// Held up waiting for something, the share of the last ten seconds,
    /// and a step in, the last minute's and five minutes', each with the
    /// share in which everything waited where the kernel keeps that and
    /// it did, in the words the memory's pressure says them in.
    pub(crate) fn pressured(&self, id: &str, pressure: &Value, full: bool) -> Made {
        let some = &pressure["some"];
        let now = number(&some["avg10_pct"]);
        let thing = id.split(':').next().unwrap_or(id);
        // The kernel's own file, which names the disk's "io".
        let file = format!(
            "/proc/pressure/{}",
            if thing == "disk" { "io" } else { thing }
        );
        let kept = |window: &str| {
            let mut kept = Vec::new();
            for line in ["some", "full"] {
                let share = &pressure[line][format!("{window}_pct")];
                if let Some(share) = share.as_f64().filter(|_| line == "some" || full) {
                    kept.push((format!("{line} {window}"), format!("{share:.2}")));
                }
            }
            for line in ["some", "full"] {
                let total = &pressure[line]["total_us"];
                if let Some(total) = total.as_f64().filter(|_| line == "some" || full) {
                    let said = format!("{} \u{b5}s", self.words.number(total, 0));
                    kept.push((format!("{line} total"), said));
                }
            }
            kept
        };
        let each = |window: &str, name: &str| {
            let share = number(&some[window]);
            let whole = number(&pressure["full"][window]);
            let some = self.words.number(share, 0);
            let said = if full && whole > 0.0 {
                let full = self.words.number(whole, 0);
                self.says(
                    "memory:pressure",
                    "pair",
                    &[("some", &some), ("full", &full)],
                )
            } else {
                self.says("memory:pressure", "some", &[("some", &some)])
            };
            let graph = bar(share / 100.0, share >= PRESSED_PCT);
            let made = self.figure(&format!("{id}-{name}"), said, Some(graph), String::new());
            let avg = window.trim_end_matches("_pct");
            self.as_linux_keeps(made, thing, &file, kept(avg))
        };
        let inside = vec![
            each("avg10_pct", "10"),
            each("avg60_pct", "60"),
            each("avg300_pct", "300"),
        ];
        let pct = self.words.number(now, 0);
        let made = self.figure(
            id,
            self.says(id, "side", &[("pct", &pct)]),
            Some(bar(now / 100.0, now >= PRESSED_PCT)),
            self.says(id, "note", &[]),
        );
        self.holder(made, thing, inside)
    }

    /// The memory: how much is used as its headline, and its parts.
    fn memory(&self, m: &Value, using: Option<&[Value]>) -> Made {
        let (used, total) = (number(&m["used_b"]), number(&m["total_b"]));
        let said = self.says(
            "memory",
            "head",
            &[
                ("used", &self.bytes(used)),
                ("share", &percent(used / total)),
            ],
        );
        let name = self.words.name("machine:memory", &[]);
        let inside = ordered(self.words, "machine:memory", self.memory_parts(m, using));
        Made {
            id: "memory".to_owned(),
            kind: "memory",
            glyph: self.words.glyph("machine:memory"),
            hue: "machine",
            side: vec![item(&said)],
            copies: said,
            graph: Some(bar(used / total, false)),
            drawer: Some(self.holding("memory", &name, &inside)),
            name,
            inside,
            ..Made::default()
        }
    }

    fn memory_parts(&self, m: &Value, using: Option<&[Value]>) -> Vec<Made> {
        let info = &m["info_b"];
        let total = number(&m["total_b"]);
        let field = |name: &str| info[name].as_f64();
        let used = self.figure(
            "memory:used",
            self.says(
                "memory:used",
                "side",
                &[
                    ("used", &self.bytes(number(&m["used_b"]))),
                    ("total", &self.bytes(total)),
                ],
            ),
            Some(bar(number(&m["used_b"]) / total, false)),
            self.says(
                "memory:used",
                "note",
                &[("available", &self.bytes(number(&m["available_b"])))],
            ),
        );
        let mut rows = vec![self.meminfo(used, m, &["MemTotal", "MemAvailable", "MemFree"])];
        if m["pressure"]["some"].is_object() {
            rows.push(self.pressured("memory:pressure", &m["pressure"], true));
        }
        if let Some(swap) = field("SwapTotal").filter(|swap| *swap > 0.0) {
            let swapped = swap - field("SwapFree").unwrap_or(0.0);
            let cached = field("SwapCached").unwrap_or(0.0);
            let made = self.figure(
                "memory:swap",
                self.says(
                    "memory:swap",
                    "side",
                    &[("used", &self.bytes(swapped)), ("total", &self.bytes(swap))],
                ),
                Some(bar(swapped / swap, false)),
                if cached > 0.0 {
                    self.says("memory:swap", "cached", &[("cached", &self.bytes(cached))])
                } else {
                    String::new()
                },
            );
            rows.push(self.meminfo(made, m, &["SwapTotal", "SwapFree", "SwapCached"]));
        }
        if let Some(cached) = field("Cached") {
            let made = self.figure(
                "memory:cached",
                self.bytes(cached),
                Some(bar(cached / total, false)),
                self.says("memory:cached", "note", &[]),
            );
            let fields = ["Cached", "Active(file)", "Inactive(file)", "Shmem"];
            rows.push(self.meminfo(made, m, &fields));
        }
        rows.extend(self.held_by(m, total));
        if let Some(limit) = field("CommitLimit").filter(|limit| *limit > 0.0) {
            let committed = field("Committed_AS").unwrap_or(0.0);
            let made = self.figure(
                "memory:committed",
                percent(committed / limit),
                Some(bar(committed / limit, false)),
                self.says(
                    "memory:committed",
                    "note",
                    &[
                        ("committed", &self.bytes(committed)),
                        ("limit", &self.bytes(limit)),
                    ],
                ),
            );
            rows.push(self.meminfo(made, m, &["Committed_AS", "CommitLimit"]));
        }
        rows.extend(self.moving(m));
        rows.push(self.using(using, total));
        rows.extend(self.modules(&m["modules"]));
        rows.extend(self.counts(m));
        rows
    }

    /// Paging, faults, swapping and what the kernel ended for memory, each
    /// where the kernel says it.
    fn moving(&self, m: &Value) -> Vec<Made> {
        let mut rows = Vec::new();
        if m["paged_in_b_per_s"].is_number() {
            let made = self.figure(
                "memory:paging",
                self.both(&m["paged_in_b_per_s"], &m["paged_out_b_per_s"]),
                None,
                self.says("memory:paging", "note", &[]),
            );
            rows.push(self.vmstat(made, m, &["pgpgin", "pgpgout"]));
        }
        if let Some(faults) = m["faults_per_s"].as_f64() {
            let major = number(&m["major_faults_per_s"]);
            let made = self.figure(
                "memory:faults",
                self.often(faults),
                None,
                if major > 0.0 {
                    self.says("memory:faults", "note", &[("major", &self.often(major))])
                } else {
                    String::new()
                },
            );
            rows.push(self.vmstat(made, m, &["pgfault", "pgmajfault"]));
        }
        if m["swapped_in_b_per_s"].is_number() {
            let (inward, outward) = (&m["compressed_in_b_per_s"], &m["compressed_out_b_per_s"]);
            // What zswap took and gave back, said only while it moves.
            let note = if number(inward) > 0.0 || number(outward) > 0.0 {
                self.says(
                    "memory:swapping",
                    "compressed",
                    &[("both", &self.both(inward, outward))],
                )
            } else {
                String::new()
            };
            let made = self.figure(
                "memory:swapping",
                self.both(&m["swapped_in_b_per_s"], &m["swapped_out_b_per_s"]),
                None,
                note,
            );
            let fields = ["pswpin", "pswpout", "zswpin", "zswpout"];
            rows.push(self.vmstat(made, m, &fields));
        }
        if let Some(kills) = m["oom_kills"].as_u64() {
            let made = self.figure(
                "memory:out-of-memory",
                kills.to_string(),
                None,
                self.says("memory:out-of-memory", "note", &[]),
            );
            rows.push(self.vmstat(made, m, &["oom_kill"]));
        }
        rows
    }

    /// What the memory is held by, biggest first, and a step in, each
    /// part, what zswap holds compressed and the huge pages.
    fn held_by(&self, m: &Value, total: f64) -> Option<Made> {
        let (info, counted) = (&m["info_b"], &m["huge_pages"]);
        let held = kept_by(info);
        let first = held.first()?;
        let mut inside: Vec<Made> = held
            .iter()
            .map(|(id, bytes)| {
                let part = format!("memory:held-{id}");
                let made = self.figure(
                    &part,
                    self.bytes(*bytes),
                    Some(bar(bytes / total, false)),
                    self.says(&part, "note", &[]),
                );
                let fields = HELD_FROM
                    .iter()
                    .find(|(held, _)| held == id)
                    .map_or(&[][..], |(_, fields)| fields);
                self.meminfo(made, m, fields)
            })
            .collect();
        let field = |name: &str| number(&info[name]);
        let (stored, pool) = (field("Zswapped"), field("Zswap"));
        if stored > 0.0 && pool > 0.0 {
            let made = self.figure(
                "memory:held-compressed",
                self.says(
                    "memory:held-compressed",
                    "side",
                    &[("stored", &self.bytes(stored)), ("pool", &self.bytes(pool))],
                ),
                Some(bar(pool / total, false)),
                self.says(
                    "memory:held-compressed",
                    "note",
                    &[("ratio", &self.words.number(stored / pool, 1))],
                ),
            );
            inside.push(self.meminfo(made, m, &["Zswapped", "Zswap"]));
        }
        inside.extend(self.huge(info, counted).map(|made| {
            let fields = [
                "AnonHugePages",
                "Hugetlb",
                "HugePages_Total",
                "HugePages_Free",
            ];
            self.meminfo(made, m, &fields)
        }));
        let side = format!(
            "{} {}",
            self.name(&format!("memory:held-{}", first.0)),
            self.bytes(first.1)
        );
        let made = self.figure(
            "memory:held",
            side,
            None,
            self.says("memory:held", "note", &[]),
        );
        Some(self.holder(made, "memory", inside))
    }

    /// The huge pages, where any are in use or reserved.
    fn huge(&self, info: &Value, counted: &Value) -> Option<Made> {
        let anonymous = number(&info["AnonHugePages"]);
        let reserved = counted["HugePages_Total"].as_u64().unwrap_or(0);
        (anonymous > 0.0 || reserved > 0).then(|| {
            self.figure(
                "memory:held-huge",
                self.bytes(anonymous + number(&info["Hugetlb"])),
                None,
                self.says(
                    "memory:held-huge",
                    "note",
                    &[("reserved", &reserved.to_string())],
                ),
            )
        })
    }

    /// The programs using the memory most, in their places, while they
    /// were read just now; else its place held, saying it's reading.
    fn using(&self, using: Option<&[Value]>, total: f64) -> Made {
        let id = "memory:using";
        let holders: Vec<&Value> = using
            .unwrap_or_default()
            .iter()
            .filter(|each| {
                each["key"]
                    .as_str()
                    .is_some_and(|key| key.starts_with("using:memory:"))
            })
            .collect();
        if using.is_none() || holders.is_empty() {
            return Made {
                copies: String::new(),
                ..self.figure(id, "\u{2026}".to_owned(), None, String::new())
            };
        }
        let inside = holders
            .iter()
            .enumerate()
            .map(|(at, each)| {
                let resident = number(&each["resident_b"]);
                let processes = each["processes"].as_u64().unwrap_or(1);
                let note = if processes > 1 {
                    self.says(id, "processes", &[("n", &processes.to_string())])
                } else {
                    String::new()
                };
                let made = Made {
                    name: each["name"].as_str().unwrap_or_default().to_owned(),
                    ..self.figure(
                        &format!("{id}:{}", at + 1),
                        self.bytes(resident),
                        Some(bar(resident / total, false)),
                        note,
                    )
                };
                // Each process's resident pages, the 24th field, summed.
                let kept = vec![("rss".to_owned(), self.kb(resident))];
                self.as_linux_keeps(made, "memory", "/proc/<pid>/stat", kept)
            })
            .collect();
        let first = &holders[0];
        let side = format!(
            "{} {}",
            first["name"].as_str().unwrap_or_default(),
            self.bytes(number(&first["resident_b"]))
        );
        let made = self.figure(id, side, None, self.says(id, "note", &[]));
        self.holder(made, "memory", inside)
    }

    /// The memory's type and speed, with each kind of module a step in.
    fn modules(&self, listed: &Value) -> Option<Made> {
        let alike = modules(listed);
        let first = alike.first()?;
        let typed = |one: &Module| {
            if one.speed.is_empty() {
                one.kind.clone()
            } else {
                self.says(
                    "memory:type",
                    "speed",
                    &[("type", &one.kind), ("speed", &one.speed)],
                )
            }
        };
        let by = |one: &Module| {
            if one.maker.is_empty() {
                String::new()
            } else {
                self.says("memory:type", "by", &[("maker", &one.maker)])
            }
        };
        let count: usize = alike.iter().map(|one| one.count).sum();
        let held = if alike.len() == 1 && !first.size.is_empty() {
            let phrase = if count == 1 { "module" } else { "modules" };
            self.says(
                "memory:type",
                phrase,
                &[("count", &count.to_string()), ("size", &first.size)],
            )
        } else {
            String::new()
        };
        let inside = alike
            .iter()
            .enumerate()
            .map(|(at, one)| {
                let sized = if one.size.is_empty() {
                    &one.kind
                } else {
                    &one.size
                };
                let name = if one.count > 1 {
                    format!("{} \u{d7} {sized}", one.count)
                } else {
                    sized.clone()
                };
                let note = items(&[by(one), one.part.clone(), one.locators.join(DOT)]);
                let made = Made {
                    name,
                    ..self.figure(&format!("memory:module:{}", at + 1), typed(one), None, note)
                };
                // The firmware's own fields for a memory device (SMBIOS
                // type 17), as dmidecode names them.
                let kept: Vec<(String, String)> = [
                    ("Type", one.kind.clone()),
                    ("Speed", one.speed.clone()),
                    ("Size", one.size.clone()),
                    ("Manufacturer", one.maker.clone()),
                    ("Part Number", one.part.clone()),
                    ("Locator", one.locators.join(", ")),
                ]
                .map(|(name, said)| (name.to_owned(), said))
                .into();
                self.as_linux_keeps(made, "memory", "/sys/firmware/dmi/entries/17-*", kept)
            })
            .collect();
        let made = self.figure("memory:type", typed(first), None, items(&[held, by(first)]));
        Some(self.holder(made, "memory", inside))
    }

    /// Every count the kernel keeps of its memory, by its own name, so
    /// none it gives is left unseen (#2204's bar): those in bytes, then
    /// the huge pages it counts by number.
    fn counts(&self, m: &Value) -> Option<Made> {
        let sized = m["info_b"].as_object()?;
        let empty = serde_json::Map::new();
        let counted = m["huge_pages"].as_object().unwrap_or(&empty);
        if sized.is_empty() {
            return None;
        }
        let row = |name: &str, said: String| {
            let made = Made {
                name: name.to_owned(),
                ..self.figure(&format!("memory:count:{name}"), said, None, String::new())
            };
            self.meminfo(made, m, &[name])
        };
        let inside = sized
            .iter()
            .map(|(name, bytes)| row(name, self.bytes(number(bytes))))
            .chain(
                counted
                    .iter()
                    .map(|(name, count)| row(name, number(count).to_string())),
            )
            .collect();
        let made = self.figure(
            "memory:counts",
            (sized.len() + counted.len()).to_string(),
            None,
            self.says("memory:counts", "note", &[]),
        );
        Some(self.holder(made, "memory", inside))
    }

    /// How long it's been up, as a time the shell words, and its parts:
    /// idle first, as what moves most; asleep after, since it moves least
    /// (VKCF3Z, 2026-11-07); then when it started and its Linux.
    fn uptime(&self, boot: &Value, now: u64) -> Made {
        let booted_ms = boot["booted_at_s"]
            .as_i64()
            .unwrap_or(0)
            .saturating_mul(1000);
        let up_s = crate::float(now) / 1000.0;
        let linux = |made: Made, from: &str, kept: &[(&str, Option<String>)]| {
            let kept = kept
                .iter()
                .filter_map(|(name, said)| Some(((*name).to_owned(), said.clone()?)))
                .collect();
            self.as_linux_keeps(made, "uptime", from, kept)
        };
        let seconds = |field: &str| boot[field].as_f64().map(|s| format!("{s:.2}"));
        let mut inside = Vec::new();
        if let Some(idle) = boot["idle_ratio"].as_f64() {
            let made = self.figure(
                "uptime:idle",
                percent(idle),
                Some(bar(idle, false)),
                self.says("uptime:idle", "note", &[]),
            );
            // `/proc/uptime`'s two fields, as proc(5) names them.
            let kept = [("uptime", seconds("uptime_s")), ("idle", seconds("idle_s"))];
            inside.push(linux(made, "/proc/uptime", &kept));
        }
        if let Some(asleep) = boot["asleep_s"].as_f64() {
            let said = if asleep > 0.0 {
                crate::power_rows::lasted(asleep)
            } else {
                self.says("uptime:asleep", "never", &[])
            };
            let note = if up_s > 0.0 && asleep > 0.0 {
                self.says(
                    "uptime:asleep",
                    "note",
                    &[("share", &percent(asleep / up_s))],
                )
            } else {
                String::new()
            };
            let graph = (up_s > 0.0).then(|| bar(asleep / up_s, false));
            let made = self.figure("uptime:asleep", said, graph, note);
            // The time asleep is how far the clock counting it has run
            // ahead of the one that stops while the machine sleeps.
            let kept = [("CLOCK_BOOTTIME - CLOCK_MONOTONIC", seconds("asleep_s"))];
            inside.push(linux(made, "clock_gettime(2)", &kept));
        }
        let started = Made {
            side: vec![time::at(booted_ms, Form::Moment)],
            copies: String::new(),
            ..self.figure("uptime:started", String::new(), None, String::new())
        };
        let btime = boot["booted_at_s"].as_i64().map(|s| s.to_string());
        inside.push(linux(started, "/proc/stat", &[("btime", btime)]));
        if let Some(kernel) = boot["kernel"].as_str() {
            let build = boot["kernel_build"].as_str().map(str::to_owned);
            let made = self.figure("uptime:kernel", kernel.to_owned(), None, String::new());
            let both = [
                ("osrelease", Some(kernel.to_owned())),
                ("version", build.clone()),
            ];
            let made = linux(made, "/proc/sys/kernel", &both);
            inside.push(match build {
                Some(build) => {
                    let release =
                        self.figure("uptime:release", kernel.to_owned(), None, String::new());
                    let version = self.figure("uptime:build", build.clone(), None, String::new());
                    let parts = vec![
                        linux(
                            release,
                            "/proc/sys/kernel/osrelease",
                            &[("osrelease", Some(kernel.to_owned()))],
                        ),
                        linux(
                            version,
                            "/proc/sys/kernel/version",
                            &[("version", Some(build))],
                        ),
                    ];
                    self.holder(made, "cpu", parts)
                }
                None => made,
            });
        }
        // The words around the time, "up {time}", go with it, so the
        // shell says them as the time fills them.
        let mut lasted = time::at(booted_ms, Form::Lasted);
        lasted["said"] = Value::String(self.says("uptime", "head", &[]));
        let name = self.words.name("machine:uptime", &[]);
        let inside = ordered(self.words, "machine:uptime", inside);
        Made {
            id: "uptime".to_owned(),
            kind: "uptime",
            glyph: self.words.glyph("machine:cpu"),
            hue: "machine",
            side: vec![lasted],
            drawer: Some(self.holding("uptime", &name, &inside)),
            name,
            inside,
            ..Made::default()
        }
    }
}

/// A note's items, joined with dots, those empty given way.
pub(crate) fn items(parts: &[String]) -> String {
    parts
        .iter()
        .filter(|part| !part.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(DOT)
}

/// What the memory is held by, from `/proc/meminfo`'s fields in bytes,
/// biggest first: programs' own memory, files kept from the disk (the
/// page cache, less what's shared, which the kernel counts in it), what
/// programs share, the system's own, buffers for the disk, and what waits
/// to be written. Each by its part's id, for those the kernel says.
fn kept_by(info: &Value) -> Vec<(&'static str, f64)> {
    let has = |field: &str| info[field].is_number();
    let sum = |fields: &[&str]| {
        fields
            .iter()
            .map(|field| number(&info[*field]))
            .sum::<f64>()
    };
    let mut parts = Vec::new();
    if has("AnonPages") {
        parts.push(("programs", number(&info["AnonPages"])));
    }
    if has("Cached") {
        let files = number(&info["Cached"]) - number(&info["Shmem"]);
        parts.push(("files", files.max(0.0)));
    }
    if has("Shmem") {
        parts.push(("shared", number(&info["Shmem"])));
    }
    if has("Slab") || has("KernelStack") || has("PageTables") {
        let system = [
            "Slab",
            "KernelStack",
            "PageTables",
            "SecPageTables",
            "Percpu",
            "VmallocUsed",
        ];
        parts.push(("kernel", sum(&system)));
    }
    if has("Buffers") {
        parts.push(("buffers", number(&info["Buffers"])));
    }
    if has("Dirty") || has("Writeback") {
        parts.push(("unwritten", sum(&["Dirty", "Writeback"])));
    }
    parts.sort_by(|a, b| b.1.total_cmp(&a.1));
    parts
}

/// One kind of memory module, those alike counted once, and where they
/// sit. A field the firmware leaves blank is left out, never guessed.
#[derive(Default, PartialEq)]
struct Module {
    kind: String,
    speed: String,
    size: String,
    maker: String,
    part: String,
    count: usize,
    locators: Vec<String>,
}

fn modules(listed: &Value) -> Vec<Module> {
    let mut alike: Vec<Module> = Vec::new();
    for module in listed.as_array().map(Vec::as_slice).unwrap_or_default() {
        let said = |field: &str| match &module[field] {
            Value::String(text) => text.trim().to_owned(),
            Value::Null => String::new(),
            other => other.to_string(),
        };
        let one = Module {
            kind: said("type"),
            speed: said("speed"),
            size: said("size"),
            maker: said("manufacturer"),
            part: said("part"),
            count: 1,
            locators: Vec::new(),
        };
        let locator = said("locator");
        let same = alike.iter().position(|kept| {
            (&kept.kind, &kept.speed, &kept.size, &kept.maker, &kept.part)
                == (&one.kind, &one.speed, &one.size, &one.maker, &one.part)
        });
        let at = if let Some(at) = same {
            alike[at].count += 1;
            at
        } else {
            alike.push(one);
            alike.len() - 1
        };
        if !locator.is_empty() && !alike[at].locators.contains(&locator) {
            alike[at].locators.push(locator);
        }
    }
    alike
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// The words these cases say: the launcher's own (`Words::launchers`).
    fn words() -> Words {
        Words::launchers("", "")
    }

    const GB: f64 = 1e9;

    fn memory() -> Value {
        json!({
            "key": "memory:main",
            "total_b": 16.0 * GB,
            "used_b": 8.0 * GB,
            "available_b": 8.0 * GB,
            "info_b": { "MemTotal": 16.0 * GB, "AnonPages": 3.0 * GB, "Cached": 4.0 * GB,
                "Shmem": 1.0 * GB, "Zswapped": 0.4 * GB, "Zswap": 0.1 * GB },
            "huge_pages": { "HugePages_Total": 0 },
            "pressure": { "some": { "avg10_pct": 12.0, "avg60_pct": 6.0, "avg300_pct": 3.0 },
                "full": { "avg10_pct": 2.0, "avg60_pct": 0.0, "avg300_pct": 0.0 } },
            "modules": [
                { "type": "DDR5", "speed": "8533 MT/s", "size": "16 GB",
                    "manufacturer": "Samsung", "locator": "A" },
                { "type": "DDR5", "speed": "8533 MT/s", "size": "16 GB",
                    "manufacturer": "Samsung", "locator": "B" }]
        })
    }

    fn boot() -> Value {
        json!({ "key": "boot:b007", "booted_at_s": 1_700_000_000, "asleep_s": 3600,
            "idle_ratio": 0.5, "kernel": "7.2.0", "kernel_build": "#1 SMP" })
    }

    fn row<'a>(rows: &'a [Value], id: &str) -> &'a Value {
        let key = format!("row:machine:{id}");
        rows.iter()
            .find(|row| row["key"] == key.as_str())
            .unwrap_or_else(|| panic!("no {key} among {rows:?}"))
    }

    fn said(row: &Value) -> &Value {
        &row["side"][0]["said"]
    }

    #[test]
    fn memorys_rows_are_worded_and_in_its_words_order() {
        let held = Held {
            memory: Some(&memory()),
            ..Held::default()
        };
        let rows = read(held, 0, &words()).rows;
        let top = row(&rows, "memory");
        assert_eq!(said(top), "8.00 GB used, 50%");
        assert_eq!(top["depth"], json!(0));
        let inside: Vec<&str> = rows
            .iter()
            .filter(|row| row["parent"] == "row:machine:memory")
            .filter_map(|row| row["name"].as_str())
            .collect();
        assert_eq!(
            inside,
            [
                "Used",
                "Pressure",
                "Using It",
                "Cached",
                "Held By",
                "Every Count",
                "Type"
            ],
            "its parts as its words list them"
        );
        // Words that don't list Cached put it last, rather than leave it out.
        let mut unlisted = Words::launchers_vocabulary();
        for thing in unlisted["things"]
            .as_array_mut()
            .expect("things are a list")
        {
            if thing["id"] == "machine:memory" {
                let parts = thing["parts"]
                    .as_array_mut()
                    .expect("Memory lists its parts");
                parts.retain(|part| part != "machine:memory:cached");
            }
        }
        let unlisted = Words::new(&unlisted, "", "");
        let last = read(held, 0, &unlisted).rows;
        let last: Vec<&str> = last
            .iter()
            .filter(|row| row["parent"] == "row:machine:memory")
            .filter_map(|row| row["name"].as_str())
            .collect();
        assert_eq!(
            last.last(),
            Some(&"Cached"),
            "a part its words don't list goes last"
        );
        assert_eq!(said(row(&rows, "memory:used")), "8.00 GB of 16.0 GB");
        let pressure = row(&rows, "memory:pressure");
        assert_eq!(said(pressure), "12% held up");
        assert_eq!(
            pressure["graph"]["warn"],
            json!(true),
            "a tenth or more warns"
        );
        let last = row(&rows, "memory:pressure-10");
        assert_eq!(said(last), "12% something waited \u{b7} 2% everything");
        assert_eq!(last["depth"], json!(2));
        assert_eq!(
            said(row(&rows, "memory:held")),
            "Programs 3.00 GB",
            "held by, biggest first"
        );
        assert_eq!(
            said(row(&rows, "memory:held-compressed")),
            "400 MB in 100 MB"
        );
        let kind = row(&rows, "memory:type");
        assert_eq!(said(kind), "DDR5 at 8533 MT/s");
        assert_eq!(kind["note"], "2 modules of 16 GB \u{b7} by Samsung");
        let alike = row(&rows, "memory:module:1");
        assert_eq!(
            alike["name"], "2 \u{d7} 16 GB",
            "alike modules counted once"
        );
        assert_eq!(alike["note"], "by Samsung \u{b7} A \u{b7} B");
        assert_eq!(
            said(row(&rows, "memory:count:Shmem")),
            "1.00 GB",
            "every count, by its own name"
        );
        assert_eq!(said(row(&rows, "memory:count:HugePages_Total")), "0");
    }

    #[test]
    fn using_it_names_the_programs_only_while_they_were_read() {
        let memory = memory();
        let unread = Held {
            memory: Some(&memory),
            ..Held::default()
        };
        let rows = read(unread, 0, &words()).rows;
        let using = row(&rows, "memory:using");
        assert_eq!(said(using), "\u{2026}", "its place held, still reading");
        assert_eq!(using["acts"], json!([]), "a placeholder copies nothing");
        let programs = [
            json!({ "key": "using:cpu:1", "name": "cargo", "share_ratio": 0.5 }),
            json!({ "key": "using:memory:1", "name": "firefox", "resident_b": 2.0 * GB,
                "processes": 12 }),
            json!({ "key": "using:memory:2", "name": "kitty", "resident_b": 0.2 * GB,
                "processes": 1 }),
        ];
        let read_now = Held {
            memory: Some(&memory),
            using: Some(&programs),
            ..Held::default()
        };
        let rows = read(read_now, 0, &words()).rows;
        assert_eq!(said(row(&rows, "memory:using")), "firefox 2.00 GB");
        let first = row(&rows, "memory:using:1");
        assert_eq!(
            (first["name"].clone(), first["note"].clone()),
            (json!("firefox"), json!("12 processes"))
        );
        assert_eq!(
            row(&rows, "memory:using:2")["note"],
            Value::Null,
            "one process says nothing"
        );
        assert!(
            !rows.iter().any(|row| row["name"] == "cargo"),
            "the processor's programs are the processor's"
        );
    }

    #[test]
    fn the_uptime_travels_as_times_the_shell_words() {
        let boot = boot();
        let held = Held {
            boot: Some(&boot),
            ..Held::default()
        };
        // Up two hours, one of them asleep.
        let rows = read(held, 7_200_000, &words()).rows;
        let top = row(&rows, "uptime");
        assert_eq!(
            top["side"],
            json!([{ "at": 1_700_000_000_000_i64, "as": "lasted", "said": "up {time}" }])
        );
        assert_eq!(top["glyph"], "processor");
        let started = row(&rows, "uptime:started");
        assert_eq!(
            started["side"],
            json!([{ "at": 1_700_000_000_000_i64, "as": "moment" }])
        );
        assert_eq!(
            started["acts"],
            json!([]),
            "the shell copies the time it words"
        );
        let asleep = row(&rows, "uptime:asleep");
        assert_eq!(said(asleep), "1 h");
        assert_eq!(asleep["note"], "50% of the time since it started");
        let names: Vec<&str> = rows
            .iter()
            .filter(|row| row["parent"] == "row:machine:uptime")
            .filter_map(|row| row["name"].as_str())
            .collect();
        assert_eq!(names, ["Idle", "Asleep", "Started", "Linux"]);
        assert_eq!(said(row(&rows, "uptime:build")), "#1 SMP");
    }

    #[test]
    fn nothing_held_makes_no_rows() {
        assert_eq!(read(Held::default(), 0, &words()), Ready::default());
    }

    /// The memory as a machine in use reads it: swapping, paging and
    /// faulting, with something waiting to be written and huge pages.
    fn memory_in_use() -> Value {
        let mut m = memory();
        for (field, bytes) in [
            ("MemAvailable", 8.0 * GB),
            ("MemFree", 1.024 * GB),
            ("Buffers", 0.2 * GB),
            ("Dirty", 0.01 * GB),
            ("Slab", 0.5 * GB),
            ("SwapTotal", 8.0 * GB),
            ("SwapFree", 6.0 * GB),
            ("SwapCached", 0.1 * GB),
            ("Committed_AS", 12.0 * GB),
            ("CommitLimit", 16.0 * GB),
            ("AnonHugePages", 0.2 * GB),
        ] {
            m["info_b"][field] = json!(bytes);
        }
        m["pressure"]["some"]["total_us"] = json!(649_860_568);
        for (field, rate) in [
            ("paged_in_b_per_s", 4096.0),
            ("paged_out_b_per_s", 0.0),
            ("faults_per_s", 120.0),
            ("major_faults_per_s", 2.0),
            ("swapped_in_b_per_s", 0.0),
            ("swapped_out_b_per_s", 8192.0),
            ("compressed_in_b_per_s", 0.0),
            ("compressed_out_b_per_s", 0.0),
        ] {
            m[field] = json!(rate);
        }
        m["oom_kills"] = json!(1);
        m["vmstat"] = json!({ "pgpgin": 81_920, "pgpgout": 40_960, "pgfault": 1_234_567,
            "pgmajfault": 89, "pswpin": 12, "pswpout": 345, "zswpin": 0, "zswpout": 0,
            "oom_kill": 1 });
        m
    }

    /// Every row Memory and the Uptime serve has a drawer giving a little
    /// more than its line (Joel, 2026-11-11 22:00): a row holding more,
    /// what each a step in says; any other, the fields Linux keeps it
    /// from, by their own names, and the file it's read from.
    #[test]
    fn every_memory_and_uptime_row_has_a_drawer_saying_more() {
        let using = [json!({ "key": "using:memory:1", "name": "firefox",
            "resident_b": 2.0 * GB, "processes": 12 })];
        let (m, b) = (memory_in_use(), boot());
        let held = Held {
            memory: Some(&m),
            boot: Some(&b),
            using: Some(&using),
            ..Held::default()
        };
        let rows = read(held, 7_200_000, &words()).rows;
        let bare: Vec<&Value> = rows
            .iter()
            .filter(|row| {
                let drawer = &row["drawer"];
                let facts = drawer["facts"].as_array().is_some_and(|f| !f.is_empty());
                !(drawer["title"].is_string() && (facts || drawer["body"].is_string()))
            })
            .map(|row| &row["key"])
            .collect();
        assert!(
            bare.is_empty(),
            "rows whose drawer says nothing more: {bare:?}"
        );
        let facts = |id: &str| -> Vec<String> {
            row(&rows, id)["drawer"]["facts"]
                .as_array()
                .unwrap()
                .iter()
                .map(|fact| format!("{}={}", text(&fact["name"]), text(&fact["said"])))
                .collect()
        };
        assert_eq!(
            facts("memory:used"),
            [
                "MemTotal=15,625,000 kB",
                "MemAvailable=7,812,500 kB",
                "MemFree=1,000,000 kB",
                "Read From=/proc/meminfo"
            ],
            "Used says the fields it's worked out from, as Linux says them"
        );
        assert_eq!(
            facts("memory:pressure-10"),
            [
                "some avg10=12.00",
                "full avg10=2.00",
                "some total=649,860,568 \u{b5}s",
                "Read From=/proc/pressure/memory"
            ],
            "a pressure's window says the kernel's own line"
        );
        assert_eq!(
            facts("memory:faults"),
            [
                "pgfault=1,234,567",
                "pgmajfault=89",
                "Read From=/proc/vmstat"
            ],
            "a rate says the counts since startup it's measured from"
        );
        assert_eq!(
            facts("memory:pressure")[0],
            "Last 10 Seconds=12% something waited \u{b7} 2% everything",
            "a row holding more says what each a step in says"
        );
        let drawer = &row(&rows, "memory:counts")["drawer"];
        assert_eq!(
            (
                drawer["facts"].as_array().map(Vec::len),
                drawer["body"].as_str()
            ),
            (Some(8), Some("and 10 more, a step in")),
            "Every Count says its first eight and how many more"
        );
        assert_eq!(
            row(&rows, "memory:used")["drawer"]["eyebrow"],
            "on this machine \u{b7} memory"
        );
    }

    fn text(value: &Value) -> String {
        value
            .as_str()
            .map_or_else(|| value.to_string(), str::to_owned)
    }
}
