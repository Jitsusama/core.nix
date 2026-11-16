//! The server as its unit runs it, on a port of its own, beside a
//! stand-in daemon speaking the daemon interface on a socket: files from
//! the documents and from a mount, the door shut to a stranger's name and
//! a stranger's page, and a feed whose pages share one watch on the
//! daemon, hear each change, can't act, and are told when the daemon is
//! gone.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tungstenite::client::IntoClientRequest;
use tungstenite::{Message, WebSocket};

/// How long a test waits for the server, or for a line it's owed.
const PATIENCE: Duration = Duration::from_secs(10);

/// A daemon standing in for the node: it answers hello, a watch of
/// `session` with two records and a get, and says what a test hands it to
/// every watch it holds.
struct StandIn {
    watches: Arc<AtomicUsize>,
    said_to: Arc<Mutex<Vec<UnixStream>>>,
    heard: Arc<Mutex<Vec<Value>>>,
}

impl StandIn {
    fn start(socket: &Path) -> Self {
        let listener = UnixListener::bind(socket).expect("the stand-in listens");
        let stand_in = Self {
            watches: Arc::default(),
            said_to: Arc::default(),
            heard: Arc::default(),
        };
        let (watches, said_to, heard) = (
            Arc::clone(&stand_in.watches),
            Arc::clone(&stand_in.said_to),
            Arc::clone(&stand_in.heard),
        );
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let (watches, said_to, heard) = (
                    Arc::clone(&watches),
                    Arc::clone(&said_to),
                    Arc::clone(&heard),
                );
                thread::spawn(move || answer(stream, &watches, &said_to, &heard));
            }
        });
        stand_in
    }

    /// Says `line` to every watch, as the server's watch is asked under id 2.
    fn say(&self, mut line: Value) {
        line["id"] = json!(2);
        for watcher in self.said_to.lock().expect("the watches").iter_mut() {
            writeln!(watcher, "{line}").expect("the stand-in says it");
        }
    }
}

fn answer(
    stream: UnixStream,
    watches: &AtomicUsize,
    said_to: &Mutex<Vec<UnixStream>>,
    heard: &Mutex<Vec<Value>>,
) {
    let mut out = stream.try_clone().expect("its stream");
    for line in BufReader::new(stream).lines() {
        let Ok(asked) = serde_json::from_str::<Value>(&line.unwrap_or_default()) else {
            return;
        };
        heard.lock().expect("what it heard").push(asked.clone());
        let id = asked["id"].clone();
        let answer = match asked["ask"].as_str() {
            Some("hello") => json!({"id": id, "version": 1, "domain": "node"}),
            Some("watch") => {
                watches.fetch_add(1, Ordering::SeqCst);
                said_to
                    .lock()
                    .expect("the watches")
                    .push(out.try_clone().expect("its stream"));
                json!({"id": id, "records": [
                    {"key": "session:a", "title": "A", "cwd": "/home/joel/a"},
                    {"key": "session:b", "title": "B", "cwd": "/home/joel/b"}
                ]})
            }
            Some("get") => json!({"id": id, "records": [
                {"key": "session:a", "title": "A", "cwd": "/home/joel/a"}
            ]}),
            _ => json!({"id": id, "error": {"code": "unknown-ask"}}),
        };
        if writeln!(out, "{answer}").is_err() {
            return;
        }
    }
}

/// The server, stopped and its folder taken away once it's dropped, and
/// a test failed whose server had already ended under it, since a server
/// that dies serves nothing and must not pass for one that refused.
struct Served {
    child: Child,
    port: u16,
    folder: PathBuf,
    stand_in: StandIn,
}

impl Drop for Served {
    fn drop(&mut self) {
        let ended = self.child.try_wait().ok().flatten();
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.folder);
        if let Some(status) = ended
            && !thread::panicking()
        {
            panic!("the server ended under the test: {status}");
        }
    }
}

fn served(name: &str) -> Served {
    let folder = std::env::temp_dir().join(format!("local-sites-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    let (documents, kit) = (folder.join("sites"), folder.join("kit"));
    std::fs::create_dir_all(documents.join("mycelium")).expect("the documents");
    std::fs::create_dir_all(&kit).expect("the kit");
    std::fs::write(documents.join("mycelium/index.html"), "<h1>map</h1>").expect("a page");
    std::fs::write(kit.join("theme.css"), ":root {}").expect("the kit's file");
    let socket = folder.join("node.sock");
    let stand_in = StandIn::start(&socket);
    // A port nobody holds, given up for the server to take.
    let port = TcpListener::bind("127.0.0.1:0")
        .and_then(|free| free.local_addr())
        .expect("a free port")
        .port();
    let config = json!({
        "listen": format!("127.0.0.1:{port}"),
        "names": ["localhost"],
        "documents": documents,
        "mounts": {"kit": kit},
        "feeds": {
            "node": {"socket": socket, "kinds": {"session": {"fields": ["title"]}}},
            "away": {"socket": folder.join("nobody.sock"), "kinds": {"machine": {}}}
        }
    });
    let file = folder.join("config.json");
    std::fs::write(&file, config.to_string()).expect("the config");
    let child = Command::new(env!("CARGO_BIN_EXE_local-sites"))
        .arg("--config")
        .arg(&file)
        .spawn()
        .expect("the server starts");
    let started = Instant::now();
    while TcpStream::connect(("127.0.0.1", port)).is_err() {
        assert!(started.elapsed() < PATIENCE, "the server never listened");
        // Nothing to wait on but the port itself, so it's looked at again.
        thread::sleep(Duration::from_millis(20));
    }
    Served {
        child,
        port,
        folder,
        stand_in,
    }
}

/// What a GET of `path`, naming the server `host`, is answered: its status
/// line and its body.
fn get(port: u16, host: &str, path: &str) -> (String, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("it connects");
    write!(stream, "GET {path} HTTP/1.1\r\nHost: {host}\r\n\r\n").expect("it asks");
    let mut answer = String::new();
    stream.read_to_string(&mut answer).expect("it's answered");
    let status = answer.lines().next().unwrap_or("").to_owned();
    let body = answer
        .split_once("\r\n\r\n")
        .map_or("", |(_, body)| body)
        .to_owned();
    (status, body)
}

fn page(port: u16, feed: &str, origin: &str) -> Result<WebSocket<TcpStream>, String> {
    let mut request = format!("ws://localhost:{port}/feed/{feed}")
        .into_client_request()
        .expect("a request");
    request
        .headers_mut()
        .insert("origin", origin.parse().expect("an origin"));
    let stream = TcpStream::connect(("127.0.0.1", port)).expect("it connects");
    stream.set_read_timeout(Some(PATIENCE)).expect("a timeout");
    tungstenite::client(request, stream)
        .map(|(socket, _)| socket)
        .map_err(|why| why.to_string())
}

fn own_page(served: &Served, feed: &str) -> WebSocket<TcpStream> {
    let origin = format!("http://localhost:{}", served.port);
    page(served.port, feed, &origin).expect("its own page is admitted")
}

fn ask(page: &mut WebSocket<TcpStream>, asked: &Value) {
    page.send(Message::text(asked.to_string()))
        .expect("the page asks");
}

fn told(page: &mut WebSocket<TcpStream>) -> Value {
    // A ping or a pong carries no line; the next message is read instead.
    loop {
        if let Message::Text(line) = page.read().expect("the page is told something") {
            return serde_json::from_str(line.as_str()).expect("a JSON line");
        }
    }
}

#[test]
fn it_serves_the_documents_and_a_mount_at_its_own_path() {
    let served = served("files");
    let host = format!("localhost:{}", served.port);
    assert_eq!(
        get(served.port, &host, "/mycelium/"),
        ("HTTP/1.1 200 OK".to_owned(), "<h1>map</h1>".to_owned())
    );
    assert_eq!(get(served.port, &host, "/kit/theme.css").1, ":root {}");
    assert_eq!(
        get(served.port, &host, "/mycelium").0,
        "HTTP/1.1 301 Moved Permanently"
    );
    assert_eq!(
        get(served.port, &host, "/nothing").0,
        "HTTP/1.1 404 Not Found"
    );
}

#[test]
fn a_request_naming_another_host_is_refused() {
    let served = served("host");
    let rebound = format!("evil.example:{}", served.port);
    assert_eq!(
        get(served.port, &rebound, "/mycelium/").0,
        "HTTP/1.1 403 Forbidden"
    );
}

#[test]
fn a_strangers_page_cant_open_a_feed() {
    let served = served("origin");
    let why = page(served.port, "node", "http://evil.example").expect_err("refused");
    assert!(why.contains("403"), "{why}");
}

#[test]
fn pages_share_one_watch_and_each_hears_every_change_trimmed() {
    let served = served("watch");
    let mut first = own_page(&served, "node");
    ask(
        &mut first,
        &json!({"id": "map", "ask": "watch", "kind": "session"}),
    );
    assert_eq!(
        told(&mut first),
        json!({"id": "map", "state": "connecting"})
    );
    assert_eq!(told(&mut first), json!({"id": "map", "state": "live"}));
    assert_eq!(
        told(&mut first),
        json!({"id": "map", "records": [
            {"key": "session:a", "title": "A"},
            {"key": "session:b", "title": "B"}
        ]}),
        "a field the feed doesn't show never reaches the page"
    );
    let mut second = own_page(&served, "node");
    ask(
        &mut second,
        &json!({"id": 1, "ask": "watch", "kind": "session", "keys": ["session:b"]}),
    );
    assert_eq!(
        told(&mut second),
        json!({"id": 1, "records": [{"key": "session:b", "title": "B"}]}),
        "a later page is told what the watch holds, narrowed to its keys"
    );
    assert_eq!(told(&mut second), json!({"id": 1, "state": "live"}));
    served.stand_in.say(json!({"changed": [
        {"key": "session:a", "title": "A again"},
        {"key": "session:b", "title": "B again"}
    ]}));
    assert_eq!(
        told(&mut first),
        json!({"id": "map", "changed": [
            {"key": "session:a", "title": "A again"},
            {"key": "session:b", "title": "B again"}
        ]})
    );
    assert_eq!(
        told(&mut second),
        json!({"id": 1, "changed": [{"key": "session:b", "title": "B again"}]})
    );
    assert_eq!(
        served.stand_in.watches.load(Ordering::SeqCst),
        1,
        "two pages, one watch on the daemon"
    );
    let asked = served.stand_in.heard.lock().expect("what it heard").clone();
    let watch = asked
        .iter()
        .find(|asked| asked["ask"] == "watch")
        .expect("a watch");
    assert_eq!(
        watch["fields"],
        json!(["title"]),
        "the daemon is asked only what's shown"
    );
}

#[test]
fn a_page_can_get_but_never_act() {
    let served = served("act");
    let mut page = own_page(&served, "node");
    ask(
        &mut page,
        &json!({"id": 1, "ask": "steer", "address": "ABC123", "steer": "stop"}),
    );
    assert_eq!(told(&mut page)["error"]["code"], "refused");
    ask(
        &mut page,
        &json!({"id": 2, "ask": "get", "keys": ["session:a"]}),
    );
    assert_eq!(
        told(&mut page),
        json!({"id": 2, "records": [{"key": "session:a", "title": "A"}]})
    );
    let asked = served.stand_in.heard.lock().expect("what it heard").clone();
    assert!(
        asked.iter().all(|asked| asked["ask"] != "steer"),
        "the steer reached the daemon: {asked:?}"
    );
}

#[test]
fn a_daemon_that_isnt_there_is_said_broken_with_why() {
    let served = served("away");
    let mut page = own_page(&served, "away");
    ask(
        &mut page,
        &json!({"id": 1, "ask": "watch", "kind": "machine"}),
    );
    assert_eq!(told(&mut page)["state"], "connecting");
    let broken = told(&mut page);
    assert_eq!(broken["state"], "broken");
    assert!(
        broken["reason"]
            .as_str()
            .is_some_and(|why| why.contains("nobody.sock")),
        "{broken}"
    );
}
