//! A secret released to one command, as Joel's yes to an agent's ask
//! has Passwords ask 1Password's manager for it: only a reference its
//! own configuration lists, matched byte for byte and never with a
//! query, read by op only on Joel's own ask, and the value said once,
//! in the answer, never in what the manager logs.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use serde_json::{Value, json};

/// The one reference the manager is started listing.
const LISTED: &str = "op://Agents/GitHub token/credential";
/// What the stand-in op reads for any reference: made up, and looking it.
const SECRET: &str = "made-up-secret-0123";
/// How long a line may take to come, well past the manager's own waits.
const PATIENCE: Duration = Duration::from_secs(20);

/// The manager started listing [`LISTED`] alone, beside a stand-in op
/// that logs each run's arguments and reads [`SECRET`] for any of them.
struct Released {
    root: PathBuf,
    child: Child,
    lines: Receiver<Value>,
}

impl Released {
    fn start(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("release-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("a home is made");
        let op = root.join("op");
        fs::write(
            &op,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\nprintf '%s' '{SECRET}'\n",
                root.join("op.log").display()
            ),
        )
        .expect("the stand-in op is written");
        fs::set_permissions(&op, fs::Permissions::from_mode(0o755)).expect("it may run");
        let mut child = Command::new(env!("CARGO_BIN_EXE_passwords-1password"))
            .arg("--op")
            .arg(&op)
            .args(["--release", LISTED])
            .env("HOME", &root)
            .env("XDG_CONFIG_HOME", root.join("config"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(fs::File::create(root.join("logged")).expect("its log is kept"))
            .spawn()
            .expect("the manager starts");
        let out = BufReader::new(child.stdout.take().expect("its output is read"));
        let (said, lines) = channel();
        std::thread::spawn(move || {
            for line in out.lines() {
                let Ok(line) = line else { break };
                let heard: Value = serde_json::from_str(&line)
                    .unwrap_or_else(|why| panic!("{line:?} isn't JSON: {why}"));
                if said.send(heard).is_err() {
                    break;
                }
            }
        });
        Self { root, child, lines }
    }

    /// Its answer to a release of `reference`, Joel's own where `asked`.
    fn release(&mut self, id: u64, reference: &str, asked: bool) -> Value {
        let ask = json!({"id": id, "ask": "release", "reference": reference, "asked": asked});
        let stdin = self.child.stdin.as_mut().expect("it's asked on its input");
        writeln!(stdin, "{ask}").expect("the ask is written");
        loop {
            let said = self
                .lines
                .recv_timeout(PATIENCE)
                .expect("the manager answers in time");
            if said["id"] == json!(id) {
                return said;
            }
        }
    }

    /// Each run of op so far, its arguments a line.
    fn ran(&self) -> String {
        fs::read_to_string(self.root.join("op.log")).unwrap_or_default()
    }

    /// What the manager logged so far.
    fn logged(&self) -> String {
        fs::read_to_string(self.root.join("logged")).unwrap_or_default()
    }
}

impl Drop for Released {
    fn drop(&mut self) {
        drop(self.child.stdin.take());
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn a_listed_reference_joel_asked_for_is_read_once_and_answered_unlogged() {
    let mut manager = Released::start("listed");
    let answer = manager.release(1, LISTED, true);
    assert_eq!(answer["value"], SECRET, "the value answers it: {answer}");
    assert_eq!(
        manager.ran(),
        format!("read --no-newline {LISTED}\n"),
        "op reads it, once and without a newline"
    );
    assert!(
        !manager.logged().contains(SECRET),
        "the value is never logged: {}",
        manager.logged()
    );
}

#[test]
fn a_reference_not_listed_is_refused_before_op_runs() {
    let mut manager = Released::start("unlisted");
    for (id, reference) in [
        (1, "op://Agents/Bank/password"),
        // No normalising: case, a trailing slash or a space reads apart.
        (2, "op://agents/GitHub token/credential"),
        (3, "op://Agents/GitHub token/credential/"),
        (4, " op://Agents/GitHub token/credential"),
        (5, ""),
    ] {
        let answer = manager.release(id, reference, true);
        assert_eq!(
            answer["error"]["code"], "refused",
            "{reference:?} is refused: {answer}"
        );
        assert_eq!(answer["error"]["message"], "not listed", "{answer}");
        assert!(answer.get("value").is_none(), "no value: {answer}");
    }
    assert_eq!(manager.ran(), "", "op never ran");
}

#[test]
fn a_query_on_a_listed_reference_is_refused_before_op_runs() {
    let mut manager = Released::start("query");
    let answer = manager.release(1, &format!("{LISTED}?attribute=otp"), true);
    assert_eq!(answer["error"]["code"], "refused", "{answer}");
    assert!(answer.get("value").is_none(), "no value: {answer}");
    assert_eq!(manager.ran(), "", "op never ran");
}

#[test]
fn a_release_not_asked_by_joel_is_refused_before_op_runs() {
    let mut manager = Released::start("unasked");
    let answer = manager.release(1, LISTED, false);
    assert_eq!(answer["error"]["code"], "refused", "{answer}");
    assert!(answer.get("value").is_none(), "no value: {answer}");
    assert_eq!(manager.ran(), "", "op never ran");
}
