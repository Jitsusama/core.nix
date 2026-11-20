//! The relay, run as its program against an origin of the tests' own: an
//! HTTP/2 server over TLS, with a certificate made as the tests run.
//!
//! Each test asks the relay's socket as pi does, in HTTP/1.1, and reads
//! what the origin was handed: a request passes whole, only the origin is
//! reached, many requests share a few connections at once, answers stream
//! as they come, a connection the origin lets go is opened again, a
//! request the origin resets before its answer retires its connection and
//! goes once more on another, never one kept after a reset, with a burst
//! of them sharing one opened for them, and goes direct with the relay's
//! own 421 when reset again or with nowhere to go, one a fallen connection
//! lost is a 502 and no reset,
//! the origin hearing a lost one once, a body too large or left unfinished
//! goes nowhere, the latter as pi's cancel, never a 421, a relay failing
//! past its share stands down, sending pi direct, tries its path as it
//! stands down, and takes requests again once the path answers, or after,
//! the counts are said as the relay leaves, an origin out of
//! reach is said as a 502 or a 504 without queueing, a
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
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
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
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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
/// How long the origin holds `/reset-once-held` the first time before it
/// resets it, long past the twenty requests that stand a relay down.
const HELD_THEN_RESET: Duration = Duration::from_secs(8);

/// How long `/cut` waits after its first half before its connection falls,
/// and `/broken` before its answer breaks, so the relay has the half first.
const CUT_AFTER: Duration = Duration::from_millis(100);
/// How long `/cut-late` waits after its first half before its connection
/// falls, long past a stand-down of a second and the return after it.
const CUT_LATE: Duration = Duration::from_secs(10);
/// How long the tests wait for the relay to listen.
const LISTENS_WITHIN: Duration = Duration::from_secs(10);
/// How long the relay is given to hear the origin's goodbye on a
/// connection before it's asked again, as a relay sent one moments ago is.
const GOODBYE_HEARD_WITHIN: Duration = Duration::from_millis(200);
/// What `/sized` answers, its length named.
const SIZED: &[u8] = b"whole";
/// What a relay standing down says in its 421, pinned whole, as pi goes
/// direct on these words alone.
const STANDS_DOWN_WORDS: &[u8] =
    b"this relay forwards to its own origin alone, and stands down from it for now, so go \
      directly\n";
/// How long the tests let the relay take to reach an origin, in seconds.
const CONNECT_WITHIN: &str = "1";
/// How often a test looks again for something it waits on.
const LOOK_EVERY: Duration = Duration::from_millis(20);
/// How far apart the tests that ask over a stretch of time ask, close
/// enough that a relay counting the asks would never see quiet.
const ASK_EVERY: Duration = Duration::from_millis(100);

/// How many requests for `path` the origin was handed.
fn handed_to(origin: &Origin, path: &str) -> usize {
    origin
        .handed
        .lock()
        .expect("the origin's record")
        .iter()
        .filter(|handed| handed.uri.ends_with(path))
        .count()
}

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
    /// How it fails every request, as a path that has failed would: one
    /// of `ANSWERS`, `RESETS`, `ANSWERS_THE_FIRST_TWO` or `ANSWERS_502`.
    failing: Arc<AtomicU8>,
    _tls: Tls,
}

/// The origin answers each request as its route says.
const ANSWERS: u8 = 0;
/// The origin resets every request once it has read it.
const RESETS: u8 = 1;
/// The origin answers each connection's first two requests and resets
/// the rest, as fresh connections were reset after one to three on 18
/// November, so only a trial that asks a third finds it failing.
const ANSWERS_THE_FIRST_TWO: u8 = 2;
/// The origin answers every request 502, as one failing behind its front.
const ANSWERS_502: u8 = 3;

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

/// What one of the origin's connections shares with its requests: which
/// of the origin's it is, counted from 0, what the origin keeps, a way for
/// `/bye` to let the connection go, and one for `/drop` to let it fall.
#[derive(Clone)]
struct Served {
    connection: usize,
    kept: Arc<Mutex<Vec<Handed>>>,
    held: Arc<Held>,
    failing: Arc<AtomicU8>,
    /// How many requests this connection has been asked.
    asked: Arc<AtomicUsize>,
    bye: Arc<Notify>,
    gone: Arc<Notify>,
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
    let failing = Arc::new(AtomicU8::new(ANSWERS));
    let (counted, finished, kept, holding, failed) = (
        Arc::clone(&connections),
        Arc::clone(&ended),
        Arc::clone(&handed),
        Arc::clone(&held),
        Arc::clone(&failing),
    );
    tokio::spawn(async move {
        let mut silent = Vec::new();
        while let Ok((stream, _)) = listener.accept().await {
            let connection = counted.fetch_add(1, Ordering::SeqCst);
            if connection > 0 && answering == Serves::TheFirst {
                silent.push(stream);
                continue;
            }
            let acceptor = acceptor.clone();
            let finished = Arc::clone(&finished);
            let served = Served {
                connection,
                kept: Arc::clone(&kept),
                held: Arc::clone(&holding),
                failing: Arc::clone(&failed),
                asked: Arc::new(AtomicUsize::new(0)),
                bye: Arc::new(Notify::new()),
                gone: Arc::new(Notify::new()),
            };
            tokio::spawn(async move {
                let Ok(tls) = acceptor.accept(stream).await else {
                    return;
                };
                let bye = Arc::clone(&served.bye);
                let gone = Arc::clone(&served.gone);
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
                    () = gone.notified() => {}
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
        failing,
        _tls: tls,
    }
}

type Answer = http_body_util::combinators::BoxBody<Bytes, io::Error>;

/// `body`, as the origin answers, though it can't fail.
fn boxed<B>(body: B) -> Answer
where
    B: hyper::body::Body<Data = Bytes, Error = Infallible> + Send + Sync + 'static,
{
    body.map_err(|never| match never {}).boxed()
}

/// The most of a body the relay reads, as its own constant has it.
const BODY_AT_MOST: usize = 64 * 1024 * 1024;

/// The origin's answer: `/echo` hands the body back, `/held` waits a
/// moment first, `/stream` sends its halves a gap apart and `/long` a
/// longer one, `/sized` names its length and ends in a frame of its own,
/// `/broken` fails after its first half, `/cut` lets its connection fall
/// with no word after its first half, `/cut-late` does so `CUT_LATE`
/// after it, `/bye` answers
/// and lets its connection go, `/reset` reads the request and resets
/// its stream rather than answer, `/reset-once` does so the first time
/// it's asked and echoes after, `/reset-held` holds each request
/// `HELD_THEN_RESET` and then resets it, `/reset-once-held` does so the
/// first time it's asked and echoes after, `/reset-on-the-first` does so on the
/// origin's first connection and echoes on any other,
/// `/reset-but-on-the-second` does so on every connection but the
/// origin's second, `/calm-down` resets saying to calm down,
/// `/reset-then-calm-down` resets the first time and says to calm down
/// after, `/drop` reads it and lets the connection fall with no word at
/// all, and `/reset-then-drop` resets the first time and drops after. An
/// echo says which of the origin's connections it came on, counted from
/// 0. While the origin is failing, it fails every request, on any route,
/// once it has read it, as `failing` says.
async fn answer(request: Request<Incoming>, served: Served) -> Result<Response<Answer>, h2::Error> {
    let Served {
        connection,
        kept,
        held,
        failing,
        asked,
        bye,
        gone,
    } = served;
    let nth = asked.fetch_add(1, Ordering::SeqCst) + 1;
    let (parts, body) = request.into_parts();
    let body = body
        .collect()
        .await
        .map(|all| all.to_bytes().to_vec())
        .unwrap_or_default();
    let path = parts.uri.path().to_owned();
    let heard = {
        let mut record = kept.lock().expect("the origin's record");
        record.push(Handed {
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
        record
            .iter()
            .filter(|handed| handed.uri.ends_with(&path))
            .count()
    };
    match failing.load(Ordering::SeqCst) {
        RESETS => return Err(h2::Error::from(h2::Reason::PROTOCOL_ERROR)),
        ANSWERS_THE_FIRST_TWO if nth > 2 => {
            return Err(h2::Error::from(h2::Reason::PROTOCOL_ERROR));
        }
        ANSWERS_502 => {
            let failed = Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(boxed(Full::new(Bytes::new())));
            return Ok(failed.expect("a 502"));
        }
        _ => {}
    }
    if (path == "/reset-once-held" && heard == 1) || path == "/reset-held" {
        tokio::time::sleep(HELD_THEN_RESET).await;
    }
    if let Some(reason) = resets(&path, heard, connection) {
        return Err(h2::Error::from(reason));
    }
    let answer = match path.as_str() {
        "/stream" | "/long" | "/sized" | "/broken" | "/cut" | "/cut-late" => in_halves(&path, gone),
        "/held" => {
            let now = held.now.fetch_add(1, Ordering::SeqCst) + 1;
            held.most.fetch_max(now, Ordering::SeqCst);
            tokio::time::sleep(HELD).await;
            held.now.fetch_sub(1, Ordering::SeqCst);
            Response::builder().body(boxed(Full::new(Bytes::from_static(b"held"))))
        }
        "/bye" => {
            bye.notify_one();
            Response::builder().body(boxed(Full::new(Bytes::from_static(b"bye"))))
        }
        "/drop" | "/reset-then-drop" => {
            gone.notify_one();
            return std::future::pending().await;
        }
        _ => Response::builder()
            .status(StatusCode::CREATED)
            .header("x-origin-said", "echo")
            .header("x-origin-connection", connection.to_string())
            .body(boxed(Full::new(Bytes::from(body)))),
    };
    Ok(answer.expect("an answer"))
}

/// The origin's answer for `path`, one of the routes `answer` names whose
/// body comes in halves, or ends or fails after its first; `/cut` and
/// `/cut-late` let its connection fall by `gone`.
fn in_halves(path: &str, gone: Arc<Notify>) -> hyper::http::Result<Response<Answer>> {
    match path {
        "/sized" => {
            let (mut sender, body) = Channel::<Bytes, Infallible>::new(1);
            tokio::spawn(async move {
                let _ = sender.send_data(Bytes::from_static(SIZED)).await;
            });
            Response::builder()
                .header("content-length", SIZED.len().to_string())
                .body(boxed(body))
        }
        "/broken" => {
            let (mut sender, body) = Channel::<Bytes, io::Error>::new(1);
            tokio::spawn(async move {
                let _ = sender.send_data(Bytes::from_static(b"first\n")).await;
                // The first half goes out before the answer breaks, so it's
                // broken off midway rather than never begun.
                tokio::time::sleep(CUT_AFTER).await;
                sender.abort(io::Error::other("the origin breaks off this answer"));
            });
            Response::builder()
                .header("content-type", "text/event-stream")
                .body(body.boxed())
        }
        "/cut" | "/cut-late" => {
            let after = if path == "/cut-late" {
                CUT_LATE
            } else {
                CUT_AFTER
            };
            let (mut sender, body) = Channel::<Bytes, Infallible>::new(1);
            tokio::spawn(async move {
                let _ = sender.send_data(Bytes::from_static(b"first\n")).await;
                // The first half goes out before the connection falls, so
                // the answer is broken off midway rather than never begun.
                tokio::time::sleep(after).await;
                gone.notify_one();
            });
            Response::builder()
                .header("content-type", "text/event-stream")
                .body(boxed(body))
        }
        _ => {
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
                .body(boxed(body))
        }
    }
}

/// The reason the origin resets a request for `path` with, the `heard`th
/// it was asked for, on its `connection`, if it does: `answer` says which
/// routes do. Each says a protocol error, as the resets of 18 November
/// did, but `/calm-down`'s, which says to calm down. hyper's server resets
/// with the reason of the h2 error its service fails with.
fn resets(path: &str, heard: usize, connection: usize) -> Option<h2::Reason> {
    let resets = match path {
        "/calm-down" => return Some(h2::Reason::ENHANCE_YOUR_CALM),
        "/reset-then-calm-down" if heard > 1 => return Some(h2::Reason::ENHANCE_YOUR_CALM),
        "/reset" | "/reset-held" => true,
        "/reset-once" | "/reset-once-held" | "/reset-then-calm-down" | "/reset-then-drop" => {
            heard == 1
        }
        "/reset-on-the-first" => connection == 0,
        "/reset-but-on-the-second" => connection != 1,
        _ => false,
    };
    resets.then_some(h2::Reason::PROTOCOL_ERROR)
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
async fn ask<B>(socket: &Path, request: Request<B>) -> Got
where
    B: hyper::body::Body + Send + 'static,
    B::Data: Send,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
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
    // Spelt as h2 refuses to send it, which the relay mustn't pass on.
    headers.insert("te", "Trailers".parse().expect("a value"));

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
    assert_eq!(
        value("te").as_deref(),
        Some(&b"trailers"[..]),
        "a TE of trailers wasn't said as h2 sends it"
    );

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
    // Its body read whole first, so pi isn't left writing what the relay
    // stopped reading.
    let (got, answered_after_the_body) = ask_in_halves(&relay.socket, "example.com", "/echo").await;
    assert_eq!(
        got.status,
        StatusCode::MISDIRECTED_REQUEST,
        "another host, its body in halves"
    );
    assert!(
        answered_after_the_body,
        "the other-origin 421 came before the body was read whole"
    );
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
async fn a_request_reset_then_lost_as_it_went_again_goes_direct() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let relay = Relay::start(&format!("https://{host}"), &origin.roots, 1);

    let got = ask(&relay.socket, request(&host, "/reset-then-drop", b"hi")).await;

    assert_eq!(
        (got.status, got.body.as_slice()),
        (
            StatusCode::MISDIRECTED_REQUEST,
            b"this relay can't carry a request to its origin, so go directly\n".as_slice()
        ),
        "a request reset, then lost as it went again, wasn't sent direct"
    );
    assert_eq!(
        handed_to(&origin, "/reset-then-drop"),
        2,
        "a request reset wasn't sent once more, or was sent more"
    );
    let said = relay.said();
    assert!(
        said.contains("couldn't be carried again, so pi goes directly"),
        "a request lost as it went again wasn't said as sent direct: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn a_request_reset_then_told_to_calm_down_is_a_502() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let relay = Relay::start(&format!("https://{host}"), &origin.roots, 1);

    let got = ask(
        &relay.socket,
        request(&host, "/reset-then-calm-down", b"hi"),
    )
    .await;

    // The origin's own verdict, it's for pi's retry to wait out, not to
    // go straight back to the origin with at once.
    assert_eq!(
        (got.status, got.body.as_slice()),
        (
            StatusCode::BAD_GATEWAY,
            b"this relay's request to its origin failed\n".as_slice()
        ),
        "a request told to calm down as it went again wasn't a 502"
    );
    assert_eq!(
        handed_to(&origin, "/reset-then-calm-down"),
        2,
        "a request reset wasn't sent once more, or was sent more"
    );
    let said = relay.said();
    assert!(
        said.contains("pi is answered 502"),
        "a request told to calm down as it went again wasn't said as a 502: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn a_request_the_origin_resets_twice_goes_direct_and_retires_its_connection() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let mut relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &["--idle-exit", "1"],
    );
    let mut asked = request(&host, "/reset", SECRET_BODY);
    asked
        .headers_mut()
        .insert("authorization", SECRET_TOKEN.parse().expect("a value"));

    let got = ask(&relay.socket, asked).await;

    // Reset again on a fresh connection, it goes direct with the relay's
    // own words, as a 502's would go again whole on pi's retry, only later.
    assert_eq!(
        got.status,
        StatusCode::MISDIRECTED_REQUEST,
        "a request the origin reset twice didn't go direct"
    );
    assert_eq!(
        got.body, b"this relay can't carry a request to its origin, so go directly\n",
        "a request reset twice wasn't sent direct in the words pi goes direct on"
    );
    for _ in 0..2 {
        let again = ask(&relay.socket, request(&host, "/echo", b"again")).await;
        assert_eq!(again.status, StatusCode::CREATED, "a request after a reset");
    }
    // The second reset, on 0.1, keeps it, as 0.0 was retired just now, so
    // the echoes go on 0.1 too.
    assert_eq!(
        origin.connections.load(Ordering::SeqCst),
        2,
        "a connection was opened over the one kept after the second reset"
    );
    assert!(
        relay.left_within(LISTENS_WITHIN).is_some(),
        "the relay didn't leave once idle"
    );
    let said = relay.said();
    assert!(
        said.contains("the origin reset a request on connection 0.0, so it's retired"),
        "the reset, or the connection it came on, went unsaid: {said}"
    );
    assert!(
        said.contains("connection 0.1, kept, as one was retired lately"),
        "the second reset's connection wasn't said as kept: {said}"
    );
    assert!(
        said.contains("pi goes directly") && !said.contains("pi is answered 502"),
        "a request reset twice wasn't said as sent direct: {said}"
    );
    // Sent again and reset again, it's a failed reset, not one the resend
    // saved.
    assert!(
        said.contains(
            "3 asked, 0 refused, 1 failed, 1 of them reset; 0 reset and sent again; pi \
             cancelled 0"
        ),
        "a resend reset again wasn't counted as failed: {said}"
    );
    assert!(
        said.contains("connection 0.1 opened"),
        "the connection opened after the reset went unsaid by its number: {said}"
    );
    assert_eq!(
        handed_to(&origin, "/reset"),
        2,
        "a request the origin reset wasn't sent once more, or was sent more than once more"
    );
    assert!(
        !said.contains("sekrit"),
        "the relay logged a header or a body: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn a_request_reset_before_its_answer_goes_once_more_on_a_fresh_connection() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let relay = Relay::start(&format!("https://{host}"), &origin.roots, 1);
    let mut asked = request(&host, "/reset-once", SECRET_BODY);
    asked
        .headers_mut()
        .insert("authorization", SECRET_TOKEN.parse().expect("a value"));

    let got = ask(&relay.socket, asked).await;

    assert_eq!(
        got.status,
        StatusCode::CREATED,
        "a request reset before its answer wasn't sent again"
    );
    assert_eq!(
        got.body, SECRET_BODY,
        "the request sent again lost its body"
    );
    let handed: Vec<Handed> = origin
        .handed
        .lock()
        .expect("the origin's record")
        .iter()
        .filter(|handed| handed.uri.ends_with("/reset-once"))
        .cloned()
        .collect();
    assert_eq!(handed.len(), 2, "the origin didn't hear the request twice");
    for again in &handed {
        assert_eq!(again.body, SECRET_BODY, "a body went short or changed");
        assert!(
            again
                .headers
                .iter()
                .any(|(name, value)| name == "authorization" && value == SECRET_TOKEN.as_bytes()),
            "a request went without its own authorization"
        );
    }
    assert_eq!(
        origin.connections.load(Ordering::SeqCst),
        2,
        "the request went again on the connection that reset it"
    );
    let said = relay.said();
    assert!(
        said.contains("the origin reset a request on connection 0.0, so it's retired"),
        "the reset went unsaid: {said}"
    );
    assert!(
        said.contains("sent again on connection 0.1"),
        "the request sent again didn't say on which connection: {said}"
    );
    assert!(
        !said.contains("sekrit"),
        "the relay logged a header or a body: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn a_reset_on_a_connection_kept_opens_none_over_it() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let relay = Relay::start(&format!("https://{host}"), &origin.roots, 1);

    // Reset on 0.0, which is retired, and again on 0.1, which is kept,
    // since its slot retired one just now.
    let twice = ask(&relay.socket, request(&host, "/reset", b"")).await;
    // Reset on 0.1, kept still, with no other open to go on: opening one
    // over it would spend the allowance its slot kept it by.
    let kept = ask(&relay.socket, request(&host, "/reset-once", b"hello")).await;
    let after = ask(&relay.socket, request(&host, "/reset-once", b"hello")).await;

    assert_eq!(
        twice.status,
        StatusCode::MISDIRECTED_REQUEST,
        "a request reset twice didn't go direct"
    );
    assert_eq!(
        kept.status,
        StatusCode::MISDIRECTED_REQUEST,
        "a request reset on a connection kept went again, or didn't go direct"
    );
    assert_eq!(after.status, StatusCode::CREATED, "a request after them");
    assert_eq!(
        origin.connections.load(Ordering::SeqCst),
        2,
        "a connection was opened over one kept after a reset"
    );
    let said = relay.said();
    assert!(
        said.contains("connection 0.1, kept, as one was retired lately"),
        "the second reset didn't keep its connection: {said}"
    );
    assert!(
        !said.contains("sent again on connection 0.2"),
        "a request went again on a connection opened over one kept: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn a_reset_goes_once_more_on_another_connection_already_open() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let relay = Relay::start(&format!("https://{host}"), &origin.roots, 2);
    warm_both(&relay, &host).await;

    let got = ask(&relay.socket, request(&host, "/reset-once", b"hello")).await;

    assert_eq!(got.status, StatusCode::CREATED, "a request reset once");
    assert_eq!(got.body, b"hello");
    assert_eq!(
        origin.connections.load(Ordering::SeqCst),
        2,
        "a connection was opened for a request another open one could take"
    );
    let said = relay.said();
    let (reset, again) = [("0.0", "1.0"), ("1.0", "0.0")]
        .into_iter()
        .find(|(reset, _)| said.contains(&format!("reset a request on connection {reset},")))
        .unwrap_or_else(|| panic!("the reset went unsaid: {said}"));
    assert!(
        said.contains(&format!(
            "reset on connection {reset} is sent again on connection {again}"
        )),
        "the request reset on {reset} didn't go on the other connection: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn a_reset_on_a_connection_kept_goes_once_more_on_another_open() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let relay = Relay::start(&format!("https://{host}"), &origin.roots, 2);
    warm_both(&relay, &host).await;

    // Taken in turn, slot 0's requests are reset: on 0.0, which is
    // retired, then on 0.1, kept since its slot retired one just now. Each
    // goes once more on 1.0, which the origin answers.
    for _ in 0..10 {
        let got = ask(
            &relay.socket,
            request(&host, "/reset-but-on-the-second", b"hi"),
        )
        .await;
        assert_eq!(got.status, StatusCode::CREATED, "a request one slot resets");
        assert_eq!(got.body, b"hi");
        // A while between requests, for slot 0's connection to open behind
        // the one its request was carried on, so a later turn lands on it.
        tokio::time::sleep(LOOK_EVERY * 5).await;
    }

    assert_eq!(
        origin.connections.load(Ordering::SeqCst),
        3,
        "a connection was opened over one kept after a reset"
    );
    let said = relay.said();
    assert!(
        said.contains("connection 0.1, kept, as one was retired lately"),
        "no reset came on the connection kept: {said}"
    );
    assert!(
        said.contains("reset on connection 0.1 is sent again on connection 1.0"),
        "a request reset on the connection kept didn't go on the other: {said}"
    );
}

/// Opens both of a two-connection relay's connections: the second request
/// is carried on the first's connection while the other is opened behind
/// it, and it's open once a request is answered on it.
async fn warm_both(relay: &Relay, host: &str) {
    let asked = Instant::now();
    while asked.elapsed() < LISTENS_WITHIN {
        let warm = ask(&relay.socket, request(host, "/echo", b"")).await;
        assert_eq!(warm.status, StatusCode::CREATED, "a request to open them");
        if warm.headers["x-origin-connection"] == "1" {
            return;
        }
        tokio::time::sleep(LOOK_EVERY).await;
    }
    panic!("the relay's second connection never took a request");
}

#[tokio::test(flavor = "current_thread")]
async fn a_burst_reset_on_one_connection_shares_the_one_opened_for_it() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let mut relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &["--idle-exit", "1"],
    );
    let path = "/reset-on-the-first";

    let (first, second, third, fourth) = tokio::join!(
        ask(&relay.socket, request(&host, path, b"one")),
        ask(&relay.socket, request(&host, path, b"two")),
        ask(&relay.socket, request(&host, path, b"three")),
        ask(&relay.socket, request(&host, path, b"four")),
    );

    for got in [&first, &second, &third, &fourth] {
        assert_eq!(got.status, StatusCode::CREATED, "a request in the burst");
    }
    assert_eq!(
        (first.body, second.body, third.body, fourth.body),
        (
            b"one".to_vec(),
            b"two".to_vec(),
            b"three".to_vec(),
            b"four".to_vec()
        ),
        "a request sent again lost its own body"
    );
    assert_eq!(
        origin.connections.load(Ordering::SeqCst),
        2,
        "a burst reset on one connection opened more than one for it"
    );
    assert!(
        relay.left_within(LISTENS_WITHIN).is_some(),
        "the relay didn't leave once idle"
    );
    let said = relay.said();
    assert!(
        said.contains(
            "4 asked, 0 refused, 0 failed, 0 of them reset; 4 reset and sent again; pi \
             cancelled 0"
        ),
        "the burst wasn't each reset and sent again once, by the counts: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn a_burst_reset_while_no_connection_opens_waits_on_one_attempt() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("a port");
    let origin = origin_on(listener, Serves::TheFirst, tls());
    let host = origin.address.to_string();
    let mut relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &["--connect-within", CONNECT_WITHIN, "--idle-exit", "1"],
    );

    // Each is reset on the one connection that answers, and the one
    // opened to send them again never does. The origin counts each
    // attempt as it's accepted, so a third would show.
    let got = tokio::join!(
        ask(&relay.socket, request(&host, "/reset", b"")),
        ask(&relay.socket, request(&host, "/reset", b"")),
        ask(&relay.socket, request(&host, "/reset", b"")),
    );

    // Direct, never a 504 or a 502 for pi to retry later down the same
    // path, which a request reset with nowhere else to go would be.
    for answered in [&got.0, &got.1, &got.2] {
        assert_eq!(
            answered.status,
            StatusCode::MISDIRECTED_REQUEST,
            "a request reset with nowhere to go again didn't go direct"
        );
    }
    assert_eq!(
        origin.connections.load(Ordering::SeqCst),
        2,
        "each request reset made its own attempt to connect"
    );
    assert!(
        relay.left_within(LISTENS_WITHIN).is_some(),
        "the relay didn't leave once idle"
    );
    let said = relay.said();
    assert!(
        said.contains("the origin reset a request on connection 0.0, so it's retired"),
        "the reset went unsaid: {said}"
    );
    assert!(
        said.contains("couldn't be sent again, as no other connection would take it"),
        "a request reset and not sent again went unsaid: {said}"
    );
    assert!(
        said.contains(
            "3 asked, 0 refused, 3 failed, 3 of them reset; 0 reset and sent again; pi \
             cancelled 0"
        ),
        "a request reset and not sent again was counted as sent: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn a_body_over_64_mib_goes_nowhere_its_421s_said_and_one_pi_left_its_cancel() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let mut relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &["--idle-exit", "4", "--stand-down-for", "10"],
    );

    let answer = asked_too_much(&relay.socket, &host).await;
    assert!(
        answer.starts_with(b"HTTP/1.1 413"),
        "a body too large was answered {:?}",
        String::from_utf8_lossy(&answer)
    );
    assert_eq!(handed_to(&origin, "/echo"), 0, "a body too large went on");
    // For another origin, it's the 421 that sends pi direct all the same.
    let answer = asked_too_much(&relay.socket, "example.com").await;
    assert!(
        answer.starts_with(b"HTTP/1.1 421"),
        "another origin's body too large was answered {:?}",
        String::from_utf8_lossy(&answer)
    );
    // A body pi leaves is its cancel, though, not a 421 nobody hears.
    left_halfway(&relay.socket, "example.com").await;
    // And standing down, the 421 too, as the origin asked directly may
    // take what the relay won't.
    fail_past_its_share(&relay.socket, &host).await;
    let answer = asked_too_much(&relay.socket, &host).await;
    assert!(
        answer.starts_with(b"HTTP/1.1 421"),
        "a body too large while it stood down was answered {:?}",
        String::from_utf8_lossy(&answer)
    );
    // And one pi leaves while it stands down is its cancel all the same.
    left_halfway(&relay.socket, &host).await;

    assert!(
        relay.left_within(LISTENS_WITHIN * 2).is_some(),
        "the relay didn't leave once idle"
    );
    let said = relay.said();
    assert_eq!(
        said.matches("a request's body passed 64 MiB, so it's refused")
            .count(),
        1,
        "a body too large went unsaid, or another origin's was said as one: {said}"
    );
    assert!(
        said.contains("25 asked, 3 refused, 17 failed")
            && said.contains("; pi cancelled 2;")
            && said.contains("; 1 refused as it stood down"),
        "a body too large wasn't counted as refused, or one pi left as its cancel: {said}"
    );
    // Nothing came once its stand-down was over, so the last were all
    // asked within it, as the counts need.
    assert!(
        !said.contains("so it takes requests again"),
        "a request came once its stand-down was over, so proves nothing of it: {said}"
    );
}

/// Asks for `/echo` at `host` as a pi that leaves halfway through its
/// body, as on Escape, letting be whatever the relay says after.
async fn left_halfway(socket: &Path, host: &str) {
    let stream = UnixStream::connect(socket)
        .await
        .expect("the socket answers");
    let (mut reading, mut writing) = stream.into_split();
    let head = format!("POST /echo HTTP/1.1\r\nhost: {host}\r\ncontent-length: 10\r\n\r\nhi");
    writing
        .write_all(head.as_bytes())
        .await
        .expect("half a body written");
    writing.shutdown().await.expect("pi leaves");
    let mut left = Vec::new();
    // Whatever the relay says to a pi that left, if it says anything,
    // since only its counts tell what it made of it.
    let _ = tokio::time::timeout(LISTENS_WITHIN, reading.read_to_end(&mut left)).await;
}

/// What the relay answers, its head alone, to a request for `/echo` at
/// `host` with a body one byte over what it carries.
async fn asked_too_much(socket: &Path, host: &str) -> Vec<u8> {
    let stream = UnixStream::connect(socket)
        .await
        .expect("the socket answers");
    let (mut reading, mut writing) = stream.into_split();
    let length = BODY_AT_MOST + 1;
    let head = format!("POST /echo HTTP/1.1\r\nhost: {host}\r\ncontent-length: {length}\r\n\r\n");
    // Written apart from the reading, with its failure let be, since the
    // relay answers before it has read the rest and may close on it.
    let writer = tokio::spawn(async move {
        let _ = writing.write_all(head.as_bytes()).await;
        let _ = writing.write_all(&vec![b'x'; length]).await;
    });

    let mut answer = Vec::new();
    let answered = tokio::time::timeout(LISTENS_WITHIN, async {
        let mut piece = [0; 1024];
        while !answer.windows(4).any(|end| end == b"\r\n\r\n") {
            match reading.read(&mut piece).await {
                Ok(0) | Err(_) => break,
                Ok(read) => answer.extend_from_slice(&piece[..read]),
            }
        }
    })
    .await;
    writer.abort();
    assert!(
        answered.is_ok(),
        "the relay never answered a body too large"
    );
    answer
}

/// What pi gets asking with a body in halves, the second sent a while
/// after the first, and whether the answer came only once both had gone.
async fn ask_in_halves(socket: &Path, host: &str, path: &str) -> (Got, bool) {
    let (mut halves, body) = Channel::<Bytes, Infallible>::new(1);
    let written = tokio::spawn(async move {
        let _ = halves
            .send_data(Bytes::from_static(b"the first half, "))
            .await;
        tokio::time::sleep(HELD).await;
        let _ = halves
            .send_data(Bytes::from_static(b"then the second"))
            .await;
        Instant::now()
    });
    let asked = Request::builder()
        .method("POST")
        .uri(path)
        .header("host", host)
        .body(body)
        .expect("a request");
    let got = ask(socket, asked).await;
    let answered = Instant::now();
    let written = written.await.expect("the body written");
    (got, answered >= written)
}

/// Three requests answered and seventeen reset, so seventeen failed of
/// twenty finished, past the share that stands the relay down.
async fn fail_past_its_share(socket: &Path, host: &str) {
    for _ in 0..3 {
        let got = ask(socket, request(host, "/echo", b"hi")).await;
        assert_eq!(got.status, StatusCode::CREATED, "a request answered");
    }
    // Each is reset, and then reset again or kept with nowhere else to go,
    // so it goes direct.
    for _ in 0..17 {
        let got = ask(socket, request(host, "/reset", b"")).await;
        assert_eq!(
            got.status,
            StatusCode::MISDIRECTED_REQUEST,
            "a request the origin reset twice"
        );
        // In the words of one it couldn't carry, not the stand-down's, so
        // it stood down only past its share.
        assert_eq!(
            (got.body.as_slice(), retry_after(&got)),
            (
                b"this relay can't carry a request to its origin, so go directly\n".as_slice(),
                None
            ),
            "a request the origin reset twice went direct as the relay stood down"
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn a_relay_failing_past_its_share_stands_down_sending_pi_direct_then_takes_requests_again() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let mut relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &["--idle-exit", "6", "--stand-down-for", "26"],
    );

    fail_past_its_share(&relay.socket, &host).await;
    let stood_down = Instant::now();

    // Standing down from the last of them, it sends the next direct.
    let got = ask(&relay.socket, request(&host, "/echo", b"hi")).await;
    assert_eq!(
        got.status,
        StatusCode::MISDIRECTED_REQUEST,
        "a request while the relay stands down"
    );
    let words = String::from_utf8_lossy(&got.body).into_owned();
    assert!(
        words.starts_with("this relay ") && words.trim_end().ends_with("so go directly"),
        "the stand-down's 421 doesn't say what sends pi direct: {words}"
    );
    // Standing down retires its connections, so none stays open on the
    // path it stood down from.
    let retiring = Instant::now();
    while origin.ended.load(Ordering::SeqCst) < origin.connections.load(Ordering::SeqCst) {
        assert!(
            retiring.elapsed() < Duration::from_secs(2),
            "a connection stayed open while it stood down"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let first = retry_after(&got);
    assert!(
        first.is_some_and(|seconds| (1..=27).contains(&seconds)),
        "the stand-down's 421 doesn't say it's over within its 26 s: {first:?}"
    );

    // Quiet past its idle time, and the time it takes to see it and go,
    // it's still there, standing down: leaving would forget it. The body
    // comes in halves, the second a while after the head, and is read
    // whole before the 421, so pi isn't left writing what the relay
    // stopped reading: pi's client hands back an answer that came early.
    tokio::time::sleep_until((stood_down + Duration::from_secs(11)).into()).await;
    let (got, answered_after_the_body) = ask_in_halves(&relay.socket, &host, "/echo").await;
    assert_eq!(
        got.status,
        StatusCode::MISDIRECTED_REQUEST,
        "a request while the relay stands down, past its idle time"
    );
    assert!(
        answered_after_the_body,
        "the stand-down's 421 came before the body was read whole"
    );
    assert_eq!(
        got.body, STANDS_DOWN_WORDS,
        "past its idle time, the 421 isn't the stand-down's"
    );
    let later = retry_after(&got);
    assert!(
        later.is_some_and(|seconds| (1..=16).contains(&seconds) && Some(seconds) < first),
        "the stand-down's 421 11 s in doesn't count down from {first:?}: {later:?}"
    );
    assert_eq!(
        handed_to(&origin, "/echo"),
        3,
        "a request went on while it stood down"
    );

    // Past its stand-down, and within its idle time after, it carries one,
    // on a connection opened afresh. It may not ask sooner than the
    // stand-down's end, which came before the last reset was answered.
    // Asked 30 s in, a relay that didn't hold on past its stand-down has
    // left, being idle from 17 s and seeing it once its 26 s are over, by
    // 27.5, and one that did is still there until 32 at the soonest.
    tokio::time::sleep_until((stood_down + Duration::from_secs(30)).into()).await;
    let got = ask(&relay.socket, request(&host, "/echo", b"hi")).await;
    assert_eq!(
        got.status,
        StatusCode::CREATED,
        "a request once it stood down"
    );
    assert_eq!(got.body, b"hi");

    // Left once idle, it says its counts.
    assert!(
        relay.left_within(Duration::from_secs(20)).is_some(),
        "the relay didn't leave once idle after its stand-down"
    );
    let said = relay.said();
    assert!(
        said.contains(
            "17 of the 20 requests it finished in ten minutes failed, so it stands down for 26 \
             s, retiring its connections"
        ),
        "the stand-down went unsaid: {said}"
    );
    assert!(
        said.contains(
            "it stands down 26 s, too short to try its path every 30 s, so a stand-down ends \
             on its timer alone"
        ),
        "a stand-down too short for any trial went unsaid: {said}"
    );
    let again = said
        .find("it stood down 26 s, so it takes requests again, on connections opened afresh")
        .unwrap_or_else(|| panic!("taking requests again went unsaid: {said}"));
    // The connection's own line, as the counts said on leaving say
    // "opened" too. This only shows one opened after the return; that the
    // return retires those before it is the retire test's to pin.
    assert!(
        said[again..]
            .lines()
            .any(|line| line.starts_with("model-relay: connection ") && line.contains(" opened, ")),
        "no connection was opened afresh once it took requests again: {said}"
    );
    assert!(
        said.contains("; 2 refused as it stood down"),
        "the counts don't say what the stand-down sent direct: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn standing_down_it_tries_its_path_and_takes_requests_again_once_it_answers() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &["--stand-down-for", "600", "--try-path-every", "1"],
    );
    let trials = || trials_of(&origin);

    fail_past_its_share(&relay.socket, &host).await;
    origin.failing.store(RESETS, Ordering::SeqCst);
    let stood_down = Instant::now();
    // Its first trial waits two periods, not one, so none has come a
    // period and a half in.
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert_eq!(
        trials(),
        0,
        "its path was tried before its first trial was due"
    );

    // Tried 2 s in and each second after, a path still failing keeps it
    // standing down. Three trials by 5 s in, as one every two seconds
    // would make two.
    tokio::time::sleep_until((stood_down + Duration::from_secs(5)).into()).await;
    let got = ask(&relay.socket, request(&host, "/echo", b"hi")).await;
    assert_eq!(
        got.status,
        StatusCode::MISDIRECTED_REQUEST,
        "a request while its path still fails wasn't sent direct"
    );
    let words = String::from_utf8_lossy(&got.body).into_owned();
    assert!(
        words.contains("stands down from it"),
        "while its path fails, the 421 isn't the stand-down's: {words}"
    );
    let tried = trials();
    assert!(
        tried >= 3,
        "its path wasn't tried each second as it stood down: {tried} trials"
    );
    assert_eq!(
        handed_to(&origin, "/echo"),
        3,
        "a request went on while it stood down"
    );

    // A path that answers a trial's first two requests and fails the
    // third, or answers each with a 502, still keeps it standing down.
    for (failing, how) in [
        (ANSWERS_THE_FIRST_TWO, "failing partway"),
        (ANSWERS_502, "answering 502"),
    ] {
        origin.failing.store(failing, Ordering::SeqCst);
        let before = trials();
        tokio::time::sleep(Duration::from_secs(3)).await;
        let got = ask(&relay.socket, request(&host, "/echo", b"hi")).await;
        assert_eq!(
            got.status,
            StatusCode::MISDIRECTED_REQUEST,
            "a request while its path was {how} wasn't sent direct"
        );
        assert!(
            trials() > before,
            "its path wasn't tried while it was {how}"
        );
    }

    // Mended, the path answers its next trial, a second or so on, and the
    // relay takes requests again then, not ten minutes on.
    origin.failing.store(ANSWERS, Ordering::SeqCst);
    let mended = Instant::now();
    loop {
        let got = ask(&relay.socket, request(&host, "/echo", b"hi")).await;
        if got.status == StatusCode::CREATED {
            break;
        }
        assert_eq!(
            got.status,
            StatusCode::MISDIRECTED_REQUEST,
            "a request as its path mended"
        );
        assert!(
            mended.elapsed() < Duration::from_secs(10),
            "it stood down on, 10 s after its path answered again"
        );
        tokio::time::sleep(ASK_EVERY).await;
    }
    let taken = trials();
    // Taking requests again, it tries its path no more.
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert_eq!(
        trials(),
        taken,
        "it tried its path after it took requests again"
    );

    stands_down_doubled_and_waits_twice_as_long(&relay, &origin, &host).await;
    let said = relay.said();
    assert!(
        said.contains("so it stands down for 1200 s"),
        "the second stand-down soon after didn't double: {said}"
    );
    says_each_trial(&said);
}

#[tokio::test(flavor = "current_thread")]
async fn a_relay_leaving_says_its_trials_apart_from_its_connections_in_its_counts() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let mut relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &[
            "--idle-exit",
            "6",
            "--stand-down-for",
            "26",
            "--try-path-every",
            "1",
        ],
    );

    // Standing down, it tries its path, which answers, so it takes
    // requests again. Mended, it holds on until it's asked, and then,
    // idle, leaves, saying its counts.
    fail_past_its_share(&relay.socket, &host).await;
    let stood_down = Instant::now();
    while ask(&relay.socket, request(&host, "/echo", b"hi"))
        .await
        .status
        != StatusCode::CREATED
    {
        assert!(
            stood_down.elapsed() < Duration::from_secs(10),
            "it stood down on, 10 s after its path answered again"
        );
        tokio::time::sleep(ASK_EVERY).await;
    }
    assert!(
        relay.left_within(Duration::from_secs(20)).is_some(),
        "the relay didn't leave once idle after its path answered"
    );
    counts_trials_apart(&relay.said());
}

/// The counts said as it left give its trials a figure of their own, one
/// for each trial connection it said, and count as opened only the
/// connections for requests.
fn counts_trials_apart(said: &str) {
    let counts = said
        .lines()
        .rfind(|line| line.contains(" trials of its path"))
        .unwrap_or_else(|| panic!("no counts said its trials: {said}"));
    let figure = |of: &str| -> usize {
        counts
            .split("; ")
            .find(|part| part.contains(of))
            .and_then(|part| part.split(' ').next())
            .and_then(|number| number.parse().ok())
            .unwrap_or_else(|| panic!("the counts don't say {of}: {counts}"))
    };
    let said_opened = |by: &str| {
        said.lines()
            .filter(|line| line.starts_with(by) && line.contains(" opened"))
            .count()
    };
    let trials = said_opened("model-relay: trial connection ");
    assert!(trials > 0, "no trial connection was said: {said}");
    assert_eq!(
        (figure(" trials of its path"), figure(" connections opened")),
        (trials, said_opened("model-relay: connection ")),
        "the counts didn't say trials apart from connections for requests: {counts}"
    );
}

/// Failing again soon after it took requests again, a relay trying its
/// path each second stands down doubled, for twenty minutes, and waits
/// twice as long for its first trial: four periods, so none by three
/// seconds in, and one by seven. The requests it answered since it mended
/// count toward its share, so it's reset until it says it stands down,
/// rather than a set number of times.
#[tokio::test(flavor = "current_thread")]
async fn a_stand_down_begun_while_it_waits_on_a_trial_waits_its_own_first_trial_from_then() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &["--stand-down-for", "15", "--try-path-every", "7"],
    );
    let trials = || trials_of(&origin);

    fail_past_its_share(&relay.socket, &host).await;
    origin.failing.store(RESETS, Ordering::SeqCst);
    let stood_down = Instant::now();
    // Its first trial comes 14 s in and fails, so it waits on the next,
    // 21 s in, when the stand-down ends on its timer at 15 s.
    tokio::time::sleep_until((stood_down + Duration::from_millis(15_500)).into()).await;
    assert!(
        trials() > 0,
        "its first trial didn't come before its stand-down ended"
    );

    // Failing again at once, it stands down doubled, 30 s, while the trial
    // loop still waits on the last stand-down's next trial.
    let mut resets = 0;
    loop {
        let got = ask(&relay.socket, request(&host, "/reset", b"")).await;
        if String::from_utf8_lossy(&got.body).contains("stands down from it") {
            break;
        }
        resets += 1;
        assert!(
            resets <= 20,
            "20 resets in a row didn't stand it down again"
        );
    }
    let again = Instant::now();
    let before = trials();

    // Its own first trial is due 28 s from then, two periods doubled. Told
    // of it, the loop waits from then; left asleep, it would wait from when
    // its last wait ended, about 21 s in, so 5 s later.
    tokio::time::sleep_until((again + Duration::from_secs(25)).into()).await;
    assert_eq!(
        trials(),
        before,
        "a stand-down begun mid-wait tried its path before its own first trial"
    );
    tokio::time::sleep_until((again + Duration::from_secs(31)).into()).await;
    assert!(
        trials() > before,
        "a stand-down begun while it waited on a trial waited out the old wait first"
    );
    let said = relay.said();
    assert!(
        said.contains("so it stands down for 30 s"),
        "the second stand-down didn't double: {said}"
    );
}

async fn stands_down_doubled_and_waits_twice_as_long(relay: &Relay, origin: &Origin, host: &str) {
    let trials = || trials_of(origin);
    let mut resets = 0;
    loop {
        let got = ask(&relay.socket, request(host, "/reset", b"")).await;
        assert_eq!(
            got.status,
            StatusCode::MISDIRECTED_REQUEST,
            "a request the origin reset twice"
        );
        if String::from_utf8_lossy(&got.body).contains("stands down from it") {
            break;
        }
        resets += 1;
        assert!(
            resets <= 20,
            "20 resets in a row didn't stand it down again"
        );
    }
    origin.failing.store(RESETS, Ordering::SeqCst);
    let doubled = Instant::now();
    let before = trials();
    tokio::time::sleep_until((doubled + Duration::from_secs(3)).into()).await;
    assert_eq!(
        trials(),
        before,
        "a doubled stand-down tried its path as soon as a first one would"
    );
    tokio::time::sleep_until((doubled + Duration::from_secs(7)).into()).await;
    assert!(
        trials() > before,
        "a doubled stand-down never tried its path"
    );
}

/// How many trials of its path the origin was asked, each a `HEAD`.
fn trials_of(origin: &Origin) -> usize {
    origin
        .handed
        .lock()
        .expect("the origin's record")
        .iter()
        .filter(|handed| handed.method == "HEAD")
        .count()
}

/// That what a relay `said` names each of its trials as they went: one
/// failing at once, partway, or answered 502, on a connection of its own
/// that closed, then one answered, which took requests again.
fn says_each_trial(said: &str) {
    assert!(
        said.contains("failed request 1 of 3") && said.contains("so it stands down still"),
        "a trial that failed went unsaid: {said}"
    );
    assert!(
        said.contains("failed request 3 of 3"),
        "a trial failing at its third request went unsaid: {said}"
    );
    assert!(
        said.contains("failed request 1 of 3: answered 502 Bad Gateway"),
        "a trial answered 502 went unsaid: {said}"
    );
    assert!(
        said.contains("answered 3 requests in a row"),
        "the trial that answered went unsaid: {said}"
    );
    assert!(
        said.contains("model-relay: trial connection 0 opened\n")
            && said.contains("model-relay: trial connection 0 closed"),
        "a trial's connection wasn't said as its own, or wasn't closed: {said}"
    );
    assert!(
        said.contains("and its path answers again, so it takes requests again"),
        "taking requests again on its path answering went unsaid: {said}"
    );
}

/// The seconds a 421's Retry-After says, if it says any.
fn retry_after(got: &Got) -> Option<u64> {
    got.headers
        .get("retry-after")
        .and_then(|value| value.to_str().ok()?.parse::<u64>().ok())
}

#[tokio::test(flavor = "current_thread")]
async fn taking_requests_again_it_retires_a_connection_opened_while_it_stood_down() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &["--stand-down-for", "12"],
    );

    // Taken before the stand-down, and held by the origin through its
    // start, then reset, so it goes once more on a connection opened while
    // the relay stands down.
    let socket = relay.socket.clone();
    let to = host.clone();
    let held =
        tokio::spawn(async move { ask(&socket, request(&to, "/reset-once-held", b"held")).await });
    let reached = Instant::now();
    while handed_to(&origin, "/reset-once-held") == 0 {
        assert!(
            reached.elapsed() < LISTENS_WITHIN,
            "the held request never reached the origin"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    fail_past_its_share(&relay.socket, &host).await;
    let stood_down = Instant::now();
    assert!(
        !held.is_finished() && handed_to(&origin, "/reset-once-held") == 1,
        "the held request went once more before the stand-down, so proves nothing"
    );
    let got = held.await.expect("the held request");
    assert_eq!(
        got.status,
        StatusCode::CREATED,
        "the held request, sent once more"
    );
    assert!(
        stood_down.elapsed() < Duration::from_secs(11),
        "the held request was answered after the stand-down, so proves nothing"
    );
    let opened_meanwhile = got.headers.get("x-origin-connection").cloned();

    tokio::time::sleep_until((stood_down + Duration::from_secs(13)).into()).await;
    let got = ask(&relay.socket, request(&host, "/echo", b"hi")).await;
    assert_eq!(
        got.status,
        StatusCode::CREATED,
        "a request once it stood down"
    );
    assert!(
        opened_meanwhile.is_some(),
        "the held request's answer didn't say its connection"
    );
    assert_ne!(
        got.headers.get("x-origin-connection").cloned(),
        opened_meanwhile,
        "taking requests again, it went on the connection opened while it stood down"
    );
    let said = relay.said();
    // The requests reset to stand it down go again too, but before it
    // does, so the last sent again is the held request.
    let stood = said.find("so it stands down for 12 s");
    let again = said.rfind("is sent again on connection");
    assert!(
        stood.is_some() && again.is_some() && stood < again,
        "it didn't stand down before the held request went once more: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn answers_broken_off_midway_as_their_connection_falls_stand_it_down() {
    let origin = origin().await;
    let host = origin.address.to_string();
    // Its stand-down and idle time short, so it leaves and says its counts,
    // yet the stand-down long enough to outlast the request asked after it.
    let mut relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &["--idle-exit", "1", "--stand-down-for", "3"],
    );

    for _ in 0..3 {
        let got = ask(&relay.socket, request(&host, "/echo", b"hi")).await;
        assert_eq!(got.status, StatusCode::CREATED, "a request answered");
    }
    // Each answer begins, so carrying it went well, and is lost midway with
    // its connection: judged only as its answer ends, it fails.
    for _ in 0..17 {
        let (mut pi, connection) = pi_asks(&relay.socket).await;
        let cut = pi
            .send_request(request(&host, "/cut", b""))
            .await
            .expect("an answer begun");
        assert_eq!(cut.status(), StatusCode::OK, "an answer to be cut off");
        assert!(
            cut.into_body().collect().await.is_err(),
            "an answer cut off midway came whole"
        );
        drop(pi);
        connection.abort();
    }
    let got = ask(&relay.socket, request(&host, "/echo", b"hi")).await;

    assert_eq!(
        got.status,
        StatusCode::MISDIRECTED_REQUEST,
        "answers broken off midway didn't stand it down"
    );
    assert!(
        relay.left_within(LISTENS_WITHIN).is_some(),
        "the relay didn't leave once idle after its stand-down"
    );
    let said = relay.said();
    assert!(
        said.contains("17 of the 20 requests it finished in ten minutes failed"),
        "the answers broken off weren't judged as failed: {said}"
    );
    assert!(
        said.contains("; 17 broken off midway"),
        "the counts don't say how many answers broke off: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn answers_pi_gives_up_on_never_stand_it_down() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let mut relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &["--idle-exit", "1"],
    );

    // pi hangs up partway through each answer, as its person pressing
    // Escape does: the relay's path did nothing wrong.
    for _ in 0..25 {
        let (mut pi, connection) = pi_asks(&relay.socket).await;
        let long = pi
            .send_request(request(&host, "/long", b""))
            .await
            .expect("an answer begun");
        drop(long);
        drop(pi);
        connection.abort();
    }
    let got = ask(&relay.socket, request(&host, "/echo", b"hi")).await;
    let left = relay.left_within(LISTENS_WITHIN);

    assert_eq!(
        got.status,
        StatusCode::CREATED,
        "a request after pi gave up on 25"
    );
    assert!(left.is_some(), "the relay didn't leave once idle");
    let said = relay.said();
    assert!(
        !said.contains("stands down") && said.contains("pi cancelled 25"),
        "answers pi gave up on were judged, or not counted as its cancels: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn answers_sent_before_it_took_requests_again_are_never_judged() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &["--stand-down-for", "1"],
    );

    // Twenty answers begun before the stand-down, on the one connection,
    // which falls only once the relay takes requests again: they rode a
    // connection it has since retired, so they say nothing of the path.
    let mut late = Vec::new();
    for _ in 0..20 {
        let (mut pi, connection) = pi_asks(&relay.socket).await;
        let cut = pi
            .send_request(request(&host, "/cut-late", b""))
            .await
            .expect("an answer begun");
        assert_eq!(cut.status(), StatusCode::OK, "an answer to be cut off late");
        late.push((tokio::spawn(cut.into_body().collect()), pi, connection));
    }
    fail_past_its_share(&relay.socket, &host).await;
    let stood_down = Instant::now();
    tokio::time::sleep_until((stood_down + Duration::from_millis(1500)).into()).await;
    let got = ask(&relay.socket, request(&host, "/echo", b"hi")).await;
    assert_eq!(
        got.status,
        StatusCode::CREATED,
        "a request once it stood down"
    );
    assert!(
        late.iter().all(|(cut, _, _)| !cut.is_finished()),
        "the answers were cut off before it took requests again, so prove nothing"
    );
    for (cut, pi, connection) in late {
        assert!(
            cut.await.expect("an answer read").is_err(),
            "an answer cut off late came whole"
        );
        drop(pi);
        connection.abort();
    }
    let got = ask(&relay.socket, request(&host, "/echo", b"hi")).await;

    assert_eq!(
        got.status,
        StatusCode::CREATED,
        "answers sent before it took requests again stood it down"
    );
    let said = relay.said();
    assert_eq!(
        said.matches("so it stands down for").count(),
        1,
        "answers sent before it took requests again were judged: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn requests_sent_again_after_it_took_requests_again_are_judged() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &["--stand-down-for", "1"],
    );

    // Twenty first sent before the stand-down, each held and then reset
    // every time it's heard, so each is sent again once the relay takes
    // requests again: judged by when that attempt went, they fail.
    let held: Vec<_> = (0..20)
        .map(|_| {
            let socket = relay.socket.clone();
            let to = host.clone();
            tokio::spawn(async move { ask(&socket, request(&to, "/reset-held", b"")).await })
        })
        .collect();
    let reached = Instant::now();
    while handed_to(&origin, "/reset-held") < 20 {
        assert!(
            reached.elapsed() < LISTENS_WITHIN,
            "the held requests never all reached the origin"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    fail_past_its_share(&relay.socket, &host).await;
    let stood_down = Instant::now();
    tokio::time::sleep_until((stood_down + Duration::from_millis(1500)).into()).await;
    let got = ask(&relay.socket, request(&host, "/echo", b"hi")).await;
    assert_eq!(
        got.status,
        StatusCode::CREATED,
        "a request once it stood down"
    );
    assert_eq!(
        handed_to(&origin, "/reset-held"),
        20,
        "the held requests went again before it took requests again, so prove nothing"
    );
    assert!(
        held.iter().all(|asked| !asked.is_finished()),
        "the held requests were answered before it took requests again, so prove nothing"
    );
    for asked in held {
        let got = asked.await.expect("a held request");
        assert_eq!(
            got.status,
            StatusCode::MISDIRECTED_REQUEST,
            "a held request reset each time didn't go direct"
        );
    }

    let said = relay.said();
    assert_eq!(
        said.matches("so it stands down for").count(),
        2,
        "requests sent again after it took requests again went unjudged: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn answers_the_origin_breaks_off_itself_never_stand_it_down() {
    let origin = origin().await;
    let host = origin.address.to_string();
    // Its idle time short, so it leaves and says its counts.
    let mut relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &["--idle-exit", "1"],
    );

    // Each answer begins, then fails with an error of the origin's own,
    // which would fail direct too: the relay's path did nothing wrong.
    for _ in 0..20 {
        let (mut pi, connection) = pi_asks(&relay.socket).await;
        let broken = pi
            .send_request(request(&host, "/broken", b""))
            .await
            .expect("an answer begun");
        assert_eq!(
            broken.status(),
            StatusCode::OK,
            "an answer to be broken off"
        );
        assert!(
            broken.into_body().collect().await.is_err(),
            "an answer the origin broke off came whole"
        );
        drop(pi);
        connection.abort();
    }
    let got = ask(&relay.socket, request(&host, "/echo", b"hi")).await;

    assert_eq!(
        got.status,
        StatusCode::CREATED,
        "answers the origin broke off itself stood it down"
    );
    assert!(
        relay.left_within(LISTENS_WITHIN).is_some(),
        "the relay didn't leave once idle"
    );
    let said = relay.said();
    assert!(
        !said.contains("stands down"),
        "answers the origin broke off itself were judged as the path failing: {said}"
    );
    assert!(
        said.contains("; 20 broken off midway"),
        "the counts don't say the answers broken off unjudged: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn connections_falling_before_an_answer_stand_it_down() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let relay = Relay::start(&format!("https://{host}"), &origin.roots, 1);

    for _ in 0..3 {
        let got = ask(&relay.socket, request(&host, "/echo", b"hi")).await;
        assert_eq!(got.status, StatusCode::CREATED, "a request answered");
    }
    // Each reaches the origin, which lets its connection fall before it
    // answers: the path failing.
    for _ in 0..17 {
        let got = ask(&relay.socket, request(&host, "/drop", b"")).await;
        assert_eq!(
            got.status,
            StatusCode::BAD_GATEWAY,
            "a request whose connection fell"
        );
    }
    let got = ask(&relay.socket, request(&host, "/echo", b"hi")).await;

    assert_eq!(
        got.status,
        StatusCode::MISDIRECTED_REQUEST,
        "connections falling before an answer didn't stand it down"
    );
    let said = relay.said();
    assert!(
        said.contains("17 of the 20 requests it finished in ten minutes failed"),
        "connections falling weren't judged as failed: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn a_reset_saying_to_calm_down_is_a_502_and_goes_no_more() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let relay = Relay::start(&format!("https://{host}"), &origin.roots, 2);

    let got = ask(&relay.socket, request(&host, "/calm-down", b"")).await;

    assert_eq!(
        got.status,
        StatusCode::BAD_GATEWAY,
        "a request reset saying to calm down"
    );
    assert_eq!(
        handed_to(&origin, "/calm-down"),
        1,
        "a request reset saying to calm down was sent again"
    );
    let said = relay.said();
    assert!(
        said.contains("the origin reset a request on connection 0.0, so it's retired")
            && said.contains("pi is answered 502"),
        "the reset went unsaid: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn a_body_pi_leaves_unfinished_is_counted_as_a_cancel() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let mut relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &["--idle-exit", "1"],
    );
    let mut stream = UnixStream::connect(&relay.socket)
        .await
        .expect("the socket answers");
    let head = format!("POST /echo HTTP/1.1\r\nhost: {host}\r\ncontent-length: 100\r\n\r\npartly");
    stream
        .write_all(head.as_bytes())
        .await
        .expect("the head and part of the body");
    drop(stream);

    assert!(
        relay.left_within(LISTENS_WITHIN).is_some(),
        "the relay didn't leave once idle"
    );
    assert_eq!(
        handed_to(&origin, "/echo"),
        0,
        "a body left unfinished went on"
    );
    let said = relay.said();
    assert!(
        said.contains(
            "1 asked, 0 refused, 0 failed, 0 of them reset; 0 reset and sent again; pi cancelled 1"
        ),
        "a body pi left unfinished wasn't counted as its cancel: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn a_connection_says_how_many_requests_pi_gave_up_on_it() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let mut relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &["--idle-exit", "1"],
    );
    // Answers read whole, one ending in its last frame of data and one
    // after it, and one the origin broke off: none of them pi gave up on.
    let echoed = ask(&relay.socket, request(&host, "/echo", b"hello")).await;
    let sized = ask(&relay.socket, request(&host, "/sized", b"")).await;
    let (mut pi, _) = pi_asks(&relay.socket).await;
    let broken = pi
        .send_request(request(&host, "/broken", b""))
        .await
        .expect("an answer begun")
        .into_body()
        .collect()
        .await;

    // pi gives up before an answer begins and partway through one, as it
    // does when its person presses Escape.
    let (mut pi, connection) = pi_asks(&relay.socket).await;
    let waited =
        tokio::time::timeout(HELD / 3, pi.send_request(request(&host, "/held", b""))).await;
    drop(pi);
    connection.abort();
    let (mut pi, connection) = pi_asks(&relay.socket).await;
    let long = pi
        .send_request(request(&host, "/long", b""))
        .await
        .expect("an answer begun");
    drop(long);
    drop(pi);
    connection.abort();
    let reset = ask(&relay.socket, request(&host, "/reset", b"")).await;
    let left = relay.left_within(LISTENS_WITHIN);

    assert_eq!(echoed.body, b"hello");
    assert_eq!(sized.body, SIZED);
    assert!(broken.is_err(), "an answer the origin broke off came whole");
    assert!(waited.is_err(), "pi was answered before it gave up");
    assert_eq!(
        reset.status,
        StatusCode::MISDIRECTED_REQUEST,
        "a request the origin reset twice didn't go direct"
    );
    assert!(left.is_some(), "the relay didn't leave once idle");
    let said = relay.said();
    assert!(
        said.contains("connection 0.0, so it's retired, after 6 asked and"),
        "the reset didn't say how many requests its connection carried: {said}"
    );
    assert!(
        said.contains("connection 0.0 carried 6 asked, 2 of them cancelled"),
        "the connection didn't count as given up on just the two pi left, or counted a \
         request handed back unsent: {said}"
    );
}

/// A connection to the relay as pi makes one, and the task that runs it.
async fn pi_asks(
    socket: &Path,
) -> (
    hyper::client::conn::http1::SendRequest<Full<Bytes>>,
    tokio::task::JoinHandle<hyper::Result<()>>,
) {
    let stream = UnixStream::connect(socket)
        .await
        .expect("the socket answers");
    let (sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(stream))
        .await
        .expect("an HTTP/1.1 connection");
    (sender, tokio::spawn(connection))
}

#[tokio::test(flavor = "current_thread")]
async fn a_connection_that_falls_mid_request_is_a_502_and_no_reset() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let relay = Relay::start(&format!("https://{host}"), &origin.roots, 1);

    let mut asked = request(&host, "/drop", SECRET_BODY);
    asked
        .headers_mut()
        .insert("authorization", SECRET_TOKEN.parse().expect("a value"));

    let got = ask(&relay.socket, asked).await;

    assert_eq!(
        got.status,
        StatusCode::BAD_GATEWAY,
        "a request its connection lost"
    );
    assert_eq!(
        handed_to(&origin, "/drop"),
        1,
        "the origin heard a request its connection lost more than once"
    );
    let said = relay.said();
    assert!(
        said.contains("a request couldn't be carried"),
        "the failure went unsaid: {said}"
    );
    assert!(
        !said.contains("the origin reset"),
        "a connection that fell was taken for a reset: {said}"
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
async fn requests_no_connection_would_take_never_stand_it_down() {
    let origin = origin().await;
    // A port nothing listens on: bound, read and let go.
    let closed = TcpListener::bind("127.0.0.1:0").await.expect("a port");
    let address = closed.local_addr().expect("its address");
    drop(closed);
    let host = address.to_string();
    let relay = Relay::start(&format!("https://{host}"), &origin.roots, 1);

    // Never sent, each would fail direct too, so none is judged, and
    // the last is a 502 like the first rather than a stand-down's 421.
    for _ in 0..25 {
        let got = ask(&relay.socket, request(&host, "/v1/messages", b"")).await;
        assert_eq!(
            got.status,
            StatusCode::BAD_GATEWAY,
            "a request no connection would take"
        );
    }
    let said = relay.said();
    assert!(
        !said.contains("stands down"),
        "requests never sent stood it down: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn resets_saying_to_calm_down_never_stand_it_down() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let relay = Relay::start(&format!("https://{host}"), &origin.roots, 1);

    // The origin's own verdict, each would fail direct too, so none is
    // judged, and the last is a 502 like the first.
    for _ in 0..25 {
        let got = ask(&relay.socket, request(&host, "/calm-down", b"")).await;
        assert_eq!(
            got.status,
            StatusCode::BAD_GATEWAY,
            "a request the origin said to calm down for"
        );
    }
    let said = relay.said();
    assert!(
        !said.contains("stands down"),
        "resets saying to calm down stood it down: {said}"
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
