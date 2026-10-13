//! When a filesystem needs a pass, and of what kind: a band is answered
//! at once, a band about to arrive is answered ahead of it, and routine
//! upkeep comes round only when the machine can spare it.

use crate::band::{Band, Thresholds};
use crate::clock::Awake;
use crate::moments::{Power, Pressure};

/// How a filesystem's free space is being used up, in bytes a second,
/// averaged over about two minutes so one burst doesn't set it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fill {
    rate: Option<f64>,
    last: Option<(f64, u64)>,
}

impl Fill {
    /// The time over which the rate follows a change, in seconds.
    const SMOOTHING: f64 = 120.0;

    /// Takes a reading of free space at a monotonic time.
    pub fn observe(&mut self, now: f64, free: u64) {
        if let Some((then, was)) = self.last
            && now > then
        {
            let seconds = now - then;
            let sample = (was as f64 - free as f64) / seconds;
            let weight = 1.0 - (-seconds / Self::SMOOTHING).exp();
            self.rate = Some(match self.rate {
                Some(rate) => rate + weight * (sample - rate),
                None => sample,
            });
        }
        self.last = Some((now, free));
    }

    /// Bytes a second being used up; negative while space is coming back.
    pub fn rate(&self) -> Option<f64> {
        self.rate
    }

    /// Seconds until free space falls below `below` at the present rate,
    /// or none while it isn't falling.
    pub fn until(&self, free: u64, below: u64) -> Option<f64> {
        if free < below {
            return Some(0.0);
        }
        match self.rate {
            Some(rate) if rate > 0.0 => Some((free - below) as f64 / rate),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Rest,
    /// A healthy disk's routine work.
    Upkeep,
    /// Free space back to the target, planned by this band's windows.
    Reach(Band),
}

/// What a filesystem's decision is made from.
pub struct Situation<'a> {
    pub band: Band,
    pub free: u64,
    pub thresholds: Thresholds,
    pub fill: &'a Fill,
    /// How long a pass is expected to take, in seconds.
    pub pass_seconds: f64,
    /// When upkeep last ran, in wall seconds.
    pub upkept: Option<f64>,
    /// Wall seconds.
    pub now: f64,
    /// Whether the machine can spare routine work now.
    pub spare: bool,
}

/// How long before a forecast band a pass starts, beyond the time a pass
/// takes, so it's done before the band arrives.
const FORECAST_MARGIN: f64 = 300.0;

/// How often routine upkeep comes round.
const UPKEEP_EVERY: f64 = 6.0 * 3600.0;

pub fn decide(situation: &Situation) -> Decision {
    let next = match situation.band {
        Band::Healthy => Some((Band::Pressure, situation.thresholds.pressure)),
        Band::Pressure => Some((Band::Urgent, situation.thresholds.urgent)),
        Band::Urgent => Some((Band::Critical, situation.thresholds.critical)),
        Band::Critical => None,
    };
    if let Some((band, start)) = next
        && situation
            .fill
            .until(situation.free, start)
            .is_some_and(|until| until < situation.pass_seconds + FORECAST_MARGIN)
    {
        return Decision::Reach(band);
    }
    if situation.band > Band::Healthy {
        return Decision::Reach(situation.band);
    }
    let due = situation
        .upkept
        .is_none_or(|upkept| situation.now - upkept >= UPKEEP_EVERY);
    if due && situation.spare {
        Decision::Upkeep
    } else {
        Decision::Rest
    }
}

/// How long after boot or resume routine work waits, so opening the lid is
/// never met with a walk.
const SETTLING: f64 = 600.0;
/// The battery charge routine work needs off mains, in percent.
const BATTERY_FLOOR: u8 = 50;
/// The share of time tasks may stall on the disk or on memory, in
/// percent over ten seconds, before routine work holds off. Processors
/// aren't counted: routine work runs at idle priority and yields them.
const STALL_CEILING: f64 = 10.0;

/// Whether the machine can spare routine work: on mains or a battery over
/// half charged, not saving power, awake for ten minutes, and not stalling
/// on the disk or memory.
pub fn spare(power: Power, saver: bool, awake: &Awake, now: f64, pressure: Pressure) -> bool {
    let settled = awake
        .spans()
        .last()
        .is_some_and(|(start, _)| now - start >= SETTLING);
    powered(power, saver) && settled && calm(pressure)
}

/// Whether the power allows work that can wait: on mains or a battery
/// well charged, and not set to save it.
pub fn powered(power: Power, saver: bool) -> bool {
    let charged = power.mains || power.battery.is_some_and(|charge| charge > BATTERY_FLOOR);
    charged && !saver
}

/// Whether tasks stall on the disk and on memory little enough for work
/// that can wait.
pub fn calm(pressure: Pressure) -> bool {
    pressure.io.some < STALL_CEILING && pressure.memory.some < STALL_CEILING
}
