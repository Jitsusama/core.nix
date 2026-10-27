//! When a filesystem needs a pass, and of what kind: a band is answered
//! at once, a band about to arrive is answered ahead of it, and routine
//! upkeep comes round only when the machine can spare it.

use crate::band::{Band, Thresholds};
use crate::clock::Awake;
use crate::moments::{Power, Pressure};
use crate::units::float;

/// How a filesystem's free space is being used up, in bytes a second:
/// averaged over about two minutes, so one burst doesn't set it but a
/// pass can answer a band before it arrives; and over about two hours, the
/// trend a person is told, since a disk whose builds write and free
/// gigabytes every few minutes swings by tens of gigabytes an hour over
/// two.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fill {
    rate: Option<f64>,
    last: Option<(f64, u64)>,
    trend: Trend,
}

/// The two-hour average, its weights summed so that before two hours have
/// passed it is the average of what was seen, not pulled towards zero.
#[derive(Clone, Debug, Default, PartialEq)]
struct Trend {
    weighted: f64,
    weight: f64,
    watched: f64,
}

impl Fill {
    /// The time over which the rate follows a change, in seconds.
    const SMOOTHING: f64 = 120.0;

    /// The least time a rate is measured over. A busy disk's free space
    /// jumps by what a writer flushes, so two readings moments apart, as
    /// steps come right after a pass, measure the flush, not the fill.
    const SHORTEST: f64 = 1.0;

    /// The time over which the trend follows a change, in seconds.
    const TREND: f64 = 7200.0;

    /// How long the trend is watched before it's told: less is the last
    /// few minutes' swing.
    const TREND_FIRST: f64 = 600.0;

    /// Takes a reading of free space at a monotonic time. One too soon
    /// after the last counted is passed over, and the next is measured
    /// from the last.
    pub fn observe(&mut self, now: f64, free: u64) {
        if let Some((then, _)) = self.last
            && now - then < Self::SHORTEST
        {
            return;
        }
        if let Some((then, was)) = self.last {
            let seconds = now - then;
            let sample = (float(was) - float(free)) / seconds;
            let weight = 1.0 - (-seconds / Self::SMOOTHING).exp();
            self.rate = Some(match self.rate {
                Some(rate) => rate + weight * (sample - rate),
                None => sample,
            });
            let kept = (-seconds / Self::TREND).exp();
            let trend = &mut self.trend;
            trend.weighted = trend.weighted * kept + sample * (1.0 - kept);
            trend.weight = trend.weight * kept + (1.0 - kept);
            trend.watched += seconds;
        }
        self.last = Some((now, free));
    }

    /// Bytes a second being used up; negative while space is coming back.
    pub fn rate(&self) -> Option<f64> {
        self.rate
    }

    /// Bytes a second being used up over the last two hours or so,
    /// negative while space is coming back; none until it's been watched
    /// ten minutes.
    pub fn trend(&self) -> Option<f64> {
        let trend = &self.trend;
        (trend.watched >= Self::TREND_FIRST && trend.weight > 0.0)
            .then(|| trend.weighted / trend.weight)
    }

    /// Seconds until free space falls below `below` at the trend, or none
    /// while it isn't falling, either over the trend or now: a disk getting
    /// room back isn't told it fills, however the last two hours went.
    pub fn steady_until(&self, free: u64, below: u64) -> Option<f64> {
        if !self.rate.is_some_and(|rate| rate > 0.0) {
            return None;
        }
        if free < below {
            return Some(0.0);
        }
        match self.trend() {
            Some(trend) if trend > 0.0 => Some(float(free - below) / trend),
            _ => None,
        }
    }

    /// Seconds until free space falls below `below` at the present rate,
    /// or none while it isn't falling.
    pub fn until(&self, free: u64, below: u64) -> Option<f64> {
        if free < below {
            return Some(0.0);
        }
        match self.rate {
            Some(rate) if rate > 0.0 => Some(float(free - below) / rate),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Rest,
    /// A healthy disk's routine work.
    Upkeep,
    /// Nix's garbage alone, once something has let it go.
    Collect,
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
/// The battery charge routine work needs off the charger, in percent.
const BATTERY_FLOOR: u8 = 50;
/// The share of time tasks may stall on the disk or on memory, in
/// percent over ten seconds, before work that can be skipped holds off.
/// Processors aren't counted: such work runs at idle priority and yields
/// them.
const STALL_CEILING: f64 = 10.0;

/// Whether the machine can spare routine work: on the charger or a battery over
/// half charged, not saving power, and awake for ten minutes. How busy it
/// is doesn't count, since a workstation that's always busy would never be
/// tidied; routine passes pace their deletes instead (`pace.rs`).
pub fn spare(power: Power, saver: bool, awake: &Awake, now: f64) -> bool {
    let settled = awake
        .spans()
        .last()
        .is_some_and(|(start, _)| now - start >= SETTLING);
    powered(power, saver) && settled
}

/// Whether the power allows work that can wait: on the charger or a battery
/// well charged, and not set to save it.
pub fn powered(power: Power, saver: bool) -> bool {
    let charged = power.on_charger || power.battery.is_some_and(|charge| charge > BATTERY_FLOOR);
    charged && !saver
}

/// Whether tasks stall on the disk and on memory little enough for work
/// that can be skipped altogether, such as deduplicating the store.
pub fn calm(pressure: Pressure) -> bool {
    pressure.io.some < STALL_CEILING && pressure.memory.some < STALL_CEILING
}
