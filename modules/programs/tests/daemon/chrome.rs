//! What Chrome has open, read by the daemon itself with no command: from
//! the profile's session file, as its watch begins and again as Chrome
//! writes the file while it's watched, and from the extension's host on
//! its socket first, where it's there.

use std::io::{Read as _, Write as _};
use std::os::unix::net::UnixListener;
use std::thread;

use chrome_session::written::{session, tab};
use serde_json::json;

use super::Running;

const OPENED: &str = "opened:chrome";

/// Writes the profile's session file as Chrome writes a new one, holding
/// one tab a page.
fn sessioned(running: &Running, made: u64, pages: &[&str]) {
    let mut commands = Vec::new();
    for (at, address) in (10..).zip(pages) {
        commands.extend(tab(1, at, at - 10, address, 1_793_000_000_000));
    }
    std::fs::write(
        running.path(&format!("chrome/Sessions/Session_{made}")),
        session(&commands),
    )
    .unwrap();
}

#[test]
fn chromes_open_pages_are_read_from_its_session_and_again_as_it_writes_it() {
    let running = Running::new("chrome-session");
    sessioned(&running, 1, &["https://a.example.org/"]);
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "watch", "keys": [OPENED]}));
    assert_eq!(
        client.changed(OPENED)["sites"],
        json!([{
            "id": "", "window": "1", "front": false, "looked_at_ms": 1_793_000_000_000_u64,
            "address": "https://a.example.org/", "title": "Title 10",
        }]),
        "the session file's one tab, with no id"
    );
    sessioned(
        &running,
        2,
        &["https://a.example.org/", "https://b.example.org/"],
    );
    let told = client.changed(OPENED);
    let addresses: Vec<&str> = told["sites"]
        .as_array()
        .unwrap()
        .iter()
        .map(|site| site["address"].as_str().unwrap())
        .collect();
    assert_eq!(
        addresses,
        ["https://a.example.org/", "https://b.example.org/"],
        "the newer session file's tabs, told as Chrome wrote it"
    );
}

#[test]
fn the_extensions_host_is_asked_first_on_its_socket() {
    let running = Running::new("chrome-host");
    sessioned(&running, 1, &["https://saved.example.org/"]);
    let listener = UnixListener::bind(running.path("chrome-open-tabs.sock")).unwrap();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut asked = String::new();
        stream.read_to_string(&mut asked).unwrap();
        if asked == "list\n" {
            let said = "412\t7\ttrue\t1793000000123\thttps://live.example.org/\tLive\n";
            stream.write_all(said.as_bytes()).unwrap();
        }
    });
    let said = running
        .client()
        .ask(&json!({"id": 1, "ask": "get", "keys": [OPENED]}));
    assert_eq!(
        said["records"][0]["sites"],
        json!([{
            "id": "412", "window": "7", "front": true, "looked_at_ms": 1_793_000_000_123_u64,
            "address": "https://live.example.org/", "title": "Live",
        }]),
        "the host's tab, by the id it gave, rather than the file's"
    );
}
