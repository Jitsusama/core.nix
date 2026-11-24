//! The hour as cadence's card counts it, from every pain kept.

use super::heard::Heard;
use super::{DAY_MS, HOUR_MS, Hour, stalled};

/// The hour as cadence's card says it. A pain said in the void counts in
/// none: no cell was asked of it. One not yet fixed counts however old,
/// as only its fix closes it; one past a day that nobody took up, whose
/// question's thread was never seen, counts as lost rather than open.
pub(super) fn hour_of(kept: &[Heard], at: i64) -> Hour {
    let in_hour = |t: i64| t >= at - HOUR_MS;
    let said: Vec<&Heard> = kept.iter().filter(|h| !stalled::is(h)).collect();
    let standing: Vec<&Heard> = said
        .iter()
        .copied()
        .filter(|h| !h.carried && !h.void && (h.at >= at - DAY_MS || h.closed_by.is_none()))
        .collect();
    let lost = |h: &&&Heard| {
        h.at < at - DAY_MS && h.closed_by.is_none() && h.taken_by.is_none() && h.thread.is_empty()
    };
    let open = |h: &&&Heard| h.closed_by.is_none() && h.taken_by.is_none() && !lost(h);
    let worst = standing.iter().filter(open).max_by_key(|h| (h.hits, h.at));
    Hour {
        at,
        heard: said.iter().filter(|h| in_hour(h.at)).count(),
        stalled: kept
            .iter()
            .filter(|h| stalled::is(h) && in_hour(h.at))
            .count(),
        routed: kept
            .iter()
            .filter(|h| h.routed_at.is_some_and(in_hour))
            .count(),
        open: standing.iter().filter(open).count(),
        lost: standing.iter().filter(lost).count(),
        worst: worst.map(|h| h.words.clone()),
        worst_hits: worst.map_or(0, |h| h.hits),
        taken: standing
            .iter()
            .filter(|h| h.closed_by.is_none() && h.taken_by.is_some())
            .count(),
        closed: standing
            .iter()
            .filter(|h| h.at >= at - DAY_MS && h.closed_by.is_some())
            .count(),
        troubles: Vec::new(),
        cellless_since: None,
        unclosed: None,
    }
}
