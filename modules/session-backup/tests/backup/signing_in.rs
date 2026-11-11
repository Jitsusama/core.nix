//! Signing in: a token Drive stops taking is fetched again, and a login that
//! can't reach Drive, names no quota project or isn't there is said plainly.

use std::fs;

use serde_json::{Value, json};

use super::{Account, DAY, StandIn, only_backup_calls, owning};

#[test]
fn signs_in_again_when_drive_stops_taking_the_token() {
    let google = StandIn::start();
    let account = Account::new(&google);
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    google.google.lock().unwrap().expire = 1;

    let done = account.run().unwrap();

    assert_eq!(done.sent, 1);
    let tokens = google
        .calls()
        .iter()
        .filter(|call| call.as_str() == "POST /token")
        .count();
    assert_eq!(tokens, 2);
    only_backup_calls(&google);
}

#[test]
fn says_how_to_sign_in_when_the_login_may_not_use_drive() {
    let google = StandIn::start();
    let account = Account::new(&google);
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    google.google.lock().unwrap().no_drive_scope = true;

    let why = account.run().unwrap_err();

    assert!(
        why.contains("may not use Drive") && why.contains("https://www.googleapis.com/auth/drive"),
        "{why}"
    );
    assert_eq!(google.names(), [] as [std::string::String; 0]);
}

#[test]
fn names_the_logins_quota_project_on_every_drive_call() {
    let google = StandIn::start();
    let account = owning(&google, "pi sessions");
    let credentials = account.root.join("adc.json");
    let mut login: Value =
        serde_json::from_str(&fs::read_to_string(&credentials).unwrap()).unwrap();
    login["quota_project_id"] = json!("the-project");
    fs::write(&credentials, login.to_string()).unwrap();
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    google.google.lock().unwrap().needs_project = true;

    assert_eq!(account.run().unwrap().sent, 1);

    let projects = google.google.lock().unwrap().projects.clone();
    assert_ne!(projects, Vec::<Option<String>>::new());
    assert!(
        projects
            .iter()
            .all(|project| project.as_deref() == Some("the-project")),
        "{projects:?}"
    );
    only_backup_calls(&google);
}

#[test]
fn says_how_to_name_a_quota_project_when_drive_asks_for_one() {
    let google = StandIn::start();
    let account = owning(&google, "pi sessions");
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    google.google.lock().unwrap().needs_project = true;

    let why = account.run().unwrap_err();

    assert!(why.contains("set-quota-project"), "{why}");
    assert_eq!(google.names(), Vec::<String>::new());
}

#[test]
fn says_how_to_sign_in_when_there_is_no_login() {
    let google = StandIn::start();
    let account = Account::new(&google);
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    fs::remove_file(account.root.join("adc.json")).unwrap();

    let why = account.run().unwrap_err();

    assert!(
        why.contains("gcloud auth application-default login")
            && why.contains("https://www.googleapis.com/auth/drive"),
        "{why}"
    );
    assert_eq!(google.calls(), [] as [std::string::String; 0]);
}
