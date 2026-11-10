//! 1Password's manager run as Passwords runs it, against a stand-in op
//! that says what op says while the app won't answer it, and 1Password's
//! settings file as the app writes it: with Integrate with 1Password CLI
//! off, every answer says it's refused by that setting, and once it's
//! turned on, the next says only that 1Password asks to let it in.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};

use serde_json::{Value, json};

/// What the manager says while 1Password keeps its command out.
const KEPT_OUT: &str =
    "1Password needs Integrate with 1Password CLI on, in Settings \u{203a} Developer";
/// What it says while 1Password only waits on Joel's leave.
const LET_IN: &str = "1Password asks you to let the launcher in";
/// As the app writes its settings with the setting turned on.
const ON: &str = r#"{"version": 1, "developers.cliSharedLockState.enabled": true}"#;
/// As a new machine's app writes them, the setting never turned on.
const OFF: &str = r#"{"version": 1, "security.authenticatedUnlock.enabled": true}"#;

/// A home of its own, with 1Password's settings in it and a stand-in op
/// that answers as op does while the app won't let it in.
struct Home {
    root: PathBuf,
}

impl Home {
    fn new(name: &str, settings: &str) -> Self {
        let root = std::env::temp_dir().join(format!("kept-out-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("config/1Password/settings")).expect("a home is made");
        let op = root.join("op");
        fs::write(
            &op,
            "#!/bin/sh\necho '[ERROR] 2026/11/09 18:00:00 account is not signed in' >&2\nexit 1\n",
        )
        .expect("the stand-in op is written");
        fs::set_permissions(&op, fs::Permissions::from_mode(0o755)).expect("op may run");
        let home = Self { root };
        home.set(settings);
        home
    }

    fn set(&self, settings: &str) {
        fs::write(self.settings(), settings).expect("the settings are written");
    }

    fn settings(&self) -> PathBuf {
        self.root.join("config/1Password/settings/settings.json")
    }

    fn start(&self) -> (Child, BufReader<ChildStdout>) {
        let mut child = Command::new(env!("CARGO_BIN_EXE_passwords-1password"))
            .arg("--op")
            .arg(self.root.join("op"))
            .env("HOME", &self.root)
            .env("XDG_CONFIG_HOME", self.root.join("config"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("the manager starts");
        let out = BufReader::new(child.stdout.take().expect("its output is read"));
        (child, out)
    }
}

impl Drop for Home {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn heard(out: &mut BufReader<ChildStdout>) -> Value {
    let mut line = String::new();
    out.read_line(&mut line).expect("the manager answers");
    serde_json::from_str(&line).unwrap_or_else(|why| panic!("{line:?} isn't JSON: {why}"))
}

fn asked(child: &mut Child, out: &mut BufReader<ChildStdout>, ask: &Value) -> Value {
    let stdin = child.stdin.as_mut().expect("it's asked on its input");
    writeln!(stdin, "{ask}").expect("the ask is written");
    heard(out)
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
    let (mut child, mut out) = home.start();
    let starting = heard(&mut out);
    assert_eq!(
        stands(&starting),
        ("refused", KEPT_OUT),
        "as it starts: {starting}"
    );
    let let_in = json!({"id": 1, "ask": "let-in"});
    let refused = asked(&mut child, &mut out, &let_in);
    assert_eq!(
        stands(&refused),
        ("refused", KEPT_OUT),
        "unlocking: {refused}"
    );
    assert_eq!(refused["error"]["message"], KEPT_OUT, "said why: {refused}");
    // Joel turns it on, and the very next answer says only his leave is wanted.
    home.set(ON);
    let waiting = asked(&mut child, &mut out, &let_in);
    assert_eq!(
        stands(&waiting),
        ("signed-out", LET_IN),
        "turned on: {waiting}"
    );
    drop(child.stdin.take());
    let _ = child.wait();
}

#[test]
fn let_in_it_starts_waiting_only_on_joels_leave() {
    let home = Home::new("on", ON);
    let (mut child, mut out) = home.start();
    let starting = heard(&mut out);
    assert_eq!(
        stands(&starting),
        ("signed-out", LET_IN),
        "as it starts: {starting}"
    );
    drop(child.stdin.take());
    let _ = child.wait();
}

#[test]
fn a_settings_file_not_there_says_nothing_of_the_setting() {
    let home = Home::new("unread", ON);
    fs::remove_file(home.settings()).expect("the settings go");
    assert!(
        !Path::new(&home.settings()).exists(),
        "no settings are left"
    );
    let (mut child, mut out) = home.start();
    let starting = heard(&mut out);
    assert_eq!(
        stands(&starting),
        ("signed-out", LET_IN),
        "as it starts: {starting}"
    );
    let said = asked(&mut child, &mut out, &json!({"id": 1, "ask": "let-in"}));
    assert_eq!(
        stands(&said),
        ("signed-out", LET_IN),
        "op's leave stands: {said}"
    );
    drop(child.stdin.take());
    let _ = child.wait();
}
