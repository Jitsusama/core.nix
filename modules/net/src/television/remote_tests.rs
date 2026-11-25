//! The remote's words, and its talk with a TV that speaks as Samsung's do
//! on a socket of the test's own, so no TV and no TLS are needed.

use std::net::{Ipv4Addr, Ipv6Addr, TcpListener};
use std::thread;

use super::*;

/// A fake TV: says each of `opening` as the channel opens, then answers
/// the next message it hears, if any, by what it said.
fn tv(opening: Vec<String>) -> (u16, thread::JoinHandle<Option<String>>) {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a port");
    let port = listener.local_addr().expect("its address").port();
    let heard = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("the remote reaches it");
        let mut socket = tungstenite::accept(stream).expect("the remote speaks WebSocket");
        for said in opening {
            socket.send(Message::text(said)).expect("it speaks");
        }
        loop {
            match socket.read() {
                Ok(Message::Text(text)) => return Some(text.as_str().to_owned()),
                Ok(Message::Close(_)) | Err(_) => return None,
                Ok(_) => {}
            }
        }
    });
    (port, heard)
}

/// The remote's end of the fake TV on `port`.
fn remote(port: u16) -> WebSocket<TcpStream> {
    let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).expect("it answers");
    stream.set_read_timeout(Some(TICK)).expect("a timeout");
    let url = format!("ws://127.0.0.1:{port}{CHANNEL}?name=bmV0");
    tungstenite::client::client(url, stream)
        .expect("the channel opens")
        .0
}

fn connected(token: Option<&str>) -> String {
    let mut data = json!({"clients": [], "id": "3e0cfa6b"});
    if let Some(token) = token {
        data["token"] = json!(token);
    }
    json!({"event": "ms.channel.connect", "data": data}).to_string()
}

fn soon() -> Instant {
    Instant::now() + Duration::from_secs(5)
}

#[test]
fn a_tv_allowing_the_remote_hands_its_token_and_takes_the_key() {
    let (port, heard) = tv(vec![connected(Some("12345678"))]);
    let handed = talk(&mut remote(port), "KEY_VOLUP", soon());
    assert_eq!(handed, Ok(Some("12345678".into())));
    let pressed: Value =
        serde_json::from_str(&heard.join().expect("it ran").expect("it heard a key"))
            .expect("JSON");
    assert_eq!(pressed["method"], "ms.remote.control");
    assert_eq!(pressed["params"]["DataOfCmd"], "KEY_VOLUP");
    assert_eq!(pressed["params"]["Cmd"], "Click");
}

#[test]
fn a_tv_knowing_the_remote_hands_no_token_and_takes_the_key() {
    let (port, heard) = tv(vec![connected(None)]);
    assert_eq!(talk(&mut remote(port), "KEY_MUTE", soon()), Ok(None));
    assert!(heard.join().expect("it ran").is_some(), "no key was heard");
}

#[test]
fn the_tvs_own_news_before_it_connects_is_read_past() {
    let news = json!({"event": "ed.edenTV.update", "data": {"update_type": "ed.edenApp.update"}});
    let (port, heard) = tv(vec![news.to_string(), "not json".into(), connected(None)]);
    assert_eq!(talk(&mut remote(port), "KEY_HOME", soon()), Ok(None));
    assert!(heard.join().expect("it ran").is_some(), "no key was heard");
}

#[test]
fn a_tv_refusing_the_remote_or_giving_up_on_allow_takes_no_key() {
    for event in ["ms.channel.unauthorized", "ms.channel.timeOut"] {
        let (port, heard) = tv(vec![json!({"event": event}).to_string()]);
        assert_eq!(
            talk(&mut remote(port), "KEY_POWER", soon()),
            Err(Why::NotAllowed),
            "{event}"
        );
        assert_eq!(heard.join().expect("it ran"), None, "{event} heard a key");
    }
}

#[test]
fn a_tv_saying_nothing_before_the_deadline_takes_no_key() {
    let (port, heard) = tv(Vec::new());
    let until = Instant::now() + Duration::from_millis(1200);
    let mut socket = remote(port);
    assert_eq!(talk(&mut socket, "KEY_POWER", until), Err(Why::NotAllowed));
    drop(socket);
    assert_eq!(heard.join().expect("it ran"), None, "a key was heard");
}

#[test]
fn a_token_that_isnt_plain_is_never_taken() {
    let long = "9".repeat(65);
    for token in ["", "12 34", "1&name=x", long.as_str()] {
        assert_eq!(
            told(&connected(Some(token))),
            Told::Connected { token: None },
            "{token:?}"
        );
    }
}

#[test]
fn the_channel_names_the_asker_in_base64_and_carries_a_token() {
    let address = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 90));
    assert_eq!(
        url(address, "net on desk", None),
        "wss://10.0.0.90:8002/api/v2/channels/samsung.remote.control?name=bmV0IG9uIGRlc2s="
    );
    assert_eq!(
        url(address, "net", Some("12345678")),
        "wss://10.0.0.90:8002/api/v2/channels/samsung.remote.control?name=bmV0&token=12345678"
    );
    assert!(
        !url(address, "net", Some("1&x=2")).contains("token"),
        "a token that isn't plain is carried"
    );
    assert!(url(IpAddr::V6(Ipv6Addr::LOCALHOST), "net", None).starts_with("wss://[::1]:8002/"));
}

#[test]
fn base64_pads_as_standard_base64_does() {
    for (plain, coded) in [
        ("", ""),
        ("n", "bg=="),
        ("ne", "bmU="),
        ("net", "bmV0"),
        ("net on", "bmV0IG9u"),
    ] {
        assert_eq!(base64(plain.as_bytes()), coded, "{plain:?}");
    }
    assert_eq!(base64(&[0xfb, 0xff]), "+/8=");
}

#[test]
fn a_pin_is_the_certificates_sha256() {
    assert_eq!(
        pin_of(b"abc")
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn only_a_certificate_other_than_the_pinned_one_is_refused() {
    let pinned = Pinned::new(Some(pin_of(b"the TV")));
    *pinned.seen.lock().expect("held") = Some(pin_of(b"the TV"));
    assert!(!pinned.refused(), "the pinned one is refused");
    *pinned.seen.lock().expect("held") = Some(pin_of(b"another box"));
    assert!(pinned.refused(), "another is taken");
    let first = Pinned::new(None);
    *first.seen.lock().expect("held") = Some(pin_of(b"the TV"));
    assert!(!first.refused(), "the first one shown is refused");
}
