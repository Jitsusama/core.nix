use disk_guard::activity::Use;
use disk_guard::anchor::Dir;
use disk_guard::pass::Outcome;
use disk_guard::plan::{Candidate, Class};
use disk_guard::reclaim::{Looker, remove_in_place, target};
use std::collections::VecDeque;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Answers each judgement and waiter check from a script, in turn.
#[derive(Default)]
pub(crate) struct Script {
    pub(crate) uses: VecDeque<Use>,
    pub(crate) waiters: VecDeque<Vec<i32>>,
    pub(crate) asked_waiters_for: Vec<Vec<u64>>,
    /// What it was asked or told, in order.
    pub(crate) heard: Vec<&'static str>,
}

impl Looker for Script {
    fn judge(&mut self, _target: &Path, _checkout: Option<&Path>) -> io::Result<Use> {
        self.heard.push("judge");
        Ok(self.uses.pop_front().unwrap_or(Use::Free))
    }

    fn moved(&mut self) {
        self.heard.push("moved");
    }

    fn waiters(&mut self, inodes: &[u64]) -> io::Result<Vec<i32>> {
        self.heard.push("waiters");
        self.asked_waiters_for.push(inodes.to_vec());
        Ok(self.waiters.pop_front().unwrap_or_default())
    }
}

/// A built target under a root, with cargo's lock and a benchmark baseline.
fn built(root: &Path) -> PathBuf {
    let target = root.join("app/target");
    for dir in [
        "debug/deps",
        "debug/.fingerprint/app-0123",
        "criterion/base",
    ] {
        fs::create_dir_all(target.join(dir)).unwrap();
    }
    fs::write(target.join("debug/.cargo-lock"), "").unwrap();
    fs::write(
        target.join("debug/deps/app-0123456789abcdef"),
        vec![0u8; 4096],
    )
    .unwrap();
    fs::write(target.join("criterion/base/estimates.json"), "{}").unwrap();
    fs::write(
        target.join("CACHEDIR.TAG"),
        "Signature: 8a477f597d28d172789f06886806bc55\n",
    )
    .unwrap();
    target
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn candidate(path: &Path, occupied: bool) -> Candidate {
    Candidate {
        path: path.to_owned(),
        class: Class::Target,
        idle: 0.0,
        size: 0,
        occupied,
        spent: false,
        checkout: None,
    }
}

fn reclaimed(script: &mut Script, occupied: bool) -> (tempfile::TempDir, PathBuf, Outcome) {
    let root = tempfile::tempdir().unwrap();
    let path = built(root.path());
    let anchor = Dir::open(root.path()).unwrap();
    let outcome = target(
        &anchor,
        root.path(),
        &candidate(&path, occupied),
        None,
        script,
    );
    (root, path, outcome)
}

#[test]
fn an_idle_target_is_emptied_but_for_its_benchmarks() {
    let mut script = Script::default();
    let (_root, path, outcome) = reclaimed(&mut script, false);
    assert_eq!(outcome, Outcome::Reclaimed);
    assert_eq!(names(&path), ["criterion"]);
    // The lock was held while the waiters were checked.
    assert_eq!(script.asked_waiters_for.len(), 1);
    assert_eq!(script.asked_waiters_for[0].len(), 1);
    // Told of the move before judging again, so it looks afresh.
    assert_eq!(script.heard, ["judge", "moved", "waiters", "judge"]);
}

#[test]
fn a_target_judged_busy_is_never_touched() {
    for judged in [Use::Busy(7), Use::Unknown(7)] {
        let mut script = Script {
            uses: [judged].into(),
            ..Default::default()
        };
        let (_root, path, outcome) = reclaimed(&mut script, false);
        assert!(matches!(outcome, Outcome::Kept(_)), "{outcome:?}");
        assert_eq!(names(&path), ["CACHEDIR.TAG", "criterion", "debug"]);
    }
}

#[test]
fn a_target_someone_sat_down_in_goes_only_if_planned_as_occupied() {
    let mut script = Script {
        uses: [Use::Occupied(7)].into(),
        ..Default::default()
    };
    let (_root, path, outcome) = reclaimed(&mut script, false);
    assert!(matches!(outcome, Outcome::Kept(_)), "{outcome:?}");
    assert_eq!(names(&path), ["CACHEDIR.TAG", "criterion", "debug"]);
    let mut script = Script {
        uses: [Use::Occupied(7), Use::Occupied(7)].into(),
        ..Default::default()
    };
    assert_eq!(reclaimed(&mut script, true).2, Outcome::Reclaimed);
}

#[test]
fn a_target_whose_lock_cargo_holds_is_kept() {
    use rustix::fs::{FlockOperation, flock};
    let root = tempfile::tempdir().unwrap();
    let path = built(root.path());
    let lock = fs::File::open(path.join("debug/.cargo-lock")).unwrap();
    flock(&lock, FlockOperation::LockShared).unwrap();
    let anchor = Dir::open(root.path()).unwrap();
    let outcome = target(
        &anchor,
        root.path(),
        &candidate(&path, false),
        None,
        &mut Script::default(),
    );
    assert!(matches!(outcome, Outcome::Kept(_)), "{outcome:?}");
    assert_eq!(names(&path), ["CACHEDIR.TAG", "criterion", "debug"]);
}

#[test]
fn a_build_waiting_on_the_lock_gets_its_target_back_intact() {
    let mut script = Script {
        waiters: [vec![4242]].into(),
        ..Default::default()
    };
    let (_root, path, outcome) = reclaimed(&mut script, false);
    assert!(matches!(outcome, Outcome::Kept(_)), "{outcome:?}");
    assert_eq!(names(&path), ["CACHEDIR.TAG", "criterion", "debug"]);
    assert_eq!(
        fs::read(path.join("debug/deps/app-0123456789abcdef"))
            .unwrap()
            .len(),
        4096
    );
}

#[test]
fn a_build_starting_as_the_target_moved_gets_it_back() {
    let mut script = Script {
        uses: [Use::Free, Use::Busy(9)].into(),
        ..Default::default()
    };
    let (_root, path, outcome) = reclaimed(&mut script, false);
    assert!(matches!(outcome, Outcome::Kept(_)), "{outcome:?}");
    assert_eq!(names(&path), ["CACHEDIR.TAG", "criterion", "debug"]);
}

#[test]
fn a_target_reached_through_a_symlink_fails_untouched() {
    let root = tempfile::tempdir().unwrap();
    let real = built(root.path());
    std::os::unix::fs::symlink(root.path().join("app"), root.path().join("link")).unwrap();
    let anchor = Dir::open(root.path()).unwrap();
    let through = root.path().join("link/target");
    let outcome = target(
        &anchor,
        root.path(),
        &candidate(&through, false),
        None,
        &mut Script::default(),
    );
    assert!(matches!(outcome, Outcome::Failed(_)), "{outcome:?}");
    assert_eq!(names(&real), ["CACHEDIR.TAG", "criterion", "debug"]);
}

#[test]
fn deleting_in_place_that_fails_part_way_has_taken_the_fingerprints() {
    // On a tree too deep to remove, what's left is rebuilt rather than
    // trusted, since every profile's fingerprints are gone, a triple's too.
    let root = tempfile::tempdir().unwrap();
    let path = built(root.path());
    let mut deep = path.join("debug/deps/deep");
    for _ in 0..300 {
        deep.push("d");
    }
    fs::create_dir_all(&deep).unwrap();
    fs::create_dir_all(path.join("x86_64-unknown-linux-gnu/release/.fingerprint/app")).unwrap();
    let anchor = Dir::open(&path).unwrap();
    assert!(remove_in_place(&anchor, &["criterion"]).is_err());
    assert!(!path.join("debug/.fingerprint").exists());
    assert!(!path.join("x86_64-unknown-linux-gnu").exists());
    assert!(path.join("criterion/base/estimates.json").exists());
}

#[test]
fn deleting_in_place_keeps_what_it_is_told_to() {
    let root = tempfile::tempdir().unwrap();
    let path = built(root.path());
    remove_in_place(&Dir::open(&path).unwrap(), &["criterion"]).unwrap();
    assert_eq!(names(&path), ["criterion"]);
}
