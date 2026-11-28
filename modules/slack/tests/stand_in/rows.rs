//! slack-unreads' rows and messages: each conversation served as its
//! row, what's unread watched and each change said as it changes, and
//! the messages wanting Joel said whole once every history was read and
//! held until he reads them in Slack.

use std::io::{BufRead, BufReader, Write};
use std::process::Command;
use std::time::Duration;

use super::{
    StandIn, WORDS, asked, credentials, forget, listed, socket, unreads, unreads_from, until,
    wants_joel,
};

/// Whether Joel has read Ada's direct message, for `reading`.
static READ: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Slack as `wants_joel` says it until Joel reads Ada's direct message,
/// and once he has, nothing unread there and a second mention in
/// #launcher.
fn reading(line: &str) -> String {
    if READ.load(std::sync::atomic::Ordering::SeqCst) && line.contains("client.counts") {
        concat!(
            r#"{"ok": true, "ims": [{"id": "D1", "has_unreads": false, "mention_count": 0,"#,
            r#""last_read": "1730100000.000100"}],"#,
            r#""mpims": [], "channels": [{"id": "C2", "has_unreads": true, "mention_count": 2},"#,
            r#"{"id": "C3", "has_unreads": true, "mention_count": 0,"#,
            r#""last_read": "1730200000.000100"}],"#,
            r#""threads": {"has_unreads": true, "mention_count": 3}}"#
        )
        .into()
    } else {
        wants_joel(line)
    }
}

/// One connection to a running slack-unreads' socket, kept open, as the
/// shell's is.
struct Connection {
    writing: std::os::unix::net::UnixStream,
    reading: BufReader<std::os::unix::net::UnixStream>,
}

impl Connection {
    fn to(run: &std::path::Path) -> Connection {
        let stream = std::os::unix::net::UnixStream::connect(socket(run)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        Connection {
            writing: stream.try_clone().unwrap(),
            reading: BufReader::new(stream),
        }
    }

    fn ask(&mut self, ask: &serde_json::Value) {
        writeln!(self.writing, "{ask}").unwrap();
    }

    /// The next line said that `wanted` takes, passing over the rest.
    fn heard(&mut self, wanted: impl Fn(&serde_json::Value) -> bool) -> serde_json::Value {
        loop {
            let mut line = String::new();
            self.reading.read_line(&mut line).unwrap();
            assert!(!line.is_empty(), "the daemon hung up");
            let said: serde_json::Value = serde_json::from_str(&line).unwrap();
            if wanted(&said) {
                return said;
            }
        }
    }

    /// The answer to an ask, by its id.
    fn answer(&mut self, ask: &serde_json::Value) -> serde_json::Value {
        self.ask(ask);
        let id = ask["id"].clone();
        self.heard(|said| said["id"] == id && said.get("changed").is_none())
    }
}

/// Each record's key, in the order said.
fn keys(records: &serde_json::Value) -> Vec<&str> {
    records
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["key"].as_str().unwrap())
        .collect()
}

#[test]
fn handed_the_launchers_words_it_serves_each_conversation_as_its_row() {
    let stand_in = StandIn::new(wants_joel);
    let path = credentials("unreads-rows");
    let run = path.with_extension("rows");
    let mut command = Command::new(env!("CARGO_BIN_EXE_slack-unreads"));
    command.env("LAUNCHER_WORDS", WORDS);
    let program = unreads_from(command, &stand_in, &path, &run);
    until("the socket never came", || socket(&run).exists());
    let rows = |place: &str| {
        asked(
            &run,
            &serde_json::json!({"id": 1, "ask": "list", "kind": "row", "place": place}),
        )["records"]
            .clone()
    };
    until("no row was served", || {
        rows("slack-unread")
            .as_array()
            .is_some_and(|all| !all.is_empty())
    });
    let unread = rows("slack-unread");
    assert_eq!(
        keys(&unread),
        [
            "row:slack-unread:D1",
            "row:slack-unread:C2",
            "row:slack-unread:threads",
            "row:slack-unread:C3"
        ]
    );
    let ada = &unread[0]["row"];
    assert_eq!(
        (&ada["name"], &ada["side"], &ada["note"]),
        (
            &serde_json::json!("Ada Lovelace"),
            &serde_json::json!([{"said": "2 to read"}]),
            &serde_json::json!("a direct message")
        )
    );
    assert_eq!(keys(&rows("slack-recent"))[0], "row:slack-recent:C3");
    program.stop();
    let _ = std::fs::remove_dir_all(socket(&run).parent().unwrap());
    forget(&path);
}

#[test]
fn what_is_unread_is_listed_and_watched_each_change_said_as_it_changes() {
    let stand_in = StandIn::new(reading);
    let path = credentials("unreads-watch");
    let run = path.with_extension("watch");
    let program = unreads(&stand_in, &path, &run);
    until("the socket never came", || socket(&run).exists());
    let mut shell = Connection::to(&run);
    until("Slack was never heard", || {
        let listed = asked(
            &run,
            &serde_json::json!({"id": 1, "ask": "list", "kind": "slack-account"}),
        );
        listed["records"][0]["slack-account"]["standing"] == "ok"
    });
    let watch = serde_json::json!({
        "id": "all", "ask": "watch", "kind": ["slack-unread", "slack-account"],
    });
    let held = shell.answer(&watch);
    // What wants Joel first, then what's merely unread, then the account,
    // and nothing of Recent, which wasn't asked for.
    assert_eq!(
        keys(&held["records"]),
        [
            "slack-unread:D1",
            "slack-unread:C2",
            "slack-unread:threads",
            "slack-unread:C3",
            "slack-account:slack"
        ]
    );
    let ada = &held["records"][0];
    assert_eq!(
        (&ada["slack-unread"]["name"], &ada["slack-unread"]["count"]),
        (&serde_json::json!("Ada Lovelace"), &serde_json::json!(2))
    );
    assert!(
        ada["heard"].as_u64().is_some_and(|at| at > 0),
        "Ada's conversation should say when it was heard, not {}",
        ada["heard"]
    );
    let threads = held["records"][2].clone();
    // Joel reads Ada's message, and a second mention comes in #launcher:
    // only those two are said, Ada's as gone.
    READ.store(true, std::sync::atomic::Ordering::SeqCst);
    let changed = shell.heard(|said| said["id"] == "all" && said.get("changed").is_some());
    assert_eq!(
        (keys(&changed["changed"]), &changed["gone"]),
        (
            vec!["slack-unread:C2"],
            &serde_json::json!(["slack-unread:D1"])
        )
    );
    assert_eq!(changed["changed"][0]["slack-unread"]["count"], 2);
    // Threads, asked about again and again, weren't heard again.
    let now = shell.answer(&serde_json::json!({
        "id": 2, "ask": "get", "keys": ["slack-unread:threads", "slack-unread:D1"],
    }));
    assert_eq!(now["records"], serde_json::json!([threads]));
    // Recent is its own kind, the one read last first.
    let recent = shell.answer(&serde_json::json!({
        "id": 3, "ask": "list", "kind": "slack-recent",
    }));
    assert_eq!(keys(&recent["records"])[0], "slack-recent:C3");
    assert_eq!(
        recent["records"][0]["slack-recent"]["read"],
        "1730200000.000100"
    );
    // Messages wanting Joel are held only once their conversation's
    // history says them, which this Slack never does, so a watch of them,
    // as Search Slack's is, holds nothing; a kind it doesn't know is said
    // so.
    let message = shell.answer(&serde_json::json!({
        "id": 4, "ask": "watch", "kind": "slack-message",
    }));
    assert_eq!(message["records"], serde_json::json!([]));
    let unknown = shell.answer(&serde_json::json!({
        "id": 6, "ask": "list", "kind": "slack-channel",
    }));
    assert_eq!(unknown["error"]["code"], "unknown-kind");
    for id in [serde_json::json!("all"), serde_json::json!(4)] {
        let unwatched = shell.answer(&serde_json::json!({"id": id, "ask": "unwatch"}));
        assert_eq!(unwatched["unwatched"], true);
    }
    let health = shell.answer(&serde_json::json!({"id": 5, "ask": "health"}));
    assert_eq!(
        (&health["watches"], &health["sources"]["slack"]["state"]),
        (&serde_json::json!(0), &serde_json::json!("answering"))
    );
    program.stop();
    let _ = std::fs::remove_dir_all(socket(&run).parent().unwrap());
    forget(&path);
}

/// Whether Joel has read Ada's direct message in Slack, for `raising`.
static ADA_READ: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Slack with two messages from Ada in her direct message, one he's read
/// before them, and in #launcher one from Grace mentioning him beside one
/// that doesn't; then, once `ADA_READ`, Ada's read in Slack.
fn raising(line: &str) -> String {
    let ada = ADA_READ.load(std::sync::atomic::Ordering::SeqCst);
    let said = if line.contains("client.counts") {
        let (unread, count, read) = if ada {
            (false, 0, "1730100002.000100")
        } else {
            (true, 2, "1730100000.000100")
        };
        return format!(
            concat!(
                r#"{{"ok": true, "ims": [{{"id": "D1", "has_unreads": {}, "mention_count": {},"#,
                r#""latest": "1730100002.000100", "last_read": "{}"}}], "mpims": [],"#,
                r#""channels": [{{"id": "C2", "has_unreads": true, "mention_count": 1,"#,
                r#""latest": "1730100003.000100", "last_read": "1730100000.000100"}}]}}"#
            ),
            unread, count, read
        );
    } else if line.contains("conversations.history?channel=D1") {
        concat!(
            r#"{"ok": true, "messages": ["#,
            r#"{"ts": "1730100002.000100", "user": "U7", "text": "are you there?"},"#,
            r#"{"ts": "1730100001.000100", "user": "U7", "text": "hi <@U0>"}]}"#
        )
    } else if line.contains("conversations.history?channel=C2") {
        concat!(
            r#"{"ok": true, "messages": ["#,
            r#"{"ts": "1730100003.000100", "user": "U8", "text": "<@U0> can you look"},"#,
            r#"{"ts": "1730100002.000100", "user": "U8", "text": "lunch?"}]}"#
        )
    } else if line.contains("users.info?user=U8") {
        r#"{"ok": true, "user": {"id": "U8", "profile": {"real_name": "Grace Hopper"}}}"#
    } else if line.contains("auth.test") {
        r#"{"ok": true, "url": "https://ada.slack.com/"}"#
    } else {
        return wants_joel(line);
    };
    said.into()
}

/// Whether Slack gives #launcher's history, for `missing`.
static LAUNCHER_ANSWERS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Slack with a direct message from Ada he's never opened, its history
/// over two pages, and in #launcher a mention of him whose history Slack
/// doesn't give until `LAUNCHER_ANSWERS`.
fn missing(line: &str) -> String {
    let said = if line.contains("client.counts") {
        concat!(
            r#"{"ok": true, "ims": [{"id": "D3", "has_unreads": true, "mention_count": 1,"#,
            r#""latest": "1730100002.000100"}], "mpims": [],"#,
            r#""channels": [{"id": "C2", "has_unreads": true, "mention_count": 1,"#,
            r#""latest": "1730100003.000100", "last_read": "1730100000.000100"}]}"#
        )
    } else if line.contains("conversations.history?channel=D3") && line.contains("cursor=p2") {
        concat!(
            r#"{"ok": true, "has_more": false, "messages": ["#,
            r#"{"ts": "1730100001.000100", "user": "U7", "text": "hello, new here"}]}"#
        )
    } else if line.contains("conversations.history?channel=D3") {
        concat!(
            r#"{"ok": true, "has_more": true, "messages": ["#,
            r#"{"ts": "1730100002.000100", "user": "U7", "text": "are you there?"}],"#,
            r#""response_metadata": {"next_cursor": "p2"}}"#
        )
    } else if line.contains("conversations.history?channel=C2") {
        if !LAUNCHER_ANSWERS.load(std::sync::atomic::Ordering::SeqCst) {
            return r#"{"ok": false, "error": "internal_error"}"#.into();
        }
        concat!(
            r#"{"ok": true, "messages": ["#,
            r#"{"ts": "1730100003.000100", "user": "U8", "text": "<@U0> can you look"}]}"#
        )
    } else {
        return raising(line);
    };
    said.into()
}

#[test]
fn the_messages_are_said_whole_only_once_every_history_wanted_was_read() {
    let stand_in = StandIn::new(missing);
    let path = credentials("unreads-missing");
    let run = path.with_extension("missing");
    let program = unreads(&stand_in, &path, &run);
    until("the socket never came", || socket(&run).exists());
    // Ada's two, from the conversation he's never opened, read over both
    // pages of its history; #launcher's isn't given, so they aren't whole.
    until("Ada's messages were never held", || {
        listed(&run, "slack-message").len() == 2
    });
    let texts = |run: &std::path::Path| -> Vec<String> {
        listed(run, "slack-message")
            .iter()
            .map(|one| one["text"].as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(texts(&run), ["are you there?", "hello, new here"]);
    assert_eq!(
        listed(&run, "slack-account"),
        [serde_json::json!({"standing": "ok"})]
    );
    // A conversation never read has no mark to read on from, and none to
    // say.
    assert_eq!(listed(&run, "slack-read"), Vec::<serde_json::Value>::new());
    assert!(
        stand_in
            .heard()
            .iter()
            .filter(|heard| heard.line.contains("conversations.history?channel=D3"))
            .all(|heard| !heard.line.contains("oldest")),
        "a conversation never read should be asked for from no mark, in {:?}",
        stand_in.heard()
    );
    LAUNCHER_ANSWERS.store(true, std::sync::atomic::Ordering::SeqCst);
    until("the messages were never said whole", || {
        listed(&run, "slack-account")
            == [serde_json::json!({"standing": "ok", "messages": "whole"})]
    });
    assert_eq!(
        texts(&run),
        ["@someone can you look", "are you there?", "hello, new here"]
    );
    program.stop();
    let _ = std::fs::remove_dir_all(socket(&run).parent().unwrap());
    forget(&path);
}

#[test]
fn each_message_wanting_joel_is_held_until_he_reads_it_in_slack() {
    let stand_in = StandIn::new(raising);
    let path = credentials("unreads-raising");
    let run = path.with_extension("raising");
    let program = unreads(&stand_in, &path, &run);
    until("the socket never came", || socket(&run).exists());
    until("no message was held", || {
        listed(&run, "slack-message").len() == 3
    });
    // Newest first: Grace's mention, not her other message, then Ada's
    // two, each with Search Slack's fields and none done.
    let held = listed(&run, "slack-message");
    assert_eq!(
        held[0],
        serde_json::json!({"channel": "C2", "ts": "1730100003.000100", "place": "#launcher",
            "who": "Grace Hopper", "first": "@someone can you look",
            "mine": false, "text": "@someone can you look", "at": 1_730_100_003_000_u64,
            "link": "https://ada.slack.com/archives/C2/p1730100003000100", "team": "T1"})
    );
    // Every history wanted was read, so they're all there are.
    assert_eq!(
        listed(&run, "slack-account"),
        [serde_json::json!({"standing": "ok", "messages": "whole"})]
    );
    let said: Vec<(&str, &str)> = held[1..]
        .iter()
        .map(|one| (one["who"].as_str().unwrap(), one["text"].as_str().unwrap()))
        .collect();
    assert_eq!(
        said,
        [
            ("Ada Lovelace", "are you there?"),
            ("Ada Lovelace", "hi @someone")
        ]
    );
    assert_eq!(
        listed(&run, "slack-read")
            .iter()
            .map(|mark| (
                mark["channel"].as_str().unwrap(),
                mark["read"].as_str().unwrap()
            ))
            .collect::<Vec<_>>(),
        [("C2", "1730100000.000100"), ("D1", "1730100000.000100")]
    );
    let mut shell = Connection::to(&run);
    let watch = serde_json::json!({
        "id": "read", "ask": "watch", "kind": ["slack-message", "slack-read"],
    });
    shell.answer(&watch);
    // Joel reads Ada's messages in Slack: both are said done, read in
    // Slack, with her conversation's mark, by key, and nothing goes,
    // which a change says by naming nothing gone.
    ADA_READ.store(true, std::sync::atomic::Ordering::SeqCst);
    let changed = shell.heard(|said| said["id"] == "read" && said.get("changed").is_some());
    assert_eq!(
        (keys(&changed["changed"]), changed.get("gone")),
        (
            vec![
                "slack-message:D1:1730100001.000100",
                "slack-message:D1:1730100002.000100",
                "slack-read:D1"
            ],
            None
        )
    );
    let done = &changed["changed"][0]["slack-message"]["done"];
    assert_eq!(done["how"], "readInSlack");
    let at = done["at"].as_str().unwrap();
    assert!(
        at.len() == 20 && at.ends_with('Z') && at.as_bytes()[10] == b'T',
        "{at}"
    );
    assert_eq!(
        changed["changed"][2]["slack-read"]["read"],
        "1730100002.000100"
    );
    // Each conversation's history was read once, as its unreads came,
    // however often Slack was asked how they stood.
    let rounds = times(&stand_in, "client.counts");
    until("Slack wasn't asked again", || {
        times(&stand_in, "client.counts") >= rounds + 3
    });
    program.stop();
    let asked = [
        "conversations.history?channel=D1",
        "conversations.history?channel=C2",
        "users.info?user=U8",
        "auth.test",
    ]
    .map(|what| times(&stand_in, what));
    assert_eq!(asked, [1, 1, 1, 1]);
    let heard = stand_in.heard();
    assert!(
        heard.iter().all(|one| one.line.starts_with("GET /api/")),
        "{heard:?}"
    );
    let _ = std::fs::remove_dir_all(socket(&run).parent().unwrap());
    forget(&path);
}

/// How many times the stand-in was asked something its line holds.
fn times(stand_in: &StandIn, what: &str) -> usize {
    stand_in
        .heard()
        .iter()
        .filter(|heard| heard.line.contains(what))
        .count()
}
