//! The door as a fenced gh reaches it: the real program, a stand-in for
//! GitHub on localhost with a certificate of the tests' own, and a token
//! read by a stand-in gh from a file, so every call refused is seen never
//! to reach GitHub, and every call passed to reach it with the token.
//!
//! Long because each case drives the real program through the same
//! helpers, a control socket, a session's tunnel and the stand-in, and
//! its cases are each a call a fenced command makes, which read best
//! beside the calls they're told apart from.

use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use base64::Engine;
use pi_github_door::http;
use rcgen::{BasicConstraints, CertificateParams, IsCa, Issuer, KeyPair};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName};
use rustls::{
    ClientConfig, ClientConnection, RootCertStore, ServerConfig, ServerConnection, StreamOwned,
};
use serde_json::{Value, json};

/// What the fence holds as `GH_TOKEN`.
const PLACEHOLDER: &str = "pi-github-door-placeholder";

/// A request as the stand-in for GitHub saw it.
#[derive(Debug, Clone)]
struct Seen {
    method: String,
    target: String,
    host: String,
    authorization: Option<String>,
    body: Vec<u8>,
}

/// The stand-in for GitHub: where it listens, and what it's seen.
struct GitHub {
    address: SocketAddr,
    seen: Arc<Mutex<Vec<Seen>>>,
}

fn provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

/// A CA of the tests' own, as PEM, and a server config for GitHub's hosts
/// it signs.
fn github_certificate() -> (String, Arc<ServerConfig>) {
    let ca_key = KeyPair::generate().expect("a key");
    let mut ca = CertificateParams::new(Vec::<String>::new()).expect("params");
    ca.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let ca_cert = ca.self_signed(&ca_key).expect("signed");
    let issuer = Issuer::new(ca, ca_key);
    let key = KeyPair::generate().expect("a key");
    let leaf = CertificateParams::new(vec!["api.github.com".to_owned(), "github.com".to_owned()])
        .expect("params")
        .signed_by(&key, &issuer)
        .expect("signed");
    let mut config = ServerConfig::builder_with_provider(provider())
        .with_safe_default_protocol_versions()
        .expect("versions")
        .with_no_client_auth()
        .with_single_cert(
            vec![leaf.der().clone()],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.serialize_der())),
        )
        .expect("a config");
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    (ca_cert.pem(), Arc::new(config))
}

/// Start the stand-in, answering 401 to any token but `token good`, 200
/// with `{"ok": true}` otherwise.
fn github(config: Arc<ServerConfig>) -> GitHub {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("a port");
    let address = listener.local_addr().expect("bound");
    let seen = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&seen);
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let config = Arc::clone(&config);
            let kept = Arc::clone(&kept);
            thread::spawn(move || {
                let connection = ServerConnection::new(config).expect("a connection");
                let mut tls = StreamOwned::new(connection, stream);
                let mut buffer = Vec::new();
                let Ok(head) = http::read_head(&mut tls, &mut buffer) else {
                    return;
                };
                let body =
                    http::read_body(&mut tls, &mut buffer, &head, 1 << 20).unwrap_or_default();
                let authorization = head.header("authorization").map(str::to_owned);
                kept.lock().expect("held").push(Seen {
                    method: head.method.clone(),
                    target: head.target.clone(),
                    host: head.header("host").unwrap_or_default().to_owned(),
                    authorization: authorization.clone(),
                    body,
                });
                let good = matches!(
                    authorization.as_deref(),
                    Some("token good" | "Basic eC1hY2Nlc3MtdG9rZW46Z29vZA==")
                );
                let (status, body) = if good {
                    (200, r#"{"ok":true}"#)
                } else {
                    (401, r#"{"message":"Bad credentials"}"#)
                };
                let _ = write!(
                    tls,
                    "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                tls.conn.send_close_notify();
                let _ = tls.flush();
            });
        }
    });
    GitHub { address, seen }
}

/// The door running, its folder and what it says.
struct Running {
    child: Child,
    folder: tempfile::TempDir,
    github: GitHub,
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Running {
    fn control(&self) -> PathBuf {
        self.folder.path().join("control.sock")
    }

    fn journal(&self) -> String {
        fs::read_to_string(self.folder.path().join("journal")).unwrap_or_default()
    }

    fn token_file(&self) -> PathBuf {
        self.folder.path().join("token")
    }

    /// The control socket's answer to `ask`.
    fn ask(&self, ask: &Value) -> Value {
        let mut stream = UnixStream::connect(self.control()).expect("the control socket");
        writeln!(stream, "{ask}").expect("asked");
        let mut line = String::new();
        BufReader::new(&stream)
            .read_line(&mut line)
            .expect("answered");
        serde_json::from_str(&line).expect("JSON")
    }

    /// The session `id` opened: its socket and its bundle.
    fn open(&self, id: &str) -> (PathBuf, PathBuf) {
        let answer = self.ask(&json!({"ask": "open", "session": id}));
        assert_eq!(answer["ok"], json!(true), "{answer}");
        (
            PathBuf::from(answer["socket"].as_str().expect("a socket")),
            PathBuf::from(answer["bundle"].as_str().expect("a bundle")),
        )
    }

    fn seen(&self) -> Vec<Seen> {
        self.github.seen.lock().expect("held").clone()
    }
}

/// Start the door, with the stand-in for GitHub behind it and a stand-in
/// gh printing `token`.
fn door(token: &str) -> Running {
    let folder = tempfile::tempdir().expect("a folder");
    let (ca, config) = github_certificate();
    let github = github(config);
    let roots = folder.path().join("roots.pem");
    fs::write(&roots, &ca).expect("written");
    // The system's roots, which the session's bundle starts with; the
    // tests' CA stands in for them.
    let system = folder.path().join("system.pem");
    fs::write(&system, &ca).expect("written");
    fs::write(folder.path().join("token"), format!("{token}\n")).expect("written");
    let gh = folder.path().join("gh");
    fs::write(
        &gh,
        format!(
            "#!/bin/sh\ncat '{}'\n",
            folder.path().join("token").display()
        ),
    )
    .expect("written");
    fs::set_permissions(&gh, fs::Permissions::from_mode(0o755)).expect("made runnable");
    let journal = fs::File::create(folder.path().join("journal")).expect("a journal");
    let address = github.address.to_string();
    let child = Command::new(env!("CARGO_BIN_EXE_pi-github-door"))
        .arg("serve")
        .arg("--control")
        .arg(folder.path().join("control.sock"))
        .arg("--sessions")
        .arg(folder.path().join("sessions"))
        .arg("--system-bundle")
        .arg(&system)
        .arg("--roots")
        .arg(&roots)
        .arg("--gh")
        .arg(&gh)
        .args(["--resolve", &format!("api.github.com={address}")])
        .args(["--resolve", &format!("github.com={address}")])
        .stdout(Stdio::null())
        .stderr(journal)
        .spawn()
        .expect("the door started");
    let running = Running {
        child,
        folder,
        github,
    };
    let deadline = Instant::now() + Duration::from_secs(10);
    while UnixStream::connect(running.control()).is_err() {
        assert!(
            Instant::now() < deadline,
            "the door never listened: {}",
            running.journal()
        );
        // Polled, since the door says nothing when it's listening.
        thread::sleep(Duration::from_millis(20));
    }
    running
}

/// An answer: its status and its body.
#[derive(Debug)]
struct Answer {
    status: u16,
    body: String,
}

/// Ask for a tunnel to `target` over `stream`, saying how it was answered.
fn connect(stream: &mut impl ReadWrite, target: &str) -> String {
    write!(
        stream,
        "CONNECT {target} HTTP/1.1\r\nHost: {target}\r\n\r\n"
    )
    .expect("asked");
    let mut said = Vec::new();
    let mut byte = [0u8; 1];
    while !said.ends_with(b"\r\n\r\n") {
        if stream.read(&mut byte).expect("answered") == 0 {
            break;
        }
        said.push(byte[0]);
    }
    String::from_utf8_lossy(&said).into_owned()
}

trait ReadWrite: Read + Write {}
impl<T: Read + Write> ReadWrite for T {}

/// A client trusting the roots in `bundle`.
fn client(bundle: &Path) -> Arc<ClientConfig> {
    let mut roots = RootCertStore::empty();
    for der in CertificateDer::pem_file_iter(bundle).expect("a bundle") {
        roots.add(der.expect("a root")).expect("added");
    }
    Arc::new(
        ClientConfig::builder_with_provider(provider())
            .with_safe_default_protocol_versions()
            .expect("versions")
            .with_root_certificates(roots)
            .with_no_client_auth(),
    )
}

/// `request` sent through a tunnel already open on `stream` to `host`,
/// trusting `bundle`, and how it was answered; an error when TLS failed.
fn through<S: Read + Write>(
    stream: S,
    host: &str,
    bundle: &Path,
    request: &[u8],
) -> io::Result<Answer> {
    let name = ServerName::try_from(host.to_owned()).expect("a name");
    let connection = ClientConnection::new(client(bundle), name).expect("a connection");
    let mut tls = StreamOwned::new(connection, stream);
    tls.write_all(request)?;
    tls.flush()?;
    let mut answer = Vec::new();
    match tls.read_to_end(&mut answer) {
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof && !answer.is_empty() => {}
        Err(error) => return Err(error),
    }
    let text = String::from_utf8_lossy(&answer).into_owned();
    let status = text
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    let body = text
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_owned())
        .unwrap_or_default();
    Ok(Answer { status, body })
}

/// `request` sent to `host` through the session's socket.
fn call(socket: &Path, bundle: &Path, host: &str, request: &str) -> Answer {
    let mut stream = UnixStream::connect(socket).expect("the session's socket");
    let said = connect(&mut stream, &format!("{host}:443"));
    assert!(said.starts_with("HTTP/1.1 200"), "{said}");
    through(stream, host, bundle, request.as_bytes()).expect("answered")
}

fn get(path: &str, authorization: Option<&str>) -> String {
    let authorization = authorization
        .map(|value| format!("Authorization: {value}\r\n"))
        .unwrap_or_default();
    format!(
        "GET {path} HTTP/1.1\r\nHost: api.github.com\r\n{authorization}User-Agent: test\r\n\r\n"
    )
}

fn post(path: &str, host: &str, body: &str) -> String {
    format!(
        "POST {path} HTTP/1.1\r\nHost: {host}\r\nAuthorization: token {PLACEHOLDER}\r\n\
         Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

#[test]
fn a_read_reaches_github_with_the_token_in_the_placeholders_place() {
    let running = door("good");
    let (socket, bundle) = running.open("s1");
    let answer = call(
        &socket,
        &bundle,
        "api.github.com",
        &get("/user?x=1", Some(&format!("token {PLACEHOLDER}"))),
    );
    assert_eq!(answer.status, 200, "{answer:?} {}", running.journal());
    assert_eq!(answer.body, r#"{"ok":true}"#);
    let seen = running.seen();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].authorization.as_deref(), Some("token good"));
    assert_eq!(seen[0].host, "api.github.com");
    assert_eq!(seen[0].target, "/user?x=1");
    let journal = running.journal();
    assert!(
        journal.contains("pi-github-door: passed session=s1 claimed-session=")
            && journal.contains("method=GET host=api.github.com path=/user status=200"),
        "{journal}"
    );
    assert!(
        !journal.contains("good"),
        "the token is never said: {journal}"
    );
}

#[test]
fn a_read_without_a_credential_is_sent_with_the_token_too() {
    let running = door("good");
    let (socket, bundle) = running.open("s1");
    assert_eq!(
        call(&socket, &bundle, "api.github.com", &get("/user", None)).status,
        200
    );
    assert_eq!(
        running.seen()[0].authorization.as_deref(),
        Some("token good")
    );
}

#[test]
fn a_refused_call_never_reaches_github_and_names_its_route() {
    let running = door("good");
    let (socket, bundle) = running.open("s1");
    let merge = format!(
        "PUT /repos/o/r/pulls/1/merge HTTP/1.1\r\nHost: api.github.com\r\n\
         Authorization: token {PLACEHOLDER}\r\nContent-Length: 2\r\n\r\n{{}}"
    );
    let answer = call(&socket, &bundle, "api.github.com", &merge);
    assert_eq!(answer.status, 403);
    assert!(
        answer.body.contains("review_offer merge"),
        "{}",
        answer.body
    );
    let push = post("/o/r.git/git-receive-pack", "github.com", "0000");
    let answer = call(&socket, &bundle, "github.com", &push);
    assert_eq!(answer.status, 403);
    assert!(answer.body.contains("work tool"), "{}", answer.body);
    assert!(running.seen().is_empty(), "{:?}", running.seen());
    let journal = running.journal();
    assert!(journal.contains("refused why=merge "), "{journal}");
    assert!(journal.contains("refused why=receive-pack "), "{journal}");
}

#[test]
fn a_mutation_named_for_another_is_refused_by_its_field() {
    let running = door("good");
    let (socket, bundle) = running.open("s1");
    let harm = "mutation addSubIssue { deleteRepository(input: {repositoryId: \"R\"}) \
        { clientMutationId } }";
    let body = json!({ "query": harm }).to_string();
    let answer = call(
        &socket,
        &bundle,
        "api.github.com",
        &post("/graphql", "api.github.com", &body),
    );
    assert_eq!(answer.status, 403, "{answer:?}");
    assert!(running.seen().is_empty());
    assert!(
        running.journal().contains("refused why=mutation "),
        "{}",
        running.journal()
    );
    assert!(
        running
            .journal()
            .contains("fields=mutation:deleteRepository"),
        "{}",
        running.journal()
    );
}

#[test]
fn a_listed_mutation_and_a_query_pass() {
    let running = door("good");
    let (socket, bundle) = running.open("s1");
    let position = "mutation { updateProjectV2ItemPosition(input: {projectId: \"P\", \
        itemId: \"I\"}) { clientMutationId } }";
    let reorder = json!({ "query": position }).to_string();
    assert_eq!(
        call(
            &socket,
            &bundle,
            "api.github.com",
            &post("/graphql", "api.github.com", &reorder)
        )
        .status,
        200
    );
    let query = json!({"query": "query { viewer { login } }"}).to_string();
    assert_eq!(
        call(
            &socket,
            &bundle,
            "api.github.com",
            &post("/graphql", "api.github.com", &query)
        )
        .status,
        200
    );
    let seen = running.seen();
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[0].body, reorder.as_bytes());
    assert!(
        running
            .journal()
            .contains("fields=mutation:updateProjectV2ItemPosition status=200"),
        "{}",
        running.journal()
    );
}

#[test]
fn any_credential_but_the_placeholder_is_refused() {
    let running = door("good");
    let (socket, bundle) = running.open("s1");
    let answer = call(
        &socket,
        &bundle,
        "api.github.com",
        &get("/user", Some("token ghp_someone_elses")),
    );
    assert_eq!(answer.status, 403);
    assert!(running.seen().is_empty());
    assert!(
        running.journal().contains("refused why=authorization "),
        "{}",
        running.journal()
    );
}

#[test]
fn a_401_reads_the_token_again_and_tries_once_more() {
    let running = door("stale");
    let (socket, bundle) = running.open("s1");
    let first = call(&socket, &bundle, "api.github.com", &get("/user", None));
    assert_eq!(first.status, 401, "the same token isn't tried twice");
    assert_eq!(running.seen().len(), 1);
    fs::write(running.token_file(), "good\n").expect("written");
    let second = call(&socket, &bundle, "api.github.com", &get("/user", None));
    assert_eq!(second.status, 200, "{second:?}");
    let seen = running.seen();
    assert_eq!(seen.len(), 3);
    assert_eq!(seen[1].authorization.as_deref(), Some("token stale"));
    assert_eq!(seen[2].authorization.as_deref(), Some("token good"));
}

#[test]
fn git_fetch_carries_the_token_as_git_sends_one() {
    let running = door("good");
    let (socket, bundle) = running.open("s1");
    let basic =
        base64::engine::general_purpose::STANDARD.encode(format!("x-access-token:{PLACEHOLDER}"));
    let fetch = format!(
        "POST /o/r.git/git-upload-pack HTTP/1.1\r\nHost: github.com\r\n\
         Authorization: Basic {basic}\r\nTransfer-Encoding: chunked\r\n\r\n\
         4\r\n0000\r\n0\r\n\r\n"
    );
    assert_eq!(
        call(&socket, &bundle, "github.com", &fetch).status,
        200,
        "{}",
        running.journal()
    );
    let seen = running.seen();
    assert_eq!(seen[0].host, "github.com");
    assert_eq!(seen[0].body, b"0000", "chunks joined");
    assert_eq!(seen[0].method, "POST");
}

#[test]
fn another_host_is_tunnelled_untouched_and_unsaid() {
    let running = door("good");
    let (socket, _) = running.open("s1");
    let echo = TcpListener::bind(("127.0.0.1", 0)).expect("a port");
    let address = echo.local_addr().expect("bound");
    thread::spawn(move || {
        let (mut stream, _) = echo.accept().expect("accepted");
        let mut said = [0u8; 5];
        stream.read_exact(&mut said).expect("read");
        stream.write_all(&said).expect("echoed");
    });
    let mut stream = UnixStream::connect(&socket).expect("the session's socket");
    assert!(connect(&mut stream, &address.to_string()).starts_with("HTTP/1.1 200"));
    stream.write_all(b"hello").expect("sent");
    let mut back = [0u8; 5];
    stream.read_exact(&mut back).expect("echoed");
    assert_eq!(&back, b"hello");
    drop(stream);
    assert!(
        !running.journal().contains("127.0.0.1"),
        "{}",
        running.journal()
    );
}

#[test]
fn one_sessions_bundle_doesnt_trust_anothers_door() {
    let running = door("good");
    let (socket, _) = running.open("s1");
    let (_, other_bundle) = running.open("s2");
    let mut stream = UnixStream::connect(&socket).expect("the session's socket");
    assert!(connect(&mut stream, "api.github.com:443").starts_with("HTTP/1.1 200"));
    assert!(
        through(
            stream,
            "api.github.com",
            &other_bundle,
            get("/user", None).as_bytes()
        )
        .is_err()
    );
    assert!(running.seen().is_empty());
}

#[test]
fn github_on_another_port_is_refused() {
    let running = door("good");
    let (socket, _) = running.open("s1");
    let mut stream = UnixStream::connect(&socket).expect("the session's socket");
    assert!(connect(&mut stream, "api.github.com:8443").starts_with("HTTP/1.1 403"));
    assert!(
        running.journal().contains("refused why=port "),
        "{}",
        running.journal()
    );
}

#[test]
fn a_session_opens_once_closes_and_is_lifted_by_name() {
    let running = door("good");
    let (socket, bundle) = running.open("s1");
    assert_eq!(
        running.open("s1"),
        (socket.clone(), bundle.clone()),
        "an open is idempotent"
    );
    let text = fs::read_to_string(&bundle).expect("a bundle");
    assert_eq!(
        text.matches("BEGIN CERTIFICATE").count(),
        2,
        "the system's roots and its own"
    );
    assert_eq!(
        fs::metadata(socket.parent().expect("a folder"))
            .expect("there")
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        running.ask(&json!({"ask": "lift", "session": "s1"}))["ok"],
        json!(true)
    );
    assert_eq!(
        running.ask(&json!({"ask": "close", "session": "s1"}))["ok"],
        json!(true)
    );
    assert!(!socket.exists());
    assert_eq!(
        running.ask(&json!({"ask": "lift", "session": "s1"}))["ok"],
        json!(false)
    );
    assert_eq!(
        running.ask(&json!({"ask": "open", "session": "../x"}))["ok"],
        json!(false)
    );
}

#[test]
fn the_relay_sends_githubs_hosts_to_the_door_and_the_rest_straight_out() {
    let running = door("good");
    let (socket, bundle) = running.open("s1");
    let mut watched = Command::new("sleep")
        .arg("60")
        .spawn()
        .expect("a process to watch");
    let output = Command::new(env!("CARGO_BIN_EXE_pi-github-door"))
        .args(["relay", "--socket"])
        .arg(&socket)
        .args(["--while", &watched.id().to_string()])
        .output()
        .expect("the relay");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let port: u16 = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .expect("a port");
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("the relay");
    assert!(connect(&mut stream, "api.github.com:443").starts_with("HTTP/1.1 200"));
    let answer = through(
        stream,
        "api.github.com",
        &bundle,
        get("/user", None).as_bytes(),
    )
    .expect("answered");
    assert_eq!(answer.status, 200, "{answer:?} {}", running.journal());
    let echo = TcpListener::bind(("127.0.0.1", 0)).expect("a port");
    let address = echo.local_addr().expect("bound");
    thread::spawn(move || {
        let (mut stream, _) = echo.accept().expect("accepted");
        stream.write_all(b"straight").expect("said");
    });
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("the relay");
    assert!(connect(&mut stream, &address.to_string()).starts_with("HTTP/1.1 200"));
    let mut said = [0u8; 8];
    stream.read_exact(&mut said).expect("read");
    assert_eq!(&said, b"straight");
    let _ = watched.kill();
    let _ = watched.wait();
    let deadline = Instant::now() + Duration::from_secs(10);
    while TcpStream::connect(("127.0.0.1", port)).is_ok() {
        assert!(
            Instant::now() < deadline,
            "the relay outlived the process it watched"
        );
        // Polled, since the relay looks for it every 2 s and says nothing.
        thread::sleep(Duration::from_millis(100));
    }
}
