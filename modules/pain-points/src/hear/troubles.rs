//! What kept a run from routing as it should, each said as a trouble so a
//! broken listener never reads as a quiet hour, and how long the node has
//! named no sessions, which is a trouble only once it has gone on.

use super::route::Routes;
use super::{Hour, SEND_TRIES};
use std::fs;
use std::path::Path;

/// How long the node may name no sessions while pains wait before a run
/// says so as a trouble: past the third run after the first, so a node
/// that blinks as the machine goes down fails nothing, as the next run
/// sends what waited, and one that stays broken still fails the unit. It
/// sits off the five-minute beat, so a run started late still trips it.
pub const CELLLESS_AT_MOST_MS: i64 = 14 * 60_000;

/// How old the run before may be for its count of runs without cells to
/// carry on: past two runs' gap and short of three, off the beat, so a
/// late run carries it on and a machine that was off starts it again.
const CELLLESS_GAP_MS: i64 = 12 * 60_000;

/// Since when the node has named no sessions while pains waited, carried
/// from the run before when it was recent, or from now when this run is
/// the first without cells; none when this run has cells or nothing waits.
pub fn cellless_since(state: &Path, at: i64, cellless: bool) -> Option<i64> {
    let before: Option<Hour> = fs::read(state.join("hour.json"))
        .ok()
        .and_then(|text| serde_json::from_slice(&text).ok())
        // A run dated ahead of this one, its clock wrong or its state
        // copied in, carries nothing on, so it can't hold an outage back.
        .filter(|before: &Hour| (at - CELLLESS_GAP_MS..=at).contains(&before.at));
    cellless.then(|| {
        before
            .and_then(|b| b.cellless_since)
            .filter(|since| *since <= at)
            .unwrap_or(at)
    })
}

/// What kept a run from routing as it should: sends that failed this run,
/// or wait as its time for sends ran out, pains given up on, and a node
/// that has named no sessions for long.
pub fn troubles(
    state: &Path,
    routes: &Routes,
    (failed, out_of_time, given_up): (usize, usize, usize),
    no_cells: bool,
) -> Vec<String> {
    let mut troubles: Vec<String> = Routes::trouble(state).into_iter().collect();
    troubles.extend(refused(routes));
    if failed > 0 {
        troubles.push(format!("{failed} sends failed"));
    }
    if out_of_time > 0 {
        troubles.push(format!(
            "{out_of_time} sends wait for the next run, as its time for sends ran out"
        ));
    }
    if given_up > 0 {
        troubles.push(format!(
            "gave up on {given_up} pains the node never took, after {SEND_TRIES} sends each"
        ));
    }
    if routes.sends() && no_cells {
        troubles
            .push("the mycelium named no sessions for a quarter hour, so none were routed".into());
    }
    troubles
}

/// What the routes say that keeps a run from sending as it should: no
/// cell named never to send to, or a fallback or keeper that is one.
fn refused(routes: &Routes) -> Vec<String> {
    let mut troubles = Vec::new();
    if routes.from.is_some() && routes.never.is_empty() {
        troubles.push("routes.json names no cell never to send to, so nothing was sent".into());
    }
    for (what, to) in [("fallback", &routes.fallback), ("keeper", &routes.ledgers)] {
        if let Some(to) = to.as_ref().filter(|to| routes.refuses(to)) {
            troubles.push(format!(
                "routes.json's {what} {to} is one never to send to, so what goes to it waits"
            ));
        }
    }
    troubles
}
