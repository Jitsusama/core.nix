//! What the door says in the journal of each copy, paste, listing,
//! clearing and refusal, one line apiece in a shape that never changes.
//!
//! A reader of the journal keeps only fields, never free text, and never
//! what was copied: the door says the session a client runs for
//! (src/peer.rs), the selection, the type and how many bytes, and for a
//! refusal why, as one word from a fixed set. A type is said only when
//! it holds nothing a field shouldn't (src/kinds.rs):
//!
//! ```text
//! copied session=01a1... selection=primary type=text/plain;charset=utf-8 bytes=12
//! pasted session=01a1... selection=clipboard type=image/png bytes=40960
//! listed session=01a1... selection=clipboard types=6
//! cleared session=01a1... selection=primary
//! refused session=01a1... selection=clipboard why=secret
//! refused session=unknown selection=- why=unfenced peer=no-bubblewrap
//! ```

use crate::kinds::sayable;
use crate::peer::Outside;
use crate::wire::{Selection, Why};

/// A copy put on a selection.
pub fn copied(session: &str, selection: Selection, kind: &str, bytes: usize) -> String {
    format!(
        "copied session={session} selection={} type={} bytes={bytes}",
        selection.word(),
        sayable(kind)
    )
}

/// A selection read for a client.
pub fn pasted(session: &str, selection: Selection, kind: &str, bytes: usize) -> String {
    format!(
        "pasted session={session} selection={} type={} bytes={bytes}",
        selection.word(),
        sayable(kind)
    )
}

/// A selection's types listed for a client.
pub fn listed(session: &str, selection: Selection, types: usize) -> String {
    format!(
        "listed session={session} selection={} types={types}",
        selection.word()
    )
}

/// A selection emptied.
pub fn cleared(session: &str, selection: Selection) -> String {
    format!("cleared session={session} selection={}", selection.word())
}

/// Something refused, for a fenced client's session, on a selection when
/// one was asked about.
pub fn refused(session: &str, selection: Option<Selection>, why: Why) -> String {
    format!(
        "refused session={session} selection={} why={}",
        selection.map_or("-", Selection::word),
        why.word()
    )
}

/// A client refused as outside the fence, and how the door knew.
pub fn outside(outside: Outside) -> String {
    format!(
        "{} peer={}",
        refused(crate::peer::UNKNOWN, None, Why::Unfenced),
        outside.word()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_line_holds_its_fields_and_never_free_text() {
        assert_eq!(
            copied("s1", Selection::Primary, "text/plain", 3),
            "copied session=s1 selection=primary type=text/plain bytes=3"
        );
        assert_eq!(
            pasted("s1", Selection::Clipboard, "a b\nrefused", 9),
            "pasted session=s1 selection=clipboard type=other bytes=9"
        );
        assert_eq!(
            listed("s1", Selection::Clipboard, 6),
            "listed session=s1 selection=clipboard types=6"
        );
        assert_eq!(
            cleared("s1", Selection::Primary),
            "cleared session=s1 selection=primary"
        );
        assert_eq!(
            refused("s1", Some(Selection::Clipboard), Why::Secret),
            "refused session=s1 selection=clipboard why=secret"
        );
        assert_eq!(
            outside(Outside::NoBubblewrap),
            "refused session=unknown selection=- why=unfenced peer=no-bubblewrap"
        );
    }
}
