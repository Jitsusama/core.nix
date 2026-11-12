use super::*;

/// A note of three paragraphs under a heading, with a reference.
const NOTE: &str = "# Note\n\nOne says\nthis much.\n\nTwo says\nthat.\n\nThree, [linked].\n\n\
                    [linked]: https://example.com\n";

/// `NOTE` with `from` changed to `to`, which it holds once.
fn changed(text: &str, from: &str, to: &str) -> String {
    assert_eq!(
        text.matches(from).count(),
        1,
        "{from} isn't in the note once"
    );
    text.replacen(from, to, 1)
}

#[test]
fn notes_changed_in_different_paragraphs_are_apart() {
    let ours = changed(NOTE, "this much.", "this much more.");
    let theirs = changed(NOTE, "that.\n", "that.\nAnd more.\n");
    let head = changed(&ours, "that.\n", "that.\nAnd more.\n");
    assert!(
        prose_apart(NOTE, &ours, &theirs, &head),
        "a line changed in one paragraph and one added to another"
    );
    let added = changed(NOTE, "Three", "Before three.\n\nThree");
    let head = changed(&ours, "Three", "Before three.\n\nThree");
    assert!(
        prose_apart(NOTE, &ours, &added, &head),
        "a paragraph added two away from the other side's"
    );
    assert!(
        would_prose_be_apart(NOTE, &ours, &theirs),
        "the same two sides, judged before the rebase"
    );
}

#[test]
fn a_note_both_sides_changed_apart_stands_apart_where_the_gate_asks() {
    // 12 November's case: #3769 and a change merged before it edited
    // different paragraphs of docs/design.md.
    use crate::ready::{Side, stands_apart};
    let ours = changed(NOTE, "this much.", "this much more.");
    let theirs = changed(NOTE, "that.", "that, too.");
    let head = changed(&ours, "that.", "that, too.");
    let (at_head, at_main) = (|_: &str| head.clone(), |_: &str| theirs.clone());
    let (at_base, checked) = (|_: &str| NOTE.to_owned(), |_: &str| ours.clone());
    for (file, apart) in [
        ("docs/design.md", true),
        ("docs/glossary.md", false),
        ("README.md", false),
    ] {
        let files = [file.to_owned()];
        let branch = Side {
            changed: &files,
            gone: &[],
            text: &at_head,
        };
        let main = Side {
            changed: &files,
            gone: &[],
            text: &at_main,
        };
        assert_eq!(
            stands_apart(file, &branch, &main, &at_base, &checked),
            apart,
            "{file}, changed in different paragraphs by each side"
        );
    }
}

#[test]
fn notes_changed_in_one_paragraph_or_beside_each_other_are_not_apart() {
    let ours = changed(NOTE, "One says", "One said");
    let theirs = changed(NOTE, "this much.", "this little.");
    let head = changed(&ours, "this much.", "this little.");
    assert!(
        !prose_apart(NOTE, &ours, &theirs, &head),
        "two lines of one paragraph"
    );
    let joined = changed(NOTE, "this much.\n\nTwo", "this much.\nTwo");
    let two = changed(NOTE, "that.", "that, too.");
    let head = changed(&joined, "that.", "that, too.");
    assert!(
        !prose_apart(NOTE, &joined, &two, &head),
        "the blank line between them taken away"
    );
    let ours = changed(NOTE, "this much.", "this much more.");
    let theirs = changed(NOTE, "Three", "Three, again");
    let head = changed(&ours, "Three", "Three, still");
    assert!(
        !prose_apart(NOTE, &ours, &theirs, &head),
        "a head that isn't both sides' changes"
    );
}

#[test]
fn a_heading_or_a_reference_either_side_touches_is_not_apart() {
    let theirs = changed(NOTE, "that.", "that, too.");
    for ours in [
        changed(NOTE, "# Note", "# Notes"),
        changed(NOTE, "Three, [linked].", "Three, [linked].\n\n## Later"),
        changed(NOTE, "https://example.com", "https://example.org"),
        changed(
            NOTE,
            "this much.",
            "this much.\n\n[more]: https://example.net",
        ),
    ] {
        let head = changed(&ours, "that.", "that, too.");
        assert!(
            !prose_apart(NOTE, &ours, &theirs, &head),
            "another part of the note may read it:\n{ours}"
        );
        assert!(!would_prose_be_apart(NOTE, &theirs, &ours), "{ours}");
    }
}

#[test]
fn only_docs_notes_no_check_reads_are_prose() {
    assert!(is_prose("docs/design.md"), "a note in docs/");
    assert!(
        is_prose("docs/decisions/0001-a.md"),
        "a note in a folder of docs/"
    );
    assert!(
        !is_prose("docs/glossary.md"),
        "the glossary's checks read its words"
    );
    assert!(!is_prose("README.md"), "a note outside docs/");
    assert!(!is_prose("docs/notes.json"), "a file in docs/ but no note");
}
