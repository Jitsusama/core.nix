//! How a thing that wanted Joel stopped wanting him, so the shell can let
//! its notice or its to-do go by itself rather than asking him to mark it
//! (Joel, 2026-11-08: "i shouldn't be required to mark a notice as read if
//! I looked at the slack message in slack for example").
//!
//! Mail that wanted him and is gone from Gmail's list is asked for its
//! labels once, which say whether he read it, archived it or it's no
//! longer important; mail he read or archived here says so too. A meeting
//! is done once it has ended; one he declined isn't held at all
//! (calendar.rs). Each is told for
//! `KEPT` after, under its own record with a `done` of `{ how, at }`, and
//! learnt on the looks the program already makes, never by asking more
//! often (watch.rs).

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// How long a thing done is still told, so a shell that was away when it
/// went hears it went, and why.
pub const KEPT: i64 = 12 * 60 * 60 * 1000;

/// How a thing stopped wanting Joel, the way it went.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Went {
    /// Read in Gmail, or anywhere but here.
    ReadInGmail,
    /// Taken out of the inbox in Gmail.
    ArchivedInGmail,
    /// Gmail no longer marks it important.
    Unimportant,
    /// Deleted, put in the trash, marked spam, or otherwise gone.
    Gone,
    /// Marked read from the shell.
    ReadHere,
    /// Archived from the shell.
    ArchivedHere,
    /// A meeting that has ended.
    Ended,
}

/// When and how a thing stopped wanting Joel, `at` in milliseconds since
/// the epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Done {
    pub how: Went,
    pub at: i64,
}

/// How mail gone from what wants Joel left, from its labels as Gmail
/// gives them, or none where it still wants him and was only pushed off
/// the list by newer mail. Mail in the trash or marked spam is gone,
/// whatever else it's labelled, since neither has an inbox to be read or
/// archived from. Read is said before archived, since mail read and
/// archived at once was read first.
pub fn left(labels: Option<&Value>) -> Option<Went> {
    let Some(labels) = labels else {
        return Some(Went::Gone);
    };
    let has = |label: &str| {
        labels
            .get("labelIds")
            .and_then(Value::as_array)
            .is_some_and(|all| all.iter().any(|one| one.as_str() == Some(label)))
    };
    if has("TRASH") || has("SPAM") {
        Some(Went::Gone)
    } else if !has("UNREAD") {
        Some(Went::ReadInGmail)
    } else if !has("INBOX") {
        Some(Went::ArchivedInGmail)
    } else if !has("IMPORTANT") {
        Some(Went::Unimportant)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn mail_gone_from_the_list_says_how_by_its_labels() {
        let labels = |all: &[&str]| json!({ "labelIds": all });
        assert_eq!(
            left(Some(&labels(&["INBOX", "IMPORTANT"]))),
            Some(Went::ReadInGmail)
        );
        assert_eq!(
            left(Some(&labels(&["IMPORTANT"]))),
            Some(Went::ReadInGmail),
            "read and archived was read first"
        );
        assert_eq!(
            left(Some(&labels(&["UNREAD", "IMPORTANT"]))),
            Some(Went::ArchivedInGmail)
        );
        assert_eq!(
            left(Some(&labels(&["UNREAD", "INBOX"]))),
            Some(Went::Unimportant)
        );
        assert_eq!(left(None), Some(Went::Gone));
    }

    #[test]
    fn mail_in_the_trash_or_spam_is_gone_however_else_it_is_labelled() {
        let labels = |all: &[&str]| json!({ "labelIds": all });
        assert_eq!(
            left(Some(&labels(&["TRASH", "IMPORTANT"]))),
            Some(Went::Gone)
        );
        assert_eq!(
            left(Some(&labels(&["SPAM", "UNREAD", "INBOX"]))),
            Some(Went::Gone)
        );
    }

    #[test]
    fn mail_only_pushed_off_the_list_is_not_done() {
        let still = json!({ "labelIds": ["UNREAD", "INBOX", "IMPORTANT"] });
        assert_eq!(left(Some(&still)), None);
    }

    #[test]
    fn done_is_told_by_how_and_when() {
        let done = Done {
            how: Went::ReadInGmail,
            at: 1,
        };
        assert_eq!(
            serde_json::to_string(&done).unwrap(),
            r#"{"how":"readInGmail","at":1}"#
        );
    }
}
