//! The broker over its socket, with no systemd to reach: hello names it;
//! a scope asked by a process that isn't an agent's is refused, the
//! process named by the kernel rather than by the ask; a status refuses a
//! name that isn't a unit's, says a unit's couldn't be asked when systemd
//! can't be reached, and refuses a list too long; and an ask it doesn't
//! know is refused.

use std::path::PathBuf;
use std::process::Command;

use daemon_client::Daemon;
use serde_json::{Value, json};

fn broker(name: &str) -> Daemon {
    let folder: PathBuf =
        std::env::temp_dir().join(format!("jobs-broker-{name}-{}", std::process::id()));
    let socket = folder.join("jobs-broker.sock");
    let mut command = Command::new(env!("CARGO_BIN_EXE_jobs-broker"));
    command
        .args(["serve", "--socket"])
        .arg(&socket)
        // No systemd to reach, wherever the test runs.
        .env(
            "DBUS_SESSION_BUS_ADDRESS",
            format!("unix:path={}", folder.join("no-bus").display()),
        );
    Daemon::start(&mut command, socket, folder)
}

fn code(answer: &Value) -> &str {
    answer["error"]["code"].as_str().unwrap_or("")
}

#[test]
fn hello_names_the_broker() {
    let mut client = broker("hello").client();
    let hello = client.ask(&json!({"id": 1, "ask": "hello"}));
    assert_eq!(hello["domain"], "jobs-broker", "{hello}");
}

#[test]
fn a_scope_is_never_made_for_a_process_outside_agents_work() {
    let mut client = broker("scope").client();
    // The test runs in a build's cgroup, or a terminal's, never an agent's
    // here, since its process isn't started by pi.
    let answer = client.ask(&json!({"id": 1, "ask": "scope", "quest": "q1"}));
    assert!(answer.get("scope").is_none(), "no scope was made: {answer}");
    assert_ne!(
        code(&answer),
        "no-peer",
        "the kernel named who asked: {answer}"
    );
    assert!(!code(&answer).is_empty(), "refused with a reason: {answer}");
}

#[test]
fn a_status_says_only_of_units() {
    let mut client = broker("status").client();
    let answer = client.ask(&json!({
        "id": 1, "ask": "status", "units": ["jobs-sorter.service", "/etc/passwd", 7]
    }));
    let units = answer["units"].as_array().expect("one answer a unit");
    assert_eq!(units.len(), 3, "{answer}");
    let error = units[0]["error"].as_str().unwrap_or("");
    assert!(
        error.starts_with("systemd can't be asked"),
        "a unit's name, with no systemd to ask: {answer}"
    );
    for refused in &units[1..] {
        assert_eq!(refused["error"], "not a unit's name", "{answer}");
    }
    let many: Vec<String> = (0..33).map(|at| format!("u{at}.service")).collect();
    let too_many = client.ask(&json!({"id": 2, "ask": "status", "units": many}));
    assert_eq!(code(&too_many), "bad-units", "{too_many}");
}

#[test]
fn an_ask_it_doesnt_know_is_refused() {
    let mut client = broker("unknown").client();
    for ask in ["stop", "start", "restart"] {
        let answer = client.ask(&json!({"id": 1, "ask": ask, "unit": "niri.service"}));
        assert_eq!(code(&answer), "unknown-ask", "{ask}: {answer}");
    }
}
