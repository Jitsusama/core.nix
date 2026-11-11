//! How each running node keeps time, read from PipeWire's profiler as
//! pw-top reads it (its `profiler_profile` and `print_node`), and served
//! as a `timing` record each, so Sound's drawers say a node's Dropouts,
//! Load and Cycle with no pw-top running beside the shell.
//!
//! The profiler says, many times a second, one object for each driver's
//! cycle: the driver's clock, then a block for the driver and one for each
//! node following it. What a second of them says is kept, each node's last
//! cycle in it standing for the node, and it stands whole as the daemon's
//! timer says the second is out, as pw-top prints a frame each second. A
//! node that ran in none of a second's cycles is no longer running, and
//! goes; so does every node once the whole graph idles, since then the
//! profiler says nothing at all and the second stands empty.

use std::collections::BTreeMap;

use pipewire::spa::pod::Value;
use pipewire::spa::pod::deserialize::PodDeserializer;
use pipewire::spa::sys;
use pipewire::spa::utils::Fraction;
use serde_json::{Value as Json, json};

const NANOS: f64 = 1e9;
const MILLIS: f64 = 1e3;

/// How one node kept time in its last cycle.
#[derive(Clone, Debug, PartialEq)]
pub struct Timed {
    pub id: u32,
    /// The frames its driver moves each cycle, and the clock's rate.
    pub quantum: u64,
    pub rate: u32,
    /// Its busy time over the cycle, or none where it didn't wake.
    pub busy: Option<f64>,
    /// Whether it woke and hadn't finished when the cycle ended, so it
    /// took more than all of it, pw-top's "+++".
    pub unfinished: bool,
    /// Its xruns since it began.
    pub errors: u32,
    /// Whether it drives its cycle, rather than following a driver.
    pub drives: bool,
}

impl Timed {
    /// Its `timing` record: the figures as heard, the cycle's length in
    /// milliseconds among them, never words.
    pub fn record(&self) -> Json {
        json!({
            "key": format!("timing:{}", self.id), "kind": "timing", "id": self.id,
            "quantum": self.quantum, "rate": self.rate, "ms": self.ms(), "busy": self.busy,
            "unfinished": self.unfinished, "errors": self.errors, "drives": self.drives,
        })
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "a cycle's frames are a few thousand"
    )]
    fn ms(&self) -> f64 {
        self.quantum as f64 / f64::from(self.rate) * MILLIS
    }
}

/// The frame standing, and the one gathering.
#[derive(Debug, Default)]
pub struct Timings {
    stood: BTreeMap<u32, Timed>,
    gathering: BTreeMap<u32, Timed>,
}

impl Timings {
    /// Takes one profile into the frame gathering.
    pub fn hear(&mut self, profile: &[u8]) {
        for timed in profiled(profile) {
            self.gathering.insert(timed.id, timed);
        }
    }

    /// Stands what was gathered since the last, as a second is out,
    /// saying whether the records changed, so they're told again only
    /// then. A second the profiler said nothing in stands empty.
    pub fn stand(&mut self) -> bool {
        let frame = std::mem::take(&mut self.gathering);
        let changed = frame != self.stood;
        self.stood = frame;
        changed
    }

    /// Each node's `timing` record, by its id.
    pub fn records(&self) -> Vec<Json> {
        self.stood.values().map(Timed::record).collect()
    }

    /// Lets go of every frame, as no watch wants them.
    pub fn forget(&mut self) {
        *self = Self::default();
    }
}

/// Every node one profile says ran, each driver's cycle in the order said,
/// so a node said in two keeps the later.
pub fn profiled(profile: &[u8]) -> Vec<Timed> {
    let Ok((_, Value::Struct(objects))) = PodDeserializer::deserialize_any_from(profile) else {
        return Vec::new();
    };
    let mut heard = Vec::new();
    for object in objects {
        let Value::Object(object) = object else {
            continue;
        };
        if object.type_ != sys::SPA_TYPE_OBJECT_Profiler {
            continue;
        }
        let mut cycle = Cycle::default();
        for property in &object.properties {
            match property.key {
                sys::SPA_PROFILER_info => cycle.xruns = info_xruns(&property.value),
                sys::SPA_PROFILER_clock => cycle.clock = clock(&property.value),
                sys::SPA_PROFILER_driverBlock => {
                    heard.extend(block(&property.value, &cycle, true));
                }
                sys::SPA_PROFILER_followerBlock => {
                    heard.extend(block(&property.value, &cycle, false));
                }
                _ => {}
            }
        }
    }
    heard
}

/// What a driver's cycle says before its blocks: its clock's duration and
/// rate, and the driver's xruns.
#[derive(Default)]
struct Cycle {
    clock: Option<(u64, Fraction)>,
    xruns: Option<i32>,
}

fn fields(value: &Value) -> &[Value] {
    match value {
        Value::Struct(fields) => fields,
        _ => &[],
    }
}

fn int(fields: &[Value], at: usize) -> Option<i32> {
    match fields.get(at) {
        Some(Value::Int(n)) => Some(*n),
        _ => None,
    }
}

fn long(fields: &[Value], at: usize) -> Option<i64> {
    match fields.get(at) {
        Some(Value::Long(n)) => Some(*n),
        _ => None,
    }
}

/// Info: counter, three loads, and the xruns, fifth.
fn info_xruns(value: &Value) -> Option<i32> {
    int(fields(value), 4)
}

/// Clock: flags, id, name, nsec, rate, position, then duration.
fn clock(value: &Value) -> Option<(u64, Fraction)> {
    let fields = fields(value);
    let Some(Value::Fraction(rate)) = fields.get(4) else {
        return None;
    };
    let duration = u64::try_from(long(fields, 6)?).ok()?;
    Some((duration, *rate))
}

/// One node's block: id, name, `prev_signal`, signal, awake, finish,
/// status, latency, and its own xruns where it says them. Its cycle is
/// its driver's, as pw-top says a follower's: the clock's duration over
/// its rate, so one with no clock said has none, and isn't heard.
fn block(value: &Value, cycle: &Cycle, drives: bool) -> Option<Timed> {
    let fields = fields(value);
    let id = u32::try_from(int(fields, 0)?).ok()?;
    let (prev, awake, finish) = (long(fields, 2)?, long(fields, 4)?, long(fields, 5)?);
    let (duration, rate) = cycle.clock?;
    let quantum = duration.checked_mul(u64::from(rate.num))?;
    if quantum == 0 || rate.denom == 0 {
        return None;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "a cycle's frames are a few thousand"
    )]
    let period = quantum as f64 / f64::from(rate.denom) * NANOS;
    let (busy, unfinished) = if finish >= awake {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a cycle's nanoseconds, under 2^53"
        )]
        let took = (finish - awake) as f64;
        (Some(took / period), false)
    } else {
        (None, awake > prev)
    };
    let errors = int(fields, 8).or(cycle.xruns).unwrap_or(0);
    Some(Timed {
        id,
        quantum,
        rate: rate.denom,
        busy,
        unfinished,
        errors: u32::try_from(errors).unwrap_or(0),
        drives,
    })
}

#[cfg(test)]
#[path = "timing_tests.rs"]
mod tests;
