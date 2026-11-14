//! The proxy as ssh reaches it: the real program on a socket of its own,
//! a fake agent behind it that signs whatever reaches it, and host keys
//! made for each test, so every refusal is seen not to reach the agent.
//! Some proxies also hold a builders' key and lend it to the builders
//! root's file names, as nix-offload's lent builders are reached.
//!
//! Long because each case drives the real program through the same
//! helpers, a socket, a fake agent and keys made for it, and its cases
//! are each a refusal a fenced command must meet, which read best beside
//! the logins they stop.

use std::fs;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use signature::{Signer, Verifier};
use ssh_key::private::{Ed25519Keypair, KeypairData};
use ssh_key::public::KeyData;
use ssh_key::{HashAlg, LineEnding, PrivateKey, PublicKey, Signature};

const FAILURE: u8 = 5;
const SUCCESS: u8 = 6;
const REQUEST_IDENTITIES: u8 = 11;
const IDENTITIES_ANSWER: u8 = 12;
const SIGN_REQUEST: u8 = 13;
const SIGN_RESPONSE: u8 = 14;
const ADD_IDENTITY: u8 = 17;
const EXTENSION: u8 = 27;

/// The key the fake agent holds, as a wire blob.
const LOGIN_KEY: &[u8] =
    b"\x00\x00\x00\x0bssh-ed25519\x00\x00\x00\x20aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn string(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&u32::try_from(bytes.len()).expect("short").to_be_bytes());
    out.extend_from_slice(bytes);
}

fn send(stream: &mut UnixStream, body: &[u8]) {
    let length = u32::try_from(body.len()).expect("short");
    stream
        .write_all(&length.to_be_bytes())
        .expect("the length is sent");
    stream.write_all(body).expect("the body is sent");
}

fn receive(stream: &mut UnixStream) -> Option<Vec<u8>> {
    let mut length = [0; 4];
    stream.read_exact(&mut length).ok()?;
    let mut body = vec![0; usize::try_from(u32::from_be_bytes(length)).expect("fits")];
    stream.read_exact(&mut body).ok()?;
    Some(body)
}

/// A host key made from `seed`, with its wire blob.
fn host(seed: u8) -> (Ed25519Keypair, Vec<u8>) {
    let keypair = Ed25519Keypair::from_seed(&[seed; 32]);
    let blob = PublicKey::from(KeyData::Ed25519(keypair.public))
        .to_bytes()
        .expect("a host key encodes");
    (keypair, blob)
}

/// `host`'s signature over `session`, as a session binding carries it.
fn signed(host: &Ed25519Keypair, session: &[u8]) -> Vec<u8> {
    let signature: Signature = host.try_sign(session).expect("the host signs");
    Vec::try_from(signature).expect("a signature encodes")
}

/// A `session-bind@openssh.com` request.
fn bind(host_blob: &[u8], session: &[u8], signature: &[u8], forwarding: bool) -> Vec<u8> {
    let mut body = vec![EXTENSION];
    string(&mut body, b"session-bind@openssh.com");
    string(&mut body, host_blob);
    string(&mut body, session);
    string(&mut body, signature);
    body.push(u8::from(forwarding));
    body
}

/// A login as joel, as ssh asks its agent to sign one.
fn login(session: &[u8], key: &[u8]) -> Vec<u8> {
    login_as(session, key, b"joel")
}

/// A login as `user`, as ssh asks its agent to sign one.
fn login_as(session: &[u8], key: &[u8], user: &[u8]) -> Vec<u8> {
    let mut data = Vec::new();
    string(&mut data, session);
    data.push(50);
    string(&mut data, user);
    string(&mut data, b"ssh-connection");
    string(&mut data, b"publickey");
    data.push(1);
    string(&mut data, b"ssh-ed25519");
    string(&mut data, key);
    data
}

/// A request to sign `data` with `key`.
fn sign(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut body = vec![SIGN_REQUEST];
    string(&mut body, key);
    string(&mut body, data);
    body.extend_from_slice(&0u32.to_be_bytes());
    body
}

/// The proxy, the fake agent behind it and how often that agent signed,
/// with what the proxy says kept in a file.
struct Proxy {
    child: Child,
    socket: PathBuf,
    signed: Arc<AtomicUsize>,
    dir: PathBuf,
}

impl Proxy {
    /// A proxy told of the hosts in `hosts`, one OpenSSH key a line.
    fn start(name: &str, hosts: &str) -> Self {
        Self::launch(name, hosts, None, Behind::Signing)
    }

    /// A proxy told of the hosts in `hosts` that also holds `builders`'s
    /// key and lends it to the builders `machines` names, the text of
    /// root's file of the builders it vouches for.
    fn lending(name: &str, hosts: &str, builders: &Ed25519Keypair, machines: &str) -> Self {
        Self::launch(name, hosts, Some((builders, machines)), Behind::Signing)
    }

    /// A lending proxy like [`Proxy::lending`] whose agent behind takes
    /// each question and never answers, as one waiting on its person.
    fn stalled(name: &str, builders: &Ed25519Keypair, machines: &str) -> Self {
        Self::launch(name, "", Some((builders, machines)), Behind::Stalled)
    }

    /// Write the file of lent builders a lending proxy reads afresh at
    /// each bind.
    fn lend(&self, machines: &str) {
        fs::write(self.dir.join("lent"), machines).expect("the lent builders are written");
    }

    fn launch(
        name: &str,
        hosts: &str,
        builders: Option<(&Ed25519Keypair, &str)>,
        behind: Behind,
    ) -> Self {
        let dir =
            std::env::temp_dir().join(format!("pi-agent-proxy-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a folder for the test");
        let upstream = dir.join("upstream.sock");
        let signed = Arc::new(AtomicUsize::new(0));
        match behind {
            Behind::Signing => fake_agent(&upstream, Arc::clone(&signed)),
            Behind::Stalled => stalled_agent(&upstream),
        }
        fs::write(dir.join("hosts"), hosts).expect("the hosts file is written");
        let socket = dir.join("proxy.sock");
        let mut command = Command::new(env!("CARGO_BIN_EXE_pi-agent-proxy"));
        command
            .arg("--hosts")
            .arg(dir.join("hosts"))
            .arg("--upstream")
            .arg(&upstream)
            .arg("--socket")
            .arg(&socket);
        if let Some((builders, machines)) = builders {
            let key = PrivateKey::new(KeypairData::from(builders.clone()), "builders")
                .expect("a builders' key is made");
            let written = key
                .to_openssh(LineEnding::LF)
                .expect("a builders' key writes");
            fs::write(dir.join("builders"), written.as_bytes()).expect("the key is written");
            fs::write(dir.join("lent"), machines).expect("the lent builders are written");
            command
                .arg("--key")
                .arg(dir.join("builders"))
                .arg("--lent")
                .arg(dir.join("lent"));
        }
        let child = command
            .stderr(Stdio::from(
                fs::File::create(dir.join("said")).expect("a file for what it says"),
            ))
            .spawn()
            .expect("the proxy starts");
        let started = Instant::now();
        while !socket.exists() {
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "the proxy never bound its socket"
            );
            // A short poll for the socket the proxy binds as it starts.
            thread::sleep(Duration::from_millis(5));
        }
        Proxy {
            child,
            socket,
            signed,
            dir,
        }
    }

    fn client(&self) -> UnixStream {
        UnixStream::connect(&self.socket).expect("a client reaches the proxy")
    }

    fn signatures(&self) -> usize {
        self.signed.load(Ordering::SeqCst)
    }

    /// Each line the proxy has said so far. It says one before it
    /// answers, so a line is there once its answer is received.
    fn lines(&self) -> Vec<String> {
        fs::read_to_string(self.dir.join("said"))
            .expect("what it said is read")
            .lines()
            .map(str::to_owned)
            .collect()
    }
}

/// The fingerprint a host's key has in the proxy's lines.
fn fingerprint(blob: &[u8]) -> String {
    PublicKey::from_bytes(blob)
        .expect("a host key reads")
        .fingerprint(HashAlg::Sha256)
        .to_string()
}

/// The session this test's process claims, as the proxy reads it from
/// the client's environment: its `PI_SESSION_ID` when that is plain,
/// else `unknown`.
fn claimed() -> String {
    match std::env::var("PI_SESSION_ID") {
        Ok(id)
            if !id.is_empty()
                && id.len() <= 64
                && id
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_') =>
        {
            id
        }
        _ => "unknown".to_owned(),
    }
}

/// The words a refusal's line may give for why, as src/said.rs has them.
const REASONS: &[&str] = &[
    "unreadable",
    "request",
    "extension",
    "forwarded",
    "untold-host",
    "unsigned",
    "no-session",
    "not-login",
    "mismatch",
    "no-holder",
    "upstream",
    "busy",
    "account",
    "not-lent",
    "lent-host",
];

/// Whether `line` has the fixed shape a journal reader keeps fields
/// from: a signature's host, key, account and claimed session, or a
/// refusal's reason word, host and claimed session, with its sentence
/// only after ` -- `.
fn shaped(line: &str) -> bool {
    let Some(said) = line.strip_prefix("pi-agent-proxy: ") else {
        return false;
    };
    let host = |host: &str| host == "none" || (host.starts_with("SHA256:") && !host.contains(' '));
    let session = claimed();
    if let Some(signed) = said.strip_prefix("signed host=") {
        let fields: Vec<&str> = signed.split(' ').collect();
        return fields.len() == 4
            && host(fields[0])
            && ["key=builders", "key=agent"].contains(&fields[1])
            && fields[2]
                .strip_prefix("user=")
                .is_some_and(|user| !user.is_empty())
            && fields[3] == format!("claimed-session={session}");
    }
    let Some(refused) = said.strip_prefix("refused why=") else {
        return false;
    };
    let Some((fields, sentence)) = refused.split_once(" -- ") else {
        return false;
    };
    let Some((why, rest)) = fields.split_once(" host=") else {
        return false;
    };
    let Some((at, claim)) = rest.split_once(" claimed-session=") else {
        return false;
    };
    REASONS.contains(&why) && host(at) && claim == session && !sentence.is_empty()
}

impl Drop for Proxy {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// What the agent behind a test's proxy does.
#[derive(Clone, Copy)]
enum Behind {
    /// Holds `LOGIN_KEY` and signs whatever reaches it.
    Signing,
    /// Takes each client and never answers it.
    Stalled,
}

/// An agent at `path` that takes each client and never answers.
fn stalled_agent(path: &Path) {
    let listener = UnixListener::bind(path).expect("the stalled agent binds");
    thread::spawn(move || {
        let mut held = Vec::new();
        for stream in listener.incoming() {
            held.push(stream.expect("the stalled agent takes each client"));
        }
    });
}

/// An agent at `path` holding `LOGIN_KEY`, signing whatever reaches it.
fn fake_agent(path: &Path, signed: Arc<AtomicUsize>) {
    let listener = UnixListener::bind(path).expect("the fake agent binds");
    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.expect("the fake agent takes each client");
            let signed = Arc::clone(&signed);
            thread::spawn(move || {
                while let Some(request) = receive(&mut stream) {
                    let mut answer = Vec::new();
                    match request.first() {
                        Some(&REQUEST_IDENTITIES) => {
                            answer.push(IDENTITIES_ANSWER);
                            answer.extend_from_slice(&1u32.to_be_bytes());
                            string(&mut answer, LOGIN_KEY);
                            string(&mut answer, b"login key");
                        }
                        Some(&SIGN_REQUEST) => {
                            signed.fetch_add(1, Ordering::SeqCst);
                            answer.push(SIGN_RESPONSE);
                            string(&mut answer, b"the agent's signature");
                        }
                        _ => answer.push(FAILURE),
                    }
                    send(&mut stream, &answer);
                }
            });
        }
    });
}

fn hosts_file(keys: &[&Ed25519Keypair]) -> String {
    let mut text = String::from("# the hosts the fence may log in to\n\n");
    for key in keys {
        let line = PublicKey::from(KeyData::Ed25519(key.public))
            .to_openssh()
            .expect("a host key writes");
        text.push_str(&line);
        text.push('\n');
    }
    text
}

#[test]
fn lists_the_keys_of_the_agents_behind_it() {
    let (allowed, _) = host(1);
    let proxy = Proxy::start("lists", &hosts_file(&[&allowed]));
    let mut client = proxy.client();

    send(&mut client, &[REQUEST_IDENTITIES]);
    let answer = receive(&mut client).expect("an answer");

    let mut expected = vec![IDENTITIES_ANSWER];
    expected.extend_from_slice(&1u32.to_be_bytes());
    string(&mut expected, LOGIN_KEY);
    string(&mut expected, b"login key");
    assert_eq!(answer, expected, "the agent's one key, with its comment");
}

#[test]
fn signs_a_login_to_a_session_its_allowed_host_signed() {
    let (allowed, blob) = host(1);
    let proxy = Proxy::start("signs", &hosts_file(&[&allowed]));
    let mut client = proxy.client();
    let session = b"session one";

    send(
        &mut client,
        &bind(&blob, session, &signed(&allowed, session), false),
    );
    assert_eq!(
        receive(&mut client),
        Some(vec![SUCCESS]),
        "the binding is taken"
    );
    send(&mut client, &sign(LOGIN_KEY, &login(session, LOGIN_KEY)));
    let answer = receive(&mut client).expect("an answer");

    let mut expected = vec![SIGN_RESPONSE];
    string(&mut expected, b"the agent's signature");
    assert_eq!(
        answer, expected,
        "the agent's signature comes back as it gave it"
    );
    assert_eq!(
        proxy.lines(),
        [format!(
            "pi-agent-proxy: signed host={} key=agent user=joel claimed-session={}",
            fingerprint(&blob),
            claimed()
        )],
        "the login it signed is said with its host, key, account and claimed session"
    );
    assert_eq!(proxy.signatures(), 1, "the agent signed once");
}

#[test]
fn refuses_to_sign_with_no_session_bound() {
    let (allowed, _) = host(1);
    let proxy = Proxy::start("unbound", &hosts_file(&[&allowed]));
    let mut client = proxy.client();

    send(
        &mut client,
        &sign(LOGIN_KEY, &login(b"session one", LOGIN_KEY)),
    );

    assert_eq!(
        receive(&mut client),
        Some(vec![FAILURE]),
        "no session, no signature"
    );
    assert_eq!(proxy.signatures(), 0, "the agent was never asked");
}

#[test]
fn refuses_a_host_it_is_not_told_of() {
    let (allowed, _) = host(1);
    let (other, other_blob) = host(2);
    let proxy = Proxy::start("other-host", &hosts_file(&[&allowed]));
    let mut client = proxy.client();
    let session = b"session two";

    send(
        &mut client,
        &bind(&other_blob, session, &signed(&other, session), false),
    );
    assert_eq!(
        receive(&mut client),
        Some(vec![FAILURE]),
        "the binding is refused"
    );
    send(&mut client, &sign(LOGIN_KEY, &login(session, LOGIN_KEY)));

    assert_eq!(
        receive(&mut client),
        Some(vec![FAILURE]),
        "the login is refused"
    );
    assert_eq!(proxy.signatures(), 0, "the agent was never asked");
    let lines = proxy.lines();
    assert!(
        lines[0].starts_with(&format!(
            "pi-agent-proxy: refused why=untold-host host={} claimed-session={} -- ",
            fingerprint(&other_blob),
            claimed()
        )),
        "the binding's refusal names the host it was for: {lines:?}"
    );
    assert!(
        lines[1].starts_with(&format!(
            "pi-agent-proxy: refused why=no-session host=none claimed-session={} -- ",
            claimed()
        )),
        "the login's refusal says it had no session: {lines:?}"
    );
}

#[test]
fn refuses_an_allowed_host_named_for_a_session_it_did_not_sign() {
    let (allowed, blob) = host(1);
    let (other, _) = host(2);
    let proxy = Proxy::start("forged", &hosts_file(&[&allowed]));
    let mut client = proxy.client();
    // Another host's session, passed off as the allowed host's.
    let session = b"a session with another host";

    send(
        &mut client,
        &bind(&blob, session, &signed(&other, session), false),
    );
    assert_eq!(
        receive(&mut client),
        Some(vec![FAILURE]),
        "a forged binding is refused"
    );
    send(&mut client, &sign(LOGIN_KEY, &login(session, LOGIN_KEY)));

    assert_eq!(
        receive(&mut client),
        Some(vec![FAILURE]),
        "its login is refused"
    );
    assert_eq!(proxy.signatures(), 0, "the agent was never asked");
}

#[test]
fn refuses_anything_but_a_login_to_the_bound_session() {
    let (allowed, blob) = host(1);
    let proxy = Proxy::start("not-a-login", &hosts_file(&[&allowed]));
    let mut client = proxy.client();
    let session = b"session one";
    send(
        &mut client,
        &bind(&blob, session, &signed(&allowed, session), false),
    );
    assert_eq!(
        receive(&mut client),
        Some(vec![SUCCESS]),
        "the binding is taken"
    );

    let mut commit = b"SSHSIG".to_vec();
    string(&mut commit, b"git");
    let mut other_key = LOGIN_KEY.to_vec();
    other_key[20] = b'b';
    let asks = [
        ("a commit's signature", sign(LOGIN_KEY, &commit)),
        (
            "a login to another session",
            sign(LOGIN_KEY, &login(b"session two", LOGIN_KEY)),
        ),
        (
            "a login naming another key",
            sign(LOGIN_KEY, &login(session, &other_key)),
        ),
    ];
    for (what, ask) in asks {
        send(&mut client, &ask);
        assert_eq!(
            receive(&mut client),
            Some(vec![FAILURE]),
            "{what} is refused"
        );
    }
    assert_eq!(proxy.signatures(), 0, "the agent was never asked");
    let reasons: Vec<String> = proxy
        .lines()
        .iter()
        .filter_map(|line| line.split_once("why=").map(|(_, rest)| rest.to_owned()))
        .collect();
    let host = fingerprint(&blob);
    let claim = claimed();
    assert!(
        reasons[0].starts_with(&format!("not-login host=none claimed-session={claim} -- "))
            && reasons[1].starts_with(&format!("no-session host=none claimed-session={claim} -- "))
            && reasons[2].starts_with(&format!("mismatch host={host} claimed-session={claim} -- ")),
        "a commit, another session and another key each say why, with the \
         session's host once it's known: {reasons:?}"
    );
}

#[test]
fn refuses_a_session_reached_through_a_forwarded_agent() {
    let (allowed, blob) = host(1);
    let proxy = Proxy::start("forwarded", &hosts_file(&[&allowed]));
    let mut client = proxy.client();
    let session = b"session one";

    send(
        &mut client,
        &bind(&blob, session, &signed(&allowed, session), true),
    );
    assert_eq!(
        receive(&mut client),
        Some(vec![FAILURE]),
        "a forwarding binding is refused"
    );
    send(
        &mut client,
        &bind(&blob, session, &signed(&allowed, session), false),
    );
    receive(&mut client);
    send(&mut client, &sign(LOGIN_KEY, &login(session, LOGIN_KEY)));

    assert_eq!(
        receive(&mut client),
        Some(vec![FAILURE]),
        "nothing is signed on it after"
    );
    assert_eq!(proxy.signatures(), 0, "the agent was never asked");
}

/// How many clients the proxy answers at once, as src/main.rs has it.
const MOST_CLIENTS: usize = 64;

/// Whether `client` is answered when it asks for the keys. One refused
/// may be closed before it asks, so a write that fails is no answer.
fn answered(client: &mut UnixStream) -> bool {
    client.write_all(&[0, 0, 0, 1, REQUEST_IDENTITIES]).is_ok() && receive(client).is_some()
}

#[test]
fn answers_only_so_many_clients_at_once() {
    let (allowed, _) = host(1);
    let proxy = Proxy::start("crowd", &hosts_file(&[&allowed]));
    let mut held: Vec<UnixStream> = (0..MOST_CLIENTS).map(|_| proxy.client()).collect();
    for (n, client) in held.iter_mut().enumerate() {
        assert!(answered(client), "client {n} of {MOST_CLIENTS} is answered");
    }

    let mut one_more = proxy.client();
    assert!(
        !answered(&mut one_more),
        "a client past {MOST_CLIENTS} is closed unanswered"
    );

    drop(held.pop());
    let started = Instant::now();
    while !answered(&mut proxy.client()) {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "a client that went never gave its place back"
        );
        // A short poll while the proxy sees the client that went close.
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn refuses_to_change_the_agents_behind_it() {
    let (allowed, _) = host(1);
    let proxy = Proxy::start("changes", &hosts_file(&[&allowed]));
    let mut client = proxy.client();

    for (what, kind) in [
        ("adding a key", ADD_IDENTITY),
        ("locking", 22),
        ("removing every key", 19),
    ] {
        send(&mut client, &[kind, 0, 0, 0, 0]);
        assert_eq!(
            receive(&mut client),
            Some(vec![FAILURE]),
            "{what} is refused"
        );
    }
}

#[test]
fn says_each_refusal_and_signature_in_one_fixed_shape() {
    let (allowed, blob) = host(1);
    let (other, other_blob) = host(2);
    let proxy = Proxy::start("shaped", &hosts_file(&[&allowed]));
    let mut client = proxy.client();
    let session = b"session one";
    let forged = b"a session with another host";
    let asks = [
        bind(
            &other_blob,
            b"session two",
            &signed(&other, b"session two"),
            false,
        ),
        bind(&blob, forged, &signed(&other, forged), false),
        bind(&blob, session, &signed(&allowed, session), false),
        sign(LOGIN_KEY, &login(session, LOGIN_KEY)),
        sign(LOGIN_KEY, &login(b"session three", LOGIN_KEY)),
        vec![ADD_IDENTITY, 0, 0, 0, 0],
        vec![EXTENSION, 0, 0, 0, 3, b'w', b'h', b'o'],
        vec![SIGN_REQUEST, 0, 0],
    ];
    for ask in asks {
        send(&mut client, &ask);
        receive(&mut client).expect("an answer");
    }

    let lines = proxy.lines();
    assert_eq!(
        lines.len(),
        7,
        "one line each but the binding taken: {lines:?}"
    );
    for line in &lines {
        assert!(shaped(line), "a line a reader can keep fields from: {line}");
    }
    let reasons: Vec<&str> = lines
        .iter()
        .filter_map(|line| line.split_once("why=")?.1.split_once(' '))
        .map(|(why, _)| why)
        .collect();
    assert_eq!(
        reasons,
        [
            "untold-host",
            "unsigned",
            "no-session",
            "request",
            "extension",
            "unreadable"
        ],
        "each refusal names its reason"
    );
}

/// The line for a builder lent by `lender` with `host`'s key, as root
/// writes one in its file of the builders it vouches for.
fn machine(lender: &str, host: &Ed25519Keypair) -> String {
    let key = PublicKey::from(KeyData::Ed25519(host.public))
        .to_openssh()
        .expect("a host key writes");
    format!("{lender} offload-{lender}-abc123 {key}\n")
}

/// The builders' key made for these tests, with its wire blob.
fn builders() -> (Ed25519Keypair, Vec<u8>) {
    host(9)
}

/// A client of `proxy` with a session to `host` bound, or refused.
fn bound(proxy: &Proxy, host: &Ed25519Keypair, blob: &[u8], session: &[u8]) -> (UnixStream, bool) {
    let mut client = proxy.client();
    send(
        &mut client,
        &bind(blob, session, &signed(host, session), false),
    );
    let taken = receive(&mut client) == Some(vec![SUCCESS]);
    (client, taken)
}

#[test]
fn signs_a_lent_builders_login_with_the_builders_key() {
    let (builders, key) = builders();
    let (builder, blob) = host(3);
    let proxy = Proxy::lending("lent", "", &builders, &machine("nix-offload", &builder));
    let session = b"a builder's session";
    let (mut client, taken) = bound(&proxy, &builder, &blob, session);
    assert!(
        taken,
        "a lent builder's binding is taken though --hosts names it not"
    );

    let data = login_as(session, &key, b"nix-offload");
    send(&mut client, &sign(&key, &data));
    let answer = receive(&mut client).expect("an answer");

    assert_eq!(answer.first(), Some(&SIGN_RESPONSE), "the login is signed");
    let signature = Signature::try_from(&answer[5..]).expect("a signature reads");
    KeyData::Ed25519(builders.public)
        .verify(&data, &signature)
        .expect("the builders' key signed the login itself");
    assert_eq!(proxy.signatures(), 0, "the agent behind was never asked");
    assert_eq!(
        proxy.lines(),
        [format!(
            "pi-agent-proxy: signed host={} key=builders user=nix-offload claimed-session={}",
            fingerprint(&blob),
            claimed()
        )],
        "the login is said with the builders' key and account"
    );
}

#[test]
fn lists_the_builders_key_first() {
    let (builders, key) = builders();
    let (builder, _) = host(3);
    let proxy = Proxy::lending(
        "lists-lent",
        "",
        &builders,
        &machine("nix-offload", &builder),
    );
    let mut client = proxy.client();

    send(&mut client, &[REQUEST_IDENTITIES]);
    let answer = receive(&mut client).expect("an answer");

    let mut expected = vec![IDENTITIES_ANSWER];
    expected.extend_from_slice(&2u32.to_be_bytes());
    string(&mut expected, &key);
    string(&mut expected, b"nix-offload builders");
    string(&mut expected, LOGIN_KEY);
    string(&mut expected, b"login key");
    assert_eq!(answer, expected, "the builders' key, then the agent's");
}

#[test]
fn refuses_the_builders_key_as_another_account() {
    let (builders, key) = builders();
    let (builder, blob) = host(3);
    let proxy = Proxy::lending(
        "account",
        "",
        &builders,
        &machine("nix-offload-vm-4", &builder),
    );
    let session = b"a builder's session";
    let (mut client, taken) = bound(&proxy, &builder, &blob, session);
    assert!(taken, "the lent builder's binding is taken");

    send(&mut client, &sign(&key, &login_as(session, &key, b"root")));

    assert_eq!(
        receive(&mut client),
        Some(vec![FAILURE]),
        "root's login is refused"
    );
    let lines = proxy.lines();
    assert!(
        lines[0].contains("why=account "),
        "the refusal says it was the account: {lines:?}"
    );
}

#[test]
fn refuses_the_builders_key_to_a_host_not_lent() {
    let (builders, key) = builders();
    let (allowed, blob) = host(1);
    let (builder, _) = host(3);
    let proxy = Proxy::lending(
        "not-lent",
        &hosts_file(&[&allowed]),
        &builders,
        &machine("nix-offload", &builder),
    );
    let session = b"a told host's session";
    let (mut client, taken) = bound(&proxy, &allowed, &blob, session);
    assert!(taken, "a told host's binding is taken");

    send(
        &mut client,
        &sign(&key, &login_as(session, &key, b"nix-offload")),
    );

    assert_eq!(
        receive(&mut client),
        Some(vec![FAILURE]),
        "the login is refused"
    );
    let lines = proxy.lines();
    assert!(
        lines[0].contains("why=not-lent "),
        "the refusal says the host isn't lent: {lines:?}"
    );
}

#[test]
fn refuses_an_agents_key_to_a_lent_builder() {
    let (builders, _) = builders();
    let (builder, blob) = host(3);
    let proxy = Proxy::lending(
        "lent-host",
        "",
        &builders,
        &machine("nix-offload", &builder),
    );
    let session = b"a builder's session";
    let (mut client, taken) = bound(&proxy, &builder, &blob, session);
    assert!(taken, "the lent builder's binding is taken");

    send(
        &mut client,
        &sign(LOGIN_KEY, &login_as(session, LOGIN_KEY, b"nix-offload")),
    );

    assert_eq!(
        receive(&mut client),
        Some(vec![FAILURE]),
        "the login is refused"
    );
    assert_eq!(proxy.signatures(), 0, "the agent behind was never asked");
    let lines = proxy.lines();
    assert!(
        lines[0].contains("why=lent-host "),
        "the refusal says the host is a lent builder: {lines:?}"
    );
}

#[test]
fn lends_only_to_builders_named_as_such() {
    let (builders, _) = builders();
    let (standing, blob) = host(4);
    let proxy = Proxy::lending(
        "standing",
        "",
        &builders,
        &machine("standing-builder", &standing),
    );

    let (_, taken) = bound(&proxy, &standing, &blob, b"a standing builder's session");

    assert!(
        !taken,
        "a machine not named as a lent builder is no lent host"
    );
    let lines = proxy.lines();
    assert!(
        lines[0].contains("why=untold-host "),
        "the refusal says the host is untold: {lines:?}"
    );
}

#[test]
fn stops_lending_once_a_builder_leaves_roots_file() {
    let (builders, _) = builders();
    let (builder, blob) = host(3);
    let proxy = Proxy::lending("leaves", "", &builders, &machine("nix-offload", &builder));
    let (_, first) = bound(&proxy, &builder, &blob, b"while lent");
    assert!(first, "the builder's binding is taken while it's lent");

    proxy.lend("");
    let (_, second) = bound(&proxy, &builder, &blob, b"once returned");

    assert!(!second, "its binding is refused once it's no longer lent");
}

#[test]
fn refuses_a_lent_builder_reached_through_a_forwarded_agent() {
    let (builders, _) = builders();
    let (builder, blob) = host(3);
    let proxy = Proxy::lending(
        "lent-forwarded",
        "",
        &builders,
        &machine("nix-offload", &builder),
    );
    let mut client = proxy.client();
    let session = b"a builder's session";

    send(
        &mut client,
        &bind(&blob, session, &signed(&builder, session), true),
    );

    assert_eq!(
        receive(&mut client),
        Some(vec![FAILURE]),
        "a forwarded binding is refused"
    );
}

#[test]
fn lists_the_builders_key_though_the_agent_behind_never_answers() {
    let (builders, key) = builders();
    let (builder, _) = host(3);
    let proxy = Proxy::stalled("stalled", &builders, &machine("nix-offload", &builder));
    let mut client = proxy.client();
    let asked = Instant::now();

    send(&mut client, &[REQUEST_IDENTITIES]);
    let answer = receive(&mut client).expect("an answer");

    assert!(
        asked.elapsed() < Duration::from_secs(8),
        "the agent behind is given up on after 5 s, not waited on: {:?}",
        asked.elapsed()
    );
    let mut expected = vec![IDENTITIES_ANSWER];
    expected.extend_from_slice(&1u32.to_be_bytes());
    string(&mut expected, &key);
    string(&mut expected, b"nix-offload builders");
    assert_eq!(answer, expected, "the builders' key alone");
}
