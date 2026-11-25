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
const ALLOWED: Duration = Duration::from_secs(35);

/// How long each read waits before the deadline is looked at again.
const TICK: Duration = Duration::from_millis(500);

/// Why a key wasn't sent.
#[derive(Debug, PartialEq)]
pub enum Why {
    /// Nothing answered at the TV's address: it's asleep, off or gone.
    Unreachable(String),
    /// Something answered with a certificate other than the one pinned:
    /// not the TV this machine was allowed by, or the TV was reset.
    NotTheTv,
    /// The TV said no, or nobody pressed Allow before it gave up asking.
    NotAllowed,
    /// The TV answered, then broke off or said something else: why.
    Broke(String),
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

/// Waits on `socket` until the TV says the channel is open, then presses
/// `code`, answering the token the TV handed, if it handed a new one. A
/// read that times out is waited past until `until`, so the socket's own
/// read timeout sets only how often the deadline is looked at.
pub fn talk<S: Read + Write>(
    socket: &mut WebSocket<S>,
    code: &str,
    until: Instant,
) -> Result<Option<String>, Why> {
    let token = loop {
        if Instant::now() >= until {
            return Err(Why::NotAllowed);
        }
        match socket.read() {
            Ok(Message::Text(text)) => match told(text.as_str()) {
                Told::Connected { token } => break token,
                Told::Unauthorized | Told::TimedOut => return Err(Why::NotAllowed),
                Told::Other => {}
            },
            Ok(Message::Close(_)) => return Err(Why::Broke("it closed the channel".into())),
            Ok(_) => {}
            Err(tungstenite::Error::Io(error)) if waited(&error) => {}
            Err(error) => return Err(Why::Broke(error.to_string())),
        }
    };
    socket
        .send(Message::text(press_message(code)))
        .map_err(|error| Why::Broke(error.to_string()))?;
    // The TV takes the key as it reads it, so a close it doesn't answer
    // costs nothing but the wait.
    let _ = socket.close(None);
    let _ = socket.flush();
    Ok(token)
}

/// Whether a read stopped only because its timeout passed.
fn waited(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
    )
}

/// Presses `code` on the TV at `address`, as `asker`, carrying what was
/// kept of it, if anything was. Answers what to keep of it afterwards:
/// its pin, and its token, the new one where it handed one.
pub fn press(address: IpAddr, asker: &str, code: &str, kept: Option<&Kept>) -> Result<Kept, Why> {
    let pinned = Arc::new(Pinned::new(kept.map(|kept| kept.pin)));
    let stream = TcpStream::connect_timeout(&SocketAddr::new(address, PORT), REACH)
        .map_err(|error| Why::Unreachable(error.to_string()))?;
    stream
        .set_read_timeout(Some(TICK))
        .and_then(|()| stream.set_write_timeout(Some(REACH)))
        .map_err(|error| Why::Broke(error.to_string()))?;
    let name = ServerName::IpAddress(address.into());
    let connection = ClientConnection::new(configured(Arc::clone(&pinned))?, name)
        .map_err(|error| Why::Broke(error.to_string()))?;
    let tls = StreamOwned::new(connection, stream);
    let token = kept.and_then(|kept| kept.token.as_deref());
    let request = url(address, asker, token)
        .into_client_request()
        .map_err(|error| Why::Broke(error.to_string()))?;
    let until = Instant::now() + ALLOWED;
    let opened = tungstenite::client::client(request, tls);
    let seen = pinned.seen();
    let mut socket = match opened {
        Ok((socket, _)) => socket,
        Err(_) if pinned.refused() => return Err(Why::NotTheTv),
        Err(error) => return Err(Why::Broke(error.to_string())),
    };
    let pin = seen.ok_or_else(|| Why::Broke("it showed no certificate".into()))?;
    let handed = talk(&mut socket, code, until)?;
    Ok(Kept {
        token: handed.or_else(|| token.map(str::to_owned)),
        pin,
    })
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
