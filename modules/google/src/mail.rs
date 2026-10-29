//! Joel's mail that wants him: unread, in the inbox, and marked important
//! by Gmail, newest first, as many as the shell would list.
//!
//! Gmail lists only ids, so each one is asked for once, for who sent it,
//! who it went to, its subject, when it came and its text; what was asked
//! is kept between looks (watch.rs), so a quiet inbox costs one list a
//! minute.

use serde::Serialize;
use serde_json::Value;

use crate::calendar;

/// What mail wants Joel, as Gmail's search writes it.
pub const WANTS: &str = "in:inbox is:unread is:important";

/// How many the shell is told of, at most.
pub const SHOWN: usize = 20;

/// How much of a body is kept: enough to read in the launcher's drawer,
/// never a whole thread quoted below it.
pub const BODY_MOST: usize = 4000;

/// What Joel does with a piece of mail that wants him, each taking one of
/// Gmail's labels off it, after which it wants him no more.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tidy {
    /// Mark it read: it stays in the inbox.
    Read,
    /// Take it out of the inbox, read or not, as Gmail's Archive does.
    Archive,
}

impl Tidy {
    /// The label Gmail takes off.
    pub fn label(self) -> &'static str {
        match self {
            Tidy::Read => "UNREAD",
            Tidy::Archive => "INBOX",
        }
    }
}

/// One piece of mail, as the shell is told it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Mail {
    /// The message's own id, which reading or archiving it names.
    pub id: String,
    /// Who sent it, by name when the sender gives one.
    pub from: String,
    /// The address it came from, which a reply goes to.
    pub address: String,
    /// Who it went to, each by name when the header gives one.
    pub to: String,
    pub subject: String,
    /// When it came, in milliseconds since the epoch.
    pub at: i64,
    /// Its conversation in Gmail.
    pub page: String,
    /// What it says, as plain text: its plain part, or its HTML part as
    /// text when it has no plain one, at most `BODY_MOST` characters.
    pub body: String,
    /// The name of each file it carries, in order, as Gmail lists them
    /// under it.
    pub attachments: Vec<String>,
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

/// One piece of mail, from Gmail's answer with the whole message, or none
/// when the answer isn't one.
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
        address: address(header("From")),
        to: split(header("To"))
            .iter()
            .map(|one| sender(one))
            .collect::<Vec<_>>()
            .join(", "),
        subject: header("Subject").trim().to_owned(),
        // Gmail writes when it came as a string of milliseconds.
        at: answer
            .get("internalDate")
            .and_then(Value::as_str)
            .and_then(|at| at.parse().ok())
            .unwrap_or(0),
        page: format!("https://mail.google.com/mail/u/0/#inbox/{thread}"),
        body: answer.get("payload").map(body).unwrap_or_default(),
        attachments: answer.get("payload").map(attached).unwrap_or_default(),
    })
}

/// The name of each file a message carries: every part with a filename,
/// found depth first, but for what its HTML shows inline (a signature's
/// logo, a pasted picture), which lives in a multipart/related beside the
/// HTML and which Gmail doesn't list as an attachment either.
fn attached(payload: &Value) -> Vec<String> {
    let mut names = Vec::new();
    carried(payload, &mut names);
    names
}

fn carried(part: &Value, into: &mut Vec<String>) {
    let text = |field: &str| part.get(field).and_then(Value::as_str).unwrap_or("");
    let name = text("filename").trim();
    if !name.is_empty() {
        into.push(name.to_owned());
        return;
    }
    if text("mimeType").eq_ignore_ascii_case("multipart/related") {
        // Its first part is what it relates the rest to, which may itself
        // hold attachments, as an alternative's parts never do.
        if let Some(first) = part.pointer("/parts/0") {
            carried(first, into);
        }
        return;
    }
    for inner in part
        .get("parts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        carried(inner, into);
    }
}

/// The address a From header names, or nothing when it names none.
pub fn address(from: &str) -> String {
    let from = from.trim();
    match from.rsplit_once('<') {
        Some((_, address)) => address.trim_end_matches('>').trim().to_owned(),
        None if from.contains('@') => from.to_owned(),
        None => String::new(),
    }
}

/// The people a header lists, split at its commas but not those inside a
/// quoted name, as "Lovelace, Ada" is.
fn split(header: &str) -> Vec<String> {
    let mut people = Vec::new();
    let (mut one, mut quoted) = (String::new(), false);
    for c in header.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                one.push(c);
            }
            ',' if !quoted => people.push(std::mem::take(&mut one)),
            _ => one.push(c),
        }
    }
    people.push(one);
    people
        .into_iter()
        .map(|one| one.trim().to_owned())
        .filter(|one| !one.is_empty())
        .collect()
}

/// A message's text: its first plain part, found depth first, or else its
/// first HTML part as text; never an attachment's.
fn body(payload: &Value) -> String {
    let mut parts = Vec::new();
    gather(payload, &mut parts);
    let of = |kind: &str| {
        parts
            .iter()
            .find(|(mime, _)| mime.eq_ignore_ascii_case(kind))
            .map(|(_, text)| text.as_str())
    };
    if let Some(text) = of("text/plain") {
        return calendar::clipped(folded(text), BODY_MOST);
    }
    of("text/html")
        .map(calendar::plain)
        .map_or_else(String::new, |text| calendar::clipped(text, BODY_MOST))
}

/// Each part that holds text of its own, in order, with its kind.
fn gather(part: &Value, into: &mut Vec<(String, String)>) {
    let text = |field: &str| part.get(field).and_then(Value::as_str).unwrap_or("");
    if !text("filename").is_empty() {
        return;
    }
    if let Some(data) = part.pointer("/body/data").and_then(Value::as_str) {
        into.push((text("mimeType").to_owned(), decoded(data)));
    }
    for inner in part
        .get("parts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        gather(inner, into);
    }
}

/// Plain text as it reads: each line's trailing space gone, and blank
/// lines run to one.
fn folded(text: &str) -> String {
    let mut kept = String::with_capacity(text.len());
    let mut gap = false;
    for line in text.lines().map(str::trim_end) {
        if line.is_empty() {
            gap = !kept.is_empty();
            continue;
        }
        if !kept.is_empty() {
            kept.push_str(if gap { "\n\n" } else { "\n" });
        }
        kept.push_str(line);
        gap = false;
    }
    kept
}

/// What Gmail's URL-safe base64 holds, as text, with anything not UTF-8
/// replaced, and the padding Gmail sometimes leaves off not needed.
fn decoded(data: &str) -> String {
    let value = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'-' | b'+' => Some(62),
        b'_' | b'/' => Some(63),
        _ => None,
    };
    let mut bytes = Vec::with_capacity(data.len() / 4 * 3);
    let (mut held, mut bits) = (0u32, 0u32);
    for six in data.bytes().filter_map(value) {
        held = (held << 6) | u32::from(six);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            bytes.push(u8::try_from((held >> bits) & 0xff).unwrap_or(0));
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
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
                address: "ada@example.com".into(),
                to: String::new(),
                subject: "Lunch?".into(),
                at: 1_793_044_800_000,
                page: "https://mail.google.com/mail/u/0/#inbox/t1".into(),
                body: String::new(),
                attachments: Vec::new(),
            })
        );
    }

    /// Gmail's URL-safe base64 of some text, unpadded as it often is.
    fn encoded(text: &str) -> String {
        const LETTERS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut out = String::new();
        for chunk in text.as_bytes().chunks(3) {
            let n = chunk
                .iter()
                .enumerate()
                .fold(0u32, |n, (at, byte)| n | u32::from(*byte) << (16 - 8 * at));
            for at in 0..=chunk.len() {
                out.push(char::from(LETTERS[(n >> (18 - 6 * at) & 63) as usize]));
            }
        }
        out
    }

    #[test]
    fn mail_says_who_it_went_to_each_by_name() {
        let answer = json!({"id": "m", "threadId": "t", "payload": {"headers": [
            {"name": "To", "value": "\"Gerber, Joel\" <joel@example.com>, grace@example.com"}
        ]}});
        assert_eq!(read(&answer).unwrap().to, "Gerber, Joel, grace@example.com");
    }

    #[test]
    fn mail_reads_as_its_plain_part_before_its_html_and_never_an_attachment() {
        let part = |mime: &str, text: &str| {
            json!({"mimeType": mime,
            "body": {"data": encoded(text)}})
        };
        let answer = json!({"id": "m", "threadId": "t", "payload": {
        "mimeType": "multipart/mixed", "parts": [
            {"mimeType": "text/plain", "filename": "notes.txt",
                "body": {"data": encoded("an attachment")}},
            {"mimeType": "multipart/alternative", "parts": [
                part("text/html", "<p>Lunch?</p>"),
                part("text/plain", "Lunch on Friday?  \r\n\r\n\r\nAda <ada@example.com>\r\n")
            ]}
        ]}});
        assert_eq!(
            read(&answer).unwrap().body,
            "Lunch on Friday?\n\nAda <ada@example.com>"
        );
    }

    #[test]
    fn mail_lists_the_files_it_carries_but_not_what_its_html_shows_inline() {
        let answer = json!({"id": "m", "threadId": "t", "payload": {
        "mimeType": "multipart/mixed", "parts": [
            {"mimeType": "multipart/related", "parts": [
                {"mimeType": "multipart/alternative", "parts": [
                    {"mimeType": "text/plain", "body": {"data": encoded("Menu attached")}},
                    {"mimeType": "text/html", "body": {"data": encoded("<p>Menu</p>")}}
                ]},
                {"mimeType": "image/png", "filename": "logo.png", "body": {}}
            ]},
            {"mimeType": "application/pdf", "filename": "menu.pdf", "body": {}},
            {"mimeType": "image/jpeg", "filename": " patio.jpg ", "body": {}}
        ]}});
        let mail = read(&answer).unwrap();
        assert_eq!(mail.attachments, ["menu.pdf", "patio.jpg"]);
        assert_eq!(mail.body, "Menu attached");
    }

    #[test]
    fn mail_with_no_files_carries_none() {
        let answer = json!({"id": "m", "threadId": "t", "payload": {
            "mimeType": "text/plain", "body": {"data": encoded("Hi")}}});
        assert!(read(&answer).unwrap().attachments.is_empty());
    }

    #[test]
    fn mail_with_only_html_reads_as_its_text() {
        let answer = json!({"id": "m", "threadId": "t", "payload": {
            "mimeType": "text/html",
            "body": {"data": encoded("<p>Caf\u{e9} at <b>noon</b>?</p>")}}});
        assert_eq!(read(&answer).unwrap().body, "Caf\u{e9} at noon?");
    }

    #[test]
    fn a_long_body_is_cut_at_a_word() {
        let long = "word ".repeat(BODY_MOST);
        let answer = json!({"id": "m", "threadId": "t", "payload": {
            "mimeType": "text/plain", "body": {"data": encoded(&long)}}});
        let body = read(&answer).unwrap().body;
        assert!(body.chars().count() <= BODY_MOST);
        assert!(body.ends_with("word"), "{}", &body[body.len() - 10..]);
    }

    #[test]
    fn a_from_header_gives_its_address() {
        assert_eq!(address("Ada Lovelace <ada@example.com>"), "ada@example.com");
        assert_eq!(address("ada@example.com"), "ada@example.com");
        assert_eq!(address("Ada"), "");
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
