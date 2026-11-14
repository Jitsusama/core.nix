//! The tab in front of one of niri's windows of Chrome, got by niri's id
//! for it: Chrome's tabs read from its extension's host as the get comes,
//! and the window's title asked of a stand-in windows daemon then, the
//! window matched to the one tab in front its title begins with, and none
//! where two of Chrome's windows have tabs of that title in front.

use std::io::{BufRead, BufReader, Read as _, Write as _};
use std::os::unix::net::UnixListener;
use std::thread;

use serde_json::{Value, json};

use super::Running;

/// Stands in for the extension's host, listing Chrome's windows 1, 2, 3
/// and 4: 1's tab in front is Inbox, with Docs behind it; 2's is Docs;
/// and both 3's and 4's are Same.
fn hosted(running: &Running) {
    let listener = UnixListener::bind(running.path("chrome-open-tabs.sock")).unwrap();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut asked = String::new();
            stream.read_to_string(&mut asked).unwrap();
            let said = [
                "10\t1\ttrue\t3\thttps://mail.example.org/\tInbox",
                "11\t1\tfalse\t2\thttps://docs.example.org/behind\tDocs",
                "20\t2\ttrue\t1\thttps://docs.example.org/front\tDocs",
                "30\t3\ttrue\t1\thttps://c.example.org/\tSame",
                "40\t4\ttrue\t1\thttps://d.example.org/\tSame",
            ];
            if asked == "list\n" {
                stream
                    .write_all((said.join("\n") + "\n").as_bytes())
                    .unwrap();
            }
        }
    });
}

/// Stands in for the windows daemon: its watch is answered with no
/// window, and a get with the record of each of niri's windows it names,
/// 70, 80 and 90, titled as Chrome titles them.
fn windowed(running: &Running) {
    let listener = UnixListener::bind(running.path("daemons/windows.sock")).unwrap();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let stream = stream.unwrap();
            thread::spawn(move || {
                let mut lines = BufReader::new(stream.try_clone().unwrap()).lines();
                while let Some(Ok(line)) = lines.next() {
                    let asked: Value = serde_json::from_str(&line).unwrap();
                    let titled = |key: &str| match key {
                        "window:70" => Some("Inbox - Google Chrome"),
                        "window:80" => Some("Docs - Google Chrome"),
                        "window:90" => Some("Same - Google Chrome"),
                        _ => None,
                    };
                    let records: Vec<Value> = asked["keys"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .filter_map(|key| {
                            Some(json!({"key": key, "owner": "chrome", "title": titled(key)?}))
                        })
                        .collect();
                    writeln!(
                        &stream,
                        "{}",
                        json!({"id": asked["id"], "records": records})
                    )
                    .unwrap();
                }
            });
        }
    });
}

#[test]
fn a_chrome_windows_front_tab_is_got_by_niris_window() {
    let running = Running::new("front");
    hosted(&running);
    windowed(&running);
    let said = running
        .client()
        .ask(&json!({"id": 1, "ask": "get", "keys": [
            "front:chrome:70", "front:chrome:80", "front:chrome:90",
        ]}));
    let fronts: Vec<(&Value, &Value, &Value)> = said["records"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|record| (&record["window"], &record["title"], &record["address"]))
        .collect();
    assert_eq!(
        fronts,
        [
            (
                &json!(70),
                &json!("Inbox"),
                &json!("https://mail.example.org/")
            ),
            (
                &json!(80),
                &json!("Docs"),
                &json!("https://docs.example.org/front")
            ),
        ],
        "each window's tab in front, and none where two windows' are alike: {said}"
    );
}
