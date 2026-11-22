//! Job folders a run left in the quest's handover and never took away. A
//! run takes its folder away once its job ends here, but one killed first,
//! as by its caller's timeout, leaves it, thousands of files a test
//! suite's tree. The quest's share then carries every leftover with the
//! next job, and the next job's wait for its own files gives up first: on
//! 21 November a run had a leftover to carry beside its own, twice its
//! files, and stopped near half. So each run clears what's spent before
//! it leaves its own.
//!
//! A run holds a lock on its folder's owner file for as long as it lives,
//! and the system lets the lock go when the process ends, however it
//! ends. A process id would not do: under an agent's fence a run's id is
//! its fence's own, so another fence's live run can look gone, and an id
//! is used again once its process ends.
//!
//! The handover crosses to every machine the quest is shared with, so a
//! folder here may be another machine's job, its lock held somewhere
//! else. So a folder this machine left is spent once no run holds it, and
//! any other, another machine's or one that says nothing of who left it,
//! once it has lain untouched a day.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::time::{Duration, SystemTime};

/// The file naming the machine and process that left a job's folder,
/// `MACHINE<TAB>PID`, locked while that process lives.
const OWNER: &str = "elsewhere-owner";

/// The owner file while it's being written and locked, hidden so that no
/// crossing carries it.
const OWNING: &str = ".elsewhere-owner";

/// What a spent folder is renamed with before it's removed, so one whose
/// removal was cut short is still known for spent, its owner file gone or
/// not. Hidden, so that no crossing carries it.
const SPENT: &str = ".spent-";

/// How long another machine's folder may lie untouched before it's spent:
/// past any job's run, so another machine's job is never cut short.
pub const UNTOUCHED_A_DAY: Duration = Duration::from_hours(24);

/// Makes the job's folder `job`, names in it this `machine` and process as
/// what left it, and holds it: until the file answered is closed, as it is
/// when the process ends, no run takes the folder for spent.
pub fn owned(job: &Path, machine: &str) -> io::Result<File> {
    fs::create_dir_all(job)?;
    // Locked before it's named, so no run finds it named and not held.
    let owning = job.join(OWNING);
    let mut owner = File::create(&owning)?;
    owner.lock()?;
    writeln!(owner, "{machine}\t{}", std::process::id())?;
    fs::rename(&owning, job.join(OWNER))?;
    Ok(owner)
}

/// Removes each spent job's folder in `handover`, as this `machine` finds
/// it `now`, and says how many went. A folder that won't go is left for
/// the next run, since none of this is the job's own business.
pub fn clear(handover: &Path, machine: &str, now: SystemTime) -> usize {
    let Ok(entries) = fs::read_dir(handover) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter(|entry| {
            let (job, name) = (entry.path(), entry.file_name());
            let name = name.to_string_lossy();
            if name.starts_with(SPENT) {
                return fs::remove_dir_all(&job).is_ok();
            }
            if !spent(&job, machine, now) {
                return false;
            }
            let taken = handover.join(format!("{SPENT}{name}"));
            fs::rename(&job, &taken).is_ok() && fs::remove_dir_all(&taken).is_ok()
        })
        .count()
}

/// Whether the job's folder `job` is spent: this `machine`'s once no run
/// holds it, any other once it has lain untouched a day by `now`.
fn spent(job: &Path, machine: &str, now: SystemTime) -> bool {
    let owner = job.join(OWNER);
    match fs::read_to_string(&owner) {
        Ok(said) if said.split('\t').next() == Some(machine) => !held(&owner),
        _ => fs::metadata(job)
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|modified| now.duration_since(modified).ok())
            .is_some_and(|since| since >= UNTOUCHED_A_DAY),
    }
}

/// Whether a run holds the `owner` file: held unless its lock is had, so
/// a file that won't open or lock is never taken for spent.
fn held(owner: &Path) -> bool {
    File::open(owner).map_or(true, |file| file.try_lock().is_err())
}

#[cfg(test)]
mod tests {
    use super::{SPENT, UNTOUCHED_A_DAY, clear, owned};
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime};

    /// A handover of the test's own, emptied, and removed once the test
    /// is done.
    struct Handover(PathBuf);

    impl Handover {
        fn new(what: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("elsewhere-spent-{what}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).unwrap();
            Self(root)
        }

        /// A job's folder, left by `machine`'s run that ended.
        fn left(&self, id: &str, machine: &str) -> PathBuf {
            let job = self.0.join(id);
            std::fs::create_dir_all(job.join("tree")).unwrap();
            std::fs::write(job.join("tree/Spec.tla"), "spec").unwrap();
            std::fs::write(job.join(super::OWNER), format!("{machine}\t1\n")).unwrap();
            job
        }

        /// A job's folder that says nothing of who left it.
        fn unowned(&self, id: &str) -> PathBuf {
            let job = self.0.join(id);
            std::fs::create_dir_all(&job).unwrap();
            job
        }

        fn clear(&self, now: SystemTime) -> usize {
            clear(&self.0, "kestrel", now)
        }
    }

    impl Drop for Handover {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn only_this_machines_folders_no_run_holds_are_cleared() {
        let handover = Handover::new("gone");
        let gone = handover.left("1-1", "kestrel");
        let running = handover.0.join("2-2");
        let _held = owned(&running, "kestrel").unwrap();
        let osprey = handover.left("1-3", "osprey");
        let unowned = handover.unowned("4-4");
        assert_eq!(handover.clear(SystemTime::now()), 1, "only the one gone");
        assert!(!gone.exists(), "no run holds it");
        assert!(running.exists(), "its run still holds it");
        assert!(osprey.exists(), "osprey's run may hold it there");
        assert!(unowned.exists(), "nothing says who left it");
    }

    #[test]
    fn a_folder_untouched_a_day_is_cleared_unless_a_run_here_holds_it() {
        let handover = Handover::new("day");
        let osprey = handover.left("1-1", "osprey");
        let unowned = handover.unowned("2-2");
        let running = handover.0.join("3-3");
        let _held = owned(&running, "kestrel").unwrap();
        let a_day_on = SystemTime::now() + UNTOUCHED_A_DAY;
        assert_eq!(handover.clear(a_day_on), 2, "the two no run here holds");
        assert!(!osprey.exists(), "osprey's, a day old, is cleared");
        assert!(!unowned.exists(), "one nothing owns, a day old, is cleared");
        assert!(running.exists(), "a run here holding it keeps it, a day on");
    }

    #[test]
    fn a_folder_untouched_less_than_a_day_is_kept() {
        let handover = Handover::new("hours");
        let osprey = handover.left("1-1", "osprey");
        let unowned = handover.unowned("2-2");
        let nearly = SystemTime::now() + UNTOUCHED_A_DAY - Duration::from_mins(1);
        assert_eq!(handover.clear(nearly), 0, "neither is a day old");
        assert!(osprey.exists() && unowned.exists(), "both are kept");
    }

    #[test]
    fn a_jobs_folder_is_held_until_its_owner_file_closes() {
        let handover = Handover::new("owned");
        let job = handover.0.join("5-5");
        let held = owned(&job, "kestrel").unwrap();
        let said = std::fs::read_to_string(job.join(super::OWNER)).unwrap();
        assert_eq!(said, format!("kestrel\t{}\n", std::process::id()));
        assert_eq!(handover.clear(SystemTime::now()), 0, "its run holds it");
        drop(held);
        assert_eq!(handover.clear(SystemTime::now()), 1, "no run holds it");
        assert!(!job.exists(), "its run gone, the job is cleared");
    }

    #[test]
    fn a_folder_whose_clearing_was_cut_short_goes_next_time() {
        let handover = Handover::new("cut");
        let cut = handover.unowned(&format!("{SPENT}6-6"));
        assert_eq!(handover.clear(SystemTime::now()), 1, "it was spent");
        assert!(!cut.exists(), "what was left of it is gone");
    }

    #[test]
    fn a_handover_not_there_yet_clears_nothing() {
        let missing = Path::new("/nonexistent/handover");
        assert_eq!(clear(missing, "kestrel", SystemTime::now()), 0);
    }
}
