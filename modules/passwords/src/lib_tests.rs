//! The tests of the daemon's own logic: the entries held without their
//! secrets, a manager's lists, the search, the asks read from a line,
//! the standings, and what the checks found, kept and read back.

use std::collections::HashMap;

use super::*;
use crate::audit::Health;
use crate::entry::text;

fn login(id: &str, title: &str, version: u64) -> Value {
    json!({
        "id": id, "title": title, "category": "login", "version": version,
        "username": "ada", "websites": [{"address": "https://github.com", "primary": true}],
    })
}

fn detailed(id: &str, version: u64) -> Value {
    json!({
        "id": id, "title": "GitHub", "category": "login", "version": version,
        "details": [
            {"id": "username", "label": "username", "type": "text",
             "purpose": "username", "value": "ada"},
            {"id": "password", "label": "password", "type": "hidden",
             "purpose": "password", "value": "hunter2"},
            {"id": "otp", "label": "one-time password", "type": "code",
             "value": "otpauth://totp/x?secret=ABC"},
            {"id": "pin", "label": "PIN", "type": "text", "hidden": true, "value": "1234"},
        ],
    })
}

#[test]
fn a_hidden_detail_never_keeps_its_value() {
    let entry = Entry::without_secrets(&detailed("a", 1)).unwrap();
    let said = serde_json::to_string(&entry.record("1password", 0)).unwrap();
    for secret in ["hunter2", "otpauth", "1234"] {
        assert!(!said.contains(secret), "{secret} was kept: {said}");
    }
    assert!(
        said.contains("\"value\":\"ada\""),
        "the username went too: {said}"
    );
    let password = entry.detail(None).unwrap();
    assert_eq!(password.get("hidden"), Some(&Value::Bool(true)));
    assert_eq!(text(password, "id"), Some("password"));
    assert_eq!(entry.code().and_then(|code| text(code, "id")), Some("otp"));
}

#[test]
fn the_password_is_the_hidden_detail_when_none_says_its_purpose() {
    let entry = Entry::without_secrets(&json!({
        "id": "c", "details": [
            {"id": "number", "type": "text", "value": "4111"},
            {"id": "otp", "type": "code"},
            {"id": "cvv", "type": "hidden"},
        ],
    }))
    .unwrap();
    assert_eq!(
        entry.detail(None).and_then(|detail| text(detail, "id")),
        Some("cvv")
    );
    assert_eq!(
        entry.detail(Some("number")).and_then(|d| text(d, "id")),
        Some("number")
    );
}

#[test]
fn an_entry_without_an_id_is_never_held() {
    assert_eq!(Entry::without_secrets(&json!({"title": "No one"})), None);
    assert_eq!(Entry::without_secrets(&json!({"id": ""})), None);
    assert_eq!(Entry::without_secrets(&json!("text")), None);
}

#[test]
fn a_list_says_what_changed_and_what_went() {
    let mut held = Held::of(&["1password".to_owned()]);
    let first = held.list(
        "1password",
        &[login("a", "GitHub", 1), login("b", "Bank", 1)],
        5,
    );
    assert_eq!(first.changed, ["entry:1password:a", "entry:1password:b"]);
    assert_eq!(first.gone, Vec::<String>::new());
    let again = held.list("1password", &[login("a", "GitHub", 1)], 9);
    assert!(again.changed.is_empty(), "{again:?}");
    assert_eq!(again.gone, ["entry:1password:b"]);
    assert!(
        held.listed("1password"),
        "a manager that listed isn't listed"
    );
}

#[test]
fn a_list_keeps_an_entry_read_in_full_until_its_version_moves() {
    let mut held = Held::of(&["1password".to_owned()]);
    held.list("1password", &[login("a", "GitHub", 1)], 1);
    assert_eq!(
        held.entry("1password", &detailed("a", 1), 2).as_deref(),
        Some("entry:1password:a")
    );
    held.list("1password", &[login("a", "GitHub", 1)], 3);
    assert!(
        held.get("entry:1password:a").unwrap().detailed(),
        "a list of the same version dropped what was read in full"
    );
    let moved = held.list("1password", &[login("a", "GitHub", 2)], 4);
    assert_eq!(moved.changed, ["entry:1password:a"]);
    assert!(
        !held.get("entry:1password:a").unwrap().detailed(),
        "a new version kept what was read of the old in full"
    );
}

#[test]
fn a_search_finds_titles_first_then_anything_else() {
    let mut held = Held::of(&["1password".to_owned()]);
    held.list(
        "1password",
        &[
            login("a", "Work Mail", 1),
            login("b", "GitHub", 1),
            login("c", "Bank", 1),
        ],
        1,
    );
    let found: Vec<String> = held
        .search("git", 10)
        .iter()
        .map(|record| record["key"].as_str().unwrap().to_owned())
        .collect();
    // Every login here has GitHub's address, but only one is titled so.
    assert_eq!(found[0], "entry:1password:b");
    assert_eq!(found.len(), 3);
    assert_eq!(held.search("hub", 10), Vec::<Value>::new());
    assert_eq!(held.search("", 10), Vec::<Value>::new());
    assert_eq!(held.search("git", 1).len(), 1);
}

#[test]
fn asks_are_read_with_their_defaults() {
    assert_eq!(
        ask(r#"{"id":1,"ask":"list","asked":true}"#),
        Ask::List {
            id: json!(1),
            limit: None,
            fresh: false,
            asked: true,
            held: false,
            patience: PATIENCE,
            of: Of::Entries
        }
    );
    assert_eq!(
        ask(r#"{"id":7,"ask":"watch","kind":"row"}"#),
        Ask::Watch {
            id: json!(7),
            of: Of::Rows
        }
    );
    assert_eq!(
        ask(r#"{"id":9,"ask":"watch","kind":"drawer",
                "keys":["drawer:passwords:entry:onepassword:a",
                         "row:passwords:entry:onepassword:b"]}"#),
        Ask::Watch {
            id: json!(9),
            of: Of::Drawers(vec!["drawer:passwords:entry:onepassword:a".to_owned()])
        },
        "a drawer watch kept a key that names no drawer"
    );
    assert_eq!(
        ask(r#"{"id":10,"ask":"search","kind":"drawer"}"#),
        Ask::UnknownKind {
            id: json!(10),
            kind: "drawer".to_owned()
        }
    );
    assert_eq!(
        ask(r#"{"id":8,"ask":"search","kind":"row"}"#),
        Ask::UnknownKind {
            id: json!(8),
            kind: "row".to_owned()
        }
    );
    assert!(
        matches!(
            ask(r#"{"id":6,"ask":"list","held":true}"#),
            Ask::List {
                held: true,
                asked: false,
                ..
            }
        ),
        "a list of what's held wasn't read as one, unasked"
    );
    assert_eq!(
        ask(r#"{"id":2,"ask":"copy","key":"entry:1password:a"}"#),
        Ask::Copy {
            id: json!(2),
            key: "entry:1password:a".to_owned(),
            detail: None
        }
    );
    assert!(
        matches!(
            ask(r#"{"id":3,"ask":"reveal","key":"person:ada"}"#),
            Ask::Unknown { .. }
        ),
        "a reveal, which no client may ask, wasn't unknown"
    );
    assert!(
        matches!(
            ask(r#"{"id":4,"ask":"list","kind":"person"}"#),
            Ask::UnknownKind { .. }
        ),
        "a list of a kind this doesn't keep wasn't refused for its kind"
    );
    assert!(
        matches!(ask("not json"), Ask::Unknown { .. }),
        "a line that isn't JSON wasn't unknown"
    );
    assert_eq!(
        ask(r#"{"id":5,"ask":"get","keys":["entry:1password:a","nope"]}"#),
        Ask::Get {
            id: json!(5),
            keys: vec!["entry:1password:a".to_owned()],
            asked: false,
            held: false,
            patience: PATIENCE
        }
    );
}

#[test]
fn the_launcher_watches_entries_and_asks_in_one() {
    // The Passwords place and its asks share one connection, so the
    // launcher watches the entries with the asks' rows, never the
    // entries' own rows, which only the passwords place's rows draw.
    for kinds in [r#"["entry","row"]"#, r#"["row","entry"]"#] {
        assert_eq!(
            ask(&format!(
                r#"{{"id":"all","ask":"watch","kind":{kinds},"place":"asks"}}"#
            )),
            Ask::Watch {
                id: json!("all"),
                of: Of::EntriesAndAsks
            },
            "the kinds {kinds} with the asks' place weren't one watch"
        );
    }
    assert!(
        matches!(
            ask(r#"{"id":1,"ask":"list","kind":["entry","row"],"place":"asks"}"#),
            Ask::List {
                of: Of::EntriesAndAsks,
                ..
            }
        ),
        "a list of the entries and the asks wasn't one list"
    );
    assert!(
        matches!(
            ask(r#"{"id":2,"ask":"watch","kind":["entry","row"]}"#),
            Ask::UnknownKind { .. }
        ),
        "entries with every row were watched as entries alone"
    );
    assert!(
        matches!(
            ask(r#"{"id":3,"ask":"watch","kind":["entry","drawer"],"place":"asks"}"#),
            Ask::UnknownKind { .. }
        ),
        "entries with drawers were watched though no place keeps them together"
    );
    assert!(
        matches!(
            ask(r#"{"id":4,"ask":"search","place":"asks","words":"deploy"}"#),
            Ask::Unknown { .. }
        ),
        "a search of the asks was answered with entries"
    );
}

#[test]
fn a_manager_says_answers_and_standings() {
    assert_eq!(
        said(r#"{"state":"signed-out","reason":"Let the launcher in"}"#),
        Said::Stands(Standing::SignedOut("Let the launcher in".to_owned()))
    );
    let Said::Answer { id, answer } = said(r#"{"id":7,"entries":[]}"#) else {
        panic!("an answer wasn't read as one");
    };
    assert_eq!(id, 7);
    assert!(
        answer.contains_key("entries") && !answer.contains_key("id"),
        "an answer should keep its entries and lose its id: {answer:?}"
    );
    assert_eq!(said("{}"), Said::Other);
}

#[test]
fn the_daemon_answers_once_any_manager_does() {
    let mut held = Held::of(&["a".to_owned(), "b".to_owned()]);
    assert_eq!(held.state(), "starting");
    held.stand("a", Standing::SignedOut("Let it in".to_owned()));
    assert_eq!(held.state(), "signed-out");
    held.stand("b", Standing::Answering);
    assert_eq!(held.state(), "answering");
    assert_eq!(held.standings()["a"]["state"], "signed-out");
}

#[test]
fn an_audit_is_asked_only_as_joels_own() {
    assert_eq!(
        ask(r#"{"id":1,"ask":"audit","asked":true}"#),
        Ask::Audit {
            id: json!(1),
            asked: true
        }
    );
    assert!(
        matches!(
            ask(r#"{"id":2,"ask":"audit"}"#),
            Ask::Audit { asked: false, .. }
        ),
        "an audit not said to be Joel's was read as his"
    );
}

#[test]
fn an_entrys_health_is_kept_until_its_version_moves() {
    let mut held = Held::of(&["1password".to_owned()]);
    held.list(
        "1password",
        &[login("a", "GitHub", 1), login("b", "Bank", 1)],
        1,
    );
    let found = HashMap::from([
        (
            "entry:1password:a".to_owned(),
            Health {
                reused: 1,
                breached: Some(3),
            },
        ),
        (
            "entry:1password:b".to_owned(),
            Health {
                reused: 1,
                breached: None,
            },
        ),
    ]);
    let changed = held.audited(&["1password".to_owned()], found, 10, true);
    assert_eq!(changed, ["entry:1password:a", "entry:1password:b"]);
    let a = held.record("entry:1password:a").unwrap();
    assert_eq!(a["health"], json!({"reused": 1, "breached": 3}));
    assert_eq!(held.records()[1]["health"], json!({"reused": 1}));
    // Listed again unchanged, it keeps its health; its version moved,
    // it may hold another password, so it holds none.
    held.list(
        "1password",
        &[login("a", "GitHub", 1), login("b", "Bank", 2)],
        2,
    );
    assert_eq!(
        held.health("entry:1password:a"),
        Some(&Health {
            reused: 1,
            breached: Some(3),
        })
    );
    assert_eq!(
        held.record("entry:1password:b").unwrap().get("health"),
        None
    );
    // An audit finding nothing of an entry takes what the last found.
    let changed = held.audited(&["1password".to_owned()], HashMap::new(), 20, true);
    assert_eq!(changed, ["entry:1password:a"]);
    assert_eq!(held.health("entry:1password:a"), None);
}

fn troubled(reused: u64, breached: Option<u64>) -> Health {
    Health { reused, breached }
}

#[test]
fn what_checks_found_starts_holds_and_clears() {
    let managers = ["1password".to_owned()];
    let mut held = Held::of(&managers);
    held.list("1password", &[login("a", "GitHub", 1)], 1);
    assert_eq!(held.checked()["at"], Value::Null);
    let a = "entry:1password:a".to_owned();
    held.audited(
        &managers,
        HashMap::from([(a.clone(), troubled(0, Some(0)))]),
        5,
        true,
    );
    // A check finding nothing before any found trouble clears nothing.
    assert_eq!(
        held.checked(),
        json!({"at": 5, "since": null, "cleared": null,
               "breached": 0, "reused": 0, "troubled": 0})
    );
    held.audited(
        &managers,
        HashMap::from([(a.clone(), troubled(1, Some(3)))]),
        10,
        true,
    );
    held.audited(
        &managers,
        HashMap::from([(a.clone(), troubled(1, Some(0)))]),
        20,
        true,
    );
    // Trouble found at 10 is still what's been found since, at 20.
    assert_eq!(
        held.checked(),
        json!({"at": 20, "since": 10, "cleared": null,
               "breached": 0, "reused": 1, "troubled": 1})
    );
    // One that couldn't ask about breaches can't say none is breached.
    held.audited(
        &managers,
        HashMap::from([(a.clone(), troubled(0, None))]),
        30,
        false,
    );
    assert_eq!(held.checked()["since"], 10);
    held.audited(
        &managers,
        HashMap::from([(a, troubled(0, Some(0)))]),
        40,
        true,
    );
    assert_eq!(held.checked()["since"], Value::Null);
    assert_eq!(held.checked()["cleared"], 40);
}

#[test]
fn what_checks_found_is_kept_without_a_secret_and_read_back() {
    let managers = ["1password".to_owned()];
    let mut held = Held::of(&managers);
    held.list(
        "1password",
        &[login("a", "GitHub", 1), login("b", "Bank", 1)],
        1,
    );
    assert!(held.unsaved().is_none(), "nothing was checked yet");
    let found = HashMap::from([
        ("entry:1password:a".to_owned(), troubled(2, Some(1204))),
        ("entry:1password:b".to_owned(), troubled(0, Some(0))),
    ]);
    held.audited(&managers, found, 10, true);
    let kept = held.unsaved().expect("a check is kept");
    // Only the troubled entry, by its two counts.
    assert_eq!(
        kept,
        json!({"version": 1, "at": 10, "since": 10, "cleared": null,
               "health": {"entry:1password:a": {"reused": 2, "breached": 1204}}})
    );
    assert!(held.unsaved().is_none(), "it was kept already");
    // A daemon starting again reads it, and dresses the entry once its
    // manager lists it; one of a manager no longer named it leaves out.
    let mut again = Held::of(&managers);
    let mut with_another = kept.clone();
    with_another["health"]["entry:gone:z"] = json!({"reused": 1});
    assert!(again.load(&with_another), "what was kept wasn't read back");
    assert_eq!(again.checked()["troubled"], 1);
    again.list("1password", &[login("a", "GitHub", 1)], 2);
    assert_eq!(
        again.record("entry:1password:a").unwrap()["health"],
        json!({"reused": 2, "breached": 1204})
    );
    // A list no longer giving it lets it go, and says it's to be kept.
    again.list("1password", &[login("b", "Bank", 1)], 3);
    assert_eq!(again.checked()["troubled"], 0);
    assert_eq!(again.unsaved().unwrap()["health"], json!({}));
    assert!(
        !again.load(&json!({"version": 2})),
        "a version this doesn't write was read"
    );
}

#[test]
fn keys_name_their_manager_and_id() {
    assert_eq!(named(&key("1password", "abc")), Some(("1password", "abc")));
    assert_eq!(named("entry:1password:"), None);
    assert_eq!(named("person:ada"), None);
}
