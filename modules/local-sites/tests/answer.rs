//! An answer pressed on a page of the sites' own, as the server's unit
//! runs it, beside one stand-in daemon that holds the looks as the to-do
//! daemon does and takes a steer as the node does: carried once, with the
//! steer the launcher sends, and refused from a document's origin, from
//! another site, without the cookie a page of the sites' own is given, for a
//! value the look doesn't offer, not sent as JSON, and a second time.
//!
//! The server takes answers only where the kernel tells who holds a
//! connection (src/peer.rs). The machine's own would tell the test's
//! process as whatever unit runs it, an agent's where an agent runs it, so
//! each server is given a folder laid out as `/proc` instead, telling each
//! connection the test opens as held by the person's browser, or by an
//! agent's process where a case says so. They run alike anywhere.

use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::os::unix::fs::{MetadataExt, symlink};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

mod common;

/// The ssh-tpm-agent gate's places, which the server judges agents' by.
const GATE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../ssh-tpm-agent/gate.conf");

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

/// The look `id` the stand-in holds, as `look()` but for who it's from:
/// `l1` is ABC123's; `ended` is a session's that has ended, on a quest
/// whose own session goes on; `away` is one whose machine is out of reach,
/// on that quest; `nowhere` names no session; and `slow` is SLOW12's, whose
/// steer is answered only once l1's is carried.
fn held(id: &str) -> Option<Value> {
    let mut look = look();
    look["id"] = json!(id);
    let from = |look: &mut Value, address: &str| {
        look["address"] = json!(address);
        look["quest"] = json!("QEST-20261124-LIVE12");
    };
    match id {
        "l1" => {}
        "ended" => from(&mut look, "gone12"),
        "away" => from(&mut look, "away12"),
        "slow" => look["address"] = json!("slow12"),
        "nowhere" => {
            look.as_object_mut()?.remove("address");
        }
        _ => return None,
    }
    Some(look)
}

/// A daemon standing in for the to-do daemon and the node at once: it
/// answers a get of a look it holds (`held`) with that look, and a steer
/// as the node does, refusing one to GONE12, whose session has ended, and
/// one to AWAY12, whose machine is out of reach, and keeping every steer
/// it was asked.
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
            Some("get") => {
                let key = asked["keys"][0].as_str().unwrap_or("");
                let records: Vec<Value> = key
                    .strip_prefix("look:")
                    .and_then(held)
                    .map(|look| json!({"key": key, "kind": "look", "look": look}))
                    .into_iter()
                    .collect();
                json!({"id": id, "records": records})
            }
            Some("steer") => {
                steered.lock().expect("the steers").push(asked.clone());
                let why = match asked["address"].as_str() {
                    Some("GONE12") => Some("no session goes by GONE12 now"),
                    Some("AWAY12") => Some("AWAY12's machine is out of reach"),
                    Some("SLOW12") => {
                        held_until_l1_is_steered(steered);
                        None
                    }
                    _ => None,
                };
                why.map_or_else(
                    || json!({"id": id, "steered": {"machine": "here"}}),
                    |why| json!({"id": id, "error": {"code": "refused", "message": why}}),
                )
            }
            _ => json!({"id": id, "error": {"code": "unknown-ask"}}),
        };
        if writeln!(out, "{answer}").is_err() {
            return;
        }
    }
}

/// The person's service manager's unit, as a cgroup path begins below it.
const MANAGER: &str = "0::/user.slice/user-1000.slice/user@1000.service/";

/// Who holds a connection the test opens, as the stood-in kernel tells it.
#[derive(Clone, Copy)]
enum Holder {
    /// The person's browser, in an app's scope below the service manager.
    Person,
    /// A command an agent ran, in the queue agents' commands wait in.
    Agent,
    /// Nothing the kernel tells: the connection is in none of its tables.
    Untold,
}

impl Holder {
    /// The process told as holding it, none for a connection untold.
    const fn pid(self) -> Option<u32> {
        match self {
            Self::Person => Some(910),
            Self::Agent => Some(920),
            Self::Untold => None,
        }
    }
}

/// A folder laid out as `/proc`, as the server reads the kernel's.
struct Kernel {
    root: PathBuf,
    /// The inode the next connection's socket is given.
    inode: AtomicU64,
}

impl Kernel {
    /// One telling a service manager, the person's browser below it and
    /// an agent's command, all of this account, and no connection yet.
    fn new(root: PathBuf) -> Self {
        std::fs::create_dir_all(root.join("net")).expect("its net folder");
        std::fs::create_dir_all(root.join("self")).expect("its self");
        let head = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when \
                    retrnsmt   uid  timeout inode\n";
        std::fs::write(root.join("net/tcp"), head).expect("its table");
        let kernel = Self {
            root,
            inode: AtomicU64::new(7000),
        };
        kernel.process(900, ("systemd", 1), "init.scope");
        kernel.process(910, ("chromium", 900), "app.slice/app-chromium.scope");
        kernel.process(920, ("curl", 900), "jobs.slice/run-u7.service");
        kernel
    }

    /// Process `pid`, named and child of the parent `(name, parent)`, in
    /// the unit `unit` below the service manager.
    fn process(&self, pid: u32, (name, parent): (&str, u32), unit: &str) {
        let at = self.root.join(pid.to_string());
        std::fs::create_dir_all(at.join("fd")).expect("its fd folder");
        let status = format!("Name:\t{name}\nPPid:\t{parent}\n");
        std::fs::write(at.join("status"), status).expect("its status");
        std::fs::write(at.join("cgroup"), format!("{MANAGER}{unit}\n")).expect("its cgroup");
    }

    /// The connection `stream` entered in the kernel's table as this
    /// account's, its socket held open by `holder`.
    fn opened(&self, stream: &TcpStream, holder: Holder) {
        let Some(pid) = holder.pid() else {
            return;
        };
        let inode = self.inode.fetch_add(1, Ordering::Relaxed);
        let ends = (stream.local_addr(), stream.peer_addr());
        let (Ok(own), Ok(server)) = ends else {
            panic!("the connection has no two ends");
        };
        let uid = std::fs::metadata(&self.root)
            .expect("the kernel's folder")
            .uid();
        let (own, server) = (written(own), written(server));
        let line = format!(
            "   1: {own} {server} 01 00000000:00000000 00:00000000 00000000 {uid:>5}        0 \
             {inode} 1 0000000000000000 20 4 30 10 -1\n"
        );
        let mut table = OpenOptions::new()
            .append(true)
            .open(self.root.join("net/tcp"))
            .expect("its table");
        table
            .write_all(line.as_bytes())
            .expect("the connection entered");
        let fd = self.root.join(format!("{pid}/fd/{inode}"));
        symlink(format!("socket:[{inode}]"), fd).expect("its socket held");
    }
}

/// An address as the kernel's table writes one: its address's bytes in the
/// machine's own order, its port in hexadecimal.
fn written(address: SocketAddr) -> String {
    let SocketAddr::V4(address) = address else {
        panic!("the test connects over IPv4");
    };
    let word = u32::from_ne_bytes(address.ip().octets());
    format!("{word:08X}:{:04X}", address.port())
}

/// The server, stopped and its folder taken away once it's dropped. A server
/// that died answers nothing, and must not pass for one that refused.
struct Served {
    child: Child,
    port: u16,
    folder: PathBuf,
    steered: Arc<Mutex<Vec<Value>>>,
    kernel: Kernel,
}

impl Drop for Served {
    fn drop(&mut self) {
        common::stop(&mut self.child, &self.folder);
    }
}

/// One server starting at a time, so no two are given the same free port.
static STARTING: Mutex<()> = Mutex::new(());

fn served(name: &str) -> Served {
    served_by(name, Path::new(GATE))
}

/// A server as `served` starts one, judging agents' by the gate `places`.
fn served_by(name: &str, places: &Path) -> Served {
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
    let kernel = Kernel::new(folder.join("proc"));
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
        "answers": {"looks": socket, "node": socket, "processes": kernel.root, "places": places},
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
        kernel,
    }
}

/// What the person's browser asking `ask` with `extra` header lines and
/// `body` is answered: its status line, its header lines and its body.
fn asked(served: &Served, ask: &str, extra: &str, body: &str) -> (String, String, String) {
    asked_by(
        served,
        (Holder::Person, "sites.localhost"),
        ask,
        (extra, body),
    )
}

/// What a request of `ask` that `holder` sends naming the server `name` is
/// answered, with `extra` header lines and `body`.
fn asked_by(
    served: &Served,
    (holder, name): (Holder, &str),
    ask: &str,
    (extra, body): (&str, &str),
) -> (String, String, String) {
    let port = served.port;
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("it connects");
    served.kernel.opened(&stream, holder);
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
fn given(served: &Served) -> String {
    let ours = "Sec-Fetch-Site: same-origin\r\n";
    let (status, headers, body) = asked(served, "GET /answer", ours, "");
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

/// Waits, as a session slow to take a steer, until l1's answer was steered
/// beside it, or the test's patience runs out.
fn held_until_l1_is_steered(steered: &Mutex<Vec<Value>>) {
    let began = Instant::now();
    while began.elapsed() < PATIENCE {
        let steers = steered.lock().expect("the steers");
        if steers.iter().any(|steer| steer["address"] == "ABC123") {
            return;
        }
        drop(steers);
        thread::sleep(Duration::from_millis(10));
    }
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
fn a_gate_it_cant_read_leaves_the_sites_served_and_answers_in_for_you() {
    let gate = std::env::temp_dir().join(format!("local-sites-gate-{}", std::process::id()));
    std::fs::write(&gate, "allow /app\\.slice/\n").expect("a gate refusing no place");
    let served = served_by("ungated", &gate);
    let (status, _, body) = asked(&served, "GET /", "", "");
    assert_eq!(status, "HTTP/1.1 200 OK", "the home wasn't served: {body}");
    let ours = "Sec-Fetch-Site: same-origin\r\n";
    let (status, _, body) = asked(&served, "GET /answer", ours, "");
    assert_eq!(
        status, "HTTP/1.1 404 Not Found",
        "answers were taken: {body}"
    );
    let _ = std::fs::remove_file(&gate);
}

#[test]
fn only_a_page_asking_from_this_origin_is_given_what_its_answers_carry() {
    let served = served("given");
    let ours = "Sec-Fetch-Site: same-origin\r\n";
    let (status, headers, _) = asked(&served, "GET /answer", ours, "");
    assert_eq!(status, "HTTP/1.1 204 No Content");
    let given = cookie(&headers, "sites-answer").expect("the page was given nothing");
    assert!(
        given.contains("Path=/answer") && given.contains("HttpOnly"),
        "{given}"
    );
    assert!(given.contains("SameSite=Strict"), "{given}");
    let elsewhere = "Sec-Fetch-Site: cross-site\r\n";
    let (status, headers, body) = asked(&served, "GET /answer", elsewhere, "");
    assert_eq!(status, "HTTP/1.1 403 Forbidden", "{body}");
    assert!(body.contains("\"refused\":\"site\""), "{body}");
    assert_eq!(
        cookie(&headers, "sites-answer"),
        None,
        "another site was given it"
    );
    for page in ["GET /", "GET /note/"] {
        let (_, headers, _) = asked(&served, page, "", "");
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
    let pressed = given(&served);
    let page = from_a_page(served.port, &pressed);
    let (status, _, body) = asked(&served, "POST /answer", &page, &pressing("yes"));
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
    let (status, _, body) = asked(&served, "POST /answer", &page, &pressing("no"));
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
    let page = from_a_page(served.port, &given(&served));
    let pressed = json!({"look": "ended", "value": "yes"}).to_string();
    let (status, _, body) = asked(&served, "POST /answer", &page, &pressed);
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
fn an_answer_a_session_refuses_for_another_reason_goes_nowhere_else_and_may_come_again() {
    let served = served("away");
    let page = from_a_page(served.port, &given(&served));
    let pressed = json!({"look": "away", "value": "yes"}).to_string();
    for tries in 1..=2 {
        let (status, _, body) = asked(&served, "POST /answer", &page, &pressed);
        assert_eq!(status, "HTTP/1.1 502 Bad Gateway", "{body}");
        assert!(body.contains("\"refused\":\"unsteered\""), "{body}");
        let told: Vec<Value> = served
            .steered
            .lock()
            .expect("the steers")
            .iter()
            .map(|steer| steer["address"].clone())
            .collect();
        assert_eq!(
            told,
            vec![json!("AWAY12"); tries],
            "an answer its session may still take went to its quest's, or wasn't sent again"
        );
    }
    let nowhere = json!({"look": "nowhere", "value": "yes"}).to_string();
    let (status, _, body) = asked(&served, "POST /answer", &page, &nowhere);
    assert_eq!(status, "HTTP/1.1 409 Conflict", "{body}");
    assert!(body.contains("\"refused\":\"unaddressed\""), "{body}");
}

#[test]
fn a_session_slow_to_answer_holds_up_no_other_looks_press_and_its_own_once() {
    let served = served("slow");
    let page = from_a_page(served.port, &given(&served));
    let slow = json!({"look": "slow", "value": "yes"}).to_string();
    thread::scope(|both| {
        let first = both.spawn(|| asked(&served, "POST /answer", &page, &slow));
        let began = Instant::now();
        while !served
            .steered
            .lock()
            .expect("the steers")
            .iter()
            .any(|steer| steer["address"] == "SLOW12")
        {
            assert!(
                began.elapsed() < PATIENCE,
                "the slow look's answer never went"
            );
            thread::sleep(Duration::from_millis(10));
        }
        let (status, _, body) = asked(&served, "POST /answer", &page, &slow);
        assert_eq!(status, "HTTP/1.1 409 Conflict", "{body}");
        assert!(body.contains("\"refused\":\"answering\""), "{body}");
        // The slow session takes its steer only once this one is carried.
        let (status, _, body) = asked(&served, "POST /answer", &page, &pressing("yes"));
        assert_eq!(status, "HTTP/1.1 200 OK", "{body}");
        let (status, _, body) = first.join().expect("the first press");
        assert_eq!(
            status, "HTTP/1.1 200 OK",
            "the slow look's press was held up: {body}"
        );
    });
}

#[test]
fn an_answer_longer_than_a_look_and_a_value_or_saying_no_length_is_never_read() {
    let served = served("long");
    let pressed = given(&served);
    let port = served.port;
    let page = from_a_page(port, &pressed);
    let long = format!("{{\"look\": \"l1\", \"value\": \"{}\"}}", "y".repeat(5000));
    let (status, _, _) = asked(&served, "POST /answer", &page, &long);
    assert_eq!(
        status, "HTTP/1.1 413 Content Too Large",
        "a long answer was read"
    );
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("it connects");
    served.kernel.opened(&stream, Holder::Person);
    write!(
        stream,
        "POST /answer HTTP/1.1\r\nHost: sites.localhost:{port}\r\n{page}\r\n"
    )
    .expect("it asks");
    let mut answer = String::new();
    stream.read_to_string(&mut answer).expect("it's answered");
    assert!(
        answer.starts_with("HTTP/1.1 413 Content Too Large"),
        "an answer saying no length was read: {answer}"
    );
    assert!(
        served.steered.lock().expect("the steers").is_empty(),
        "an answer never read was carried"
    );
}

#[test]
fn an_answer_from_anywhere_else_is_refused_and_never_carried() {
    let served = served("refused");
    let pressed = given(&served);
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
        let (status, _, body) = asked(&served, "POST /answer", &extra, &pressing(value));
        assert!(status.starts_with("HTTP/1.1 4"), "{word}: {status}");
        assert!(
            body.contains(&format!("\"refused\":\"{word}\"")),
            "{word}: {body}"
        );
    }
    let unknown = json!({"look": "l2", "value": "yes"}).to_string();
    let (status, _, _) = asked(&served, "POST /answer", &page, &unknown);
    assert_eq!(
        status, "HTTP/1.1 404 Not Found",
        "a look nobody holds was answered"
    );
    let (status, _, _) = asked(&served, "PUT /answer", &page, "");
    assert_eq!(status, "HTTP/1.1 405 Method Not Allowed");
    // Under the documents' name a document's own origin is the one asked,
    // so there an answer is never taken, nor its cookie given.
    let documents = format!(
        "Origin: http://documents.sites.localhost:{port}\r\nSec-Fetch-Site: same-origin\r\n\
         Cookie: {pressed}\r\n{json}"
    );
    let named = (Holder::Person, "documents.sites.localhost");
    let (status, _, _) = asked_by(
        &served,
        named,
        "POST /answer",
        (&documents, &pressing("yes")),
    );
    assert_eq!(
        status, "HTTP/1.1 403 Forbidden",
        "a document pressed an answer"
    );
    let (status, headers, _) = asked_by(&served, named, "GET /answer", (&documents, ""));
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

#[test]
fn a_press_the_kernel_tells_as_an_agents_or_no_ones_is_refused() {
    let served = served("agents");
    let page = from_a_page(served.port, &given(&served));
    let ours = "Sec-Fetch-Site: same-origin\r\n";
    for (holder, word) in [(Holder::Agent, "agent"), (Holder::Untold, "untold")] {
        // Every header a page sends, the cookie included, is an agent's to
        // copy; only the kernel's word on who holds the connection differs.
        let by = (holder, "sites.localhost");
        let (status, _, body) = asked_by(&served, by, "POST /answer", (&page, &pressing("yes")));
        assert_eq!(status, "HTTP/1.1 403 Forbidden", "{word}: {body}");
        assert!(
            body.contains(&format!("\"refused\":\"{word}\"")),
            "{word}: {body}"
        );
        let (status, headers, body) = asked_by(&served, by, "GET /answer", (ours, ""));
        assert_eq!(status, "HTTP/1.1 403 Forbidden", "{word}: {body}");
        assert_eq!(
            cookie(&headers, "sites-answer"),
            None,
            "{word}: an agent was given the cookie"
        );
    }
    assert!(
        served.steered.lock().expect("the steers").is_empty(),
        "an agent's press was carried"
    );
}
