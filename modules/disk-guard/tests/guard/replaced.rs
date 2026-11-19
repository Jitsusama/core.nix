use crate::discover::{CARGO_TAG, file};
use disk_guard::activity::Use;
use disk_guard::anchor::Dir;
use disk_guard::pass::Outcome;
use disk_guard::plan::{Candidate, Class};
use disk_guard::reclaim;
use disk_guard::replaced::{Build, Unit, Uses, replaced, watched_dirs};
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
fn artifact(deps: &Path, name: &str, seconds: i64) -> PathBuf {
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
        // A test binary, which no build reads, is no library to replace.
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
        written,
        size: 10,
    }
}

/// The fingerprint cargo writes beside `deps` for `package`'s `unit`
/// built at `digest`, built with `features` under the profile and mode
/// cargo hashes as `profile`.
pub(crate) fn fingerprinted(
    deps: &Path,
    package: &str,
    digest: &str,
    unit: &str,
    features: &str,
    profile: u64,
) -> PathBuf {
    let fingerprint = deps
        .parent()
        .unwrap()
        .join(format!(".fingerprint/{package}-{digest}"));
    let json = serde_json::json!({
        "rustc": 9, "features": features, "declared_features": "[]", "target": 1,
        "profile": profile, "path": 5, "deps": [], "local": [], "rustflags": [],
        "config": 3, "compile_kind": 0,
    });
    crate::discover::file(&fingerprint.join(format!("{unit}.json")), &json.to_string());
    for file in [
        unit.to_owned(),
        format!("dep-{unit}"),
        "invoked.timestamp".to_owned(),
    ] {
        crate::discover::file(&fingerprint.join(file), "fingerprint");
    }
    fingerprint
}

/// The variant `fingerprinted` names, with no features under profile 2.
const PLAIN: &str = r#""[]" 1 2 0 [] 3"#;

/// A file in `deps` that runs, written at `seconds`.
fn runnable(deps: &Path, name: &str, seconds: i64) -> PathBuf {
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
    fingerprinted(deps, package, digest, unit, "[]", 2);
}

/// A library of `package` cargo built in `deps` with `features` under
/// `profile`, written at `seconds`: only its metadata when checked, as
/// profile 7 stands for here.
fn library(deps: &Path, package: &str, digest: &str, features: &str, profile: u64, seconds: i64) {
    artifact(deps, &format!("lib{package}-{digest}.rmeta"), seconds);
    if profile != 7 {
        artifact(deps, &format!("lib{package}-{digest}.rlib"), seconds);
    }
    artifact(deps, &format!("{package}-{digest}.d"), seconds);
    fingerprinted(
        deps,
        package,
        digest,
        &format!("lib-{package}"),
        features,
        profile,
    );
}

#[test]
fn each_variant_of_a_crate_keeps_its_newest_build() {
    // A crate built with two sets of features, or both checked and built,
    // holds a build of each, all in use: only the same variant's newer
    // build replaces one.
    let root = tempfile::tempdir().unwrap();
    let deps = root.path().join("target/debug/deps");
    library(&deps, "app", OLD, r#"["a"]"#, 2, 1_000);
    library(&deps, "app", NEW, r#"["a"]"#, 2, 5_000);
    library(&deps, "app", "aaaaaaaaaaaaaaaa", r#"["b"]"#, 2, 500);
    library(&deps, "app", "bbbbbbbbbbbbbbbb", r#"["a"]"#, 7, 400);
    library(&deps, "app", "cccccccccccccccc", r#"["a"]"#, 7, 600);
    let found: Vec<_> = replaced(&root.path().join("target"))
        .into_iter()
        .map(|build| build.digest)
        .collect();
    assert_eq!(
        found,
        [OLD, "bbbbbbbbbbbbbbbb"],
        "a variant's newest build went, or an older one of the same variant stayed"
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
    // The profile's own copy of an older binary is the one a person runs,
    // so it's never taken from under them.
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
        finds: Use::Busy(4242),
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
    // One that only sits there, as a shell naming the target or an
    // editor's macro server does, builds from no older build.
    looker.finds = Use::Occupied(4243);
    let outcome = reclaim::build(&anchor, root.path(), &target, &candidate, &mut looker);
    assert_eq!(
        outcome,
        Outcome::Reclaimed,
        "a process sitting in the target kept an older build"
    );
}

/// Says `finds` of the target and checkout it was made for, and that
/// anything else is free, so a take judging the wrong path is seen.
struct Judge {
    target: PathBuf,
    checkout: PathBuf,
    finds: Use,
}

impl reclaim::Looker for Judge {
    fn judge(&mut self, target: &Path, checkout: Option<&Path>) -> std::io::Result<Use> {
        let asked = target == self.target && checkout == Some(self.checkout.as_path());
        Ok(if asked { self.finds } else { Use::Free })
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
fn a_target_with_no_builds_replaces_nothing() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(replaced(&root.path().join("target")), Vec::<Build>::new());
    fs::create_dir_all(root.path().join("target/debug/deps")).unwrap();
    assert_eq!(replaced(&root.path().join("target")), Vec::<Build>::new());
}

/// Dates everything in `fingerprint` `hours` ago.
#[cfg(target_os = "linux")]
fn aged_fingerprint(fingerprint: &Path, hours: f64) {
    for entry in fs::read_dir(fingerprint).unwrap() {
        crate::survey::written_ago(&entry.unwrap().path(), hours * 3600.0);
    }
}

/// A build of `package`'s integration test in `deps`, written `hours` ago,
/// with its dep-info and the fingerprint naming its unit.
#[cfg(target_os = "linux")]
fn test_binary(deps: &Path, package: &str, digest: &str, hours: f64) -> PathBuf {
    use crate::survey::written_ago;
    let unit = "test-integration-test-integration";
    let binary = runnable(deps, &format!("integration-{digest}"), 0);
    file(&binary, &"x".repeat(4096));
    let info = artifact(deps, &format!("integration-{digest}.d"), 0);
    for path in [&binary, &info] {
        written_ago(path, hours * 3600.0);
    }
    aged_fingerprint(&fingerprinted(deps, package, digest, unit, "[]", 2), hours);
    binary
}

// A Mac hears nothing, so there every replaced build waits its day.
#[cfg(target_os = "linux")]
#[test]
fn a_budgeted_builds_replaced_artifacts_go_an_hour_unused_and_one_opened_within_it_stays() {
    // On 18 November a gate's build held 28 GiB of test binaries a newer
    // build of the same test had replaced, nearly all one to six hours
    // old, so the day a replaced build waited took none of them.
    use crate::survey::{find, now, offers_using, written_ago};
    use crate::survey_trees::{budgeted, heard};
    use disk_guard::plan::{self, Group};
    use disk_guard::survey::Offer;
    use std::collections::BTreeMap;
    let (_home, made, config) = budgeted(&[("app", 2.0)], 100);
    let target = made[0].join("target");
    let deps = target.join("debug/deps");
    // budgeted() builds the newer library; both need the fingerprint
    // naming their unit to be judged by the hour.
    aged_fingerprint(
        &fingerprinted(&deps, "app", "0123456789abcdef", "lib-app", "[]", 2),
        2.0,
    );
    let library = deps.join("libapp-1111111111111111.rlib");
    file(&library, "old");
    written_ago(&library, 3.0 * 3600.0);
    aged_fingerprint(
        &fingerprinted(&deps, "app", "1111111111111111", "lib-app", "[]", 2),
        3.0,
    );
    let unused = test_binary(&deps, "server", "1111111111111111", 3.0);
    let newest = test_binary(&deps, "server", "2222222222222222", 2.0);
    // Another package's test of the same name, the older opened since.
    let opened = test_binary(&deps, "client", "3333333333333333", 3.0);
    test_binary(&deps, "client", "4444444444444444", 2.0);
    let mut uses = heard(&made, 3.0);
    // One opened a minute inside the hour stays; one a minute past it goes.
    // Written out rather than read from the rule, so a changed hour fails.
    uses.opened(&opened, now() - 3540.0);
    uses.opened(&unused, now() - 3660.0);
    let found = offers_using(&config, &[], &|_| false, &BTreeMap::new(), &uses);
    let spent = |found: &[Offer], path: &Path| {
        let offer = find(found, path);
        offer.candidate.spent && plan::windows(&offer.candidate).0 == Group::Waste
    };
    assert!(
        spent(&found, &deps.join("app-1111111111111111")),
        "a replaced library heard unused three hours stayed"
    );
    assert!(
        spent(&found, &unused),
        "a replaced test binary last opened a minute past the hour stayed"
    );
    assert!(
        !spent(&found, &opened),
        "a replaced test binary opened a minute inside the hour went"
    );
    assert!(
        found.iter().all(|offer| offer.candidate.path != newest),
        "the newest build of a test was offered"
    );
    // Watched three hours but heard only half an hour, as after the guard
    // restarts, or watched only by an earlier guard, an open in the hour
    // may have gone unheard.
    let mut earlier = Uses::default();
    let mut restarted = Uses::default();
    for dir in watched_dirs(&target) {
        earlier.watching(&dir, now() - 3.0 * 3600.0);
        restarted.watching(&dir, now() - 3.0 * 3600.0);
        let handle = disk_guard::opens::handle(&dir).unwrap();
        restarted.marked(&dir, now() - 1800.0, handle);
    }
    for (why, uses) in [("heard half an hour", restarted), ("unheard", earlier)] {
        let found = offers_using(&config, &[], &|_| false, &BTreeMap::new(), &uses);
        assert!(
            !spent(&found, &unused),
            "a replaced test binary went within its day, {why}"
        );
    }
    // Outside a budget, nobody asked for builds to be held small.
    let unbudgeted = crate::survey::config(&format!(
        r#""cargo": {{ "kind": "cargo", "roots": ["{}"] }}"#,
        made[0].parent().unwrap().display()
    ));
    let found = offers_using(&unbudgeted, &[], &|_| false, &BTreeMap::new(), &uses);
    assert!(
        !spent(&found, &unused),
        "a replaced test binary went within its day outside a budget"
    );
}

// Without a fingerprint naming its unit, a replaced library can't be told
// from a live variant, so it waits its day whatever was heard.
#[cfg(target_os = "linux")]
#[test]
fn a_budgeted_library_with_no_fingerprint_waits_its_day() {
    use crate::survey::{find, offers_using, written_ago};
    use crate::survey_trees::{budgeted, heard};
    use std::collections::BTreeMap;
    let (_home, made, config) = budgeted(&[("app", 2.0)], 100);
    let deps = made[0].join("target/debug/deps");
    let library = deps.join("libapp-1111111111111111.rlib");
    file(&library, "old");
    written_ago(&library, 3.0 * 3600.0);
    let uses = heard(&made, 3.0);
    let found = offers_using(&config, &[], &|_| false, &BTreeMap::new(), &uses);
    assert!(
        !find(&found, &deps.join("app-1111111111111111"))
            .candidate
            .spent,
        "a replaced library with no fingerprint went within its day"
    );
}
