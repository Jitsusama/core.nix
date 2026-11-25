//! The relay, run as its program against an origin of the tests' own: an
//! HTTP/2 server over TLS, with a certificate made as the tests run.
//!
//! Each test asks the relay's socket as pi does, in HTTP/1.1, and reads
//! what the origin was handed: a request passes whole, only the origin is
//! reached, many requests share a few connections at once, answers stream
//! as they come, many at once in small frames whole however late pi reads
//! them, a connection the origin lets go is opened again, a
//! request the origin resets before its answer retires its connection and
//! goes once more on another, never one kept after a reset, with a burst
//! of them sharing one opened for them, and goes direct with the relay's
//! own 421 when reset again or with nowhere to go, one a goodbye left out,
//! against an origin writing its frames by hand, goes once more and then
//! direct, one a fallen connection lost is a 502 and no reset, the origin
//! hearing a lost one once, a body too large or left unfinished goes
//! nowhere, the latter as pi's cancel, never a 421, the counts are said
//! as the relay leaves, answers broken off among them, an origin out of
//! reach is said as a 502 or a 504 without queueing, a
//! connection that won't open isn't asked for again on every request, nor
//! does a failure outlast the origin coming back, the relay leaves once
//! idle but never mid-answer, even while refusing other hosts, stopped it
//! ends its answers whole and sends the rest direct, within its drain time
//! or at once at a second stop, its socket
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
/// How long it waits between the halves of `/longer`, past the ten
/// seconds a relay leaving idle takes late connections for and the five a
/// stopped relay gives those it sends direct once its answers have ended,
/// so a relay waiting only either is seen cutting one.
const LONGER_GAP: Duration = Duration::from_secs(14);
/// How long after its stop the stopped relay is asked again, past the
/// idle leave's ten seconds, and three inside `LONGER_GAP`, so a slow
/// builder still asks while the answer streams.
const ASKED_AGAIN_AFTER: Duration = Duration::from_secs(11);
/// How long after its stop a stopped relay is asked on a connection taken
/// before it: inside the half second a stop once still forwarded in, and
/// long enough for the relay to have heard its SIGTERM.
const PROBED_AFTER_THE_STOP: Duration = Duration::from_millis(250);
/// How long a relay takes to hear a stop, or to take a connection, before
/// the next step of a test leans on it having done so.
const HEARD_WITHIN: Duration = Duration::from_millis(500);
/// How long a stopped relay gives those it sends direct once its answers
/// have ended, as its own constant has it.
const SAYS_GO_DIRECT_WITHIN: Duration = Duration::from_secs(5);
/// How long the drain test lets a stopped relay wait on its answers.
const DRAINS_WITHIN: Duration = Duration::from_secs(2);
/// How soon past its drain a stopped relay has cut an answer outlasting
/// it: well inside `SAYS_GO_DIRECT_WITHIN`, which a cut left to the relay
/// leaving would take.
const CUT_WITHIN: Duration = Duration::from_secs(2);
/// How long an answer outlasting the drain surely streams before it's
/// cut: well past half of `DRAINS_WITHIN`, so a drain cut short fails, and
/// short of it by more than a stop takes to be heard.
const STREAMS_AT_LEAST: Duration = Duration::from_millis(1500);
/// What the counts say last of an answer the relay's stop cut rather than
/// pi.
const ONE_CUT_NONE_CANCELLED: &str = "pi cancelled 0; 1 cut as it was stopped;";
/// How often `/drip` sends a piece of its answer, which never ends: a
/// relay hears pi gone only as it next writes to it, so a pi leaving is
/// heard within this.
const DRIPS_EVERY: Duration = Duration::from_millis(200);
/// When the drain test connects late: long past the relay hearing its
/// stop, and inside its drain.
const TAKEN_LATE_AFTER: Duration = Duration::from_secs(1);
/// The idle time a relay that should leave idle is given.
const IDLE_EXIT_IN_TESTS: Duration = Duration::from_secs(3);
/// When a relay given `IDLE_EXIT_IN_TESTS` is surely leaving idle: past
/// its idle time, the quarter of it it looks every, and the half second
/// it gives a connection taken just then.
const LEAVING_IDLE_BY: Duration = Duration::from_secs(5);
/// How often a pi saying nothing connects to a relay leaving idle.
const SILENT_PI_EVERY: Duration = Duration::from_millis(100);
/// Words only the relay's counts say, not a connection's tally.
const COUNTS_SAID: &str = " taking requests;";
/// The exit for a relay told something it can't take.
const USAGE: i32 = 64;
/// How long the origin holds each request the sharing test sends.
const HELD: Duration = Duration::from_millis(300);

/// How long `/cut` waits after its first half before its connection falls,
/// and `/broken` before its answer breaks, so the relay has the half first.
const CUT_AFTER: Duration = Duration::from_millis(100);
/// How long the tests wait for the relay to listen.
const LISTENS_WITHIN: Duration = Duration::from_secs(10);
/// How long the relay is given to hear the origin's goodbye on a
/// connection before it's asked again, as a relay sent one moments ago is.
const GOODBYE_HEARD_WITHIN: Duration = Duration::from_millis(200);
/// What `/sized` answers, its length named.
const SIZED: &[u8] = b"whole";
/// `/small-frames`'s answer: as many pieces as a long streamed answer
/// sends, each far shorter than h2's 256-byte mark for a small DATA frame,
/// and in all more than the sockets and buffers between the relay and pi
/// hold, so a pi not reading leaves the relay unable to read the rest.
const SMALL_FRAME: &[u8] = &[b'x'; 64];
const SMALL_FRAMES: usize = 16 * 1024;
/// How many `/small-frames` answers pi leaves unread at once on the
/// relay's one connection, as a machine under load leaves them: about the
/// ninety requests sixty sessions hold, all on one connection as retired
/// connections can leave them. At the relay's windows they cost 18 MiB of
/// h2's 32 MiB allowance; a 16 MiB connection window allows 8 MiB, and a
/// 256 KiB stream window would cost 72 MiB, so either fails.
const STALLED_ANSWERS: usize = 96;
/// The relay's stream window in `/small-frames` pieces: the least the
/// origin must have sent of each answer for the relay's windows to fill.
const WINDOW_PIECES: usize = 64 * 1024 / SMALL_FRAME.len();
/// How long the origin sends nothing more before the test takes it to be
/// held back.
const SETTLES_FOR: Duration = Duration::from_secs(1);
/// How often the test reads how much the origin has sent.
const SENT_READ_EVERY: Duration = Duration::from_millis(100);
/// The longest the test waits for the windows to fill, which a connection
/// ended early never lets them do.
const FILLS_WITHIN: Duration = Duration::from_secs(30);
// The relay's four 421 sentences, each pinned whole here. pi's client
// goes direct on these exact words alone, with their one line break or
// none, and on any other 421 takes it as the origin's answer. So one
// reworded in the relay fails these tests, and if it's meant, pi's turns
// fail on it too until pi learns it.

/// What a relay asked for another origin says in its 421.
const OTHER_ORIGIN_WORDS: &[u8] = b"this relay forwards to its own origin alone\n";
/// What a relay says in its 421 to a request taken as it leaves.
const LEAVING_WORDS: &[u8] = b"this relay is leaving, so go directly\n";
/// What a relay says in its 421 of a request it couldn't carry.
const CANT_CARRY_WORDS: &[u8] = b"this relay can't carry a request to its origin, so go directly\n";
/// How long the tests let the relay take to reach an origin, in seconds.
const CONNECT_WITHIN: &str = "1";
/// As long, for a test that needs a connection still opening when a
/// drain of `DRAINS_WITHIN` ends, in seconds.
const CONNECT_OUTLASTING_THE_DRAIN: &str = "10";
/// How long a request takes to reach the relay and wait there on a
/// connection, generous for a loaded machine.
const WAITS_WITHIN: Duration = Duration::from_millis(500);
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
    /// How many pieces of `/small-frames` answers it has sent.
    sent: Arc<AtomicUsize>,
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

/// What one of the origin's connections shares with its requests: which
/// of the origin's it is, counted from 0, what the origin keeps, a way for
/// `/bye` to let the connection go, and one for `/drop` to let it fall.
#[derive(Clone)]
struct Served {
    connection: usize,
    kept: Arc<Mutex<Vec<Handed>>>,
    held: Arc<Held>,
    sent: Arc<AtomicUsize>,
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
    let sent = Arc::new(AtomicUsize::new(0));
    let (counted, finished, kept, holding, sending) = (
        Arc::clone(&connections),
        Arc::clone(&ended),
        Arc::clone(&handed),
        Arc::clone(&held),
        Arc::clone(&sent),
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
                sent: Arc::clone(&sending),
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
        sent,
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
/// with no word after its first half, `/bye` answers and lets its
/// connection go, `/reset` reads the request and resets its stream rather
/// than answer, `/reset-once` does so the first time it's asked and
/// echoes after, `/reset-on-the-first` does so on the origin's first
/// connection and echoes on any other, `/reset-but-on-the-second` does so
/// on every connection but the origin's second, `/calm-down` resets
/// saying to calm down, `/reset-then-calm-down` resets the first time and
/// says to calm down after, `/drop` reads it and lets the connection fall
/// with no word at all, `/reset-then-drop` resets the first time and
/// drops after, and `/small-frames` streams its answer in `SMALL_FRAMES`
/// small pieces. An echo says which of the origin's connections it came
/// on, counted from 0.
async fn answer(request: Request<Incoming>, served: Served) -> Result<Response<Answer>, h2::Error> {
    let Served {
        connection,
        kept,
        held,
        sent,
        bye,
        gone,
    } = served;
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
    if let Some(reason) = resets(&path, heard, connection) {
        return Err(h2::Error::from(reason));
    }
    let answer = match path.as_str() {
        "/stream" | "/long" | "/longer" | "/sized" | "/broken" | "/cut" => in_halves(&path, gone),
        "/small-frames" => in_small_frames(sent),
        "/drip" => {
            let (mut sender, body) = Channel::<Bytes, Infallible>::new(1);
            tokio::spawn(async move {
                // Until the relay drops the stream, with pi or with a cut.
                while sender
                    .send_data(Bytes::from_static(b"drip\n"))
                    .await
                    .is_ok()
                {
                    tokio::time::sleep(DRIPS_EVERY).await;
                }
            });
            Response::builder()
                .header("content-type", "text/event-stream")
                .body(boxed(body))
        }
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
/// body comes in halves, or ends or fails after its first; `/cut` lets
/// its connection fall by `gone`.
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
        "/cut" => {
            let (mut sender, body) = Channel::<Bytes, Infallible>::new(1);
            tokio::spawn(async move {
                let _ = sender.send_data(Bytes::from_static(b"first\n")).await;
                // The first half goes out before the connection falls, so
                // the answer is broken off midway rather than never begun.
                tokio::time::sleep(CUT_AFTER).await;
                gone.notify_one();
            });
            Response::builder()
                .header("content-type", "text/event-stream")
                .body(boxed(body))
        }
        _ => {
            let gap = match path {
                "/long" => LONG_GAP,
                "/longer" => LONGER_GAP,
                _ => STREAM_GAP,
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

/// `/small-frames`'s answer: `SMALL_FRAMES` pieces of `SMALL_FRAME`, each
/// sent as a DATA frame of its own as the window allows, and counted in
/// `sent` as it goes.
fn in_small_frames(sent: Arc<AtomicUsize>) -> hyper::http::Result<Response<Answer>> {
    let (mut sender, body) = Channel::<Bytes, Infallible>::new(64);
    tokio::spawn(async move {
        for _ in 0..SMALL_FRAMES {
            if sender
                .send_data(Bytes::from_static(SMALL_FRAME))
                .await
                .is_err()
            {
                return;
            }
            sent.fetch_add(1, Ordering::SeqCst);
        }
    });
    Response::builder()
        .header("content-type", "text/event-stream")
        .body(boxed(body))
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
        "/reset" => true,
        "/reset-once" | "/reset-then-calm-down" | "/reset-then-drop" => heard == 1,
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
    /// What it said before a test read on, kept for `said`.
    heard: String,
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
        let mut child = Command::new(env!("CARGO_BIN_EXE_model-relay"))
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
        // A socket left behind refuses, so only the relay's own answers. Its
        // inode can't tell the two apart, as a filesystem may give the new
        // socket the number the old one freed.
        while let Err(why) = std::os::unix::net::UnixStream::connect(&socket) {
            if started.elapsed() >= LISTENS_WITHIN {
                let state = match child.try_wait() {
                    Ok(Some(status)) => format!("it left with {status}"),
                    _ => "it's still running".to_owned(),
                };
                panic!(
                    "the relay never listened at {}: {why}, and {state}",
                    socket.display()
                );
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        Relay {
            child,
            socket,
            _folder: folder,
            heard: String::new(),
        }
    }

    /// What the relay has said up to `mark`, read as it says it, so a test
    /// can act the moment it does; all it said, should it leave first.
    fn heard_by(&mut self, mark: &str) -> &str {
        if let Some(stderr) = self.child.stderr.as_mut() {
            let mut piece = [0; 1024];
            while !self.heard.contains(mark) {
                match stderr.read(&mut piece) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => self
                        .heard
                        .push_str(&String::from_utf8_lossy(&piece[..read])),
                }
            }
        }
        &self.heard
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

    /// Asks the relay to stop as systemd does, with SIGTERM, leaving it
    /// to end as it will.
    fn stop(&self) {
        let sent = Command::new("kill")
            .arg("-TERM")
            .arg(self.child.id().to_string())
            .status()
            .expect("kill runs");
        assert!(sent.success(), "the relay couldn't be sent SIGTERM");
    }

    /// Stops the relay and gives back all it said.
    fn said(mut self) -> String {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let mut said = std::mem::take(&mut self.heard);
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
async fn answers_in_small_frames_pass_whole_while_pi_reads_late() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let relay = Relay::start(&format!("https://{host}"), &origin.roots, 1);
    let mut stalled = Vec::new();
    for _ in 0..STALLED_ANSWERS {
        stalled.push(begun(&relay.socket, request(&host, "/small-frames", b"")).await);
    }
    // Pis too busy to read, so what the origin sends waits in the relay's
    // windows; h2 charges each small frame waiting there against what it
    // allows the whole connection. Pi reads nothing until the origin has
    // sent every answer's window and then nothing more for a second.
    let sent = settled(&origin.sent).await;
    let began: Vec<StatusCode> = stalled.iter().map(Response::status).collect();
    let mut reading = tokio::task::JoinSet::new();
    for answer in stalled {
        reading.spawn(async move {
            let body = answer
                .into_body()
                .collect()
                .await
                .map(http_body_util::Collected::to_bytes)
                .unwrap_or_default();
            (body.len(), body.iter().all(|&byte| byte == SMALL_FRAME[0]))
        });
    }
    let read = reading.join_all().await;
    let after = ask(&relay.socket, request(&host, "/echo", b"after")).await;

    let said = relay.said();
    assert!(
        !said.contains("too_many_data_frames"),
        "h2 ended the connection over the frames pi hadn't read: {said}"
    );
    assert!(
        began.iter().all(|&status| status == StatusCode::OK),
        "an answer didn't begin, beginning as {began:?}; the relay said: {said}"
    );
    assert!(
        read.iter()
            .all(|&(length, same)| length == SMALL_FRAMES * SMALL_FRAME.len() && same),
        "an answer was cut short or changed, read as {read:?}; the relay said: {said}"
    );
    assert!(
        sent >= STALLED_ANSWERS * WINDOW_PIECES,
        "the origin sent {sent} pieces before pi read any, fewer than every answer's window, \
         so the relay's windows can't have filled"
    );
    assert!(
        sent < STALLED_ANSWERS * SMALL_FRAMES,
        "the origin sent every piece while pi read none, so nothing waited in the relay"
    );
    assert_eq!(
        after.headers["x-origin-connection"], "0",
        "the answers' connection was let go: {said}"
    );
}

/// What `sent` counts once it has risen past every stalled answer's window
/// and then held still for `SETTLES_FOR`, or at `FILLS_WITHIN`, which a
/// connection ended early reaches.
async fn settled(sent: &AtomicUsize) -> usize {
    let filled = STALLED_ANSWERS * WINDOW_PIECES;
    let gives_up_at = tokio::time::Instant::now() + FILLS_WITHIN;
    let mut last = sent.load(Ordering::SeqCst);
    let mut still_since = tokio::time::Instant::now();
    loop {
        // Read at intervals, since nothing says when the origin's sends end.
        tokio::time::sleep(SENT_READ_EVERY).await;
        let now = sent.load(Ordering::SeqCst);
        let at = tokio::time::Instant::now();
        if now != last {
            last = now;
            still_since = at;
        }
        if (now >= filled && at - still_since >= SETTLES_FOR) || at >= gives_up_at {
            return now;
        }
    }
}

/// The answer to `request`, asked of the relay at `socket` on a connection
/// of its own and begun, its status left for the caller to judge and its
/// body for it to read.
async fn begun<B>(socket: &Path, request: Request<B>) -> Response<Incoming>
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
    sender.send_request(request).await.expect("an answer")
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
        assert_eq!(
            (got.status, got.body.as_slice()),
            (StatusCode::MISDIRECTED_REQUEST, OTHER_ORIGIN_WORDS),
            "{what}"
        );
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
        (StatusCode::MISDIRECTED_REQUEST, CANT_CARRY_WORDS),
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
        got.body, CANT_CARRY_WORDS,
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
            "3 asked, 0 refused, 1 failed, 1 of them reset or left out; 0 reset or left out \
             and sent again; pi cancelled 0"
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
            "4 asked, 0 refused, 0 failed, 0 of them reset or left out; 4 reset or left out \
             and sent again; pi cancelled 0"
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
            "3 asked, 0 refused, 3 failed, 3 of them reset or left out; 0 reset or left out \
             and sent again; pi cancelled 0"
        ),
        "a request reset and not sent again was counted as sent: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn a_body_over_64_mib_goes_nowhere_another_origins_421_said_and_one_pi_left_its_cancel() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let mut relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &["--idle-exit", "4"],
    );

    let mine = asked_too_much(&relay.socket, &host).await;
    let handed = handed_to(&origin, "/echo");
    // For another origin, it's the 421 that sends pi direct all the same.
    let others = asked_too_much(&relay.socket, "example.com").await;
    // A body pi leaves is its cancel, though, not a 421 nobody hears.
    left_halfway(&relay.socket, "example.com").await;
    // And one for its own origin is its cancel all the same.
    left_halfway(&relay.socket, &host).await;
    let left = relay.left_within(LISTENS_WITHIN * 2);
    // Its words are read before anything is judged, since a relay that
    // answered wrongly is the one whose log says why.
    let said = relay.said();

    assert!(
        mine.as_deref()
            .is_some_and(|head| head.starts_with(b"HTTP/1.1 413")),
        "a body too large was answered {:?}, the relay saying: {said}",
        mine.as_deref().map(String::from_utf8_lossy)
    );
    assert_eq!(handed, 0, "a body too large went on: {said}");
    assert!(
        others
            .as_deref()
            .is_some_and(|head| head.starts_with(b"HTTP/1.1 421")),
        "another origin's body too large was answered {:?}, the relay saying: {said}",
        others.as_deref().map(String::from_utf8_lossy)
    );
    assert!(left.is_some(), "the relay didn't leave once idle: {said}");
    assert_eq!(
        said.matches("a request's body passed 64 MiB, so it's refused")
            .count(),
        1,
        "a body too large went unsaid, or another origin's was said as one: {said}"
    );
    assert!(
        said.contains("4 asked, 2 refused, 0 failed") && said.contains("; pi cancelled 2;"),
        "a body too large wasn't counted as refused, or one pi left as its cancel: {said}"
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
/// `host` with a body one byte over what it carries, or none when it says
/// nothing in time, left for the caller to judge beside the relay's log.
async fn asked_too_much(socket: &Path, host: &str) -> Option<Vec<u8>> {
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
    answered.ok().map(|()| answer)
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

/// Which requests a draining origin leaves out of a goodbye, counted
/// across its connections from 0.
#[derive(Clone, Copy)]
enum Drains {
    /// Every other, the first of each pair, as rotations crossing each
    /// request's first try would.
    EveryOther,
    /// Every one, as a drain before a deploy that ends every connection.
    All,
    /// Every one, with a goodbye saying to calm down rather than that
    /// nothing was wrong.
    CalmingDown,
}

/// How many requests the draining tests ask in a row, enough that a
/// handling that held for only some would show.
const ASKED_IN_A_ROW: usize = 25;

/// What a draining origin was asked, and how many it left out.
struct Draining {
    address: SocketAddr,
    asked: Arc<AtomicUsize>,
    left_out: Arc<AtomicUsize>,
}

/// An origin writing HTTP/2 by hand over `tls`, leaving out what `drains`
/// says of a goodbye saying nothing was wrong, or to calm down, and
/// answering the rest 200
/// with no body: h2's own server counts a request it has read as begun,
/// so it won't leave one out.
async fn draining_origin(drains: Drains, tls: Tls) -> Draining {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("a port");
    let address = listener.local_addr().expect("its address");
    let (asked, left_out) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
    let (counted, dropped) = (Arc::clone(&asked), Arc::clone(&left_out));
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let acceptor = tls.acceptor.clone();
            let (asked, left_out) = (Arc::clone(&counted), Arc::clone(&dropped));
            tokio::spawn(async move {
                if let Ok(mut stream) = acceptor.accept(stream).await {
                    Box::pin(drain_frames(&mut stream, drains, &asked, &left_out)).await;
                }
            });
        }
    });
    Draining {
        address,
        asked,
        left_out,
    }
}

/// Reads a connection's frames, answering them as `draining_origin` says,
/// until the client leaves.
async fn drain_frames<S>(
    stream: &mut S,
    drains: Drains,
    asked: &AtomicUsize,
    left_out: &AtomicUsize,
) where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    /// The bytes a client opens HTTP/2 with, before its first frame.
    const PREFACE: usize = 24;
    /// A frame's head: its length, type, flags and stream.
    const HEAD: usize = 9;
    const DATA: u8 = 0;
    const HEADERS: u8 = 1;
    const SETTINGS: u8 = 4;
    const PING: u8 = 6;
    const GOAWAY: u8 = 7;
    /// A frame's flag ending its stream, or acknowledging, by its type.
    const END_STREAM_OR_ACK: u8 = 1;
    const END_HEADERS: u8 = 4;
    /// `:status: 200`, from HPACK's static table.
    const OK: u8 = 0x88;

    if stream
        .write_all(&[0, 0, 0, SETTINGS, 0, 0, 0, 0, 0])
        .await
        .is_err()
    {
        return;
    }
    let (mut read, mut at, mut said_goodbye) = (Vec::new(), PREFACE, false);
    let mut piece = [0; 16_384];
    loop {
        while read.len() >= at + HEAD {
            let length = (usize::from(read[at]) << 16)
                | (usize::from(read[at + 1]) << 8)
                | usize::from(read[at + 2]);
            if read.len() < at + HEAD + length {
                break;
            }
            let (kind, flags) = (read[at + 3], read[at + 4]);
            let id = u32::from_be_bytes([read[at + 5], read[at + 6], read[at + 7], read[at + 8]])
                & 0x7fff_ffff;
            let payload = read[at + HEAD..at + HEAD + length].to_vec();
            at += HEAD + length;
            let reply = match kind {
                SETTINGS if flags & END_STREAM_OR_ACK == 0 => {
                    vec![0, 0, 0, SETTINGS, END_STREAM_OR_ACK, 0, 0, 0, 0]
                }
                PING if flags & END_STREAM_OR_ACK == 0 => {
                    [vec![0, 0, 8, PING, END_STREAM_OR_ACK, 0, 0, 0, 0], payload].concat()
                }
                // Once it has said goodbye, the client fails what it sends
                // after on its own.
                HEADERS | DATA if flags & END_STREAM_OR_ACK != 0 && !said_goodbye => {
                    let every = matches!(drains, Drains::All | Drains::CalmingDown);
                    if every || asked.fetch_add(1, Ordering::SeqCst).is_multiple_of(2) {
                        if every {
                            asked.fetch_add(1, Ordering::SeqCst);
                        }
                        left_out.fetch_add(1, Ordering::SeqCst);
                        said_goodbye = true;
                        // It names the last stream begun before this
                        // one, so it leaves this one out, saying nothing
                        // was wrong, or to calm down.
                        let last = id.saturating_sub(2).to_be_bytes();
                        let reason = if matches!(drains, Drains::CalmingDown) {
                            u32::from(h2::Reason::ENHANCE_YOUR_CALM)
                        } else {
                            u32::from(h2::Reason::NO_ERROR)
                        };
                        [
                            &[0, 0, 8, GOAWAY, 0, 0, 0, 0, 0][..],
                            &last,
                            &reason.to_be_bytes(),
                        ]
                        .concat()
                    } else {
                        let head = [0, 0, 1, HEADERS, END_HEADERS | END_STREAM_OR_ACK];
                        [&head[..], &id.to_be_bytes(), &[OK]].concat()
                    }
                }
                _ => Vec::new(),
            };
            if stream.write_all(&reply).await.is_err() {
                return;
            }
        }
        match stream.read(&mut piece).await {
            Ok(got) if got > 0 => read.extend_from_slice(&piece[..got]),
            _ => return,
        }
    }
}

/// A request a goodbye saying nothing was wrong left out goes once more,
/// on a connection opened for it, and is answered there. With one slot,
/// each goodbye's connection must be let go rather than retired, or the
/// slot's allowance for resets keeps it and the request goes direct.
#[tokio::test(flavor = "current_thread")]
async fn a_request_a_goodbye_left_out_goes_once_more_and_is_answered() {
    let tls = tls();
    let origin = draining_origin(Drains::EveryOther, tls.clone()).await;
    let host = origin.address.to_string();
    let relay = Relay::start(&format!("https://{host}"), &tls.roots, 1);
    for _ in 0..ASKED_IN_A_ROW {
        let got = ask(&relay.socket, request(&host, "/echo", b"hello")).await;
        assert_eq!(
            got.status,
            StatusCode::OK,
            "a request a goodbye left out wasn't answered once more: {}",
            String::from_utf8_lossy(&got.body)
        );
    }
    assert_eq!(
        (
            origin.asked.load(Ordering::SeqCst),
            origin.left_out.load(Ordering::SeqCst)
        ),
        (2 * ASKED_IN_A_ROW, ASKED_IN_A_ROW),
        "each request wasn't left out once and answered once"
    );
    let said = relay.said();
    assert!(
        said.contains("left a request out of its goodbye on connection 0.0, so it's let go")
            && said.contains("left out on connection 0.0 is sent again on connection 0.1"),
        "requests a goodbye left out weren't said let go and sent again: {said}"
    );
}

/// A request goodbyes saying nothing was wrong left out twice goes direct
/// with the relay's 421.
#[tokio::test(flavor = "current_thread")]
async fn a_request_left_out_again_goes_direct() {
    let tls = tls();
    let origin = draining_origin(Drains::All, tls.clone()).await;
    let host = origin.address.to_string();
    let relay = Relay::start(&format!("https://{host}"), &tls.roots, 2);
    for _ in 0..ASKED_IN_A_ROW {
        let got = ask(&relay.socket, request(&host, "/echo", b"hello")).await;
        assert_eq!(
            (got.status, got.body.as_ref()),
            (StatusCode::MISDIRECTED_REQUEST, CANT_CARRY_WORDS),
            "a request left out twice didn't go direct"
        );
    }
    assert_eq!(
        origin.left_out.load(Ordering::SeqCst),
        2 * ASKED_IN_A_ROW,
        "each request wasn't left out twice"
    );
    let said = relay.said();
    assert!(
        said.contains("; pi goes directly"),
        "requests left out twice weren't said sent direct: {said}"
    );
    // Each try is on a connection of its own, the one its slot has open,
    // so each is let go: retired, a resend's would be kept or out of use.
    assert_eq!(
        said.matches("left a request out of its goodbye on connection ")
            .count(),
        2 * ASKED_IN_A_ROW,
        "each try left out wasn't said: {said}"
    );
    assert_eq!(
        said.lines()
            .filter(|line| line.contains("out of its goodbye") && line.contains("so it's let go"))
            .count(),
        2 * ASKED_IN_A_ROW,
        "a connection a goodbye ended, first try or again, wasn't let go: {said}"
    );
}

/// A request a goodbye saying to calm down left out is the origin's
/// verdict, answered 502 and never sent again, and its connection is let
/// go.
#[tokio::test(flavor = "current_thread")]
async fn a_request_a_goodbye_saying_to_calm_down_left_out_is_answered_502() {
    let tls = tls();
    let origin = draining_origin(Drains::CalmingDown, tls.clone()).await;
    let host = origin.address.to_string();
    let relay = Relay::start(&format!("https://{host}"), &tls.roots, 1);
    for _ in 0..ASKED_IN_A_ROW {
        let got = ask(&relay.socket, request(&host, "/echo", b"hello")).await;
        assert_eq!(
            (got.status, got.body.as_ref()),
            (
                StatusCode::BAD_GATEWAY,
                b"this relay's request to its origin failed\n".as_slice()
            ),
            "a request told to calm down wasn't answered 502"
        );
    }
    assert_eq!(
        origin.left_out.load(Ordering::SeqCst),
        ASKED_IN_A_ROW,
        "a request told to calm down was sent again"
    );
    let said = relay.said();
    assert_eq!(
        said.lines()
            .filter(|line| line.contains("out of its goodbye")
                && line.contains("so it's let go")
                && line.contains("pi is answered 502"))
            .count(),
        ASKED_IN_A_ROW,
        "a goodbye saying to calm down wasn't said, its connection let go: {said}"
    );
    assert!(
        !said.contains(" is sent again on "),
        "goodbyes saying to calm down were sent again: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn answers_pi_gives_up_on_are_its_cancels() {
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
        said.contains("pi cancelled 25"),
        "answers pi gave up on weren't counted as its cancels: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn answers_the_origin_breaks_off_are_counted_broken_off() {
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
        "a request after answers the origin broke off itself"
    );
    assert!(
        relay.left_within(LISTENS_WITHIN).is_some(),
        "the relay didn't leave once idle"
    );
    let said = relay.said();
    assert!(
        said.contains("; 20 broken off midway"),
        "the counts don't say the answers broken off: {said}"
    );
}

/// An answer lost midway with its connection to the origin fails by
/// another road through h2 than one the origin breaks off on a live
/// connection, and is counted broken off all the same.
#[tokio::test(flavor = "current_thread")]
async fn answers_lost_midway_with_their_connection_are_counted_broken_off() {
    let origin = origin().await;
    let host = origin.address.to_string();
    // Its idle time short, so it leaves and says its counts.
    let mut relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &["--idle-exit", "1"],
    );

    for _ in 0..3 {
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

    assert!(
        relay.left_within(LISTENS_WITHIN).is_some(),
        "the relay didn't leave once idle"
    );
    let said = relay.said();
    assert!(
        said.contains("; 3 broken off midway"),
        "the counts don't say the answers lost with their connection as broken off: {said}"
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
            "1 asked, 0 refused, 0 failed, 0 of them reset or left out; 0 reset or left out and \
             sent again; pi cancelled 1"
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
        said.contains("connection 0.0 carried 6 asked, 2 of them cancelled and 0 cut"),
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
async fn a_relay_stopped_mid_answer_ends_it_whole_and_sends_the_rest_direct() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let mut relay = Relay::start(&format!("https://{host}"), &origin.roots, 1);
    // A connection kept open and asking nothing is closed the moment the
    // relay starts leaving, which is how the test knows it has. Connected
    // first, it's taken before the one below, so before the stop.
    let stream = UnixStream::connect(&relay.socket)
        .await
        .expect("the socket answers");
    let (mut kept_sender, kept) =
        hyper::client::conn::http1::handshake::<_, Full<Bytes>>(TokioIo::new(stream))
            .await
            .expect("an HTTP/1.1 connection");
    let kept = tokio::spawn(kept);
    let (_pi, first, body) = longer_begun(&relay, &host).await;

    // Stopped between the answer's halves, which come LONGER_GAP apart.
    relay.stop();
    let stopped = Instant::now();
    // Asked on a connection taken before the stop, as a pi that wrote just
    // after it would: nothing new may begin, so it's closed or sent direct.
    tokio::time::sleep(PROBED_AFTER_THE_STOP).await;
    let probed = answered_on(&mut kept_sender, &host).await;
    let closed = tokio::time::timeout(Duration::from_secs(2), kept).await;
    assert!(
        relay.left().is_none(),
        "the relay left at once when stopped, mid-answer"
    );
    assert!(
        closed.is_ok(),
        "the relay never closed a kept connection, so never began to leave; the relay \
         said:\n{}",
        relay.said()
    );
    drop(kept_sender);
    let got = ask(&relay.socket, request(&host, "/echo", b"")).await;
    // A late connection on which pi says nothing holds the relay only the
    // few seconds it gives those it sends direct, not until hyper gives up.
    let silent = UnixStream::connect(&relay.socket)
        .await
        .expect("the socket answers");
    // Still streaming its answer, it sends one asked later direct too.
    tokio::time::sleep_until((stopped + ASKED_AGAIN_AFTER).into()).await;
    let later = ask(&relay.socket, request(&host, "/echo", b"")).await;
    let rest = body.collect().await.expect("the rest").to_bytes();

    assert!(
        probed.is_none_or(|status| status == StatusCode::MISDIRECTED_REQUEST),
        "a request on a connection kept from before the stop was carried: {probed:?}"
    );
    assert_eq!(
        &first[..],
        b"first\n",
        "the answer's first half didn't pass"
    );
    assert_eq!(
        &rest[..],
        b"second\n",
        "an answer in hand when the relay was stopped didn't end whole"
    );
    assert_eq!(
        (got.status, got.body.as_slice()),
        (StatusCode::MISDIRECTED_REQUEST, LEAVING_WORDS),
        "a request taken after the relay was stopped wasn't sent direct in the words pi goes \
         direct on"
    );
    assert_eq!(
        (later.status, later.body.as_slice()),
        (StatusCode::MISDIRECTED_REQUEST, LEAVING_WORDS),
        "a request taken while the stopped relay's answer still streamed wasn't sent direct"
    );
    assert_eq!(
        origin.handed.lock().expect("the record").len(),
        1,
        "a request taken after the relay was stopped reached the origin"
    );
    let left = relay.left_within(SAYS_GO_DIRECT_WITHIN + Duration::from_secs(3));
    assert!(
        left.is_some_and(|status| status.success()),
        "the relay didn't leave cleanly once its answer ended: {left:?}"
    );
    let said = relay.said();
    drop(silent);
    // The kept one and the one streaming, both taken before the stop.
    assert!(
        said.contains("stopped with 2 connections taken"),
        "it didn't say what it waited on when stopped: {said}"
    );
    assert!(
        said.contains("the connections it had taken ended within"),
        "it didn't say its answers ended: {said}"
    );
    assert!(
        said.contains("1 connections hadn't finished within 5 s, so they're cut"),
        "it didn't say it cut the connection pi said nothing on, alone: {said}"
    );
    // The requests it sent direct moved its counts, so it says them again.
    assert!(
        said.split_once("stopped with")
            .is_some_and(|(_, after)| after.contains(COUNTS_SAID)),
        "it didn't say its counts as it left: {said}"
    );
}

/// What `/echo` is answered on `sender`'s connection, if it's answered.
async fn answered_on(
    sender: &mut hyper::client::conn::http1::SendRequest<Full<Bytes>>,
    host: &str,
) -> Option<StatusCode> {
    sender.ready().await.ok()?;
    sender
        .send_request(request(host, "/echo", b""))
        .await
        .ok()
        .map(|answer| answer.status())
}

/// A `/longer` answer begun on a connection of its own: the connection's
/// sender, kept so pi is still there for the rest, its first half, and the
/// body still to come.
async fn longer_begun(
    relay: &Relay,
    host: &str,
) -> (
    hyper::client::conn::http1::SendRequest<Full<Bytes>>,
    Bytes,
    Incoming,
) {
    let (mut sender, connection) = pi_asks(&relay.socket).await;
    let response = sender
        .send_request(request(host, "/longer", b""))
        .await
        .expect("an answer");
    // The connection outlives this function, so its task is left running.
    drop(connection);
    let mut body = response.into_body();
    loop {
        let frame = body.frame().await.expect("a frame").expect("a good frame");
        if let Ok(first) = frame.into_data() {
            break (sender, first, body);
        }
    }
}

#[tokio::test(flavor = "current_thread")]
async fn a_stopped_relay_cuts_an_answer_longer_than_its_drain() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let drain = DRAINS_WITHIN.as_secs().to_string();
    let mut relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &["--drain-within", &drain],
    );
    let (_pi, _, body) = longer_begun(&relay, &host).await;

    relay.stop();
    let stopped = Instant::now();
    // Sent direct and saying nothing, it holds the relay its few seconds
    // past the drain, so a cut answer can't be one cut as the relay left.
    tokio::time::sleep(TAKEN_LATE_AFTER).await;
    let silent = UnixStream::connect(&relay.socket)
        .await
        .expect("the socket answers");
    let rest = body
        .collect()
        .await
        .map(http_body_util::Collected::to_bytes);
    let cut_after = stopped.elapsed();

    let whole = matches!(&rest, Ok(rest) if &rest[..] == b"second\n");
    assert!(!whole, "an answer outlasting the drain came whole");
    assert!(
        cut_after < DRAINS_WITHIN + CUT_WITHIN,
        "an answer outlasting the drain was cut only {cut_after:?} after the stop"
    );
    assert!(
        cut_after >= STREAMS_AT_LEAST,
        "an answer was cut {cut_after:?} after the stop, before its drain ran out"
    );
    let left = relay.left_within(DRAINS_WITHIN + SAYS_GO_DIRECT_WITHIN + Duration::from_secs(3));
    assert!(
        left.is_some_and(|status| status.success()),
        "the relay didn't leave on its own once its drain ran out: {left:?}"
    );
    let said = relay.said();
    drop(silent);
    assert!(
        said.contains(&format!(
            "1 connections it had taken hadn't ended within {drain} s, so they're cut"
        )),
        "it didn't say it cut the answer it held: {said}"
    );
    assert!(
        said.contains("1 connections hadn't finished within 5 s, so they're cut"),
        "it didn't count the connection it sent direct alone as late: {said}"
    );
    assert!(
        said.contains(ONE_CUT_NONE_CANCELLED),
        "its counts didn't say it cut the answer, rather than pi: {said}"
    );
    assert!(
        said.contains("connection 0.0 carried 1 asked, 0 of them cancelled and 1 cut"),
        "its connection didn't count the answer as cut, rather than pi's: {said}"
    );
    // Counts said before its runtime drops what it cut can say none was cut,
    // or not, as the race goes, so it's the one line said after that's pinned.
    assert!(
        said_once_after(&said, "so they're cut"),
        "it didn't say its counts once after the cut, saying the cut: {said}"
    );
}

/// Whether `said` holds one counts line after `mark`, saying one answer cut.
fn said_once_after(said: &str, mark: &str) -> bool {
    said.split_once(mark).is_some_and(|(_, after)| {
        after.matches(COUNTS_SAID).count() == 1 && after.contains(ONE_CUT_NONE_CANCELLED)
    })
}

#[tokio::test(flavor = "current_thread")]
async fn a_request_waiting_on_its_first_connection_as_a_drain_ends_is_cut() {
    let origin = origin().await;
    // A port that takes connections and never says a word of TLS, so the
    // relay's first connection is still opening when the drain ends.
    let silent = TcpListener::bind("127.0.0.1:0").await.expect("a port");
    let host = silent.local_addr().expect("its address").to_string();
    tokio::spawn(async move {
        let mut held = Vec::new();
        while let Ok((stream, _)) = silent.accept().await {
            held.push(stream);
        }
    });
    let drain = DRAINS_WITHIN.as_secs().to_string();
    let mut relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &[
            "--connect-within",
            CONNECT_OUTLASTING_THE_DRAIN,
            "--drain-within",
            &drain,
        ],
    );
    let (mut pi, _connection) = pi_asks(&relay.socket).await;

    let asked = pi.send_request(request(&host, "/v1/messages", b""));
    let (answered, late) = tokio::join!(asked, async {
        tokio::time::sleep(WAITS_WITHIN).await;
        relay.stop();
        tokio::time::sleep(HEARD_WITHIN).await;
        // One that asks during the drain moves the counts, so counts said
        // before the cut was dropped would say none was, rather than
        // being the same line said twice and so said once.
        ask(&relay.socket, request(&host, "/v1/messages", b"")).await
    });

    assert_eq!(
        late.status,
        StatusCode::MISDIRECTED_REQUEST,
        "a request during the drain wasn't sent direct"
    );
    assert!(
        answered.is_err(),
        "a request still waiting on a connection when its drain ended was answered: \
         {answered:?}"
    );
    let left = relay.left_within(SAYS_GO_DIRECT_WITHIN + Duration::from_secs(3));
    assert!(
        left.is_some_and(|status| status.success()),
        "the relay didn't leave on its own once its drain ran out: {left:?}"
    );
    let said = relay.said();
    assert!(
        said.contains("1 connections it had taken hadn't ended within"),
        "the drain didn't end on the request waiting: {said}"
    );
    assert!(
        said.contains("2 asked, 1 refused, 0 failed") && said.contains(ONE_CUT_NONE_CANCELLED),
        "the request it cut as it waited on a connection wasn't counted cut: {said}"
    );
    assert!(
        said_once_after(&said, "so they're cut"),
        "it didn't say its counts once after the cut, saying the cut: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn a_request_pi_leaves_while_its_first_connection_opens_is_its_cancel() {
    let origin = origin().await;
    // A port that takes connections and never says a word of TLS, so the
    // relay's first connection is still opening when pi leaves.
    let silent = TcpListener::bind("127.0.0.1:0").await.expect("a port");
    let host = silent.local_addr().expect("its address").to_string();
    tokio::spawn(async move {
        let mut held = Vec::new();
        while let Ok((stream, _)) = silent.accept().await {
            held.push(stream);
        }
    });
    let mut relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &[
            "--connect-within",
            CONNECT_OUTLASTING_THE_DRAIN,
            "--idle-exit",
            "1",
        ],
    );
    let (mut pi, connection) = pi_asks(&relay.socket).await;
    let asked = pi.send_request(request(&host, "/v1/messages", b""));
    let _ = tokio::time::timeout(WAITS_WITHIN, asked).await;
    // pi leaves as its person pressing Escape does, and its socket closes
    // once the aborted task has been dropped.
    drop(pi);
    connection.abort();
    let _ = connection.await;

    let left = relay.left_within(LISTENS_WITHIN);
    let said = relay.said();
    assert!(
        left.is_some_and(|status| status.success()),
        "the relay didn't leave once idle after pi left: {left:?}, saying: {said}"
    );
    assert!(
        said.contains("1 asked, 0 refused, 0 failed")
            && said.contains("; pi cancelled 1; 0 cut as it was stopped;"),
        "a request pi left while its connection opened wasn't its cancel: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn an_answer_pi_leaves_during_a_drain_is_its_cancel_and_not_a_cut() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let drain = DRAINS_WITHIN.as_secs().to_string();
    let mut relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &["--drain-within", &drain],
    );
    let (_pi, _, _body) = longer_begun(&relay, &host).await;
    let (mut leaver, connection) = pi_asks(&relay.socket).await;
    // One that keeps writing, so the relay hears pi leave it at once.
    let left_alone = leaver
        .send_request(request(&host, "/drip", b""))
        .await
        .expect("an answer begun");

    relay.stop();
    // Well inside the drain, so the relay is still waiting on both answers
    // when pi leaves one, as its person pressing Escape does, and hears it
    // a drip later.
    tokio::time::sleep(DRAINS_WITHIN / 4).await;
    drop(left_alone);
    drop(leaver);
    connection.abort();
    // Its socket closes only once the aborted task has been dropped.
    let _ = connection.await;
    // The origin drips from this process, so the drain is waited out here,
    // where it can, rather than in the blocking wait for the relay to
    // leave.
    tokio::time::sleep(DRAINS_WITHIN).await;

    let left = relay.left_within(DRAINS_WITHIN + SAYS_GO_DIRECT_WITHIN + Duration::from_secs(3));
    assert!(
        left.is_some_and(|status| status.success()),
        "the relay didn't leave on its own once its drain ran out: {left:?}"
    );
    let said = relay.said();
    assert!(
        said.contains("pi cancelled 1; 1 cut as it was stopped;"),
        "an answer pi left during the drain wasn't its cancel, or the one the drain \
         ended wasn't cut: {said}"
    );
    assert!(
        said.contains("connection 0.0 carried 2 asked, 1 of them cancelled and 1 cut"),
        "its connection didn't tell pi's cancel from the cut: {said}"
    );
}

/// A pi that has said a request for `host` and only part of its body,
/// held open so the relay is still reading it.
async fn half_a_body(socket: &Path, host: &str) -> UnixStream {
    let mut stream = UnixStream::connect(socket)
        .await
        .expect("the socket answers");
    let head =
        format!("POST /v1/messages HTTP/1.1\r\nhost: {host}\r\ncontent-length: 10\r\n\r\nhi");
    stream
        .write_all(head.as_bytes())
        .await
        .expect("half a body written");
    stream
}

#[tokio::test(flavor = "current_thread")]
async fn a_request_whose_body_a_drain_cuts_midway_is_cut() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let drain = DRAINS_WITHIN.as_secs().to_string();
    let mut relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &["--drain-within", &drain],
    );
    let _pi = half_a_body(&relay.socket, &host).await;
    tokio::time::sleep(WAITS_WITHIN).await;

    relay.stop();
    tokio::time::sleep(HEARD_WITHIN).await;
    // One that asks during the drain moves the counts, so counts said
    // before the cut was dropped would say none was.
    let late = ask(&relay.socket, request(&host, "/v1/messages", b"")).await;
    assert_eq!(
        late.status,
        StatusCode::MISDIRECTED_REQUEST,
        "a request during the drain wasn't sent direct"
    );

    let left = relay.left_within(DRAINS_WITHIN + SAYS_GO_DIRECT_WITHIN + Duration::from_secs(3));
    let said = relay.said();
    assert!(
        left.is_some_and(|status| status.success()),
        "the relay didn't leave on its own once its drain ran out: {left:?}, saying: {said}"
    );
    assert!(
        said.contains("1 connections it had taken hadn't ended within"),
        "the drain didn't wait on the body still being read: {said}"
    );
    assert!(
        said.contains("2 asked, 1 refused, 0 failed") && said_once_after(&said, "so they're cut"),
        "a request the drain cut as its body was read wasn't counted cut, once: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn a_stop_as_it_leaves_idle_cuts_a_body_still_being_read() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let mut relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &["--idle-exit", "2"],
    );
    // Another origin's request holds no idle leave back, even while its
    // body is read, so the relay leaves idle with it in hand.
    let _pi = half_a_body(&relay.socket, "example.com").await;
    let heard = relay.heard_by("so it leaves; its socket starts it again");
    assert!(
        heard.contains("so it leaves; its socket starts it again"),
        "the relay didn't leave idle: {heard}"
    );

    relay.stop();
    let left = relay.left_within(SAYS_GO_DIRECT_WITHIN + Duration::from_secs(3));
    let said = relay.said();
    assert!(
        left.is_some_and(|status| status.success()),
        "the relay didn't leave once stopped as it left: {left:?}, saying: {said}"
    );
    assert!(
        said_once_after(&said, "stopped as it left"),
        "a body it was reading when stopped as it left wasn't counted cut, once: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn a_relay_stopped_again_leaves_at_once() {
    let origin = origin().await;
    let host = origin.address.to_string();
    let mut relay = Relay::start(&format!("https://{host}"), &origin.roots, 1);
    let (_pi, _, _body) = longer_begun(&relay, &host).await;

    relay.stop();
    // Apart, so the relay has heard the first and begun its drain before
    // the second comes, rather than taking both as one. A pi connecting
    // between them, saying nothing, is one it's sending direct.
    tokio::time::sleep(HEARD_WITHIN).await;
    let _late = UnixStream::connect(&relay.socket)
        .await
        .expect("the socket answers");
    // One that asks between them moves the counts, so counts said before
    // the cut was dropped would say none was; its connection closes once
    // answered, so it's not among those cut.
    let asked = ask(&relay.socket, request(&host, "/v1/messages", b"")).await;
    assert_eq!(
        asked.status,
        StatusCode::MISDIRECTED_REQUEST,
        "a request between the stops wasn't sent direct"
    );
    tokio::time::sleep(HEARD_WITHIN).await;
    relay.stop();

    let left = relay.left_within(Duration::from_secs(3));
    assert!(
        left.is_some_and(|status| status.success()),
        "the relay didn't leave at once when stopped again: {left:?}"
    );
    let said = relay.said();
    assert!(
        said.contains(
            "stopped again, so it leaves now, cutting the 1 connections it had taken and the 1 \
             it was sending direct"
        ),
        "it didn't say why it left, and what it cut: {said}"
    );
    assert!(
        said.contains(COUNTS_SAID),
        "it didn't say its counts as it was stopped: {said}"
    );
    // Said once its runtime has dropped the answer, the only time it can.
    assert!(
        said.contains(ONE_CUT_NONE_CANCELLED),
        "its counts didn't say it cut the answer, rather than pi: {said}"
    );
    assert!(
        said_once_after(&said, "stopped again"),
        "it didn't say its counts once after the second stop, saying the cut: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn a_relay_stopped_as_it_leaves_idle_ends_what_it_sends_direct() {
    let origin = origin().await;
    let (mut relay, silent) = leaving_idle_held_by_a_silent_pi(&origin).await;

    relay.stop();

    let left = relay.left_within(SAYS_GO_DIRECT_WITHIN + Duration::from_secs(3));
    assert!(
        left.is_some_and(|status| status.success()),
        "a relay stopped as it left idle waited on a pi saying nothing: {left:?}"
    );
    let said = relay.said();
    drop(silent);
    assert!(
        said.contains("stopped as it left, so it ends what it was sending direct"),
        "it wasn't stopped as it left idle, or didn't say so: {said}"
    );
    assert!(
        said.contains("connections hadn't finished within 5 s, so they're cut"),
        "it didn't say it cut the connections pi said nothing on: {said}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn a_relay_stopped_twice_as_it_leaves_idle_leaves_at_once() {
    let origin = origin().await;
    let (mut relay, silent) = leaving_idle_held_by_a_silent_pi(&origin).await;

    relay.stop();
    // Apart, so the relay has begun giving the silent pi its few seconds
    // before the second comes, rather than taking both as one.
    tokio::time::sleep(Duration::from_secs(1)).await;
    relay.stop();

    let left = relay.left_within(Duration::from_secs(2));
    assert!(
        left.is_some_and(|status| status.success()),
        "a relay stopped again as it left idle didn't leave at once: {left:?}"
    );
    let said = relay.said();
    drop(silent);
    assert!(
        said.contains("stopped as it left"),
        "it wasn't stopped as it left idle: {said}"
    );
    assert!(
        said.contains("stopped again, so it cuts the ") && !said.contains("hadn't finished within"),
        "it didn't say the second stop cut what it was sending direct: {said}"
    );
}

/// A relay that leaves once idle for `IDLE_EXIT_IN_TESTS`, and the
/// connections pi made to it every `SILENT_PI_EVERY` meanwhile, saying
/// nothing on any. Those taken before it began to leave are closed then;
/// those taken after are sent direct and hold it as it leaves, until
/// hyper's own timeout for a request would end them.
async fn leaving_idle_held_by_a_silent_pi(origin: &Origin) -> (Relay, Vec<UnixStream>) {
    let host = origin.address.to_string();
    let idle = IDLE_EXIT_IN_TESTS.as_secs().to_string();
    let relay = Relay::start_with(
        &format!("https://{host}"),
        &origin.roots,
        1,
        &["--idle-exit", &idle],
    );
    let until = Instant::now() + LEAVING_IDLE_BY;
    let mut silent = Vec::new();
    while Instant::now() < until {
        // A relay already gone is told by what it said, not here.
        if let Ok(stream) = UnixStream::connect(&relay.socket).await {
            silent.push(stream);
        }
        tokio::time::sleep(SILENT_PI_EVERY).await;
    }
    (relay, silent)
}

#[test]
fn flags_it_no_longer_takes_and_a_drain_of_none_are_refused_as_usage() {
    let folder = tempfile::tempdir().expect("a folder for the socket");
    let socket = folder.path().join("agents.sock");
    for (flag, why) in [
        (
            ["--stand-down-for", "10"],
            "no flag called --stand-down-for",
        ),
        (
            ["--try-path-every", "30"],
            "no flag called --try-path-every",
        ),
        (
            ["--drain-within", "0"],
            "--drain-within takes at least a second",
        ),
    ] {
        // Taken, the relay would leave idle within a second, not as usage.
        let ran = Command::new(env!("CARGO_BIN_EXE_model-relay"))
            .arg("--socket")
            .arg(&socket)
            .args(["--origin", "https://models.example", "--idle-exit", "1"])
            .args(flag)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .expect("a relay runs");
        let said = said_of(&ran);
        assert_eq!(
            ran.status.code(),
            Some(USAGE),
            "{flag:?} wasn't refused as usage: {said}"
        );
        // Any usage error exits so, so it's the words that say this one.
        assert!(
            said.contains(why),
            "{flag:?} was refused for something else: {said}"
        );
    }
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
        (got.status, got.body.as_slice()),
        (StatusCode::MISDIRECTED_REQUEST, CANT_CARRY_WORDS),
        "a fault of the relay's own wasn't sent direct in the words pi goes direct on"
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
async fn a_request_taken_as_it_leaves_is_told_its_leaving_in_the_words_pi_goes_direct_on() {
    let origin = origin().await;
    let host = origin.address.to_string();
    // Idle for 4 s, it takes connections 4 s more as it leaves, which is
    // the window the request below has to arrive in on a busy builder.
    let mut relay = Relay::start_with(
        &format!("https://{}", origin.address),
        &origin.roots,
        1,
        &["--idle-exit", "4"],
    );
    // Another host's request isn't the relay's work, so the relay goes
    // idle and leaves while its body is still coming. Its connection is
    // held until it's answered, so the relay goes on taking connections
    // as it leaves rather than leaving the moment the kept one closes.
    let (mut halves, body) = Channel::<Bytes, Infallible>::new(1);
    let socket = relay.socket.clone();
    let elsewhere = tokio::spawn(async move {
        let asked = Request::builder()
            .method("POST")
            .uri("/echo")
            .header("host", "elsewhere.example")
            .body(body)
            .expect("a request");
        ask(&socket, asked).await
    });
    halves
        .send_data(Bytes::from_static(b"the first half, "))
        .await
        .expect("the first half sent");
    // A connection kept open and asking nothing is closed the moment the
    // relay starts leaving, which is how the test knows it has.
    let stream = UnixStream::connect(&relay.socket)
        .await
        .expect("the socket answers");
    let (kept_sender, connection) =
        hyper::client::conn::http1::handshake::<_, Full<Bytes>>(TokioIo::new(stream))
            .await
            .expect("an HTTP/1.1 connection");
    let closed = tokio::time::timeout(Duration::from_secs(15), connection).await;
    assert!(
        closed.is_ok(),
        "the relay never closed a kept connection, so never left; the relay said:\n{}",
        relay.said()
    );
    drop(kept_sender);

    let Ok(got) = tokio::time::timeout(
        Duration::from_secs(4),
        ask(&relay.socket, request(&host, "/echo", b"")),
    )
    .await
    else {
        panic!(
            "a request taken as the relay left went unanswered within its 4 s of taking \
             connections; the relay said:\n{}",
            relay.said()
        );
    };
    // Its body ended, the other host's request gets its own answer.
    drop(halves);
    let refused = tokio::time::timeout(Duration::from_secs(5), elsewhere)
        .await
        .expect("another host's request went unanswered as the relay left")
        .expect("the other host's request ran");

    assert_eq!(
        (got.status, got.body.as_slice()),
        (StatusCode::MISDIRECTED_REQUEST, LEAVING_WORDS),
        "a request taken as the relay left wasn't sent direct in the words pi goes direct on"
    );
    assert_eq!(
        (refused.status, refused.body.as_slice()),
        (StatusCode::MISDIRECTED_REQUEST, OTHER_ORIGIN_WORDS),
        "another host's request, finished as the relay left, wasn't refused as another origin's"
    );
    assert_eq!(
        origin.handed.lock().expect("the record").len(),
        0,
        "a request taken as the relay left reached the origin"
    );
    let left = relay.left_within(Duration::from_secs(5));
    assert!(
        left.is_some_and(|status| status.success()),
        "the relay didn't leave cleanly after the request it refused: {left:?}; the relay \
         said:\n{}",
        relay.said()
    );
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
