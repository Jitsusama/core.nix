//! Capture's Recent's rows over the socket: each thing taken is a row as
//! it lands, its drawer whole once the shell holds it ahead, and a watch
//! of another place is told none of them.

use serde_json::json;

use super::Running;

#[test]
fn captures_rows_are_served_as_things_are_taken_and_whole_once_ahead() {
    let running = Running::new("taken-rows");
    let mut client = running.client();
    let shots = running.path("shots").to_string_lossy().into_owned();
    let first = format!("row:taken:{shots}/2026-11-10 09-00-00.png");
    client.ask(&json!({
        "id": 1, "ask": "watch", "kind": "row", "place": "taken", "folders": [shots],
    }));
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
