//! An update, run against a package in a local repository: what it
//! builds and checks before it swaps a version in, how the versions share
//! what they install, and how often it swaps.

mod common;

use std::fs::{self, File};
use std::path::Path;
use std::process::Command;
use std::time::{Duration, SystemTime};

use common::Fixture;
use pi_packages::config::{Config, Package};
use pi_packages::hold::{self, Hold};
use pi_packages::run::{self, Done, Pace, SPACING};

impl Fixture {
    /// The package alone, with no neighbour an update would try to fetch.
    fn alone(&self) -> Config {
        let mut config = self.config.clone();
        config.packages.truncate(1);
        config
    }

    /// The package twice: once as itself and once through a link to its
    /// repository, so it has two folders and two `current`s, as two
    /// packages would.
    fn twice(&self) -> Config {
        let twin = self.upstream.with_file_name("upstream-twin");
        std::os::unix::fs::symlink(&self.upstream, &twin).expect("a second name");
        let url = format!("file://{}", twin.display());
        let mut config = self.alone();
        config
            .packages
            .push(Package::new(&url, "main").expect("a package"));
        config
    }

    /// Every package's update, as the timer or a hand run makes it at `now`.
    fn update_all(config: &Config, now: SystemTime, pace: Pace) -> Vec<Done> {
        run::update(config, now, pace)
            .into_iter()
            .map(|(_, outcome)| outcome.expect("an update").done)
            .collect()
    }

    /// The commit the branch is at.
    fn head(&self) -> String {
        let out = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.upstream)
            .output()
            .expect("git runs");
        String::from_utf8(out.stdout)
            .expect("a commit")
            .trim()
            .to_owned()
    }
}

#[test]
fn an_unchanged_dependency_has_one_path_across_versions() {
    // Node keeps an ES module for the life of a pi, by its real path, so
    // a pi that takes a new version imports again each dependency found
    // at a new path, and keeps the copy it had too.
    let fixture = Fixture::new();
    let place = fixture.place();
    fixture.update();
    let first = fs::canonicalize(place.current().join("node_modules/shared"))
        .expect("shared in the first version");

    fixture.advance("second");

    let second = fs::canonicalize(place.current().join("node_modules/shared"))
        .expect("shared in the second version");
    assert_eq!(second, first, "the second version has a copy of its own");
}

#[test]
fn a_build_that_fails_leaves_current_as_it_was() {
    let fixture = Fixture::new();
    let place = fixture.place();
    fixture.update();
    let installed = place.current_commit();

    fs::write(fixture.upstream.join("dep/fail"), "").expect("a failing build");
    fixture.commit("broken");
    let broken = fixture.update();

    assert!(matches!(broken.done, Done::Failed { .. }), "{broken:?}");
    assert_eq!(place.current_commit(), installed);
    let left: Vec<_> = fs::read_dir(place.versions())
        .expect("versions")
        .flatten()
        .map(|entry| entry.file_name())
        .collect();
    assert_eq!(left.len(), 1, "a half-built version was kept: {left:?}");
}

#[test]
fn an_unbuilt_native_module_is_not_swapped_in() {
    let fixture = Fixture::new();
    let place = fixture.place();
    // The build runs but writes no binding, as a skipped build leaves it.
    fs::write(fixture.upstream.join("dep/make.js"), "\n").expect("a build that builds nothing");
    fixture.commit("unbuilt");

    let outcome = fixture.update();

    let Done::Failed { why, .. } = &outcome.done else {
        panic!("it was swapped in: {outcome:?}");
    };
    assert!(why.contains("dep's native build did not run"), "{why}");
    assert_eq!(place.current_commit(), None);
}

#[test]
fn a_dependency_installed_here_is_the_one_installed_copy() {
    // The neighbour is installed under the name the package depends on,
    // as the harness is beside a package that pins it.
    let fixture = Fixture::new();
    let neighbour = fixture.neighbour();
    let its_version = neighbour.versions().join("abc");
    fs::create_dir_all(&its_version).expect("the neighbour's version");
    fs::write(
        its_version.join("package.json"),
        r#"{"name":"dep","version":"2.0.0","main":"index.js"}"#,
    )
    .expect("its manifest");
    fs::write(its_version.join("index.js"), "module.exports = 2;\n").expect("its code");
    std::os::unix::fs::symlink(Path::new("versions/abc"), neighbour.current())
        .expect("its current");

    let outcome = fixture.update();

    assert!(matches!(outcome.done, Done::Moved { .. }), "{outcome:?}");
    let version = fixture.place().current();
    let link = version.join("node_modules/dep");
    assert_eq!(
        fs::read_link(&link).ok(),
        Some(neighbour.current()),
        "dep is pnpm's copy, not the neighbour's current"
    );
    assert_eq!(
        fs::read_to_string(link.join("index.js")).ok().as_deref(),
        Some("module.exports = 2;\n"),
        "dep does not read as the neighbour"
    );
}

#[test]
fn a_dependency_no_neighbour_names_stays_pnpms() {
    let fixture = Fixture::new();

    let outcome = fixture.update();

    assert!(matches!(outcome.done, Done::Moved { .. }), "{outcome:?}");
    let dep = fixture.place().current().join("node_modules/dep");
    assert_eq!(
        fs::read_to_string(dep.join("index.js")).ok().as_deref(),
        Some("module.exports = 1;\n"),
        "dep is not the copy pnpm installed"
    );
}

#[test]
fn a_package_naming_another_pnpm_installs_with_this_one() {
    // pnpm downloads the pnpm a manifest names and runs that instead, and
    // a prebuilt pnpm is one NixOS won't run without nix-ld, so a
    // machine without it never installed a package that named pnpm
    // 9.15.0. With the registry out of reach, a download fails the update.
    let mut fixture = Fixture::new();
    for name in ["npm_config_registry", "pnpm_config_registry"] {
        fixture
            .config
            .env
            .push((name.to_owned(), "http://127.0.0.1:9/".to_owned()));
    }
    fs::write(
        fixture.upstream.join("package.json"),
        r#"{"name":"fixture","version":"1.0.0","packageManager":"pnpm@9.15.0","dependencies":{"dep":"file:./dep"}}"#,
    )
    .expect("the manifest");
    fixture.commit("names pnpm 9");

    let outcome = fixture.update();

    assert!(matches!(outcome.done, Done::Moved { .. }), "{outcome:?}");
}

#[test]
fn a_branch_already_current_is_left_alone() {
    let fixture = Fixture::new();
    fixture.update();

    let again = fixture.update();

    assert!(matches!(again.done, Done::Current { .. }), "{again:?}");
}

#[test]
fn an_update_under_way_is_left_to_finish() {
    let fixture = Fixture::new();
    let place = fixture.place();
    fs::create_dir_all(&place.dir).expect("its folder");
    let held = File::create(place.dir.join(".lock")).expect("the lock");
    held.lock().expect("held");

    let outcome = fixture.update();

    assert_eq!(outcome.done, Done::Busy);
    assert_eq!(place.current_commit(), None);
}

/// Half an hour: inside [`SPACING`] after a swap.
const HALF_AN_HOUR: Duration = Duration::from_mins(30);

/// Just past [`SPACING`] after a swap.
const PAST_THE_HOUR: Duration = SPACING.saturating_add(Duration::from_secs(1));

#[test]
fn a_swap_waits_an_hour_after_the_last() {
    let fixture = Fixture::new();
    let config = fixture.alone();
    let start = SystemTime::now();
    let first = fixture.head();
    assert_eq!(
        Fixture::update_all(&config, start, Pace::Spaced),
        [Done::Moved {
            from: None,
            to: first.clone()
        }]
    );
    fixture.change("second");

    let held = Fixture::update_all(&config, start + HALF_AN_HOUR, Pace::Spaced);

    let [Done::Held { commit, until }] = held.as_slice() else {
        panic!("a swap within the hour went ahead: {held:?}");
    };
    assert_eq!(*commit, fixture.head());
    let waited = until.duration_since(start).expect("a time after the swap");
    assert!(
        waited.abs_diff(SPACING) < Duration::from_secs(1),
        "held for {waited:?} after the last swap, not an hour"
    );
    assert_eq!(fixture.place().current_commit(), Some(first.clone()));
    assert_eq!(
        fixture.versions(),
        std::slice::from_ref(&first),
        "a held update built a version"
    );

    fixture.change("third");
    let later = Fixture::update_all(&config, start + PAST_THE_HOUR, Pace::Spaced);

    assert_eq!(
        later,
        [Done::Moved {
            from: Some(first),
            to: fixture.head()
        }]
    );
}

#[test]
fn an_update_told_now_swaps_inside_the_hour() {
    let fixture = Fixture::new();
    let config = fixture.alone();
    let start = SystemTime::now();
    let first = fixture.head();
    Fixture::update_all(&config, start, Pace::Spaced);
    fixture.change("second");

    let moved = Fixture::update_all(&config, start + Duration::from_mins(1), Pace::Now);

    assert_eq!(
        moved,
        [Done::Moved {
            from: Some(first),
            to: fixture.head()
        }]
    );
}

#[test]
fn a_hold_keeps_current_even_told_now() {
    let fixture = Fixture::new();
    let config = fixture.alone();
    let start = SystemTime::now();
    let first = fixture.head();
    Fixture::update_all(&config, start, Pace::Spaced);
    fixture.change("second");
    let held = Hold {
        why: "a switch is under way".to_owned(),
        until: None,
    };
    hold::put(&config, &held).expect("a hold");

    let later = start + PAST_THE_HOUR;
    for pace in [Pace::Spaced, Pace::Now] {
        assert_eq!(
            Fixture::update_all(&config, later, pace),
            [Done::OnHold {
                commit: fixture.head(),
                hold: held.clone()
            }],
            "an update at {pace:?} under a hold"
        );
    }
    assert_eq!(
        fixture.place().current_commit(),
        Some(first.clone()),
        "a held update moved current"
    );
    assert_eq!(
        fixture.versions(),
        std::slice::from_ref(&first),
        "a held update built a version"
    );

    assert!(
        hold::release(&config).expect("a release"),
        "no hold stood to release"
    );
    assert_eq!(
        Fixture::update_all(&config, later, Pace::Spaced),
        [Done::Moved {
            from: Some(first),
            to: fixture.head()
        }],
        "the update after the release"
    );
}

#[test]
fn a_hold_for_a_time_ends_by_itself() {
    let fixture = Fixture::new();
    let config = fixture.alone();
    let start = SystemTime::now();
    let first = fixture.head();
    Fixture::update_all(&config, start, Pace::Spaced);
    fixture.change("second");
    let held = Hold {
        why: "for an hour and a half".to_owned(),
        until: Some(start + Duration::from_mins(90)),
    };
    hold::put(&config, &held).expect("a hold");

    let inside = Fixture::update_all(&config, start + Duration::from_mins(89), Pace::Now);
    let after = Fixture::update_all(&config, start + Duration::from_mins(91), Pace::Now);

    assert!(
        matches!(inside.as_slice(), [Done::OnHold { .. }]),
        "an update inside the hold went ahead: {inside:?}"
    );
    assert_eq!(
        after,
        [Done::Moved {
            from: Some(first),
            to: fixture.head()
        }],
        "the update after the hold ran out"
    );
}

#[test]
fn a_package_with_no_version_is_installed_under_a_hold() {
    let fixture = Fixture::new();
    let config = fixture.alone();
    let hold = Hold {
        why: "a switch is under way".to_owned(),
        until: None,
    };
    hold::put(&config, &hold).expect("a hold");

    let installed = Fixture::update_all(&config, SystemTime::now(), Pace::Spaced);

    assert_eq!(
        installed,
        [Done::Moved {
            from: None,
            to: fixture.head()
        }],
        "a hold kept a package with no version from being installed"
    );
}

#[test]
fn a_hold_file_that_cant_be_read_holds() {
    let fixture = Fixture::new();
    let config = fixture.alone();
    fs::create_dir_all(&config.root).expect("the root");
    fs::write(hold::file(&config), "garbled").expect("a garbled hold");

    let hold = hold::standing(&config, SystemTime::now());

    assert!(
        hold.is_some_and(|hold| hold.until.is_none() && hold.why.contains("garbled")),
        "a garbled hold file let swaps through"
    );
}

#[test]
fn a_package_with_no_version_is_installed_inside_the_hour() {
    let fixture = Fixture::new();
    let config = fixture.alone();
    let now = SystemTime::now();
    let marker = run::last_swap(&config);
    fs::create_dir_all(marker.parent().expect("the root")).expect("the root");
    File::create(&marker)
        .and_then(|file| file.set_modified(now))
        .expect("a swap a moment ago");

    let installed = Fixture::update_all(&config, now, Pace::Spaced);

    assert_eq!(
        installed,
        [Done::Moved {
            from: None,
            to: fixture.head()
        }]
    );
}

#[test]
fn packages_that_moved_within_the_hour_swap_in_the_same_run() {
    let fixture = Fixture::new();
    let config = fixture.twice();
    let start = SystemTime::now();
    let first = fixture.head();
    Fixture::update_all(&config, start, Pace::Spaced);
    fixture.change("second");

    let held = Fixture::update_all(&config, start + HALF_AN_HOUR, Pace::Spaced);
    let later = Fixture::update_all(&config, start + PAST_THE_HOUR, Pace::Spaced);

    assert!(
        held.len() == 2 && held.iter().all(|done| matches!(done, Done::Held { .. })),
        "a package swapped within the hour: {held:?}"
    );
    let moved = Done::Moved {
        from: Some(first),
        to: fixture.head(),
    };
    assert_eq!(later, [moved.clone(), moved]);
}
