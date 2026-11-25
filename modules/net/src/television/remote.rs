//! A Samsung TV's remote keys, sent as Samsung's own phone app sends them:
//! over a WebSocket at `wss://<address>:8002/api/v2/channels/
//! samsung.remote.control`, the only way a TV saying `TokenAuthSupport`
//! takes them. The first time, the TV asks on its own screen whether to
//! allow the asker, and once Joel presses Allow it hands back a token,
//! which every later key carries so the TV doesn't ask again.
//!
//! The TV's certificate is its own, signed by nobody a machine trusts, so
//! it's pinned instead: the first answer's SHA-256 is kept with the
//! token, and a box at the TV's address answering with any other
//! certificate is refused before a key is sent.

use std::io::{self, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, ClientConnection, DigitallySignedStruct, SignatureScheme, StreamOwned};
use serde_json::{Value, json};
use tungstenite::client::IntoClientRequest;
use tungstenite::handshake::HandshakeError;
use tungstenite::{Message, WebSocket};

use super::kept::{Kept, Pin};

/// The port a Samsung TV takes remote keys on, over TLS.
pub const PORT: u16 = 8002;

/// The channel its remote keys go to.
const CHANNEL: &str = "/api/v2/channels/samsung.remote.control";

/// How long reaching the TV may take: it's on the home network, so a
/// TV that doesn't answer in this is asleep or off.
const REACH: Duration = Duration::from_secs(3);

/// How long the TV may take to say the channel is open. Its Allow prompt
/// waits about 30 s for a press on its remote, so a first key waits that
/// long; a TV that knows the token answers at once.
pub const ALLOWED: Duration = Duration::from_secs(35);

/// How long the TV has to answer the channel's close once the key is
/// sent: the close is WebSocket's own handshake, so the key is read
/// before the channel ends rather than lost in a reset.
const LINGER: Duration = Duration::from_secs(1);

/// Why a key wasn't sent.
#[derive(Debug, PartialEq)]
pub enum Why {
    /// Nothing answered at the TV's address: it's asleep, off or gone.
    Unreachable(String),
    /// Something answered with a certificate other than the one pinned:
    /// not the TV this machine was allowed by, or the TV was reset.
    NotTheTv,
    /// The TV refused an asker carrying no token: it was told Deny.
    Refused,
    /// The TV refused the token it handed: it forgot this machine, or
    /// was told Deny since. What it allowed is forgotten but its pin, so
    /// the next key asks Allow again of the same TV.
    Forgotten,
    /// Asked for the first time, the TV stopped asking before anybody
    /// pressed Allow, or said nothing before the deadline.
    Unanswered,
    /// The TV answered, then broke off or stopped answering: why. Trying
    /// again may reach it.
    Broke(String),
    /// What answered doesn't speak as a Samsung TV does, in TLS or in
    /// WebSocket: why. Trying again reaches the same.
    Unlike(String),
}

/// What a socket's reads and writes wait at most.
pub trait Waits {
    fn wait_reads(&self, longest: Duration) -> io::Result<()>;
    fn wait_writes(&self, longest: Duration) -> io::Result<()>;
}

impl Waits for TcpStream {
    fn wait_reads(&self, longest: Duration) -> io::Result<()> {
        self.set_read_timeout(Some(longest))
    }

    fn wait_writes(&self, longest: Duration) -> io::Result<()> {
        self.set_write_timeout(Some(longest))
    }
}

/// A socket held to one deadline: each read or write waits only what's
/// left of it, so neither a TV slow through the handshake nor one
/// trickling a byte at a time holds net past it. Once it passes, every
/// read and write fails as timed out without touching the socket.
#[derive(Debug)]
pub struct Due<S> {
    pub socket: S,
    pub until: Instant,
}

impl<S> Due<S> {
    fn left(&self) -> io::Result<Duration> {
        self.until
            .checked_duration_since(Instant::now())
            .filter(|left| !left.is_zero())
            .ok_or_else(|| io::Error::from(io::ErrorKind::TimedOut))
    }
}

/// A read or write that waited all that was left: its socket says it
/// would block, which here means the deadline came.
fn timed_out<T>(done: io::Result<T>) -> io::Result<T> {
    done.map_err(|error| match error.kind() {
        io::ErrorKind::WouldBlock => io::Error::from(io::ErrorKind::TimedOut),
        _ => error,
    })
}

impl<S: Read + Waits> Read for Due<S> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        let left = self.left()?;
        self.socket.wait_reads(left)?;
        timed_out(self.socket.read(into))
    }
}

impl<S: Write + Waits> Write for Due<S> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let left = self.left()?;
        self.socket.wait_writes(left)?;
        timed_out(self.socket.write(bytes))
    }

    fn flush(&mut self) -> io::Result<()> {
        let left = self.left()?;
        self.socket.wait_writes(left)?;
        timed_out(self.socket.flush())
    }
}

/// What the TV said as the channel opened.
#[derive(Debug, PartialEq)]
pub enum Told {
    /// The channel is open, with a token when the TV handed a new one.
    Connected { token: Option<String> },
    /// The TV refused the asker.
    Unauthorized,
    /// The TV stopped waiting for Allow.
    TimedOut,
    /// Anything else, as the TV's own news, which is read past.
    Other,
}

/// What the TV's message `text` says.
pub fn told(text: &str) -> Told {
    let Ok(said) = serde_json::from_str::<Value>(text) else {
        return Told::Other;
    };
    match said.get("event").and_then(Value::as_str) {
        Some("ms.channel.connect") => Told::Connected {
            token: said
                .pointer("/data/token")
                .and_then(Value::as_str)
                .filter(|token| plain(token))
                .map(str::to_owned),
        },
        Some("ms.channel.unauthorized") => Told::Unauthorized,
        Some("ms.channel.timeOut") => Told::TimedOut,
        _ => Told::Other,
    }
}

/// Whether a token is one a URL carries as it is: the TV's are digits.
fn plain(token: &str) -> bool {
    !token.is_empty() && token.len() <= 64 && token.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// Where the TV at `address` takes keys from `asker`, the name its Allow
/// prompt shows, with `token` once it has given one.
pub fn url(address: IpAddr, asker: &str, token: Option<&str>) -> String {
    let host = match address {
        IpAddr::V4(v4) => v4.to_string(),
        IpAddr::V6(v6) => format!("[{v6}]"),
    };
    let mut url = format!(
        "wss://{host}:{PORT}{CHANNEL}?name={}",
        base64(asker.as_bytes())
    );
    if let Some(token) = token.filter(|token| plain(token)) {
        url.push_str("&token=");
        url.push_str(token);
    }
    url
}

/// The message pressing the key Samsung calls `code`, as its app sends it.
pub fn press_message(code: &str) -> String {
    json!({
        "method": "ms.remote.control",
        "params": {
            "Cmd": "Click",
            "DataOfCmd": code,
            "Option": "false",
            "TypeOfRemote": "SendRemoteKey",
        },
    })
    .to_string()
}

/// Standard base64 with padding, as the TV reads the asker's name; the
/// URL-safe form's `-` and `_` it doesn't take, and `+`, `/` and `=` pass
/// in a query as they are.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let joined = chunk.iter().enumerate().fold(0u32, |all, (at, byte)| {
            all | u32::from(*byte) << (16 - 8 * at)
        });
        for at in 0..4 {
            if at <= chunk.len() {
                let index = (joined >> (18 - 6 * at)) & 0x3f;
                out.push(char::from(ALPHABET[index as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Why a key didn't go, and the token the TV handed before it failed,
/// which is kept all the same, since the TV holds it as given.
#[derive(Debug, PartialEq)]
pub struct Unsent {
    pub why: Why,
    pub handed: Option<String>,
}

impl From<Why> for Unsent {
    fn from(why: Why) -> Self {
        Unsent { why, handed: None }
    }
}

/// Waits on `socket` until the TV says the channel is open, then presses
/// `code`, answering the token the TV handed, if it handed a new one.
/// `carried` says whether the asker carried a token: only a TV asked for
/// the first time waits on a press of Allow, so only then is a silence
/// to the deadline a missing Allow, and a refusal a Deny. A read that
/// times out is waited past until `until`; a socket held to `until` by
/// `Due` fails each read past it, so no read outlasts it.
pub fn talk<S: Read + Write>(
    socket: &mut WebSocket<S>,
    code: &str,
    until: Instant,
    carried: bool,
) -> Result<Option<String>, Unsent> {
    let quiet = || {
        if carried {
            Why::Broke("it stopped answering".into())
        } else {
            Why::Unanswered
        }
    };
    let token = loop {
        if Instant::now() >= until {
            return Err(quiet().into());
        }
        match socket.read() {
            Ok(Message::Text(text)) => match told(text.as_str()) {
                Told::Connected { token } => break token,
                Told::Unauthorized if carried => return Err(Why::Forgotten.into()),
                Told::Unauthorized => return Err(Why::Refused.into()),
                Told::TimedOut => return Err(Why::Unanswered.into()),
                Told::Other => {}
            },
            Ok(Message::Close(_)) => {
                return Err(Why::Broke("it closed the channel".into()).into());
            }
            Ok(_) => {}
            Err(tungstenite::Error::Io(error)) if waited(&error) => {}
            Err(error) => return Err(broke(error).into()),
        }
    };
    match socket.send(Message::text(press_message(code))) {
        Ok(()) => Ok(token),
        Err(error) => Err(Unsent {
            why: broke(error),
            handed: token,
        }),
    }
}

/// Ends the channel as WebSocket does: says it's closing, then reads
/// until the TV answers its close, it hangs up or `until` passes.
pub fn close<S: Read + Write>(socket: &mut WebSocket<S>, until: Instant) {
    if socket.close(None).is_err() {
        return;
    }
    while Instant::now() < until {
        match socket.read() {
            Ok(_) => {}
            Err(tungstenite::Error::Io(error)) if waited(&error) => {}
            Err(_) => return,
        }
    }
}

/// Why the channel broke, as a retry would see it: the socket breaking
/// or going quiet may not happen again, but TLS or WebSocket spoken
/// otherwise than a TV speaks it will. rustls says its own errors as
/// invalid data.
pub fn broke(error: tungstenite::Error) -> Why {
    match error {
        tungstenite::Error::Io(error) if error.kind() == io::ErrorKind::InvalidData => {
            Why::Unlike(error.to_string())
        }
        tungstenite::Error::Io(error) if error.kind() == io::ErrorKind::TimedOut => {
            Why::Broke("it stopped answering".into())
        }
        tungstenite::Error::Io(error) => Why::Broke(error.to_string()),
        tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed => {
            Why::Broke("it closed the channel".into())
        }
        error => Why::Unlike(error.to_string()),
    }
}

/// Why the channel didn't open: a certificate other than the pinned one
/// before anything else, since rustls says that as any TLS failure.
fn not_opened(pinned: &Pinned, error: tungstenite::Error) -> Why {
    if pinned.refused() {
        Why::NotTheTv
    } else {
        broke(error)
    }
}

/// What to keep of the TV once a key went: the token it handed, or else
/// the one the key carried, and the pin its certificate gave.
pub fn kept_after(handed: Option<String>, carried: Option<&str>, pin: Pin) -> Kept {
    Kept {
        token: handed.or_else(|| carried.map(str::to_owned)),
        pin,
    }
}

/// A key that didn't go, and what to keep of the TV all the same, where
/// that differs from what the key carried.
#[derive(Debug, PartialEq)]
pub struct Missed {
    pub why: Why,
    pub kept: Option<Kept>,
}

impl From<Why> for Missed {
    fn from(why: Why) -> Self {
        Missed { why, kept: None }
    }
}

/// What to keep of the TV once a key didn't go: a token it handed first,
/// which it now expects; none but the pin once it refused the token
/// carried, so the next key asks Allow afresh of the same TV; and
/// otherwise nothing new.
pub fn kept_unsent(unsent: Unsent, carried: Option<&str>, pin: Pin) -> Missed {
    let kept = match (&unsent.why, unsent.handed) {
        (Why::Forgotten, _) => Some(Kept { token: None, pin }),
        (_, Some(handed)) => Some(kept_after(Some(handed), carried, pin)),
        (_, None) => None,
    };
    Missed {
        why: unsent.why,
        kept,
    }
}

/// Whether a read stopped only because its timeout passed.
fn waited(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
    )
}

/// The channel to the TV, over TLS on a socket held to one deadline.
type Channel<S> = WebSocket<StreamOwned<ClientConnection, Due<S>>>;

/// Opens the channel at `address` on `stream`, as `asker` carrying
/// `token`, through the TLS and WebSocket handshakes, held to `until`,
/// and answers it with the pin of the certificate the TV showed.
fn opened<S: Read + Write + Waits>(
    stream: S,
    address: IpAddr,
    asker: &str,
    token: Option<&str>,
    pinned: &Arc<Pinned>,
    until: Instant,
) -> Result<(Channel<S>, Pin), Why> {
    let name = ServerName::IpAddress(address.into());
    let connection = ClientConnection::new(configured(Arc::clone(pinned))?, name)
        .map_err(|error| Why::Unlike(error.to_string()))?;
    let tls = StreamOwned::new(
        connection,
        Due {
            socket: stream,
            until,
        },
    );
    let request = url(address, asker, token)
        .into_client_request()
        .map_err(|error| Why::Unlike(error.to_string()))?;
    let socket = match tungstenite::client::client(request, tls) {
        Ok((socket, _)) => socket,
        Err(HandshakeError::Failure(error)) => return Err(not_opened(pinned, error)),
        // A socket that would block is told as timed out by `Due`, so the
        // handshake is never left halfway; this is only for its type.
        Err(HandshakeError::Interrupted(_)) => {
            return Err(Why::Broke("it stopped answering".into()));
        }
    };
    let pin = pinned
        .seen()
        .ok_or_else(|| Why::Unlike("it showed no certificate".into()))?;
    Ok((socket, pin))
}

/// Presses `code` on the TV at `address`, as `asker`, carrying what was
/// kept of it, if anything was. Answers what to keep of it afterwards:
/// its pin, and its token, the new one where it handed one; a key that
/// didn't go says what to keep all the same.
pub fn press(
    address: IpAddr,
    asker: &str,
    code: &str,
    kept: Option<&Kept>,
) -> Result<Kept, Missed> {
    let pinned = Arc::new(Pinned::new(kept.map(|kept| kept.pin)));
    let stream = TcpStream::connect_timeout(&SocketAddr::new(address, PORT), REACH)
        .map_err(|error| Why::Unreachable(error.to_string()))?;
    // One deadline holds the TLS and WebSocket handshakes and the wait
    // for Allow alike, so a TV slow anywhere fails once, at the end.
    let until = Instant::now() + ALLOWED;
    let token = kept.and_then(|kept| kept.token.as_deref());
    let (mut socket, pin) = opened(stream, address, asker, token, &pinned, until)?;
    let handed = talk(&mut socket, code, until, token.is_some())
        .map_err(|unsent| kept_unsent(unsent, token, pin))?;
    let lingered = Instant::now() + LINGER;
    socket.get_mut().sock.until = lingered;
    close(&mut socket, lingered);
    Ok(kept_after(handed, token, pin))
}

/// TLS that trusts only the pinned certificate, or, with none pinned yet,
/// whatever the TV shows, which is then pinned.
fn configured(pinned: Arc<Pinned>) -> Result<Arc<ClientConfig>, Why> {
    let provider = Arc::clone(&pinned.provider);
    let config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|error| Why::Broke(error.to_string()))?
        .dangerous()
        .with_custom_certificate_verifier(pinned)
        .with_no_client_auth();
    Ok(Arc::new(config))
}

/// A certificate's pin: the SHA-256 of its whole DER.
pub fn pin_of(certificate: &[u8]) -> Pin {
    let digest = ring::digest::digest(&ring::digest::SHA256, certificate);
    let mut pin = [0; 32];
    pin.copy_from_slice(digest.as_ref());
    pin
}

/// What the TV's certificate is held to, and what it showed.
#[derive(Debug)]
struct Pinned {
    want: Option<Pin>,
    seen: Mutex<Option<Pin>>,
    provider: Arc<CryptoProvider>,
}

impl Pinned {
    fn new(want: Option<Pin>) -> Self {
        Pinned {
            want,
            seen: Mutex::new(None),
            provider: Arc::new(rustls::crypto::ring::default_provider()),
        }
    }

    fn seen(&self) -> Option<Pin> {
        *self.seen.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Whether the TV showed a certificate other than the pinned one.
    fn refused(&self) -> bool {
        matches!((self.want, self.seen()), (Some(want), Some(seen)) if want != seen)
    }
}

impl ServerCertVerifier for Pinned {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let pin = pin_of(end_entity.as_ref());
        *self.seen.lock().unwrap_or_else(PoisonError::into_inner) = Some(pin);
        match self.want {
            Some(want) if want != pin => Err(rustls::Error::General(
                "the certificate isn't the one pinned".into(),
            )),
            _ => Ok(ServerCertVerified::assertion()),
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signed: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(
            message,
            certificate,
            signed,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signed: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            certificate,
            signed,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

#[cfg(test)]
#[path = "remote_tests.rs"]
mod tests;
