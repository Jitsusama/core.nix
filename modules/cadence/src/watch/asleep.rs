//! How long the machine has slept since it booted, so a tick tells a sleep
//! from a tick that was skipped or failed while awake: only the kernel's
//! clocks know which a gap between ticks was.

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

/// A clock's reading in seconds, or None when it can't be read.
fn clock(id: i32) -> Option<i64> {
    let mut time = Timespec {
        seconds: 0,
        nanoseconds: 0,
    };
    // SAFETY: time is a valid timespec the kernel fills in.
    if unsafe { clock_gettime(id, &raw mut time) } != 0 {
        return None;
    }
    Some(time.seconds)
}

/// Seconds asleep since boot: boot time counts sleep and monotonic time
/// doesn't, so the gap between them is the time spent asleep. None when
/// either clock can't be read.
pub fn since_boot() -> Option<i64> {
    Some(clock(CLOCK_BOOTTIME)? - clock(CLOCK_MONOTONIC)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_machine_reads_its_own_sleep() {
        let asleep = since_boot().expect("the kernel's clocks couldn't be read");
        assert!(asleep >= 0, "boot time read behind monotonic time");
    }
}
