//! Slack's messages' rows, ready for the launcher: each message wanting
//! Joel, done ones too, as the row For You's Slack row and its History
//! show it (SlackMessage.qml), worded, newest first and with its acts.
//! A message has no title, so its row is named by who said it, its first
//! words after the name and when it came beside it. Its drawer, the
//! whole message with it, is a record of its own, which the row names
//! and the shell watches by key only while it shows, so a message's text
//! never rides in its row, as Google's daemon keeps Mail's
//! (QEST-20261107-RTOIMQ attachments/google-rows.md). Its times travel as
//! `{at, as}` for the shell to word against its own clock, so no row
//! changes each minute. SlackMessage.js and SlackMessage.qml make the
//! same row today; these take their place as the shell reads them.

use serde_json::{Value, json};

use crate::messages::Raised;

/// The place of each message wanting Joel, newest first.
pub const MESSAGES: &str = "slack-messages";

/// How far apart rows' orders are, as rows.rs spaces them.
const SPACED: u64 = 1024;

/// The words messages' rows are said in, from the launcher's vocabulary.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Words {
    /// The glyph role every message's row shows.
    pub glyph: String,
    /// Joel, as a message of his own is named: "You".
    pub you: String,
    /// "in {place} · message", the drawer's eyebrow in a channel.
    pub place: String,
    /// "direct message", the drawer's eyebrow in one.
    pub direct: String,
    /// "Sent", the drawer's fact of when.
    pub sent: String,
}

impl Words {
    /// A message's words from the whole vocabulary, or the first one it
    /// lacks.
    ///
    /// # Errors
    ///
    /// The first word the vocabulary's `slack:message` or
    /// `slack:message:sent` lacks.
    pub fn read(vocabulary: &Value) -> Result<Words, String> {
        let you = word(vocabulary, "slack:message", &["said", "you"])?;
        Ok(Words {
            glyph: word(vocabulary, "slack:message", &["glyph"])?,
            you: capitalised(&you),
            place: word(vocabulary, "slack:message", &["said", "where"])?,
            direct: word(vocabulary, "slack:message", &["said", "direct"])?,
            sent: word(vocabulary, "slack:message:sent", &["name"])?,
        })
    }
}

/// An act the shell does itself, opening `link` in what opens it, as
/// ready-rows.md's acts name it: one of `{ key, ask }` sent back on the
/// socket, `{ key, open }` or `{ key, copy }` (Y2YBTS, 2026-11-10).
pub(crate) fn open(key: &str, link: &str) -> Value {
    json!({"key": key, "open": link})
}

/// An act the shell does itself, copying `text` to the clipboard.
pub(crate) fn copy(key: &str, text: &str) -> Value {
    json!({"key": key, "copy": text})
}

/// The word at `at` in the vocabulary's thing `id`, or what's missing:
/// the thing, or the word in it.
pub(crate) fn word(vocabulary: &Value, id: &str, at: &[&str]) -> Result<String, String> {
    let thing = vocabulary["things"]
        .as_array()
        .and_then(|things| things.iter().find(|thing| thing["id"] == id))
        .ok_or_else(|| format!("the vocabulary has no {id}"))?;
    at.iter()
        .fold(thing, |within, field| &within[field])
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("the vocabulary's {id} has no {}", at.join(".")))
}

/// A word as it opens a row's name or a fact, as SlackMessage.qml's `you`
/// and Slack.js's presence say it.
pub(crate) fn capitalised(word: &str) -> String {
    let mut letters = word.chars();
    letters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(letters).collect()
    })
}

/// Each message's row and its drawer, as records keyed as the launcher
/// watches them, newest first, by Slack's time where two came at once.
#[must_use]
pub fn records(messages: &[Raised], words: &Words) -> Vec<(String, Value)> {
    let mut newest: Vec<&Raised> = messages.iter().collect();
    newest.sort_by(|one, other| other.at.cmp(&one.at).then_with(|| other.ts.cmp(&one.ts)));
    let mut made = Vec::with_capacity(newest.len() * 2);
    for (one, at) in newest.into_iter().zip(1..) {
        let id = format!("{}:{}", one.channel, one.ts);
        let drawer = format!("drawer:{MESSAGES}:{id}");
        let key = format!("row:{MESSAGES}:{id}");
        let row = row(one, &id, at * SPACED, &drawer, words);
        made.push((key.clone(), json!({"key": key, "kind": "row", "row": row})));
        let said = json!({"key": drawer, "kind": "drawer", "drawer": drawn(one, words)});
        made.push((drawer, said));
    }
    made
}

/// Whether it's in a direct message, whose conversation Slack names with
/// a D.
fn direct(one: &Raised) -> bool {
    one.channel.starts_with('D')
}

/// Who said it, Joel as `you`.
fn who(one: &Raised, words: &Words) -> String {
    if one.mine {
        words.you.clone()
    } else {
        one.who.clone()
    }
}

/// One message's row: Return opens it where Slack shows it, else its
/// conversation in Slack's app; inside are Open in Slack, then Copy the
/// Link where Slack has said where it is.
fn row(one: &Raised, id: &str, order: u64, drawer: &str, words: &Words) -> Value {
    let opens = if one.link.is_empty() {
        format!("slack://channel?team={}&id={}", one.team, one.channel)
    } else {
        one.link.clone()
    };
    let mut acts = vec![open("verb:slack:message:open", &opens)];
    if !one.link.is_empty() {
        acts.push(copy("verb:slack:message:link", &one.link));
    }
    json!({
        "key": format!("row:{MESSAGES}:{id}"),
        "row": format!("slack:message:{id}"),
        "place": MESSAGES,
        "kind": "message",
        "order": order,
        "name": who(one, words),
        "glyph": words.glyph,
        "hue": "program",
        "depth": 0,
        "side": [{"at": one.at, "as": "came"}],
        "note": one.first,
        "drawer": drawer,
        "acts": acts,
        "held": {
            "channel": one.channel, "ts": one.ts, "place": one.place, "mine": one.mine,
            "at": one.at, "done": one.done,
        },
    })
}

/// Its drawer: where it is in the eyebrow, the conversation named where
/// it's a channel, who said it, when as a fact, and it all as prose.
fn drawn(one: &Raised, words: &Words) -> Value {
    let eyebrow = if direct(one) {
        words.direct.clone()
    } else {
        words.place.replace("{place}", &one.place)
    };
    json!({
        "eyebrow": eyebrow,
        "eyebrowNames": !direct(one),
        "title": who(one, words),
        "facts": [{"name": words.sent, "at": one.at, "as": "moment"}],
        "body": one.text,
        "prose": true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::Done;

    fn words() -> Words {
        Words {
            glyph: "chat".to_owned(),
            you: "You".to_owned(),
            place: "in {place} \u{b7} message".to_owned(),
            direct: "direct message".to_owned(),
            sent: "Sent".to_owned(),
        }
    }

    fn message(channel: &str, ts: &str, at: u64) -> Raised {
        Raised {
            key: format!("slack-message:{channel}:{ts}"),
            channel: channel.to_owned(),
            ts: ts.to_owned(),
            place: "#deploys".to_owned(),
            who: "Ada Lovelace".to_owned(),
            user: "U7".to_owned(),
            named: true,
            mine: false,
            first: "the deploy is out".to_owned(),
            text: "the deploy is out\nand green".to_owned(),
            at,
            link: format!("https://ada.slack.com/archives/{channel}/p{ts}"),
            team: "T1".to_owned(),
            done: None,
        }
    }

    #[test]
    fn each_message_is_a_row_newest_first_and_its_drawer_a_record_of_its_own() {
        let older = message("C1", "1.000100", 1_000);
        let newer = message("D2", "2.000100", 2_000);
        let made = records(&[older, newer], &words());
        let keys: Vec<&str> = made.iter().map(|(key, _)| key.as_str()).collect();
        assert_eq!(
            keys,
            [
                "row:slack-messages:D2:2.000100",
                "drawer:slack-messages:D2:2.000100",
                "row:slack-messages:C1:1.000100",
                "drawer:slack-messages:C1:1.000100",
            ]
        );
        let row = &made[2].1["row"];
        assert_eq!(
            (
                &row["row"],
                &row["order"],
                &row["name"],
                &row["note"],
                &row["side"]
            ),
            (
                &json!("slack:message:C1:1.000100"),
                &json!(2048),
                &json!("Ada Lovelace"),
                &json!("the deploy is out"),
                &json!([{"at": 1_000, "as": "came"}])
            )
        );
        assert_eq!(row["drawer"], "drawer:slack-messages:C1:1.000100");
        assert!(
            row.get("body").is_none(),
            "the message's text rode in its row"
        );
        let drawer = &made[3].1["drawer"];
        assert_eq!(
            (&drawer["eyebrow"], &drawer["eyebrowNames"], &drawer["body"]),
            (
                &json!("in #deploys \u{b7} message"),
                &json!(true),
                &json!("the deploy is out\nand green")
            )
        );
        // A time fact is named as ready-rows.md names every drawer fact.
        assert_eq!(
            drawer["facts"],
            json!([{"name": "Sent", "at": 1_000, "as": "moment"}])
        );
        assert_eq!(made[1].1["drawer"]["eyebrow"], "direct message");
        assert_eq!(made[1].1["drawer"]["eyebrowNames"], false);
    }

    #[test]
    fn its_acts_open_it_where_slack_shows_it_and_copy_that_link() {
        let made = records(&[message("C1", "1.000100", 1_000)], &words());
        assert_eq!(
            made[0].1["row"]["acts"],
            json!([
                {"key": "verb:slack:message:open",
                 "open": "https://ada.slack.com/archives/C1/p1.000100"},
                {"key": "verb:slack:message:link",
                 "copy": "https://ada.slack.com/archives/C1/p1.000100"},
            ])
        );
    }

    #[test]
    fn without_a_link_it_opens_its_conversation_in_slack_and_copies_nothing() {
        let mut one = message("C1", "1.000100", 1_000);
        one.link = String::new();
        let made = records(&[one], &words());
        assert_eq!(
            made[0].1["row"]["acts"],
            json!([{"key": "verb:slack:message:open", "open": "slack://channel?team=T1&id=C1"}])
        );
    }

    #[test]
    fn joels_own_is_named_you_and_a_done_one_says_how_in_what_it_holds() {
        let mut one = message("C1", "1.000100", 1_000);
        one.mine = true;
        one.done = Some(Done {
            how: "readInSlack",
            at: "2024-10-31T18:40:00Z".to_owned(),
            millis: 1,
        });
        let made = records(&[one], &words());
        assert_eq!(made[0].1["row"]["name"], "You");
        assert_eq!(made[1].1["drawer"]["title"], "You");
        assert_eq!(
            made[0].1["row"]["held"]["done"],
            json!({"how": "readInSlack", "at": "2024-10-31T18:40:00Z"})
        );
    }

    #[test]
    fn the_words_are_read_from_the_vocabulary_and_one_missing_is_named() {
        let mut vocabulary = json!({"things": [
            {"id": "slack:message", "glyph": "chat",
             "said": {"you": "you", "where": "in {place}", "direct": "direct message"}},
            {"id": "slack:message:sent", "name": "Sent"},
        ]});
        let read = Words::read(&vocabulary).unwrap();
        assert_eq!((read.you.as_str(), read.sent.as_str()), ("You", "Sent"));
        vocabulary["things"][1]
            .as_object_mut()
            .unwrap()
            .remove("name");
        assert_eq!(
            Words::read(&vocabulary),
            Err("the vocabulary's slack:message:sent has no name".to_owned())
        );
    }
}
