use disk_guard::band::Band;
use disk_guard::plan::{Candidate, Class, Group, plan};
use std::path::PathBuf;

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
    assert!(taken(std::slice::from_ref(&tree), Band::Pressure).is_empty());
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
    assert!(taken(&both, Band::Healthy).is_empty());
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
        candidate("producer", Class::Producer, 0.0),
        candidate("variant", Class::Variant, 3.0 * HOUR),
        candidate("garbage", Class::Garbage, 0.0),
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
fn a_plan_stops_once_its_predicted_free_covers_the_shortfall() {
    let candidates: Vec<_> = (0..10)
        .map(|n| candidate(&format!("t{n}"), Class::Target, (10 + n) as f64 * HOUR))
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
