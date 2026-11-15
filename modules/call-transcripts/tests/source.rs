//! transcripts-people as the people daemon runs it: an address written,
//! the calls with it said, and said again as call-transcripts' daemon
//! tells a call changed or gone, over a stand-in calls.sock that answers
//! the watch as call-transcripts does.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};

/// The next line it says, or a failure naming what was awaited.
fn next(said: &Receiver<Value>, awaited: &str) -> Value {
    said.recv_timeout(Duration::from_secs(10))
        .unwrap_or_else(|_| panic!("transcripts-people never said {awaited}"))
}

/// The next line it says of `address` that passes `test`, passing over
/// what it said before the stand-in's answer reached it.
fn until(said: &Receiver<Value>, address: &str, test: impl Fn(&Value) -> bool) -> Value {
    loop {
        let line = next(said, address);
        if line["email"] == address && test(&line) {
            return line;
        }
    }
}

/// A stand-in of call-transcripts' socket at `socket`, which waits for
/// the watch of every call and then says each answer it's told.
fn stand_in(socket: &Path) -> Sender<Value> {
    let _ = std::fs::remove_file(socket);
    let listener = UnixListener::bind(socket).expect("the stand-in's socket");
    let (tell, told) = mpsc::channel::<Value>();
    thread::spawn(move || {
        let (stream, _) = listener.accept().expect("transcripts-people connects");
        let mut asked = BufReader::new(stream.try_clone().expect("the stream, to read"));
        // Its hello, then its watch of every call, as Daemon.qml asks.
        let mut line = String::new();
        while !line.contains("\"watch\"") {
            line.clear();
            assert!(
                asked.read_line(&mut line).expect("what it asks") > 0,
                "transcripts-people hung up before it watched"
            );
        }
        assert!(line.contains("\"call\""), "it watches calls, not {line}");
        let mut out = stream;
        for answer in told {
            writeln!(out, "{answer}").expect("the stand-in says its answer");
        }
    });
    tell
}

/// transcripts-people following `socket`, as the people daemon runs it:
/// what writes its asks, and each line it says, parsed.
fn start(socket: &Path) -> (Child, ChildStdin, Receiver<Value>) {
    let words = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../quickshell/launcher/words.json"
    );
    let mut source = Command::new(env!("CARGO_BIN_EXE_transcripts-people"))
        .arg(socket)
        .env("LAUNCHER_WORDS", words)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("transcripts-people starts");
    let asks = source.stdin.take().expect("its stdin");
    let lines = BufReader::new(source.stdout.take().expect("its stdout"));
    let (to, said) = mpsc::channel();
    thread::spawn(move || {
        for line in lines.lines().map_while(Result::ok) {
            let parsed = serde_json::from_str(&line).expect("each line it says is JSON");
            if to.send(parsed).is_err() {
                break;
            }
        }
    });
    (source, asks, said)
}

#[test]
fn an_address_is_answered_with_the_calls_with_it_and_again_as_they_change() {
    let folder = std::env::temp_dir().join(format!("transcripts-people-{}", std::process::id()));
    std::fs::create_dir_all(&folder).expect("a folder for the stand-in's socket");
    let socket = folder.join("calls.sock");
    let tell = stand_in(&socket);
    let ada = json!({
        "key": "call:person:ada@example.com:1000", "kind": "call", "title": "Ada Lovelace",
        "started": 1000, "ended": 1000 + 32 * 60_000, "event": null, "person": "Ada@Example.com",
        "folder": "/calls/ada", "transcript": "/calls/ada/transcript.md", "opening": null,
        "summary": null, "transcribing": false
    });
    let ben = json!({
        "key": "call:person:ben@example.com:2000", "kind": "call", "title": "", "started": 2000,
        "ended": null, "event": null, "person": "ben@example.com", "folder": "/calls/ben",
        "transcript": null, "opening": null, "summary": null, "transcribing": true
    });
    tell.send(json!({"id": 2, "records": [ada, ben]}))
        .expect("the stand-in runs");
    let (mut source, mut asks, said) = start(&socket);

    writeln!(asks, "?ada@example.com").expect("an ask ahead");
    let first = until(&said, "ada@example.com", |line| {
        line["between"]
            .as_array()
            .is_some_and(|between| !between.is_empty())
    });
    assert_eq!(first["known"], false, "it knows nobody: {first}");
    let between = first["between"].as_array().expect("what's between them");
    assert_eq!(
        between.len(),
        1,
        "only the call with her, by her address in any case"
    );
    let met = &between[0];
    assert_eq!(
        (&met["key"], &met["kind"], &met["at"], &met["row"]["name"]),
        (
            &json!("transcript:call:person:ada@example.com:1000"),
            &json!("met"),
            &json!(1000),
            &json!("Call With Ada Lovelace")
        ),
        "her call as the thing they met at: {met}"
    );

    tell.send(json!({"id": 2, "changed": [
        {"key": "call:person:ada@example.com:1000", "summary": "- Ship it"}
    ]}))
    .expect("the stand-in runs");
    let summed = until(&said, "ada@example.com", |line| {
        line["between"][0]["row"]["drawer"]["body"] == "\u{2022} Ship it"
    });
    assert_eq!(
        summed["between"].as_array().map(Vec::len),
        Some(1),
        "still one call"
    );

    writeln!(asks, "ben@example.com").expect("a step-in");
    let writing = until(&said, "ben@example.com", |_| true);
    assert_eq!(
        writing["between"][0]["row"]["note"], "being written down",
        "a call being written says so: {writing}"
    );
    writeln!(asks, "carol@example.com").expect("a step-in");
    let nobody = until(&said, "carol@example.com", |_| true);
    assert_eq!(
        nobody["between"],
        json!([]),
        "no call with her, said at once"
    );

    tell.send(json!({"id": 2, "gone": ["call:person:ada@example.com:1000"]}))
        .expect("the stand-in runs");
    until(&said, "ada@example.com", |line| {
        line["between"] == json!([])
    });

    drop(asks);
    let ended = source.wait().expect("transcripts-people ends");
    assert!(
        ended.success(),
        "it ends cleanly once nothing writes: {ended}"
    );
    let _ = std::fs::remove_dir_all(&folder);
}
