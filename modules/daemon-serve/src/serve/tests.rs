//! The serving side's cases: watches by kind, place and pace, what a
//! daemon hears of them, its own asks and how its sources stand, each
//! asked over a socket as the shell asks.

use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::mpsc::channel;

use daemon_client::Client;

use super::*;

const ROWS: Domain = Domain {
    name: "agents",
    kinds: &["row", "summary"],
    sole: &[],
    named_only: &[],
};

/// A folder of the test's own, gone when it ends.
struct Folder(PathBuf);

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn served(name: &str, own: Own) -> (Arc<Shared>, Client, Folder) {
    let folder = std::env::temp_dir().join(format!("daemon-serve-{name}-{}", std::process::id()));
    let socket = folder.join("daemon.sock");
    let shared = Shared::new(ROWS, own);
    accept(
        Arc::clone(&shared),
        listen(&socket).expect("the socket is bound"),
    );
    (shared, Client::connect(&socket), Folder(folder))
}

fn row(place: &str, id: &str, name: &str) -> Value {
    json!({"key": format!("row:{place}:{id}"), "kind": "row", "row": {"name": name}})
}

#[test]
fn a_watch_is_answered_whole_then_told_only_what_moved() {
    let (shared, mut client, _folder) = served("moved", Box::new(|_| None));
    shared.change(
        vec![row("agents", "O", "kelp"), row("agents", "P", "reef")],
        json!({}),
    );
    let answer = client.ask(&json!({"id": 1, "ask": "watch", "kind": "row"}));
    assert_eq!(
        answer["records"].as_array().map(Vec::len),
        Some(2),
        "both rows, whole"
    );
    shared.change(vec![row("agents", "O", "harbour")], json!({}));
    let told = client.heard();
    assert_eq!(
        told["changed"],
        json!([row("agents", "O", "harbour")]),
        "the row that moved"
    );
    assert_eq!(
        told["gone"],
        json!(["row:agents:P"]),
        "and the key that went"
    );
}

#[test]
fn a_watch_of_some_fields_hears_them_alone_and_only_as_they_move() {
    let (shared, mut client, _folder) = served("fields", Box::new(|_| None));
    let kelp = |name: &str, order: u64| json!({"key": "row:agents:O", "row": {"name": name}, "order": order});
    shared.change(vec![kelp("kelp", 100)], json!({}));
    let asked = json!({"id": 1, "ask": "watch", "kind": "row", "fields": ["order"]});
    assert_eq!(
        client.ask(&asked)["records"],
        json!([{"key": "row:agents:O", "order": 100}]),
        "its key and its field"
    );
    // Renamed, which the watch doesn't hear, then used, which it does.
    shared.change(vec![kelp("reef", 100)], json!({}));
    shared.change(vec![kelp("reef", 200)], json!({}));
    assert_eq!(
        client.heard()["changed"],
        json!([{"key": "row:agents:O", "order": 200}]),
        "only the use, and only its field"
    );
}

#[test]
fn a_list_and_a_whole_answer_keep_the_order_the_daemon_gave() {
    let (shared, mut client, _folder) = served("order", Box::new(|_| None));
    // Given against the order of their keys, as windows are given the
    // one used last first.
    shared.change(
        vec![row("agents", "P", "reef"), row("agents", "O", "kelp")],
        json!({}),
    );
    let keys = |answer: &Value| -> Vec<String> {
        let records = answer["records"].as_array().cloned().unwrap_or_default();
        records
            .iter()
            .map(|record| record["key"].to_string())
            .collect()
    };
    let given = [r#""row:agents:P""#, r#""row:agents:O""#];
    assert_eq!(
        keys(&client.ask(&json!({"id": 1, "ask": "list", "kind": "row"}))),
        given,
        "listed"
    );
    assert_eq!(
        keys(&client.ask(&json!({"id": 2, "ask": "watch", "kind": "row"}))),
        given,
        "watched"
    );
}

#[test]
fn a_list_of_a_place_has_only_its_rows() {
    let (shared, mut client, _folder) = served("listed", Box::new(|_| None));
    shared.change(
        vec![row("agents", "O", "kelp"), row("mail", "m", "hi")],
        json!({}),
    );
    let asked = json!({"id": 1, "ask": "list", "kind": "row", "place": "mail"});
    assert_eq!(
        client.ask(&asked)["records"],
        json!([row("mail", "m", "hi")]),
        "only Mail's row"
    );
}

#[test]
fn a_daemon_reads_the_keys_its_watches_name() {
    let (shared, mut client, _folder) = served("keyed", Box::new(|_| None));
    shared.change(vec![row("agents", "O", "kelp")], json!({}));
    client.ask(&json!({"id": 1, "ask": "watch", "kind": "row"}));
    client.ask(&json!({"id": 2, "ask": "watch", "keys": ["row:agents:O"]}));
    assert_eq!(
        shared.keys_watched().into_iter().collect::<Vec<_>>(),
        vec!["row:agents:O".to_owned()],
        "the one key named"
    );
    client.ask(&json!({"id": 2, "ask": "unwatch"}));
    assert!(shared.keys_watched().is_empty(), "none once it's unwatched");
}

#[test]
fn a_watch_of_a_place_hears_only_its_rows() {
    let (shared, mut client, _folder) = served("place", Box::new(|_| None));
    shared.change(
        vec![row("agents", "O", "kelp"), row("mail", "m", "hi")],
        json!({}),
    );
    let asked = json!({"id": 1, "ask": "watch", "kind": "row", "place": "agents"});
    let answer = client.ask(&asked);
    assert_eq!(
        answer["records"],
        json!([row("agents", "O", "kelp")]),
        "only Agents' row"
    );
    shared.change(vec![row("agents", "O", "reef")], json!({}));
    let told = client.heard();
    assert_eq!(
        told["changed"],
        json!([row("agents", "O", "reef")]),
        "only Agents' row"
    );
    assert!(
        told.get("gone").is_none(),
        "Mail's row going is another place's: {told}"
    );
}

#[test]
fn a_paced_watch_is_told_all_that_moved_in_one_line_once_its_pace_is_up() {
    let (shared, mut client, _folder) = served("paced", Box::new(|_| None));
    shared.change(
        vec![row("agents", "O", "kelp"), row("agents", "P", "reef")],
        json!({}),
    );
    let pace = Duration::from_millis(300);
    client.ask(&json!({"id": 1, "ask": "watch", "kind": "row", "every": 300}));
    let began = Instant::now();
    shared.change(
        vec![row("agents", "O", "harbour"), row("agents", "P", "reef")],
        json!({}),
    );
    shared.change(vec![row("agents", "O", "buoy")], json!({}));
    let told = client.heard();
    assert!(
        began.elapsed() >= pace,
        "told before its pace was up: {told}"
    );
    assert_eq!(
        told["changed"],
        json!([row("agents", "O", "buoy")]),
        "the row as it last read, once"
    );
    assert_eq!(
        told["gone"],
        json!(["row:agents:P"]),
        "and the key that went"
    );
}

#[test]
fn a_watch_asked_again_keeps_on_at_its_new_pace_and_tells_what_it_held_back() {
    let (shared, mut client, _folder) = served("kept", Box::new(|_| None));
    shared.change(
        vec![row("agents", "O", "kelp"), row("agents", "P", "reef")],
        json!({}),
    );
    let watch = json!({"id": "all", "ask": "watch", "kind": "row", "place": "agents"});
    client.ask(&watch);
    let mut shut = watch.clone();
    shut["every"] = json!(60_000);
    client.send(&shut);
    // Asks are answered in turn, so once this is, the watch is paced.
    client.ask(&json!({"id": "paced", "ask": "health"}));
    shared.change(vec![row("agents", "O", "harbour")], json!({}));
    shared.change(
        vec![row("agents", "O", "buoy"), row("agents", "Q", "lantern")],
        json!({}),
    );
    client.send(&json!({"id": "health", "ask": "health"}));
    let next = client.heard();
    assert_eq!(
        next["id"], "health",
        "paced, it was told nothing yet, and asked again nothing whole: {next}"
    );
    client.send(&watch);
    let told = client.heard();
    assert_eq!(
        (&told["id"], &told["changed"], &told["gone"]),
        (
            &json!("all"),
            &json!([row("agents", "O", "buoy"), row("agents", "Q", "lantern")]),
            &json!(["row:agents:P"])
        ),
        "at once again, it's told at once all it held back, in one line"
    );
}

#[test]
fn a_daemon_hears_each_watch_begin_and_end() {
    let (shared, mut client, folder) = served("heeded", Box::new(|_| None));
    let (telling, told) = channel();
    shared.on_watches(Box::new(move || {
        let _ = telling.send(());
    }));
    assert!(!shared.watches("row", "agents"), "nothing watches yet");
    let watch = json!({"id": 1, "ask": "watch", "kind": "row", "place": "agents"});
    client.ask(&watch);
    assert!(told.recv().is_ok(), "told as it begins");
    assert!(shared.watches("row", "agents"), "its place is watched");
    assert!(!shared.watches("row", "mail"), "another isn't");
    client.ask(&json!({"id": 1, "ask": "unwatch"}));
    assert!(told.recv().is_ok(), "told as it's unwatched");
    assert!(!shared.watches("row", "agents"), "unwatched, it isn't");
    let mut another = Client::connect(&folder.0.join("daemon.sock"));
    another.ask(&watch);
    assert!(told.recv().is_ok(), "told as the other begins");
    drop(another);
    assert!(told.recv().is_ok(), "told as its connection ends");
    assert!(!shared.watches("row", "agents"), "its watch went with it");
}

#[test]
fn the_daemons_own_asks_are_its_to_answer() {
    let (telling, told) = channel();
    let own: Own = Box::new(move |said| {
        (said["ask"] == "sharing").then(|| {
            let _ = telling.send(said["sharing"].clone());
            json!({"sharing": said["sharing"]})
        })
    });
    let (_shared, mut client, _folder) = served("own", own);
    let answer = client.ask(&json!({"id": 1, "ask": "sharing", "sharing": true}));
    assert_eq!(
        answer,
        json!({"id": 1, "sharing": true}),
        "answered by the daemon"
    );
    assert_eq!(told.recv().ok(), Some(json!(true)), "which heard it");
    let refused = client.ask(&json!({"id": 2, "ask": "dance"}));
    assert_eq!(
        refused["error"]["code"], "unknown-ask",
        "one it doesn't know is refused"
    );
    let hello = client.ask(&json!({"id": 3, "ask": "hello"}));
    assert_eq!(
        (hello["domain"].as_str(), hello["version"].as_u64()),
        (Some("agents"), Some(1))
    );
}

#[test]
fn a_peered_ask_is_answered_knowing_who_asked() {
    let own: Own = Box::new(|said| (said["ask"] == "who").then(|| json!({"who": "unknown"})));
    let (shared, mut client, _folder) = served("peered", own);
    // The peer's credentials, as the kernel gives them for the socket, are
    // this test's process, which is what asked.
    shared.peered(
        &["who"],
        Box::new(|_, stream| {
            let fd = std::os::fd::AsRawFd::as_raw_fd(stream);
            let mut said = std::mem::MaybeUninit::<[u32; 3]>::zeroed();
            let mut length = 12u32;
            // SAFETY: SO_PEERCRED fills three u32s (pid, uid, gid) into a
            // buffer of that size, which `length` says.
            let read = unsafe {
                libc_getsockopt(fd, 1, 17, said.as_mut_ptr().cast(), &raw mut length)
            };
            // SAFETY: zeroed, and filled when the call succeeded.
            let [pid, _, _] = unsafe { said.assume_init() };
            (read == 0).then(|| json!({"who": pid}))
        }),
    );
    let answer = client.ask(&json!({"id": 1, "ask": "who"}));
    assert_eq!(
        answer,
        json!({"id": 1, "who": std::process::id()}),
        "answered by what knows its connection, not by the plain own asks"
    );
}

unsafe extern "C" {
    #[link_name = "getsockopt"]
    fn libc_getsockopt(
        fd: i32,
        level: i32,
        name: i32,
        value: *mut std::ffi::c_void,
        length: *mut u32,
    ) -> i32;
}

#[test]
fn a_slow_ask_holds_up_no_ask_after_it() {
    let (letting, waiting) = channel::<()>();
    let waiting = Mutex::new(waiting);
    let own: Own = Box::new(move |said| {
        (said["ask"] == "share").then(|| {
            // As a share waits on the desktop's portal for Joel to pick.
            let _ = waiting.lock().map(|waiting| waiting.recv());
            json!({"done": true})
        })
    });
    let (shared, mut client, _folder) = served("slow", own);
    shared.slow(&["share"]);
    client.send(&json!({"id": 1, "ask": "share"}));
    let hello = client.ask(&json!({"id": 2, "ask": "hello"}));
    assert_eq!(
        hello["state"], "answering",
        "answered while the share waits: {hello}"
    );
    letting.send(()).expect("the share is let go");
    assert_eq!(
        client.heard(),
        json!({"id": 1, "done": true}),
        "the share answered once it's done"
    );
}

#[test]
fn health_says_why_a_source_stands_as_it_does_and_every_other_line_its_word() {
    let (shared, mut client, _folder) = served("reason", Box::new(|_| None));
    let why = "Tuple's stream ended";
    shared.change(
        Vec::new(),
        json!({"tuple": {"state": "refused", "reason": why}}),
    );
    let hello = client.ask(&json!({"id": 1, "ask": "hello"}));
    assert_eq!(
        hello["sources"],
        json!({"tuple": "refused"}),
        "hello says the word"
    );
    let health = client.ask(&json!({"id": 2, "ask": "health"}));
    assert_eq!(
        health["sources"],
        json!({"tuple": {"state": "refused", "reason": why}}),
        "health says why"
    );
    client.ask(&json!({"id": 3, "ask": "watch", "kind": "row"}));
    shared.change(Vec::new(), json!({"tuple": "answering"}));
    assert_eq!(
        client.heard(),
        json!({"id": 3, "sources": {"tuple": "answering"}}),
        "a watch is told the word, nothing moved since hello having said nothing more"
    );
    let health = client.ask(&json!({"id": 4, "ask": "health"}));
    assert_eq!(
        health["sources"],
        json!({"tuple": {"state": "answering", "reason": ""}}),
        "a source given by its word has no reason"
    );
}

#[test]
fn a_watch_hears_how_the_sources_stand_when_they_moved_since_its_hello() {
    let (shared, mut client, _folder) = served("since-hello", Box::new(|_| None));
    shared.change(Vec::new(), json!({"tuple": "starting"}));
    client.ask(&json!({"id": "hello", "ask": "hello"}));
    // Tuple answering after hello said it was starting, with no watch yet
    // to tell.
    shared.change(Vec::new(), json!({"tuple": "answering"}));
    client.ask(&json!({"id": "all", "ask": "watch", "kind": "row"}));
    assert_eq!(
        client.heard(),
        json!({"id": "all", "sources": {"tuple": "answering"}}),
        "the watch hears what hello no longer says"
    );
}
