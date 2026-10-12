use disk_guard::anchor::Dir;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::Path;

const MIB: u64 = 1 << 20;

fn sandbox() -> crate::sandbox::Sandbox {
    crate::sandbox::Sandbox::new()
}

/// Bytes that won't compress or share extents, so the filesystem has to
/// allocate all of them.
fn noise(path: &Path, bytes: u64) {
    let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
    let data: Vec<u8> = (0..bytes)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect();
    fs::write(path, data).unwrap();
}

#[test]
fn a_root_opens_real_directories_beneath_it() {
    let root = sandbox();
    fs::create_dir_all(root.path().join("a/b")).unwrap();
    let dir = Dir::open(root.path()).unwrap();
    assert!(dir.child(Path::new("a/b")).is_ok());
    assert!(dir.child(Path::new("a/missing")).is_err());
}

#[test]
fn nothing_beneath_a_root_is_reached_through_a_symlink() {
    let root = sandbox();
    let elsewhere = sandbox();
    fs::create_dir_all(elsewhere.path().join("b")).unwrap();
    symlink(elsewhere.path(), root.path().join("a")).unwrap();
    let dir = Dir::open(root.path()).unwrap();
    assert!(dir.child(Path::new("a")).is_err());
    assert!(dir.child(Path::new("a/b")).is_err());
}

#[test]
fn nothing_above_a_root_is_reached() {
    let root = sandbox();
    fs::create_dir_all(root.path().join("a")).unwrap();
    let dir = Dir::open(&root.path().join("a")).unwrap();
    assert!(dir.child(Path::new("..")).is_err());
    assert!(dir.child(Path::new("../a")).is_err());
    assert!(dir.child(root.path()).is_err());
}

#[test]
fn a_file_is_not_a_directory() {
    let root = sandbox();
    fs::write(root.path().join("f"), "x").unwrap();
    assert!(Dir::open(&root.path().join("f")).is_err());
    assert!(
        Dir::open(root.path())
            .unwrap()
            .child(Path::new("f"))
            .is_err()
    );
}

#[test]
fn size_counts_what_is_allocated_beneath() {
    let root = sandbox();
    fs::create_dir_all(root.path().join("t/deps")).unwrap();
    noise(&root.path().join("t/deps/one"), MIB);
    noise(&root.path().join("t/two"), 2 * MIB);
    let size = Dir::open(root.path())
        .unwrap()
        .child(Path::new("t"))
        .unwrap()
        .size()
        .unwrap();
    assert!((3 * MIB..3 * MIB + 256 * 1024).contains(&size), "{size}");
}

/// What a directory holds, by name, sorted, with trash shown as one name.
fn listing(path: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .map(|name| {
            if name.starts_with(".disk-guard-trash-") {
                "trash".into()
            } else {
                name
            }
        })
        .collect();
    names.sort();
    names
}

fn target(root: &Path) -> std::path::PathBuf {
    let target = root.join("target");
    fs::create_dir_all(target.join("debug/deps")).unwrap();
    fs::create_dir_all(target.join("criterion/base")).unwrap();
    fs::write(target.join("debug/deps/libx.rlib"), "rlib").unwrap();
    fs::write(target.join("criterion/base/estimates.json"), "{}").unwrap();
    fs::write(target.join("CACHEDIR.TAG"), "tag").unwrap();
    target
}

#[test]
fn detaching_moves_all_but_what_is_kept_into_one_trash() {
    let root = sandbox();
    let target = target(root.path());
    let dir = Dir::open(&target).unwrap();
    let detached = dir.detach(&["criterion"]).unwrap();
    assert_eq!(listing(&target), ["criterion", "trash"]);
    let trash = target.join(detached.name());
    assert_eq!(listing(&trash), ["CACHEDIR.TAG", "debug"]);
}

#[test]
fn restoring_puts_everything_back_as_it_was() {
    let root = sandbox();
    let target = target(root.path());
    let dir = Dir::open(&target).unwrap();
    dir.detach(&["criterion"]).unwrap().restore().unwrap();
    assert_eq!(listing(&target), ["CACHEDIR.TAG", "criterion", "debug"]);
    assert_eq!(
        fs::read_to_string(target.join("debug/deps/libx.rlib")).unwrap(),
        "rlib"
    );
}

#[test]
fn emptying_removes_the_trash_and_nothing_it_points_at() {
    let root = sandbox();
    let elsewhere = sandbox();
    fs::write(elsewhere.path().join("precious"), "keep").unwrap();
    let target = target(root.path());
    symlink(elsewhere.path(), target.join("debug/away")).unwrap();
    symlink(elsewhere.path().join("precious"), target.join("debug/file")).unwrap();
    // Go's module cache makes its directories read-only.
    fs::create_dir_all(target.join("debug/locked/inner")).unwrap();
    fs::write(target.join("debug/locked/inner/f"), "x").unwrap();
    for path in ["debug/locked/inner", "debug/locked"] {
        fs::set_permissions(target.join(path), fs::Permissions::from_mode(0o555)).unwrap();
    }
    fs::create_dir_all(target.join("debug/shut/inner")).unwrap();
    fs::set_permissions(target.join("debug/shut"), fs::Permissions::from_mode(0o000)).unwrap();
    let dir = Dir::open(&target).unwrap();
    dir.detach(&["criterion"]).unwrap().empty().unwrap();
    assert_eq!(listing(&target), ["criterion"]);
    assert_eq!(
        fs::read_to_string(elsewhere.path().join("precious")).unwrap(),
        "keep"
    );
}

fn mode(path: &Path) -> u32 {
    fs::symlink_metadata(path).unwrap().permissions().mode() & 0o777
}

/// A target holding read-only and unreadable directories right at its top,
/// as a module cache's are, which a move into the trash has to change.
fn locked_down(root: &Path) -> std::path::PathBuf {
    let target = target(root);
    fs::create_dir_all(target.join("sealed/inner")).unwrap();
    fs::write(target.join("sealed/inner/f"), "x").unwrap();
    fs::set_permissions(target.join("sealed"), fs::Permissions::from_mode(0o555)).unwrap();
    fs::create_dir_all(target.join("shut")).unwrap();
    fs::set_permissions(target.join("shut"), fs::Permissions::from_mode(0o000)).unwrap();
    target
}

#[test]
fn read_only_directories_at_the_top_are_detached_and_emptied() {
    let root = sandbox();
    let target = locked_down(root.path());
    let dir = Dir::open(&target).unwrap();
    dir.detach(&["criterion"]).unwrap().empty().unwrap();
    assert_eq!(listing(&target), ["criterion"]);
}

#[test]
fn read_only_directories_at_the_top_are_restored_as_they_were() {
    let root = sandbox();
    let target = locked_down(root.path());
    let dir = Dir::open(&target).unwrap();
    dir.detach(&["criterion"]).unwrap().restore().unwrap();
    assert_eq!(
        listing(&target),
        ["CACHEDIR.TAG", "criterion", "debug", "sealed", "shut"]
    );
    assert_eq!(mode(&target.join("sealed")), 0o555);
    assert_eq!(mode(&target.join("shut")), 0o000);
    fs::set_permissions(target.join("sealed"), fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        fs::read_to_string(target.join("sealed/inner/f")).unwrap(),
        "x"
    );
}

#[test]
fn a_directory_that_took_the_name_meanwhile_keeps_its_own_mode() {
    let root = sandbox();
    let target = locked_down(root.path());
    let dir = Dir::open(&target).unwrap();
    let detached = dir.detach(&["criterion"]).unwrap();
    fs::create_dir(target.join("sealed")).unwrap();
    fs::set_permissions(target.join("sealed"), fs::Permissions::from_mode(0o750)).unwrap();
    assert!(detached.restore().is_err());
    assert_eq!(mode(&target.join("sealed")), 0o750);
    assert_eq!(mode(&target.join("shut")), 0o000);
}

#[test]
fn a_trash_left_behind_is_found_and_never_moved_into_another() {
    let root = sandbox();
    let target = target(root.path());
    let dir = Dir::open(&target).unwrap();
    let first = dir.detach(&["criterion"]).unwrap().name().to_owned();
    fs::create_dir(target.join("release")).unwrap();
    let second = dir.detach(&["criterion"]).unwrap().name().to_owned();
    assert_ne!(first, second);
    assert_eq!(listing(&target.join(&second)), ["release"]);
    let leftovers = dir.leftovers().unwrap();
    assert_eq!(leftovers.len(), 2);
    for trash in leftovers {
        trash.empty().unwrap();
    }
    assert_eq!(listing(&target), ["criterion"]);
}

#[test]
fn size_follows_no_symlink_and_counts_a_link_once() {
    let root = sandbox();
    let elsewhere = sandbox();
    noise(&elsewhere.path().join("big"), 4 * MIB);
    fs::create_dir(root.path().join("t")).unwrap();
    symlink(elsewhere.path(), root.path().join("t/away")).unwrap();
    symlink(elsewhere.path().join("big"), root.path().join("t/big")).unwrap();
    noise(&root.path().join("t/one"), MIB);
    fs::hard_link(root.path().join("t/one"), root.path().join("t/again")).unwrap();
    let size = Dir::open(&root.path().join("t")).unwrap().size().unwrap();
    assert!((MIB..MIB + 128 * 1024).contains(&size), "{size}");
}
