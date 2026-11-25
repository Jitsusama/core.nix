//! Whether a review round of ours read a head, or an earlier commit of its
//! change that a clean rebase carried to it, and how an earlier commit is
//! told to be the same change. From 17 to 19 November 240 of 429 reviewed
//! pull requests merged at a head no round had read, a fix pushed after
//! the council going to main unread.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Command;

use super::{pins, seen};
use crate::{candidates, candidates_at, git_at, patch_of, ready, short};

/// How a change from a base to a head, in a repo, is told from others.
pub type PatchId = fn(&Path, &str, &str) -> Option<String>;

/// Whether a review round of ours read `commit`, or an earlier commit of
/// the same change that a clean rebase carried here (`same_patch`). The
/// earlier commits are looked for among those the rounds read, so a
/// reviewed commit with no pass of its own still counts. Why not, when not.
///
/// Only the rounds of `commit` and of the earlier commits that could be its
/// change have their base asked after, one git each: on 21 November the
/// rounds held about 2,300 bases, and asking after every one held each
/// `ready` past 15 minutes while four ran at once.
///
/// The rounds mirrored from other machines are asked only when this
/// machine's read nothing for the head, so a mirror that's broken or slow
/// never holds back a head reviewed here.
///
/// A head no round read that only pins newer commits of repos our rounds
/// already review needs none of its own (`pins.rs`), and is told so, or
/// why it needs one after all. It's asked after only once the rounds
/// found nothing, for the same reason as the mirrors: its asks go to
/// GitHub and Nix, which may be slow.
pub fn read_by_a_round(commit: &str, main: &str, files: &[String]) -> Result<(), String> {
    read_in_repo(
        Path::new("."),
        (commit, main, files),
        || seen::witnessed(&seen::rounds_folder()),
        || seen::mirrored(&seen::mirrors_folder()),
        &pins::Live,
    )
}

/// [`read_by_a_round`] of `head`, its commit, main and files, in the repo
/// at `repo`, with this machine's `rounds`, the `mirrors`, and `asks` to
/// put a pin's questions to. Rounds that can't be read are taken as
/// reading nothing, so a pin is still asked after, and a pin turned down
/// is refused naming both why the rounds didn't do and why it doesn't.
fn read_in_repo(
    repo: &Path,
    (commit, main, files): (&str, &str, &[String]),
    rounds: impl FnOnce() -> Result<HashSet<seen::Read>, String>,
    mirrors: impl FnOnce() -> seen::Mirrored,
    asks: &impl pins::Asks,
) -> Result<(), String> {
    let read = rounds().and_then(|rounds| {
        here_or_mirrored(rounds, mirrors, |rounds| {
            read_in(
                commit,
                rounds,
                on_main_at(repo, main),
                |witnessed| candidates_at(repo, commit, main, files, witnessed).unwrap_or_default(),
                |read| same_patch(repo, commit, main, read, read_patch_id),
            )
        })
    });
    let Err(unread) = read else {
        return Ok(());
    };
    match pins::ours_alone(repo, commit, main, asks) {
        Some(pins::Pinned::Alone(said)) => {
            eprintln!("merge-net: {said}");
            Ok(())
        }
        Some(pins::Pinned::Declined(why)) => Err(format!(
            "{unread}; {} moves only flake.lock, but {why}",
            short(commit)
        )),
        None => Err(unread),
    }
}

/// Whether `ask` finds the head read among this machine's `rounds`, or,
/// only when it doesn't, among them and those `mirrors` reads. A refusal
/// says which mirrors were asked and which couldn't be.
fn here_or_mirrored(
    mut rounds: HashSet<seen::Read>,
    mirrors: impl FnOnce() -> seen::Mirrored,
    mut ask: impl FnMut(&HashSet<seen::Read>) -> Result<(), String>,
) -> Result<(), String> {
    let Err(unread) = ask(&rounds) else {
        return Ok(());
    };
    let mirrored = mirrors();
    // No mirrored round means nothing new to find the head among, so the
    // head isn't asked after again, only told with why.
    if mirrored.rounds.is_subset(&rounds) {
        return Err(seen::unread_with_mirrors(unread, &mirrored));
    }
    rounds.extend(mirrored.rounds.iter().cloned());
    ask(&rounds).map_err(|why| seen::unread_with_mirrors(why, &mirrored))
}

/// Whether `rounds` read `commit` in full, or an earlier commit of its
/// change: `could_be`, given every commit the rounds read, sorted, names
/// those that might be its change, and `same` keeps those of the ones read
/// in full that are. A base is put to `on_main` only for the rounds of
/// `commit` and of what `could_be` names, which keeps the gits to a few.
fn read_in(
    commit: &str,
    rounds: &HashSet<seen::Read>,
    mut on_main: impl FnMut(&str) -> bool,
    could_be: impl FnOnce(&[String]) -> Vec<String>,
    same: impl FnOnce(Vec<String>) -> Vec<String>,
) -> Result<(), String> {
    let mut full = |among: &HashSet<&str>| {
        let of_them = rounds
            .iter()
            .filter(|read| among.contains(read.commit.as_str()));
        in_full(of_them, &mut on_main)
    };
    if !full(&HashSet::from([commit])).is_empty() {
        return Ok(());
    }
    let mut witnessed: Vec<String> = rounds
        .iter()
        .map(|read| read.commit.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    witnessed.sort();
    let could_be = could_be(&witnessed);
    let mut read: Vec<String> = full(&could_be.iter().map(String::as_str).collect())
        .into_iter()
        .collect();
    read.sort();
    if same(read).is_empty() {
        // A range round that read the head itself, from a base that's no
        // commit on main, is told apart from no round at all: asking the
        // same range again reads from the same base. A rebased head, read
        // by no round itself, is left to `seen::unread`, since what could
        // be its change was searched above.
        let mut bases: Vec<String> = rounds
            .iter()
            .filter(|read| read.commit == commit)
            .map(|read| seen::base_named(read.from.as_deref().unwrap_or_default()))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        bases.sort();
        Err(if bases.is_empty() {
            seen::unread(short(commit))
        } else {
            seen::read_from_no_base(short(commit), &bases.join(", "))
        })
    } else {
        Ok(())
    }
}

/// The commits of `rounds` read in full: each a change's round read, and
/// each a range's whose base `on_main` finds on main, so a round over only
/// a head's newest commit doesn't stand for the commits under it.
fn in_full<'a>(
    rounds: impl Iterator<Item = &'a seen::Read>,
    on_main: &mut impl FnMut(&str) -> bool,
) -> HashSet<String> {
    rounds
        .filter(|read| read.from.as_deref().is_none_or(&mut *on_main))
        .map(|read| read.commit.clone())
        .collect()
}

/// Whether a base is on `main`, in the repo at `repo`, one git a base. A
/// git that couldn't check one, as in a repo it can't open, counts it as
/// off main, as the gate must, but says git's own words, so its refusal
/// isn't read as a review gap when it's the net that's broken.
fn on_main_at<'a>(repo: &'a Path, main: &'a str) -> impl FnMut(&str) -> bool + 'a {
    let mut known = HashMap::new();
    move |base: &str| {
        *known.entry(base.to_owned()).or_insert_with(|| {
            // One git would read as an option is no base.
            if base.is_empty() || base.starts_with('-') {
                return false;
            }
            match is_ancestor(repo, base, main) {
                Ok(on_main) => on_main,
                Err(said) => {
                    eprintln!("merge-net: {}", couldnt_check(base, &said));
                    false
                }
            }
        })
    }
}

/// Whether `base` is an ancestor of `main` in the repo at `repo`, as
/// `merge-base --is-ancestor` answers, its 1 being no; anything else git
/// ends with is its failure, in its own words.
fn is_ancestor(repo: &Path, base: &str, main: &str) -> Result<bool, String> {
    let ran = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["merge-base", "--is-ancestor", base, main])
        .output()
        .map_err(|error| format!("running git: {error}"))?;
    match ran.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(String::from_utf8_lossy(&ran.stderr).trim().to_owned()),
    }
}

/// What's said of a base git couldn't check, with what git said.
fn couldnt_check(base: &str, said: &str) -> String {
    format!(
        "git couldn't check whether the base {} is on main, so it counts as off main: {said}",
        seen::base_named(base)
    )
}

/// The earlier commits of `commit`'s change among `records` touching
/// `files` whose own change, against their base on main, is the same patch
/// as `commit`'s, by `id`.
pub fn same_change(
    commit: &str,
    main: &str,
    files: &[String],
    records: &[String],
    id: PatchId,
) -> Vec<String> {
    let candidates = candidates(commit, main, files, records).unwrap_or_default();
    same_patch(Path::new("."), commit, main, candidates, id)
}

/// Those of `candidates`, in the repo at `repo`, whose own change against
/// their base on `main` is the same patch as `commit`'s against its own,
/// by `id`: by `read_patch_id`, a clean rebase of a reviewed commit is one.
fn same_patch(
    repo: &Path,
    commit: &str,
    main: &str,
    candidates: Vec<String>,
    id: PatchId,
) -> Vec<String> {
    let base_of = |at: &str| git_at(repo, &["merge-base", main, at]).ok();
    let base = base_of(commit).unwrap_or_else(|| main.to_owned());
    let Some(change) = id(repo, &base, commit) else {
        return Vec::new();
    };
    candidates
        .into_iter()
        .filter(|candidate| {
            ready::when_same(
                candidate,
                &change,
                base_of,
                |base, at| id(repo, base, at),
                |_| Some(()),
            )
            .is_some()
        })
        .collect()
}

/// The patch id a review carries by, from `base` to `head` in the repo at
/// `repo`: with git's lines of context, so a block moved within its file
/// is another change, and whitespace kept, since it is code in some
/// languages. A rebase over an edit beside the change asks for a round
/// again, which costs little beside a head merging unread. The diff's
/// shape is named rather than left to git's config, where
/// `diff.context=0` would drop the context the id is read with.
fn read_patch_id(repo: &Path, base: &str, head: &str) -> Option<String> {
    let shape = [
        "--unified=3",
        "--no-color",
        "--no-ext-diff",
        "--no-textconv",
    ];
    // Verbatim ids are stable ones too; git takes only one of the two.
    patch_of(repo, base, head, &shape, "--verbatim")
}

#[cfg(test)]
mod tests;
