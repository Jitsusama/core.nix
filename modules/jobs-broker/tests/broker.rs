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

/// The broker serving a folder of its own, for as long as it's held: a
/// `Daemon` dropped stops it.
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

/// Whether this kernel names a socket's peer by pidfd (Linux 6.5 on).
fn kernel_names_peers() -> bool {
    use std::os::fd::AsRawFd;
    let (ours, _theirs) = std::os::unix::net::UnixStream::pair().expect("a socket pair");
    let mut fd: libc::c_int = -1;
    let mut length = libc::socklen_t::try_from(size_of::<libc::c_int>()).expect("an int's size");
    // SAFETY: SO_PEERPIDFD (77) writes one descriptor into an int of the
    // size `length` gives, and the socket is open for the call.
    let read = unsafe {
        libc::getsockopt(
            ours.as_raw_fd(),
            libc::SOL_SOCKET,
            77,
            (&raw mut fd).cast(),
            &raw mut length,
        )
    };
    if read == 0 && fd >= 0 {
        // SAFETY: the kernel just handed this descriptor to this process.
        unsafe { libc::close(fd) };
        return true;
    }
    false
}

fn code(answer: &Value) -> &str {
    answer["error"]["code"].as_str().unwrap_or("")
}

#[test]
fn hello_names_the_broker() {
    let daemon = broker("hello");
    let mut client = daemon.client();
    let hello = client.ask(&json!({"id": 1, "ask": "hello"}));
    assert_eq!(hello["domain"], "jobs-broker", "{hello}");
}

#[test]
fn a_scope_is_never_made_for_a_process_outside_agents_work() {
    let daemon = broker("scope");
    let mut client = daemon.client();
    // The test runs in a build's cgroup, or a terminal's, never an agent's
    // here, since its process isn't started by pi.
    let answer = client.ask(&json!({"id": 1, "ask": "scope", "quest": "q1"}));
    assert!(answer.get("scope").is_none(), "no scope was made: {answer}");
    // The lent builders run Linux 6.1, which names no peer by pidfd, so
    // there the refusal can only say so.
    let unnamed = answer.to_string().contains("names no peer by pidfd");
    assert_ne!(
        unnamed,
        kernel_names_peers(),
        "refused for want of a pidfd only where the kernel has none: {answer}"
    );
    if unnamed {
        return;
    }
    assert_ne!(
        code(&answer),
        "no-peer",
        "the kernel named who asked: {answer}"
    );
    assert!(!code(&answer).is_empty(), "refused with a reason: {answer}");
}

#[test]
fn a_status_says_only_of_units() {
    let daemon = broker("status");
    let mut client = daemon.client();
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
    let daemon = broker("unknown");
    let mut client = daemon.client();
    for ask in ["stop", "start", "restart"] {
        let answer = client.ask(&json!({"id": 1, "ask": ask, "unit": "niri.service"}));
        assert_eq!(code(&answer), "unknown-ask", "{ask}: {answer}");
    }
}
