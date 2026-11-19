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
//! after it went is said as a 502, for pi's retry. One the origin reset
//! before its answer began retires the connection it went on, which may be
//! refusing every request it's given. Reset saying its stream was refused
//! or a protocol error, it goes once more on another: one opened since,
//! another already open, or one opened for it, shared by every request
//! reset there with it, never the one that reset it. Reset again, reset
//! saying anything else, or with no other connection to go on, as when its
//! slot retired one lately or none would open, it's a 502. Nothing of an
//! answer came, and one bad connection would otherwise fail every request
//! on it, each a turn some agent retries whole. A reset saying a protocol
//! error doesn't promise the origin acted on nothing, so a model's answer
//! may now and then be asked for twice, which pi's own retry of the whole
//! turn would do anyway. To send one again, it reads each body whole, to
//! 64 MiB, before it goes, from pi on this machine, and holds it until its
//! answer begins. After the machine sleeps, the connections it held are
//! retired and opened afresh, since the network they ran on may be gone
//! and pings would take their time to say so. A ping is what finds a lost
//! connection, and an unanswered one fails every request on it, so below
//! h2 each layer holds little ahead of one: a body goes in short frames,
//! through few records and a kernel holding little unsent, and on a slow
//! uplink a ping is answered in time.
//!
//! Past 15 in 100 failed of the requests it finished in ten minutes, 20
//! at least, it stands down (src/standing.rs says why): it retires its
//! connections and answers each request with its own 421, forwarding
//! nothing, so pi sends it direct, until ten minutes have passed, or
//! twice the last stand-down, to an hour, when the last ended under an
//! hour before. A request is judged once its answer ends, so one broken
//! off midway fails, and only one that went down a connection, or that
//! each connection it was offered closed under before it went: one that
//! never reached the origin, as when none would open, would fail direct
//! too, so it says nothing of the relay's connections. Nor does a reset
//! giving the origin's own verdict, such as to calm down, so it isn't
//! judged. Standing down, and for its idle time after, it doesn't leave,
//! which would forget it; taking requests again, it retires the
//! connections opened meanwhile by requests it had already taken, and
//! judges none of those. The last stand-down, which the next doubles, is
//! forgotten when it leaves or restarts. A sleep forgets a stand-down,
//! and what was sent before it, as the clock it ran on stopped and the
//! network it judged may be gone.
//!
//! It says on stderr only what a reader may keep: each connection opened
//! and retired, why one couldn't be made, and counts of what it served,
//! every ten minutes and as it leaves once idle; stopped by systemd, it
//! says nothing of the stretch since the last.
//! Nothing of a request's or an answer's headers or body is said, a failure
//! included, and nothing is written to disk. A request it couldn't carry is
//! answered as a 502, or a 504 when the origin couldn't be reached in time,
//! never by closing the socket, so pi's own retry decides what follows. A
//! body over 64 MiB is a 413, one pi leaves unfinished a 400 and one
//! still coming after a minute a 408, both counted as pi's cancel.
//!
//! A switch leaves a running relay be, since a restart would end every
//! answer still streaming. So it leaves on its own once nothing has been
//! forwarded for a while, with no answer in flight: it stops taking
//! connections, gives one it took just then a moment to say its request,
//! then closes each as its answer ends. systemd keeps the socket, so
//! whoever asks next is answered by whatever build and origin the machine
//! now names. A request refused as another host's isn't counted, so a
//! relay left naming an origin the machine no longer does still leaves.
//!
//! Long because the pool, the drain and the forwarding each read and set
//! the same few counters and slots under one relay, and split apart they
//! would hand each other its insides to keep one account of what is open.

use std::convert::Infallible;
use std::io;
use std::net::SocketAddr;
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex as Held, PoisonError};
use std::task::{Context, Poll};
use std::time::{Duration, Instant, SystemTime};

use bytes::{Buf, Bytes};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full, LengthLimitError, Limited};
use hyper::body::{Body as HttpBody, Frame, Incoming, SizeHint};
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

use standing::{Standing, Taking};

mod standing;

/// The port an https origin names when it names none.
const HTTPS_PORT: u16 = 443;
/// How long a connection to the origin may take to open, TLS included,
/// unless the relay is told otherwise.
pub const CONNECT_WITHIN: Duration = Duration::from_secs(10);
/// How long the relay waits, asked nothing and answering nothing, before
/// it leaves, unless told otherwise.
pub const IDLE_EXIT: Duration = Duration::from_secs(120);
/// How long its first stand-down lasts, unless the relay is told
/// otherwise.
pub const STANDS_DOWN_FOR: Duration = Duration::from_secs(600);
/// The longest a first stand-down may be told to last.
pub const STANDS_DOWN_FOR_AT_MOST: Duration = Duration::from_hours(24);
/// How often an open connection is pinged, so one the network lost is
/// found before a request is sent down it.
const PING_EVERY: Duration = Duration::from_secs(20);
/// How long a ping may wait for its answer before the connection is closed,
/// with every request on it. Long enough for a lossy uplink's resends to
/// back off a few times, since closing a live connection fails each request
/// it carries. One the network lost is found at most a minute after its
/// last frame was read, `PING_EVERY` and this together.
const PING_ANSWERED_WITHIN: Duration = Duration::from_secs(40);
/// The most each layer beneath h2's queue holds ahead of a ping: the one
/// DATA frame h2 is writing, rustls's records, and the kernel's unsent
/// bytes. hyper pings once nothing has been read for `PING_EVERY`, which
/// on an upload is the whole send, and h2 writes a pending ping before its
/// next frame, but behind whatever those layers already hold. Left alone,
/// a frame is as long as the stream's window (the origin allows 64 KiB
/// and frames to 16 MiB), rustls holds 64 KiB, and the kernel what
/// `net.ipv4.tcp_notsent_lowat` says, 128 KiB on our machines: measured on
/// 17 November, 128 KB waited on a connection moving 70 kbit/s, the ping
/// went unanswered within 20 s, and hyper closed the connection with every
/// request on it. At 16 KiB a layer, 48 KiB at most is ahead of a ping,
/// about six seconds at that rate, besides what is already in flight,
/// which the connection's window bounds and a lossy uplink keeps small.
const HELD_AHEAD_AT_MOST: usize = 16 * 1024;
/// The most of an error's account the journal keeps for one line, since
/// the origin's goodbye carries debug data of its choosing.
const SAID_AT_MOST: usize = 300;
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
/// How long a leaving relay still takes connections, to send each direct,
/// when its idle time is longer.
const TAKES_LATE_CONNECTIONS_FOR: Duration = Duration::from_secs(10);
/// How long a connection taken just as the relay leaves has to say its
/// request, which pi sends as it connects.
const LEAVING_GRACE: Duration = Duration::from_millis(500);
/// How far the wall clock may run ahead of the relay's own between two
/// requests before the machine is taken to have slept.
const SLEPT_AT_LEAST: Duration = Duration::from_secs(10);

/// What a relay answers with: the origin's body, or a line of its own.
type Body = BoxBody<Bytes, hyper::Error>;
/// What a relay sends to its origin: pi's body, read whole and sent in
/// pieces, so it can go again.
type Sent = InPieces<Full<Bytes>>;

/// A body handed on in pieces of at most `HELD_AHEAD_AT_MOST`, so that no
/// DATA frame h2 writes is longer, since h2 cuts a frame from one piece
/// and never joins two.
struct InPieces<B> {
    body: B,
    rest: Bytes,
}

impl<B> InPieces<B> {
    /// `body`, to be handed on in pieces.
    fn new(body: B) -> Self {
        Self {
            body,
            rest: Bytes::new(),
        }
    }

    /// The next piece of what's left of the last frame read, if any is.
    fn piece(&mut self) -> Option<Bytes> {
        if self.rest.is_empty() {
            return None;
        }
        let length = self.rest.len().min(HELD_AHEAD_AT_MOST);
        Some(self.rest.split_to(length))
    }
}

impl<B> HttpBody for InPieces<B>
where
    B: HttpBody<Data = Bytes> + Unpin,
{
    type Data = Bytes;
    type Error = B::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, B::Error>>> {
        if let Some(piece) = self.piece() {
            return Poll::Ready(Some(Ok(Frame::data(piece))));
        }
        let read = Pin::new(&mut self.body).poll_frame(cx);
        let Poll::Ready(Some(Ok(frame))) = read else {
            return read;
        };
        match frame.into_data() {
            Ok(data) => {
                self.rest = data;
                // An empty frame read stays an empty frame.
                let piece = self.piece().unwrap_or_default();
                Poll::Ready(Some(Ok(Frame::data(piece))))
            }
            Err(trailers) => Poll::Ready(Some(Ok(trailers))),
        }
    }

    fn is_end_stream(&self) -> bool {
        self.rest.is_empty() && self.body.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        let left = self.body.size_hint();
        let held = self.rest.len() as u64;
        let mut hint = SizeHint::new();
        hint.set_lower(left.lower().saturating_add(held));
        if let Some(upper) = left.upper() {
            hint.set_upper(upper.saturating_add(held));
        }
        hint
    }
}

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
    /// How long its first stand-down lasts.
    pub stands_down_for: Duration,
}

impl Default for Clocks {
    fn default() -> Clocks {
        Clocks {
            connect_within: CONNECT_WITHIN,
            idle_exit: Some(IDLE_EXIT),
            stands_down_for: STANDS_DOWN_FOR,
        }
    }
}

/// What a relay has done, as counts alone.
#[derive(Default)]
struct Counts {
    asked: AtomicU64,
    refused: AtomicU64,
    failed: AtomicU64,
    reset: AtomicU64,
    /// Requests reset before their answers began, sent once more and
    /// answered, so the resets a resend saved; those it didn't are with
    /// the failed ones reset.
    again: AtomicU64,
    cancelled: AtomicU64,
    opened: AtomicU64,
    open: AtomicUsize,
    /// Requests refused as it stood down, so each went direct, counted
    /// with the refused too.
    stood_down: AtomicU64,
    /// Answers broken off midway, judged or not, so a stand-down for
    /// streams that broke shows in the counts as well as resets do.
    broken: AtomicU64,
    /// What the counts were when last said, but for how many connections
    /// were taking requests.
    said_as: Held<[u64; 9]>,
}

impl Counts {
    /// Says the counts, when any has moved since they were last said: a
    /// request asked before then may have ended since.
    fn say(&self) {
        let now = [
            &self.asked,
            &self.refused,
            &self.failed,
            &self.reset,
            &self.again,
            &self.cancelled,
            &self.opened,
            &self.stood_down,
            &self.broken,
        ]
        .map(|count| count.load(Ordering::Relaxed));
        {
            let mut said_as = self.said_as.lock().unwrap_or_else(PoisonError::into_inner);
            if *said_as == now {
                return;
            }
            *said_as = now;
        }
        let [
            asked,
            refused,
            failed,
            reset,
            again,
            cancelled,
            opened,
            stood_down,
            broken,
        ] = now;
        eprintln!(
            "model-relay: {asked} asked, {refused} refused, {failed} failed, {reset} of them \
             reset; {again} reset and sent again; pi cancelled {cancelled}; {opened} \
             connections opened, {} taking requests; {stood_down} refused as it stood down; \
             {broken} broken off midway",
            self.open.load(Ordering::Relaxed),
        );
    }
}

/// What's open for a request reset to go once more on.
enum ForAgain {
    /// A connection, other than the one that reset it.
    Open(usize, Kept),
    /// None, as its slot kept the one that reset it and no other is open.
    Kept,
    /// None, with the one that reset it retired, so one may be opened.
    Closed,
}

/// A request reset and waiting on a connection to go once more. Should pi
/// leave meanwhile, hyper drops the request with nothing said, so it's
/// counted as pi's cancel.
struct WaitingAgain<'a>(Option<&'a Counts>);

impl<'a> WaitingAgain<'a> {
    /// A request starting to wait, for `counts`.
    fn new(counts: &'a Counts) -> Self {
        WaitingAgain(Some(counts))
    }

    /// Its wait is over, with or without a connection.
    fn over(mut self) {
        self.0 = None;
    }
}

impl Drop for WaitingAgain<'_> {
    fn drop(&mut self) {
        if let Some(counts) = self.0 {
            counts.cancelled.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// What one connection to the origin has carried: the requests sent down
/// it, and how many of those pi gave up on before their answers ended.
/// hyper cancels each of those at the origin, if it had reached it, and on
/// 18 November the origin reset every stream on a connection at once,
/// several connections in the same second, whatever their age: an origin
/// that counts the cancels a connection carries would do that, and a
/// shared connection gathers the cancels of every session on it.
///
/// It says its last count once nothing holds it, the connection retired
/// and every answer it carried ended or given up on, since an answer can
/// still be read after its connection has gone.
#[derive(Default)]
struct Tally {
    at: usize,
    number: u64,
    asked: AtomicU64,
    cancelled: AtomicU64,
}

impl Tally {
    /// The tally of connection `at`'s `number`.
    fn of(at: usize, number: u64) -> Tally {
        Tally {
            at,
            number,
            asked: AtomicU64::new(0),
            cancelled: AtomicU64::new(0),
        }
    }

    /// How the log says it so far.
    fn said(&self) -> String {
        format!(
            "after {} asked and {} cancelled on it",
            self.asked.load(Ordering::Relaxed),
            self.cancelled.load(Ordering::Relaxed)
        )
    }
}

impl Drop for Tally {
    fn drop(&mut self) {
        let asked = self.asked.load(Ordering::Relaxed);
        // A connection that never opened, or carried nothing, has nothing
        // to say.
        if asked > 0 {
            eprintln!(
                "model-relay: connection {}.{} carried {asked} asked, {} of them cancelled",
                self.at,
                self.number,
                self.cancelled.load(Ordering::Relaxed)
            );
        }
    }
}

/// One request on a connection, from when it's sent down it until its
/// answer ends or fails. Dropped before either, pi gave up on it, and it's
/// counted as cancelled.
struct OnTheWay {
    tally: Arc<Tally>,
    counts: Arc<Counts>,
    ended: AtomicBool,
    /// When it was sent, so a request sent before a sleep, or before the
    /// relay took requests again, isn't judged.
    sent: Instant,
}

impl OnTheWay {
    /// A request sent down the connection `tally` counts for.
    fn sent(tally: &Arc<Tally>, counts: &Arc<Counts>) -> OnTheWay {
        tally.asked.fetch_add(1, Ordering::Relaxed);
        OnTheWay {
            tally: Arc::clone(tally),
            counts: Arc::clone(counts),
            ended: AtomicBool::new(false),
            sent: Instant::now(),
        }
    }

    /// It came to an end of its own: answered whole, or failed. Says
    /// whether it ended just now, rather than before.
    fn end(&self) -> bool {
        !self.ended.swap(true, Ordering::Relaxed)
    }

    /// hyper handed it back unsent, so it was never this connection's.
    fn unsent(&self) {
        self.tally.asked.fetch_sub(1, Ordering::Relaxed);
        self.end();
    }
}

impl Drop for OnTheWay {
    fn drop(&mut self) {
        if !self.ended.load(Ordering::Relaxed) {
            self.tally.cancelled.fetch_add(1, Ordering::Relaxed);
            self.counts.cancelled.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// An answer's body, whose request ends once the body has: read to its
/// end, or as far as its length said, or failed. The relay judges the
/// request then, so one broken off midway is judged failed.
struct Answering<B> {
    body: B,
    on_the_way: OnTheWay,
    left: Option<u64>,
    relay: Arc<Relay>,
}

impl<B: HttpBody> Answering<B> {
    /// `body`, the answer to the request `on_the_way`, which `relay`
    /// carried.
    fn new(body: B, on_the_way: OnTheWay, relay: Arc<Relay>) -> Self {
        let left = body.size_hint().exact();
        let answering = Self {
            body,
            on_the_way,
            left,
            relay,
        };
        if left == Some(0) {
            answering.ended(None);
        }
        answering
    }
}

impl<B> Answering<B> {
    /// Ends the request, answered whole or failing with `why`, and has
    /// the relay judge it, once, as `fails_the_path` says.
    fn ended(&self, why: Option<&(dyn std::error::Error + 'static)>) {
        if !self.on_the_way.end() {
            return;
        }
        let Some(why) = why else {
            return self.relay.finished(self.on_the_way.sent, false);
        };
        self.relay.counts.broken.fetch_add(1, Ordering::Relaxed);
        if fails_the_path(why) {
            self.relay.finished(self.on_the_way.sent, true);
        }
    }
}

impl<B> HttpBody for Answering<B>
where
    B: HttpBody + Unpin,
    B::Error: std::error::Error + 'static,
{
    type Data = B::Data;
    type Error = B::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<B::Data>, B::Error>>> {
        let read = Pin::new(&mut self.body).poll_frame(cx);
        match &read {
            Poll::Ready(None) => self.ended(None),
            Poll::Ready(Some(Err(why))) => {
                self.ended(Some(why as &(dyn std::error::Error + 'static)));
            }
            Poll::Ready(Some(Ok(frame))) => {
                let length = frame.data_ref().map_or(0, |data| data.remaining() as u64);
                // hyper stops reading an answer once it has written the
                // length it was given, before the frame that ends it.
                if let Some(left) = self.left.as_mut() {
                    *left = left.saturating_sub(length);
                    if *left == 0 {
                        self.ended(None);
                    }
                }
            }
            Poll::Pending => {}
        }
        read
    }

    fn is_end_stream(&self) -> bool {
        // hyper stops reading a body that says it has ended, so this is
        // where an answer read whole may be heard of last.
        let ended = self.body.is_end_stream();
        if ended {
            self.ended(None);
        }
        ended
    }

    fn size_hint(&self) -> SizeHint {
        self.body.size_hint()
    }
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

/// One of the connections a relay keeps: the one open, if it is, how many
/// it has kept, and the lock whoever opens it holds, so it's opened once.
struct Slot<S = SendRequest<Sent>> {
    open: Held<Option<Kept<S>>>,
    kept: AtomicU64,
    opening: Mutex<()>,
    retired: Held<Option<Instant>>,
}

/// How seldom a slot retires a connection for a reset, at most. One bad
/// connection is retired at its first reset, and its fresh one isn't
/// retired again for this long. A retired connection stays up while its
/// answers stream, minutes for a long one, so an origin resetting on every
/// connection would otherwise have the relay hold a connection for each
/// reset, where it means to hold a few.
const RETIRED_FOR_A_RESET_AT_MOST_EVERY: Duration = Duration::from_secs(60);

/// The largest body the relay reads, which it holds whole until its answer
/// begins so a reset request can go again. pi's turns run to a few
/// megabytes, images and all, and the socket is the account's alone, so
/// this only keeps a runaway sender from taking the relay's memory.
const BODY_AT_MOST: usize = 64 * 1024 * 1024;
/// How long the relay waits on a body to end. pi writes its whole body at
/// once over a socket on this machine, so one still coming after this was
/// left unfinished, and a request in hand would otherwise keep the relay,
/// and its build, from ever leaving.
const BODY_WITHIN: Duration = Duration::from_secs(60);

/// A connection a slot keeps: its sender, which of the slot's it is, so
/// what retires it can't retire one opened since, and what it has carried.
#[derive(Clone)]
struct Kept<S = SendRequest<Sent>> {
    number: u64,
    sender: S,
    tally: Arc<Tally>,
}

/// What a slot keeps of a connection: something to send down, which says
/// when the connection beneath it has closed.
trait Sender: Clone {
    /// Whether its connection has closed.
    fn is_closed(&self) -> bool;
}

impl Sender for SendRequest<Sent> {
    fn is_closed(&self) -> bool {
        SendRequest::is_closed(self)
    }
}

impl<S> Slot<S> {
    /// A slot with no connection yet.
    fn new() -> Self {
        Slot {
            open: Held::new(None),
            kept: AtomicU64::new(0),
            opening: Mutex::new(()),
            retired: Held::new(None),
        }
    }

    /// The number the next connection opened in this slot is known by.
    fn next_number(&self) -> u64 {
        self.kept.fetch_add(1, Ordering::Relaxed)
    }
}

impl<S: Sender> Slot<S> {
    /// This connection, while it takes requests.
    fn sender(&self) -> Option<Kept<S>> {
        self.open
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .filter(|kept| !kept.sender.is_closed())
            .cloned()
    }

    /// Keeps `sender` as this connection's, counted by `tally`.
    fn keep(&self, number: u64, sender: S, tally: Arc<Tally>) -> Kept<S> {
        let kept = Kept {
            number,
            sender,
            tally,
        };
        *self.open.lock().unwrap_or_else(PoisonError::into_inner) = Some(kept.clone());
        kept
    }

    /// Takes no more requests down connection `number` for a reset, if
    /// it's still the one taking them and the slot hasn't retired one
    /// within `RETIRED_FOR_A_RESET_AT_MOST_EVERY`, and says what came of it.
    /// Answers already on it carry on. One the origin said goodbye to is
    /// gone already, so its reset spends none of the slot's allowance.
    fn retire(&self, number: u64) -> Retiring {
        let mut open = self.open.lock().unwrap_or_else(PoisonError::into_inner);
        if open
            .as_ref()
            .is_none_or(|kept| kept.number != number || kept.sender.is_closed())
        {
            return Retiring::Gone;
        }
        let mut retired = self.retired.lock().unwrap_or_else(PoisonError::into_inner);
        if retired.is_some_and(|when| when.elapsed() < RETIRED_FOR_A_RESET_AT_MOST_EVERY) {
            return Retiring::Kept;
        }
        *open = None;
        *retired = Some(Instant::now());
        Retiring::Retired
    }
}

/// What a reset did to the connection it came on. Told apart in the log
/// because a spell's later resets mostly come on requests still in flight
/// on a connection already retired, and said as kept they read as the
/// relay holding on to a bad connection.
#[derive(Debug, PartialEq, Eq)]
enum Retiring {
    /// It takes no more requests.
    Retired,
    /// It takes no more requests already: retired before, let go after a
    /// sleep, or closed by the origin's goodbye.
    Gone,
    /// It's still taking requests, as the slot retired one lately.
    Kept,
}

impl Retiring {
    /// How the log says it.
    fn said(&self) -> &'static str {
        match self {
            Retiring::Retired => "so it's retired",
            Retiring::Gone => "already retired",
            Retiring::Kept => "kept, as one was retired lately",
        }
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
    /// The relay's clock and the wall's when a request was last taken, or a
    /// connection last asked for, to tell when the machine slept between.
    asked_at: Held<(Instant, SystemTime)>,
    /// Whether it has begun to leave, when a request is sent direct.
    leaving: AtomicBool,
    connections: Arc<Connections>,
    counts: Arc<Counts>,
    busy: Arc<Busy>,
    /// Whether it takes requests, judged by those it finished lately.
    standing: Standing,
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
            slots: (0..connections.max(1)).map(|_| Slot::new()).collect(),
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
            standing: Standing::new(clocks.stands_down_for),
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
        // Said as it leaves too, or the last stretch's would go unsaid.
        relay.counts.say();
        let idle_for = relay.clocks.idle_exit.unwrap_or_default().as_secs();
        eprintln!("model-relay: idle for {idle_for} s, so it leaves; its socket starts it again");
        relay.leaving.store(true, Ordering::SeqCst);
        // Nobody is left to tell when every connection has gone already.
        let _ = leave.send(true);
        // Pis that keep connecting would keep a relay taking them going for
        // ever, so it takes them only a while; after that they wait on the
        // socket for whatever systemd starts next.
        let taking_for = relay
            .clocks
            .idle_exit
            .map_or(TAKES_LATE_CONNECTIONS_FOR, |idle| {
                idle.min(TAKES_LATE_CONNECTIONS_FOR)
            });
        let drained = relay.connections.none_held();
        tokio::pin!(drained);
        tokio::select! {
            () = relay.accept(&listener, &leaving) => {}
            () = tokio::time::sleep(taking_for) => drained.await,
            () = &mut drained => {}
        }
        // Once more for those refused while it left.
        relay.counts.say();
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

    /// Whether it has had nothing in hand for its idle time, and hasn't
    /// stood down within it, which leaving would forget.
    fn is_idle(&self) -> bool {
        self.clocks.idle_exit.is_some_and(|idle_exit| {
            self.busy
                .quiet_for()
                .is_some_and(|quiet| quiet >= idle_exit)
                && !self.standing.holds_on(Instant::now(), idle_exit)
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
        // Another origin's request is no work of the relay's, so it's no
        // reason to stay, even while its body is read.
        let in_hand = names_origin.then(|| self.busy.one());
        // Its body read whole before any 421, so pi isn't writing one the
        // relay has stopped reading, failing on a broken pipe, when the
        // 421 would send it direct. A body too large still gets the 421, as
        // it was never to go to the origin anyway.
        let read = read_whole(body).await;
        // A body pi left, or one still coming past its deadline, which pi
        // would have written at once, is a cancel whatever else, as nobody
        // waits on a 421 for it. The tests can't wait out the deadline, so
        // a late body's place here is held by reading alone.
        if let Err(why @ (Unread::Gone | Unread::Late)) = read {
            return self.unread(why);
        }
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
        // The sleep looked for first, as a sleep forgets a stand-down, and
        // the stand-down's 421 said for a body too large too, as the origin
        // asked directly may take one the relay won't.
        self.retire_after_a_sleep();
        if let Some(standing_down) = self.standing_down() {
            return standing_down;
        }
        let whole = match read {
            Ok(whole) => whole,
            Err(unread) => return self.unread(unread),
        };
        if let Err(refused) = self.pointed(&mut parts) {
            return *refused;
        }
        let (answer, on_the_way) = match self.carry(&parts, &whole).await {
            Ok(answered) => answered,
            Err(not_carried) => return *not_carried,
        };
        let (mut parts, body) = answer.into_parts();
        parts.version = Version::HTTP_11;
        without_connection_headers(&mut parts.headers);
        // The request stays in hand until its answer is sent or given up on,
        // so the relay never leaves mid-answer.
        let body = Answering::new(body, on_the_way, Arc::clone(self)).map_err(move |why| {
            let _ = &in_hand;
            why
        });
        Response::from_parts(parts, body.boxed())
    }

    /// The relay's own 421, sending pi direct, while it stands down; or
    /// none, said as it takes requests again when it does just now.
    fn standing_down(&self) -> Option<Response<Body>> {
        match self.standing.taking(Instant::now()) {
            Taking::Yes => None,
            Taking::Again(lasted) => {
                // Requests it had taken before it stood down may have
                // opened connections since, on the path it stood down from.
                self.retire_every();
                eprintln!(
                    "model-relay: it stood down {} s, so it takes requests again, on \
                     connections opened afresh",
                    lasted.as_secs()
                );
                None
            }
            Taking::No(more) => {
                self.counts.refused.fetch_add(1, Ordering::Relaxed);
                self.counts.stood_down.fetch_add(1, Ordering::Relaxed);
                // Its words hold the ones pi goes direct on, as for the
                // 421 of a request for another host, and are true here
                // too.
                let mut answer = said(
                    StatusCode::MISDIRECTED_REQUEST,
                    "this relay forwards to its own origin alone, and stands down from it for \
                     now, so go directly\n",
                );
                answer
                    .headers_mut()
                    .insert(header::RETRY_AFTER, (more.as_secs() + 1).into());
                Some(answer)
            }
        }
    }

    /// Hears a request `sent` down a connection, and answered or `failed`
    /// for its path, and stands down when the share failed lately says to.
    /// A sleep is looked for first, as the streams it cut break before pi
    /// asks anything new, and they're not to be judged. No test can sleep
    /// the machine, so this order is held by reading it alone.
    fn finished(&self, sent: Instant, failed: bool) {
        self.retire_after_a_sleep();
        let Some(stood) = self.standing.heard(Instant::now(), sent, failed) else {
            return;
        };
        eprintln!(
            "model-relay: {} of the {} requests it finished in ten minutes failed, so it \
             stands down for {} s, retiring its connections, and pi goes directly",
            stood.failed,
            stood.finished,
            stood.lasting.as_secs()
        );
        self.retire_every();
    }

    /// Points a request pi asked of the origin at the origin itself, or
    /// says why pi is refused.
    fn pointed(&self, parts: &mut hyper::http::request::Parts) -> Result<(), Box<Response<Body>>> {
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
            return Err(Box::new(said(
                StatusCode::BAD_REQUEST,
                "this relay couldn't read the request's path\n",
            )));
        };
        parts.uri = uri;
        parts.version = Version::HTTP_2;
        without_connection_headers(&mut parts.headers);
        Ok(())
    }

    /// What pi is answered for a body that wasn't read whole, counted and
    /// said.
    fn unread(&self, why: Unread) -> Response<Body> {
        match why {
            Unread::TooLarge => {
                self.counts.refused.fetch_add(1, Ordering::Relaxed);
                eprintln!("model-relay: a request's body passed 64 MiB, so it's refused");
                said(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "this relay carries a body of 64 MiB at most\n",
                )
            }
            // Still coming after the deadline, it's said, so a pi writing
            // slowly is told from one that left.
            Unread::Late => {
                self.counts.cancelled.fetch_add(1, Ordering::Relaxed);
                eprintln!(
                    "model-relay: a request's body was still coming after {} s, so it's \
                     dropped",
                    BODY_WITHIN.as_secs()
                );
                said(
                    StatusCode::REQUEST_TIMEOUT,
                    "this relay waited too long on the request's body\n",
                )
            }
            // pi went before its body ended, as it does when its person
            // presses Escape, so it's a cancel, and nobody hears the answer.
            Unread::Gone => {
                self.counts.cancelled.fetch_add(1, Ordering::Relaxed);
                said(
                    StatusCode::BAD_REQUEST,
                    "this relay couldn't read the request's body\n",
                )
            }
        }
    }

    /// Sends a request to the origin until its answer begins, or says what
    /// pi is answered when it never does.
    async fn carry(
        self: &Arc<Relay>,
        parts: &hyper::http::request::Parts,
        whole: &Bytes,
    ) -> Result<(Response<Incoming>, OnTheWay), Box<Response<Body>>> {
        let (mut request, mut went) = (sendable(parts, whole), Instant::now());
        let mut sent = 0;
        let mut taken = self.sender().await;
        // The connection the origin reset this request on, once it has.
        // Sent again, it's answered or fails on that send, so a request
        // reset is one not yet sent again for as long as the loop goes.
        let mut reset_on: Option<(usize, u64)> = None;
        let mut down;
        let answered = loop {
            let (
                at,
                Kept {
                    number,
                    mut sender,
                    tally,
                },
            ) = match taken {
                Ok(taken) => taken,
                Err(why) => {
                    self.counts.failed.fetch_add(1, Ordering::Relaxed);
                    // Never sent, it says nothing of the connections, as
                    // it would have failed direct too, so it isn't judged.
                    let Some((was_at, was)) = reset_on else {
                        return Err(Box::new(why.said()));
                    };
                    self.finished(went, true);
                    // Reset, and then not sent again, so it's counted with
                    // the requests reset that failed, and a 502 as any
                    // reset is, never a 421 sending pi direct: the origin
                    // may have acted on it.
                    self.counts.reset.fetch_add(1, Ordering::Relaxed);
                    eprintln!(
                        "model-relay: a request reset on connection {was_at}.{was} couldn't be \
                         sent again, as no other connection would take it; pi is answered 502"
                    );
                    return Err(Box::new(not_answered()));
                }
            };
            // Dropped while it waits for the answer, as when pi hangs up, it's
            // counted as cancelled here; judged, it's by when this attempt went.
            let on_the_way = OnTheWay::sent(&tally, &self.counts);
            (went, down) = (on_the_way.sent, (at, number, tally));
            let tried = sender.try_send_request(request).await;
            let mut unread = None;
            let failed = match tried {
                Ok(answer) => Ok(answer),
                Err(mut failed) => {
                    unread = failed.take_message();
                    Err(failed)
                }
            };
            // Gone to the origin, it's said as sent again, answered or not.
            if let Some((was_at, was)) = reset_on
                && unread.is_none()
            {
                eprintln!(
                    "model-relay: a request reset on connection {was_at}.{was} is sent again \
                     on connection {at}.{number}"
                );
            }
            match failed {
                Ok(answer) => {
                    if reset_on.is_some() {
                        self.counts.again.fetch_add(1, Ordering::Relaxed);
                    }
                    break Ok((answer, on_the_way));
                }
                Err(failed) => {
                    if unread.is_some() {
                        on_the_way.unsent();
                    } else {
                        on_the_way.end();
                    }
                    match unread {
                        // Its connection was retired before the request
                        // went, so it goes again, on a connection still open.
                        Some(unread) if sent < SENT_AGAIN_AT_MOST => {
                            sent += 1;
                            request = unread;
                            // Reset already, it never goes back to the
                            // connection that reset it, which `sender`
                            // could hand it.
                            taken = match reset_on {
                                Some((at, number)) => {
                                    let waiting = WaitingAgain::new(&self.counts);
                                    let another = self.another(at, number).await;
                                    waiting.over();
                                    another
                                }
                                None => self.sender().await,
                            };
                        }
                        // Reset before its answer began, saying the origin
                        // may be asked again, so it goes once more, on
                        // another connection, if there's one it may go on.
                        None if reset_on.is_none() && goes_once_more(failed.error()) => {
                            let retiring = self.slots[at].retire(number);
                            // Said before any wait for a connection, so a
                            // pi that hangs up meanwhile leaves it said.
                            Relay::said_reset(
                                failed.error(),
                                &down,
                                &retiring,
                                "it goes once more, if another connection will take it",
                            );
                            reset_on = Some((at, number));
                            request = sendable(parts, whole);
                            let waiting = WaitingAgain::new(&self.counts);
                            taken = self.another(at, number).await;
                            waiting.over();
                        }
                        _ => break Err(failed.into_error()),
                    }
                }
            }
        };
        // Answered, it's judged once its answer ends, as it may break off.
        if answered.as_ref().is_err_and(|why| fails_the_path(why)) {
            self.finished(went, true);
        }
        answered.map_err(|why| Box::new(self.not_carried(&why, &down, reset_on)))
    }

    /// A connection for a request reset on slot `at`'s `number` to go once
    /// more on, or why there's none: `fresh`'s, refused when it has none.
    async fn another(&self, at: usize, number: u64) -> Result<(usize, Kept), Unreachable> {
        self.fresh(at, number).await.unwrap_or_else(|| {
            // Said, so a journal of these isn't read as an origin refusing
            // connections, when the relay chose to open none.
            eprintln!(
                "model-relay: slot {at} kept connection {at}.{number}, as it retired one \
                 lately, and no other is open, so none is opened for a request it reset"
            );
            Err(Unreachable::Refused)
        })
    }

    /// What pi is answered for a request that went `down` a connection and
    /// came back with no answer, and won't be sent again; `reset_on` is the
    /// connection that reset it before, if one did and it was sent again.
    fn not_carried(
        &self,
        why: &hyper::Error,
        down: &Down,
        reset_on: Option<(usize, u64)>,
    ) -> Response<Body> {
        // hyper's error says what failed, never what was sent, which a
        // test holds it to.
        self.counts.failed.fetch_add(1, Ordering::Relaxed);
        if reset_by_the_origin(why) {
            // It's a 502 rather than a 421 sending pi direct, since only a
            // reset saying the stream was refused promises the origin acted
            // on nothing, and these said a protocol error.
            let (at, number, _) = down;
            let retiring = self.slots[*at].retire(*number);
            Relay::said_reset(why, down, &retiring, "pi is answered 502");
            self.counts.reset.fetch_add(1, Ordering::Relaxed);
        } else if let Some((was_at, was)) = reset_on {
            // Reset, then lost some other way when it went again, so it's
            // still counted with the requests reset that failed.
            self.counts.reset.fetch_add(1, Ordering::Relaxed);
            eprintln!(
                "model-relay: a request reset on connection {was_at}.{was} couldn't be \
                 carried again: {}",
                with_causes(why)
            );
        } else {
            eprintln!(
                "model-relay: a request couldn't be carried: {}",
                with_causes(why)
            );
        }
        not_answered()
    }

    /// Says the origin reset a request on connection `at`'s `number`, what
    /// `retiring` it did to the connection, and what came of the request.
    fn said_reset(why: &hyper::Error, (at, number, tally): &Down, retiring: &Retiring, then: &str) {
        // On 18 November the origin reset 1,545 of 24,218 requests, each
        // within a second of its asking, in spells of minutes that took 10
        // to 15% of every session's: the shape one connection of several
        // refusing all it's given would make. Retired, such a connection
        // costs one request. The number says whether a fresh connection is
        // reset too, which one bad one wouldn't be, and the tally whether
        // the connections reset carried more of pi's cancels than those
        // left be.
        let done = retiring.said();
        eprintln!(
            "model-relay: the origin reset a request on connection {at}.{number}, {done}, \
             {}: {}; {then}",
            tally.said(),
            with_causes(why)
        );
    }

    /// A connection other than slot `at`'s `number`, which the origin reset
    /// a request on, for the request to go once more: one opened in the
    /// slot since, another slot's that's open, or, with the reset one
    /// retired, one opened for it. None when the slot kept the reset one,
    /// as it retired one lately, and no other is open: opening one over it
    /// would undo that allowance. A burst of requests reset on one
    /// connection shares the one opened for the first, and when that
    /// attempt fails, those that waited on it hear so, as `open` has it.
    async fn fresh(&self, at: usize, number: u64) -> Option<Result<(usize, Kept), Unreachable>> {
        let waited_from = Instant::now();
        // Looked for before the slot's lock, which an open behind holds
        // while it connects, and again once it's held.
        match self.open_for_again(at, number) {
            ForAgain::Open(slot, kept) => return Some(Ok((slot, kept))),
            ForAgain::Kept => return None,
            ForAgain::Closed => {}
        }
        let _opening = self.slots[at].opening.lock().await;
        match self.open_for_again(at, number) {
            ForAgain::Open(slot, kept) => return Some(Ok((slot, kept))),
            ForAgain::Kept => return None,
            ForAgain::Closed => {}
        }
        let failed = *self.failed.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((when, why)) = failed
            && when >= waited_from
        {
            return Some(Err(why));
        }
        Some(self.open_now(at).await.map(|kept| (at, kept)))
    }

    /// What's open for a request reset on slot `at`'s `number` to go once
    /// more on, as `fresh` has it.
    fn open_for_again(&self, at: usize, number: u64) -> ForAgain {
        let kept = self.slots[at].sender();
        if let Some(kept) = &kept
            && kept.number > number
        {
            return ForAgain::Open(at, kept.clone());
        }
        match (self.open_beside(at), kept) {
            (Some((slot, other)), _) => ForAgain::Open(slot, other),
            (None, Some(_)) => ForAgain::Kept,
            (None, None) => ForAgain::Closed,
        }
    }

    /// A connection open in a slot other than `at`, the next after it first.
    fn open_beside(&self, at: usize) -> Option<(usize, Kept)> {
        let count = self.slots.len();
        (1..count)
            .map(|step| (at + step) % count)
            .find_map(|other| self.slots[other].sender().map(|kept| (other, kept)))
    }

    /// A connection to send down: the next in turn if it's open, or any
    /// other that is while the next is opened behind it, or, with none
    /// open, the next not being opened already, once it's opened.
    async fn sender(self: &Arc<Relay>) -> Result<(usize, Kept), Unreachable> {
        self.retire_after_a_sleep();
        let count = self.slots.len();
        let at = self.next.fetch_add(1, Ordering::Relaxed) % count;
        if let Some(kept) = self.slots[at].sender() {
            return Ok((at, kept));
        }
        if let Some(taken) = self.open_beside(at) {
            self.open_behind(at);
            return Ok(taken);
        }
        // One being opened behind may be stuck on an origin that takes
        // no more connections, and another may open at once.
        let free = (0..count)
            .map(|step| (at + step) % count)
            .find(|&slot| self.slots[slot].opening.try_lock().is_ok())
            .unwrap_or(at);
        self.open(free).await.map(|kept| (free, kept))
    }

    /// Retires every connection when the wall clock ran ahead of the
    /// relay's own since it was last asked or heard, as it does while the machine
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
        self.retire_every();
        if self.standing.forget(now.0) {
            eprintln!("model-relay: the machine slept, so it no longer stands down");
        }
    }

    /// Retires every connection, so each is opened afresh when next asked
    /// for. An answer already streaming on one carries on.
    fn retire_every(&self) {
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
    async fn open(&self, at: usize) -> Result<Kept, Unreachable> {
        let waited_from = Instant::now();
        let _opening = self.slots[at].opening.lock().await;
        if let Some(kept) = self.slots[at].sender() {
            return Ok(kept);
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
    async fn open_now(&self, at: usize) -> Result<Kept, Unreachable> {
        let number = self.slots[at].next_number();
        let tally = Arc::new(Tally::of(at, number));
        match self.connect(at, number).await {
            Ok(sender) => {
                let kept = self.slots[at].keep(number, sender, tally);
                // The origin is back, so others may be opened behind.
                *self.failed.lock().unwrap_or_else(PoisonError::into_inner) = None;
                Ok(kept)
            }
            Err(why) => {
                *self.failed.lock().unwrap_or_else(PoisonError::into_inner) =
                    Some((Instant::now(), why));
                Err(why)
            }
        }
    }

    /// Opens connection `at` to the origin, saying so by its slot and its
    /// `number` there, and says when it's retired.
    async fn connect(&self, at: usize, number: u64) -> Result<SendRequest<Sent>, Unreachable> {
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
            let name = ServerName::try_from(self.origin.host.clone())
                .map_err(|why| (io::Error::other(why), Unreachable::Unfit))?;
            let held_ahead = |tls: &mut rustls::ClientConnection| {
                tls.set_buffer_limit(Some(HELD_AHEAD_AT_MOST));
            };
            let tls = self
                .tls
                .connect_with(name, tcp, held_ahead)
                .await
                .map_err(|why| {
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
                eprintln!(
                    "model-relay: connection {at} couldn't be made: {}",
                    with_causes(&why)
                );
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
        eprintln!("model-relay: connection {at}.{number} opened, {open} taking requests");
        let counts = Arc::clone(&self.counts);
        tokio::spawn(async move {
            // This ends once the connection takes no new requests, which
            // after the origin's goodbye may be before its last answers
            // have streamed: so the connection is said as retired, not
            // closed, and counted as no longer taking requests.
            let ended = connection.await;
            let open = counts.open.fetch_sub(1, Ordering::Relaxed) - 1;
            match ended {
                Ok(()) => {
                    eprintln!(
                        "model-relay: connection {at}.{number} retired, \
                         {open} taking requests"
                    );
                }
                Err(why) => {
                    eprintln!(
                        "model-relay: connection {at}.{number} retired on {}, \
                         {open} taking requests",
                        with_causes(&why)
                    );
                }
            }
        });
        Ok(sender)
    }
}

/// Whether the origin reset the request's stream rather than answer it,
/// as hyper says through the h2 error beneath its own.
fn reset_by_the_origin(why: &hyper::Error) -> bool {
    reset_saying(why).is_some()
}

/// Whether the origin reset a request saying it may be asked again.
fn goes_once_more(why: &hyper::Error) -> bool {
    reset_saying(why).is_some_and(goes_again)
}

/// Whether a reset giving `reason` is one to send again: its stream
/// refused, which promises the origin did nothing, or a protocol error, as
/// on 18 November. An error of the origin's own may come after its work
/// was done and billed, and one saying to calm down, or that it cancelled
/// the request, isn't one to send again at once.
fn goes_again(reason: h2::Reason) -> bool {
    [h2::Reason::REFUSED_STREAM, h2::Reason::PROTOCOL_ERROR].contains(&reason)
}

/// Whether a request failing with `why` failed for the relay's path to
/// the origin, and so is judged: its connection lost, or the origin
/// resetting it or saying goodbye with a refused stream or a protocol
/// error, as on 18 November. The origin's own verdict, such as to calm
/// down or an error of its own, says nothing of the path, as it would
/// fail direct too. What hyper or h2 raise themselves is judged, whether
/// a connection's task gone, which hyper calls the user's error, or h2
/// refusing to send what was asked: the request's body is whole in
/// memory and can't fail, and pi direct speaks HTTP/1.1, so it wouldn't
/// meet h2's refusals, and a stand-down sending it there is the cure.
fn fails_the_path(why: &(dyn std::error::Error + 'static)) -> bool {
    let mut beneath = Some(why);
    while let Some(cause) = beneath {
        if let Some(h2) = cause.downcast_ref::<h2::Error>() {
            return match h2.reason() {
                Some(reason) if h2.is_remote() => goes_again(reason),
                _ => true,
            };
        }
        beneath = cause.source();
    }
    true
}

/// The reason the origin gave for resetting the request's stream, when it
/// reset it, found in the h2 error beneath hyper's.
fn reset_saying(why: &hyper::Error) -> Option<h2::Reason> {
    let mut beneath: Option<&(dyn std::error::Error + 'static)> = Some(why);
    while let Some(cause) = beneath {
        if let Some(h2) = cause.downcast_ref::<h2::Error>() {
            return if h2.is_reset() && h2.is_remote() {
                h2.reason()
            } else {
                None
            };
        }
        beneath = cause.source();
    }
    None
}

/// Readies a connection to the origin: each frame goes as it's written,
/// and the kernel holds no more than `HELD_AHEAD_AT_MOST` of them unsent.
fn fit(tcp: &TcpStream) -> io::Result<()> {
    tcp.set_nodelay(true)?;
    let unsent = u32::try_from(HELD_AHEAD_AT_MOST).map_err(io::Error::other)?;
    socket2::SockRef::from(tcp).set_tcp_notsent_lowat(unsent)
}

/// `why`, with every error beneath it, since hyper says only the kind of
/// failure on top ("http2 error") and which one beneath ("keep-alive timed
/// out", or the origin's goodbye). None of them holds what was sent.
fn with_causes(why: &dyn std::error::Error) -> String {
    let mut said = why.to_string();
    let mut beneath = why.source();
    while let Some(cause) = beneath {
        said.push_str(": ");
        said.push_str(&cause.to_string());
        beneath = cause.source();
    }
    if said.len() > SAID_AT_MOST {
        let mut end = SAID_AT_MOST;
        while !said.is_char_boundary(end) {
            end -= 1;
        }
        said.truncate(end);
        said.push_str("...");
    }
    said
}

/// A TCP connection to `host`'s `port`, from the first address the name
/// resolves to that answers.
async fn reach(host: &str, port: u16, within: Duration) -> io::Result<TcpStream> {
    let addresses: Vec<SocketAddr> = tokio::net::lookup_host((host, port)).await?.collect();
    if addresses.is_empty() {
        return Err(io::Error::other(format!("{host} resolved to no address")));
    }
    let tcp = reach_any(&addresses, within).await?;
    fit(&tcp)?;
    Ok(tcp)
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
/// clock does; a wall clock set back is no sleep. One stepped forward
/// reads as one, ending a stand-down early, which costs a return judged
/// afresh.
fn slept(then: (Instant, SystemTime), now: (Instant, SystemTime)) -> bool {
    let on_the_wall = now.1.duration_since(then.1).unwrap_or_default();
    on_the_wall > now.0.duration_since(then.0) + SLEPT_AT_LEAST
}

/// An answer of the relay's own: a status and a line saying why.
///
/// A 421 of the relay's own is said only when nothing of the request
/// went to the origin, as pi sends a request the relay answers with one
/// straight to the origin itself. pi knows one by its whole body being
/// one line, "this relay forwards to its own origin alone", or one that
/// begins "this relay " and ends "so go directly", so a new one is
/// worded so. A refusal once something went is a 5xx.
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

/// Where a request went: the slot, the connection's number there, and
/// the tally that counts what the connection carried.
type Down = (usize, u64, Arc<Tally>);

/// The request `parts` names with its body `whole`, made afresh each time
/// it goes, since sending one hands it to hyper.
fn sendable(parts: &hyper::http::request::Parts, whole: &Bytes) -> Request<Sent> {
    let mut request = Request::new(InPieces::new(Full::new(whole.clone())));
    *request.method_mut() = parts.method.clone();
    *request.uri_mut() = parts.uri.clone();
    *request.version_mut() = parts.version;
    *request.headers_mut() = parts.headers.clone();
    request
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
    // hyper drops a TE spelt any way but "trailers" exactly before h2
    // sees it, so one kept is said that way, or it would be lost.
    if only_trailers && headers.contains_key(header::TE) {
        headers.insert(header::TE, header::HeaderValue::from_static("trailers"));
    } else {
        headers.remove(header::TE);
    }
}

/// Why a request's body wasn't read whole.
#[derive(Clone, Copy, Debug)]
enum Unread {
    /// It passed `BODY_AT_MOST`.
    TooLarge,
    /// It was still coming after `BODY_WITHIN`.
    Late,
    /// pi went before it ended.
    Gone,
}

/// Reads a request's body whole, so it can go again. pi's requests carry
/// no trailers to keep.
async fn read_whole<B>(body: B) -> Result<Bytes, Unread>
where
    B: hyper::body::Body<Data = Bytes>,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    let reading = tokio::time::timeout(BODY_WITHIN, Limited::new(body, BODY_AT_MOST).collect());
    match reading.await {
        Ok(Ok(read)) => Ok(read.to_bytes()),
        Ok(Err(why)) if why.is::<LengthLimitError>() => Err(Unread::TooLarge),
        Err(_) => Err(Unread::Late),
        Ok(Err(_)) => Err(Unread::Gone),
    }
}

/// Says the counts every `COUNTS_EVERY`, when any has moved since.
async fn say_counts(counts: Arc<Counts>) {
    loop {
        tokio::time::sleep(COUNTS_EVERY).await;
        counts.say();
    }
}

/// What pi is answered for a request the origin took and didn't answer: a
/// 502, for pi's own retry.
fn not_answered() -> Response<Body> {
    said(
        StatusCode::BAD_GATEWAY,
        "this relay's request to its origin failed\n",
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant, SystemTime};

    use bytes::Bytes;
    use http_body_util::{BodyExt, Full};
    use hyper::body::Body as _;

    use super::{
        BODY_WITHIN, HELD_AHEAD_AT_MOST, InPieces, Origin, Retiring, SAID_AT_MOST, SLEPT_AT_LEAST,
        Sender, Slot, Unread, fails_the_path, goes_again, reach, reach_any, read_whole, slept,
        with_causes,
    };

    #[tokio::test(start_paused = true)]
    async fn a_body_still_coming_past_its_deadline_is_late_and_one_pi_left_is_gone() {
        // A piece halfway, and the rest never, so a deadline on the whole
        // body is told from one on each piece.
        let (mut writing, body) = http_body_util::channel::Channel::<Bytes, std::io::Error>::new(1);
        let coming = tokio::spawn(async move {
            tokio::time::sleep(BODY_WITHIN / 2).await;
            let _ = writing.send_data(Bytes::from_static(b"hi")).await;
            // Held open for good, since letting go would end the body.
            std::future::pending::<()>().await;
        });
        let began = tokio::time::Instant::now();
        // Bounded itself, so a read_whole with no deadline fails here
        // rather than leaving paused time with nothing to wake it.
        let read = tokio::time::timeout(BODY_WITHIN * 2, read_whole(body))
            .await
            .expect("read_whole waited on a body with no deadline");
        let waited = began.elapsed();
        assert!(
            matches!(read, Err(Unread::Late))
                && (BODY_WITHIN..BODY_WITHIN + Duration::from_millis(5)).contains(&waited),
            "a body still coming past its deadline was read as {read:?} after {waited:?}"
        );
        coming.abort();

        let (writing, body) = http_body_util::channel::Channel::<Bytes, std::io::Error>::new(1);
        writing.abort(std::io::Error::from(std::io::ErrorKind::BrokenPipe));
        let read = read_whole(body).await;
        assert!(
            matches!(read, Err(Unread::Gone)),
            "a body pi left was read as {read:?}"
        );
    }

    #[test]
    fn only_a_reset_refusing_its_stream_or_saying_a_protocol_error_goes_again() {
        for reason in [h2::Reason::REFUSED_STREAM, h2::Reason::PROTOCOL_ERROR] {
            assert!(
                goes_again(reason),
                "a reset saying {reason:?} wasn't sent again"
            );
        }
        for reason in [
            h2::Reason::NO_ERROR,
            h2::Reason::INTERNAL_ERROR,
            h2::Reason::CANCEL,
            h2::Reason::ENHANCE_YOUR_CALM,
            h2::Reason::STREAM_CLOSED,
            h2::Reason::FLOW_CONTROL_ERROR,
        ] {
            assert!(
                !goes_again(reason),
                "a reset saying {reason:?} was sent again"
            );
        }
    }

    /// A connection kept for a test, closed only when the test closes it.
    #[derive(Clone, Default)]
    struct Line(Arc<AtomicBool>);

    impl Line {
        /// Closes it, as the origin's goodbye would.
        fn close(&self) {
            self.0.store(true, Ordering::Relaxed);
        }
    }

    impl Sender for Line {
        fn is_closed(&self) -> bool {
            self.0.load(Ordering::Relaxed)
        }
    }

    #[test]
    fn a_reset_retires_only_the_connection_still_taking_requests() {
        let slot = Slot::new();
        let reset = slot.keep(slot.next_number(), Line::default(), Arc::default());
        let since = slot.keep(slot.next_number(), Line::default(), Arc::default());
        assert_eq!(
            slot.retire(reset.number),
            Retiring::Gone,
            "a reset on a connection no longer kept wasn't said as one"
        );
        assert_eq!(
            slot.sender().map(|kept| kept.number),
            Some(since.number),
            "a reset that came late retired the connection opened since it"
        );
        assert_eq!(
            slot.retire(since.number),
            Retiring::Retired,
            "a connection's own reset didn't retire it"
        );
        assert!(
            slot.sender().is_none(),
            "a connection its own reset retired is still taking requests"
        );
        assert_eq!(
            slot.retire(since.number),
            Retiring::Gone,
            "a second reset on a connection already retired was said as kept"
        );
    }

    #[test]
    fn a_reset_on_a_connection_the_origin_closed_spends_no_allowance() {
        let slot = Slot::new();
        let closed = Line::default();
        let kept = slot.keep(slot.next_number(), closed.clone(), Arc::default());
        closed.close();
        assert_eq!(
            slot.retire(kept.number),
            Retiring::Gone,
            "a reset on a connection the origin closed wasn't said as gone"
        );
        let fresh = slot.keep(slot.next_number(), Line::default(), Arc::default());
        assert_eq!(
            slot.retire(fresh.number),
            Retiring::Retired,
            "a closed connection's reset spent the slot's allowance for a bad one"
        );
    }

    #[test]
    fn a_slot_retires_a_connection_for_a_reset_at_most_so_often() {
        let slot = Slot::new();
        let first = slot.keep(slot.next_number(), Line::default(), Arc::default());
        assert_eq!(
            slot.retire(first.number),
            Retiring::Retired,
            "a connection's first reset didn't retire it"
        );
        let fresh = slot.keep(slot.next_number(), Line::default(), Arc::default());
        assert_eq!(
            slot.retire(fresh.number),
            Retiring::Kept,
            "a fresh connection was retired at once, so resets everywhere would open one each"
        );
        assert_eq!(slot.sender().map(|kept| kept.number), Some(fresh.number));
    }

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

    #[tokio::test(flavor = "current_thread")]
    async fn a_connection_to_the_origin_holds_few_bytes_unsent() {
        let origin = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a port");
        let port = origin.local_addr().expect("its address").port();

        let tcp = reach("127.0.0.1", port, Duration::from_secs(2))
            .await
            .expect("a connection");

        assert_eq!(
            socket2::SockRef::from(&tcp)
                .tcp_notsent_lowat()
                .ok()
                .map(|lowat| lowat as usize),
            Some(HELD_AHEAD_AT_MOST),
            "a ping would wait behind whatever the kernel holds unsent"
        );
        assert_eq!(
            tcp.nodelay().ok(),
            Some(true),
            "a small frame would wait on the one before"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn a_body_goes_on_in_pieces_no_longer_than_a_frame_may_be() {
        let whole: Vec<u8> = (0..=u8::MAX)
            .cycle()
            .take(2 * HELD_AHEAD_AT_MOST + 99)
            .collect();
        let mut body = InPieces::new(Full::new(Bytes::from(whole.clone())));
        let mut pieces = Vec::new();
        let mut sent = Vec::new();
        let mut ended = Vec::new();

        while let Some(frame) = body.frame().await {
            let piece = frame.expect("a frame").into_data().expect("data");
            pieces.push(piece.len());
            sent.extend_from_slice(&piece);
            ended.push(body.is_end_stream());
        }

        assert_eq!(
            pieces,
            [HELD_AHEAD_AT_MOST, HELD_AHEAD_AT_MOST, 99],
            "a frame would hold more ahead of a ping"
        );
        assert_eq!(
            ended,
            [false, false, true],
            "h2 would end the stream with pieces still held, or never end it"
        );
        assert!(sent == whole, "the body would change on its way");
    }

    #[test]
    fn an_error_is_said_with_what_lies_beneath_it() {
        let beneath = std::io::Error::other(Beneath(
            "keep-alive timed out",
            std::io::Error::other("no answer"),
        ));
        let why = std::io::Error::other(Beneath("http2 error", beneath));

        assert_eq!(
            with_causes(&why),
            "http2 error: keep-alive timed out: no answer",
            "the journal would say only the kind of failure, never which"
        );
    }

    #[test]
    fn an_error_says_no_more_than_a_line_may_hold() {
        // Two bytes a character, so the cut has to find a character's edge.
        let goodbye = "\u{e9}".repeat(SAID_AT_MOST);
        let why = std::io::Error::other(Beneath("http2 error", std::io::Error::other(goodbye)));

        let said = with_causes(&why);

        assert!(
            said.len() <= SAID_AT_MOST + "...".len(),
            "the origin's goodbye would fill the journal: {} bytes",
            said.len()
        );
        assert!(
            said.starts_with("http2 error: \u{e9}") && said.ends_with("\u{e9}..."),
            "the line would lose the error's kind, or not say it was cut: {said}"
        );
        assert!(
            said.len() > SAID_AT_MOST - "\u{e9}".len(),
            "the line would keep less than it may: {} bytes",
            said.len()
        );
    }

    /// The socket lost or a reason h2 gives itself fails the path, beneath
    /// hyper's error or alone. The origin's own verdicts come in a frame
    /// from it, which only the relay's tests can send, and h2's refusals
    /// can't be built outside it.
    #[test]
    fn what_the_socket_and_h2_raise_fails_the_path() {
        let lost = std::io::Error::from(std::io::ErrorKind::BrokenPipe);
        assert!(fails_the_path(&lost), "a socket lost wasn't judged");
        for reason in [
            h2::Reason::PROTOCOL_ERROR,
            h2::Reason::CANCEL,
            h2::Reason::INTERNAL_ERROR,
        ] {
            let raised = Beneath("http2 error", h2::Error::from(reason));
            assert!(fails_the_path(&raised), "h2's own {reason:?} wasn't judged");
        }
    }

    /// A connection's task gone with a request in hand fails the path,
    /// though hyper calls it the user's error.
    #[tokio::test(flavor = "current_thread")]
    async fn a_connections_task_gone_fails_the_path() {
        use tokio::io::AsyncReadExt as _;

        let (near, mut far) = tokio::io::duplex(HELD_AHEAD_AT_MOST);
        let (mut sender, connection) = hyper::client::conn::http1::handshake::<_, Full<Bytes>>(
            hyper_util::rt::TokioIo::new(near),
        )
        .await
        .expect("a connection");
        let task = tokio::spawn(connection);
        let asked = sender.send_request(
            hyper::Request::get("/")
                .header(hyper::header::HOST, "origin")
                .body(Full::new(Bytes::new()))
                .expect("a request"),
        );
        // Its head written, so the connection's task holds the request.
        let mut piece = [0; 64];
        let read = far.read(&mut piece).await.expect("the request's head");
        assert!(read > 0, "the connection's task wrote nothing");
        task.abort();

        let why = asked
            .await
            .expect_err("a request answered once its connection's task went");
        assert!(
            why.is_user(),
            "hyper no longer calls it the user's error: {why:?}"
        );
        assert!(
            fails_the_path(&why),
            "a connection's task gone wasn't judged: {why:?}"
        );
    }

    /// An error with one beneath it, as hyper's are.
    #[derive(Debug)]
    struct Beneath<E>(&'static str, E);

    impl<E> std::fmt::Display for Beneath<E> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(self.0)
        }
    }

    impl<E: std::error::Error + 'static> std::error::Error for Beneath<E> {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.1)
        }
    }
}
