//! A candidate whose gate said nothing of its riders, because no builder
//! took it or it never started: the train tries again with its riders
//! aboard, until it has tried long enough that they fall back to running
//! again alone, told why.

use std::path::Path;
use std::time::Duration;

use super::fall;
use super::gated::Gated;
use super::splitting::{Splitting, UNTAKEN_FOR};
use crate::state::short;

/// How long the train waits after a candidate no builder took before it
/// cuts the next.
const UNTAKEN_PAUSE: Duration = Duration::from_mins(1);

/// After the gate said nothing of the riders `merged` into `candidate`,
/// as `gated` answered: tell them, and while they stay aboard, wait
/// before the train cuts the next candidate.
pub(super) fn wait_untaken(
    root: &Path,
    gated: &Gated,
    candidate: &str,
    log: &Path,
    merged: &[String],
    splitting: &mut Splitting,
) {
    let untaken = Untaken {
        candidate,
        log,
        unstarted: match gated {
            Gated::Unstarted(why) => Some(why.as_str()),
            _ => None,
        },
    };
    if unsent(root, &untaken, merged, splitting, crate::state::now()) {
        std::thread::sleep(UNTAKEN_PAUSE);
    }
}

/// A candidate whose gate said nothing of its riders: no builder took
/// it, or, as `unstarted` says, it never started.
pub(super) struct Untaken<'a> {
    pub(super) candidate: &'a str,
    pub(super) log: &'a Path,
    pub(super) unstarted: Option<&'a str>,
}

/// After no builder took the candidate `untaken` of `merged` riders, or
/// its gate never started, `now`: whether the train tries again, its
/// riders aboard, or the riders of that candidate fall back to running
/// again alone, as `splitting` says. Riders that boarded since rode none
/// of the untaken candidates, so they stay aboard and start afresh. A
/// gate that never started is said as the train's own fault, since its
/// log may never have been made and no builder was asked (round 9's F2,
/// F3, F6 and F8).
pub(super) fn unsent(
    root: &Path,
    untaken: &Untaken<'_>,
    merged: &[String],
    splitting: &mut Splitting,
    now: u64,
) -> bool {
    let candidate = short(untaken.candidate);
    let log = untaken.log.display();
    if splitting.untaken(now) {
        match untaken.unstarted {
            Some(cause) => println!(
                "merge-net: the gate of the candidate {candidate} couldn't start, as {cause}, so \
                 the train tries again with its riders aboard"
            ),
            None => println!(
                "merge-net: no builder took the candidate {candidate}, its words in {log}, so the \
                 train tries again with its riders aboard"
            ),
        }
        return true;
    }
    let minutes = UNTAKEN_FOR / 60;
    let why = match untaken.unstarted {
        Some(cause) => format!(
            "the train's gate couldn't start for {minutes} minutes, the last for {candidate} as \
             {cause}, so no builder was asked and nothing of its riders was tested"
        ),
        None => format!(
            "no builder or evaluator took the train's candidates for {minutes} minutes, the last \
             {candidate}; its words are in {log}"
        ),
    };
    println!("merge-net: {why}, so the riders of that candidate fall");
    for head in merged {
        let _ = fall(root, head, &why);
    }
    false
}
