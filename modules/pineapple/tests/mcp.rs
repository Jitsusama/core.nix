//! A tool gateway's tools called over HTTP, against a fake gateway on a
//! port of this machine's own: what it is asked, in what order, with which
//! session and sign-in, and what the caller is told.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::{fs, thread};

use pineapple::deaf::{Deafness, Round};
use pineapple::ears::{Ears, Through};
use pineapple::mcp::{Called, Gateway};
use pineapple::sign_in::SignIn;
use serde_json::{Value, json};

/// One request the fake gateway took.
#[derive(Clone, Debug)]
struct Seen {
    id: Value,
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
    /// Bytes promised beyond the body, so the answer is cut short.
    short: usize,
}

/// What refuses a sign-in: unauthorized, forbidden, and a redirect, which
/// sends its caller off to sign in again.
const REFUSALS: [u16; 3] = [401, 403, 302];

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
        short: 0,
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
        short: 0,
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
                id: request["id"].clone(),
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
                reply.body.len() + reply.short,
                reply.body
            );
        }
    });
    (url, seen)
}

/// A sign-in program in `dir` printing `tok-1`, then `tok-2`, and so on.
fn sign_in(dir: &Path) -> PathBuf {
    program(dir, "echo \"tok-$n\"")
}

/// How many times the program in `dir` ran.
fn runs(dir: &Path) -> u32 {
    fs::read_to_string(dir.join("sign-in.count"))
        .ok()
        .and_then(|count| count.trim().parse().ok())
        .unwrap_or(0)
}

/// A sign-in program in `dir` counting its runs in `$n`, then doing `then`.
fn program(dir: &Path, then: &str) -> PathBuf {
    fs::create_dir_all(dir).expect("a folder");
    let program = dir.join("sign-in");
    fs::write(
        &program,
        format!(
            "#!/bin/sh\nn=$(cat \"$0.count\" 2>/dev/null || echo 0); n=$((n + 1))\n\
             echo $n > \"$0.count\"; {then}\n"
        ),
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
    for refused in REFUSALS {
        let dir = scratch(&format!("refused-{refused}"));
        let (url, seen) = gateway(move |one, calls| match (one.method.as_str(), calls) {
            ("initialize", _) => opened("s1"),
            ("tools/call", 0) => status(refused),
            ("tools/call", _) => tool("me", false),
            _ => status(202),
        });
        let mut gateway = Gateway::new(&url, sign_in(&dir).as_os_str());
        assert_eq!(
            gateway.call("slack_who_am_i", &json!({})),
            Ok("me".to_owned()),
            "a sign-in renewed since a {refused} is taken up without a restart"
        );
        assert_eq!(
            last(&seen).sign_in.as_deref(),
            Some("Bearer tok-2"),
            "the program is run again for a fresh sign-in after a {refused}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}

#[test]
fn a_sign_in_refused_twice_is_said_as_refused() {
    for refused in REFUSALS {
        let dir = scratch(&format!("refused-twice-{refused}"));
        let (url, _) = gateway(move |one, _| match one.method.as_str() {
            "initialize" => opened("s1"),
            "tools/call" => status(refused),
            _ => status(202),
        });
        let mut gateway = Gateway::new(&url, sign_in(&dir).as_os_str());
        assert_eq!(
            gateway.call("slack_who_am_i", &json!({})),
            Err(Called::Refused),
            "a gateway refusing every sign-in with {refused} is said so, not asked for good"
        );
        assert_eq!(
            runs(&dir),
            2,
            "the sign-in is fetched once more after a {refused}, and no more"
        );
        let _ = fs::remove_dir_all(&dir);
    }
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
fn a_sign_in_refused_as_the_gateway_opens_is_asked_for_again_once() {
    for refused in REFUSALS {
        let dir = scratch(&format!("refused-opening-{refused}"));
        let (url, seen) = gateway(move |one, _| match one.method.as_str() {
            "initialize" if one.sign_in.as_deref() == Some("Bearer tok-1") => status(refused),
            "initialize" => opened("s1"),
            "tools/call" => tool("me", false),
            _ => status(202),
        });
        let mut gateway = Gateway::new(&url, sign_in(&dir).as_os_str());
        assert_eq!(
            gateway.call("slack_who_am_i", &json!({})),
            Ok("me".to_owned()),
            "a gateway refusing the sign-in with {refused} as it opens gets a fresh one"
        );
        assert_eq!(
            last(&seen).sign_in.as_deref(),
            Some("Bearer tok-2"),
            "the program is run again once after a {refused} as it opens"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}

#[test]
fn a_sign_in_refused_twice_as_the_gateway_opens_is_said_as_refused() {
    for refused in REFUSALS {
        let dir = scratch(&format!("refused-opening-twice-{refused}"));
        let (url, seen) = gateway(move |one, _| match one.method.as_str() {
            "initialize" => status(refused),
            _ => status(202),
        });
        let mut gateway = Gateway::new(&url, sign_in(&dir).as_os_str());
        assert_eq!(
            gateway.call("slack_who_am_i", &json!({})),
            Err(Called::Refused),
            "a gateway refusing every sign-in with {refused} as it opens is said so"
        );
        assert_eq!(runs(&dir), 2, "the sign-in is fetched twice, no more");
        assert!(
            !methods(&seen).contains(&"tools/call".to_owned()),
            "no tool is asked of a gateway that never opened"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}

#[test]
fn a_gateway_naming_no_session_is_opened_once() {
    let dir = scratch("sessionless");
    let (url, seen) = gateway(|one, _| match one.method.as_str() {
        "initialize" => json_reply(&json!({ "jsonrpc": "2.0", "id": 1, "result": {} })),
        "tools/call" => tool("me", false),
        _ => status(202),
    });
    let mut gateway = Gateway::new(&url, sign_in(&dir).as_os_str());
    for _ in 0..2 {
        assert_eq!(
            gateway.call("slack_who_am_i", &json!({})),
            Ok("me".to_owned()),
            "a gateway with no sessions answers all the same"
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
        "it is opened once, though it named no session"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_locked_keyring_is_rested_from_rather_than_asked_each_call() {
    let dir = scratch("locked");
    let (url, seen) = gateway(|_, _| status(202));
    let mut gateway = Gateway::new(&url, program(&dir, "exit 75").as_os_str());
    for _ in 0..3 {
        assert_eq!(
            gateway.call("slack_who_am_i", &json!({})),
            Err(Called::NoSignIn(SignIn::Locked)),
            "a locked keyring is said as no sign-in"
        );
    }
    assert_eq!(
        runs(&dir),
        1,
        "its unlock prompt is raised once, not each call"
    );
    assert!(
        methods(&seen).is_empty(),
        "nothing is asked without a sign-in"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_sign_in_is_looked_for_again_each_call() {
    let dir = scratch("missing");
    let (url, _) = gateway(|_, _| status(202));
    let mut gateway = Gateway::new(&url, program(&dir, "exit 1").as_os_str());
    for _ in 0..2 {
        assert_eq!(
            gateway.call("slack_who_am_i", &json!({})),
            Err(Called::NoSignIn(SignIn::Missing)),
            "a program with nothing to give is said as no sign-in"
        );
    }
    assert_eq!(runs(&dir), 2, "one that asks nobody is run again");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_tool_saying_something_else_is_not_available_turns_nothing_on() {
    let dir = scratch("not-available");
    let (url, seen) = gateway(|one, _| match one.method.as_str() {
        "initialize" => opened("s1"),
        "tools/call" => tool("Channel C1 is not available", true),
        _ => status(202),
    });
    let mut gateway = Gateway::new(&url, sign_in(&dir).as_os_str());
    assert_eq!(
        gateway.call("slack_read_thread", &json!({})),
        Err(Called::Failed("Channel C1 is not available".to_owned())),
        "the tool's own failure is said"
    );
    let asked: Vec<Value> = seen
        .lock()
        .expect("what was seen")
        .iter()
        .map(|one| one.params["name"].clone())
        .collect();
    assert!(
        !asked.contains(&json!("tool_gateway_enable_backend")),
        "no backend is turned on: {asked:?}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_session_failing_otherwise_is_let_go_and_opened_afresh() {
    let dir = scratch("bad-session");
    let (url, seen) = gateway(|one, calls| match (one.method.as_str(), calls) {
        ("initialize", _) => opened("s1"),
        // Rejected as JSON-RPC says, then with nothing said.
        ("tools/call", 0) => Reply {
            status: 400,
            ..json_reply(&json!({
                "jsonrpc": "2.0", "id": 1,
                "error": { "code": -32600, "message": "no such session" },
            }))
        },
        ("tools/call", 2) => status(500),
        ("tools/call", _) => tool("me", false),
        _ => status(202),
    });
    let mut gateway = Gateway::new(&url, sign_in(&dir).as_os_str());
    for how in ["as JSON-RPC says", "with nothing said"] {
        assert!(
            gateway.call("slack_who_am_i", &json!({})).is_err(),
            "a session the gateway rejects {how} fails its call"
        );
        assert_eq!(
            gateway.call("slack_who_am_i", &json!({})),
            Ok("me".to_owned()),
            "and the next, after one rejected {how}, is asked in a session opened afresh"
        );
    }
    let opened = methods(&seen)
        .iter()
        .filter(|method| *method == "initialize")
        .count();
    assert_eq!(opened, 3, "the gateway is opened again after each");
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
        "tools/call" => {
            let answer = json!({
                "jsonrpc": "2.0", "id": one.id,
                "result": { "content": [{ "type": "text", "text": "me" }] },
            });
            let notice = json!({ "jsonrpc": "2.0", "method": "notifications/message" });
            let another = json!({
                "jsonrpc": "2.0", "id": 999,
                "result": { "content": [{ "type": "text", "text": "not mine" }] },
            });
            let pretty = serde_json::to_string_pretty(&answer).expect("the answer");
            let split: String = pretty
                .lines()
                .flat_map(|line| ["data: ", line, "\n"])
                .collect();
            Reply {
                kind: "text/event-stream",
                body: format!(
                    "event: message\ndata: {notice}\n\nevent: message\ndata: {another}\n\n\
                     event: message\n{split}\nevent: message\ndata: {notice}\n\n"
                ),
                ..status(200)
            }
        }
        _ => status(202),
    });
    let mut gateway = Gateway::new(&url, sign_in(&dir).as_os_str());
    assert_eq!(
        gateway.call("slack_who_am_i", &json!({})),
        Ok("me".to_owned()),
        "a gateway may answer as a stream, over several lines, among a notice and an \
         answer to another request before it and a notice after"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn an_answer_that_isnt_json_rpc_is_unreadable_unless_the_gateway_is_down_or_busy() {
    let pages = [
        ("text/html", "<html>sign in</html>"),
        ("application/json", r#"{"message":"slow down"}"#),
    ];
    for (code, unreadable) in [(200, true), (400, true), (429, false), (502, false)] {
        for (at, (kind, page)) in pages.into_iter().enumerate() {
            let dir = scratch(&format!("not-json-rpc-{code}-{at}"));
            let (url, _) = gateway(move |one, _| match one.method.as_str() {
                "initialize" => opened("s1"),
                "tools/call" => Reply {
                    kind,
                    body: page.to_owned(),
                    ..status(code)
                },
                _ => status(202),
            });
            let mut gateway = Gateway::new(&url, sign_in(&dir).as_os_str());
            let called = gateway.call("slack_who_am_i", &json!({}));
            assert!(
                if unreadable {
                    matches!(called, Err(Called::Unreadable(_)))
                } else {
                    matches!(called, Err(Called::Unreachable(_)))
                },
                "a {kind} page in a {code} is unreadable, a cause that lasts, unless it's the \
             gateway's own failing or a spent rate: {called:?}"
            );
            let _ = fs::remove_dir_all(&dir);
        }
    }
}

#[test]
fn an_answer_cut_short_says_it_couldnt_be_read() {
    let dir = scratch("cut-short");
    let (url, _) = gateway(|one, _| match one.method.as_str() {
        "initialize" => opened("s1"),
        "tools/call" => Reply {
            short: 10,
            ..tool("me", false)
        },
        _ => status(202),
    });
    let mut gateway = Gateway::new(&url, sign_in(&dir).as_os_str());
    let called = gateway.call("slack_who_am_i", &json!({}));
    assert!(
        matches!(&called, Err(Called::Unreachable(why)) if why.contains("couldn't read")),
        "an answer ending before its length says it couldn't be read, not that it \
         isn't JSON-RPC: {called:?}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_gateway_passing_on_slacks_refusal_counts_as_refused() {
    for (said, round) in [
        ("invalid_auth", Round::Deaf(Deafness::Refused)),
        ("token_revoked", Round::Deaf(Deafness::Refused)),
        ("channel_not_found", Round::Unasked),
    ] {
        let dir = scratch(&format!("slack-said-{said}"));
        let (url, _) = gateway(move |one, _| match one.method.as_str() {
            "initialize" => opened("s1"),
            "tools/call" => tool(&format!("Slack said: {said}"), true),
            _ => status(202),
        });
        let mut through = Through {
            gateway: Gateway::new(&url, sign_in(&dir).as_os_str()),
        };
        assert_eq!(
            through.search("pineapple"),
            Err(round),
            "a search failing with Slack's {said} counts as {round:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
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
