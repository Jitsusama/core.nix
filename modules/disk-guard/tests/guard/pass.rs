use disk_guard::band::{Band, Space, Thresholds};
use disk_guard::pass::{
    Disk, End, Failing, Goal, Learned, Outcome, Reclaimer, Report, failed_line, failing, pass, told,
};
use disk_guard::plan::{Candidate, Class};
use disk_guard::units::float;
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

const GIB: u64 = 1 << 30;

/// A finished pass's report, with what came of each path it tried.
fn report(freed: u64, outcomes: Vec<Outcome>) -> Report {
    Report {
        end: End::Reached,
        band: Band::Filling,
        freed,
        actions: outcomes
            .into_iter()
            .enumerate()
            .map(|(number, outcome)| (PathBuf::from(format!("/tmp/taken-{number}")), outcome))
            .collect(),
    }
}

#[test]
fn a_pass_that_took_something_says_when_how_many_and_how_much_it_freed() {
    let report = report(
        8 * GIB + GIB / 4,
        vec![
            Outcome::Reclaimed,
            Outcome::Kept("in use".to_string()),
            Outcome::Reclaimed,
            Outcome::Failed("refused".to_string()),
        ],
    );
    assert_eq!(
        told(&report, 12.34, "2026-11-09 10:33:05").as_deref(),
        Some(
            "disk-guard: at 2026-11-09 10:33:05 a pass while filling took 2 of the 3 it \
             tried, freeing 8.2 GiB in 12.3 s"
        )
    );
}

#[test]
fn a_pass_whose_tries_all_failed_says_so_too() {
    let report = report(0, vec![Outcome::Failed("refused".to_string())]);
    assert_eq!(
        told(&report, 0.5, "2026-11-09 10:33:05").as_deref(),
        Some(
            "disk-guard: at 2026-11-09 10:33:05 a pass while filling took 0 of the 1 it \
             tried, freeing 0 B in 0.5 s"
        )
    );
}

#[test]
fn a_pass_that_only_judged_says_nothing() {
    let kept = report(0, vec![Outcome::Kept("in use".to_string())]);
    assert_eq!(told(&kept, 3.0, "2026-11-09 10:33:05"), None);
    assert_eq!(
        told(&report(0, Vec::new()), 3.0, "2026-11-09 10:33:05"),
        None
    );
}

// On 11 November a laptop's system guard spent 90 minutes with every
// delete of the store's garbage refused, each pass saying only that it
// took 0 of the 6 it tried, and its notice that nothing was left to clear.
#[test]
fn a_filling_pass_that_freed_nothing_for_its_failures_is_an_error() {
    let refused = "warning: unable to download 'https://cache.example/x.narinfo'\n\
                   retrying in 250 ms\n\
                   error: unable to download 'https://cache.example/x.narinfo': \
                   Could not resolve hostname\n";
    let report = report(
        0,
        vec![
            Outcome::Failed(refused.to_string()),
            Outcome::Kept("in use".to_string()),
            Outcome::Failed(refused.to_string()),
        ],
    );
    let failing = failing(&report).expect("a filling pass that freed nothing is failing");
    assert_eq!(
        failing,
        Failing {
            tried: 2,
            failed: 2,
            timed_out: false,
            last: "error: unable to download 'https://cache.example/x.narinfo': Could not \
                   resolve hostname"
                .to_string(),
        }
    );
    assert_eq!(
        failed_line(&failing, &report, "2026-11-11 15:40:00"),
        "disk-guard: error: at 2026-11-11 15:40:00 a pass while filling freed nothing: 2 of \
         the 2 it tried failed; the last: error: unable to download \
         'https://cache.example/x.narinfo': Could not resolve hostname"
    );
}

#[test]
fn a_timeout_is_an_error_even_when_the_rest_freed_something() {
    let report = report(
        3 * GIB,
        vec![
            Outcome::Reclaimed,
            Outcome::Failed("nix ran past 600s".to_string()),
        ],
    );
    let failing = failing(&report).expect("a delete timing out is failing");
    assert!(failing.timed_out, "{failing:?}");
    assert_eq!(
        failed_line(&failing, &report, "2026-11-11 15:40:00"),
        "disk-guard: error: at 2026-11-11 15:40:00 a pass while filling freed only 3.0 GiB: 1 \
         of the 2 it tried failed, one timing out; the last: nix ran past 600s"
    );
}

#[test]
fn what_freed_something_failed_at_nothing_or_ran_while_healthy_isnt_an_error() {
    let partly = report(
        GIB,
        vec![Outcome::Reclaimed, Outcome::Failed("refused".to_string())],
    );
    assert_eq!(failing(&partly), None, "a pass that freed something");
    let kept = report(0, vec![Outcome::Kept("in use".to_string())]);
    assert_eq!(failing(&kept), None, "a pass that only judged");
    let mut healthy = report(0, vec![Outcome::Failed("nix ran past 600s".to_string())]);
    healthy.band = Band::Healthy;
    assert_eq!(failing(&healthy), None, "upkeep on a healthy disk");
}

// On 14 November two switches each rooted paths a pass had listed as
// garbage, and each left an error line though the guard lists again.
#[test]
fn a_batch_gone_stale_since_the_listing_isnt_an_error() {
    let stale = || Outcome::Failed("3 paths came alive since the listing".to_string());
    let switched = report(0, vec![Outcome::Kept("in use".to_string()), stale()]);
    assert_eq!(
        failing(&switched),
        None,
        "a stale batch read as a failing guard"
    );
    let both = report(0, vec![stale(), Outcome::Failed("refused".to_string())]);
    let failing = failing(&both).expect("a refusal beside a stale batch is still failing");
    assert_eq!(
        (failing.failed, failing.last.as_str()),
        (1, "refused"),
        "a stale batch was counted among the failures"
    );
}
const HOUR: f64 = 3600.0;

const PRODUCER: Class = Class::Producer {
    every: None,
    from: Some(Band::Healthy),
};

/// A 100 GiB disk whose frees show only once settled, as Btrfs's do, and
/// which another writer may fill as the pass goes.
struct FakeDisk {
    free: u64,
    pending: u64,
    /// Bytes another writer takes at each settle, one entry per settle.
    fills: Vec<u64>,
    settles: usize,
    clock: f64,
    /// Seconds each reclaim takes, so batches end on their clock.
    each: f64,
}

impl FakeDisk {
    fn new(free: u64) -> FakeDisk {
        FakeDisk {
            free,
            pending: 0,
            fills: Vec::new(),
            settles: 0,
            clock: 0.0,
            each: 0.0,
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
    /// How many it takes before a stop waits, if one comes.
    stops_after: Option<usize>,
}

impl Reclaimer for FakeReclaimer<'_> {
    fn stopping(&self) -> bool {
        self.stops_after
            .is_some_and(|after| self.taken.len() >= after)
    }

    fn reclaim(&mut self, candidate: &Candidate) -> Outcome {
        {
            let mut disk = self.disk.borrow_mut();
            disk.clock += disk.each;
        }
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
        filling: 15 * GIB,
        nearly_full: 8 * GIB,
        critically_full: 3 * GIB,
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
            idle: idle + float(u64::try_from(count - n).unwrap()),
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
        stops_after: None,
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
fn a_command_is_measured_before_anything_more_is_taken() {
    let producer = Candidate {
        path: PathBuf::from("/home/u/src#gc"),
        class: PRODUCER,
        idle: f64::INFINITY,
        size: 0,
        occupied: false,
        spent: false,
        checkout: None,
    };
    let mut candidates = vec![producer.clone()];
    candidates.extend(targets(10, 7.0 * HOUR));
    let run = run(
        FakeDisk::new(14 * GIB),
        &candidates,
        Goal::Reach,
        [(producer.path.clone(), 6 * GIB)].into(),
        Vec::new(),
    );
    assert_eq!(run.end, End::Reached);
    assert_eq!(run.taken, [producer.path], "what it freed was enough");
}

#[test]
fn a_command_is_measured_alone_so_what_it_frees_is_no_one_elses() {
    let producer = Candidate {
        path: PathBuf::from("/home/u/src#gc"),
        class: PRODUCER,
        idle: f64::INFINITY,
        size: 0,
        occupied: false,
        spent: false,
        checkout: None,
    };
    let temp = Candidate {
        path: PathBuf::from("/tmp/tmp.Ab3dE9fGh1"),
        class: Class::Temp,
        idle: 30.0 * 86_400.0,
        size: GIB,
        ..producer.clone()
    };
    let mut candidates = vec![temp.clone(), producer.clone()];
    candidates.extend(targets(10, 7.0 * HOUR));
    let run = run(
        FakeDisk::new(14 * GIB),
        &candidates,
        Goal::Reach,
        [(producer.path.clone(), 4 * GIB)].into(),
        Vec::new(),
    );
    assert_eq!(run.end, End::Reached);
    assert_eq!(run.taken[..2], [temp.path, producer.path]);
    assert_eq!(run.taken.len(), 3, "one target made up the rest");
    let ratio = run.learned.ratio(Class::Temp);
    assert!(
        (ratio - 1.0).abs() < 0.05,
        "temp freed its own size: {ratio}"
    );
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
    crate::assert_exactly(run.learned.ratio(Class::Target), 1.0);
}

/// `count` temp files of `size` each, long idle, so due at any band.
fn temps(count: usize, size: u64) -> Vec<Candidate> {
    (0..count)
        .map(|n| Candidate {
            path: PathBuf::from(format!("/tmp/tmp.{n:04}")),
            class: Class::Temp,
            idle: 30.0 * 86_400.0 + float(u64::try_from(count - n).unwrap()),
            size,
            occupied: false,
            spent: false,
            checkout: None,
        })
        .collect()
}

#[test]
fn small_batches_a_writer_swamps_dont_stop_the_pass() {
    // Batches of temp files a few MiB each can't be told from a writer
    // filling the disk as they go, so they measure nothing, and the pass
    // still reaches the targets that make up the shortfall.
    let mut candidates = temps(60, 16 << 20);
    candidates.extend(targets(10, 7.0 * HOUR));
    let nothing = candidates[..60]
        .iter()
        .map(|c| (c.path.clone(), 0))
        .collect();
    let mut disk = FakeDisk::new(14 * GIB);
    disk.each = 1.0;
    let run = run(disk, &candidates, Goal::Reach, nothing, Vec::new());
    assert_eq!(run.end, End::Reached);
    assert_eq!(run.free, 20 * GIB);
    // Too small to learn from.
    crate::assert_exactly(run.learned.ratio(Class::Temp), 1.0);
}

#[test]
fn a_group_that_stalls_gives_way_to_the_next() {
    // Temp files that free nothing, as hard links to files kept elsewhere
    // do, stop their own group after three batches, not the pass.
    let mut candidates = temps(20, GIB);
    candidates.extend(targets(10, 7.0 * HOUR));
    let nothing = candidates[..20]
        .iter()
        .map(|c| (c.path.clone(), 0))
        .collect();
    let mut disk = FakeDisk::new(14 * GIB);
    disk.each = 1.0;
    let run = run(disk, &candidates, Goal::Reach, nothing, Vec::new());
    assert_eq!(run.end, End::Reached);
    let temp = |path: &PathBuf| path.starts_with("/tmp");
    assert_eq!(
        run.taken.iter().filter(|path| temp(path)).count(),
        15,
        "three batches of five"
    );
    assert_eq!(run.taken.iter().filter(|path| !temp(path)).count(), 6);
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
    // Idle 3 hours goes only once nearly full; freeing lifts the disk to
    // filling after the first batch, and the pass goes on with nearly full's
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

fn temps_of_two_days(count: usize) -> Vec<Candidate> {
    (0..count)
        .map(|n| Candidate {
            path: PathBuf::from(format!("/tmp/tmp{n:02}cdefgh")),
            class: Class::Temp,
            idle: 2.0 * 86400.0,
            size: GIB,
            occupied: false,
            spent: false,
            checkout: None,
        })
        .collect()
}

#[test]
fn a_routine_pass_ends_unfinished_after_twenty_minutes() {
    // On 12 November a laptop's upkeep had a day and a half of entries
    // due, and a pass that never ended never recorded its upkeep.
    let mut disk = FakeDisk::new(50 * GIB);
    disk.each = 100.0;
    let run = run(
        disk,
        &temps_of_two_days(20),
        Goal::Upkeep,
        BTreeMap::new(),
        Vec::new(),
    );
    assert_eq!(run.end, End::Unfinished, "took {:?}", run.taken);
    assert_eq!(
        run.taken.len(),
        12,
        "twelve entries of 100 s fill twenty minutes: {:?}",
        run.taken
    );
}

#[test]
fn a_pass_for_free_space_runs_as_long_as_it_needs() {
    // Only routine work waits for the next step: a filling disk's pass
    // goes on to its target.
    let mut disk = FakeDisk::new(10 * GIB);
    disk.each = 100.0;
    let run = run(
        disk,
        &temps_of_two_days(20),
        Goal::Reach,
        BTreeMap::new(),
        Vec::new(),
    );
    assert_eq!(run.end, End::Reached, "took {:?}", run.taken);
    assert_eq!(
        run.taken.len(),
        10,
        "10 GiB short of the target: {:?}",
        run.taken
    );
}

#[test]
fn a_stop_ends_a_pass_before_its_next_candidate() {
    let candidates: Vec<Candidate> = (0..5)
        .map(|n| Candidate {
            path: PathBuf::from(format!("/tmp/tmp{n}bcdefgh")),
            class: Class::Temp,
            idle: 2.0 * 86400.0,
            size: GIB,
            occupied: false,
            spent: false,
            checkout: None,
        })
        .collect();
    let disk = std::cell::RefCell::new(FakeDisk::new(50 * GIB));
    let mut reclaimer = FakeReclaimer {
        disk: &disk,
        frees: BTreeMap::new(),
        kept: Vec::new(),
        taken: Vec::new(),
        stops_after: Some(2),
    };
    let report = pass(
        &mut Shared(&disk),
        &mut reclaimer,
        &candidates,
        &thresholds(),
        Band::Healthy,
        Goal::Upkeep,
        &mut Learned::default(),
    );
    assert_eq!(report.end, End::Stopped);
    assert_eq!(reclaimer.taken.len(), 2, "took {:?}", reclaimer.taken);
}
