//! Which build the train goes as, against stand-in builds and the link
//! the watch follows, in a state folder of each test's own.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::super::tests::root;
use super::*;

/// Two builds standing in for merge-net's, under the state folder `root`,
/// and the link the watch follows, pointing at the first.
fn builds(root: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let store = root.join("store");
    fs::create_dir_all(&store).unwrap();
    let (old, new) = (store.join("old"), store.join("new"));
    fs::write(&old, "").unwrap();
    fs::write(&new, "").unwrap();
    let link = root.join("profile");
    std::os::unix::fs::symlink(&old, &link).unwrap();
    (
        fs::canonicalize(old).unwrap(),
        fs::canonicalize(new).unwrap(),
        link,
    )
}

#[test]
fn the_train_starts_as_the_build_installed_not_the_askers() {
    let root = root("starts-installed");
    let (old, new, link) = builds(&root);
    assert_eq!(
        build(&root, Ok(new.clone())),
        Some(new.clone()),
        "with no installed build named, the train didn't start as the asker's"
    );
    install(&root, &link).unwrap();
    assert_eq!(
        build(&root, Ok(new.clone())),
        Some(old.clone()),
        "the train started as the asker's build, not the one the watch follows"
    );
    fs::remove_file(&link).unwrap();
    assert_eq!(
        build(&root, Ok(new.clone())),
        Some(new),
        "an installed build that can't be read didn't leave the asker's"
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_ride_goes_on_as_the_build_a_switch_installed() {
    let root = root("goes-on-installed");
    let (old, new, link) = builds(&root);
    assert_eq!(
        newer(&root, Ok(old.clone()), None),
        None,
        "with no installed build named, a ride went on as another"
    );
    install(&root, &link).unwrap();
    assert_eq!(
        newer(&root, Ok(old.clone()), None),
        None,
        "a ride as the installed build went on as another"
    );
    fs::remove_file(&link).unwrap();
    std::os::unix::fs::symlink(&new, &link).unwrap();
    assert_eq!(
        newer(&root, Ok(old), None),
        Some(new.clone()),
        "a ride from before the switch didn't go on as the build it installed"
    );
    let unreadable = Err(io::Error::other("no exe"));
    assert_eq!(
        newer(&root, unreadable, Some(&new)),
        None,
        "a ride handed over to the installed build, unable to read its own path, handed over again"
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn the_watch_names_its_build_absolute_keeping_the_link() {
    let root = root("names-absolute");
    let (old, new, link) = builds(&root);
    let cwd = std::env::current_dir().unwrap();
    install(&root, &pathdiff(&link, &cwd)).unwrap();
    let named = fs::read_to_string(root.join("installed")).unwrap();
    assert!(
        Path::new(named.trim()).is_absolute(),
        "the watch named its build by a path relative to its own folder: {named}"
    );
    fs::remove_file(&link).unwrap();
    std::os::unix::fs::symlink(&new, &link).unwrap();
    assert_eq!(
        build(&root, Ok(old)),
        Some(new),
        "the named build didn't follow the link a switch moved"
    );
    fs::remove_dir_all(&root).unwrap();
}

/// `path` relative to `from`, by as many `..` as `from` is deep.
fn pathdiff(path: &Path, from: &Path) -> PathBuf {
    let mut relative = PathBuf::new();
    for _ in from.components().skip(1) {
        relative.push("..");
    }
    relative.join(path.strip_prefix("/").unwrap())
}
