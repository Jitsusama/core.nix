//! Puts work first in line when memory runs out. The kernel's OOM killer
//! ends the process with the highest score, its size plus its
//! `oom_score_adj`, and a process inherits the adjustment its parent had,
//! not its cgroup's. Everything in the account starts at 200, and a
//! Chromium app sets its own processes to 300, so the kernel ended Slack
//! rather than the git that ran memory out on 30 October. Each process
//! under a `--no-real-time` cgroup, an agent's work, is raised to 1000, the
//! most there is, so among work alike the largest goes. Raising needs no
//! privilege; lowering would.

use std::fs;
use std::path::Path;

/// The adjustment work is given.
pub const FIRST: i32 = 1000;

/// Raises each of `pids` to [`FIRST`] in `proc`, the kernel's process
/// files, leaving any already there. A process that left meanwhile is no
/// loss; anything else is said once.
pub fn raise(proc: &Path, pids: &[u32], warned: &mut bool) {
    for pid in pids {
        let file = proc.join(pid.to_string()).join("oom_score_adj");
        let Some(now) = fs::read_to_string(&file)
            .ok()
            .and_then(|text| text.trim().parse::<i32>().ok())
        else {
            continue;
        };
        if now >= FIRST {
            continue;
        }
        if let Err(error) = fs::write(&file, format!("{FIRST}\n"))
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
