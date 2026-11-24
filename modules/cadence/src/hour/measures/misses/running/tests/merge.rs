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

/// A history whose merge of #7 sits above a root no read ever ends, as a
/// history too long to read does in effect: the root's object is made a
/// pipe nobody writes. Gives the history, the merge and the pipe.
fn endless_below(name: &str) -> (String, String, std::path::PathBuf) {
    let repo = scratch(name);
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
    let object = repo.join(".git/objects").join(&root[..2]).join(&root[2..]);
    std::fs::remove_file(&object).expect("the root's object removed");
    let made = std::process::Command::new("mkfifo")
        .arg(&object)
        .status()
        .expect("mkfifo runs");
    assert!(made.success(), "the root's object wasn't made a pipe");
    (repo.to_string_lossy().into_owned(), merge, object)
}

#[test]
fn a_merge_is_found_without_reading_main_below_it() {
    // Only a walk left at the merge answers at all.
    let (path, merge, object) = endless_below("merge-walk");
    let (sent, answer) = std::sync::mpsc::channel();
    std::thread::spawn(move || sent.send(merge_of(&path, 7)).ok());
    let answer = answer.recv_timeout(std::time::Duration::from_secs(30));
    if answer.is_err() {
        // A walk stuck at the root is let go, so a failing run leaves no git
        // waiting behind it.
        std::fs::OpenOptions::new().write(true).open(&object).ok();
    }
    assert_eq!(
        answer.ok().and_then(Result::ok).flatten(),
        Some(merge),
        "the merge wasn't found before main's history below it was read"
    );
}

#[test]
fn a_merge_not_on_main_is_left_at_its_time() {
    // #8 never merged, so its walk reads to the root, which never ends.
    let (path, _, object) = endless_below("merge-unfound");
    let (sent, answer) = std::sync::mpsc::channel();
    let within = std::time::Duration::from_secs(1);
    let began = std::time::Instant::now();
    std::thread::spawn(move || sent.send(merge_within(&path, 8, within)).ok());
    let answer = answer.recv_timeout(std::time::Duration::from_secs(30));
    let took = began.elapsed();
    if answer.is_err() {
        std::fs::OpenOptions::new().write(true).open(&object).ok();
    }
    assert_eq!(
        answer,
        Ok(Err(within)),
        "a walk for a merge not on main wasn't left at its time, saying so"
    );
    // A walk that ended on its own, short of its time, didn't wait for one.
    assert!(
        took >= within,
        "the walk ended in {took:?}, before its time"
    );
}

#[test]
fn a_walk_left_at_its_time_is_the_hours_blindness_not_its_owners() {
    assert_eq!(
        said_of_walk(Some(UNFOUND), true),
        Some(LEFT),
        "a walk left at its time with no merge found didn't say it ran out of time"
    );
    assert_eq!(
        said_of_walk(Some(UNFOUND), false),
        Some(UNFOUND),
        "a walk that read to main's root said it ran out of time"
    );
    assert_eq!(
        said_of_walk(Some(NOT_CARRIED), true),
        Some(NOT_CARRIED),
        "a left walk hid a reason that wasn't about the merge"
    );
    assert_eq!(
        said_of_walk(None, true),
        None,
        "a change that runs was held for a walk left at its time"
    );
    // Read as the card and the owing read the hour's own blindness, so the
    // change's owner is never asked about a loaded machine's clock.
    assert!(
        LEFT.starts_with(UNSEEN),
        "a walk left at its time would be asked of the change's owner: {LEFT}"
    );
}

#[test]
fn the_walk_flushes_each_record_whatever_it_inherits() {
    // The walk above is run again in a test run of its own that hands git
    // GIT_FLUSH=0, which holds every record in git's buffer until the walk
    // ends: only a walk that flushes each finds the merge there.
    let (_, here) = module_path!()
        .split_once("::")
        .expect("a test's path names its crate");
    let test = format!("{here}::a_merge_is_found_without_reading_main_below_it");
    let run = std::process::Command::new(std::env::current_exe().expect("this test run"))
        .args([test.as_str(), "--exact"])
        .env("GIT_FLUSH", "0")
        .output()
        .expect("a test run of its own");
    let said = String::from_utf8_lossy(&run.stdout);
    assert!(
        run.status.success() && said.contains("1 passed"),
        "an inherited GIT_FLUSH=0 held the merge until main's root: {said}"
    );
}
