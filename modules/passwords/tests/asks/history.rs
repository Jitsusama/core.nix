//! What a released command may do to its repository, and what the row and
//! the answer say of it: a worktree writes its history and nothing later
//! git runs read, a repository's own checkout and a forged `.git` say
//! where it can't commit, and a `.git` the command made is set aside.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{LISTED, holds, release, releasing_saying, stored_bash, writable};
use crate::common::{Running, stored_folder};

/// What a row's Commits fact says, if it has one.
fn commits(row: &Value) -> Option<Value> {
    row["drawer"]["facts"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|fact| fact["name"] == "Commits")
        .map(|fact| fact["said"].clone())
}

/// What an answer's note tells the agent, if anything.
fn note(answer: &Value) -> &str {
    answer["note"].as_str().unwrap_or_default()
}

/// Lays out a repository at `common` with a worktree of it, named
/// `name`, whose own git folder says it belongs to `back`.
fn worktree_of(common: &Path, name: &str, back: &Path) -> PathBuf {
    for folder in ["objects", "refs", "logs", "hooks"] {
        std::fs::create_dir_all(common.join(folder)).unwrap();
    }
    std::fs::write(common.join("config"), "").unwrap();
    let own = common.join("worktrees").join(name);
    std::fs::create_dir_all(own.join("logs")).unwrap();
    std::fs::write(own.join("commondir"), "../..\n").unwrap();
    std::fs::write(own.join("gitdir"), format!("{}\n", back.display())).unwrap();
    std::fs::write(own.join("HEAD"), "ref: refs/heads/main\n").unwrap();
    std::fs::write(own.join("index"), "").unwrap();
    own
}

#[test]
fn a_release_from_a_repositorys_own_checkout_says_it_can_tag_but_not_commit() {
    let running = Running::releasing("asks-own", &[LISTED]);
    let git = running.folder.join(".git");
    for folder in ["objects", "refs", "hooks"] {
        std::fs::create_dir_all(git.join(folder)).unwrap();
    }
    std::fs::write(git.join("config"), "").unwrap();
    let released = releasing_saying(&running, "0223456789abcdef0123456789abcdef");
    assert_eq!(
        commits(&released.row),
        Some(json!("can't from here, only tags")),
        "the row didn't say a release here can't commit: {}",
        released.row
    );
    assert!(
        note(&released.answer).starts_with("Commits: can't from here, only tags."),
        "the agent wasn't told it can't commit from here: {}",
        released.answer
    );
    assert!(
        note(&released.answer).contains("worktree"),
        "the agent wasn't told where it could: {}",
        released.answer
    );
}

#[test]
fn a_released_command_in_a_worktree_writes_its_history_but_not_what_runs() {
    let running = Running::releasing("asks-worktree", &[LISTED]);
    let tree = &running.folder;
    let dot_git = tree.join(".git");
    let common = tree.join("home/src/repo/.git");
    let own = worktree_of(&common, "released", &dot_git);
    std::fs::write(&dot_git, format!("gitdir: {}\n", own.display())).unwrap();
    let released = releasing_saying(&running, "e123456789abcdef0123456789abcdef");
    assert_eq!(
        commits(&released.row),
        None,
        "a worktree, where a release commits, said it can't"
    );
    assert_eq!(
        note(&released.answer),
        "",
        "the agent was told something of a release that commits"
    );
    let args = released.args;
    let bind = Path::new("--bind");
    let ro = Path::new("--ro-bind");
    let kept = holds(&args, &[ro, &dot_git, &dot_git]);
    assert!(
        kept > holds(&args, &[bind, tree, tree]),
        "the worktree's .git could be pointed elsewhere: {args:?}"
    );
    for written in ["objects", "refs", "logs"] {
        let written = common.join(written);
        holds(&args, &[bind, &written, &written]);
    }
    let opened = holds(&args, &[bind, &own, &own]);
    for pointer in ["commondir", "gitdir"] {
        let pointer = own.join(pointer);
        assert!(
            holds(&args, &[ro, &pointer, &pointer]) > opened,
            "{} could be pointed elsewhere: {args:?}",
            pointer.display()
        );
    }
    let settings = own.join("config.worktree");
    assert!(
        holds(&args, &[ro, Path::new("/dev/null"), &settings]) > opened,
        "the worktree's own settings could be written: {args:?}"
    );
    let writable = writable(&args);
    assert_eq!(
        writable.len(),
        6,
        "the command may write more than its tree, the history and the cache: {writable:?}"
    );
}

#[test]
fn a_released_command_writes_no_history_a_forged_git_file_points_at() {
    let running = Running::releasing("asks-forged", &[LISTED]);
    let tree = &running.folder;
    let dot_git = tree.join(".git");
    let common = tree.join("home/src/repo/.git");
    let own = worktree_of(&common, "elsewhere", &tree.join("elsewhere/.git"));
    std::fs::write(&dot_git, format!("gitdir: {}\n", own.display())).unwrap();
    let released = releasing_saying(&running, "f123456789abcdef0123456789abcdef");
    assert_eq!(
        commits(&released.row),
        Some(json!("can't from here, nor tags")),
        "the row didn't say a release here writes no history: {}",
        released.row
    );
    assert!(
        note(&released.answer).starts_with("Commits: can't from here, nor tags,"),
        "the agent wasn't told it writes no history here: {}",
        released.answer
    );
    let args = released.args;
    holds(&args, &[Path::new("--ro-bind"), &dot_git, &dot_git]);
    let writable = writable(&args);
    assert!(
        writable
            .iter()
            .all(|path| !Path::new(path).starts_with(&common)),
        "a worktree that isn't this tree's was opened: {writable:?}"
    );
}

/// The names in `folder` that a `.git` was set aside as.
fn set_aside(folder: &Path) -> Vec<String> {
    std::fs::read_dir(folder)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| name.starts_with(".git.planted-"))
        .collect()
}

#[test]
fn a_git_a_released_command_made_is_set_aside_and_one_long_set_aside_goes() {
    let running = Running::releasing("asks-planted", &[LISTED]);
    let tree = &running.folder;
    std::fs::create_dir_all(tree.join("old/.git")).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let day = 24 * 60 * 60;
    let stale = tree.join(format!("stale/.git.planted-{}", now - 8 * day));
    let fresh = tree.join(format!("fresh/.git.planted-{}", now - day));
    for kept in [&stale, &fresh] {
        std::fs::create_dir_all(kept.join("hooks")).unwrap();
    }
    // The command's PATH holds only the profiles, bash's folder alone
    // here, so mkdir is named by where it is.
    let planting = format!(
        "{mkdir} -p '{tree}/a/.git' && : > '{tree}/a/.git/config' && {mkdir} -p '{tree}/b' \
         && echo 'gitdir: /elsewhere' > '{tree}/b/.git'",
        mkdir = stored_folder("mkdir").join("mkdir").display(),
        tree = tree.display()
    );
    let argv = vec![
        stored_bash().to_str().unwrap().to_owned(),
        "-c".to_owned(),
        planting,
    ];
    let released = release(&running, "2223456789abcdef0123456789abcdef", &argv);
    assert_eq!(
        released.answer["exit"], 0,
        "the command planted nothing: {}",
        released.answer
    );
    for planted in ["a", "b"] {
        let folder = tree.join(planted);
        assert!(
            std::fs::symlink_metadata(folder.join(".git")).is_err(),
            "the .git the command made in {planted} was left for git to find"
        );
        assert_eq!(
            set_aside(&folder).len(),
            1,
            "the .git the command made in {planted} wasn't kept aside"
        );
    }
    assert!(
        tree.join("old/.git").is_dir(),
        "a .git there before the command ran was set aside"
    );
    assert!(!stale.exists(), "a .git set aside over a week ago was kept");
    assert!(fresh.exists(), "a .git set aside a day ago went");
    let note = note(&released.answer);
    assert!(
        note.contains("a/.git") && note.contains("b/.git") && note.contains("submodule"),
        "the agent wasn't told what was set aside, or what to do instead: {}",
        released.answer
    );
}
