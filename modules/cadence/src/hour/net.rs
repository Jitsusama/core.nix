//! The two measures the merge net feeds the hour: the merges it reverted,
//! from cadence's own `reverted.json`, and the checks broken on main,
//! from the net's results. Each file is read whole or the measure is
//! said unread, since read as empty a file torn or barred would count none
//! reverted and none broken, and close both lines as fixed while nothing
//! was looked at. A file not there is one never written, and holds nothing:
//! the tick writes `reverted.json` only where a watcher is set, and the net
//! writes its results only with its first run, as its own journal reads.
//! A results line that isn't text or isn't JSON leaves the line unread too,
//! as a check read from its other runs could be broken by a pass dropped.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use super::measures::{self, Measure};
use super::{NET, NET_RULES};
use crate::reverted::Told;
use crate::whole::text_lines;

/// The file at `path` whole, `None` when it was never written, or why it
/// can't be read.
fn read(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
        Err(err) => Err(format!("{} can't be read: {err}", path.display())),
    }
}

/// The merges the net reverted, from cadence's `reverted.json` in
/// `folder`, and the checks broken on main, from the net's results under
/// `state_home`, each read whole or said unread.
pub fn read_both(
    folder: &Path,
    state_home: &Path,
    core: &Path,
    quarantine: &str,
    now: i64,
) -> [Measure; 2] {
    let kept = read(&folder.join("reverted.json")).and_then(|bytes| match bytes {
        Some(bytes) => serde_json::from_slice::<Told>(&bytes)
            .map(|_| String::from_utf8_lossy(&bytes).into_owned())
            .map_err(|err| format!("reverted.json isn't JSON of its shape: {err}")),
        None => Ok(String::new()),
    });
    let reverts = match kept {
        Ok(text) => measures::reverts(&text, now, NET_RULES),
        Err(why) => measures::unread(measures::reverts("", now, NET_RULES), &why),
    };
    let broken = |text: &str, main: &measures::Main| {
        measures::broken(text, now, NET, main, &measures::failed_in)
    };
    let results = read(&state_home.join("net/results.jsonl"))
        .and_then(|bytes| results(&bytes.unwrap_or_default()));
    let broken = match results {
        Ok(text) => broken(&text, &measures::Main::read(core, quarantine)),
        Err(why) => measures::unread(broken("", &measures::Main::default()), &why),
    };
    [reverts, broken]
}

/// The net's results as text with every line JSON, or the first line that
/// isn't, since dropped it could be the pass that ends a check's failing
/// runs. A last line with no end yet is one the net is still appending, in
/// its one write per line, and is left for the next hour to read.
fn results(bytes: &[u8]) -> Result<String, String> {
    let (text, not_text) = text_lines(bytes);
    if let Some(n) = not_text {
        return Err(format!("results.jsonl line {n} isn't text"));
    }
    let ended = if text.is_empty() || text.ends_with('\n') {
        text.as_str()
    } else {
        text.rsplit_once('\n').map_or("", |(ended, _)| ended)
    };
    let torn = ended.lines().position(|line| {
        !line.trim().is_empty() && serde_json::from_str::<serde_json::Value>(line).is_err()
    });
    match torn {
        Some(n) => Err(format!("results.jsonl line {} isn't JSON", n + 1)),
        None => Ok(ended.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// The net's lines for runs of the check "broke", each ended.
    fn run_lines(runs: &[(i64, bool)]) -> String {
        runs.iter()
            .map(|(at, passed)| {
                let run = serde_json::json!({"check": "broke", "commit": "", "at": at,
                    "passed": passed, "stalled": false, "log": ""});
                format!("{run}\n")
            })
            .collect()
    }

    #[test]
    fn a_results_line_torn_or_misshapen_leaves_its_line_unread_and_one_being_written_waits() {
        let (root, folder, state) = folders();
        let core = root.path().join("core.git");
        let path = state.join("net/results.jsonl");
        let failing = run_lines(&[(100, true), (880, false), (940, false)]);
        // Dropped, the torn pass at 960 would leave "broke" failing since 880.
        let torn = format!("{failing}{{\"check\":\"broke\",\"at\":960\n");
        let not_text = [failing.as_bytes(), b"\xff\n"].concat();
        for (bytes, why) in [
            (torn.into_bytes(), "results.jsonl line 4 isn't JSON"),
            (not_text, "results.jsonl line 4 isn't text"),
        ] {
            fs::write(&path, bytes).expect("results that aren't whole");
            let [_, broken] = read_both(&folder, &state, &core, "", 1_000);
            assert_eq!(broken.target, None, "results not whole were judged: {why}");
            assert!(
                broken.says.contains(why),
                "results not whole didn't say {why:?}: {}",
                broken.says
            );
        }
        // A last line with no end yet is still being appended, not torn.
        let writing = format!("{failing}{{\"check\":\"broke\",\"at\":960");
        fs::write(&path, writing).expect("results being appended");
        let [_, broken] = read_both(&folder, &state, &core, "", 1_000);
        assert_eq!(
            (broken.target, broken.now),
            (Some(0.0), 1.0),
            "results with a line being appended weren't judged by their ended lines: {}",
            broken.says
        );
        fs::write(folder.join("reverted.json"), r#"{"reverts":5}"#).expect("a misshapen record");
        let [reverts, _] = read_both(&folder, &state, &core, "", 1_000);
        assert!(
            reverts.target.is_none() && reverts.says.contains("isn't JSON of its shape"),
            "a reverted.json of another shape was judged: {}",
            reverts.says
        );
    }

    /// A scratch root with cadence's folder and the net's, each empty.
    fn folders() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let root = tempfile::tempdir().expect("a scratch folder");
        let (folder, state) = (root.path().join("cadence"), root.path().join("state"));
        fs::create_dir_all(state.join("net")).expect("the net's folder");
        fs::create_dir_all(&folder).expect("cadence's folder");
        (root, folder, state)
    }

    #[test]
    fn the_nets_files_barred_or_torn_leave_their_lines_unjudged_saying_why() {
        let (root, folder, state) = folders();
        let core = root.path().join("core.git");
        // A folder where each file should be can't be read as one.
        fs::create_dir_all(folder.join("reverted.json")).expect("a barred reverted.json");
        fs::create_dir_all(state.join("net/results.jsonl")).expect("barred results");
        let [reverts, broken] = read_both(&folder, &state, &core, "", 1_000);
        assert_eq!(reverts.target, None, "a barred reverted.json was judged");
        assert_eq!(broken.target, None, "barred results were judged");
        assert!(
            reverts.says.contains("(unread: ")
                && reverts.says.contains("reverted.json can't be read"),
            "a barred reverted.json didn't say why: {}",
            reverts.says
        );
        assert!(
            broken.says.contains("(unread: ")
                && broken.says.contains("results.jsonl can't be read"),
            "barred results didn't say why: {}",
            broken.says
        );
        fs::remove_dir(folder.join("reverted.json")).expect("the barred folder gone");
        fs::write(folder.join("reverted.json"), "{\"reverts\":").expect("a torn file");
        let [reverts, _] = read_both(&folder, &state, &core, "", 1_000);
        assert_eq!(reverts.target, None, "a torn reverted.json was judged");
        assert!(
            reverts.says.contains("reverted.json isn't JSON"),
            "a torn reverted.json didn't say why: {}",
            reverts.says
        );
    }

    #[test]
    fn the_nets_files_never_written_or_whole_are_judged_by_what_they_hold() {
        let (root, folder, state) = folders();
        let core = root.path().join("core.git");
        let [reverts, broken] = read_both(&folder, &state, &core, "", 1_000);
        assert_eq!(
            (reverts.target, reverts.now, broken.target, broken.now),
            (Some(0.0), 0.0, Some(0.0), 0.0),
            "files never written, by a machine with no watcher or a net yet to run, were not \
             judged as holding nothing: {} / {}",
            reverts.says,
            broken.says
        );
        fs::write(
            folder.join("reverted.json"),
            r#"{"reverts":{"ended-in:4083":999}}"#,
        )
        .expect("a whole reverted.json");
        fs::write(
            state.join("net/results.jsonl"),
            run_lines(&[(100, true), (880, false), (940, false)]),
        )
        .expect("whole results");
        let [reverts, broken] = read_both(&folder, &state, &core, "", 1_000);
        assert_eq!(
            (reverts.now, reverts.behind, broken.now, broken.behind),
            (1.0, vec!["#4083".to_owned()], 1.0, vec!["broke".to_owned()]),
            "whole files' contents didn't reach their measures"
        );
    }
}
