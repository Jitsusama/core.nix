//! Docker's settings as a released command finds them: emptied where
//! they hold a login, through a link to the file it names, and where they
//! can't be read to tell; left be where they're missing, a dangling link
//! or a folder; and a fence list from before emptied files, naming none,
//! still lets a yes release.

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{found, holds, released_with};
use crate::common::{DOCKER_CONFIG, FENCE_LIST, KEPT_SETTINGS, Running};

/// Bubblewrap's arguments emptying `file`, by binding settings that say
/// nothing over it, from the daemon's own folder.
fn emptying<'a>(empty: &'a Path, file: &'a Path) -> [&'a Path; 3] {
    [Path::new("--ro-bind"), empty, file]
}

/// The settings saying nothing the daemon binds over a file it empties.
fn empty_settings(running: &Running) -> PathBuf {
    running.folder.join("runtime/passwords/emptied.json")
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
        !args.iter().any(|arg| Path::new(arg) == docker),
        "a list naming no emptied files emptied Docker's settings: {args:?}"
    );
}

#[test]
fn settings_are_emptied_to_nothing_where_none_saying_nothing_can_be_written() {
    // A login is never left readable for want of a file to bind over it:
    // with the daemon's folder taken by a file, /dev/null is bound instead.
    let running = Running::releasing("asks-docker-unwritten", &[super::LISTED]);
    let runtime = running.folder.join("runtime");
    std::fs::create_dir_all(&runtime).unwrap();
    std::fs::write(runtime.join("passwords"), "").unwrap();
    let args = released_with(&running, "a523456789abcdef0123456789abcdef");
    let docker = docker_settings(&running);
    holds(&args, &emptying(Path::new("/dev/null"), &docker));
    // Podman refuses /dev/null, so the journal says which file, and why.
    let logged = running.logged();
    assert!(
        logged.contains(&format!("emptying {} with /dev/null", docker.display()))
            && logged.contains("File exists"),
        "the journal doesn't say what was emptied with /dev/null, or why: {logged}"
    );
}

/// A daemon whose list also empties `named` under ~/.ssh where it holds
/// a registry's login.
fn emptying_in_ssh(test: &str, named: &str) -> Running {
    let running = Running::releasing(test, &[super::LISTED]);
    let mut list: Value = serde_json::from_str(FENCE_LIST).unwrap();
    let inside = serde_json::json!({"path": format!(".ssh/{named}"), "when": ["auths"]});
    list["emptied"].as_array_mut().unwrap().push(inside);
    std::fs::write(running.fence_list(), list.to_string()).unwrap();
    running
}

#[test]
fn settings_in_ssh_are_left_to_its_emptying() {
    // ~/.ssh is made empty and read-only too, so a file the list empties
    // inside it is hidden already, and bubblewrap couldn't bind over it.
    let running = emptying_in_ssh("asks-docker-in-ssh", "settings.json");
    let ssh = running.folder.join("home/.ssh");
    std::fs::write(ssh.join("settings.json"), DOCKER_CONFIG).unwrap();
    let args = released_with(&running, "a623456789abcdef0123456789abcdef");
    let ssh = std::fs::canonicalize(&ssh).unwrap();
    holds(&args, &[Path::new("--tmpfs"), &ssh]);
    assert!(
        !args
            .iter()
            .any(|arg| Path::new(arg) == ssh.join("settings.json")),
        "something inside ~/.ssh was bound over: {args:?}"
    );
}

#[test]
fn settings_ssh_binds_back_are_emptied_over_it() {
    // ~/.ssh binds back the files it keeps, so one the list empties among
    // them would be read whole unless it's bound over after.
    let running = emptying_in_ssh("asks-docker-ssh-kept", "config");
    let ssh = running.folder.join("home/.ssh");
    std::fs::write(ssh.join("config"), DOCKER_CONFIG).unwrap();
    let args = released_with(&running, "a723456789abcdef0123456789abcdef");
    let kept = std::fs::canonicalize(ssh.join("config")).unwrap();
    let back = found(&args, &[Path::new("--ro-bind"), &kept, &kept]);
    let over = found(&args, &emptying(&empty_settings(&running), &kept));
    assert!(
        matches!((back, over), (Some(back), Some(over)) if back < over),
        "a file ~/.ssh keeps was not emptied after it was bound back: {args:?}"
    );
}

#[test]
fn settings_deeper_in_ssh_are_left_to_its_emptying() {
    // ~/.ssh binds back only files directly in it, so one deeper, though
    // its name is one it keeps, is hidden already.
    let running = emptying_in_ssh("asks-docker-ssh-deeper", "sub/config");
    let ssh = running.folder.join("home/.ssh");
    std::fs::create_dir_all(ssh.join("sub")).unwrap();
    std::fs::write(ssh.join("sub/config"), DOCKER_CONFIG).unwrap();
    let args = released_with(&running, "a923456789abcdef0123456789abcdef");
    let sub = std::fs::canonicalize(ssh.join("sub")).unwrap();
    assert!(
        !args.iter().any(|arg| Path::new(arg).starts_with(&sub)),
        "something deeper in ~/.ssh was bound over: {args:?}"
    );
}

#[test]
fn settings_ssh_keeps_are_emptied_though_the_list_hides_ssh() {
    // ~/.ssh's own emptying binds back what it keeps after the list's
    // folder is made empty, so the list hiding ~/.ssh hides none of those.
    let running = emptying_in_ssh("asks-docker-ssh-listed", "config");
    let mut list: Value =
        serde_json::from_str(&std::fs::read_to_string(running.fence_list()).unwrap()).unwrap();
    list["home"].as_array_mut().unwrap().push(".ssh/".into());
    std::fs::write(running.fence_list(), list.to_string()).unwrap();
    let ssh = running.folder.join("home/.ssh");
    std::fs::write(ssh.join("config"), DOCKER_CONFIG).unwrap();
    let args = released_with(&running, "aa23456789abcdef0123456789abcdef");
    let kept = std::fs::canonicalize(ssh.join("config")).unwrap();
    let back = found(&args, &[Path::new("--ro-bind"), &kept, &kept]);
    let over = found(&args, &emptying(&empty_settings(&running), &kept));
    assert!(
        matches!((back, over), (Some(back), Some(over)) if back < over),
        "a file ~/.ssh keeps was not emptied after it was bound back: {args:?}"
    );
}

#[test]
fn settings_through_a_linked_ssh_are_emptied() {
    // A linked ~/.ssh is not made empty, bubblewrap mounting on no link,
    // so a file the list empties through it is no folder's to hide.
    let running = emptying_in_ssh("asks-docker-ssh-linked", "settings.json");
    let ssh = running.folder.join("home/.ssh");
    let outside = running.folder.join("outside-ssh");
    std::fs::remove_dir_all(&ssh).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, &ssh).unwrap();
    std::fs::write(outside.join("settings.json"), DOCKER_CONFIG).unwrap();
    let args = released_with(&running, "a823456789abcdef0123456789abcdef");
    let settings = std::fs::canonicalize(outside.join("settings.json")).unwrap();
    holds(&args, &emptying(&empty_settings(&running), &settings));
}

#[test]
fn settings_in_a_folder_the_list_hides_are_left_to_it() {
    // Bubblewrap can't make a file to bind over inside a folder already
    // made empty and read-only, so binding there too would keep every yes
    // from releasing; the folder hides them already.
    let running = Running::releasing("asks-docker-in-hidden", &[super::LISTED]);
    let mut list: Value = serde_json::from_str(FENCE_LIST).unwrap();
    list["home"].as_array_mut().unwrap().push(".docker/".into());
    std::fs::write(running.fence_list(), list.to_string()).unwrap();
    let args = released_with(&running, "a423456789abcdef0123456789abcdef");
    let folder = docker_settings(&running).parent().unwrap().to_path_buf();
    holds(&args, &[Path::new("--tmpfs"), &folder]);
    assert!(
        !args
            .iter()
            .any(|arg| Path::new(arg).starts_with(&folder) && Path::new(arg) != folder),
        "something inside a folder the list hides was bound over: {args:?}"
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
    let empty = empty_settings(&running);
    holds(
        &args,
        &emptying(&empty, &std::fs::canonicalize(&outside).unwrap()),
    );
    assert!(
        found(&args, &emptying(&empty, &docker)).is_none(),
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
        let nowhere = running.folder.join("nowhere.json");
        assert!(
            !args
                .iter()
                .any(|arg| Path::new(arg).starts_with(&docker) || Path::new(arg) == nowhere),
            "Docker's settings, {shape}, were bound over: {args:?}"
        );
    }
}

#[test]
fn dockers_settings_that_cant_be_read_are_emptied() {
    // Settings holding no login but not text can't be read to tell, for
    // any reader, root too, where a file's mode stops none.
    let running = Running::releasing("asks-docker-unreadable", &[super::LISTED]);
    let docker = docker_settings(&running);
    let mut unreadable = KEPT_SETTINGS.as_bytes().to_vec();
    unreadable.extend([0xff, 0xfe]);
    std::fs::write(&docker, unreadable).unwrap();
    let args = released_with(&running, "c223456789abcdef0123456789abcdef");
    holds(&args, &emptying(&empty_settings(&running), &docker));
}

#[test]
fn emptied_settings_say_nothing_in_a_shape_every_reader_takes() {
    // Podman, which is the docker command here, refuses settings of no
    // bytes at all, where Docker reads them as none, so what's bound over
    // a file emptied is settings saying nothing rather than no settings.
    let running = Running::releasing("asks-docker-empty-shape", &[super::LISTED]);
    let args = released_with(&running, "a323456789abcdef0123456789abcdef");
    let empty = empty_settings(&running);
    holds(&args, &emptying(&empty, &docker_settings(&running)));
    assert_eq!(
        std::fs::read_to_string(&empty).unwrap(),
        "{}\n",
        "the settings bound over Docker's say something"
    );
}
