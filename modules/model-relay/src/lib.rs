//! The relay pi's model requests share, so a machine's many sessions reach
//! their model's origin over a few long-lived connections.
//!
//! Each pi sends its model requests over HTTP/1.1, one connection for each
//! request in flight, so sixty sessions uploading at once hold some ninety
//! connections to one origin. On an uplink of about ten megabits, ninety
//! flows each hold about one segment a round trip, and any fair queue on
//! the way drops about one segment in five: measured on 16 November, a
//! fifth to a quarter of what the agents sent went twice. A few connections
//! carrying every request as HTTP/2's streams hold windows that a queue can
//! shape without dropping, so the same bytes cross once.
//!
//! So the relay answers pi on a socket only its account can reach, in
//! HTTP/1.1, and forwards each request to its one origin over at most a
//! few HTTP/2 connections it keeps open, opening each as it's first needed
//! and again whenever the origin lets it go. What it forwards is the
//! request as pi made it: its method, path and query, every header but
//! the ones that belong to the connection (HTTP/2 forbids them), and its
//! body byte for byte, compressed or not. It holds no credential and adds
//! none: each request's own authorization goes as it came. A request
//! naming any other host is refused with a 421 and goes nowhere.
//!
//! A request never waits on a connection being opened while another is
//! open: it takes the open one, and the missing one is opened behind it.
//! Only when none is open does a request wait, and then on one attempt per
//! connection, so when the origin can't be reached every request waiting
//! hears so as that attempt fails, rather than each making its own in turn.
//! A connection that fails to open behind others isn't tried again for a
//! while, so an origin that takes fewer connections isn't asked on every
//! request. A request hyper hands back unsent, as when its connection was
//! retired before it went, goes again on another; one the origin dropped
//! after it went is said as a 502, for pi's retry. After the machine
//! sleeps, the connections it held are retired and opened afresh, since
//! the network they ran on may be gone and pings would take their time to
//! say so.
//!
//! It says on stderr only what a reader may keep: each connection opened
//! and retired, why one couldn't be made, and counts of what it served.
//! Nothing of a request's or an answer's headers or body is said, a failure
//! included, and nothing is written to disk. A request it couldn't carry is
//! answered as a 502, or a 504 when the origin couldn't be reached in time,
//! never by closing the socket, so pi's own retry decides what follows.
//!
//! A switch leaves a running relay be, since a restart would end every
//! answer still streaming. So it leaves on its own once nothing has been
//! forwarded for a while, with no answer in flight: it stops taking
//! connections, gives one it took just then a moment to say its request,
//! then closes each as its answer ends. systemd keeps the socket, so
//! whoever asks next is answered by whatever build and origin the machine
//! now names. A request refused as another host's isn't counted, so a
//! relay left naming an origin the machine no longer does still leaves.

use std::convert::Infallible;
use std::io;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex as Held, PoisonError};
use std::time::{Duration, Instant, SystemTime};

use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::client::conn::http2::SendRequest;
use hyper::header::{self, HeaderMap, HeaderName};
use hyper::http::uri::{Authority, PathAndQuery, Scheme};
use hyper::{Request, Response, StatusCode, Uri, Version};
use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use tokio::net::{TcpStream, UnixListener};
use tokio::sync::{Mutex, Notify, watch};
use tokio_rustls::TlsConnector;

/// The port an https origin names when it names none.
const HTTPS_PORT: u16 = 443;
/// How long a connection to the origin may take to open, TLS included,
/// unless the relay is told otherwise.
pub const CONNECT_WITHIN: Duration = Duration::from_secs(10);
/// How long the relay waits, asked nothing and answering nothing, before
/// it leaves, unless told otherwise.
pub const IDLE_EXIT: Duration = Duration::from_secs(120);
/// How often an open connection is pinged, so one the network lost is
/// found before a request is sent down it.
const PING_EVERY: Duration = Duration::from_secs(20);
/// How long a ping may wait for its answer before the connection is closed.
const PING_ANSWERED_WITHIN: Duration = Duration::from_secs(20);
/// How often the counts are said, when anything was asked since.
const COUNTS_EVERY: Duration = Duration::from_secs(600);
/// How long it waits before accepting again after an accept fails, so a
/// machine out of descriptors isn't spun on.
const ACCEPT_AGAIN_AFTER: Duration = Duration::from_millis(100);
/// The most times a request hyper handed back unsent goes again.
const SENT_AGAIN_AT_MOST: usize = 2;
/// How long after an attempt to open a connection failed before one is
/// opened behind others again.
const OPEN_BEHIND_AGAIN_AFTER: Duration = Duration::from_secs(30);
/// How long a connection taken just as the relay leaves has to say its
/// request, which pi sends as it connects.
const LEAVING_GRACE: Duration = Duration::from_millis(500);
/// How far the wall clock may run ahead of the relay's own between two
/// requests before the machine is taken to have slept.
const SLEPT_AT_LEAST: Duration = Duration::from_secs(10);

/// What a relay answers with: the origin's body, or a line of its own.
type Body = BoxBody<Bytes, hyper::Error>;

/// The one https origin a relay forwards to.
#[derive(Clone, Debug)]
pub struct Origin {
    authority: Authority,
    host: String,
    port: u16,
}

impl Origin {
    /// Reads an origin, an https URL naming a host and maybe a port alone.
    ///
    /// # Errors
    ///
    /// When it isn't a URL, isn't https, or carries credentials, a path or
    /// a query, none of which an origin has.
    pub fn parse(text: &str) -> Result<Origin, String> {
        let uri: Uri = text.parse().map_err(|_| format!("{text} isn't a URL"))?;
        if uri.scheme() != Some(&Scheme::HTTPS) {
            return Err("it forwards over https alone".to_owned());
        }
        let authority = uri.authority().ok_or("it names no host")?.clone();
        if authority.as_str().contains('@') {
            return Err("an origin carries no credentials".to_owned());
        }
        if uri
            .path_and_query()
            .is_some_and(|asked| asked.as_str() != "/")
        {
            return Err("an origin is a host alone, with no path or query".to_owned());
        }
        let host = authority
            .host()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_owned();
        let port = port_of(&authority).ok_or("its port isn't one")?;
        Ok(Origin {
            authority,
            host,
            port,
        })
    }

    /// Whether a request naming `asked` as its host is asking this origin.
    fn is_named_by(&self, asked: &str) -> bool {
        asked.parse::<Authority>().is_ok_and(|named| {
            named.host().eq_ignore_ascii_case(self.authority.host())
                && port_of(&named) == Some(self.port)
        })
    }
}

/// The port `authority` names, or https's when it names none; nothing when
/// what follows its host isn't a port, as `:84433` isn't, which the http
/// crate reads as no port at all.
fn port_of(authority: &Authority) -> Option<u16> {
    let text = authority.as_str();
    let host = authority.host();
    let after = text.rfind(host).map_or("", |at| &text[at + host.len()..]);
    if after.is_empty() {
        return Some(HTTPS_PORT);
    }
    after.strip_prefix(':')?.parse().ok()
}

/// The roots a relay trusts the origin's certificate to chain to.
pub struct Roots(rustls::RootCertStore);

impl Roots {
    /// The public roots, as Mozilla's programme keeps them.
    #[must_use]
    pub fn public() -> Roots {
        Roots(rustls::RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        })
    }

    /// Only the roots a PEM file holds.
    ///
    /// # Errors
    ///
    /// When the file can't be read, or holds no certificate a root can be.
    pub fn from_pem(file: &Path) -> Result<Roots, String> {
        let mut store = rustls::RootCertStore::empty();
        for certificate in CertificateDer::pem_file_iter(file).map_err(|why| why.to_string())? {
            store
                .add(certificate.map_err(|why| why.to_string())?)
                .map_err(|why| why.to_string())?;
        }
        if store.is_empty() {
            return Err(format!("{} holds no certificate", file.display()));
        }
        Ok(Roots(store))
    }
}

/// How long a relay gives things: a connection to open, and itself to
/// sit idle before it leaves (never, when none).
#[derive(Clone, Copy, Debug)]
pub struct Clocks {
    /// How long a connection to the origin may take to open.
    pub connect_within: Duration,
    /// How long it sits asked nothing before it leaves, if ever.
    pub idle_exit: Option<Duration>,
}

impl Default for Clocks {
    fn default() -> Clocks {
        Clocks {
            connect_within: CONNECT_WITHIN,
            idle_exit: Some(IDLE_EXIT),
        }
    }
}

/// What a relay has done, as counts alone.
#[derive(Default)]
struct Counts {
    asked: AtomicU64,
    refused: AtomicU64,
    failed: AtomicU64,
    opened: AtomicU64,
    open: AtomicUsize,
}

/// What a relay is doing now: the requests it has in hand, their answers
/// included, and when it last had none.
struct Busy {
    now: AtomicUsize,
    quiet_since: Held<Instant>,
}

impl Busy {
    /// One request more in hand, until what's returned is dropped.
    fn one(self: &Arc<Busy>) -> InHand {
        self.now.fetch_add(1, Ordering::SeqCst);
        InHand(Arc::clone(self))
    }

    /// How long it has had nothing in hand, or none while it has.
    fn quiet_for(&self) -> Option<Duration> {
        let since = *self
            .quiet_since
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        (self.now.load(Ordering::SeqCst) == 0).then(|| since.elapsed())
    }
}

/// A request in hand, from when it's asked until its answer is sent or
/// given up on.
struct InHand(Arc<Busy>);

impl Drop for InHand {
    fn drop(&mut self) {
        if self.0.now.fetch_sub(1, Ordering::SeqCst) == 1 {
            *self
                .0
                .quiet_since
                .lock()
                .unwrap_or_else(PoisonError::into_inner) = Instant::now();
        }
    }
}

/// Why a connection to the origin couldn't be had.
#[derive(Clone, Copy)]
enum Unreachable {
    /// It wasn't made in the time it had.
    Late,
    /// It was refused, or failed on the way.
    Refused,
    /// It was reached, but not as this relay can carry a request to it:
    /// its certificate isn't one the relay trusts, or it doesn't speak
    /// HTTP/2. pi reaches it directly instead, as it would with no relay.
    Unfit,
}

impl Unreachable {
    /// What pi is answered: a 504 or a 502 for its retry, or a 421 that
    /// sends it direct.
    fn said(self) -> Response<Body> {
        match self {
            Unreachable::Late => said(
                StatusCode::GATEWAY_TIMEOUT,
                "this relay couldn't reach its origin in time\n",
            ),
            Unreachable::Refused => said(
                StatusCode::BAD_GATEWAY,
                "this relay couldn't reach its origin\n",
            ),
            Unreachable::Unfit => said(
                StatusCode::MISDIRECTED_REQUEST,
                "this relay can't carry a request to its origin, so go directly\n",
            ),
        }
    }
}

/// How many of pi's connections a relay holds, and a way to hear when it
/// holds none.
#[derive(Default)]
struct Connections {
    now: AtomicUsize,
    none: Notify,
}

impl Connections {
    /// One more connection, held until what it gives back is dropped.
    fn one(self: &Arc<Connections>) -> Holding {
        self.now.fetch_add(1, Ordering::SeqCst);
        Holding(Arc::clone(self))
    }

    /// Ends once no connection is held.
    async fn none_held(&self) {
        loop {
            let none = self.none.notified();
            tokio::pin!(none);
            none.as_mut().enable();
            if self.now.load(Ordering::SeqCst) == 0 {
                return;
            }
            none.await;
        }
    }
}

/// A connection a relay holds.
struct Holding(Arc<Connections>);

impl Drop for Holding {
    fn drop(&mut self) {
        if self.0.now.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.0.none.notify_waiters();
        }
    }
}

/// One of the connections a relay keeps: the sender of the one open, if
/// it is, and the lock whoever opens it holds, so it's opened once.
struct Slot {
    open: Held<Option<SendRequest<Incoming>>>,
    opening: Mutex<()>,
}

impl Slot {
    /// A sender down this connection, while it takes requests.
    fn sender(&self) -> Option<SendRequest<Incoming>> {
        self.open
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .filter(|sender| !sender.is_closed())
            .cloned()
    }

    /// Keeps `sender` as this connection's.
    fn keep(&self, sender: SendRequest<Incoming>) {
        *self.open.lock().unwrap_or_else(PoisonError::into_inner) = Some(sender);
    }
}

/// A relay: its origin, how it reaches it, and the connections it keeps.
pub struct Relay {
    origin: Origin,
    tls: TlsConnector,
    clocks: Clocks,
    slots: Vec<Slot>,
    next: AtomicUsize,
    /// When an attempt to open a connection last failed, and why.
    failed: Held<Option<(Instant, Unreachable)>>,
    /// The relay's clock and the wall's when a connection was last asked
    /// for, to tell when the machine slept between.
    asked_at: Held<(Instant, SystemTime)>,
    /// Whether it has begun to leave, when a request is sent direct.
    leaving: AtomicBool,
    connections: Arc<Connections>,
    counts: Arc<Counts>,
    busy: Arc<Busy>,
}

impl Relay {
    /// A relay for `origin`, keeping at most `connections` connections to
    /// it, trusting `roots`, on `clocks`.
    ///
    /// # Errors
    ///
    /// When TLS can't be set up with the protocol versions it's safe to
    /// offer, which only a broken build of rustls would do.
    pub fn new(
        origin: Origin,
        connections: usize,
        roots: Roots,
        clocks: Clocks,
    ) -> Result<Relay, String> {
        let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|why| why.to_string())?
        .with_root_certificates(roots.0)
        .with_no_client_auth();
        config.alpn_protocols = vec![b"h2".to_vec()];
        Ok(Relay {
            origin,
            tls: TlsConnector::from(Arc::new(config)),
            clocks,
            slots: (0..connections.max(1))
                .map(|_| Slot {
                    open: Held::new(None),
                    opening: Mutex::new(()),
                })
                .collect(),
            next: AtomicUsize::new(0),
            failed: Held::new(None),
            asked_at: Held::new((Instant::now(), SystemTime::now())),
            leaving: AtomicBool::new(false),
            connections: Arc::new(Connections::default()),
            counts: Arc::new(Counts::default()),
            busy: Arc::new(Busy {
                now: AtomicUsize::new(0),
                quiet_since: Held::new(Instant::now()),
            }),
        })
    }

    /// Answers `listener` until the process is stopped, or until it has
    /// sat idle as long as its clocks say. Then it leaves: each connection
    /// it holds is closed as its answer ends, and while any is open, a
    /// request on a connection taken since is sent direct with a 421, so
    /// no pi waits on a relay that's going.
    pub async fn serve(self, listener: UnixListener) {
        eprintln!(
            "model-relay: forwarding to {} over at most {} connections",
            self.origin.authority,
            self.slots.len()
        );
        let relay = Arc::new(self);
        tokio::spawn(say_counts(Arc::clone(&relay.counts)));
        let (leave, leaving) = watch::channel(false);
        loop {
            tokio::select! {
                () = relay.accept(&listener, &leaving) => {}
                () = relay.idle() => {}
            }
            // Taking no more, it gives a connection it took just now the
            // moment pi needs to say its request, which then holds it here.
            tokio::time::sleep(LEAVING_GRACE).await;
            if relay.is_idle() {
                break;
            }
        }
        let idle_for = relay.clocks.idle_exit.unwrap_or_default().as_secs();
        eprintln!("model-relay: idle for {idle_for} s, so it leaves; its socket starts it again");
        relay.leaving.store(true, Ordering::SeqCst);
        // Nobody is left to tell when every connection has gone already.
        let _ = leave.send(true);
        tokio::select! {
            () = relay.accept(&listener, &leaving) => {}
            () = relay.connections.none_held() => {}
        }
    }

    /// Answers each connection `listener` takes. One taken before `leaving`
    /// says so is closed as its answer ends once it does; one taken after
    /// answers a request and closes.
    async fn accept(self: &Arc<Relay>, listener: &UnixListener, leaving: &watch::Receiver<bool>) {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let relay = Arc::clone(self);
                    let mut leaving = leaving.clone();
                    let holding = self.connections.one();
                    tokio::spawn(async move {
                        let _holding = holding;
                        let late = *leaving.borrow_and_update();
                        let service = hyper::service::service_fn(move |request| {
                            let relay = Arc::clone(&relay);
                            async move { Ok::<_, Infallible>(relay.forward(request).await) }
                        });
                        let connection = hyper::server::conn::http1::Builder::new()
                            .timer(TokioTimer::new())
                            .keep_alive(!late)
                            .serve_connection(TokioIo::new(stream), service);
                        tokio::pin!(connection);
                        // A pi that hangs up mid-answer ends only its own
                        // connection, and there's nobody left to tell.
                        if late {
                            let _ = connection.await;
                            return;
                        }
                        tokio::select! {
                            _ = connection.as_mut() => return,
                            _ = leaving.wait_for(|leave| *leave) => {
                                connection.as_mut().graceful_shutdown();
                            }
                        }
                        let _ = connection.await;
                    });
                }
                Err(why) => {
                    eprintln!("model-relay: couldn't accept a connection: {why}");
                    tokio::time::sleep(ACCEPT_AGAIN_AFTER).await;
                }
            }
        }
    }

    /// Ends once the relay has had nothing in hand for its idle time, or
    /// never, when it has none.
    async fn idle(&self) {
        let Some(idle_exit) = self.clocks.idle_exit else {
            return std::future::pending().await;
        };
        let look_every = (idle_exit / 4).max(Duration::from_millis(100));
        loop {
            tokio::time::sleep(look_every).await;
            if self.is_idle() {
                return;
            }
        }
    }

    /// Whether it has had nothing in hand for its idle time.
    fn is_idle(&self) -> bool {
        self.clocks.idle_exit.is_some_and(|idle_exit| {
            self.busy
                .quiet_for()
                .is_some_and(|quiet| quiet >= idle_exit)
        })
    }

    /// Carries one request to the origin and its answer back.
    async fn forward(self: &Arc<Relay>, request: Request<Incoming>) -> Response<Body> {
        self.counts.asked.fetch_add(1, Ordering::Relaxed);
        let (mut parts, body) = request.into_parts();
        let host = parts
            .headers
            .get(header::HOST)
            .and_then(|value| value.to_str().ok());
        let names_origin = host.is_some_and(|named| self.origin.is_named_by(named))
            && parts
                .uri
                .authority()
                .is_none_or(|named| self.origin.is_named_by(named.as_str()));
        if !names_origin {
            self.counts.refused.fetch_add(1, Ordering::Relaxed);
            return said(
                StatusCode::MISDIRECTED_REQUEST,
                "this relay forwards to its own origin alone\n",
            );
        }
        if self.leaving.load(Ordering::SeqCst) {
            self.counts.refused.fetch_add(1, Ordering::Relaxed);
            return said(
                StatusCode::MISDIRECTED_REQUEST,
                "this relay is leaving, so go directly\n",
            );
        }
        let in_hand = self.busy.one();
        let path = parts
            .uri
            .path_and_query()
            .cloned()
            .unwrap_or_else(|| PathAndQuery::from_static("/"));
        let Ok(uri) = Uri::builder()
            .scheme(Scheme::HTTPS)
            .authority(self.origin.authority.clone())
            .path_and_query(path)
            .build()
        else {
            self.counts.refused.fetch_add(1, Ordering::Relaxed);
            return said(
                StatusCode::BAD_REQUEST,
                "this relay couldn't read the request's path\n",
            );
        };
        parts.uri = uri;
        parts.version = Version::HTTP_2;
        without_connection_headers(&mut parts.headers);

        let mut request = Request::from_parts(parts, body);
        let mut sent = 0;
        let answered = loop {
            let mut sender = match self.sender().await {
                Ok(sender) => sender,
                Err(why) => {
                    self.counts.failed.fetch_add(1, Ordering::Relaxed);
                    return why.said();
                }
            };
            match sender.try_send_request(request).await {
                Ok(answer) => break Ok(answer),
                Err(mut failed) => match failed.take_message() {
                    // Its connection was retired before the request went,
                    // so it goes again, on a connection still open.
                    Some(unread) if sent < SENT_AGAIN_AT_MOST => {
                        sent += 1;
                        request = unread;
                    }
                    _ => break Err(failed.into_error()),
                },
            }
        };
        match answered {
            Ok(answer) => {
                let (mut parts, body) = answer.into_parts();
                parts.version = Version::HTTP_11;
                without_connection_headers(&mut parts.headers);
                // The request stays in hand until its answer is sent or
                // given up on, so the relay never leaves mid-answer.
                let body = body.map_err(move |why| {
                    let _ = &in_hand;
                    why
                });
                Response::from_parts(parts, body.boxed())
            }
            Err(why) => {
                self.counts.failed.fetch_add(1, Ordering::Relaxed);
                // hyper's error says what failed, never what was sent,
                // which a test holds it to.
                eprintln!("model-relay: a request couldn't be carried: {why}");
                said(
                    StatusCode::BAD_GATEWAY,
                    "this relay's request to its origin failed\n",
                )
            }
        }
    }

    /// A connection to send down: the next in turn if it's open, or any
    /// other that is while the next is opened behind it, or, with none
    /// open, the next not being opened already, once it's opened.
    async fn sender(self: &Arc<Relay>) -> Result<SendRequest<Incoming>, Unreachable> {
        self.retire_after_a_sleep();
        let count = self.slots.len();
        let at = self.next.fetch_add(1, Ordering::Relaxed) % count;
        if let Some(sender) = self.slots[at].sender() {
            return Ok(sender);
        }
        if let Some(sender) = (1..count).find_map(|step| self.slots[(at + step) % count].sender()) {
            self.open_behind(at);
            return Ok(sender);
        }
        // One being opened behind may be stuck on an origin that takes
        // no more connections, and another may open at once.
        let free = (0..count)
            .map(|step| (at + step) % count)
            .find(|&slot| self.slots[slot].opening.try_lock().is_ok())
            .unwrap_or(at);
        self.open(free).await
    }

    /// Retires every connection when the wall clock ran ahead of the
    /// relay's own since it was last asked, as it does while the machine
    /// sleeps: the network they ran on may be gone, and their pings would
    /// take up to `PING_EVERY` and `PING_ANSWERED_WITHIN` to say so. An
    /// answer already streaming on one carries on.
    fn retire_after_a_sleep(&self) {
        let now = (Instant::now(), SystemTime::now());
        let then = std::mem::replace(
            &mut *self.asked_at.lock().unwrap_or_else(PoisonError::into_inner),
            now,
        );
        if !slept(then, now) {
            return;
        }
        eprintln!("model-relay: the machine slept, so its connections are opened afresh");
        for slot in &self.slots {
            *slot.open.lock().unwrap_or_else(PoisonError::into_inner) = None;
        }
    }

    /// Opens connection `at` without anybody waiting on it, unless it's
    /// being opened already, or an attempt failed lately.
    fn open_behind(self: &Arc<Relay>, at: usize) {
        let lately = self
            .failed
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_some_and(|(when, _)| when.elapsed() < OPEN_BEHIND_AGAIN_AFTER);
        if lately {
            return;
        }
        let relay = Arc::clone(self);
        tokio::spawn(async move {
            let Ok(_opening) = relay.slots[at].opening.try_lock() else {
                return;
            };
            if relay.slots[at].sender().is_none() {
                // Its failure is kept as one opened in front is, so a
                // request that waited on this attempt hears it.
                let _ = relay.open_now(at).await;
            }
        });
    }

    /// Connection `at`, opened by this request or by the one before it
    /// that was opening it. A request that waited on an attempt that
    /// failed hears that failure, rather than making its own.
    async fn open(&self, at: usize) -> Result<SendRequest<Incoming>, Unreachable> {
        let waited_from = Instant::now();
        let _opening = self.slots[at].opening.lock().await;
        if let Some(sender) = self.slots[at].sender() {
            return Ok(sender);
        }
        let failed = *self.failed.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((when, why)) = failed
            && when >= waited_from
        {
            return Err(why);
        }
        self.open_now(at).await
    }

    /// Opens connection `at` and keeps it, or keeps why it couldn't be,
    /// for whoever waited on the attempt. Its opener holds its lock.
    async fn open_now(&self, at: usize) -> Result<SendRequest<Incoming>, Unreachable> {
        match self.connect(at).await {
            Ok(sender) => {
                self.slots[at].keep(sender.clone());
                // The origin is back, so others may be opened behind.
                *self.failed.lock().unwrap_or_else(PoisonError::into_inner) = None;
                Ok(sender)
            }
            Err(why) => {
                *self.failed.lock().unwrap_or_else(PoisonError::into_inner) =
                    Some((Instant::now(), why));
                Err(why)
            }
        }
    }

    /// Opens connection `at` to the origin, saying so, and says when it's
    /// retired.
    async fn connect(&self, at: usize) -> Result<SendRequest<Incoming>, Unreachable> {
        let within = self.clocks.connect_within;
        let opening = async {
            let on_the_way = |why: io::Error| {
                let unreachable = if why.kind() == io::ErrorKind::TimedOut {
                    Unreachable::Late
                } else {
                    Unreachable::Refused
                };
                (why, unreachable)
            };
            let tcp = reach(&self.origin.host, self.origin.port, within)
                .await
                .map_err(on_the_way)?;
            tcp.set_nodelay(true).map_err(on_the_way)?;
            let name = ServerName::try_from(self.origin.host.clone())
                .map_err(|why| (io::Error::other(why), Unreachable::Unfit))?;
            let tls = self.tls.connect(name, tcp).await.map_err(|why| {
                // rustls refusing the origin is the relay's own doing; the
                // network failing beneath it isn't.
                if why
                    .get_ref()
                    .is_some_and(<dyn std::error::Error + Send + Sync>::is::<rustls::Error>)
                {
                    (why, Unreachable::Unfit)
                } else {
                    on_the_way(why)
                }
            })?;
            if tls.get_ref().1.alpn_protocol() != Some(b"h2") {
                return Err((
                    io::Error::other("the origin doesn't speak HTTP/2"),
                    Unreachable::Unfit,
                ));
            }
            hyper::client::conn::http2::Builder::new(TokioExecutor::new())
                .timer(TokioTimer::new())
                .keep_alive_interval(PING_EVERY)
                .keep_alive_timeout(PING_ANSWERED_WITHIN)
                .keep_alive_while_idle(true)
                .adaptive_window(true)
                .handshake(TokioIo::new(tls))
                .await
                .map_err(|why| (io::Error::other(why), Unreachable::Refused))
        };
        let (sender, connection) = match tokio::time::timeout(within, opening).await {
            Ok(Ok(opened)) => opened,
            Ok(Err((why, unreachable))) => {
                eprintln!("model-relay: connection {at} couldn't be made: {why}");
                return Err(unreachable);
            }
            Err(_) => {
                eprintln!(
                    "model-relay: connection {at} wasn't made within {} ms",
                    within.as_millis()
                );
                return Err(Unreachable::Late);
            }
        };
        self.counts.opened.fetch_add(1, Ordering::Relaxed);
        let open = self.counts.open.fetch_add(1, Ordering::Relaxed) + 1;
        eprintln!("model-relay: connection {at} opened, {open} taking requests");
        let counts = Arc::clone(&self.counts);
        tokio::spawn(async move {
            // This ends once the connection takes no new requests, which
            // after the origin's goodbye may be before its last answers
            // have streamed: so the connection is said as retired, not
            // closed, and counted as no longer taking requests.
            let ended = connection.await;
            let open = counts.open.fetch_sub(1, Ordering::Relaxed) - 1;
            match ended {
                Ok(()) => eprintln!("model-relay: connection {at} retired, {open} taking requests"),
                Err(why) => {
                    eprintln!(
                        "model-relay: connection {at} retired on {why}, {open} taking requests"
                    );
                }
            }
        });
        Ok(sender)
    }
}

/// A TCP connection to `host`'s `port`, from the first address the name
/// resolves to that answers.
async fn reach(host: &str, port: u16, within: Duration) -> io::Result<TcpStream> {
    let addresses: Vec<SocketAddr> = tokio::net::lookup_host((host, port)).await?.collect();
    if addresses.is_empty() {
        return Err(io::Error::other(format!("{host} resolved to no address")));
    }
    reach_any(&addresses, within).await
}

/// A TCP connection to one of `addresses`, trying each in turn with an
/// even share of what's left of `within`, so an address that never
/// answers can't hide one that would, and one that refuses at once leaves
/// its share to the rest. The error says it timed out only when every
/// address did, and otherwise says the last refusal.
///
/// One after another rather than racing them, as a browser does, since an
/// origin the relay is for resolves to one address family; one with an
/// address that never answers costs each opening that address's share.
async fn reach_any(addresses: &[SocketAddr], within: Duration) -> io::Result<TcpStream> {
    let started = Instant::now();
    let mut refused = None;
    for (tried, address) in addresses.iter().enumerate() {
        let left = within.saturating_sub(started.elapsed());
        let untried = u32::try_from(addresses.len() - tried).unwrap_or(u32::MAX);
        match tokio::time::timeout(left / untried, TcpStream::connect(address)).await {
            Ok(Ok(tcp)) => return Ok(tcp),
            Ok(Err(why)) => refused = Some(why),
            Err(_) => {}
        }
    }
    Err(refused
        .unwrap_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "no address answered in time")))
}

/// Whether the machine slept between `then` and `now`, each the relay's
/// clock and the wall's: the wall ran more than `SLEPT_AT_LEAST` ahead.
/// This takes `Instant` to stand still in a suspend, as Linux's monotonic
/// clock does; a wall clock set back is no sleep.
fn slept(then: (Instant, SystemTime), now: (Instant, SystemTime)) -> bool {
    let on_the_wall = now.1.duration_since(then.1).unwrap_or_default();
    on_the_wall > now.0.duration_since(then.0) + SLEPT_AT_LEAST
}

/// An answer of the relay's own: a status and a line saying why.
fn said(status: StatusCode, why: &'static str) -> Response<Body> {
    let mut answer = Response::new(
        Full::new(Bytes::from_static(why.as_bytes()))
            .map_err(|never| match never {})
            .boxed(),
    );
    *answer.status_mut() = status;
    answer.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("text/plain"),
    );
    answer
}

/// The headers that belong to one hop's connection, which HTTP/2 forbids
/// and the next hop mustn't read as its own: those any `Connection` line
/// names, and the standing ones, `Host` among them, since HTTP/2 names its
/// host as `:authority`. `TE` stays only as `trailers`, the one value
/// HTTP/2 allows.
fn without_connection_headers(headers: &mut HeaderMap) {
    let named: Vec<HeaderName> = headers
        .get_all(header::CONNECTION)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .filter_map(|name| HeaderName::from_bytes(name.trim().as_bytes()).ok())
        .collect();
    for name in named {
        headers.remove(name);
    }
    for name in [
        header::CONNECTION,
        header::HOST,
        header::TRANSFER_ENCODING,
        header::UPGRADE,
    ] {
        headers.remove(name);
    }
    for name in ["keep-alive", "proxy-connection"] {
        headers.remove(name);
    }
    let only_trailers = headers
        .get_all(header::TE)
        .iter()
        .all(|value| value.as_bytes().eq_ignore_ascii_case(b"trailers"));
    if !only_trailers {
        headers.remove(header::TE);
    }
}

/// Says the counts every `COUNTS_EVERY`, when anything was asked since.
async fn say_counts(counts: Arc<Counts>) {
    let mut last = 0;
    loop {
        tokio::time::sleep(COUNTS_EVERY).await;
        let asked = counts.asked.load(Ordering::Relaxed);
        if asked == last {
            continue;
        }
        last = asked;
        eprintln!(
            "model-relay: {asked} asked, {} refused, {} failed; {} connections opened, {} taking requests",
            counts.refused.load(Ordering::Relaxed),
            counts.failed.load(Ordering::Relaxed),
            counts.opened.load(Ordering::Relaxed),
            counts.open.load(Ordering::Relaxed),
        );
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant, SystemTime};

    use super::{Origin, SLEPT_AT_LEAST, reach_any, slept};

    #[test]
    fn an_origin_is_an_https_host_alone() {
        for origin in [
            "https://models.example",
            "https://models.example/",
            "https://models.example:8443",
        ] {
            assert!(Origin::parse(origin).is_ok(), "{origin} is an origin");
        }
        for not_one in [
            "http://models.example",
            "https://models.example/v1",
            "https://models.example/?q=1",
            "https://someone:secret@models.example",
            "models.example",
            "https://models.example:84433",
            "https://models.example:",
        ] {
            assert!(Origin::parse(not_one).is_err(), "{not_one} isn't an origin");
        }
    }

    #[test]
    fn a_request_names_its_origin_by_host_and_port() {
        let origin = Origin::parse("https://Models.Example").expect("an origin");
        for (asked, names_it) in [
            ("models.example", true),
            ("models.example:443", true),
            ("models.example:8443", false),
            ("models.example:65979", false),
            ("other.example", false),
            ("models.example.other.example", false),
            ("", false),
        ] {
            assert_eq!(origin.is_named_by(asked), names_it, "{asked:?}");
        }
    }

    #[test]
    fn a_sleep_is_a_wall_clock_running_ahead() {
        let then = (Instant::now(), SystemTime::now());
        let a_second = Duration::from_secs(1);
        let after = |own: Duration, wall: Duration| (then.0 + own, then.1 + wall);
        for (now, was_a_sleep) in [
            (after(a_second, a_second), false),
            (after(a_second, a_second + SLEPT_AT_LEAST), false),
            (
                after(
                    a_second,
                    a_second + SLEPT_AT_LEAST + Duration::from_millis(1),
                ),
                true,
            ),
            ((then.0 + a_second, then.1 - a_second), false),
        ] {
            assert_eq!(
                slept(then, now),
                was_a_sleep,
                "{:?} ahead",
                now.1.duration_since(then.1)
            );
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn an_address_that_refuses_leaves_the_rest_to_be_tried() {
        let closed = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
        let refusing = closed.local_addr().expect("its address");
        drop(closed);
        let open = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a port");
        let answering = open.local_addr().expect("its address");
        let within = Duration::from_secs(2);

        let reached = reach_any(&[refusing, answering], within).await;
        let none = reach_any(&[refusing], within).await;

        assert_eq!(
            reached.expect("the one that answers").peer_addr().ok(),
            Some(answering)
        );
        assert_eq!(
            none.err().map(|why| why.kind()),
            Some(std::io::ErrorKind::ConnectionRefused),
            "a refusal isn't said as a timeout"
        );
    }
}
