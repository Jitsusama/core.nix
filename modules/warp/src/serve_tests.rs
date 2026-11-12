use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use super::*;
use crate::contract::{self, Contract};

/// warp-cli as the contract answers it, but for a link that goes down
/// once it's asked to disconnect or the test takes it down, listening
/// once, where the test gives it lines to tell.
struct Switching {
    cli: Contract,
    up: Arc<AtomicBool>,
    listened: Mutex<Option<std::io::PipeReader>>,
    /// How many questions other than the status it was asked.
    asked: Arc<AtomicUsize>,
}

impl Ask for Switching {
    fn listen(&self) -> Option<Listening> {
        let lines = lock(&self.listened).take()?;
        Some(Listening {
            lines: Box::new(BufReader::new(lines)),
            stop: Box::new(|| {}),
        })
    }

    fn ask(&self, question: &[&str]) -> Result<Said, Unasked> {
        match question {
            ["status"] => Ok(Said {
                ok: true,
                text: if self.up.load(Ordering::SeqCst) {
                    r#"{"status": "Connected", "reason": "NetworkHealthy"}"#
                } else {
                    r#"{"status": "Disconnected", "reason": "Manual"}"#
                }
                .into(),
            }),
            ["disconnect"] => {
                self.up.store(false, Ordering::SeqCst);
                Ok(Said {
                    ok: true,
                    text: "Success\n".into(),
                })
            }
            _ => {
                self.asked.fetch_add(1, Ordering::SeqCst);
                self.cli.ask(question)
            }
        }
    }
}

struct Client {
    lines: BufReader<UnixStream>,
    stream: UnixStream,
}

impl Client {
    fn new(socket: &Path) -> Self {
        let started = Instant::now();
        let stream = loop {
            match UnixStream::connect(socket) {
                Ok(stream) => break stream,
                Err(why) => {
                    assert!(
                        started.elapsed() < Duration::from_secs(10),
                        "the socket never answered: {why}"
                    );
                    // The daemon binds its socket on a thread of its
                    // own; a moment is long enough for it.
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .expect("a timeout can be set");
        Self {
            lines: BufReader::new(stream.try_clone().expect("it clones")),
            stream,
        }
    }

    fn say(&mut self, said: &Value) {
        writeln!(self.stream, "{said}").expect("the daemon reads");
    }

    fn heard(&mut self) -> Value {
        let mut line = String::new();
        self.lines.read_line(&mut line).expect("the daemon says");
        serde_json::from_str(&line).expect("a line is JSON")
    }
}

fn serving(name: &str) -> std::path::PathBuf {
    serving_listened(name, None, Arc::new(AtomicBool::new(true)))
}

fn serving_listened(
    name: &str,
    listened: Option<std::io::PipeReader>,
    up: Arc<AtomicBool>,
) -> std::path::PathBuf {
    serving_counted(name, listened, up, Arc::new(AtomicUsize::new(0)))
}

fn serving_counted(
    name: &str,
    listened: Option<std::io::PipeReader>,
    up: Arc<AtomicBool>,
    asked: Arc<AtomicUsize>,
) -> std::path::PathBuf {
    let socket = std::env::temp_dir().join(format!("warp-{name}-{}.sock", std::process::id()));
    let cli = Switching {
        cli: Contract::whole(),
        up,
        listened: Mutex::new(listened),
        asked,
    };
    let at = socket.clone();
    std::thread::spawn(move || serve(&at, cli));
    socket
}

#[test]
fn a_state_watch_hears_how_warp_stands_from_the_listener_alone() {
    let (lines, mut tell) = std::io::pipe().expect("a pipe opens");
    let asked = Arc::new(AtomicUsize::new(0));
    let socket = serving_counted(
        "state",
        Some(lines),
        Arc::new(AtomicBool::new(true)),
        Arc::clone(&asked),
    );
    let mut client = Client::new(&socket);
    client.say(&json!({"id": "all", "ask": "watch", "kind": "warp-state"}));
    assert_eq!(
        client.heard(),
        json!({"id": "all", "records": []}),
        "nothing heard yet is said as nothing"
    );
    let told = contract::file("listen-status.jsonl");
    let told: Vec<&str> = told.lines().collect();
    writeln!(tell, "{}", told[0]).expect("the daemon listens");
    let first = client.heard();
    let state = &first["changed"][0];
    assert_eq!(state["key"], "warp:state");
    assert_eq!(state["kind"], "warp-state");
    assert_eq!(
        state["said"],
        json!({"status": "Connected", "reason": "NetworkHealthy"})
    );
    assert_eq!(
        state["since_ms"],
        Value::Null,
        "a state heard first may have stood for hours, so it says no since"
    );
    writeln!(tell, "{}", told[1]).expect("the daemon listens");
    let degraded = client.heard();
    assert_eq!(
        degraded["changed"][0]["said"],
        json!({"status": "Connected", "reason": "NetworkDegraded"})
    );
    assert!(
        degraded["changed"][0]["since_ms"].is_u64(),
        "a state seen to begin says when: {degraded}"
    );
    // Its figures moving tell nothing, so the next heard is the drop.
    writeln!(tell, "{}", told[2]).expect("the daemon listens");
    writeln!(tell, "{}", told[3]).expect("the daemon listens");
    assert_eq!(
        client.heard()["changed"][0]["said"],
        json!({"status": "Disconnected", "reason": "ManualDisconnection"})
    );
    assert_eq!(
        asked.load(Ordering::SeqCst),
        0,
        "a state watch sets off none of the link's reads"
    );
}

#[test]
fn a_status_asked_says_how_warp_stands_or_why_it_cant() {
    let asked = |ok, text: &str| {
        standing(Ok(Said {
            ok,
            text: text.into(),
        }))
    };
    assert_eq!(
        asked(true, &contract::file("status.degraded.json")),
        json!({"status": "Connected", "reason": "NetworkDegraded"})
    );
    assert_eq!(
        asked(false, &contract::file("daemon-down.json")),
        json!({"error": "FailedToConnectToDaemon"}),
        "WARP's service down is said by warp-cli's own code"
    );
    assert_eq!(asked(true, ""), json!({"error": "unreadable"}));
    assert_eq!(
        standing(Err(Unasked::Missing("warp-cli".into()))),
        json!({"error": "missing"})
    );
}

#[test]
fn a_state_watch_is_told_the_status_asked_where_nothing_listens() {
    let socket = serving_listened("unlistened", None, Arc::new(AtomicBool::new(false)));
    let mut client = Client::new(&socket);
    client.say(&json!({"id": "all", "ask": "watch", "kind": ["warp-state"]}));
    assert_eq!(client.heard()["records"], json!([]));
    assert_eq!(
        client.heard()["changed"][0]["said"],
        json!({"status": "Disconnected", "reason": "Manual"})
    );
}

#[test]
fn a_state_watch_is_told_again_within_seconds_as_the_status_asked_changes() {
    let up = Arc::new(AtomicBool::new(false));
    let socket = serving_listened("healing", None, Arc::clone(&up));
    let mut client = Client::new(&socket);
    client.say(&json!({"id": "all", "ask": "watch", "kind": ["warp-state"]}));
    assert_eq!(client.heard()["records"], json!([]));
    assert_eq!(
        client.heard()["changed"][0]["said"]["status"],
        "Disconnected"
    );
    up.store(true, Ordering::SeqCst);
    let started = Instant::now();
    let healed = client.heard();
    assert_eq!(
        healed["changed"][0]["said"],
        json!({"status": "Connected", "reason": "NetworkHealthy"}),
        "a link back is told"
    );
    assert!(
        started.elapsed() < Duration::from_secs(7),
        "it was told only after {:?}",
        started.elapsed()
    );
    assert!(
        healed["changed"][0]["since_ms"].is_u64(),
        "a state seen to begin says when"
    );
}

#[test]
fn only_a_status_or_reason_other_than_the_last_sets_a_read_off() {
    let told = contract::file("listen-status.jsonl");
    let told: Vec<&str> = told.lines().collect();
    let mut last = None;
    assert!(
        !changed(told[0], &mut last),
        "the first says only how it stands"
    );
    assert!(!changed(told[0], &mut last), "the same again is no change");
    assert!(changed(told[1], &mut last), "a link degrading is a change");
    assert!(
        !changed(told[2], &mut last),
        "a degraded link's figures moving are the two-second reads'"
    );
    assert!(
        !changed("resolved example.com", &mut last),
        "a DNS log sets nothing off"
    );
    assert!(changed(told[3], &mut last), "a link going down is a change");
}

#[test]
fn a_change_warp_tells_is_read_at_once() {
    let (lines, mut tell) = std::io::pipe().expect("a pipe opens");
    let up = Arc::new(AtomicBool::new(true));
    let mut client = Client::new(&serving_listened("listened", Some(lines), Arc::clone(&up)));
    client.say(&json!({"id": "all", "ask": "watch", "kind": "warp"}));
    assert_eq!(client.heard()["records"], json!([]));
    let first = client.heard();
    assert_eq!(first["changed"][0]["said"]["status"]["state"], "Connected");
    let told = contract::file("listen-status.jsonl");
    let told: Vec<&str> = told.lines().collect();
    writeln!(tell, "{}", told[0]).expect("the daemon listens");
    up.store(false, Ordering::SeqCst);
    writeln!(tell, "{}", told[3]).expect("the daemon listens");
    let next = client.heard();
    assert_eq!(
        next["changed"][0]["said"]["status"]["state"],
        "Disconnected"
    );
    let read_at = |told: &Value| told["changed"][0]["read_at_ms"].as_u64().unwrap_or(0);
    let between = read_at(&next).saturating_sub(read_at(&first));
    assert!(
        between < 1000,
        "the link is read as WARP tells it went down, not when the next read comes due \
         two seconds on, but {between} ms after the first"
    );
}

#[test]
fn a_watch_hears_the_link_as_read_and_again_after_a_switch() {
    let mut client = Client::new(&serving("watch"));
    client.say(&json!({"id": "hello", "ask": "hello", "version": 1}));
    assert_eq!(client.heard()["domain"], "warp");
    client.say(&json!({"id": "all", "ask": "watch", "kind": "warp"}));
    assert_eq!(
        client.heard(),
        json!({"id": "all", "records": []}),
        "nothing read yet is said as nothing, not as an old link"
    );
    let first = client.heard();
    assert_eq!(first["changed"][0]["key"], "warp:link");
    assert_eq!(first["changed"][0]["said"]["status"]["state"], "Connected");
    client.say(&json!({"id": 2, "ask": "act", "act": "disconnect"}));
    // A read come due while switching may land before the answer.
    let answer = loop {
        let heard = client.heard();
        if heard["id"] == 2 {
            break heard;
        }
    };
    assert_eq!(answer["switched"]["act"], "disconnect");
    assert_eq!(answer["switched"]["done"], true);
    assert_eq!(
        client.heard()["changed"][0]["said"]["status"]["state"],
        "Disconnected",
        "the link is read again as the switch is answered"
    );
}

#[test]
fn an_ask_kind_or_act_it_doesnt_know_is_refused() {
    let mut client = Client::new(&serving("refused"));
    client.say(&json!({"id": 1, "ask": "watch", "kind": "disk"}));
    assert_eq!(client.heard()["error"]["code"], "unknown-kind");
    client.say(&json!({"id": 2, "ask": "act", "act": "reconnect"}));
    assert_eq!(client.heard()["error"]["code"], "unknown-act");
    client.say(&json!({"id": 3, "ask": "overclock"}));
    assert_eq!(client.heard()["error"]["code"], "unknown-ask");
}

#[test]
fn a_get_reads_afresh_what_isnt_held_fresh() {
    let mut client = Client::new(&serving("get"));
    client.say(&json!({"id": 1, "ask": "get", "keys": ["warp:link"]}));
    let got = client.heard();
    assert_eq!(got["id"], 1);
    assert_eq!(got["records"][0]["said"]["status"]["state"], "Connected");
}
