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

/// Where journald numbered a line: the counter, and its number by it.
pub type Numbered = (String, u64);

/// Send each of `histories`' lines through `send` in the order journald
/// wrote them, a line of an earlier history first where two can't be told
/// apart, until `send` says nothing hears them. Each history is read only
/// as far as the merge needs, so the merge holds one line of each, and
/// is dropped as soon as it's read no further; what `send` takes is its
/// own to hold. Says where each history ended, as its last cursor, or
/// none for a history with none.
///
/// A history ends at its first line numbered past `last`, the journal's
/// last line as the replay began, by the same counter: it was written
/// since, and its follower hears it from the line before, as each hears
/// every line after. It's cut as that line is read, not once it would be
/// heard: by one counter it would be heard after every other history's
/// lines, and its journalctl would wait meanwhile. So the lines heard of
/// every history end at one moment, however many are written meanwhile,
/// and a clock stepped back can't move it, as it moves a time
/// (`journal::reading`). A history is cut only at a line it gives,
/// though: one asking `--grep` reads on through lines it doesn't match
/// until it finds one past `last` or reaches the journal's end, and the
/// merge waits on it meanwhile.
/// A line of another counter, or none, is heard whatever `last` is.
///
/// The merge holds each history in journald's order, as journalctl writes
/// it. While every history's next line is numbered by one counter, they're
/// ordered by their numbers, which a clock stepped back can't reorder: on
/// 17 and 18 November every followed line shared one counter, through a
/// reboot. Otherwise all are ordered by when they were written, so one
/// rule decides each pick and the history a line sits in never does.
pub fn merged<I>(
    histories: Vec<I>,
    last: Option<&Numbered>,
    mut send: impl FnMut(String) -> bool,
) -> Vec<Option<String>>
where
    I: Iterator<Item = String>,
{
    let mut histories: Vec<Option<I>> = histories.into_iter().map(Some).collect();
    let mut ends = vec![None; histories.len()];
    let mut heads: Vec<Option<Head>> = histories
        .iter_mut()
        .map(|history| next_of(history, last))
        .collect();
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
        *slot = next_of(history, last);
    }
    ends
}

/// The next line of `history`, or none once it has ended or reached a
/// line written after `last`. Then it's read no further, so it's dropped
/// at once, ending its journalctl rather than leaving it blocked on its
/// pipe until every other history has ended.
fn next_of<I: Iterator<Item = String>>(
    history: &mut Option<I>,
    last: Option<&Numbered>,
) -> Option<Head> {
    let next = history
        .as_mut()
        .and_then(head)
        .filter(|next| !written_after(next.numbered.as_ref(), last));
    if next.is_none() {
        *history = None;
    }
    next
}

/// Whether a line journald `numbered` was written after `last`, by the
/// same counter.
fn written_after(numbered: Option<&Numbered>, last: Option<&Numbered>) -> bool {
    matches!(
        (numbered, last),
        (Some((counter, number)), Some((last_counter, last_number)))
            if counter == last_counter && number > last_number
    )
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
    numbered: Option<Numbered>,
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
        numbered: number_of(counter, number),
        at: at.and_then(|at| at.parse().ok()).unwrap_or_default(),
        cursor,
        line,
    })
}

/// Where journald numbered `line`, or nothing of a line it didn't.
pub fn numbering(line: &str) -> Option<Numbered> {
    let Stamp {
        counter, number, ..
    } = stamp(line);
    number_of(counter, number)
}

fn number_of(counter: Option<String>, number: Option<String>) -> Option<Numbered> {
    counter.zip(number.and_then(|number| number.parse().ok()))
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
