//! Asks heard over Passwords' own connection: where the passwords daemon
//! answers secret asks, one watch of entries and asks rows hears both, and
//! never Passwords' own rows, and the yes an ask's row draws releases there.

use serde_json::{Value, json};

use super::{LISTED, answer, nowhere, put_up, saying, stored_bash, yes};
use crate::common::Running;

#[test]
fn one_watch_hears_the_entries_and_the_asks_but_not_the_entries_rows() {
    // The Passwords place and its asks share this one watch where asks
    // are on, so the asks' rows come with the entries and the yes they
    // draw reaches the same daemon, with no second connection.
    let running = Running::releasing("asks-beside-entries", &[LISTED]);
    let id = "e123456789abcdef0123456789abcdef";
    let bash = stored_bash();
    let argv = saying(&bash);
    put_up(
        &running,
        id,
        LISTED,
        &argv.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    // Another watch of rows, as the launcher's other places keep, so the
    // entries' own rows are made and held before the asks' watch opens,
    // and it would hear them were they not kept from it. Both watches
    // are told over the daemon's one channel, so each reads only the
    // lines bearing its own id.
    let mut client = running.client();
    client.say(&json!({"id": "rows", "ask": "watch", "kind": "row"}));
    let listed = client.ask(&json!({"id": 1, "ask": "list", "asked": true}));
    assert!(
        listed.get("error").is_none(),
        "the list was refused: {listed}"
    );
    loop {
        let heard = client.hear();
        if heard["id"] != "rows" {
            continue;
        }
        let records = heard["records"].as_array().or(heard["changed"].as_array());
        if records.into_iter().flatten().any(|record| {
            record["key"]
                .as_str()
                .is_some_and(|key| key.starts_with("row:passwords:"))
        }) {
            break;
        }
    }
    client.say(&json!({"id": "all", "ask": "watch", "kind": ["entry", "row"], "place": "asks"}));
    let mut first = client.hear();
    while first["id"] == "rows" {
        first = client.hear();
    }
    assert_eq!(
        first["id"], "all",
        "the watch wasn't answered first: {first}"
    );
    assert!(
        first.get("error").is_none(),
        "the watch was refused: {first}"
    );
    let listed = client.ask(&json!({"id": 2, "ask": "list", "asked": true}));
    assert!(
        listed.get("error").is_none(),
        "the list was refused: {listed}"
    );
    let mut keys: Vec<String> = Vec::new();
    let mut row = Value::Null;
    let mut summary = Value::Null;
    let mut heard = first;
    loop {
        let records = heard["records"].as_array().or(heard["changed"].as_array());
        for record in records.into_iter().flatten() {
            let key = record["key"].as_str().unwrap_or_default().to_owned();
            if key == format!("row:asks:{id}") {
                row = record["row"].clone();
            }
            if key == "summary:asks" {
                summary = record["summary"].clone();
            }
            keys.push(key);
        }
        // The asks' summary comes too, so the place counts the asks that
        // wait rather than every one drawn.
        if !row.is_null() && !summary.is_null() && keys.iter().any(|key| key.starts_with("entry:"))
        {
            break;
        }
        heard = client.hear();
        while heard["id"] == "rows" {
            heard = client.hear();
        }
    }
    assert!(
        !keys.iter().any(|key| key.starts_with("row:passwords:")),
        "the asks' watch heard the entries' own rows: {keys:?}"
    );
    let said = client.ask(&yes(&row, 812));
    assert_eq!(said["releasing"], true, "the yes: {said}");
    let answer = answer(&running, id);
    assert_eq!(answer["state"], "ran", "the answer: {answer}");
    nowhere(&running);
}
