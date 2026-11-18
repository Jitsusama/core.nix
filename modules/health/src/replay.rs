//! The day read again in the order it was written. Each journal follower
//! is its own journalctl, and on a start each would replay the day from
//! midnight at its own pace: the follower of reloads, about two hundred
//! lines a day, ran ahead of the one of services stopping and started,
//! about sixty thousand, so a reload late in the day was heard before the
//! morning's stops. Each follower's history is read first, without
//! following, and the histories are merged in the order journald wrote
//! their lines; then each follows on from its last line (`source.rs`).

use serde::Deserialize;

/// What the merge reads of a line: where journald numbered it, when it
/// was written, and where in the journal it sits, to follow on from.
#[derive(Deserialize)]
struct Stamp {
    #[serde(rename = "__SEQNUM_ID")]
    counter: Option<String>,
    #[serde(rename = "__SEQNUM")]
    number: Option<String>,
    #[serde(rename = "__REALTIME_TIMESTAMP")]
    at: Option<String>,
    #[serde(rename = "__CURSOR")]
    cursor: Option<String>,
}

/// Send each of `histories`' lines through `send` in the order journald
/// wrote them, a line of an earlier history first where two can't be told
/// apart, until `send` says nothing hears them. Each history is read only
/// as far as the merge needs, so the merge holds one line of each; what
/// `send` takes is its own to hold. Says where each history ended, as its
/// last cursor, or none for a history with none.
///
/// The merge holds each history in journald's order, as journalctl writes
/// it. While every history's next line is numbered by one counter, they're
/// ordered by their numbers, which a clock stepped back can't reorder: on
/// 17 and 18 November every followed line shared one counter, through a
/// reboot. Otherwise all are ordered by when they were written, so one
/// rule decides each pick and the history a line sits in never does.
pub fn merged<I>(histories: Vec<I>, mut send: impl FnMut(String) -> bool) -> Vec<Option<String>>
where
    I: Iterator<Item = String>,
{
    let mut histories = histories;
    let mut ends = vec![None; histories.len()];
    let mut heads: Vec<Option<Head>> = histories.iter_mut().map(head).collect();
    while let Some(at) = earliest(&heads) {
        let (Some(slot), Some(history), Some(end)) =
            (heads.get_mut(at), histories.get_mut(at), ends.get_mut(at))
        else {
            break;
        };
        let Some(Head { cursor, line, .. }) = slot.take() else {
            break;
        };
        // A line with no cursor can't be followed on from, so the end
        // stays the last line that has one.
        if cursor.is_some() {
            *end = cursor;
        }
        if !send(line) {
            break;
        }
        *slot = head(history);
    }
    ends
}

/// Which of `heads` was written first, the earliest history's of those
/// that can't be told apart.
fn earliest(heads: &[Option<Head>]) -> Option<usize> {
    let mut counters = heads
        .iter()
        .flatten()
        .map(|head| head.numbered.as_ref().map(|(counter, _)| counter));
    let first_counter = counters.next().flatten();
    let one_counter = first_counter.is_some() && counters.all(|counter| counter == first_counter);
    let key = |head: &Head| match &head.numbered {
        Some((_, number)) if one_counter => *number,
        _ => head.at,
    };
    heads
        .iter()
        .enumerate()
        .filter_map(|(at, head)| head.as_ref().map(|head| (key(head), at)))
        .min()
        .map(|(_, at)| at)
}

/// A history's next line, read for the merge.
struct Head {
    /// The counter journald numbered it by, and its number.
    numbered: Option<(String, u64)>,
    /// When it was written, in microseconds; a line no time is read of
    /// counts as the earliest, so it's heard at once.
    at: u64,
    /// Where it sits in the journal.
    cursor: Option<String>,
    /// The line itself.
    line: String,
}

/// The next line of `history`, or none once it has ended.
fn head(history: &mut impl Iterator<Item = String>) -> Option<Head> {
    let line = history.next()?;
    let Stamp {
        counter,
        number,
        at,
        cursor,
    } = stamp(&line);
    Some(Head {
        numbered: counter.zip(number.and_then(|number| number.parse().ok())),
        at: at.and_then(|at| at.parse().ok()).unwrap_or_default(),
        cursor,
        line,
    })
}

/// What the merge reads of `line`, or nothing of a line that isn't JSON.
fn stamp(line: &str) -> Stamp {
    serde_json::from_str(line).unwrap_or(Stamp {
        counter: None,
        number: None,
        at: None,
        cursor: None,
    })
}

#[cfg(test)]
mod tests;
