use disk_guard::anchor::Dir;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;

const MIB: u64 = 1 << 20;

fn sandbox() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
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
