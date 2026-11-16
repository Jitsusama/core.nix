//! Brakes the system's builds and agents' jobs while a call is on, so a
//! call never shares the processors with them.
//!
//! At 13:09 on 16 November a call ran beside a load of 63 to 105, with
//! the processors' some-pressure at 63% over a minute: Nix building work
//! here in build.slice and agents' work in jobs.slice. Their low weights
//! give way to the call's app only once it asks, and a call's audio and
//! video ask in bursts too short to win back a processor taken by a
//! compile. So through a call each of the two is held to a quarter of
//! the processors by a quota, and given them all back once it ends, as
//! [`calls::Call`] says, a few seconds after the microphone closes.
//!
//! Both are set as runtime properties, gone at the next boot: jobs.slice
//! on the account's own manager, build.slice on the system's, which
//! work-slices.rules lets an administrator set without a password.
//!
//! Each slice is followed apart, since one can be set while the other
//! fails, and starts unknown: a sorter that stopped mid-call may have left
//! a brake on, so a new one with no call lets both go. The unit lets them
//! go as it stops, too.

use std::time::{Duration, Instant};

/// The slices braked through a call: which manager each is on, `--user`
/// or `--system`, and the slice.
pub const BRAKED: [(&str, &str); 2] = [("--user", "jobs.slice"), ("--system", "build.slice")];

/// A quota systemd reads as none.
const UNLIMITED: u64 = u64::MAX;

/// How long a slice that couldn't be set waits to be tried again, so a
/// refusal costs two busctl runs and two journal lines a minute rather
/// than a second.
const AGAIN: Duration = Duration::from_secs(30);

/// A quota of a quarter of `processors`, rounded up, in microseconds of
/// processor time each second, as systemd's `CPUQuotaPerSecUSec` takes.
pub fn quota(processors: usize) -> u64 {
    let quarter = u64::try_from(processors.div_ceil(4))
        .unwrap_or(u64::MAX)
        .max(1);
    quarter.saturating_mul(1_000_000)
}

/// The quota a slice is given while `calling` or not.
pub fn quota_for(calling: bool, processors: usize) -> u64 {
    if calling {
        quota(processors)
    } else {
        UNLIMITED
    }
}

/// Where the brake stands on each slice of [`BRAKED`], by its place there:
/// braked or let go as last set, or unknown, and when setting it last
/// failed.
#[derive(Default)]
pub struct Brake {
    stands: [Option<bool>; 2],
    failed: [Option<Instant>; 2],
}

impl Brake {
    /// The places in [`BRAKED`] of the slices to set `now`, with a call
    /// `calling` or not: those not already standing so, but for one that
    /// failed less than [`AGAIN`] ago.
    pub fn due(&self, calling: bool, now: Instant) -> Vec<usize> {
        (0..BRAKED.len())
            .filter(|&slice| self.stands[slice] != Some(calling))
            .filter(|&slice| {
                self.failed[slice].is_none_or(|at| now.saturating_duration_since(at) >= AGAIN)
            })
            .collect()
    }

    /// The slice at `slice` now stands as `calling` asks.
    pub fn set(&mut self, slice: usize, calling: bool) {
        self.stands[slice] = Some(calling);
        self.failed[slice] = None;
    }

    /// Setting the slice at `slice` failed `now`.
    pub fn failed(&mut self, slice: usize, now: Instant) {
        self.failed[slice] = Some(now);
    }

    /// Whether every slice stands as `calling` asks.
    pub fn stands(&self, calling: bool) -> bool {
        self.stands.iter().all(|stands| *stands == Some(calling))
    }
}

/// What busctl is given to set `slice`'s quota on `manager` to `quota`, as
/// a runtime property, refused rather than asking for a password, since
/// the sorter's loop would wait on one.
pub fn quota_call(manager: &str, slice: &str, quota: u64) -> Vec<String> {
    let quota = quota.to_string();
    [
        manager,
        "--quiet",
        "--allow-interactive-authorization=no",
        "call",
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
        "SetUnitProperties",
        "sba(sv)",
        slice,
        "true",
        "1",
        "CPUQuotaPerSecUSec",
        "t",
        &quota,
    ]
    .iter()
    .map(|arg| (*arg).to_string())
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_call_starting_brakes_and_its_end_lets_go() {
        let now = Instant::now();
        let mut brake = Brake::default();
        brake.set(0, false);
        brake.set(1, false);
        assert!(
            brake.due(false, now).is_empty(),
            "slices with no call were set"
        );
        assert_eq!(
            brake.due(true, now),
            [0, 1],
            "a call starting braked not both"
        );
        brake.set(0, true);
        brake.set(1, true);
        assert!(brake.stands(true), "both set didn't stand braked");
        assert!(
            brake.due(true, now).is_empty(),
            "a held brake was set again"
        );
        assert_eq!(
            brake.due(false, now),
            [0, 1],
            "a call ending let go of not both"
        );
        assert_eq!(quota_for(true, 16), 4_000_000, "a call didn't hold 16 to 4");
        assert_eq!(
            quota_for(false, 16),
            u64::MAX,
            "no call didn't give all back"
        );
    }

    // A sorter that stopped mid-call may have left a brake on.
    #[test]
    fn a_new_sorter_with_no_call_lets_go_of_both() {
        let brake = Brake::default();
        assert_eq!(
            brake.due(false, Instant::now()),
            [0, 1],
            "a new sorter left what another may have braked"
        );
    }

    #[test]
    fn a_half_set_brake_is_let_go_and_a_refusal_waits() {
        let now = Instant::now();
        let mut brake = Brake::default();
        brake.set(0, true);
        brake.failed(1, now);
        assert_eq!(
            brake.due(false, now + Duration::from_secs(1)),
            [0],
            "the braked half wasn't let go, or the refused one was tried at once"
        );
        assert_eq!(
            brake.due(false, now + AGAIN),
            [0, 1],
            "a refused slice wasn't tried again once it had waited"
        );
        assert!(!brake.stands(true), "a half-set brake stood as braked");
    }

    #[test]
    fn a_small_machine_keeps_one_processor_for_its_builds() {
        assert_eq!(quota(2), 1_000_000, "two processors left builds none");
        assert_eq!(
            quota(7),
            2_000_000,
            "seven processors gave builds other than 2"
        );
        assert_eq!(
            quota(12),
            3_000_000,
            "twelve processors gave builds other than 3"
        );
    }

    #[test]
    fn a_quota_is_set_on_the_slices_own_manager_for_this_boot_alone() {
        assert_eq!(
            quota_call("--system", "build.slice", 4_000_000),
            [
                "--system",
                "--quiet",
                "--allow-interactive-authorization=no",
                "call",
                "org.freedesktop.systemd1",
                "/org/freedesktop/systemd1",
                "org.freedesktop.systemd1.Manager",
                "SetUnitProperties",
                "sba(sv)",
                "build.slice",
                "true",
                "1",
                "CPUQuotaPerSecUSec",
                "t",
                "4000000"
            ],
            "not build.slice's quota on the system's manager, for this boot, unasked"
        );
    }
}
