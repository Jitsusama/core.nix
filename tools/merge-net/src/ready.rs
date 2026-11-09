//! `merge-net ready COMMIT`: whether a branch's head may merge, asked
//! before merging by whatever merges it. It may when it sits on origin's
//! main as it is now, and changed-checks built what it named there.
//!
//! On 8 November four checks were red on main by 11:00. changed-checks
//! named each at its merge, but one branch was built before main gained
//! what it collided with, and the others built only some of what was named,
//! or nothing. Both show in nix-offload's record of what passed at each
//! commit (`~/.local/state/nix-offload/passed/COMMIT`), which keeps a line
//! for each changed-checks run sent that passed, with its arguments.

use std::env;
use std::fs;
use std::path::PathBuf;

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
    /// The same change passed at this commit, but main has since moved
    /// these of its files, so changed-checks runs again at the head.
    Shares(String, Vec<String>),
    /// No commit of the same change passed.
    Unchecked,
}

/// Whether the head, changing `files`, is the same change as one of
/// `candidates`, newest first, that a changed-checks run passed at (its
/// `record`, judged as `built_there` does with `on_main`). `same` says,
/// of a candidate, whether it makes the same change and, if so, which
/// files main moved since the base it was checked on. On 8 November a
/// rule let a change merge after a rebase onto main without running its
/// checks again when main's new commits share none of its files, so the
/// gate takes what the rule takes. When they share some, only changed-checks
/// at the head counts, since which checks read a file shows only in their
/// derivations: on 9 November #2504 merged with words.json's checks rerun
/// at its head after main moved two other of its files, which no check
/// that ran read. Nix builds only what the rebase moved, and takes the
/// rest from the builders.
pub fn checked_before(
    candidates: &[String],
    files: &[String],
    record: &dyn Fn(&str) -> String,
    on_main: &dyn Fn(&str) -> bool,
    same: &dyn Fn(&str) -> Option<Vec<String>>,
) -> Before {
    let mut first_shared = None;
    for candidate in candidates {
        if !built_there(&record(candidate), on_main) {
            continue;
        }
        let Some(moved) = same(candidate) else {
            continue;
        };
        let shared: Vec<String> = moved
            .into_iter()
            .filter(|file| files.contains(file))
            .collect();
        if shared.is_empty() {
            return Before::Clean(candidate.clone());
        }
        first_shared.get_or_insert(Before::Shares(candidate.clone(), shared));
    }
    first_shared.unwrap_or(Before::Unchecked)
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

/// The file nix-offload keeps what passed at `commit` in.
pub fn record_of(commit: &str) -> Option<PathBuf> {
    let state = env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))?;
    Some(state.join("nix-offload/passed").join(commit))
}

/// What nix-offload kept as passed at `commit`, or nothing.
pub fn record(commit: &str) -> String {
    record_of(commit)
        .and_then(|file| fs::read_to_string(file).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAIN: &str = "aaaa";

    fn on_main(commit: &str) -> bool {
        commit == MAIN
    }

    #[test]
    fn a_run_that_built_what_it_named_is_ready() {
        let record = "{\"check\":\"checks.x86_64-linux.b\",\"dir\":null}\n\
                      {\"dir\":null,\"printed\":\"changed  b\\n\",\
                      \"run\":[\"--system\",\"x86_64-linux\"]}\n";
        assert!(built_there(record, &on_main));
    }

    #[test]
    fn a_run_that_only_named_or_kept_a_base_is_not() {
        let named = "{\"dir\":null,\"printed\":\"\",\
                     \"run\":[\"--name-only\",\"--system\",\"x86_64-linux\"]}\n\
                     {\"dir\":null,\"printed\":\"\",\
                     \"run\":[\"--keep-base\",\"--system\",\"x86_64-linux\"]}\n\
                     {\"check\":\"checks.x86_64-linux.b\",\"dir\":null}\n";
        assert!(!built_there(named, &on_main));
        assert!(!built_there("", &on_main), "nothing passed there");
    }

    #[test]
    fn a_run_against_a_base_main_lacks_is_not() {
        let run = |base: &str| {
            format!(
                "{{\"dir\":null,\"printed\":\"\",\"run\":[\"--base\",\"{base}\",\
                 \"--system\",\"x86_64-linux\"]}}\n"
            )
        };
        assert!(built_there(&run(MAIN), &on_main));
        assert!(
            !built_there(&run("bbbb"), &on_main),
            "a base on the branch skips what came before it"
        );
    }

    #[test]
    fn the_same_change_checked_before_a_clean_rebase_is_ready() {
        let passed = "{\"dir\":null,\"printed\":\"\",\"run\":[\"--system\",\"x86_64-linux\"]}\n";
        let record = |commit: &str| {
            if commit == "old" || commit == "older" {
                passed.to_owned()
            } else {
                String::new()
            }
        };
        let files = vec!["a.rs".to_owned(), "b.md".to_owned()];
        let candidates = vec!["other".to_owned(), "old".to_owned(), "older".to_owned()];
        // Main moved other files since: the rebase changed nothing checked.
        let moved = |commit: &str| (commit == "old").then(|| vec!["c.nix".to_owned()]);
        assert_eq!(
            checked_before(&candidates, &files, &record, &on_main, &moved),
            Before::Clean("old".to_owned())
        );
        // Main moved one of its files: changed-checks runs again at the
        // head, whatever single checks passed there.
        let moved =
            |commit: &str| (commit == "old").then(|| vec!["b.md".to_owned(), "c.nix".to_owned()]);
        assert_eq!(
            checked_before(&candidates, &files, &record, &on_main, &moved),
            Before::Shares("old".to_owned(), vec!["b.md".to_owned()])
        );
        // Only single checks passed at a later commit of the same change:
        // they don't say what read the shared file.
        let rerun = "{\"check\":\"checks.x86_64-linux.formatting\",\"dir\":null}\n";
        let record_rechecked = |commit: &str| match commit {
            "old" => passed.to_owned(),
            "rebased" => rerun.to_owned(),
            _ => String::new(),
        };
        let candidates = vec!["rebased".to_owned(), "old".to_owned()];
        let moved = |commit: &str| match commit {
            "old" => Some(vec!["b.md".to_owned(), "c.nix".to_owned()]),
            "rebased" => Some(vec!["c.nix".to_owned()]),
            _ => None,
        };
        assert_eq!(
            checked_before(&candidates, &files, &record_rechecked, &on_main, &moved),
            Before::Shares("old".to_owned(), vec!["b.md".to_owned()])
        );
        // changed-checks passed at that later commit, and main moved none of
        // the change's files since its base.
        let record_rerun = |commit: &str| match commit {
            "old" | "rebased" => passed.to_owned(),
            _ => String::new(),
        };
        assert_eq!(
            checked_before(&candidates, &files, &record_rerun, &on_main, &moved),
            Before::Clean("rebased".to_owned())
        );
        let candidates = vec!["other".to_owned(), "old".to_owned(), "older".to_owned()];
        // An earlier commit of another change, or the same change that
        // never passed, says nothing.
        let moved = |commit: &str| (commit == "other").then(Vec::new);
        assert_eq!(
            checked_before(&candidates, &files, &record, &on_main, &moved),
            Before::Unchecked
        );
    }

    #[test]
    fn a_run_in_another_folder_of_the_repository_is_not() {
        let record = "{\"dir\":\"sub\",\"printed\":\"\",\"run\":[\"--system\",\"x86_64-linux\"]}\n";
        assert!(!built_there(record, &on_main));
    }
}
