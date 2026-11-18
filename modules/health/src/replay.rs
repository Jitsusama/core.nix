//! The day read again in the order it was written. Each journal follower
//! is its own journalctl, and on a start each would replay the day from
//! midnight at its own pace: the follower of reloads, about two hundred
//! lines a day, ran ahead of the one of services stopping and started,
//! about sixty thousand, so a reload late in the day was heard before the
//! morning's stops. Each follower's history is read first, without
//! following, and the histories are merged by when each line was written;
//! then each follows on from its last line.

use std::iter::Peekable;

use serde::Deserialize;

/// What the merge reads of a line: when it was written, and where in the
/// journal it sits, to follow on from.
#[derive(Deserialize)]
struct Stamp {
    #[serde(rename = "__REALTIME_TIMESTAMP")]
    at: Option<String>,
    #[serde(rename = "__CURSOR")]
    cursor: Option<String>,
}

/// Send each of `histories`' lines through `send` in the order they were
/// written, a line of an earlier history first where two were written at
/// once, until `send` says nothing hears them. Each history is read only
/// as far as the merge needs, so the day is never held whole. Says where
/// each history ended, as its last line's cursor, or none for a history
/// with no lines.
pub fn merged<I>(histories: Vec<I>, mut send: impl FnMut(String) -> bool) -> Vec<Option<String>>
where
    I: Iterator<Item = String>,
{
    let mut ends = vec![None; histories.len()];
    let mut histories: Vec<Peekable<I>> = histories.into_iter().map(Iterator::peekable).collect();
    for (history, end) in histories.iter_mut().zip(&mut ends) {
        for line in history.by_ref() {
            *end = stamp(&line).cursor.or(end.take());
            if !send(line) {
                return ends;
            }
        }
    }
    ends
}

/// What `line` says of when it was written and where it sits.
fn stamp(line: &str) -> Stamp {
    serde_json::from_str(line).unwrap_or(Stamp {
        at: None,
        cursor: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(at: u64, cursor: &str) -> String {
        format!(r#"{{"__CURSOR":"{cursor}","__REALTIME_TIMESTAMP":"{at}","MESSAGE":"m"}}"#)
    }

    #[test]
    fn the_histories_are_heard_in_the_order_they_were_written() {
        let reloads = vec![line(9, "r9")];
        let units = vec![line(1, "u1"), line(5, "u5"), line(10, "u10")];
        let kernel = vec![line(5, "k5")];
        let mut heard = Vec::new();
        let ends = merged(
            vec![
                reloads.into_iter(),
                units.into_iter(),
                kernel.into_iter(),
                Vec::new().into_iter(),
            ],
            |line| {
                heard.push(stamp(&line).cursor.unwrap_or_default());
                true
            },
        );
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
    fn the_merge_stops_once_nothing_hears_it() {
        let mut heard = 0;
        merged(vec![vec![line(1, "a"), line(2, "b")].into_iter()], |_| {
            heard += 1;
            false
        });
        assert_eq!(heard, 1, "lines were sent after nothing heard them");
    }

    #[test]
    fn a_line_with_no_time_is_heard_at_once() {
        let mut heard = Vec::new();
        merged(
            vec![
                vec![line(1, "a")].into_iter(),
                vec!["not json".to_owned(), line(2, "b")].into_iter(),
            ],
            |line| {
                heard.push(line);
                true
            },
        );
        assert_eq!(
            heard[0], "not json",
            "a line no time could be read of was held back"
        );
    }
}
