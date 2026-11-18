//! Docker's settings as a released command finds them: emptied where
//! they hold a login, through a link to the file it names, and where they
//! can't be read to tell; left be where they're missing, a dangling link
//! or a folder; and a fence list from before emptied files, naming none,
//! still lets a yes release.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{found, holds, released_with};
use crate::common::{DOCKER_CONFIG, FENCE_LIST, KEPT_SETTINGS, Running};

/// Bubblewrap's arguments emptying `file`.
fn emptying(file: &Path) -> [&Path; 3] {
    [Path::new("--ro-bind"), Path::new("/dev/null"), file]
}

/// Docker's settings in the running home, by the path the daemon binds:
/// the real one, every link on the way resolved.
fn docker_settings(running: &Running) -> PathBuf {
    std::fs::canonicalize(running.folder.join("home/.docker"))
        .unwrap()
        .join("config.json")
}

#[test]
fn a_fence_list_from_before_emptied_files_still_releases() {
    let running = Running::releasing("asks-docker-old-list", &[super::LISTED]);
    let mut list: Value = serde_json::from_str(FENCE_LIST).unwrap();
    list.as_object_mut().unwrap().remove("emptied");
    std::fs::write(running.fence_list(), list.to_string()).unwrap();
    let args = released_with(&running, "a223456789abcdef0123456789abcdef");
    let docker = docker_settings(&running);
    assert!(
        found(&args, &emptying(&docker)).is_none(),
        "a list naming no emptied files emptied Docker's settings: {args:?}"
    );
}

#[test]
fn a_link_to_dockers_settings_empties_the_file_it_names() {
    let running = Running::releasing("asks-docker-linked", &[super::LISTED]);
    let outside = running.folder.join("outside-docker.json");
    std::fs::write(&outside, DOCKER_CONFIG).unwrap();
    let docker = docker_settings(&running);
    std::fs::remove_file(&docker).unwrap();
    std::os::unix::fs::symlink(&outside, &docker).unwrap();
    let args = released_with(&running, "b223456789abcdef0123456789abcdef");
    holds(&args, &emptying(&std::fs::canonicalize(&outside).unwrap()));
    assert!(
        found(&args, &emptying(&docker)).is_none(),
        "the link itself was bound over, not only the file it names: {args:?}"
    );
}

#[test]
fn dockers_settings_missing_dangling_or_a_folder_bind_nothing_and_release() {
    // Most machines have no Docker settings at all, so a listed file that
    // isn't there, or is no file, must never keep a yes from releasing.
    for (shape, id) in [
        ("missing", "d223456789abcdef0123456789abcdef"),
        ("dangling", "e223456789abcdef0123456789abcdef"),
        ("folder", "f223456789abcdef0123456789abcdef"),
    ] {
        let running = Running::releasing(&format!("asks-docker-{shape}"), &[super::LISTED]);
        let docker = docker_settings(&running);
        std::fs::remove_file(&docker).unwrap();
        match shape {
            "dangling" => {
                std::os::unix::fs::symlink(running.folder.join("nowhere.json"), &docker).unwrap();
            }
            "folder" => std::fs::create_dir(&docker).unwrap(),
            _ => {}
        }
        let args = released_with(&running, id);
        assert!(
            !args.iter().any(|arg| Path::new(arg).starts_with(&docker)),
            "Docker's settings, {shape}, were bound over: {args:?}"
        );
    }
}

#[test]
fn dockers_settings_that_cant_be_read_are_emptied() {
    let running = Running::releasing("asks-docker-unreadable", &[super::LISTED]);
    let docker = docker_settings(&running);
    std::fs::write(&docker, KEPT_SETTINGS).unwrap();
    std::fs::set_permissions(&docker, std::fs::Permissions::from_mode(0o000)).unwrap();
    let args = released_with(&running, "c223456789abcdef0123456789abcdef");
    std::fs::set_permissions(&docker, std::fs::Permissions::from_mode(0o600)).unwrap();
    holds(&args, &emptying(&docker));
}
