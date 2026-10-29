//! When the machine queue wants a builder: jobs a builder elsewhere could
//! take, held back for room this machine doesn't have.

use serde_json::Value;

/// What a job waits on that a builder elsewhere gives: a VM slot; a build
/// slot, which the build cap and CPU pressure beside VMs hold back as
/// `behind`; or its turn behind those. Memory isn't among them, since
/// evaluation stays here, nor planning, which is Nix being asked what to
/// build.
const RELIEVED: [&str; 3] = ["vms", "behind", "order"];

/// The kinds whose work Nix runs, and so could run elsewhere. A `vm-test`
/// runs its driver in the session itself, here.
const SENT: [&str; 2] = ["build", "vm-check"];

/// How old the queue's own reading may be and still be believed. It writes
/// one every few seconds while it runs, so an older one is a queue that
/// stopped, whose waiting jobs would otherwise keep a builder all day.
const STALE: u64 = 120;

/// Whether any job a builder could take has waited, at `now`, at least
/// `wait` seconds since it asked. A queue that can't be read, or that
/// stopped writing, wants nothing.
pub fn wants(queue: &str, now: u64, wait: u64) -> bool {
    let Ok(queue) = serde_json::from_str::<Value>(queue) else {
        return false;
    };
    let fresh = queue["at"]
        .as_str()
        .and_then(seconds_of)
        .is_some_and(|at| now.saturating_sub(at) <= STALE);
    fresh
        && queue["jobs"].as_array().is_some_and(|jobs| {
            jobs.iter().any(|job| {
                job["state"] == "waiting"
                    && RELIEVED.iter().any(|on| job["waitingOn"] == *on)
                    && SENT.iter().any(|kind| job["kind"] == *kind)
                    && job["since"]
                        .as_str()
                        .and_then(seconds_of)
                        .is_some_and(|since| now.saturating_sub(since) >= wait)
            })
        })
}

/// Seconds since the epoch of a time as the queue writes it,
/// `2026-10-29T14:57:57.000Z`, always in UTC.
pub fn seconds_of(time: &str) -> Option<u64> {
    let number = |range: std::ops::Range<usize>| time.get(range)?.parse::<u64>().ok();
    let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
    let (hour, minute, second) = (number(11..13)?, number(14..16)?, number(17..19)?);
    if !(1..=12).contains(&month) || time.get(10..11)? != "T" {
        return None;
    }
    // Days from the epoch to a civil date, as Howard Hinnant's
    // days_from_civil counts them, in a year starting in March.
    let (year, month) = if month <= 2 {
        (year - 1, month + 9)
    } else {
        (year, month - 3)
    };
    let era = year / 400;
    let of_era = year - era * 400;
    let of_year = (153 * month + 2) / 5 + day - 1;
    let of_cycle = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    let days = (era * 146_097 + of_cycle).checked_sub(719_468)?;
    Some(days * 86_400 + hour * 3600 + minute * 60 + second)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-10-29 at 15:00:00 UTC.
    const NOW: u64 = 1_793_286_000;

    fn queue(jobs: &str) -> String {
        queue_at("2026-10-29T14:59:50.000Z", jobs)
    }

    fn queue_at(at: &str, jobs: &str) -> String {
        format!(r#"{{"v":1,"at":"{at}","jobs":[{jobs}]}}"#)
    }

    fn job(kind: &str, state: &str, on: &str, asked: u64) -> String {
        let since = time_of(asked);
        format!(r#"{{"kind":"{kind}","state":"{state}","waitingOn":"{on}","since":"{since}"}}"#)
    }

    /// The queue's form of a time in the hour before `NOW`, to the minute.
    fn time_of(seconds: u64) -> String {
        let before = (NOW - seconds) / 60;
        format!("2026-10-29T14:{:02}:00.000Z", 60 - before)
    }

    #[test]
    fn a_build_held_for_a_slot_long_enough_wants_a_builder() {
        assert!(wants(
            &queue(&job("build", "waiting", "order", NOW - 300)),
            NOW,
            180
        ));
        assert!(wants(
            &queue(&job("vm-check", "waiting", "vms", NOW - 180)),
            NOW,
            180
        ));
        assert!(wants(
            &queue(&job("build", "waiting", "behind", NOW - 240)),
            NOW,
            180
        ));
    }

    #[test]
    fn a_job_that_just_asked_wants_nothing_yet() {
        assert!(!wants(
            &queue(&job("build", "waiting", "order", NOW - 60)),
            NOW,
            180
        ));
    }

    #[test]
    fn only_what_a_builder_relieves_wants_one() {
        for job in [
            job("build", "waiting", "memory", NOW - 600),
            job("build", "waiting", "planning", NOW - 600),
            job("build", "running", "order", NOW - 600),
            job("vm-test", "waiting", "vms", NOW - 600),
        ] {
            assert!(!wants(&queue(&job), NOW, 180), "{job}");
        }
    }

    #[test]
    fn one_job_wanting_is_enough() {
        let jobs = [
            job("build", "waiting", "memory", NOW - 600),
            job("vm-check", "waiting", "vms", NOW - 600),
        ]
        .join(",");
        assert!(wants(&queue(&jobs), NOW, 180));
    }

    #[test]
    fn a_queue_that_stopped_writing_wants_nothing() {
        let waiting = job("build", "waiting", "order", NOW - 600);
        assert!(!wants(
            &queue_at("2026-10-29T14:57:00.000Z", &waiting),
            NOW,
            180
        ));
        assert!(!wants(&format!(r#"{{"jobs":[{waiting}]}}"#), NOW, 180));
    }

    #[test]
    fn a_queue_that_cant_be_read_wants_nothing() {
        assert!(!wants("", NOW, 180));
        assert!(!wants(r#"{"v":1}"#, NOW, 180));
    }

    #[test]
    fn a_queues_times_read_as_the_epoch_counts_them() {
        assert_eq!(seconds_of("1970-01-01T00:00:00.000Z"), Some(0));
        assert_eq!(seconds_of("2026-10-29T14:57:57.000Z"), Some(1_793_285_877));
        assert_eq!(seconds_of("2024-02-29T12:00:00Z"), Some(1_709_208_000));
        assert_eq!(seconds_of("not a time"), None);
    }
}
