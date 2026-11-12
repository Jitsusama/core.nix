//! What each saying of Slack's makes. How a watch is told it, only what
//! changed and what went, is daemon-serve's, and its own cases prove it.

use super::*;
use crate::messages::{Done, Mark, Raised};
use crate::unreads::{Kind, Seen};

/// The one account's key.
const ACCOUNT: &str = "slack-account:slack";

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

/// Each record a saying made, by its key.
fn by_key(made: Vec<Value>) -> BTreeMap<String, Value> {
    made.into_iter()
        .map(|record| {
            (
                record["key"].as_str().unwrap_or_default().to_owned(),
                record,
            )
        })
        .collect()
}

/// The keys whose records one saying changed or made from the one
/// before, as a watch of every key would be told them.
fn changed(was: &BTreeMap<String, Value>, now: &BTreeMap<String, Value>) -> Vec<String> {
    now.iter()
        .filter(|(key, record)| was.get(*key) != Some(record))
        .map(|(key, _)| key.clone())
        .collect()
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
        "only the message and the account while Slack is out of reach"
    );
}

#[test]
fn the_account_says_the_messages_are_whole_only_once_they_are() {
    let mut said = ok(vec![], vec![]);
    let account = |said: Option<&Said>| records(said).pop().unwrap().1["slack-account"].clone();
    assert_eq!(
        account(Some(&said)),
        json!({"standing": "ok"}),
        "said whole before they were"
    );
    said.whole = true;
    assert_eq!(
        account(Some(&said)),
        json!({"standing": "ok", "messages": "whole"}),
        "not said whole once they were"
    );
    assert_eq!(
        account(None),
        json!({"standing": "none"}),
        "no sign-in not said"
    );
}

#[test]
fn a_message_wanting_joel_is_a_record_and_its_being_read_a_change_to_it() {
    let mut held = Held::default();
    let mut said = ok(vec![one("D1", Kind::Dm, 1, "1730300001.000100")], vec![]);
    said.messages = vec![message("D1", "1730300001.000100", None)];
    said.marks = vec![Mark {
        channel: "D1".to_owned(),
        read: "1730300000.000100".to_owned(),
        at: "2024-10-30T14:53:20Z".to_owned(),
    }];
    let first = by_key(held.heard(Some(&said), 10));
    let message = "slack-message:D1:1730300001.000100";
    assert_eq!(
        first[message]["slack-message"]["done"],
        Value::Null,
        "a message wanting him said done"
    );
    let done = Done {
        how: "readInSlack",
        at: "2024-10-31T18:40:00Z".to_owned(),
        millis: 1_730_400_000_000,
    };
    said.conversations.clear();
    said.messages = vec![message("D1", "1730300001.000100", Some(done))];
    said.marks[0].read = "1730300001.000100".to_owned();
    let second = by_key(held.heard(Some(&said), 20));
    // Read in Slack, the message and its mark change, and neither goes.
    assert_eq!(
        changed(&first, &second),
        [message, "slack-read:D1"],
        "what reading it in Slack changed"
    );
    assert!(second.contains_key(message), "a message read in Slack went");
    assert_eq!(
        second[message]["slack-message"]["done"],
        json!({"how": "readInSlack", "at": "2024-10-31T18:40:00Z"}),
        "read in Slack, not said done"
    );
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
        "the records Slack's answer makes"
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
        made[0],
        json!({"key": "slack-seen:C5", "kind": "slack-seen", "slack-seen": {
            "id": "C5", "name": "#aquifer", "read": "1730300000.000100"}}),
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
    let keys = |made: Vec<(String, Value)>| {
        made.into_iter()
            .map(|(_, record)| record)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        keys(records(Some(&said))),
        [json!({"key": ACCOUNT, "kind": "slack-account",
                "slack-account": {"standing": "unreachable"}})],
        "more than the account while Slack is out of reach"
    );
    assert_eq!(
        keys(records(None)),
        [json!({"key": ACCOUNT, "kind": "slack-account",
                "slack-account": {"standing": "none"}})],
        "more than the account with nobody signed in"
    );
}

#[test]
fn a_record_is_heard_when_it_changes_not_each_time_slack_is_asked() {
    let mut held = Held::default();
    let first = ok(
        vec![one("D1", Kind::Dm, 1, "1"), one("D2", Kind::Dm, 1, "1")],
        vec![],
    );
    held.heard(Some(&first), 10);
    let second = ok(
        vec![one("D1", Kind::Dm, 2, "2"), one("D2", Kind::Dm, 1, "1")],
        vec![],
    );
    let made = by_key(held.heard(Some(&second), 20));
    let heard = |key: &str| made[key]["heard"].as_u64();
    assert_eq!(
        (
            heard("slack-unread:D1"),
            heard("slack-unread:D2"),
            heard(ACCOUNT)
        ),
        (Some(20), Some(10), Some(10)),
        "when each was heard"
    );
}

#[test]
fn how_slack_stands_is_its_word_and_why() {
    let said = ok(vec![], vec![]);
    let stands = |said: Option<&Said>| Stands::of(said).sources();
    assert_eq!(
        stands(Some(&said)),
        json!({"slack": {"state": "answering", "reason": ""}}),
        "Slack answering"
    );
    assert_eq!(
        stands(None),
        json!({"slack": {"state": "signed-out", "reason": ""}}),
        "nobody signed in"
    );
    let out = Said {
        state: State::Unreachable,
        ..said
    };
    assert_eq!(
        stands(Some(&out)),
        json!({"slack": {"state": "refused", "reason": "Slack couldn't be reached"}}),
        "Slack out of reach"
    );
}

#[test]
fn rows_are_made_by_their_place_once_words_are_given() {
    let mut held = Held::default();
    let said = ok(
        vec![one("D1", Kind::Dm, 2, "1")],
        vec![one("C1", Kind::Channel, 0, "1")],
    );
    let rows = |made: &BTreeMap<String, Value>| -> Vec<String> {
        made.keys()
            .filter(|key| key.starts_with("row:"))
            .cloned()
            .collect()
    };
    let without = by_key(held.heard(Some(&said), 10));
    assert!(rows(&without).is_empty(), "rows made without words");
    held.words = Some(Words {
        glyph: "chat".to_owned(),
        count: "{count} to read".to_owned(),
        ..Words::default()
    });
    let with = by_key(held.heard(Some(&said), 20));
    assert_eq!(
        rows(&with),
        ["row:slack-recent:C1", "row:slack-unread:D1"],
        "each conversation's row, keyed by its place"
    );
    assert_eq!(
        with["row:slack-unread:D1"]["row"]["side"],
        json!([{"said": "2 to read"}]),
        "an unread row's count"
    );
    let again = by_key(held.heard(Some(&said), 30));
    assert_eq!(
        changed(&with, &again),
        Vec::<String>::new(),
        "a row unchanged was said again"
    );
}

#[test]
fn a_conversation_coming_first_is_the_one_row_changed() {
    let mut held = Held {
        words: Some(Words::default()),
        ..Held::default()
    };
    let first = vec![one("D1", Kind::Dm, 1, "1"), one("C1", Kind::Unread, 1, "1")];
    let was = by_key(held.heard(Some(&ok(first.clone(), vec![])), 10));
    let mut now = first;
    now.insert(0, one("D2", Kind::Dm, 1, "2"));
    let made = by_key(held.heard(Some(&ok(now, vec![])), 20));
    let rows: Vec<(String, Value)> = changed(&was, &made)
        .into_iter()
        .filter(|key| key.starts_with("row:"))
        .map(|key| (key.clone(), made[&key]["row"]["order"].clone()))
        .collect();
    assert_eq!(
        rows,
        [("row:slack-unread:D2".to_owned(), json!(0))],
        "the rows below it were said again"
    );
}

#[test]
fn a_messages_row_and_its_drawer_are_said_while_slack_is_out_of_reach() {
    let mut held = Held {
        words: Some(Words::default()),
        ..Held::default()
    };
    let mut said = ok(vec![], vec![]);
    said.messages = vec![message("D1", "1730300001.000100", None)];
    // Said through Slack out of reach too, as the message is.
    said.state = State::Unreachable;
    let made = by_key(held.heard(Some(&said), 10));
    let shown: Vec<&String> = made
        .keys()
        .filter(|key| key.starts_with("row:") || key.starts_with("drawer:"))
        .collect();
    assert_eq!(
        shown,
        [
            "drawer:slack-messages:D1:1730300001.000100",
            "row:slack-messages:D1:1730300001.000100"
        ],
        "a message's row and drawer"
    );
    assert_eq!(
        made["drawer:slack-messages:D1:1730300001.000100"]["drawer"]["body"], "hi",
        "the drawer says the message"
    );
}
