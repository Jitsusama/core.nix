use std::fs;
use std::path::Path;

use super::super::measures::{Class, Measure};
use super::{MOST, unless_whole, whole};

/// An asks list of `n` lines, each a record of its own.
fn list(n: usize) -> String {
    (0..n)
        .map(|i| format!("{{\"at\":{i},\"words\":\"ask {i}\"}}\n"))
        .collect()
}

/// The asks list `text` written at `folder`'s `asks.jsonl` and read as
/// the hour reads it, keeping its count in `folder`.
fn read_list(folder: &Path, text: &str) -> (String, Option<String>) {
    let path = folder.join("asks.jsonl");
    fs::write(&path, text).expect("the asks list is written");
    whole(&path, folder)
}

/// A measure under `key` that read two slipped asks against a target of
/// none slipped.
fn read_two(key: &str) -> Measure {
    Measure {
        key: key.to_owned(),
        class: Class::Slip,
        says: format!("{key}, as read"),
        now: 2.0,
        target: Some(0.0),
        owner: "J7XHQN".to_owned(),
        behind: vec!["an ask · J7XHQN".to_owned()],
    }
}

#[test]
fn a_whole_list_is_read_and_the_most_lines_kept() {
    let folder = tempfile::tempdir().expect("a scratch folder");
    let (text, why) = read_list(folder.path(), &list(4));
    assert_eq!(text, list(4), "a whole list wasn't read as written");
    assert!(why.is_none(), "a whole list was said unread: {why:?}");
    let kept = fs::read_to_string(folder.path().join(MOST)).expect("the count is kept");
    assert_eq!(kept, "4", "the most lines read weren't kept");
}

#[test]
fn a_missing_list_is_unread_not_empty() {
    let folder = tempfile::tempdir().expect("a scratch folder");
    let (text, why) = whole(&folder.path().join("asks.jsonl"), folder.path());
    assert!(text.is_empty(), "a missing list read as something: {text}");
    assert!(
        why.as_deref()
            .is_some_and(|why| why.contains("can't be read")),
        "a missing list wasn't said unread: {why:?}"
    );
}

#[test]
fn a_list_written_over_with_under_half_its_lines_is_unread_until_restored() {
    let folder = tempfile::tempdir().expect("a scratch folder");
    read_list(folder.path(), &list(853));
    let (text, why) = read_list(folder.path(), &list(3));
    assert!(text.is_empty(), "a list written over was read: {text}");
    assert_eq!(
        why.as_deref(),
        Some("it holds 3 lines, under half the 853 read in it before"),
        "a list written over wasn't said unread, with why"
    );
    let (again, why) = read_list(folder.path(), &list(3));
    assert!(
        again.is_empty() && why.is_some(),
        "a second read forgot the list had been whole: {why:?}"
    );
    let (restored, why) = read_list(folder.path(), &list(853));
    assert!(
        restored == list(853) && why.is_none(),
        "a restored list wasn't read whole again: {why:?}"
    );
}

#[test]
fn a_list_that_grows_or_keeps_half_its_lines_is_read() {
    let folder = tempfile::tempdir().expect("a scratch folder");
    read_list(folder.path(), &list(10));
    let (half, why) = read_list(folder.path(), &list(5));
    assert!(
        half == list(5) && why.is_none(),
        "a list holding half the lines read before was said unread: {why:?}"
    );
    let kept = fs::read_to_string(folder.path().join(MOST)).expect("the count is kept");
    assert_eq!(kept, "10", "a shorter read lowered the most lines kept");
    read_list(folder.path(), &list(12));
    let kept = fs::read_to_string(folder.path().join(MOST)).expect("the count is kept");
    assert_eq!(kept, "12", "a longer read didn't raise the most lines kept");
}

#[test]
fn only_the_measures_read_from_the_asks_list_are_said_unread() {
    let keys = [
        "slips",
        "slips-derived",
        "slips-parked",
        "slips-parked-derived",
        "ask-to-first-slice",
        "ask-to-first-slice-derived",
        "holds-undated",
        "quarantine-undated",
        "reverts",
    ];
    let mut all = keys.map(read_two);
    unless_whole(&mut all, Some("it holds 3 lines"));
    for measure in &all[..7] {
        assert!(
            measure.target.is_none() && measure.now.abs() < f64::EPSILON,
            "{} read from an unread list kept its count: {measure:?}",
            measure.key
        );
        assert_eq!(
            measure.behind,
            ["the asks list wasn't read whole: it holds 3 lines"],
            "{} didn't say why it went unread",
            measure.key
        );
    }
    for measure in &all[7..] {
        assert_eq!(
            measure.target,
            Some(0.0),
            "{}, read from elsewhere, was said unread",
            measure.key
        );
    }
}

#[test]
fn a_whole_list_leaves_every_measure_as_read() {
    let mut all = [read_two("slips"), read_two("holds-undated")];
    unless_whole(&mut all, None);
    assert!(
        all.iter()
            .all(|m| m.target == Some(0.0) && m.behind.len() == 1),
        "a whole list changed a measure: {all:?}"
    );
}
