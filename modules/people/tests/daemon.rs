//! The daemon over its socket, with stand-in sources of its own: a get is
//! a step-in, answered within its patience; a watch asks ahead, says what's
//! held, then each change, a burst gathered into one; a source signed out
//! is never waited on; one that stops is started again and asked again
//! about everyone watched; and an ask or kind it doesn't know is refused.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// A source on the line protocol, in the shell the tests run in: it
/// writes each line it's asked to its log, and knows Ada, Grace and Eve
/// by their addresses, everyone else as nobody, and says a step-in apart
/// from an ask ahead.
const KNOWING: &str = r#"
log=$1
while read -r line; do
  printf '%s\n' "$line" >> "$log"
  case $line in
    '?'*) address=${line#?} way=ahead ;;
    *) address=$line way=step ;;
  esac
  case $address in
    ada@example.com|grace@example.com|eve@example.com)
      printf '{"email":"%s","name":"%s","way":"%s"}\n' "$address" "${address%@*}" "$way" ;;
    *) printf '{"email":"%s","id":null}\n' "$address" ;;
  esac
done
"#;

/// A source whose sign-in is refused, as slack-people says it.
const SIGNED_OUT: &str = r#"
while read -r line; do echo '{"signedOut":true}'; done
"#;

/// A source that answers one line, then stops, as a crash would.
const STOPPING: &str = r#"
log=$1
read -r line
printf '%s\n' "$line" >> "$log"
printf '{"email":"%s","name":"once"}\n' "${line#?}"
"#;

/// The daemon, running with these sources, and where its test keeps
/// what each source logged.
struct Running {
    daemon: Child,
    socket: PathBuf,
    folder: PathBuf,
}

impl Running {
    fn with(test: &str, sources: &[(&str, &str)]) -> Running {
        let folder = std::env::temp_dir().join(format!("people-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        let shell = std::env::var("BASH").unwrap_or_else(|_| "sh".to_owned());
        let listed: Vec<Value> = sources
            .iter()
            .map(|(name, script)| {
                let log = folder.join(format!("{name}.log"));
                json!({"name": name, "command": [shell, "-c", script, name, log]})
            })
            .collect();
        let config = folder.join("sources.json");
        std::fs::write(&config, json!({"sources": listed}).to_string()).unwrap();
        let socket = folder.join("daemons").join("people.sock");
        let daemon = Command::new(env!("CARGO_BIN_EXE_people"))
            .arg(&socket)
            .arg(&config)
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        until("the socket never came", || socket.exists());
        Running {
            daemon,
            socket,
            folder,
        }
    }

    fn client(&self) -> Client {
        let stream = UnixStream::connect(&self.socket).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        Client {
            lines: BufReader::new(stream.try_clone().unwrap()),
            stream,
        }
    }

    /// What a source was asked, a line each.
    fn asked(&self, source: &str) -> Vec<String> {
        std::fs::read_to_string(self.folder.join(format!("{source}.log")))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.daemon.kill();
        let _ = self.daemon.wait();
        let _ = std::fs::remove_dir_all(&self.folder);
    }
}

struct Client {
    stream: UnixStream,
    lines: BufReader<UnixStream>,
}

impl Client {
    /// The answer to an ask, passing over what a watch pushed meanwhile.
    fn ask(&mut self, asked: &Value) -> Value {
        writeln!(self.stream, "{asked}").unwrap();
        loop {
            let said = self.next();
            if said["id"] == asked["id"] {
                return said;
            }
        }
    }

    fn next(&mut self) -> Value {
        let mut line = String::new();
        self.lines.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    }
}

fn until(what: &str, held: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !held() {
        assert!(Instant::now() < deadline, "{what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn a_get_is_a_step_in_answered_by_every_source_within_its_patience() {
    let running = Running::with("get", &[("slack", KNOWING)]);
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "hello", "client": "test", "version": 1}));
    assert_eq!(
        (&said["id"], &said["version"], &said["domain"]),
        (&json!(1), &json!(1), &json!("people"))
    );
    let said = client
        .ask(&json!({"id": 2, "ask": "get", "keys": ["person:Ada@Example.com"], "patience": 5000}));
    assert_eq!(
        said["records"],
        json!([{"key": "person:ada@example.com", "kind": "person", "heard": said["records"][0]["heard"],
                "sources": {"slack": {"name": "ada", "way": "step"}}}])
    );
    assert_eq!(said["waiting"], json!([]));
    assert_eq!(running.asked("slack"), ["ada@example.com"]);
    let said = client.ask(&json!({"id": 3, "ask": "list"}));
    assert_eq!(said["records"].as_array().map(Vec::len), Some(1));
}

#[test]
fn a_watch_asks_ahead_says_what_is_held_and_gathers_a_burst_into_one_change() {
    let running = Running::with("watch", &[("slack", KNOWING)]);
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "watch", "kind": "person", "keys": [
        "person:ada@example.com", "person:grace@example.com", "person:eve@example.com"]}));
    assert_eq!(said["records"], json!([]));
    let mut changed = Vec::new();
    let mut messages = 0;
    while changed.len() < 3 {
        let said = client.next();
        assert_eq!(said["id"], 1);
        changed.extend(said["changed"].as_array().unwrap().iter().cloned());
        messages += 1;
    }
    assert!(messages <= 2, "three answers came as {messages} changes");
    assert!(
        changed
            .iter()
            .all(|record| record["sources"]["slack"]["way"] == "ahead")
    );
    assert_eq!(
        running.asked("slack"),
        ["?ada@example.com", "?grace@example.com", "?eve@example.com"]
    );
    // Watched again, everyone is held already, and nobody is asked again.
    let said = client.ask(&json!({"id": 2, "ask": "watch", "keys": ["person:ada@example.com"]}));
    assert_eq!(said["records"][0]["key"], "person:ada@example.com");
    assert_eq!(running.asked("slack").len(), 3);
    // The same watch asked again by its id replaces the one before, and
    // one unwatched is gone.
    client.ask(&json!({"id": 1, "ask": "watch", "keys": ["person:grace@example.com"]}));
    client.ask(&json!({"id": 2, "ask": "unwatch"}));
    assert_eq!(
        client.ask(&json!({"id": 3, "ask": "health"}))["watching"],
        1
    );
}

#[test]
fn a_source_signed_out_is_never_waited_on() {
    let running = Running::with("signed-out", &[("slack", KNOWING), ("vault", SIGNED_OUT)]);
    let mut client = running.client();
    // The first ask has the refused source say so.
    client
        .ask(&json!({"id": 1, "ask": "get", "keys": ["person:ada@example.com"], "patience": 2000}));
    let began = Instant::now();
    let said = client.ask(
        &json!({"id": 2, "ask": "get", "keys": ["person:grace@example.com"], "patience": 5000}),
    );
    assert!(
        began.elapsed() < Duration::from_secs(4),
        "it waited on a source signed out"
    );
    assert_eq!(said["records"][0]["sources"]["slack"]["name"], "grace");
    let said = client.ask(&json!({"id": 3, "ask": "hello"}));
    assert_eq!(
        said["sources"],
        json!({"slack": "answering", "vault": "signed-out"})
    );
}

#[test]
fn a_source_that_stops_is_started_again_and_asked_again_about_everyone_watched() {
    let running = Running::with("stopping", &[("slack", STOPPING)]);
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "watch", "keys": ["person:ada@example.com"]}));
    until("the source wasn't asked again once started again", || {
        running.asked("slack").len() >= 2
    });
    assert!(
        running
            .asked("slack")
            .iter()
            .all(|line| line == "?ada@example.com")
    );
    let said = client.ask(&json!({"id": 2, "ask": "health"}));
    assert!(said["sources"]["slack"]["state"].is_string());
}

#[test]
fn a_get_nothing_has_answered_in_time_is_not_ready() {
    let silent = "while read -r line; do :; done";
    let running = Running::with("silent", &[("slack", silent)]);
    let mut client = running.client();
    let said = client
        .ask(&json!({"id": 1, "ask": "get", "keys": ["person:ada@example.com"], "patience": 100}));
    assert_eq!(said["error"]["code"], "not-ready");
}

#[test]
fn an_ask_or_a_kind_it_doesnt_know_is_refused_by_its_code() {
    let running = Running::with("refused", &[("slack", KNOWING)]);
    let mut client = running.client();
    assert_eq!(
        client.ask(&json!({"id": 1, "ask": "shout"}))["error"]["code"],
        "unknown-ask"
    );
    assert_eq!(
        client.ask(&json!({"id": 2, "ask": "list", "kind": "file"}))["error"]["code"],
        "unknown-kind"
    );
}
