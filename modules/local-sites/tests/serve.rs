//! The server as its unit runs it, on a port of its own, beside a
//! stand-in daemon speaking the daemon interface on a socket: files from
//! the documents and from a mount, a file the browser holds answered by
//! its tag alone and one written since sent again, the door shut to a
//! stranger's name and a stranger's page, the documents on an origin of
//! their own that reads nothing of the sites', a document sandboxed and the
//! sites' own pages held to their own scripts, and a feed whose pages share
//! one watch on the daemon, hear each change, can't act, and are told
//! when the daemon is gone.

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

mod common;

/// How long a test waits for the server, or for a line it's owed.
const PATIENCE: Duration = Duration::from_secs(10);

/// The name the documents are served under.
const DOCUMENTS: &str = "documents.localhost";

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
        common::stop(&mut self.child, &self.folder);
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
    std::fs::write(kit.join("index.html"), "<h1>kit</h1>").expect("the kit's page");
    std::fs::create_dir_all(folder.join("home")).expect("the home");
    std::fs::write(folder.join("home/index.html"), "<h1>home</h1>").expect("its page");
    let socket = folder.join("node.sock");
    let stand_in = StandIn::start(&socket);
    // A port given up is free for any test to be given next, so one server
    // starts at a time, from its port's choosing until it listens; and one
    // that can't listen anyway, its port taken by another's reader, starts
    // again on another.
    let _starting = STARTING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for _ in 0..TRIES {
        let (child, port) = started(&folder, &socket);
        if let Some(child) = listening(child, port) {
            return Served {
                child,
                port,
                folder,
                stand_in,
            };
        }
    }
    panic!("the server never listened, on {TRIES} ports");
}

/// One server starting at a time, so no two are given the same free port.
static STARTING: Mutex<()> = Mutex::new(());

/// How many ports a server is started on before the test gives up.
const TRIES: usize = 5;

/// A server started on a port nobody held, with the documents in `folder`.
fn started(folder: &Path, socket: &Path) -> (Child, u16) {
    let (documents, kit) = (folder.join("sites"), folder.join("kit"));
    let port = TcpListener::bind("127.0.0.1:0")
        .and_then(|free| free.local_addr())
        .expect("a free port")
        .port();
    let config = json!({
        "listen": format!("127.0.0.1:{port}"),
        "names": ["localhost"],
        "documentsName": DOCUMENTS,
        "documents": documents,
        "home": folder.join("home"),
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
    (child, port)
}

/// The server once it listens on `port`, or none when it ended first.
fn listening(mut child: Child, port: u16) -> Option<Child> {
    let started = Instant::now();
    while TcpStream::connect(("127.0.0.1", port)).is_err() {
        if child.try_wait().ok().flatten().is_some() {
            return None;
        }
        assert!(started.elapsed() < PATIENCE, "the server never listened");
        // Nothing to wait on but the port itself, so it's looked at again.
        thread::sleep(Duration::from_millis(20));
    }
    Some(child)
}

/// What a GET of `path`, naming the server `host`, is answered: its status
/// line and its body.
fn get(port: u16, host: &str, path: &str) -> (String, String) {
    let (status, _, body) = asked(port, host, &format!("GET {path}"), "");
    (status, body)
}

/// What `ask`, a method and a path, with `extra` header lines is answered:
/// its status line, its header lines and its body.
fn asked(port: u16, host: &str, ask: &str, extra: &str) -> (String, String, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("it connects");
    write!(stream, "{ask} HTTP/1.1\r\nHost: {host}\r\n{extra}\r\n").expect("it asks");
    let mut answer = String::new();
    stream.read_to_string(&mut answer).expect("it's answered");
    let (head, body) = answer.split_once("\r\n\r\n").unwrap_or((&answer, ""));
    let (status, headers) = head.split_once("\r\n").unwrap_or((head, ""));
    (status.to_owned(), headers.to_owned(), body.to_owned())
}

/// The value of the header `name` among `headers`, however its name is cased.
fn header<'a>(headers: &'a str, name: &str) -> Option<&'a str> {
    headers.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.eq_ignore_ascii_case(name).then(|| value.trim())
    })
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
    let documents = format!("{DOCUMENTS}:{}", served.port);
    assert_eq!(
        get(served.port, &documents, "/mycelium/"),
        ("HTTP/1.1 200 OK".to_owned(), "<h1>map</h1>".to_owned())
    );
    assert_eq!(get(served.port, &host, "/kit/theme.css").1, ":root {}");
    assert_eq!(
        get(served.port, &documents, "/kit/theme.css").1,
        ":root {}",
        "a document reads the kit beside it"
    );
    assert_eq!(
        get(served.port, &documents, "/mycelium").0,
        "HTTP/1.1 301 Moved Permanently"
    );
    assert_eq!(
        get(served.port, &host, "/nothing").0,
        "HTTP/1.1 404 Not Found"
    );
}

#[test]
fn a_folder_asked_without_its_slash_is_sent_on_from_this_server_alone() {
    let served = served("folder");
    let documents = format!("{DOCUMENTS}:{}", served.port);
    for path in ["/mycelium", "//mycelium"] {
        let (status, headers, _) = asked(served.port, &documents, &format!("GET {path}"), "");
        assert_eq!(status, "HTTP/1.1 301 Moved Permanently", "{path}");
        let location = header(&headers, "location");
        // `//mycelium/` would send the browser to the host `mycelium`; one
        // from `./` stays where it was asked.
        assert_eq!(
            location,
            Some("./mycelium/"),
            "{path} was sent to another host"
        );
    }
    // A folder a session could make whose name a browser would read as a
    // host, `/\evil.example/`, is refused before it could be sent on.
    std::fs::create_dir_all(served.folder.join("sites/\\evil.example")).expect("its folder");
    for path in ["/\\evil.example", "/%5Cevil.example"] {
        let (status, headers, _) = asked(served.port, &documents, &format!("GET {path}"), "");
        assert_eq!(status, "HTTP/1.1 403 Forbidden", "{path}");
        assert_eq!(header(&headers, "location"), None, "{path} was sent on");
    }
}

#[test]
fn a_file_unchanged_since_its_last_visit_is_not_sent_again() {
    let served = served("unchanged");
    let sites = format!("localhost:{}", served.port);
    let documents = format!("{DOCUMENTS}:{}", served.port);
    for (host, path, kind) in [
        (&documents, "/mycelium/", "text/html; charset=utf-8"),
        (&sites, "/kit/theme.css", "text/css; charset=utf-8"),
    ] {
        let (_, headers, sent) = asked(served.port, host, &format!("GET {path}"), "");
        let tag = header(&headers, "etag").expect("a file is sent with its tag");
        let (status, asked_head, body) = asked(served.port, host, &format!("HEAD {path}"), "");
        assert_eq!(
            (status.as_str(), body.as_str()),
            ("HTTP/1.1 200 OK", ""),
            "HEAD {path}"
        );
        for said in [&headers, &asked_head] {
            let length = sent.len().to_string();
            let told = ["content-type", "content-length", "cache-control", "etag"]
                .map(|name| header(said, name));
            let owed = [
                Some(kind),
                Some(length.as_str()),
                Some("no-cache"),
                Some(tag),
            ];
            assert_eq!(told, owed, "{path} was sent without what it's owed");
        }
        for ask in [format!("GET {path}"), format!("HEAD {path}")] {
            let held = format!("If-None-Match: {tag}\r\n");
            let (status, headers, body) = asked(served.port, host, &ask, &held);
            assert_eq!(status, "HTTP/1.1 304 Not Modified", "{ask} was sent again");
            assert_eq!(body, "", "{ask} was answered unchanged with a body");
            assert_eq!(header(&headers, "etag"), Some(tag), "{ask} lost its tag");
            for unsaid in ["content-length", "content-type"] {
                let said = header(&headers, unsaid);
                assert_eq!(said, None, "{ask} was answered unchanged with a {unsaid}");
            }
        }
        let listed = format!("If-None-Match: \"other\", W/{tag}\r\n");
        let (status, _, _) = asked(served.port, host, &format!("GET {path}"), &listed);
        let unchanged = "HTTP/1.1 304 Not Modified";
        assert_eq!(status, unchanged, "{path} missed its weak tag in a list");
    }
}

#[test]
fn a_file_written_since_its_last_visit_is_sent_again() {
    let served = served("written");
    let host = format!("{DOCUMENTS}:{}", served.port);
    let page = served.folder.join("sites/mycelium/index.html");
    // Each rewrite keeps the page's size and puts its write time back, so a
    // tag of the size and that time alone can't see it. What the tag sees
    // instead here is the change time; within one tick of the clock, its
    // inode would see a rename too, which no test can make the clock do.
    for (how, page_now) in [("in place", "<h1>pam</h1>"), ("by rename", "<h1>amp</h1>")] {
        let (_, headers, _) = asked(served.port, &host, "GET /mycelium/", "");
        let tag = header(&headers, "etag").expect("the page's tag").to_owned();
        let written = std::fs::metadata(&page).and_then(|about| about.modified());
        let written = written.expect("the page's time");
        if how == "by rename" {
            let beside = page.with_extension("new");
            std::fs::write(&beside, page_now).expect("the new page is written");
            std::fs::rename(&beside, &page).expect("it takes the page's place");
        } else {
            std::fs::write(&page, page_now).expect("the page is rewritten");
        }
        let file = std::fs::File::options().write(true).open(&page);
        let set = file.and_then(|file| file.set_modified(written));
        set.expect("the page's time is put back");
        let held = format!("If-None-Match: {tag}\r\n");
        let (status, headers, body) = asked(served.port, &host, "GET /mycelium/", &held);
        assert_eq!(
            (status.as_str(), body.as_str()),
            ("HTTP/1.1 200 OK", page_now),
            "a page rewritten {how} was answered as unchanged"
        );
        let moved = header(&headers, "etag") != Some(tag.as_str());
        assert!(moved, "a page rewritten {how} kept its tag");
    }
}

#[test]
fn a_file_that_cant_be_read_is_said_so() {
    use std::os::unix::fs::PermissionsExt;
    let served = served("unreadable");
    let host = format!("{DOCUMENTS}:{}", served.port);
    let page = served.folder.join("sites/mycelium/index.html");
    let shut = std::fs::Permissions::from_mode(0o000);
    std::fs::set_permissions(&page, shut).expect("the page is shut");
    let (status, body) = get(served.port, &host, "/mycelium/");
    assert_eq!(
        (status.as_str(), body.as_str()),
        (
            "HTTP/1.1 500 Internal Server Error",
            "the file couldn't be read\n"
        ),
        "a page that can't be read was answered otherwise"
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

/// The policy a session's file is served under by the sites' names.
const SANDBOXED: &str = "sandbox allow-scripts allow-popups allow-forms; frame-ancestors 'self'";

/// The policy everything is served under by the documents' name.
const DOCUMENTED: &str =
    "sandbox allow-scripts allow-same-origin allow-popups allow-forms; frame-ancestors 'self'";

/// The policy every page of the sites' own is served under.
const OWN: &str = "script-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'";

#[test]
fn every_document_runs_sandboxed_whatever_it_is_and_however_its_answered() {
    let served = served("sandbox");
    let sites = format!("localhost:{}", served.port);
    let documents = format!("{DOCUMENTS}:{}", served.port);
    let mark = served.folder.join("sites/mycelium/mark.svg");
    std::fs::write(mark, "<svg><script>alert(1)</script></svg>").expect("a drawing");
    for (host, path, sandbox) in [
        (&documents, "/mycelium/", DOCUMENTED),
        (&documents, "/mycelium/mark.svg", DOCUMENTED),
        (&documents, "/kit/theme.css", DOCUMENTED),
        (&sites, "/mycelium/mark.svg", SANDBOXED),
    ] {
        let (_, headers, _) = asked(served.port, host, &format!("GET {path}"), "");
        let policy = header(&headers, "content-security-policy");
        assert_eq!(policy, Some(sandbox), "{path} at {host} wasn't sandboxed");
        let sniffed = header(&headers, "x-content-type-options");
        assert_eq!(
            sniffed,
            Some("nosniff"),
            "{path} may be read as another kind"
        );
        let tag = header(&headers, "etag").expect("its tag");
        let held = format!("If-None-Match: {tag}\r\n");
        let (status, kept, _) = asked(served.port, host, &format!("GET {path}"), &held);
        assert_eq!(status, "HTTP/1.1 304 Not Modified", "{path} was sent again");
        let policy = header(&kept, "content-security-policy");
        assert_eq!(
            policy,
            Some(sandbox),
            "{path} at {host} kept unchanged lost its sandbox"
        );
    }
}

#[test]
fn a_page_goes_to_the_origin_of_whoever_made_it_and_nowhere_else() {
    let served = served("across");
    let sites = format!("localhost:{}", served.port);
    let documents = format!("{DOCUMENTS}:{}", served.port);
    for (host, path, to) in [
        (
            &sites,
            "/mycelium/?at=2",
            format!("http://{documents}/mycelium/?at=2"),
        ),
        (&documents, "/", format!("http://{sites}/")),
        (&documents, "/kit/", format!("http://{sites}/kit/")),
    ] {
        let (status, headers, _) = asked(served.port, host, &format!("GET {path}"), "");
        assert_eq!(
            status, "HTTP/1.1 307 Temporary Redirect",
            "{path} at {host}"
        );
        let location = header(&headers, "location");
        assert_eq!(location, Some(to.as_str()), "{path} at {host} went astray");
    }
    // A path not from `/` is refused by the answer before any redirect, so
    // these show that refusal; the door's own is pinned in door.rs.
    for whole in ["http://evil.example/mycelium/", "evil.example/mycelium/"] {
        let (status, headers, _) = asked(served.port, &sites, &format!("GET {whole}"), "");
        assert_eq!(status, "HTTP/1.1 400 Bad Request", "{whole} was taken");
        assert_eq!(header(&headers, "location"), None, "{whole} was sent on");
    }
}

#[test]
fn a_sessions_script_is_never_served_by_the_sites_names() {
    let served = served("script");
    let script = served.folder.join("sites/notes.js");
    std::fs::write(script, "fetch('/feed/node');").expect("a session's script");
    let sites = format!("localhost:{}", served.port);
    let (status, _) = get(served.port, &sites, "/notes.js");
    assert_eq!(
        status, "HTTP/1.1 403 Forbidden",
        "a page of the sites' own could run it"
    );
    let documents = format!("{DOCUMENTS}:{}", served.port);
    let (status, _) = get(served.port, &documents, "/notes.js");
    assert_eq!(
        status, "HTTP/1.1 200 OK",
        "its own document couldn't run it"
    );
}

#[test]
fn the_documents_origin_reads_nothing_of_the_sites() {
    let served = served("documents");
    let documents = format!("http://{DOCUMENTS}:{}", served.port);
    let why = page(served.port, "node", &documents).expect_err("refused");
    assert!(why.contains("403"), "a document read a feed: {why}");
    let mut request = format!("ws://{DOCUMENTS}:{}/feed/node", served.port)
        .into_client_request()
        .expect("a request");
    let origin = format!("http://localhost:{}", served.port);
    request
        .headers_mut()
        .insert("origin", origin.parse().expect("an origin"));
    let stream = TcpStream::connect(("127.0.0.1", served.port)).expect("it connects");
    let opened = tungstenite::client(request, stream);
    assert!(opened.is_err(), "a feed opened under the documents' name");
    let sites = format!("localhost:{}", served.port);
    let from = format!("Origin: {documents}\r\nSec-Fetch-Site: same-site\r\n");
    // Nothing here shares anything yet; this keeps it so.
    let (_, headers, _) = asked(served.port, &sites, "GET /kit/theme.css", &from);
    let shared = header(&headers, "access-control-allow-origin");
    assert_eq!(
        shared, None,
        "a page of the sites' own was shared with a document"
    );
}

#[test]
fn a_refusal_sent_before_its_body_is_read_still_arrives() {
    let served = served("unread");
    let sites = format!("localhost:{}", served.port);
    // A body of several segments the server never reads, and the two ends
    // `asked` writes after it: refused unread, closing with it waiting would
    // reset the connection under its answer.
    let body = "v".repeat(12 * 1024);
    let form = format!("Content-Length: {}\r\n\r\n{body}", body.len() + 2);
    let (status, _, _) = asked(served.port, &sites, "POST /", &form);
    assert_eq!(
        status, "HTTP/1.1 405 Method Not Allowed",
        "a refusal sent unread never arrived"
    );
}

#[test]
fn every_answer_keeps_its_origin_to_a_process_of_its_own() {
    let served = served("cluster");
    let sites = format!("localhost:{}", served.port);
    let documents = format!("{DOCUMENTS}:{}", served.port);
    let rebound = format!("evil.example:{}", served.port);
    for (host, path) in [
        (&sites, "/"),
        (&sites, "/kit/theme.css"),
        (&sites, "/mycelium/"),
        (&sites, "/nothing"),
        (&documents, "/mycelium/"),
        (&documents, "/"),
        (&documents, "/nothing"),
        (&rebound, "/"),
    ] {
        let (_, headers, _) = asked(served.port, host, &format!("GET {path}"), "");
        let kept = header(&headers, "origin-agent-cluster");
        assert_eq!(kept, Some("?1"), "{path} at {host} may share a process");
    }
}

#[test]
fn the_sites_own_pages_run_only_their_own_scripts_and_share_nothing() {
    let served = served("own");
    let host = format!("localhost:{}", served.port);
    let swept = "Cookie: __Host-sites-swept=1\r\n";
    for path in ["/kit/", "/kit/theme.css", "/"] {
        let (status, headers, _) = asked(served.port, &host, &format!("GET {path}"), swept);
        assert_eq!(status, "HTTP/1.1 200 OK", "{path}");
        let policy = header(&headers, "content-security-policy");
        assert_eq!(policy, Some(OWN), "{path} may run a script written into it");
    }
    let (_, home) = get(served.port, &host, "/");
    assert_eq!(home, "<h1>home</h1>", "the home's own folder comes first");
    let strip = served.folder.join("sites/strip.css");
    std::fs::write(strip, "h1 { display: none }").expect("a session's sheet");
    let (status, headers, body) = asked(served.port, &host, "GET /strip.css", "");
    assert_eq!(
        status, "HTTP/1.1 503 Service Unavailable",
        "a session's sheet stood in for one the home's builder hasn't written"
    );
    assert_eq!(header(&headers, "retry-after"), Some("2"), "no later");
    assert!(
        body.contains("journalctl --user -u local-sites-home"),
        "it didn't say where to look: {body}"
    );
    for path in ["/kit/theme.css", "/kit/", "/"] {
        let (_, headers, _) = asked(served.port, &host, &format!("GET {path}"), swept);
        let shared = header(&headers, "access-control-allow-origin");
        assert_eq!(shared, None, "{path} was shared with another origin");
    }
}

#[test]
fn a_browser_is_swept_once_by_the_first_page_of_the_sites_own() {
    let served = served("sweep");
    let host = format!("localhost:{}", served.port);
    let (_, first, _) = asked(served.port, &host, "GET /kit/", "");
    assert_eq!(header(&first, "clear-site-data"), Some("\"storage\""));
    let cookie = header(&first, "set-cookie").expect("the sweep is kept");
    // `__Host-` and `Secure`, so no other host can set it for this one.
    assert!(cookie.starts_with("__Host-sites-swept=1;"), "{cookie}");
    assert!(
        cookie.contains("Secure") && cookie.contains("HttpOnly") && cookie.contains("SameSite=Lax"),
        "{cookie}"
    );
    assert!(
        !cookie.to_ascii_lowercase().contains("domain"),
        "the documents' name would be sent it: {cookie}"
    );
    let tag = header(&first, "etag").expect("its tag");
    let held = format!("If-None-Match: {tag}\r\n");
    let (status, _, _) = asked(served.port, &host, "GET /kit/", &held);
    assert_eq!(
        status, "HTTP/1.1 200 OK",
        "a sweep went in an answer a browser may skip"
    );
    let planted = "Cookie: sites-swept=1\r\n";
    let (_, headers, _) = asked(served.port, &host, "GET /kit/", planted);
    let swept = header(&headers, "clear-site-data");
    assert_eq!(
        swept,
        Some("\"storage\""),
        "a plain cookie kept the sweep off"
    );
    let pictured = "Sec-Fetch-Site: cross-site\r\nSec-Fetch-Dest: image\r\n";
    let (_, headers, _) = asked(served.port, &host, "GET /kit/", pictured);
    let swept = header(&headers, "clear-site-data");
    assert_eq!(
        swept, None,
        "another site's picture of a page swept the browser"
    );
    let after = format!("{held}Cookie: theme=dark; __Host-sites-swept=1\r\n");
    let (status, headers, _) = asked(served.port, &host, "GET /kit/", &after);
    assert_eq!(
        status, "HTTP/1.1 304 Not Modified",
        "a swept browser was sent it again"
    );
    assert_eq!(
        header(&headers, "clear-site-data"),
        None,
        "a browser was swept twice"
    );
    let documents = format!("{DOCUMENTS}:{}", served.port);
    for (host, path) in [(&host, "/mycelium/"), (&documents, "/mycelium/")] {
        let (_, document, _) = asked(served.port, host, &format!("GET {path}"), "");
        let swept = header(&document, "clear-site-data");
        assert_eq!(swept, None, "a document at {host} swept the browser");
    }
}

#[test]
fn no_page_keeps_a_worker() {
    let served = served("worker");
    let sites = format!("localhost:{}", served.port);
    let documents = format!("{DOCUMENTS}:{}", served.port);
    let worker = served.folder.join("sites/mycelium/worker.js");
    std::fs::write(worker, "self.onfetch = () => {};").expect("a worker's script");
    for (host, path) in [
        (&documents, "/mycelium/worker.js"),
        (&sites, "/mycelium/worker.js"),
        (&sites, "/kit/theme.css"),
    ] {
        let ask = format!("GET {path}");
        let (status, _, body) = asked(served.port, host, &ask, "Service-Worker: script\r\n");
        // Answered with a worker that lets go of itself, since a refusal
        // would keep one a document installed before standing.
        assert_eq!(status, "HTTP/1.1 200 OK", "{path} at {host}");
        assert!(
            body.contains("self.registration.unregister()"),
            "a page at {host} could keep {path} as a worker: {body}"
        );
    }
    let (status, _) = get(served.port, &documents, "/mycelium/worker.js");
    assert_eq!(
        status, "HTTP/1.1 200 OK",
        "a script asked as a script is still sent"
    );
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
