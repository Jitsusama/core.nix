//! Programs' search over the socket: a row is found by its name, by what
//! its entry says it is and its keywords, and by what its key binds do,
//! as the watch says them, with its name's letters to bring forward
//! where the name was found; a place it doesn't serve is refused; and
//! health counts the `changed` asks heard.

use serde_json::json;

use super::Running;

#[test]
fn a_search_finds_a_program_by_its_name_its_keywords_and_its_binds() {
    let running = Running::new("search");
    let mut client = running.client();
    let settled = client.ask(&json!({
        "id": "settings", "ask": "settings", "also": {"viewer": "Read aloud"},
    }));
    assert_eq!(
        settled["settings"]["also"],
        json!({"viewer": "Read aloud"}),
        "the settings are answered as held: {settled}"
    );
    // A later ask naming only what's shown leaves what binds do be.
    client.ask(&json!({"id": "settings", "ask": "settings", "shown": []}));
    client.ask(&json!({"id": 1, "ask": "watch", "kind": "row"}));
    let search = |client: &mut daemon_client::Client, id: u64, words: &str| {
        client.ask(&json!({"id": id, "ask": "search", "place": "programs", "words": words}))
    };
    let said = search(&mut client, 2, "vie");
    assert_eq!(said["words"], "vie", "the answer says what it answers");
    assert_eq!(said["keys"], json!(["row:programs:viewer"]));
    assert_eq!(said["marks"]["row:programs:viewer"], json!([[0, 3]]));
    let said = search(&mut client, 3, "book");
    assert_eq!(said["keys"], json!(["row:programs:viewer"]), "by a keyword");
    assert_eq!(
        said["marks"],
        json!({}),
        "its name has nothing to bring forward"
    );
    let said = search(&mut client, 4, "documents");
    assert_eq!(
        said["keys"],
        json!(["row:programs:viewer"]),
        "by its comment"
    );
    let said = search(&mut client, 5, "aloud");
    assert_eq!(
        said["keys"],
        json!(["row:programs:viewer"]),
        "by what its bind does"
    );
    let said = search(&mut client, 6, "iew");
    assert_eq!(
        said["keys"],
        json!([]),
        "letters inside a word find nothing"
    );
    let said = client.ask(&json!({"id": 7, "ask": "search", "place": "mail", "words": "a"}));
    assert_eq!(said["error"]["code"], "unknown-place", "{said}");
    assert_eq!(said["words"], "a", "a refusal says what it refuses");
}

#[test]
fn health_counts_the_changes_programs_tell() {
    let running = Running::new("told");
    let mut client = running.client();
    for id in 1..=3 {
        client.ask(&json!({
            "id": id, "ask": "changed", "keys": ["listed:kitty:open"], "from": 4242,
        }));
    }
    let said = client.ask(&json!({"id": 4, "ask": "health"}));
    assert_eq!(said["told"], json!({"asks": 3, "tellers": 1}), "{said}");
}
