//! Programs' rows over the socket: each counts the windows a stand-in
//! windows daemon says its program has open, made again only as it says
//! they change.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;

use serde_json::{Value, json};

use super::Running;

#[test]
fn programs_rows_count_the_windows_the_windows_daemon_says_each_has_open() {
    let running = Running::new("rows");
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "watch", "kind": "row"}));
    let rows = said["records"].as_array().unwrap();
    assert_eq!(
        rows.len(),
        1,
        "the viewer alone, the far one hidden: {rows:?}"
    );
    let row = &rows[0]["row"];
    assert_eq!(rows[0]["key"], "row:programs:viewer");
    assert_eq!(row["thing"], "program:viewer");
    assert_eq!(row["count"], "document viewer");
    assert_eq!(row["verb"], "start");
    assert_eq!(row["order"], 1024);
    // The windows daemon begins after this one, so its socket is heard made.
    let listener = UnixListener::bind(running.path("daemons/windows.sock")).unwrap();
    let (windows, _) = listener.accept().unwrap();
    let mut asked = String::new();
    BufReader::new(windows.try_clone().unwrap())
        .read_line(&mut asked)
        .unwrap();
    let asked: Value = serde_json::from_str(&asked).unwrap();
    assert_eq!(
        asked["fields"],
        json!(["owner", "order", "pid", "opened"]),
        "a retitle wakes nothing"
    );
    let tell = |said: Value| writeln!(&windows, "{said}").unwrap();
    tell(json!({"id": "programs", "records": [
        {"key": "window:4", "owner": "viewer", "order": 10},
        {"key": "window:6", "owner": "kitty", "order": 30},
    ]}));
    let row = client.changed("row:programs:viewer");
    assert_eq!(row["row"]["count"], "document viewer \u{b7} 1 window");
    assert_eq!(row["row"]["verb"], "go to");
    assert_eq!(row["row"]["window"], 4);
    tell(
        json!({"id": "programs", "changed": [{"key": "window:5", "owner": "viewer", "order": 20}]}),
    );
    let row = client.changed("row:programs:viewer");
    assert_eq!(row["row"]["count"], "document viewer \u{b7} 2 windows");
    assert_eq!(row["row"]["window"], 5, "the one looked at last");
    tell(json!({"id": "programs", "gone": ["window:4", "window:5"]}));
    let row = client.changed("row:programs:viewer");
    assert_eq!(row["row"]["verb"], "start");
    assert_eq!(
        row["row"]["order"], 1024,
        "a row keeps its place where it can"
    );
}
