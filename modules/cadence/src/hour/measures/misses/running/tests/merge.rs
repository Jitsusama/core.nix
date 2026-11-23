//! How a change's merge is found on main: by its subject alone, and by a
//! walk that stops at the merge rather than reading all of main's history.

use super::*;

#[test]
fn a_merge_is_found_by_its_subject_not_a_later_body() {
    let found = concat!(
        "1111111 fix(pi): quote the crash (#5800)\n",
        "2222222 Merge pull request #5800 from me/fix\n",
    );
    assert_eq!(subject_merging(found, 5800), Some("1111111".to_owned()));
    let body = "3333333 Merge pull request #5801 from me/next\n";
    assert_eq!(
        subject_merging(body, 5800),
        None,
        "a later merge whose body named the change was taken as its merge"
    );
}

#[test]
fn a_merge_is_found_without_reading_main_below_it() {
    let repo = scratch("merge-walk");
    std::fs::create_dir_all(&repo).expect("a scratch history");
    git(&repo, &["init", "-q", "-b", "main"]);
    for message in ["the root", "a step", "fix(x): the fix (#7)"] {
        git(&repo, &["commit", "-q", "--allow-empty", "-m", message]);
    }
    git(
        &repo,
        &[
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "a later one\n\nAfter (#7)",
        ],
    );
    let sha = |rev: &str| {
        let out = std::process::Command::new("git")
            .args(["-C", &repo.to_string_lossy(), "rev-parse", rev])
            .output()
            .expect("git runs");
        assert!(out.status.success(), "git couldn't name {rev}");
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
    };
    let (merge, root) = (sha("HEAD~1"), sha("HEAD~3"));
    // The root's object made a pipe nobody writes, so reading it waits for
    // good, as a history too long to read does in effect: only a walk left
    // at the merge answers at all.
    let object = repo.join(".git/objects").join(&root[..2]).join(&root[2..]);
    std::fs::remove_file(&object).expect("the root's object removed");
    let made = std::process::Command::new("mkfifo")
        .arg(&object)
        .status()
        .expect("mkfifo runs");
    assert!(made.success(), "the root's object wasn't made a pipe");
    let path = repo.to_string_lossy().into_owned();
    let (sent, answer) = std::sync::mpsc::channel();
    std::thread::spawn(move || sent.send(merge_of(&path, 7)).ok());
    let answer = answer.recv_timeout(std::time::Duration::from_secs(30));
    if answer.is_err() {
        // A walk stuck at the root is let go, so a failing run leaves no git
        // waiting behind it.
        std::fs::OpenOptions::new().write(true).open(&object).ok();
    }
    assert_eq!(
        answer.ok().flatten(),
        Some(merge),
        "the merge wasn't found before main's history below it was read"
    );
}
