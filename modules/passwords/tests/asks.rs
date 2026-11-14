//! Secret asks answered by the daemon, as the shell runs it: an ask put
//! up in the runtime folder's `secret-asks` is served as a row among
//! Passwords' own, and only a yes from the launcher, on the daemon's own
//! standard input, reads its listed reference from its manager and runs
//! its command with the value. With nothing listed it watches no folder
//! and draws no row; an item not listed, or a program outside the
//! root-owned folders, is refused unseen; a yes on the row's own key, one
//! before the line could be read, or one for what changed since it was
//! drawn releases nothing; and the value never reaches an answer, the
//! journal or a command line.

mod common;

use std::path::{Path, PathBuf};
use std::time::Duration;

use common::{Client, RELEASED_VALUE, Running, until};
use serde_json::{Value, json};

/// The one reference listed, and one that isn't.
const LISTED: &str = "op://Test/Throwaway/password";
const UNLISTED: &str = "op://Private/Something Real/password";

/// The shell's own bash, in the store, so it's a root-owned program.
fn stored_bash() -> PathBuf {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .filter_map(|folder| std::fs::canonicalize(folder.join("bash")).ok())
        .find(|bash| bash.starts_with("/nix/store"))
        .expect("no bash in the store on PATH")
}

/// Puts an ask up as the harness does, written aside and renamed into
/// place, asking for `item` to run `argv` with it as `GIVEN`.
fn put_up(running: &Running, id: &str, item: &str, argv: &[&str]) {
    let asks = running.asks();
    std::fs::create_dir_all(&asks).unwrap();
    let asked_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let ask = json!({
        "version": 1, "id": id, "asked_ms": u64::try_from(asked_ms).unwrap(),
        "session": "TESTER", "quest": null, "item": item,
        "why": "To try the line.", "argv": argv,
        "tree": running.folder.to_str().unwrap(), "given": "GIVEN", "minutes": 1,
    });
    let aside = asks.join(format!(".{id}.writing"));
    std::fs::write(&aside, ask.to_string()).unwrap();
    std::fs::rename(&aside, asks.join(format!("{id}.ask.json"))).unwrap();
}

/// A command that says the value it was given, which its answer masks.
fn saying(bash: &Path) -> Vec<String> {
    vec![
        bash.to_str().unwrap().to_owned(),
        "-c".to_owned(),
        "printf 'got %s\\n' \"$GIVEN\"".to_owned(),
    ]
}

/// The row of an ask, once a watch of rows is told of it.
fn row_of(client: &mut Client, id: &str) -> Value {
    let key = format!("row:asks:{id}");
    client.say(&json!({"id": "w", "ask": "watch", "kind": "row"}));
    loop {
        let heard = client.hear();
        let records = heard["records"].as_array().or(heard["changed"].as_array());
        if let Some(record) = records
            .into_iter()
            .flatten()
            .find(|record| record["key"] == key)
        {
            return record["row"].clone();
        }
    }
}

/// The keys of the rows held now, asking no manager anything.
fn row_keys(client: &mut Client) -> Vec<String> {
    let held = client.ask(&json!({"id": "l", "ask": "list", "kind": "row", "held": true}));
    held["records"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|record| record["key"].as_str().map(str::to_owned))
        .collect()
}

/// The yes the launcher sends for a row, the line shown `shown` ms.
fn yes(row: &Value, shown: u64) -> Value {
    let mut asked = row["acts"][0]["ask"].clone();
    asked["id"] = json!(7);
    asked["shown"] = json!(shown);
    asked["opening"] = json!(1);
    asked
}

/// An ask's answer, once it's written.
fn answer(running: &Running, id: &str) -> Value {
    let path = running.asks().join(format!("{id}.answer.json"));
    until("the answer", || path.exists());
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// Whether the manager was asked to release anything.
fn released(running: &Running) -> bool {
    running.asked_of("1password").contains("\"release\"")
}

/// Says the value is nowhere it may not be: the log, the asks folder or
/// the command line bubblewrap was given.
fn nowhere(running: &Running) {
    assert!(
        !running.logged().contains(RELEASED_VALUE),
        "the log holds the value"
    );
    assert!(
        !running.bwrapped().contains(RELEASED_VALUE),
        "a command line holds the value"
    );
    for entry in std::fs::read_dir(running.asks()).into_iter().flatten() {
        let path = entry.unwrap().path();
        let held = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            !held.contains(RELEASED_VALUE),
            "{} holds the value",
            path.display()
        );
    }
}

#[test]
fn with_nothing_listed_an_ask_draws_no_row_and_a_yes_releases_nothing() {
    let running = Running::releasing("asks-off", &[]);
    let id = "0123456789abcdef0123456789abcdef";
    let bash = stored_bash();
    let argv = saying(&bash);
    put_up(
        &running,
        id,
        LISTED,
        &argv.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let mut client = running.client();
    std::thread::sleep(Duration::from_millis(1500));
    let keys = row_keys(&mut client);
    assert!(
        !keys.iter().any(|key| key.starts_with("row:asks:")),
        "an ask drew a row with nothing listed: {keys:?}"
    );
    let refused = client.ask(
        &json!({"id": 7, "ask": "release", "key": format!("ask:{id}"),
                                     "digest": "0", "shown": 812, "opening": 1}),
    );
    assert_eq!(
        refused["error"]["message"], "not listed",
        "a yes with nothing listed: {refused}"
    );
    assert!(
        running.asks().join(format!("{id}.ask.json")).exists()
            && !running.asks().join(format!("{id}.answer.json")).exists(),
        "an ask was answered with nothing listed"
    );
    assert!(!released(&running), "the manager was asked to release");
}

#[test]
fn a_listed_ask_is_drawn_and_its_yes_runs_the_command_with_the_value_masked() {
    let running = Running::releasing("asks-yes", &[LISTED]);
    let id = "1123456789abcdef0123456789abcdef";
    let bash = stored_bash();
    let argv = saying(&bash);
    put_up(
        &running,
        id,
        LISTED,
        &argv.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let mut client = running.client();
    let row = row_of(&mut client, id);
    assert_eq!(row["thing"], format!("ask:{id}"), "the row: {row}");
    assert_eq!(row["state"], "waiting", "the row: {row}");
    let said = client.ask(&yes(&row, 812));
    assert_eq!(said["releasing"], true, "the yes: {said}");
    let answer = answer(&running, id);
    assert_eq!(answer["state"], "ran", "the answer: {answer}");
    assert_eq!(answer["exit"], 0, "the answer: {answer}");
    assert_eq!(
        answer["output"], "got [released value]\n",
        "the output wasn't masked: {answer}"
    );
    let asked = running.asked_of("1password");
    assert!(
        asked.contains(&format!("\"reference\":\"{LISTED}\"")) && asked.contains("\"asked\":true"),
        "the manager wasn't asked for the listed reference as Joel: {asked}"
    );
    let bwrapped = running.bwrapped();
    for kept in [
        "--die-with-parent",
        "--unshare-pid",
        "--new-session",
        "--chdir",
    ] {
        assert!(
            bwrapped.lines().any(|line| line == kept),
            "bubblewrap wasn't given {kept}: {bwrapped}"
        );
    }
    nowhere(&running);
    until("the ask taken away", || {
        !running.asks().join(format!("{id}.ask.json")).exists()
    });
}

#[test]
fn an_ask_for_an_item_not_listed_is_refused_unseen() {
    let running = Running::releasing("asks-unlisted", &[LISTED]);
    let id = "2123456789abcdef0123456789abcdef";
    let bash = stored_bash();
    let argv = saying(&bash);
    put_up(
        &running,
        id,
        UNLISTED,
        &argv.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let answer = answer(&running, id);
    assert_eq!(answer["state"], "refused", "the answer: {answer}");
    assert_eq!(answer["why"], "not listed", "the answer: {answer}");
    let mut client = running.client();
    let keys = row_keys(&mut client);
    assert!(
        !keys.iter().any(|key| key.starts_with("row:asks:")),
        "an unlisted ask drew a row: {keys:?}"
    );
    assert!(!released(&running), "the manager was asked to release");
}

#[test]
fn an_ask_to_run_a_program_outside_the_root_owned_folders_is_refused() {
    let running = Running::releasing("asks-unowned", &[LISTED]);
    let id = "3123456789abcdef0123456789abcdef";
    let program = running.folder.join("mine");
    std::fs::write(&program, "#!/bin/sh\necho \"$GIVEN\"\n").unwrap();
    std::fs::set_permissions(
        &program,
        std::os::unix::fs::PermissionsExt::from_mode(0o755),
    )
    .unwrap();
    put_up(&running, id, LISTED, &[program.to_str().unwrap()]);
    let answer = answer(&running, id);
    assert_eq!(answer["state"], "refused", "the answer: {answer}");
    assert_eq!(
        answer["why"], "not a root-owned program",
        "the answer: {answer}"
    );
    assert!(!released(&running), "the manager was asked to release");
}

#[test]
fn a_yes_on_the_rows_key_or_too_soon_releases_nothing_and_a_no_refuses() {
    let running = Running::releasing("asks-no", &[LISTED]);
    let id = "4123456789abcdef0123456789abcdef";
    let bash = stored_bash();
    let argv = saying(&bash);
    put_up(
        &running,
        id,
        LISTED,
        &argv.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let mut client = running.client();
    let row = row_of(&mut client, id);
    let mut by_row = yes(&row, 812);
    by_row["key"] = json!(format!("row:asks:{id}"));
    let said = client.ask(&by_row);
    assert!(said["error"].is_object(), "a yes by the row's key: {said}");
    let said = client.ask(&yes(&row, 120));
    assert_eq!(said["error"]["code"], "refused", "a yes too soon: {said}");
    let mut no = row["acts"][1]["ask"].clone();
    no["id"] = json!(8);
    let said = client.ask(&no);
    assert_eq!(said["refused"], true, "the no: {said}");
    let answer = answer(&running, id);
    assert_eq!(answer["state"], "refused", "the answer: {answer}");
    assert_eq!(answer["why"], "refused by Joel", "the answer: {answer}");
    assert!(
        answer.get("output").is_none(),
        "a no ran something: {answer}"
    );
    assert!(!released(&running), "the manager was asked to release");
    nowhere(&running);
}

#[test]
fn a_yes_for_what_changed_since_it_was_drawn_releases_nothing() {
    let running = Running::releasing("asks-changed", &[LISTED]);
    let id = "5123456789abcdef0123456789abcdef";
    let bash = stored_bash();
    let argv = saying(&bash);
    put_up(
        &running,
        id,
        LISTED,
        &argv.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let mut client = running.client();
    let row = row_of(&mut client, id);
    let mut stale = yes(&row, 812);
    stale["digest"] = json!("0".repeat(64));
    let said = client.ask(&stale);
    assert_eq!(said["error"]["code"], "changed", "a stale yes: {said}");
    assert!(!released(&running), "the manager was asked to release");
    assert!(
        row_keys(&mut client).contains(&format!("row:asks:{id}")),
        "the ask's row went"
    );
}
