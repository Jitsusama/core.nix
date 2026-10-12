//! Awake time: how long the machine has been on and awake since a moment.
//!
//! Idle windows count awake time rather than wall time, so a build left on
//! a Friday evening is as fresh on Monday morning as it was when the lid
//! closed. The daemons read the clocks every few seconds and keep the spans
//! the machine was awake, in wall time, so they survive restarts and reboots.

use serde::{Deserialize, Serialize};

/// A local time, as year, month, day, hour, minute and second, in wall
/// seconds. It goes through the C library so the zone and its daylight
/// saving are read as everything else on the machine reads them; a day
/// past a month's end rolls into the next, as `mktime` normalizes it.
pub(crate) fn local(fields: [i32; 6]) -> Option<f64> {
    let [year, month, day, hour, minute, second] = fields;
    // SAFETY: tm is plain data, zeroed is a valid value, and mktime reads
    // and normalizes only the struct it's given.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    tm.tm_year = year - 1900;
    tm.tm_mon = month - 1;
    tm.tm_mday = day;
    tm.tm_hour = hour;
    tm.tm_min = minute;
    tm.tm_sec = second;
    // Whether daylight saving applies is for mktime to work out.
    tm.tm_isdst = -1;
    // SAFETY: as above.
    let seconds = unsafe { libc::mktime(&mut tm) };
    (seconds != -1).then_some(seconds as f64)
}

/// One reading of the clocks, in seconds: wall time, time since boot
/// without suspends, time spent suspended since boot, and the boot's id.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tick {
    pub wall: f64,
    pub monotonic: f64,
    pub suspended: f64,
    pub boot: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Awake {
    /// Spans the machine was awake, as wall times, oldest first.
    spans: Vec<(f64, f64)>,
    last: Option<Tick>,
}

/// Suspended time growing by more than this between ticks was a suspend,
/// rather than the clocks' own drift.
const SUSPEND_SECONDS: f64 = 1.0;

impl Awake {
    pub fn observe(&mut self, tick: &Tick) {
        match self.last.take() {
            Some(last) if last.boot == tick.boot && tick.wall >= last.wall => {
                let awake = (tick.monotonic - last.monotonic).max(0.0);
                if tick.suspended - last.suspended > SUSPEND_SECONDS {
                    self.open(tick.wall - awake, tick.wall);
                } else if let Some(span) = self.spans.last_mut() {
                    // Within one boot and with no suspend, a gap between
                    // ticks is the daemon restarting, and was awake.
                    span.1 = tick.wall;
                } else {
                    self.open(tick.wall - awake, tick.wall);
                }
            }
            Some(last) if last.boot == tick.boot => {
                // The wall clock was set back: whatever was recorded past
                // it never happened as far as this clock knows.
                self.spans.retain(|span| span.0 <= tick.wall);
                for span in &mut self.spans {
                    span.1 = span.1.min(tick.wall);
                }
                let awake = (tick.monotonic - last.monotonic).max(0.0);
                self.open(tick.wall - awake, tick.wall);
            }
            _ => {
                // A boot this hasn't seen: it has been awake for its
                // monotonic time, which suspends don't advance.
                self.open(tick.wall - tick.monotonic.max(0.0), tick.wall);
            }
        }
        self.last = Some(tick.clone());
    }

    fn open(&mut self, start: f64, end: f64) {
        let after = self.spans.last().map_or(f64::NEG_INFINITY, |span| span.1);
        self.spans.push((start.max(after).min(end), end));
    }

    /// The awake time from `then` to `now`, in seconds. Before the first
    /// recorded span nothing is known, so wall time stands in.
    pub fn since(&self, then: f64, now: f64) -> f64 {
        if now <= then {
            return 0.0;
        }
        let first = self.spans.first().map_or(f64::INFINITY, |span| span.0);
        let unrecorded = (now.min(first) - then).max(0.0);
        let recorded: f64 = self
            .spans
            .iter()
            .map(|(start, end)| (end.min(now) - start.max(then)).max(0.0))
            .sum();
        unrecorded + recorded
    }

    /// Drops spans that ended before `wall`, which only ever count towards
    /// windows longer than any band keeps.
    pub fn forget_before(&mut self, wall: f64) {
        self.spans.retain(|span| span.1 >= wall);
    }

    pub fn spans(&self) -> &[(f64, f64)] {
        &self.spans
    }
}
