//! `there --changed` checking a change on a main a lent builder keeps
//! evaluated, when neither keeps its own base. From 13:40 to 17:40 on 12
//! November, 100 of 211 runs evaluated their base, a median 397 s each, and
//! took 1,156 s against 812 s for the rest; 81 of those bases had merged
//! under 6 minutes before, while keep-main takes 11 to keep one. So the
//! change is replayed, commit by commit, onto the newest main a builder
//! keeps, and that is what runs: the same patch on a base whose checks need
//! no evaluating. `merge-net ready` takes its pass for the change as it
//! takes any earlier pass of it, by its subjects, its files and its patch
//! id, where main has moved none of its files since.
//!
//! A change isn't replayed when main moved, between the two bases, a file
//! in a folder the change touches, or one that reaches every check, since
//! the pass could then say something of the change it wouldn't say on its
//! own base; nor when it holds a merge, or a commit doesn't apply cleanly.
//! It then runs on its own base, as it would have.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::there;

/// What a lent builder is asked, to list what its changed-checks keeps:
/// each base's checks as `<base>-<system>.json`.
pub const LISTING: &str = "ls \"${XDG_CACHE_HOME:-$HOME/.cache}/changed-checks\"";

/// How far back along main a kept base is looked for. keep-main keeps a
/// head every 11 minutes or so, and main merges about one a minute at its
/// busiest, so a kept one is never this far back while it runs.
const LOOKS_BACK: &str = "300";

/// Files whose move on main reaches every check.
const EVERYWHERE: [&str; 3] = ["flake.nix", "flake.lock", "checks.nix"];

/// Who wrote and committed a commit, and when, given again as they were,
/// so the same change replayed onto the same main is the same commit, and
/// a pass kept for it is found again.
const PEOPLE: [&str; 6] = [
    "GIT_AUTHOR_NAME",
    "GIT_AUTHOR_EMAIL",
    "GIT_AUTHOR_DATE",
    "GIT_COMMITTER_NAME",
    "GIT_COMMITTER_EMAIL",
    "GIT_COMMITTER_DATE",
];

/// A change replayed onto a kept base: the commit that runs, and the base.
#[derive(Debug, PartialEq, Eq)]
pub struct Replayed {
    pub commit: String,
    pub onto: String,
}

/// The bases whose checks for `system` a builder's `listing` keeps.
pub fn kept_in(listing: &str, system: &str) -> HashSet<String> {
    let ending = format!("-{system}.json");
    listing
        .lines()
        .filter_map(|name| name.trim().strip_suffix(&ending))
        .filter(|base| base.len() == 40 && base.chars().all(|c| c.is_ascii_hexdigit()))
        .map(str::to_owned)
        .collect()
}

/// Why a change touching `change` can't be checked on a base that differs
/// from its own by `moved`, or none: a file reaching every check moved, or
/// one in a folder the change touches.
pub fn refusal(moved: &[String], change: &[String]) -> Option<String> {
    if let Some(file) = moved
        .iter()
        .find(|file| EVERYWHERE.contains(&file.as_str()))
    {
        return Some(format!("main moved {file}, which every check reads"));
    }
    let touched: HashSet<&str> = change.iter().map(|file| folder(file)).collect();
    moved
        .iter()
        .find(|file| touched.contains(folder(file)))
        .map(|file| format!("main moved {file}, beside what the change touches"))
}

/// The folder a file's change stays within: its first two folders, such as
/// `modules/nix-offload`, or the file itself at the top or one below.
fn folder(file: &str) -> &str {
    match file.match_indices('/').nth(1) {
        Some((at, _)) => &file[..at],
        None => file,
    }
}

/// `commit`'s change from `base` replayed onto the newest commit of
/// `theirs` whose checks `kept` holds, in the git tree `top`; none when
/// `base` is kept itself; why not when it can't be. Each commit keeps its
/// author, committer and message, so the same change onto the same base
/// is the same commit each time, and keeps the subjects `ready` finds an
/// earlier pass of it by.
pub fn replayed(
    top: &Path,
    base: &str,
    commit: &str,
    theirs: &str,
    kept: &HashSet<String>,
    scratch: &Path,
) -> Result<Option<Replayed>, String> {
    if kept.contains(base) {
        return Ok(None);
    }
    let git = |arguments: &[&str]| there::git_with(top, arguments, None, &[]);
    let onto = git(&["rev-list", "--first-parent", "-n", LOOKS_BACK, theirs])
        .unwrap_or_default()
        .lines()
        .find(|head| kept.contains(*head))
        .map(str::to_owned)
        .ok_or("no lent builder keeps a recent main evaluated")?;
    let change = format!("{base}..{commit}");
    if !git(&["rev-list", "--merges", &change])
        .ok_or("git couldn't list the change's commits")?
        .is_empty()
    {
        return Err("the change holds a merge".to_owned());
    }
    let names = |from: &str, to: &str| {
        git(&["diff", "--no-renames", "--name-only", from, to])
            .map(|names| names.lines().map(str::to_owned).collect::<Vec<_>>())
            .ok_or("git couldn't say what moved")
    };
    if let Some(why) = refusal(&names(&onto, base)?, &names(base, commit)?) {
        return Err(why);
    }
    let commits = git(&["rev-list", "--reverse", &change]).unwrap_or_default();
    let mut parent = onto.clone();
    for each in commits.lines() {
        parent = applied(top, each, &parent, scratch)?;
    }
    Ok(Some(Replayed {
        commit: parent,
        onto,
    }))
}

/// `commit` applied onto `parent` as a commit of its own, by the same
/// people, at the same times, with the same message.
fn applied(top: &Path, commit: &str, parent: &str, scratch: &Path) -> Result<String, String> {
    let git =
        |arguments: &[&str], vars: &[(&str, &str)]| there::git_with(top, arguments, None, vars);
    let short = commit.get(..12).unwrap_or(commit);
    let merged_base = format!("{commit}^");
    let tree = git(
        &[
            "merge-tree",
            "--write-tree",
            "--merge-base",
            &merged_base,
            parent,
            commit,
        ],
        &[],
    )
    .ok_or_else(|| format!("{short} doesn't apply cleanly"))?;
    let tree = tree.lines().next().unwrap_or_default().to_owned();
    let said = git(
        &[
            "show",
            "-s",
            "--format=%an%x00%ae%x00%aI%x00%cn%x00%ce%x00%cI",
            commit,
        ],
        &[],
    )
    .ok_or_else(|| format!("{short} couldn't be read"))?;
    let people: Vec<(&str, &str)> = PEOPLE.into_iter().zip(said.split('\0')).collect();
    if people.len() != PEOPLE.len() {
        return Err(format!("{short}'s people couldn't be read"));
    }
    let message = git(&["show", "-s", "--format=%B", commit], &[])
        .ok_or_else(|| format!("{short}'s message couldn't be read"))?;
    fs::create_dir_all(scratch).map_err(|error| format!("no scratch to replay in: {error}"))?;
    let file = scratch.join(format!("replay-{}-{short}", std::process::id()));
    fs::write(&file, format!("{message}\n")).map_err(|error| format!("{error}"))?;
    let made = git(
        &[
            "commit-tree",
            &tree,
            "-p",
            parent,
            "-F",
            &file.to_string_lossy(),
        ],
        &people,
    );
    let _ = fs::remove_file(&file);
    made.ok_or_else(|| format!("{short} couldn't be committed again"))
}

#[cfg(test)]
mod tests;
