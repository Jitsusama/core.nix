//! The daemon interface over what the program holds (daemon.rs), asked
//! over its socket as the shell asks it: what a watch is told at first,
//! then only the records a change touched and those that went; one kind
//! watched alone; an act handed to the program's asks; and the program
//! itself taking the socket systemd hands it.

use std::env;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use google::daemon::Daemon;
use google::drive::{Document, Kind};
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
        kind: Kind::Document,
        link: format!("https://docs.google.com/document/d/{id}"),
        viewed: Some(NOW),
        changed: None,
        owner: Some("Ada Lovelace".into()),
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
            .arg(folder.join("google.sock"))
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
