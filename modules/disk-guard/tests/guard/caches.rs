use disk_guard::band::Band;
use disk_guard::caches::{Entry, go_build, go_entry, last_written, whole};
use disk_guard::clock::Awake;
use disk_guard::plan::Class;
use rustix::fs::{AtFlags, CWD, Timespec, Timestamps, utimensat};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

fn touch(path: &Path, secs: i64) {
    let at = Timespec {
        tv_sec: secs,
        tv_nsec: 0,
    };
    utimensat(
        CWD,
        path,
        &Timestamps {
            last_access: at,
            last_modification: at,
        },
        AtFlags::SYMLINK_NOFOLLOW,
    )
    .unwrap();
}

fn hash(n: u8) -> String {
    format!("{n:02x}").repeat(32)
}

#[test]
fn gos_build_cache_is_read_entry_by_entry_with_its_last_use() {
    let root = tempfile::tempdir().unwrap();
    let cache = root.path();
    fs::create_dir_all(cache.join("00")).unwrap();
    fs::create_dir_all(cache.join("ab")).unwrap();
    fs::create_dir_all(cache.join("zz")).unwrap();
    fs::write(cache.join("README"), "go").unwrap();
    fs::write(cache.join("trim.txt"), "1").unwrap();
    let action = cache.join("00").join(format!("{}-a", hash(0)));
    let output = cache.join("ab").join(format!("{}-d", hash(0xab)));
    fs::write(&action, "action").unwrap();
    fs::write(&output, "output!").unwrap();
    touch(&action, 1_000);
    touch(&output, 2_000);
    fs::write(cache.join("ab").join("notes"), "x").unwrap();
    fs::write(cache.join("zz").join(format!("{}-a", hash(1))), "x").unwrap();
    fs::create_dir(cache.join("ab").join(format!("{}-a", hash(2)))).unwrap();
    symlink(&action, cache.join("ab").join(format!("{}-a", hash(3)))).unwrap();
    let mut found = go_build(cache);
    found.sort_by(|a, b| a.path.cmp(&b.path));
    assert_eq!(
        found,
        [
            Entry {
                path: action.clone(),
                last_used: 1_000.0,
                size: 6
            },
            Entry {
                path: output.clone(),
                last_used: 2_000.0,
                size: 7
            },
        ]
    );
}

#[test]
fn a_go_entry_is_a_cache_aged_by_its_last_use() {
    let entry = Entry {
        path: PathBuf::from("/c/go-build/00/x-a"),
        last_used: 1_000.0,
        size: 6,
    };
    let made = go_entry(&entry, 1, Band::Filling, &Awake::default(), 5_000.0);
    assert_eq!(
        made.class,
        Class::Cache {
            rank: 1,
            from: Band::Filling
        }
    );
    crate::assert_exactly(made.idle, 4_000.0);
    assert_eq!(made.size, 6);
    assert_eq!(made.path, entry.path);
}

#[test]
fn a_cache_without_a_protocol_goes_whole() {
    let made = whole(Path::new("/c/npm/_cacache"), 3, Band::NearlyFull, 42, 0.0);
    assert_eq!(
        made.class,
        Class::Cache {
            rank: 3,
            from: Band::NearlyFull
        }
    );
    assert_eq!(made.path, Path::new("/c/npm/_cacache"));
    assert_eq!(made.size, 42);
    crate::assert_exactly(made.idle, 0.0);
}

#[test]
fn a_whole_cache_is_as_idle_as_its_newest_write() {
    let root = tempfile::tempdir().unwrap();
    let cache = root.path().join("lima");
    let deep = cache.join("download").join("by-url-sha256").join("aa");
    fs::create_dir_all(&deep).unwrap();
    fs::write(cache.join("old"), "x").unwrap();
    fs::write(deep.join("data"), "image").unwrap();
    touch(&cache.join("old"), 1_000);
    touch(&deep.join("data"), 3_000);
    for dir in [
        &deep,
        &deep.parent().unwrap().to_owned(),
        &cache.join("download"),
        &cache,
    ] {
        touch(dir, 2_000);
    }
    assert_eq!(
        last_written(&cache, 100, f64::MAX),
        Some(3_000.0),
        "its newest write is the one deepest in it"
    );
    assert_eq!(
        last_written(&cache, 3, f64::MAX),
        None,
        "one too large to walk is taken as in use"
    );
    assert_eq!(
        last_written(&cache, 0, 2_000.0),
        Some(2_000.0),
        "a write recent enough ends the walk before it reads further"
    );
    assert_eq!(last_written(&root.path().join("gone"), 100, f64::MAX), None);
    let made = whole(&cache, 3, Band::NearlyFull, 42, 500.0);
    crate::assert_exactly(made.idle, 500.0);
}
