//! The relay, run as its program against an origin of the tests' own: an
//! HTTP/2 server over TLS, with a certificate made as the tests run.
//!
//! Each test asks the relay's socket as pi does, in HTTP/1.1, and reads
//! what the origin was handed: a request passes whole, only the origin is
//! reached, many requests share a few connections at once, answers stream
//! as they come, a connection the origin lets go is opened again, an
//! origin out of reach is said as a 502 or a 504 without queueing, a
//! connection that won't open isn't asked for again on every request, nor
//! does a failure outlast the origin coming back, the relay leaves once
//! idle but never mid-answer, even while refusing other hosts, its socket
//! is the account's alone and never taken from a relay that answers it,
//! and nothing of a header or a body reaches the relay's log, a failure
//! included.
//!
//! A request hyper hands back unsent goes again on another connection;
//! no test makes that happen when it's wanted, since it needs a goodbye
//! to cross a request on its way out.
//!
//! Long because its cases are most of it: each runs the relay's program
//! against the one origin made here, with its certificate, routes and
//! counters, which splitting would leave two files making alike.

use std::convert::Infallible;
use std::io::{self, Read};
use std::net::SocketAddr;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bytes::Bytes;
use http_body_util::channel::Channel;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::{TokioExecutor, TokioIo};
use rcgen::{BasicConstraints, CertificateParams, IsCa, Issuer, KeyPair};
use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio::net::{TcpListener, UnixStream};
use tokio::sync::Notify;
use tokio_rustls::TlsAcceptor;

/// A header value and a body no log line may hold.
const SECRET_TOKEN: &str = "Bearer sekrit-token-4f1c";
const SECRET_BODY: &[u8] = b"{\"prompt\":\"sekrit-body-9a2e\"}";

/// How long the origin waits between the two halves of a streamed answer.
const STREAM_GAP: Duration = Duration::from_millis(1500);
/// How long it waits between the halves of `/long`, past a relay's idle
/// time of a second and the grace it gives before leaving.
const LONG_GAP: Duration = Duration::from_secs(3);
/// How long the origin holds each request the sharing test sends.
const HELD: Duration = Duration::from_millis(300);
/// How long the tests wait for the relay to listen.
const LISTENS_WITHIN: Duration = Duration::from_secs(10);
/// How long the relay is given to hear the origin's goodbye on a
/// connection before it's asked again, as a relay sent one moments ago is.
const GOODBYE_HEARD_WITHIN: Duration = Duration::from_millis(200);
/// How long the tests let the relay take to reach an origin, in seconds.
const CONNECT_WITHIN: &str = "1";
/// How often a test looks again for something it waits on.
const LOOK_EVERY: Duration = Duration::from_millis(20);
/// How far apart the tests that ask over a stretch of time ask, close
/// enough that a relay counting the asks would never see quiet.
const ASK_EVERY: Duration = Duration::from_millis(100);

/// What the origin was handed, one request at a time.
#[derive(Clone, Debug)]
struct Handed {
    method: String,
    uri: String,
    version: hyper::Version,
    headers: Vec<(String, Vec<u8>)>,
    body: Vec<u8>,
}

/// The tests' origin: where it listens, the root its certificate chains
/// to, and what it has been handed.
struct Origin {
    address: SocketAddr,
    roots: PathBuf,
    /// Connections taken, TLS or not, and connections ended.
    connections: Arc<AtomicUsize>,
    ended: Arc<AtomicUsize>,
    handed: Arc<Mutex<Vec<Handed>>>,
    held: Arc<Held>,
    _tls: Tls,
}

/// An origin's TLS: what it answers with, and the root a relay trusts
/// it by, kept in a folder of its own. Another origin may answer with the
/// same, as one that comes back does.
#[derive(Clone)]
struct Tls {
    acceptor: TlsAcceptor,
    roots: PathBuf,
    _folder: Arc<tempfile::TempDir>,
}

/// How many `/held` requests the origin holds now, and the most it held
/// at once.
#[derive(Default)]
struct Held {
    now: AtomicUsize,
    most: AtomicUsize,
}

/// What one of the origin's connections shares with its requests: what
/// the origin keeps, and a way for `/bye` to let the connection go.
#[derive(Clone)]
struct Served {
    kept: Arc<Mutex<Vec<Handed>>>,
    held: Arc<Held>,
    bye: Arc<Notify>,
}

/// Which connections the origin answers.
#[derive(Clone, Copy, PartialEq)]
enum Serves {
    All,
    /// It answers the first and holds every later one without a word, as
    /// an origin taking no more connections might.
    TheFirst,
}

/// Starts an origin answering HTTP/2 alone, as the model's proxy does.
async fn origin() -> Origin {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("a port");
    origin_on(listener, Serves::All, tls())
}

/// A certificate for 127.0.0.1 under a root of its own, made now.
fn tls() -> Tls {
    let folder = tempfile::tempdir().expect("a folder for the roots");
    let ca_key = KeyPair::generate().expect("a key for the root");
    let mut ca_params = CertificateParams::new(Vec::<String>::new()).expect("the root's params");
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let ca = ca_params.self_signed(&ca_key).expect("the root");
    let issuer = Issuer::new(ca_params, ca_key);
    let leaf_key = KeyPair::generate().expect("a key for the origin");
    let leaf = CertificateParams::new(vec!["127.0.0.1".to_owned()])
        .expect("the origin's params")
        .signed_by(&leaf_key, &issuer)
        .expect("the origin's certificate");
    let roots = folder.path().join("roots.pem");
    std::fs::write(&roots, ca.pem()).expect("the roots written");

    let chain: Vec<CertificateDer<'static>> = vec![leaf.der().clone()];
    let private = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(leaf_key.serialize_der()));
    let mut config =
        ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .expect("TLS versions")
            .with_no_client_auth()
            .with_single_cert(chain, private)
            .expect("the origin's TLS");
    config.alpn_protocols = vec![b"h2".to_vec()];
    Tls {
        acceptor: TlsAcceptor::from(Arc::new(config)),
        roots,
        _folder: Arc::new(folder),
    }
}

/// Starts an origin as `origin` does, on `listener`, answering what
/// `answering` says with `tls`.
fn origin_on(listener: TcpListener, answering: Serves, tls: Tls) -> Origin {
    let acceptor = tls.acceptor.clone();
    let address = listener.local_addr().expect("its address");
    let connections = Arc::new(AtomicUsize::new(0));
    let ended = Arc::new(AtomicUsize::new(0));
    let handed = Arc::new(Mutex::new(Vec::new()));
    let held = Arc::new(Held::default());
    let (counted, finished, kept, holding) = (
        Arc::clone(&connections),
        Arc::clone(&ended),
        Arc::clone(&handed),
        Arc::clone(&held),
    );
    tokio::spawn(async move {
        let mut silent = Vec::new();
        while let Ok((stream, _)) = listener.accept().await {
            if counted.fetch_add(1, Ordering::SeqCst) > 0 && answering == Serves::TheFirst {
                silent.push(stream);
                continue;
            }
            let acceptor = acceptor.clone();
            let finished = Arc::clone(&finished);
            let served = Served {
                kept: Arc::clone(&kept),
                held: Arc::clone(&holding),
                bye: Arc::new(Notify::new()),
            };
            tokio::spawn(async move {
                let Ok(tls) = acceptor.accept(stream).await else {
                    return;
                };
                let bye = Arc::clone(&served.bye);
                let service =
                    hyper::service::service_fn(move |request| answer(request, served.clone()));
                let connection = hyper::server::conn::http2::Builder::new(TokioExecutor::new())
                    .serve_connection(TokioIo::new(tls), service);
                tokio::pin!(connection);
                tokio::select! {
                    _ = connection.as_mut() => {}
                    () = bye.notified() => {
                        connection.as_mut().graceful_shutdown();
                        let _ = connection.await;
                    }
                }
                finished.fetch_add(1, Ordering::SeqCst);
            });
        }
    });
    Origin {
        address,
        roots: tls.roots.clone(),
        connections,
        ended,
        handed,
        held,
        _tls: tls,
    }
}

type Answer = http_body_util::combinators::BoxBody<Bytes, Infallible>;

/// The origin's answer: `/echo` hands the body back, `/held` waits a
/// moment first, `/stream` sends its halves a gap apart and `/long` a
/// longer one, `/bye` answers
/// and lets its connection go, and `/reset` reads the request and resets
/// its stream rather than answer.
async fn answer(request: Request<Incoming>, served: Served) -> Result<Response<Answer>, io::Error> {
    let Served { kept, held, bye } = served;
    let (parts, body) = request.into_parts();
    let body = body
        .collect()
        .await
        .map(|all| all.to_bytes().to_vec())
        .unwrap_or_default();
    let path = parts.uri.path().to_owned();
    kept.lock().expect("the origin's record").push(Handed {
        method: parts.method.to_string(),
        uri: parts.uri.to_string(),
        version: parts.version,
        headers: parts
            .headers
            .iter()
            .map(|(name, value)| (name.as_str().to_owned(), value.as_bytes().to_vec()))
            .collect(),
        body: body.clone(),
    });
    let answer = match path.as_str() {
        "/stream" | "/long" => {
            let gap = if path == "/long" {
                LONG_GAP
            } else {
                STREAM_GAP
            };
            let (mut sender, body) = Channel::<Bytes, Infallible>::new(2);
            tokio::spawn(async move {
                let _ = sender.send_data(Bytes::from_static(b"first\n")).await;
                // An answer paused mid-body for its route's named gap, so
                // the relay is seen passing what came before the rest.
                tokio::time::sleep(gap).await;
                let _ = sender.send_data(Bytes::from_static(b"second\n")).await;
            });
            Response::builder()
                .header("content-type", "text/event-stream")
                .body(body.boxed())
        }
        "/held" => {
            let now = held.now.fetch_add(1, Ordering::SeqCst) + 1;
            held.most.fetch_max(now, Ordering::SeqCst);
            tokio::time::sleep(HELD).await;
            held.now.fetch_sub(1, Ordering::SeqCst);
            Response::builder().body(Full::new(Bytes::from_static(b"held")).boxed())
        }
        "/bye" => {
            bye.notify_one();
            Response::builder().body(Full::new(Bytes::from_static(b"bye")).boxed())
        }
        "/reset" => return Err(io::Error::other("the origin resets this stream")),
        _ => Response::builder()
            .status(StatusCode::CREATED)
            .header("x-origin-said", "echo")
            .body(Full::new(Bytes::from(body)).boxed()),
    };
    Ok(answer.expect("an answer"))
}

/// The relay, run as its program, and where it listens.
struct Relay {
    child: Child,
    socket: PathBuf,
    _folder: tempfile::TempDir,
}

impl Relay {
    /// Starts the relay for `origin`, over at most `connections`
    /// connections, trusting only `roots`.
    fn start(origin: &str, roots: &Path, connections: usize) -> Relay {
        Relay::start_with(origin, roots, connections, &[])
    }

    /// Starts the relay as `start` does, with `more` flags.
    fn start_with(origin: &str, roots: &Path, connections: usize, more: &[&str]) -> Relay {
        let folder = tempfile::tempdir().expect("a folder for the socket");
        let socket = folder.path().join("agents.sock");
        Relay::start_at(folder, socket, origin, roots, connections, more)
    }

    /// Starts the relay as `start_with` does, listening at `socket`, in
    /// `folder`, which may hold something there already.
    fn start_at(
        folder: tempfile::TempDir,
        socket: PathBuf,
        origin: &str,
        roots: &Path,
        connections: usize,
        more: &[&str],
    ) -> Relay {
        let before = std::fs::symlink_metadata(&socket)
            .ok()
            .map(|there| there.ino());
        let child = Command::new(env!("CARGO_BIN_EXE_model-relay"))
            .arg("--socket")
            .arg(&socket)
            .arg("--origin")
            .arg(origin)
            .arg("--roots")
            .arg(roots)
            .arg("--connections")
            .arg(connections.to_string())
            .args(more)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the relay starts");
        let started = Instant::now();
        while std::fs::symlink_metadata(&socket)
            .ok()
            .map(|there| there.ino())
            == before
            || std::os::unix::net::UnixStream::connect(&socket).is_err()
        {
            assert!(
                started.elapsed() < LISTENS_WITHIN,
                "the relay never listened"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        Relay {
            child,
            socket,
            _folder: folder,
        }
    }

    /// Whether the relay has left, and how.
    fn left(&mut self) -> Option<std::process::ExitStatus> {
        self.child.try_wait().expect("the relay's state")
    }

    /// Whether the relay left on its own, and how, within `within`.
    fn left_within(&mut self, within: Duration) -> Option<std::process::ExitStatus> {
        let started = Instant::now();
        while started.elapsed() < within {
            if let Some(status) = self.left() {
                return Some(status);
            }
            std::thread::sleep(LOOK_EVERY);
        }
        None
    }

    /// Stops the relay and gives back all it said.
    fn said(mut self) -> String {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let mut said = String::new();
        if let Some(mut stderr) = self.child.stderr.take() {
            stderr.read_to_string(&mut said).expect("the relay's log");
        }
        said
    }
}

/// What pi gets back from the socket.
struct Got {
    status: StatusCode,
    headers: hyper::HeaderMap,
    body: Vec<u8>,
}

/// Asks the socket as pi does, in HTTP/1.1 with the Host it names.
async fn ask(socket: &Path, request: Request<Full<Bytes>>) -> Got {
    let stream = UnixStream::connect(socket)
        .await
        .expect("the socket answers");
    let (mut sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(stream))
        .await
        .expect("an HTTP/1.1 connection");
    tokio::spawn(connection);
    let response = sender.send_request(request).await.expect("an answer");
    let (parts, body) = response.into_parts();
    let body = body
        .collect()
        .await
        .expect("the whole body")
        .to_bytes()
        .to_vec();
    Got {
        status: parts.status,
        headers: parts.headers,
        body,
    }
}

/// A request for `path` naming `host`, as pi's fetch sends one.
fn request(host: &str, path: &str, body: &'static [u8]) -> Request<Full<Bytes>> {
    Request::builder()
        .method("POST")
        .uri(path)
        .header("host", host)
        .header("content-length", body.len().to_string())
        .body(Full::new(Bytes::from_static(body)))
        .expect("a request")
}

#[tokio::test(flavor = "current_thread")]
async fn a_request_passes_whole_and_nothing_of_it_is_logged() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let relay = Relay::start(&format!("https://{host}"), &origin.roots, 2);
    let mut asked = request(&host, "/v1/messages?beta=true", SECRET_BODY);
    let headers = asked.headers_mut();
    headers.insert("authorization", SECRET_TOKEN.parse().expect("a value"));
    headers.insert("x-api-key", "key-of-its-own".parse().expect("a value"));
    headers.insert("anthropic-version", "2023-06-01".parse().expect("a value"));
    headers.insert("content-encoding", "zstd".parse().expect("a value"));
    headers.insert("connection", "keep-alive".parse().expect("a value"));
    // A second Connection line naming a header of this hop alone, which
    // hyper's own stripping reads past, since it reads only the first.
    headers.append("connection", "x-this-hop".parse().expect("a value"));
    headers.insert("x-this-hop", "1".parse().expect("a value"));

    let got = ask(&relay.socket, asked).await;

    assert_eq!(got.status, StatusCode::CREATED);
    assert_eq!(got.headers["x-origin-said"], "echo");
    assert_eq!(got.body, SECRET_BODY);
    let handed = origin.handed.lock().expect("the record").clone();
    assert_eq!(handed.len(), 1);
    let one = &handed[0];
    assert_eq!(one.method, "POST");
    assert_eq!(one.version, hyper::Version::HTTP_2);
    assert_eq!(one.uri, format!("https://{host}/v1/messages?beta=true"));
    assert_eq!(one.body, SECRET_BODY);
    let value = |name: &str| {
        one.headers
            .iter()
            .find(|(named, _)| named == name)
            .map(|(_, value)| value.clone())
    };
    assert_eq!(
        value("authorization").as_deref(),
        Some(SECRET_TOKEN.as_bytes())
    );
    assert_eq!(value("x-api-key").as_deref(), Some(&b"key-of-its-own"[..]));
    assert_eq!(
        value("anthropic-version").as_deref(),
        Some(&b"2023-06-01"[..])
    );
    assert_eq!(value("content-encoding").as_deref(), Some(&b"zstd"[..]));
    assert_eq!(
        value("connection"),
        None,
        "HTTP/2 carries no connection header"
    );
    assert_eq!(
        value("x-this-hop"),
        None,
        "a header Connection named went on"
    );
    assert_eq!(value("host"), None, "HTTP/2 names its host as :authority");

    let said = relay.said();
    assert!(
        !said.contains("sekrit"),
        "the relay logged a header or a body: {said}"
    );
    assert!(
        !said.contains("key-of-its-own"),
        "the relay logged a header: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn a_body_of_many_pieces_passes_whole() {
    // Longer than a piece the relay hands h2 and than the stream's window,
    // as a conversation's request is, and not a whole number of either.
    const LONG: usize = 200 * 1024 + 7;
    let origin = origin().await;
    let host = origin.address.to_string();
    let relay = Relay::start(&format!("https://{host}"), &origin.roots, 2);
    let body: Vec<u8> = (0..=u8::MAX).cycle().take(LONG).collect();
    let asked = Request::builder()
        .method("POST")
        .uri("/v1/messages")
        .header("host", &host)
        .header("content-length", LONG.to_string())
        .body(Full::new(Bytes::from(body.clone())))
        .expect("a request");

    let got = ask(&relay.socket, asked).await;

    assert_eq!(got.status, StatusCode::CREATED);
    let handed = origin.handed.lock().expect("the record").clone();
    assert_eq!(handed.len(), 1);
    assert!(
        handed[0].body == body,
        "the origin was handed {} bytes, not the {LONG} sent, or not the same ones",
        handed[0].body.len()
    );
    assert!(got.body == body, "the answer came back changed");
}

#[tokio::test(flavor = "current_thread")]
async fn only_its_origin_is_reached() {
    let origin = origin().await;
    let relay = Relay::start(&format!("https://{}", origin.address), &origin.roots, 2);

    let host = origin.address.to_string();
    let mut absolute = request(&host, "https://other.example/v1/messages", SECRET_BODY);
    *absolute.uri_mut() = "https://other.example/v1/messages".parse().expect("a URI");
    let mut nameless = request(&host, "/v1/messages", SECRET_BODY);
    nameless.headers_mut().remove("host");
    let other_port = request(
        &format!("{}:1", origin.address.ip()),
        "/v1/messages",
        SECRET_BODY,
    );

    for (asked, what) in [
        (
            request("example.com", "/v1/messages", SECRET_BODY),
            "another host",
        ),
        (absolute, "another host in the request line"),
        (nameless, "no host"),
        (other_port, "another port"),
    ] {
        let got = ask(&relay.socket, asked).await;
        assert_eq!(got.status, StatusCode::MISDIRECTED_REQUEST, "{what}");
    }
    assert_eq!(
        origin.handed.lock().expect("the record").len(),
        0,
        "a refused request reached the origin"
    );
    let said = relay.said();
    assert!(!said.contains("sekrit"), "the relay logged a body: {said}");
}

#[tokio::test(flavor = "current_thread")]
async fn many_requests_share_a_few_connections() {
    const ASKED: usize = 24;
    let origin = origin().await;
    let host = origin.address.to_string();
    let relay = Relay::start(&format!("https://{host}"), &origin.roots, 2);

    let asks: Vec<_> = (0..ASKED)
        .map(|_| {
            let (socket, host) = (relay.socket.clone(), host.clone());
            tokio::spawn(async move { ask(&socket, request(&host, "/held", b"")).await })
        })
        .collect();
    for asked in asks {
        assert_eq!(asked.await.expect("an ask").status, StatusCode::OK);
    }

    assert_eq!(origin.handed.lock().expect("the record").len(), ASKED);
    let connections = origin.connections.load(Ordering::SeqCst);
    assert!(
        (1..=2).contains(&connections),
        "{ASKED} requests took {connections} connections to the origin"
    );
    let most = origin.held.most.load(Ordering::SeqCst);
    assert!(
        most > 2,
        "the origin held at most {most} requests at once, so they went one a connection"
    );
    drop(relay.said());
}

#[tokio::test(flavor = "current_thread")]
async fn a_connection_the_origin_lets_go_is_opened_again() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let relay = Relay::start(&format!("https://{host}"), &origin.roots, 1);

    assert_eq!(
        ask(&relay.socket, request(&host, "/bye", b"")).await.status,
        StatusCode::OK
    );
    let said_bye = Instant::now();
    while origin.ended.load(Ordering::SeqCst) == 0 {
        assert!(
            said_bye.elapsed() < LISTENS_WITHIN,
            "the origin's connection never ended"
        );
        tokio::time::sleep(LOOK_EVERY).await;
    }
    // The origin has closed, and the relay reads that on its own time.
    tokio::time::sleep(GOODBYE_HEARD_WITHIN).await;
    for _ in 0..3 {
        let got = ask(&relay.socket, request(&host, "/echo", b"again")).await;
        assert_eq!(
            got.status,
            StatusCode::CREATED,
            "a request after the origin's goodbye"
        );
    }

    assert_eq!(origin.connections.load(Ordering::SeqCst), 2);
    drop(relay.said());
}

#[tokio::test(flavor = "current_thread")]
async fn a_request_the_origin_resets_is_a_502_and_nothing_of_it_is_logged() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let relay = Relay::start(&format!("https://{host}"), &origin.roots, 1);
    let mut asked = request(&host, "/reset", SECRET_BODY);
    asked
        .headers_mut()
        .insert("authorization", SECRET_TOKEN.parse().expect("a value"));

    let got = ask(&relay.socket, asked).await;

    assert_eq!(got.status, StatusCode::BAD_GATEWAY);
    let said = relay.said();
    assert!(
        said.contains("couldn't be carried"),
        "the failure went unsaid: {said}"
    );
    assert!(
        !said.contains("sekrit"),
        "the relay logged a header or a body: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn an_origin_that_never_answers_is_a_504_without_a_queue() {
    const ASKED: usize = 8;
    let origin = origin().await;
    // A port that takes connections and never says a word of TLS.
    let silent = TcpListener::bind("127.0.0.1:0").await.expect("a port");
    let host = silent.local_addr().expect("its address").to_string();
    tokio::spawn(async move {
        let mut held = Vec::new();
        while let Ok((stream, _)) = silent.accept().await {
            held.push(stream);
        }
    });
    let relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        2,
        &["--connect-within", CONNECT_WITHIN],
    );
    let asked = Instant::now();

    let asks: Vec<_> = (0..ASKED)
        .map(|_| {
            let (socket, host) = (relay.socket.clone(), host.clone());
            tokio::spawn(async move { ask(&socket, request(&host, "/v1/messages", b"")).await })
        })
        .collect();
    for one in asks {
        assert_eq!(
            one.await.expect("an ask").status,
            StatusCode::GATEWAY_TIMEOUT
        );
    }

    let took = asked.elapsed();
    assert!(
        took < Duration::from_millis(2500),
        "{ASKED} requests took {took:?}, each waiting on the one before"
    );
    drop(relay.said());
}

#[tokio::test(flavor = "current_thread")]
async fn a_relay_leaves_once_idle_but_never_mid_answer() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let mut relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &["--idle-exit", "1"],
    );

    let streamed = ask(&relay.socket, request(&host, "/stream", b"")).await;

    assert_eq!(
        streamed.body, b"first\nsecond\n",
        "an answer longer than the idle time came whole"
    );
    let left = relay.left_within(Duration::from_secs(5));
    assert!(
        left.is_some_and(|status| status.success()),
        "the relay didn't leave once idle: {left:?}"
    );
    let said = relay.said();
    assert!(said.contains("idle"), "it left without saying why: {said}");
}

#[tokio::test(flavor = "current_thread")]
async fn an_answer_streams_as_it_comes() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let relay = Relay::start(&format!("https://{host}"), &origin.roots, 2);
    let stream = UnixStream::connect(&relay.socket)
        .await
        .expect("the socket answers");
    let (mut sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(stream))
        .await
        .expect("an HTTP/1.1 connection");
    tokio::spawn(connection);
    let asked = Instant::now();

    let response = sender
        .send_request(request(&host, "/stream", b""))
        .await
        .expect("an answer");
    let mut body = response.into_body();
    let first = loop {
        let frame = body.frame().await.expect("a frame").expect("a good frame");
        if let Ok(data) = frame.into_data() {
            break data;
        }
    };

    assert_eq!(&first[..], b"first\n");
    assert!(
        asked.elapsed() < STREAM_GAP,
        "the first half waited {:?}, for the second",
        asked.elapsed()
    );
    let rest = body.collect().await.expect("the rest").to_bytes();
    assert_eq!(&rest[..], b"second\n");
    drop(relay.said());
}

#[tokio::test(flavor = "current_thread")]
async fn an_origin_out_of_reach_is_said_as_a_502() {
    let origin = origin().await;
    // A port nothing listens on: bound, read and let go.
    let closed = TcpListener::bind("127.0.0.1:0").await.expect("a port");
    let address = closed.local_addr().expect("its address");
    drop(closed);
    let host = address.to_string();
    let relay = Relay::start(&format!("https://{host}"), &origin.roots, 2);
    let mut asked = request(&host, "/v1/messages", SECRET_BODY);
    asked
        .headers_mut()
        .insert("authorization", SECRET_TOKEN.parse().expect("a value"));

    let got = ask(&relay.socket, asked).await;
    let again = ask(&relay.socket, request(&host, "/v1/messages", b"")).await;

    assert_eq!(got.status, StatusCode::BAD_GATEWAY);
    assert_eq!(
        again.status,
        StatusCode::BAD_GATEWAY,
        "the relay answers on after a failure"
    );
    let said = relay.said();
    assert!(
        !said.contains("sekrit"),
        "the relay logged a header or a body: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn a_failure_lasts_no_longer_than_the_origin_is_away() {
    let away = TcpListener::bind("127.0.0.1:0").await.expect("a port");
    let address = away.local_addr().expect("its address");
    drop(away);
    let host = address.to_string();
    let tls = tls();
    let relay = Relay::start(&format!("https://{host}"), &tls.roots, 2);

    let while_away = ask(&relay.socket, request(&host, "/echo", b"")).await;
    let back = origin_on(
        TcpListener::bind(address).await.expect("the port again"),
        Serves::All,
        tls,
    );
    let first = ask(&relay.socket, request(&host, "/echo", b"")).await;
    let second = ask(&relay.socket, request(&host, "/echo", b"")).await;
    // The second is carried on the first's connection while another is
    // opened behind it, which a failure still on record would stop.
    let asked = Instant::now();
    while back.connections.load(Ordering::SeqCst) < 2 && asked.elapsed() < LISTENS_WITHIN / 5 {
        tokio::time::sleep(LOOK_EVERY).await;
    }

    assert_eq!(while_away.status, StatusCode::BAD_GATEWAY);
    assert_eq!(
        (first.status, second.status),
        (StatusCode::CREATED, StatusCode::CREATED),
        "a failure while the origin was away outlasted it"
    );
    assert_eq!(
        back.connections.load(Ordering::SeqCst),
        2,
        "no second connection was opened once the origin was back"
    );
    drop(relay.said());
}

#[tokio::test(flavor = "current_thread")]
async fn a_connection_that_wont_open_isnt_asked_for_on_every_request() {
    const ASKED: usize = 20;
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("a port");
    let origin = origin_on(listener, Serves::TheFirst, tls());
    let host = origin.address.to_string();
    let relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        2,
        &["--connect-within", CONNECT_WITHIN],
    );
    let asked = Instant::now();

    // Each request is answered on the first connection at once, while the
    // second is opened behind them and never opens. Asked across twice the
    // time an attempt has, a relay trying again on every request would be
    // seen trying again.
    for _ in 0..ASKED {
        let got = ask(&relay.socket, request(&host, "/echo", b"")).await;
        assert_eq!(got.status, StatusCode::CREATED, "a request on the open one");
        tokio::time::sleep(ASK_EVERY).await;
    }

    let took = asked.elapsed();
    assert!(
        took > Duration::from_secs(2),
        "asked within {took:?}, before an attempt could fail"
    );
    assert_eq!(
        origin.connections.load(Ordering::SeqCst),
        2,
        "the relay asked for a connection that wouldn't open again and again"
    );
    drop(relay.said());
}

#[tokio::test(flavor = "current_thread")]
async fn a_relay_refusing_other_hosts_still_leaves_once_idle() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let mut relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &["--idle-exit", "1"],
    );
    let asked = Instant::now();

    // A pi naming another origin is refused, and is no reason to stay.
    while relay.left().is_none() && asked.elapsed() < Duration::from_secs(5) {
        if let Ok(stream) = UnixStream::connect(&relay.socket).await
            && let Ok((mut sender, connection)) =
                hyper::client::conn::http1::handshake(TokioIo::new(stream)).await
        {
            tokio::spawn(connection);
            if let Ok(got) = sender
                .send_request(request("models.elsewhere", "/v1/messages", b""))
                .await
            {
                assert_eq!(got.status(), StatusCode::MISDIRECTED_REQUEST);
            }
        }
        tokio::time::sleep(ASK_EVERY).await;
    }

    let left = relay.left();
    assert!(
        left.is_some_and(|status| status.success()),
        "a relay asked only for other hosts never left: {left:?}"
    );
    drop(relay.said());
}

#[tokio::test(flavor = "current_thread")]
async fn its_socket_is_the_accounts_alone_and_kept_while_it_answers() {
    use std::os::unix::fs::PermissionsExt;
    let origin = origin().await;
    let host = origin.address.to_string();
    let relay = Relay::start(&format!("https://{host}"), &origin.roots, 1);

    let mode = std::fs::metadata(&relay.socket)
        .expect("the socket")
        .permissions()
        .mode();
    let folder = relay.socket.parent().expect("its folder");
    let held: Vec<_> = std::fs::read_dir(folder)
        .expect("the folder")
        .map(|entry| entry.expect("an entry").file_name())
        .collect();
    let second = relay_at(&relay.socket, &host);
    let still = ask(&relay.socket, request(&host, "/echo", b"")).await;

    assert_eq!(mode & 0o777, 0o600, "the socket is open to others");
    assert_eq!(held, ["agents.sock"], "its folder holds more than it");
    assert!(
        !second.status.success() && said_of(&second).contains("already answers"),
        "a second relay took the socket of one answering it: {}",
        said_of(&second)
    );
    assert_eq!(
        still.status,
        StatusCode::CREATED,
        "the first relay lost its socket"
    );
    drop(relay.said());
}

/// A relay run at `socket` for `host`, to its end: it leaves within a
/// second of quiet, so one that took the socket says so in seconds.
fn relay_at(socket: &Path, host: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_model-relay"))
        .arg("--socket")
        .arg(socket)
        .arg("--origin")
        .arg(format!("https://{host}"))
        .args(["--idle-exit", "1"])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .expect("a relay runs")
}

/// What a relay run to its end said.
fn said_of(ran: &std::process::Output) -> String {
    String::from_utf8_lossy(&ran.stderr).into_owned()
}

#[tokio::test(flavor = "current_thread")]
async fn a_file_at_its_socket_is_left_be_and_a_socket_left_behind_is_taken() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let folder = tempfile::tempdir().expect("a folder");
    let notes = folder.path().join("notes.txt");
    std::fs::write(&notes, "kept").expect("a file");
    let left = folder.path().join("agents.sock");
    drop(std::os::unix::net::UnixListener::bind(&left).expect("a socket"));
    // A link to nothing is a file of its own, not a way to somewhere free.
    let link = folder.path().join("link.sock");
    std::os::unix::fs::symlink(folder.path().join("gone"), &link).expect("a link");

    let on_the_file = relay_at(&notes, &host);
    let on_the_link = relay_at(&link, &host);
    let relay = Relay::start_at(
        folder,
        left,
        &format!("https://{host}"),
        &origin.roots,
        1,
        &[],
    );
    let got = ask(&relay.socket, request(&host, "/echo", b"")).await;

    assert!(
        !on_the_file.status.success() && said_of(&on_the_file).contains("isn't a socket"),
        "a relay took a file that isn't a socket: {}",
        said_of(&on_the_file)
    );
    assert_eq!(
        std::fs::read_to_string(&notes).expect("the file"),
        "kept",
        "the file was touched"
    );
    assert!(
        said_of(&on_the_link).contains("isn't a socket") && link.symlink_metadata().is_ok(),
        "a relay took a link to nothing: {}",
        said_of(&on_the_link)
    );
    assert_eq!(
        got.status,
        StatusCode::CREATED,
        "a socket left behind wasn't taken"
    );
    drop(relay.said());
}

#[tokio::test(flavor = "current_thread")]
async fn an_origin_it_cant_trust_sends_pi_direct() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let elsewhere = tls();
    let relay = Relay::start(&format!("https://{host}"), &elsewhere.roots, 1);

    let got = ask(&relay.socket, request(&host, "/echo", b"")).await;

    assert_eq!(
        got.status,
        StatusCode::MISDIRECTED_REQUEST,
        "a fault of the relay's own was said as the origin's"
    );
    drop(relay.said());
}

#[tokio::test(flavor = "current_thread")]
async fn a_relay_answers_beside_an_answer_longer_than_its_idle_time() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let mut relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        2,
        &["--idle-exit", "1"],
    );
    let stream = UnixStream::connect(&relay.socket)
        .await
        .expect("the socket answers");
    let (mut sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(stream))
        .await
        .expect("an HTTP/1.1 connection");
    tokio::spawn(connection);
    let long = sender
        .send_request(request(&host, "/long", b""))
        .await
        .expect("an answer");

    // Past the idle time and the grace, with the long answer still going.
    tokio::time::sleep(LONG_GAP / 2 + LONG_GAP / 4).await;
    let beside = tokio::time::timeout(
        LONG_GAP / 4,
        ask(&relay.socket, request(&host, "/echo", b"")),
    )
    .await;
    let stayed = relay.left().is_none();
    let body = long
        .into_body()
        .collect()
        .await
        .expect("the rest")
        .to_bytes();

    assert_eq!(
        beside.map(|got| got.status).ok(),
        Some(StatusCode::CREATED),
        "a request beside a long answer wasn't answered"
    );
    assert!(stayed, "the relay left mid-answer");
    assert_eq!(&body[..], b"first\nsecond\n");
    drop(relay.said());
}

#[tokio::test(flavor = "current_thread")]
async fn a_connection_pi_keeps_open_doesnt_keep_the_relay() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let mut relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &["--idle-exit", "1"],
    );
    let stream = UnixStream::connect(&relay.socket)
        .await
        .expect("the socket answers");
    let (mut sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(stream))
        .await
        .expect("an HTTP/1.1 connection");
    let kept = tokio::spawn(connection);
    let first = sender
        .send_request(request(&host, "/echo", b""))
        .await
        .expect("an answer");
    drop(first.into_body().collect().await);

    // The sender is kept, so the connection stays open as pi's keep-alive
    // would, until the relay closes it.
    let left = relay.left_within(Duration::from_secs(5));
    let closed = tokio::time::timeout(Duration::from_secs(1), kept).await;

    assert!(
        left.is_some_and(|status| status.success()),
        "a kept connection kept the relay: {left:?}"
    );
    assert!(closed.is_ok(), "the relay left the kept connection open");
    drop(sender);
    drop(relay.said());
}

#[tokio::test(flavor = "current_thread")]
async fn pis_knocking_while_it_leaves_dont_keep_the_relay() {
    let origin = origin().await;
    let mut relay = Relay::start_with(
        &format!("https://{}", origin.address),
        &origin.roots,
        1,
        &["--idle-exit", "1"],
    );
    // A request for another host isn't the relay's work, so it goes idle
    // and leaves while these keep coming, each connection held a while
    // before its request, so some are always open.
    let socket = relay.socket.clone();
    let knocking = tokio::spawn(async move {
        loop {
            let socket = socket.clone();
            tokio::spawn(async move {
                let Ok(stream) = UnixStream::connect(&socket).await else {
                    return;
                };
                let Ok((mut sender, connection)) =
                    hyper::client::conn::http1::handshake(TokioIo::new(stream)).await
                else {
                    return;
                };
                tokio::spawn(connection);
                // Silent for a while, as a pi slow to send would be.
                tokio::time::sleep(HELD).await;
                drop(
                    sender
                        .send_request(request("elsewhere.example", "/echo", b""))
                        .await,
                );
            });
            tokio::time::sleep(LOOK_EVERY).await;
        }
    });

    // Waited on beside the runtime, so the knocking goes on meanwhile.
    let (left, relay) =
        tokio::task::spawn_blocking(move || (relay.left_within(Duration::from_secs(6)), relay))
            .await
            .expect("the wait ends");
    knocking.abort();

    assert!(
        left.is_some_and(|status| status.success()),
        "pis knocking kept a leaving relay: {left:?}"
    );
    drop(relay.said());
}
