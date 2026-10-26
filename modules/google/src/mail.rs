//! Joel's mail that wants him: unread, in the inbox, and marked important
//! by Gmail, newest first, as many as the shell would list.
//!
//! Gmail lists only ids, so each one is asked for once, for who sent it,
//! its subject and when it came; what was asked is kept between looks
//! (watch.rs), so a quiet inbox costs one list a minute.

use serde::Serialize;
use serde_json::Value;

/// What mail wants Joel, as Gmail's search writes it.
pub const WANTS: &str = "in:inbox is:unread is:important";

/// How many the shell is told of, at most.
pub const SHOWN: usize = 20;

/// One piece of mail, as the shell is told it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Mail {
    /// The message's own id, which reading or archiving it names.
    pub id: String,
    /// Who sent it, by name when the sender gives one.
    pub from: String,
    pub subject: String,
    /// When it came, in milliseconds since the epoch.
    pub at: i64,
    /// Its conversation in Gmail.
    pub page: String,
}

/// The ids of the mail a search answered with, in its order.
pub fn listed(answer: &Value) -> Vec<String> {
    answer
        .get("messages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|message| message.get("id").and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}

/// One piece of mail, from Gmail's answer with its metadata, or none when
/// the answer isn't one.
pub fn read(answer: &Value) -> Option<Mail> {
    let id = answer.get("id").and_then(Value::as_str)?;
    let thread = answer.get("threadId").and_then(Value::as_str)?;
    let header = |name: &str| {
        answer
            .pointer("/payload/headers")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .find(|header| {
                header
                    .get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|named| named.eq_ignore_ascii_case(name))
            })
            .and_then(|header| header.get("value").and_then(Value::as_str))
            .unwrap_or("")
    };
    Some(Mail {
        id: id.to_owned(),
        from: sender(header("From")),
        subject: header("Subject").trim().to_owned(),
        // Gmail writes when it came as a string of milliseconds.
        at: answer
            .get("internalDate")
            .and_then(Value::as_str)
            .and_then(|at| at.parse().ok())
            .unwrap_or(0),
        page: format!("https://mail.google.com/mail/u/0/#inbox/{thread}"),
    })
}

/// Who a From header names: the name before the address, unquoted, or the
/// address alone when there's no name.
pub fn sender(from: &str) -> String {
    let from = from.trim();
    let Some((name, address)) = from.rsplit_once('<') else {
        return from.to_owned();
    };
    let name = name.trim().trim_matches('"').trim();
    if name.is_empty() {
        address.trim_end_matches('>').trim().to_owned()
    } else {
        name.to_owned()
    }
}

/// How many in the inbox are unread, from Gmail's answer about the inbox.
pub fn unread(answer: &Value) -> Option<u64> {
    answer.get("messagesUnread").and_then(Value::as_u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_search_lists_its_ids_in_order() {
        let answer = json!({"messages": [{"id": "b", "threadId": "t"}, {"id": "a", "threadId": "t"}],
            "resultSizeEstimate": 2});
        assert_eq!(listed(&answer), ["b", "a"]);
        assert!(listed(&json!({"resultSizeEstimate": 0})).is_empty());
    }

    #[test]
    fn mail_says_who_sent_it_its_subject_when_and_its_page() {
        let answer = json!({"id": "m1", "threadId": "t1", "internalDate": "1793044800000",
        "payload": {"headers": [
            {"name": "Subject", "value": " Lunch? "},
            {"name": "from", "value": "\"Lovelace, Ada\" <ada@example.com>"}
        ]}});
        assert_eq!(
            read(&answer),
            Some(Mail {
                id: "m1".into(),
                from: "Lovelace, Ada".into(),
                subject: "Lunch?".into(),
                at: 1_793_044_800_000,
                page: "https://mail.google.com/mail/u/0/#inbox/t1".into(),
            })
        );
    }

    #[test]
    fn mail_without_headers_is_still_mail() {
        let found = read(&json!({"id": "m", "threadId": "t"})).unwrap();
        assert_eq!((found.from.as_str(), found.subject.as_str()), ("", ""));
        assert_eq!(read(&json!({"error": {"code": 404}})), None);
    }

    #[test]
    fn a_sender_is_named_by_name_or_else_address() {
        assert_eq!(sender("Ada Lovelace <ada@example.com>"), "Ada Lovelace");
        assert_eq!(sender("<ada@example.com>"), "ada@example.com");
        assert_eq!(sender("ada@example.com"), "ada@example.com");
    }

    #[test]
    fn the_inbox_says_how_many_are_unread() {
        assert_eq!(
            unread(&json!({"id": "INBOX", "messagesUnread": 12})),
            Some(12)
        );
        assert_eq!(unread(&json!({})), None);
    }
}
