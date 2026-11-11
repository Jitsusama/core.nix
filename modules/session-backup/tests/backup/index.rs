//! The index: a row for each log sent, sealed beside it, kept for the next run
//! when Drive is busy, and read back for logs sent before there was one.

use std::fs;
use std::path::PathBuf;

use age::secrecy::ExposeSecret;
use serde_json::{Value, json};
use session_backup::config::Config;

use super::{Account, DAY, StandIn, keeping, only_backup_calls};

/// The account's settings, keeping an index, with the harness's records of
/// its sessions beside it.
fn indexing(account: &mut Account) {
    account.config.index = Some(account.root.join("state/index.jsonl"));
    account.config.records = Some(account.root.join("records"));
}

/// A session's log as pi writes one: its header, the name pi gave it twice
/// over, a question, two answers on two models, and a last line cut short.
const SESSION: &str = concat!(
    r#"{"type":"session","version":3,"id":"s-1","timestamp":"2026-10-01T10:00:00.000Z","cwd":"/home/a/src/thing"}"#,
    "\n",
    r#"{"type":"session_info","timestamp":"2026-10-01T10:00:05.000Z","name":"First"}"#,
    "\n",
    r#"{"type":"message","timestamp":"2026-10-01T10:01:00.000Z","message":{"role":"user","content":"hi"}}"#,
    "\n",
    r#"{"type":"message","timestamp":"2026-10-01T10:02:00.000Z","message":{"role":"assistant","model":"big","usage":{"cost":{"total":0.504}}}}"#,
    "\n",
    r#"{"type":"session_info","timestamp":"2026-10-01T10:03:00.000Z","name":"Better Name"}"#,
    "\n",
    r#"{"type":"message","timestamp":"2026-10-01T11:30:00.000Z","message":{"role":"assistant","model":"small","usage":{"cost":{"total":0.25}}}}"#,
    "\n",
    r#"{"type":"message","timest"#,
);

/// The names Drive holds the index's rows under.
fn index_objects(google: &StandIn) -> Vec<String> {
    google
        .names()
        .into_iter()
        .filter(|name| name.starts_with("laptop/index/"))
        .collect()
}

/// The rows in a sealed index object, or in the index on the machine.
fn rows(text: &str) -> Vec<Value> {
    text.lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn indexes_each_log_it_sends_and_sends_the_rows_sealed() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    indexing(&mut account);
    fs::create_dir_all(account.root.join("records")).unwrap();
    fs::write(
        account.root.join("records/s-1.json"),
        r#"{"sessionId":"s-1","quest":"QEST-1"}"#,
    )
    .unwrap();
    account.log("--a--", "one.jsonl", SESSION, 3 * DAY);

    let done = account.run().unwrap();

    assert_eq!((done.sent, done.indexed), (1, 1));
    let objects = index_objects(&google);
    assert_eq!(objects.len(), 1, "{:?}", google.names());
    assert!(objects[0].ends_with("Z.jsonl.zst.age"), "{objects:?}");
    let sent = rows(&account.open(&google.file(&objects[0])));
    let log = google
        .names()
        .into_iter()
        .find(|name| name.starts_with("laptop/--a--/one."))
        .unwrap();
    assert_eq!(
        sent,
        [json!({
            "name": log,
            "id": "s-1",
            "folder": "/home/a/src/thing",
            "started": "2026-10-01T10:00:00.000Z",
            "ended": "2026-10-01T11:30:00.000Z",
            "title": "Better Name",
            "quest": "QEST-1",
            "models": ["big", "small"],
            "turns": 2,
            "cost": 0.75,
            "bytes": SESSION.len(),
        })]
    );
    let kept = fs::read_to_string(account.root.join("state/index.jsonl")).unwrap();
    assert_eq!(rows(&kept), sent);

    let again = account.run().unwrap();
    assert_eq!(again.indexed, 0);
    assert_eq!(index_objects(&google).len(), 1);
    only_backup_calls(&google);
}

#[test]
fn gives_a_row_to_each_log_sent_before_there_was_an_index() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    account.log("--a--", "one.jsonl", "{\"one\":1}\n", 3 * DAY);
    account.log("--a--", "two.jsonl", SESSION, 4 * DAY);
    account.run().unwrap();
    indexing(&mut account);

    let done = account.run().unwrap();

    assert_eq!((done.sent, done.before, done.indexed), (0, 2, 2));
    let sent = rows(&account.open(&google.file(&index_objects(&google)[0])));
    // Oldest first. A session the harness kept no record of has no quest,
    // and a log that isn't pi's says what it can.
    assert_eq!(sent[0]["id"], "s-1");
    assert_eq!(sent[0]["quest"], Value::Null);
    assert_eq!(sent[1]["id"], Value::Null);
    assert_eq!(sent[1]["turns"], 0);
    only_backup_calls(&google);
}

#[test]
fn leaves_the_rows_for_the_next_run_when_drive_stays_busy() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    account.log("--a--", "one.jsonl", SESSION, 3 * DAY);
    account.run().unwrap();
    indexing(&mut account);
    // The first send and both of its retries.
    google.google.lock().unwrap().fail_sends = 3;

    let stopped = account.run().unwrap();

    assert_eq!(stopped.indexed, 0);
    assert!(stopped.busy.is_some(), "{stopped:?}");
    assert!(
        !account.root.join("state/index.jsonl").exists(),
        "a run Drive kept busy wrote the index"
    );
    assert_eq!(index_objects(&google), [] as [std::string::String; 0]);

    let done = account.run().unwrap();
    assert_eq!(done.indexed, 1);
    assert_eq!(index_objects(&google).len(), 1);
}

#[test]
fn a_log_sent_before_it_is_removed_keeps_its_row() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    keeping(&mut account, 90);
    indexing(&mut account);
    let old = account.log("--a--", "old.jsonl", SESSION, 100 * DAY);

    account.run().unwrap();
    account.run().unwrap();

    assert!(
        !old.exists(),
        "the second run left the old log on the machine"
    );
    let kept = fs::read_to_string(account.root.join("state/index.jsonl")).unwrap();
    assert_eq!(rows(&kept).len(), 1);
}

/// The account with `count` logs sent and then removed from the machine,
/// before it kept an index, the oldest `SESSION`.
fn removed(account: &mut Account, count: usize) {
    keeping(account, 90);
    account.log("--a--", "old-0.jsonl", SESSION, 100 * DAY);
    for at in 1..count {
        let days = 100 - u32::try_from(at).unwrap();
        account.log("--a--", &format!("old-{at}.jsonl"), "{}\n", DAY * days);
    }
    account.run().unwrap();
    assert_eq!(account.run().unwrap().removed, count);
}

/// The files left in the account's state folder.
fn state(account: &Account) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(account.root.join("state"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

#[test]
fn reads_a_log_gone_from_the_machine_back_for_its_row() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    removed(&mut account, 1);
    indexing(&mut account);

    let done = account.run().unwrap();

    assert_eq!((done.sent, done.indexed, done.unread), (0, 1, None));
    let sent = rows(&account.open(&google.file(&index_objects(&google)[0])));
    assert_eq!(sent[0]["id"], "s-1");
    assert_eq!(sent[0]["title"], "Better Name");
    assert_eq!(sent[0]["bytes"], SESSION.len());
    // The copy read back is gone, and nothing else is left beside the ledger.
    assert_eq!(
        state(&account),
        ["index.jsonl", "sent.jsonl", "sent.lock"],
        "only the index is new"
    );
    // An index object is never read back as a log.
    let again = account.run().unwrap();
    assert_eq!(again.indexed, 0);
    assert_eq!(index_objects(&google).len(), 1);
    only_backup_calls(&google);
}

#[test]
fn reads_back_only_as_many_as_the_settings_say_a_run() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    removed(&mut account, 3);
    indexing(&mut account);
    account.config.backfill = 2;

    assert_eq!(account.run().unwrap().indexed, 2);
    assert_eq!(account.run().unwrap().indexed, 1);
    assert_eq!(account.run().unwrap().indexed, 0);
    assert_eq!(index_objects(&google).len(), 2);
}

#[test]
fn carries_on_without_the_rows_no_key_here_can_read() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    removed(&mut account, 1);
    indexing(&mut account);
    let other = account.root.join("other.txt");
    fs::write(
        &other,
        age::x25519::Identity::generate()
            .to_string()
            .expose_secret(),
    )
    .unwrap();
    account.config.identities = vec![other];
    account.log("--a--", "new.jsonl", "new\n", 3 * DAY);

    let done = account.run().unwrap();

    assert!(
        done.unread
            .as_deref()
            .unwrap_or("")
            .contains("no key here opens"),
        "{done:?}"
    );
    // The new log still went, and has its row.
    assert_eq!((done.sent, done.indexed, done.busy), (1, 1, None));
    assert_eq!(state(&account), ["index.jsonl", "sent.jsonl", "sent.lock"]);
}

#[test]
fn reads_nothing_back_when_the_settings_name_no_key() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    removed(&mut account, 1);
    indexing(&mut account);
    account.config.identities = Vec::new();

    let done = account.run().unwrap();

    assert_eq!((done.indexed, done.unread), (0, None));
    assert!(
        !google
            .calls()
            .iter()
            .any(|call| call.starts_with("GET /drive/v3/files/id-")),
        "{:?}",
        google.calls()
    );
}

#[test]
fn reads_where_the_index_and_the_records_are_from_the_settings() {
    let settings = Config::from_json(&json!({
        "sessions": "/s", "ledger": "/l", "folder": "f", "machine": "h",
        "recipients": ["age1someone"], "credentials": "/c",
        "index": "/i.jsonl", "records": "/r", "backfillPerRun": 7,
    }))
    .unwrap();
    assert_eq!(settings.index, Some(PathBuf::from("/i.jsonl")));
    assert_eq!(settings.records, Some(PathBuf::from("/r")));
    assert_eq!(settings.backfill, 7);
}
