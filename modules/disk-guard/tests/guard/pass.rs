//! A pass over a fake disk: what it takes, measures, learns and waits for,
//! and where it ends. What a finished pass says is in `pass_lines`.

use disk_guard::band::{Band, Space, Thresholds};
use disk_guard::decide::Decision;
use disk_guard::pass::{
    Disk, End, Goal, Learned, Outcome, Reclaimer, left_to_judge, pass, skipped_after,
};
use disk_guard::plan::{Candidate, Class};
use disk_guard::units::float;
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};

const GIB: u64 = 1 << 30;

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
    /// Whether what it keeps waits for another process, as garbage does.
    waits: bool,
    /// How many it has kept.
    held: usize,
}

impl Reclaimer for FakeReclaimer<'_> {
    fn stopping(&self) -> bool {
        self.stops_after
            .is_some_and(|after| self.taken.len() >= after)
    }

    fn waiting(&self) -> bool {
        self.waits && self.held > 0
    }

    fn reclaim(&mut self, candidate: &Candidate) -> Outcome {
        {
            let mut disk = self.disk.borrow_mut();
            disk.clock += disk.each;
        }
        if self.kept.contains(&candidate.path) {
            self.held += 1;
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
        waits: false,
        held: 0,
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

/// How a pass of `goal` over ten temp files ends when each is kept to
/// wait, and how many it asked for.
fn waiting_on_each(goal: Goal) -> (End, usize) {
    let candidates = temps_of_two_days(10);
    let disk = std::cell::RefCell::new(FakeDisk::new(14 * GIB));
    let mut reclaimer = FakeReclaimer {
        disk: &disk,
        frees: BTreeMap::new(),
        kept: candidates.iter().map(|c| c.path.clone()).collect(),
        taken: Vec::new(),
        stops_after: None,
        waits: true,
        held: 0,
    };
    let report = pass(
        &mut Shared(&disk),
        &mut reclaimer,
        &candidates,
        &thresholds(),
        Band::Filling,
        goal,
        &mut Learned::default(),
    );
    (report.end, reclaimer.held)
}

#[test]
fn a_pass_reaching_for_room_ends_where_what_it_kept_waits() {
    // Garbage kept for Nix's rest or another writer goes first as the
    // cheapest loss, so what costs more waits with it rather than going in
    // its place.
    let (end, asked) = waiting_on_each(Goal::Reach);
    assert_eq!(end, End::Waiting);
    assert_eq!(asked, 1, "went on past the wait");
}

#[test]
fn a_waiting_pass_learns_nothing_from_a_batch_that_stalled() {
    // A temp freed nothing before the garbage after it was kept to wait.
    let candidates = temps(10, GIB / 4);
    let disk = std::cell::RefCell::new(FakeDisk::new(14 * GIB));
    let first = candidates[0].path.clone();
    let mut reclaimer = FakeReclaimer {
        disk: &disk,
        frees: BTreeMap::from([(first.clone(), 0)]),
        kept: candidates[1..].iter().map(|c| c.path.clone()).collect(),
        taken: Vec::new(),
        stops_after: None,
        waits: true,
        held: 0,
    };
    let mut learned = Learned::default();
    let report = pass(
        &mut Shared(&disk),
        &mut reclaimer,
        &candidates,
        &thresholds(),
        Band::Filling,
        Goal::Reach,
        &mut learned,
    );
    assert_eq!(report.end, End::Waiting);
    assert_eq!(reclaimer.taken, [first], "not the one temp, then the wait");
    let ratio = learned.ratio(Class::Temp);
    assert!(
        (ratio - 1.0).abs() < f64::EPSILON,
        "a stalled batch taught a ratio of {ratio}"
    );
}

#[test]
fn upkeep_looks_at_all_that_is_due_past_what_waits() {
    let (end, asked) = waiting_on_each(Goal::Upkeep);
    assert_ne!(end, End::Waiting, "upkeep ended at a wait");
    assert_eq!(asked, 10, "upkeep stopped at a wait");
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
fn a_filling_disk_s_pass_ends_unfinished_after_two_hours() {
    // On 23 November an account's reach judged a survey's 88,000 offers
    // for 15 hours, never seeing what was new and never saying a pass.
    let mut disk = FakeDisk::new(14 * GIB);
    disk.each = 10.0;
    let run = run(
        disk,
        &temps(2000, 1 << 20),
        Goal::Reach,
        BTreeMap::new(),
        Vec::new(),
    );
    assert_eq!(run.end, End::Unfinished, "took {}", run.taken.len());
    assert!(
        (720..2000).contains(&run.taken.len()),
        "two hours of 10 s entries is 720, short of all 2,000: took {}",
        run.taken.len()
    );
}

#[test]
fn a_nearly_full_disk_s_pass_goes_on_past_two_hours() {
    // A survey there would leave the disk with nothing cleared while it
    // walks, so the pass goes on through what it was offered.
    let mut disk = FakeDisk::new(7 * GIB);
    disk.each = 10.0;
    let run = run(
        disk,
        &temps(2000, 1 << 20),
        Goal::Reach,
        BTreeMap::new(),
        Vec::new(),
    );
    assert_eq!(run.end, End::Exhausted, "took {}", run.taken.len());
    assert_eq!(run.taken.len(), 2000, "every offer judged");
}

fn paths(names: &[&str]) -> BTreeSet<PathBuf> {
    names.iter().map(PathBuf::from).collect()
}

const FILLING: Decision = Decision::Reach(Band::Filling);

#[test]
fn a_cut_reach_keeps_the_files_it_tried_and_never_a_named_offer() {
    // The oldest garbage batch is named garbage-0 every round, whatever
    // paths it holds, so keeping its name would skip garbage nobody judged.
    let tried = paths(&["/d/cache", "/nix/store#garbage-0", "/d#podman-prune"]);
    let first = skipped_after(None, FILLING, &End::Unfinished, &tried);
    assert_eq!(first, Some(paths(&["/d/cache"])), "only the file is kept");
    let second = skipped_after(first, FILLING, &End::Unfinished, &paths(&["/d/logs"]));
    assert_eq!(
        second,
        Some(paths(&["/d/cache", "/d/logs"])),
        "a second cut adds to the first"
    );
}

#[test]
fn a_reach_that_waits_stalls_or_stops_and_a_collection_keep_what_was_skipped() {
    let skipped = Some(paths(&["/d/cache"]));
    let tried = paths(&["/d/logs"]);
    for end in [End::Waiting, End::Stalled, End::Stopped] {
        assert_eq!(
            skipped_after(skipped.clone(), FILLING, &end, &tried),
            skipped,
            "a filling reach ending {end:?} keeps it as it was"
        );
    }
    // Nix's garbage is collected in a reach's rest, which the spell
    // outlasts.
    assert_eq!(
        skipped_after(skipped.clone(), Decision::Collect, &End::Reached, &tried),
        skipped,
        "a collection keeps it"
    );
}

#[test]
fn a_reach_that_gets_through_runs_at_a_worse_band_or_upkeep_skips_nothing_more() {
    let skipped = Some(paths(&["/d/cache"]));
    let tried = paths(&["/d/logs"]);
    for end in [End::Reached, End::Exhausted, End::Failed("no".into())] {
        assert_eq!(
            skipped_after(skipped.clone(), FILLING, &end, &tried),
            None,
            "a filling reach ending {end:?} lets it go"
        );
    }
    let nearly = Decision::Reach(Band::NearlyFull);
    assert_eq!(
        skipped_after(skipped.clone(), nearly, &End::Unfinished, &tried),
        None,
        "no length cuts a nearly full reach, so it keeps nothing"
    );
    assert_eq!(
        skipped_after(skipped, Decision::Upkeep, &End::Unfinished, &tried),
        None,
        "upkeep comes only to a healthy disk, past the spell"
    );
}

/// An offer's path, for `left_to_judge`, named since a closure can't say the borrow it returns.
fn path(offer: &PathBuf) -> &Path {
    offer
}

#[test]
fn a_reach_skips_what_was_judged_unless_that_leaves_it_nothing() {
    let offers: Vec<PathBuf> = ["/d/cache", "/d/logs"].map(PathBuf::from).to_vec();
    assert_eq!(
        left_to_judge(&offers, Some(&paths(&["/d/cache"])), path),
        vec![PathBuf::from("/d/logs")],
        "what was judged is skipped"
    );
    assert_eq!(
        left_to_judge(&offers, Some(&paths(&["/d/cache", "/d/logs"])), path),
        offers,
        "given nothing, the reach would call the disk exhausted over offers it never saw"
    );
    assert_eq!(
        left_to_judge(&offers, None, path),
        offers,
        "nothing skipped"
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
        waits: false,
        held: 0,
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
