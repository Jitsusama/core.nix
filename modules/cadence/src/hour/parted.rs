//! What a core change and main have each changed since they parted, asked
//! of cadence's own clone of core, so the hour can tell a date waiting on
//! the change's merge that main has moved files beneath.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use super::measures::Measure;
use super::measures::at_risk::{REFUSALS, Shared, at_risk, drifted};
use super::moved::{MAIN, git};
use crate::tracking::Tracking;

/// Joel's dates at risk and the list's own, from the asks list `asks` at
/// `now`: a core change asked of cadence's clone `core` once however many
/// pieces wait on it, and a branch of the zone's gate's record under
/// `state_home`.
pub(super) fn dates_at_risk(state_home: &Path, core: &Path, asks: &str, now: i64) -> [Measure; 2] {
    let refusals = fs::read_to_string(state_home.join(REFUSALS)).ok();
    let asked: RefCell<BTreeMap<u32, Shared>> = RefCell::new(BTreeMap::new());
    let of_core = |number: u32| {
        let known = asked.borrow().get(&number).cloned();
        known.unwrap_or_else(|| {
            let read = shared(core, crate::CORE, number);
            asked.borrow_mut().insert(number, read.clone());
            read
        })
    };
    let of_world = |branch: &str| drifted(refusals.as_deref(), branch, now);
    [Tracking::His, Tracking::Derived].map(|which| at_risk(asks, now, which, &of_core, &of_world))
}

/// Where main is once [`MAIN`] fetches it.
const MAIN_HERE: &str = "refs/remotes/cadence/main";

/// The files core change `number` touches that main has also changed
/// since the two parted, sorted, asked of `clone`, which the tick keeps,
/// fetched from `origin` with main so both are current. The change's head
/// goes to a ref of cadence's own, apart from the tick's. A file renamed
/// on either side counts under both its names, so a rename beside an edit
/// is still shared. Why git couldn't say, where it couldn't.
fn shared(clone: &Path, origin: &str, number: u32) -> Shared {
    if !clone.exists() {
        return Err("cadence keeps no clone of core yet".to_owned());
    }
    let head = format!("refs/cadence/pull/{number}");
    let wanted = format!("+refs/pull/{number}/head:{head}");
    let fetch = [
        "fetch",
        "--quiet",
        "--no-write-fetch-head",
        origin,
        MAIN,
        wanted.as_str(),
    ];
    git(clone, "fetching the change", &fetch).map_err(|err| err.to_string())?;
    let parting = ["merge-base", MAIN_HERE, head.as_str()];
    let base = git(clone, "finding where it parted", &parting).map_err(|err| err.to_string())?;
    let changed = |to: &str| {
        let args = ["diff", "--name-only", "--no-renames", base.trim(), to];
        git(clone, "reading what changed", &args)
            .map(|out| out.lines().map(str::to_owned).collect::<BTreeSet<_>>())
            .map_err(|err| err.to_string())
    };
    let ours = changed(&head)?;
    let main = changed(MAIN_HERE)?;
    Ok(ours.intersection(&main).cloned().collect())
}

#[cfg(test)]
mod tests;
