//! What the shell asks of the account, one JSON object to a line on the
//! socket: so far only to answer an invitation,
//! `{"answer": {"id": "…", "with": "yes"}}`, with yes, maybe or no.
//!
//! A line that isn't one of these is let go. The id goes into the path
//! of a request to Google, so only the letters, digits and underscores
//! Google's event ids are made of are taken.

use serde::Deserialize;

use crate::calendar::Answer;

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum Ask {
    /// Answer the invitation to the meeting `id`.
    Answer { id: String, with: Answer },
}

/// What a line asks, or none when it asks nothing this program does.
pub fn read(line: &str) -> Option<Ask> {
    let ask: Ask = serde_json::from_str(line).ok()?;
    match &ask {
        Ask::Answer { id, with } => {
            let named = !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
            (named && *with != Answer::Waiting).then_some(ask)
        }
    }
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
    fn an_id_that_isnt_googles_is_not_asked() {
        for id in ["", "../calendarList", "e1?sendUpdates=all", "e 1"] {
            let line = serde_json::json!({"answer": {"id": id, "with": "yes"}}).to_string();
            assert_eq!(read(&line), None, "{id}");
        }
    }

    #[test]
    fn waiting_is_no_answer_and_anything_else_is_let_go() {
        assert_eq!(read(r#"{"answer": {"id": "e1", "with": "waiting"}}"#), None);
        assert_eq!(read(r#"{"answer": {"id": "e1", "with": "sure"}}"#), None);
        assert_eq!(read(r#"{"delete": {"id": "e1"}}"#), None);
        assert_eq!(read("not json"), None);
    }
}
