//! The daemon over its socket, with no sound server to hear, as the
//! build's sandbox has none: hello says how it stands on the socket
//! systemd hands it, and that PipeWire isn't answering; a list or a
//! watch answers no devices rather than waiting on one; an ask or a kind
//! it doesn't know is refused, and so is an act on a device it hasn't
//! heard, or one missing what it needs; and a run its unit wouldn't make
//! ends at once in the terminal kit's words. What it hears of a server is
//! proven in devices.rs and levels.rs, over the changes PipeWire says,
//! and how each act is done in acts.rs.

use std::path::PathBuf;
use std::process::{Command, Stdio};

use daemon_client::{Client, Daemon, until};
use serde_json::json;

/// The daemon, started by `launcher` and its arguments where there are
/// any, each given the socket's path where it says `{socket}`, in a
/// folder of its own where no sound server answers.
fn started(test: &str, launcher: &[&str]) -> Daemon {
    let folder: PathBuf = std::env::temp_dir().join(format!("sound-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(folder.join("daemons")).unwrap();
    let socket = folder.join("daemons").join("sound.sock");
    let program = env!("CARGO_BIN_EXE_sound");
    let mut command = Command::new(launcher.first().copied().unwrap_or(program));
    for argument in launcher.iter().skip(1) {
        command.arg(argument.replace("{socket}", &socket.to_string_lossy()));
    }
    if !launcher.is_empty() {
        command.arg(program);
    }
    // PipeWire is looked for in the test's own folder, so a test run
    // beside a sound server never reaches it.
    command
        .arg(&socket)
        .env("XDG_RUNTIME_DIR", &folder)
        .env("PIPEWIRE_RUNTIME_DIR", &folder)
        .env("PIPEWIRE_REMOTE", folder.join("no-server"))
        .stderr(Stdio::null());
    Daemon::start(&mut command, socket, folder)
}

/// How PipeWire stands, as hello says it.
fn server(client: &mut Client) -> String {
    let said = client.ask(&json!({"id": 1, "ask": "hello", "version": 1}));
    said["sources"]["pipewire"]["state"]
        .as_str()
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn hello_says_it_answers_and_that_pipewire_doesnt() {
    let daemon = started(
        "hello",
        &[
            "systemd-socket-activate",
            "--listen={socket}",
            "-E",
            "XDG_RUNTIME_DIR",
            "-E",
            "PIPEWIRE_RUNTIME_DIR",
            "-E",
            "PIPEWIRE_REMOTE",
        ],
    );
    let mut client = daemon.client();
    let said = client.ask(&json!({"id": 1, "ask": "hello", "version": 1}));
    assert_eq!(said["domain"], "sound");
    assert_eq!(said["version"], 1);
    assert_eq!(said["state"], "answering");
    until("PipeWire was never found missing", || {
        server(&mut daemon.client()) == "not answering"
    });
}

#[test]
fn with_no_server_there_are_no_devices_and_no_waiting() {
    let daemon = started("empty", &[]);
    let mut client = daemon.client();
    let listed = client.ask(&json!({"id": 1, "ask": "list", "kind": "device"}));
    assert_eq!(listed["records"], json!([]));
    let watched = client.ask(&json!({"id": 2, "ask": "watch", "kind": "device"}));
    assert_eq!(watched["records"], json!([]));
    // Sound's rows, as the launcher watches its place's.
    let rows = json!({"id": 4, "ask": "watch", "kind": "row", "place": "sound"});
    assert_eq!(client.ask(&rows)["records"], json!([]));
    // How nodes keep time, as Sound watches it while a drawer is out and
    // lets it go once it's shut.
    let timing = json!({"id": 5, "ask": "watch", "kind": "timing"});
    let timed = client.ask(&timing);
    assert_eq!(
        timed["records"],
        json!([]),
        "nothing measured with no server: {timed}"
    );
    let unwatched = client.ask(&json!({"id": 5, "ask": "unwatch"}));
    assert_eq!(unwatched["unwatched"], true, "let go of: {unwatched}");
    let health = client.ask(&json!({"id": 3, "ask": "health"}));
    assert_eq!(health["watches"], 2);
}

#[test]
fn an_ask_or_kind_it_doesnt_know_is_refused() {
    let daemon = started("unknown", &[]);
    let mut client = daemon.client();
    let said = client.ask(&json!({"id": 1, "ask": "play"}));
    assert_eq!(said["error"]["code"], "unknown-ask");
    let said = client.ask(&json!({"id": 2, "ask": "list", "kind": "window"}));
    assert_eq!(said["error"]["code"], "unknown-kind");
}

#[test]
fn an_act_it_cant_do_is_refused_at_once() {
    let daemon = started("acts", &[]);
    let mut client = daemon.client();
    let mut refusal = |asked: serde_json::Value| client.ask(&asked)["error"]["code"].clone();
    let muting = json!({"id": 1, "ask": "act", "act": "mute", "key": "device:a", "muted": true});
    assert_eq!(refusal(muting), "no-device");
    let volume = json!({"id": 2, "ask": "act", "act": "volume", "key": "device:a"});
    assert_eq!(refusal(volume), "bad-act");
    let louder = json!({"id": 3, "ask": "act", "act": "louder", "key": "device:a"});
    assert_eq!(refusal(louder), "unknown-act");
}

/// Handed no socket, a flag with no file or a flag it doesn't take, it
/// ends at once as asked wrongly, nothing on stdout, which no one reads,
/// and no socket left for the shell to find.
#[test]
fn a_run_its_unit_wouldnt_make_ends_at_once_in_the_kits_words() {
    let folder: PathBuf = std::env::temp_dir().join(format!("sound-asked-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    let socket = folder.join("sound.sock");
    let at = socket.to_str().unwrap();
    for args in [&[][..], &[at, "--marks"], &[at, "--loudly", "yes"]] {
        let ran = Command::new(env!("CARGO_BIN_EXE_sound"))
            .args(args)
            .env_remove("JOURNAL_STREAM")
            .stdin(Stdio::null())
            .output()
            .unwrap();
        let said = String::from_utf8_lossy(&ran.stderr);
        assert_eq!(
            ran.status.code(),
            Some(64),
            "{args:?} ended {:?}, saying {said}",
            ran.status.code()
        );
        assert!(
            ran.stdout.is_empty(),
            "{args:?} said {:?} on stdout",
            String::from_utf8_lossy(&ran.stdout)
        );
        assert!(
            said.contains("sound --help"),
            "{args:?} said {said:?}, pointing nowhere"
        );
        assert!(!socket.exists(), "{args:?} left a socket at {at}");
    }
}
