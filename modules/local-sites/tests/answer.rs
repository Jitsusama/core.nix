//! An answer pressed on a page of the sites' own, as the server's unit
//! runs it, beside one stand-in daemon that holds the looks as the to-do
//! daemon does and takes a steer as the node does: carried once, with the
//! steer the launcher sends, and refused from a document's origin, from
//! another site, without the cookie a page of the sites' own is given, for a
//! value the look doesn't offer, not sent as JSON, and a second time.
//!
//! The server takes answers only where the kernel tells who holds a
//! connection (src/peer.rs), so these run on Linux alone, as the build runs
//! them: there the test, which presses, is the person's, below no agent's
//! unit and no pi.
#![cfg(target_os = "linux")]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

mod common;

/// How long a test waits for the server.
const PATIENCE: Duration = Duration::from_secs(10);

/// The look the stand-in holds, waiting; it says no time it came, so it
/// never goes quiet under a test.
fn look() -> Value {
    json!({
        "v": 1, "id": "l1", "title": "<b>Merge</b> it?", "target": "https://example.com/pr/1",
        "kind": "pr", "state": "waiting", "address": "abc123",
        "choices": [{"value": "yes", "label": "Merge It"}, {"value": "no", "label": "Leave It"}],
    })
}

/// A look whose session has ended, on a quest whose own session goes on.
fn ended_look() -> Value {
    let mut look = look();
    look["id"] = json!("ended");
    look["address"] = json!("gone12");
    look["quest"] = json!("QEST-20261124-LIVE12");
    look
}

/// A daemon standing in for the to-do daemon and the node at once: it
/// answers a get of `look:l1` or `look:ended` with that look, and a steer
/// as the node does, refusing one to GONE12, whose session has ended, and
/// keeping every steer it was asked.
fn stand_in(socket: &Path) -> Arc<Mutex<Vec<Value>>> {
    let listener = UnixListener::bind(socket).expect("the stand-in listens");
    let steered = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&steered);
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let kept = Arc::clone(&kept);
            thread::spawn(move || answer(stream, &kept));
        }
    });
    steered
}

fn answer(stream: UnixStream, steered: &Mutex<Vec<Value>>) {
    let mut out = stream.try_clone().expect("its stream");
    for line in BufReader::new(stream).lines() {
        let Ok(asked) = serde_json::from_str::<Value>(&line.unwrap_or_default()) else {
            return;
        };
        let id = asked["id"].clone();
        let answer = match asked["ask"].as_str() {
            Some("hello") => json!({"id": id, "version": 1, "domain": "stand-in"}),
            Some("get") if asked["keys"] == json!(["look:l1"]) => {
                json!({"id": id, "records": [{"key": "look:l1", "kind": "look", "look": look()}]})
            }
            Some("get") if asked["keys"] == json!(["look:ended"]) => {
                json!({"id": id, "records": [{"key": "look:ended", "kind": "look", "look": ended_look()}]})
            }
            Some("get") => json!({"id": id, "records": []}),
            Some("steer") => {
                steered.lock().expect("the steers").push(asked.clone());
                if asked["address"] == "GONE12" {
                    let why = "no session goes by GONE12 now";
                    json!({"id": id, "error": {"code": "refused", "message": why}})
                } else {
                    json!({"id": id, "steered": {"machine": "here"}})
                }
            }
            _ => json!({"id": id, "error": {"code": "unknown-ask"}}),
        };
        if writeln!(out, "{answer}").is_err() {
            return;
        }
    }
}

/// The server, stopped and its folder taken away once it's dropped. A server
/// that died answers nothing, and must not pass for one that refused.
struct Served {
    child: Child,
    port: u16,
    folder: PathBuf,
    steered: Arc<Mutex<Vec<Value>>>,
}

impl Drop for Served {
    fn drop(&mut self) {
        common::stop(&mut self.child, &self.folder);
    }
}

/// One server starting at a time, so no two are given the same free port.
static STARTING: Mutex<()> = Mutex::new(());

fn served(name: &str) -> Served {
    let folder =
        std::env::temp_dir().join(format!("local-sites-answer-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    let (documents, home) = (folder.join("sites"), folder.join("home"));
    std::fs::create_dir_all(documents.join("note")).expect("the documents");
    std::fs::create_dir_all(&home).expect("the home");
    std::fs::write(documents.join("note/index.html"), "<p>a note</p>").expect("a document");
    std::fs::write(home.join("index.html"), "<h1>home</h1>").expect("the home's page");
    let socket = folder.join("both.sock");
    let steered = stand_in(&socket);
    let _starting = STARTING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let port = TcpListener::bind("127.0.0.1:0")
        .and_then(|free| free.local_addr())
        .expect("a free port")
        .port();
    let config = json!({
        "listen": format!("127.0.0.1:{port}"),
        "names": ["sites.localhost"],
        "documentsName": "documents.sites.localhost",
        "documents": documents,
        "home": home,
        "mounts": {},
        "feeds": {},
        "answers": {"looks": socket, "node": socket},
    });
    let file = folder.join("config.json");
    std::fs::write(&file, config.to_string()).expect("the config");
    let mut child = Command::new(env!("CARGO_BIN_EXE_local-sites"))
        .arg("--config")
        .arg(&file)
        .spawn()
        .expect("the server starts");
    let started = Instant::now();
    while TcpStream::connect(("127.0.0.1", port)).is_err() {
        assert!(
            child.try_wait().ok().flatten().is_none(),
            "the server ended"
        );
        assert!(started.elapsed() < PATIENCE, "the server never listened");
        thread::sleep(Duration::from_millis(20));
    }
    Served {
        child,
        port,
        folder,
        steered,
    }
}

/// What a request of `ask` with `extra` header lines and `body` is
/// answered: its status line, its header lines and its body.
fn asked(port: u16, ask: &str, extra: &str, body: &str) -> (String, String, String) {
    asked_at("sites.localhost", port, ask, (extra, body))
}

/// What a request of `ask` naming the server `name` is answered, with
/// `extra` header lines and `body`.
fn asked_at(
    name: &str,
    port: u16,
    ask: &str,
    (extra, body): (&str, &str),
) -> (String, String, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("it connects");
    let length = body.len();
    write!(
        stream,
        "{ask} HTTP/1.1\r\nHost: {name}:{port}\r\nContent-Length: {length}\r\n{extra}\r\n{body}"
    )
    .expect("it asks");
    let mut answer = String::new();
    stream.read_to_string(&mut answer).expect("it's answered");
    let (head, body) = answer.split_once("\r\n\r\n").unwrap_or((&answer, ""));
    let (status, headers) = head.split_once("\r\n").unwrap_or((head, ""));
    (status.to_owned(), headers.to_owned(), body.to_owned())
}

/// The cookie a page of the sites' own is given once it asks from this
/// origin, as `name=value`.
fn given(port: u16) -> String {
    let ours = "Sec-Fetch-Site: same-origin\r\n";
    let (status, headers, body) = asked(port, "GET /answer", ours, "");
    assert_eq!(status, "HTTP/1.1 204 No Content", "{body}");
    let given = cookie(&headers, "sites-answer").expect("the page was given nothing");
    given.split(';').next().expect("its value").to_owned()
}

/// The cookie `name` that an answer's `headers` set, whole.
fn cookie(headers: &str, name: &str) -> Option<String> {
    headers.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        let value = value.trim();
        (key.eq_ignore_ascii_case("set-cookie") && value.starts_with(&format!("{name}=")))
            .then(|| value.to_owned())
    })
}

/// The answer's body for `value`.
fn pressing(value: &str) -> String {
    json!({"look": "l1", "value": value}).to_string()
}

/// The header lines a page of the sites' own sends its answer with.
fn from_a_page(port: u16, pressed: &str) -> String {
    format!(
        "Origin: http://sites.localhost:{port}\r\nSec-Fetch-Site: same-origin\r\n\
         Cookie: {pressed}\r\nContent-Type: application/json\r\n"
    )
}

#[test]
fn only_a_page_asking_from_this_origin_is_given_what_its_answers_carry() {
    let served = served("given");
    let ours = "Sec-Fetch-Site: same-origin\r\n";
    let (status, headers, _) = asked(served.port, "GET /answer", ours, "");
    assert_eq!(status, "HTTP/1.1 204 No Content");
    let given = cookie(&headers, "sites-answer").expect("the page was given nothing");
    assert!(
        given.contains("Path=/answer") && given.contains("HttpOnly"),
        "{given}"
    );
    assert!(given.contains("SameSite=Strict"), "{given}");
    let elsewhere = "Sec-Fetch-Site: cross-site\r\n";
    let (status, headers, body) = asked(served.port, "GET /answer", elsewhere, "");
    assert_eq!(status, "HTTP/1.1 403 Forbidden", "{body}");
    assert!(body.contains("\"refused\":\"site\""), "{body}");
    assert_eq!(
        cookie(&headers, "sites-answer"),
        None,
        "another site was given it"
    );
    for page in ["GET /", "GET /note/"] {
        let (_, headers, _) = asked(served.port, page, "", "");
        assert_eq!(
            cookie(&headers, "sites-answer"),
            None,
            "{page} was given it unasked"
        );
    }
}

#[test]
fn an_answer_pressed_on_a_page_is_carried_once_as_the_launcher_carries_it() {
    let served = served("carried");
    let pressed = given(served.port);
    let page = from_a_page(served.port, &pressed);
    let (status, _, body) = asked(served.port, "POST /answer", &page, &pressing("yes"));
    assert_eq!(status, "HTTP/1.1 200 OK", "{body}");
    let steers = served.steered.lock().expect("the steers").clone();
    assert_eq!(steers.len(), 1, "{steers:?}");
    assert_eq!(steers[0]["address"], "ABC123", "the steer went elsewhere");
    let steer = &steers[0]["steer"];
    assert_eq!(
        (
            &steer["act"],
            &steer["look"],
            &steer["value"],
            &steer["from"]
        ),
        (
            &json!("answer"),
            &json!("l1"),
            &json!("yes"),
            &json!("joel")
        ),
        "{steer}"
    );
    assert!(
        steer["at"].as_str().is_some_and(|at| at.ends_with('Z')),
        "{steer}"
    );
    assert_eq!(
        steer["via"], "sites",
        "the steer didn't say it came by the sites"
    );
    let (status, _, body) = asked(served.port, "POST /answer", &page, &pressing("no"));
    assert_eq!(
        status, "HTTP/1.1 409 Conflict",
        "a look was answered twice: {body}"
    );
    assert!(body.contains("\"answered\""), "{body}");
    assert_eq!(served.steered.lock().expect("the steers").len(), 1);
}

#[test]
fn an_answer_whose_session_ended_goes_to_its_quests_as_the_launchers_does() {
    let served = served("ended");
    let page = from_a_page(served.port, &given(served.port));
    let pressed = json!({"look": "ended", "value": "yes"}).to_string();
    let (status, _, body) = asked(served.port, "POST /answer", &page, &pressed);
    assert_eq!(status, "HTTP/1.1 200 OK", "{body}");
    let told: Vec<Value> = served
        .steered
        .lock()
        .expect("the steers")
        .iter()
        .map(|steer| steer["address"].clone())
        .collect();
    assert_eq!(
        told,
        [json!("GONE12"), json!("LIVE12")],
        "the answer wasn't tried at its own session, then at its quest's"
    );
}

#[test]
fn an_answer_from_anywhere_else_is_refused_and_never_carried() {
    let served = served("refused");
    let pressed = given(served.port);
    let port = served.port;
    let page = from_a_page(port, &pressed);
    let json = "Content-Type: application/json\r\n";
    let ours = format!("Origin: http://sites.localhost:{port}\r\nSec-Fetch-Site: same-origin\r\n");
    for (extra, value, word) in [
        // A form sends no JSON, so an injected form can't press.
        (
            format!("{ours}Cookie: {pressed}\r\nContent-Type: text/plain\r\n"),
            "yes",
            "type",
        ),
        // A session's file under the sites' names runs in an origin of no one's.
        (
            format!("Origin: null\r\nSec-Fetch-Site: cross-site\r\nCookie: {pressed}\r\n{json}"),
            "yes",
            "origin",
        ),
        // A page elsewhere, carrying the cookie as a request it sends would.
        (
            format!(
                "Origin: http://evil.example\r\nSec-Fetch-Site: cross-site\r\nCookie: {pressed}\r\n{json}"
            ),
            "yes",
            "origin",
        ),
        (
            format!(
                "Origin: http://sites.localhost:{port}\r\nSec-Fetch-Site: same-site\r\nCookie: {pressed}\r\n{json}"
            ),
            "yes",
            "site",
        ),
        // A document, same-site with the sites but never same-origin.
        (
            format!(
                "Origin: http://documents.sites.localhost:{port}\r\nSec-Fetch-Site: same-site\r\nCookie: {pressed}\r\n{json}"
            ),
            "yes",
            "origin",
        ),
        (format!("{ours}{json}"), "yes", "cookie"),
        (
            format!("{ours}Cookie: sites-answer=0\r\n{json}"),
            "yes",
            "cookie",
        ),
        (page.clone(), "maybe", "unoffered"),
        (page.clone(), "Merge It", "unoffered"),
    ] {
        let (status, _, body) = asked(port, "POST /answer", &extra, &pressing(value));
        assert!(status.starts_with("HTTP/1.1 4"), "{word}: {status}");
        assert!(
            body.contains(&format!("\"refused\":\"{word}\"")),
            "{word}: {body}"
        );
    }
    let unknown = json!({"look": "l2", "value": "yes"}).to_string();
    let (status, _, _) = asked(port, "POST /answer", &page, &unknown);
    assert_eq!(
        status, "HTTP/1.1 404 Not Found",
        "a look nobody holds was answered"
    );
    let (status, _, _) = asked(port, "PUT /answer", &page, "");
    assert_eq!(status, "HTTP/1.1 405 Method Not Allowed");
    // Under the documents' name a document's own origin is the one asked,
    // so there an answer is never taken, nor its cookie given.
    let documents = format!(
        "Origin: http://documents.sites.localhost:{port}\r\nSec-Fetch-Site: same-origin\r\n\
         Cookie: {pressed}\r\n{json}"
    );
    let named = "documents.sites.localhost";
    let (status, _, _) = asked_at(named, port, "POST /answer", (&documents, &pressing("yes")));
    assert_eq!(
        status, "HTTP/1.1 403 Forbidden",
        "a document pressed an answer"
    );
    let (status, headers, _) = asked_at(named, port, "GET /answer", (&documents, ""));
    assert_eq!(
        status, "HTTP/1.1 403 Forbidden",
        "a document asked for the cookie"
    );
    assert_eq!(
        cookie(&headers, "sites-answer"),
        None,
        "a document was given it"
    );
    assert!(
        served.steered.lock().expect("the steers").is_empty(),
        "a refused answer was carried"
    );
}
