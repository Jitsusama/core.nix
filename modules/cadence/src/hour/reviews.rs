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

/// The trouble's word for what a record read in the window lacks of what
/// every record of its kind holds, or none when it lacks nothing. One
/// lacking it is said unread rather than left out: a round naming no
/// change or keeping no list of runs, or a fixes record holding no fixes, would
/// otherwise lose what it holds unsaid.
type Lacks = fn(&Value) -> Option<&'static str>;

fn rounds_lack(record: &Value) -> Option<&'static str> {
    if !names_its_change(record) {
        Some("it names no change")
    } else if !record["runs"].is_array() {
        Some("it keeps no list of runs")
    } else {
        None
    }
}

fn fixes_lack(record: &Value) -> Option<&'static str> {
    (!record["fixes"].is_array()).then_some("it holds no fixes")
}

fn names_its_change(record: &Value) -> bool {
    record["change"]["id"].is_string() && record["change"]["repo"]["key"].is_string()
}

/// This machine's records of review rounds in `folder`, written since
/// `since`. One with no round in that time is quiet, as is one not made
/// yet, which merge-net reads as holding no round; one that can't be
/// listed, a record in it that can't be read, and newest records of
/// another shape are said in `trouble`.
#[must_use]
pub fn rounds(folder: &Path, since: SystemTime) -> Records {
    kept(folder, since, Some(rounds_shaped), rounds_lack)
}

/// This machine's records of fixes for findings in `folder`, as `rounds`
/// reads its rounds, save that no shape is judged across them: a record
/// holding no fixes is already said unread, and a fix is read for no
/// more than the round it names.
#[must_use]
pub fn fixes(folder: &Path, since: SystemTime) -> Records {
    kept(folder, since, None, fixes_lack)
}

/// The records in `folder` written since `since` that `lacks` nothing,
/// the newest judged by `shaped` where it's given.
fn kept(
    folder: &Path,
    since: SystemTime,
    shaped: Option<fn(&str) -> bool>,
    lacks: Lacks,
) -> Records {
    let at = folder.display();
    match listed(folder, since, lacks) {
        Ok((runs, unread)) => Records {
            trouble: unread
                .into_iter()
                .chain(shaped.and_then(|shaped| unshaped(&runs, shaped, &format!("at {at}"))))
                .collect(),
            runs,
        },
        Err(error) if error.kind() == ErrorKind::NotFound => Records::default(),
        Err(error) => Records {
            runs: Vec::new(),
            trouble: vec![format!(
                "the review records at {at} can't be listed ({error})"
            )],
        },
    }
}

/// The records in `folder` written since `since`, newest first, with why
/// each record that couldn't be read wasn't, or why the folder can't be
/// listed. A record is a `.json` that is a plain file once a link to it is
/// followed, so a half-written `.tmp`, a pipe or a device beside it isn't
/// read; one that `lacks` what its kind holds can't be read.
fn listed(
    folder: &Path,
    since: SystemTime,
    lacks: Lacks,
) -> io::Result<(Vec<String>, Vec<String>)> {
    let mut texts: Vec<(SystemTime, String)> = Vec::new();
    let mut unread = Vec::new();
    let paths = fs::read_dir(folder)?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "json"));
    for path in paths {
        match record(&path, since, lacks) {
            Ok(Some(found)) => texts.push(found),
            Ok(None) => {}
            Err(error) => unread.push(format!(
                "the review record {} can't be read ({error})",
                path.display()
            )),
        }
    }
    texts.sort_by_key(|(written, _)| Reverse(*written));
    Ok((texts.into_iter().map(|(_, text)| text).collect(), unread))
}

/// The error a link loop gives, ELOOP, which the standard library names
/// only on nightly.
#[cfg(target_os = "macos")]
const LOOP: i32 = 62;
/// The error a link loop gives, ELOOP, which the standard library names
/// only on nightly.
#[cfg(not(target_os = "macos"))]
const LOOP: i32 = 40;

/// The record at `path`, with when it was written, or none where it isn't
/// a plain file, was written before `since`, or was removed once listed,
/// as the harness prunes; else why it can't be read. One that isn't JSON
/// or `lacks` what its kind holds can't be read either, as its rounds
/// would be left out unsaid, nor can a link that may hold a record but
/// can't be followed, as one denied, or a link to nothing (a missing name,
/// a path through a file, or a loop) left since `since`, which is no
/// record pruned; one left to nothing before it is as old as a record
/// unread.
fn record(
    path: &Path,
    since: SystemTime,
    lacks: Lacks,
) -> io::Result<Option<(SystemTime, String)>> {
    let gone = |error: io::Error| {
        if error.kind() == ErrorKind::NotFound {
            Ok(None)
        } else {
            Err(error)
        }
    };
    let found = match fs::metadata(path) {
        Ok(found) => found,
        Err(error) => {
            let link = fs::symlink_metadata(path)
                .ok()
                .filter(|link| link.file_type().is_symlink());
            let kind = error.kind();
            let nothing = matches!(kind, ErrorKind::NotFound | ErrorKind::NotADirectory)
                || error.raw_os_error() == Some(LOOP);
            return match link {
                None => gone(error),
                Some(_) if !nothing => Err(io::Error::new(
                    error.kind(),
                    format!("a link that can't be followed: {error}"),
                )),
                Some(link) if link.modified().is_ok_and(|left| left < since) => Ok(None),
                Some(_) => Err(io::Error::new(
                    error.kind(),
                    format!("a link to nothing: {error}"),
                )),
            };
        }
    };
    let written = found.modified()?;
    if !found.is_file() || written < since {
        return Ok(None);
    }
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => return gone(error),
    };
    let read = serde_json::from_str::<Value>(&text).map_err(io::Error::other)?;
    if let Some(lacking) = lacks(&read) {
        return Err(io::Error::other(lacking));
    }
    Ok(Some((written, text)))
}

/// Whether `text`, a change's record of rounds, is the shape read here:
/// its change has an id and a repository, and its newest round an id, its
/// kind, when it started and its reviewers' outcomes. The kind is what
/// says a council from a judge, so a harness that renamed it would
/// otherwise count no round and close every drag.
fn rounds_shaped(text: &str) -> bool {
    let Ok(record) = serde_json::from_str::<Value>(text) else {
        return false;
    };
    names_its_change(&record)
        && record["runs"]
            .as_array()
            .and_then(|runs| runs.last())
            .is_some_and(|run| {
                run["id"].is_string()
                    && run["round"].is_string()
                    && run["startedAt"].is_string()
                    && run["outcomes"].is_array()
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
/// mirror fills. A root or node that can't be listed, a record in a node
/// that can't be read, a node holding a runs folder and no records, as if
/// mirrored one folder too high, and one whose newest records are of
/// another shape are said in `trouble`.
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
            Ok(true) => match listed(&node, since, rounds_lack) {
                Ok((runs, _)) if runs.is_empty() && node.join("runs").is_dir() => format!(
                    "the mirror from {name} holds a runs folder and no rounds, as if mirrored \
                     one folder too high"
                ),
                Ok((runs, unread)) => {
                    mirrored.trouble.extend(unread);
                    let from = format!("mirrored from {name}");
                    if let Some(why) = unshaped(&runs, rounds_shaped, &from) {
                        why
                    } else {
                        mirrored.runs.extend(runs);
                        continue;
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
    use std::os::unix::fs::PermissionsExt;
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
            "runs": [{"id": "council-1", "round": "council", "startedAt": "2026-11-23T07:00:00Z",
                       "outcomes": []}],
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
        // A node whose only record was written before the hour's window.
        put(&root.join("old/four.json"), &round("four"));
        let long_ago = SystemTime::now() - Duration::from_secs(7200);
        fs::File::options()
            .write(true)
            .open(root.join("old/four.json"))
            .and_then(|file| file.set_modified(long_ago))
            .expect("the old record's time set");
        // A record cut off as it was copied, which can't be read, and one
        // naming no change, which can't either.
        put(&root.join("torn/seven.json"), "{\"change\": ");
        put(
            &root.join("runless/nine.json"),
            &round("nine").replace("\"runs\"", "\"rounds\""),
        );
        // A harness that renamed its outcomes, every other field kept, and
        // one that renamed its rounds' kind.
        put(
            &root.join("renamed/six.json"),
            &round("six").replace("\"outcomes\"", "\"answers\""),
        );
        put(
            &root.join("rekinded/eight.json"),
            &round("eight").replace("\"round\"", "\"kind\""),
        );
        let since = SystemTime::now() - Duration::from_secs(3600);
        let read = mirrored(&root, since);
        assert_eq!(
            read.runs,
            [round("five"), round("one")],
            "a half-written, hidden, stray or old file was read, or a node's records weren't"
        );
        let said = |at: usize, node: &str, why: &str| {
            read.trouble
                .get(at)
                .is_some_and(|t| t.contains(node) && t.contains(why))
        };
        assert!(
            read.trouble.len() == 6
                && said(0, "from dangling", "can't be read")
                && said(1, "from high", "too high")
                && said(2, "from rekinded", "of the shape")
                && said(3, "from renamed", "of the shape")
                && said(4, "runless/nine.json", "it keeps no list of runs")
                && said(5, "torn/seven.json", "can't be read"),
            "a node unread, mirrored too high or of another shape, or a record unread, went \
             unsaid: {:?}",
            read.trouble
        );
        let none = mirrored(&root.join("none"), UNIX_EPOCH);
        assert!(
            none.runs.is_empty() && none.trouble.is_empty(),
            "a machine no mirror fills was said to have trouble: {none:?}"
        );
        let unlisted = mirrored(&root.join("stray.json"), UNIX_EPOCH);
        assert!(
            unlisted
                .trouble
                .first()
                .is_some_and(|t| t.contains("can't be listed")),
            "a mirrors folder that can't be listed went unsaid: {unlisted:?}"
        );
    }

    #[test]
    fn records_are_read_newest_first_and_judged_by_the_newest_five() {
        let scratch = tempfile::tempdir().expect("the test's folder made");
        let runs = scratch.path().join("runs");
        let shapeless = round("x").replace("\"outcomes\"", "\"answers\"");
        let write = |name: &str, text: &str, ago: u64| {
            let path = runs.join(name);
            put(&path, text);
            fs::File::options()
                .write(true)
                .open(&path)
                .and_then(|file| file.set_modified(SystemTime::now() - Duration::from_secs(ago)))
                .expect("the record's time set");
        };
        // The one record of the shape read here is the sixth newest.
        write("old.json", &round("old"), 600);
        for n in 0..5 {
            write(&format!("new-{n}.json"), &shapeless, 60 * (5 - n));
        }
        let read = rounds(&runs, UNIX_EPOCH);
        assert!(
            read.runs.last() == Some(&round("old"))
                && read.trouble.len() == 1
                && read.trouble[0].contains("newest 5"),
            "records weren't read newest first, or a shape seen only past the newest five \
             was taken as the harness's: {read:?}"
        );
        // Once a record of the shape is among the newest five, it is read.
        write("newer.json", &round("newer"), 30);
        let read = rounds(&runs, UNIX_EPOCH);
        assert!(
            read.runs.first() == Some(&round("newer")) && read.trouble.is_empty(),
            "a folder whose newest five hold one of the shape was said unread: {read:?}"
        );
    }

    #[test]
    fn a_folder_with_no_round_is_quiet_and_one_unread_says_so() {
        let scratch = tempfile::tempdir().expect("the test's folder made");
        let runs = scratch.path().join("runs");
        put(&runs.join("one.json"), &round("one"));
        // A pipe named as a record would hang a read, so it's read on a
        // thread of its own, and let go if the read waits on it.
        let pipe = runs.join("pipe.json");
        let made = std::process::Command::new("mkfifo").arg(&pipe).status();
        assert!(made.is_ok_and(|s| s.success()), "the test's pipe made");
        let (sent, answer) = std::sync::mpsc::channel();
        let folder = runs.clone();
        std::thread::spawn(move || sent.send(rounds(&folder, UNIX_EPOCH)).ok());
        let answer = answer.recv_timeout(Duration::from_secs(30));
        if answer.is_err() {
            fs::File::options().write(true).open(&pipe).ok();
        }
        let read = answer.expect("a pipe named as a record was read, and the read hung");
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
        // A folder not made yet holds no round, as merge-net reads it.
        let gone = rounds(&scratch.path().join("gone"), UNIX_EPOCH);
        assert!(
            gone.runs.is_empty() && gone.trouble.is_empty(),
            "a records folder not made yet was said unread: {gone:?}"
        );
        let unlisted = rounds(&runs.join("one.json"), UNIX_EPOCH);
        assert!(
            unlisted
                .trouble
                .first()
                .is_some_and(|t| t.contains("can't be listed")),
            "a records folder that can't be listed went unsaid: {unlisted:?}"
        );
        put(&scratch.path().join("fixes/one.json"), "{\"fixes\": []}");
        put(&scratch.path().join("fixes/two.json"), "{\"change\": {}}");
        let fixed = fixes(&scratch.path().join("fixes"), UNIX_EPOCH);
        assert!(
            fixed.runs.len() == 1
                && fixed.trouble.len() == 1
                && fixed.trouble[0].contains("two.json can't be read (it holds no fixes"),
            "a fixes record wasn't read, or one holding no fixes went unsaid: {fixed:?}"
        );
    }

    #[test]
    fn records_unread_are_said_and_links_to_nothing_until_they_are_old() {
        let scratch = tempfile::tempdir().expect("the test's folder made");
        let torn = scratch.path().join("torn");
        put(&torn.join("one.json"), &round("one"));
        put(&torn.join("two.json"), "{\"change\": ");
        std::os::unix::fs::symlink(torn.join("gone.json"), torn.join("three.json"))
            .expect("the test's link made");
        put(&torn.join("four.json"), "{\"runs\": []}");
        put(
            &torn.join("five.json"),
            &round("five").replace("\"runs\"", "\"rounds\""),
        );
        // A link to itself and one through a file, which hold nothing as
        // surely as a link to a missing name.
        let link = |to: &str, at: &str| {
            std::os::unix::fs::symlink(torn.join(to), torn.join(at)).expect("the test's link made");
        };
        link("six.json", "six.json");
        link("one.json/inside.json", "seven.json");
        let read = rounds(&torn, UNIX_EPOCH);
        let unread = |name: &str| read.trouble.iter().any(|t| t.contains(name));
        assert!(
            read.runs == [round("one")]
                && read.trouble.len() == 6
                && unread("two.json can't be read")
                && unread("three.json can't be read (a link to nothing")
                && unread("four.json can't be read (it names no change")
                && unread("five.json can't be read (it keeps no list of runs")
                && unread("six.json can't be read (a link to nothing")
                && unread("seven.json can't be read (a link to nothing"),
            "a record that can't be read, a link to nothing, or one naming no change or \
             keeping no runs was left out unsaid: {read:?}"
        );
        // A link left pointing at nothing before the window is as old as
        // any record unread there, so it stops nothing filing.
        let later = rounds(&torn, SystemTime::now() + Duration::from_secs(3600));
        assert!(
            later.runs.is_empty() && later.trouble.is_empty(),
            "a link left pointing at nothing before the window was said unread: {later:?}"
        );
        // A link into a folder it may not read may hold a record, so it's
        // unread at any age; a runner reading every folder can't test it.
        let locked = scratch.path().join("locked");
        put(&locked.join("one.json"), &round("one"));
        let denied = scratch.path().join("denied");
        fs::create_dir_all(&denied).expect("the test's folder made");
        std::os::unix::fs::symlink(locked.join("one.json"), denied.join("one.json"))
            .expect("the test's link made");
        let mode = |bits| fs::set_permissions(&locked, fs::Permissions::from_mode(bits));
        mode(0o000).expect("the test's folder locked");
        let shut = fs::metadata(denied.join("one.json"))
            .is_err_and(|error| error.kind() == ErrorKind::PermissionDenied);
        let read = rounds(&denied, SystemTime::now() + Duration::from_secs(3600));
        mode(0o755).expect("the test's folder unlocked");
        let why = "a link that can't be followed";
        assert!(
            !shut || read.trouble.iter().any(|t| t.contains(why)),
            "an old link into a folder denied was taken as pruned: {read:?}"
        );
    }
}
