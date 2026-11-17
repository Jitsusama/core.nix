//! Landing a passed candidate: its riders merged through the hub once
//! main is still its base, main's tree held to the candidate's, and the
//! merges reverted when it isn't.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use super::hub::Hub;
use super::{Merging, RECUTS, cut};
use crate::state::short;

/// How a passed candidate's landing went.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Landing {
    /// It landed as this commit.
    Landed(String),
    /// Its first riders, `heads`, landed as `landed`; GitHub refused the
    /// next for `why`.
    Partly {
        landed: String,
        heads: Vec<String>,
        why: String,
    },
    /// What GitHub merged wasn't what the train tested, for this reason,
    /// so it reverted every merge it made.
    Reverted(String),
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

/// Origin's main once it holds `head`, as GitHub may answer a merge a
/// moment before a fetch sees it, which would read as main holding a tree
/// no gate ran and revert merges that were right; main as it stands after
/// `SHOWS_WITHIN`.
fn main_holding(git: &dyn Fn(&[&str]) -> Result<String, String>, head: &str) -> Option<String> {
    let began = std::time::Instant::now();
    loop {
        let main = main_of(git)?;
        let holds = git(&["merge-base", "--is-ancestor", head, &main]).is_ok();
        if holds || began.elapsed() > SHOWS_WITHIN {
            return Some(main);
        }
        std::thread::sleep(Duration::from_secs(2));
    }
}

/// How long a merge GitHub answered may take to show in a fetch of main.
const SHOWS_WITHIN: Duration = Duration::from_secs(30);

/// Merge each of `riding` through `hub`, in the order `candidate` merged
/// them on main as it stands, until one is refused, then hold main's tree
/// to the candidate's after as many merges. GitHub merges each on main as
/// it is when asked, so one that landed meanwhile, or a merge GitHub
/// works out otherwise than git, gives main a tree no gate ran; then each
/// merge made is reverted, the newest first.
fn through(
    git: &dyn Fn(&[&str]) -> Result<String, String>,
    hub: &dyn Hub,
    candidate: &str,
    riding: &[Merging],
) -> Landing {
    let mut made: Vec<&Merging> = Vec::new();
    let mut refused = None;
    for merging in riding {
        match hub.merge(merging) {
            Ok(()) => made.push(merging),
            Err(why) => {
                refused = Some(why);
                break;
            }
        }
    }
    if made.is_empty() {
        return Landing::Refused(refused.unwrap_or_default());
    }
    let tested = format!("{candidate}~{}^{{tree}}", riding.len() - made.len());
    let main = main_holding(git, made.last().map_or("", |merging| &merging.head));
    let tree_of = |commit: &str| git(&["rev-parse", commit]).ok();
    let same = main.as_ref().is_some_and(|main| {
        let (now, then) = (tree_of(&format!("{main}^{{tree}}")), tree_of(&tested));
        now.is_some() && now == then
    });
    match (main, refused) {
        (Some(main), None) if same => Landing::Landed(main),
        (Some(landed), Some(why)) if same => Landing::Partly {
            landed,
            heads: made.iter().map(|merging| merging.head.clone()).collect(),
            why,
        },
        (main, _) => {
            let mut why = main.map_or_else(
                || "origin's main couldn't be read after GitHub's merges".to_owned(),
                |main| {
                    format!(
                        "main's tree at {} after GitHub's merges isn't the one the train tested \
                         at {}",
                        short(&main),
                        short(candidate)
                    )
                },
            );
            for merging in made.iter().rev() {
                if let Err(error) = hub.revert(merging.number) {
                    why = format!("{why}, and reverting #{} failed: {error}", merging.number);
                    eprintln!("merge-net: {why}");
                    break;
                }
            }
            println!("merge-net: {why}, so the train reverted its merges");
            Landing::Reverted(why)
        }
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
