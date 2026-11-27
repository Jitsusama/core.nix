//! `merge-net ready COMMIT`: whether a branch's head may merge, asked
//! before merging by whatever merges it. It may when it sits on origin's
//! main as it is now, changed-checks built what it named there, and
//! `needs` was asked of the change (`gated.rs`). A head behind main may
//! merge as it stands, as a merge commit, when main moved none of its
//! files since its base, as a rebased head's earlier run is judged
//! ([`speakers`]).
//!
//! On 8 November four checks were red on main by 11:00. changed-checks
//! named each at its merge, but one branch was built before main gained
//! what it collided with, and the others built only some of what was named,
//! or nothing. Both show in nix-offload's record of what passed at each
//! commit (`/var/lib/nix-offload-passed/COMMIT` where its keeper stands,
//! else `~/.local/state/nix-offload/passed/COMMIT`), which keeps a line
//! for each changed-checks run sent that passed, with its arguments.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::apart::{
    LISTS, apart, cases_apart, checks_apart, descriptions_apart, is_cases, is_prose, lines_apart,
    prose_apart,
};

/// Whether nix-offload's record for a commit holds a changed-checks run that
/// passed there and built what it named, against a base `on_main` says
/// origin's main holds, so it named everything a merge onto main moves.
/// A run that only named or only kept a base built nothing.
pub fn built_there(record: &str, on_main: &dyn Fn(&str) -> bool) -> bool {
    record
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|line| line["dir"].is_null())
        .filter_map(|line| line["run"].as_array().cloned())
        .any(|run| {
            let words: Vec<&str> = run.iter().filter_map(serde_json::Value::as_str).collect();
            let built = !words
                .iter()
                .any(|word| matches!(*word, "--name-only" | "--keep-base"));
            let base = words
                .iter()
                .position(|word| *word == "--base")
                .and_then(|at| words.get(at + 1));
            built && base.is_none_or(|base| on_main(base))
        })
}

/// What an earlier commit of the same change says of a head with no run of
/// its own.
#[derive(Debug, PartialEq, Eq)]
pub enum Before {
    /// The same change passed at this commit, and main has since moved
    /// none of its files, so the rebase changed nothing it was checked on.
    Clean(String),
    /// The same change passed at this commit, the newest that did, but
    /// main has since moved these of its files, so changed-checks runs
    /// again at the head or the train checks it merged, however many
    /// earlier commits passed.
    Shares(String, Vec<String>),
    /// No commit of the same change passed.
    Unchecked,
}

impl Before {
    /// Whether this earlier pass lets the head merge as it stands, with no
    /// run of its own or the train's: only when main moved none of its
    /// files, since no run saw what main moved.
    pub fn speaks_for_the_head(&self) -> bool {
        matches!(self, Self::Clean(_))
    }
}

/// How many passes of the same change sharing files with main's moves are
/// read before the newest is answered: what main moved since a base is a
/// diff of hundreds of files, so a day of a change's passes isn't each
/// read. Candidates come newest record first (a head behind main before
/// them, as [`speakers`] puts it), not newest base, so a clean
/// pass past these two can go unread; that costs the head a run at most,
/// since the cap only ever turns a `Clean` into `Shares`, never the other
/// way.
const SHARING_READ: usize = 2;

/// Whether the head is the same change as one of `candidates`, newest
/// first, that a changed-checks run passed at (its `record`, judged as
/// `built_there` does with `on_main`). `same` says, of a candidate,
/// whether it makes the same change and, if so, which files it shares
/// with what main moved since the base it was checked on, as `shared`
/// works them out. On 8 November a
/// rule let a change merge after a rebase onto main without running its
/// checks again when main's new commits share none of its files, so the
/// gate takes what the rule takes. When they share some, only changed-checks
/// at the head counts, since which checks read a file shows only in their
/// derivations: on 9 November #2504 merged with words.json's checks rerun
/// at its head after main moved two other of its files, which no check
/// that ran read. Nix builds only what the rebase moved, and takes the
/// rest from the builders. Passing at more commits speaks no louder: no
/// run saw what main moved since, and at 11:07 on 26 November #6727,
/// passed at two commits, merged as it stood after main moved a file it
/// shared, so formatting went red on main. A change main outruns boards
/// the train, whose candidate run checks the merged tree.
pub fn checked_before(
    candidates: &[String],
    record: &dyn Fn(&str) -> String,
    on_main: &dyn Fn(&str) -> bool,
    same: &dyn Fn(&str) -> Option<Vec<String>>,
) -> Before {
    let mut newest_shared: Option<(String, Vec<String>)> = None;
    let mut sharing = 0;
    for candidate in candidates {
        if sharing == SHARING_READ {
            break;
        }
        if !built_there(&record(candidate), on_main) {
            continue;
        }
        let Some(shared) = same(candidate) else {
            continue;
        };
        if shared.is_empty() {
            return Before::Clean(candidate.clone());
        }
        sharing += 1;
        newest_shared.get_or_insert((candidate.clone(), shared));
    }
    newest_shared.map_or(Before::Unchecked, |(newest, shared)| {
        Before::Shares(newest, shared)
    })
}

/// The commits whose changed-checks runs may speak for `head`, newest
/// first, from the `candidates` of the same change: `head` itself first
/// when it's `behind` origin's main, since its own run checked it against
/// the main it branched from, as an earlier commit's run checks a rebased
/// head. A head on main with a run of its own is judged by it before
/// this, so it never speaks for itself here.
pub fn speakers(head: &str, behind: bool, candidates: &[String]) -> Vec<String> {
    let others = candidates
        .iter()
        .filter(|candidate| *candidate != head)
        .cloned();
    if behind {
        std::iter::once(head.to_owned()).chain(others).collect()
    } else {
        others.collect()
    }
}

/// What `then` makes of `candidate`'s base (`base_of`), only when the
/// change from that base to it has the patch id `change` (`id_of`).
/// Nearly every candidate is another change, and its id, from a diff of
/// that change alone, tells it apart in milliseconds, while what main
/// moved since a base hours old is a diff of hundreds of files, with
/// renames found. On 9 November the gate read that for each of a day's
/// 1,245 passed records before it compared ids, so `ready` took five
/// minutes and the review tool's merge gave up on #2687.
pub fn when_same<T>(
    candidate: &str,
    change: &str,
    base_of: impl Fn(&str) -> Option<String>,
    id_of: impl Fn(&str, &str) -> Option<String>,
    then: impl FnOnce(String) -> Option<T>,
) -> Option<T> {
    let base = base_of(candidate)?;
    if id_of(&base, candidate).as_deref() != Some(change) {
        return None;
    }
    then(base)
}

/// One side of a rebase: the files it changed, the paths it renamed or
/// deleted, and what a file holds on that side.
pub struct Side<'a> {
    pub changed: &'a [String],
    pub gone: &'a [String],
    pub text: &'a dyn Fn(&str) -> String,
}

/// The files a branch shares with what main moved since its base: those
/// both changed, and those either side changed that name a path the other
/// renamed or deleted. On 9 November #2597 renamed a script after #2613,
/// merged since its run, named the old path in a list #2597 never touched,
/// and main broke when #2597 merged with no run of its own.
pub fn shared(branch: &Side, main: &Side) -> Vec<String> {
    let mut shared: Vec<String> = main
        .changed
        .iter()
        .filter(|file| branch.changed.contains(file))
        .cloned()
        .collect();
    for file in names_gone(branch, main) {
        if !shared.contains(&file) {
            shared.push(file);
        }
    }
    shared
}

/// The files either side changed that name a path the other renamed or
/// deleted, which no earlier run speaks for, since the run that reads the
/// old path never saw it gone.
fn names_gone(branch: &Side, main: &Side) -> Vec<String> {
    let mut naming: Vec<String> = Vec::new();
    for (side, other) in [(main, branch), (branch, main)] {
        for file in side.changed {
            let text = (side.text)(file);
            if !naming.contains(file) && other.gone.iter().any(|path| names(&text, file, path)) {
                naming.push(file.clone());
            }
        }
    }
    naming
}

/// Whether `text`, the file at `file`, names `path`: whole, from the
/// repository's root, or from the file's own folder, as a Nix path does.
fn names(text: &str, file: &str, path: &str) -> bool {
    let folder = file.rfind('/').map_or("", |at| &file[..=at]);
    text.contains(path)
        || path
            .strip_prefix(folder)
            .is_some_and(|near| !folder.is_empty() && text.contains(&format!("./{near}")))
}

/// Whether `file`, which the branch shares with main, is a JSON file,
/// check-descriptions.nix, a folder's checks.nix, a list of modules, a
/// pytest file or a note in docs/, both changed `apart`, so it stands as
/// two files would.
/// One naming a path either side renamed or deleted stays shared, as
/// `shared` holds. `base` gives a file where the branch was checked from,
/// and `checked` where it was checked at.
pub fn stands_apart(
    file: &str,
    branch: &Side,
    main: &Side,
    base: &dyn Fn(&str) -> String,
    checked: &dyn Fn(&str) -> String,
) -> bool {
    let json = std::path::Path::new(file)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
    let checks = file == "checks.nix" || file.ends_with("/checks.nix");
    let read: fn(&str, &str, &str, &str) -> bool = if file == "check-descriptions.nix" {
        descriptions_apart
    } else if file == "checks.nix" {
        // The root's checks.nix gives checks and lists every folder's.
        |base, branch, main, head| {
            checks_apart(base, branch, main, head) || lines_apart(base, branch, main, head)
        }
    } else if LISTS.contains(&file) {
        lines_apart
    } else if checks {
        checks_apart
    } else if is_cases(file) {
        cases_apart
    } else if is_prose(file) {
        prose_apart
    } else if json {
        apart
    } else {
        return false;
    };
    let both = [branch, main]
        .iter()
        .all(|side| side.changed.iter().any(|changed| changed == file));
    if !both {
        return false;
    }
    let head = (branch.text)(file);
    let names_gone = branch
        .gone
        .iter()
        .chain(main.gone)
        .any(|path| head.contains(path.as_str()));
    !names_gone && read(&base(file), &checked(file), &(main.text)(file), &head)
}

/// The commits nix-offload kept a passed record for in the last day,
/// newest first.
pub fn recent_records() -> Vec<String> {
    let Some(folder) = record_of("") else {
        return Vec::new();
    };
    let day_ago = std::time::SystemTime::now() - std::time::Duration::from_hours(24);
    let mut kept: Vec<(std::time::SystemTime, String)> = fs::read_dir(folder)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let at = entry.metadata().ok()?.modified().ok()?;
            let name = entry.file_name().into_string().ok()?;
            (at >= day_ago && name.len() == 40).then_some((at, name))
        })
        .collect();
    kept.sort_by_key(|(at, _)| std::cmp::Reverse(*at));
    kept.into_iter().map(|(_, name)| name).collect()
}

/// Where nix-offload's keeper keeps what passed, which only it may change,
/// as nix-offload's `keep::FOLDER` names it.
const KEPT: &str = "/var/lib/nix-offload-passed";

/// The keeper's socket unit, as `keep::UNIT` names it: there while the
/// machine runs the keeper, so a folder one left behind isn't read.
const KEEPER: &str = "/etc/systemd/system/nix-offload-keep.socket";

/// nix-offload's folder under the account's state.
pub fn offload_state() -> Option<PathBuf> {
    let state = env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))?;
    Some(state.join("nix-offload"))
}

/// The file nix-offload keeps what passed at `commit` in: its keeper's,
/// where the machine has one, else the account's own.
pub fn record_of(commit: &str) -> Option<PathBuf> {
    if Path::new(KEEPER).exists() {
        return Some(Path::new(KEPT).join(commit));
    }
    Some(offload_state()?.join("passed").join(commit))
}

/// What nix-offload kept as passed at `commit`, or nothing.
pub fn record(commit: &str) -> String {
    record_of(commit)
        .and_then(|file| fs::read_to_string(file).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
