//! When the machine can spare the guard's routine work.

/// How much of the last ten seconds tasks stalled on one resource, in
/// percent: some of them, and all of them at once.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stall {
    pub some: f64,
    pub full: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pressure {
    pub io: Stall,
    pub memory: Stall,
    pub cpu: Stall,
}

/// Where the machine's power comes from: mains, and the system battery's
/// charge in percent when it has one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Power {
    pub mains: bool,
    pub battery: Option<u8>,
}
