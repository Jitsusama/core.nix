//! What the safe word's Slack reader read on past and never read, said
//! to Joel in rows of their own: as minutes before each was raised, kept a
//! week, and one search's spans within ten minutes said as one row.
//! Slack's rows are kept by pineapple's `missed` as every reader's are, so
//! these cases pin its rules as well as Slack's names, byte for byte.

use std::path::Path;
use std::slice;

use pineapple::deaf::{MEET, Search, keep_missed, missed};

const AT: u64 = 1_795_000_000_000;

const MINUTE: u64 = 60_000;

#[test]
fn what_was_never_read_is_said_as_minutes_before_the_row() {
    let tag = missed(
        Search::Tag,
        Some(AT - 41 * MINUTE + 1),
        Some(AT - 13 * MINUTE),
        AT,
    );
    assert_eq!(
        tag["why"],
        "The safe word never read what tagged you in Slack without saying pineapple from 41 to \
         13 minutes before this was raised, as more was said than it could read, so a stop said \
         then wasn't heard",
        "a span begun part way into a minute is said from the whole one"
    );
    assert_eq!(
        tag["id"],
        format!(
            "pineapple-slack-missed-tag-{}-{}",
            AT - 41 * MINUTE + 1,
            AT - 13 * MINUTE
        ),
        "named for its span, so written again it's the same row"
    );
    assert_eq!(
        tag["doneWhen"],
        serde_json::json!({"by": "person"}),
        "no round reads it again, so only Joel says he has looked"
    );
    let phrase = missed(Search::Phrase, Some(AT - MINUTE), Some(AT - 10), AT);
    assert_eq!(
        phrase["why"],
        "The safe word never read what said pineapple in Slack without tagging you in the 1 \
         minute before this was raised, as more was said than it could read, so a stop said \
         then wasn't heard",
        "one ending this minute is said up to the row"
    );
    assert_eq!(
        phrase["id"],
        format!("pineapple-slack-missed-phrase-{}-{}", AT - MINUTE, AT - 10),
        "each search's rows named apart"
    );
    assert!(
        missed(Search::Tag, None, Some(AT - MINUTE), AT)["why"]
            .as_str()
            .is_some_and(|why| why.contains(
                "before this was raised, since when it last read can't be told, as more"
            )),
        "and one with no last read it can tell is said with no minutes"
    );
}

#[test]
fn each_span_never_read_is_kept_a_week_beside_the_others() {
    let folder = std::env::temp_dir().join(format!(
        "pineapple-missed-file-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
    ));
    let file = folder.join("missed.json");
    std::fs::create_dir_all(&folder).expect("the folder is made");
    std::fs::write(&file, "not JSON").expect("a file it can't read");
    let week = 7 * 24 * 60 * MINUTE;
    let row = |search, at| missed(search, Some(at - MINUTE), Some(at), at);
    assert!(
        keep_missed(&file, row(Search::Tag, AT), AT).is_err()
            && std::fs::read_to_string(&file).is_ok_and(|kept| kept == "not JSON"),
        "a file it can't read may hold rows, so it isn't written over"
    );
    std::fs::write(&file, r#"{"v":2}"#).expect("a file holding no rows it can read");
    assert!(
        keep_missed(&file, row(Search::Tag, AT), AT).is_err()
            && std::fs::read_to_string(&file).is_ok_and(|kept| kept == r#"{"v":2}"#),
        "nor is one that reads but holds no rows, as it may be a newer shape"
    );
    std::fs::remove_file(&file).expect("taken away");
    let first = AT - week - 1;
    keep_missed(&file, row(Search::Tag, first), first).expect("written where none was");
    keep_missed(&file, row(Search::Phrase, AT - week), AT - week).expect("written");
    let ending_sooner = missed(Search::Tag, Some(AT - MINUTE), Some(AT - 5), AT - 5);
    keep_missed(&file, ending_sooner, AT - 5).expect("written");
    keep_missed(&file, row(Search::Tag, AT), AT).expect("written in its place");
    keep_missed(&file, row(Search::Tag, AT), AT).expect("written again");
    let read: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).expect("read")).expect("JSON");
    let ids: Vec<_> = read["items"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|row| row["id"].clone())
        .collect();
    assert_eq!(
        ids,
        [
            format!(
                "pineapple-slack-missed-phrase-{}-{}",
                AT - week - MINUTE,
                AT - week
            ),
            format!("pineapple-slack-missed-tag-{}-{AT}", AT - MINUTE),
        ],
        "one whose span ended a week ago stays, one older goes, and the newest joins them \
         once, however often it's written, in place of one begun as it was"
    );
    assert_eq!(read["quest"], "pineapple", "in the shape For You reads");
    let _ = std::fs::remove_dir_all(&folder);
}

/// The rows `file` holds, read back.
fn rows_in(file: &Path) -> Vec<serde_json::Value> {
    let read: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(file).expect("read")).expect("JSON");
    read["items"].as_array().expect("rows").clone()
}

#[test]
fn a_busy_stretch_read_on_round_after_round_is_said_in_one_row() {
    let folder = std::env::temp_dir().join(format!(
        "pineapple-missed-meet-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
    ));
    let file = folder.join("missed.json");
    std::fs::create_dir_all(&folder).expect("the folder is made");
    assert_eq!(MEET, 10 * MINUTE, "the nearness this case is written for");
    let tag = |from: u64, to: u64| missed(Search::Tag, Some(AT - from), Some(AT - to), AT - to);
    let far = tag(100 * MINUTE, 90 * MINUTE);
    keep_missed(&file, far.clone(), AT - 90 * MINUTE).expect("written");
    let phrase = missed(
        Search::Phrase,
        Some(AT - 30 * MINUTE),
        Some(AT - 25 * MINUTE),
        AT,
    );
    keep_missed(&file, phrase.clone(), AT - 25 * MINUTE).expect("written");
    keep_missed(&file, tag(30 * MINUTE, 25 * MINUTE), AT - 25 * MINUTE).expect("written");
    keep_missed(&file, tag(26 * MINUTE, 20 * MINUTE), AT - 20 * MINUTE).expect("written");
    let overlapping = rows_in(&file);
    assert_eq!(
        (
            overlapping.len(),
            &overlapping[2]["id"],
            &overlapping[2]["whole"]
        ),
        (
            3,
            &serde_json::json!(format!(
                "pineapple-slack-missed-tag-{}-{}",
                AT - 30 * MINUTE,
                AT - 20 * MINUTE
            )),
            &serde_json::json!(true)
        ),
        "spans that overlap are one, said whole: {overlapping:?}"
    );
    keep_missed(&file, tag(15 * MINUTE, 12 * MINUTE), AT - 12 * MINUTE).expect("written");
    let rows = rows_in(&file);
    assert_eq!(
        rows.iter().map(|row| row["id"].clone()).collect::<Vec<_>>(),
        [
            far["id"].clone(),
            phrase["id"].clone(),
            serde_json::json!(format!(
                "pineapple-slack-missed-tag-{}-{}",
                AT - 30 * MINUTE,
                AT - 12 * MINUTE
            )),
        ],
        "one five minutes past joins them, while one farther off and the other search's \
         stay their own"
    );
    assert_eq!(
        rows[2]["why"],
        "The safe word never read parts of what tagged you in Slack without saying pineapple \
         in the 18 minutes before this was raised, as more was said than it could read, so a \
         stop said then wasn't heard",
        "a read stretch lies between, so it's said as parts, raised as the newest was"
    );
    assert_eq!(rows[2]["whole"], false, "and kept so as written again");
    keep_missed(&file, tag(13 * MINUTE, 10 * MINUTE), AT - 10 * MINUTE).expect("written");
    let again = rows_in(&file);
    assert!(
        again.len() == 3 && again[2]["whole"] == false,
        "a row once said as parts stays so, though the next overlaps it: {again:?}"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

/// What `kept` rows, then `row` kept at `AT`, leave in a file of their own,
/// saying as it's kept what the row it wrote says.
fn kept_after(
    name: &str,
    kept: &[serde_json::Value],
    row: serde_json::Value,
) -> Vec<serde_json::Value> {
    let folder = std::env::temp_dir().join(format!(
        "pineapple-missed-{name}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
    ));
    let file = folder.join("missed.json");
    std::fs::create_dir_all(&folder).expect("the folder is made");
    let earlier = serde_json::json!({"v": 2, "items": kept});
    std::fs::write(&file, earlier.to_string()).expect("the earlier rows are written");
    let said = keep_missed(&file, row, AT).expect("written");
    let rows = rows_in(&file);
    let _ = std::fs::remove_dir_all(&folder);
    assert_eq!(
        rows.last().map(|row| row["why"].clone()),
        Some(serde_json::json!(said)),
        "what's said as it's kept is the row written, met rows and all: {rows:?}"
    );
    rows
}

#[test]
fn spans_meet_up_to_ten_minutes_apart_and_an_unknown_start_meets_only_itself() {
    let tag = |from: u64, to: u64| missed(Search::Tag, Some(AT - from), Some(AT - to), AT - to);
    let earlier = tag(30 * MINUTE, 20 * MINUTE);
    let met = kept_after(
        "edge",
        slice::from_ref(&earlier),
        tag(10 * MINUTE, 5 * MINUTE),
    );
    assert!(
        met.len() == 1 && met[0]["whole"] == false,
        "exactly ten minutes apart they meet, as parts: {met:?}"
    );
    let apart = kept_after(
        "past",
        slice::from_ref(&earlier),
        tag(10 * MINUTE - 1, 5 * MINUTE),
    );
    assert_eq!(
        apart.len(),
        2,
        "a millisecond farther they don't: {apart:?}"
    );
    let touching = kept_after(
        "touch",
        slice::from_ref(&earlier),
        tag(20 * MINUTE, 10 * MINUTE),
    );
    assert!(
        touching.len() == 1 && touching[0]["whole"] == true,
        "one beginning where the other ends is said whole: {touching:?}"
    );
    let mut legacy = earlier.clone();
    legacy.as_object_mut().expect("a row").remove("whole");
    let old = kept_after("legacy", &[legacy], tag(25 * MINUTE, 10 * MINUTE));
    assert!(
        old.len() == 1 && old[0]["whole"] == true,
        "a row written before rows met is taken as whole: {old:?}"
    );

    let days = 3 * 24 * 60 * MINUTE;
    let dated = tag(days + 10 * MINUTE, days);
    let unknown = missed(Search::Tag, None, Some(AT - MINUTE), AT);
    let both = kept_after("unknown", slice::from_ref(&dated), unknown.clone());
    assert_eq!(
        both.iter().map(|row| row["id"].clone()).collect::<Vec<_>>(),
        [dated["id"].clone(), unknown["id"].clone()],
        "one whose start can't be told takes in no dated row"
    );
    let again = kept_after("unknown-again", slice::from_ref(&unknown), unknown.clone());
    assert_eq!(again.len(), 1, "though it's the same row written again");
    let after = kept_after(
        "after-unknown",
        &[missed(
            Search::Tag,
            None,
            Some(AT - 20 * MINUTE),
            AT - 20 * MINUTE,
        )],
        tag(15 * MINUTE, 10 * MINUTE),
    );
    assert_eq!(
        after.len(),
        2,
        "nor does a dated one take in a row whose start can't be told: {after:?}"
    );
}

#[test]
fn rows_written_before_rows_met_are_folded_into_one() {
    let tag = |from: u64, to: u64| missed(Search::Tag, Some(AT - from), Some(AT - to), AT - to);
    let rows = kept_after(
        "chain",
        &[
            tag(60 * MINUTE, 45 * MINUTE),
            tag(46 * MINUTE, 30 * MINUTE),
            tag(31 * MINUTE, 15 * MINUTE),
        ],
        tag(16 * MINUTE, 0),
    );
    assert_eq!(
        rows.iter().map(|row| row["id"].clone()).collect::<Vec<_>>(),
        [serde_json::json!(format!(
            "pineapple-slack-missed-tag-{}-{AT}",
            AT - 60 * MINUTE
        ))],
        "the first two meet only once those after them are taken, and are taken too: \
         {rows:?}"
    );
    assert_eq!(
        rows[0]["whole"], true,
        "each overlapping the next, it's whole"
    );
}

#[test]
fn rows_met_are_whole_by_what_they_span_not_the_order_theyre_met_in() {
    let tag = |from: u64, to: u64| missed(Search::Tag, Some(AT - from), Some(AT - to), AT - to);
    let rows = kept_after(
        "order",
        &[tag(30 * MINUTE, 20 * MINUTE), tag(21 * MINUTE, 9 * MINUTE)],
        tag(10 * MINUTE, 0),
    );
    assert!(
        rows.len() == 1 && rows[0]["whole"] == true,
        "the oldest is met first, near but not touching the newest, yet the one between \
         bridges them, so all of it is whole: {rows:?}"
    );
    let gap = kept_after(
        "order-gap",
        &[tag(30 * MINUTE, 22 * MINUTE), tag(21 * MINUTE, 9 * MINUTE)],
        tag(10 * MINUTE, 0),
    );
    assert!(
        gap.len() == 1 && gap[0]["whole"] == false,
        "a minute read between any two makes it parts: {gap:?}"
    );
    let nested = kept_after(
        "nested",
        &[tag(30 * MINUTE, 10 * MINUTE), tag(28 * MINUTE, 25 * MINUTE)],
        tag(12 * MINUTE, 0),
    );
    assert!(
        nested.len() == 1 && nested[0]["whole"] == true,
        "one inside another leaves the furthest reached where it was, so what follows \
         the outer one joins it: {nested:?}"
    );
    let starts = kept_after(
        "starts",
        &[tag(40 * MINUTE, 35 * MINUTE), tag(20 * MINUTE, 15 * MINUTE)],
        tag(36 * MINUTE, 0),
    );
    assert!(
        starts.len() == 1 && starts[0]["whole"] == true,
        "spans are taken by where they start, not where they end, so a long one \
         covers the short one ending before it: {starts:?}"
    );
}

#[test]
fn a_row_meets_one_said_after_it_only_within_ten_minutes_and_unknowns_only_their_own() {
    let tag = |from: u64, to: u64| missed(Search::Tag, Some(AT - from), Some(AT - to), AT - to);
    let later = kept_after(
        "later",
        &[tag(9 * MINUTE, 5 * MINUTE)],
        tag(30 * MINUTE, 20 * MINUTE),
    );
    assert_eq!(
        later.len(),
        2,
        "a row kept that began more than ten minutes after this one ends stays its own: \
         {later:?}"
    );
    let edge = kept_after(
        "later-edge",
        &[tag(10 * MINUTE, 5 * MINUTE)],
        tag(30 * MINUTE, 20 * MINUTE),
    );
    assert_eq!(edge.len(), 1, "one ten minutes after meets: {edge:?}");
    let unknowns = kept_after(
        "unknowns",
        &[missed(
            Search::Tag,
            None,
            Some(AT - 2 * MINUTE),
            AT - 2 * MINUTE,
        )],
        missed(Search::Tag, None, Some(AT - MINUTE), AT),
    );
    assert_eq!(
        unknowns.len(),
        2,
        "two whose starts can't be told, ending even a minute apart, stay their own: \
         {unknowns:?}"
    );
}

#[test]
fn rows_of_the_phrase_search_met_are_said_as_its_own() {
    let phrase =
        |from: u64, to: u64| missed(Search::Phrase, Some(AT - from), Some(AT - to), AT - to);
    let rows = kept_after(
        "phrase-met",
        &[phrase(30 * MINUTE, 20 * MINUTE)],
        phrase(25 * MINUTE, 10 * MINUTE),
    );
    assert_eq!(
        rows.iter().map(|row| row["id"].clone()).collect::<Vec<_>>(),
        [serde_json::json!(format!(
            "pineapple-slack-missed-phrase-{}-{}",
            AT - 30 * MINUTE,
            AT - 10 * MINUTE
        ))],
        "two of its spans that overlap are one row, named for its search: {rows:?}"
    );
    assert!(
        rows[0]["why"]
            .as_str()
            .is_some_and(|why| why.contains("said pineapple in Slack without tagging you")),
        "and said as its search's: {rows:?}"
    );
}
