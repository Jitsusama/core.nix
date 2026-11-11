//! The daemon over its socket, hearing a stand-in for niri's event
//! stream, with data folders of its own: hello says how it stands on the
//! socket systemd hands it; a watch is answered with every window named
//! by its program, the one used last first; a retitle tells only its
//! window, a focus moving only the two it moved between, and a resize
//! nothing but a watch naming layouts; a window closed is told gone; an entry added names its
//! windows' program; niri gone and back describes the desktop again;
//! and an ask or kind it doesn't know is refused.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use daemon_client::{Client, Daemon, until};
use serde_json::{Value, json};

/// niri's description of the desktop as its stream begins: two
/// workspaces, a kitty for a session, a Chrome and a foot.
const BEGINNING: [&str; 3] = [
    r#"{"WorkspacesChanged":{"workspaces":[{"id":1,"idx":1,"name":null,"output":"eDP-1","is_urgent":false,"is_active":true,"is_focused":true,"active_window_id":10},{"id":2,"idx":2,"name":null,"output":"eDP-1","is_urgent":false,"is_active":false,"is_focused":false,"active_window_id":13}]}}"#,
    r#"{"WindowsChanged":{"windows":[{"id":10,"title":"pi","app_id":"pi-session.3","pid":100,"workspace_id":1,"is_focused":true,"is_floating":false,"is_urgent":false,"layout":{"pos_in_scrolling_layout":[1,1],"tile_size":[640,800]},"focus_timestamp":{"secs":1000,"nanos":0}},{"id":11,"title":"Inbox - Google Chrome","app_id":"google-chrome","pid":101,"workspace_id":1,"is_focused":false,"is_floating":false,"is_urgent":false,"layout":{"pos_in_scrolling_layout":[2,1],"tile_size":[640,800]},"focus_timestamp":{"secs":900,"nanos":0}},{"id":13,"title":"~","app_id":"foot","pid":102,"workspace_id":2,"is_focused":false,"is_floating":false,"is_urgent":false,"layout":{"pos_in_scrolling_layout":[1,1],"tile_size":[640,800]},"focus_timestamp":{"secs":950,"nanos":0}}]}}"#,
    r#"{"OverviewOpenedOrClosed":{"is_open":false}}"#,
];

/// A stand-in for niri's socket, which answers an event stream with
/// `BEGINNING` and then whatever the test sends it, and hangs up when the
/// test says to.
struct Niri {
    send: Sender<Option<String>>,
    streams: Arc<Mutex<usize>>,
}

impl Niri {
    fn new(socket: &Path) -> Niri {
        let listener = UnixListener::bind(socket).expect("the stand-in listens");
        let (send, lines): (Sender<Option<String>>, Receiver<Option<String>>) = mpsc::channel();
        let streams = Arc::new(Mutex::new(0));
        let counted = Arc::clone(&streams);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = stream.expect("the stand-in takes each connection");
                let mut asked = String::new();
                BufReader::new(stream.try_clone().unwrap())
                    .read_line(&mut asked)
                    .unwrap();
                assert_eq!(
                    asked.trim(),
                    "\"EventStream\"",
                    "it asks niri for its events"
                );
                writeln!(stream, "{{\"Ok\":\"Handled\"}}").unwrap();
                for event in BEGINNING {
                    writeln!(stream, "{event}").unwrap();
                }
                *counted.lock().unwrap() += 1;
                // Until the test says to hang up.
                while let Ok(Some(line)) = lines.recv() {
                    writeln!(stream, "{line}").unwrap();
                }
                let _ = stream.shutdown(std::net::Shutdown::Both);
            }
        });
        Niri { send, streams }
    }

    fn says(&self, event: &Value) {
        self.send.send(Some(event.to_string())).unwrap();
    }

    fn hangs_up(&self) {
        self.send.send(None).unwrap();
    }

    fn streams(&self) -> usize {
        *self.streams.lock().unwrap()
    }
}

struct Running {
    daemon: Daemon,
    niri: Niri,
}

impl Running {
    fn new(test: &str) -> Running {
        Running::through(test, &[])
    }

    /// Started by `launcher` and its arguments, each given the socket's
    /// path where it says `{socket}`, rather than on its own.
    fn through(test: &str, launcher: &[&str]) -> Running {
        let folder = std::env::temp_dir().join(format!("windows-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        for (path, text) in [
            (
                "far/applications/kitty.desktop",
                "[Desktop Entry]\nType=Application\nName=kitty\n",
            ),
            (
                "far/applications/google-chrome.desktop",
                "[Desktop Entry]\nType=Application\nName=Google Chrome\n",
            ),
        ] {
            let file = folder.join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, text).unwrap();
        }
        std::fs::create_dir_all(folder.join("near/applications")).unwrap();
        std::fs::create_dir_all(folder.join("daemons")).unwrap();
        let niri = Niri::new(&folder.join("niri.sock"));
        let socket = folder.join("daemons").join("windows.sock");
        let program = env!("CARGO_BIN_EXE_windows");
        let mut command = match launcher.split_first() {
            Some((first, arguments)) => {
                let mut command = Command::new(first);
                for argument in arguments {
                    command.arg(argument.replace("{socket}", &socket.to_string_lossy()));
                }
                command.arg(program);
                command
            }
            None => Command::new(program),
        };
        command
            .arg(&socket)
            .env("NIRI_SOCKET", folder.join("niri.sock"))
            .env("XDG_DATA_HOME", folder.join("near"))
            .env("XDG_DATA_DIRS", folder.join("far"))
            .stderr(Stdio::null());
        let running = Running {
            daemon: Daemon::start(&mut command, socket, folder),
            niri,
        };
        // One systemd starts begins only as it's first asked.
        if launcher.is_empty() {
            running.hears_niri();
        }
        running
    }

    fn hears_niri(&self) {
        until("the daemon never heard niri", || self.niri.streams() == 1);
    }

    fn path(&self, path: &str) -> PathBuf {
        self.daemon.folder().join(path)
    }

    /// A client watching the windows, once the daemon holds all three.
    fn watching(&self) -> (Client, Vec<Value>) {
        let mut client = self.daemon.client();
        loop {
            let said = client.ask(&json!({"id": 1, "ask": "watch", "kind": "window"}));
            let records = said["records"].as_array().cloned().unwrap_or_default();
            if records.len() == 3 {
                return (client, records);
            }
            client.ask(&json!({"id": 1, "ask": "unwatch"}));
        }
    }
}

fn keys(said: &Value) -> Vec<String> {
    said["changed"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|record| record["key"].as_str().map(str::to_owned))
        .collect()
}

fn window(id: u64, title: &str, app_id: &str, workspace: u64, column: u64, secs: u64) -> Value {
    json!({"WindowOpenedOrChanged": {"window": {
        "id": id, "title": title, "app_id": app_id, "pid": 100 + id, "workspace_id": workspace,
        "is_focused": false, "is_floating": false, "is_urgent": false,
        "layout": {"pos_in_scrolling_layout": [column, 1], "tile_size": [640, 800]},
        "focus_timestamp": {"secs": secs, "nanos": 0}}}})
}

#[test]
fn hello_says_it_answers_on_the_socket_systemd_hands_it() {
    // systemd-socket-activate hands on only the environment it's told to.
    let running = Running::through(
        "hello",
        &[
            "systemd-socket-activate",
            "--listen={socket}",
            "-E",
            "NIRI_SOCKET",
            "-E",
            "XDG_DATA_HOME",
            "-E",
            "XDG_DATA_DIRS",
        ],
    );
    let said = running
        .daemon
        .client()
        .ask(&json!({"id": 1, "ask": "hello", "version": 1}));
    running.hears_niri();
    assert_eq!(said["domain"], "windows");
    assert_eq!(said["version"], 1);
    assert_eq!(said["state"], "answering");
    assert_eq!(said["sources"]["entries"]["count"], 2);
}

#[test]
fn a_watch_answers_every_window_named_the_one_used_last_first() {
    let running = Running::new("watch");
    let (_, records) = running.watching();
    let order: Vec<&Value> = records.iter().map(|record| &record["key"]).collect();
    assert_eq!(order, ["window:10", "window:13", "window:11"]);
    assert_eq!(records[0]["app_id"], "kitty");
    assert_eq!(records[0]["session"], true);
    assert_eq!(records[0]["focused"], true);
    assert_eq!(records[1]["elsewhere"], true);
    assert_eq!(records[1]["program"], "foot");
    assert_eq!(records[2]["named"], "Inbox");
    assert_eq!(records[2]["entry"], "google-chrome");
}

#[test]
fn a_retitle_tells_only_its_window() {
    let running = Running::new("retitle");
    let (mut client, _) = running.watching();
    running.niri.says(&window(
        11,
        "Calendar - Google Chrome",
        "google-chrome",
        1,
        2,
        900,
    ));
    let said = client.heard();
    assert_eq!(keys(&said), ["window:11"]);
    assert_eq!(said["changed"][0]["named"], "Calendar");
}

#[test]
fn a_focus_moving_tells_only_its_two_windows_and_a_resize_nothing() {
    let running = Running::new("focus");
    let (mut client, _) = running.watching();
    running
        .niri
        .says(&json!({"WindowLayoutsChanged": {"changes": [
        [10, {"pos_in_scrolling_layout": [1, 1], "tile_size": [900, 800]}]]}}));
    running
        .niri
        .says(&json!({"WindowFocusChanged": {"id": 11}}));
    let said = client.heard();
    assert_eq!(keys(&said), ["window:10", "window:11"]);
    running.niri.says(&json!({"WindowFocusTimestampChanged": {
        "id": 11, "focus_timestamp": {"secs": 1100, "nanos": 0}}}));
    let said = client.heard();
    assert_eq!(keys(&said), ["window:11"]);
    assert_eq!(said["changed"][0]["order"], 1_100_000);
}

#[test]
fn a_resize_is_told_only_to_a_watch_naming_layouts() {
    let running = Running::new("layouts");
    running.watching();
    let mut unnamed = running.daemon.client();
    let said = unnamed.ask(&json!({"id": 1, "ask": "watch"}));
    let kinds: Vec<&str> = said["records"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|record| record["key"].as_str())
        .collect();
    assert!(kinds.contains(&"screenshot"), "{kinds:?}");
    assert!(
        !kinds.iter().any(|key| key.starts_with("layout:")),
        "{kinds:?}"
    );
    let mut tiles = running.daemon.client();
    let said = tiles.ask(&json!({"id": 2, "ask": "watch", "kind": "layout"}));
    assert_eq!(said["records"].as_array().map(Vec::len), Some(3));
    running
        .niri
        .says(&json!({"WindowLayoutsChanged": {"changes": [
        [10, {"pos_in_scrolling_layout": [1, 1], "tile_size": [900, 800]}]]}}));
    let said = tiles.heard();
    assert_eq!(keys(&said), ["layout:10"]);
    assert_eq!(said["changed"][0]["tile_size"], json!([900, 800]));
    // The watch naming no kind hears the next thing that isn't a tile,
    // and not the resize before it.
    running
        .niri
        .says(&window(11, "Calendar", "google-chrome", 1, 2, 900));
    let heard = keys(&unnamed.heard());
    assert!(
        !heard.contains(&"layout:10".to_owned()),
        "a watch naming no kind heard the resize: {heard:?}"
    );
}

#[test]
fn a_window_closed_is_told_gone() {
    let running = Running::new("closed");
    let (mut client, _) = running.watching();
    running.niri.says(&json!({"WindowClosed": {"id": 13}}));
    assert_eq!(client.gone(), [json!("window:13")]);
}

#[test]
fn an_entry_added_names_its_windows_program() {
    let running = Running::new("entry");
    let (mut client, _) = running.watching();
    std::fs::write(
        running.path("near/applications/foot.desktop"),
        "[Desktop Entry]\nType=Application\nName=Foot\n",
    )
    .unwrap();
    let foot = client.changed("window:13");
    assert_eq!(foot["entry"], "foot");
    assert_eq!(foot["program"], "Foot");
}

#[test]
fn niri_gone_and_back_describes_the_desktop_again() {
    let running = Running::new("again");
    let (mut client, _) = running.watching();
    running.niri.says(&window(14, "Notes", "foot", 1, 3, 1200));
    assert_eq!(keys(&client.heard()), ["window:14"]);
    running.niri.hangs_up();
    until("the daemon never asked niri again", || {
        running.niri.streams() == 2
    });
    // The new niri's description has no window 14, so it's gone.
    assert_eq!(client.gone(), [json!("window:14")]);
    let health = running
        .daemon
        .client()
        .ask(&json!({"id": 2, "ask": "health"}));
    assert_eq!(health["sources"]["niri"]["state"], "answering");
}

#[test]
fn an_ask_or_kind_it_doesnt_know_is_refused() {
    let running = Running::new("refused");
    let mut client = running.daemon.client();
    let said = client.ask(&json!({"id": 1, "ask": "close"}));
    assert_eq!(said["error"]["code"], "unknown-ask");
    let said = client.ask(&json!({"id": 2, "ask": "watch", "kind": "program"}));
    assert_eq!(said["error"]["code"], "unknown-kind");
}
