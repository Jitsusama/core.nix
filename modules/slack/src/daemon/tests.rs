//! Slack's daemon's cases: the records what Slack said makes, and what a
//! watch of them hears, asked over a socket as the shell asks.

use daemon_client::Client;

use super::*;
use crate::messages::{Done, Mark, Raised};
use crate::unreads::{Kind, Seen};

/// The one account's key.
const ACCOUNT: &str = "slack-account:slack";

/// A folder of the test's own, gone when it ends.
struct Folder(PathBuf);

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The daemon serving on a socket of its own, signed in to no Slack, and
/// a client of it.
fn served(name: &str) -> (Daemon, Client, Folder) {
    let folder = std::env::temp_dir().join(format!("slack-daemon-{name}-{}", std::process::id()));
    let socket = folder.join("slack.sock");
    let daemon = listen(&socket, folder.join("no-credentials"), String::new());
    (daemon, Client::connect(&socket), Folder(folder))
}

/// The keys a line's changed records hold, in its order.
fn keys(line: &Value) -> Vec<String> {
    line["changed"]
        .as_array()
        .map(|records| {
            records
                .iter()
                .filter_map(|record| record["key"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// Whether nothing was told since: health, asked now, is the next line.
fn told_nothing(client: &mut Client) -> Value {
    client.send(&json!({"id": "nothing", "ask": "health"}));
    client.heard()
}

fn one(id: &str, kind: Kind, count: u64, at: &str) -> Conversation {
    Conversation {
        id: id.to_owned(),
        name: "Ada".to_owned(),
        kind,
        count,
        link: format!("slack://channel?team=T1&id={id}"),
        at: at.to_owned(),
    }
}

fn ok(conversations: Vec<Conversation>, recent: Vec<Conversation>) -> Said {
    Said {
        state: State::Ok,
        conversations,
        recent,
        messages: Vec::new(),
        marks: Vec::new(),
        seen: Vec::new(),
        whole: false,
    }
}

fn message(channel: &str, ts: &str, done: Option<Done>) -> Raised {
    Raised {
        key: search::key(channel, ts),
        channel: channel.to_owned(),
        ts: ts.to_owned(),
        place: "Ada".to_owned(),
        who: "Ada".to_owned(),
        user: "U7".to_owned(),
        named: true,
        mine: false,
        first: "hi".to_owned(),
        text: "hi".to_owned(),
        at: 1_730_300_001_000,
        link: "https://ada.slack.com/archives/D1/p1730300001000100".to_owned(),
        team: "T1".to_owned(),
        done,
    }
}

#[test]
fn a_message_wanting_joel_is_still_said_while_slack_doesnt_answer() {
    let mut said = ok(vec![one("D1", Kind::Dm, 1, "1")], vec![]);
    said.messages = vec![message("D1", "1730300001.000100", None)];
    said.state = State::Unreachable;
    let keys: Vec<String> = records(Some(&said))
        .into_iter()
        .map(|(key, _)| key)
        .collect();
    assert_eq!(
        keys,
        ["slack-message:D1:1730300001.000100", ACCOUNT],
        "the message and the account, no conversation"
    );
}

#[test]
fn the_account_says_the_messages_are_whole_only_once_they_are() {
    let mut said = ok(vec![], vec![]);
    let account = |said: Option<&Said>| {
        records(said)
            .pop()
            .map(|(_, record)| record["slack-account"].clone())
    };
    assert_eq!(
        account(Some(&said)),
        Some(json!({"standing": "ok"})),
        "not yet whole"
    );
    said.whole = true;
    assert_eq!(
        account(Some(&said)),
        Some(json!({"standing": "ok", "messages": "whole"})),
        "whole"
    );
    assert_eq!(
        account(None),
        Some(json!({"standing": "none"})),
        "nobody signed in"
    );
}

#[test]
fn a_message_wanting_joel_is_a_record_and_its_being_read_a_change_to_it() {
    let (daemon, mut client, _folder) = served("read");
    let asked = json!({"id": "m", "ask": "watch", "kind": ["slack-message", "slack-read"]});
    client.ask(&asked);
    let mut said = ok(vec![one("D1", Kind::Dm, 1, "1730300001.000100")], vec![]);
    said.messages = vec![message("D1", "1730300001.000100", None)];
    said.marks = vec![Mark {
        channel: "D1".to_owned(),
        read: "1730300000.000100".to_owned(),
        at: "2024-10-30T14:53:20Z".to_owned(),
    }];
    daemon.say(Some(&said), 10);
    let wanting = "slack-message:D1:1730300001.000100";
    let first = client.heard();
    assert_eq!(keys(&first), [wanting, "slack-read:D1"], "{first}");
    assert_eq!(
        first["changed"][0]["slack-message"]["done"],
        Value::Null,
        "not read yet"
    );
    let standing = client.heard();
    assert_eq!(
        standing["sources"],
        json!({"slack": "answering"}),
        "{standing}"
    );
    let done = Done {
        how: "readInSlack",
        at: "2024-10-31T18:40:00Z".to_owned(),
        millis: 1_730_400_000_000,
    };
    said.conversations.clear();
    said.messages = vec![message("D1", "1730300001.000100", Some(done))];
    said.marks[0].read = "1730300001.000100".to_owned();
    daemon.say(Some(&said), 20);
    // Read in Slack, the message and its mark change, and nothing goes.
    let read = client.heard();
    assert_eq!(keys(&read), [wanting, "slack-read:D1"], "{read}");
    assert_eq!(
        read["changed"][0]["slack-message"]["done"],
        json!({"how": "readInSlack", "at": "2024-10-31T18:40:00Z"}),
        "read in Slack"
    );
    assert_eq!(read.get("gone"), None, "nothing went: {read}");
}

#[test]
fn each_unread_each_read_last_and_the_account_is_a_record_in_slacks_order() {
    let said = ok(
        vec![
            one("D1", Kind::Dm, 2, "1730300000.000100"),
            one("C3", Kind::Unread, 0, "1730100000.000100"),
        ],
        vec![one("C1", Kind::Channel, 0, "1730700000.000100")],
    );
    let made: Vec<Value> = records(Some(&said)).into_iter().map(|(_, r)| r).collect();
    assert_eq!(
        made,
        [
            json!({"key": "slack-unread:D1", "kind": "slack-unread", "slack-unread": {
                "id": "D1", "name": "Ada", "kind": "dm", "count": 2,
                "link": "slack://channel?team=T1&id=D1", "latest": "1730300000.000100"}}),
            json!({"key": "slack-unread:C3", "kind": "slack-unread", "slack-unread": {
                "id": "C3", "name": "Ada", "kind": "unread", "count": 0,
                "link": "slack://channel?team=T1&id=C3", "latest": "1730100000.000100"}}),
            json!({"key": "slack-recent:C1", "kind": "slack-recent", "slack-recent": {
                "id": "C1", "name": "Ada", "kind": "channel",
                "link": "slack://channel?team=T1&id=C1", "read": "1730700000.000100"}}),
            json!({"key": "slack-account:slack", "kind": "slack-account",
                   "slack-account": {"standing": "ok"}}),
        ],
        "in Slack's order"
    );
}

#[test]
fn each_conversations_read_mark_is_a_record_while_slack_answers() {
    let mut said = ok(vec![], vec![]);
    said.seen = vec![Seen {
        id: "C5".to_owned(),
        name: "#aquifer".to_owned(),
        read: "1730300000.000100".to_owned(),
    }];
    let made: Vec<Value> = records(Some(&said)).into_iter().map(|(_, r)| r).collect();
    assert_eq!(
        made.first(),
        Some(
            &json!({"key": "slack-seen:C5", "kind": "slack-seen", "slack-seen": {
            "id": "C5", "name": "#aquifer", "read": "1730300000.000100"}})
        ),
        "a conversation's read mark is said by its name"
    );
    said.state = State::Unreachable;
    let keys: Vec<String> = records(Some(&said)).into_iter().map(|(k, _)| k).collect();
    assert_eq!(keys, [ACCOUNT], "no mark outlives Slack answering");
}

#[test]
fn only_the_account_is_held_while_slack_doesnt_answer_or_nobody_signed_in() {
    let mut said = ok(vec![one("D1", Kind::Dm, 2, "")], vec![]);
    said.state = State::Unreachable;
    let made = |made: Vec<(String, Value)>| {
        made.into_iter()
            .map(|(_, record)| record)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        made(records(Some(&said))),
        [json!({"key": ACCOUNT, "kind": "slack-account",
                "slack-account": {"standing": "unreachable"}})],
        "Slack out of reach"
    );
    assert_eq!(
        made(records(None)),
        [json!({"key": ACCOUNT, "kind": "slack-account",
                "slack-account": {"standing": "none"}})],
        "nobody signed in"
    );
}

#[test]
fn a_record_is_heard_when_it_changes_not_each_time_slack_is_asked() {
    let (daemon, _client, _folder) = served("heard");
    let first = ok(
        vec![one("D1", Kind::Dm, 1, "1"), one("D2", Kind::Dm, 1, "1")],
        vec![],
    );
    daemon.say(Some(&first), 10);
    let second = ok(
        vec![one("D1", Kind::Dm, 2, "2"), one("D2", Kind::Dm, 1, "1")],
        vec![],
    );
    daemon.say(Some(&second), 20);
    let heard = |key: &str| daemon.serving.held(key).and_then(|r| r["heard"].as_u64());
    assert_eq!(
        (
            heard("slack-unread:D1"),
            heard("slack-unread:D2"),
            heard(ACCOUNT)
        ),
        (Some(20), Some(10), Some(10)),
        "only the one that changed was heard again"
    );
}

#[test]
fn a_watch_of_messages_hears_only_how_slack_stands_as_that_changes() {
    let (daemon, mut client, _folder) = served("stands");
    // Asked before Slack is, as a shell starting the daemon by its
    // socket asks.
    let answer = client.ask(&json!({"id": "all", "ask": "watch", "kind": "slack-message"}));
    assert_eq!(answer, json!({"id": "all", "records": []}), "nothing held");
    let said = ok(vec![one("D1", Kind::Dm, 1, "1")], vec![]);
    daemon.say(Some(&said), 10);
    daemon.say(Some(&said), 20);
    daemon.say(None, 30);
    assert_eq!(
        (client.heard(), client.heard()),
        (
            json!({"id": "all", "sources": {"slack": "answering"}}),
            json!({"id": "all", "sources": {"slack": "signed-out"}})
        ),
        "told how Slack stands each time it moved, and nothing else"
    );
}

#[test]
fn health_says_why_slack_is_out_of_reach() {
    let (daemon, mut client, _folder) = served("reason");
    let mut said = ok(vec![], vec![]);
    said.state = State::Unreachable;
    daemon.say(Some(&said), 10);
    let health = client.ask(&json!({"id": 1, "ask": "health"}));
    assert_eq!(
        health["sources"]["slack"],
        json!({"state": "refused", "reason": "Slack couldn't be reached"}),
        "{health}"
    );
    let hello = client.ask(&json!({"id": 2, "ask": "hello"}));
    assert_eq!(
        (&hello["domain"], &hello["sources"]),
        (&json!("slack"), &json!({"slack": "refused"})),
        "{hello}"
    );
}

#[test]
fn rows_are_heard_by_a_watch_of_their_place_once_words_are_given() {
    let (daemon, mut client, _folder) = served("rows");
    let said = ok(
        vec![one("D1", Kind::Dm, 2, "1")],
        vec![one("C1", Kind::Channel, 0, "1")],
    );
    daemon.say(Some(&said), 10);
    let listed = client.ask(&json!({"id": 1, "ask": "list", "kind": "row"}));
    assert_eq!(listed["records"], json!([]), "rows made without words");
    daemon.words(Words {
        glyph: "chat".to_owned(),
        count: "{count} to read".to_owned(),
        ..Words::default()
    });
    let unread =
        client.ask(&json!({"id": 2, "ask": "watch", "kind": "row", "place": "slack-unread"}));
    let recent =
        client.ask(&json!({"id": 3, "ask": "watch", "kind": "row", "place": ["slack-recent"]}));
    assert_eq!(
        (&unread["records"], &recent["records"]),
        (&json!([]), &json!([])),
        "no rows before Slack answered again"
    );
    daemon.say(Some(&said), 20);
    let (first, second) = (client.heard(), client.heard());
    assert_eq!(
        [(&first["id"], keys(&first)), (&second["id"], keys(&second))],
        [
            (&json!(2), vec!["row:slack-unread:D1".to_owned()]),
            (&json!(3), vec!["row:slack-recent:C1".to_owned()])
        ],
        "each watch hears its place's row: {first} {second}"
    );
    assert_eq!(
        first["changed"][0]["row"]["side"],
        json!([{"said": "2 to read"}]),
        "said in the words given"
    );
    // Asked again with nothing new, Slack's answer tells no watch.
    daemon.say(Some(&said), 30);
    let next = told_nothing(&mut client);
    assert_eq!(
        next["id"], "nothing",
        "a row unchanged was said again: {next}"
    );
    let listed =
        client.ask(&json!({"id": 4, "ask": "list", "kind": "row", "place": "slack-recent"}));
    assert_eq!(
        keys(&json!({"changed": listed["records"]})),
        ["row:slack-recent:C1"],
        "a list of a place says only its rows"
    );
}

#[test]
fn a_conversation_coming_first_is_the_one_row_a_watch_hears() {
    let (daemon, mut client, _folder) = served("first");
    daemon.words(Words::default());
    let first = vec![one("D1", Kind::Dm, 1, "1"), one("C1", Kind::Unread, 1, "1")];
    daemon.say(Some(&ok(first.clone(), vec![])), 10);
    client.ask(&json!({"id": 1, "ask": "watch", "kind": "row", "place": "slack-unread"}));
    let mut now = first;
    now.insert(0, one("D2", Kind::Dm, 1, "2"));
    daemon.say(Some(&ok(now, vec![])), 20);
    let line = client.heard();
    let changed: Vec<(Value, Value)> = line["changed"]
        .as_array()
        .map(|records| {
            records
                .iter()
                .map(|record| (record["key"].clone(), record["row"]["order"].clone()))
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(
        changed,
        [(json!("row:slack-unread:D2"), json!(0))],
        "only the new row, first: {line}"
    );
    let next = told_nothing(&mut client);
    assert_eq!(
        next["id"], "nothing",
        "the rows below it were said again: {next}"
    );
}

#[test]
fn a_messages_drawer_is_heard_only_by_a_watch_of_its_key() {
    let (daemon, mut client, _folder) = served("drawer");
    daemon.words(Words::default());
    let mut said = ok(vec![], vec![]);
    said.messages = vec![message("D1", "1730300001.000100", None)];
    // Said through Slack out of reach too, as the message is.
    said.state = State::Unreachable;
    let drawer = "drawer:slack-messages:D1:1730300001.000100";
    client.ask(&json!({"id": "rows", "ask": "watch", "kind": "row"}));
    client.ask(&json!({"id": "drawer", "ask": "watch", "kind": "drawer", "keys": [drawer]}));
    daemon.say(Some(&said), 10);
    // Each watch hears what moved, then how Slack stands.
    let lines: Vec<Value> = (0..4)
        .map(|_| client.heard())
        .filter(|line| line.get("changed").is_some())
        .collect();
    let heard: Vec<(&Value, Vec<String>)> =
        lines.iter().map(|line| (&line["id"], keys(line))).collect();
    assert_eq!(
        heard,
        [
            (
                &json!("rows"),
                vec!["row:slack-messages:D1:1730300001.000100".to_owned()]
            ),
            (&json!("drawer"), vec![drawer.to_owned()])
        ],
        "{lines:?}"
    );
    assert_eq!(
        lines[1]["changed"][0]["drawer"]["body"], "hi",
        "the drawer says the message"
    );
}

#[test]
fn a_get_says_what_it_holds_by_key() {
    let (daemon, mut client, _folder) = served("get");
    daemon.say(Some(&ok(vec![one("D1", Kind::Dm, 1, "1")], vec![])), 10);
    let got =
        client.ask(&json!({"id": 1, "ask": "get", "keys": ["slack-unread:D1", "slack-unread:D9"]}));
    assert_eq!(
        keys(&json!({"changed": got["records"]})),
        ["slack-unread:D1"],
        "the one held, and none for one not: {got}"
    );
    let found = client.ask(&json!({"id": 2, "ask": "get", "keys": ["slack-message:D1:1.2"]}));
    assert_eq!(
        found["error"]["code"], "signed-out",
        "a message found is asked of Slack, which nobody signed in to: {found}"
    );
}
