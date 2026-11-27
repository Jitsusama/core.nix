//! Whether something listens at an address, its name asked of
//! systemd-resolved itself rather than through the C library.

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv6Addr, SocketAddr, SocketAddrV6, TcpStream, ToSocketAddrs};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

/// How long a standing builder has to answer before it's left out.
const ANSWER: Duration = Duration::from_secs(2);

/// Whether something listens at an address, `host:port`.
pub fn answers(address: &str) -> bool {
    found(address, Path::new(RESOLVED))
        .iter()
        .any(|address| TcpStream::connect_timeout(address, ANSWER).is_ok())
}

/// systemd-resolved's own socket, asked directly rather than through the C
/// library. With nscd running, as on NixOS, the library reaches resolved only
/// through it, and once a lookup there times out, as one for a `.local` name
/// nobody answers does after 5 s, it leaves nscd out of the process's next
/// lookups and falls back to plain DNS, which finds no `.local` name at all.
/// On 30 October an absent builder kept one that answered out of every tick.
const RESOLVED: &str = "/run/systemd/resolve/io.systemd.Resolve";

/// How long resolved has to say where a name is.
const RESOLVING: Duration = Duration::from_secs(6);

/// The socket addresses `host:port` names: itself when it's an address,
/// else what resolved says the host is, else, where resolved can't be
/// reached, what the C library says.
fn found(address: &str, resolved: &Path) -> Vec<SocketAddr> {
    if let Ok(address) = address.parse::<SocketAddr>() {
        return vec![address];
    }
    address
        .rsplit_once(':')
        .and_then(|(host, port)| Some((host, port.parse::<u16>().ok()?)))
        .and_then(|(host, port)| asked(resolved, host, port))
        .unwrap_or_else(|| {
            address
                .to_socket_addrs()
                .map(Iterator::collect)
                .unwrap_or_default()
        })
}

/// Ask resolved where `host` is, over varlink: a JSON call ended by a NUL
/// byte, answered the same way. `None` when its socket can't be reached;
/// a name it can't find, or a reply that doesn't come, finds nothing rather
/// than falling back to the library that would lose every later name.
fn asked(resolved: &Path, host: &str, port: u16) -> Option<Vec<SocketAddr>> {
    let mut stream = UnixStream::connect(resolved).ok()?;
    let call = json!({
        "method": "io.systemd.Resolve.ResolveHostname",
        "parameters": { "name": host },
    });
    let mut reply = Vec::new();
    let answered = stream
        .set_read_timeout(Some(RESOLVING))
        .and_then(|()| stream.write_all(call.to_string().as_bytes()))
        .and_then(|()| stream.write_all(b"\0"))
        .and_then(|()| BufReader::new(&stream).read_until(0, &mut reply));
    if answered.is_err() || reply.pop() != Some(0) {
        return Some(Vec::new());
    }
    let reply: Value = serde_json::from_slice(&reply).unwrap_or_default();
    Some(
        reply["parameters"]["addresses"]
            .as_array()
            .map(|addresses| {
                addresses
                    .iter()
                    .filter_map(|found| address_of(found, port))
                    .collect()
            })
            .unwrap_or_default(),
    )
}

/// One of resolved's addresses, its bytes and family, at `port`. A
/// link-local one keeps the interface it was found on, without which it
/// can't be reached.
fn address_of(found: &Value, port: u16) -> Option<SocketAddr> {
    let bytes: Vec<u8> = found["address"]
        .as_array()?
        .iter()
        .map(|byte| byte.as_u64().and_then(|byte| u8::try_from(byte).ok()))
        .collect::<Option<_>>()?;
    if let Ok(v4) = <[u8; 4]>::try_from(bytes.as_slice()) {
        return Some(SocketAddr::from((v4, port)));
    }
    let v6 = Ipv6Addr::from(<[u8; 16]>::try_from(bytes.as_slice()).ok()?);
    let interface = found["ifindex"]
        .as_u64()
        .and_then(|index| u32::try_from(index).ok())
        .filter(|_| v6.is_unicast_link_local())
        .unwrap_or(0);
    Some(SocketAddr::V6(SocketAddrV6::new(v6, port, 0, interface)))
}

#[cfg(test)]
#[path = "resolving_tests.rs"]
mod tests;
