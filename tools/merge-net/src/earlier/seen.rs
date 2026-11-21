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
/// the harness keeps its review state elsewhere. So is one whose round
/// files name no commit read at all, which is what a change to the
/// harness's record shape would look like, rather than refusing every
/// head as unread.
pub fn witnessed(folder: &Path) -> Result<HashSet<String>, String> {
    let files = fs::read_dir(folder).map_err(|error| {
        format!(
            "the review rounds at {} can't be read ({error}), so no head can be shown read; \
             point MERGE_NET_REVIEW_RUNS at the harness's review/runs",
            folder.display()
        )
    })?;
    let texts: Vec<String> = files
        .flatten()
        .filter(|file| file.path().extension().is_some_and(|e| e == "json"))
        .filter_map(|file| fs::read_to_string(file.path()).ok())
        .collect();
    let seen: HashSet<String> = texts.iter().flat_map(|text| read_in(text)).collect();
    if seen.is_empty() && !texts.is_empty() {
        return Err(format!(
            "the {} review round files at {} name no commit read in full as merge-net \
             reads them, so the harness's record shape may have changed: earlier/seen.rs \
             must follow it",
            texts.len(),
            folder.display()
        ));
    }
    Ok(seen)
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
                .filter(|witness| is_commit_id(witness))
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Whether `witness` is a full commit id, which is all a round names, so
/// a symbolic name such as a branch's, which git would read as whatever
/// it points at now, never stands for a read.
fn is_commit_id(witness: &str) -> bool {
    matches!(witness.len(), 40 | 64)
        && witness
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
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

    /// A full commit id of the letter `letter`, as a round names one.
    fn id(letter: char) -> String {
        letter.to_string().repeat(40)
    }

    fn round(extra: &Value) -> Value {
        let mut round = json!({
            "id": "council-1",
            "round": "council",
            "startedAt": "2026-11-19T17:00:00.000Z",
            "witness": id('a'),
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
        assert_eq!(read(&[round(&json!({}))]), [id('a')]);
        assert_eq!(
            read(&[round(
                &json!({"round": "stack", "witness": null, "witnesses": [id('c'), id('d')]})
            )]),
            [id('c'), id('d')]
        );
        assert_eq!(
            read(&[round(&json!({"outcomes": [
                {"participantId": "a", "failure": "timed out"},
                {"participantId": "b", "stopped": true, "findingIds": [2]},
            ]}))]),
            [id('a')]
        );
    }

    // A witness that isn't a full commit id, which git would read as
    // whatever it names now, stands for no read.
    #[test]
    fn a_witness_that_is_no_commit_id_reads_nothing() {
        for witness in [
            "main",
            "HEAD",
            "aaa",
            &id('A'),
            &id('g'),
            &format!("{}a", id('a')),
        ] {
            assert!(
                read(&[round(&json!({"witness": witness}))]).is_empty(),
                "{witness}"
            );
        }
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
        let _ = fs::remove_dir_all(&folder);
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
            &[round(&json!({"witness": id('b')}))],
        );
        file("notes.txt", &[round(&json!({"witness": id('c')}))]);
        let seen = witnessed(&folder).unwrap();
        let gone = witnessed(&folder.join("gone")).unwrap_err();
        assert!(
            gone.contains("can't be read"),
            "a folder that's gone is said so: {gone}"
        );
        fs::remove_dir_all(&folder).unwrap();
        assert_eq!(seen, HashSet::from([id('a'), id('b')]), "{seen:?}");
    }

    // Round files of which none names a commit read, as a change to the
    // harness's record shape would leave them, are said so rather than
    // read as no head ever reviewed.
    #[test]
    fn round_files_naming_no_read_commit_are_said_so() {
        let folder =
            std::env::temp_dir().join(format!("merge-net-seen-shape-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        assert_eq!(witnessed(&folder), Ok(HashSet::new()), "an empty folder");
        fs::write(
            folder.join("github_github_o_r_1.json"),
            json!({"change": {}, "rounds": [round(&json!({}))]}).to_string(),
        )
        .unwrap();
        let said = witnessed(&folder).unwrap_err();
        fs::remove_dir_all(&folder).unwrap();
        assert!(said.contains("record shape"), "{said}");
    }
}
