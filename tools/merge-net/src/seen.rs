//! Whether one of our review rounds read a head before it merges. From
//! Tuesday 17 November 18:00 to Thursday, 240 of 429 reviewed pull
//! requests merged at a head no council or judge had read: a fix pushed
//! after the round went to main unread.
//!
//! The harness keeps each change's rounds in its review state, one file a
//! change, each round naming the commit it was asked of as its witness.
//! A round speaks for a head only when it's a council or a stack round,
//! whose reviewers read the code (a judge reads their findings), they read
//! that commit (no `unpinned`), it was collected (neither `open` nor
//! `closed`), and one of them answered. The harness's own merge reads its
//! records the same way (`readSomething` in its review store).

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// Where the harness keeps its review rounds: `MERGE_NET_REVIEW_RUNS`, or
/// its state folder's `review/runs`.
pub fn rounds_folder() -> PathBuf {
    if let Some(folder) = std::env::var_os("MERGE_NET_REVIEW_RUNS").filter(|f| !f.is_empty()) {
        return PathBuf::from(folder);
    }
    let state = std::env::var_os("XDG_STATE_HOME")
        .filter(|f| !f.is_empty())
        .map_or_else(
            || PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state"),
            PathBuf::from,
        );
    state.join("pi/agentic-harness.pi/review/runs")
}

/// Every commit a round kept under `folder` read in full. A folder that
/// can't be read is said so, since a council asked at the head wouldn't
/// help where the net can't see the rounds at all, as on a machine where
/// the harness keeps its review state elsewhere.
pub fn witnessed(folder: &Path) -> Result<HashSet<String>, String> {
    let files = fs::read_dir(folder).map_err(|error| {
        format!(
            "the review rounds at {} can't be read ({error}), so no head can be shown read; \
             point MERGE_NET_REVIEW_RUNS at the harness's review/runs",
            folder.display()
        )
    })?;
    Ok(files
        .flatten()
        .filter(|file| file.path().extension().is_some_and(|e| e == "json"))
        .filter_map(|file| fs::read_to_string(file.path()).ok())
        .flat_map(|text| read_in(&text))
        .collect())
}

/// The commits the rounds of one change's file `text` read in full.
fn read_in(text: &str) -> Vec<String> {
    let Ok(change) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    let Some(rounds) = change["runs"].as_array() else {
        return Vec::new();
    };
    rounds
        .iter()
        .filter(|round| speaks(round))
        .flat_map(|round| {
            round["witness"]
                .as_str()
                .into_iter()
                .chain(
                    round["witnesses"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str),
                )
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Whether `round` is a council or stack round that was collected, whose
/// reviewers read the commit it names, and one of whom answered. One
/// stopped at its wall still said what it had.
fn speaks(round: &Value) -> bool {
    let flag = |name: &str| round[name].as_bool().unwrap_or(false);
    let answered = round["outcomes"].as_array().is_some_and(|outcomes| {
        outcomes
            .iter()
            .any(|outcome| outcome.get("failure").is_none())
    });
    matches!(round["round"].as_str(), Some("council" | "stack"))
        && !flag("open")
        && !flag("closed")
        && round.get("unpinned").is_none()
        && round["startedAt"].is_string()
        && answered
}

/// Why `commit`, which no round of ours read, may not merge.
pub fn unread(short: &str) -> String {
    format!(
        "no review round of ours read {short}, nor an earlier commit of the same change: \
         ask a council (review_ask) at this head, and ask ready again once it's collected"
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn round(extra: &Value) -> Value {
        let mut round = json!({
            "id": "council-1",
            "round": "council",
            "startedAt": "2026-11-19T17:00:00.000Z",
            "witness": "aaa",
            "outcomes": [{"participantId": "a", "findingIds": [1], "answerPath": "/x"}],
        });
        for (key, value) in extra.as_object().unwrap() {
            round[key] = value.clone();
        }
        round
    }

    fn read(rounds: &[Value]) -> Vec<String> {
        read_in(&json!({"change": {}, "runs": rounds}).to_string())
    }

    #[test]
    fn a_collected_round_reads_its_witness() {
        assert_eq!(read(&[round(&json!({}))]), ["aaa"]);
        assert_eq!(
            read(&[round(
                &json!({"round": "stack", "witness": null, "witnesses": ["c", "d"]})
            )]),
            ["c", "d"]
        );
        assert_eq!(
            read(&[round(&json!({"outcomes": [
                {"participantId": "a", "failure": "timed out"},
                {"participantId": "b", "stopped": true, "findingIds": [2]},
            ]}))]),
            ["aaa"]
        );
    }

    // A judge or critique, which read findings rather than code, a round
    // still running, given up on with nothing behind it, read from another
    // tree than its witness, or with every reviewer failed speaks for no
    // head.
    #[test]
    fn a_round_that_didnt_read_its_witness_in_full_reads_nothing() {
        for extra in [
            json!({"round": "judge"}),
            json!({"round": "critique"}),
            json!({"open": true}),
            json!({"closed": true}),
            json!({"unpinned": {"read": "checkout"}}),
            json!({"outcomes": [{"participantId": "a", "failure": "timed out"}]}),
            json!({"outcomes": []}),
            json!({"startedAt": null}),
        ] {
            assert!(read(&[round(&extra)]).is_empty(), "{extra}");
        }
        assert_eq!(read_in("not json"), Vec::<String>::new());
    }

    #[test]
    fn every_change_s_rounds_are_read_from_the_folder() {
        let folder = std::env::temp_dir().join(format!("merge-net-seen-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let file = |name: &str, rounds: &[Value]| {
            fs::write(
                folder.join(name),
                json!({"change": {}, "runs": rounds}).to_string(),
            )
            .unwrap();
        };
        file("github_github_o_r_1.json", &[round(&json!({}))]);
        file(
            "local-range_github_o_r_main..b.json",
            &[round(&json!({"witness": "bbb"}))],
        );
        file("notes.txt", &[round(&json!({"witness": "ccc"}))]);
        let seen = witnessed(&folder).unwrap();
        let gone = witnessed(&folder.join("gone")).unwrap_err();
        assert!(
            gone.contains("can't be read"),
            "a folder that's gone is said so: {gone}"
        );
        fs::remove_dir_all(&folder).unwrap();
        assert_eq!(
            seen,
            HashSet::from(["aaa".to_owned(), "bbb".to_owned()]),
            "{seen:?}"
        );
    }
}
