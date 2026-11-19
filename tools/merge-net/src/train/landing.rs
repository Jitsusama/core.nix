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
    /// GitHub refused the rider at `refused`, for `why`, after merging the
    /// riders at `kept`, which stay landed as `landed`, the tree the
    /// candidate had after them; the riders after the refused one ride
    /// again.
    Partly {
        landed: String,
        kept: Vec<String>,
        refused: String,
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

/// How long the train first waits on GitHub before asking again, doubled
/// at each ask; none in tests, whose stand-in answers at once.
const PAUSE: Duration = if cfg!(test) {
    Duration::ZERO
} else {
    Duration::from_secs(2)
};

/// How many times a merge GitHub refuses is asked, the pauses between
/// summing to half a minute: GitHub refuses a pull request while it works
/// out whether it can merge, which it does again after each of the
/// train's merges.
const MERGE_TRIES: usize = 5;

/// How a merge GitHub was asked for went.
enum Merge {
    /// It merged as this commit.
    Made(String),
    /// GitHub refused it, for this reason, and reads it unmerged.
    Refused(String),
    /// GitHub answered with an error and couldn't say after whether it
    /// merged, for this reason, so main may hold it.
    Unknown(String),
}

/// Merge `merging` through `hub`. A merge whose answer was lost may have
/// landed all the same, so each refusal is checked against whether GitHub
/// reads it merged, and asked again while it reads unmerged.
fn merge(hub: &dyn Hub, merging: &Merging) -> Merge {
    let mut pause = PAUSE;
    for tried in 1..=MERGE_TRIES {
        let why = match hub.merge(merging) {
            Ok(made) => return Merge::Made(made),
            Err(why) => why,
        };
        match merged(hub, merging.number) {
            Ok(Some(made)) => return Merge::Made(made),
            Ok(None) if tried == MERGE_TRIES => return Merge::Refused(why),
            Ok(None) => {}
            Err(error) => {
                return Merge::Unknown(format!("{why}, and GitHub couldn't say after: {error}"));
            }
        }
        std::thread::sleep(pause);
        pause *= 2;
    }
    Merge::Refused("it was never asked".to_owned())
}

/// Whether GitHub reads pull request `number` merged, asked three times
/// before its silence counts as not knowing.
fn merged(hub: &dyn Hub, number: u64) -> Result<Option<String>, String> {
    thrice(|| hub.merged(number))
}

/// What `ask` answers, asked up to three times while it fails, as one
/// failed call to GitHub says nothing of what it holds.
fn thrice<T>(ask: impl Fn() -> Result<T, String>) -> Result<T, String> {
    let mut asked = ask();
    for _ in 1..3 {
        if asked.is_ok() {
            break;
        }
        std::thread::sleep(PAUSE);
        asked = ask();
    }
    asked
}

/// Merge each of `riding` through `hub`, in the order `candidate` merged
/// them, then hold the tree GitHub gives the train's own last merge to the
/// candidate's. GitHub merges each on main as it is when asked, so one
/// that landed meanwhile, or a merge GitHub works out otherwise than git,
/// gives main a tree no gate ran; then each merge made is reverted, the
/// newest first. A merge GitHub refuses stops it short, and the merges
/// before it stay, held to the tree the candidate had after them: each
/// passed its own checks, and on 19 November reverting two of them for a
/// draft behind them cost both their owners a revert of the revert. A
/// change merged after the train's last merge leaves it landed, as that
/// merge's tree is still the one tested. A merge GitHub can't say it made
/// stops it at once, keeping every merge and saying so, as reverting
/// under one that stands would leave main a tree nobody wrote.
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
            Merge::Made(commit) => made.push((merging, commit)),
            Merge::Refused(why) => {
                refused = Some(format!("GitHub refused #{}: {why}", merging.number));
                break;
            }
            Merge::Unknown(why) => {
                let why = format!("GitHub can't say whether #{} merged: {why}", merging.number);
                println!("merge-net: {why}, so the train reverted none of its merges");
                let mut kept: Vec<String> = made.iter().map(|(at, _)| at.head.clone()).collect();
                kept.push(merging.head.clone());
                return Landing::Reverted {
                    reverted: Vec::new(),
                    kept,
                    why,
                };
            }
        }
    }
    let Some((_, last)) = made.last() else {
        return Landing::Refused(refused.unwrap_or_default());
    };
    // Each rider is a merge along the candidate's first parent, so the
    // tree after the last made is that many merges back from its top.
    let short_by = riding.len() - made.len();
    let tested = git(&["rev-parse", &format!("{candidate}~{short_by}^{{tree}}")]);
    let landed_as_tested = |why: String| match (thrice(|| hub.tree(last)), &tested) {
        (Ok(merged), Ok(tested)) if merged == *tested => Ok(()),
        (Ok(_), _) => Err(format!(
            "{why}GitHub's merge {} doesn't hold the tree the train tested at {}",
            short(last),
            short(candidate)
        )),
        (Err(error), _) => Err(format!(
            "{why}GitHub couldn't say what its merge {} holds: {error}",
            short(last)
        )),
    };
    if let Some(why) = refused {
        let Some(refusing) = riding.get(made.len()) else {
            return reverted(hub, &made, why);
        };
        return match landed_as_tested(format!("{why}, and ")) {
            Ok(()) => Landing::Partly {
                landed: last.clone(),
                kept: made.iter().map(|(at, _)| at.head.clone()).collect(),
                refused: refusing.head.clone(),
                why,
            },
            Err(why) => reverted(hub, &made, why),
        };
    }
    match landed_as_tested(String::new()) {
        Ok(()) => Landing::Landed(last.clone()),
        Err(why) => reverted(hub, &made, why),
    }
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
                kept = made[..=at]
                    .iter()
                    .map(|(merging, _)| merging.head.clone())
                    .collect();
                break;
            }
        }
    }
    println!(
        "merge-net: {why}, so the train reverted {} of its {} merges",
        reverted.len(),
        made.len()
    );
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
