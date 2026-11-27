//! What kept a run from routing as it should, each said as a trouble so a
//! broken listener never reads as a quiet hour, how long the node has
//! named no sessions or no hubs, each a trouble only once it has gone on,
//! and needs that couldn't be kept, which stay a trouble for the hour.

use super::route::Routes;
use super::{HOUR_MS, Hour, SEND_TRIES};
use serde::{Deserialize, Serialize};
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
    outage_since(state, at, cellless, |before| before.cellless_since)
}

/// Since when the node couldn't be asked for its hubs, carried as the
/// count of runs without cells is, so a node that blinks for a run as it
/// restarts fails nothing (#6774's round, F15 and F8): the pains a hub
/// would take wait for the next run, and past [`CELLLESS_AT_MOST_MS`] go
/// to the fallback, as [`hubless`] then says.
pub fn hubless_since(state: &Path, at: i64, hubless: bool) -> Option<i64> {
    outage_since(state, at, hubless, |before| before.hubless_since)
}

/// Since when an outage `of` the run before counts has gone on, carried
/// from that run when it was recent, or from now when this run is the
/// first out; none when this run isn't `out`.
fn outage_since(state: &Path, at: i64, out: bool, of: fn(&Hour) -> Option<i64>) -> Option<i64> {
    let before: Option<Hour> = fs::read(state.join("hour.json"))
        .ok()
        .and_then(|text| serde_json::from_slice(&text).ok())
        // A run dated ahead of this one, its clock wrong or its state
        // copied in, carries nothing on, so it can't hold an outage back.
        .filter(|before: &Hour| (at - CELLLESS_GAP_MS..=at).contains(&before.at));
    out.then(|| {
        before
            .as_ref()
            .and_then(of)
            .filter(|since| *since <= at)
            .unwrap_or(at)
    })
}

/// Needs put to hubs that a run couldn't keep, and why: the judge reading
/// needs never sees them, so the pains they carried weigh on no holder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unkept {
    /// When the run that couldn't keep them ran.
    pub at: i64,
    pub why: String,
}

impl Unkept {
    /// As the hour's troubles say it.
    #[must_use]
    pub fn trouble(&self) -> String {
        format!("the needs put to hubs couldn't be kept: {}", self.why)
    }
}

/// The needs this run couldn't keep, as `why`, or else those the run
/// before carries from within the hour. Cadence's card reads only the
/// last run's troubles once an hour, so a trouble said by the one
/// five-minute run that lost them would almost never reach it (#6774's
/// round, F22). One dated ahead of this run carries nothing, so a wrong
/// clock can't hold it past its hour.
pub fn unkept(state: &Path, at: i64, why: Option<String>) -> Option<Unkept> {
    if let Some(why) = why {
        return Some(Unkept { at, why });
    }
    within_the_hour(state, at, |before| before.unkept, |before| before.at)
}

/// Pains a run sent to the fallback because the node named no hubs past
/// its blink, and why it named none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hubless {
    /// When the run that sent them ran.
    pub at: i64,
    /// How many it sent there.
    pub sent: usize,
    pub why: String,
}

impl Hubless {
    /// As the hour's troubles say it.
    #[must_use]
    pub fn trouble(&self) -> String {
        let pains = match self.sent {
            1 => "a pain no cell of its own takes".to_owned(),
            sent => format!("{sent} pains no cell of their own takes"),
        };
        format!(
            "the mycelium named no hubs, so {pains} went to the fallback: {}",
            self.why
        )
    }
}

/// The pains this run sent to the fallback for want of hubs, `sent` of
/// them as the node couldn't be asked, `why`, or else those the run
/// before carries from within the hour, as unkept needs are carried. A
/// node out of reach costs nothing while no pain needs a hub, so it's a
/// trouble only once one went to the fallback for it (#6774's round, F1
/// and F8).
pub fn hubless(state: &Path, at: i64, sent: usize, why: Option<&str>) -> Option<Hubless> {
    if let Some(why) = why.filter(|_| sent > 0) {
        return Some(Hubless {
            at,
            sent,
            why: why.to_owned(),
        });
    }
    within_the_hour(state, at, |before| before.hubless, |before| before.at)
}

/// What `of` the run before carries, while `when` it ran is within the
/// hour before `at`. One dated ahead of this run carries nothing.
fn within_the_hour<T>(
    state: &Path,
    at: i64,
    of: fn(Hour) -> Option<T>,
    when: fn(&T) -> i64,
) -> Option<T> {
    fs::read(state.join("hour.json"))
        .ok()
        .and_then(|text| serde_json::from_slice::<Hour>(&text).ok())
        .and_then(of)
        .filter(|before| when(before) <= at && at - when(before) < HOUR_MS)
}

/// What kept a run from routing as it should: sends that failed this run,
/// or wait as its time for sends ran out or the mycelium couldn't be
/// started, pains given up on, and a node that has named no sessions for
/// long.
pub fn troubles(
    state: &Path,
    routes: &Routes,
    (failed, out_of_time, unstarted, given_up): (usize, usize, usize, usize),
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
    if unstarted > 0 {
        troubles.push(format!(
            "{unstarted} sends wait for the next run, as the mycelium couldn't be started"
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
