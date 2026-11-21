//! How long the machine has slept in this boot, so a tick tells a sleep or
//! a boot from a tick that was skipped or failed while awake: only the
//! kernel knows which a gap between ticks was.

use std::fs;

use serde::{Deserialize, Serialize};

/// The machine's sleep as the kernel reads it at one moment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asleep {
    /// The boot it was read in, so a boot is seen whatever the sleep reads:
    /// a boot after one that never slept reads no sleep either side.
    pub boot: String,
    /// Milliseconds asleep in that boot.
    pub millis: i64,
}

/// A clock's reading, as the kernel's `struct timespec` lays it out.
#[repr(C)]
struct Timespec {
    seconds: i64,
    nanoseconds: i64,
}

// std links libc, so this comes from it.
unsafe extern "C" {
    fn clock_gettime(clock: i32, time: *mut Timespec) -> i32;
}

const CLOCK_MONOTONIC: i32 = 1;
const CLOCK_BOOTTIME: i32 = 7;

/// A clock's reading in milliseconds, or None when it can't be read. The
/// part of a second is kept, so the gap between two clocks holds still
/// while the machine is awake rather than wavering by a second.
fn clock(id: i32) -> Option<i64> {
    let mut time = Timespec {
        seconds: 0,
        nanoseconds: 0,
    };
    // SAFETY: time is a valid timespec the kernel fills in.
    if unsafe { clock_gettime(id, &raw mut time) } != 0 {
        return None;
    }
    Some(time.seconds * 1000 + time.nanoseconds / 1_000_000)
}

/// The machine's sleep now: boot time counts sleep and monotonic time
/// doesn't, so the gap between them is the time spent asleep in this boot.
/// None when the boot or either clock can't be read.
pub fn now() -> Option<Asleep> {
    let boot = fs::read_to_string("/proc/sys/kernel/random/boot_id").ok()?;
    let millis = clock(CLOCK_BOOTTIME)? - clock(CLOCK_MONOTONIC)?;
    Some(Asleep {
        boot: boot.trim().to_owned(),
        millis: millis.max(0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_machine_reads_its_own_sleep_and_boot() {
        let asleep = now().expect("the kernel's boot or clocks couldn't be read");
        assert!(!asleep.boot.is_empty(), "the boot read empty");
        let again = now().expect("the kernel's boot or clocks couldn't be read again");
        assert!(
            again.boot == asleep.boot && (again.millis - asleep.millis).abs() < 1000,
            "two readings a moment apart differ: {asleep:?}, {again:?}"
        );
    }
}
