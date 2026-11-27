//! The words the far line is built from and says: a quest's id as a
//! folder's name, a word quoted for the far shell, and the line it says
//! as a job starts.

use crate::far::{Heard, plain_quest, quoted};

#[test]
fn a_quest_names_a_folder_and_nothing_more() {
    assert!(
        plain_quest("QEST-20261115-ABC123"),
        "a quest's id was refused"
    );
    for odd in ["QEST-20261115-ABC12/", "../QEST", "QEST-2026111-ABC123", ""] {
        assert!(!plain_quest(odd), "{odd:?} was taken for a quest");
    }
    assert_eq!(
        Heard::Started.line("1-2"),
        "elsewhere: running 1-2",
        "the start line moved"
    );
}

#[test]
fn a_word_is_quoted_only_when_a_shell_would_split_it() {
    assert_eq!(quoted("-workers"), "-workers", "a plain word was quoted");
    assert_eq!(quoted(""), "''", "an empty word was lost");
    assert_eq!(
        quoted("$HOME"),
        "'$HOME'",
        "a variable was left for the far shell"
    );
}
