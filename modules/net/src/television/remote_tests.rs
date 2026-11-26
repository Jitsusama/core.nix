//! The remote's words, and its talk with a TV that speaks as Samsung's do
//! on a socket of the test's own, so no TV and no TLS are needed.

use std::cell::RefCell;
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
    stream
        .set_read_timeout(Some(Duration::from_millis(500)))
        .expect("a timeout");
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

/// What the remote on `socket` heard pressing `code` by `until`, why
/// alone where the key didn't go.
fn talked<S: Read + Write>(
    socket: &mut WebSocket<S>,
    code: &str,
    until: Instant,
    carried: bool,
) -> Result<Option<String>, Why> {
    talk(socket, code, until, carried).map_err(|unsent| unsent.why)
}

#[test]
fn a_tv_allowing_the_remote_hands_its_token_and_takes_the_key() {
    let (port, heard) = tv(vec![connected(Some("12345678"))]);
    let handed = talked(&mut remote(port), "KEY_VOLUP", soon(), false);
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
    assert_eq!(
        talked(&mut remote(port), "KEY_MUTE", soon(), true),
        Ok(None)
    );
    assert!(heard.join().expect("it ran").is_some(), "no key was heard");
}

#[test]
fn the_tvs_own_news_before_it_connects_is_read_past() {
    let news = json!({"event": "ed.edenTV.update", "data": {"update_type": "ed.edenApp.update"}});
    let (port, heard) = tv(vec![news.to_string(), "not json".into(), connected(None)]);
    assert_eq!(
        talked(&mut remote(port), "KEY_HOME", soon(), true),
        Ok(None)
    );
    assert!(heard.join().expect("it ran").is_some(), "no key was heard");
}

#[test]
fn a_tv_refusing_the_remote_or_giving_up_on_allow_takes_no_key() {
    for (event, carried, why) in [
        ("ms.channel.unauthorized", false, Why::Refused),
        ("ms.channel.unauthorized", true, Why::Forgotten),
        ("ms.channel.timeOut", false, Why::Unanswered),
    ] {
        let (port, heard) = tv(vec![json!({"event": event}).to_string()]);
        assert_eq!(
            talked(&mut remote(port), "KEY_POWER", soon(), carried),
            Err(why),
            "{event}, carrying a token: {carried}"
        );
        assert_eq!(heard.join().expect("it ran"), None, "{event} heard a key");
    }
}

#[test]
fn a_tv_saying_nothing_before_the_deadline_takes_no_key() {
    // Only a TV asked for the first time waits on Allow, so a silence to
    // the deadline after a token was carried is a TV gone quiet.
    for (carried, why) in [
        (false, Why::Unanswered),
        (true, Why::Broke("it stopped answering".into())),
    ] {
        let (port, heard) = tv(Vec::new());
        let until = Instant::now() + Duration::from_millis(1200);
        let mut socket = remote(port);
        assert_eq!(
            talked(&mut socket, "KEY_POWER", until, carried),
            Err(why),
            "carrying a token: {carried}"
        );
        drop(socket);
        assert_eq!(heard.join().expect("it ran"), None, "a key was heard");
    }
}

/// A box that says `said` as one WebSocket frame from a server, then
/// fails every write: a TV handing a token and breaking off at once.
struct Handing {
    said: io::Cursor<Vec<u8>>,
}

impl Handing {
    fn new(said: &str) -> Self {
        let length = u8::try_from(said.len()).expect("a short frame");
        assert!(length < 126, "a frame that needs a longer length");
        let mut frame = vec![0x81, length];
        frame.extend_from_slice(said.as_bytes());
        Handing {
            said: io::Cursor::new(frame),
        }
    }
}

impl Read for Handing {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        match self.said.read(into)? {
            0 => Err(io::ErrorKind::ConnectionReset.into()),
            read => Ok(read),
        }
    }
}

impl Write for Handing {
    fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
        Err(io::ErrorKind::BrokenPipe.into())
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::ErrorKind::BrokenPipe.into())
    }
}

#[test]
fn a_token_handed_before_the_key_failed_to_go_is_answered_all_the_same() {
    let socket = Handing::new(&connected(Some("12345678")));
    let mut socket = WebSocket::from_raw_socket(socket, tungstenite::protocol::Role::Client, None);
    let unsent = talk(&mut socket, "KEY_MUTE", soon(), false).expect_err("the key went");
    assert!(
        matches!(unsent.why, Why::Broke(_)),
        "a broken pipe reads as {:?}",
        unsent.why
    );
    assert_eq!(unsent.handed.as_deref(), Some("12345678"));
}

#[test]
fn a_key_that_didnt_go_keeps_a_handed_token_and_forgets_a_refused_one() {
    let pin = pin_of(b"the TV");
    let handed = Unsent {
        why: Why::Broke("reset".into()),
        handed: Some("87654321".into()),
    };
    assert_eq!(
        kept_unsent(handed, None, pin).kept,
        Some(Kept {
            token: Some("87654321".into()),
            pin
        })
    );
    let refused = Unsent::from(Why::Forgotten);
    assert_eq!(
        kept_unsent(refused, Some("12345678"), pin).kept,
        Some(Kept { token: None, pin }),
        "a refused token is kept, so the TV is never asked Allow again"
    );
    let quiet = Unsent::from(Why::Broke("it stopped answering".into()));
    assert_eq!(
        kept_unsent(quiet, Some("12345678"), pin).kept,
        None,
        "a key that broke off changes what was kept"
    );
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
    let v6 = url(IpAddr::V6(Ipv6Addr::LOCALHOST), "net", None);
    assert!(
        v6.starts_with("wss://[::1]:8002/"),
        "an IPv6 address isn't bracketed: {v6}"
    );
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
        crate::television::kept::hex(&pin_of(b"abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

/// A box that answers one byte at a time, forever, each a little later:
/// a frame that says it's long, then its body.
struct Trickle {
    sent: usize,
}

impl Read for Trickle {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        thread::sleep(Duration::from_millis(10));
        let header = [0x81, 126, 0xff, 0xff];
        into[0] = header.get(self.sent).copied().unwrap_or(b'a');
        self.sent += 1;
        Ok(1)
    }
}

impl Write for Trickle {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Waits for Trickle {
    fn wait_reads(&self, _longest: Duration) -> io::Result<()> {
        Ok(())
    }

    fn wait_writes(&self, _longest: Duration) -> io::Result<()> {
        Ok(())
    }
}

/// A socket that never has anything, as one whose timeout passed.
struct Silent;

impl Read for Silent {
    fn read(&mut self, _into: &mut [u8]) -> io::Result<usize> {
        Err(io::ErrorKind::WouldBlock.into())
    }
}

impl Waits for Silent {
    fn wait_reads(&self, _longest: Duration) -> io::Result<()> {
        Ok(())
    }

    fn wait_writes(&self, _longest: Duration) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_box_trickling_a_frame_holds_the_remote_no_longer_than_its_deadline() {
    // A first press waits on Allow, so its deadline passing is Allow
    // unanswered; after a token was carried it's the TV gone quiet.
    for (carried, why) in [
        (true, Why::Broke("it stopped answering".into())),
        (false, Why::Unanswered),
    ] {
        let started = Instant::now();
        let until = started + Duration::from_millis(300);
        let socket = Due {
            socket: Trickle { sent: 0 },
            until,
        };
        let mut socket =
            WebSocket::from_raw_socket(socket, tungstenite::protocol::Role::Client, None);
        assert_eq!(
            talked(&mut socket, "KEY_POWER", until, carried),
            Err(why),
            "carrying a token: {carried}"
        );
        let took = started.elapsed();
        assert!(
            took < Duration::from_secs(2),
            "a trickle held the remote {took:?}, carrying a token: {carried}"
        );
        assert!(
            socket.get_ref().socket.sent > 4,
            "the trickle wasn't read at all, carrying a token: {carried}"
        );
    }
}

#[test]
fn a_read_waiting_out_whats_left_fails_as_timed_out_and_none_runs_past_it() {
    let mut due = Due {
        socket: Silent,
        until: soon(),
    };
    let mut into = [0; 1];
    let waited = due
        .read(&mut into)
        .expect_err("a silent socket read something");
    assert_eq!(waited.kind(), io::ErrorKind::TimedOut);
    due.until = Instant::now();
    let past = due
        .read(&mut into)
        .expect_err("a read ran past the deadline");
    assert_eq!(past.kind(), io::ErrorKind::TimedOut);
}

#[test]
fn a_retry_is_offered_only_where_the_socket_broke() {
    let tls = io::Error::new(io::ErrorKind::InvalidData, "bad certificate");
    assert!(
        matches!(broke(tungstenite::Error::Io(tls)), Why::Unlike(_)),
        "TLS spoken wrong is offered a retry"
    );
    let reset = io::Error::from(io::ErrorKind::ConnectionReset);
    assert!(
        matches!(broke(tungstenite::Error::Io(reset)), Why::Broke(_)),
        "a reset isn't offered a retry"
    );
    assert!(
        matches!(broke(tungstenite::Error::ConnectionClosed), Why::Broke(_)),
        "a hang-up isn't offered a retry"
    );
    let protocol = tungstenite::Error::Protocol(tungstenite::error::ProtocolError::WrongHttpMethod);
    assert!(
        matches!(broke(protocol), Why::Unlike(_)),
        "WebSocket spoken wrong is offered a retry"
    );
}

fn shown(certificate: &[u8], pinned: &Pinned) -> Result<ServerCertVerified, rustls::Error> {
    let name = ServerName::IpAddress(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 90)).into());
    pinned.verify_server_cert(
        &CertificateDer::from(certificate.to_vec()),
        &[],
        &name,
        &[],
        UnixTime::now(),
    )
}

#[test]
fn a_certificate_other_than_the_pinned_one_fails_the_handshake_as_not_the_tv() {
    let pinned = Pinned::new(Some(pin_of(b"the TV")));
    assert!(
        shown(b"the TV", &pinned).is_ok(),
        "the pinned certificate is refused"
    );
    assert!(
        shown(b"another box", &pinned).is_err(),
        "another certificate is taken"
    );
    assert_eq!(pinned.seen(), Some(pin_of(b"another box")));
    let failed = io::Error::new(io::ErrorKind::InvalidData, "invalid peer certificate");
    assert_eq!(
        not_opened(&pinned, tungstenite::Error::Io(failed)),
        Why::NotTheTv
    );
}

#[test]
fn the_first_certificate_shown_is_taken_and_a_failure_then_is_no_impostor() {
    let first = Pinned::new(None);
    assert!(
        shown(b"the TV", &first).is_ok(),
        "the first certificate is refused"
    );
    assert_eq!(first.seen(), Some(pin_of(b"the TV")));
    let reset = io::Error::from(io::ErrorKind::ConnectionReset);
    assert!(
        matches!(
            not_opened(&first, tungstenite::Error::Io(reset)),
            Why::Broke(_)
        ),
        "a reset after the first certificate reads as an impostor"
    );
}

#[test]
fn a_key_handed_no_token_keeps_the_one_it_carried() {
    let pin = pin_of(b"the TV");
    assert_eq!(
        kept_after(None, Some("12345678"), pin).token.as_deref(),
        Some("12345678")
    );
    assert_eq!(
        kept_after(Some("87654321".into()), Some("12345678"), pin)
            .token
            .as_deref(),
        Some("87654321")
    );
    assert_eq!(kept_after(None, None, pin).token, None);
    assert_eq!(kept_after(None, None, pin).pin, pin);
}

/// A fake TV that opens the channel, takes a key, then hears the close:
/// it answers it where `answers`, and otherwise holds the channel open,
/// saying nothing, for longer than the remote lingers. It ends saying
/// whether it heard the close.
fn closing_tv(answers: bool) -> (u16, thread::JoinHandle<bool>) {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a port");
    let port = listener.local_addr().expect("its address").port();
    let heard = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("the remote reaches it");
        let mut socket = tungstenite::accept(stream).expect("the remote speaks WebSocket");
        socket
            .send(Message::text(connected(None)))
            .expect("it speaks");
        loop {
            match socket.read() {
                Ok(Message::Close(_)) => break,
                Ok(_) => {}
                Err(_) => return false,
            }
        }
        if answers {
            // The close's answer goes on the next flush, as tungstenite
            // queues it once the close is read.
            let _ = socket.flush();
            while socket.read().is_ok() {}
        } else {
            thread::sleep(LINGER * 3);
        }
        true
    });
    (port, heard)
}

#[test]
fn the_channel_closes_once_the_tv_answers_its_close() {
    let (port, heard) = closing_tv(true);
    let mut socket = remote(port);
    assert_eq!(talked(&mut socket, "KEY_MUTE", soon(), true), Ok(None));
    let started = Instant::now();
    close(&mut socket, started + Duration::from_secs(5));
    let took = started.elapsed();
    assert!(heard.join().expect("it ran"), "the TV heard no close");
    assert!(
        took < LINGER,
        "an answered close waited {took:?}, as if unanswered"
    );
}

#[test]
fn a_close_the_tv_never_answers_is_waited_on_only_as_long_as_the_linger() {
    let (port, heard) = closing_tv(false);
    let mut socket = remote(port);
    assert_eq!(talked(&mut socket, "KEY_MUTE", soon(), true), Ok(None));
    let started = Instant::now();
    close(&mut socket, started + LINGER);
    let took = started.elapsed();
    assert!(
        took >= LINGER && took < LINGER * 2,
        "an unanswered close waited {took:?} against a linger of {LINGER:?}"
    );
    drop(socket);
    assert!(heard.join().expect("it ran"), "the TV heard no close");
}

#[test]
fn a_tv_silent_through_the_handshake_is_left_at_the_deadline() {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a port");
    let port = listener.local_addr().expect("its address").port();
    let held = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("the remote reaches it");
        thread::sleep(Duration::from_secs(3));
        drop(stream);
    });
    let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).expect("it answers");
    let started = Instant::now();
    let until = started + Duration::from_millis(300);
    let pinned = Arc::new(Pinned::new(None));
    let address = IpAddr::V4(Ipv4Addr::LOCALHOST);
    let opened = opened(stream, address, "net", None, &pinned, until).map(|_| ());
    let took = started.elapsed();
    assert_eq!(opened, Err(Why::Broke("it stopped answering".into())));
    assert!(
        took < Duration::from_secs(2),
        "a silent handshake held the remote {took:?}"
    );
    held.join().expect("it ran");
}

/// A socket that answers a byte after `each`, noting each wait it's given
/// and each read it's asked for.
struct Recording {
    each: Duration,
    waits: RefCell<Vec<Duration>>,
    reads: usize,
}

impl Read for Recording {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        self.reads += 1;
        thread::sleep(self.each);
        into[0] = b'a';
        Ok(1)
    }
}

impl Waits for Recording {
    fn wait_reads(&self, longest: Duration) -> io::Result<()> {
        self.waits.borrow_mut().push(longest);
        Ok(())
    }

    fn wait_writes(&self, _longest: Duration) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn each_read_waits_only_whats_left_and_none_past_the_deadline_reaches_the_socket() {
    let started = Instant::now();
    let deadline = Duration::from_millis(200);
    let mut due = Due {
        socket: Recording {
            each: Duration::from_millis(40),
            waits: RefCell::new(Vec::new()),
            reads: 0,
        },
        until: started + deadline,
    };
    let mut into = [0; 1];
    let stopped = loop {
        if let Err(error) = due.read(&mut into) {
            break error;
        }
    };
    assert_eq!(stopped.kind(), io::ErrorKind::TimedOut);
    let waits = due.socket.waits.borrow().clone();
    assert!(waits.len() >= 2, "too few reads to judge: {waits:?}");
    assert!(
        waits.windows(2).all(|pair| pair[1] < pair[0]) && waits[0] <= deadline,
        "a read waited more than was left: {waits:?}"
    );
    let reads = due.socket.reads;
    assert_eq!(
        due.read(&mut into).map_err(|error| error.kind()),
        Err(io::ErrorKind::TimedOut)
    );
    assert_eq!(
        due.socket.reads, reads,
        "a read past the deadline reached the socket"
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
