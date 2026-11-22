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

/// Where other machines' review rounds are mirrored, one folder (or a link
/// to one) a node, beside the rounds folder: `review/runs-from`, as the
/// harness's own review gate reads them. Rounds now run on another machine
/// while this one's memory holds councils back, and a mirror copies its
/// rounds here rather than merging them into ours.
pub fn mirrors_folder() -> PathBuf {
    mirrors_beside(&rounds_folder())
}

/// The mirrors folder beside the rounds folder `rounds`.
fn mirrors_beside(rounds: &Path) -> PathBuf {
    rounds.with_file_name("runs-from")
}

/// The rounds mirrored from other machines: what they read, the nodes whose
/// rounds were read, and why any node's weren't.
#[derive(Debug, Default)]
pub struct Mirrored {
    pub rounds: HashSet<Read>,
    pub asked: Vec<String>,
    pub trouble: Vec<String>,
}

/// Every round mirrored under `root`, node by node in name order. A root
/// not made yet mirrors nothing, as on a machine no mirror fills. A node
/// that can't be read, or whose rounds can't be, is said so and left out
/// rather than refusing every head: the rounds here still count. A stray
/// file beside the nodes is left out, and so is a hidden name, as a sync
/// tool's own folder.
///
/// A range's base is kept only when it names a commit: one spelled as a
/// ref was resolved in the other machine's clone, where a branch of that
/// name can be another commit than here, so it stands for no head.
pub fn mirrored(root: &Path) -> Mirrored {
    let mut mirrored = Mirrored::default();
    let nodes = match fs::read_dir(root) {
        Ok(nodes) => nodes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return mirrored,
        Err(error) => {
            mirrored.trouble.push(format!(
                "the rounds mirrored under {} can't be listed ({error})",
                root.display()
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
        // A link to a folder is a node too, so its target is asked after.
        match fs::metadata(&node) {
            Ok(found) if found.is_dir() => match rounds_in(&node) {
                Ok(rounds) => {
                    let rounds: HashSet<Read> = rounds.into_iter().map(as_read_here).collect();
                    if rounds.is_empty() && node.join("runs").is_dir() {
                        mirrored.trouble.push(format!(
                            "the mirror from {name} holds a runs folder and no rounds, as if \
                             mirrored one folder too high"
                        ));
                    }
                    mirrored.rounds.extend(rounds);
                    mirrored.asked.push(name);
                }
                Err(Unreadable::Unlisted(error)) => mirrored
                    .trouble
                    .push(format!("the mirror from {name} can't be listed ({error})")),
                Err(Unreadable::Unshaped(newest)) => mirrored.trouble.push(format!(
                    "the newest {newest} round files mirrored from {name} hold no round of the \
                     shape merge-net reads, so that machine's harness may write another shape"
                )),
            },
            Ok(_) => {}
            Err(error) => mirrored
                .trouble
                .push(format!("the mirror from {name} can't be read ({error})")),
        }
    }
    mirrored
}

/// A mirrored round as it can stand here: a range's base spelled as a ref
/// becomes no base, which is on no main.
fn as_read_here(read: Read) -> Read {
    let from = read.from.map(|base| {
        let a_commit = base.len() == 40 && base.bytes().all(|b| b.is_ascii_hexdigit());
        if a_commit { base } else { String::new() }
    });
    Read { from, ..read }
}

/// `why`, a head unread, with what the mirrors said: which nodes' rounds
/// were asked as well, that a round run there counts once its mirror has
/// it, and any that couldn't be read.
pub fn unread_with_mirrors(why: String, mirrored: &Mirrored) -> String {
    use std::fmt::Write;
    let mut said = why;
    // Writing to a String can't fail.
    if !mirrored.asked.is_empty() {
        let _ = write!(
            said,
            "; the rounds mirrored from {} were asked too, and a round run there counts \
             here once its mirror holds it, a minute or two after it's collected",
            mirrored.asked.join(", ")
        );
    }
    for trouble in &mirrored.trouble {
        let _ = write!(said, "; {trouble}");
    }
    said
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
    rounds_in(folder).map_err(|why| match why {
        Unreadable::Unlisted(error) => format!(
            "the review rounds at {} can't be read ({error}), so no head can be shown \
             read; the harness keeps them in its state folder's review/runs",
            folder.display()
        ),
        Unreadable::Unshaped(newest) => format!(
            "the newest {newest} review round files at {} hold no round of the shape \
             merge-net reads, so the harness's record shape may have changed: \
             earlier/seen.rs must follow it",
            folder.display()
        ),
    })
}

/// Why a folder's rounds weren't read, said by whoever asked: this
/// machine's folder and a mirror call for different advice.
enum Unreadable {
    /// The folder can't be listed.
    Unlisted(std::io::Error),
    /// The newest this many round files are none of the shape read here.
    Unshaped(usize),
}

/// Every commit a round kept under `folder` read, as `witnessed` says.
fn rounds_in(folder: &Path) -> Result<HashSet<Read>, Unreadable> {
    let files = match fs::read_dir(folder) {
        Ok(files) => files,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(HashSet::new()),
        Err(error) => return Err(Unreadable::Unlisted(error)),
    };
    let mut texts: Vec<(std::time::SystemTime, String)> = files
        .flatten()
        .filter(|file| file.path().extension().is_some_and(|e| e == "json"))
        .filter_map(|file| {
            let written = file.metadata().and_then(|m| m.modified()).ok()?;
            Some((written, fs::read_to_string(file.path()).ok()?))
        })
        .collect();
    texts.sort_by_key(|text| std::cmp::Reverse(text.0));
    let newest = &texts[..texts.len().min(SHAPE_JUDGED_BY)];
    if !newest.is_empty() && !newest.iter().any(|(_, text)| shaped(text)) {
        return Err(Unreadable::Unshaped(newest.len()));
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
        assert_eq!(read_in("not json"), Vec::<Read>::new());
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

    // Each node's rounds are read, a link to a folder's too, in name order
    // past a stray file; a root not made yet mirrors nothing.
    #[test]
    fn every_mirrored_node_is_read_in_name_order() {
        let root = folder("mirrors");
        let elsewhere = folder("mirrored-elsewhere");
        let change =
            |commit: char| json!({"change": {}, "runs": [round(&json!({"witness": id(commit)}))]});
        let range = |base: &str, commit: char| {
            json!({
                "change": {"provider": "local-range", "id": format!("{base}..x")},
                "runs": [round(&json!({"witness": id(commit)}))],
            })
        };
        fs::create_dir(root.join("zebra")).unwrap();
        written(&root.join("zebra"), "one.json", &change('b'), 0);
        written(&elsewhere, "two.json", &change('c'), 0);
        std::os::unix::fs::symlink(&elsewhere, root.join("aardvark")).unwrap();
        fs::write(root.join("stray"), "").unwrap();
        fs::create_dir(root.join(".sync-tool")).unwrap();
        written(&root.join(".sync-tool"), "three.json", &change('d'), 0);
        fs::create_dir(root.join("ranges")).unwrap();
        written(
            &root.join("ranges"),
            "by-ref.json",
            &range("origin/main", 'e'),
            0,
        );
        written(
            &root.join("ranges"),
            "by-commit.json",
            &range(&id('9'), 'f'),
            0,
        );
        let mirrored = mirrored(&root);
        let gone = super::mirrored(&root.join("gone"));
        fs::remove_dir_all(&root).unwrap();
        fs::remove_dir_all(&elsewhere).unwrap();
        assert_eq!(
            mirrored.asked,
            ["aardvark", "ranges", "zebra"],
            "the nodes asked, a hidden one left out"
        );
        let by_ref = Read {
            commit: id('e'),
            from: Some(String::new()),
        };
        let by_commit = Read {
            commit: id('f'),
            from: Some(id('9')),
        };
        assert_eq!(
            mirrored.rounds,
            HashSet::from([change_read('b'), change_read('c'), by_ref, by_commit]),
            "the rounds read, a range's base kept only when it names a commit"
        );
        assert!(mirrored.trouble.is_empty(), "{:?}", mirrored.trouble);
        assert!(
            gone.asked.is_empty() && gone.trouble.is_empty() && gone.rounds.is_empty(),
            "a root not made yet mirrored something: {gone:?}"
        );
    }

    // A node that can't be read is said and left out, so the rest still
    // count; one holding a runs folder is said to be mirrored too high.
    #[test]
    fn a_mirror_that_cant_be_read_is_said_and_the_rest_count() {
        let root = folder("mirrors-trouble");
        std::os::unix::fs::symlink(root.join("nowhere"), root.join("dangling")).unwrap();
        fs::create_dir_all(root.join("high/runs")).unwrap();
        fs::create_dir(root.join("good")).unwrap();
        written(
            &root.join("good"),
            "one.json",
            &json!({"change": {}, "runs": [round(&json!({}))]}),
            0,
        );
        fs::create_dir(root.join("newer")).unwrap();
        for each in 0..SHAPE_JUDGED_BY {
            written(
                &root.join("newer"),
                &format!("new-{each}.json"),
                &json!({"change": {}, "rounds": [round(&json!({}))]}),
                0,
            );
        }
        let mirrored = mirrored(&root);
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(mirrored.asked, ["good", "high"], "the nodes asked");
        assert_eq!(
            mirrored.rounds,
            HashSet::from([change_read('a')]),
            "the good node's rounds"
        );
        assert!(
            mirrored.trouble.len() == 3
                && mirrored.trouble[0].starts_with("the mirror from dangling can't be read")
                && mirrored.trouble[1].contains("from high holds a runs folder")
                && mirrored.trouble[2].contains("mirrored from newer hold no round of the shape")
                && !mirrored
                    .trouble
                    .iter()
                    .any(|said| said.contains("review/runs")),
            "a mirror's trouble wasn't said as a mirror's: {:?}",
            mirrored.trouble
        );
    }

    // The mirrors sit beside the rounds folder, wherever that is set.
    #[test]
    fn the_mirrors_sit_beside_the_rounds() {
        assert_eq!(
            mirrors_beside(Path::new("/state/pi/agentic-harness.pi/review/runs")),
            Path::new("/state/pi/agentic-harness.pi/review/runs-from"),
            "the mirrors folder isn't beside the rounds folder"
        );
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
