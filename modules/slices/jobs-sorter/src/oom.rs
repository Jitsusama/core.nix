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

use std::fs;
use std::path::Path;

/// The adjustment work is given.
pub const FIRST: i32 = 1000;

/// The adjustment work waiting on work elsewhere is given.
pub const AFTER: i32 = 900;

/// The variable, with its value, that marks work waiting on work elsewhere.
pub const ELSEWHERE: &[u8] = b"WORK_RUNS_ELSEWHERE=1";

/// Puts each of `pids` at [`FIRST`] in `proc`, the kernel's process files,
/// or at [`AFTER`] when its environment carries [`ELSEWHERE`], leaving any
/// already where it goes. A process that left meanwhile is no loss;
/// anything else is said once.
pub fn raise(proc: &Path, pids: &[u32], warned: &mut bool) {
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
        let wanted = if marked { AFTER } else { FIRST };
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
        raise(&proc, &[20, 21, 22, 23], &mut warned);
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
        assert!(!warned);
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
        raise(&proc, &[10, 11, 12, 13], &mut warned);
        let adj = |pid: u32| {
            fs::read_to_string(proc.join(pid.to_string()).join("oom_score_adj")).unwrap()
        };
        assert_eq!(adj(10), "1000\n");
        assert_eq!(adj(11), "1000\n");
        assert_eq!(adj(12), "1000\n");
        assert!(!warned, "a process that left is no loss");
        fs::remove_dir_all(&proc).unwrap();
    }
}
