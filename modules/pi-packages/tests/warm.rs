//! The warm before a swap: each live pi run once on the version to come,
//! before `current` moves, a failure said and the swap going ahead, a
//! hold put meanwhile standing, and a removed version's cache entries
//! going with it.

mod common;

use std::fs::{self, File};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, SystemTime};

use common::Fixture;
use pi_packages::hold::{self, Hold};
use pi_packages::jiti;
use pi_packages::run::Done;
use pi_packages::version::Place;
use serde_json::{Value, json};

/// A process that says, through a file like a pi's, that it loaded `root`
/// and which pi it is. It ends with the test.
struct Saying(Child);

impl Drop for Saying {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn saying(place: &Place, root: &Path, pi: &Value) -> Saying {
    let sleeping = Saying(
        Command::new("sleep")
            .arg("600")
            .spawn()
            .expect("a process starts"),
    );
    fs::create_dir_all(place.loaded()).expect("the loaded folder");
    let file = place.loaded().join(sleeping.0.id().to_string());
    fs::write(&file, json!({ "root": root, "pi": pi }).to_string()).expect("what it loaded");
    sleeping
}

/// A pi that writes down what it was given into `seen`, and when it ran,
/// where `current` led, then runs `then` and ends with `code`.
struct FakePi {
    _scratch: tempfile::TempDir,
    seen: PathBuf,
    said: Value,
}

impl FakePi {
    fn new(place: &Place, code: u8, then: &str) -> Self {
        let scratch = tempfile::tempdir().expect("a scratch folder");
        let seen = scratch.path().join("seen");
        let bundle = scratch.path().join("bundle");
        let tmp = scratch.path().join("tmp");
        for folder in [&seen, &bundle, &tmp] {
            fs::create_dir_all(folder).expect("a folder");
        }
        fs::write(bundle.join("cli.js"), "").expect("pi's entry");
        let shell = which("sh");
        let node = scratch.path().join("node");
        let script = format!(
            "#!{shell}\n\
             echo \"$@\" > {seen}/args\n\
             {cp} \"$PI_CODING_AGENT_DIR/settings.json\" {seen}/settings.json\n\
             echo \"$TMPDIR\" > {seen}/tmp\n\
             {readlink} {current} > {seen}/current\n\
             {then}\n\
             exit {code}\n",
            shell = shell.display(),
            // The warm's PATH is the installer's, with no coreutils.
            cp = which("cp").display(),
            readlink = which("readlink").display(),
            seen = seen.display(),
            current = place.current().display(),
        );
        fs::write(&node, script).expect("the fake");
        fs::set_permissions(&node, fs::Permissions::from_mode(0o755)).expect("runnable");
        let said = json!({ "node": node, "bundle": bundle, "tmp": tmp });
        Self {
            _scratch: scratch,
            seen,
            said,
        }
    }

    fn saw(&self, what: &str) -> Option<String> {
        fs::read_to_string(self.seen.join(what))
            .ok()
            .map(|text| text.trim().to_owned())
    }

    fn tmp(&self) -> PathBuf {
        PathBuf::from(self.said["tmp"].as_str().expect("a folder"))
    }
}

fn which(name: &str) -> PathBuf {
    std::env::var_os("PATH")
        .and_then(|path| {
            std::env::split_paths(&path)
                .map(|dir| dir.join(name))
                .find(|candidate| candidate.is_file())
        })
        .unwrap_or_else(|| panic!("{name} is not on the PATH"))
}

#[test]
fn a_live_pi_is_warmed_on_the_version_to_come_before_the_swap() {
    let fixture = Fixture::new();
    let place = fixture.place();
    fixture.update();
    let before = fs::read_link(place.current()).expect("current");
    let old = fs::canonicalize(place.current()).expect("current resolves");
    let pi = FakePi::new(&place, 0, "");
    let _live = saying(&place, &old, &pi.said);

    let outcome = fixture.advance("second");

    let Done::Moved { to, .. } = &outcome.done else {
        panic!("current did not move: {outcome:?}");
    };
    assert_eq!(outcome.unwarmed, Vec::<String>::new(), "{outcome:?}");
    let bundle = pi.said["bundle"].as_str().expect("a bundle");
    assert_eq!(
        pi.saw("args").as_deref(),
        Some(format!("{bundle}/cli.js --mode rpc --no-session").as_str()),
        "pi was not run in RPC mode from its bundle"
    );
    assert_eq!(
        pi.saw("current").map(PathBuf::from),
        Some(before),
        "the warm ran after the swap"
    );
    let settings: Value =
        serde_json::from_str(&pi.saw("settings.json").expect("its settings")).expect("JSON");
    let new = fs::canonicalize(place.versions().join(to)).expect("the new version");
    // The neighbour has no version yet, so the warm can't name it.
    assert_eq!(fixture.neighbour().current_commit(), None);
    assert_eq!(
        settings["packages"],
        json!([new]),
        "the warm didn't load the version to come"
    );
    assert_eq!(
        pi.saw("tmp").map(PathBuf::from),
        Some(pi.tmp()),
        "the warm wrote somewhere the pi doesn't read"
    );
    assert!(
        !place.dir.join(".warming").exists(),
        "the scratch state was kept"
    );
}

#[test]
fn a_failed_warm_is_said_and_the_swap_goes_ahead() {
    let fixture = Fixture::new();
    let place = fixture.place();
    fixture.update();
    let old = fs::canonicalize(place.current()).expect("current resolves");
    let pi = FakePi::new(&place, 3, "");
    let _live = saying(&place, &old, &pi.said);

    let outcome = fixture.advance("second");

    assert_eq!(outcome.unwarmed.len(), 1, "{outcome:?}");
    assert!(
        outcome.unwarmed[0].contains("exit status: 3"),
        "{:?}",
        outcome.unwarmed
    );
}

#[test]
fn a_pi_whose_bundle_is_gone_is_not_run() {
    let fixture = Fixture::new();
    let place = fixture.place();
    fixture.update();
    let old = fs::canonicalize(place.current()).expect("current resolves");
    let pi = FakePi::new(&place, 0, "");
    let mut said = pi.said.clone();
    said["bundle"] = json!("/nowhere/at/all");
    let _live = saying(&place, &old, &said);

    let outcome = fixture.advance("second");

    assert_eq!(pi.saw("args"), None, "a pi that isn't there was run");
    assert_eq!(outcome.unwarmed, Vec::<String>::new(), "{outcome:?}");
}

#[test]
fn a_hold_put_while_the_warm_runs_stands() {
    let fixture = Fixture::new();
    let place = fixture.place();
    fixture.update();
    let before = place.current_commit();
    let old = fs::canonicalize(place.current()).expect("current resolves");
    let hold = Hold {
        why: "a drill".to_owned(),
        until: None,
    };
    hold::put(&fixture.config, &hold).expect("a hold");
    let aside = fixture.config.root.join("hold-aside");
    fs::rename(hold::file(&fixture.config), &aside).expect("put aside");
    let pi = FakePi::new(
        &place,
        0,
        &format!(
            "{} {} {}",
            which("mv").display(),
            aside.display(),
            hold::file(&fixture.config).display()
        ),
    );
    let _live = saying(&place, &old, &pi.said);
    fixture.change("second");

    let outcome = fixture.update();

    assert!(matches!(outcome.done, Done::OnHold { .. }), "{outcome:?}");
    assert_eq!(place.current_commit(), before, "current moved under a hold");
}

#[test]
fn a_removed_version_takes_its_cache_entries_with_it() {
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
    let pi = FakePi::new(&place, 0, "");
    let _live = saying(&place, &second, &pi.said);
    let cache = jiti::cache_of(&pi.tmp());
    fs::create_dir_all(&cache).expect("a cache");
    let entry_of = |version: &Path| jiti::entry_names(&version.join("extension.ts"))[0].clone();
    let (gone, kept) = (entry_of(&first), entry_of(&second));
    for name in [&gone, &kept] {
        fs::write(cache.join(name), "").expect("an entry");
    }

    let third = fixture.advance("third");

    assert_eq!(third.removed, 1, "{third:?}");
    assert_eq!(fixture.versions().len(), 2, "only the first version went");
    assert!(
        !cache.join(&gone).exists(),
        "a removed version's entry was kept"
    );
    assert!(cache.join(&kept).exists(), "a kept version's entry went");
}
