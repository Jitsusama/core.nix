//! The quests daemon run over a stand-in for the realm's node, serving
//! its two lists of quests and its sessions as the node's watches do,
//! and asked over its own socket as the launcher asks it.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::process::Command;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use daemon_client::Daemon;
use serde_json::{Value, json};

/// The ended quest found by its name, its row in the answer, since the
/// watch serves only the open ones, asked until the node's ended list has
/// landed; and named by its id, as Machine's disk names what fills it,
/// its title, asked of the daemon, which knows every quest.
fn the_ended_one_is_found_and_titled(daemon: &Daemon) {
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
    let titles = searcher.ask(&json!({"id": 10, "ask": "titles", "ids": ["QEST-E"]}));
    assert_eq!(
        titles["titles"],
        json!({"QEST-E": "Tune the banner"}),
        "{titles}"
    );
}

/// A stand-in node at `socket`: each watch answered with its kind's
/// records whole, and the open quests' watch then told each change sent
/// down `later`. Every watch is held open, as the node holds it, so the
/// daemon never hears one end and makes its records again unasked.
fn node(socket: &Path, later: Receiver<Value>) {
    let listener = UnixListener::bind(socket).expect("the stand-in node listens");
    thread::spawn(move || {
        let mut later = Some(later);
        let mut held = Vec::new();
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
                        "status": "active", "priority": "driving", "rank": 1,
                        "start": "/home/joel/src/laptop"},
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
            } else {
                held.push(stream);
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
    client.send(
        &json!({"id": 2, "ask": "watch", "kind": ["row", "summary", "folders"],
        "place": "quests"}),
    );
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
    the_ended_one_is_found_and_titled(&daemon);
    let b = &held["row:quests:QEST-B"]["row"];
    assert_eq!(
        b["side"],
        json!([{"said": "subquest"}, {"said": "queued"}, {"said": "S"}])
    );
    assert_eq!(b["held"]["folder"], "/home/joel");
    assert_eq!(
        held.get("folders:quests").map(|record| &record["folders"]),
        Some(&json!([{"folder": "/home/joel/src/laptop", "name": "~/src/laptop"}])),
        "where Start New Work's words can be said, home never: {held:?}"
    );
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

/// What's told on the watch until `until` holds of everything held.
fn heard_until(
    client: &mut daemon_client::Client,
    held: &mut BTreeMap<String, Value>,
    until: impl Fn(&BTreeMap<String, Value>) -> bool,
) {
    while !until(held) {
        let said = client.heard();
        for record in said["records"].as_array().into_iter().flatten() {
            held.insert(
                record["key"].as_str().unwrap_or("").to_owned(),
                record.clone(),
            );
        }
        for record in said["changed"].as_array().into_iter().flatten() {
            held.insert(
                record["key"].as_str().unwrap_or("").to_owned(),
                record.clone(),
            );
        }
        for key in said["gone"].as_array().into_iter().flatten() {
            held.remove(key.as_str().unwrap_or(""));
        }
    }
}

/// The quests daemon over the stand-in node, reading `asks` and keeping
/// what he looked at in `looked`; its socket in `run`, which goes with it.
fn moved_daemon(node: &Path, asks: &Path, looked: &Path, run: &Path) -> Daemon {
    let socket = run.join("quests.sock");
    std::fs::create_dir_all(run).expect("its folder");
    Daemon::start(
        Command::new(env!("CARGO_BIN_EXE_quests"))
            .arg("--vocabulary")
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../quickshell/launcher/words.json"
            ))
            .args(["--node".as_ref(), node.as_os_str()])
            .args(["--socket".as_ref(), socket.as_os_str()])
            .args(["--asks".as_ref(), asks.as_os_str()])
            .args(["--looked".as_ref(), looked.as_os_str()])
            .env("HOME", "/home/joel"),
        socket,
        run.to_owned(),
    )
}

/// A watch of what moved, held as it's told.
fn moved_watch(daemon: &Daemon) -> (daemon_client::Client, BTreeMap<String, Value>) {
    let mut client = daemon.client();
    client.ask(&json!({"id": 1, "ask": "hello"}));
    client.send(&json!({"id": 2, "ask": "watch", "kind": ["moved"], "place": "quests"}));
    let mut held = BTreeMap::new();
    heard_until(&mut client, &mut held, |held| {
        held.contains_key("moved:quests")
    });
    (client, held)
}

fn entries(held: &BTreeMap<String, Value>) -> Option<usize> {
    held.get("moved:quests")
        .and_then(|record| record["moved"].as_array())
        .map(Vec::len)
}

fn write_asks(asks: &Path, lines: &[Value]) {
    let lines: Vec<String> = lines.iter().map(Value::to_string).collect();
    std::fs::write(asks, lines.join("\n") + "\n").expect("the asks");
}

/// Gives `looked` what was seen, as the launcher does once he's looked.
fn give_looked(daemon: &Daemon, seen: &Value) -> Value {
    let mut asker = daemon.client();
    asker.ask(&json!({"id": 1, "ask": "hello"}));
    asker.ask(&json!({"id": 2, "ask": "looked", "seen": seen}))
}

/// Waits for a first start to keep the asks there as shown, which says
/// they've been read.
fn kept_at_start(looked: &Path) {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !looked.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "a first start never kept the asks there as shown in {}",
            looked.display()
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// The moment `seconds` from now, as the asks' keeper writes one, in UTC
/// (Howard Hinnant's civil date from days).
fn moment_in(seconds: i64) -> String {
    let since = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("a clock past 1970")
        .as_secs();
    let at = i64::try_from(since).expect("seconds in range") + seconds;
    let (days, of_day) = (at.div_euclid(86_400) + 719_468, at.rem_euclid(86_400));
    let era = days.div_euclid(146_097);
    let of_era = days - era * 146_097;
    let year_of_era = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted = (5 * of_year + 2) / 153;
    let day = of_year - (153 * shifted + 2) / 5 + 1;
    let month = if shifted < 10 {
        shifted + 3
    } else {
        shifted - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        of_day / 3600,
        of_day % 3600 / 60,
        of_day % 60
    )
}

/// A date What Moved says is next goes once it passes, though nothing
/// else moves, rather than standing as one still owed.
#[test]
fn a_next_date_once_passed_is_no_longer_said() {
    let folder = std::env::temp_dir().join(format!("quests-due-{}", std::process::id()));
    std::fs::create_dir_all(&folder).expect("a folder");
    let node_socket = folder.join("node.sock");
    let (_tell, later) = mpsc::channel();
    node(&node_socket, later);
    let asks = folder.join("asks.jsonl");
    let looked = folder.join("state/looked");
    let at = "2026-11-20T10:00:00-05:00";
    let old = json!({"at": at, "owner": "QEST-A", "words": "the old"});
    write_asks(&asks, std::slice::from_ref(&old));
    let daemon = moved_daemon(&node_socket, &asks, &looked, &folder.join("run"));
    let (mut client, mut held) = moved_watch(&daemon);
    kept_at_start(&looked);
    let soon = json!({"at": at, "owner": "QEST-A", "words": "the soon",
        "expected": moment_in(4)});
    write_asks(&asks, &[old, soon]);
    let record = moved_named(&mut client, &mut held);
    assert_eq!(
        record["moved"][0]["side"][0]["said"], "next due {time}",
        "what it took on is owed soon: {record}"
    );
    heard_until(&mut client, &mut held, |held| {
        held["moved:quests"]["moved"][0]["side"] == json!([])
    });
    drop((client, daemon));
    let _ = std::fs::remove_dir_all(&folder);
}

/// Whether what moved shows the ask of these words.
fn said_of(held: &BTreeMap<String, Value>, words: &str) -> bool {
    held.get("moved:quests")
        .and_then(|record| record["seen"].as_array())
        .is_some_and(|seen| {
            seen.iter()
                .any(|one| one["key"].as_str().is_some_and(|key| key.ends_with(words)))
        })
}

/// Waits for what moved to name something, then gives its record.
fn moved_named(client: &mut daemon_client::Client, held: &mut BTreeMap<String, Value>) -> Value {
    heard_until(client, held, |held| entries(held).is_some_and(|n| n > 0));
    held["moved:quests"].clone()
}

/// A first start says nothing of the asks already there. An ask he was
/// shown landing, written while the daemon runs, is served as what moved
/// for its quest in the launcher's words; given back as looked at it goes,
/// and what he looked at is kept, so a restart says only what moved while
/// it was down. While the asks' keeper holds their lock the asks told last
/// stand, and a look it can't keep is refused.
#[test]
fn what_moved_since_he_looked_is_served_until_he_looks() {
    let folder = std::env::temp_dir().join(format!("quests-moved-{}", std::process::id()));
    std::fs::create_dir_all(&folder).expect("a folder");
    let node_socket = folder.join("node.sock");
    let (_tell, later) = mpsc::channel();
    node(&node_socket, later);
    let asks = folder.join("asks.jsonl");
    let looked = folder.join("state/looked");
    let at = "2026-11-20T10:00:00-05:00";
    // Both there before the first start, so said only were nothing taken
    // as shown: one landed, one still owed.
    let fix = json!({"at": at, "owner": "A (B)", "words": "the fix",
        "landed": "2026-11-20T11:00:00-05:00"});
    let next = json!({"at": at, "owner": "QEST-A", "words": "the next",
        "expected": "2099-01-01T00:00:00-05:00"});
    write_asks(&asks, &[fix.clone(), next.clone()]);
    let daemon = moved_daemon(&node_socket, &asks, &looked, &folder.join("run"));
    let (mut client, mut held) = moved_watch(&daemon);
    kept_at_start(&looked);

    // The one he was shown owed lands, stamped before he could have looked.
    let mut landed = next;
    landed["landed"] = json!("2026-11-20T09:00:00-05:00");
    write_asks(&asks, &[fix.clone(), landed.clone()]);
    let record = moved_named(&mut client, &mut held);
    let moved = &record["moved"][0];
    assert_eq!(
        (entries(&held), &moved["note"]),
        (Some(1), &json!("landed: the next")),
        "a first start took what was there as shown, and only the landing moved: {record}"
    );
    assert_eq!(moved["key"], "moved:quests:QEST-A", "{moved}");
    assert_eq!(moved["name"], "Bootstrap the laptop", "{moved}");
    assert_eq!(
        moved["side"],
        json!([]),
        "a landed ask owes no date: {moved}"
    );

    let refused = give_looked(&daemon, &Value::Null);
    assert_eq!(
        refused["error"]["code"], "bad-input",
        "looked names what was seen: {refused}"
    );
    let folder_of = looked.parent().expect("its folder").to_owned();
    std::fs::set_permissions(&folder_of, std::fs::Permissions::from_mode(0o500))
        .expect("its folder shut");
    let unkept = give_looked(&daemon, &record["seen"]);
    std::fs::set_permissions(&folder_of, std::fs::Permissions::from_mode(0o700))
        .expect("its folder open");
    assert_eq!(
        unkept["error"]["code"], "failed",
        "a look it couldn't keep is refused: {unkept}"
    );
    let answer = give_looked(&daemon, &record["seen"]);
    assert_eq!(answer["done"], true, "{answer}");
    heard_until(&mut client, &mut held, |held| entries(held) == Some(0));

    // While its keeper holds the file's lock, mid-write, the asks told
    // last stand; once it lets go, what it wrote is read.
    let third = json!({"at": at, "owner": "QEST-A", "words": "the third",
        "landed": "2026-11-20T12:00:00-05:00"});
    write_asks(&asks, &[fix.clone(), landed.clone(), third.clone()]);
    let record = moved_named(&mut client, &mut held);
    let lock = std::fs::File::create(folder.join("asks.jsonl.lock")).expect("the keeper's lock");
    lock.lock().expect("the keeper holding it");
    let mid = json!({"at": at, "owner": "QEST-A", "words": "mid-write"});
    write_asks(&asks, &[fix, landed.clone(), third.clone(), mid]);
    // Three looks at it locked, the most it waits before saying so.
    std::thread::sleep(Duration::from_secs(7));
    let (_, now) = moved_watch(&daemon);
    assert_eq!(
        now["moved:quests"], record,
        "what it read last stands while the lock's held: {now:?}"
    );
    drop(lock);
    heard_until(&mut client, &mut held, |held| said_of(held, "mid-write"));

    // Put down by its keeper, the fix is let go of what's kept once he
    // looks; a fifth taken on says the file was read again.
    let fifth = json!({"at": at, "owner": "QEST-A", "words": "the fifth"});
    write_asks(&asks, &[landed.clone(), third.clone(), fifth.clone()]);
    heard_until(&mut client, &mut held, |held| said_of(held, "the fifth"));
    let answer = give_looked(&daemon, &held["moved:quests"]["seen"]);
    assert_eq!(answer["done"], true, "{answer}");
    heard_until(&mut client, &mut held, |held| entries(held) == Some(0));
    drop((client, daemon));
    let kept = std::fs::read_to_string(&looked).expect("what was looked at");
    assert!(
        !kept.contains("the fix") && kept.contains("the third"),
        "only what can still move is kept: {kept}"
    );

    // Moved while nothing ran: only that is said, as what he looked at
    // was kept.
    let fourth = json!({"at": at, "owner": "QEST-A", "words": "the fourth",
        "landed": "2026-11-21T02:00:00-05:00"});
    write_asks(&asks, &[landed, third, fifth, fourth]);
    let daemon = moved_daemon(&node_socket, &asks, &looked, &folder.join("run"));
    let (mut client, mut held) = moved_watch(&daemon);
    let record = moved_named(&mut client, &mut held);
    assert_eq!(
        (entries(&held), &record["moved"][0]["note"]),
        (Some(1), &json!("landed: the fourth")),
        "only what moved while down: {record}"
    );
    drop((client, daemon));
    let _ = std::fs::remove_dir_all(&folder);
}
