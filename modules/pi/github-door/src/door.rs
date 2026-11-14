//! A session's socket: each tunnel a fenced client asks for, answered.
//!
//! A client asks for a tunnel with CONNECT, as a proxy is asked. To one
//! of GitHub's four hosts on 443, the door answers as that host, with a
//! certificate its session's authority signs (src/authority.rs), reads
//! the one request that comes, judges it (src/judge.rs), and sends what
//! it allows on to GitHub with the person's token in the placeholder's
//! place, passing GitHub's answer back untouched. To any other host it's
//! a plain tunnel, which it neither reads nor says anything of. Each
//! connection carries one request and is closed after its answer, so the
//! door never has to find where one answer ends.

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream, ToSocketAddrs};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, ClientConnection, RootCertStore, ServerConnection, StreamOwned};

use crate::authority::Authority;
use crate::cap::{self, Count, Counted};
use crate::http::{self, Head, Unread};
use crate::judge::{self, Host};
use crate::peer;
use crate::said::{Call, Refusal, Seen, Why};
use crate::token::{self, Secret, Token};

/// The most a request's body may be: far past any GraphQL document, and
/// past what a fetch's negotiation sends for a large repository.
const MOST_BODY: usize = 64 * 1024 * 1024;
/// The most clients a session's socket answers at once, each on a thread
/// of its own, so a command opening connections without end can't run
/// the door out of threads.
const MOST_CLIENTS: usize = 64;
/// How long a read may wait on a client or on GitHub before the door
/// gives up on it, so a stalled connection doesn't hold its thread.
const QUIET: Duration = Duration::from_secs(120);
/// How long reaching a host may take.
const REACH: Duration = Duration::from_secs(15);
/// The headers the door drops from a request: each says something of
/// this hop alone, or is one it sets itself.
const DROPPED: [&str; 12] = [
    "host",
    "authorization",
    "connection",
    "keep-alive",
    "proxy-connection",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
    "content-length",
    "expect",
];
/// Headers asking GitHub to read the method as another.
const OVERRIDES: [&str; 3] = [
    "x-http-method-override",
    "x-http-method",
    "x-method-override",
];

/// How the door reaches GitHub.
pub struct Upstream {
    client: Arc<ClientConfig>,
    /// Where a host is reached, when not where its name resolves: the
    /// tests' stand-in for GitHub.
    resolve: HashMap<String, SocketAddr>,
}

impl Upstream {
    /// GitHub, its certificates checked against the roots in `roots`, a
    /// PEM bundle, and each host in `resolve` reached where it says.
    pub fn new(roots: &[u8], resolve: HashMap<String, SocketAddr>) -> Result<Self, String> {
        let mut store = RootCertStore::empty();
        let mut read = 0;
        for der in rustls::pki_types::pem::PemObject::pem_slice_iter(roots) {
            let der: CertificateDer<'_> =
                der.map_err(|error| format!("a root won't read: {error}"))?;
            if store.add(der).is_ok() {
                read += 1;
            }
        }
        if read == 0 {
            return Err("no root certificate could be read".to_owned());
        }
        let client =
            ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_safe_default_protocol_versions()
                .map_err(|error| error.to_string())?
                .with_root_certificates(store)
                .with_no_client_auth();
        Ok(Upstream {
            client: Arc::new(client),
            resolve,
        })
    }

    /// A TCP connection to `host` on `port`.
    fn reach(&self, host: &str, port: u16) -> io::Result<TcpStream> {
        let addresses: Vec<SocketAddr> = match self.resolve.get(host) {
            Some(address) => vec![*address],
            None => (host, port).to_socket_addrs()?.collect(),
        };
        let mut last = io::Error::new(io::ErrorKind::NotFound, "the host has no address");
        for address in addresses {
            match TcpStream::connect_timeout(&address, REACH) {
                Ok(stream) => {
                    stream.set_read_timeout(Some(QUIET))?;
                    return Ok(stream);
                }
                Err(error) => last = error,
            }
        }
        Err(last)
    }

    /// A TLS connection to GitHub's `host`.
    fn connect(&self, host: Host) -> io::Result<StreamOwned<ClientConnection, TcpStream>> {
        let tcp = self.reach(host.name(), 443)?;
        let name = ServerName::try_from(host.name().to_owned()).map_err(io::Error::other)?;
        let connection =
            ClientConnection::new(Arc::clone(&self.client), name).map_err(io::Error::other)?;
        Ok(StreamOwned::new(connection, tcp))
    }
}

/// What every session's calls go through: the token, GitHub, and the
/// mutations let through.
pub struct Door {
    pub token: Token,
    pub upstream: Upstream,
    pub allowed: Vec<&'static str>,
}

/// One session: its authority, its count, and how many clients it's
/// answering.
pub struct Session {
    pub id: String,
    pub authority: Authority,
    count: Mutex<Count>,
    clients: AtomicUsize,
    /// Whether the session has been closed, which ends its socket's loop.
    pub closed: AtomicBool,
}

impl Session {
    /// A new session named `id`, with an authority of its own.
    pub fn new(id: &str) -> Result<Self, String> {
        Ok(Session {
            id: id.to_owned(),
            authority: Authority::new(id)?,
            count: Mutex::new(Count::new(Instant::now())),
            clients: AtomicUsize::new(0),
            closed: AtomicBool::new(false),
        })
    }

    /// Lift its cap for the rest of this hour.
    pub fn lift(&self) {
        self.count
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .lift();
    }
}

/// A client's place among a session's [`MOST_CLIENTS`], given back as it
/// goes.
struct Place(Arc<Session>);

impl Place {
    fn take(session: &Arc<Session>) -> Option<Self> {
        if session.clients.fetch_add(1, Ordering::SeqCst) >= MOST_CLIENTS {
            session.clients.fetch_sub(1, Ordering::SeqCst);
            return None;
        }
        Some(Place(Arc::clone(session)))
    }
}

impl Drop for Place {
    fn drop(&mut self) {
        self.0.clients.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Answer `session`'s socket until the session is closed.
pub fn serve(door: &Arc<Door>, session: &Arc<Session>, listener: &UnixListener) {
    for incoming in listener.incoming() {
        if session.closed.load(Ordering::SeqCst) {
            return;
        }
        match incoming {
            Ok(stream) => {
                let Some(place) = Place::take(session) else {
                    eprintln!(
                        "pi-github-door: busy session={} -- refused a client, since \
                         {MOST_CLIENTS} are answered already",
                        crate::said::plain(&session.id)
                    );
                    continue;
                };
                let door = Arc::clone(door);
                thread::spawn(move || client(&door, &place.0, stream));
            }
            Err(error) => eprintln!("pi-github-door: a client couldn't be taken: {error}"),
        }
    }
}

/// Answer one client's tunnel.
fn client(door: &Door, session: &Session, mut stream: UnixStream) {
    // Neither can fail on a socket that's open.
    let _ = stream.set_read_timeout(Some(QUIET));
    let _ = stream.set_write_timeout(Some(QUIET));
    let claimed = peer::claimed_session(&stream);
    let mut buffer = Vec::new();
    let head = match http::read_head(&mut stream, &mut buffer) {
        Ok(head) => head,
        Err(Unread::Gone) => return,
        Err(_) => {
            // Nothing a client can act on is lost: what it sent wasn't a request.
            let _ = http::write_answer(&mut stream, 400, "the GitHub door reads only CONNECT");
            return;
        }
    };
    let Some((host, port)) = tunnel_to(&head) else {
        let _ = http::write_answer(&mut stream, 400, "the GitHub door reads only CONNECT");
        return;
    };
    let Some(github) = Host::named(&host) else {
        tunnel(door, stream, &buffer, &host, port);
        return;
    };
    let call = Call {
        session: session.id.clone(),
        claimed,
        method: "CONNECT".to_owned(),
        host: github.name().to_owned(),
        seen: Seen::Nothing,
    };
    if port != 443 {
        let refusal = Refusal::new(
            Why::Port,
            format!("a tunnel to {host} on {port}; GitHub is reached on 443"),
        );
        eprintln!("{}", call.refused_line(&refusal));
        let _ = http::write_answer(&mut stream, refusal.why.status(), &said_to(&refusal));
        return;
    }
    if stream
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        .is_err()
    {
        return;
    }
    let config = match session.authority.answer_as(github) {
        Ok(config) => config,
        Err(why) => {
            eprintln!(
                "pi-github-door: no certificate for {} -- {why}",
                github.name()
            );
            return;
        }
    };
    let Ok(mut connection) = ServerConnection::new(config) else {
        return;
    };
    if !buffer.is_empty()
        && (connection.read_tls(&mut buffer.as_slice()).is_err()
            || connection.process_new_packets().is_err())
    {
        return;
    }
    let mut tls = StreamOwned::new(connection, stream);
    answer(door, session, &mut tls, github, call);
    tls.conn.send_close_notify();
    // The client may already be gone; there's nothing left to tell it.
    let _ = tls.flush();
    let _ = tls.sock.shutdown(Shutdown::Both);
}

/// The host and port a CONNECT asks for.
fn tunnel_to(head: &Head) -> Option<(String, u16)> {
    if head.method != "CONNECT" {
        return None;
    }
    let (host, port) = head.target.rsplit_once(':')?;
    let host = host.trim_start_matches('[').trim_end_matches(']');
    if host.is_empty() {
        return None;
    }
    Some((host.to_ascii_lowercase(), port.parse().ok()?))
}

/// A tunnel to a host that isn't GitHub's, untouched and unseen.
fn tunnel(door: &Door, mut client: UnixStream, early: &[u8], host: &str, port: u16) {
    let Ok(mut far) = door.upstream.reach(host, port) else {
        let _ = client.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n");
        return;
    };
    if client
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        .is_err()
        || far.write_all(early).is_err()
    {
        return;
    }
    // A tunnel may sit quiet for as long as its ends like.
    let _ = client.set_read_timeout(None);
    let _ = far.set_read_timeout(None);
    let (Ok(mut client_back), Ok(mut far_back)) = (client.try_clone(), far.try_clone()) else {
        return;
    };
    let out = thread::spawn(move || {
        let _ = io::copy(&mut client_back, &mut far_back);
        let _ = far_back.shutdown(Shutdown::Write);
    });
    let _ = io::copy(&mut far, &mut client);
    let _ = client.shutdown(Shutdown::Write);
    let _ = out.join();
}

/// What a refusal says to the client, as GitHub's error message.
fn said_to(refusal: &Refusal) -> String {
    format!(
        "the GitHub door refused this ({}): {}",
        refusal.why.word(),
        refusal.detail
    )
}

/// Answer the one request a tunnel to GitHub's `host` carries.
fn answer(door: &Door, session: &Session, tls: &mut impl ReadWrite, host: Host, mut call: Call) {
    let mut buffer = Vec::new();
    let head = match http::read_head(tls, &mut buffer) {
        Ok(head) => head,
        Err(Unread::Gone | Unread::Io(_)) => return,
        Err(Unread::Malformed(why)) => {
            return refuse(tls, &call, &Refusal::new(Why::Unreadable, why));
        }
        Err(Unread::TooLarge) => {
            return refuse(
                tls,
                &call,
                &Refusal::new(Why::TooLarge, "a request past the most the door reads"),
            );
        }
    };
    call.method.clone_from(&head.method);
    call.seen = Seen::Path(head.target.split('?').next().unwrap_or_default().to_owned());
    if let Err(refusal) = headers_allowed(&head) {
        return refuse(tls, &call, &refusal);
    }
    if head
        .header("expect")
        .is_some_and(|value| value.eq_ignore_ascii_case("100-continue"))
        && tls.write_all(b"HTTP/1.1 100 Continue\r\n\r\n").is_err()
    {
        return;
    }
    let body = match http::read_body(tls, &mut buffer, &head, MOST_BODY) {
        Ok(body) => body,
        Err(Unread::TooLarge) => {
            return refuse(
                tls,
                &call,
                &Refusal::new(Why::TooLarge, "a body past 64 MiB"),
            );
        }
        Err(Unread::Malformed(why)) => {
            return refuse(tls, &call, &Refusal::new(Why::Unreadable, why));
        }
        Err(Unread::Gone | Unread::Io(_)) => return,
    };
    let judged = judge::judge(
        &judge::Call {
            method: &head.method,
            host,
            target: &head.target,
            body: &body,
        },
        &door.allowed,
    );
    match judged {
        Ok(seen) => call.seen = seen,
        Err(refusal) => return refuse(tls, &call, &refusal),
    }
    let counted = session
        .count
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .call(Instant::now());
    match counted {
        Counted::Fine => {}
        Counted::Loud(calls) => eprintln!(
            "{}",
            crate::said::many_calls_line(&call.session, &call.claimed, calls)
        ),
        Counted::Refused(left) => {
            return refuse(
                tls,
                &call,
                &Refusal::new(Why::Cap, cap::refused_detail(left)),
            );
        }
    }
    let secret = match door.token.current() {
        Ok(secret) => secret,
        Err(why) => return refuse(tls, &call, &Refusal::new(Why::NoToken, why)),
    };
    match forward(door, host, &head, &body, &secret, tls) {
        Ok(status) => eprintln!("{}", call.passed_line(status)),
        Err(why) => refuse(
            tls,
            &call,
            &Refusal::new(Why::Upstream, format!("GitHub couldn't be reached: {why}")),
        ),
    }
}

/// A stream both ways, as a TLS connection is.
trait ReadWrite: Read + Write {}
impl<T: Read + Write> ReadWrite for T {}

/// Whether a request's headers are ones the door sends on: no credential
/// but the placeholder, and no method said another way.
fn headers_allowed(head: &Head) -> Result<(), Refusal> {
    let credentials = head.count("authorization");
    let placeholder = credentials == 1
        && head
            .header("authorization")
            .is_some_and(token::is_placeholder);
    if credentials > 0 && !placeholder {
        return Err(Refusal::new(
            Why::Authorization,
            format!(
                "a credential other than the fence's placeholder; inside the fence GH_TOKEN \
                 is {}, and the door adds the person's token itself",
                token::PLACEHOLDER
            ),
        ));
    }
    if OVERRIDES.iter().any(|name| head.count(name) > 0) {
        return Err(Refusal::new(
            Why::MethodOverride,
            "a header asking GitHub to read the method as another",
        ));
    }
    Ok(())
}

/// Say a refusal in the journal and to the client.
fn refuse(tls: &mut impl Write, call: &Call, refusal: &Refusal) {
    eprintln!("{}", call.refused_line(refusal));
    // The client may be gone; the journal already holds the refusal.
    let _ = http::write_answer(tls, refusal.why.status(), &said_to(refusal));
}

/// Send the request on to GitHub with `secret`, and its answer back to
/// `client`, saying GitHub's status. A 401 is tried once more when the
/// token reads differently now, since it may have been replaced.
fn forward(
    door: &Door,
    host: Host,
    head: &Head,
    body: &[u8],
    secret: &Secret,
    client: &mut impl Write,
) -> io::Result<u16> {
    let (mut upstream, mut buffer, mut status) = send(door, host, head, body, secret)?;
    if status == 401
        && token::authorization(host, secret).is_some()
        && let Some(renewed) = door.token.renewed(secret)
    {
        (upstream, buffer, status) = send(door, host, head, body, &renewed)?;
    }
    client.write_all(&buffer)?;
    match io::copy(&mut upstream, client) {
        Ok(_) => {}
        // GitHub may close without saying it's done; what came still stands.
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {}
        Err(error) => return Err(error),
    }
    client.flush()?;
    Ok(status)
}

/// One request sent to GitHub, and its answer's status, with what's been
/// read of the answer so far.
fn send(
    door: &Door,
    host: Host,
    head: &Head,
    body: &[u8],
    secret: &Secret,
) -> io::Result<(StreamOwned<ClientConnection, TcpStream>, Vec<u8>, u16)> {
    let mut headers: Vec<(String, Vec<u8>)> =
        vec![("Host".to_owned(), host.name().as_bytes().to_vec())];
    headers.extend(
        head.headers
            .iter()
            .filter(|(name, _)| {
                !DROPPED
                    .iter()
                    .any(|dropped| dropped.eq_ignore_ascii_case(name))
            })
            .cloned(),
    );
    if let Some(value) = token::authorization(host, secret) {
        headers.push(("Authorization".to_owned(), value.into_bytes()));
    }
    headers.push(("Connection".to_owned(), b"close".to_vec()));
    let mut upstream = door.upstream.connect(host)?;
    http::write_request(&mut upstream, &head.method, &head.target, &headers, body)?;
    let mut buffer = Vec::new();
    let status = http::read_status(&mut upstream, &mut buffer).map_err(|unread| match unread {
        Unread::Io(error) => error,
        other => io::Error::other(format!("{other:?}")),
    })?;
    Ok((upstream, buffer, status))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn head(raw: &str) -> Head {
        let mut buffer = raw.as_bytes().to_vec();
        http::read_head(&mut io::empty(), &mut buffer).expect("a head")
    }

    #[test]
    fn a_tunnel_is_asked_for_by_host_and_port() {
        assert_eq!(
            tunnel_to(&head("CONNECT API.github.com:443 HTTP/1.1\r\n\r\n")),
            Some(("api.github.com".to_owned(), 443))
        );
        assert_eq!(
            tunnel_to(&head("CONNECT [::1]:8443 HTTP/1.1\r\n\r\n")),
            Some(("::1".to_owned(), 8443))
        );
        assert_eq!(tunnel_to(&head("GET http://x/ HTTP/1.1\r\n\r\n")), None);
        assert_eq!(tunnel_to(&head("CONNECT nowhere HTTP/1.1\r\n\r\n")), None);
    }

    #[test]
    fn only_the_placeholder_passes_as_a_credential() {
        assert!(headers_allowed(&head("GET / HTTP/1.1\r\n\r\n")).is_ok());
        assert!(
            headers_allowed(&head(&format!(
                "GET / HTTP/1.1\r\nAuthorization: token {}\r\n\r\n",
                token::PLACEHOLDER
            )))
            .is_ok()
        );
        let other = headers_allowed(&head(
            "GET / HTTP/1.1\r\nAuthorization: token ghp_x\r\n\r\n",
        ));
        assert_eq!(other.expect_err("refused").why, Why::Authorization);
        let twice = format!(
            "GET / HTTP/1.1\r\nAuthorization: token {0}\r\nAuthorization: token {0}\r\n\r\n",
            token::PLACEHOLDER
        );
        assert_eq!(
            headers_allowed(&head(&twice)).expect_err("refused").why,
            Why::Authorization
        );
    }

    #[test]
    fn a_method_said_another_way_is_refused() {
        let overridden = head("POST / HTTP/1.1\r\nX-HTTP-Method-Override: DELETE\r\n\r\n");
        assert_eq!(
            headers_allowed(&overridden).expect_err("refused").why,
            Why::MethodOverride
        );
    }
}
