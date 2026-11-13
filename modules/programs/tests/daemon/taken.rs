//! Capture's Recent's rows over the socket: each thing taken is a row as
//! it lands, its drawer whole once the shell holds it ahead, and a watch
//! of another place is told none of them; and what's typed inside Recent
//! finds among everything taken, past the nine it lists.

use std::time::{Duration, SystemTime};

use serde_json::json;

use super::Running;

#[test]
fn captures_rows_are_served_as_things_are_taken_and_whole_once_ahead() {
    let running = Running::new("taken-rows");
    let mut client = running.client();
    let shots = running.path("shots").to_string_lossy().into_owned();
    let first = format!("row:taken:{shots}/2026-11-10 09-00-00.png");
    client.ask(&json!({"id": "settings", "ask": "settings", "folders": [shots]}));
    client.ask(&json!({"id": 1, "ask": "watch", "kind": "row", "place": "taken"}));
    let row = client.changed(&first);
    assert_eq!(
        row["row"]["name"], "2026-11-10 09-00-00.png",
        "a shot's row: {row}"
    );
    assert_eq!(
        row["row"]["side"][1],
        json!({"said": "~/shots", "keep": 1}),
        "a folder that isn't the usual one is said: {row}"
    );
    assert_eq!(
        row["row"]["drawer"]["facts"],
        json!([
            {"name": "Taken", "at": row["row"]["held"]["at"], "as": "moment"},
            {"name": "Size", "said": "4\u{a0}B"}
        ]),
        "not ahead, its drawer says only what the list does: {row}"
    );
    // A programs watch hears only Programs' rows.
    let programs =
        client.ask(&json!({"id": 2, "ask": "watch", "kind": "row", "place": "programs"}));
    assert!(
        programs["records"]
            .as_array()
            .is_some_and(|records| records.iter().all(|record| record["key"] != first.as_str())),
        "a watch of another place was told Capture's rows: {programs}"
    );
    std::fs::write(running.path("shots/2026-11-11 01-00-00.png"), "shot").unwrap();
    let taken = client.changed(&format!("row:taken:{shots}/2026-11-11 01-00-00.png"));
    assert!(
        taken["row"]["order"].as_i64() < row["row"]["order"].as_i64(),
        "a thing just taken doesn't lead: {taken}"
    );
    client.send(&json!({"id": "ahead", "ask": "watch", "kind": ["row"], "keys": [first]}));
    let whole = client.changed(&first);
    assert_eq!(
        whole["row"]["drawer"]["facts"][2],
        json!({"name": "Dimensions", "said": "4\u{a0}\u{d7}\u{a0}3"}),
        "ahead, its drawer doesn't say what file-preview did: {whole}"
    );
}

#[test]
fn what_was_taken_is_searched_newest_first_past_recents_nine() {
    let running = Running::new("taken-search");
    let mut client = running.client();
    let shots = running.path("shots").to_string_lossy().into_owned();
    let key = |at: u64| format!("row:taken:{shots}/zz {at:02}.png");
    // Eleven shots a second apart, each made apart and moved in, so the
    // list never reads one before its time is set.
    let apart = running.path("apart");
    std::fs::create_dir_all(&apart).unwrap();
    let since = SystemTime::UNIX_EPOCH + Duration::from_hours(500_000);
    for at in 1..=11 {
        let file = apart.join(format!("zz {at:02}.png"));
        std::fs::write(&file, "shot").unwrap();
        let opened = std::fs::File::options().write(true).open(&file).unwrap();
        opened
            .set_modified(since + Duration::from_secs(at))
            .unwrap();
    }
    client.ask(&json!({"id": "settings", "ask": "settings", "folders": [shots]}));
    client.ask(&json!({"id": 1, "ask": "watch", "kind": "row", "place": "taken"}));
    for at in 1..=11 {
        let name = format!("zz {at:02}.png");
        std::fs::rename(apart.join(&name), running.path(&format!("shots/{name}"))).unwrap();
    }
    client.changed(&key(11));
    let search = |client: &mut daemon_client::Client, id: u64, words: &str, most: u64| {
        client.ask(&json!({
            "id": id, "ask": "search", "place": "taken", "words": words, "most": most,
        }))
    };
    let said = search(&mut client, 2, "zz", 50);
    assert_eq!(
        said["keys"],
        json!((1..=11).rev().map(&key).collect::<Vec<_>>()),
        "not every shot by its name, newest first: {said}"
    );
    assert_eq!(said["marks"][key(11)], json!([[0, 2]]), "{said}");
    let rows = said["rows"]
        .as_object()
        .expect("an answer carries the rows found");
    assert_eq!(
        rows.keys().cloned().collect::<Vec<_>>(),
        [key(1), key(2)],
        "only the two past Recent's nine come with their rows: {said}"
    );
    assert!(
        rows[&key(1)]["order"].is_null(),
        "a row found is placed by the answer's keys alone: {said}"
    );
    let said = search(&mut client, 3, "zz", 3);
    assert_eq!(
        said["keys"],
        json!([key(11), key(10), key(9)]),
        "the search doesn't stop at its most: {said}"
    );
    let said = search(&mut client, 4, "zz 2026", 50);
    assert_eq!(said["keys"], json!([]), "every word must find: {said}");
    let said = search(&mut client, 5, "  ", 50);
    assert_eq!(
        said["keys"],
        json!([]),
        "nothing typed finds nothing: {said}"
    );
    // Ahead, a shot found past the nine is served whole, still unplaced:
    // first as it's held, which may be before file-preview's answer is, as
    // it was on a builder at 20:10 on 12 November, then with that answer.
    client.send(&json!({"id": "ahead", "ask": "watch", "kind": ["row"], "keys": [key(1)]}));
    let whole = loop {
        let row = client.changed(&key(1));
        if row["row"]["drawer"]["facts"][2]["name"] == "Dimensions" {
            break row;
        }
    };
    assert!(
        whole["row"]["order"].is_null(),
        "ahead, a found shot joined Recent's list: {whole}"
    );
    let said = search(&mut client, 6, "zz", 50);
    assert!(
        said["rows"].get(key(1)).is_none(),
        "a found shot served since came again in the answer: {said}"
    );
}
