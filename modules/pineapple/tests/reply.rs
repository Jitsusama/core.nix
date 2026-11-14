//! The replier's one request against a stand-in for Slack on this
//! machine: a reply posts to chat.postMessage with Joel's sign-in, in the
//! conversation and thread it names, saying its words and unfurling
//! nothing; and a reply Slack refuses is said to have failed, with why.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use pineapple::reply::{Post, send};
use slack_people::Account;

/// How long a case waits on the stand-in to hear the request.
const PATIENCE: Duration = Duration::from_secs(10);

/// What the stand-in heard: the request line, the headers and the body.
struct Heard {
    line: String,
    headers: Vec<String>,
    body: String,
}

/// A stand-in that hears one request and answers it with `answer`.
fn stand_in(answer: &'static str) -> (String, mpsc::Receiver<Heard>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port on this machine");
    let origin = format!("http://{}", listener.local_addr().expect("its address"));
    let (tell, heard) = mpsc::channel();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("one request");
        let mut reader = BufReader::new(stream.try_clone().expect("the stream again"));
        let mut line = String::new();
        reader.read_line(&mut line).expect("its request line");
        let mut headers = Vec::new();
        loop {
            let mut header = String::new();
            reader.read_line(&mut header).expect("a header");
            if header.trim().is_empty() {
                break;
            }
            headers.push(header.trim_end().to_owned());
        }
        let length = headers
            .iter()
            .find_map(|header| {
                let (name, value) = header.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())?
            })
            .unwrap_or(0);
        let mut body = vec![0; length];
        reader.read_exact(&mut body).expect("its body");
        let reply = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{answer}",
            answer.len()
        );
        stream.write_all(reply.as_bytes()).expect("the answer");
        tell.send(Heard {
            line: line.trim_end().to_owned(),
            headers,
            body: String::from_utf8(body).expect("a form is text"),
        })
        .expect("the case listening");
    });
    (origin, heard)
}

fn joel() -> Account {
    Account {
        token: "xoxc-stand-in".to_owned(),
        cookie: "xoxd-stand-in".to_owned(),
        team: "T0TEAM0001".to_owned(),
        user: "U0JOEL0001".to_owned(),
    }
}

fn post() -> Post {
    Post {
        channel: "C0ONE00001".to_owned(),
        thread_ts: "1700000000.000001".to_owned(),
        text: "Automated, not Joel.\nStopped.".to_owned(),
    }
}

#[test]
fn a_reply_posts_in_its_thread_with_joels_sign_in() {
    let (origin, heard) = stand_in(r#"{"ok": true}"#);
    assert_eq!(send(&origin, &joel(), &post()), Ok(()), "Slack took it");
    let heard = heard.recv_timeout(PATIENCE).expect("the stand-in heard it");
    assert!(
        heard.line.starts_with("POST /api/chat.postMessage "),
        "it posts a message, and asks nothing else: {}",
        heard.line
    );
    assert!(
        heard
            .headers
            .iter()
            .any(|header| header == "authorization: Bearer xoxc-stand-in"
                || header == "Authorization: Bearer xoxc-stand-in"),
        "it goes from Joel's own sign-in: {:?}",
        heard.headers
    );
    for part in [
        "channel=C0ONE00001",
        "thread_ts=1700000000.000001",
        "text=Automated%2C+not+Joel.%0AStopped.",
        "unfurl_links=false",
        "unfurl_media=false",
    ] {
        assert!(
            heard.body.split('&').any(|one| one == part),
            "the form says {part}: {}",
            heard.body
        );
    }
}

#[test]
fn a_reply_slack_refuses_is_said_to_have_failed() {
    let (origin, _heard) = stand_in(r#"{"ok": false, "error": "not_in_channel"}"#);
    assert_eq!(
        send(&origin, &joel(), &post()),
        Err("Slack answered 200: not_in_channel".to_owned()),
        "a refusal says why"
    );
}
