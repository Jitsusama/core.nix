//! Which versions an update and a sweep keep: the one a live pi loaded
//! or says it loaded, the one before current, and nothing else.

mod common;
mod running;
mod tree;

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

use common::Fixture;
use pi_packages::run::{self, Done};
use pi_packages::version::Place;
// A node session, or any process standing in a version, as a running pi does.
use running::Running as Session;
use serde_json::json;
use tree::files;

impl Fixture {
    /// Remove what nothing uses, as a pi starting or ending has it done.
    fn sweep(&self) -> usize {
        let swept = run::sweep(&self.config);
        let (_, removed) = swept.into_iter().next().expect("the package");
        removed.expect("a sweep").expect("not held")
    }
}

/// A process that stands in a folder of its own and says, through a file
/// like a pi's, that it loaded `root`. It ends with the test.
fn saying(place: &Place, root: &Path) -> (Session, PathBuf) {
    let elsewhere = place.dir.join("elsewhere");
    fs::create_dir_all(&elsewhere).expect("a folder");
    let sleeping = Session(
        Command::new("sleep")
            .arg("600")
            .current_dir(&elsewhere)
            .spawn()
            .expect("a process starts"),
    );
    fs::create_dir_all(place.loaded()).expect("the loaded folder");
    let file = place.loaded().join(sleeping.0.id().to_string());
    fs::write(&file, json!({ "root": root }).to_string()).expect("what it loaded");
    (sleeping, file)
}

/// A node that loaded the package's dependency through `current` and
/// stands in the version it found, as a running pi does.
fn session(place: &Place) -> Session {
    let version = fs::canonicalize(place.current()).expect("current resolves");
    Session(
        Command::new("node")
            .args([
                "-e",
                "require(process.argv[1]); setInterval(() => {}, 1000)",
            ])
            .arg(place.current().join("node_modules/dep"))
            .current_dir(version)
            .stdout(Stdio::null())
            .spawn()
            .expect("a session starts"),
    )
}

#[test]
fn a_live_session_keeps_its_version_through_an_update() {
    let fixture = Fixture::new();
    let place = fixture.place();
    let first = fixture.update();
    assert!(
        matches!(first.done, Done::Moved { from: None, .. }),
        "{first:?}"
    );
    let old = fs::canonicalize(place.current()).expect("current resolves");
    let mut live = session(&place);
    let before = files(&old);

    fs::write(
        fixture.upstream.join("extension.ts"),
        "export default () => 2;\n",
    )
    .expect("a change");
    fixture.commit("second");
    let second = fixture.update();

    let Done::Moved { from: Some(_), to } = &second.done else {
        panic!("current did not move: {second:?}");
    };
    assert_eq!(place.current_commit().as_deref(), Some(to.as_str()));
    assert_ne!(
        fs::canonicalize(place.current()).expect("current resolves"),
        old
    );
    assert!(
        live.0.try_wait().expect("the session").is_none(),
        "the session died"
    );
    assert_eq!(files(&old), before, "a file the session loaded changed");
    assert_eq!(
        second.removed, 0,
        "the version the session runs was removed"
    );

    // No longer the version before current, it stays for the session alone.
    let third = fixture.advance("third");
    assert_eq!(third.removed, 0, "{third:?}");
    assert!(old.exists(), "the version the session runs was removed");

    drop(live);
    assert_eq!(fixture.sweep(), 1);
    assert!(!old.exists(), "a version nothing uses was kept");
}

#[test]
fn the_version_before_current_is_kept_and_no_other() {
    let fixture = Fixture::new();
    let place = fixture.place();
    fixture.update();
    let first = place.current_commit().expect("a commit");
    fixture.advance("second");
    let second = place.current_commit().expect("a commit");

    assert_eq!(fixture.sweep(), 0, "the version before current went");
    let third = fixture.advance("third");

    assert_eq!(third.removed, 1, "{third:?}");
    let current = place.current_commit().expect("a commit");
    let mut kept = vec![second, current];
    kept.sort();
    assert_eq!(fixture.versions(), kept);
    assert!(
        !place.retired().join(first).exists(),
        "a removed version's marker was kept"
    );
}

#[test]
fn a_version_a_live_pi_says_it_loaded_is_kept_until_it_ends() {
    let fixture = Fixture::new();
    let place = fixture.place();
    fixture.update();
    let old = fs::canonicalize(place.current()).expect("current resolves");
    let (pi, file) = saying(&place, &old);
    fixture.advance("second");
    fixture.advance("third");

    assert!(old.exists(), "a version a live pi runs was removed");
    assert!(file.exists(), "a live pi's file was removed");

    drop(pi);
    assert_eq!(fixture.sweep(), 1);
    assert!(!old.exists(), "a version no pi runs was kept");
    assert!(!file.exists(), "an ended pi's file was kept");
}

#[test]
fn every_version_retired_while_a_pi_runs_is_kept_until_it_ends() {
    // O38FUO [317b309a], 7 November: node keeps where `current` led when a
    // pi first looked, so pis started days before resolved one package's
    // imports through versions a sweep had removed, and failed to load the
    // extension at their next reload.
    let fixture = Fixture::new();
    let place = fixture.place();
    fixture.update();
    let first = fs::canonicalize(place.current()).expect("current resolves");
    let (pi, file) = saying(&place, &first);
    fixture.advance("second");
    // It reloads onto the second and says so, as a pi does.
    let second = fs::canonicalize(place.current()).expect("current resolves");
    fs::write(&file, json!({ "root": second }).to_string()).expect("what it loaded");
    fixture.advance("third");
    fixture.advance("fourth");

    assert!(
        first.exists(),
        "a version a running pi resolves through went"
    );

    drop(pi);
    assert_eq!(
        fixture.sweep(),
        2,
        "the first and second stayed past the pi"
    );
    assert!(!first.exists(), "a version no pi resolves through was kept");
}

#[test]
fn a_pi_said_of_one_package_keeps_what_it_resolved_of_another() {
    let fixture = Fixture::new();
    let place = fixture.place();
    fixture.update();
    let first = fs::canonicalize(place.current()).expect("current resolves");
    let neighbour = fixture.neighbour();
    let (pi, _file) = saying(&neighbour, &neighbour.dir);
    fixture.advance("second");
    fixture.advance("third");

    assert!(
        first.exists(),
        "a version a running pi resolves through went"
    );

    drop(pi);
    assert_eq!(fixture.sweep(), 1);
}

#[test]
fn a_version_retired_before_every_pi_started_goes() {
    let fixture = Fixture::new();
    let place = fixture.place();
    fixture.update();
    let commit = place.current_commit().expect("a commit");
    let first = fs::canonicalize(place.current()).expect("current resolves");
    fixture.advance("second");
    let hour_ago = SystemTime::now() - Duration::from_secs(3_600);
    File::options()
        .write(true)
        .open(place.retired().join(&commit))
        .and_then(|open| open.set_modified(hour_ago))
        .expect("retired an hour ago");
    let second = fs::canonicalize(place.current()).expect("current resolves");
    let (_pi, _file) = saying(&place, &second);

    let third = fixture.advance("third");

    assert_eq!(third.removed, 1, "{third:?}");
    assert!(
        !first.exists(),
        "a version retired before any pi started was kept"
    );
}

#[test]
fn a_file_older_than_its_process_keeps_nothing() {
    let fixture = Fixture::new();
    let place = fixture.place();
    fixture.update();
    let old = fs::canonicalize(place.current()).expect("current resolves");
    // As a pid the system gave again: the file was written before the
    // process now holding the pid started.
    let (_pi, file) = saying(&place, &old);
    let written = SystemTime::now() - Duration::from_secs(3_600);
    File::options()
        .write(true)
        .open(&file)
        .and_then(|open| open.set_modified(written))
        .expect("an old file");
    fixture.advance("second");
    fixture.advance("third");

    assert!(!old.exists(), "a reused pid kept a version");
    assert!(!file.exists(), "a file that counts for nothing was kept");
}

#[test]
fn a_version_in_the_former_folder_goes_once_nothing_uses_it() {
    let fixture = Fixture::new();
    let place = fixture.place();
    fixture.update();
    // As an install from before versions were called that left it.
    let commit = place.current_commit().expect("a commit");
    fs::create_dir_all(place.former_versions()).expect("the former folder");
    let former = place.former_versions().join(&commit);
    fs::rename(place.versions().join(&commit), &former).expect("moved back");
    let staged = place.dir.join(".current.new");
    std::os::unix::fs::symlink(Path::new("releases").join(&commit), &staged).expect("a link");
    fs::rename(&staged, place.current()).expect("current");
    let live = session(&place);

    fixture.commit("second");
    let second = fixture.update();

    assert!(matches!(second.done, Done::Moved { .. }), "{second:?}");
    assert!(
        fs::read_link(place.current())
            .expect("current")
            .starts_with("versions"),
        "current is not in versions/"
    );
    assert!(former.exists(), "a version a session runs was removed");
    fixture.advance("third");
    assert!(former.exists(), "a version a session runs was removed");

    drop(live);
    assert_eq!(fixture.sweep(), 1);
    assert!(
        !place.former_versions().exists(),
        "the former folder was kept"
    );
}
