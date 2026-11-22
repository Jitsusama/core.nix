//! Job folders a run left in the quest's handover and never took away. A
//! run takes its folder away once its job ends here, but one killed first,
//! as by its caller's timeout, leaves it, thousands of files a test
//! suite's tree. The quest's share then carries every leftover with the
//! next job, and the next job's wait for its own files gives up first: on
//! 21 November a run had a leftover to carry beside its own, twice its
//! files, and stopped near half. So each run clears what's spent before
//! it leaves its own.
//!
//! The handover crosses to every machine the quest is shared with, so a
//! folder here may be another machine's job, its process somewhere else.
//! Each job's folder names the machine and the process that left it, and
//! a folder is spent when this machine left it and its process is gone,
//! or when it has lain untouched a day, whoever left it.

use std::fs;
use std::io;
use std::path::Path;
use std::time::{Duration, SystemTime};

/// The file naming the machine and process that left a job's folder,
/// `MACHINE<TAB>PID`, written before anything else in it.
const OWNER: &str = "elsewhere-owner";

/// How long a folder may lie untouched before it's spent whoever left
/// it: past any job's run, so another machine's job is never cut short.
pub const UNTOUCHED_A_DAY: Duration = Duration::from_hours(24);

/// Makes the job's folder `job` and names in it this `machine` and
/// process as what left it.
pub fn owned(job: &Path, machine: &str) -> io::Result<()> {
    fs::create_dir_all(job)?;
    fs::write(
        job.join(OWNER),
        format!("{machine}\t{}\n", std::process::id()),
    )
}

/// Removes each spent job's folder in `handover`, as `now` and this
/// `machine` find it, whether a process is `alive` telling which of this
/// machine's are gone, and says how many went. A folder that won't go is
/// left for the next run, since none of this is the job's own business.
pub fn clear(
    handover: &Path,
    machine: &str,
    alive: impl Fn(u32) -> bool,
    now: SystemTime,
) -> usize {
    let Ok(entries) = fs::read_dir(handover) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter(|entry| spent(&entry.path(), machine, &alive, now))
        .filter(|entry| fs::remove_dir_all(entry.path()).is_ok())
        .count()
}

/// Whether the job's folder `job` is spent.
fn spent(job: &Path, machine: &str, alive: &impl Fn(u32) -> bool, now: SystemTime) -> bool {
    let untouched = fs::metadata(job)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|modified| now.duration_since(modified).ok())
        .is_some_and(|since| since >= UNTOUCHED_A_DAY);
    if untouched {
        return true;
    }
    let Ok(owner) = fs::read_to_string(job.join(OWNER)) else {
        return false;
    };
    let mut words = owner.trim_end().split('\t');
    let (Some(left_by), Some(pid)) = (words.next(), words.next()) else {
        return false;
    };
    left_by == machine && pid.parse().is_ok_and(|pid| !alive(pid))
}

#[cfg(test)]
mod tests {
    use super::{UNTOUCHED_A_DAY, clear, owned};
    use std::path::{Path, PathBuf};
    use std::time::SystemTime;

    /// A handover of the test's own, emptied.
    fn handover(what: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("elsewhere-spent-{what}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    /// A job's folder in `handover`, left by `machine`'s process `pid`.
    fn left(handover: &Path, id: &str, machine: &str, pid: u32) -> PathBuf {
        let job = handover.join(id);
        std::fs::create_dir_all(job.join("tree")).unwrap();
        std::fs::write(job.join("tree/Spec.tla"), "spec").unwrap();
        std::fs::write(job.join(super::OWNER), format!("{machine}\t{pid}\n")).unwrap();
        job
    }

    #[test]
    fn only_this_machines_jobs_whose_process_is_gone_are_cleared() {
        let handover = handover("gone");
        let gone = left(&handover, "1-1", "kestrel", 1);
        let running = left(&handover, "2-2", "kestrel", 2);
        let osprey = left(&handover, "1-3", "osprey", 1);
        let unowned = handover.join("4-4");
        std::fs::create_dir_all(&unowned).unwrap();
        let cleared = clear(&handover, "kestrel", |pid| pid == 2, SystemTime::now());
        assert_eq!(cleared, 1);
        assert!(!gone.exists(), "its process is gone");
        assert!(running.exists(), "its process still runs");
        assert!(osprey.exists(), "osprey's process may run there");
        assert!(unowned.exists(), "nothing says who left it");
    }

    #[test]
    fn a_folder_untouched_a_day_is_cleared_whoever_left_it() {
        let handover = handover("day");
        let osprey = left(&handover, "1-1", "osprey", 1);
        let unowned = handover.join("2-2");
        std::fs::create_dir_all(&unowned).unwrap();
        let a_day_on = SystemTime::now() + UNTOUCHED_A_DAY;
        assert_eq!(clear(&handover, "kestrel", |_| true, a_day_on), 2);
        assert!(!osprey.exists(), "osprey's, a day old, is cleared");
        assert!(!unowned.exists(), "one nothing owns, a day old, is cleared");
    }

    #[test]
    fn a_job_names_this_machine_and_process_as_what_left_it() {
        let handover = handover("owned");
        let job = handover.join("5-5");
        owned(&job, "kestrel").unwrap();
        let here = std::process::id();
        assert_eq!(
            clear(&handover, "kestrel", |pid| pid == here, SystemTime::now()),
            0
        );
        assert_eq!(clear(&handover, "kestrel", |_| false, SystemTime::now()), 1);
        assert!(!job.exists(), "its process gone, the job is cleared");
    }

    #[test]
    fn a_handover_not_there_yet_clears_nothing() {
        let missing = handover("missing").join("none");
        assert_eq!(clear(&missing, "kestrel", |_| false, SystemTime::now()), 0);
    }
}
