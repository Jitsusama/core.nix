//! The tests of where resolved says a name is, and of whether something
//! listens there.

use super::*;
use std::net::TcpListener;

/// A stand-in for resolved's socket: answers one request with `reply`,
/// and hands back what it was asked.
fn resolved(reply: &'static str) -> (tempfile::TempDir, std::thread::JoinHandle<String>) {
    use std::io::{Read, Write};
    let dir = tempfile::tempdir().expect("a folder");
    let socket =
        std::os::unix::net::UnixListener::bind(dir.path().join("resolve")).expect("a socket");
    let asked = std::thread::spawn(move || {
        let (mut stream, _) = socket.accept().expect("a request");
        let mut request = Vec::new();
        let mut byte = [0];
        while stream.read_exact(&mut byte).is_ok() && byte[0] != 0 {
            request.push(byte[0]);
        }
        stream.write_all(reply.as_bytes()).expect("the reply");
        stream.write_all(b"\0").expect("its end");
        String::from_utf8(request).expect("text")
    });
    (dir, asked)
}

#[test]
fn a_name_is_asked_of_resolved_itself_with_its_port_kept() {
    let (dir, asked) = resolved(
        r#"{"parameters":{"addresses":[
            {"ifindex":3,"family":2,"address":[192,0,2,45]},
            {"ifindex":3,"family":10,"address":[254,128,0,0,0,0,0,0,0,0,0,0,0,0,0,45]}
        ],"name":"home.local","flags":0}}"#,
    );
    let found = found("home.local:22", &dir.path().join("resolve"));
    let request: serde_json::Value =
        serde_json::from_str(&asked.join().expect("the stand-in")).expect("json");
    assert_eq!(request["method"], "io.systemd.Resolve.ResolveHostname");
    assert_eq!(request["parameters"]["name"], "home.local");
    assert_eq!(
        found,
        [
            "192.0.2.45:22".parse::<SocketAddr>().expect("v4"),
            SocketAddr::V6(SocketAddrV6::new("fe80::2d".parse().expect("v6"), 22, 0, 3)),
        ]
    );
}

#[test]
fn a_name_resolved_cannot_find_answers_nothing() {
    let (dir, asked) = resolved(r#"{"error":"io.systemd.Resolve.NoNameServers","parameters":{}}"#);
    assert_eq!(
        found("gone.local:22", &dir.path().join("resolve")),
        [] as [std::net::SocketAddr; 0]
    );
    asked.join().expect("the stand-in");
}

#[test]
fn an_address_is_never_asked_and_a_name_falls_back_without_resolved() {
    let nowhere = Path::new("/nonexistent/resolve");
    assert_eq!(
        found("127.0.0.1:9", nowhere),
        ["127.0.0.1:9".parse::<std::net::SocketAddr>().expect("v4")]
    );
    assert_eq!(
        found("not an address", nowhere),
        [] as [std::net::SocketAddr; 0]
    );
}

// A Mac's loopback holds 127.0.0.1 alone, so the address of its own is
// Linux's.
#[cfg(target_os = "linux")]
#[test]
fn a_builder_answers_only_while_something_listens() {
    // On a loopback address of its own, since a port it frees on the
    // one every other test binds may be taken by one of them at once.
    let listening = TcpListener::bind("127.0.0.3:0").expect("a port");
    let address = listening.local_addr().expect("its address").to_string();
    assert!(answers(&address), "a builder listening didn't answer");
    drop(listening);
    // A child another test forks holds the listener open until it
    // execs, which beside the proofs' tests took long enough to be
    // answered, so it's given a moment to go.
    let ending = std::time::Instant::now() + Duration::from_secs(5);
    while answers(&address) && std::time::Instant::now() < ending {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        !answers(&address),
        "a builder answered after it stopped listening"
    );
    assert!(
        !answers("not an address"),
        "something that isn't an address answered"
    );
}
