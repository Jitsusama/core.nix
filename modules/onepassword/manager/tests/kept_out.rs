//! 1Password's manager run as Passwords runs it, against a stand-in op
//! that says what op says while the app won't answer it, 1Password's
//! settings file as the app writes it, and the file its unit keeps while
//! the app is open: with Integrate with 1Password CLI off, it says it's
//! refused by that setting, and once it's turned on, only that 1Password
//! asks to let it in, by itself, with nothing asked (2026-11-10, Joel
//! found the refusal kept after he turned it on). While the app
//! is closed it says so, and Joel's own ask opens it first.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use serde_json::{Value, json};

/// What the manager says while 1Password keeps its command out.
const KEPT_OUT: &str =
    "1Password needs Integrate with 1Password CLI on, in Settings \u{203a} Developer";
/// What it says while 1Password only waits on Joel's leave.
const LET_IN: &str = "1Password asks you to let the launcher in";
/// What it says while the app is closed.
// What it says before anything has asked 1Password, as a fresh manager does.
const UNASKED: &str = "1Password is asked as you next open Passwords";
const CLOSED: &str = "1Password is closed, and Unlock opens it";
/// What it says while it opens the app for Joel's ask.
const OPENING: &str = "1Password is opening, to let the launcher in";
/// As the app writes its settings with the setting turned on.
const ON: &str = r#"{"version": 1, "developers.cliSharedLockState.enabled": true}"#;
/// As a new machine's app writes them, the setting never turned on.
const OFF: &str = r#"{"version": 1, "security.authenticatedUnlock.enabled": true}"#;
/// What op says, as it does while the app won't let it in.
const NOT_SIGNED_IN: &str = "account is not signed in";
/// How long a line may take to come, well past the manager's own waits.
const PATIENCE: Duration = Duration::from_secs(20);

/// A home of its own, with 1Password's settings in it, a stand-in op that
/// answers as op does while the app won't let it in, and a stand-in
/// systemctl that opens the app by making the file its unit keeps.
struct Home {
    root: PathBuf,
}

impl Home {
    fn new(name: &str, settings: &str) -> Self {
        let root = std::env::temp_dir().join(format!("kept-out-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("config/1Password/settings")).expect("a home is made");
        fs::create_dir_all(root.join("run/onepassword")).expect("a runtime folder is made");
        let home = Self { root };
        home.op(&format!(
            "echo '[ERROR] 2026/11/09 18:00:00 {NOT_SIGNED_IN}' >&2\nexit 1"
        ));
        home.script(
            "systemctl",
            &format!(
                "echo \"$*\" >> '{}'\ntouch '{}'",
                home.root.join("systemctl.log").display(),
                home.opened().display()
            ),
        );
        home.set(settings);
        home
    }

    fn script(&self, name: &str, body: &str) {
        let path = self.root.join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("a stand-in is written");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("it may run");
    }

    /// The stand-in op, as a shell script's body.
    fn op(&self, body: &str) {
        self.script("op", body);
    }

    fn set(&self, settings: &str) {
        fs::write(self.settings(), settings).expect("the settings are written");
    }

    fn settings(&self) -> PathBuf {
        self.root.join("config/1Password/settings/settings.json")
    }

    /// The file 1Password's unit keeps while the app runs.
    fn opened(&self) -> PathBuf {
        self.root.join("run/onepassword/open")
    }

    /// What systemctl was asked, a line an ask.
    fn opens(&self) -> String {
        fs::read_to_string(self.root.join("systemctl.log")).unwrap_or_default()
    }

    /// The manager, as Passwords runs it where the app isn't watched.
    fn start(&self) -> Manager {
        self.started(&[])
    }

    /// The manager, as Passwords runs it beside 1Password's unit.
    fn start_watching_the_app(&self) -> Manager {
        let opened = self.opened();
        let systemctl = self.root.join("systemctl");
        self.started(&[
            "--opened",
            opened.to_str().expect("a path"),
            "--open-unit",
            "onepassword.service",
            "--systemctl",
            systemctl.to_str().expect("a path"),
        ])
    }

    fn started(&self, more: &[&str]) -> Manager {
        let mut child = Command::new(env!("CARGO_BIN_EXE_passwords-1password"))
            .arg("--op")
            .arg(self.root.join("op"))
            .args(more)
            .env("HOME", &self.root)
            .env("XDG_CONFIG_HOME", self.root.join("config"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
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
        Manager { child, lines }
    }
}

impl Drop for Home {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// The manager running, its lines read as they come.
struct Manager {
    child: Child,
    lines: Receiver<Value>,
}

impl Manager {
    /// The next line it says, whatever it is.
    fn heard(&self) -> Value {
        self.lines
            .recv_timeout(PATIENCE)
            .expect("the manager says something in time")
    }

    /// Its answer to `ask`, and every standing it said by itself before.
    fn asked(&mut self, ask: &Value) -> (Value, Vec<Value>) {
        let stdin = self.child.stdin.as_mut().expect("it's asked on its input");
        writeln!(stdin, "{ask}").expect("the ask is written");
        let mut before = Vec::new();
        loop {
            let said = self.heard();
            if said.get("id") == ask.get("id") {
                return (said, before);
            }
            before.push(said);
        }
    }
}

impl Drop for Manager {
    fn drop(&mut self) {
        drop(self.child.stdin.take());
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn stands(said: &Value) -> (&str, &str) {
    (
        said["state"].as_str().unwrap_or(""),
        said["reason"].as_str().unwrap_or(""),
    )
}

#[test]
fn kept_out_it_says_the_setting_to_turn_on_and_let_in_once_it_is() {
    let home = Home::new("turned-on", OFF);
    let mut manager = home.start();
    let starting = manager.heard();
    assert_eq!(
        stands(&starting),
        ("refused", KEPT_OUT),
        "as it starts: {starting}"
    );
    let let_in = json!({"id": 1, "ask": "let-in"});
    let (refused, _) = manager.asked(&let_in);
    assert_eq!(
        stands(&refused),
        ("refused", KEPT_OUT),
        "unlocking: {refused}"
    );
    assert_eq!(refused["error"]["message"], KEPT_OUT, "said why: {refused}");
    // Joel turns it on, and the very next answer says only his leave is wanted.
    home.set(ON);
    let (waiting, _) = manager.asked(&let_in);
    assert_eq!(
        stands(&waiting),
        ("signed-out", LET_IN),
        "turned on: {waiting}"
    );
}

#[test]
fn turned_on_with_nothing_asked_it_says_so_by_itself() {
    let home = Home::new("turned-on-unasked", OFF);
    let manager = home.start();
    let starting = manager.heard();
    assert_eq!(
        stands(&starting),
        ("refused", KEPT_OUT),
        "as it starts: {starting}"
    );
    home.set(ON);
    let told = manager.heard();
    assert_eq!(
        stands(&told),
        ("signed-out", UNASKED),
        "once Joel turns it on: {told}"
    );
    assert!(told.get("id").is_none(), "said by itself: {told}");
}

#[test]
fn let_in_it_starts_waiting_only_on_joels_leave() {
    let home = Home::new("on", ON);
    let manager = home.start();
    let starting = manager.heard();
    assert_eq!(
        stands(&starting),
        ("signed-out", UNASKED),
        "as it starts: {starting}"
    );
}

#[test]
fn a_settings_file_not_there_says_nothing_of_the_setting() {
    let home = Home::new("unread", ON);
    fs::remove_file(home.settings()).expect("the settings go");
    assert!(
        !Path::new(&home.settings()).exists(),
        "no settings are left"
    );
    let mut manager = home.start();
    let starting = manager.heard();
    assert_eq!(
        stands(&starting),
        ("signed-out", UNASKED),
        "as it starts: {starting}"
    );
    let (said, _) = manager.asked(&json!({"id": 1, "ask": "let-in"}));
    assert_eq!(
        stands(&said),
        ("signed-out", LET_IN),
        "op's leave stands: {said}"
    );
}

#[test]
fn the_app_closing_is_said_at_once_and_opening_again_too() {
    let home = Home::new("closing", ON);
    fs::write(home.opened(), "").expect("the app is open");
    let manager = home.start_watching_the_app();
    let starting = manager.heard();
    assert_eq!(
        stands(&starting),
        ("signed-out", UNASKED),
        "as it starts: {starting}"
    );
    fs::remove_file(home.opened()).expect("the app closes");
    let closed = manager.heard();
    assert_eq!(stands(&closed), ("signed-out", CLOSED), "closed: {closed}");
    fs::write(home.opened(), "").expect("the app opens again");
    let open = manager.heard();
    assert_eq!(stands(&open), ("signed-out", UNASKED), "open: {open}");
}

#[test]
fn unlocking_while_the_app_is_closed_opens_it_first_and_says_so() {
    let home = Home::new("opening", ON);
    let mut manager = home.start_watching_the_app();
    let starting = manager.heard();
    assert_eq!(
        stands(&starting),
        ("signed-out", CLOSED),
        "as it starts closed: {starting}"
    );
    let (answer, before) = manager.asked(&json!({"id": 1, "ask": "let-in"}));
    assert!(
        before
            .iter()
            .any(|said| stands(said) == ("signed-out", OPENING)),
        "said it's opening 1Password before op ran: {before:?}"
    );
    assert_eq!(
        home.opens().trim(),
        "--user start onepassword.service",
        "1Password's unit is started"
    );
    assert_eq!(
        stands(&answer),
        ("signed-out", LET_IN),
        "then waits on Joel's leave: {answer}"
    );
}

#[test]
fn the_shells_own_list_never_opens_the_app() {
    let home = Home::new("unasked", ON);
    let mut manager = home.start_watching_the_app();
    let _ = manager.heard();
    let (answer, _) = manager.asked(&json!({"id": 1, "ask": "list"}));
    assert_eq!(home.opens(), "", "nothing was opened: {answer}");
    assert_eq!(
        stands(&answer),
        ("signed-out", CLOSED),
        "it says the app is closed: {answer}"
    );
}

#[test]
fn op_failing_otherwise_says_its_own_words_as_refused() {
    let home = Home::new("other", ON);
    home.op("echo '[ERROR] 2026/11/10 12:00:24 connecting to desktop app: gone' >&2\nexit 1");
    let mut manager = home.start();
    let _ = manager.heard();
    let (answer, _) = manager.asked(&json!({"id": 1, "ask": "list", "asked": true}));
    assert_eq!(
        stands(&answer),
        ("refused", "connecting to desktop app: gone"),
        "a standing for Passwords to say: {answer}"
    );
}
