use std::cell::Cell;

use super::*;

/// A line written at `at`, sitting at `cursor`, with journald's number
/// left out, so the merge orders it by time.
fn line(at: u64, cursor: &str) -> String {
    format!(r#"{{"__CURSOR":"{cursor}","__REALTIME_TIMESTAMP":"{at}","MESSAGE":"m"}}"#)
}

/// A line journald numbered `number` by counter `s`, written at `at`.
fn numbered(number: u64, at: u64, cursor: &str) -> String {
    counted("s", number, at, cursor)
}

/// A line journald numbered `number` by `counter`, written at `at`.
fn counted(counter: &str, number: u64, at: u64, cursor: &str) -> String {
    format!(
        r#"{{"__CURSOR":"{cursor}","__SEQNUM_ID":"{counter}","__SEQNUM":"{number}","__REALTIME_TIMESTAMP":"{at}","MESSAGE":"m"}}"#
    )
}

fn cursors(histories: Vec<Vec<String>>) -> (Vec<String>, Vec<Option<String>>) {
    let mut heard = Vec::new();
    let ends = merged(
        histories.into_iter().map(Vec::into_iter).collect(),
        |line| {
            heard.push(stamp(&line).cursor.unwrap_or_else(|| line.clone()));
            true
        },
    );
    (heard, ends)
}

#[test]
fn the_histories_are_heard_in_the_order_they_were_written() {
    let (heard, ends) = cursors(vec![
        vec![line(9, "r9")],
        vec![line(1, "u1"), line(5, "u5"), line(10, "u10")],
        vec![line(5, "k5")],
        Vec::new(),
    ]);
    assert_eq!(
        heard,
        ["u1", "u5", "k5", "r9", "u10"],
        "a history ran ahead of another, or two written at once lost their order"
    );
    assert_eq!(
        ends,
        [
            Some("r9".to_owned()),
            Some("u10".to_owned()),
            Some("k5".to_owned()),
            None
        ],
        "a history's end wasn't its last line"
    );
}

#[test]
fn journalds_numbers_order_the_day_through_a_clock_stepped_back() {
    // The clock stepped back between the first line and the third, so the
    // times disagree with the order journald wrote them in.
    let (heard, _) = cursors(vec![
        vec![numbered(1, 500, "a1"), numbered(3, 300, "a3")],
        vec![numbered(2, 400, "b2")],
    ]);
    assert_eq!(
        heard,
        ["a1", "b2", "a3"],
        "the merge followed the clock rather than journald's numbers"
    );
}

#[test]
fn lines_of_two_counters_are_heard_by_when_they_were_written() {
    // A new counter starts its numbers low, so numbers can't order it
    // against the old one's.
    let (heard, _) = cursors(vec![
        vec![counted("s", 900, 100, "s900")],
        vec![counted("t", 1, 200, "t1")],
    ]);
    assert_eq!(heard, ["s900", "t1"], "two counters' numbers were compared");
}

#[test]
fn whichever_history_a_line_sits_in_the_same_line_is_heard_first() {
    // Numbered and unnumbered heads at once, the clock disagreeing with
    // the numbers: whatever the histories' order, one rule picks.
    let heads = [numbered(1, 500, "a"), line(400, "b"), numbered(2, 300, "c")];
    let first = |order: [usize; 3]| {
        cursors(order.iter().map(|at| vec![heads[*at].clone()]).collect())
            .0
            .first()
            .cloned()
    };
    let firsts: Vec<_> = [[0, 1, 2], [2, 1, 0], [1, 0, 2], [1, 2, 0]]
        .into_iter()
        .map(first)
        .collect();
    assert!(
        firsts.iter().all(|heard| heard.as_deref() == Some("c")),
        "the history a line sat in chose what was heard first: {firsts:?}"
    );
}

#[test]
fn the_merge_stops_once_nothing_hears_it() {
    let mut heard = 0;
    merged(vec![vec![line(1, "a"), line(2, "b")].into_iter()], |_| {
        heard += 1;
        false
    });
    assert_eq!(heard, 1, "lines were sent after nothing heard them");
}

#[test]
fn each_history_is_read_only_as_far_as_the_merge_needs() {
    // Two long histories, written in turn, each ending after a thousand
    // lines, so a merge that read one whole fails here rather than holding
    // the build.
    fn endless(read: &Cell<u64>, offset: u64) -> impl Iterator<Item = String> + '_ {
        std::iter::from_fn(move || {
            read.set(read.get() + 1);
            (read.get() <= 1000).then(|| line(read.get() * 2 + offset, "c"))
        })
    }
    let read = [Cell::new(0_u64), Cell::new(0_u64)];
    let mut heard = 0;
    merged(vec![endless(&read[0], 0), endless(&read[1], 1)], |_| {
        heard += 1;
        heard < 10
    });
    let read: Vec<u64> = read.iter().map(Cell::get).collect();
    assert!(
        read.iter().all(|read| *read <= 6),
        "a history was read further than the ten lines heard needed: {read:?}"
    );
}

#[test]
fn a_line_with_no_time_is_heard_at_once() {
    let (heard, _) = cursors(vec![
        vec![line(1, "a")],
        vec!["not json".to_owned(), line(2, "b")],
    ]);
    assert_eq!(
        heard.first().map(String::as_str),
        Some("not json"),
        "a line no time could be read of was held back"
    );
}

#[test]
fn a_history_ending_in_a_line_with_no_cursor_ends_at_its_last_cursor() {
    let (_, ends) = cursors(vec![vec![line(1, "a"), "not json".to_owned()]]);
    assert_eq!(
        ends,
        [Some("a".to_owned())],
        "a history's follower would start the day again"
    );
}
