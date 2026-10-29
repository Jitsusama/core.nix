use disk_guard::finished::{Answers, Checkout, Predicate, ask, changed, read};
use regex_lite::Regex;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

/// Runs git in a fixture repository, reading none of the machine's own
/// configuration, so the fixture is the same wherever the tests run.
pub(crate) fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.com")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.com")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_owned()
}

fn repository(dir: &Path) -> String {
    git(dir, &["init", "-q", "-b", "main"]);
    git(dir, &["commit", "-q", "--allow-empty", "-m", "first"]);
    git(
        dir,
        &[
            "remote",
            "add",
            "origin",
            "https://example.com/team/project.git",
        ],
    );
    git(
        dir,
        &[
            "remote",
            "add",
            "fork",
            "https://example.com/me/project.git",
        ],
    );
    git(dir, &["rev-parse", "HEAD"])
}

#[test]
fn a_checkout_is_read_with_its_branch_head_and_remote() {
    let dir = tempfile::tempdir().unwrap();
    let head = repository(dir.path());
    assert_eq!(
        read(dir.path()).unwrap(),
        Checkout {
            branch: Some("main".into()),
            head: head.clone(),
            remote: "https://example.com/team/project.git".into(),
        }
    );
    git(dir.path(), &["checkout", "-q", "-b", "work"]);
    git(dir.path(), &["config", "branch.work.remote", "fork"]);
    assert_eq!(
        read(dir.path()).unwrap().remote,
        "https://example.com/me/project.git"
    );
    git(dir.path(), &["checkout", "-q", "--detach"]);
    assert_eq!(read(dir.path()).unwrap().branch, None);
}

#[test]
fn a_checkout_with_tracked_work_not_committed_is_changed() {
    let dir = tempfile::tempdir().unwrap();
    repository(dir.path());
    fs::write(dir.path().join("lib.rs"), "one").unwrap();
    git(dir.path(), &["add", "lib.rs"]);
    git(dir.path(), &["commit", "-q", "-m", "lib"]);
    // A build folder git doesn't know of is no one's work.
    fs::create_dir(dir.path().join("target")).unwrap();
    fs::write(dir.path().join("target/out"), "").unwrap();
    assert!(!changed(dir.path()), "as committed");
    fs::write(dir.path().join("lib.rs"), "two").unwrap();
    assert!(changed(dir.path()), "edited");
    git(dir.path(), &["checkout", "-q", "--", "lib.rs"]);
    fs::write(dir.path().join("new.rs"), "").unwrap();
    git(dir.path(), &["add", "new.rs"]);
    assert!(changed(dir.path()), "added");
}

#[test]
fn a_directory_that_isnt_a_checkout_reads_as_nothing() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(read(dir.path()), None);
}

fn predicate(script: &str) -> Predicate {
    Predicate {
        remote: Regex::new("^https://example\\.com/team/").unwrap(),
        command: vec![
            OsString::from("sh"),
            OsString::from("-c"),
            OsString::from(script),
        ],
        timeout: Duration::from_secs(5),
    }
}

fn checkout() -> Checkout {
    Checkout {
        branch: Some("done".into()),
        head: "abc".into(),
        remote: "https://example.com/team/project.git".into(),
    }
}

#[test]
fn a_predicate_is_asked_in_the_checkout_with_its_branch_head_and_remote() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().canonicalize().unwrap();
    let script = format!(
        r#"[ "$(pwd)" = "{}" ] && [ "$DISK_GUARD_BRANCH" = done ] && [ "$DISK_GUARD_HEAD" = abc ] \
           && [ "$DISK_GUARD_REMOTE" = https://example.com/team/project.git ] \
           && echo '{{"finished": true}}' || echo '{{"finished": false}}'"#,
        real.display()
    );
    assert!(ask(&predicate(&script), dir.path(), &checkout()));
    let unfinished = Checkout {
        branch: Some("open".into()),
        ..checkout()
    };
    assert!(!ask(&predicate(&script), dir.path(), &unfinished));
}

#[test]
fn anything_but_a_plain_yes_is_not_finished() {
    let dir = tempfile::tempdir().unwrap();
    for script in [
        r#"echo '{"finished": true}'; exit 1"#,
        r"echo 'finished'",
        r#"echo '{"finished": "true"}'"#,
        r#"echo '{"finished": true, "why": "merged"}'"#,
        r#"sleep 10; echo '{"finished": true}'"#,
    ] {
        let mut predicate = predicate(script);
        predicate.timeout = Duration::from_millis(300);
        assert!(!ask(&predicate, dir.path(), &checkout()), "{script}");
    }
    assert!(ask(
        &predicate(r#"echo '{"finished": true}'"#),
        dir.path(),
        &checkout()
    ));
}

#[test]
fn a_predicate_is_chosen_by_the_checkouts_remote() {
    let predicates = [predicate("true")];
    assert!(
        disk_guard::finished::predicate_for(&predicates, "https://example.com/team/x").is_some()
    );
    assert!(disk_guard::finished::predicate_for(&predicates, "https://example.com/me/x").is_none());
}

#[test]
fn a_finished_head_is_remembered_and_an_unfinished_one_asked_again_hourly() {
    let mut answers = Answers::default();
    let dir = Path::new("/home/u/src/project");
    let mut asked = 0;
    let mut answer = |finished: bool| {
        asked += 1;
        finished
    };
    assert!(!answers.finished(dir, "abc", 0.0, || answer(false)));
    assert!(!answers.finished(dir, "abc", 3_000.0, || answer(true)));
    assert!(answers.finished(dir, "abc", 3_700.0, || answer(true)));
    assert!(answers.finished(dir, "abc", 900_000.0, || answer(false)));
    assert!(!answers.finished(dir, "def", 900_000.0, || answer(false)));
    assert_eq!(asked, 3);
}

#[test]
fn the_answers_for_a_checkout_that_went_are_forgotten() {
    let root = tempfile::tempdir().unwrap();
    let here = root.path().join("here");
    std::fs::create_dir(&here).unwrap();
    let gone = root.path().join("gone");
    let mut answers = Answers::default();
    answers.finished(&here, "abc", 0.0, || true);
    answers.finished(&gone, "abc", 0.0, || true);
    answers.forget_gone();
    let mut asked = Vec::new();
    answers.finished(&here, "abc", 1.0, || {
        asked.push("here");
        true
    });
    answers.finished(&gone, "abc", 1.0, || {
        asked.push("gone");
        true
    });
    assert_eq!(asked, ["gone"]);
}
