//! The relay, inside the fence: the proxy gh and git are pointed at.
//!
//! gh reaches a proxy only over TCP, and the door is a Unix socket the
//! fence binds in, so the relay listens on localhost for the fenced
//! command and passes each tunnel on. A tunnel to one of GitHub's four
//! hosts goes to the session's socket, where the door answers it; any
//! other goes straight out from the relay, which the fence's network
//! allows anyway, so a door that's down or restarting breaks gh alone,
//! never a fenced command's other downloads, and the door sees nothing
//! of them.
//!
//! Plain HTTP comes too, as a proxy is asked for it, with the whole
//! address in the request line: Ruby's `Net::HTTP`, which dev drives, reads
//! only `http_proxy`, so a fence pointing it here must carry it. Each
//! request is read whole and goes straight out on a connection of its
//! own, in the form a server is asked, without the hop's own headers,
//! and only its answer comes back, told the connection closes and cut
//! at the length it names, so a client gets one request a connection.
//! GitHub's hosts are refused over plain HTTP by name, since the door
//! answers them only through a tunnel; one reached by its address goes
//! out as any host does, which carries no token, since the door alone
//! holds it. Anything else, a request with no host or one for HTTPS, is
//! refused, saying why, rather than guessed at.
//!
//! Localhost is shared by every process on the machine, other sessions'
//! fences among them, so the relay answers only a connection a process
//! in its own fence holds: it finds the client's socket in the kernel's
//! table and looks for it among the processes its own `/proc` shows,
//! which in the fence's pid namespace are the fence's alone.
//!
//! `pi-github-door relay` binds the port, prints it, and leaves a child
//! answering it until the process it was told to watch is gone, so a
//! shell can say `HTTPS_PROXY=http://127.0.0.1:$(pi-github-door relay
//! ...)`, and point `http_proxy` at the same port, and carry on.

use std::fs;
use std::io::{self, Write};
use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::os::fd::{AsRawFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use crate::http;
use crate::judge::Host;

/// How often the relay looks for the process it watches.
const LOOK: Duration = Duration::from_secs(2);
/// How long reaching a host may take.
const REACH: Duration = Duration::from_secs(15);

/// Bind a port on localhost and leave a child relaying it to the door at
/// `socket`, for as long as process `watch` lives; the port is the
/// answer.
pub fn launch(socket: &Path, watch: u32) -> Result<u16, String> {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .map_err(|error| format!("no port on localhost: {error}"))?;
    let port = listener
        .local_addr()
        .map_err(|error| error.to_string())?
        .port();
    let program =
        std::env::current_exe().map_err(|error| format!("it can't find itself: {error}"))?;
    Command::new(program)
        .args(["relaying", "--socket"])
        .arg(socket)
        .args(["--while", &watch.to_string()])
        .stdin(Stdio::from(OwnedFd::from(listener)))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("the relay couldn't start: {error}"))?;
    Ok(port)
}

/// Relay each connection `listener` takes until process `watch` is gone.
pub fn serve(listener: &TcpListener, socket: &Path, watch: u32) {
    let watched = PathBuf::from(format!("/proc/{watch}"));
    while watched.exists() {
        if !ready(listener) {
            continue;
        }
        let Ok((stream, _)) = listener.accept() else {
            continue;
        };
        if !own(&stream) {
            // Not a process of this fence's; it learns nothing but that.
            let _ = stream.shutdown(Shutdown::Both);
            continue;
        }
        let socket = socket.to_owned();
        thread::spawn(move || relay(stream, &socket));
    }
}

/// Whether a connection waits on `listener`, waiting at most [`LOOK`].
fn ready(listener: &TcpListener) -> bool {
    let mut asked = libc::pollfd {
        fd: listener.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    let wait = libc::c_int::try_from(LOOK.as_millis()).unwrap_or(libc::c_int::MAX);
    // SAFETY: one pollfd, alive for the call, for a descriptor the
    // listener holds open.
    let ready = unsafe { libc::poll(&raw mut asked, 1, wait) };
    ready > 0 && asked.revents & libc::POLLIN != 0
}

/// What the relay carries, said when it's asked anything else.
const CARRIES: &str = "this proxy carries CONNECT, and plain HTTP with its whole address";

/// Pass one client's tunnel or plain request on: GitHub's tunnels to the
/// door, the rest straight out.
fn relay(mut client: TcpStream, socket: &Path) {
    let _ = client.set_read_timeout(Some(REACH));
    let mut buffer = Vec::new();
    let head = match http::read_head(&mut client, &mut buffer) {
        Ok(head) => head,
        Err(http::Unread::Gone) => return,
        Err(unread) => {
            let said = format!("{}; {CARRIES}", unread_said(&unread));
            let _ = http::write_answer(&mut client, 400, &said);
            return;
        }
    };
    if head.method != "CONNECT" {
        plain(client, &head, buffer);
        return;
    }
    let target = head.target.clone();
    let Some(host) = target
        .rsplit_once(':')
        .map(|(host, _)| unbracketed(host).to_owned())
    else {
        let said = format!("a CONNECT names its host's port; {CARRIES}");
        let _ = http::write_answer(&mut client, 400, &said);
        return;
    };
    let _ = client.set_read_timeout(None);
    if Host::named(&host).is_some() {
        let Ok(mut door) = UnixStream::connect(socket) else {
            let _ = http::write_answer(
                &mut client,
                502,
                "the GitHub door isn't answering; it may be restarting",
            );
            return;
        };
        let asked = format!("CONNECT {target} HTTP/1.1\r\nHost: {target}\r\n\r\n");
        if door.write_all(asked.as_bytes()).is_err() || door.write_all(&buffer).is_err() {
            return;
        }
        let (Ok(back), Ok(door_back)) = (client.try_clone(), door.try_clone()) else {
            return;
        };
        splice(client, door, door_back, back);
    } else {
        let Some(mut far) = reach(&target) else {
            let _ = client.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n");
            return;
        };
        if client
            .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
            .is_err()
            || far.write_all(&buffer).is_err()
        {
            return;
        }
        let (Ok(back), Ok(far_back)) = (client.try_clone(), far.try_clone()) else {
            return;
        };
        splice(client, far, far_back, back);
    }
}

/// The most of a plain request's body the relay reads before sending it on.
const MOST_PLAIN_BODY: usize = 64 * 1024 * 1024;
/// How long a plain request's host, or its client, may take no bytes or
/// give none before the relay gives up on them.
const QUIET: Duration = Duration::from_secs(300);
/// How long a refused client may go on sending before it's closed.
const LINGER: Duration = Duration::from_secs(2);

/// Carry one plain HTTP request, `head` with `read` read past it,
/// straight out, and its answer back. The request is read whole, body
/// and all, and sent in origin form with the relay's own Host, length and
/// `Connection: close`, and without the hop's own headers; only the
/// answer is copied back, so nothing more the client sends ever reaches
/// the host, whether or not the host honours the close.
fn plain(mut client: TcpStream, head: &http::Head, mut read: Vec<u8>) {
    let _ = client.set_write_timeout(Some(QUIET));
    let (host, port, path) = match absolute(&head.target) {
        Ok(found) => found,
        Err(why) => return refuse(client, 400, &format!("{why}; {CARRIES}")),
    };
    if Host::named(unbracketed(host)).is_some() {
        return refuse(
            client,
            403,
            "GitHub is reached only over HTTPS, through the door",
        );
    }
    // The body is read before anything goes on, so a client waiting to be
    // told to go on is told here, as the host would have.
    match head.header("expect") {
        None if head.count("expect") == 0 => {}
        Some(expected) if expected.eq_ignore_ascii_case("100-continue") => {
            if client.write_all(b"HTTP/1.1 100 Continue\r\n\r\n").is_err() {
                return;
            }
        }
        _ => return refuse(client, 417, "the relay expects only 100-continue"),
    }
    let body = match http::read_body(&mut client, &mut read, head, MOST_PLAIN_BODY) {
        Ok(body) => body,
        Err(unread) => {
            let status = if matches!(unread, http::Unread::TooLarge) {
                413
            } else {
                400
            };
            return refuse(client, status, &unread_said(&unread));
        }
    };
    let authority = if port == 80 {
        host.to_owned()
    } else {
        format!("{host}:{port}")
    };
    let Some(mut far) = reach(&format!("{host}:{port}")) else {
        let said = format!("{authority} couldn't be reached");
        return refuse(client, 502, &said);
    };
    let _ = far.set_read_timeout(Some(QUIET));
    let _ = far.set_write_timeout(Some(QUIET));
    let headers = passed_on(&head.headers, &authority);
    // A host may answer before it's read the whole request, and close; its
    // answer still comes back.
    let asked = http::write_request(&mut far, &head.method, &path, &headers, &body).is_ok();
    let mut back = Vec::new();
    let answer = loop {
        match http::read_answer(&mut far, &mut back) {
            // The relay told the client to go on itself, if it asked.
            Ok(answer) if (100..200).contains(&answer.status) => {}
            Ok(answer) => break answer,
            Err(_) if !asked => {
                let said = format!("{authority} closed before it was asked");
                return refuse(client, 502, &said);
            }
            Err(unread) => {
                let said = format!("{authority} gave no answer: {}", unread_said(&unread));
                return refuse(client, 502, &said);
            }
        }
    };
    pass_back(&mut client, &mut far, &answer, &back, &head.method);
    close(client);
}

/// Pass `answer` back to `client`, told the connection closes, then its
/// body: `back`, read past its head, and the rest of `far`, cut at the
/// length it names, so nothing past the one answer comes back and the
/// client needn't wait on a host that holds the connection open.
fn pass_back(
    client: &mut TcpStream,
    far: &mut TcpStream,
    answer: &http::Answer,
    back: &[u8],
    method: &str,
) {
    let named = connection_names(&answer.headers);
    let mut head = format!("HTTP/1.1 {} {}\r\n", answer.status, answer.reason).into_bytes();
    for (name, value) in &answer.headers {
        let lower = name.to_ascii_lowercase();
        // Its framing stays as the host sent it, since the body is too.
        let framing = matches!(lower.as_str(), "content-length" | "transfer-encoding");
        let hop = matches!(
            lower.as_str(),
            "connection" | "proxy-connection" | "keep-alive"
        ) || named.contains(&lower);
        if hop && !framing {
            continue;
        }
        head.extend_from_slice(name.as_bytes());
        head.extend_from_slice(b": ");
        head.extend_from_slice(value);
        head.extend_from_slice(b"\r\n");
    }
    head.extend_from_slice(b"Connection: close\r\n\r\n");
    if client.write_all(&head).is_err() {
        return;
    }
    let bodiless = method == "HEAD" || matches!(answer.status, 204 | 304);
    let length = if bodiless {
        Some(0)
    } else if answer
        .headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("transfer-encoding"))
    {
        // Chunks end where the host closes, which it was asked to.
        None
    } else {
        answer
            .header("content-length")
            .and_then(|value| value.parse::<u64>().ok())
    };
    let Some(length) = length else {
        if client.write_all(back).is_ok() {
            let _ = io::copy(far, client);
        }
        return;
    };
    let held = usize::try_from(length)
        .unwrap_or(usize::MAX)
        .min(back.len());
    if client.write_all(&back[..held]).is_ok() {
        let _ = io::copy(&mut io::Read::take(far, length - held as u64), client);
    }
}

/// Tell `client` `status`, saying why, and close it.
fn refuse(mut client: TcpStream, status: u16, said: &str) {
    let _ = http::write_answer(&mut client, status, said);
    close(client);
}

/// Close `client`, taking what it still sends for a moment first, so a
/// client still sending its body hears the answer rather than a reset.
fn close(mut client: TcpStream) {
    let _ = client.shutdown(Shutdown::Write);
    let _ = client.set_read_timeout(Some(LINGER));
    let mut sink = io::sink();
    let _ = io::copy(
        &mut io::Read::take(&mut client, MOST_PLAIN_BODY as u64),
        &mut sink,
    );
    let _ = client.shutdown(Shutdown::Both);
}

/// What a request that couldn't be read is told.
fn unread_said(unread: &http::Unread) -> String {
    match unread {
        http::Unread::Gone => "nothing was asked".to_owned(),
        http::Unread::Malformed(why) => why.clone(),
        http::Unread::TooLarge => "a body past 64 MiB".to_owned(),
        http::Unread::Io(error) => format!("the request couldn't be read: {error}"),
    }
}

/// The host, port and path of an absolute `http://` address, when it
/// names a host and no user, or why not.
fn absolute(target: &str) -> Result<(&str, u16, String), &'static str> {
    let scheme = target.get(..8).unwrap_or(target);
    if scheme.eq_ignore_ascii_case("https://") {
        return Err("HTTPS is carried only through CONNECT");
    }
    if !scheme
        .get(..7)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("http://"))
    {
        return Err("a plain request names its whole http:// address");
    }
    let rest = &target[7..];
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, path) = rest.split_at(end);
    if authority.contains('@') {
        return Err("an address naming a user isn't carried");
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) if !port.contains(']') => (
            host,
            port.parse()
                .map_err(|_| "an address whose port isn't one")?,
        ),
        _ => (authority, 80),
    };
    // A host is a name, or an IPv6 address in its brackets, and nothing
    // that reads two ways, such as a second port.
    let bracketed = host.starts_with('[') && host.ends_with(']');
    let stray = host.contains(['[', ']']) && !bracketed;
    if unbracketed(host).is_empty() || stray || (host.contains(':') && !bracketed) {
        return Err("an address whose host reads more than one way");
    }
    // A fragment is the client's alone, never sent.
    let path = path.split('#').next().unwrap_or_default();
    let path = match path.chars().next() {
        Some('/') => path.to_owned(),
        Some('?') => format!("/{path}"),
        _ => "/".to_owned(),
    };
    Ok((host, port, path))
}

/// A host without the brackets an IPv6 address is written in.
fn unbracketed(host: &str) -> &str {
    host.trim_start_matches('[').trim_end_matches(']')
}

/// The headers a request holding `headers` is sent on with: the relay's
/// own Host, `authority`, first, then the rest but the hop's own, those
/// the relay writes itself and any Connection names, and
/// `Connection: close`.
fn passed_on(headers: &[(String, Vec<u8>)], authority: &str) -> Vec<(String, Vec<u8>)> {
    let named = connection_names(headers);
    let kept = |name: &str| {
        let name = name.to_ascii_lowercase();
        !NOT_PASSED.contains(&name.as_str()) && !named.contains(&name)
    };
    let mut kept_on = vec![("Host".to_owned(), authority.as_bytes().to_vec())];
    kept_on.extend(headers.iter().filter(|(name, _)| kept(name)).cloned());
    kept_on.push(("Connection".to_owned(), b"close".to_vec()));
    kept_on
}

/// The headers each Connection header in `headers` names, lower-cased.
fn connection_names(headers: &[(String, Vec<u8>)]) -> Vec<String> {
    headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("Connection"))
        .flat_map(|(_, value)| {
            String::from_utf8_lossy(value)
                .split(',')
                .map(|name| name.trim().to_ascii_lowercase())
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Headers never passed on: the hop's own to the proxy, and those the
/// relay writes itself, since it reads the body whole and sends it with
/// its length.
const NOT_PASSED: [&str; 11] = [
    "connection",
    "proxy-connection",
    "keep-alive",
    "proxy-authorization",
    "te",
    "trailer",
    "upgrade",
    "expect",
    "host",
    "content-length",
    "transfer-encoding",
];

/// A TCP connection to `target`, a host and port.
fn reach(target: &str) -> Option<TcpStream> {
    use std::net::ToSocketAddrs;
    target
        .to_socket_addrs()
        .ok()?
        .find_map(|address| TcpStream::connect_timeout(&address, REACH).ok())
}

/// Copy `client` to `far` and `far_back` to `client_back` until each side
/// ends, closing each half as its side does.
fn splice<A, B>(mut client: TcpStream, mut far: A, mut far_back: B, mut client_back: TcpStream)
where
    A: io::Write + Closes + Send + 'static,
    B: io::Read,
{
    let out = thread::spawn(move || {
        let _ = io::copy(&mut client, &mut far);
        far.close_write();
    });
    let _ = io::copy(&mut far_back, &mut client_back);
    let _ = client_back.shutdown(Shutdown::Write);
    let _ = out.join();
}

/// A stream whose writing half can be closed.
trait Closes {
    fn close_write(&self);
}

impl Closes for TcpStream {
    fn close_write(&self) {
        let _ = self.shutdown(Shutdown::Write);
    }
}

impl Closes for UnixStream {
    fn close_write(&self) {
        let _ = self.shutdown(Shutdown::Write);
    }
}

/// Whether a process this relay's `/proc` shows holds the far end of
/// `stream`: in the fence, a process of its own fence.
fn own(stream: &TcpStream) -> bool {
    let (Ok(peer), Ok(local)) = (stream.peer_addr(), stream.local_addr()) else {
        return false;
    };
    let Some(inode) = inode_of(peer, local) else {
        return false;
    };
    let wanted = format!("socket:[{inode}]");
    let Ok(processes) = fs::read_dir("/proc") else {
        return false;
    };
    processes
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .bytes()
                .all(|byte| byte.is_ascii_digit())
        })
        .any(|entry| {
            fs::read_dir(entry.path().join("fd")).is_ok_and(|fds| {
                fds.flatten().any(|fd| {
                    fs::read_link(fd.path()).is_ok_and(|link| link.as_os_str() == wanted.as_str())
                })
            })
        })
}

/// The inode of the socket bound at `ours` and connected to `theirs`, as
/// the kernel's TCP table says.
fn inode_of(ours: SocketAddr, theirs: SocketAddr) -> Option<u64> {
    let table = fs::read_to_string("/proc/net/tcp").ok()?;
    inode_in(&table, ours, theirs)
}

/// The inode of the socket bound at `ours` and connected to `theirs` in a
/// TCP table as `/proc/net/tcp` holds it.
fn inode_in(table: &str, ours: SocketAddr, theirs: SocketAddr) -> Option<u64> {
    let (local, remote) = (hex(ours)?, hex(theirs)?);
    table.lines().skip(1).find_map(|line| {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let matches =
            fields.get(1) == Some(&local.as_str()) && fields.get(2) == Some(&remote.as_str());
        matches.then(|| fields.get(9)?.parse().ok()).flatten()
    })
}

/// An IPv4 address and port as the kernel's table writes them.
fn hex(address: SocketAddr) -> Option<String> {
    let IpAddr::V4(ip) = address.ip() else {
        return None;
    };
    Some(format!(
        "{:08X}:{:04X}",
        u32::from_ne_bytes(ip.octets()),
        address.port()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_absolute_address_gives_its_host_port_and_path() {
        assert_eq!(
            absolute("http://rubygems.org/gems?x=1"),
            Ok(("rubygems.org", 80, "/gems?x=1".to_owned()))
        );
        assert_eq!(
            absolute("HTTP://[::1]:8080?x"),
            Ok(("[::1]", 8080, "/?x".to_owned()))
        );
        assert_eq!(absolute("http://[::1]"), Ok(("[::1]", 80, "/".to_owned())));
        assert_eq!(
            absolute("http://rubygems.org/gems#top"),
            Ok(("rubygems.org", 80, "/gems".to_owned())),
            "a fragment is never sent"
        );
        for (refused, why) in [
            ("/gems", "whole"),
            ("https://rubygems.org/", "CONNECT"),
            ("http://user@rubygems.org/", "user"),
            ("http:///gems", "host"),
            ("http://rubygems.org:port/", "port"),
            ("http://api.github.com:80:80/", "host"),
            ("http://[::1/", "host"),
            ("http://::1/", "host"),
        ] {
            let said = absolute(refused).expect_err(refused);
            assert!(said.contains(why), "{refused}: {said}");
        }
    }

    #[test]
    fn a_connection_is_found_in_the_table() {
        let table = concat!(
            "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt",
            "   uid  timeout inode\n",
            "   0: 0100007F:D431 0100007F:1F90 01 00000000:00000000 00:00000000 00000000",
            "  1000        0 123456 1 0000000000000000 20 4 30 10 -1\n",
        );
        let ours: SocketAddr = "127.0.0.1:54321".parse().expect("an address");
        let theirs: SocketAddr = "127.0.0.1:8080".parse().expect("an address");
        if cfg!(target_endian = "little") {
            assert_eq!(inode_in(table, ours, theirs), Some(123_456));
        }
        assert_eq!(inode_in(table, theirs, ours), None);
    }

    #[test]
    fn a_connection_this_process_holds_is_its_own() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("a port");
        let _client = TcpStream::connect(listener.local_addr().expect("bound")).expect("connected");
        let (accepted, _) = listener.accept().expect("accepted");
        assert!(own(&accepted), "a connection this process holds is its own");
    }
}
