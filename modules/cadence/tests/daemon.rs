//! `cadence-spec`, which runs `cadence serve`, over a stand-in account: health's day file holding one
//! of Joel's readings out, and the quest owning it, asked over its own
//! socket as the launcher's Spec asks it.

use std::collections::BTreeMap;
use std::fs;
use std::process::Command;

use daemon_client::Daemon;
use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde_json::{Value, json};

#[test]
fn spec_is_served_from_the_axes_with_its_headline_and_owners_named() {
    let folder = std::env::temp_dir().join(format!("cadence-serve-{}", std::process::id()));
    let state = folder.join("state");
    let today = Timestamp::now()
        .to_zoned(TimeZone::system())
        .strftime("%Y-%m-%d")
        .to_string();
    fs::create_dir_all(state.join("health")).expect("a health folder");
    fs::write(
        state.join(format!("health/{today}.json")),
        json!({"figures": {"crashes.his": 2.0, "agents.died": 0.0}}).to_string(),
    )
    .expect("a day file");
    let quest =
        folder.join("data/pi/agentic-harness.pi/quest-workflow/quests/QEST-20261111-80ECO2");
    fs::create_dir_all(&quest).expect("a quest folder");
    fs::write(
        quest.join("README.md"),
        "---\nid: QEST-20261111-80ECO2\n---\n\n# The Cadence\n",
    )
    .expect("its README");
    let socket = folder.join("cadence-spec.sock");
    let daemon = Daemon::start(
        Command::new(env!("CARGO_BIN_EXE_cadence-spec"))
            .arg("--vocabulary")
            .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/words.json"))
            .arg("--socket")
            .arg(&socket)
            .env("HOME", &folder)
            .env("XDG_STATE_HOME", &state)
            .env("XDG_DATA_HOME", folder.join("data"))
            // Nothing it reads by command runs, so only the files above count.
            .env("PATH", ""),
        socket,
        folder.clone(),
    );
    let mut client = daemon.client();
    client.ask(&json!({"id": 1, "ask": "hello"}));
    client.send(&json!({"id": 2, "ask": "watch", "kind": ["row", "head"], "place": "spec"}));
    let mut held: BTreeMap<String, Value> = BTreeMap::new();
    while !held.contains_key("head:spec") {
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
    assert_eq!(held["head:spec"]["said"], "1 out of spec", "{held:?}");
    assert_eq!(held["head:spec"]["glyph"], "failed");
    let quality = &held["row:spec:quality"]["row"];
    assert_eq!(quality["side"], json!([{"said": "1 out"}]), "{quality}");
    assert_eq!(quality["glyph"], "failed");
    let crashes = held
        .values()
        .map(|record| &record["row"])
        .find(|row| {
            row["name"]
                .as_str()
                .is_some_and(|n| n.starts_with("2 crashes of his"))
        })
        .unwrap_or_else(|| panic!("no row for his crashes among {held:?}"));
    assert_eq!(crashes["parent"], "row:spec:quality");
    assert_eq!(crashes["owner"], "80ECO2");
    assert_eq!(crashes["side"], json!([{"said": "The Cadence"}]));
    let orders: Vec<i64> = ["row:spec:resources", "row:spec:quality", "row:spec:stress"]
        .iter()
        .map(|key| held[*key]["row"]["order"].as_i64().unwrap_or(-1))
        .collect();
    assert!(
        orders.is_sorted() && orders[0] >= 0,
        "the axes in cadence's order: {orders:?}"
    );
    drop(client);
    drop(daemon);
    let _ = fs::remove_dir_all(&folder);
}
