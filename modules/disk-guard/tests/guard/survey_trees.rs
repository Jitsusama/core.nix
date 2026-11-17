//! What cargo builds and package managers install in checkouts and trees,
//! as the survey offers and sizes it.

use crate::discover::{checkout, dir, file};
use crate::survey::{
    DAY, config, find, now, offers, offers_sized, offers_using, real, src, surveyed, written_ago,
};
use disk_guard::activity::Process;
use disk_guard::plan::Class;
use disk_guard::replaced::Uses;
use disk_guard::survey::{Action, Sizes};
use std::collections::BTreeMap;
use std::ffi::OsString;
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
    assert!(found.iter().all(|offer| offer.candidate.path != newest));
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
    assert!(!find(&unfinished, &target).candidate.spent);
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
fn a_tree_unbuilt_since_an_earlier_survey_keeps_the_size_it_found() {
    // On 17 November a laptop's guard spent five and a half hours sizing
    // installs one package at a time, and every survey sized each build
    // and install whole again, so no pass came while its disk sat at 91%.
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let roots = dir(&home.join("src"));
    let (_, target, modules) = src(&roots);
    for old in [&modules, &modules.join("left-pad")] {
        written_ago(old, 2.0 * 3600.0);
    }
    let config = config(&format!(
        r#""cargo": {{ "kind": "cargo", "roots": ["{0}"] }},
           "installs": {{ "kind": "installs", "roots": ["{0}"] }}"#,
        roots.display()
    ));
    let (found, sizes) = offers_sized(&config, now(), Sizes::default());
    let built = find(&found, &target).candidate.size;
    let installed = find(&found, &modules).candidate.size;
    assert_eq!(
        sizes.walked, 2,
        "the build and the install weren't each sized once"
    );
    let (found, sizes) = offers_sized(&config, now(), sizes);
    assert_eq!(sizes.walked, 2, "a tree unchanged since was walked again");
    assert_eq!(find(&found, &target).candidate.size, built);
    assert_eq!(find(&found, &modules).candidate.size, installed);
    file(&modules.join("right-pad/index.js"), &"y".repeat(20_000));
    let (found, sizes) = offers_sized(&config, now(), sizes);
    assert_eq!(
        sizes.walked, 3,
        "an install written since wasn't sized again"
    );
    assert!(
        find(&found, &modules).candidate.size > installed,
        "an install written since kept its old size"
    );
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
            .any(|offer| offer.candidate.path == variant)
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
            .any(|offer| offer.candidate.path.starts_with(&target))
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
