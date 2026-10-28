//! What the shell asks of the account, one JSON object to a line on the
//! socket: to answer an invitation, `{"answer": {"id": "…", "with": "yes"}}`
//! with yes, maybe or no; to cancel a meeting Joel holds,
//! `{"cancel": {"id": "…"}}`; to move one Google lets him change,
//! `{"move": {"id": "…", "start": …, "end": …}}` in milliseconds since
//! the epoch; to mark mail read, `{"read": {"id": "…"}}`; and to archive
//! it, `{"archive": {"id": "…"}}`.
//!
//! A line that isn't one of these is let go. The id goes into the path
//! of a request to Google, so only the letters, digits and underscores
//! Google's event and message ids are made of are taken.

use serde::Deserialize;

use crate::calendar::Answer;

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum Ask {
    /// Answer the invitation to the meeting `id`.
    Answer { id: String, with: Answer },
    /// Cancel the meeting `id`, which Joel holds, telling every guest.
    Cancel { id: String },
    /// Move the meeting `id` to run from `start` to `end`, telling everyone
    /// on it.
    Move { id: String, start: i64, end: i64 },
    /// Mark the mail `id` read.
    Read { id: String },
    /// Take the mail `id` out of the inbox.
    Archive { id: String },
}

/// What a line asks, or none when it asks nothing this program does.
pub fn read(line: &str) -> Option<Ask> {
    let ask: Ask = serde_json::from_str(line).ok()?;
    let taken = match &ask {
        Ask::Answer { id, with } => named(id) && *with != Answer::Waiting,
        Ask::Cancel { id } | Ask::Read { id } | Ask::Archive { id } => named(id),
        // A meeting ends after it starts, and both are after 1970.
        Ask::Move { id, start, end } => named(id) && 0 < *start && start < end,
    };
    taken.then_some(ask)
}

/// Whether an id is one Google could have made.
fn named(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_answer_is_read_with_the_meeting_it_names() {
        assert_eq!(
            read(r#"{"answer": {"id": "abc_20261026T200000Z", "with": "maybe"}}"#),
            Some(Ask::Answer {
                id: "abc_20261026T200000Z".into(),
                with: Answer::Maybe
            })
        );
    }

    #[test]
    fn a_cancel_is_read_with_the_meeting_it_names() {
        assert_eq!(
            read(r#"{"cancel": {"id": "abc_20261026T200000Z"}}"#),
            Some(Ask::Cancel {
                id: "abc_20261026T200000Z".into()
            })
        );
    }

    #[test]
    fn a_move_is_read_with_the_meeting_and_its_new_times() {
        assert_eq!(
            read(r#"{"move": {"id": "e1", "start": 1793000000000, "end": 1793001800000}}"#),
            Some(Ask::Move {
                id: "e1".into(),
                start: 1_793_000_000_000,
                end: 1_793_001_800_000
            })
        );
        for (start, end) in [
            (1_793_001_800_000_i64, 1_793_000_000_000_i64),
            (5, 5),
            (-5, 5),
        ] {
            let line = serde_json::json!({"move": {"id": "e1", "start": start, "end": end}});
            assert_eq!(read(&line.to_string()), None, "{start} to {end}");
        }
    }

    #[test]
    fn mail_is_read_or_archived_by_its_id() {
        assert_eq!(
            read(r#"{"read": {"id": "19a2b3c4d5e6f708"}}"#),
            Some(Ask::Read {
                id: "19a2b3c4d5e6f708".into()
            })
        );
        assert_eq!(
            read(r#"{"archive": {"id": "19a2b3c4d5e6f708"}}"#),
            Some(Ask::Archive {
                id: "19a2b3c4d5e6f708".into()
            })
        );
    }

    #[test]
    fn an_id_that_isnt_googles_is_not_asked() {
        for id in ["", "../calendarList", "e1?sendUpdates=all", "e 1"] {
            let line = serde_json::json!({"answer": {"id": id, "with": "yes"}}).to_string();
            assert_eq!(read(&line), None, "{id}");
            let line = serde_json::json!({"archive": {"id": id}}).to_string();
            assert_eq!(read(&line), None, "{id}");
            let line = serde_json::json!({"cancel": {"id": id}}).to_string();
            assert_eq!(read(&line), None, "{id}");
            let line = serde_json::json!({"move": {"id": id, "start": 1, "end": 2}}).to_string();
            assert_eq!(read(&line), None, "{id}");
        }
    }

    #[test]
    fn waiting_is_no_answer_and_anything_else_is_let_go() {
        assert_eq!(read(r#"{"answer": {"id": "e1", "with": "waiting"}}"#), None);
        assert_eq!(read(r#"{"answer": {"id": "e1", "with": "sure"}}"#), None);
        assert_eq!(read(r#"{"delete": {"id": "e1"}}"#), None);
        assert_eq!(read(r#"{"read": {"id": "m1", "labels": ["TRASH"]}}"#), None);
        assert_eq!(read("not json"), None);
    }
}
