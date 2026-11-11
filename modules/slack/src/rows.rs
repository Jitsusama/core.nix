//! Slack's rows, ready for the launcher: each conversation unread and
//! each Joel read last as the row Slack's Unread and Recent show it,
//! worded, ordered and with its acts, so the shell lays it out and words
//! nothing (Joel, 2026-11-10: "the daemons need to be the data layer, and
//! they need to serve the launcher"). The shape is Y2YBTS's ready-row
//! record (QEST-20261023-Y2YBTS attachments/speed/ready-rows.md), as
//! Google's daemon serves Mail's: a `row:<place>:<id>` record, the place
//! `slack-unread` or `slack-recent`, whose `row` is the launcher's own key
//! for the conversation, `slack:conversation:<id>`, so one in both places
//! is one thing. SlackUnreads.js and SlackProgram.qml make the same rows
//! today; these take their place as the shell reads them. Each message
//! wanting Joel is a row too, of `slack-messages` (`message_rows.rs`).

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::message_rows;
use crate::unreads::{Conversation, Kind, Said, State};

/// The place of each conversation unread, in Slack's order: direct
/// messages, then channels mentioning Joel, then threads, then channels
/// merely unread (unreads.rs's `counted`).
pub const UNREAD: &str = "slack-unread";

/// The place of each conversation Joel read last, newest first.
pub const RECENT: &str = "slack-recent";

/// How far apart rows' orders are, so a row placed between two later
/// needs no other moved (ready-rows.md's spaced order).
const SPACED: u64 = 1024;

/// The kinds a conversation's note names, as Slack's daemon says them.
const KINDS: [&str; 5] = ["dm", "mention", "thread", "unread", "channel"];

/// The words conversations' rows are said in, from the launcher's
/// vocabulary (launcher/words.json), which the daemon is handed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Words {
    /// The glyph role every conversation's row shows.
    pub glyph: String,
    /// "{count} to read", beside one counted.
    pub count: String,
    /// Each kind's note, as "a direct message", by the kind's own word.
    pub kinds: BTreeMap<String, String>,
    /// The words messages' rows are said in.
    pub messages: message_rows::Words,
}

impl Words {
    /// A conversation's words from the whole vocabulary, or the first one
    /// it lacks, so a vocabulary that lost one is said rather than shown
    /// as blanks.
    ///
    /// # Errors
    ///
    /// The first word the vocabulary's `slack:conversation`, or a
    /// message's, lacks.
    pub fn read(vocabulary: &Value) -> Result<Words, String> {
        let conversation = vocabulary["things"]
            .as_array()
            .and_then(|things| {
                things
                    .iter()
                    .find(|thing| thing["id"] == "slack:conversation")
            })
            .ok_or("the vocabulary has no slack:conversation")?;
        let word = |at: &[&str]| -> Result<String, String> {
            at.iter()
                .fold(conversation, |within, field| &within[field])
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| {
                    format!(
                        "the vocabulary's slack:conversation has no {}",
                        at.join(".")
                    )
                })
        };
        let mut kinds = BTreeMap::new();
        for kind in KINDS {
            kinds.insert(kind.to_owned(), word(&["said", kind])?);
        }
        Ok(Words {
            glyph: word(&["glyph"])?,
            count: word(&["said", "count"])?,
            kinds,
            messages: message_rows::Words::read(vocabulary)?,
        })
    }
}

/// Every record of a row what Slack last said makes, keyed as the
/// launcher watches them: each conversation's row, then each message's
/// row and its drawer. A message is said through Slack refusing or out
/// of reach, as `daemon::records` holds it, since one unread stays
/// unread until Slack says it was read.
#[must_use]
pub fn records(said: &Said, words: &Words) -> Vec<(String, Value)> {
    let mut made: Vec<(String, Value)> = rows(said, words)
        .into_iter()
        .map(|row| {
            let key = row["key"].as_str().unwrap_or_default().to_owned();
            (key.clone(), json!({"key": key, "kind": "row", "row": row}))
        })
        .collect();
    made.extend(message_rows::records(&said.messages, &words.messages));
    made
}

/// Every row what Slack last said makes, unread first and then Recent,
/// each in its place's order; none but while Slack answered, so an old
/// count is never shown as a current one, as `daemon::records` holds
/// conversations.
#[must_use]
pub fn rows(said: &Said, words: &Words) -> Vec<Value> {
    if said.state != State::Ok {
        return Vec::new();
    }
    let placed = |place: &'static str, all: &[Conversation]| -> Vec<Value> {
        all.iter()
            .zip(1..)
            .map(|(one, at)| row(one, place, at * SPACED, words))
            .collect()
    };
    let mut made = placed(UNREAD, &said.conversations);
    made.extend(placed(RECENT, &said.recent));
    made
}

/// One conversation's row in `place`: its name as Slack says it, how
/// many to read beside it where it's counted, what kind it is in its
/// note, and Open in Slack, the one act a conversation has without a
/// link to share. Return opens it, as its act does.
fn row(one: &Conversation, place: &str, order: u64, words: &Words) -> Value {
    let kind = kind(one.kind);
    let side = if one.count > 0 {
        vec![json!({"said": words.count.replace("{count}", &one.count.to_string())})]
    } else {
        Vec::new()
    };
    json!({
        "key": format!("row:{place}:{}", one.id),
        "row": format!("slack:conversation:{}", one.id),
        "place": place,
        "kind": kind,
        "order": order,
        "name": one.name,
        "glyph": words.glyph,
        "hue": "program",
        "depth": 0,
        "side": side,
        "note": words.kinds.get(kind).cloned().unwrap_or_default(),
        "acts": [{"key": "verb:slack:open", "ask": {"open": {"link": one.link}}}],
        "held": {"id": one.id, "count": one.count, "at": one.at},
    })
}

/// A kind's own word, as the daemon's records say it.
fn kind(kind: Kind) -> &'static str {
    match kind {
        Kind::Dm => "dm",
        Kind::Mention => "mention",
        Kind::Thread => "thread",
        Kind::Unread => "unread",
        Kind::Channel => "channel",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words() -> Words {
        Words {
            glyph: "chat".to_owned(),
            count: "{count} to read".to_owned(),
            kinds: KINDS
                .iter()
                .map(|kind| ((*kind).to_owned(), format!("a {kind}")))
                .collect(),
            messages: message_rows::Words::default(),
        }
    }

    fn one(id: &str, kind: Kind, count: u64) -> Conversation {
        Conversation {
            id: id.to_owned(),
            name: format!("{id}'s"),
            kind,
            count,
            link: format!("slack://channel?team=T1&id={id}"),
            at: "1730300000.000100".to_owned(),
        }
    }

    fn said(state: State) -> Said {
        Said {
            state,
            conversations: vec![one("D1", Kind::Dm, 2), one("C3", Kind::Unread, 0)],
            recent: vec![one("C1", Kind::Channel, 0)],
            messages: Vec::new(),
            marks: Vec::new(),
            whole: false,
        }
    }

    #[test]
    fn each_conversation_is_a_row_of_its_place_in_slacks_order() {
        let made = rows(&said(State::Ok), &words());
        let placed: Vec<(&str, u64)> = made
            .iter()
            .map(|row| (row["key"].as_str().unwrap(), row["order"].as_u64().unwrap()))
            .collect();
        assert_eq!(
            placed,
            [
                ("row:slack-unread:D1", 1024),
                ("row:slack-unread:C3", 2048),
                ("row:slack-recent:C1", 1024),
            ]
        );
        assert_eq!(
            made[0],
            json!({
                "key": "row:slack-unread:D1", "row": "slack:conversation:D1",
                "place": "slack-unread", "kind": "dm", "order": 1024, "name": "D1's",
                "glyph": "chat", "hue": "program", "depth": 0,
                "side": [{"said": "2 to read"}], "note": "a dm",
                "acts": [{"key": "verb:slack:open",
                          "ask": {"open": {"link": "slack://channel?team=T1&id=D1"}}}],
                "held": {"id": "D1", "count": 2, "at": "1730300000.000100"},
            })
        );
    }

    #[test]
    fn one_uncounted_says_nothing_beside_it() {
        let made = rows(&said(State::Ok), &words());
        assert_eq!(made[1]["side"], json!([]));
        assert_eq!(made[2]["side"], json!([]));
        assert_eq!(made[2]["note"], "a channel");
    }

    #[test]
    fn no_row_is_made_while_slack_doesnt_answer() {
        assert_eq!(
            rows(&said(State::Unreachable), &words()),
            Vec::<Value>::new()
        );
        assert_eq!(rows(&said(State::SignedOut), &words()), Vec::<Value>::new());
    }

    #[test]
    fn the_words_are_read_from_the_vocabulary_and_one_missing_is_named() {
        let mut said: serde_json::Map<String, Value> = KINDS
            .iter()
            .map(|kind| ((*kind).to_owned(), json!(*kind)))
            .collect();
        said.insert("count".to_owned(), json!("{count} to read"));
        let mut vocabulary = json!({"things": [
            {"id": "slack:conversation", "glyph": "chat", "said": said},
            {"id": "slack:message", "glyph": "chat",
             "said": {"you": "you", "where": "in {place}", "direct": "direct message"}},
            {"id": "slack:message:sent", "name": "Sent"},
        ]});
        let read = Words::read(&vocabulary).unwrap();
        assert_eq!(
            (read.glyph.as_str(), read.kinds["thread"].as_str()),
            ("chat", "thread")
        );
        vocabulary["things"][0]["said"]
            .as_object_mut()
            .unwrap()
            .remove("mention");
        assert_eq!(
            Words::read(&vocabulary),
            Err("the vocabulary's slack:conversation has no said.mention".to_owned())
        );
    }

    // The launcher's own vocabulary, handed to the build as the unit
    // hands it to the daemon (package.nix), so a word it renames is
    // caught here rather than on Joel's screen.
    #[test]
    fn the_launchers_own_vocabulary_holds_every_word() {
        let path = std::env::var("LAUNCHER_WORDS").expect("LAUNCHER_WORDS names words.json");
        let text = std::fs::read_to_string(path).unwrap();
        let vocabulary: Value = serde_json::from_str(&text).unwrap();
        let read = Words::read(&vocabulary).unwrap();
        assert_eq!(read.count, "{count} to read");
        assert_eq!(read.kinds["dm"], "a direct message");
        assert_eq!(
            (read.messages.you.as_str(), read.messages.sent.as_str()),
            ("You", "Sent")
        );
    }
}
