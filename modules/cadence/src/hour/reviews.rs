//! The harness's records of review rounds and their fixes, read as
//! merge-net reads them: round files only, and each other machine's mirror
//! beside them, saying why a record couldn't be read rather than leaving
//! its rounds out unsaid, which would count the wrong rounds past five.

use std::cmp::Reverse;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::Path;
use std::time::SystemTime;

use serde_json::Value;

use super::measures::rounds::Records;

/// How many of a folder's newest records its shape is judged by, as
/// merge-net judges a mirror's: the harness rewrites a change's record at
/// each round, so the newest show the shape it writes now.
const SHAPE_JUDGED_BY: usize = 5;

/// This machine's records of review rounds in `folder`, written since
/// `since`. One with no round in that time is quiet; one not there, not
/// readable, or whose newest records are of another shape is said in
/// `trouble`.
#[must_use]
pub fn rounds(folder: &Path, since: SystemTime) -> Records {
    kept(folder, since, rounds_shaped)
}

/// This machine's records of fixes for findings in `folder`, as `rounds`
/// reads its rounds.
#[must_use]
pub fn fixes(folder: &Path, since: SystemTime) -> Records {
    kept(folder, since, |text| {
        serde_json::from_str::<Value>(text).is_ok_and(|record| record["fixes"].is_array())
    })
}

/// The records in `folder` written since `since`, judged by `shaped`.
fn kept(folder: &Path, since: SystemTime, shaped: fn(&str) -> bool) -> Records {
    let at = folder.display();
    match listed(folder, since) {
        Ok(runs) => Records {
            trouble: unshaped(&runs, shaped, &format!("at {at}")).into_iter().collect(),
            runs,
        },
        Err(error) => Records {
            runs: Vec::new(),
            trouble: vec![format!("the review records at {at} can't be listed ({error})")],
        },
    }
}

/// The records in `folder` written since `since`, newest first, or why it
/// can't be listed. A record is a `.json` that is a plain file once a link
/// to it is followed, so a half-written `.tmp`, a pipe or a device beside
/// it isn't read.
fn listed(folder: &Path, since: SystemTime) -> io::Result<Vec<String>> {
    let mut texts: Vec<(SystemTime, String)> = fs::read_dir(folder)?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "json"))
        .filter_map(|path| {
            let found = fs::metadata(&path).ok()?;
            let written = found.modified().ok()?;
            if !found.is_file() || written < since {
                return None;
            }
            Some((written, fs::read_to_string(&path).ok()?))
        })
        .collect();
    texts.sort_by_key(|(written, _)| Reverse(*written));
    Ok(texts.into_iter().map(|(_, text)| text).collect())
}

/// Whether `text`, a change's record of rounds, is the shape read here:
/// its change has an id and a repository, and its newest round an id, when
/// it started and its reviewers' outcomes.
fn rounds_shaped(text: &str) -> bool {
    let Ok(record) = serde_json::from_str::<Value>(text) else {
        return false;
    };
    let change = &record["change"];
    change["id"].is_string()
        && change["repo"]["key"].is_string()
        && record["runs"]
            .as_array()
            .and_then(|runs| runs.last())
            .is_some_and(|run| {
                run["id"].is_string() && run["startedAt"].is_string() && run["outcomes"].is_array()
            })
}

/// Why `records`, newest first, kept `from` somewhere, can't be read: none
/// of the newest is `shaped`, so the harness that wrote them may write
/// another shape.
fn unshaped(records: &[String], shaped: fn(&str) -> bool, from: &str) -> Option<String> {
    let newest = &records[..records.len().min(SHAPE_JUDGED_BY)];
    (!newest.is_empty() && !newest.iter().any(|text| shaped(text))).then(|| {
        format!(
            "the newest {} review records {from} hold none of the shape cadence reads, so that \
             harness may write another",
            newest.len()
        )
    })
}

/// The records written since `since` mirrored under `root`, a folder (or a
/// link to one) a node, as merge-net's `mirrored` reads them: a hidden
/// name, as a sync tool's own folder, and a stray file beside the nodes
/// are left out. A root not there mirrors nothing, as on a machine no
/// mirror fills. A root or node that can't be listed, a node holding a
/// runs folder and no records, as if mirrored one folder too high, and one
/// whose newest records are of another shape are said in `trouble`.
#[must_use]
pub fn mirrored(root: &Path, since: SystemTime) -> Records {
    let mut mirrored = Records::default();
    let nodes = match fs::read_dir(root) {
        Ok(nodes) => nodes,
        Err(error) if error.kind() == ErrorKind::NotFound => return mirrored,
        Err(error) => {
            let root = root.display();
            mirrored.trouble.push(format!(
                "the rounds mirrored under {root} can't be listed ({error})"
            ));
            return mirrored;
        }
    };
    let mut names: Vec<String> = nodes
        .flatten()
        .map(|node| node.file_name().to_string_lossy().into_owned())
        .filter(|name| !name.starts_with('.'))
        .collect();
    names.sort();
    for name in names {
        let node = root.join(&name);
        let said = match fs::metadata(&node).map(|found| found.is_dir()) {
            Ok(false) => continue,
            Ok(true) => match listed(&node, since) {
                Ok(runs) if runs.is_empty() && node.join("runs").is_dir() => format!(
                    "the mirror from {name} holds a runs folder and no rounds, as if mirrored \
                     one folder too high"
                ),
                Ok(runs) => {
                    let from = format!("mirrored from {name}");
                    match unshaped(&runs, rounds_shaped, &from) {
                        Some(why) => why,
                        None => {
                            mirrored.runs.extend(runs);
                            continue;
                        }
                    }
                }
                Err(error) => format!("the mirror from {name} can't be listed ({error})"),
            },
            Err(error) => format!("the mirror from {name} can't be read ({error})"),
        };
        mirrored.trouble.push(said);
    }
    mirrored
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    fn put(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().expect("a file the test names in a folder"))
            .expect("the test's folder made");
        fs::write(path, text).expect("the test's file written");
    }

    /// A change's record of rounds as the harness writes one, its change
    /// named `id`.
    fn round(id: &str) -> String {
        serde_json::json!({
            "change": {"id": id, "repo": {"key": "github:Jitsusama/core.nix"}},
            "runs": [{"id": "council-1", "startedAt": "2026-11-23T07:00:00Z", "outcomes": []}],
        })
        .to_string()
    }

    #[test]
    fn mirrors_are_read_as_merge_net_reads_them() {
        let scratch = tempfile::tempdir().expect("the test's folder made");
        let root = scratch.path().join("runs-from");
        put(&root.join("mac/one.json"), &round("one"));
        // A node linked to its folder, as a sync tool may keep it, and a
        // link left pointing at nothing, which can't be read.
        put(&scratch.path().join("elsewhere/five.json"), &round("five"));
        std::os::unix::fs::symlink(scratch.path().join("elsewhere"), root.join("linked"))
            .expect("the test's link made");
        std::os::unix::fs::symlink(scratch.path().join("gone"), root.join("dangling"))
            .expect("the test's link made");
        put(&root.join("mac/one.json.tmp"), "half");
        put(&root.join(".sync-tool/two.json"), &round("hidden"));
        put(&root.join("stray.json"), &round("stray"));
        put(&root.join("high/runs/three.json"), &round("high"));
        put(&root.join("old/four.json"), &round("four"));
        // A harness that renamed its outcomes, every other field kept.
        put(
            &root.join("renamed/six.json"),
            &round("six").replace("\"outcomes\"", "\"answers\""),
        );
        let read = mirrored(&root, UNIX_EPOCH);
        assert_eq!(
            read.runs,
            [round("five"), round("one"), round("four")],
            "a half-written, hidden or stray file was read, or a node's records weren't"
        );
        let said = |at: usize, node: &str, why: &str| {
            read.trouble
                .get(at)
                .is_some_and(|t| t.contains(&format!("from {node}")) && t.contains(why))
        };
        assert!(
            read.trouble.len() == 3
                && said(0, "dangling", "can't be read")
                && said(1, "high", "too high")
                && said(2, "renamed", "of the shape"),
            "a node unread, mirrored too high or of another shape went unsaid: {:?}",
            read.trouble
        );
        let none = mirrored(&root.join("none"), UNIX_EPOCH);
        assert!(
            none.runs.is_empty() && none.trouble.is_empty(),
            "a machine no mirror fills was said to have trouble: {none:?}"
        );
        let unlisted = mirrored(&root.join("stray.json"), UNIX_EPOCH);
        assert!(
            unlisted.trouble[0].contains("can't be listed"),
            "a mirrors folder that can't be listed went unsaid: {unlisted:?}"
        );
    }

    #[test]
    fn a_folder_with_no_round_is_quiet_and_one_unread_says_so() {
        let scratch = tempfile::tempdir().expect("the test's folder made");
        let runs = scratch.path().join("runs");
        put(&runs.join("one.json"), &round("one"));
        // A pipe named as a record would hang a read.
        let pipe = runs.join("pipe.json");
        let made = std::process::Command::new("mkfifo").arg(&pipe).status();
        assert!(made.is_ok_and(|s| s.success()), "the test's pipe made");
        let read = rounds(&runs, UNIX_EPOCH);
        assert!(
            read.runs == [round("one")] && read.trouble.is_empty(),
            "a record wasn't read, or a pipe was: {read:?}"
        );
        let later = SystemTime::now() + Duration::from_secs(60);
        let quiet = rounds(&runs, later);
        assert!(
            quiet.runs.is_empty() && quiet.trouble.is_empty(),
            "a day with no round here read as unread: {quiet:?}"
        );
        let gone = rounds(&scratch.path().join("gone"), UNIX_EPOCH);
        assert!(
            gone.trouble[0].contains("can't be listed"),
            "a records folder not there went unsaid: {gone:?}"
        );
        put(&runs.join("two.json"), "{\"runs\": []}");
        put(&scratch.path().join("fixes/one.json"), "{\"fixes\": []}");
        let fixed = fixes(&scratch.path().join("fixes"), UNIX_EPOCH);
        let shapeless = fixes(&runs, UNIX_EPOCH);
        assert!(
            fixed.trouble.is_empty() && !shapeless.trouble.is_empty(),
            "a fixes record wasn't read, or records of another shape were: {shapeless:?}"
        );
    }
}
