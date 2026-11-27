//! slack-unreads without a sign-in it can use: a refused one held as
//! signed out with nothing counted, none at all holding nothing and
//! asking Slack nothing, and its watchdog told all the same.

use std::process::Command;
use std::thread;
use std::time::Duration;

use super::{
    StandIn, asked, credentials, forget, kept, listed, socket, unreads, unreads_from, until,
    waiting, wants_joel,
};

#[test]
fn a_refused_sign_in_is_held_as_signed_out_with_nothing_counted() {
    let stand_in = StandIn::new(|_| r#"{"ok": false, "error": "invalid_auth"}"#.into());
    let path = credentials("unreads-refused");
    let run = path.with_extension("refused");
    let mut program = unreads(&stand_in, &path, &run);
    until("Joel was never asked to sign in", || {
        kept(&waiting(&run)).is_some()
    });
    // Over the socket, only the account is held, saying so.
    let held = asked(&run, &serde_json::json!({"id": 1, "ask": "list"}));
    assert_eq!(
        held["records"],
        serde_json::json!([{"key": "slack-account:slack", "kind": "slack-account",
            "slack-account": {"standing": "signedOut"}, "heard": held["records"][0]["heard"]}])
    );
    let hello = asked(&run, &serde_json::json!({"id": 2, "ask": "hello"}));
    assert_eq!(hello["sources"]["slack"], "signed-out");
    program.kill().unwrap();
    program.wait().unwrap();
    // As For You reads it, once, done when he signs in again.
    let asked = kept(&waiting(&run)).unwrap();
    assert_eq!(
        (
            &asked["items"][0]["title"],
            &asked["items"][0]["doneWhen"]["signal"]
        ),
        (
            &serde_json::json!("Sign In to Slack"),
            &serde_json::json!("signin:slack")
        )
    );
    let _ = std::fs::remove_file(waiting(&run));
    let _ = std::fs::remove_dir_all(socket(&run).parent().unwrap());
    forget(&path);
}

#[test]
fn with_no_sign_in_nothing_is_held_and_slack_is_never_asked() {
    let stand_in = StandIn::new(wants_joel);
    let missing = std::env::temp_dir().join("slack-unreads-no-such-file.json");
    let run = std::env::temp_dir().join(format!("slack-unreads-none-{}", std::process::id()));
    let mut program = unreads(&stand_in, &missing, &run);
    until("the socket never came", || socket(&run).exists());
    // Long enough for several rounds of asking.
    thread::sleep(Duration::from_millis(300));
    assert_eq!(
        listed(&run, "slack-account"),
        [serde_json::json!({"standing": "none"})]
    );
    assert_eq!(listed(&run, "slack-unread"), [] as [serde_json::Value; 0]);
    program.kill().unwrap();
    program.wait().unwrap();
    assert!(
        !waiting(&run).exists(),
        "with no sign-in, nothing should be held as waiting at {}",
        waiting(&run).display()
    );
    let heard = stand_in.heard();
    assert!(
        heard.is_empty(),
        "with no sign-in, Slack shouldn't be asked: {heard:?}"
    );
    let _ = std::fs::remove_dir_all(socket(&run).parent().unwrap());
}

/// With no sign-in it still tells systemd's watchdog it's there each
/// round, since waiting on one is no hang: systemd restarted it every
/// three minutes while the harness's sign-in folder was empty (16V6F4,
/// 2026-11-13).
#[test]
fn with_no_sign_in_it_still_tells_its_watchdog() {
    let stand_in = StandIn::new(wants_joel);
    let missing = std::env::temp_dir().join("slack-unreads-no-such-file.json");
    let run = std::env::temp_dir().join(format!("slack-unreads-watched-{}", std::process::id()));
    let notify = run.with_extension("notify");
    let _ = std::fs::remove_file(&notify);
    let systemd = std::os::unix::net::UnixDatagram::bind(&notify).unwrap();
    systemd
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_slack-unreads"));
    command.env("NOTIFY_SOCKET", &notify);
    let mut program = unreads_from(command, &stand_in, &missing, &run);
    let mut told = [0; 64];
    let heard = systemd.recv(&mut told);
    program.kill().unwrap();
    program.wait().unwrap();
    let _ = std::fs::remove_file(&notify);
    let _ = std::fs::remove_dir_all(socket(&run).parent().unwrap());
    let length = heard.expect("the watchdog was never told, with no sign-in");
    assert_eq!(&told[..length], b"WATCHDOG=1");
}
