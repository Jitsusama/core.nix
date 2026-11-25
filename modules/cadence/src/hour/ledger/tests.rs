use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use super::super::measures::{self, Class, Measure};
use super::{FROM_ASKS, SHORT_LASTS, Seen, unless_whole, whole};
use crate::tracking::Tracking;

const NOW: i64 = 1_794_700_000;

/// An asks list of `n` asks, each a record of its own, with a blank line
/// and a line of something else between them, neither of which is one.
fn list(n: usize) -> String {
    (0..n).fold(String::from("\n{\"ts\": 1}\n"), |mut text, i| {
        let _ = writeln!(text, "{{\"at\":\"2026-11-25T0{i}\",\"words\":\"ask {i}\"}}");
        text
    })
}

/// The asks list `text` written at `folder`'s `asks.jsonl` and read at
/// `at` as the hour reads it, beside what it has `seen`.
fn read_at(folder: &Path, text: &str, seen: &mut Seen, at: i64) -> (String, Option<String>) {
    let path = folder.join("asks.jsonl");
    fs::write(&path, text).expect("the asks list is written");
    whole(&path, seen, at)
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
fn a_whole_list_is_read_and_only_its_asks_counted() {
    let folder = tempfile::tempdir().expect("a scratch folder");
    let mut seen = Seen::default();
    let (text, why) = read_at(folder.path(), &list(4), &mut seen, NOW);
    assert_eq!(text, list(4), "a whole list wasn't read as written");
    assert!(why.is_none(), "a whole list was said unread: {why:?}");
    assert_eq!(seen.most, 4, "a line that's no ask was counted");
}

#[test]
fn a_list_never_seen_and_not_there_holds_no_asks_but_one_gone_is_unread() {
    let folder = tempfile::tempdir().expect("a scratch folder");
    let path = folder.path().join("asks.jsonl");
    let (text, why) = whole(&path, &mut Seen::default(), NOW);
    assert!(
        text.is_empty() && why.is_none(),
        "a list never kept here was said unread: {why:?}"
    );
    let mut seen = Seen {
        most: 853,
        short_since: None,
    };
    let (text, why) = whole(&path, &mut seen, NOW);
    assert!(text.is_empty(), "a list gone read as something: {text}");
    assert!(
        why.as_deref()
            .is_some_and(|why| why.contains("can't be read")),
        "a list gone once seen wasn't said unread: {why:?}"
    );
}

#[test]
fn a_list_written_over_with_under_half_its_asks_is_unread_until_restored() {
    let folder = tempfile::tempdir().expect("a scratch folder");
    let mut seen = Seen::default();
    read_at(folder.path(), &list(853), &mut seen, NOW);
    let over = "{\"ts\": 1}\n{\"ts\": 2}\n{\"ts\": 3}\n";
    let (text, why) = read_at(folder.path(), over, &mut seen, NOW);
    assert!(text.is_empty(), "a list written over was read: {text}");
    assert_eq!(
        why.as_deref(),
        Some("it holds 0 ask records, under half the 853 read in it"),
        "a list written over wasn't said unread, with why"
    );
    let (restored, why) = read_at(folder.path(), &list(853), &mut seen, NOW + 240);
    assert!(
        restored == list(853) && why.is_none(),
        "a restored list wasn't read whole again: {why:?}"
    );
    assert_eq!(
        seen,
        Seen {
            most: 853,
            short_since: None
        },
        "a restored list didn't count as whole again"
    );
}

#[test]
fn a_list_short_for_two_hours_is_taken_as_pruned_and_counted_afresh() {
    let folder = tempfile::tempdir().expect("a scratch folder");
    let mut seen = Seen::default();
    read_at(folder.path(), &list(10), &mut seen, NOW);
    let (_, early) = read_at(folder.path(), &list(4), &mut seen, NOW);
    let (_, still) = read_at(folder.path(), &list(4), &mut seen, NOW + SHORT_LASTS - 1);
    assert!(
        early.is_some() && still.is_some(),
        "a list short for under two hours was read: {early:?} {still:?}"
    );
    let (text, why) = read_at(folder.path(), &list(4), &mut seen, NOW + SHORT_LASTS);
    assert!(
        text == list(4) && why.is_none(),
        "a list short for two hours wasn't taken as pruned: {why:?}"
    );
    assert_eq!(seen.most, 4, "the pruned list wasn't counted afresh");
}

#[test]
fn a_list_holding_no_asks_is_never_taken_as_pruned() {
    let folder = tempfile::tempdir().expect("a scratch folder");
    let mut seen = Seen::default();
    read_at(folder.path(), &list(10), &mut seen, NOW);
    let over = "{\"ts\": 1}\n";
    read_at(folder.path(), over, &mut seen, NOW);
    let (text, why) = read_at(folder.path(), over, &mut seen, NOW + 2 * SHORT_LASTS);
    assert!(text.is_empty(), "a list holding no asks was read: {text}");
    assert!(
        why.is_some(),
        "a list holding no asks for two hours was taken as pruned"
    );
    assert_eq!(
        seen.most, 10,
        "a list holding no asks lowered the most seen"
    );
}

#[test]
fn a_byte_that_is_no_text_leaves_the_list_read() {
    let folder = tempfile::tempdir().expect("a scratch folder");
    let path = folder.path().join("asks.jsonl");
    let mut bytes = list(4).into_bytes();
    bytes.extend_from_slice(b"{\"at\":\"x\",\"words\":\"torn \xff\"}\n");
    fs::write(&path, bytes).expect("the asks list is written");
    let mut seen = Seen::default();
    let (text, why) = whole(&path, &mut seen, NOW);
    assert!(
        why.is_none(),
        "a list with one torn byte was said unread: {why:?}"
    );
    assert!(
        text.starts_with(&list(4)),
        "the whole lines before the torn byte weren't read: {text}"
    );
    assert_eq!(seen.most, 5, "the ask holding a torn byte wasn't counted");
}

#[test]
fn a_list_that_grows_or_keeps_half_its_asks_is_read() {
    let folder = tempfile::tempdir().expect("a scratch folder");
    let mut seen = Seen::default();
    read_at(folder.path(), &list(10), &mut seen, NOW);
    let (half, why) = read_at(folder.path(), &list(5), &mut seen, NOW);
    assert!(
        half == list(5) && why.is_none(),
        "a list holding half the asks read before was said unread: {why:?}"
    );
    assert_eq!(seen.most, 10, "a shorter read lowered the most asks seen");
    read_at(folder.path(), &list(12), &mut seen, NOW);
    assert_eq!(seen.most, 12, "a longer read didn't raise the most");
}

#[test]
fn every_measure_the_asks_list_feeds_is_one_it_says_unread() {
    let fed = [
        measures::slips("", NOW, Tracking::His),
        measures::slips("", NOW, Tracking::Derived),
        measures::parked("", Tracking::His),
        measures::parked("", Tracking::Derived),
        measures::to_first_slice("", NOW, Tracking::His),
        measures::to_first_slice("", NOW, Tracking::Derived),
        measures::holds::holds(&[], "", NOW),
    ];
    for measure in &fed {
        let key = measure.key.strip_suffix("-derived").unwrap_or(&measure.key);
        assert!(
            FROM_ASKS.contains(&key),
            "{} reads the asks list but isn't said unread with it",
            measure.key
        );
    }
}

#[test]
fn only_the_measures_read_from_the_asks_list_are_said_unread() {
    let keys = [
        "slips",
        "slips-derived",
        "slips-parked",
        "ask-to-first-slice-derived",
        "holds-undated",
        "quarantine-undated",
        "reverts",
    ];
    let mut all = keys.map(read_two);
    unless_whole(&mut all, Some("it holds 3 asks"));
    for measure in &all[..5] {
        assert_eq!(measure.target, None, "{} kept its judgement", measure.key);
        assert_eq!(
            measure.says,
            format!(
                "{}, as read (the asks list unread: it holds 3 asks)",
                measure.key
            ),
            "{} didn't say why it went unjudged",
            measure.key
        );
        assert_eq!(measure.behind.len(), 1, "{} lost its lines", measure.key);
    }
    for measure in &all[5..] {
        assert_eq!(
            measure,
            &read_two(&measure.key),
            "{}, read from elsewhere, was changed",
            measure.key
        );
    }
}

#[test]
fn a_whole_list_leaves_every_measure_as_read() {
    let mut all = [read_two("slips"), read_two("holds-undated")];
    unless_whole(&mut all, None);
    assert_eq!(
        all,
        [read_two("slips"), read_two("holds-undated")],
        "a whole list changed a measure"
    );
}
