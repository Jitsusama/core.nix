//! slack-unreads: what wants Joel held whole, each conversation named
//! once, a notice said by its history, the socket systemd hands it, and
//! a search asked over it.

use std::process::Command;

use super::{
    StandIn, asked, credentials, forget, listed, socket, unreads, unreads_from, until, waiting,
    wanting,
};

#[test]
fn what_wants_joel_is_held_whole_each_conversation_named_once() {
    let stand_in = StandIn::new(wanting);
    let path = credentials("unreads");
    let run = path.with_extension("unreads");
    // Asked before, as when Slack refused the sign-in an earlier run had.
    std::fs::write(waiting(&run), "{}").unwrap();
    let mut program = unreads(&stand_in, &path, &run);
    until("the socket never came", || socket(&run).exists());
    until("Slack was never heard", || {
        !listed(&run, "slack-unread").is_empty()
    });
    // Slack answering, nothing waits on Joel's signing in.
    until("Joel was still asked to sign in", || {
        !waiting(&run).exists()
    });
    assert_eq!(
        listed(&run, "slack-unread"),
        [
            serde_json::json!({"id": "D1", "name": "Ada Lovelace", "kind": "dm", "count": 2,
                "link": "slack://channel?team=T1&id=D1", "latest": ""}),
            serde_json::json!({"id": "C2", "name": "#launcher", "kind": "mention", "count": 1,
                "link": "slack://channel?team=T1&id=C2", "latest": ""}),
            serde_json::json!({"id": "threads", "name": "Threads", "kind": "thread",
                "count": 3, "link": "slack://open?team=T1", "latest": ""}),
            serde_json::json!({"id": "C3", "name": "#random", "kind": "unread", "count": 0,
                "link": "slack://channel?team=T1&id=C3", "latest": ""}),
        ]
    );
    // What he read last, newest first, named as the unread are.
    assert_eq!(
        listed(&run, "slack-recent"),
        [
            serde_json::json!({"id": "C3", "name": "#random", "kind": "channel",
                "link": "slack://channel?team=T1&id=C3", "read": "1730200000.000100"}),
            serde_json::json!({"id": "D1", "name": "Ada Lovelace", "kind": "dm",
                "link": "slack://channel?team=T1&id=D1", "read": "1730100000.000100"}),
        ]
    );
    // Asked again and again, each conversation is named only once a run.
    let asked = |method: &str| {
        stand_in
            .heard()
            .iter()
            .filter(|heard| heard.line.contains(method))
            .count()
    };
    until("Slack wasn't asked again", || asked("client.counts") >= 4);
    program.kill().unwrap();
    program.wait().unwrap();
    assert_eq!((asked("conversations.info"), asked("users.info")), (3, 1));
    let heard = stand_in.heard();
    assert!(
        heard.iter().all(|heard| heard.line.starts_with("GET /api/")
            && heard
                .headers
                .iter()
                .any(|header| header == "authorization: Bearer xoxc-mine"
                    || header == "Authorization: Bearer xoxc-mine")),
        "{heard:?}"
    );
    let _ = std::fs::remove_dir_all(socket(&run).parent().unwrap());
    forget(&path);
}

/// Slack as it answers a search for "launcher" and what's around a
/// message, or that it's had too many searches, for "busy"; and what
/// wants Joel, as `wanting` does.
fn searching(line: &str) -> String {
    if line.contains("search.messages") && line.contains("query=busy") {
        r#"{"ok": false, "error": "ratelimited"}"#.into()
    } else if line.contains("search.messages") && line.contains("query=grace") {
        concat!(
            r#"{"ok": true, "messages": {"matches": [{"channel": {"id": "C1","#,
            r#""is_channel": true, "name": "launcher"}, "ts": "1730300002.000100","#,
            r#""user": "U8", "username": "grace", "text": "Grace was here","#,
            r#""permalink": "https://slack/p2"}]}}"#
        )
        .into()
    } else if line.contains("users.info?user=U8") {
        r#"{"ok": false, "error": "user_not_found"}"#.into()
    } else if line.contains("search.messages") {
        concat!(
            r#"{"ok": true, "messages": {"matches": [{"channel": {"id": "C1","#,
            r#""is_channel": true, "name": "launcher"}, "ts": "1730300000.000100","#,
            r#""user": "U9", "username": "ada", "text": "The launcher is fast","#,
            r#""permalink": "https://slack/p1"}]}}"#
        )
        .into()
    } else if line.contains("conversations.history") && line.contains("latest=") {
        concat!(
            r#"{"ok": true, "messages": [{"ts": "1730300000.000100", "user": "U9","#,
            r#""text": "The launcher is fast"}, {"ts": "1730299999.000100", "user": "U0","#,
            r#""text": "Is it?"}]}"#
        )
        .into()
    } else if line.contains("conversations.history") {
        concat!(
            r#"{"ok": true, "messages": [{"ts": "1730300000.000100", "user": "U9","#,
            r#""text": "The launcher is fast"}]}"#
        )
        .into()
    } else if line.contains("users.info?user=U9") {
        r#"{"ok": true, "user": {"id": "U9", "name": "ada", "profile": {"real_name": "Ada L"}}}"#
            .into()
    } else {
        wanting(line)
    }
}

/// Slack as it answers for #random's newest messages, a post and a reply
/// also sent to the channel, and what wants Joel, as `wanting` does.
fn posting(line: &str) -> String {
    if line.contains("conversations.history") && line.contains("channel=C3") {
        concat!(
            r#"{"ok": true, "messages": [{"ts": "1730300000.000100", "user": "U9","#,
            r#""text": "Lunch at noon?"}, {"ts": "1730299990.000100", "user": "U0","#,
            r#""subtype": "thread_broadcast", "thread_ts": "1730200000.000100","#,
            r#""text": "On it"}]}"#
        )
        .into()
    } else {
        wanting(line)
    }
}

#[test]
fn a_notice_from_a_channel_is_said_a_post_or_a_reply_by_its_history() {
    let stand_in = StandIn::new(posting);
    let path = credentials("unreads-posted");
    let run = path.with_extension("posted");
    let mut program = unreads(&stand_in, &path, &run);
    until("the socket never came", || socket(&run).exists());
    let posted = |id: u64, channel: &str, words: &str| {
        asked(
            &run,
            &serde_json::json!({"id": id, "ask": "posted", "channel": channel,
                "words": words, "at": 1_730_300_001_000_u64}),
        )
    };
    // Unknown until a round has named the channel.
    until("#random was never named", || {
        posted(1, "#random", "").get("error").is_none()
    });
    let post = |id: u64, channel: &str, words: &str| posted(id, channel, words)["post"].clone();
    assert_eq!(post(2, "#random", "Lunch at noon?"), true, "a post");
    assert_eq!(post(3, "random", "On it"), false, "a broadcast reply");
    assert_eq!(post(4, "#random", "sure"), false, "a thread's reply");
    let unknown = posted(5, "#nowhere", "hi");
    assert_eq!(unknown["error"]["code"], "unknown", "{unknown}");
    program.kill().unwrap();
    program.wait().unwrap();
    let _ = std::fs::remove_dir_all(socket(&run).parent().unwrap());
    forget(&path);
}

#[test]
fn a_socket_systemd_hands_it_is_the_one_it_answers_on() {
    // As systemd does for slack-unreads.socket: the socket is listened on
    // before the daemon runs, and handed to it as it starts, so the ask
    // that started it is answered. A daemon that made a socket of its own
    // would leave that ask on one nobody accepts from.
    let stand_in = StandIn::new(wanting);
    let path = credentials("unreads-handed");
    let run = path.with_extension("handed");
    std::fs::create_dir_all(socket(&run).parent().unwrap()).unwrap();
    let mut launcher = Command::new("systemd-socket-activate");
    launcher
        .arg(format!("--listen={}", socket(&run).display()))
        .arg(env!("CARGO_BIN_EXE_slack-unreads"));
    let mut program = unreads_from(launcher, &stand_in, &path, &run);
    until("the socket never came", || socket(&run).exists());
    let hello = asked(
        &run,
        &serde_json::json!({"id": 1, "ask": "hello", "version": 1}),
    );
    assert_eq!(hello["domain"], "slack");
    program.kill().unwrap();
    program.wait().unwrap();
}

#[test]
fn a_search_asked_over_the_socket_says_each_message_and_what_is_around_it() {
    let stand_in = StandIn::new(searching);
    let path = credentials("unreads-search");
    let run = path.with_extension("search");
    let mut program = unreads(&stand_in, &path, &run);
    until("the socket never came", || socket(&run).exists());
    let hello = || {
        asked(
            &run,
            &serde_json::json!({"id": 1, "ask": "hello", "version": 1}),
        )
    };
    assert_eq!(hello()["domain"], "slack");
    // Starting until Slack is first asked, then answering.
    until("Slack never answered", || {
        hello()["sources"]["slack"] == "answering"
    });
    let found = asked(
        &run,
        &serde_json::json!({"id": 2, "ask": "search", "words": "launcher"}),
    );
    let record = &found["records"][0];
    assert_eq!(
        (
            &record["key"],
            &record["place"],
            &record["who"],
            &record["first"],
            &record["team"]
        ),
        (
            &serde_json::json!("slack-message:C1:1730300000.000100"),
            &serde_json::json!("#launcher"),
            // Named as a message held for Joel is, never by a handle.
            &serde_json::json!("Ada L"),
            &serde_json::json!("The launcher is fast"),
            // A match that doesn't say its workspace is the sign-in's.
            &serde_json::json!("T1")
        )
    );
    let near = asked(
        &run,
        &serde_json::json!({"id": 3, "ask": "around", "keys": [record["key"]]}),
    );
    let around: Vec<(&str, bool)> = near["records"][0]["around"]
        .as_array()
        .unwrap()
        .iter()
        .map(|one| {
            (
                one["text"].as_str().unwrap(),
                one["asked"].as_bool().unwrap(),
            )
        })
        .collect();
    assert_eq!(around, [("Is it?", false), ("The launcher is fast", true)]);
    let busy = asked(
        &run,
        &serde_json::json!({"id": 4, "ask": "search", "words": "busy"}),
    );
    assert_eq!(busy["error"]["code"], "too-many");
    // Someone Slack won't name keeps their handle, and the search answers.
    let refused = asked(
        &run,
        &serde_json::json!({"id": 5, "ask": "search", "words": "grace"}),
    );
    assert_eq!(refused["records"][0]["who"], "grace", "{refused}");
    // Ada, named once, is never asked for again.
    asked(
        &run,
        &serde_json::json!({"id": 6, "ask": "search", "words": "launcher"}),
    );
    let asked_for_ada = stand_in
        .heard()
        .iter()
        .filter(|heard| heard.line.contains("users.info?user=U9"))
        .count();
    assert_eq!(asked_for_ada, 1, "{:?}", stand_in.heard());
    program.kill().unwrap();
    program.wait().unwrap();
    // Still only reading.
    assert!(
        stand_in
            .heard()
            .iter()
            .all(|heard| heard.line.starts_with("GET /api/")),
        "{:?}",
        stand_in.heard()
    );
    let _ = std::fs::remove_dir_all(socket(&run).parent().unwrap());
    forget(&path);
}
