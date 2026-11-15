//! The misses Joel found himself, and whether the check each one's owner
//! named has merged: a miss closes on the merge itself, read from the
//! repository's main, never on the owner's word that a check exists.

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;

use super::{Class, Measure, count, first_owner, lines};
use crate::command;

/// A change a check names, such as `core #4568`: its repository, as cadence
/// calls it, and its number there.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Named {
    pub repo: String,
    pub number: u64,
}

/// The changes `check` names in core or the harness, the two whose mains
/// cadence mirrors, so only theirs can be seen to have merged: each `#N`
/// after the repository's name, as `core #4568`, `core#4500` or
/// `harness #1592`.
fn named(check: &str) -> Vec<Named> {
    check
        .match_indices('#')
        .filter_map(|(at, _)| {
            let digits: String = check[at + 1..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            let word = check[..at]
                .trim_end()
                .rsplit(|c: char| c.is_whitespace() || c == '(' || c == ',')
                .next()?
                .to_lowercase();
            let repo = match word.as_str() {
                "core" | "core.nix" => "core",
                "harness" | "agentic-harness.pi" => "harness",
                _ => return None,
            };
            Some(Named {
                repo: repo.to_owned(),
                number: digits.parse().ok()?,
            })
        })
        .collect()
}

/// Every change the misses' checks name in the mirrored repositories, so
/// each can be looked for once among the merges.
#[must_use]
pub fn named_in(text: &str) -> BTreeSet<Named> {
    lines(text)
        .filter_map(|miss| miss["check"].as_str().map(named))
        .flatten()
        .collect()
}

/// Those of `named` merged on their repository's main, as its mirror last
/// fetched it: a merge commit saying it merged the pull request. The whole
/// history is searched, so a fix stays closed however long ago it landed.
pub fn merged(named: &BTreeSet<Named>, core: &Path, harness: &Path) -> BTreeSet<Named> {
    named
        .iter()
        .filter(|change| {
            let mirror = if change.repo == "core" { core } else { harness };
            let grep = format!("--grep=Merge pull request #{} from", change.number);
            command(
                "git",
                &[
                    "-C",
                    &mirror.to_string_lossy(),
                    "log",
                    "--first-parent",
                    "-F",
                    &grep,
                    "-1",
                    "--format=%H",
                    "main",
                ],
            )
            .is_ok_and(|found| !found.trim().is_empty())
        })
        .cloned()
        .collect()
}

/// The misses Joel found himself, from the orchestrator's `misses.jsonl`:
/// one stays open until its owner names the check that would have caught
/// it and that check has merged, as `merged` holds them. A check naming no
/// change cadence can see merge, such as a memory or World's, leaves it
/// open, since its words alone would tell Joel it's fixed before it is.
#[must_use]
pub fn misses(text: &str, merged: &BTreeSet<Named>) -> Measure {
    let closed = |miss: &Value| {
        let changes = miss["check"].as_str().map(named).unwrap_or_default();
        !changes.is_empty() && changes.iter().all(|change| merged.contains(change))
    };
    let open: Vec<(String, String)> = lines(text)
        .filter(|miss| !closed(miss))
        .map(|miss| {
            let what = miss["what"]
                .as_str()
                .or_else(|| miss["words"].as_str())
                .unwrap_or_default()
                .to_owned();
            (
                what,
                first_owner(miss["owner"].as_str().unwrap_or_default()),
            )
        })
        .collect();
    Measure {
        key: "misses".to_owned(),
        class: Class::Miss,
        says: "misses Joel found himself, with no merged check yet to catch the next".to_owned(),
        now: count(open.len()),
        target: Some(0.0),
        // The oldest open miss's owner moves it first.
        owner: open.first().map(|(_, o)| o.clone()).unwrap_or_default(),
        behind: open
            .iter()
            .map(|(what, owner)| format!("{what} · {owner}"))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(repo: &str, number: u64) -> Named {
        Named {
            repo: repo.to_owned(),
            number,
        }
    }

    #[test]
    fn a_miss_stays_open_until_a_check_is_named_for_it() {
        let text = concat!(
            r#"{"words":"w","what":"the map had no network","owner":"KNURBN (VKCF3Z)","check":null}"#,
            "\n",
            r#"{"words":"w","what":"History paused","owner":"RCTRF8","check":"core#4500"}"#,
            "\n",
        );
        let merged = BTreeSet::from([change("core", 4500)]);
        let m = misses(text, &merged);
        assert!((m.now - 1.0).abs() < 0.5, "one open, got {}", m.now);
        assert_eq!(m.owner, "KNURBN", "its first owner moves it");
        assert_eq!(
            m.behind,
            ["the map had no network · KNURBN"],
            "it says what it was"
        );
    }

    #[test]
    fn a_miss_whose_check_has_not_merged_stays_open() {
        let miss = |check: &str| {
            serde_json::json!({"words": "w", "what": "the cat waits", "owner": "XY9H5C",
                "check": check})
            .to_string()
        };
        let two = format!(
            "{}\n{}\n",
            miss("core #4568 (the deleted-row check; merge ~07:30)"),
            miss("rule memory #5331, plus harness #1592 (mesh gate), merge ~06:00")
        );
        assert_eq!(
            named_in(&two),
            BTreeSet::from([change("core", 4568), change("harness", 1592)]),
            "a check's changes in the mirrored repositories weren't each found"
        );
        let open = misses(&two, &BTreeSet::new());
        assert_eq!(
            open.behind,
            ["the cat waits · XY9H5C", "the cat waits · XY9H5C"],
            "a miss closed on a check named but not merged"
        );
        let one = misses(&two, &BTreeSet::from([change("harness", 1592)]));
        assert!(
            (one.now - 1.0).abs() < 0.5,
            "a merged check didn't close its miss, or an unmerged one did: {}",
            one.now
        );
        let elsewhere = misses(&miss("World #2450121"), &BTreeSet::new());
        assert!(
            (elsewhere.now - 1.0).abs() < 0.5,
            "a check cadence can't see merge closed its miss"
        );
    }
}
