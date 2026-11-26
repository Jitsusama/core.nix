//! Landing a passed candidate: its riders merged through the hub once
//! main is still its base, the train's last merge held to the candidate's
//! tree, and the merges reverted when it isn't.

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

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
    /// GitHub refused the first rider, at `refused`, for `why`, so nothing
    /// merged; the riders after it ride again.
    Turned { refused: String, why: String },
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
            return through(git, hub, &base, &going, &riding);
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

/// How many more times GitHub is asked whether a merge landed when one of
/// its answers stated no refusal, the pauses between summing to five
/// minutes. A server error, or a call that timed out or lost its link,
/// says nothing of the merge, which GitHub may finish after it: at 11:15
/// on 25 November it answered each of the train's merges of #6398 with an
/// HTTP 502, the train turned #6398 away after half a minute, and GitHub
/// read it merged at 11:19, so the 17 riders left aboard ran their checks
/// again.
const SETTLE_ASKS: usize = 20;

/// How long the train waits between those asks; none in tests.
const SETTLE_PAUSE: Duration = if cfg!(test) {
    Duration::ZERO
} else {
    Duration::from_secs(15)
};

/// How long those asks may take in all, however slowly GitHub answers
/// each: twenty reads that each time out three times would otherwise hold
/// the train for about two hours. Six minutes leaves room for the five
/// of pauses and the reads between.
const SETTLE_FOR: Duration = Duration::from_secs(6 * 60);

/// What `gh` adds to an answer in which GitHub refused the merge, as
/// `gh: Pull Request is still a draft (HTTP 405)`; any other failure,
/// a server error, a timeout or a lost link, states no refusal.
const STATED_REFUSAL: &str = "(HTTP 4";

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
    let mut unstated = false;
    for tried in 1..=MERGE_TRIES {
        let why = match hub.merge(merging) {
            Ok(made) => return Merge::Made(made),
            Err(why) => why,
        };
        // GitHub may refuse a merge it is still finishing, so one answer
        // stating no refusal leaves the merge in doubt however the later
        // ones read.
        unstated |= !why.contains(STATED_REFUSAL);
        match merged(hub, merging.number) {
            Ok(Some(made)) => return Merge::Made(made),
            Ok(None) if tried == MERGE_TRIES && unstated => {
                return settled(hub, merging.number, why);
            }
            Ok(None) if tried == MERGE_TRIES => return Merge::Refused(why),
            Ok(None) => {}
            // A merge already in doubt is waited out past a failed read
            // too, rather than stopping the whole train at once.
            Err(_) if unstated => return settled(hub, merging.number, why),
            Err(error) => {
                return Merge::Unknown(format!("{why}, and GitHub couldn't say after: {error}"));
            }
        }
        std::thread::sleep(pause);
        pause *= 2;
    }
    Merge::Refused("it was never asked".to_owned())
}

/// How a merge in doubt, GitHub's last answer to it `why`, went: made once
/// GitHub reads pull request `number` merged, refused when its last read
/// of `SETTLE_ASKS`, or the last within `SETTLE_FOR`, still says
/// unmerged, and unknown when that last read failed. A read failing
/// between is asked past, as one failed read says nothing, and an unknown
/// merge stops the whole train.
fn settled(hub: &dyn Hub, number: u64, why: String) -> Merge {
    println!(
        "merge-net: an answer to #{number}'s merge stated no refusal, so the train asks \
         for up to five minutes whether it landed; its last answer: {}",
        one_line(&why)
    );
    let until = Instant::now() + SETTLE_FOR;
    let mut read = Ok(None);
    for _ in 0..SETTLE_ASKS {
        std::thread::sleep(SETTLE_PAUSE);
        read = merged(hub, number);
        if let Ok(Some(made)) = read {
            println!(
                "merge-net: GitHub reads #{number} merged, as {}",
                short(&made)
            );
            return Merge::Made(made);
        }
        if Instant::now() >= until {
            break;
        }
    }
    match read {
        Err(error) => {
            println!(
                "merge-net: GitHub still couldn't say whether #{number} merged: {}",
                one_line(&error)
            );
            Merge::Unknown(format!("{why}, and GitHub couldn't say after: {error}"))
        }
        Ok(_) => {
            println!("merge-net: GitHub still reads #{number} unmerged, so it leaves the train");
            Merge::Refused(why)
        }
    }
}

/// `text` as one line of at most 300 characters, for train.log: gh's
/// errors can run long and over several lines.
fn one_line(text: &str) -> String {
    let joined = text.split_whitespace().collect::<Vec<_>>().join(" ");
    match joined.char_indices().nth(300) {
        Some((at, _)) => format!("{} ...", &joined[..at]),
        None => joined,
    }
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
/// draft behind them cost both their owners a revert of the revert. When
/// GitHub refuses the first, it alone leaves the train, which merged
/// nothing. A
/// change merged after the train's last merge leaves it landed, as that
/// merge's tree is still the one tested. A merge GitHub can't say it made
/// stops it at once, keeping every merge and saying so, as reverting
/// under one that stands would leave main a tree nobody wrote.
fn through(
    git: &dyn Fn(&[&str]) -> Result<String, String>,
    hub: &dyn Hub,
    base: &str,
    candidate: &str,
    riding: &[Merging],
) -> Landing {
    let mut made: Vec<(&Merging, String)> = Vec::new();
    let mut refused = None;
    for merging in riding {
        match merge(hub, merging) {
            Merge::Made(commit) => made.push((merging, commit)),
            Merge::Refused(why) => {
                refused = Some((
                    format!("GitHub refused #{}: {why}", merging.number),
                    merging,
                ));
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
        return match refused {
            Some((why, refusing)) => Landing::Turned {
                refused: refusing.head.clone(),
                why,
            },
            None => Landing::Refused("the train had no riders to merge".to_owned()),
        };
    };
    // The candidate as it stood after the riders made: just before its
    // merge of the first rider not made, or all of it when it holds none.
    let unmade: Vec<&str> = riding[made.len()..]
        .iter()
        .map(|merging| merging.head.as_str())
        .collect();
    let tested = before_any(git, base, candidate, &unmade)
        .and_then(|at| git(&["rev-parse", &format!("{at}^{{tree}}")]).map(|tree| (at, tree)));
    let landed_as_tested = |why: String| match (thrice(|| hub.tree(last)), &tested) {
        (Ok(merged), Ok((_, tree))) if merged == *tree => Ok(()),
        (Ok(_), Ok((at, _))) => Err(format!(
            "{why}GitHub's merge {} doesn't hold the tree the train tested at {}",
            short(last),
            short(at)
        )),
        (Ok(_), Err(error)) => Err(format!(
            "{why}the train couldn't say which tree it tested: {error}"
        )),
        (Err(error), _) => Err(format!(
            "{why}GitHub couldn't say what its merge {} holds: {error}",
            short(last)
        )),
    };
    if let Some((why, refusing)) = refused {
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

/// The candidate, cut on `base`, as it stood before it merged any of
/// `heads`: the first parent of its oldest merge of one, or `candidate`
/// itself when it merged none, found by each merge's second parent
/// rather than counted, since a rider whose head main already held left
/// no merge of its own. Only the commits the candidate added to `base`
/// are read, so a merge in main's own history is never taken for one.
fn before_any(
    git: &dyn Fn(&[&str]) -> Result<String, String>,
    base: &str,
    candidate: &str,
    heads: &[&str],
) -> Result<String, String> {
    let listed = git(&[
        "rev-list",
        "--first-parent",
        "--parents",
        &format!("{base}..{candidate}"),
    ])?;
    let first = listed.lines().rev().find_map(|line| {
        let mut commits = line.split(' ');
        let (_, before, merged) = (commits.next()?, commits.next()?, commits.next()?);
        heads.contains(&merged).then(|| before.to_owned())
    });
    Ok(first.unwrap_or_else(|| candidate.to_owned()))
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
