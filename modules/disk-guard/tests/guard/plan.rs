use disk_guard::band::Band;
use disk_guard::plan::{Candidate, Class, Group, plan, windows};
use disk_guard::units::{float, whole};
use std::path::PathBuf;

/// A small seeded generator, so a failing case can be named and replayed.
struct Draw(u64);

impl Draw {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    /// A draw below `n`, as an index or a count.
    fn index(&mut self, n: usize) -> usize {
        usize::try_from(self.below(n as u64)).unwrap()
    }

    fn band(&mut self) -> Band {
        [Band::Healthy, Band::Pressure, Band::Urgent, Band::Critical][self.index(4)]
    }

    fn class(&mut self) -> Class {
        let bool = |draw: &mut Draw| draw.below(2) == 0;
        match self.below(17) {
            0 => Class::Target,
            1 => Class::Install,
            2 => Class::Variant,
            3 => Class::Scratch {
                healthy: bool(self).then_some(30.0 * DAY),
            },
            4 => Class::Snapshot,
            5 => Class::Workspace,
            6 => Class::Temp,
            7 => Class::Trash,
            8 => Class::Cache {
                rank: u8::try_from(self.below(40)).unwrap(),
                from: self.band(),
            },
            9 => Class::Coredump,
            10 => Class::NixBuild,
            11 => Class::Replaced,
            12 => Class::Garbage {
                kept: bool(self),
                order: u16::try_from(self.below(20)).unwrap(),
            },
            13 => Class::Generations { kept: bool(self) },
            14 => Class::Producer {
                every: bool(self).then_some(6.0 * HOUR),
                from: bool(self).then(|| self.band()),
            },
            15 => Class::Log,
            _ => Class::Podman { unused: bool(self) },
        }
    }

    fn candidate(&mut self, n: usize) -> Candidate {
        Candidate {
            path: PathBuf::from(format!("/home/u/{n}")),
            class: self.class(),
            // From fresh to a fortnight, often at a window's very edge.
            idle: [
                0.0,
                HOUR,
                2.0 * HOUR,
                6.0 * HOUR,
                DAY,
                DAY + HOUR,
                7.0 * DAY,
            ][self.index(7)]
                + float(self.below(2)) * float(self.below(14 * whole(DAY))),
            size: self.below(4 * GIB),
            occupied: self.below(4) == 0,
            spent: self.below(4) == 0,
            checkout: None,
        }
    }
}

const HOUR: f64 = 3600.0;
const DAY: f64 = 86400.0;
const GIB: u64 = 1 << 30;

fn candidate(name: &str, class: Class, idle: f64) -> Candidate {
    Candidate {
        path: PathBuf::from(format!("/home/u/{name}")),
        class,
        idle,
        size: GIB,
        occupied: false,
        spent: false,
        checkout: None,
    }
}

fn occupied(mut candidate: Candidate) -> Candidate {
    candidate.occupied = true;
    candidate
}

fn spent(mut candidate: Candidate) -> Candidate {
    candidate.spent = true;
    candidate
}

/// The names a plan takes, in order, with the whole shortfall unbounded.
fn taken(candidates: &[Candidate], band: Band) -> Vec<String> {
    plan(candidates, band, None, |_| 1.0)
        .iter()
        .map(|step| {
            candidates[step.index]
                .path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

/// One of each kind, aged so each band takes a different share.
fn everything() -> Vec<Candidate> {
    vec![
        candidate("target-5h", Class::Target, 5.0 * HOUR),
        candidate("target-7h", Class::Target, 7.0 * HOUR),
        spent(candidate("target-merged", Class::Target, 0.0)),
        occupied(candidate("target-shell-23h", Class::Target, 23.0 * HOUR)),
        occupied(candidate("target-shell-25h", Class::Target, 25.0 * HOUR)),
        candidate("install-7h", Class::Install, 7.0 * HOUR),
        occupied(candidate("install-shell-2d", Class::Install, 2.0 * DAY)),
        candidate("variant-8d", Class::Variant, 8.0 * DAY),
        candidate("variant-2d", Class::Variant, 2.0 * DAY),
        candidate(
            "cache-go",
            Class::Cache {
                rank: 0,
                from: Band::Urgent,
            },
            0.0,
        ),
        candidate("temp-2d", Class::Temp, 2.0 * DAY),
        candidate("trash-31d", Class::Trash, 31.0 * DAY),
        candidate("trash-8d", Class::Trash, 8.0 * DAY),
        candidate("core-2d", Class::Coredump, 2.0 * DAY),
        candidate("core-1h", Class::Coredump, HOUR),
        candidate("replaced-2d", Class::Replaced, 2.0 * DAY),
        candidate("scratch-2d", Class::Scratch { healthy: None }, 2.0 * DAY),
        candidate(
            "scratch-31d",
            Class::Scratch {
                healthy: Some(30.0 * DAY),
            },
            31.0 * DAY,
        ),
    ]
}

fn sorted(mut names: Vec<String>) -> Vec<String> {
    names.sort();
    names
}

#[test]
fn healthy_takes_only_waste_and_week_old_leftovers() {
    let names = sorted(taken(&everything(), Band::Healthy));
    let expected = [
        "scratch-31d",
        "target-merged",
        "temp-2d",
        "trash-31d",
        "variant-8d",
    ];
    assert_eq!(names, expected);
}

#[test]
fn pressure_adds_idle_targets_but_not_installs_or_occupied_trees() {
    let names = sorted(taken(&everything(), Band::Pressure));
    let expected = [
        "replaced-2d",
        "scratch-2d",
        "scratch-31d",
        "target-7h",
        "target-merged",
        "temp-2d",
        "trash-31d",
        "trash-8d",
        "variant-2d",
        "variant-8d",
    ];
    assert_eq!(names, expected);
}

#[test]
fn urgent_adds_caches_idle_installs_old_cores_and_trees_occupied_a_day() {
    let names = taken(&everything(), Band::Urgent);
    for name in [
        "cache-go",
        "install-7h",
        "core-2d",
        "target-shell-25h",
        "target-5h",
    ] {
        assert!(names.iter().any(|n| n == name), "{name} stayed: {names:?}");
    }
    for name in ["target-shell-23h", "install-shell-2d", "core-1h"] {
        assert!(!names.iter().any(|n| n == name), "{name} went: {names:?}");
    }
}

#[test]
fn critical_takes_everything_offered() {
    assert_eq!(
        taken(&everything(), Band::Critical).len(),
        everything().len()
    );
}

#[test]
fn a_finished_tree_someone_sits_in_is_judged_as_occupied() {
    let tree = occupied(spent(candidate("merged-shell", Class::Target, HOUR)));
    assert_eq!(
        taken(std::slice::from_ref(&tree), Band::Pressure),
        Vec::<String>::new()
    );
    assert_eq!(
        plan(&[tree], Band::Critical, None, |_| 1.0)[0].group,
        Group::Occupied
    );
}

#[test]
fn a_cache_is_taken_from_its_own_band_on() {
    let soon = candidate(
        "soon",
        Class::Cache {
            rank: 0,
            from: Band::Pressure,
        },
        0.0,
    );
    let late = candidate(
        "late",
        Class::Cache {
            rank: 0,
            from: Band::Critical,
        },
        0.0,
    );
    let both = [soon, late];
    assert_eq!(taken(&both, Band::Healthy), Vec::<String>::new());
    assert_eq!(taken(&both, Band::Pressure), ["soon"]);
    assert_eq!(taken(&both, Band::Urgent), ["soon"]);
    assert_eq!(taken(&both, Band::Critical), ["soon", "late"]);
}

#[test]
fn the_plan_goes_cheapest_loss_first() {
    let candidates = vec![
        occupied(candidate("occupied", Class::Target, 0.0)),
        candidate("replaced", Class::Replaced, 2.0 * DAY),
        candidate("idle-newer", Class::Target, HOUR),
        candidate("idle-older", Class::Target, 9.0 * HOUR),
        candidate(
            "cache-modules",
            Class::Cache {
                rank: 5,
                from: Band::Urgent,
            },
            0.0,
        ),
        candidate(
            "cache-build",
            Class::Cache {
                rank: 0,
                from: Band::Urgent,
            },
            0.0,
        ),
        candidate(
            "producer",
            Class::Producer {
                every: None,
                from: Some(Band::Pressure),
            },
            2.0 * HOUR,
        ),
        candidate("variant", Class::Variant, 3.0 * HOUR),
        candidate(
            "garbage",
            Class::Garbage {
                kept: true,
                order: 0,
            },
            0.0,
        ),
        candidate("snapshot", Class::Snapshot, 0.0),
    ];
    let order = taken(&candidates, Band::Critical);
    let expected = [
        "snapshot",
        "garbage",
        "variant",
        "producer",
        "cache-build",
        "cache-modules",
        "idle-older",
        "idle-newer",
        "replaced",
        "occupied",
    ];
    assert_eq!(order, expected);
}

#[test]
fn waste_goes_largest_first() {
    // Waste costs nothing to lose, so what frees most goes first: taken
    // longest idle first on 28 October, six thousand small temp entries
    // from before the boot kept a pass from 185 GiB of spent builds for an
    // hour.
    let sized = |mut candidate: Candidate, size: u64| {
        candidate.size = size;
        candidate
    };
    let candidates = vec![
        sized(candidate("temp-old", Class::Temp, 10.0 * DAY), 4096),
        sized(
            spent(candidate("build", Class::Target, 8.0 * HOUR)),
            50 * GIB,
        ),
        sized(candidate("trash-old", Class::Trash, 31.0 * DAY), GIB),
    ];
    assert_eq!(
        taken(&candidates, Band::Healthy),
        ["build", "trash-old", "temp-old"]
    );
}

#[test]
fn a_plan_stops_once_its_predicted_free_covers_the_shortfall() {
    let candidates: Vec<_> = (0..10)
        .map(|n| candidate(&format!("t{n}"), Class::Target, f64::from(10 + n) * HOUR))
        .collect();
    let steps = plan(&candidates, Band::Pressure, Some(3 * GIB), |_| 1.0);
    assert_eq!(steps.len(), 3);
    // Compressed targets free half their size, so twice as many are planned.
    let steps = plan(&candidates, Band::Pressure, Some(3 * GIB), |class| {
        if class == Class::Target { 0.5 } else { 1.0 }
    });
    assert_eq!(steps.len(), 6);
    assert_eq!(steps[0].predicted, GIB / 2);
}

#[test]
fn a_producer_runs_on_its_schedule_and_from_its_band_at_most_hourly() {
    let runs = |every: Option<f64>, from: Option<Band>, idle: f64, band: Band| {
        let producer = [candidate("gc", Class::Producer { every, from }, idle)];
        !plan(&producer, band, None, |_| 1.0).is_empty()
    };
    let six = Some(6.0 * HOUR);
    assert!(runs(six, None, 7.0 * HOUR, Band::Healthy), "due");
    assert!(!runs(six, None, 5.0 * HOUR, Band::Healthy), "not yet due");
    assert!(
        !runs(six, None, 5.0 * HOUR, Band::Critical),
        "no band of its own"
    );
    let pressure = Some(Band::Pressure);
    assert!(!runs(None, pressure, f64::INFINITY, Band::Healthy));
    assert!(runs(None, pressure, 2.0 * HOUR, Band::Pressure));
    assert!(
        !runs(None, pressure, 0.5 * HOUR, Band::Pressure),
        "ran within the hour"
    );
    assert!(runs(None, pressure, 2.0 * HOUR, Band::Critical));
    assert!(
        runs(six, pressure, 7.0 * HOUR, Band::Healthy),
        "either says when"
    );
}

#[test]
fn podman_prunes_after_the_caches_from_urgent_and_its_unused_images_at_critical() {
    let dangling = Class::Podman { unused: false };
    let unused = Class::Podman { unused: true };
    let cache = Class::Cache {
        rank: 200,
        from: Band::Urgent,
    };
    let candidates = [
        candidate("podman-unused", unused, 2.0 * HOUR),
        candidate("podman", dangling, 2.0 * HOUR),
        candidate("cache", cache, 0.0),
        candidate("idle", Class::Target, 7.0 * HOUR),
    ];
    assert_eq!(taken(&candidates, Band::Pressure), ["idle"]);
    assert_eq!(
        taken(&candidates, Band::Urgent),
        ["cache", "podman", "idle"]
    );
    assert_eq!(
        taken(&candidates, Band::Critical),
        ["cache", "podman", "idle", "podman-unused"]
    );
    let recent = [
        candidate("podman-unused", unused, 0.5 * HOUR),
        candidate("podman", dangling, 0.5 * HOUR),
    ];
    assert!(taken(&recent, Band::Critical).is_empty(), "at most hourly");
    assert!(dangling.command() && unused.command());
}

#[test]
fn garbage_goes_in_its_own_order_and_healthy_keeps_what_its_budget_allows() {
    let garbage = |kept, order| Class::Garbage { kept, order };
    // Idle says nothing of garbage's order: the policy already set it.
    let candidates = [
        candidate("second", garbage(false, 1), 9.0 * HOUR),
        candidate("kept", garbage(true, 2), 10.0 * HOUR),
        candidate("first", garbage(false, 0), 1.0 * HOUR),
    ];
    assert_eq!(taken(&candidates, Band::Healthy), ["first", "second"]);
    assert_eq!(
        taken(&candidates, Band::Pressure),
        ["first", "second", "kept"]
    );
    assert!(garbage(true, 0).command(), "measured alone");
}

#[test]
fn an_oversized_log_is_trimmed_as_waste_in_every_band() {
    let log = [candidate("app.log", Class::Log, 0.0)];
    for band in [Band::Healthy, Band::Pressure, Band::Urgent, Band::Critical] {
        let steps = plan(&log, band, None, |_| 1.0);
        assert_eq!(steps.len(), 1, "{band:?}");
        assert_eq!(steps[0].group, Group::Waste, "{band:?}");
    }
}

/// Whether a band's window has passed for a candidate, read from the table
/// alone.
fn due(candidate: &Candidate, band: Band) -> bool {
    windows(candidate).1[band as usize].is_some_and(|window| candidate.idle >= window)
}

#[test]
fn property_a_plan_takes_only_what_is_due_cheapest_first_and_stops_once_covered() {
    for case in 1..=3000_u64 {
        let mut draw = Draw(case.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let band = draw.band();
        let candidates: Vec<Candidate> = (0..draw.index(40)).map(|n| draw.candidate(n)).collect();
        let ratio = |class: Class| if class == Class::Target { 0.5 } else { 1.0 };
        let all = plan(&candidates, band, None, ratio);
        let mut indices: Vec<usize> = all.iter().map(|step| step.index).collect();
        indices.sort_unstable();
        indices.dedup();
        assert_eq!(indices.len(), all.len(), "case {case}: a candidate twice");
        let wanted: Vec<usize> = (0..candidates.len())
            .filter(|&index| due(&candidates[index], band))
            .collect();
        assert_eq!(indices, wanted, "case {case}: what's due in {band:?}");
        for step in &all {
            assert_eq!(
                step.group,
                windows(&candidates[step.index]).0,
                "case {case}"
            );
        }
        assert!(
            all.windows(2).all(|pair| pair[0].group <= pair[1].group),
            "case {case}: cheapest loss first"
        );
        let shortfall = draw.below(16 * GIB);
        let bounded = plan(&candidates, band, Some(shortfall), ratio);
        assert_eq!(bounded, all[..bounded.len()], "case {case}: a prefix");
        let predicted = |steps: &[_]| -> u64 {
            steps
                .iter()
                .map(|step: &disk_guard::plan::Step| step.predicted)
                .sum()
        };
        if let Some((_, before)) = bounded.split_last() {
            assert!(
                predicted(before) < shortfall,
                "case {case}: went on past covering the shortfall"
            );
        }
        if predicted(&all) >= shortfall {
            assert!(
                predicted(&bounded) >= shortfall,
                "case {case}: stopped short"
            );
        } else {
            assert_eq!(bounded.len(), all.len(), "case {case}: took all it could");
        }
    }
}
