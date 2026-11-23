//! Code's rows in the order Joel's own visits give, read from zoxide's
//! database: the repository he was in last first; a later watch in the
//! order his newest visit gives; nothing read from a database no setting
//! names; and a database that never answers holding no other ask, its
//! order let go.

use std::io::Write as _;
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use daemon_client::until;
use serde_json::json;

use super::{cloned, real, row_of};
use crate::Running;

/// zoxide's database as it lays it out at version 3, each folder with
/// when it was last visited, in seconds.
fn zoxide_database(folders: &[(&Path, u64)]) -> Vec<u8> {
    let mut bytes = 3_u32.to_le_bytes().to_vec();
    bytes.extend((folders.len() as u64).to_le_bytes());
    for (path, seconds) in folders {
        let path = path.to_string_lossy();
        bytes.extend((path.len() as u64).to_le_bytes());
        bytes.extend(path.as_bytes());
        bytes.extend(1.0_f64.to_le_bytes());
        bytes.extend(seconds.to_le_bytes());
    }
    bytes
}

#[test]
fn the_repository_joel_was_in_last_comes_first() {
    let running = Running::new("code-visited");
    // beta first, so git's last write and A to Z both put alpha first and
    // only the visit can lift beta.
    cloned(&running, "beta", "git@github.com:someone/beta.git");
    cloned(&running, "alpha", "git@github.com:someone/alpha.git");
    // The shell went in through a link, as zoxide then keeps it.
    std::os::unix::fs::symlink(running.path("src"), running.path("linked")).unwrap();
    let within = running.path("linked/beta/docs");
    std::fs::create_dir_all(&within).unwrap();
    std::fs::create_dir_all(running.path("visits")).unwrap();
    std::fs::write(
        running.path("visits/db.zo"),
        zoxide_database(&[(&within, 1_700_000_000)]),
    )
    .unwrap();
    let mut client = running.client();
    let answer = client.ask(&json!({"id": 1, "ask": "watch", "kind": "row", "place": "code"}));
    let order = |key: &str| {
        row_of(&answer, key).unwrap_or_else(|| panic!("no {key}: {answer}"))["row"]["order"]
            .as_u64()
            .unwrap()
    };
    assert!(
        order("row:code:github:someone/beta") < order("row:code:github:someone/alpha"),
        "the repository Joel was in last didn't come first, A to Z did: {answer}"
    );
}

#[test]
fn a_watch_after_joel_went_somewhere_answers_in_the_order_it_gave() {
    let running = Running::new("code-visited-since");
    cloned(&running, "beta", "git@github.com:someone/beta.git");
    cloned(&running, "alpha", "git@github.com:someone/alpha.git");
    std::fs::create_dir_all(running.path("visits")).unwrap();
    std::fs::write(running.path("visits/db.zo"), zoxide_database(&[])).unwrap();
    running
        .client()
        .ask(&json!({"id": 1, "ask": "watch", "kind": "row", "place": "code"}));
    let beta = real(&running.path("src/beta"));
    std::fs::write(
        running.path("visits/db.zo"),
        zoxide_database(&[(&beta, 1_700_000_000)]),
    )
    .unwrap();
    let answer = running
        .client()
        .ask(&json!({"id": 2, "ask": "watch", "kind": "row", "place": "code"}));
    let order = |key: &str| {
        row_of(&answer, key).unwrap_or_else(|| panic!("no {key}: {answer}"))["row"]["order"]
            .as_u64()
            .unwrap()
    };
    assert!(
        order("row:code:github:someone/beta") < order("row:code:github:someone/alpha"),
        "a later watch answered in the order before Joel went into beta: {answer}"
    );
}

#[test]
fn with_no_database_named_a_database_zoxide_left_orders_nothing() {
    let running = Running::unvisited("code-unvisited");
    cloned(&running, "beta", "git@github.com:someone/beta.git");
    cloned(&running, "alpha", "git@github.com:someone/alpha.git");
    // Where zoxide keeps its database by default, a visit that would lift
    // beta were it read.
    let within = running.path("src/beta");
    std::fs::create_dir_all(running.path("near/zoxide")).unwrap();
    std::fs::write(
        running.path("near/zoxide/db.zo"),
        zoxide_database(&[(&within, 1_700_000_000)]),
    )
    .unwrap();
    let mut client = running.client();
    let answer = client.ask(&json!({"id": 1, "ask": "watch", "kind": "row", "place": "code"}));
    let order = |key: &str| {
        row_of(&answer, key).unwrap_or_else(|| panic!("no {key}: {answer}"))["row"]["order"]
            .as_u64()
            .unwrap()
    };
    assert!(
        order("row:code:github:someone/alpha") < order("row:code:github:someone/beta"),
        "a database no setting named ordered Code, not A to Z: {answer}"
    );
}

#[test]
fn a_database_that_never_answers_holds_no_other_ask() {
    let running = Running::new("code-visits-stuck");
    cloned(&running, "alpha", "git@github.com:someone/alpha.git");
    std::fs::create_dir_all(running.path("visits")).unwrap();
    std::fs::write(running.path("visits/db.zo"), zoxide_database(&[])).unwrap();
    let mut first = running.client();
    first.ask(&json!({"id": 1, "ask": "watch", "kind": "row", "place": "code"}));
    // A pipe nobody writes, so reading the database waits until the case
    // writes it, as a database on a mount that stopped answering would.
    let database = running.path("visits/db.zo");
    std::fs::remove_file(&database).unwrap();
    let made = Command::new("mkfifo").arg(&database).status();
    assert!(made.unwrap().success(), "mkfifo failed");
    let mut second = running.client();
    second.send(&json!({"id": 2, "ask": "watch", "kind": "row", "place": "code"}));
    // A pipe opened to write without waiting opens only once a reader
    // holds it, so the second watch's read is under way before the third
    // ask; held open and unwritten, it keeps that read waiting.
    let writer = std::cell::RefCell::new(None);
    until("the second watch never read zoxide's database", || {
        let opened = std::fs::OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(&database)
            .ok();
        let open = opened.is_some();
        *writer.borrow_mut() = opened;
        open
    });
    let mut third = running.client();
    // Timed, since the read gives up at its deadline of two seconds: an
    // ask held that long behind the read would still be answered.
    let asked = Instant::now();
    let said = third.ask(&json!({"id": 3, "ask": "watch", "kind": "row", "place": "tabs"}));
    let waited = asked.elapsed();
    // Another of Code's own watches is answered too, its order let go
    // rather than waiting on the database with the second.
    let mut fourth = running.client();
    let coded = fourth.ask(&json!({"id": 4, "ask": "watch", "kind": "row", "place": "code"}));
    // Freed before asserting, so no thread is left waiting on the pipe.
    let mut writer = writer.into_inner().expect("the pipe was opened to write");
    writer.write_all(&zoxide_database(&[])).unwrap();
    drop(writer);
    assert!(
        said.get("error").is_none(),
        "an ask needing the daemon's lock wasn't answered while zoxide's database \
         was read: {said}"
    );
    assert!(
        waited < Duration::from_secs(1),
        "an ask needing the daemon's lock waited {waited:?} on zoxide's database"
    );
    assert!(
        coded.get("error").is_none(),
        "a watch of Code's rows waited on zoxide's database: {coded}"
    );
}
