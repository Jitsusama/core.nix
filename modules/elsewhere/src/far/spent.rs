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
//! is used again once its process ends. The owner file is hidden, so no
//! share carries it: a share writes what it receives by renaming over the
//! path, which would leave the lock on a file no longer there.
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
/// `MACHINE<TAB>PID`, locked while that process lives. Hidden, so that no
/// crossing carries it or writes over it.
const OWNER: &str = ".elsewhere-owner";

/// What a spent folder is renamed with before it's removed, so one whose
/// removal was cut short is still known for spent, its owner file gone or
/// not. Hidden, so that no crossing carries it.
const SPENT: &str = ".spent-";

/// How long another machine's folder may lie untouched before it's spent:
/// past any job's run, so another machine's job is never cut short.
pub const UNTOUCHED_A_DAY: Duration = Duration::from_hours(24);

/// What a clearing did: how many spent folders went, and each that
/// wouldn't, by name, with why.
#[derive(Debug, Default)]
pub struct Cleared {
    pub gone: usize,
    pub kept: Vec<(String, io::Error)>,
}

/// Makes the job's folder `job`, names in it this `machine` and process as
/// what left it, and holds it: until the file answered is closed, as it is
/// when the process ends, no run takes the folder for spent.
pub fn owned(job: &Path, machine: &str) -> io::Result<File> {
    fs::create_dir_all(job)?;
    let mut owner = File::create(job.join(OWNER))?;
    // Locked before it names anyone, so a run that finds this machine
    // named finds it held; one that finds it empty leaves it a day.
    owner.lock()?;
    writeln!(owner, "{machine}\t{}", std::process::id())?;
    Ok(owner)
}

/// Removes each spent job's folder in `handover`, as this `machine` finds
/// it `now`. Every folder from `scratches` down to the handover must be a
/// folder, not a link, so that nothing outside the quest is ever removed.
/// A folder that won't go is left for the next run, and said.
pub fn clear(scratches: &Path, handover: &Path, machine: &str, now: SystemTime) -> Cleared {
    let mut cleared = Cleared::default();
    if !plain(scratches, handover) {
        return cleared;
    }
    let Ok(entries) = fs::read_dir(handover) else {
        return cleared;
    };
    for entry in entries.flatten() {
        if !entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        let (job, name) = (
            entry.path(),
            entry.file_name().to_string_lossy().into_owned(),
        );
        let removed = if name.starts_with(SPENT) {
            fs::remove_dir_all(&job)
        } else if spent(&job, machine, now) {
            let taken = handover.join(format!("{SPENT}{name}"));
            fs::rename(&job, &taken).and_then(|()| fs::remove_dir_all(&taken))
        } else {
            continue;
        };
        match removed {
            Ok(()) => cleared.gone += 1,
            Err(error) => cleared.kept.push((name, error)),
        }
    }
    cleared
}

/// Whether `handover` lies under `scratches` and it and every folder
/// between them is a folder, not a link.
fn plain(scratches: &Path, handover: &Path) -> bool {
    handover.strip_prefix(scratches).is_ok_and(|below| {
        let mut at = scratches.to_path_buf();
        below.components().all(|part| {
            at.push(part);
            fs::symlink_metadata(&at).is_ok_and(|meta| meta.is_dir())
        })
    })
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
    use super::{Cleared, SPENT, UNTOUCHED_A_DAY, clear, owned};
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime};

    /// A quest's scratches of the test's own, its handover emptied, and
    /// removed once the test is done.
    struct Handover {
        scratches: PathBuf,
        path: PathBuf,
    }

    impl Handover {
        fn new(what: &str) -> Self {
            let scratches =
                std::env::temp_dir().join(format!("elsewhere-spent-{what}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&scratches);
            let path = scratches.join("QEST-20261115-ABC123/handover/elsewhere");
            std::fs::create_dir_all(&path).unwrap();
            Self { scratches, path }
        }

        /// A job's folder, left by `machine`'s run that ended.
        fn left(&self, id: &str, machine: &str) -> PathBuf {
            let job = self.path.join(id);
            std::fs::create_dir_all(job.join("tree")).unwrap();
            std::fs::write(job.join("tree/Spec.tla"), "spec").unwrap();
            std::fs::write(job.join(super::OWNER), format!("{machine}\t1\n")).unwrap();
            job
        }

        /// A job's folder that says nothing of who left it.
        fn unowned(&self, id: &str) -> PathBuf {
            let job = self.path.join(id);
            std::fs::create_dir_all(&job).unwrap();
            job
        }

        fn clear(&self, now: SystemTime) -> Cleared {
            clear(&self.scratches, &self.path, "kestrel", now)
        }
    }

    impl Drop for Handover {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.scratches);
        }
    }

    /// When `job` was last touched.
    fn touched(job: &Path) -> SystemTime {
        std::fs::metadata(job).unwrap().modified().unwrap()
    }

    #[test]
    fn only_this_machines_folders_no_run_holds_are_cleared() {
        let handover = Handover::new("gone");
        let gone = handover.left("1-1", "kestrel");
        let running = handover.path.join("2-2");
        let _held = owned(&running, "kestrel").unwrap();
        let osprey = handover.left("1-3", "osprey");
        let unowned = handover.unowned("4-4");
        assert_eq!(
            handover.clear(SystemTime::now()).gone,
            1,
            "only the one gone"
        );
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
        let running = handover.path.join("3-3");
        let _held = owned(&running, "kestrel").unwrap();
        let a_day_on = touched(&osprey).max(touched(&unowned)) + UNTOUCHED_A_DAY;
        assert_eq!(
            handover.clear(a_day_on).gone,
            2,
            "the two no run here holds"
        );
        assert!(
            !osprey.exists(),
            "osprey's, a day old to the instant, is cleared"
        );
        assert!(!unowned.exists(), "one nothing owns, a day old, is cleared");
        assert!(running.exists(), "a run here holding it keeps it, a day on");
    }

    #[test]
    fn a_folder_untouched_less_than_a_day_is_kept() {
        let handover = Handover::new("hours");
        let osprey = handover.left("1-1", "osprey");
        let unowned = handover.unowned("2-2");
        let nearly =
            touched(&osprey).min(touched(&unowned)) + UNTOUCHED_A_DAY - Duration::from_millis(1);
        assert_eq!(handover.clear(nearly).gone, 0, "neither is a day old");
        assert!(osprey.exists() && unowned.exists(), "both are kept");
    }

    #[test]
    fn a_jobs_folder_is_held_until_its_owner_file_closes() {
        let handover = Handover::new("owned");
        let job = handover.path.join("5-5");
        let held = owned(&job, "kestrel").unwrap();
        let said = std::fs::read_to_string(job.join(super::OWNER)).unwrap();
        assert_eq!(said, format!("kestrel\t{}\n", std::process::id()));
        assert_eq!(
            handover.clear(SystemTime::now()).gone,
            0,
            "its run holds it"
        );
        drop(held);
        assert_eq!(handover.clear(SystemTime::now()).gone, 1, "no run holds it");
        assert!(!job.exists(), "its run gone, the job is cleared");
    }

    #[test]
    fn a_folder_whose_clearing_was_cut_short_goes_next_time() {
        let handover = Handover::new("cut");
        let cut = handover.unowned(&format!("{SPENT}6-6"));
        assert_eq!(handover.clear(SystemTime::now()).gone, 1, "it was spent");
        assert!(!cut.exists(), "what was left of it is gone");
    }

    #[test]
    fn a_handover_that_is_a_link_clears_nothing_it_leads_to() {
        let handover = Handover::new("link");
        let elsewhere = handover.scratches.join("elsewhere-else");
        std::fs::create_dir_all(elsewhere.join(format!("{SPENT}7-7"))).unwrap();
        std::fs::remove_dir(&handover.path).unwrap();
        std::os::unix::fs::symlink(&elsewhere, &handover.path).unwrap();
        assert_eq!(handover.clear(SystemTime::now()).gone, 0, "it's a link");
        assert!(
            elsewhere.join(format!("{SPENT}7-7")).exists(),
            "what the link leads to was cleared"
        );
    }

    #[test]
    fn a_folder_that_wont_go_is_said_by_name() {
        use std::os::unix::fs::PermissionsExt;
        let handover = Handover::new("stuck");
        let stuck = handover.unowned(&format!("{SPENT}8-8"));
        std::fs::create_dir_all(stuck.join("tree")).unwrap();
        std::fs::write(stuck.join("tree/Spec.tla"), "spec").unwrap();
        let locked = std::fs::Permissions::from_mode(0o500);
        std::fs::set_permissions(stuck.join("tree"), locked).unwrap();
        let cleared = handover.clear(SystemTime::now());
        let open = std::fs::Permissions::from_mode(0o700);
        std::fs::set_permissions(stuck.join("tree"), open).unwrap();
        // Whoever may remove what they can't write, as root may, sees it go.
        if stuck.exists() {
            assert_eq!(cleared.gone, 0, "it didn't go");
            let names: Vec<&str> = cleared.kept.iter().map(|(name, _)| name.as_str()).collect();
            assert_eq!(names, [format!("{SPENT}8-8")], "it wasn't said by name");
        }
    }

    #[test]
    fn a_handover_not_there_yet_clears_nothing() {
        let missing = Path::new("/nonexistent/handover");
        let cleared = clear(
            Path::new("/nonexistent"),
            missing,
            "kestrel",
            SystemTime::now(),
        );
        assert_eq!((cleared.gone, cleared.kept.len()), (0, 0));
    }
}
