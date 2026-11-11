//! The Processor's rows, ready for the launcher in Y2YBTS's ready-row
//! form, beside the Memory and the Uptime (`machine_rows.rs`): how busy
//! it is as its headline, and each of its parts a step in, in the order
//! its words list them. They're what the launcher's `Machine.qml` and
//! `Machine.js` made of the same processor and using records, moved here
//! so the shell words, sorts and builds nothing (Joel, 2026-11-10: "the
//! daemons need to be the data layer").
//!
//! What held it back and what kept it from resting come from the
//! processor's telemetry, asked only while they can be seen, so they're
//! made apart (`telemetry_rows.rs`) and placed among these by the same
//! words' order (`machine_rows.rs`).
//!
//! The programs using the processor most are read only while the
//! launcher asks for them, so Using It says them only while they were
//! read just now, and holds its place saying it's still reading
//! otherwise.
//!
//! Long because each part, from each core to each cache, is a row with
//! its words, and its cases stay beside the pieces they prove.

use serde_json::Value;

use crate::machine_rows::{Say, items, number, ordered, percent};
use crate::parts;
use crate::rows::{Made, bar, item, strip};
use crate::words::Words;

/// A core this busy or busier is maxed.
const MAXED: f64 = 0.95;
/// Past this many maxed cores they're counted, not named.
const NAMED: usize = 4;
/// The kinds of core, in the order Kinds lists them.
const KINDS: [&str; 3] = ["performance", "efficient", "low-power"];
/// The cache types Linux names, each with its own words.
const CACHE_TYPES: [&str; 3] = ["data", "instruction", "unified"];
/// Where the processor's time went, as Time Spent's parts list it, each
/// by its part's id and the kernel's fields it sums. Guests' time is in
/// the user's already, and said beside it as what of it ran a virtual
/// machine. Time stolen is only ever taken by a machine this one runs
/// inside, so on one that runs on its own hardware it's left out.
const SPENT: [(&str, &[&str]); 7] = [
    ("user", &["user"]),
    ("system", &["system"]),
    ("iowait", &["iowait"]),
    ("interrupts", &["irq", "softirq"]),
    ("niced", &["nice"]),
    ("stolen", &["steal"]),
    ("guests", &["guest", "guest_nice"]),
];
/// The NPU's driver counts its clock in megahertz.
const MEGAHERTZ: f64 = 1000.0;
/// A clock's units, from the hertz it's in.
const HERTZ: [&str; 4] = ["Hz", "kHz", "MHz", "GHz"];

/// The processor's row, from its record and the programs using it most
/// where they were read just now, or none while it hasn't said how busy
/// it is, so the shell holds its place.
pub fn row(processor: &Value, using: Option<&[Value]>, words: &Words) -> Option<Made> {
    let busy = processor["busy_ratio"].as_f64()?;
    let say = Cpu {
        say: Say { words },
        p: processor,
    };
    let said = say.says("head", &[("busy", &percent(busy))]);
    Some(Made {
        id: "cpu".to_owned(),
        kind: "processor",
        name: words.name("machine:cpu", &[]),
        glyph: words.glyph("machine:cpu"),
        hue: "machine",
        side: vec![item(&said)],
        graph: Some(bar(busy, false)),
        note: say.since(),
        inside: ordered(words, "machine:cpu", say.parts(using)),
        ..Made::default()
    })
}

/// A core as its record says it.
struct Core<'a>(&'a Value);

impl Core<'_> {
    fn n(&self) -> u64 {
        self.0["n"].as_u64().unwrap_or(0)
    }

    fn load(&self) -> f64 {
        number(&self.0["busy_ratio"])
    }

    fn khz(&self, field: &str) -> f64 {
        number(&self.0[field])
    }

    fn kind(&self) -> &str {
        self.0["kind"].as_str().unwrap_or_default()
    }
}

/// The processor's parts, as its words say them.
struct Cpu<'a> {
    say: Say<'a>,
    p: &'a Value,
}

impl Cpu<'_> {
    fn says(&self, phrase: &str, fill: &[(&str, &str)]) -> String {
        self.say.says("cpu", phrase, fill)
    }

    fn figure(&self, id: &str, said: String, graph: Option<Value>, note: String) -> Made {
        self.say.figure(&format!("cpu:{id}"), said, graph, note)
    }

    fn holder(&self, made: Made, inside: Vec<Made>) -> Made {
        self.say.holder(made, "cpu", inside)
    }

    /// A leaf worth having, as a model is, which copies what it says.
    fn leaf(&self, id: &str, said: &str) -> Made {
        self.figure(id, said.to_owned(), None, String::new())
    }

    /// A part whose figures aren't held yet: its place held, saying it's
    /// still reading, copying nothing.
    fn reading(&self, id: &str) -> Made {
        Made {
            copies: String::new(),
            ..self.figure(id, "\u{2026}".to_owned(), None, String::new())
        }
    }

    /// What its rates are measured over.
    fn since(&self) -> String {
        let since = match self.p["since"].as_str() {
            Some("boot") => "since-boot",
            Some("asked") => "since-asked",
            _ => "since-now",
        };
        self.says(since, &[])
    }

    fn ghz(&self, khz: f64) -> String {
        self.say.measure(khz * 1000.0, &HERTZ)
    }

    fn degrees(&self, value: f64) -> String {
        let n = self.say.words.number(value, 0);
        self.say.says("cpu:temperature", "degrees", &[("n", &n)])
    }

    /// Core numbers as a note says them: "core 4", "cores 0 to 3", or
    /// each named where they don't run unbroken, "cores 0, 2 and 5".
    fn cores_said(&self, numbers: &[u64]) -> String {
        match numbers {
            [] => String::new(),
            [one] => self
                .say
                .says("cpu:cores", "one", &[("n", &one.to_string())]),
            [first, .., last] if last.checked_sub(*first) == unbroken(numbers) => self.say.says(
                "cpu:cores",
                "run",
                &[("first", &first.to_string()), ("last", &last.to_string())],
            ),
            [rest @ .., last] => {
                let listed: Vec<String> = rest.iter().map(u64::to_string).collect();
                self.say.says(
                    "cpu:cores",
                    "list",
                    &[("list", &listed.join(", ")), ("last", &last.to_string())],
                )
            }
        }
    }

    /// The maxed cores: none, by number while they're few, or how many of
    /// how many.
    fn maxed(&self, cores: &[Core]) -> Made {
        let flat: Vec<u64> = cores
            .iter()
            .filter(|core| core.load() >= MAXED)
            .map(Core::n)
            .collect();
        let said = if flat.is_empty() {
            self.say.says("cpu:maxed", "none", &[])
        } else if flat.len() > NAMED {
            self.say.says(
                "cpu:maxed",
                "many",
                &[
                    ("n", &flat.len().to_string()),
                    ("all", &cores.len().to_string()),
                ],
            )
        } else {
            self.cores_said(&flat)
        };
        self.figure("maxed", said, None, self.say.says("cpu:maxed", "note", &[]))
    }

    fn parts(&self, using: Option<&[Value]>) -> Vec<Made> {
        let p = self.p;
        let cores: Vec<Core> = p["cores"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .filter(|core| core["online"].as_bool() == Some(true))
            .map(Core)
            .collect();
        let mut rows = vec![self.cores(&cores), self.maxed(&cores)];
        rows.extend(self.spent());
        // Every task waiting for a processor is never said of the whole
        // machine (the kernel keeps it for a cgroup alone), so only some is.
        if p["pressure"]["some"].is_object() {
            rows.push(self.say.pressured("cpu:pressure", &p["pressure"], false));
        }
        rows.extend(self.clock(&cores));
        rows.extend(self.temperature());
        rows.push(self.using(using));
        rows.extend(self.counts());
        rows.extend(self.npu());
        rows.extend(self.kinds(&cores));
        rows.extend(self.model());
        rows
    }

    fn cores(&self, cores: &[Core]) -> Made {
        let loads: Vec<f64> = cores.iter().map(Core::load).collect();
        let inside = cores
            .iter()
            .map(|core| {
                let clock = core.khz("clock_khz");
                let note = items(&[
                    core.kind().to_owned(),
                    if clock > 0.0 {
                        self.ghz(clock)
                    } else {
                        String::new()
                    },
                    core.0["temperature_c"]
                        .as_f64()
                        .map(|value| self.degrees(value))
                        .unwrap_or_default(),
                ]);
                let n = core.n().to_string();
                Made {
                    name: self.say.words.name("machine:cpu:core", &[("n", &n)]),
                    ..self.figure(
                        &format!("core:{n}"),
                        percent(core.load()),
                        Some(bar(core.load(), false)),
                        note,
                    )
                }
            })
            .collect();
        let made = self.figure(
            "cores",
            cores.len().to_string(),
            Some(strip(&loads, false)),
            self.since(),
        );
        self.holder(made, inside)
    }

    fn spent(&self) -> Option<Made> {
        let time = self.p["time"].as_object()?;
        let guest = self.p["guest"].as_bool() == Some(true);
        let inside: Vec<Made> = SPENT
            .iter()
            .filter(|(id, _)| guest || *id != "stolen")
            .filter(|(_, fields)| {
                fields
                    .iter()
                    .any(|field| time.get(*field).is_some_and(Value::is_number))
            })
            .map(|(id, fields)| {
                let share: f64 = fields.iter().map(|field| number(&time[*field])).sum();
                let part = format!("spent-{id}");
                let note = self.say.says(&format!("cpu:{part}"), "note", &[]);
                self.figure(&part, percent(share), Some(bar(share, false)), note)
            })
            .collect();
        if inside.is_empty() {
            return None;
        }
        let busy = number(&self.p["busy_ratio"]);
        let made = self.figure(
            "spent",
            percent(busy),
            Some(bar(busy, false)),
            String::new(),
        );
        Some(self.holder(made, inside))
    }

    fn clock(&self, cores: &[Core]) -> Option<Made> {
        let clocks: Vec<f64> = cores
            .iter()
            .map(|core| core.khz("clock_khz"))
            .filter(|clock| *clock > 0.0)
            .collect();
        if clocks.is_empty() {
            return None;
        }
        let average = mean(&clocks);
        let tops = khz_of(cores, "max_khz");
        let most = if tops.is_empty() { 0.0 } else { mean(&tops) };
        let (graph, note) = if most > 0.0 {
            let most_said = self.ghz(most);
            (
                Some(bar(average / most, false)),
                self.say.says("cpu:clock", "of", &[("most", &most_said)]),
            )
        } else {
            (None, self.say.says("cpu:clock", "mean", &[]))
        };
        let made = self.figure("clock", self.ghz(average), graph, note);
        Some(self.holder(made, self.clock_parts(cores)))
    }

    /// The clock's range, the base it's sold at, and how it's driven.
    fn clock_parts(&self, cores: &[Core]) -> Vec<Made> {
        let p = self.p;
        let (lows, tops, bases) = (
            khz_of(cores, "min_khz"),
            khz_of(cores, "max_khz"),
            khz_of(cores, "base_khz"),
        );
        let span = |values: &[f64]| {
            let (low, high) = (lowest(values), highest(values));
            (low, high, (high - low).abs() > f64::EPSILON)
        };
        let mut rows = Vec::new();
        if !lows.is_empty() && !tops.is_empty() {
            let (top_least, top_most, differ) = span(&tops);
            let said = self.say.says(
                "cpu:clock-range",
                "side",
                &[
                    ("low", &self.ghz(lowest(&lows))),
                    ("high", &self.ghz(top_most)),
                ],
            );
            let note = if differ {
                let slowest = self.ghz(top_least);
                self.say
                    .says("cpu:clock-range", "kinds", &[("least", &slowest)])
            } else {
                String::new()
            };
            rows.push(self.figure("clock-range", said, None, note));
        }
        if !bases.is_empty() {
            let (low, high, differ) = span(&bases);
            let said = if differ {
                self.say.says(
                    "cpu:clock-base",
                    "range",
                    &[("low", &self.ghz(low)), ("high", &self.ghz(high))],
                )
            } else {
                self.ghz(low)
            };
            let note = self.say.says("cpu:clock-base", "note", &[]);
            rows.push(self.figure("clock-base", said, None, note));
        }
        for (field, id) in [
            ("driver", "clock-driver"),
            ("governor", "clock-governor"),
            ("preference", "clock-preference"),
        ] {
            if let Some(said) = p[field].as_str().filter(|said| !said.is_empty()) {
                rows.push(self.leaf(id, said));
            }
        }
        if let Some(turbo) = p["turbo"].as_bool() {
            let phrase = if turbo { "on" } else { "off" };
            let said = self
                .say
                .says("cpu:clock-turbo", &format!("side-{phrase}"), &[]);
            let note = self.say.says("cpu:clock-turbo", phrase, &[]);
            rows.push(self.figure("clock-turbo", said, None, note));
        }
        rows
    }

    /// How hot it is, drawn as Heat draws it, against the most it's meant
    /// to reach and warning near it; its detail is Heat's alone.
    fn temperature(&self) -> Option<Made> {
        let heat = &self.p["temperature"];
        let value = heat["temperature_c"].as_f64()?;
        let (high, critical) = (heat["high_c"].as_f64(), heat["critical_c"].as_f64());
        let level = parts::level(value, high, critical, "processor");
        let note = critical.or(high).map_or_else(String::new, |most| {
            let degrees = self.degrees(most);
            self.say
                .says("cpu:temperature", "most", &[("degrees", &degrees)])
        });
        Some(self.figure(
            "temperature",
            self.degrees(value),
            Some(bar(level.value, level.warn)),
            note,
        ))
    }

    fn using(&self, using: Option<&[Value]>) -> Made {
        let id = "cpu:using";
        let holders: Vec<&Value> = using
            .unwrap_or_default()
            .iter()
            .filter(|each| {
                each["key"]
                    .as_str()
                    .is_some_and(|key| key.starts_with("using:cpu:"))
            })
            .collect();
        let Some(first) = holders.first() else {
            return self.reading("using");
        };
        let inside = holders
            .iter()
            .enumerate()
            .map(|(at, each)| {
                let share = number(&each["share_ratio"]);
                let processes = each["processes"].as_u64().unwrap_or(1);
                let note = if processes > 1 {
                    self.say
                        .says(id, "processes", &[("n", &processes.to_string())])
                } else {
                    String::new()
                };
                Made {
                    name: each["name"].as_str().unwrap_or_default().to_owned(),
                    ..self.figure(
                        &format!("using:{}", at + 1),
                        percent(share),
                        Some(bar(share, false)),
                        note,
                    )
                }
            })
            .collect();
        let side = format!(
            "{} {}",
            first["name"].as_str().unwrap_or_default(),
            percent(number(&first["share_ratio"]))
        );
        let made = self.figure("using", side, None, self.say.says(id, "note", &[]));
        self.holder(made, inside)
    }

    /// Its load, what's running, and how often it switches, is
    /// interrupted and starts something.
    fn counts(&self) -> Vec<Made> {
        let p = self.p;
        let words = self.say.words;
        let mut rows = Vec::new();
        if let Some(load) = p["load"].as_array().filter(|load| load.len() == 3) {
            let said: Vec<String> = load
                .iter()
                .map(|each| words.number(number(each), 2))
                .collect();
            let note = self.say.says("cpu:load", "note", &[]);
            rows.push(self.figure("load", said.join(" \u{b7} "), None, note));
        }
        if let Some(running) = p["running"].as_f64() {
            let (blocked, tasks) = (number(&p["blocked"]), number(&p["tasks"]));
            let note = items(&[
                if blocked > 0.0 {
                    let blocked = words.number(blocked, 0);
                    self.say
                        .says("cpu:running", "blocked", &[("blocked", &blocked)])
                } else {
                    String::new()
                },
                if tasks > 0.0 {
                    let tasks = words.number(tasks, 0);
                    self.say.says("cpu:running", "tasks", &[("tasks", &tasks)])
                } else {
                    String::new()
                },
            ]);
            let running = words.number(running, 0);
            rows.push(self.figure("running", running, None, note));
        }
        if let Some(switches) = p["switches_per_s"].as_f64() {
            rows.push(self.figure("switches", self.say.often(switches), None, self.since()));
        }
        if let Some(interrupts) = p["interrupts_per_s"].as_f64() {
            let deferred = number(&p["softirqs_per_s"]);
            let note = if deferred > 0.0 {
                let deferred = self.say.often(deferred);
                self.say
                    .says("cpu:interrupts", "note", &[("deferred", &deferred)])
            } else {
                String::new()
            };
            rows.push(self.figure("interrupts", self.say.often(interrupts), None, note));
        }
        if let Some(forks) = p["forks_per_s"].as_f64() {
            let note = self.say.says("cpu:started", "note", &[]);
            rows.push(self.figure("started", self.say.often(forks), None, note));
        }
        rows
    }

    /// The NPU's load sits with the processors', so the two read as a
    /// pair (VKCF3Z, 2026-10-29).
    fn npu(&self) -> Option<Made> {
        let npu = self.p["npu"].as_object()?;
        let Some(busy) = npu.get("busy_ratio").and_then(Value::as_f64) else {
            return Some(self.reading("npu-load"));
        };
        let clock = npu.get("clock_mhz").map_or(0.0, number);
        let note = if clock > 0.0 {
            let most = npu.get("max_mhz").map_or(0.0, number);
            self.say.says(
                "cpu:npu-load",
                "at",
                &[
                    ("clock", &self.ghz(clock * MEGAHERTZ)),
                    ("most", &self.ghz(most * MEGAHERTZ)),
                ],
            )
        } else {
            self.say.says("cpu:npu-load", "resting", &[])
        };
        Some(self.figure("npu-load", percent(busy), Some(bar(busy, false)), note))
    }

    fn kinds(&self, cores: &[Core]) -> Option<Made> {
        let present: Vec<&str> = KINDS
            .into_iter()
            .filter(|kind| cores.iter().any(|core| core.kind() == *kind))
            .collect();
        if present.is_empty() {
            return None;
        }
        let of = |kind: &str| -> Vec<&Core> {
            cores.iter().filter(|core| core.kind() == kind).collect()
        };
        let inside = present
            .iter()
            .map(|kind| {
                let members = of(kind);
                let clocks: Vec<f64> = members
                    .iter()
                    .map(|core| core.khz("clock_khz"))
                    .filter(|clock| *clock > 0.0)
                    .collect();
                let n = members.len().to_string();
                let said = if clocks.is_empty() {
                    n
                } else {
                    let clock = self.ghz(mean(&clocks));
                    self.say
                        .says("cpu:kinds", "at", &[("n", &n), ("clock", &clock)])
                };
                let loads: Vec<f64> = members.iter().map(|core| core.load()).collect();
                let numbers: Vec<u64> = members.iter().map(|core| core.n()).collect();
                self.figure(
                    kind,
                    said,
                    Some(strip(&loads, false)),
                    self.cores_said(&numbers),
                )
            })
            .collect();
        let counts: Vec<String> = present
            .iter()
            .map(|kind| of(kind).len().to_string())
            .collect();
        let named: Vec<String> = present.iter().map(|kind| (*kind).to_owned()).collect();
        let made = self.figure("kinds", counts.join(" \u{b7} "), None, items(&named));
        Some(self.holder(made, inside))
    }

    /// What the processor is: who made it, its family, model and stepping
    /// as it says them, its microcode, and its caches.
    fn model(&self) -> Option<Made> {
        let p = self.p;
        let model = p["model"].as_str().filter(|model| !model.is_empty())?;
        let mut inside = vec![self.leaf("model-name", model)];
        if let Some(vendor) = p["vendor"].as_str().filter(|vendor| !vendor.is_empty()) {
            inside.push(self.leaf("vendor", vendor));
        }
        if let Some(family) = p["family"].as_u64() {
            let text = |field: &str| match &p[field] {
                Value::Number(n) => n.to_string(),
                Value::String(s) => s.clone(),
                _ => String::new(),
            };
            let said = self.say.says(
                "cpu:family",
                "side",
                &[
                    ("family", &family.to_string()),
                    ("model", &text("model_number")),
                    ("stepping", &text("stepping")),
                ],
            );
            inside.push(self.leaf("family", &said));
        }
        if let Some(microcode) = p["microcode"].as_str().filter(|code| !code.is_empty()) {
            inside.push(self.leaf("microcode", microcode));
        }
        inside.extend(self.caches());
        let made = self.figure("model", model.to_owned(), None, String::new());
        Some(self.holder(made, inside))
    }

    fn caches(&self) -> Option<Made> {
        let caches = self.p["caches"]
            .as_array()
            .filter(|caches| !caches.is_empty())?;
        let size = |cache: &Value| number(&cache["size_b"]);
        let count = |cache: &Value| cache["count"].as_u64().unwrap_or(1);
        let all: f64 = caches
            .iter()
            .map(|cache| size(cache) * f64::from(u32::try_from(count(cache)).unwrap_or(1)))
            .sum();
        let inside = caches
            .iter()
            .enumerate()
            .map(|(at, cache)| {
                let level = cache["level"].as_u64().unwrap_or(0).to_string();
                let kind = cache["type"].as_str().unwrap_or_default();
                let name = if CACHE_TYPES.contains(&kind) {
                    let id = format!("machine:cpu:cache-{kind}");
                    self.say.words.name(&id, &[("level", &level)])
                } else {
                    let fill = [("level", level.as_str()), ("type", kind)];
                    self.say.words.name("machine:cpu:cache", &fill)
                };
                let one = self.say.bytes(size(cache));
                let said = if count(cache) > 1 {
                    let n = count(cache).to_string();
                    self.say
                        .says("cpu:cache", "times", &[("size", &one), ("count", &n)])
                } else {
                    one
                };
                let shared = cache["shared_by"].as_u64().unwrap_or(1);
                let cpus: Vec<u64> = cache["cpus"]
                    .as_array()
                    .map(|cpus| cpus.iter().filter_map(Value::as_u64).collect())
                    .unwrap_or_default();
                let note = items(&[
                    if shared > 1 {
                        let n = shared.to_string();
                        self.say.says("cpu:cache", "shared", &[("n", &n)])
                    } else {
                        self.say.says("cpu:cache", "own", &[])
                    },
                    self.cores_said(&cpus),
                    cache["ways"].as_u64().map_or_else(String::new, |ways| {
                        let ways = ways.to_string();
                        self.say.says("cpu:cache", "ways", &[("ways", &ways)])
                    }),
                    cache["line_b"].as_u64().map_or_else(String::new, |line| {
                        let line = line.to_string();
                        self.say.says("cpu:cache", "line", &[("line", &line)])
                    }),
                ]);
                Made {
                    name,
                    ..self.figure(&format!("cache:{}", at + 1), said, None, note)
                }
            })
            .collect();
        let note = self.say.says("cpu:caches", "note", &[]);
        let made = self.figure("caches", self.say.bytes(all), None, note);
        Some(self.holder(made, inside))
    }
}

/// The cores' clocks of one field, those it says.
fn khz_of(cores: &[Core], field: &str) -> Vec<f64> {
    cores
        .iter()
        .map(|core| core.khz(field))
        .filter(|khz| *khz > 0.0)
        .collect()
}

/// How far a run of core numbers spans where it runs unbroken.
fn unbroken(numbers: &[u64]) -> Option<u64> {
    u64::try_from(numbers.len()).ok()?.checked_sub(1)
}

/// The mean of values there are some of.
fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / f64::from(u32::try_from(values.len()).unwrap_or(u32::MAX))
}

fn lowest(values: &[f64]) -> f64 {
    values.iter().copied().fold(f64::INFINITY, f64::min)
}

fn highest(values: &[f64]) -> f64 {
    values.iter().copied().fold(f64::NEG_INFINITY, f64::max)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// The words these cases say, as words.json has them.
    fn words() -> Words {
        let vocabulary = json!({ "things": [
            { "id": "machine:cpu", "name": "Processor", "glyph": "processor",
                "said": { "head": "{busy} busy", "since-boot": "since the machine started",
                    "since-asked": "since you last asked", "since-now": "just now" },
                "parts": ["machine:cpu:spent", "machine:cpu:cores", "machine:cpu:maxed",
                    "machine:cpu:clock", "machine:cpu:using", "machine:cpu:load",
                    "machine:cpu:npu-load", "machine:cpu:kinds", "machine:cpu:model"] },
            { "id": "machine:cpu:spent", "name": "Time Spent" },
            { "id": "machine:cpu:spent-user", "name": "Programs" },
            { "id": "machine:cpu:spent-stolen", "name": "Stolen" },
            { "id": "machine:cpu:cores", "name": "Cores", "said": { "one": "core {n}",
                "run": "cores {first} to {last}", "list": "cores {list} and {last}" } },
            { "id": "machine:cpu:core", "name": "Core {n}" },
            { "id": "machine:cpu:maxed", "name": "Maxed", "said": { "none": "none",
                "many": "{n} of {all}", "note": "at 95% or more" } },
            { "id": "machine:cpu:clock", "name": "Clock Speed",
                "said": { "of": "on average, of {most} at most", "mean": "on average" } },
            { "id": "machine:cpu:clock-range", "name": "Range", "said": {
                "side": "{low} to {high}", "kinds": "its slowest cores top out at {least}" } },
            { "id": "machine:cpu:clock-turbo", "name": "Turbo", "said": { "side-on": "on",
                "on": "it may clock past its base" } },
            { "id": "machine:cpu:temperature", "name": "Temperature",
                "said": { "degrees": "{n} \u{b0}C" } },
            { "id": "machine:cpu:using", "name": "Using It",
                "said": { "note": "of the whole processor", "processes": "{n} processes" } },
            { "id": "machine:cpu:load", "name": "Load" },
            { "id": "machine:cpu:npu-load", "name": "NPU Load", "said": {
                "at": "at {clock} of {most}", "resting": "resting" } },
            { "id": "machine:cpu:kinds", "name": "Kinds", "said": { "at": "{n} at {clock}" } },
            { "id": "machine:cpu:performance", "name": "Performance" },
            { "id": "machine:cpu:efficient", "name": "Efficient" },
            { "id": "machine:cpu:model", "name": "Model" },
            { "id": "machine:cpu:model-name", "name": "Name" },
            { "id": "machine:cpu:caches", "name": "Caches" },
            { "id": "machine:cpu:cache", "name": "L{level} {type}", "said": {
                "own": "one a core", "shared": "each shared by {n} cores",
                "times": "{size} \u{d7} {count}" } },
            { "id": "machine:cpu:cache-data", "name": "L{level} Data" }] });
        Words::new(&vocabulary, "", "")
    }

    fn core(n: u64, kind: &str, load: f64, clock_khz: u64) -> Value {
        json!({ "n": n, "kind": kind, "online": true, "busy_ratio": load,
            "clock_khz": clock_khz, "max_khz": 5_000_000, "min_khz": 400_000 })
    }

    /// Two performance cores, one of them maxed, and two efficient ones,
    /// on a machine of its own, with an NPU resting.
    fn processor() -> Value {
        json!({
            "key": "processor:0",
            "busy_ratio": 0.5,
            "since": "asked",
            "guest": false,
            "time": { "user": 0.3, "steal": 0.0 },
            "cores": [core(0, "performance", 0.97, 4_000_000),
                core(1, "performance", 0.2, 3_000_000),
                core(2, "efficient", 0.1, 2_000_000),
                core(3, "efficient", 0.0, 0),
                { "n": 4, "online": false }],
            "turbo": true,
            "load": [1.5, 1.25, 1.0],
            "npu": { "busy_ratio": 0.0, "clock_mhz": 0, "max_mhz": 1400 },
            "model": "Core Ultra 7 268V",
            "caches": [{ "level": 1, "type": "data", "size_b": 48_000, "count": 4,
                "shared_by": 1, "cpus": [0, 1, 2, 3] }]
        })
    }

    fn part<'a>(made: &'a Made, id: &str) -> &'a Made {
        made.inside
            .iter()
            .find(|part| part.id == id)
            .unwrap_or_else(|| {
                let ids: Vec<&str> = made.inside.iter().map(|part| part.id.as_str()).collect();
                panic!("no {id} among {ids:?}")
            })
    }

    fn said(made: &Made) -> &str {
        made.side[0]["said"].as_str().unwrap_or_default()
    }

    #[test]
    fn the_processors_row_says_how_busy_it_is_with_its_parts_in_its_words_order() {
        let made = row(&processor(), None, &words()).expect("a row while it says how busy");
        assert_eq!(said(&made), "50% busy", "its headline");
        assert_eq!(made.note, "since you last asked", "what its rates are over");
        assert_eq!(made.glyph, "processor", "its glyph");
        let ids: Vec<&str> = made.inside.iter().map(|part| part.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "cpu:spent",
                "cpu:cores",
                "cpu:maxed",
                "cpu:clock",
                "cpu:using",
                "cpu:load",
                "cpu:npu-load",
                "cpu:kinds",
                "cpu:model"
            ],
            "in its words' order"
        );
        assert!(
            row(&json!({ "busy_ratio": null }), None, &words()).is_none(),
            "no row while it hasn't said how busy, so the shell holds its place"
        );
    }

    #[test]
    fn each_online_core_is_a_part_and_the_maxed_are_named() {
        let made = row(&processor(), None, &words()).expect("a row");
        let cores = part(&made, "cpu:cores");
        assert_eq!(said(cores), "4", "the online cores counted");
        assert_eq!(cores.inside[0].name, "Core 0", "each named by its number");
        assert_eq!(
            cores.inside[0].note, "performance \u{b7} 4.00 GHz",
            "its kind and clock"
        );
        assert_eq!(
            said(part(&made, "cpu:maxed")),
            "core 0",
            "the one maxed core"
        );
        let kinds = part(&made, "cpu:kinds");
        assert_eq!(said(kinds), "2 \u{b7} 2", "the cores of each kind");
        assert_eq!(
            said(&kinds.inside[0]),
            "2 at 3.50 GHz",
            "a kind's mean clock"
        );
        assert_eq!(
            kinds.inside[1].note, "cores 2 to 3",
            "an unbroken run of cores"
        );
    }

    #[test]
    fn maxed_cores_are_named_while_few_and_counted_past_four() {
        let words = words();
        let cpu = Cpu {
            say: Say { words: &words },
            p: &Value::Null,
        };
        let loads: Vec<Value> = (0..6).map(|n| core(n, "", 1.0, 0)).collect();
        let cores = |n: usize| -> Vec<Core> { loads[..n].iter().map(Core).collect() };
        assert_eq!(said(&cpu.maxed(&cores(0))), "none", "none maxed");
        assert_eq!(said(&cpu.maxed(&cores(3))), "cores 0 to 2", "a run");
        assert_eq!(said(&cpu.maxed(&cores(6))), "6 of 6", "counted past four");
        assert_eq!(
            cpu.cores_said(&[0, 2, 5]),
            "cores 0, 2 and 5",
            "each named, broken"
        );
    }

    #[test]
    fn time_stolen_is_left_out_on_a_machine_of_its_own() {
        let made = row(&processor(), None, &words()).expect("a row");
        let spent = part(&made, "cpu:spent");
        let ids: Vec<&str> = spent.inside.iter().map(|part| part.id.as_str()).collect();
        assert_eq!(ids, ["cpu:spent-user"], "its own hardware's time alone");
        let mut guest = processor();
        guest["guest"] = json!(true);
        let made = row(&guest, None, &words()).expect("a row");
        assert_eq!(
            part(&made, "cpu:spent").inside.len(),
            2,
            "stolen inside another"
        );
    }

    #[test]
    fn the_npu_says_it_rests_and_holds_its_place_while_unread() {
        let made = row(&processor(), None, &words()).expect("a row");
        assert_eq!(part(&made, "cpu:npu-load").note, "resting", "powered down");
        let mut unread = processor();
        unread["npu"]["busy_ratio"] = Value::Null;
        let made = row(&unread, None, &words()).expect("a row");
        let npu = part(&made, "cpu:npu-load");
        assert_eq!(said(npu), "\u{2026}", "its place held");
        assert_eq!(npu.copies, "", "a placeholder copies nothing");
    }

    #[test]
    fn using_it_names_the_programs_only_while_they_were_read() {
        let made = row(&processor(), None, &words()).expect("a row");
        assert_eq!(said(part(&made, "cpu:using")), "\u{2026}", "still reading");
        let programs = [
            json!({ "key": "using:memory:1", "name": "firefox", "resident_b": 1e9 }),
            json!({ "key": "using:cpu:1", "name": "cargo", "share_ratio": 0.25,
                "processes": 3 }),
        ];
        let made = row(&processor(), Some(&programs), &words()).expect("a row");
        let using = part(&made, "cpu:using");
        assert_eq!(said(using), "cargo 25%", "the program using most");
        assert_eq!(using.inside.len(), 1, "the processor's programs alone");
        assert_eq!(using.inside[0].note, "3 processes", "its processes counted");
    }

    #[test]
    fn a_cache_says_its_size_and_how_many_there_are() {
        let made = row(&processor(), None, &words()).expect("a row");
        let caches = part(part(&made, "cpu:model"), "cpu:caches");
        assert_eq!(said(caches), "192 kB", "all of them together");
        let cache = &caches.inside[0];
        assert_eq!(
            cache.name, "L1 Data",
            "a type Linux names, in its own words"
        );
        assert_eq!(said(cache), "48.0 kB \u{d7} 4", "its size and count");
        assert_eq!(cache.note, "one a core \u{b7} cores 0 to 3", "whose it is");
    }
}
