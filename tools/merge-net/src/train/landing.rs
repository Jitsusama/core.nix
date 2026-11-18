//! Landing a passed candidate: its riders merged through the hub once
//! main is still its base, the train's last merge held to the candidate's
//! tree, and the merges reverted when it isn't.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use super::hub::Hub;
use super::{Merging, RECUTS, cut};
use crate::state::short;

/// How a passed candidate's landing went.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Landing {
    /// It landed as this commit, the train's last merge.
    Landed(String),
    /// What GitHub merged wasn't what the train tested, for `why`, so it
    /// reverted its merges, the newest first: each rider's head beside
    /// the pull request that reverted it, and, when a revert failed, the
    /// heads of the merges it couldn't revert, left on main.
    Reverted {
        reverted: Vec<(String, u64)>,
        kept: Vec<String>,
        why: String,
    },
    /// Main moved beside it, so its riders ride the next run.
    Moved,
    /// Main refused it for this reason, though main stood still.
    Refused(String),
}

/// Land the passed `candidate`, cut on `base`, through `hub`'s merges of
/// its riders once main is still its base, cutting it again on a main
/// that moved while its pass still speaks for it, as many as `RECUTS`
/// times.
pub(super) fn land(
    checkout: &Path,
    git: &dyn Fn(&[&str]) -> Result<String, String>,
    hub: &dyn Hub,
    base: &str,
    candidate: &str,
    mergings: &[Merging],
    merged: &[String],
) -> Landing {
    let riding: Vec<Merging> = mergings
        .iter()
        .filter(|merging| merged.contains(&merging.head))
        .cloned()
        .collect();
    let (mut going, mut base) = (candidate.to_owned(), base.to_owned());
    for _ in 0..RECUTS {
        let Some(main) = main_of(git) else {
            return Landing::Refused("origin's main couldn't be read".to_owned());
        };
        if main == base {
            return through(git, hub, &going, &riding);
        }
        let whole = cut(git, &main, &riding).is_ok_and(|again| again.merged.len() == merged.len());
        let Ok(head) = git(&["rev-parse", "HEAD"]) else {
            return Landing::Moved;
        };
        (going, base) = (head, main.clone());
        if !whole || !still_passes(checkout, &going, &main) {
            println!(
                "merge-net: main moved beside the train at {}, so it runs again",
                short(&main)
            );
            return Landing::Moved;
        }
    }
    Landing::Moved
}

/// Origin's main as it stands, fetched afresh, trying three times, as a
/// fetch beside another can't lock its ref.
fn main_of(git: &dyn Fn(&[&str]) -> Result<String, String>) -> Option<String> {
    (0..3).find_map(|_| {
        git(&["fetch", "--quiet", "origin", "main"]).ok()?;
        git(&["rev-parse", "refs/remotes/origin/main"]).ok()
    })
}

/// Whether origin's main holds `merge`, fetched afresh until it does or
/// `SHOWS_WITHIN` passes, as GitHub answers a merge a moment before a
/// fetch may see it.
fn shows(git: &dyn Fn(&[&str]) -> Result<String, String>, merge: &str) -> bool {
    let began = std::time::Instant::now();
    loop {
        let holds = main_of(git)
            .is_some_and(|main| git(&["merge-base", "--is-ancestor", merge, &main]).is_ok());
        if holds || began.elapsed() > SHOWS_WITHIN {
            return holds;
        }
        std::thread::sleep(Duration::from_secs(2));
    }
}

/// How long a merge GitHub answered may take to show in a fetch of main.
const SHOWS_WITHIN: Duration = Duration::from_secs(30);

/// How many times a merge GitHub refuses as main moved under it is asked.
const MERGE_TRIES: u32 = 3;

/// Merge `merging` through `hub`, giving its merge's commit. GitHub
/// refuses a merge as "Base branch was modified" while it still works out
/// the train's own merge just before, so that one is asked again; and a
/// merge whose answer was lost may have landed all the same, so one
/// GitHub reads merged counts as made.
fn merge(hub: &dyn Hub, merging: &Merging) -> Result<String, String> {
    let mut tried = 0;
    loop {
        tried += 1;
        let why = match hub.merge(merging) {
            Ok(made) => return Ok(made),
            Err(why) => why,
        };
        if let Some(made) = hub.merged(merging.number) {
            return Ok(made);
        }
        if tried >= MERGE_TRIES || !why.contains("Base branch was modified") {
            return Err(why);
        }
        std::thread::sleep(Duration::from_secs(2));
    }
}

/// Merge each of `riding` through `hub`, in the order `candidate` merged
/// them, then hold the tree of the train's own last merge to the
/// candidate's. GitHub merges each on main as it is when asked, so one
/// that landed meanwhile, or a merge GitHub works out otherwise than git,
/// gives main a tree no gate ran, and so does stopping short at a merge
/// GitHub refuses; then each merge made is reverted, the newest first. A
/// change merged after the train's last merge leaves it landed, as that
/// merge's tree is still the one tested.
fn through(
    git: &dyn Fn(&[&str]) -> Result<String, String>,
    hub: &dyn Hub,
    candidate: &str,
    riding: &[Merging],
) -> Landing {
    let mut made: Vec<(&Merging, String)> = Vec::new();
    let mut refused = None;
    for merging in riding {
        match merge(hub, merging) {
            Ok(commit) => made.push((merging, commit)),
            Err(why) => {
                refused = Some(format!("GitHub refused #{}: {why}", merging.number));
                break;
            }
        }
    }
    let Some((_, last)) = made.last() else {
        return Landing::Refused(refused.unwrap_or_default());
    };
    if let Some(why) = refused {
        return reverted(hub, &made, why);
    }
    let tree_of = |commit: &str| git(&["rev-parse", &format!("{commit}^{{tree}}")]).ok();
    let why = if !shows(git, last) {
        format!(
            "GitHub's merge {} didn't show on origin's main within {} s",
            short(last),
            SHOWS_WITHIN.as_secs()
        )
    } else if tree_of(last).is_some() && tree_of(last) == tree_of(candidate) {
        return Landing::Landed(last.clone());
    } else {
        format!(
            "GitHub's merge {} doesn't hold the tree the train tested at {}",
            short(last),
            short(candidate)
        )
    };
    reverted(hub, &made, why)
}

/// Revert each of `made` through `hub`, the newest first, for `why`,
/// stopping at the first revert that fails, whose merge and those under it
/// stay on main, since reverting one under a merge left there would
/// leave main a tree nobody wrote.
fn reverted(hub: &dyn Hub, made: &[(&Merging, String)], mut why: String) -> Landing {
    let mut reverted = Vec::new();
    let mut kept = Vec::new();
    for (at, (merging, _)) in made.iter().enumerate().rev() {
        match hub.revert(merging) {
            Ok(number) => reverted.push((merging.head.clone(), number)),
            Err(error) => {
                why = format!("{why}, and reverting #{} failed: {error}", merging.number);
                eprintln!("merge-net: {why}");
                kept = made[..=at]
                    .iter()
                    .map(|(merging, _)| merging.head.clone())
                    .collect();
                break;
            }
        }
    }
    println!("merge-net: {why}, so the train reverted its merges");
    Landing::Reverted {
        reverted,
        kept,
        why,
    }
}

/// Whether the pass of an earlier candidate of the same changes speaks for
/// `candidate` on `main`, as `ready` takes one for a rebased change.
fn still_passes(checkout: &Path, candidate: &str, main: &str) -> bool {
    let Ok(here) = std::env::current_dir() else {
        return false;
    };
    if std::env::set_current_dir(checkout).is_err() {
        return false;
    }
    let on_main = |base: &str| {
        Command::new("git")
            .args(["merge-base", "--is-ancestor", base, main])
            .status()
            .is_ok_and(|status| status.success())
    };
    let judged = crate::earlier::judged(candidate, main, &on_main);
    let _ = std::env::set_current_dir(here);
    judged.is_ok_and(|judged| {
        matches!(
            judged.before,
            crate::ready::Before::Clean(_) | crate::ready::Before::Twice(..)
        )
    })
}

#[cfg(test)]
mod tests;
