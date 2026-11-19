use crate::discover::{CARGO_TAG, file};
use disk_guard::activity::Use;
use disk_guard::anchor::Dir;
use disk_guard::pass::Outcome;
use disk_guard::plan::{Candidate, Class};
use disk_guard::reclaim;
use disk_guard::replaced::{Build, Unit, Uses, replaced, stands, watched_dirs};
use rustix::fs::{AtFlags, CWD, Timespec, Timestamps, utimensat};
use std::fs;
use std::path::{Path, PathBuf};

fn written(path: &Path, seconds: i64) {
    let at = Timespec {
        tv_sec: seconds,
        tv_nsec: 0,
    };
    let times = Timestamps {
        last_access: at,
        last_modification: at,
    };
    utimensat(CWD, path, &times, AtFlags::SYMLINK_NOFOLLOW).unwrap();
}

/// An artifact in `deps`, written at `seconds`.
pub(crate) fn artifact(deps: &Path, name: &str, seconds: i64) -> PathBuf {
    let path = deps.join(name);
    file(&path, "built");
    written(&path, seconds);
    path
}

const OLD: &str = "1111111111111111";
const NEW: &str = "2222222222222222";

/// A target with two builds of alpha, one of beta and a test binary, in
/// the host's profile and a cross target's.
fn target(root: &Path) -> PathBuf {
    let target = root.join("target");
    for deps in [
        target.join("debug/deps"),
        target.join("x86_64-unknown-linux-gnu/release/deps"),
    ] {
        artifact(&deps, &format!("libalpha-{OLD}.rlib"), 1_000);
        artifact(&deps, &format!("libalpha-{OLD}.rmeta"), 1_000);
        artifact(&deps, &format!("alpha-{OLD}.d"), 1_000);
        artifact(&deps, &format!("libalpha-{NEW}.rlib"), 5_000);
        artifact(&deps, &format!("libalpha-{NEW}.rmeta"), 5_000);
        artifact(&deps, "libbeta-3333333333333333.rlib", 500);
        // Named as a test binary is, but it doesn't run and has no
        // fingerprint to name its unit, so it's no build.
        artifact(&deps, "alpha-4444444444444444", 100);
        artifact(&deps, "alpha-4444444444444444.d", 100);
        fs::create_dir_all(
            deps.parent()
                .unwrap()
                .join(format!(".fingerprint/alpha-{OLD}")),
        )
        .unwrap();
    }
    target
}

#[test]
fn an_older_build_of_a_crate_is_replaced_and_the_newest_stays() {
    let root = tempfile::tempdir().unwrap();
    let target = target(root.path());
    let mut found = replaced(&target);
    found.sort_by(|a, b| a.deps.cmp(&b.deps));
    let deps = |profile: &str| target.join(profile).join("deps");
    let build = |profile: &str| Build {
        deps: deps(profile),
        name: "alpha".to_owned(),
        digest: OLD.to_owned(),
        files: vec![
            deps(profile).join(format!("alpha-{OLD}.d")),
            deps(profile).join(format!("libalpha-{OLD}.rlib")),
            deps(profile).join(format!("libalpha-{OLD}.rmeta")),
        ],
        fingerprint: Some(
            target
                .join(profile)
                .join(format!(".fingerprint/alpha-{OLD}")),
        ),
        unit: None,
        in_variant: false,
        written: 1_000.0,
        size: 15,
    };
    assert_eq!(
        found,
        vec![build("debug"), build("x86_64-unknown-linux-gnu/release")]
    );
}

#[test]
fn a_target_in_a_layout_it_doesnt_know_replaces_nothing() {
    // Cargo's per-unit build layout, still behind a nightly flag, where
    // each build has its own directory and its own deps. Nothing there
    // says one replaced another, so two builds of a crate in different
    // directories are never judged against each other.
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("target");
    for digest in [OLD, NEW] {
        let deps = target.join(format!("debug/build/alpha/{digest}/deps"));
        artifact(&deps, &format!("libalpha-{digest}.rlib"), 1_000);
        artifact(&deps, &format!("libalpha-{digest}.rmeta"), 1_000);
    }
    assert_eq!(replaced(&target), vec![]);
}

fn build(deps: &Path, written: f64) -> Build {
    Build {
        deps: deps.to_owned(),
        name: "alpha".to_owned(),
        digest: OLD.to_owned(),
        files: vec![
            deps.join(format!("libalpha-{OLD}.rlib")),
            deps.join(format!("libalpha-{OLD}.rmeta")),
        ],
        fingerprint: None,
        unit: None,
        in_variant: false,
        written,
        size: 10,
    }
}

/// The fingerprint cargo writes beside `deps` for `package`'s `unit`
/// built at `digest`, of the variant `PLAIN` names but for `variant`'s
/// fields. What a newer build changes, its dependencies, declared
/// features and source path, differs for every digest, as it does when
/// one build replaces another. It holds the warnings cargo keeps for a
/// unit whose build gave some, as a test's often does.
pub(crate) fn fingerprinted(
    deps: &Path,
    package: &str,
    digest: &str,
    unit: &str,
    variant: &[(&str, serde_json::Value)],
) -> PathBuf {
    let fingerprint = deps
        .parent()
        .unwrap()
        .join(format!(".fingerprint/{package}-{digest}"));
    let built = u64::from_str_radix(&digest[..8], 16).unwrap();
    let mut json = serde_json::json!({
        "rustc": 4, "features": "[]", "declared_features": format!("[\"{digest}\"]"),
        "target": 1, "profile": 2, "path": built, "deps": [[built, "dep", false, built]],
        "local": [built], "rustflags": [], "config": 3, "compile_kind": 0,
    });
    for (field, value) in variant {
        json[*field] = value.clone();
    }
    crate::discover::file(&fingerprint.join(format!("{unit}.json")), &json.to_string());
    for file in [
        unit.to_owned(),
        format!("dep-{unit}"),
        format!("output-{unit}"),
        "invoked.timestamp".to_owned(),
    ] {
        crate::discover::file(&fingerprint.join(file), "fingerprint");
    }
    fingerprint
}

/// The variant `fingerprinted` names unless told otherwise.
const PLAIN: &str = r#"4 "[]" 1 2 0 [] 3"#;

/// A file in `deps` that runs, written at `seconds`.
pub(crate) fn runnable(deps: &Path, name: &str, seconds: i64) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = artifact(deps, name, seconds);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    written(&path, seconds);
    path
}

/// An executable cargo built in `deps` for `package`'s `unit`, as its
/// fingerprint names it, written at `seconds`, with its dep-info.
fn executable(deps: &Path, package: &str, unit: &str, name: &str, digest: &str, seconds: i64) {
    runnable(deps, &format!("{name}-{digest}"), seconds);
    artifact(deps, &format!("{name}-{digest}.d"), seconds);
    fingerprinted(deps, package, digest, unit, &[]);
}

/// A test of `server` cargo built in `deps` at `digest` as the variant
/// `PLAIN` names but for `variant`, written at `seconds`.
fn test_build(deps: &Path, digest: &str, variant: &[(&str, serde_json::Value)], seconds: i64) {
    let unit = "test-integration-test-integration";
    runnable(deps, &format!("integration-{digest}"), seconds);
    artifact(deps, &format!("integration-{digest}.d"), seconds);
    fingerprinted(deps, "server", digest, unit, variant);
}

/// Each older build `replaced` finds, by digest, and whether a newer
/// build of its own variant replaced it.
fn judged(target: &Path) -> Vec<(String, bool)> {
    let mut found: Vec<_> = replaced(target)
        .into_iter()
        .map(|build| (build.digest, build.in_variant))
        .collect();
    found.sort();
    found
}

#[test]
fn only_a_newer_build_of_its_own_variant_says_an_executable_is_done_with() {
    // A test built with two sets of features, or under two profiles,
    // holds a build of each, both in use; one differing from a newer
    // build only in what replacing it changes is done with.
    use serde_json::json;
    let variants = [
        ("rustc", json!(7)),
        ("features", json!(r#"["a"]"#)),
        ("target", json!(7)),
        ("profile", json!(7)),
        ("compile_kind", json!(7)),
        ("rustflags", json!(["-Cdebuginfo=0"])),
        ("config", json!(7)),
    ];
    for (field, value) in variants {
        let root = tempfile::tempdir().unwrap();
        let deps = root.path().join("target/debug/deps");
        test_build(&deps, OLD, &[], 1_000);
        test_build(&deps, NEW, &[], 5_000);
        test_build(&deps, "aaaaaaaaaaaaaaaa", &[(field, value.clone())], 500);
        test_build(&deps, "bbbbbbbbbbbbbbbb", &[(field, value)], 900);
        assert_eq!(
            judged(&root.path().join("target")),
            [
                (OLD.to_owned(), true),
                ("aaaaaaaaaaaaaaaa".to_owned(), true),
                ("bbbbbbbbbbbbbbbb".to_owned(), false),
            ],
            "a build of another {field} was judged done with, or an older one of its own wasn't"
        );
    }
}

#[test]
fn a_fingerprint_reached_through_a_link_names_no_unit() {
    // A folder anyone can clone can plant a link where cargo writes its
    // fingerprint's JSON, which could lead anywhere, so that last link is
    // never followed; one higher up leads only to another small file.
    let root = tempfile::tempdir().unwrap();
    let deps = root.path().join("target/debug/deps");
    test_build(&deps, OLD, &[], 1_000);
    test_build(&deps, NEW, &[], 5_000);
    let json = root.path().join(format!(
        "target/debug/.fingerprint/server-{OLD}/test-integration-test-integration.json"
    ));
    let real = root.path().join("elsewhere.json");
    fs::rename(&json, &real).unwrap();
    std::os::unix::fs::symlink(&real, &json).unwrap();
    assert_eq!(
        judged(&root.path().join("target")),
        [],
        "a fingerprint was read through a link"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn a_fingerprint_that_is_a_pipe_names_no_unit_and_never_waits() {
    // Opening a pipe nobody writes to waits for a writer, so a planted
    // one would hang every survey.
    use std::os::unix::ffi::OsStrExt;
    let root = tempfile::tempdir().unwrap();
    let deps = root.path().join("target/debug/deps");
    test_build(&deps, OLD, &[], 1_000);
    test_build(&deps, NEW, &[], 5_000);
    let json = root.path().join(format!(
        "target/debug/.fingerprint/server-{OLD}/test-integration-test-integration.json"
    ));
    fs::remove_file(&json).unwrap();
    let name = std::ffi::CString::new(json.as_os_str().as_bytes()).unwrap();
    // SAFETY: name is a valid C string naming a path in a temp directory.
    let made = unsafe { libc::mkfifo(name.as_ptr(), 0o600) };
    assert_eq!(made, 0, "the pipe couldn't be made");
    let target = root.path().join("target");
    let (said, heard) = std::sync::mpsc::channel();
    std::thread::spawn(move || said.send(judged(&target)));
    let found = heard.recv_timeout(std::time::Duration::from_secs(10));
    assert_eq!(
        found,
        Ok(vec![]),
        "a fingerprint that is a pipe was waited on or read"
    );
}

#[test]
fn a_fingerprint_past_its_size_names_no_unit() {
    // Cargo writes a few hundred bytes, and a planted file could be as
    // big as the disk; what's past the cap is never read, so the JSON
    // after a long run of blanks never parses.
    let root = tempfile::tempdir().unwrap();
    let deps = root.path().join("target/debug/deps");
    test_build(&deps, OLD, &[], 1_000);
    test_build(&deps, NEW, &[], 5_000);
    let json = root.path().join(format!(
        "target/debug/.fingerprint/server-{OLD}/test-integration-test-integration.json"
    ));
    let read = fs::read_to_string(&json).unwrap();
    fs::write(&json, format!("{}{read}", " ".repeat(256 << 10))).unwrap();
    assert_eq!(
        judged(&root.path().join("target")),
        [],
        "a fingerprint past its size was read whole"
    );
}

#[test]
fn a_library_is_replaced_whatever_its_variant_and_never_by_the_hour() {
    // Two live builds of a library can differ only in how a dependency
    // was built, so only a day unopened says one is done with.
    let root = tempfile::tempdir().unwrap();
    let deps = root.path().join("target/debug/deps");
    let features = serde_json::json!(r#"["b"]"#);
    for (digest, variant, seconds) in [
        (OLD, vec![], 1_000),
        (NEW, vec![], 5_000),
        ("aaaaaaaaaaaaaaaa", vec![("features", features)], 500),
    ] {
        artifact(&deps, &format!("libapp-{digest}.rlib"), seconds);
        artifact(&deps, &format!("app-{digest}.d"), seconds);
        fingerprinted(&deps, "app", digest, "lib-app", &variant);
    }
    assert_eq!(
        judged(&root.path().join("target")),
        [
            (OLD.to_owned(), false),
            ("aaaaaaaaaaaaaaaa".to_owned(), false)
        ],
        "an older library wasn't replaced, or one was judged done with by its variant"
    );
}

#[test]
fn an_older_executable_of_a_unit_is_replaced_and_the_newest_stays() {
    // On 18 November a gate's build held 81 builds of one test binary,
    // 28 GiB of executables in all a newer build of the same test had
    // replaced. Only one package's same unit replaces another.
    let root = tempfile::tempdir().unwrap();
    let deps = root.path().join("target/debug/deps");
    let test = "test-integration-test-integration";
    executable(&deps, "server", test, "integration", OLD, 1_000);
    executable(&deps, "server", test, "integration", NEW, 5_000);
    // Another package's test of the same name is its own newest.
    executable(
        &deps,
        "client",
        test,
        "integration",
        "aaaaaaaaaaaaaaaa",
        500,
    );
    // A binary and its unit tests share a name, and neither replaces the
    // other.
    executable(&deps, "tool", "bin-dt", "dt", "bbbbbbbbbbbbbbbb", 500);
    executable(&deps, "tool", "test-bin-dt", "dt", "cccccccccccccccc", 900);
    // The profile's own copy of an older binary shares its file, so taking
    // it would free nothing and only make cargo link it again.
    executable(&deps, "tool", "bin-dt", "dt", "dddddddddddddddd", 100);
    fs::hard_link(
        deps.join("dt-dddddddddddddddd"),
        root.path().join("target/debug/dt"),
    )
    .unwrap();
    // Nothing says which unit an executable with no fingerprint is.
    runnable(&deps, "stray-eeeeeeeeeeeeeeee", 100);
    runnable(&deps, "stray-ffffffffffffffff", 900);
    let found = replaced(&root.path().join("target"));
    let older = Build {
        deps: deps.clone(),
        name: "integration".to_owned(),
        digest: OLD.to_owned(),
        files: vec![
            deps.join(format!("integration-{OLD}")),
            deps.join(format!("integration-{OLD}.d")),
        ],
        fingerprint: Some(
            root.path()
                .join(format!("target/debug/.fingerprint/server-{OLD}")),
        ),
        unit: Some(Unit {
            name: format!("server/{test}"),
            variant: PLAIN.to_owned(),
        }),
        in_variant: true,
        written: 1_000.0,
        size: 10,
    };
    assert_eq!(
        found,
        vec![older],
        "only the older build of one package's test is replaced"
    );
}

#[test]
fn reclaiming_an_executable_takes_it_its_dep_info_and_fingerprint() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("target");
    let deps = target.join("debug/deps");
    let test = "test-integration-test-integration";
    executable(&deps, "server", test, "integration", OLD, 1_000);
    executable(&deps, "server", test, "integration", NEW, 5_000);
    let anchor = Dir::open(root.path()).unwrap();
    let candidate = Candidate {
        path: deps.join(format!("integration-{OLD}")),
        ..older(&target)
    };
    let outcome = reclaim::build(
        &anchor,
        root.path(),
        &target,
        &candidate,
        &mut crate::reclaim::Script::default(),
    );
    assert_eq!(
        outcome,
        Outcome::Reclaimed,
        "the older executable wasn't taken"
    );
    assert_eq!(
        names(&deps),
        [format!("integration-{NEW}"), format!("integration-{NEW}.d")],
        "an older executable's build was left, or the newer one's taken"
    );
    assert_eq!(
        names(&target.join("debug/.fingerprint")),
        [format!("server-{NEW}")],
        "the older executable's fingerprint stayed, or the newer one's went"
    );
}

#[test]
fn a_build_a_process_works_in_is_kept_whatever_its_locks_say() {
    // A cargo can be between taking its lock and its rustc, or a test
    // binary run without cargo, so the lock alone can't say a target is
    // free.
    let root = tempfile::tempdir().unwrap();
    let target = target(root.path());
    let anchor = Dir::open(root.path()).unwrap();
    let checkout = root.path().join("app");
    let candidate = Candidate {
        checkout: Some(checkout.clone()),
        ..older(&target)
    };
    let mut looker = Judge {
        target: target.clone(),
        checkout: checkout.clone(),
        finds: Some(Use::Busy(4242)),
    };
    let outcome = reclaim::build(&anchor, root.path(), &target, &candidate, &mut looker);
    assert!(
        matches!(&outcome, Outcome::Kept(why) if why.contains("4242")),
        "a build a process works in wasn't kept for it: {outcome:?}"
    );
    assert_eq!(
        names(&target.join("debug/deps")).len(),
        8,
        "a build a process works in lost files"
    );
    // One that can't be read might be working there, as might any when
    // the processes can't be read at all.
    looker.finds = Some(Use::Unknown(4244));
    let outcome = reclaim::build(&anchor, root.path(), &target, &candidate, &mut looker);
    assert!(
        matches!(&outcome, Outcome::Kept(why) if why.contains("4244")),
        "a build an unreadable process may work in wasn't kept: {outcome:?}"
    );
    looker.finds = None;
    let outcome = reclaim::build(&anchor, root.path(), &target, &candidate, &mut looker);
    assert!(
        matches!(&outcome, Outcome::Kept(why) if why.contains("can't be judged")),
        "a build was taken though nothing could say who works in it: {outcome:?}"
    );
    assert_eq!(
        names(&target.join("debug/deps")).len(),
        8,
        "a build no judgement cleared lost files"
    );
    // One that only sits there, as a shell naming the target or an
    // editor's macro server does, builds from no older build.
    looker.finds = Some(Use::Occupied(4243));
    let outcome = reclaim::build(&anchor, root.path(), &target, &candidate, &mut looker);
    assert_eq!(
        outcome,
        Outcome::Reclaimed,
        "a process sitting in the target kept an older build"
    );
}

/// Says `finds` of the target and checkout it was made for, or that it
/// can't read them when it finds nothing, and that anything else is free,
/// so a take judging the wrong path is seen.
struct Judge {
    target: PathBuf,
    checkout: PathBuf,
    finds: Option<Use>,
}

impl reclaim::Looker for Judge {
    fn judge(&mut self, target: &Path, checkout: Option<&Path>) -> std::io::Result<Use> {
        let asked = target == self.target && checkout == Some(self.checkout.as_path());
        if !asked {
            return Ok(Use::Free);
        }
        self.finds
            .ok_or_else(|| std::io::Error::other("the processes can't be read"))
    }

    fn moved(&mut self) {}

    fn waiters(&mut self, _inodes: &[u64]) -> std::io::Result<Vec<i32>> {
        Ok(Vec::new())
    }
}

#[test]
fn a_build_is_used_when_last_opened_and_never_before_its_deps_was_watched() {
    let deps = Path::new("/t/target/debug/deps");
    let mut uses = Uses::default();
    // Nobody watched it, so nobody knows.
    assert_eq!(uses.last_used(&build(deps, 1_000.0)), None);
    uses.watching(deps, 5_000.0);
    assert_eq!(uses.last_used(&build(deps, 1_000.0)), Some(5_000.0));
    assert_eq!(uses.last_used(&build(deps, 6_000.0)), Some(6_000.0));
    uses.opened(&deps.join(format!("libalpha-{OLD}.rmeta")), 9_000.0);
    uses.opened(&deps.join("libbeta-3333333333333333.rlib"), 12_000.0);
    assert_eq!(uses.last_used(&build(deps, 1_000.0)), Some(9_000.0));
    // Watching again later keeps when it began.
    uses.watching(deps, 20_000.0);
    assert_eq!(uses.last_used(&build(deps, 1_000.0)), Some(9_000.0));
    // Events lost to an overflow could have been opens of anything.
    uses.lost(30_000.0);
    assert_eq!(uses.last_used(&build(deps, 1_000.0)), Some(30_000.0));
}

#[test]
fn a_target_is_last_opened_when_its_lock_or_a_build_in_its_deps_was() {
    // Every cargo opens its profile's lock, even one that finds all fresh
    // and writes nothing, so the profile is watched beside its deps.
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("target");
    let profile = target.join("debug");
    let deps = profile.join("deps");
    fs::create_dir_all(&deps).unwrap();
    let kept = target.join("run");
    fs::create_dir_all(kept.join("debug/deps")).unwrap();
    fs::write(kept.join("CACHEDIR.TAG"), CARGO_TAG).unwrap();
    assert_eq!(
        watched_dirs(&target),
        vec![profile.clone(), deps.clone()],
        "a target's watch marks each profile and its deps, and nothing of a target kept in it"
    );
    let mut uses = Uses::default();
    assert_eq!(
        uses.last_opened_in(&watched_dirs(&target)),
        None,
        "a target nothing opened has no last open"
    );
    uses.opened(&deps.join(format!("libalpha-{OLD}.rlib")), 7_000.0);
    assert_eq!(
        uses.last_opened_in(&watched_dirs(&target)),
        Some(7_000.0),
        "a library a build read in its deps didn't date the target"
    );
    uses.opened(&profile.join(".cargo-build-lock"), 9_000.0);
    uses.opened(&kept.join("debug/.cargo-build-lock"), 20_000.0);
    uses.opened(
        &profile.join("incremental/alpha-1/s-1/query-cache.bin"),
        30_000.0,
    );
    assert_eq!(
        uses.last_opened_in(&watched_dirs(&target)),
        Some(9_000.0),
        "something deeper than a target's watched folders dated it"
    );
}

// A Mac gives no folder a handle, so nothing there is ever heard.
#[cfg(target_os = "linux")]
#[test]
fn a_target_is_heard_since_this_guard_marked_its_folders_as_they_are() {
    // An open the watch missed would make a build in use look old, so
    // what it says is trusted only from when this guard's marks last
    // heard everything, on the folders there now.
    let root = tempfile::tempdir().unwrap();
    let dirs = [root.path().join("debug"), root.path().join("debug/deps")];
    fs::create_dir_all(&dirs[1]).unwrap();
    let mut uses = Uses::default();
    uses.watching(&dirs[0], 1.0);
    uses.watching(&dirs[1], 1.0);
    assert_eq!(
        uses.heard_since(&dirs),
        None,
        "a target an earlier guard watched was heard by this one"
    );
    let mark = |uses: &mut Uses, dir: &Path, at: f64| {
        uses.marked(dir, at, disk_guard::opens::handle(dir).unwrap());
    };
    mark(&mut uses, &dirs[0], 100.0);
    assert_eq!(
        uses.heard_since(&dirs),
        None,
        "a target with an unmarked deps was heard"
    );
    mark(&mut uses, &dirs[1], 300.0);
    mark(&mut uses, &dirs[1], 400.0);
    assert_eq!(
        uses.heard_since(&dirs),
        Some(300.0),
        "a target isn't heard from when its last folder was first marked"
    );
    uses.lost(700.0);
    assert_eq!(
        uses.heard_since(&dirs),
        Some(700.0),
        "a target isn't heard from when the watch last lost events"
    );
    // Cleaned and built again: the mark went with the old folder.
    fs::rename(&dirs[1], root.path().join("old-deps")).unwrap();
    fs::create_dir(&dirs[1]).unwrap();
    assert_eq!(
        uses.heard_since(&dirs),
        None,
        "a deps made anew was heard through the old one's mark"
    );
    // Its handle read before it was made anew, as a mark landing on the
    // new folder after the watch named the old one would be.
    let before = disk_guard::opens::handle(&dirs[1]).unwrap();
    fs::remove_dir(&dirs[1]).unwrap();
    fs::create_dir(&dirs[1]).unwrap();
    uses.marked(&dirs[1], 800.0, before);
    assert_eq!(
        uses.heard_since(&dirs),
        None,
        "a deps made anew was heard through a handle read of the one before"
    );
    mark(&mut uses, &dirs[1], 900.0);
    assert_eq!(
        uses.heard_since(&dirs),
        Some(900.0),
        "a deps made anew isn't heard from when it was marked again"
    );
    fs::remove_dir(&dirs[1]).unwrap();
    assert_eq!(uses.heard_since(&dirs), None, "a gone deps was heard");
    uses.deaf();
    assert!(
        uses.watched.is_empty(),
        "a deaf guard still dated replaced builds by what it watched before"
    );
    fs::create_dir(&dirs[1]).unwrap();
    mark(&mut uses, &dirs[1], 1000.0);
    assert_eq!(
        uses.heard_since(&dirs),
        None,
        "a deaf guard still heard a profile it had marked"
    );
    assert_eq!(
        uses.heard_since(&[]),
        None,
        "a target with no profile was heard"
    );
}

#[test]
fn uses_of_what_is_gone_are_forgotten() {
    let root = tempfile::tempdir().unwrap();
    let deps = root.path().join("deps");
    let kept = artifact(&deps, "libalpha-2222222222222222.rlib", 1_000);
    let mut uses = Uses::default();
    uses.watching(&deps, 1.0);
    uses.watching(&root.path().join("gone"), 1.0);
    uses.opened(&kept, 2.0);
    uses.opened(&deps.join("libgone-5555555555555555.rlib"), 3.0);
    uses.forget_gone();
    assert_eq!(uses.watched.keys().collect::<Vec<_>>(), vec![&deps]);
    assert_eq!(uses.opened.keys().collect::<Vec<_>>(), vec![&kept]);
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn older(target: &Path) -> Candidate {
    Candidate {
        path: target.join(format!("debug/deps/alpha-{OLD}")),
        class: Class::Replaced,
        idle: 0.0,
        size: 15,
        occupied: false,
        spent: false,
        checkout: None,
    }
}

#[test]
fn reclaiming_a_build_takes_its_artifacts_and_fingerprint_alone() {
    let root = tempfile::tempdir().unwrap();
    let target = target(root.path());
    let anchor = Dir::open(root.path()).unwrap();
    let outcome = reclaim::build(
        &anchor,
        root.path(),
        &target,
        &older(&target),
        &mut crate::reclaim::Script::default(),
    );
    assert_eq!(outcome, Outcome::Reclaimed);
    assert_eq!(
        names(&target.join("debug/deps")),
        [
            "alpha-4444444444444444",
            "alpha-4444444444444444.d",
            &format!("libalpha-{NEW}.rlib"),
            &format!("libalpha-{NEW}.rmeta"),
            "libbeta-3333333333333333.rlib",
        ]
    );
    assert_eq!(
        names(&target.join("debug/.fingerprint")),
        Vec::<String>::new()
    );
    // The cross target's build of it is another candidate.
    assert_eq!(
        names(&target.join("x86_64-unknown-linux-gnu/release/deps")).len(),
        8
    );
}

#[test]
fn a_build_whose_target_cargo_holds_is_kept() {
    use rustix::fs::{FlockOperation, flock};
    let root = tempfile::tempdir().unwrap();
    let target = target(root.path());
    file(&target.join("debug/.cargo-lock"), "");
    let lock = fs::File::open(target.join("debug/.cargo-lock")).unwrap();
    flock(&lock, FlockOperation::LockShared).unwrap();
    let anchor = Dir::open(root.path()).unwrap();
    let outcome = reclaim::build(
        &anchor,
        root.path(),
        &target,
        &older(&target),
        &mut crate::reclaim::Script::default(),
    );
    assert!(matches!(outcome, Outcome::Kept(_)), "{outcome:?}");
    assert_eq!(names(&target.join("debug/deps")).len(), 8);
}

#[test]
fn a_build_stands_while_any_file_of_it_does_though_a_library_has_none_of_its_name() {
    // A pass on a build disk drops an offer that no longer stands, as
    // taken already by the disk beneath it.
    let root = tempfile::tempdir().unwrap();
    let deps = root.path().join("target/debug/deps");
    fs::create_dir_all(&deps).unwrap();
    let library = deps.join("serde-1111111111111111");
    fs::write(deps.join("libserde-1111111111111111.rmeta"), "").unwrap();
    assert!(
        stands(&library),
        "a library whose files stand read as taken already"
    );
    fs::remove_file(deps.join("libserde-1111111111111111.rmeta")).unwrap();
    fs::write(deps.join("serde-1111111111111111.d"), "").unwrap();
    assert!(
        stands(&library),
        "a build whose dep-info stands read as taken"
    );
    fs::remove_file(deps.join("serde-1111111111111111.d")).unwrap();
    assert!(
        !stands(&library),
        "a build none of whose files are left read as standing"
    );
    let test = deps.join("app-2222222222222222");
    fs::write(&test, "").unwrap();
    assert!(stands(&test), "an executable that stands read as taken");
}

#[test]
fn a_target_with_no_builds_replaces_nothing() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(replaced(&root.path().join("target")), Vec::<Build>::new());
    fs::create_dir_all(root.path().join("target/debug/deps")).unwrap();
    assert_eq!(replaced(&root.path().join("target")), Vec::<Build>::new());
}
