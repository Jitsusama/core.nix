//! What cargo builds and package managers install in checkouts and trees,
//! as the survey offers and sizes it.

use crate::discover::{CARGO_TAG, checkout, dir, file};
use crate::survey::{
    DAY, config, find, now, offers, offers_awake, offers_using, real, src, surveyed,
    surveyed_pinned, written_ago,
};
use disk_guard::activity::Process;
use disk_guard::clock::{Awake, Tick};
use disk_guard::config::Config;
use disk_guard::plan::{self, Class, Group};
use disk_guard::replaced::{Uses, watched_dirs};
use disk_guard::survey::{Action, BUDGET_RECENT, Sizes};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;

#[test]
fn an_older_build_is_offered_once_its_deps_is_watched() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let roots = dir(&home.join("src"));
    let (_, target, _) = src(&roots);
    let deps = target.join("debug/deps");
    for (name, ago) in [
        ("libapp-1111111111111111.rlib", 5.0 * DAY),
        ("libapp-1111111111111111.rmeta", 5.0 * DAY),
        ("libapp-0123456789abcdef.rlib", 0.0),
    ] {
        file(&deps.join(name), "built");
        written_ago(&deps.join(name), ago);
    }
    let config = config(&format!(
        r#""cargo": {{ "kind": "cargo", "roots": ["{}"] }}"#,
        roots.display()
    ));
    let replaced = deps.join("app-1111111111111111");
    let none = offers(&config, &[], &|_| false);
    assert!(
        none.iter().all(|offer| offer.candidate.path != replaced),
        "nothing watched it, so nothing knows it's unused"
    );
    let mut uses = Uses::default();
    uses.watching(&deps, now() - 3.0 * DAY);
    let found = offers_using(&config, &[], &|_| false, &BTreeMap::new(), &uses);
    let older = find(&found, &replaced);
    assert_eq!(older.candidate.class, Class::Replaced);
    assert!(
        (older.candidate.idle - 3.0 * DAY).abs() < 60.0,
        "{}",
        older.candidate.idle
    );
    assert_eq!(older.candidate.size, 10);
    assert_eq!(
        older.action,
        Action::Build {
            root: roots.clone(),
            target: target.clone()
        }
    );
    let newest = deps.join("app-0123456789abcdef");
    assert!(
        found.iter().all(|offer| offer.candidate.path != newest),
        "the newest build of a crate is never offered as replaced"
    );
}

#[test]
fn targets_and_installs_are_offered_under_their_roots() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let roots = dir(&home.join("src"));
    let (app, target, modules) = src(&roots);
    let config = config(&format!(
        r#""cargo": {{ "kind": "cargo", "roots": ["{0}"] }},
           "installs": {{ "kind": "installs", "roots": ["{0}"] }}"#,
        roots.display()
    ));
    let found = offers(&config, &[], &|dir| dir == app);
    let built = find(&found, &target);
    assert_eq!(built.candidate.class, Class::Target);
    assert!(built.candidate.spent, "its checkout's work is finished");
    assert_eq!(built.candidate.checkout.as_deref(), Some(app.as_path()));
    assert_eq!(
        built.action,
        Action::Target {
            root: roots.clone()
        }
    );
    assert_eq!(built.device, disk_guard::pool::pool(&roots).unwrap());
    let installed = find(&found, &modules);
    assert_eq!(installed.candidate.class, Class::Install);
    assert_eq!(
        installed.action,
        Action::Entry {
            root: roots.clone()
        }
    );
    let unfinished = offers(&config, &[], &|_| false);
    assert!(
        !find(&unfinished, &target).candidate.spent,
        "a build whose checkout's work goes on isn't spent"
    );
}

#[test]
fn a_root_targets_and_installs_share_is_walked_once() {
    // One walk finds both, so walking a root once a source is twice the
    // work: on 2 November the account's three roots of checkouts and
    // trees were each walked for cargo's targets and again for installs.
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let roots = dir(&home.join("src"));
    let (_, target, modules) = src(&roots);
    let config = config(&format!(
        r#""cargo": {{ "kind": "cargo", "roots": ["{0}"] }},
           "installs": {{ "kind": "installs", "roots": ["{0}"] }}"#,
        roots.display()
    ));
    let (found, walked, _) = surveyed(
        &config,
        &[],
        &|_| true,
        &BTreeMap::new(),
        &Uses::default(),
        (now(), Sizes::default()),
    );
    assert_eq!(walked, 1);
    assert_eq!(find(&found, &target).candidate.class, Class::Target);
    assert_eq!(find(&found, &modules).candidate.class, Class::Install);
}

#[test]
fn a_root_named_through_a_link_holds_what_it_offers() {
    // A Mac's /var is a link to /private/var, so its temporary folder is
    // named through a link, and what's found there is named where it lives.
    // Each offer's root must name that same place, or reclaiming refuses
    // the entry as outside its root.
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let private = dir(&home.join("private"));
    let temp = dir(&private.join("tmp"));
    let leftover = dir(&temp.join("tmp.Ab3dE9fGh1"));
    file(&leftover.join("f"), "x");
    let (_, target, modules) = src(&dir(&private.join("src")));
    std::os::unix::fs::symlink(&private, home.join("var")).unwrap();
    let config = config(&format!(
        r#""temp": {{ "kind": "temp", "path": "{0}/tmp" }},
           "cargo": {{ "kind": "cargo", "roots": ["{0}/src"] }},
           "installs": {{ "kind": "installs", "roots": ["{0}/src"] }}"#,
        home.join("var").display()
    ));
    let found = offers(&config, &[], &|_| false);
    for path in [&leftover, &target, &modules] {
        let offered = find(&found, path);
        assert!(
            path.starts_with(offered.action.root()),
            "{} is offered under {}",
            path.display(),
            offered.action.root().display()
        );
    }
}

#[test]
fn a_target_holding_only_what_reclaiming_keeps_is_not_offered() {
    // Reclaiming empties a target in place and keeps its benchmarks, so
    // offering it again would free nothing, pass after pass.
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let roots = dir(&home.join("src"));
    let emptied = checkout(&roots, "emptied", ".");
    file(&emptied.join("Cargo.toml"), "");
    let emptied_target = dir(&emptied.join("target"));
    let benched = checkout(&roots, "benched", ".");
    file(&benched.join("Cargo.toml"), "");
    let benched_target = benched.join("target");
    file(
        &benched_target.join("criterion/report/index.html"),
        "a report",
    );
    let built = checkout(&roots, "built", ".");
    file(&built.join("Cargo.toml"), "");
    let built_target = built.join("target");
    file(&built_target.join("debug/app"), "a binary");
    let config = config(&format!(
        r#""cargo": {{ "kind": "cargo", "roots": ["{}"] }}"#,
        roots.display()
    ));
    let found = offers(&config, &[], &|_| true);
    find(&found, &built_target);
    for target in [&emptied_target, &benched_target] {
        assert!(
            found.iter().all(|offer| offer.candidate.path != *target),
            "{} is offered",
            target.display()
        );
    }
}

#[test]
fn a_tree_a_build_runs_in_is_never_offered() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let roots = dir(&home.join("src"));
    let (app, target, _) = src(&roots);
    let variant = dir(&target.join("debug/incremental/app-1a2b3c"));
    dir(&variant.join("s-abcdefghi-0123456-0123456789abcdef"));
    let config = config(&format!(
        r#""cargo": {{ "kind": "cargo", "roots": ["{}"] }}"#,
        roots.display()
    ));
    assert!(
        offers(&config, &[], &|_| false)
            .iter()
            .any(|offer| offer.candidate.path == variant),
        "incremental state is offered while nothing builds there"
    );
    let cargo = Process {
        pid: 4242,
        name: "cargo".into(),
        exe: Some(PathBuf::from("/nix/store/abc-cargo/bin/cargo")),
        cmdline: vec![OsString::from("cargo"), OsString::from("build")],
        cwd: Some(app.clone()),
        readable: true,
        ..Default::default()
    };
    let found = offers(&config, &[cargo], &|_| false);
    assert!(
        !found
            .iter()
            .any(|offer| offer.candidate.path.starts_with(&target)),
        "nothing under a target cargo is building in is offered"
    );
}

#[test]
fn a_pinned_snapshots_older_builds_go_once_an_hour_unused() {
    // On 17 November a gate's snapshot, building one commit after
    // another, kept 89 builds of one test crate among 55 GiB of them: an
    // older build there an hour unused is an earlier gate's, while the
    // next gate runs too.
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let roots = dir(&home.join("src"));
    let (app, target, _) = src(&roots);
    let deps = target.join("debug/deps");
    for (name, ago) in [
        ("libapp-1111111111111111.rlib", DAY),
        ("libapp-0123456789abcdef.rlib", 0.0),
    ] {
        file(&deps.join(name), "built");
        written_ago(&deps.join(name), ago);
    }
    let config = config(&format!(
        r#""cargo": {{ "kind": "cargo", "roots": ["{}"] }}"#,
        roots.display()
    ));
    let replaced = deps.join("app-1111111111111111");
    let shell = Process {
        pid: 4242,
        name: "zsh".into(),
        cwd: Some(app.clone()),
        readable: true,
        ..Default::default()
    };
    let snapshot = |dir: &Path| dir == app;
    let spent = |ago: f64, pinned: &dyn Fn(&Path) -> bool| {
        let mut uses = Uses::default();
        uses.watching(&deps, now() - ago);
        let (found, _, _) = surveyed_pinned(
            &config,
            std::slice::from_ref(&shell),
            &|_| false,
            pinned,
            &BTreeMap::new(),
            &uses,
            (now(), Sizes::default(), &Awake::default()),
        );
        let older = find(&found, &replaced);
        (older.candidate.spent, plan::windows(&older.candidate).0)
    };
    assert_eq!(
        spent(7200.0, &snapshot),
        (true, Group::Waste),
        "an older build in a snapshot two hours unused is waste"
    );
    assert_eq!(
        spent(1800.0, &snapshot),
        (false, Group::Replaced),
        "half an hour unused, it's kept for the gates that may share it"
    );
    assert_eq!(
        spent(7200.0, &|_| false),
        (false, Group::Replaced),
        "in a checkout someone edits, the usual wait holds"
    );
}

#[test]
fn a_pinned_snapshots_incremental_goes_at_once_while_nobody_sits_there() {
    // A gate builds with incremental off, so a snapshot's incremental
    // sessions are a stray build's, and rustc's locks keep the one a
    // build running there uses.
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let roots = dir(&home.join("src"));
    let (app, target, _) = src(&roots);
    let variant = dir(&target.join("debug/incremental/app-1a2b3c"));
    dir(&variant.join("s-abcdefghi-0123456-0123456789abcdef"));
    let config = config(&format!(
        r#""cargo": {{ "kind": "cargo", "roots": ["{}"] }}"#,
        roots.display()
    ));
    let shell = Process {
        pid: 4242,
        name: "zsh".into(),
        cwd: Some(app.clone()),
        readable: true,
        ..Default::default()
    };
    let snapshot = |dir: &Path| dir == app;
    let spent = |processes: &[Process], pinned: &dyn Fn(&Path) -> bool| {
        let (found, _, _) = surveyed_pinned(
            &config,
            processes,
            &|_| false,
            pinned,
            &BTreeMap::new(),
            &Uses::default(),
            (now(), Sizes::default(), &Awake::default()),
        );
        let offered = find(&found, &variant);
        (offered.candidate.spent, plan::windows(&offered.candidate).0)
    };
    assert_eq!(
        spent(&[], &snapshot),
        (true, Group::Waste),
        "nobody sits in the snapshot, so its incremental is waste"
    );
    assert_eq!(
        spent(std::slice::from_ref(&shell), &snapshot),
        (false, Group::Variants),
        "with a session sitting there, the usual wait holds"
    );
    assert_eq!(
        spent(&[], &|_| false),
        (false, Group::Variants),
        "in a checkout someone edits, the usual wait holds"
    );
}

#[test]
fn stale_incremental_variants_are_offered_as_variants() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let roots = dir(&home.join("src"));
    let (_, target, _) = src(&roots);
    let variant = dir(&target.join("debug/incremental/app-1a2b3c"));
    dir(&variant.join("s-abcdefghi-0123456-0123456789abcdef"));
    let config = config(&format!(
        r#""cargo": {{ "kind": "cargo", "roots": ["{}"] }}"#,
        roots.display()
    ));
    let found = offers(&config, &[], &|_| false);
    let offered = find(&found, &variant);
    assert_eq!(offered.candidate.class, Class::Variant);
    assert_eq!(
        offered.action,
        Action::Variant {
            root: roots.clone()
        }
    );
}

#[test]
fn cargo_builds_past_their_budget_go_least_recently_used_first() {
    // On 17 November cargo's own filesystem held 63 GiB in 3,298 builds,
    // 59 GiB of it written within a day, so no idle window took it while
    // the disk it lies on was 83% used.
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let builds = dir(&home.join("builds"));
    let build = |name: &str, hours: f64| {
        let build = dir(&builds.join(name));
        file(&build.join("CACHEDIR.TAG"), CARGO_TAG);
        let rlib = build.join("debug/deps/libapp-0123456789abcdef.rlib");
        file(&rlib, &"x".repeat(8192));
        let tag = build.join("CACHEDIR.TAG");
        for path in [
            &rlib,
            &tag,
            &build.join("debug/deps"),
            &build.join("debug"),
            &build,
        ] {
            written_ago(path, hours * 3600.0);
        }
        build
    };
    let newest = build("aa/0001", 1.0);
    let older = build("bb/0002", 2.0);
    let oldest = build("cc/0003", 3.0);
    let source = |budget: &str| {
        config(&format!(
            r#""cargo-builds": {{ "kind": "cargo", "roots": ["{}"], "depth": 2{budget} }}"#,
            builds.display()
        ))
    };
    let unbudgeted = offers(&source(""), &[], &|_| false);
    assert!(
        unbudgeted.iter().all(|offer| !offer.candidate.spent),
        "with no budget, builds an hour or more unused aren't spent"
    );
    let size = |path: &Path| find(&unbudgeted, path).candidate.size;
    let budget = size(&newest) + size(&older);
    let found = offers(&source(&format!(r#", "budget": {budget}"#)), &[], &|_| {
        false
    });
    let spent = |path: &Path| find(&found, path).candidate.spent;
    assert!(
        !spent(&newest) && !spent(&older),
        "what fits, newest first, stays"
    );
    assert!(
        spent(&oldest),
        "the least recently used past the budget goes"
    );
    assert_eq!(
        plan::windows(&find(&found, &oldest).candidate).0,
        Group::Waste,
        "in any band"
    );
}

/// Dates everything under `path` to `hours` ago, so a checkout and its
/// build read as last used then.
fn aged(path: &Path, hours: f64) {
    if path.is_dir() {
        for entry in std::fs::read_dir(path).unwrap() {
            aged(&entry.unwrap().path(), hours);
        }
    }
    written_ago(path, hours * 3600.0);
}

/// Checkouts under one root, each with a build last used `hours` ago,
/// made in the order given, and a cargo source over them with `budget`
/// bytes: as much as one build holds, times `builds`.
fn budgeted(checkouts: &[(&str, f64)], builds: u64) -> (tempfile::TempDir, Vec<PathBuf>, Config) {
    let kept = tempfile::tempdir().unwrap();
    let home = real(kept.path());
    let roots = dir(&home.join("src"));
    let made: Vec<PathBuf> = checkouts
        .iter()
        .map(|(name, hours)| {
            let app = checkout(&roots, name, ".");
            let target = dir(&app.join("target"));
            file(&target.join("CACHEDIR.TAG"), CARGO_TAG);
            file(
                &target.join("debug/deps/libapp-0123456789abcdef.rlib"),
                &"x".repeat(8192),
            );
            aged(&app, *hours);
            app
        })
        .collect();
    let source = |budget: &str| {
        config(&format!(
            r#""cargo": {{ "kind": "cargo", "roots": ["{}"]{budget} }}"#,
            roots.display()
        ))
    };
    let unbudgeted = offers(&source(""), &[], &|_| false);
    let one = find(&unbudgeted, &made[0].join("target")).candidate.size;
    (
        kept,
        made,
        source(&format!(r#", "budget": {}"#, one * builds)),
    )
}

#[test]
fn a_build_already_spent_takes_no_room_in_the_budget() {
    // A finished tree's build goes anyway; counting it against the
    // budget would spend a build that fits in its place.
    let (_home, made, config) =
        budgeted(&[("z-older", 3.0), ("m-done", 1.5), ("a-oldest", 4.0)], 1);
    let done = made[1].clone();
    let found = offers(&config, &[], &|dir| dir == done);
    let spent = |app: &Path| find(&found, &app.join("target")).candidate.spent;
    assert!(spent(&made[1]), "the finished tree's build is spent");
    assert!(
        !spent(&made[0]),
        "the next most recently used fits the budget the spent one doesn't use"
    );
    assert!(spent(&made[2]), "the one past the budget goes");
}

#[test]
fn a_budget_of_nothing_spends_nothing() {
    // A budget that comes to nothing, as a share of a disk that can't be
    // measured would, is no budget, rather than one that takes every build.
    let (_home, made, config) = budgeted(&[("a-old", 3.0), ("b-older", 4.0)], 0);
    let found = offers(&config, &[], &|_| false);
    assert!(
        made.iter()
            .all(|app| !find(&found, &app.join("target")).candidate.spent),
        "a budget of nothing spends no build"
    );
}

/// Hours a minute either side of the budget's floor, so a case pins the
/// floor itself rather than anywhere in a wide gap.
const JUST_INSIDE: f64 = (BUDGET_RECENT - 60.0) / 3600.0;
const JUST_PAST: f64 = (BUDGET_RECENT + 60.0) / 3600.0;

#[test]
fn a_build_used_within_a_quarter_hour_or_sat_in_counts_but_stays() {
    // A run of checks reuses one build minutes apart, and a shell sits in
    // a build it will use again: both hold room, but neither is taken for
    // the budget, so the next least recently used goes in their place.
    let (_home, made, config) = budgeted(
        &[
            ("c-fits", 0.05),
            ("a-sat-in", 5.0),
            ("f-recent", JUST_INSIDE),
            ("e-old", 4.0),
        ],
        1,
    );
    let shell = Process {
        pid: 4242,
        name: "zsh".into(),
        cwd: Some(made[1].clone()),
        readable: true,
        ..Default::default()
    };
    // Heard throughout, so the quarter hour is the floor in force.
    let uses = heard(&made, 3.0);
    let found = offers_using(&config, &[shell], &|_| false, &BTreeMap::new(), &uses);
    let offered = |app: &Path| find(&found, &app.join("target")).candidate.clone();
    assert!(
        !offered(&made[2]).spent,
        "a build used a minute inside the floor stays, even past the budget"
    );
    assert!(
        offered(&made[1]).occupied && !offered(&made[1]).spent,
        "a build a shell sits in stays"
    );
    assert!(
        offered(&made[3]).spent,
        "the least recently used nobody sits in goes past the budget"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn a_build_left_past_a_quarter_hour_goes_past_the_budget() {
    // On 18 November the builds disk held 56 GiB against its 20, 55 of it
    // used within the hour, so an hour's floor held almost nothing back.
    let (_home, made, config) = budgeted(&[("c-fits", 0.05), ("b-left", JUST_PAST)], 1);
    let uses = heard(&made, 3.0);
    let found = offers_using(&config, &[], &|_| false, &BTreeMap::new(), &uses);
    assert!(
        find(&found, &made[1].join("target")).candidate.spent,
        "a build the watch heard nothing of for a minute past the floor stays past the budget"
    );
}

#[cfg(not(target_os = "linux"))]
#[test]
fn a_build_left_past_a_quarter_hour_keeps_its_hour_where_nothing_is_heard() {
    // A Mac has no watch and can't tell one folder from another made
    // where it stood, so the floor stays the hour it was.
    let (_home, made, config) = budgeted(&[("c-fits", 0.05), ("b-left", JUST_PAST)], 1);
    let uses = heard(&made, 3.0);
    let found = offers_using(&config, &[], &|_| false, &BTreeMap::new(), &uses);
    assert!(
        !find(&found, &made[1].join("target")).candidate.spent,
        "a build left a minute past the quarter hour went past the budget where nothing is heard"
    );
}

#[test]
fn a_build_the_watch_may_have_missed_keeps_its_hour() {
    // A fresh check's only sign is an open, so where one in the last
    // quarter hour could have gone unheard, a build stays the hour it
    // stayed before opens were heard.
    let (_home, made, config) = budgeted(&[("c-fits", 0.05), ("b-left", JUST_PAST)], 1);
    let watched_lately = heard(&made, 0.1);
    let mut overflowed = heard(&made, 3.0);
    overflowed.lost(now() - 600.0);
    let mut deafened = heard(&made, 3.0);
    deafened.deaf();
    // Only an earlier guard's word for it, since its marks died with it.
    let mut earlier = Uses::default();
    for dir in watched_dirs(&made[1].join("target")) {
        earlier.watching(&dir, now() - 3.0 * 3600.0);
    }
    for (why, uses) in [
        ("nothing watches it", Uses::default()),
        ("it was first marked minutes ago", watched_lately),
        ("the watch lost events minutes ago", overflowed),
        ("the guard stopped hearing", deafened),
        ("only an earlier guard watched it", earlier),
    ] {
        let found = offers_using(&config, &[], &|_| false, &BTreeMap::new(), &uses);
        assert!(
            !find(&found, &made[1].join("target")).candidate.spent,
            "a build left a minute past the floor went past the budget though {why}"
        );
    }
    // Last, since it changes the build itself: cleaned and built again,
    // as the guard's own take of a target leaves it, so its old mark
    // hears nothing of the folders there now.
    let remade = heard(&made, 3.0);
    let deps = made[1].join("target/debug/deps");
    std::fs::rename(&deps, made[1].join("target/debug/old-deps")).unwrap();
    std::fs::create_dir(&deps).unwrap();
    // Written no later than the rest of the build, so only the mark
    // tells it apart from one heard throughout.
    aged(&made[1].join("target"), JUST_PAST);
    let found = offers_using(&config, &[], &|_| false, &BTreeMap::new(), &remade);
    assert!(
        !find(&found, &made[1].join("target")).candidate.spent,
        "a build left a minute past the floor went past the budget though its deps was made anew since its mark"
    );
}

#[test]
fn a_suspend_ages_neither_a_builds_last_open_nor_its_marks() {
    // Every other idle is counted awake, and a lid closed for an hour
    // between two checks would otherwise make the build they share look
    // an hour old, or its marks an hour trusted.
    let base = now();
    let minutes = |m: f64| m * 60.0;
    let asleep = |from: f64, to: f64| {
        let mut awake = Awake::default();
        let tick = |wall: f64, suspended: f64| Tick {
            wall: base - wall,
            monotonic: minutes(240.0) - wall - suspended,
            suspended,
            boot: "b".into(),
        };
        awake.observe(&tick(minutes(240.0), 0.0));
        awake.observe(&tick(from, 0.0));
        awake.observe(&tick(to, from - to));
        awake
    };
    // Opened 65 minutes ago, 60 of them asleep, heard for hours.
    let (_home, made, config) = budgeted(&[("c-fits", 0.05), ("b-opened", 2.0)], 1);
    let mut uses = heard(&made, 3.0);
    uses.opened(
        &made[1].join("target/debug/.cargo-build-lock"),
        base - minutes(65.0),
    );
    let found = offers_awake(&config, &uses, &asleep(minutes(61.0), minutes(1.0)));
    assert!(
        !find(&found, &made[1].join("target")).candidate.spent,
        "a build opened five awake minutes ago went past the budget across a suspend"
    );
    // Written 40 minutes ago, marked 20 ago, 10 of those asleep: heard
    // only 10 awake minutes, so the hour stands.
    let (_home, made, config) = budgeted(&[("c-fits", 0.05), ("b-written", 40.0 / 60.0)], 1);
    let uses = heard(&made, 20.0 / 60.0);
    let found = offers_awake(&config, &uses, &asleep(minutes(20.0), minutes(10.0)));
    assert!(
        !find(&found, &made[1].join("target")).candidate.spent,
        "a build marked ten awake minutes ago kept only a quarter hour's floor"
    );
}

/// What the watch says of `apps`' builds when this guard has marked their
/// profiles and `deps` for `hours` and nothing was opened in them. On a
/// Mac, which gives no folder a handle, the marks name none, as nothing
/// there can.
fn heard(apps: &[PathBuf], hours: f64) -> Uses {
    let mut uses = Uses::default();
    for app in apps {
        for dir in watched_dirs(&app.join("target")) {
            uses.watching(&dir, now() - hours * 3600.0);
            let handle = disk_guard::opens::handle(&dir);
            #[cfg(target_os = "linux")]
            let handle = handle.unwrap();
            #[cfg(not(target_os = "linux"))]
            let handle = handle.unwrap_or_default();
            uses.marked(&dir, now() - hours * 3600.0, handle);
            assert!(
                uses.heard.contains_key(&dir),
                "{} wasn't marked",
                dir.display()
            );
        }
    }
    uses
}

#[test]
fn a_build_a_cargo_found_fresh_is_used_though_nothing_was_written() {
    // A cargo that finds every unit fresh writes nothing in the build, but
    // opens its profile's lock; a run of such checks minutes apart reuses
    // a build last written long before, which the budget would otherwise
    // take between two of them.
    let (_home, made, config) = budgeted(&[("c-fits", 0.05), ("b-checked", 2.0)], 1);
    let profile = made[1].join("target/debug");
    let mut uses = heard(&made, 3.0);
    uses.opened(&profile.join(".cargo-build-lock"), now() - 120.0);
    let found = offers_using(&config, &[], &|_| false, &BTreeMap::new(), &uses);
    let checked = find(&found, &made[1].join("target")).candidate.clone();
    assert!(
        !checked.spent,
        "a build a cargo opened two minutes ago went past the budget"
    );
    // Only the budget reads opens: a reader such as a backup would
    // otherwise hold back every other window.
    assert!(
        checked.idle > 3600.0,
        "an open moved the build's idle time, {}, which only the budget reads",
        checked.idle
    );
}
