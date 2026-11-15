//! `cadence axes --served WORDS` over a stand-in account: health's day file
//! holding one of Joel's readings out, and the quest owning it, read as
//! cadence-spec reads it for the launcher's Spec.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde_json::{Value, json};

const WORDS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/words.json");

/// `cadence axes --served` run over the account in `folder`, with nothing
/// it reads by command able to run, so only the files there count.
fn served(folder: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cadence"))
        .args(["axes", "--served", WORDS])
        .env("HOME", folder)
        .env("XDG_STATE_HOME", folder.join("state"))
        .env("XDG_DATA_HOME", folder.join("data"))
        .env("PATH", "")
        .output()
        .expect("cadence runs")
}

#[test]
fn spec_is_made_from_the_axes_with_its_headline_and_owners_named() {
    let account = tempfile::tempdir().expect("a stand-in account");
    let folder = account.path();
    let today = Timestamp::now()
        .to_zoned(TimeZone::system())
        .strftime("%Y-%m-%d")
        .to_string();
    fs::create_dir_all(folder.join("state/health")).expect("a health folder");
    fs::write(
        folder.join(format!("state/health/{today}.json")),
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

    let ran = served(folder);
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(ran.status.success(), "cadence failed: {said}");
    assert!(
        said.contains("s of processor"),
        "the read says its cost: {said}"
    );
    let made: Value = serde_json::from_slice(&ran.stdout).expect("one line of JSON");
    assert_eq!(made["head"]["said"], "1 reading out of spec", "{made}");
    assert_eq!(made["head"]["glyph"], "failed");
    let rows: BTreeMap<&str, &Value> = made["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|row| (row["key"].as_str().unwrap_or(""), row))
        .collect();
    let quality = rows["row:spec:quality"];
    assert_eq!(quality["side"], json!([{"said": "1 out"}]), "{quality}");
    assert_eq!(quality["glyph"], "failed");
    let crashes = rows
        .values()
        .find(|row| {
            row["name"]
                .as_str()
                .is_some_and(|n| n.starts_with("2 crashes of his"))
        })
        .unwrap_or_else(|| panic!("no row for his crashes among {rows:?}"));
    assert_eq!(crashes["parent"], "row:spec:quality");
    assert_eq!(crashes["owner"], "80ECO2");
    assert_eq!(crashes["side"], json!([{"said": "The Cadence"}]));
    let at = |key: &str| {
        made["rows"]
            .as_array()
            .and_then(|all| all.iter().position(|r| r["key"] == key))
    };
    let orders: Vec<Option<usize>> = ["row:spec:resources", "row:spec:quality", "row:spec:stress"]
        .iter()
        .map(|key| at(key))
        .collect();
    assert!(
        orders.iter().all(Option::is_some) && orders.is_sorted(),
        "the axes in cadence's order: {orders:?}"
    );
}
