//! A tool gateway's tools called over HTTP, against a fake gateway on a
//! port of this machine's own: what it is asked, in what order, with which
//! session and sign-in, and what the caller is told.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::{fs, thread};

use pineapple::mcp::{Called, Gateway};
use serde_json::{Value, json};

/// One request the fake gateway took.
#[derive(Clone, Debug)]
struct Seen {
    method: String,
    params: Value,
    session: Option<String>,
    sign_in: Option<String>,
}

/// What the fake gateway answers: its status, the session it names and
/// its body, with the type the body is sent as.
struct Reply {
    status: u16,
    session: Option<&'static str>,
    kind: &'static str,
    body: String,
}

/// A tool's answer saying `text`, failed when `failed`.
fn tool(text: &str, failed: bool) -> Reply {
    json_reply(&json!({
        "jsonrpc": "2.0", "id": 1,
        "result": { "content": [{ "type": "text", "text": text }], "isError": failed },
    }))
}

fn json_reply(body: &Value) -> Reply {
    Reply {
        status: 200,
        session: None,
        kind: "application/json",
        body: body.to_string(),
    }
}

/// An answer to `initialize`, naming `session`.
fn opened(session: &'static str) -> Reply {
    Reply {
        session: Some(session),
        ..json_reply(&json!({ "jsonrpc": "2.0", "id": 1, "result": { "protocolVersion": "x" } }))
    }
}

fn status(status: u16) -> Reply {
    Reply {
        status,
        session: None,
        kind: "text/plain",
        body: String::new(),
    }
}

/// A fake gateway answering as `answer` says of each request and how many
/// tools/call it has taken before it, with the URL to reach it and what it
/// was asked.
fn gateway(
    answer: impl Fn(&Seen, usize) -> Reply + Send + 'static,
) -> (String, Arc<Mutex<Vec<Seen>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    let url = format!("http://{}/mcp", listener.local_addr().expect("its address"));
    let seen = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&seen);
    thread::spawn(move || {
        let mut calls = 0;
        for stream in listener.incoming() {
            let mut stream = stream.expect("the stand-in gateway takes each connection");
            let mut reader = BufReader::new(stream.try_clone().expect("a stream"));
            let (mut length, mut session, mut sign_in) = (0, None, None);
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                    break;
                }
                let (name, value) = line.split_once(':').unwrap_or_default();
                let value = value.trim().to_owned();
                match name.to_ascii_lowercase().as_str() {
                    "content-length" => length = value.parse().unwrap_or(0),
                    "mcp-session-id" => session = Some(value),
                    "authorization" => sign_in = Some(value),
                    _ => {}
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).expect("its body");
            let request: Value = serde_json::from_slice(&body).unwrap_or_default();
            let one = Seen {
                method: request["method"].as_str().unwrap_or_default().to_owned(),
                params: request["params"].clone(),
                session,
                sign_in,
            };
            let reply = answer(&one, calls);
            if one.method == "tools/call" {
                calls += 1;
            }
            kept.lock().expect("what was seen").push(one);
            let session = reply
                .session
                .map(|session| format!("Mcp-Session-Id: {session}\r\n"))
                .unwrap_or_default();
            let _ = write!(
                stream,
                "HTTP/1.1 {} X\r\nContent-Type: {}\r\n{session}Content-Length: {}\r\n\
                 Connection: close\r\n\r\n{}",
                reply.status,
                reply.kind,
                reply.body.len(),
                reply.body
            );
        }
    });
    (url, seen)
}

/// A sign-in program in `dir` printing `tok-1`, then `tok-2`, and so on.
fn sign_in(dir: &Path) -> PathBuf {
    fs::create_dir_all(dir).expect("a folder");
    let program = dir.join("sign-in");
    fs::write(
        &program,
        "#!/bin/sh\nn=$(cat \"$0.count\" 2>/dev/null || echo 0); n=$((n + 1))\n\
         echo $n > \"$0.count\"; echo \"tok-$n\"\n",
    )
    .expect("a sign-in program");
    std::process::Command::new("chmod")
        .arg("+x")
        .arg(&program)
        .status()
        .expect("chmod runs");
    program
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pineapple-mcp-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

fn methods(seen: &Arc<Mutex<Vec<Seen>>>) -> Vec<String> {
    let seen = seen.lock().expect("what was seen");
    seen.iter().map(|one| one.method.clone()).collect()
}

fn last(seen: &Arc<Mutex<Vec<Seen>>>) -> Seen {
    seen.lock()
        .expect("what was seen")
        .last()
        .cloned()
        .expect("a request")
}

#[test]
fn a_gateway_is_opened_once_and_asked_with_its_session_and_sign_in() {
    let dir = scratch("opened");
    let (url, seen) = gateway(|one, _| match one.method.as_str() {
        "initialize" => opened("s1"),
        "tools/call" => tool("me", false),
        _ => status(202),
    });
    let mut gateway = Gateway::new(&url, sign_in(&dir).as_os_str());
    for _ in 0..2 {
        assert_eq!(
            gateway.call("slack_who_am_i", &json!({})),
            Ok("me".to_owned()),
            "the tool's text is what the caller is told"
        );
    }
    assert_eq!(
        methods(&seen),
        [
            "initialize",
            "notifications/initialized",
            "tools/call",
            "tools/call"
        ],
        "the gateway is opened once for both calls"
    );
    let call = last(&seen);
    assert_eq!(
        call.session.as_deref(),
        Some("s1"),
        "each call names the session"
    );
    assert_eq!(
        call.sign_in.as_deref(),
        Some("Bearer tok-1"),
        "and is signed in once"
    );
    assert_eq!(
        call.params["name"], "slack_who_am_i",
        "the tool asked is the one named"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_tool_whose_backend_went_quiet_has_it_turned_on_and_is_asked_again() {
    let dir = scratch("quiet");
    let (url, seen) = gateway(|one, calls| match (one.method.as_str(), calls) {
        ("initialize", _) => opened("s1"),
        ("tools/call", 0) => tool("Tool \"slack_read_thread\" is not available.", true),
        ("tools/call", _) => tool("the thread", false),
        _ => status(202),
    });
    let mut gateway = Gateway::new(&url, sign_in(&dir).as_os_str());
    assert_eq!(
        gateway.call("slack_read_thread", &json!({ "channel_id": "C1" })),
        Ok("the thread".to_owned()),
        "the tool is asked again once its backend is on"
    );
    let seen = seen.lock().expect("what was seen").clone();
    let asked: Vec<&Value> = seen.iter().map(|one| &one.params["name"]).collect();
    assert_eq!(
        asked[2..],
        [
            &json!("slack_read_thread"),
            &json!("tool_gateway_enable_backend"),
            &json!("slack_read_thread")
        ],
        "the backend is turned on between the two asks"
    );
    assert_eq!(
        seen[3].params["arguments"],
        json!({ "backend": "slack" }),
        "the backend turned on is the tool's own"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_refused_sign_in_is_asked_for_again_once() {
    let dir = scratch("refused");
    let (url, seen) = gateway(|one, calls| match (one.method.as_str(), calls) {
        ("initialize", _) => opened("s1"),
        ("tools/call", 0) => status(401),
        ("tools/call", _) => tool("me", false),
        _ => status(202),
    });
    let mut gateway = Gateway::new(&url, sign_in(&dir).as_os_str());
    assert_eq!(
        gateway.call("slack_who_am_i", &json!({})),
        Ok("me".to_owned()),
        "a sign-in renewed since is taken up without a restart"
    );
    assert_eq!(
        last(&seen).sign_in.as_deref(),
        Some("Bearer tok-2"),
        "the program is run again for a fresh sign-in"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_sign_in_refused_twice_is_said_as_refused() {
    let dir = scratch("refused-twice");
    let (url, _) = gateway(|one, _| match one.method.as_str() {
        "initialize" => opened("s1"),
        "tools/call" => status(401),
        _ => status(202),
    });
    let mut gateway = Gateway::new(&url, sign_in(&dir).as_os_str());
    assert_eq!(
        gateway.call("slack_who_am_i", &json!({})),
        Err(Called::Refused),
        "a gateway refusing every sign-in is said so, not asked for good"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_forgotten_session_is_opened_again() {
    let dir = scratch("forgotten");
    let (url, seen) = gateway(|one, calls| match (one.method.as_str(), calls) {
        ("initialize", 0) => opened("s1"),
        ("initialize", _) => opened("s2"),
        ("tools/call", 0) => status(404),
        ("tools/call", _) => tool("me", false),
        _ => status(202),
    });
    let mut gateway = Gateway::new(&url, sign_in(&dir).as_os_str());
    assert_eq!(
        gateway.call("slack_who_am_i", &json!({})),
        Ok("me".to_owned()),
        "the call goes through in a new session"
    );
    assert_eq!(
        last(&seen).session.as_deref(),
        Some("s2"),
        "the new session is named"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_tool_that_fails_says_why() {
    let dir = scratch("failed");
    let (url, _) = gateway(|one, _| match one.method.as_str() {
        "initialize" => opened("s1"),
        "tools/call" => tool("channel_not_found", true),
        _ => status(202),
    });
    let mut gateway = Gateway::new(&url, sign_in(&dir).as_os_str());
    assert_eq!(
        gateway.call("slack_read_thread", &json!({})),
        Err(Called::Failed("channel_not_found".to_owned())),
        "the tool's own words say why"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn an_answer_as_an_event_stream_is_read() {
    let dir = scratch("stream");
    let (url, _) = gateway(|one, _| match one.method.as_str() {
        "initialize" => opened("s1"),
        "tools/call" => Reply {
            kind: "text/event-stream",
            body: format!("event: message\ndata: {}\n\n", tool("me", false).body),
            ..status(200)
        },
        _ => status(202),
    });
    let mut gateway = Gateway::new(&url, sign_in(&dir).as_os_str());
    assert_eq!(
        gateway.call("slack_who_am_i", &json!({})),
        Ok("me".to_owned()),
        "a gateway may answer as a stream of events"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_gateway_not_there_is_unreachable() {
    let dir = scratch("unreachable");
    let port = TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .expect("a port nobody holds once let go");
    let mut gateway = Gateway::new(&format!("http://{port}/mcp"), sign_in(&dir).as_os_str());
    assert!(
        matches!(
            gateway.call("slack_who_am_i", &json!({})),
            Err(Called::Unreachable(_))
        ),
        "a gateway nobody answers for is said as unreachable"
    );
    let _ = fs::remove_dir_all(&dir);
}
