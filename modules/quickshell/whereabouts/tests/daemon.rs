//! The daemon over its socket, with a root and files of its own and no
//! GeoClue to ask: hello says it's location's; a watch is answered with
//! where it kept the laptop and the clock's zone the root's link names;
//! the link changed, clock-follows' move written, and a pin set and then
//! taken away are each said to the watch as they happen; and a kind it
//! doesn't keep is refused.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// Where the laptop was kept before the daemon started, found by Wi-Fi.
fn kept() -> Value {
    json!({
        "v": 2, "latitude": 52.34, "longitude": -115.67, "accuracy": 26000.0,
        "when": 1_790_000_000, "place": "Kelpwater, Alberta", "zone": "America/Edmonton",
        "country": "Canada",
    })
}

struct Running {
    daemon: Child,
    socket: PathBuf,
    folder: PathBuf,
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
        let daemon = Command::new(env!("CARGO_BIN_EXE_whereabouts"))
            .arg(&socket)
            .arg(folder.join("root"))
            .arg(folder.join("kept/here.json"))
            .arg(folder.join("state/pin.json"))
            .arg(folder.join("state/moved.json"))
            // Never the machine's own GeoClue, which would find where the
            // tests run and ask the web what's there.
            .env("DBUS_SYSTEM_BUS_ADDRESS", "unix:path=/nonexistent/bus")
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        until("the socket never came", || socket.exists());
        Running {
            daemon,
            socket,
            folder,
        }
    }

    fn path(&self, path: &str) -> PathBuf {
        self.folder.join(path)
    }

    fn client(&self) -> Client {
        let stream = UnixStream::connect(&self.socket).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        Client {
            lines: BufReader::new(stream.try_clone().unwrap()),
            stream,
        }
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.daemon.kill();
        let _ = self.daemon.wait();
        let _ = std::fs::remove_dir_all(&self.folder);
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

struct Client {
    stream: UnixStream,
    lines: BufReader<UnixStream>,
}

impl Client {
    /// The answer to an ask, passing over what a watch pushed meanwhile.
    fn ask(&mut self, asked: &Value) -> Value {
        writeln!(self.stream, "{asked}").unwrap();
        loop {
            let said = self.next();
            if said["id"] == asked["id"] && said.get("changed").is_none() {
                return said;
            }
        }
    }

    fn next(&mut self) -> Value {
        let mut line = String::new();
        self.lines.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    }

    /// The laptop's record as a watch is next told it, once `wanted`
    /// holds of it, passing over changes that don't.
    fn changed_until(&mut self, wanted: impl Fn(&Value) -> bool) -> Value {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            let said = self.next();
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
}

fn until(why: &str, done: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done() {
        assert!(Instant::now() < deadline, "{why}");
        std::thread::sleep(Duration::from_millis(20));
    }
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
    assert_eq!(record["place"], "Kelpwater, Alberta");
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
    zone_set(&running.folder, "America/Edmonton");
    let record = client.changed_until(|record| record["clock_zone"] == "America/Edmonton");
    assert_eq!(record["place"], "Kelpwater, Alberta");
    written(
        &running.path("state/moved.json"),
        &json!({"from": "America/Halifax", "to": "America/Edmonton",
                "place": "Kelpwater, Alberta", "when": 1_790_000_100}),
    );
    let record = client.changed_until(|record| !record["moved"].is_null());
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
    let record = client.changed_until(|record| record["hand"] == true);
    assert_eq!(record["place"], "Quaystead, Westmark");
    let kept = std::fs::read_to_string(running.path("kept/here.json")).unwrap();
    assert!(
        kept.contains("\"hand\":true"),
        "the pin wasn't kept: {kept}"
    );
    // Taken away, a here set by hand is let go, until GeoClue says one.
    std::fs::remove_file(running.path("state/pin.json")).unwrap();
    let record = client.changed_until(|record| record["latitude"].is_null());
    assert_eq!(record["hand"], Value::Null);
    assert!(!running.path("kept/here.json").exists());
}

#[test]
fn a_kind_it_doesnt_keep_is_refused() {
    let running = Running::new("refused");
    let said = running
        .client()
        .ask(&json!({"id": 2, "ask": "watch", "kind": "weather"}));
    assert_eq!(said["error"]["code"], "unknown-kind", "{said}");
}
