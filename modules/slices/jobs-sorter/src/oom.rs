//! Puts work first in line when memory runs out. The kernel's OOM killer
//! ends the process with the highest score, its size plus its
//! `oom_score_adj`, and a process inherits the adjustment its parent had,
//! not its cgroup's. Everything in the account starts at 200, and a
//! Chromium app sets its own processes to 300, so the kernel ended Slack
//! rather than the git that ran memory out on 30 October. Each process
//! under a `--no-real-time` cgroup, an agent's work, is raised to 1000, the
//! most there is, so among work alike the largest goes.
//!
//! Work that only waits on work done elsewhere says so in its environment
//! and goes just after the rest, at 900. The killer counts what a process
//! has in swap as well as in memory, so on 30 October it ended a client of
//! a build 25 minutes into its compile on a lent machine: 18 MB resident,
//! 2 GB of its finished evaluation in swap, and killing it freed no memory
//! at all. The 100 between them weighs a tenth of memory and swap together,
//! about 19 GB here, so any other work goes first. A process may lower its
//! own adjustment as far as the one its account was started with, so
//! neither needs privilege, and a marked process that inherited 1000 from
//! the shell that started it is lowered.
//!
//! With tiers (src/tiers.rs), work goes [`STEP`] lower for each tier its
//! own sits above the unranked one, so the kernel ends the lowest tier's
//! work first among work alike: with five, walgit's at 950 and
//! unranked's at 1000. Even the first tier's work waiting elsewhere, 850
//! with five, stays above every agent's session (`src/keep.rs`), so work
//! always goes before a pi, as P9R1KI set on 12 November.

use std::fs;
use std::path::Path;

use crate::tiers::{self, Order};

/// The adjustment work is given.
pub const FIRST: i32 = 1000;

/// The adjustment work waiting on work elsewhere is given.
pub const AFTER: i32 = 900;

/// How much lower work goes for each tier it sits above the unranked one:
/// a hundredth of memory and swap together, about 1.9 GB here, so a much
/// larger process in a higher tier still goes first.
pub const STEP: i32 = 10;

/// The variable, with its value, that marks work waiting on work elsewhere.
pub const ELSEWHERE: &[u8] = b"WORK_RUNS_ELSEWHERE=1";

/// The adjustment work wants, `above_last` tiers above the unranked one,
/// and `marked` as waiting on work elsewhere.
pub fn wanted(marked: bool, above_last: usize) -> i32 {
    let tier = STEP * i32::try_from(above_last).unwrap_or(i32::MAX / STEP);
    let most = if marked { AFTER } else { FIRST };
    most.saturating_sub(tier)
}

/// Puts each of `pids` at [`FIRST`] in `proc`, the kernel's process files,
/// or at [`AFTER`] when its environment carries [`ELSEWHERE`], each lower
/// by its tier as `order` ranks the one its cgroup sits in under `bases`,
/// leaving any already where it goes. A process that left meanwhile is no
/// loss; anything else is said once.
pub fn raise(proc: &Path, pids: &[u32], order: &Order, bases: &[String], warned: &mut bool) {
    for pid in pids {
        let dir = proc.join(pid.to_string());
        let file = dir.join("oom_score_adj");
        let Some(now) = fs::read_to_string(&file)
            .ok()
            .and_then(|text| text.trim().parse::<i32>().ok())
        else {
            continue;
        };
        let marked = fs::read(dir.join("environ")).is_ok_and(|environ| {
            environ
                .split(|&byte| byte == 0)
                .any(|pair| pair == ELSEWHERE)
        });
        let cgroup = fs::read_to_string(dir.join("cgroup")).unwrap_or_default();
        let above_last = bases
            .iter()
            .find_map(|base| tiers::tier_in(&cgroup, base))
            .map_or(0, |tier| order.above_last(tier));
        let wanted = wanted(marked, above_last);
        if now == wanted {
            continue;
        }
        if let Err(error) = fs::write(&file, format!("{wanted}\n"))
            && error.raw_os_error() != Some(3)
            && !*warned
        {
            eprintln!("Couldn't put {pid} first for the OOM killer: {error}");
            *warned = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_waiting_on_work_elsewhere_goes_just_after_the_rest() {
        let proc = std::env::temp_dir().join(format!("oom-elsewhere-{}", std::process::id()));
        for (pid, adj, environ) in [
            (20, "1000", "PATH=/bin\0WORK_RUNS_ELSEWHERE=1\0"),
            (21, "200", "WORK_RUNS_ELSEWHERE=1\0"),
            (22, "900", "PATH=/bin\0"),
            (23, "200", "WORK_RUNS_ELSEWHERE=10\0"),
        ] {
            let dir = proc.join(pid.to_string());
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("oom_score_adj"), format!("{adj}\n")).unwrap();
            fs::write(dir.join("environ"), environ).unwrap();
        }
        let mut warned = false;
        raise(
            &proc,
            &[20, 21, 22, 23],
            &Order::default(),
            &[],
            &mut warned,
        );
        let adj = |pid: u32| {
            fs::read_to_string(proc.join(pid.to_string()).join("oom_score_adj")).unwrap()
        };
        assert_eq!(
            adj(20),
            "900\n",
            "a marked process inherits 1000 and is lowered"
        );
        assert_eq!(adj(21), "900\n");
        assert_eq!(
            adj(22),
            "1000\n",
            "only the mark keeps a process off the most"
        );
        assert_eq!(adj(23), "1000\n", "the mark is the whole variable");
        assert!(!warned, "every score should have been written");
        fs::remove_dir_all(&proc).unwrap();
    }

    #[test]
    fn work_below_the_most_is_raised_to_it_and_the_rest_is_left() {
        let proc = std::env::temp_dir().join(format!("oom-{}", std::process::id()));
        for (pid, adj) in [(10, "200"), (11, "1000"), (12, "-100")] {
            let dir = proc.join(pid.to_string());
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("oom_score_adj"), format!("{adj}\n")).unwrap();
        }
        let mut warned = false;
        raise(
            &proc,
            &[10, 11, 12, 13],
            &Order::default(),
            &[],
            &mut warned,
        );
        let adj = |pid: u32| {
            fs::read_to_string(proc.join(pid.to_string()).join("oom_score_adj")).unwrap()
        };
        assert_eq!(adj(10), "1000\n");
        assert_eq!(adj(11), "1000\n");
        assert_eq!(adj(12), "1000\n");
        assert!(!warned, "a process that left is no loss");
        fs::remove_dir_all(&proc).unwrap();
    }

    #[test]
    fn work_in_a_higher_tier_goes_after_a_lower_tiers() {
        let proc = std::env::temp_dir().join(format!("oom-tiers-{}", std::process::id()));
        let work = "0::/user.slice/user-1000.slice/user@1000.service/jobs.slice/jobs-work.slice";
        for (pid, tier, environ) in [
            (40, "builds", "PATH=/bin\0"),
            (41, "fleet", "PATH=/bin\0"),
            (42, "unranked", "PATH=/bin\0"),
            (43, "builds", "WORK_RUNS_ELSEWHERE=1\0"),
        ] {
            let dir = proc.join(pid.to_string());
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("oom_score_adj"), "200\n").unwrap();
            fs::write(dir.join("environ"), environ).unwrap();
            let slice = format!("{work}/jobs-work-{tier}.slice/work-{pid}-1.scope\n");
            fs::write(dir.join("cgroup"), slice).unwrap();
        }
        let order = Order::new("builds,reviews,fleet").unwrap();
        let bases = ["jobs-work.slice".to_owned()];
        let mut warned = false;
        raise(&proc, &[40, 41, 42, 43], &order, &bases, &mut warned);
        let adj = |pid: u32| {
            fs::read_to_string(proc.join(pid.to_string()).join("oom_score_adj")).unwrap()
        };
        assert_eq!(
            adj(40),
            "970\n",
            "the first of three tiers is three steps down"
        );
        assert_eq!(adj(41), "990\n");
        assert_eq!(adj(42), "1000\n", "unranked work goes first");
        assert_eq!(adj(43), "870\n", "waiting elsewhere still goes after");
        assert!(!warned, "every score should have been written");
        fs::remove_dir_all(&proc).unwrap();
    }

    #[test]
    fn the_first_of_eight_tiers_waiting_elsewhere_stays_above_every_pi() {
        let lowest = wanted(true, crate::tiers::MOST);
        let highest_pi = crate::keep::adjustment(false, crate::tiers::MOST);
        assert!(
            lowest > highest_pi,
            "work at {lowest} under a pi at {highest_pi}"
        );
    }
}
