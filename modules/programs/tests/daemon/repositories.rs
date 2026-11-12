//! The repositories under a folder and one's files over the socket: each
//! read as it's asked and served ready, and the repositories said again as
//! one is cloned, removed and cloned again, or its git writes, and under a
//! folder made only once it was watched.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};

use super::Running;

fn git_in(repository: &Path, args: &[&str]) {
    let ran = Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(args)
        .status();
    assert!(ran.unwrap().success(), "git {args:?} failed");
}

/// The repositories a case reads under its src: alpha, made by git with a
/// remote on GitHub and hidden and ignored files, beta three folders deep,
/// one too deep, and a worktree in a hidden folder. Answers src and alpha.
fn repositories_made(running: &Running) -> (PathBuf, PathBuf) {
    let src = running.path("src");
    let alpha = src.join("alpha");
    std::fs::create_dir_all(&alpha).unwrap();
    git_in(&alpha, &["init", "--quiet"]);
    git_in(
        &alpha,
        &[
            "remote",
            "add",
            "origin",
            "git@github.com:someone/alpha.git",
        ],
    );
    std::fs::create_dir_all(src.join("github.com/someone/beta/.git")).unwrap();
    std::fs::create_dir_all(src.join("far/away/too/deep/.git")).unwrap();
    std::fs::create_dir_all(alpha.join(".worktrees/branch/.git")).unwrap();
    std::fs::write(alpha.join(".gitignore"), "ignored.txt\n").unwrap();
    std::fs::write(alpha.join("ignored.txt"), "not listed").unwrap();
    std::fs::write(alpha.join(".env"), "KEY=1").unwrap();
    (src, alpha)
}

/// The paths of the repositories a record says, sorted.
fn paths_of(record: &Value) -> Vec<String> {
    let mut paths: Vec<String> = record["repositories"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|one| Some(one["path"].as_str()?.to_owned()))
        .collect();
    paths.sort();
    paths
}

/// The page on GitHub a record says of one repository, or null.
fn page_of(record: &Value, path: &Path) -> Value {
    record["repositories"]
        .as_array()
        .and_then(|all| {
            all.iter()
                .find(|one| one["path"] == json!(path.display().to_string()))
        })
        .map_or(Value::Null, |one| one["page"].clone())
}

#[test]
fn the_repositories_under_a_folder_and_their_files_are_read_as_each_is_asked() {
    let running = Running::new("repositories");
    let (src, alpha) = repositories_made(&running);
    let key = format!("repositories:{}", src.display());
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "get", "keys": [key]}));
    let record = &said["records"][0];
    assert_eq!(
        paths_of(record),
        [
            alpha.display().to_string(),
            format!("{}/github.com/someone/beta", src.display()),
        ],
        "not alpha and beta alone, three folders deep at most past no hidden one: {said}"
    );
    assert_eq!(
        page_of(record, &alpha),
        json!({"address": "https://github.com/someone/alpha", "name": "someone/alpha"}),
        "alpha's page on GitHub wasn't said: {said}"
    );
    assert_eq!(
        record["pages"].as_array().map(|pages| pages
            .iter()
            .map(|one| one["path"].clone())
            .collect::<Vec<_>>()),
        Some(vec![json!(alpha.display().to_string())]),
        "alpha alone wasn't said again as on GitHub: {said}"
    );
    let files = format!("files:{}", alpha.display());
    let said = client.ask(&json!({"id": 2, "ask": "get", "keys": [files]}));
    let mut named: Vec<String> = said["records"][0]["files"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|file| Some(file["path"].as_str()?.rsplit('/').next()?.to_owned()))
        .collect();
    named.sort();
    assert_eq!(
        named,
        [".env", ".gitignore"],
        "its hidden files but not what git ignores weren't said: {said}"
    );
    std::fs::create_dir_all(alpha.join("docs")).unwrap();
    std::fs::write(alpha.join("docs/main.rs"), "fn main() {}").unwrap();
    let said = client.ask(&json!({"id": 3, "ask": "get", "keys": [files]}));
    let made = said["records"][0]["files"]
        .as_array()
        .and_then(|all| all.iter().find(|file| file["within"] == "docs"))
        .cloned();
    assert!(
        made.is_some_and(|file| file["path"] == json!(format!("{}/docs/main.rs", alpha.display()))),
        "a file made since wasn't read, said within its folder, as it was asked again: {said}"
    );
}

#[test]
fn the_repositories_watched_are_said_again_as_one_is_cloned_or_its_git_writes() {
    let running = Running::new("repositories-heard");
    let (src, alpha) = repositories_made(&running);
    let key = format!("repositories:{}", src.display());
    let mut watching = running.client();
    watching.ask(&json!({"id": 1, "ask": "watch", "kind": "repositories", "keys": [key]}));
    // Read as the watch begins, it's told first as the change it is.
    let first = watching.changed(&key);
    assert_eq!(
        paths_of(&first).len(),
        2,
        "alpha and beta weren't read as the watch began: {first}"
    );
    // Made aside and moved in whole, so the folder and its git land at once.
    std::fs::create_dir_all(running.path("gamma/.git")).unwrap();
    std::fs::rename(running.path("gamma"), src.join("gamma")).unwrap();
    let cloned = watching.changed(&key);
    assert!(
        paths_of(&cloned).contains(&format!("{}/gamma", src.display())),
        "a repository cloned while watched wasn't heard: {cloned}"
    );
    git_in(
        &alpha,
        &[
            "remote",
            "set-url",
            "origin",
            "git@github.com:someone/renamed.git",
        ],
    );
    let written = watching.changed(&key);
    assert_eq!(
        page_of(&written, &alpha)["name"],
        "someone/renamed",
        "a remote changed while watched wasn't heard: {written}"
    );
}

#[test]
fn the_repositories_under_a_folder_made_once_watched_are_each_said_as_cloned() {
    let running = Running::new("repositories-later");
    // Two folders short of there, as a new account's ~/src is.
    let src = running.path("later/src");
    let key = format!("repositories:{}", src.display());
    let mut watching = running.client();
    watching.ask(&json!({"id": 1, "ask": "watch", "kind": "repositories", "keys": [key]}));
    let first = watching.changed(&key);
    assert!(
        paths_of(&first).is_empty(),
        "a folder not there held some: {first}"
    );
    // Each cloned as git makes it, the folders on the way with it, and one
    // removed and cloned again, as a test's stand-ins are.
    for (name, cloned, count) in [
        ("one", true, 1),
        ("two", true, 2),
        ("two", false, 1),
        ("two", true, 2),
    ] {
        if cloned {
            git_in(
                &running.path("."),
                &["init", "-q", &src.join(name).display().to_string()],
            );
        } else {
            std::fs::remove_dir_all(src.join(name)).unwrap();
        }
        // Told until it holds as many, or the client's patience runs out.
        while paths_of(&watching.changed(&key)).len() != count {}
    }
}
