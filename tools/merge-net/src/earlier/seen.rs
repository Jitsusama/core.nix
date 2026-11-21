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

/// A commit a round read, and where it read from when the round was over
/// a range of commits rather than a change: such a round read only what
/// lies between, so its commit counts as read in full only where that
/// base is on main.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Read {
    pub commit: String,
    pub from: Option<String>,
}

/// How many of the newest round files the record shape is judged by: the
/// harness rewrites a change's file at each round, so the newest show the
/// shape it writes now, whatever older files hold.
const SHAPE_JUDGED_BY: usize = 5;

/// Every commit a round kept under `folder` read. A folder that can't be
/// read is said so, since a council asked at the head wouldn't help where
/// the net can't see the rounds at all, as on a machine where the harness
/// keeps its review state elsewhere; one not made yet holds no round, as
/// before the first is asked. So are round files whose newest rounds
/// aren't the shape merge-net reads, which is what a change to the
/// harness's records would look like, rather than refusing every head as
/// unread.
pub fn witnessed(folder: &Path) -> Result<HashSet<Read>, String> {
    let files = match fs::read_dir(folder) {
        Ok(files) => files,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(HashSet::new()),
        Err(error) => {
            return Err(format!(
                "the review rounds at {} can't be read ({error}), so no head can be shown \
                 read; the harness keeps them in its state folder's review/runs",
                folder.display()
            ));
        }
    };
    let mut texts: Vec<(std::time::SystemTime, String)> = files
        .flatten()
        .filter(|file| file.path().extension().is_some_and(|e| e == "json"))
        .filter_map(|file| {
            let written = file.metadata().and_then(|m| m.modified()).ok()?;
            Some((written, fs::read_to_string(file.path()).ok()?))
        })
        .collect();
    texts.sort_by(|a, b| b.0.cmp(&a.0));
    let newest = &texts[..texts.len().min(SHAPE_JUDGED_BY)];
    if !newest.is_empty() && !newest.iter().any(|(_, text)| shaped(text)) {
        return Err(format!(
            "the newest {} review round files at {} hold no round of the shape merge-net \
             reads, so the harness's record shape may have changed: earlier/seen.rs must \
             follow it",
            newest.len(),
            folder.display()
        ));
    }
    Ok(texts.iter().flat_map(|(_, text)| read_in(text)).collect())
}

/// Whether the newest round in the change's file `text` is the shape
/// merge-net reads: a kind, when it started, and the commits it was asked
/// of, whether or not it has been collected.
fn shaped(text: &str) -> bool {
    let Ok(change) = serde_json::from_str::<Value>(text) else {
        return false;
    };
    change["runs"]
        .as_array()
        .and_then(|rounds| rounds.last())
        .is_some_and(|round| {
            round["round"].is_string()
                && round["startedAt"].is_string()
                && (round["witness"].is_string() || round["witnesses"].is_array())
        })
}

/// The commits the rounds of one change's file `text` read in full, each
/// with the base it read from when the change is a range of commits.
fn read_in(text: &str) -> Vec<Read> {
    let Ok(change) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    let Some(rounds) = change["runs"].as_array() else {
        return Vec::new();
    };
    let from = (change["change"]["provider"].as_str() == Some("local-range"))
        .then(|| change["change"]["id"].as_str())
        .map(|range| {
            range
                .and_then(|range| range.split_once(".."))
                .map_or_else(String::new, |(base, _)| base.to_owned())
        });
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
                .map(|witness| Read {
                    commit: witness.to_owned(),
                    from: from.clone(),
                })
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
        "no review round of ours read {short}, nor an earlier commit of the same change \
         (a squash or a reworded subject is told as another): ask a council (review_ask) at \
         this head, and ask ready again once it's collected"
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
            .into_iter()
            .map(|read| read.commit)
            .collect()
    }

    fn change_read(commit: char) -> Read {
        Read {
            commit: id(commit),
            from: None,
        }
    }

    /// A folder of its own for a test named `name`, empty.
    fn folder(name: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("merge-net-seen-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    /// Writes the round file `name` in `folder` holding `change`, as
    /// written `ago` seconds before now.
    fn written(folder: &Path, name: &str, change: &Value, ago: u64) {
        let path = folder.join(name);
        fs::write(&path, change.to_string()).unwrap();
        let when = std::time::SystemTime::now() - std::time::Duration::from_secs(ago);
        fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(when)
            .unwrap();
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
        assert!(read_in("not json").is_empty());
    }

    // A change's rounds read its commit; a range's read it from the
    // range's base, which the net checks is on main.
    #[test]
    fn every_change_s_rounds_are_read_from_the_folder() {
        let folder = folder("read");
        written(
            &folder,
            "github_github_o_r_1.json",
            &json!({"change": {"provider": "github", "id": "o/r#1"}, "runs": [round(&json!({}))]}),
            0,
        );
        written(
            &folder,
            "local-range_github_o_r_main..b.json",
            &json!({
                "change": {"provider": "local-range", "id": "origin/main..b"},
                "runs": [round(&json!({"witness": id('b')}))],
            }),
            0,
        );
        written(
            &folder,
            "notes.txt",
            &json!({"change": {}, "runs": [round(&json!({"witness": id('c')}))]}),
            0,
        );
        let seen = witnessed(&folder).unwrap();
        fs::remove_dir_all(&folder).unwrap();
        let ranged = Read {
            commit: id('b'),
            from: Some("origin/main".to_owned()),
        };
        assert_eq!(seen, HashSet::from([change_read('a'), ranged]), "{seen:?}");
    }

    // A folder not made yet holds no round, as before the first is asked;
    // one that can't be read is said so.
    #[test]
    fn a_folder_not_made_yet_holds_no_round_and_an_unreadable_one_is_said_so() {
        let folder = folder("gone");
        assert_eq!(witnessed(&folder.join("gone")), Ok(HashSet::new()));
        fs::write(folder.join("file"), "").unwrap();
        let said = witnessed(&folder.join("file")).unwrap_err();
        fs::remove_dir_all(&folder).unwrap();
        assert!(said.contains("can't be read"), "{said}");
    }

    // Newest round files of another shape, as a change to the harness's
    // records would write them, are said so rather than read as no head
    // ever reviewed, whatever older files hold; files of the shape whose
    // rounds haven't been collected are just unread.
    #[test]
    fn newest_round_files_of_another_shape_are_said_so() {
        let folder = folder("shape");
        assert_eq!(witnessed(&folder), Ok(HashSet::new()), "an empty folder");
        written(
            &folder,
            "old.json",
            &json!({"change": {}, "runs": [round(&json!({}))]}),
            86_400,
        );
        written(
            &folder,
            "open.json",
            &json!({"change": {}, "runs": [round(&json!({"witness": id('b'), "open": true}))]}),
            60,
        );
        assert_eq!(
            witnessed(&folder),
            Ok(HashSet::from([change_read('a')])),
            "a round still open isn't another shape"
        );
        for each in 0..SHAPE_JUDGED_BY {
            written(
                &folder,
                &format!("new-{each}.json"),
                &json!({"change": {}, "rounds": [round(&json!({}))]}),
                0,
            );
        }
        let said = witnessed(&folder).unwrap_err();
        fs::remove_dir_all(&folder).unwrap();
        assert!(said.contains("record shape"), "{said}");
    }
}
