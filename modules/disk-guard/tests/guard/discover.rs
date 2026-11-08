use disk_guard::discover::{Checkout, Found, Kind, checkout_of};
use std::cell::Cell;
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

pub(crate) const CARGO_TAG: &str = "Signature: 8a477f597d28d172789f06886806bc55\n\
                         # This file is a cache directory tag created by cargo.\n";

pub(crate) fn file(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

pub(crate) fn dir(path: &Path) -> PathBuf {
    fs::create_dir_all(path).unwrap();
    path.to_owned()
}

/// A stop that comes once it has been asked `asks` times, counting every
/// ask in `asked`.
pub(crate) fn stop_after(asks: usize, asked: &Cell<usize>) -> impl Fn() -> bool + '_ {
    move || {
        asked.set(asked.get() + 1);
        asked.get() > asks
    }
}

/// A walk told to stop partway ends there, interrupted, rather than
/// walking the rest first, which held a switch's stop for 35 s.
#[test]
fn a_walk_told_to_stop_ends_there() {
    let root = tempfile::tempdir().unwrap();
    for n in 0..30 {
        checkout(root.path(), &format!("app-{n}"), ".");
    }
    let asked = Cell::new(0);
    let walked = disk_guard::discover::walk_until(root.path(), None, None, &stop_after(3, &asked));
    assert_eq!(walked.unwrap_err().kind(), io::ErrorKind::Interrupted);
    assert_eq!(asked.get(), 4, "it ends at the first ask that says stop");
}

/// A git checkout holding a crate at `crate_dir`.
pub(crate) fn checkout(root: &Path, name: &str, crate_dir: &str) -> PathBuf {
    let top = dir(&root.join(name));
    dir(&top.join(".git"));
    file(&top.join(crate_dir).join("Cargo.toml"), "[package]\n");
    file(&top.join(crate_dir).join("src/lib.rs"), "");
    top
}

/// A target kept away from its checkout, as `CARGO_TARGET_DIR` makes one,
/// whose dep-info names the sources it was built from.
fn tagged(target: &Path, sources: &[&Path]) -> PathBuf {
    file(&target.join("CACHEDIR.TAG"), CARGO_TAG);
    let names: Vec<String> = sources
        .iter()
        .map(|s| s.to_string_lossy().replace(' ', "\\ "))
        .collect();
    let deps = target.join("debug/deps");
    file(
        &deps.join("app-0123456789abcdef.d"),
        &format!("{}: {}\n", deps.display(), names.join(" ")),
    );
    target.to_owned()
}

/// A walk for no account's home, as the machine's guard walks.
fn walk(root: &Path, depth: Option<usize>) -> io::Result<Vec<Found>> {
    disk_guard::discover::walk(root, depth, None)
}

fn sorted(mut found: Vec<Found>) -> Vec<Found> {
    found.sort_by(|a, b| a.path.cmp(&b.path));
    found
}

fn target(path: PathBuf, checkout: Checkout) -> Found {
    Found {
        path,
        kind: Kind::Target,
        checkout,
    }
}

fn real(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap()
}

#[test]
fn a_target_beside_its_manifest_is_found_before_cargo_marks_it() {
    let root = tempfile::tempdir().unwrap();
    let top = checkout(&real(root.path()), "app", "crates/core");
    let path = dir(&top.join("crates/core/target/debug"));
    let found = walk(root.path(), None).unwrap();
    assert_eq!(
        found,
        [target(path.parent().unwrap().to_owned(), Checkout::At(top))]
    );
}

#[test]
fn a_target_anywhere_is_known_by_what_cargo_leaves_in_it() {
    let root = tempfile::tempdir().unwrap();
    let root = real(root.path());
    let top = checkout(&root, "app", ".");
    let source = top.join("src/lib.rs");
    let tag = tagged(&root.join("builds/app"), &[&source]);
    let info = dir(&root.join("lab/elsewhere"));
    file(&info.join(".rustc_info.json"), "{}");
    // Another tool's cache tag doesn't make a target.
    file(
        &root.join("cache/CACHEDIR.TAG"),
        "Signature: 8a477f597d28d172789f06886806bc55\n",
    );
    let found = sorted(walk(&root, None).unwrap());
    assert_eq!(
        found,
        [
            target(tag, Checkout::At(top)),
            target(info, Checkout::Unknown)
        ]
    );
}

#[test]
fn only_the_outermost_target_counts() {
    let root = tempfile::tempdir().unwrap();
    let outer = tagged(&real(root.path()).join("t"), &[]);
    file(&outer.join("debug/build/CACHEDIR.TAG"), CARGO_TAG);
    assert_eq!(
        walk(root.path(), None).unwrap(),
        [target(outer, Checkout::Unknown)]
    );
}

#[test]
fn a_target_without_a_manifest_beside_it_holds_targets() {
    // As a checkout of many projects keeps one each: tree/target/<project>.
    let root = tempfile::tempdir().unwrap();
    let tree = real(root.path()).join("tree");
    let top = checkout(&tree, "src", "a");
    let a = tagged(&tree.join("target/a"), &[&top.join("a/src/lib.rs")]);
    dir(&tree.join("target/b/debug"));
    let found = walk(root.path(), None).unwrap();
    assert_eq!(found, [target(a, Checkout::At(top))]);
}

#[test]
fn git_directories_bare_repositories_and_installs_are_not_walked() {
    let root = tempfile::tempdir().unwrap();
    let root = real(root.path());
    tagged(&root.join("app/.git/t"), &[]);
    tagged(&root.join("mirror.git/t"), &[]);
    tagged(&root.join("web/node_modules/pkg/t"), &[]);
    assert_eq!(walk(&root, None).unwrap(), []);
}

#[test]
fn a_cargo_homes_downloads_are_not_targets() {
    // Cargo tags its registry and git checkouts as it tags a target.
    let root = tempfile::tempdir().unwrap();
    let home = dir(&real(root.path()).join("scratch/.cargo"));
    file(&home.join(".package-cache"), "");
    file(&home.join(".global-cache"), "");
    file(&home.join("registry/CACHEDIR.TAG"), CARGO_TAG);
    file(&home.join("git/CACHEDIR.TAG"), CARGO_TAG);
    let shared = tagged(&home.join("target"), &[]);
    assert_eq!(
        walk(root.path(), None).unwrap(),
        [target(shared, Checkout::Unknown)]
    );
}

#[test]
fn installs_are_found_beside_their_manifest_and_lockfile() {
    let root = tempfile::tempdir().unwrap();
    let root = real(root.path());
    let web = checkout(&root, "web", ".");
    file(&web.join("package.json"), "{}");
    file(&web.join("pnpm-lock.yaml"), "");
    let modules = dir(&web.join("node_modules/left-pad"));
    let unlocked = dir(&root.join("loose/node_modules"));
    file(&root.join("loose/package.json"), "{}");
    let venv = dir(&web.join(".venv"));
    file(&venv.join("pyvenv.cfg"), "home = /usr/bin\n");
    dir(&root.join("py/.venv/lib"));
    let found = sorted(walk(&root, None).unwrap());
    let install = |path: PathBuf| Found {
        path,
        kind: Kind::Install,
        checkout: Checkout::At(web.clone()),
    };
    assert_eq!(
        found,
        [install(venv), install(modules.parent().unwrap().to_owned())]
    );
    assert!(!found.iter().any(|f| f.path == unlocked));
}

#[test]
fn symlinks_are_never_followed() {
    let root = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    tagged(&real(elsewhere.path()).join("t"), &[]);
    symlink(elsewhere.path(), root.path().join("link")).unwrap();
    assert_eq!(walk(root.path(), None).unwrap(), []);
}

#[test]
fn the_walk_goes_only_as_deep_as_it_is_told() {
    let root = tempfile::tempdir().unwrap();
    let deep = tagged(&real(root.path()).join("a/b/t"), &[]);
    assert_eq!(walk(root.path(), Some(2)).unwrap(), []);
    assert_eq!(
        walk(root.path(), Some(3)).unwrap(),
        [target(deep, Checkout::Unknown)]
    );
}

#[test]
fn a_target_whose_checkout_was_deleted_is_orphaned() {
    let root = tempfile::tempdir().unwrap();
    let root = real(root.path());
    let gone = root.join("src/gone");
    let t = tagged(&root.join("builds/gone"), &[&gone.join("src/lib.rs")]);
    dir(&root.join("src"));
    assert_eq!(checkout_of(&t, None), Checkout::Gone(gone));
}

#[test]
fn toolchain_and_registry_sources_say_nothing_of_the_checkout() {
    let root = tempfile::tempdir().unwrap();
    let root = real(root.path());
    let top = checkout(&root, "app", ".");
    let registry = root.join("home/.cargo/registry/src/serde-1.0/src/lib.rs");
    let store = Path::new("/nix/store/abc-rust/lib/rustlib/src/core/lib.rs");
    let t = tagged(
        &root.join("t"),
        &[&registry, store, &top.join("src/lib.rs")],
    );
    assert_eq!(checkout_of(&t, None), Checkout::At(top));
}

#[test]
fn a_checkout_on_another_filesystem_is_never_called_gone() {
    // A checkout on a drive that isn't mounted now looks deleted, but the
    // directory it was mounted at sat on another filesystem.
    let root = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let shm = Path::new("/dev/shm");
    let elsewhere = shm
        .metadata()
        .map(|m| m.dev() != root.path().metadata().unwrap().dev());
    if elsewhere.unwrap_or(false) {
        let source = shm.join("disk-guard-test-absent/src/lib.rs");
        let t = tagged(&root.path().join("t"), &[&source]);
        assert_eq!(checkout_of(&t, None), Checkout::Unknown);
        assert_eq!(checkout_of(&t, Some(root.path())), Checkout::Unknown);
    }
}

#[test]
fn a_build_on_its_own_filesystem_whose_checkout_left_the_home_is_orphaned() {
    // Cargo's builds can have a filesystem to themselves, which no checkout
    // is on. A checkout missing from the home's filesystem was deleted, since
    // that filesystem is here, though the builds' filesystem isn't it.
    let home = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let home = real(home.path());
    let Ok(builds) = tempfile::tempdir_in("/dev/shm") else {
        return;
    };
    let device = |path: &Path| path.metadata().unwrap().dev();
    if device(builds.path()) == device(&home) {
        return;
    }
    let gone = home.join("trees/gone");
    let t = tagged(&builds.path().join("ab/cd"), &[&gone.join("src/lib.rs")]);
    dir(&home.join("trees"));
    assert_eq!(checkout_of(&t, Some(&home)), Checkout::Gone(gone));
    assert_eq!(checkout_of(&t, None), Checkout::Unknown);
}
