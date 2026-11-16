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

/// The slices braked through a call: which manager each is on, `--user`
/// or `--system`, and the slice.
pub const BRAKED: [(&str, &str); 2] = [("--user", "jobs.slice"), ("--system", "build.slice")];

/// A quota systemd reads as none.
const UNLIMITED: u64 = u64::MAX;

/// A quota of a quarter of `processors`, at least one, in microseconds of
/// processor time each second, as systemd's `CPUQuotaPerSecUSec` takes.
pub fn quota(processors: usize) -> u64 {
    let quarter = u64::try_from(processors / 4).unwrap_or(u64::MAX).max(1);
    quarter.saturating_mul(1_000_000)
}

/// The quota to set now, given whether a call is `calling` and whether the
/// slices are `braked` already: none when they already stand as they
/// should.
pub fn wanted(calling: bool, braked: bool, processors: usize) -> Option<u64> {
    match (calling, braked) {
        (true, false) => Some(quota(processors)),
        (false, true) => Some(UNLIMITED),
        _ => None,
    }
}

/// What busctl is given to set `slice`'s quota on `manager` to `quota`, as
/// a runtime property.
pub fn quota_call(manager: &str, slice: &str, quota: u64) -> Vec<String> {
    let quota = quota.to_string();
    [
        manager,
        "--quiet",
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
        assert_eq!(
            wanted(true, false, 16),
            Some(4_000_000),
            "a call starting didn't hold the slices to a quarter of 16 processors"
        );
        assert_eq!(
            wanted(false, true, 16),
            Some(u64::MAX),
            "a call ending didn't give the slices every processor back"
        );
        assert_eq!(wanted(true, true, 16), None, "a held brake was set again");
        assert_eq!(
            wanted(false, false, 16),
            None,
            "slices with no call were set"
        );
    }

    #[test]
    fn a_small_machine_keeps_one_processor_for_its_builds() {
        assert_eq!(quota(2), 1_000_000, "two processors left builds none");
        assert_eq!(
            quota(12),
            3_000_000,
            "twelve processors gave builds other than 3"
        );
    }

    #[test]
    fn a_quota_is_set_on_the_slices_own_manager_for_this_boot_alone() {
        let args = quota_call("--system", "build.slice", 4_000_000);
        assert_eq!(
            &args[..3],
            ["--system", "--quiet", "call"],
            "not the system's manager"
        );
        assert_eq!(
            &args[8..],
            [
                "build.slice",
                "true",
                "1",
                "CPUQuotaPerSecUSec",
                "t",
                "4000000"
            ],
            "not build.slice's quota, as a runtime property"
        );
    }
}
