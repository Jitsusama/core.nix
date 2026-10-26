//! session-backup against a stand-in for Google: its token endpoint and the
//! two Drive calls the backup may make. The stand-in refuses, and the
//! tests fail on, any call that isn't a token, a list scoped to the
//! backup's folder, or a file made in it.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

use serde_json::{Value, json};
use session_backup::backup::{self, Done};
use session_backup::config::Config;

const FOLDER: &str = "folder-0042";
const DAY: Duration = Duration::from_hours(24);

/// What the stand-in was asked, and how it is to answer.
#[derive(Default)]
struct Google {
    /// Each call, as `METHOD path`.
    calls: Vec<String>,
    /// Calls the backup must never make.
    refused: Vec<String>,
    /// Files made, by name: their id and bytes.
    files: HashMap<String, (String, Vec<u8>)>,
    /// Uploads opened, by session: the name each makes.
    opened: HashMap<String, String>,
    /// Answer this many sends busy from now on, keeping nothing.
    fail_sends: usize,
    /// Keep this many sends but answer them busy, as when Drive's answer
    /// is lost after it took the file.
    lose_answers: usize,
    /// Refuse the next token this many times, as an expired one is.
    expire: usize,
    /// Refuse every Drive call as a login made without a Drive scope is.
    no_drive_scope: bool,
    tokens: usize,
}

struct StandIn {
    url: String,
    google: Arc<Mutex<Google>>,
}

impl StandIn {
    fn start() -> StandIn {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let google = Arc::new(Mutex::new(Google::default()));
        let shared = google.clone();
        let base = url.clone();
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let google = shared.clone();
                let base = base.clone();
                thread::spawn(move || serve(stream, &google, &base));
            }
        });
        StandIn { url, google }
    }

    fn calls(&self) -> Vec<String> {
        self.google.lock().unwrap().calls.clone()
    }

    fn refused(&self) -> Vec<String> {
        self.google.lock().unwrap().refused.clone()
    }

    fn file(&self, name: &str) -> Vec<u8> {
        self.google.lock().unwrap().files[name].1.clone()
    }

    fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.google.lock().unwrap().files.keys().cloned().collect();
        names.sort();
        names
    }
}

/// Answer each request on one connection, as Google keeps them open.
fn serve(stream: TcpStream, google: &Mutex<Google>, base: &str) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut writer = stream;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        let mut parts = line.split_whitespace();
        let method = parts.next().unwrap_or("").to_owned();
        let target = parts.next().unwrap_or("").to_owned();
        let mut headers = HashMap::new();
        loop {
            let mut header = String::new();
            reader.read_line(&mut header).unwrap();
            let header = header.trim_end();
            if header.is_empty() {
                break;
            }
            if let Some((key, value)) = header.split_once(':') {
                headers.insert(key.trim().to_lowercase(), value.trim().to_owned());
            }
        }
        let length: usize = headers
            .get("content-length")
            .and_then(|length| length.parse().ok())
            .unwrap_or(0);
        let mut body = vec![0; length];
        reader.read_exact(&mut body).unwrap();
        let (status, extra, answer) = answer(google, base, &method, &target, &headers, body);
        write!(
            writer,
            "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{extra}\r\n{answer}",
            answer.len()
        )
        .unwrap();
        writer.flush().unwrap();
    }
}

fn answer(
    google: &Mutex<Google>,
    base: &str,
    method: &str,
    target: &str,
    headers: &HashMap<String, String>,
    body: Vec<u8>,
) -> (u16, String, String) {
    let mut google = google.lock().unwrap();
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    google.calls.push(format!("{method} {path}"));
    let authorized = headers.get("authorization").map(String::as_str)
        == Some(&format!("Bearer token-{}", google.tokens) as &str);
    match (method, path) {
        ("POST", "/token") => {
            let form = String::from_utf8(body).unwrap();
            assert!(form.contains("grant_type=refresh_token"), "{form}");
            assert!(form.contains("refresh_token=the-refresh"), "{form}");
            google.tokens += 1;
            let token =
                json!({ "access_token": format!("token-{}", google.tokens), "expires_in": 3600 });
            (200, String::new(), token.to_string())
        }
        _ if !authorized => (401, String::new(), "{}".into()),
        _ if google.expire > 0 => {
            google.expire -= 1;
            (401, String::new(), "{}".into())
        }
        _ if google.no_drive_scope => (
            403,
            String::new(),
            json!({ "error": { "code": 403, "message": "Request had insufficient authentication scopes.",
                "status": "PERMISSION_DENIED",
                "details": [{ "reason": "ACCESS_TOKEN_SCOPE_INSUFFICIENT" }] } })
            .to_string(),
        ),
        ("GET", "/drive/v3/files") => {
            let q = decode(
                query
                    .split('&')
                    .find_map(|pair| pair.strip_prefix("q="))
                    .unwrap_or(""),
            );
            if !q.contains(&format!("'{FOLDER}' in parents")) {
                google
                    .refused
                    .push(format!("a list outside the folder: {q}"));
                return (403, String::new(), "{}".into());
            }
            let name = q.split('\'').nth(1).unwrap_or("").to_owned();
            let files: Vec<Value> = google
                .files
                .get(&name)
                .map(|(id, _)| json!({ "id": id }))
                .into_iter()
                .collect();
            (200, String::new(), json!({ "files": files }).to_string())
        }
        ("POST", "/upload/drive/v3/files") if query.contains("uploadType=resumable") => {
            let metadata: Value = serde_json::from_slice(&body).unwrap();
            if metadata["parents"] != json!([FOLDER]) {
                google
                    .refused
                    .push(format!("a file outside the folder: {metadata}"));
                return (403, String::new(), "{}".into());
            }
            let session = format!("s{}", google.opened.len());
            let name = metadata["name"].as_str().unwrap().to_owned();
            google.opened.insert(session.clone(), name);
            let location = format!("Location: {base}/upload/session/{session}\r\n");
            (200, location, "{}".into())
        }
        ("PUT", session) if session.starts_with("/upload/session/") => {
            if headers.get("content-length").is_none() {
                google.refused.push("a send without its length".into());
            }
            if google.fail_sends > 0 {
                google.fail_sends -= 1;
                return (503, String::new(), "{}".into());
            }
            let session = session.trim_start_matches("/upload/session/");
            let name = google.opened[session].clone();
            let id = format!("id-{}", google.files.len());
            google.files.insert(name, (id.clone(), body));
            if google.lose_answers > 0 {
                google.lose_answers -= 1;
                return (503, String::new(), "{}".into());
            }
            (200, String::new(), json!({ "id": id }).to_string())
        }
        _ => {
            google.refused.push(format!("{method} {target}"));
            (403, String::new(), "{}".into())
        }
    }
}

fn decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'%' {
            out.push(u8::from_str_radix(&value[at + 1..at + 3], 16).unwrap());
            at += 3;
        } else {
            out.push(bytes[at]);
            at += 1;
        }
    }
    String::from_utf8(out).unwrap()
}

/// An account with pi's sessions folder, a login, and a key to open what
/// was sealed.
struct Account {
    _home: tempfile::TempDir,
    root: PathBuf,
    identity: age::x25519::Identity,
    config: Config,
}

impl Account {
    fn new(google: &StandIn) -> Account {
        let home = tempfile::tempdir().unwrap();
        let root = home.path().to_owned();
        fs::create_dir_all(root.join("sessions")).unwrap();
        let credentials = root.join("adc.json");
        fs::write(
            &credentials,
            json!({
                "type": "authorized_user",
                "client_id": "the-client",
                "client_secret": "the-secret",
                "refresh_token": "the-refresh",
            })
            .to_string(),
        )
        .unwrap();
        let identity = age::x25519::Identity::generate();
        let config = Config::from_json(&json!({
            "sessions": root.join("sessions"),
            "ledger": root.join("state/sent.jsonl"),
            "folder": FOLDER,
            "host": "laptop",
            "recipients": [identity.to_public().to_string()],
            "credentials": credentials,
            "api": google.url,
            "token": format!("{}/token", google.url),
            "retrySeconds": [0, 0],
        }))
        .unwrap();
        Account {
            _home: home,
            root,
            identity,
            config,
        }
    }

    /// A log in pi's folder `folder`, last written `ago`.
    fn log(&self, folder: &str, name: &str, text: &str, ago: Duration) -> PathBuf {
        let dir = self.root.join("sessions").join(folder);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        fs::write(&path, text).unwrap();
        written(&path, ago);
        path
    }

    fn run(&self) -> Result<Done, String> {
        backup::run(&self.config, SystemTime::now(), None)
    }

    fn open(&self, sealed: &[u8]) -> String {
        let decryptor = age::Decryptor::new(sealed).unwrap();
        let mut packed = decryptor
            .decrypt(std::iter::once(&self.identity as &dyn age::Identity))
            .unwrap();
        let mut bytes = Vec::new();
        packed.read_to_end(&mut bytes).unwrap();
        String::from_utf8(zstd::decode_all(&bytes[..]).unwrap()).unwrap()
    }
}

fn written(path: &Path, ago: Duration) {
    File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(SystemTime::now() - ago)
        .unwrap();
}

fn only_backup_calls(google: &StandIn) {
    assert_eq!(google.refused(), Vec::<String>::new());
    for call in google.calls() {
        assert!(
            [
                "POST /token",
                "GET /drive/v3/files",
                "POST /upload/drive/v3/files"
            ]
            .contains(&call.as_str())
                || call.starts_with("PUT /upload/session/"),
            "the backup called {call}"
        );
    }
}

#[test]
fn sends_each_settled_log_once_sealed_and_packed() {
    let google = StandIn::start();
    let account = Account::new(&google);
    account.log(
        "--home-a--",
        "2026-10-01_one.jsonl",
        "{\"one\":1}\n",
        3 * DAY,
    );
    account.log("", "loose.jsonl", "{\"loose\":1}\n", 5 * DAY);
    account.log(
        "--home-a--",
        "2026-10-25_fresh.jsonl",
        "{\"fresh\":1}\n",
        DAY / 2,
    );
    account.log("--home-a--", "notes.txt", "not a log", 9 * DAY);

    let done = account.run().unwrap();

    assert_eq!((done.sent, done.found, done.before), (2, 0, 0));
    let names = google.names();
    assert_eq!(names.len(), 2);
    assert!(
        names[0].starts_with("laptop/--home-a--/2026-10-01_one."),
        "{names:?}"
    );
    assert!(names[0].ends_with("Z.jsonl.zst.age"), "{names:?}");
    assert!(names[1].starts_with("laptop/loose."), "{names:?}");
    assert_eq!(account.open(&google.file(&names[0])), "{\"one\":1}\n");
    assert_eq!(account.open(&google.file(&names[1])), "{\"loose\":1}\n");

    let again = account.run().unwrap();
    assert_eq!((again.sent, again.before), (0, 2));
    assert_eq!(google.names().len(), 2);
    only_backup_calls(&google);
}

#[test]
fn carries_on_where_a_stopped_run_left_off() {
    let google = StandIn::start();
    let account = Account::new(&google);
    account.log("--a--", "old.jsonl", "old\n", 4 * DAY);
    account.log("--a--", "older.jsonl", "older\n", 6 * DAY);
    account.log("--a--", "oldest.jsonl", "oldest\n", 8 * DAY);
    // One run sends the oldest; Drive is busy for the next one's first send
    // through both retries, so that run ends there without failing.
    backup::run(&account.config, SystemTime::now(), Some(1)).unwrap();
    google.google.lock().unwrap().fail_sends = 3;

    let stopped = account.run().unwrap();
    assert_eq!(stopped.sent, 0);
    assert!(
        stopped.busy.as_deref().unwrap_or("").contains("(503)"),
        "{stopped:?}"
    );
    assert_eq!(google.names().len(), 1);

    let done = account.run().unwrap();
    assert_eq!((done.sent, done.before), (2, 1));
    assert_eq!(google.names().len(), 3);
    assert!(google.names()[0].contains("/--a--/old."));
    // Nothing left behind half sealed.
    assert!(!account.root.join("state/sent.sealing").exists());
    only_backup_calls(&google);
}

#[test]
fn sends_again_when_drive_is_busy_for_a_moment() {
    let google = StandIn::start();
    let account = Account::new(&google);
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    account.log("--a--", "two.jsonl", "two\n", 4 * DAY);
    google.google.lock().unwrap().fail_sends = 2;

    let done = account.run().unwrap();

    assert_eq!((done.sent, done.busy), (2, None));
    assert_eq!(google.names().len(), 2);
    only_backup_calls(&google);
}

#[test]
fn never_sends_twice_when_drive_took_a_file_but_answered_busy() {
    let google = StandIn::start();
    let account = Account::new(&google);
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    google.google.lock().unwrap().lose_answers = 1;

    let done = account.run().unwrap();

    assert_eq!((done.sent, done.found, done.busy), (0, 1, None));
    assert_eq!(google.names().len(), 1);
    let opened = google
        .calls()
        .iter()
        .filter(|call| call.starts_with("POST /upload"))
        .count();
    assert_eq!(opened, 1);
    only_backup_calls(&google);
}

#[test]
fn finds_what_drive_took_before_the_ledger_said_so() {
    let google = StandIn::start();
    let account = Account::new(&google);
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    account.run().unwrap();
    // A run killed after Drive took it, before the ledger was written.
    fs::remove_file(account.root.join("state/sent.jsonl")).unwrap();

    let done = account.run().unwrap();

    assert_eq!((done.sent, done.found), (0, 1));
    assert_eq!(google.names().len(), 1);
    only_backup_calls(&google);
}

#[test]
fn copies_a_resumed_session_again_under_its_new_last_write() {
    let google = StandIn::start();
    let account = Account::new(&google);
    let log = account.log("--a--", "one.jsonl", "one\n", 10 * DAY);
    account.run().unwrap();
    fs::write(&log, "one\ntwo\n").unwrap();
    written(&log, 3 * DAY);

    let done = account.run().unwrap();

    assert_eq!(done.sent, 1);
    let names = google.names();
    assert_eq!(names.len(), 2);
    assert_eq!(account.open(&google.file(&names[1])), "one\ntwo\n");
    only_backup_calls(&google);
}

#[test]
fn signs_in_again_when_drive_stops_taking_the_token() {
    let google = StandIn::start();
    let account = Account::new(&google);
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    google.google.lock().unwrap().expire = 1;

    let done = account.run().unwrap();

    assert_eq!(done.sent, 1);
    let tokens = google
        .calls()
        .iter()
        .filter(|call| call.as_str() == "POST /token")
        .count();
    assert_eq!(tokens, 2);
    only_backup_calls(&google);
}

#[test]
fn says_how_to_sign_in_when_the_login_may_not_use_drive() {
    let google = StandIn::start();
    let account = Account::new(&google);
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    google.google.lock().unwrap().no_drive_scope = true;

    let why = account.run().unwrap_err();

    assert!(
        why.contains("may not use Drive") && why.contains("https://www.googleapis.com/auth/drive"),
        "{why}"
    );
    assert!(google.names().is_empty());
}

#[test]
fn says_how_to_sign_in_when_there_is_no_login() {
    let google = StandIn::start();
    let account = Account::new(&google);
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    fs::remove_file(account.root.join("adc.json")).unwrap();

    let why = account.run().unwrap_err();

    assert!(
        why.contains("gcloud auth application-default login")
            && why.contains("https://www.googleapis.com/auth/drive"),
        "{why}"
    );
    assert!(google.calls().is_empty());
}
