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
    fn a_run_in_another_folder_of_the_repository_is_not() {
        let record = "{\"dir\":\"sub\",\"printed\":\"\",\"run\":[\"--system\",\"x86_64-linux\"]}\n";
        assert!(!built_there(record, &on_main));
    }
}
