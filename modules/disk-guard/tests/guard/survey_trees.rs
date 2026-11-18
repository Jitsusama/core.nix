//! What cargo builds and package managers install in checkouts and trees,
//! as the survey offers and sizes it.

use crate::discover::{CARGO_TAG, checkout, dir, file};
use crate::survey::{
    DAY, config, find, now, offers, offers_using, real, src, surveyed, surveyed_pinned, written_ago,
};
use disk_guard::activity::Process;
use disk_guard::plan::{self, Class, Group};
use disk_guard::replaced::Uses;
use disk_guard::survey::{Action, Sizes};
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
            (now(), Sizes::default()),
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
            (now(), Sizes::default()),
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
