//! Every open tab in one place, Chrome's from its session file and
//! kitty's from its told open list, with the windows a stand-in windows
//! daemon says are open: one list by when each was used, a program with
//! no window left out, and kitty's Go to the Tab and Close the Tab done
//! through stand-ins for niri and kitten as each is asked. Tabs' head
//! names a program whose tabs can't be read while its window is open.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::time::{Duration, Instant};

use chrome_session::written::{session, tab};
use daemon_client::{Client, until};
use serde_json::{Value, json};

use super::Running;

const KITTY: &str = "row:tabs:terminal:4242:2";
const NOTES: &str = "row:tabs:terminal:4242:1";
const LATER: &str = "row:tabs:https://later.example.org/";
const EARLIER: &str = "row:tabs:https://earlier.example.org/";

/// Chrome's session, a page looked at long after now and one long
/// before, so kitty's tab, looked at as this machine started, is between.
fn sessioned(running: &Running) {
    let mut commands = tab(1, 10, 0, "https://later.example.org/", 4_000_000_000_000);
    commands.extend(tab(1, 11, 1, "https://earlier.example.org/", 1_000));
    std::fs::write(
        running.path("chrome/Sessions/Session_1"),
        session(&commands),
    )
    .unwrap();
}

/// kitty's open list: one kitty of one OS window, its two tabs, the second
/// the active one, in a folder of Joel's home.
fn listed(running: &Running) {
    let home = running.path("").display().to_string();
    let os = json!([{"id": 1, "wm_class": "kitty", "active_tab_history": [1], "tabs": [
        {"id": 1, "is_active": false, "title": "notes", "windows": [{"id": 1, "cwd": "/"}]},
        {"id": 2, "is_active": true, "title": "core.nix",
         "windows": [{"id": 2, "is_active": true, "cwd": format!("{home}src")}]},
    ]}]);
    std::fs::write(running.path("lists/open"), format!("4242\t{os}\n")).unwrap();
}

/// The stand-in windows daemon, as this daemon watches it.
fn windows(running: &Running) -> UnixStream {
    let listener = UnixListener::bind(running.path("daemons/windows.sock")).unwrap();
    let (windows, _) = listener.accept().unwrap();
    let mut asked = String::new();
    BufReader::new(windows.try_clone().unwrap())
        .read_line(&mut asked)
        .unwrap();
    windows
}

/// The windows daemon's answer: Chrome's one window and kitty's one.
fn open() -> Value {
    json!({"id": "programs", "records": [
        {"key": "window:7", "owner": "chrome", "pid": 30, "opened": "google-chrome", "order": 5},
        {"key": "window:8", "owner": "terminal", "pid": 4242, "opened": "kitty", "order": 1},
    ]})
}

fn tell(windows: &UnixStream, said: &Value) {
    writeln!(&*windows, "{said}").unwrap();
}

/// The rows of these keys in the order they're served, once each is:
/// some are in the watch's answer and some told after, so each is asked
/// for until all are there.
fn served(client: &mut Client, keys: &[&str]) -> Vec<Value> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let said = client.ask(&json!({"id": "got", "ask": "get", "keys": keys}));
        let mut records: Vec<Value> = said["records"].as_array().unwrap().clone();
        if records.len() == keys.len() {
            records.sort_by_key(|record| record["row"]["order"].as_i64().unwrap_or(i64::MAX));
            return records;
        }
        assert!(
            Instant::now() < deadline,
            "not every tab was served: {said}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn chromes_and_kittys_tabs_are_one_list_by_when_each_was_used() {
    let running = Running::new("tabs");
    sessioned(&running);
    listed(&running);
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "watch", "kind": "row", "place": "tabs"}));
    let windows = windows(&running);
    tell(&windows, &open());
    let rows = served(&mut client, &[LATER, KITTY, NOTES, EARLIER]);
    let keys: Vec<&Value> = rows.iter().map(|record| &record["key"]).collect();
    assert_eq!(
        keys,
        [LATER, KITTY, NOTES, EARLIER],
        "the tabs weren't one list, the one used last first: {rows:?}"
    );
    let kitty = &rows[1]["row"];
    assert_eq!(
        (&kitty["in"], &kitty["name"], &kitty["side"]),
        (
            &json!("terminal"),
            &json!("core.nix"),
            &json!([{"said": "~/src", "keep": 1}])
        ),
        "kitty's tab isn't said in kitty, by its title and folder: {kitty}"
    );
    assert_eq!(
        rows[0]["row"]["in"], "chrome",
        "Chrome's page isn't said in Chrome"
    );
    assert_eq!(
        rows[0]["row"]["drawer"]["eyebrow"],
        json!([{"said": "in chrome", "keep": 2}, {"said": "tab", "keep": 1}]),
        "Chrome's page's drawer isn't said in its program"
    );
    let said = client.ask(&json!({"id": 2, "ask": "search", "place": "tabs", "words": "core"}));
    assert_eq!(
        said["keys"],
        json!([KITTY]),
        "kitty's tab isn't found by its title: {said}"
    );
}

#[test]
fn a_program_with_no_window_open_leaves_its_tabs_out() {
    let running = Running::new("tabs-gone");
    sessioned(&running);
    listed(&running);
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "watch", "kind": "row", "place": "tabs"}));
    let windows = windows(&running);
    tell(&windows, &open());
    served(&mut client, &[LATER, KITTY, NOTES, EARLIER]);
    let gone = |client: &mut Client| {
        let mut gone = client.gone();
        gone.sort_by_key(ToString::to_string);
        gone
    };
    tell(&windows, &json!({"id": "programs", "gone": ["window:7"]}));
    let gone_now = gone(&mut client);
    assert_eq!(
        gone_now,
        [json!(EARLIER), json!(LATER)],
        "Chrome's pages weren't left out once its window shut: {gone_now:?}"
    );
    tell(&windows, &json!({"id": "programs", "gone": ["window:8"]}));
    let gone_now = gone(&mut client);
    assert_eq!(
        gone_now,
        [json!(NOTES), json!(KITTY)],
        "kitty's tabs weren't left out once its window shut: {gone_now:?}"
    );
}

/// What Tabs' head says of a kitty whose entry names it Terminal.
const SILENT: &str = "Terminal isn't answering; its tabs come back when it does.";

#[test]
fn tabs_head_says_a_program_isnt_answering_while_its_window_is_open() {
    let running = Running::new("tabs-silent");
    // kitty's list has no file, so reading it fails.
    std::fs::remove_file(running.path("lists/open")).unwrap();
    // kitty's entry names it, as its Programs row is named.
    let entry = running.path("near/applications/terminal.desktop");
    std::fs::write(
        entry,
        "[Desktop Entry]\nType=Application\nName=Terminal\nExec=kitty\n",
    )
    .unwrap();
    sessioned(&running);
    let mut client = running.client();
    let watch = json!({"id": 1, "ask": "watch", "kind": ["row", "head"], "place": "tabs"});
    client.ask(&watch);
    let windows = windows(&running);
    tell(&windows, &open());
    let head = served(&mut client, &["head:tabs"]);
    assert_eq!(
        head[0]["head"],
        json!({"key": "head:tabs", "place": "tabs", "said": SILENT,
               "silent": [{"in": "terminal", "said": SILENT}]}),
        "Tabs' head didn't say kitty's tabs can't be read, by its entry's name"
    );
    served(&mut client, &[LATER, EARLIER]);
}

#[test]
fn going_to_a_kitty_tab_focuses_its_window_then_the_tab_and_closing_closes_it() {
    let running = Running::new("tabs-acts");
    let mut client = running.client();
    let said = client.ask(&json!({"id": 1, "ask": "tab:go", "pid": 4242, "tab": 2,
        "window": 8, "token": "his-press"}));
    assert!(
        said.get("error").is_none(),
        "Go to the Tab was refused: {said}"
    );
    let asked = |tool: &str| std::fs::read_to_string(running.path(&format!("{tool}.asked")));
    until("niri and kitten weren't asked to go to the tab", || {
        asked("kitten").is_ok_and(|text| text.lines().count() == 1)
    });
    assert_eq!(
        asked("niri").unwrap(),
        "msg action focus-window --id 8\n",
        "niri wasn't asked to focus the tab's OS window"
    );
    assert_eq!(
        asked("kitten").unwrap(),
        "@ --to unix:@kitty-4242 focus-tab --match id:2\n",
        "kitten wasn't asked to focus the tab"
    );
    let said = client.ask(&json!({"id": 2, "ask": "tab:close", "pid": 4242, "tab": 2}));
    assert!(
        said.get("error").is_none(),
        "Close the Tab was refused: {said}"
    );
    until("kitten wasn't asked to close the tab", || {
        asked("kitten").is_ok_and(|text| text.lines().count() == 2)
    });
    assert_eq!(
        asked("kitten").unwrap().lines().nth(1),
        Some("@ --to unix:@kitty-4242 close-tab --match id:2"),
        "kitten wasn't asked to close the tab"
    );
}
