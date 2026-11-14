//! `passwords keys` against a stand-in SSH agent each test starts itself,
//! listening where 1Password's listens, `~/.1password/agent.sock`, in a
//! home of the test's own, so 1Password's own agent is never reached. A
//! key is made for the test alone, with nothing past its being a key.
//!
//! The tests with an agent need OpenSSH's agent, ssh-add and ssh-keygen,
//! which the build gives `passwords-lists-1passwords-ssh-keys` alone, so a
//! bare `cargo test` passes them by; that check runs them with `--ignored`.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

/// A home of the test's own, and the stand-in agent listening in it.
struct Agent {
    agent: Option<Child>,
    home: PathBuf,
}

impl Agent {
    /// A home whose 1Password socket nothing listens on.
    fn none(test: &str) -> Agent {
        // Short, since a socket's path can't pass 108 bytes.
        let home = std::env::temp_dir().join(format!("pk-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(home.join(".1password")).unwrap();
        Agent { agent: None, home }
    }

    /// A home with the stand-in agent listening where 1Password's would.
    fn listening(test: &str) -> Agent {
        let mut agent = Agent::none(test);
        let socket = agent.socket();
        agent.agent = Some(
            Command::new("ssh-agent")
                .arg("-D")
                .arg("-a")
                .arg(&socket)
                .stdout(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let started = Instant::now();
        while !socket.exists() {
            assert!(
                started.elapsed() < Duration::from_secs(20),
                "the agent never listened"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        agent
    }

    fn socket(&self) -> PathBuf {
        self.home.join(".1password/agent.sock")
    }

    /// Makes a key for the test alone and gives it to the agent.
    fn holding(self, comment: &str) -> Agent {
        let key = key_file(&self.home, comment);
        let made = Command::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-C", comment, "-f"])
            .arg(&key)
            .status()
            .unwrap();
        assert!(made.success());
        let added = Command::new("ssh-add")
            .arg("-q")
            .arg(&key)
            .env("SSH_AUTH_SOCK", self.socket())
            .status()
            .unwrap();
        assert!(added.success());
        self
    }

    /// `passwords keys` with these words, run in the test's home, as a
    /// program reads it.
    fn keys(&self, words: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_passwords"))
            .arg("keys")
            .args(words)
            .env("HOME", &self.home)
            .env_remove("SSH_AUTH_SOCK")
            .output()
            .unwrap()
    }

    fn public(&self, comment: &str) -> String {
        let line =
            std::fs::read_to_string(key_file(&self.home, comment).with_extension("pub")).unwrap();
        line.split_whitespace()
            .take(2)
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl Drop for Agent {
    fn drop(&mut self) {
        if let Some(agent) = &mut self.agent {
            let _ = agent.kill();
            let _ = agent.wait();
        }
        let _ = std::fs::remove_dir_all(&self.home);
    }
}

/// Where the test keeps the key with this comment, by its comment.
fn key_file(home: &Path, comment: &str) -> PathBuf {
    home.join(format!("key-{}", comment.replace(' ', "-")))
}

fn said(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn fingerprint(home: &Path, comment: &str) -> String {
    let said = Command::new("ssh-keygen")
        .arg("-l")
        .arg("-f")
        .arg(key_file(home, comment).with_extension("pub"))
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&said.stdout).into_owned();
    said.split_whitespace().nth(1).unwrap().to_owned()
}

#[test]
#[ignore = "needs OpenSSH, which passwords-lists-1passwords-ssh-keys gives"]
fn each_key_the_agent_holds_is_a_row() {
    let agent = Agent::listening("rows")
        .holding("stand-in signing key")
        .holding("stand-in");
    let listed = agent.keys(&[]);
    assert_eq!(
        listed.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&listed.stderr)
    );
    // Read through a pipe, nothing gives way: a head counting the keys,
    // the table's head, then a row a key with every column.
    let said = said(&listed);
    let lines: Vec<&str> = said.lines().collect();
    assert_eq!(lines.len(), 5, "{said}");
    assert_eq!(lines[0], "1Password's SSH agent \u{b7} 2 keys");
    assert!(
        lines[2].starts_with("TYPE ") && lines[2].ends_with(" PUBLIC"),
        "{said}"
    );
    let first = lines[3];
    assert!(first.starts_with("ED25519   256  "), "{said}");
    for part in [
        fingerprint(&agent.home, "stand-in signing key"),
        "stand-in signing key".to_owned(),
        agent.public("stand-in signing key"),
    ] {
        assert!(
            first.contains(&part),
            "the first key's row lacks {part}: {said}"
        );
    }
    assert!(lines[4].contains("  stand-in  "), "{said}");
}

#[test]
#[ignore = "needs OpenSSH, which passwords-lists-1passwords-ssh-keys gives"]
fn json_gives_each_key_and_its_public_half() {
    let agent = Agent::listening("json").holding("stand-in signing key");
    let said: Value = serde_json::from_str(&said(&agent.keys(&["--json"]))).unwrap();
    assert_eq!(said["schema"], "passwords.keys/1");
    let keys = &said["keys"];
    assert_eq!(keys[0]["type"], "ED25519");
    assert_eq!(keys[0]["bits"], 256);
    assert_eq!(keys[0]["comment"], "stand-in signing key");
    assert_eq!(
        keys[0]["public"].as_str(),
        Some(agent.public("stand-in signing key").as_str())
    );
    assert!(
        keys[0]["fingerprint"]
            .as_str()
            .is_some_and(|fingerprint| fingerprint.starts_with("SHA256:"))
    );
}

#[test]
#[ignore = "needs OpenSSH, which passwords-lists-1passwords-ssh-keys gives"]
fn an_agent_holding_nothing_lists_nothing() {
    let agent = Agent::listening("empty");
    let listed = agent.keys(&[]);
    assert_eq!(listed.status.code(), Some(0));
    assert_eq!(said(&listed), "1Password's SSH agent \u{b7} 0 keys\n");
    let keys: Value = serde_json::from_str(&said(&agent.keys(&["--json"]))).unwrap();
    assert_eq!(keys["keys"], serde_json::json!([]));
}

/// Nothing listens, so nothing is run, and this needs no OpenSSH.
#[test]
fn no_agent_listening_is_said_with_how_to_turn_it_on() {
    let agent = Agent::none("none");
    let listed = agent.keys(&[]);
    assert_eq!(listed.status.code(), Some(69));
    let complaint = String::from_utf8_lossy(&listed.stderr);
    assert!(
        complaint.contains("1Password's SSH agent isn't answering")
            && complaint.contains("under Developer"),
        "{complaint}"
    );
}
