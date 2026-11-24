//! When the machine can spare the guard's routine work.

/// How much tasks stalled on one resource, in percent: some of them and
/// all of them at once over the last ten seconds, and all of them at once
/// over the last minute, which work that runs for minutes starts by, since
/// a quiet ten seconds says little of the next ten minutes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stall {
    pub some: f64,
    pub full: f64,
    pub full_minute: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pressure {
    pub io: Stall,
    pub memory: Stall,
    pub cpu: Stall,
}

/// Where the machine's power comes from: whether it's on the charger, and
/// the system battery's charge in percent when it has one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Power {
    pub on_charger: bool,
    pub battery: Option<u8>,
}
