//! The lists each change adds a line to, and what one shared by a
//! rebase still needs before it merges: only its own check, which takes
//! a minute where changed-checks takes ten.

use std::process::ExitCode;

use crate::answers::{Verdict, keep};
use crate::state::short;

/// A list each change adds a line to, which on a busy day main moves in
/// the minutes a run at a rebased head takes: on 9 November #2576 was
/// refused five times after a green run, each time because main had moved
/// check-descriptions.nix or quarantine.json meanwhile.
#[derive(Debug, PartialEq, Eq)]
pub enum List {
    /// Read by the one check named, which says whether the list holds.
    Judged(String),
    /// The net's quarantine list, which no check builds from: the net,
    /// changed-checks and run-subtests read it from main, so it holds
    /// when it reads as a list of tests, each held whole once or each of
    /// its subtests once, however many owners' entries hold them.
    Quarantine,
}

/// The list at `path`, or none for a file that isn't one.
pub fn list(path: &str) -> Option<List> {
    if path == crate::world::QUARANTINE {
        return Some(List::Quarantine);
    }
    if path == "check-descriptions.nix" {
        return Some(List::Judged("every-check-says-what-it-proves".to_owned()));
    }
    path.strip_prefix("tests/rules/")
        .and_then(|name| name.strip_suffix("-waiting.nix"))
        .filter(|check| !check.contains('/'))
        .map(|check| List::Judged(check.to_owned()))
}

/// What shared lists say of a head, when every shared file is one.
#[derive(Debug, PartialEq, Eq)]
pub enum Lists {
    /// Some shared file is no list, so changed-checks runs again.
    NotAll,
    /// Each list holds, and these checks of theirs haven't passed at the
    /// head yet; none left means it may merge.
    Need(Vec<String>),
    /// The quarantine list no longer reads as one, saying why.
    Broken(String),
}

/// Whether `shared` are all lists, and what they still need at a head
/// whose `record` says what passed there and whose quarantine list holds
/// `quarantine`. A list's own check is all that reads it, so it's all
/// that runs again.
pub fn lists(shared: &[String], record: &str, quarantine: &str) -> Lists {
    let passed: Vec<String> = record
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|line| line["dir"].is_null())
        .filter_map(|line| line["check"].as_str().map(str::to_owned))
        .collect();
    let mut need = Vec::new();
    for file in shared {
        match list(file) {
            None => return Lists::NotAll,
            Some(List::Quarantine) => {
                if let Err(why) = holds_tests(quarantine) {
                    return Lists::Broken(why);
                }
            }
            Some(List::Judged(check)) => {
                let built = passed
                    .iter()
                    .any(|run| run.starts_with("checks.") && run.ends_with(&format!(".{check}")));
                if !built && !need.contains(&check) {
                    need.push(check);
                }
            }
        }
    }
    Lists::Need(need)
}

/// What `ready` says of `commit`, when every file it shares with main's
/// moves is a list each change adds to: whether each holds there with its
/// own check passed, as its record and `quarantine`, the net's list there,
/// say, merging by `passes` when it may, which holds a ready one back
/// while an earlier gate runs. None when some file isn't one.
pub fn say(
    commit: &str,
    shared: &[String],
    quarantine: &str,
    passes: &dyn Fn(&str) -> ExitCode,
) -> Option<ExitCode> {
    match lists(shared, &crate::ready::record(commit), quarantine) {
        Lists::Need(need) if need.is_empty() => {
            return Some(passes(&format!(
                "ready: {} sits on origin's main, and main moved only lists of its ({}), whose \
                 own checks passed there",
                short(commit),
                shared.join(", ")
            )));
        }
        Lists::Need(need) => {
            eprintln!(
                "merge-net: main has since moved only lists of {} ({}); build {} there, \
                 committed so it's sent, and merge again",
                short(commit),
                shared.join(", "),
                need.iter()
                    .map(|check| format!("`nix-offload there . checks.{}.{check}`", crate::SYSTEM))
                    .collect::<Vec<_>>()
                    .join(" and ")
            );
            keep(commit, Verdict::Regate, shared);
        }
        Lists::Broken(why) => eprintln!(
            "merge-net: {} at {} no longer reads as the net's list: {why}",
            crate::world::QUARANTINE,
            short(commit)
        ),
        Lists::NotAll => return None,
    }
    Some(ExitCode::FAILURE)
}

/// Whether the quarantine list reads as a list of tests, each held whole
/// in one entry alone, or each of its subtests in one entry: a test has
/// an entry for each owner (quarantine.rs), but two holding the same
/// subtest, or a whole test beside another, leave who owns it unsaid.
fn holds_tests(quarantine: &str) -> Result<(), String> {
    let entries: Vec<serde_json::Value> =
        serde_json::from_str(quarantine).map_err(|error| format!("it isn't a list: {error}"))?;
    let mut held: Vec<(&str, Option<&str>)> = Vec::new();
    for entry in &entries {
        let test = entry["test"]
            .as_str()
            .ok_or_else(|| format!("an entry names no test: {entry}"))?;
        // An entry naming no subtest holds the whole test, as quarantine.rs reads it.
        let mut subtests: Vec<Option<&str>> = entry["subtests"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(Some)
            .collect();
        let whole = subtests.is_empty();
        if whole {
            subtests.push(None);
        }
        if held
            .iter()
            .any(|(kept, part)| *kept == test && (whole || part.is_none()))
        {
            return Err(format!("{test} is on it whole and again"));
        }
        for subtest in subtests {
            if held.contains(&(test, subtest)) {
                let subtest = subtest.unwrap_or_default();
                return Err(format!("{test}'s {subtest} is on it twice"));
            }
            held.push((test, subtest));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn shared_lists_need_only_their_own_checks() {
        let quarantine = "[{\"test\":\"vm-a\"},{\"test\":\"vm-b\"}]";
        let shared = owned(&[
            "check-descriptions.nix",
            "tests/rules/every-fixed-wait-says-why-waiting.nix",
            "tools/merge-net/quarantine.json",
        ]);
        assert_eq!(
            lists(&shared, "", quarantine),
            Lists::Need(owned(&[
                "every-check-says-what-it-proves",
                "every-fixed-wait-says-why"
            ]))
        );
        let record = "{\"check\":\"checks.x86_64-linux.every-fixed-wait-says-why\",\"dir\":null}\n\
                      {\"check\":\"checks.x86_64-linux.every-check-says-what-it-proves\",\
                      \"dir\":null}\n";
        assert_eq!(lists(&shared, record, quarantine), Lists::Need(Vec::new()));
        // A check passed in another folder of the repository says nothing.
        let elsewhere =
            "{\"check\":\"checks.x86_64-linux.every-fixed-wait-says-why\",\"dir\":\"sub\"}\n";
        assert_eq!(
            lists(
                &owned(&["tests/rules/every-fixed-wait-says-why-waiting.nix"]),
                elsewhere,
                ""
            ),
            Lists::Need(owned(&["every-fixed-wait-says-why"]))
        );
        // One shared file that's no list, and changed-checks runs again.
        let mixed = owned(&["check-descriptions.nix", "checks.nix"]);
        assert_eq!(lists(&mixed, record, quarantine), Lists::NotAll);
        assert_eq!(
            lists(
                &owned(&["tests/rules/lib/waiting-waiting.nix/x"]),
                record,
                quarantine
            ),
            Lists::NotAll,
            "a file in a folder under tests/rules/ isn't a list"
        );
        assert_eq!(
            lists(
                &owned(&["tests/every-fixed-wait-says-why-waiting.nix"]),
                record,
                quarantine
            ),
            Lists::NotAll,
            "a waiting list outside tests/rules/ isn't one the net knows"
        );
    }

    #[test]
    fn a_quarantine_list_that_no_longer_reads_as_one_is_broken() {
        let shared = owned(&["tools/merge-net/quarantine.json"]);
        assert_eq!(
            lists(&shared, "", "[{\"test\":\"vm-a\"}]"),
            Lists::Need(Vec::new())
        );
        let owners = r#"[{"test":"vm-a","owner_pr":1,"subtests":["test_p"]},
                         {"test":"vm-a","owner_pr":2,"subtests":["test_q"]}]"#;
        assert_eq!(
            lists(&shared, "", owners),
            Lists::Need(Vec::new()),
            "a test listed by two owners, each for its own subtests, read as broken"
        );
        for (merged, why) in [
            (
                "[{\"test\":\"vm-a\"}\n{\"test\":\"vm-b\"}]",
                "it isn't a list",
            ),
            (
                "[{\"test\":\"vm-a\"},{\"test\":\"vm-a\",\"subtests\":[\"test_p\"]}]",
                "vm-a is on it whole and again",
            ),
            (
                r#"[{"test":"vm-a","subtests":["test_p"]},
                    {"test":"vm-a","subtests":["test_q","test_p"]}]"#,
                "vm-a's test_p is on it twice",
            ),
            ("[{\"reason\":\"x\"}]", "an entry names no test"),
        ] {
            let Lists::Broken(said) = lists(&shared, "", merged) else {
                panic!("{merged} read as a list");
            };
            assert!(said.starts_with(why), "{merged}: {said}");
        }
    }
}
