//! The keeper's cases: what it takes, what it refuses, and that it never
//! rewrites a record or removes one but as forget or age asks.

use super::*;

use std::fs::File;
use std::thread;

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const OTHER: &str = "89abcdef0123456789abcdef0123456789abcdef";

fn keep(commit: &str, lines: &serde_json::Value) -> String {
    serde_json::json!({ "keep": commit, "lines": lines }).to_string()
}

#[test]
fn a_keep_of_whole_objects_at_a_whole_commit_is_heard() {
    let asked = heard(&keep(
        COMMIT,
        &serde_json::json!([{ "check": "a" }, { "run": [] }]),
    ));
    assert_eq!(
        asked,
        Ok(Asked::Keep(
            At::new(COMMIT, None).unwrap(),
            vec![r#"{"check":"a"}"#.to_owned(), r#"{"run":[]}"#.to_owned()]
        )),
        "each line is written again whole, on one line"
    );
    assert_eq!(
        heard(&serde_json::json!({ "forget": COMMIT }).to_string()),
        Ok(Asked::Forget(At::new(COMMIT, None).unwrap())),
        "a forget names its commit"
    );
}

#[test]
fn what_could_reach_past_its_record_is_refused() {
    let long = "x".repeat(LINE_AT_MOST);
    for (asked, why) in [
        (keep("abc", &serde_json::json!([{}])), "a short commit"),
        (keep("../passed", &serde_json::json!([{}])), "a path"),
        (
            serde_json::json!({ "forget": "*" }).to_string(),
            "a forget of all",
        ),
        (
            keep(COMMIT, &serde_json::json!(["text"])),
            "a line not an object",
        ),
        (
            keep(COMMIT, &serde_json::json!([{ "printed": long }])),
            "a line past the bound",
        ),
        (keep(COMMIT, &serde_json::json!({})), "lines not a list"),
        (
            serde_json::json!({ "rewrite": COMMIT }).to_string(),
            "another verb",
        ),
        ("not json".to_owned(), "not JSON"),
    ] {
        assert!(heard(&asked).is_err(), "{why} was taken: {asked}");
    }
}

#[test]
fn a_record_only_grows_until_its_bound() {
    let folder = tempfile::tempdir().unwrap();
    let at = At::new(COMMIT, None).unwrap();
    let line = |text: &str| Asked::Keep(at.clone(), vec![text.to_owned()]);
    answer(folder.path(), &line(r#"{"check":"a"}"#)).unwrap();
    answer(folder.path(), &line(r#"{"check":"b"}"#)).unwrap();
    let kept = fs::read_to_string(folder.path().join(COMMIT)).unwrap();
    assert_eq!(
        kept, "{\"check\":\"a\"}\n{\"check\":\"b\"}\n",
        "a second keep adds to the first"
    );

    let full = File::create(folder.path().join(OTHER)).unwrap();
    full.set_len(RECORD_AT_MOST).unwrap();
    let other = Asked::Keep(At::new(OTHER, None).unwrap(), vec!["{}".to_owned()]);
    assert!(
        answer(folder.path(), &other).is_err(),
        "a record at its bound takes no more"
    );
}

#[test]
fn a_full_keeper_takes_no_new_commit_but_adds_to_one_it_holds() {
    let folder = tempfile::tempdir().unwrap();
    let keep = |commit: &str| Asked::Keep(At::new(commit, None).unwrap(), vec!["{}".to_owned()]);
    answer_holding(folder.path(), &keep(COMMIT), 1).unwrap();
    let refused = answer_holding(folder.path(), &keep(OTHER), 1);
    assert!(
        refused
            .as_ref()
            .is_err_and(|why| why.contains("holds 1 records")),
        "a new commit past the most was taken: {refused:?}"
    );
    assert_eq!(
        answer_holding(folder.path(), &keep(COMMIT), 1),
        Ok(()),
        "a commit it holds was refused more lines"
    );
}

#[test]
fn a_slow_asker_is_cut_off_within_the_whole_time_not_each_read() {
    let (mut keeper, mut asker) = UnixStream::pair().unwrap();
    let dripping = thread::spawn(move || {
        for _ in 0..20 {
            if asker.write_all(b"{").is_err() {
                return;
            }
            // A byte each 100 ms, well inside any one read's wait.
            thread::sleep(Duration::from_millis(100));
        }
    });
    let started = Instant::now();
    let said = said_within(&mut keeper, Duration::from_millis(300), 1024);
    assert!(said.is_err(), "a dripping asker was heard out: {said:?}");
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "it was held {:?}",
        started.elapsed()
    );
    drop(keeper);
    dripping.join().unwrap();
}

#[test]
fn forget_removes_one_commits_record_alone() {
    let folder = tempfile::tempdir().unwrap();
    for commit in [COMMIT, OTHER] {
        fs::write(folder.path().join(commit), "{}\n").unwrap();
    }
    let at = At::new(COMMIT, None).unwrap();
    answer(folder.path(), &Asked::Forget(at.clone())).unwrap();
    assert!(
        !folder.path().join(COMMIT).exists(),
        "the commit's record stayed"
    );
    assert!(
        folder.path().join(OTHER).exists(),
        "another commit's record went"
    );
    assert_eq!(
        answer(folder.path(), &Asked::Forget(at)),
        Ok(()),
        "forgetting what's gone is no failure"
    );
}

#[test]
fn records_untouched_for_a_month_age_out() {
    let folder = tempfile::tempdir().unwrap();
    let now = SystemTime::now();
    for commit in [COMMIT, OTHER] {
        fs::write(folder.path().join(commit), "{}\n").unwrap();
    }
    fs::write(folder.path().join("not-a-commit"), "").unwrap();
    let month_ago = now - KEPT_FOR - Duration::from_hours(1);
    for name in [COMMIT, "not-a-commit"] {
        File::options()
            .write(true)
            .open(folder.path().join(name))
            .unwrap()
            .set_modified(month_ago)
            .unwrap();
    }
    age_out(folder.path(), now);
    assert!(!folder.path().join(COMMIT).exists(), "an old record stayed");
    assert!(folder.path().join(OTHER).exists(), "a new record went");
    assert!(
        folder.path().join("not-a-commit").exists(),
        "a file no commit names went"
    );
}

#[test]
fn the_keeper_answers_what_its_asked_until_it_idles() {
    let folder = tempfile::tempdir().unwrap();
    let socket = folder.path().join("keep.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let records = folder.path().join("records");
    fs::create_dir(&records).unwrap();
    let kept = records.clone();
    let keeper = thread::spawn(move || serve(&listener, &kept, Duration::from_millis(500)));

    let asked = serde_json::json!({ "keep": COMMIT, "lines": [{ "check": "a" }] });
    assert_eq!(ask(&socket, &asked), Ok(()), "a keep was refused");
    assert_eq!(
        fs::read_to_string(records.join(COMMIT)).unwrap(),
        "{\"check\":\"a\"}\n",
        "the keep wasn't kept"
    );
    let refused = ask(&socket, &serde_json::json!({ "forget": "abc" }));
    assert!(
        refused
            .as_ref()
            .is_err_and(|why| why.starts_with("refused")),
        "a short commit's forget was taken: {refused:?}"
    );
    keeper.join().unwrap();
    assert!(
        ask_within(&socket, &asked, Duration::ZERO).is_err(),
        "the keeper was still answering after it idled"
    );
}

#[test]
fn a_keeper_whose_socket_is_away_a_moment_is_waited_for() {
    // A switch closes the socket and listens again: 9 s on 12 November,
    // which lost a gate's pass and cost its rerun 28 minutes.
    let folder = tempfile::tempdir().unwrap();
    let socket = folder.path().join("keep.sock");
    let records = folder.path().join("records");
    fs::create_dir(&records).unwrap();
    let (path, kept) = (socket.clone(), records.clone());
    let keeper = thread::spawn(move || {
        thread::sleep(Duration::from_millis(1500));
        let listener = UnixListener::bind(&path).unwrap();
        serve(&listener, &kept, Duration::from_millis(500));
    });
    let asked = serde_json::json!({ "keep": COMMIT, "lines": [{ "check": "a" }] });
    assert_eq!(
        ask_within(&socket, &asked, Duration::from_secs(20)),
        Ok(()),
        "a keeper back within the wait was given up on"
    );
    keeper.join().unwrap();
    assert!(records.join(COMMIT).exists(), "the keep wasn't kept");
}
