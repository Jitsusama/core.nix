//! What shaping costs the processor, for `shaper status`.
//!
//! The shaper's own process wakes five times a second and is cheap; what
//! costs is CAKE and the IFB, which run in the kernel's softirq time, on
//! whichever core the packets arrive on. Plain CAKE holds one lock per
//! queue, so a core that spends most of its time there is the ceiling,
//! however idle the rest are. So status shows the softirq share of the
//! whole machine and of its busiest core, as an upper bound: the network
//! card's and the disk's softirqs count there too.

use crate::rate::float;

/// What a core, or every core together, spent its time on, in ticks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ticks {
    pub softirq: u64,
    pub total: u64,
}

/// The machine's ticks first, then each core's, from `/proc/stat`.
pub fn ticks(stat: &str) -> Vec<Ticks> {
    stat.lines()
        .filter(|line| line.starts_with("cpu"))
        .map(|line| {
            // user nice system idle iowait irq softirq steal: guest time is
            // already counted in user, so it's left out of the total.
            let fields: Vec<u64> = line
                .split_whitespace()
                .skip(1)
                .take(8)
                .map(|n| n.parse().unwrap_or(0))
                .collect();
            Ticks {
                softirq: fields.get(6).copied().unwrap_or(0),
                total: fields.iter().sum(),
            }
        })
        .collect()
}

/// The share of the time between two readings spent in softirq.
pub fn share(before: Ticks, after: Ticks) -> f64 {
    let total = after.total.saturating_sub(before.total);
    if total == 0 {
        return 0.0;
    }
    float(after.softirq.saturating_sub(before.softirq)) / float(total)
}

/// The softirq share of the whole machine and of its busiest core.
pub fn softirq(before: &[Ticks], after: &[Ticks]) -> Option<(f64, f64)> {
    let (all_before, cores_before) = before.split_first()?;
    let (all_after, cores_after) = after.split_first()?;
    let busiest = cores_before
        .iter()
        .zip(cores_after)
        .map(|(b, a)| share(*b, *a))
        .fold(0.0, f64::max);
    Some((share(*all_before, *all_after), busiest))
}

/// The CPU time a cgroup has used, in microseconds, from its `cpu.stat`.
pub fn used_usec(cpu_stat: &str) -> Option<u64> {
    cpu_stat
        .lines()
        .find_map(|line| line.strip_prefix("usage_usec "))
        .and_then(|n| n.trim().parse().ok())
}

/// A cgroup's use over an interval, as a share of one core.
pub fn of_a_core(before_usec: u64, after_usec: u64, elapsed_usec: u64) -> f64 {
    if elapsed_usec == 0 {
        return 0.0;
    }
    float(after_usec.saturating_sub(before_usec)) / float(elapsed_usec)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BEFORE: &str = "cpu  100 0 50 800 10 5 20 0 0 0\n\
        cpu0 50 0 25 400 5 2 10 0 0 0\n\
        cpu1 50 0 25 400 5 3 10 0 0 0\n\
        intr 12345\n";
    const AFTER: &str = "cpu  110 0 60 900 10 5 120 0 7 0\n\
        cpu0 55 0 30 450 5 2 20 0 7 0\n\
        cpu1 55 0 30 450 5 3 100 0 0 0\n\
        intr 12399\n";

    #[test]
    fn the_machine_comes_first_then_each_core() {
        let all = ticks(BEFORE);
        assert_eq!(all.len(), 3);
        assert_eq!(
            all[0],
            Ticks {
                softirq: 20,
                total: 985
            }
        );
    }

    #[test]
    fn guest_time_counts_once() {
        // cpu0's 7 guest ticks are in its user time already.
        assert_eq!(ticks(AFTER)[1].total, 562);
    }

    #[test]
    fn softirq_is_said_for_the_machine_and_its_busiest_core() {
        let (all, busiest) = softirq(&ticks(BEFORE), &ticks(AFTER)).expect("both read");
        assert_eq!(format!("{all:.3} {busiest:.3}"), "0.455 0.600");
    }

    #[test]
    fn no_time_passing_is_no_share() {
        let same = ticks(BEFORE);
        let (all, busiest) = softirq(&same, &same).expect("both read");
        assert_eq!(format!("{all} {busiest}"), "0 0");
    }

    #[test]
    fn an_unreadable_stat_says_nothing() {
        assert!(
            softirq(&[], &ticks(AFTER)).is_none(),
            "an empty reading gave a share"
        );
    }

    #[test]
    fn a_cgroup_says_what_it_used() {
        let stat = "usage_usec 4582314\nuser_usec 892102\nsystem_usec 3690211\n";
        assert_eq!(used_usec(stat), Some(4_582_314));
        assert_eq!(used_usec("user_usec 1\n"), None);
    }

    #[test]
    fn use_is_a_share_of_one_core() {
        assert_eq!(
            format!("{:.3}", of_a_core(1_000, 21_000, 1_000_000)),
            "0.020"
        );
        assert_eq!(format!("{}", of_a_core(5, 9, 0)), "0");
    }
}
