//! What the machine keeps: an old log removed once Drive holds it, kept when
//! Drive no longer finds it, and how long to keep read from the settings.

use serde_json::{Value, json};
use session_backup::config::Config;

use super::{Account, DAY, StandIn, keeping, only_backup_calls};

#[test]
fn removes_an_old_log_from_the_machine_once_drive_holds_it() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    keeping(&mut account, 90);
    let old = account.log("--a--", "old.jsonl", "old\n", 100 * DAY);
    let recent = account.log("--a--", "recent.jsonl", "recent\n", 10 * DAY);

    // The run that sends a log never removes it: the next one does, once
    // it has asked Drive for the copy.
    let sent = account.run().unwrap();
    assert_eq!((sent.sent, sent.removed), (2, 0));
    assert!(old.exists(), "the run that sent the old log removed it");

    let done = account.run().unwrap();

    assert_eq!((done.removed, done.freed), (1, 4));
    assert!(!old.exists(), "the old log Drive holds was kept");
    assert!(recent.exists(), "a log younger than the keep was removed");
    assert_eq!(google.names().len(), 2);
    only_backup_calls(&google);
}

#[test]
fn keeps_an_old_log_drive_no_longer_finds() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    keeping(&mut account, 90);
    let old = account.log("--a--", "old.jsonl", "old\n", 100 * DAY);
    account.run().unwrap();
    // Someone removed the copy from Drive, so the log is the only one.
    google.google.lock().unwrap().files.clear();

    let done = account.run().unwrap();

    assert_eq!(done.removed, 0);
    assert!(old.exists(), "the only copy of the log was removed");
    only_backup_calls(&google);
}

#[test]
fn keeps_every_log_when_the_settings_say_no_keep() {
    let google = StandIn::start();
    let account = Account::new(&google);
    let old = account.log("--a--", "old.jsonl", "old\n", 400 * DAY);
    account.run().unwrap();

    let done = account.run().unwrap();

    assert_eq!(done.removed, 0);
    assert!(old.exists(), "a log was removed with no keep set");
}

#[test]
fn reads_keep_days_from_the_settings() {
    let settings = |keep: Value| {
        Config::from_json(&json!({
            "sessions": "/s", "ledger": "/l", "folder": "f", "machine": "h",
            "recipients": ["age1someone"], "credentials": "/c", "keepDays": keep,
        }))
        .unwrap()
        .keep
    };
    assert_eq!(settings(json!(90)), Some(90 * DAY));
    assert_eq!(settings(Value::Null), None);
    assert_eq!(settings(json!(0)), None);
}
