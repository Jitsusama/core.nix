//! session-backup against a stand-in for Google: its token endpoint and the
//! three Drive calls the backup may make. The stand-in refuses, and the
//! tests fail on, any call that isn't a token, a list scoped to the
//! backup's folder, a file made in it, or the download of one it holds;
//! and, for a backup into a folder of its own, a list of the folders of
//! that name at the top of Drive, or making that folder there.
//!
//! Each area of the backup has a file of its own here, beside this one, which
//! keeps the stand-in and the account every area uses.

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
use session_backup::config::{Config, Folder};
use session_backup::fetch;

mod fetching;
mod index;
mod old_logs;
mod own_folder;
mod sending;
mod signing_in;

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
    /// Folders made at the top of Drive, by name. The first is given the
    /// backup's folder's id.
    folders: Vec<String>,
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
    /// Refuse every Drive call naming no quota project, as Drive does for
    /// a login made with gcloud's application-default client.
    needs_project: bool,
    /// The quota project each Drive call named.
    projects: Vec<Option<String>>,
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
    let project = headers.get("x-goog-user-project").cloned();
    if path != "/token" {
        google.projects.push(project.clone());
    }
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
        _ if google.needs_project && project.is_none() => {
            let refusal = json!({ "error": { "code": 403, "message":
                "The drive.googleapis.com API requires a quota project, which is not set by default." } });
            (403, String::new(), refusal.to_string().into())
        }
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
        ("GET", "/drive/v3/files") => list(&mut google, query),
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
        ("POST", "/drive/v3/files") => make_folder(&mut google, &body),
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

/// Drive's answer to a search by name: for a file in the backup's folder,
/// or for a folder at the top of Drive. Any other search is refused.
fn list(google: &mut Google, query: &str) -> (u16, String, Vec<u8>) {
    let q = decode(
        query
            .split('&')
            .find_map(|pair| pair.strip_prefix("q="))
            .unwrap_or(""),
    );
    if q.contains("mimeType = 'application/vnd.google-apps.folder'")
        && q.contains("'root' in parents")
    {
        return find_folder(google, &q);
    }
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
    (
        200,
        String::new(),
        json!({ "files": files }).to_string().into(),
    )
}

/// Drive's answer to a search for a folder at the top of Drive, by the
/// name the query `q` names.
fn find_folder(google: &Google, q: &str) -> (u16, String, Vec<u8>) {
    let name = q.split('\'').nth(1).unwrap_or("");
    let folders: Vec<Value> = google
        .folders
        .iter()
        .position(|made| made == name)
        .map(|at| json!({ "id": made_folder(at) }))
        .into_iter()
        .collect();
    (
        200,
        String::new(),
        json!({ "files": folders }).to_string().into(),
    )
}

/// Drive making a folder, refusing anything else made outside an upload.
fn make_folder(google: &mut Google, body: &[u8]) -> (u16, String, Vec<u8>) {
    let metadata: Value = serde_json::from_slice(body).unwrap();
    if metadata
        != json!({ "name": metadata["name"], "mimeType": "application/vnd.google-apps.folder" })
    {
        google
            .refused
            .push(format!("something made that isn't a folder: {metadata}"));
        return (403, String::new(), "{}".into());
    }
    let id = made_folder(google.folders.len());
    google
        .folders
        .push(metadata["name"].as_str().unwrap().to_owned());
    (200, String::new(), json!({ "id": id }).to_string().into())
}

/// The id of the folder made `at`th: the backup's folder first.
fn made_folder(at: usize) -> String {
    if at == 0 {
        FOLDER.to_owned()
    } else {
        format!("other-{at}")
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
            "machine": "laptop",
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
                "POST /drive/v3/files",
                "POST /upload/drive/v3/files"
            ]
            .contains(&call.as_str())
                || call.starts_with("PUT /upload/session/")
                || call.starts_with("GET /drive/v3/files/id-"),
            "the backup called {call}"
        );
    }
}

/// An account whose backup goes in a folder of its own, `name`.
fn owning(google: &StandIn, name: &str) -> Account {
    let mut account = Account::new(google);
    account.config.folder = Folder::Own(name.into());
    account
}

/// The account's settings, keeping a log Drive holds for `days`.
fn keeping(account: &mut Account, days: u64) {
    account.config.keep = Some(DAY * u32::try_from(days).unwrap());
}

/// The name Drive holds the account's only copy under.
fn only_name(google: &StandIn) -> String {
    let names = google.names();
    assert_eq!(names.len(), 1, "{names:?}");
    names[0].clone()
}
