//! The daemon run over a folder of its own, with stand-ins for 1Password's
//! op and logind's loginctl, a git tree for asks to run in, and asked
//! over its socket as the launcher asks it.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use daemon_client::{Client, Daemon, until};
use serde_json::{Value, json};

/// The one item the daemon may release.
const ITEM: &str = "op://Private/Release Test/password";
/// The launcher's vocabulary, which the daemon says its rows in.
const VOCABULARY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../quickshell/launcher/words.json"
);
/// What the stand-in op says the item holds.
const VALUE: &str = "this-is-not-a-secret";

/// A program written as a shell script, for a stand-in.
fn script(path: &Path, body: &str) {
    std::fs::write(path, format!("#!/bin/sh\n{body}\n")).expect("the stand-in is written");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .expect("the stand-in runs");
}

/// A program's path on the test's PATH, since the daemon runs git with
/// none of its own.
fn on_path(program: &str) -> PathBuf {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .map(|folder| folder.join(program))
        .find(|path| path.is_file())
        .unwrap_or_else(|| panic!("{program} isn't on the test's PATH"))
}

fn git(tree: &Path, args: &[&str]) {
    let ran = Command::new("git")
        .arg("-C")
        .arg(tree)
        .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .expect("git runs");
    assert!(ran.status.success(), "git {args:?} failed: {ran:?}");
}

/// The daemon over a fresh folder: its asks, its socket, a committed
/// tree, and the stand-ins, op leaving a mark each time it runs.
struct Setting {
    daemon: Daemon,
    folder: PathBuf,
}

impl Setting {
    fn new(name: &str) -> Setting {
        let folder =
            std::env::temp_dir().join(format!("secret-asks-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(folder.join("tree")).expect("the folder is made");
        git(&folder.join("tree"), &["init", "-q"]);
        std::fs::write(folder.join("tree/README"), "a tree").expect("a file is written");
        git(&folder.join("tree"), &["add", "README"]);
        git(&folder.join("tree"), &["commit", "-q", "-m", "a commit"]);
        let marks = folder.join("op-ran");
        script(
            &folder.join("op"),
            &format!(
                "echo \"$@\" >> '{}'\n[ \"$3\" = '{ITEM}' ] || exit 1\nprintf '%s' '{VALUE}'",
                marks.display()
            ),
        );
        std::fs::write(folder.join("locked"), "no").expect("the lock is written");
        script(
            &folder.join("loginctl"),
            &format!(
                "case \"$1\" in show-user) echo 2 ;; show-session) cat '{}' ;; esac",
                folder.join("locked").display()
            ),
        );
        let socket = folder.join("secret-asks.sock");
        let mut command = Command::new(env!("CARGO_BIN_EXE_secret-asks"));
        command
            .arg("serve")
            .arg("--socket")
            .arg(&socket)
            .arg("--asks")
            .arg(folder.join("asks"))
            .arg("--vocabulary")
            .arg(VOCABULARY)
            .arg("--item")
            .arg(ITEM)
            .arg("--op")
            .arg(folder.join("op"))
            .arg("--git")
            .arg(on_path("git"))
            .arg("--loginctl")
            .arg(folder.join("loginctl"));
        let daemon = Daemon::start(&mut command, socket, folder.clone());
        Setting { daemon, folder }
    }

    /// Puts up an ask for `item` and answers its id.
    fn ask(&self, item: &str) -> String {
        let ran = Command::new(env!("CARGO_BIN_EXE_secret-asks"))
            .args(["ask", "--item", item, "--why", "To try the line.", "--asks"])
            .arg(self.folder.join("asks"))
            .arg("--tree")
            .arg(self.folder.join("tree"))
            .output()
            .expect("the ask is put up");
        assert!(ran.status.success(), "the ask wasn't put up: {ran:?}");
        String::from_utf8_lossy(&ran.stdout).trim().to_owned()
    }

    /// A client watching Asks' rows.
    fn watching(&self) -> Client {
        let mut client = self.daemon.client();
        client.ask(&json!({"id": "w", "ask": "watch", "kind": "row", "place": "asks"}));
        client
    }

    /// The answer written for an ask, once it's there.
    fn answer(&self, id: &str) -> Value {
        let path = self.folder.join("asks").join(format!("{id}.answer.json"));
        until("no answer was written", || path.exists());
        serde_json::from_str(&std::fs::read_to_string(path).expect("the answer reads"))
            .expect("the answer is JSON")
    }

    /// How many times op ran.
    fn op_ran(&self) -> usize {
        std::fs::read_to_string(self.folder.join("op-ran")).map_or(0, |marks| marks.lines().count())
    }
}

/// The row of an ask, once a watch is told of it.
fn row(client: &mut Client, id: &str) -> Value {
    client.changed(&format!("row:asks:{id}"))["row"].clone()
}

/// Asks the daemon what a row's act asks, as the launcher sends it.
fn act(client: &mut Client, row: &Value, at: usize) -> Value {
    let mut asked = row["acts"][at]["ask"].clone();
    asked["id"] = json!(7);
    client.ask(&asked)
}

#[test]
fn it_says_hello_as_the_fixture_does() {
    let setting = Setting::new("hello");
    let mut client = setting.daemon.client();
    let mut said = client.ask(&json!({"id": 1, "ask": "hello"}));
    let fixture: Value =
        serde_json::from_str(include_str!("fixture.json")).expect("the fixture is JSON");
    let mut hello = fixture["hello"].clone();
    // When it started is the one thing that differs between runs.
    said["since"] = json!(null);
    hello["since"] = json!(null);
    assert_eq!(said, hello, "its hello isn't the fixture's");
}

#[test]
fn a_yes_releases_the_item_to_the_checker_alone() {
    let setting = Setting::new("yes");
    let mut client = setting.watching();
    let id = setting.ask(ITEM);
    let drawn = row(&mut client, &id);
    assert_eq!(drawn["state"], "waiting", "the ask didn't wait");
    assert_eq!(
        act(&mut client, &drawn, 0)["releasing"],
        true,
        "the yes wasn't taken"
    );
    let answer = setting.answer(&id);
    assert_eq!(answer["state"], "ran", "the checker didn't run: {answer}");
    assert_eq!(answer["exit"], 0, "the checker failed: {answer}");
    let output = answer["output"].as_str().expect("an output");
    assert_eq!(
        output.trim(),
        "RELEASED: received 20 characters",
        "the checker said: {output}"
    );
    assert!(
        !answer.to_string().contains(VALUE),
        "the value reached the answer"
    );
    assert_eq!(setting.op_ran(), 1, "op didn't run once");
    let ask = setting.folder.join("asks").join(format!("{id}.ask.json"));
    assert!(!ask.exists(), "the ask's file stayed");
}

#[test]
fn an_ask_for_any_other_item_reads_nothing_and_draws_no_row() {
    let setting = Setting::new("other");
    let id = setting.ask("op://Private/Something Real/password");
    let answer = setting.answer(&id);
    assert_eq!(
        answer["state"], "refused",
        "another item wasn't refused: {answer}"
    );
    assert_eq!(answer["why"], "not listed", "it was refused for: {answer}");
    let mut client = setting.daemon.client();
    let held = client.ask(&json!({"id": 1, "ask": "list", "kind": "row"}));
    assert_eq!(held["records"], json!([]), "another item drew a row");
    assert_eq!(setting.op_ran(), 0, "op ran for another item");
}

#[test]
fn a_yes_for_what_changed_since_it_was_drawn_is_refused_and_drawn_again() {
    let setting = Setting::new("changed");
    let mut client = setting.watching();
    let id = setting.ask(ITEM);
    let drawn = row(&mut client, &id);
    std::fs::write(setting.folder.join("tree/new"), "a change").expect("a change is made");
    let said = act(&mut client, &drawn, 0);
    assert_eq!(
        said["error"]["code"], "changed",
        "a changed tree was released: {said}"
    );
    let again = row(&mut client, &id);
    assert_ne!(
        again["digest"], drawn["digest"],
        "the row wasn't drawn again"
    );
    assert_eq!(setting.op_ran(), 0, "op ran for a changed ask");
    assert_eq!(
        act(&mut client, &again, 0)["releasing"],
        true,
        "the new line's yes failed"
    );
}

#[test]
fn a_yes_while_locked_reads_nothing() {
    let setting = Setting::new("locked");
    let mut client = setting.watching();
    let id = setting.ask(ITEM);
    let drawn = row(&mut client, &id);
    std::fs::write(setting.folder.join("locked"), "yes").expect("the lock is written");
    let said = act(&mut client, &drawn, 0);
    assert_eq!(
        said["error"]["message"], "the screen is locked",
        "a locked yes: {said}"
    );
    assert_eq!(setting.op_ran(), 0, "op ran while locked");
}

#[test]
fn a_no_refuses_and_reads_nothing() {
    let setting = Setting::new("no");
    let mut client = setting.watching();
    let id = setting.ask(ITEM);
    let drawn = row(&mut client, &id);
    assert_eq!(
        act(&mut client, &drawn, 1)["refused"],
        true,
        "the no wasn't taken"
    );
    assert_eq!(
        setting.answer(&id)["why"],
        "refused by Joel",
        "the no wasn't answered"
    );
    assert_eq!(
        row(&mut client, &id)["acts"],
        json!([]),
        "a refused ask still offered acts"
    );
    assert_eq!(setting.op_ran(), 0, "op ran for a no");
}
