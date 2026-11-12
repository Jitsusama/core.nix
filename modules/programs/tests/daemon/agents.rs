//! kitty's open list without the panes the agents daemon names as agents',
//! heard from a stand-in for it on the socket beside this daemon's, made
//! after the daemon starts as systemd may, and served again as it tells a
//! session's window gone.

use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::net::UnixListener;

use serde_json::{Value, json};

use super::{Client, Running};

const LISTED: &str = "listed:terminal:open";

/// The told list as it's served next with `agents` left out, its first
/// answer's or a change's.
fn served_with(client: &mut Client, first: &Value, agents: u64) -> Value {
    let mut record = first.clone();
    while record["agents"] != agents {
        record = client.changed(LISTED);
    }
    record
}

#[test]
fn the_agents_panes_leave_kittys_list_as_the_agents_daemon_names_them() {
    let running = Running::new("agents");
    let tabs = json!([{"id": 1, "tabs": [
        {"id": 1, "windows": [{"id": 1}]},
        {"id": 2, "windows": [{"id": 2}]},
    ]}]);
    std::fs::write(running.path("lists/open"), format!("4242\t{tabs}\n")).unwrap();
    let listener = UnixListener::bind(running.path("daemons/agents.sock")).unwrap();
    let (mut stream, _) = listener.accept().unwrap();
    let mut asked = String::new();
    BufReader::new(&stream).read_line(&mut asked).unwrap();
    assert!(
        asked.contains("\"kind\":\"agent-window\""),
        "the agents daemon wasn't asked for its agents' windows: {asked}"
    );
    let agent = json!({
        "key": "agent-window:agents:ABC", "kind": "agent-window", "address": "ABC",
        "kitty": {"id": "2", "within": "unix:@kitty-4242"}, "window": "window:9",
    });
    writeln!(stream, "{}", json!({"id": "programs", "records": [agent]})).unwrap();
    let mut client = running.client();
    let watched = client.ask(&json!({"id": 1, "ask": "watch", "keys": [LISTED]}));
    let without = served_with(&mut client, &watched["records"][0], 1);
    let kept: Value = serde_json::from_str(
        without["lines"][0]
            .as_str()
            .and_then(|line| line.split_once('\t'))
            .unwrap()
            .1,
    )
    .unwrap();
    assert_eq!(
        kept,
        json!([{"id": 1, "tabs": [{"id": 1, "windows": [{"id": 1}]}]}]),
        "the agent's pane, and the tab it was alone in, weren't left out: {without}"
    );
    writeln!(
        stream,
        "{}",
        json!({"id": "programs", "gone": ["agent-window:agents:ABC"]})
    )
    .unwrap();
    let back = served_with(&mut client, &without, 0);
    assert_eq!(
        back["lines"],
        json!([format!("4242\t{tabs}")]),
        "the session gone, its pane wasn't served again: {back}"
    );
}
