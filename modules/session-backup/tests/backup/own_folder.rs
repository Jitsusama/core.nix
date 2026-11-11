//! A backup into a folder of its own: the folder made once at the top of Drive
//! and sent into, and found again rather than made twice.

use std::fs;

use serde_json::json;
use session_backup::config::Config;

use super::{DAY, FOLDER, StandIn, only_backup_calls, only_name, owning};

fn folders_made(google: &StandIn) -> usize {
    google
        .calls()
        .iter()
        .filter(|call| *call == "POST /drive/v3/files")
        .count()
}

#[test]
fn makes_a_folder_of_its_own_once_and_sends_into_it() {
    let google = StandIn::start();
    let account = owning(&google, "pi sessions");
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);

    assert_eq!(account.run().unwrap().sent, 1);
    account.log("--a--", "two.jsonl", "two\n", 3 * DAY);
    assert_eq!(account.run().unwrap().sent, 1);

    assert_eq!(google.google.lock().unwrap().folders, vec!["pi sessions"]);
    assert_eq!(folders_made(&google), 1);
    assert_eq!(google.names().len(), 2);
    only_backup_calls(&google);
}

#[test]
fn fetches_from_a_folder_of_its_own_without_making_another() {
    let google = StandIn::start();
    let account = owning(&google, "pi sessions");
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    account.run().unwrap();
    let name = only_name(&google);
    fs::remove_file(account.root.join("state/sent.jsonl")).unwrap();

    assert_eq!(account.fetch(&name, "back.jsonl").unwrap(), 4);

    assert_eq!(
        fs::read_to_string(account.root.join("back.jsonl")).unwrap(),
        "one\n"
    );
    assert_eq!(folders_made(&google), 1);
    only_backup_calls(&google);
}

#[test]
fn says_how_to_sign_in_for_a_folder_of_its_own() {
    let google = StandIn::start();
    let account = owning(&google, "pi sessions");
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    fs::remove_file(account.root.join("adc.json")).unwrap();

    let why = account.run().unwrap_err();

    assert!(
        why.contains("CLOUDSDK_CONFIG")
            && why.contains("https://www.googleapis.com/auth/drive.file"),
        "{why}"
    );
    assert_eq!(google.calls(), Vec::<String>::new());
}

#[test]
fn settings_name_one_folder() {
    let both = Config::from_json(&json!({
        "sessions": "/s", "ledger": "/l", "machine": "laptop", "credentials": "/c",
        "recipients": ["age1someone"], "folder": FOLDER, "ownFolder": "pi sessions",
    }));
    let neither = Config::from_json(&json!({
        "sessions": "/s", "ledger": "/l", "machine": "laptop", "credentials": "/c",
        "recipients": ["age1someone"],
    }));

    let both = both.unwrap_err();
    assert!(both.contains("name one"), "{both}");
    let neither = neither.unwrap_err();
    assert!(neither.contains("no folder"), "{neither}");
}
