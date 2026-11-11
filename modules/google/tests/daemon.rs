//! The daemon interface over what the program holds (daemon.rs), asked
//! over its socket as the shell asks it: what a watch is told at first,
//! then only the records a change touched and those that went; one kind
//! watched alone; one row moving heard as that row alone; an act handed to the program's asks; and the program
//! itself taking the socket systemd hands it.

use std::env;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use google::daemon::Daemon;
use google::drive::{Document, Is, Kind};
use google::rows::Vocabulary;
use google::state::{Standing, State};
use serde_json::{Value, json};

/// How long anything here waits on the socket before failing.
const PATIENCE: Duration = Duration::from_secs(10);

const NOW: i64 = 1_793_196_000_000;

fn folder(test: &str) -> PathBuf {
    let folder = env::temp_dir().join(format!("google-daemon-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

fn document(id: &str, title: &str) -> Document {
    Document {
        id: id.into(),
        title: title.into(),
        kind: Is::Document,
        link: format!("https://docs.google.com/document/d/{id}"),
        viewed: Some(NOW),
        changed: None,
        owner: Some("Ada Lovelace".into()),
        mime: Kind::Document.mime().into(),
        size: None,
        created: None,
        changed_by: None,
        changed_by_me: None,
        shared_with_me: None,
        shared_by: None,
        shared: false,
        starred: false,
        drive: None,
        about: None,
        across: None,
        down: None,
        lasts: None,
    }
}

fn ready(documents: Vec<Document>, heard: i64) -> State {
    State {
        account: Standing::Ready,
        documents,
        unread: Some(3),
        heard: Some(heard),
        ..State::default()
    }
}

/// A client of the daemon, as the shell is one.
struct Client {
    writing: UnixStream,
    reading: BufReader<UnixStream>,
}

impl Client {
    fn to(socket: &std::path::Path) -> Client {
        let deadline = Instant::now() + PATIENCE;
        let stream = loop {
            match UnixStream::connect(socket) {
                Ok(stream) => break stream,
                Err(error) if Instant::now() > deadline => panic!("never connected: {error}"),
                Err(_) => std::thread::sleep(Duration::from_millis(20)),
            }
        };
        stream.set_read_timeout(Some(PATIENCE)).unwrap();
        Client {
            writing: stream.try_clone().unwrap(),
            reading: BufReader::new(stream),
        }
    }

    fn ask(&mut self, said: &Value) -> Value {
        writeln!(self.writing, "{said}").unwrap();
        self.next()
    }

    fn next(&mut self) -> Value {
        let mut line = String::new();
        self.reading
            .read_line(&mut line)
            .expect("the daemon said nothing in time");
        serde_json::from_str(&line).unwrap()
    }
}

fn keys(records: &Value) -> Vec<String> {
    records
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["key"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn a_watch_hears_every_record_then_only_what_changed_and_what_went() {
    let socket = folder("watch").join("daemons/google.sock");
    let (asks, _asked) = mpsc::channel();
    let daemon = Daemon::listen(&socket, asks, NOW).unwrap();
    let mut client = Client::to(&socket);
    let hello = client.ask(&json!({"id": "hello", "ask": "hello", "version": 1}));
    assert_eq!(hello["domain"], "google");
    // Before Google is first asked, it says so.
    assert_eq!(hello["sources"]["google"], "starting");

    daemon.say(
        &ready(vec![document("one", "Plan"), document("two", "Notes")], NOW),
        NOW,
    );
    let watched = client.ask(&json!({"id": "all", "ask": "watch"}));
    assert_eq!(
        keys(&watched["records"]),
        ["file:one", "file:two", "account:google"]
    );
    let plan = &watched["records"][0];
    assert_eq!(plan["kind"], "file");
    assert_eq!(plan["heard"], NOW);
    assert_eq!(plan["file"]["title"], "Plan");
    assert_eq!(watched["records"][2]["account"]["unread"], 3);

    // Google heard again with nothing new: only the account was heard.
    daemon.say(
        &ready(
            vec![document("one", "Plan"), document("two", "Notes")],
            NOW + 60_000,
        ),
        NOW + 60_000,
    );
    let heard = client.next();
    assert_eq!(heard["id"], "all");
    assert_eq!(keys(&heard["changed"]), ["account:google"]);
    assert_eq!(heard["changed"][0]["heard"], NOW + 60_000);
    assert_eq!(heard["gone"], json!([]));

    // One renamed and one gone: the one renamed, heard as it changed, and
    // the other said gone.
    daemon.say(
        &ready(vec![document("one", "Plan, Again")], NOW + 60_000),
        NOW + 120_000,
    );
    let changed = client.next();
    assert_eq!(keys(&changed["changed"]), ["file:one"]);
    assert_eq!(changed["changed"][0]["file"]["title"], "Plan, Again");
    assert_eq!(changed["changed"][0]["heard"], NOW + 120_000);
    assert_eq!(changed["gone"], json!(["file:two"]));

    // Signed out: everything it held is gone, and the account is said
    // signed out, as a source's standing.
    daemon.say(&State::without(Standing::SignedOut), NOW + 180_000);
    let out = client.next();
    assert_eq!(keys(&out["changed"]), ["account:google"]);
    assert_eq!(out["gone"], json!(["file:one"]));
    let standing = client.next();
    assert_eq!(
        standing,
        json!({"id": "all", "sources": {"google": "signed-out"}})
    );
}

#[test]
fn a_list_or_a_watch_says_when_google_was_last_read_and_how_it_stands() {
    let socket = folder("read").join("google.sock");
    let (asks, _asked) = mpsc::channel();
    let daemon = Daemon::listen(&socket, asks, NOW).unwrap();
    let mut client = Client::to(&socket);
    // Before Google is first read, an empty list says so, so nothing takes
    // it for no meetings, as a switch's gate did across a restart
    // (2026-11-10).
    let before = client.ask(&json!({"id": 1, "ask": "list", "kind": "meeting"}));
    assert_eq!(
        (&before["records"], &before["read"], &before["sources"]),
        (&json!([]), &Value::Null, &json!({"google": "starting"}))
    );
    daemon.say(&ready(vec![], NOW - 5_000), NOW);
    let after = client.ask(&json!({"id": 2, "ask": "list", "kind": "meeting"}));
    assert_eq!(
        (&after["read"], &after["sources"]),
        (&json!(NOW - 5_000), &json!({"google": "answering"}))
    );
    let watched = client.ask(&json!({"id": 3, "ask": "watch", "kind": "meeting"}));
    assert_eq!(
        (&watched["read"], &watched["sources"]),
        (&json!(NOW - 5_000), &json!({"google": "answering"}))
    );
}

#[test]
fn a_watch_of_one_kind_hears_only_that_kind() {
    let socket = folder("kind").join("google.sock");
    let (asks, _asked) = mpsc::channel();
    let daemon = Daemon::listen(&socket, asks, NOW).unwrap();
    daemon.say(&ready(vec![document("one", "Plan")], NOW), NOW);
    let mut client = Client::to(&socket);
    let watched = client.ask(&json!({"id": 1, "ask": "watch", "kind": ["account"]}));
    assert_eq!(keys(&watched["records"]), ["account:google"]);
    daemon.say(&ready(vec![document("one", "Plan, Again")], NOW), NOW + 1);
    // Only the document changed, so this watch hears nothing; what it
    // hears next is the account heard again.
    daemon.say(
        &ready(vec![document("one", "Plan, Again")], NOW + 2),
        NOW + 2,
    );
    let next = client.next();
    assert_eq!(keys(&next["changed"]), ["account:google"]);
    assert_eq!(next["changed"][0]["heard"], NOW + 2);
    let refused = client.ask(&json!({"id": 2, "ask": "list", "kind": "person"}));
    assert_eq!(refused["error"]["code"], "unknown-kind");
    let listed = client.ask(&json!({"id": 3, "ask": "list", "kind": "file"}));
    assert_eq!(keys(&listed["records"]), ["file:one"]);
    let got = client.ask(&json!({"id": 4, "ask": "get", "keys": ["file:one", "file:none"]}));
    assert_eq!(keys(&got["records"]), ["file:one"]);
}

#[test]
fn a_row_that_moves_alone_is_the_one_row_heard() {
    let socket = folder("moves").join("google.sock");
    let (asks, _asked) = mpsc::channel();
    let daemon = Daemon::listen(&socket, asks, NOW).unwrap();
    let path = env::var("LAUNCHER_WORDS")
        .expect("LAUNCHER_WORDS names no vocabulary; google-reads-the-calendar hands it one");
    let vocabulary = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    daemon.words(Vocabulary::read(&vocabulary).0);
    let three = |ids: [&str; 3]| ready(Vec::from(ids.map(|id| document(id, id))), NOW);
    daemon.say(&three(["a", "b", "c"]), NOW);
    let mut client = Client::to(&socket);
    let watch = json!({"id": 1, "ask": "watch", "kind": "row", "place": "documents"});
    let watched = client.ask(&watch);
    let orders: Vec<&Value> = watched["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|record| &record["row"]["order"])
        .collect();
    assert_eq!(orders, [1024, 2048, 3072], "spaced by 1024: {watched}");
    // c, last looked at, now first: c alone moves, and a and b keep
    // theirs, so the watch hears one row.
    daemon.say(&three(["c", "a", "b"]), NOW + 1);
    let moved = client.next();
    assert_eq!(keys(&moved["changed"]), ["row:documents:c"], "{moved}");
    assert_eq!(moved["changed"][0]["row"]["order"], 0);
    assert_eq!(moved["gone"], json!([]));
}

#[test]
fn an_act_is_handed_to_the_programs_asks_and_one_it_doesnt_do_is_refused() {
    let socket = folder("act").join("google.sock");
    let (asks, asked) = mpsc::channel();
    let _daemon = Daemon::listen(&socket, asks, NOW).unwrap();
    let mut client = Client::to(&socket);
    let taken = client.ask(&json!({"id": 1, "ask": "act", "act": {"archive": {"id": "m1"}}}));
    assert_eq!(taken, json!({"id": 1, "taken": true}));
    let line = asked.recv_timeout(PATIENCE).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&line).unwrap(),
        json!({"archive": {"id": "m1"}})
    );
    // An id Google couldn't have made goes nowhere near a request to it.
    let refused = client.ask(&json!({"id": 2, "ask": "act", "act": {"archive": {"id": "../x"}}}));
    assert_eq!(refused["error"]["code"], "unknown-ask");
    assert!(asked.try_recv().is_err());
}

/// The program, started as systemd starts it behind google.socket.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn a_socket_systemd_hands_it_is_the_one_it_answers_on() {
    let folder = folder("handed");
    let socket = folder.join("daemons/google.sock");
    std::fs::create_dir_all(socket.parent().unwrap()).unwrap();
    // Nobody signed in, so Google is never asked.
    let running = Running(
        Command::new("systemd-socket-activate")
            .arg(format!("--listen={}", socket.display()))
            .arg(env!("CARGO_BIN_EXE_google"))
            .arg(folder.join("credentials.json"))
            // A path of its own it would bind were it not handed one.
            .arg(folder.join("unhanded.sock"))
            .stdout(Stdio::null())
            .spawn()
            .expect("systemd-socket-activate isn't on PATH"),
    );
    let mut client = Client::to(&socket);
    let hello = client.ask(&json!({"id": "hello", "ask": "hello", "version": 1}));
    assert_eq!(hello["domain"], "google");
    assert!(!folder.join("unhanded.sock").exists());
    drop(running);
}
