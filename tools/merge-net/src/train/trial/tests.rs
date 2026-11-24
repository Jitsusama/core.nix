use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use super::*;
use crate::tests::git_in;

fn heads(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn a_head_is_tried_once_per_main_and_never_beside_its_own_trial() {
    assert_eq!(
        wanted("h", "m", &[], None),
        Wanted::Start,
        "an untried head wasn't tried"
    );
    assert_eq!(
        wanted("h", "m", &heads(&["h"]), None),
        Wanted::Runs,
        "a second trial started beside the one going"
    );
    assert_eq!(
        wanted("h", "m", &[], Some("m")),
        Wanted::Tried,
        "a head tried on this main was tried again"
    );
    assert_eq!(
        wanted("h", "m2", &[], Some("m")),
        Wanted::Start,
        "a head tried on an older main wasn't tried on the new one"
    );
}

#[test]
fn no_more_than_the_cap_go_at_once() {
    let full = heads(&["a", "b", "c"]);
    assert_eq!(
        full.len(),
        AT_ONCE,
        "the test's trials no longer fill the cap"
    );
    assert_eq!(
        wanted("h", "m", &full, None),
        Wanted::Full,
        "a trial started past the cap"
    );
    assert_eq!(
        wanted("h", "m", &full[..2], None),
        Wanted::Start,
        "a trial waited with room to go"
    );
}

#[test]
fn a_units_description_names_the_head_it_tries() {
    let head = "0123456789abcdef0123456789abcdef01234567";
    assert_eq!(
        trying(&format!("{DESCRIBED}{head}")),
        Some(head),
        "the head a trial's unit tries was misread"
    );
    assert_eq!(
        trying("merge-net runs changed-checks again at x"),
        None,
        "a rerun read as a trial"
    );
    assert!(
        unit(head).starts_with("merge-net-trial-"),
        "the unit isn't found as a trial's"
    );
}

#[test]
fn the_last_trials_main_is_read_back_from_its_outcome() {
    let folder = std::env::temp_dir().join(format!("merge-net-trials-{}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("h.json"), outcome("h", "m", "t", Some(0), 1)).unwrap();
    assert_eq!(
        tried_on(&folder, "h").as_deref(),
        Some("m"),
        "the trial's main was lost"
    );
    assert_eq!(
        tried_on(&folder, "other"),
        None,
        "an untried head read as tried"
    );
    fs::remove_dir_all(&folder).unwrap();
}

/// Runs git in `repo` as [`git_in`] does, with `input` on its input,
/// failing as [`rebased`]'s git does.
fn git_with(repo: &Path, arguments: &[&str], input: Option<&str>) -> Result<String, String> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(arguments)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(words) = input {
        use std::io::Write as _;
        child
            .stdin
            .take()
            .unwrap()
            .write_all(words.as_bytes())
            .unwrap();
    }
    drop(child.stdin.take());
    let ran = child.wait_with_output().unwrap();
    if ran.status.success() {
        Ok(String::from_utf8_lossy(&ran.stdout).trim().to_owned())
    } else {
        Err(String::from_utf8_lossy(&ran.stderr).trim().to_owned())
    }
}

#[test]
fn a_trial_makes_the_same_change_on_main_under_its_own_message() {
    let repo = std::env::temp_dir().join(format!("merge-net-trial-{}", std::process::id()));
    let _ = fs::remove_dir_all(&repo);
    fs::create_dir_all(&repo).unwrap();
    git_in(&repo, &["init", "-q", "-b", "main"]);
    let commit = |words: &str, subject: &str| {
        fs::write(repo.join("w"), words).unwrap();
        git_in(&repo, &["add", "w"]);
        git_in(&repo, &["commit", "-qm", subject]);
        git_in(&repo, &["rev-parse", "HEAD"])
    };
    let base = commit("a\nb\nc\nd\ne\n", "w");
    let head = commit("a\nb\nC\nd\ne\n", "fix(w): mend c");
    git_in(&repo, &["checkout", "-q", &base]);
    // Two lines from the head's, since git takes changes to lines side by
    // side for a conflict.
    let main = commit("A\nb\nc\nd\ne\n", "feat(w): mend a");
    let git = |arguments: &[&str], input: Option<&str>| git_with(&repo, arguments, input);
    let trial = rebased(&git, &main, &head).expect("a clean merge made no trial");
    assert_eq!(
        git_in(&repo, &["rev-parse", &format!("{trial}^")]),
        main,
        "the trial doesn't sit on main"
    );
    assert_eq!(
        git_in(&repo, &["show", &format!("{trial}:w")]),
        "A\nb\nC\nd\ne",
        "the trial isn't the head merged onto main"
    );
    assert_eq!(
        git_in(&repo, &["log", "-1", "--format=%s", &trial]),
        "fix(w): mend c",
        "the trial lost the subject ready finds the change's runs by"
    );
    assert_eq!(
        crate::patch_id(&repo, &main, &trial),
        crate::patch_id(&repo, &base, &head),
        "the trial isn't the same change as the head, so ready wouldn't read its pass"
    );
    let clash = commit("A\nb\nX\nd\ne\n", "feat(w): clash on c");
    let refused = rebased(&git, &clash, &head);
    assert!(
        refused.as_ref().is_err_and(|why| why.contains("conflicts")),
        "a head that conflicts with main was tried: {refused:?}"
    );
    fs::remove_dir_all(&repo).unwrap();
}
