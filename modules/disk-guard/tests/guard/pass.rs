use disk_guard::band::{Band, Space, Thresholds};
use disk_guard::pass::{Disk, End, Goal, Learned, Outcome, Reclaimer, pass};
use disk_guard::plan::{Candidate, Class};
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

const GIB: u64 = 1 << 30;
const HOUR: f64 = 3600.0;

/// A 100 GiB disk whose frees show only once settled, as Btrfs's do, and
/// which another writer may fill as the pass goes.
struct FakeDisk {
    free: u64,
    pending: u64,
    /// Bytes another writer takes at each settle, one entry per settle.
    fills: Vec<u64>,
    settles: usize,
    clock: f64,
}

impl FakeDisk {
    fn new(free: u64) -> FakeDisk {
        FakeDisk {
            free,
            pending: 0,
            fills: Vec::new(),
            settles: 0,
            clock: 0.0,
        }
    }
}

impl Disk for FakeDisk {
    fn space(&mut self) -> io::Result<Space> {
        Ok(Space {
            total: 100 * GIB,
            free: self.free,
            btrfs: None,
        })
    }

    fn settle(&mut self) -> io::Result<()> {
        let fill = self.fills.get(self.settles).copied().unwrap_or(0);
        self.settles += 1;
        self.free = (self.free + std::mem::take(&mut self.pending)).saturating_sub(fill);
        Ok(())
    }

    fn now(&self) -> f64 {
        self.clock
    }
}

/// Reclaims by the script it's given: what each path frees, or why it's
/// kept. Its frees land on the disk only once the disk settles.
struct FakeReclaimer<'a> {
    disk: &'a std::cell::RefCell<FakeDisk>,
    frees: BTreeMap<PathBuf, u64>,
    kept: Vec<PathBuf>,
    taken: Vec<PathBuf>,
}

impl Reclaimer for FakeReclaimer<'_> {
    fn reclaim(&mut self, candidate: &Candidate) -> Outcome {
        if self.kept.contains(&candidate.path) {
            return Outcome::Kept("a build started in it".into());
        }
        self.taken.push(candidate.path.clone());
        self.disk.borrow_mut().pending += self
            .frees
            .get(&candidate.path)
            .copied()
            .unwrap_or(candidate.size);
        Outcome::Reclaimed
    }
}

/// Lets the pass hold the disk while the reclaimer adds to it.
struct Shared<'a>(&'a std::cell::RefCell<FakeDisk>);

impl Disk for Shared<'_> {
    fn space(&mut self) -> io::Result<Space> {
        self.0.borrow_mut().space()
    }
    fn settle(&mut self) -> io::Result<()> {
        self.0.borrow_mut().settle()
    }
    fn now(&self) -> f64 {
        self.0.borrow().now()
    }
}

fn thresholds() -> Thresholds {
    Thresholds {
        pressure: 15 * GIB,
        urgent: 8 * GIB,
        critical: 3 * GIB,
        target: 20 * GIB,
        margin: GIB,
    }
}

fn targets(count: usize, idle: f64) -> Vec<Candidate> {
    (0..count)
        .map(|n| Candidate {
            path: PathBuf::from(format!("/home/u/t{n:02}")),
            class: Class::Target,
            // The longest idle goes first, so t00 does.
            idle: idle + (count - n) as f64,
            size: GIB,
            occupied: false,
            spent: false,
            checkout: None,
        })
        .collect()
}

struct Run {
    end: End,
    taken: Vec<PathBuf>,
    free: u64,
    settles: usize,
    learned: Learned,
}

fn run(
    disk: FakeDisk,
    candidates: &[Candidate],
    goal: Goal,
    frees: BTreeMap<PathBuf, u64>,
    kept: Vec<PathBuf>,
) -> Run {
    let disk = std::cell::RefCell::new(disk);
    let mut reclaimer = FakeReclaimer {
        disk: &disk,
        frees,
        kept,
        taken: Vec::new(),
    };
    let mut learned = Learned::default();
    let report = pass(
        &mut Shared(&disk),
        &mut reclaimer,
        candidates,
        &thresholds(),
        Band::Healthy,
        goal,
        &mut learned,
    );
    let taken = reclaimer.taken;
    let disk = disk.into_inner();
    Run {
        end: report.end,
        taken,
        free: disk.free,
        settles: disk.settles,
        learned,
    }
}

#[test]
fn a_pass_reclaims_down_to_the_target_and_stops() {
    let run = run(
        FakeDisk::new(14 * GIB),
        &targets(10, 7.0 * HOUR),
        Goal::Reach,
        BTreeMap::new(),
        Vec::new(),
    );
    assert_eq!(run.end, End::Reached);
    assert_eq!(run.taken.len(), 6);
    assert_eq!(run.free, 20 * GIB);
    assert!(run.settles >= 1);
}

#[test]
fn what_each_kind_frees_is_learned_and_planned_by() {
    let candidates = targets(20, 7.0 * HOUR);
    let halves = candidates
        .iter()
        .map(|c| (c.path.clone(), GIB / 2))
        .collect();
    let run = run(
        FakeDisk::new(14 * GIB),
        &candidates,
        Goal::Reach,
        halves,
        Vec::new(),
    );
    assert_eq!(run.end, End::Reached);
    assert_eq!(run.taken.len(), 12);
    assert!(
        (run.learned.ratio(Class::Target) - 0.5).abs() < 0.05,
        "{}",
        run.learned.ratio(Class::Target)
    );
}

#[test]
fn deletes_that_free_nothing_stop_the_pass_after_three_batches() {
    let candidates = targets(40, 7.0 * HOUR);
    let nothing = candidates.iter().map(|c| (c.path.clone(), 0)).collect();
    let run = run(
        FakeDisk::new(14 * GIB),
        &candidates,
        Goal::Reach,
        nothing,
        Vec::new(),
    );
    assert_eq!(run.end, End::Stalled);
    assert_eq!(
        run.taken.len(),
        18,
        "three batches of the six the shortfall needs"
    );
    // A stall teaches nothing, or the next pass would plan everything.
    assert_eq!(run.learned.ratio(Class::Target), 1.0);
}

#[test]
fn a_candidate_kept_at_the_last_moment_is_passed_over() {
    let candidates = targets(10, 7.0 * HOUR);
    let kept = vec![candidates[0].path.clone()];
    let run = run(
        FakeDisk::new(14 * GIB),
        &candidates,
        Goal::Reach,
        BTreeMap::new(),
        kept,
    );
    assert_eq!(run.end, End::Reached);
    assert_eq!(run.taken.len(), 6);
    assert!(!run.taken.contains(&candidates[0].path));
}

#[test]
fn a_fill_that_outruns_the_plan_is_planned_for_again() {
    let mut disk = FakeDisk::new(14 * GIB);
    disk.fills = vec![2 * GIB];
    let run = run(
        disk,
        &targets(10, 7.0 * HOUR),
        Goal::Reach,
        BTreeMap::new(),
        Vec::new(),
    );
    assert_eq!(run.end, End::Reached);
    // The fill reads as targets freeing less than their size, so the next
    // plan takes a little more than the 2 GiB it needs, never many more.
    assert!(
        (8..=10).contains(&run.taken.len()),
        "took {}",
        run.taken.len()
    );
    assert!(run.free >= 20 * GIB);
}

#[test]
fn a_pass_with_nothing_left_to_take_says_so() {
    let run = run(
        FakeDisk::new(14 * GIB),
        &targets(3, 7.0 * HOUR),
        Goal::Reach,
        BTreeMap::new(),
        Vec::new(),
    );
    assert_eq!(run.end, End::Exhausted);
    assert_eq!(run.taken.len(), 3);
}

#[test]
fn the_worst_band_a_pass_sees_holds_until_the_target() {
    // Idle 3 hours goes only from urgent on; freeing lifts the disk into
    // pressure after the first batch, and the pass goes on with urgent's
    // windows to the target.
    let candidates = targets(40, 3.0 * HOUR);
    let halves = candidates
        .iter()
        .map(|c| (c.path.clone(), GIB / 2))
        .collect();
    let run = run(
        FakeDisk::new(7 * GIB),
        &candidates,
        Goal::Reach,
        halves,
        Vec::new(),
    );
    assert_eq!(run.end, End::Reached);
    assert_eq!(run.free, 20 * GIB);
}

#[test]
fn upkeep_takes_everything_due_on_a_healthy_disk() {
    let mut candidates = targets(3, 7.0 * HOUR);
    candidates.push(Candidate {
        path: PathBuf::from("/tmp/tmpabcdefgh"),
        class: Class::Temp,
        idle: 2.0 * 86400.0,
        size: GIB,
        occupied: false,
        spent: false,
        checkout: None,
    });
    let run = run(
        FakeDisk::new(50 * GIB),
        &candidates,
        Goal::Upkeep,
        BTreeMap::new(),
        Vec::new(),
    );
    assert_eq!(run.end, End::Exhausted);
    assert_eq!(run.taken, [Path::new("/tmp/tmpabcdefgh")]);
}
