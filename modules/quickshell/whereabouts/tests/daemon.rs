//! The daemon over its socket, with a root and files of its own and no
//! GeoClue to ask: hello says it's location's; a watch is answered with
//! where it kept the laptop and the clock's zone the root's link names;
//! the link changed, clock-follows' move written, and a pin set and then
//! taken away are each said to the watch as they happen; Location's rows,
//! worded from the launcher's words, are told only to a watch of their
//! place, and Find It Again, asked on the socket, takes the pin away and
//! the row that asked it; and a kind it doesn't keep is refused.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use daemon_client::{Client, Daemon};
use serde_json::{Value, json};

/// Where the laptop was kept before the daemon started, found by Wi-Fi.
fn kept() -> Value {
    json!({
        "v": 2, "latitude": 52.34, "longitude": -115.67, "accuracy": 26000.0,
        "when": 1_790_000_000, "place": "Kelpwater, Highreach", "zone": "America/Edmonton",
        "country": "Canada",
    })
}

/// The daemon, over a root and files of its own.
struct Running {
    daemon: Daemon,
}

impl Running {
    fn new(test: &str) -> Running {
        let folder =
            std::env::temp_dir().join(format!("whereabouts-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(folder.join("root/etc")).unwrap();
        std::fs::create_dir_all(folder.join("kept")).unwrap();
        std::fs::create_dir_all(folder.join("state")).unwrap();
        zone_set(&folder, "America/Halifax");
        std::fs::write(folder.join("kept/here.json"), kept().to_string()).unwrap();
        let socket = folder.join("daemons/location.sock");
        let mut command = Command::new(env!("CARGO_BIN_EXE_whereabouts"));
        command
            .arg(&socket)
            .arg(folder.join("root"))
            .arg(folder.join("kept/here.json"))
            .arg(folder.join("state/pin.json"))
            .arg(folder.join("state/moved.json"))
            .arg("--words")
            .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/words.json"))
            // Never the machine's own GeoClue, which would find where the
            // tests run and ask the web what's there.
            .env("DBUS_SYSTEM_BUS_ADDRESS", "unix:path=/nonexistent/bus")
            .stderr(Stdio::null());
        Running {
            daemon: Daemon::start(&mut command, socket, folder),
        }
    }

    fn folder(&self) -> &Path {
        self.daemon.folder()
    }

    fn path(&self, path: &str) -> PathBuf {
        self.folder().join(path)
    }

    fn client(&self) -> Client {
        self.daemon.client()
    }
}

/// Points the root's localtime at a zone, as timedated does: a new link
/// renamed over the old.
fn zone_set(folder: &Path, zone: &str) {
    let link = folder.join("root/etc/localtime");
    let new = folder.join("root/etc/localtime.new");
    let _ = std::fs::remove_file(&new);
    std::os::unix::fs::symlink(format!("/etc/zoneinfo/{zone}"), &new).unwrap();
    std::fs::rename(new, link).unwrap();
}

/// Writes a file whole, under another name, then renames it, as each
/// program that writes these does.
fn written(file: &Path, said: &Value) {
    let new = file.with_extension("new");
    std::fs::write(&new, said.to_string()).unwrap();
    std::fs::rename(new, file).unwrap();
}

/// The laptop's record as a watch is next told it, once `wanted` holds of
/// it, passing over changes that don't.
fn changed_until(client: &mut Client, wanted: impl Fn(&Value) -> bool) -> Value {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        let said = client.heard();
        let found = said["changed"].as_array().and_then(|records| {
            records
                .iter()
                .find(|record| record["key"] == "here:laptop")
                .cloned()
        });
        if let Some(record) = found.filter(|record| wanted(record)) {
            return record;
        }
    }
    panic!("the watch was never told what was wanted");
}

#[test]
fn hello_says_it_is_locations_and_what_it_asks() {
    let running = Running::new("hello");
    let said = running.client().ask(&json!({"id": 1, "ask": "hello"}));
    assert_eq!(said["version"], 1);
    assert_eq!(said["domain"], "location");
    assert_eq!(said["state"], "answering");
    assert_eq!(said["sources"]["clock"]["state"], "answering", "{said}");
}

#[test]
fn a_watch_is_answered_with_what_was_kept_and_the_clocks_zone() {
    let running = Running::new("watched");
    let said = running
        .client()
        .ask(&json!({"id": "w", "ask": "watch", "kind": "here", "keys": ["here:laptop"]}));
    let record = &said["records"][0];
    assert_eq!(record["key"], "here:laptop");
    assert_eq!(record["kind"], "here");
    assert_eq!(record["latitude"], 52.34);
    assert_eq!(record["accuracy_m"], 26000.0);
    assert_eq!(record["place"], "Kelpwater, Highreach");
    assert_eq!(record["zone"], "America/Edmonton");
    assert_eq!(record["hand"], false);
    assert_eq!(record["altitude_m"], Value::Null);
    assert_eq!(record["clock_zone"], "America/Halifax");
    assert_eq!(record["moved"], Value::Null);
}

#[test]
fn the_clocks_zone_and_its_move_are_said_as_they_change() {
    let running = Running::new("moves");
    let mut client = running.client();
    client.ask(&json!({"id": "w", "ask": "watch", "kind": "here"}));
    zone_set(running.folder(), "America/Edmonton");
    let record = changed_until(&mut client, |record| {
        record["clock_zone"] == "America/Edmonton"
    });
    assert_eq!(record["place"], "Kelpwater, Highreach");
    written(
        &running.path("state/moved.json"),
        &json!({"from": "America/Halifax", "to": "America/Edmonton",
                "place": "Kelpwater, Highreach", "when": 1_790_000_100}),
    );
    let record = changed_until(&mut client, |record| !record["moved"].is_null());
    assert_eq!(record["moved"]["from"], "America/Halifax");
    assert_eq!(record["moved"]["at_s"], 1_790_000_100);
}

#[test]
fn a_pin_holds_until_it_is_taken_away() {
    let running = Running::new("pinned");
    let mut client = running.client();
    client.ask(&json!({"id": "w", "ask": "watch", "kind": "here"}));
    let pin = json!({
        "v": 2, "latitude": 44.65, "longitude": -63.57, "accuracy": 1000.0, "when": 1_790_000_200,
        "place": "Quaystead, Westmark", "zone": "America/Halifax", "hand": true,
    });
    written(&running.path("state/pin.json"), &pin);
    let record = changed_until(&mut client, |record| record["hand"] == true);
    assert_eq!(record["place"], "Quaystead, Westmark");
    let kept = std::fs::read_to_string(running.path("kept/here.json")).unwrap();
    assert!(
        kept.contains("\"hand\":true"),
        "the pin wasn't kept: {kept}"
    );
    // Taken away, a here set by hand is let go, until GeoClue says one.
    std::fs::remove_file(running.path("state/pin.json")).unwrap();
    let record = changed_until(&mut client, |record| record["latitude"].is_null());
    assert_eq!(record["hand"], Value::Null);
    assert!(
        !running.path("kept/here.json").exists(),
        "a here set by hand is never kept"
    );
}

/// The keys of the records an answer or a change holds.
fn keys(said: &Value, field: &str) -> Vec<String> {
    said[field]
        .as_array()
        .map(|records| {
            records
                .iter()
                .filter_map(|record| record["key"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn locations_rows_are_told_only_to_a_watch_of_their_place() {
    let running = Running::new("rows");
    let mut client = running.client();
    let said = client.ask(&json!({"id": "r", "ask": "watch", "kind": "row", "place": "location"}));
    let rows = keys(&said, "records");
    assert!(rows.contains(&"row:location:town".to_owned()), "{said}");
    assert!(
        rows.iter().all(|key| key.starts_with("row:location:")),
        "only Location's own place: {rows:?}"
    );
    let town = said["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["key"] == "row:location:town")
        .unwrap();
    assert_eq!(
        (&town["kind"], &town["row"]["side"][0]["said"]),
        (&json!("row"), &json!("Kelpwater, Highreach")),
        "wrapped as every daemon's rows are: {town}"
    );
    assert_eq!(town["row"]["parent"], "row:on:location");
    assert!(town["row"]["order"].is_i64(), "placed: {town}");

    let said = client.ask(&json!({"id": "h", "ask": "watch", "kind": "here"}));
    assert_eq!(
        keys(&said, "records"),
        ["here:laptop"],
        "a watch of here hears no rows"
    );

    let said = client.ask(&json!({"id": 1, "ask": "get", "keys": ["row:on:location"]}));
    assert_eq!(said["records"][0]["row"]["name"], "Location", "{said}");
}

#[test]
fn finding_it_again_takes_the_pin_and_its_row_away() {
    let running = Running::new("again");
    let mut client = running.client();
    let pin = json!({
        "v": 2, "latitude": 44.65, "longitude": -63.57, "accuracy": 1000.0, "when": 1_790_000_200,
        "place": "Quaystead, Westmark", "zone": "America/Halifax", "hand": true,
    });
    written(&running.path("state/pin.json"), &pin);
    client.ask(&json!({"id": "r", "ask": "watch", "kind": "row", "place": "location"}));
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut held = keys(
        &client.ask(&json!({"id": 1, "ask": "list", "kind": "row", "place": "location"})),
        "records",
    );
    while !held.contains(&"row:location:again".to_owned()) {
        assert!(
            Instant::now() < deadline,
            "a pin never offered Find It Again: {held:?}"
        );
        held = keys(&client.heard(), "changed");
    }
    // Sent rather than asked, since the row may be let go before the act
    // is answered, and both are wanted.
    client.send(&json!({"id": 2, "ask": "act", "act": "again"}));
    let (mut done, mut gone) = (false, false);
    while !(done && gone) {
        assert!(
            Instant::now() < deadline,
            "done {done}, Find It Again gone {gone}"
        );
        let said = client.heard();
        if said["id"] == 2 {
            assert_eq!(said, json!({"id": 2, "done": true}));
            assert!(!running.path("state/pin.json").exists(), "the pin stayed");
            done = true;
        }
        gone |= said["gone"]
            .as_array()
            .is_some_and(|gone| gone.contains(&json!("row:location:again")));
    }
}

#[test]
fn a_kind_it_doesnt_keep_is_refused() {
    let running = Running::new("refused");
    let said = running
        .client()
        .ask(&json!({"id": 2, "ask": "watch", "kind": "weather"}));
    assert_eq!(said["error"]["code"], "unknown-kind", "{said}");
}
