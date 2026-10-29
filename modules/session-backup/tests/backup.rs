//! session-backup against a stand-in for Google: its token endpoint and the
//! three Drive calls the backup may make. The stand-in refuses, and the
//! tests fail on, any call that isn't a token, a list scoped to the
//! backup's folder, a file made in it, or the download of one it holds.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

use age::secrecy::ExposeSecret;
use serde_json::{Value, json};
use session_backup::backup::{self, Done};
use session_backup::config::Config;
use session_backup::fetch;

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
    /// Answer this many downloads busy from now on.
    fail_downloads: usize,
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
            "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{extra}\r\n",
            answer.len()
        )
        .unwrap();
        writer.write_all(&answer).unwrap();
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
) -> (u16, String, Vec<u8>) {
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
            (200, String::new(), token.to_string().into())
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
            .to_string()
            .into(),
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
            (200, String::new(), json!({ "files": files }).to_string().into())
        }
        ("GET", file) if file.starts_with("/drive/v3/files/") && query.contains("alt=media") => {
            let id = file.trim_start_matches("/drive/v3/files/");
            if google.fail_downloads > 0 {
                google.fail_downloads -= 1;
                return (503, String::new(), "{}".into());
            }
            let Some((_, bytes)) = google.files.values().find(|(held, _)| held == id) else {
                return (404, String::new(), "{}".into());
            };
            (200, String::new(), bytes.clone())
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
            (200, String::new(), json!({ "id": id }).to_string().into())
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
        let key = root.join("key.txt");
        fs::write(&key, identity.to_string().expose_secret()).unwrap();
        let config = Config::from_json(&json!({
            "sessions": root.join("sessions"),
            "ledger": root.join("state/sent.jsonl"),
            "folder": FOLDER,
            "host": "laptop",
            "recipients": [identity.to_public().to_string()],
            "credentials": credentials,
            // A machine without the second key skips it.
            "identities": [root.join("absent.txt"), key],
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

    /// The copy Drive holds as `name`, fetched to `to` under the root.
    fn fetch(&self, name: &str, to: &str) -> Result<u64, String> {
        fetch::fetch(&self.config, name, &self.root.join(to))
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
                || call.starts_with("PUT /upload/session/")
                || call.starts_with("GET /drive/v3/files/id-"),
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
fn leaves_the_sending_to_a_run_already_going() {
    let google = StandIn::start();
    let account = Account::new(&google);
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    fs::create_dir_all(account.root.join("state")).unwrap();
    let going = File::create(account.root.join("state/sent.lock")).unwrap();
    going.lock().unwrap();

    let done = account.run().unwrap();

    assert!(done.held, "{done:?}");
    assert_eq!(done.sent, 0);
    assert!(google.calls().is_empty(), "{:?}", google.calls());
    drop(going);
    assert_eq!(account.run().unwrap().sent, 1);
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

/// The account's settings, keeping a log Drive holds for `days`.
fn keeping(account: &mut Account, days: u64) {
    account.config.keep = Some(DAY * u32::try_from(days).unwrap());
}

#[test]
fn removes_an_old_log_from_the_machine_once_drive_holds_it() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    keeping(&mut account, 90);
    let old = account.log("--a--", "old.jsonl", "old\n", 100 * DAY);
    let recent = account.log("--a--", "recent.jsonl", "recent\n", 10 * DAY);

    // The run that sends a log never removes it: the next one does, once
    // it has asked Drive for the copy.
    let sent = account.run().unwrap();
    assert_eq!((sent.sent, sent.removed), (2, 0));
    assert!(old.exists());

    let done = account.run().unwrap();

    assert_eq!((done.removed, done.freed), (1, 4));
    assert!(!old.exists());
    assert!(recent.exists());
    assert_eq!(google.names().len(), 2);
    only_backup_calls(&google);
}

#[test]
fn keeps_an_old_log_drive_no_longer_finds() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    keeping(&mut account, 90);
    let old = account.log("--a--", "old.jsonl", "old\n", 100 * DAY);
    account.run().unwrap();
    // Someone removed the copy from Drive, so the log is the only one.
    google.google.lock().unwrap().files.clear();

    let done = account.run().unwrap();

    assert_eq!(done.removed, 0);
    assert!(old.exists());
    only_backup_calls(&google);
}

#[test]
fn keeps_every_log_when_the_settings_say_no_keep() {
    let google = StandIn::start();
    let account = Account::new(&google);
    let old = account.log("--a--", "old.jsonl", "old\n", 400 * DAY);
    account.run().unwrap();

    let done = account.run().unwrap();

    assert_eq!(done.removed, 0);
    assert!(old.exists());
}

#[test]
fn reads_keep_days_from_the_settings() {
    let settings = |keep: Value| {
        Config::from_json(&json!({
            "sessions": "/s", "ledger": "/l", "folder": "f", "host": "h",
            "recipients": ["age1someone"], "credentials": "/c", "keepDays": keep,
        }))
        .unwrap()
        .keep
    };
    assert_eq!(settings(json!(90)), Some(90 * DAY));
    assert_eq!(settings(Value::Null), None);
    assert_eq!(settings(json!(0)), None);
}

/// The name Drive holds the account's only copy under.
fn only_name(google: &StandIn) -> String {
    let names = google.names();
    assert_eq!(names.len(), 1, "{names:?}");
    names[0].clone()
}

#[test]
fn fetches_a_log_gone_from_the_machine_as_it_was() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    keeping(&mut account, 90);
    let log = account.log("--a--", "old.jsonl", "{\"one\":1}\n", 100 * DAY);
    account.run().unwrap();
    account.run().unwrap();
    assert!(!log.exists());
    let before = google.calls().len();

    let bytes = account.fetch(&only_name(&google), "back.jsonl").unwrap();

    assert_eq!(bytes, 10);
    assert_eq!(
        fs::read_to_string(account.root.join("back.jsonl")).unwrap(),
        "{\"one\":1}\n"
    );
    // The ledger says where the copy is, so Drive is asked only for it.
    assert_eq!(
        google.calls()[before..],
        ["POST /token", "GET /drive/v3/files/id-0"]
    );
    assert_eq!(
        fs::read_dir(&account.root).unwrap().count(),
        5,
        "only the fetched log is new"
    );
    only_backup_calls(&google);
}

#[test]
fn finds_a_copy_by_name_when_the_ledger_is_gone() {
    let google = StandIn::start();
    let account = Account::new(&google);
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    account.run().unwrap();
    // A machine rebuilt from nothing has no ledger.
    fs::remove_file(account.root.join("state/sent.jsonl")).unwrap();

    account.fetch(&only_name(&google), "back.jsonl").unwrap();

    assert_eq!(
        fs::read_to_string(account.root.join("back.jsonl")).unwrap(),
        "one\n"
    );
    only_backup_calls(&google);
}

#[test]
fn fetches_again_when_drive_is_busy_for_a_moment() {
    let google = StandIn::start();
    let account = Account::new(&google);
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    account.run().unwrap();
    google.google.lock().unwrap().fail_downloads = 2;

    account.fetch(&only_name(&google), "back.jsonl").unwrap();

    assert_eq!(
        fs::read_to_string(account.root.join("back.jsonl")).unwrap(),
        "one\n"
    );
}

#[test]
fn says_when_drive_holds_no_copy_of_that_name() {
    let google = StandIn::start();
    let account = Account::new(&google);

    let why = account
        .fetch("laptop/--a--/none.jsonl.zst.age", "back.jsonl")
        .unwrap_err();

    assert!(
        why.contains("holds no copy named laptop/--a--/none"),
        "{why}"
    );
    assert!(!account.root.join("back.jsonl").exists());
    only_backup_calls(&google);
}

#[test]
fn leaves_nothing_behind_when_no_key_here_opens_the_copy() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    account.run().unwrap();
    let other = account.root.join("other.txt");
    let stranger = age::x25519::Identity::generate();
    fs::write(&other, stranger.to_string().expose_secret()).unwrap();
    account.config.identities = vec![other];
    let listed = fs::read_dir(&account.root).unwrap().count();

    let why = account
        .fetch(&only_name(&google), "back.jsonl")
        .unwrap_err();

    assert!(why.contains("no key here opens"), "{why}");
    assert_eq!(fs::read_dir(&account.root).unwrap().count(), listed);
}

#[test]
fn says_when_the_settings_name_no_key_this_machine_has() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    account.run().unwrap();
    account.config.identities = vec![account.root.join("absent.txt")];

    let why = account
        .fetch(&only_name(&google), "back.jsonl")
        .unwrap_err();

    assert!(
        why.contains("none of the keys the settings name is on this machine"),
        "{why}"
    );
    assert!(!account.root.join("back.jsonl").exists());
}

#[test]
fn says_to_name_a_key_when_the_settings_name_none() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    account.run().unwrap();
    account.config.identities = Vec::new();

    let why = account
        .fetch(&only_name(&google), "back.jsonl")
        .unwrap_err();

    assert!(
        why.contains("name no key") && why.contains("identities"),
        "{why}"
    );
    // Nothing was downloaded to find that out.
    assert!(
        !google
            .calls()
            .iter()
            .any(|call| call.starts_with("GET /drive/v3/files/id-")),
        "{:?}",
        google.calls()
    );
}

#[test]
fn never_writes_over_a_file_already_there() {
    let google = StandIn::start();
    let account = Account::new(&google);
    account.log("--a--", "one.jsonl", "one\n", 3 * DAY);
    account.run().unwrap();
    fs::write(account.root.join("back.jsonl"), "mine").unwrap();

    let why = account
        .fetch(&only_name(&google), "back.jsonl")
        .unwrap_err();

    assert!(why.contains("already there"), "{why}");
    assert_eq!(
        fs::read_to_string(account.root.join("back.jsonl")).unwrap(),
        "mine"
    );
}

/// The account's settings, keeping an index, with the harness's records of
/// its sessions beside it.
fn indexing(account: &mut Account) {
    account.config.index = Some(account.root.join("state/index.jsonl"));
    account.config.records = Some(account.root.join("records"));
}

/// A session's log as pi writes one: its header, the name pi gave it twice
/// over, a question, two answers on two models, and a last line cut short.
const SESSION: &str = concat!(
    r#"{"type":"session","version":3,"id":"s-1","timestamp":"2026-10-01T10:00:00.000Z","cwd":"/home/a/src/thing"}"#,
    "\n",
    r#"{"type":"session_info","timestamp":"2026-10-01T10:00:05.000Z","name":"First"}"#,
    "\n",
    r#"{"type":"message","timestamp":"2026-10-01T10:01:00.000Z","message":{"role":"user","content":"hi"}}"#,
    "\n",
    r#"{"type":"message","timestamp":"2026-10-01T10:02:00.000Z","message":{"role":"assistant","model":"big","usage":{"cost":{"total":0.504}}}}"#,
    "\n",
    r#"{"type":"session_info","timestamp":"2026-10-01T10:03:00.000Z","name":"Better Name"}"#,
    "\n",
    r#"{"type":"message","timestamp":"2026-10-01T11:30:00.000Z","message":{"role":"assistant","model":"small","usage":{"cost":{"total":0.25}}}}"#,
    "\n",
    r#"{"type":"message","timest"#,
);

/// The names Drive holds the index's rows under.
fn index_objects(google: &StandIn) -> Vec<String> {
    google
        .names()
        .into_iter()
        .filter(|name| name.starts_with("laptop/index/"))
        .collect()
}

/// The rows in a sealed index object, or in the index on the machine.
fn rows(text: &str) -> Vec<Value> {
    text.lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn indexes_each_log_it_sends_and_sends_the_rows_sealed() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    indexing(&mut account);
    fs::create_dir_all(account.root.join("records")).unwrap();
    fs::write(
        account.root.join("records/s-1.json"),
        r#"{"sessionId":"s-1","quest":"QEST-1"}"#,
    )
    .unwrap();
    account.log("--a--", "one.jsonl", SESSION, 3 * DAY);

    let done = account.run().unwrap();

    assert_eq!((done.sent, done.indexed), (1, 1));
    let objects = index_objects(&google);
    assert_eq!(objects.len(), 1, "{:?}", google.names());
    assert!(objects[0].ends_with("Z.jsonl.zst.age"), "{objects:?}");
    let sent = rows(&account.open(&google.file(&objects[0])));
    let log = google
        .names()
        .into_iter()
        .find(|name| name.starts_with("laptop/--a--/one."))
        .unwrap();
    assert_eq!(
        sent,
        [json!({
            "name": log,
            "id": "s-1",
            "folder": "/home/a/src/thing",
            "started": "2026-10-01T10:00:00.000Z",
            "ended": "2026-10-01T11:30:00.000Z",
            "title": "Better Name",
            "quest": "QEST-1",
            "models": ["big", "small"],
            "turns": 2,
            "cost": 0.75,
            "bytes": SESSION.len(),
        })]
    );
    let kept = fs::read_to_string(account.root.join("state/index.jsonl")).unwrap();
    assert_eq!(rows(&kept), sent);

    let again = account.run().unwrap();
    assert_eq!(again.indexed, 0);
    assert_eq!(index_objects(&google).len(), 1);
    only_backup_calls(&google);
}

#[test]
fn gives_a_row_to_each_log_sent_before_there_was_an_index() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    account.log("--a--", "one.jsonl", "{\"one\":1}\n", 3 * DAY);
    account.log("--a--", "two.jsonl", SESSION, 4 * DAY);
    account.run().unwrap();
    indexing(&mut account);

    let done = account.run().unwrap();

    assert_eq!((done.sent, done.before, done.indexed), (0, 2, 2));
    let sent = rows(&account.open(&google.file(&index_objects(&google)[0])));
    // Oldest first. A session the harness kept no record of has no quest,
    // and a log that isn't pi's says what it can.
    assert_eq!(sent[0]["id"], "s-1");
    assert_eq!(sent[0]["quest"], Value::Null);
    assert_eq!(sent[1]["id"], Value::Null);
    assert_eq!(sent[1]["turns"], 0);
    only_backup_calls(&google);
}

#[test]
fn leaves_the_rows_for_the_next_run_when_drive_stays_busy() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    account.log("--a--", "one.jsonl", SESSION, 3 * DAY);
    account.run().unwrap();
    indexing(&mut account);
    // The first send and both of its retries.
    google.google.lock().unwrap().fail_sends = 3;

    let stopped = account.run().unwrap();

    assert_eq!(stopped.indexed, 0);
    assert!(stopped.busy.is_some(), "{stopped:?}");
    assert!(!account.root.join("state/index.jsonl").exists());
    assert!(index_objects(&google).is_empty());

    let done = account.run().unwrap();
    assert_eq!(done.indexed, 1);
    assert_eq!(index_objects(&google).len(), 1);
}

#[test]
fn a_log_sent_before_it_is_removed_keeps_its_row() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    keeping(&mut account, 90);
    indexing(&mut account);
    let old = account.log("--a--", "old.jsonl", SESSION, 100 * DAY);

    account.run().unwrap();
    account.run().unwrap();

    assert!(!old.exists());
    let kept = fs::read_to_string(account.root.join("state/index.jsonl")).unwrap();
    assert_eq!(rows(&kept).len(), 1);
}

/// The account with `count` logs sent and then removed from the machine,
/// before it kept an index, the oldest `SESSION`.
fn removed(account: &mut Account, count: usize) {
    keeping(account, 90);
    account.log("--a--", "old-0.jsonl", SESSION, 100 * DAY);
    for at in 1..count {
        let days = 100 - u32::try_from(at).unwrap();
        account.log("--a--", &format!("old-{at}.jsonl"), "{}\n", DAY * days);
    }
    account.run().unwrap();
    assert_eq!(account.run().unwrap().removed, count);
}

/// The files left in the account's state folder.
fn state(account: &Account) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(account.root.join("state"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

#[test]
fn reads_a_log_gone_from_the_machine_back_for_its_row() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    removed(&mut account, 1);
    indexing(&mut account);

    let done = account.run().unwrap();

    assert_eq!((done.sent, done.indexed, done.unread), (0, 1, None));
    let sent = rows(&account.open(&google.file(&index_objects(&google)[0])));
    assert_eq!(sent[0]["id"], "s-1");
    assert_eq!(sent[0]["title"], "Better Name");
    assert_eq!(sent[0]["bytes"], SESSION.len());
    // The copy read back is gone, and nothing else is left beside the ledger.
    assert_eq!(
        state(&account),
        ["index.jsonl", "sent.jsonl", "sent.lock"],
        "only the index is new"
    );
    // An index object is never read back as a log.
    let again = account.run().unwrap();
    assert_eq!(again.indexed, 0);
    assert_eq!(index_objects(&google).len(), 1);
    only_backup_calls(&google);
}

#[test]
fn reads_back_only_as_many_as_the_settings_say_a_run() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    removed(&mut account, 3);
    indexing(&mut account);
    account.config.backfill = 2;

    assert_eq!(account.run().unwrap().indexed, 2);
    assert_eq!(account.run().unwrap().indexed, 1);
    assert_eq!(account.run().unwrap().indexed, 0);
    assert_eq!(index_objects(&google).len(), 2);
}

#[test]
fn carries_on_without_the_rows_no_key_here_can_read() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    removed(&mut account, 1);
    indexing(&mut account);
    let other = account.root.join("other.txt");
    fs::write(
        &other,
        age::x25519::Identity::generate()
            .to_string()
            .expose_secret(),
    )
    .unwrap();
    account.config.identities = vec![other];
    account.log("--a--", "new.jsonl", "new\n", 3 * DAY);

    let done = account.run().unwrap();

    assert!(
        done.unread
            .as_deref()
            .unwrap_or("")
            .contains("no key here opens"),
        "{done:?}"
    );
    // The new log still went, and has its row.
    assert_eq!((done.sent, done.indexed, done.busy), (1, 1, None));
    assert_eq!(state(&account), ["index.jsonl", "sent.jsonl", "sent.lock"]);
}

#[test]
fn reads_nothing_back_when_the_settings_name_no_key() {
    let google = StandIn::start();
    let mut account = Account::new(&google);
    removed(&mut account, 1);
    indexing(&mut account);
    account.config.identities = Vec::new();

    let done = account.run().unwrap();

    assert_eq!((done.indexed, done.unread), (0, None));
    assert!(
        !google
            .calls()
            .iter()
            .any(|call| call.starts_with("GET /drive/v3/files/id-")),
        "{:?}",
        google.calls()
    );
}

#[test]
fn reads_where_the_index_and_the_records_are_from_the_settings() {
    let settings = Config::from_json(&json!({
        "sessions": "/s", "ledger": "/l", "folder": "f", "host": "h",
        "recipients": ["age1someone"], "credentials": "/c",
        "index": "/i.jsonl", "records": "/r", "backfillPerRun": 7,
    }))
    .unwrap();
    assert_eq!(settings.index, Some(PathBuf::from("/i.jsonl")));
    assert_eq!(settings.records, Some(PathBuf::from("/r")));
    assert_eq!(settings.backfill, 7);
}
