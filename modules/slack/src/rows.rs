//! Slack's rows, ready for the launcher: each conversation unread and
//! each Joel read last as the row Slack's Unread and Recent show it,
//! worded, ordered and with its acts, so the shell lays it out and words
//! nothing (Joel, 2026-11-10: "the daemons need to be the data layer, and
//! they need to serve the launcher"). The shape is Y2YBTS's ready-row
//! record (QEST-20261023-Y2YBTS attachments/speed/ready-rows.md), as
//! Google's daemon serves Mail's: a `row:<place>:<id>` record of kind
//! `row`, the place `slack-unread` or `slack-recent`, whose row's `thing`
//! is the launcher's own key for the conversation,
//! `slack:conversation:<id>`, so one in both places is one thing. Each
//! place's rows go through the ready-rows crate's `Kept`, the one shape
//! every daemon's rows take (Y2YBTS, 2026-11-11), so each keeps the order
//! it was told at where it can, and one conversation coming first is one
//! row changed rather than every row below it. SlackUnreads.js and SlackProgram.qml make the same rows
//! today; these take their place as the shell reads them. Each message
//! wanting Joel is a row too, of `slack-messages` (`message_rows.rs`).

use std::collections::BTreeMap;

use serde_json::{Value, json};

use ready_rows::kept::Kept;

use crate::message_rows;
use crate::unreads::{Conversation, Kind, Said, State};

/// The place of each conversation unread, in Slack's order: direct
/// messages, then channels mentioning Joel, then threads, then channels
/// merely unread (unreads.rs's `counted`).
pub const UNREAD: &str = "slack-unread";

/// The place of each conversation Joel read last, newest first.
pub const RECENT: &str = "slack-recent";

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
        let word = |at: &[&str]| message_rows::word(vocabulary, "slack:conversation", at);
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

/// Each place's rows as last told, so the next making keeps each row's
/// order where it can.
#[derive(Debug, Default)]
pub struct Told {
    places: BTreeMap<&'static str, Kept>,
}

impl Told {
    /// Every record of a row what Slack last said makes, keyed as the
    /// launcher watches them and in the order shown: each conversation's
    /// row, then each message's row and its drawer. A message is said
    /// through Slack refusing or out of reach, as `daemon::records`
    /// holds it, since one unread stays unread until Slack says it was
    /// read.
    pub fn records(&mut self, said: &Said, words: &Words) -> Vec<(String, Value)> {
        let (messages, drawers) = message_rows::rows(&said.messages, &words.messages);
        let placed = [
            (
                UNREAD,
                rows(said, UNREAD, &said.conversations, words),
                Vec::new(),
            ),
            (RECENT, rows(said, RECENT, &said.recent, words), Vec::new()),
            (message_rows::MESSAGES, messages, drawers),
        ];
        let mut made = Vec::new();
        for (place, rows, beside) in placed {
            // Each row, then the drawer it names, as they're shown.
            let keys: Vec<String> = rows
                .iter()
                .flat_map(|row| [&row["key"], &row["drawer"]])
                .filter_map(|key| key.as_str().map(str::to_owned))
                .collect();
            let kept = self.places.entry(place).or_default();
            kept.take(rows, beside);
            made.extend(keys.into_iter().filter_map(|key| {
                let record = kept.get(&key)?.clone();
                Some((key, record))
            }));
        }
        made
    }
}

/// Each conversation's row in `place`, in Slack's order, for `Kept` to
/// set its `order`; none but while Slack answered, so an old count is
/// never shown as a current one, as `daemon::records` holds
/// conversations.
fn rows(said: &Said, place: &'static str, all: &[Conversation], words: &Words) -> Vec<Value> {
    if said.state != State::Ok {
        return Vec::new();
    }
    all.iter().map(|one| row(one, place, words)).collect()
}

/// One conversation's row in `place`: its name as Slack says it, how
/// many to read beside it where it's counted, what kind it is in its
/// note, and Open in Slack, the one act a conversation has without a
/// link to share. Return opens it, as its act does.
fn row(one: &Conversation, place: &str, words: &Words) -> Value {
    let kind = kind(one.kind);
    let side = if one.count > 0 {
        vec![json!({"said": words.count.replace("{count}", &one.count.to_string())})]
    } else {
        Vec::new()
    };
    json!({
        "key": format!("row:{place}:{}", one.id),
        "thing": format!("slack:conversation:{}", one.id),
        "place": place,
        "kind": kind,
        "name": one.name,
        "glyph": words.glyph,
        "hue": "program",
        "depth": 0,
        "side": side,
        "note": words.kinds.get(kind).cloned().unwrap_or_default(),
        "acts": [message_rows::open("verb:slack:open", &one.link)],
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
    use crate::message_rows::launchers_vocabulary;

    fn words() -> Words {
        Words::read(&launchers_vocabulary()).expect("the launcher's words say a conversation")
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

    /// Each row's key and order, as shown.
    fn placed(made: &[(String, Value)]) -> Vec<(&str, i64)> {
        made.iter()
            .map(|(key, record)| (key.as_str(), record["row"]["order"].as_i64().unwrap()))
            .collect()
    }

    #[test]
    fn each_conversation_is_a_row_of_its_place_in_slacks_order() {
        let made = Told::default().records(&said(State::Ok), &words());
        assert_eq!(
            placed(&made),
            [
                ("row:slack-unread:D1", 1024),
                ("row:slack-unread:C3", 2048),
                ("row:slack-recent:C1", 1024),
            ]
        );
        assert_eq!(
            made[0].1,
            json!({"key": "row:slack-unread:D1", "kind": "row", "row": {
                "key": "row:slack-unread:D1", "thing": "slack:conversation:D1",
                "place": "slack-unread", "kind": "dm", "order": 1024, "name": "D1's",
                "glyph": "chat", "hue": "program", "depth": 0,
                "side": [{"said": "2 to read"}], "note": "a direct message",
                "acts": [{"key": "verb:slack:open", "open": "slack://channel?team=T1&id=D1"}],
                "held": {"id": "D1", "count": 2, "at": "1730300000.000100"},
            }})
        );
    }

    #[test]
    fn one_coming_first_takes_an_order_of_its_own_and_the_rest_keep_theirs() {
        let mut told = Told::default();
        let before = told.records(&said(State::Ok), &words());
        let mut now = said(State::Ok);
        now.conversations.insert(0, one("D9", Kind::Dm, 1));
        let after = told.records(&now, &words());
        assert_eq!(
            placed(&after),
            [
                ("row:slack-unread:D9", 0),
                ("row:slack-unread:D1", 1024),
                ("row:slack-unread:C3", 2048),
                ("row:slack-recent:C1", 1024),
            ]
        );
        // Only the new row reads differently from what was told.
        let moved: Vec<&(String, Value)> =
            after.iter().filter(|one| !before.contains(one)).collect();
        assert_eq!(moved, [&after[0]]);
    }

    #[test]
    fn one_uncounted_says_nothing_beside_it() {
        let made = Told::default().records(&said(State::Ok), &words());
        assert_eq!(made[1].1["row"]["side"], json!([]));
        assert_eq!(made[2].1["row"]["side"], json!([]));
        assert_eq!(made[2].1["row"]["note"], "a channel");
    }

    #[test]
    fn no_row_is_made_while_slack_doesnt_answer() {
        let mut told = Told::default();
        assert_eq!(told.records(&said(State::Unreachable), &words()), []);
        assert_eq!(told.records(&said(State::SignedOut), &words()), []);
    }

    #[test]
    fn the_words_are_read_from_the_vocabulary_and_one_missing_is_named() {
        let mut vocabulary = launchers_vocabulary();
        let read = Words::read(&vocabulary).expect("the launcher's words say a conversation");
        assert_eq!(
            (
                read.glyph.as_str(),
                read.kinds["dm"].as_str(),
                read.kinds["thread"].as_str(),
                read.messages.sent.as_str()
            ),
            ("chat", "a direct message", "replies in threads", "Sent"),
            "a conversation's words, as the launcher says them"
        );
        vocabulary["things"]
            .as_array_mut()
            .and_then(|things| {
                things
                    .iter_mut()
                    .find(|one| one["id"] == "slack:conversation")
            })
            .and_then(|one| one["said"].as_object_mut())
            .expect("the launcher's words say a conversation")
            .remove("mention");
        assert_eq!(
            Words::read(&vocabulary),
            Err("the vocabulary's slack:conversation has no said.mention".to_owned())
        );
    }
}
