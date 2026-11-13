//! The quests daemon run over a stand-in for the realm's node, serving
//! its two lists of quests and its sessions as the node's watches do,
//! and asked over its own socket as the launcher asks it.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::process::Command;
use std::sync::mpsc::{self, Receiver};
use std::thread;

use daemon_client::Daemon;
use serde_json::{Value, json};

/// A stand-in node at `socket`: each watch answered with its kind's
/// records whole, and the open quests' watch then told each change sent
/// down `later`.
fn node(socket: &Path, later: Receiver<Value>) {
    let listener = UnixListener::bind(socket).expect("the stand-in node listens");
    thread::spawn(move || {
        let mut later = Some(later);
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let mut asks = BufReader::new(stream.try_clone().expect("its stream"));
            let mut kind = Value::Null;
            for _ in 0..2 {
                let mut line = String::new();
                asks.read_line(&mut line).expect("an ask");
                let asked: Value = serde_json::from_str(&line).expect("an ask in JSON");
                if asked["ask"] == "watch" {
                    kind = asked["kind"].clone();
                }
            }
            let records = match kind.as_str() {
                Some("quest") => json!([
                    {"key": "quest:QEST-A", "kind": "quest", "heard": 1, "id": "QEST-A",
                        "title": "Bootstrap the laptop", "quest.kind": "quest",
                        "status": "active", "priority": "driving", "rank": 1},
                    {"key": "quest:QEST-B", "kind": "quest", "heard": 1, "id": "QEST-B",
                        "title": "Retune the prompt", "quest.kind": "subquest",
                        "parent": "QEST-A", "status": "active", "priority": "queued"}
                ]),
                Some("ended-quest") => json!([
                    {"key": "ended-quest:QEST-E", "kind": "ended-quest", "heard": 1,
                        "id": "QEST-E", "title": "Tune the banner", "status": "concluded",
                        "updated": "2025-01-05"}
                ]),
                Some("session") => json!([
                    {"key": "session:S", "standing": "running",
                        "presence": {"addresses": ["S"], "owns": ["QEST-B"]}}
                ]),
                _ => json!([]),
            };
            let mut stream = stream;
            let hello = json!({"id": 1, "version": 1, "domain": "node"});
            writeln!(stream, "{hello}\n{}", json!({"id": 2, "records": records}))
                .expect("the records");
            if kind == "quest"
                && let Some(later) = later.take()
            {
                thread::spawn(move || {
                    for change in later {
                        let _ = writeln!(stream, "{change}");
                    }
                });
            }
        }
    });
}

#[test]
fn quests_rows_are_served_in_their_order_and_only_what_moved_is_told() {
    let folder = std::env::temp_dir().join(format!("quests-daemon-{}", std::process::id()));
    std::fs::create_dir_all(&folder).expect("a folder");
    let node_socket = folder.join("node.sock");
    let (tell, later) = mpsc::channel();
    node(&node_socket, later);
    let socket = folder.join("quests.sock");
    let daemon = Daemon::start(
        Command::new(env!("CARGO_BIN_EXE_quests"))
            .arg("--vocabulary")
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../quickshell/launcher/words.json"
            ))
            .arg("--node")
            .arg(&node_socket)
            .arg("--socket")
            .arg(&socket)
            .env("HOME", "/home/joel"),
        socket,
        folder,
    );
    let mut client = daemon.client();
    client.ask(&json!({"id": 1, "ask": "hello"}));
    // The node's three watches may land as more than one making, so the
    // watch waits for what it brings: the session holding B.
    client.send(&json!({"id": 2, "ask": "watch", "kind": ["row", "summary"],
        "place": "quests"}));
    let mut held: BTreeMap<String, Value> = BTreeMap::new();
    let all_heard = |held: &BTreeMap<String, Value>| {
        held.get("row:quests:QEST-B")
            .is_some_and(|b| b["row"]["held"]["session"].is_object())
    };
    while !all_heard(&held) {
        let said = client.heard();
        for record in said["records"]
            .as_array()
            .into_iter()
            .chain(said["changed"].as_array())
            .flatten()
        {
            let key = record["key"].as_str().unwrap_or("").to_owned();
            held.insert(key, record.clone());
        }
    }
    let order = |key: &str, held: &BTreeMap<String, Value>| {
        held[key]["row"]["order"].as_i64().unwrap_or(-1)
    };
    assert!(
        order("row:quests:QEST-A", &held) < order("row:quests:QEST-B", &held),
        "driving, then queued: {held:?}"
    );
    // The ended quest is found by its name, its row in the answer, since
    // the watch serves only the open ones; asked until the node's ended
    // list has landed.
    let mut searcher = daemon.client();
    let search = json!({"id": 9, "ask": "search", "place": "quests", "words": "banner"});
    let mut found = searcher.ask(&search);
    for _ in 0..200 {
        if found["keys"] != json!([]) {
            break;
        }
        thread::sleep(std::time::Duration::from_millis(25));
        found = searcher.ask(&search);
    }
    assert_eq!(found["keys"], json!(["row:quests:QEST-E"]), "{found}");
    assert_eq!(
        found["rows"]["row:quests:QEST-E"]["name"],
        "Tune the banner"
    );
    assert_eq!(found["rows"]["row:quests:QEST-E"]["held"]["id"], "QEST-E");
    let b = &held["row:quests:QEST-B"]["row"];
    assert_eq!(
        b["side"],
        json!([{"said": "subquest"}, {"said": "queued"}, {"said": "S"}])
    );
    assert_eq!(b["held"]["folder"], "/home/joel");
    tell.send(json!({"id": 2, "changed": [{"key": "quest:QEST-A", "heard": 2}]}))
        .expect("the node is told");
    tell.send(json!({"id": 2, "changed": [{"key": "quest:QEST-A", "title": "Again"}]}))
        .expect("the node is told");
    // The first change told is the title's: the list written again, only
    // when it was heard moving, told nothing. A line saying how the
    // sources stand, as the first making's, is passed over. B is told
    // too, since a subquest's row says its parent's name; E, ended, is
    // never served at all.
    let said = loop {
        let said = client.heard();
        if said.get("changed").is_some() {
            break said;
        }
    };
    let keys: Vec<&Value> = said["changed"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|record| &record["key"])
        .collect();
    assert_eq!(
        keys,
        ["row:quests:QEST-A", "row:quests:QEST-B"],
        "the renamed row and its subquest told: {said}"
    );
    assert_eq!(said["changed"][0]["row"]["name"], "Again");
    assert_eq!(said["changed"][1]["row"]["held"]["parent"]["name"], "Again");
    assert!(
        !held.contains_key("row:quests:QEST-E"),
        "the ended quest served: {held:?}"
    );
}
