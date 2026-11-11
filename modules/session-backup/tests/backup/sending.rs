//! Sending: each settled log sent once, sealed and packed, carrying on after a
//! stopped run, waiting out a busy Drive and never sending twice.

use std::fs::{self, File};
use std::time::SystemTime;

use session_backup::backup;

use super::{Account, DAY, StandIn, only_backup_calls, written};

#[test]
fn sends_each_settled_log_once_sealed_and_packed() {
    let google = StandIn::start();
    let account = Account::new(&google);
    account.log(
        "--home-a--",
        "2026-10-01_one.jsonl",
        "{\"one\":1}\n",
        3 * DAY,
    );
    account.log("", "loose.jsonl", "{\"loose\":1}\n", 5 * DAY);
    account.log(
        "--home-a--",
        "2026-10-25_fresh.jsonl",
        "{\"fresh\":1}\n",
        DAY / 2,
    );
    account.log("--home-a--", "notes.txt", "not a log", 9 * DAY);

    let done = account.run().unwrap();

    assert_eq!((done.sent, done.found, done.before), (2, 0, 0));
    let names = google.names();
    assert_eq!(names.len(), 2);
    assert!(
        names[0].starts_with("laptop/--home-a--/2026-10-01_one."),
        "{names:?}"
    );
    assert!(names[0].ends_with("Z.jsonl.zst.age"), "{names:?}");
    assert!(names[1].starts_with("laptop/loose."), "{names:?}");
    assert_eq!(account.open(&google.file(&names[0])), "{\"one\":1}\n");
    assert_eq!(account.open(&google.file(&names[1])), "{\"loose\":1}\n");

    let again = account.run().unwrap();
    assert_eq!((again.sent, again.before), (0, 2));
    assert_eq!(google.names().len(), 2);
    only_backup_calls(&google);
}

#[test]
fn carries_on_where_a_stopped_run_left_off() {
    let google = StandIn::start();
    let account = Account::new(&google);
    account.log("--a--", "old.jsonl", "old\n", 4 * DAY);
    account.log("--a--", "older.jsonl", "older\n", 6 * DAY);
    account.log("--a--", "oldest.jsonl", "oldest\n", 8 * DAY);
    // One run sends the oldest; Drive is busy for the next one's first send
    // through both retries, so that run ends there without failing.
    backup::run(&account.config, SystemTime::now(), Some(1)).unwrap();
    google.google.lock().unwrap().fail_sends = 3;

    let stopped = account.run().unwrap();
    assert_eq!(stopped.sent, 0);
    assert!(
        stopped.busy.as_deref().unwrap_or("").contains("(503)"),
        "{stopped:?}"
    );
    assert_eq!(google.names().len(), 1);

    let done = account.run().unwrap();
    assert_eq!((done.sent, done.before), (2, 1));
    assert_eq!(google.names().len(), 3);
    let names = google.names();
    assert!(names[0].contains("/--a--/old."), "{names:?}");
    // Nothing left behind half sealed.
    assert!(
        !account.root.join("state/sent.sealing").exists(),
        "a half-sealed copy was left behind"
    );
    only_backup_calls(&google);
}

#[test]
fn sends_again_when_drive_is_busy_for_a_moment() {
    let google = StandIn::start();
    let account = Account::new(&google);
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    account.log("--a--", "two.jsonl", "two\n", 4 * DAY);
    google.google.lock().unwrap().fail_sends = 2;

    let done = account.run().unwrap();

    assert_eq!((done.sent, done.busy), (2, None));
    assert_eq!(google.names().len(), 2);
    only_backup_calls(&google);
}

#[test]
fn never_sends_twice_when_drive_took_a_file_but_answered_busy() {
    let google = StandIn::start();
    let account = Account::new(&google);
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    google.google.lock().unwrap().lose_answers = 1;

    let done = account.run().unwrap();

    assert_eq!((done.sent, done.found, done.busy), (0, 1, None));
    assert_eq!(google.names().len(), 1);
    let opened = google
        .calls()
        .iter()
        .filter(|call| call.starts_with("POST /upload"))
        .count();
    assert_eq!(opened, 1);
    only_backup_calls(&google);
}

#[test]
fn leaves_the_sending_to_a_run_already_going() {
    let google = StandIn::start();
    let account = Account::new(&google);
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    fs::create_dir_all(account.root.join("state")).unwrap();
    let going = File::create(account.root.join("state/sent.lock")).unwrap();
    going.lock().unwrap();

    let done = account.run().unwrap();

    assert!(done.held, "{done:?}");
    assert_eq!(done.sent, 0);
    assert!(google.calls().is_empty(), "{:?}", google.calls());
    drop(going);
    assert_eq!(account.run().unwrap().sent, 1);
}

#[test]
fn finds_what_drive_took_before_the_ledger_said_so() {
    let google = StandIn::start();
    let account = Account::new(&google);
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    account.run().unwrap();
    // A run killed after Drive took it, before the ledger was written.
    fs::remove_file(account.root.join("state/sent.jsonl")).unwrap();

    let done = account.run().unwrap();

    assert_eq!((done.sent, done.found), (0, 1));
    assert_eq!(google.names().len(), 1);
    only_backup_calls(&google);
}

#[test]
fn copies_a_resumed_session_again_under_its_new_last_write() {
    let google = StandIn::start();
    let account = Account::new(&google);
    let log = account.log("--a--", "one.jsonl", "one\n", 10 * DAY);
    account.run().unwrap();
    fs::write(&log, "one\ntwo\n").unwrap();
    written(&log, 3 * DAY);

    let done = account.run().unwrap();

    assert_eq!(done.sent, 1);
    let names = google.names();
    assert_eq!(names.len(), 2);
    assert_eq!(account.open(&google.file(&names[1])), "one\ntwo\n");
    only_backup_calls(&google);
}
