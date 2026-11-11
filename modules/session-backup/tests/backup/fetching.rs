//! Fetching: a log gone from the machine brought back as it was, found by name
//! without the ledger, and never written over a file already there.

use std::fs;

use age::secrecy::ExposeSecret;

use super::{Account, DAY, StandIn, keeping, only_backup_calls, only_name};

#[test]
fn fetches_a_log_gone_from_the_machine_as_it_was() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    keeping(&mut account, 90);
    let log = account.log("--a--", "old.jsonl", "{\"one\":1}\n", 100 * DAY);
    account.run().unwrap();
    account.run().unwrap();
    assert!(
        !log.exists(),
        "the second run left the old log on the machine"
    );
    let before = google.calls().len();

    let bytes = account.fetch(&only_name(&google), "back.jsonl").unwrap();

    assert_eq!(bytes, 10);
    assert_eq!(
        fs::read_to_string(account.root.join("back.jsonl")).unwrap(),
        "{\"one\":1}\n"
    );
    // The ledger says where the copy is, so Drive is asked only for it.
    assert_eq!(
        google.calls()[before..],
        ["POST /token", "GET /drive/v3/files/id-0"]
    );
    assert_eq!(
        fs::read_dir(&account.root).unwrap().count(),
        5,
        "only the fetched log is new"
    );
    only_backup_calls(&google);
}

#[test]
fn finds_a_copy_by_name_when_the_ledger_is_gone() {
    let google = StandIn::start();
    let account = Account::new(&google);
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    account.run().unwrap();
    // A machine rebuilt from nothing has no ledger.
    fs::remove_file(account.root.join("state/sent.jsonl")).unwrap();

    account.fetch(&only_name(&google), "back.jsonl").unwrap();

    assert_eq!(
        fs::read_to_string(account.root.join("back.jsonl")).unwrap(),
        "one\n"
    );
    only_backup_calls(&google);
}

#[test]
fn fetches_again_when_drive_is_busy_for_a_moment() {
    let google = StandIn::start();
    let account = Account::new(&google);
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    account.run().unwrap();
    google.google.lock().unwrap().fail_downloads = 2;

    account.fetch(&only_name(&google), "back.jsonl").unwrap();

    assert_eq!(
        fs::read_to_string(account.root.join("back.jsonl")).unwrap(),
        "one\n"
    );
}

#[test]
fn says_when_drive_holds_no_copy_of_that_name() {
    let google = StandIn::start();
    let account = Account::new(&google);

    let why = account
        .fetch("laptop/--a--/none.jsonl.zst.age", "back.jsonl")
        .unwrap_err();

    assert!(
        why.contains("holds no copy named laptop/--a--/none"),
        "{why}"
    );
    assert!(
        !account.root.join("back.jsonl").exists(),
        "a fetch of no copy wrote a file"
    );
    only_backup_calls(&google);
}

#[test]
fn leaves_nothing_behind_when_no_key_here_opens_the_copy() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    account.run().unwrap();
    let other = account.root.join("other.txt");
    let stranger = age::x25519::Identity::generate();
    fs::write(&other, stranger.to_string().expose_secret()).unwrap();
    account.config.identities = vec![other];
    let listed = fs::read_dir(&account.root).unwrap().count();

    let why = account
        .fetch(&only_name(&google), "back.jsonl")
        .unwrap_err();

    assert!(why.contains("no key here opens"), "{why}");
    assert_eq!(fs::read_dir(&account.root).unwrap().count(), listed);
}

#[test]
fn says_when_the_settings_name_no_key_this_machine_has() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    account.run().unwrap();
    account.config.identities = vec![account.root.join("absent.txt")];

    let why = account
        .fetch(&only_name(&google), "back.jsonl")
        .unwrap_err();

    assert!(
        why.contains("none of the keys the settings name is on this machine"),
        "{why}"
    );
    assert!(
        !account.root.join("back.jsonl").exists(),
        "a fetch no key opens wrote a file"
    );
}

#[test]
fn says_to_name_a_key_when_the_settings_name_none() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    account.run().unwrap();
    account.config.identities = Vec::new();

    let why = account
        .fetch(&only_name(&google), "back.jsonl")
        .unwrap_err();

    assert!(
        why.contains("name no key") && why.contains("identities"),
        "{why}"
    );
    // Nothing was downloaded to find that out.
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
fn never_writes_over_a_file_already_there() {
    let google = StandIn::start();
    let account = Account::new(&google);
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    account.run().unwrap();
    fs::write(account.root.join("back.jsonl"), "mine").unwrap();

    let why = account
        .fetch(&only_name(&google), "back.jsonl")
        .unwrap_err();

    assert!(why.contains("already there"), "{why}");
    assert_eq!(
        fs::read_to_string(account.root.join("back.jsonl")).unwrap(),
        "mine"
    );
}
