//! The daemon over its socket, with no sound server to hear, as the
//! build's sandbox has none: hello says how it stands on the socket
//! systemd hands it, and that PipeWire isn't answering; a list or a
//! watch answers no devices rather than waiting on one; and an ask or a
//! kind it doesn't know is refused. What it hears of a server is proven
//! in devices.rs and levels.rs, over the changes PipeWire says.

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
    let health = client.ask(&json!({"id": 3, "ask": "health"}));
    assert_eq!(health["watches"], 1);
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
