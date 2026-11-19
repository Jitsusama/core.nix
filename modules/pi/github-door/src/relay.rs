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
//! request goes straight out on a connection of its own, in the form a
//! server is asked, without the hop's own headers and closed after its
//! answer. GitHub's hosts are refused over plain HTTP, since the door
//! answers them only through a tunnel, and anything else, a request
//! with no host or one for HTTPS, is refused rather than guessed at.
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
//! ...)` and carry on.

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
    let Ok(head) = http::read_head(&mut client, &mut buffer) else {
        let _ = http::write_answer(&mut client, 400, CARRIES);
        return;
    };
    if head.method != "CONNECT" {
        plain(client, &head, &buffer);
        return;
    }
    let target = head.target.clone();
    let Some(host) = target
        .rsplit_once(':')
        .map(|(host, _)| unbracketed(host).to_owned())
    else {
        let _ = http::write_answer(&mut client, 405, CARRIES);
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

/// Carry one plain HTTP request, `head` with `read` read past it,
/// straight out: in origin form, without the hop's own headers, and on a
/// connection closed after its answer, so nothing more the client sends
/// reaches the host as a request of its own.
fn plain(mut client: TcpStream, head: &http::Head, read: &[u8]) {
    let Some((host, port, path)) = absolute(&head.target) else {
        let _ = http::write_answer(&mut client, 405, CARRIES);
        return;
    };
    if Host::named(unbracketed(host)).is_some() {
        let _ = http::write_answer(
            &mut client,
            403,
            "GitHub is reached only over HTTPS, through the door",
        );
        return;
    }
    let Some(mut far) = reach(&format!("{host}:{port}")) else {
        let _ = http::write_answer(&mut client, 502, "the host couldn't be reached");
        return;
    };
    let _ = client.set_read_timeout(None);
    if far.write_all(&origin_form(head, &path)).is_err() || far.write_all(read).is_err() {
        return;
    }
    let (Ok(back), Ok(far_back)) = (client.try_clone(), far.try_clone()) else {
        return;
    };
    splice(client, far, far_back, back);
}

/// The host, port and path of an absolute `http://` address, when it
/// names a host and no user.
fn absolute(target: &str) -> Option<(&str, u16, String)> {
    let scheme = target.get(..7)?;
    if !scheme.eq_ignore_ascii_case("http://") {
        return None;
    }
    let rest = &target[7..];
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, path) = rest.split_at(end);
    if authority.contains('@') {
        return None;
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) if !port.contains(']') => (host, port.parse().ok()?),
        _ => (authority, 80),
    };
    if unbracketed(host).is_empty() {
        return None;
    }
    let path = match path.chars().next() {
        Some('/') => path.to_owned(),
        Some('?') => format!("/{path}"),
        _ => "/".to_owned(),
    };
    Some((host, port, path))
}

/// A host without the brackets an IPv6 address is written in.
fn unbracketed(host: &str) -> &str {
    host.trim_start_matches('[').trim_end_matches(']')
}

/// `head` as its server is asked it: its line in origin form, `path`, and
/// its headers but the hop's own, closing the connection after.
fn origin_form(head: &http::Head, path: &str) -> Vec<u8> {
    let named: Vec<String> = head
        .headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("Connection"))
        .flat_map(|(_, value)| {
            String::from_utf8_lossy(value)
                .split(',')
                .map(|name| name.trim().to_ascii_lowercase())
                .collect::<Vec<_>>()
        })
        .collect();
    let hop = |name: &str| {
        let name = name.to_ascii_lowercase();
        HOP.contains(&name.as_str()) || named.contains(&name)
    };
    let mut out = format!("{} {path} HTTP/1.1\r\n", head.method).into_bytes();
    for (name, value) in head.headers.iter().filter(|(name, _)| !hop(name)) {
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(b": ");
        out.extend_from_slice(value);
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"Connection: close\r\n\r\n");
    out
}

/// Headers for the hop to the proxy alone, never passed on.
const HOP: [&str; 4] = [
    "connection",
    "proxy-connection",
    "keep-alive",
    "proxy-authorization",
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
            Some(("rubygems.org", 80, "/gems?x=1".to_owned()))
        );
        assert_eq!(
            absolute("HTTP://[::1]:8080?x"),
            Some(("[::1]", 8080, "/?x".to_owned()))
        );
        assert_eq!(
            absolute("http://[::1]"),
            Some(("[::1]", 80, "/".to_owned()))
        );
        for refused in [
            "/gems",
            "https://rubygems.org/",
            "http://user@rubygems.org/",
            "http:///gems",
            "http://rubygems.org:port/",
        ] {
            assert_eq!(absolute(refused), None, "{refused}");
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
