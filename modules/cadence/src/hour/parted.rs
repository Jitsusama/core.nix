//! What a core change and main have each changed since they parted, asked
//! of cadence's own clone of core, so the hour can tell a date waiting on
//! the change's merge that main has moved files beneath.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use super::measures::Measure;
use super::measures::at_risk::{REFUSALS, Shared, at_risk, drifted};
use super::moved::{MAIN, git};
use crate::tracking::Tracking;

/// How long the hour spends fetching changes before it says the rest
/// unread, so a slow network can't hold it past its own hour.
const FETCHING_AT_MOST: Duration = Duration::from_secs(5 * 60);

/// Joel's dates at risk and the list's own, from the asks list `asks` at
/// `now`: a core change asked of cadence's clone `core` once however many
/// pieces wait on it, with main fetched until once it comes, and a branch
/// of the zone's gate's record under `state_home`. The heads of changes
/// no longer asked of are let go after.
pub(super) fn dates_at_risk(state_home: &Path, core: &Path, asks: &str, now: i64) -> [Measure; 2] {
    let refusals = fs::read_to_string(state_home.join(REFUSALS)).ok();
    let asked: RefCell<BTreeMap<u32, Shared>> = RefCell::new(BTreeMap::new());
    let started = Instant::now();
    let main_fetched = Cell::new(false);
    let of_core = |number: u32| {
        if let Some(known) = asked.borrow().get(&number) {
            return known.clone();
        }
        let read = if started.elapsed() > FETCHING_AT_MOST {
            Err("the hour's time for fetching changes ran out".to_owned())
        } else {
            let read = shared(core, crate::CORE, number, !main_fetched.get());
            main_fetched.set(main_fetched.get() || read.is_ok());
            read
        };
        asked.borrow_mut().insert(number, read.clone());
        read
    };
    let of_world = |branch: &str| drifted(refusals.as_deref(), branch, now);
    let measures = [Tracking::His, Tracking::Derived]
        .map(|which| at_risk(asks, now, which, &of_core, &of_world));
    let_go(core, &asked.borrow());
    measures
}

/// Where cadence keeps the heads of the changes it reads.
const HEADS: &str = "refs/cadence/pull/";

/// Lets go of each change head in `clone` that this hour didn't ask of,
/// as `asked` holds those it did, so the clone keeps no head of a change
/// long merged or closed.
fn let_go(clone: &Path, asked: &BTreeMap<u32, Shared>) {
    if !clone.exists() {
        return;
    }
    let listing = ["for-each-ref", "--format=%(refname)", HEADS];
    let heads = match git(clone, "listing the heads kept", &listing) {
        Ok(heads) => heads,
        Err(err) => {
            eprintln!("cadence: at risk: {err}");
            return;
        }
    };
    for head in heads.lines() {
        let number = head.strip_prefix(HEADS).and_then(|n| n.parse::<u32>().ok());
        if number.is_some_and(|number| asked.contains_key(&number)) {
            continue;
        }
        if let Err(err) = git(clone, "letting a head go", &["update-ref", "-d", head]) {
            eprintln!("cadence: at risk: {err}");
        }
    }
}

/// Where main is once [`MAIN`] fetches it.
const MAIN_HERE: &str = "refs/remotes/cadence/main";

/// The files core change `number` touches that main has also changed
/// since the two parted, sorted, asked of `clone`, which the tick keeps,
/// its head fetched from `origin`, and main with it when `with_main`. The
/// change's head goes to a ref of cadence's own, apart from the tick's. A
/// file renamed on either side counts under both its names, so a rename
/// beside an edit is still shared. Why git couldn't say, where it
/// couldn't.
fn shared(clone: &Path, origin: &str, number: u32, with_main: bool) -> Shared {
    if !clone.exists() {
        return Err("cadence keeps no clone of core yet".to_owned());
    }
    let head = format!("{HEADS}{number}");
    let wanted = format!("+refs/pull/{number}/head:{head}");
    let mut fetch = vec!["fetch", "--quiet", "--no-write-fetch-head", origin];
    if with_main {
        fetch.push(MAIN);
    }
    fetch.push(&wanted);
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
