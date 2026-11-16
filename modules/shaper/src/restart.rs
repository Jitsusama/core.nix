//! Whether the shaper's unit is being restarted rather than stopped, so
//! its stop can leave the queues for the shaper it starts to take back.
//!
//! A switch restarts the shaper whenever its unit changes, and its stop
//! used to take every queue away, leaving the link unshaped until the new
//! shaper put them back: about 5 s on 16 November at 13:29, while the
//! modem's queue took the gateway's round trip from 10 to 30 ms to 102.
//! systemd says a restart's job is one, stop half included, so `systemctl
//! list-jobs` tells one from a stop. A restart systemd makes itself after
//! a crash has no job yet while the old one is cleaned up, so it takes the
//! queues away as a stop does.

/// Whether `jobs`, as `systemctl list-jobs --no-legend --plain --full`
/// says them (id, unit, type, state), hold a restart of `unit`.
pub fn pending(jobs: &str, unit: &str) -> bool {
    jobs.lines().any(|line| {
        let mut words = line.split_whitespace().skip(1);
        words.next() == Some(unit) && words.next() == Some("restart")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_restart_of_the_unit_is_pending() {
        let jobs = "4417 shaper.service restart running\n\
                    4420 shaper-record.service start waiting\n";
        assert!(pending(jobs, "shaper.service"), "its restart wasn't seen");
    }

    #[test]
    fn a_stop_or_anothers_restart_is_no_restart() {
        let stop = "4417 shaper.service stop running\n";
        assert!(
            !pending(stop, "shaper.service"),
            "a stop was read as a restart"
        );
        let other = "4417 shaper-record.service restart running\n";
        assert!(
            !pending(other, "shaper.service"),
            "another unit's restart was read as the shaper's"
        );
        assert!(
            !pending("", "shaper.service"),
            "no jobs at all read as a restart"
        );
    }
}
