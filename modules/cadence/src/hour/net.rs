//! The two measures the merge net feeds the hour: the merges it reverted,
//! from cadence's own `reverted.json`, and the checks broken on main,
//! from the net's results. Each file is read whole or the measure is
//! said unread, since read as empty a file torn or barred would count none
//! reverted and none broken, and close both lines as fixed while nothing
//! was looked at. A file not there is one never written, and holds nothing:
//! the tick writes `reverted.json` only where a watcher is set, and the net
//! writes its results only with its first run, as its own journal reads.
//! A results line that isn't text or isn't JSON is read past and named on
//! the line, rather than leaving it unread: the net's results are only ever
//! appended to, so one bad line would leave the line unread for good. Read
//! past, a torn pass can show a check broken until its next run, and a torn
//! failure can show it passing, as main's reader did before with no word of
//! it; the note on the line is what tells the two from a whole file. A
//! failure to read the file at all still leaves the line unread.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use super::measures::{self, Measure};
use super::{NET, NET_RULES};
use crate::reverted::Told;

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
    let judged = |text: &str, main: &measures::Main| {
        measures::broken(text, now, NET, main, &measures::failed_in)
    };
    let broken = match read(&state_home.join("net/results.jsonl")) {
        Ok(bytes) => {
            let (text, past) = results(&bytes.unwrap_or_default());
            let mut m = judged(&text, &measures::Main::read(core, quarantine));
            if !past.is_empty() {
                m.says = format!("{} (read past results.jsonl {})", m.says, past.join(", "));
            }
            m
        }
        Err(why) => measures::unread(judged("", &measures::Main::default()), &why),
    };
    [reverts, broken]
}

/// The net's results as text of its ended lines that are JSON, and what
/// was read past: each ended line that isn't text or isn't JSON, by its
/// number in the file. A last line with no end yet is one the net is
/// still appending, in its one write per line, and is left for the next
/// hour to read, whatever its bytes, since its end is judged before its
/// text.
fn results(bytes: &[u8]) -> (String, Vec<String>) {
    let mut past = Vec::new();
    let mut kept = String::new();
    let Some(end) = bytes.iter().rposition(|&byte| byte == b'\n') else {
        return (kept, past);
    };
    let ended = &bytes[..end];
    for (n, line) in ended.split(|&byte| byte == b'\n').enumerate() {
        match std::str::from_utf8(line) {
            Err(_) => past.push(format!("line {}, which isn't text", n + 1)),
            Ok(text)
                if text.trim().is_empty()
                    || serde_json::from_str::<serde_json::Value>(text).is_ok() =>
            {
                kept.push_str(text);
                kept.push('\n');
            }
            Ok(_) => past.push(format!("line {}, which isn't JSON", n + 1)),
        }
    }
    (kept, past)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// The net's lines for runs of the check "broke", each ended.
    fn run_lines(runs: &[(i64, bool)]) -> String {
        runs.iter()
            .map(|(at, passed)| {
                serde_json::json!({"check": "broke", "commit": "", "at": at,
                    "passed": passed, "stalled": false, "log": ""})
                .to_string()
                    + "\n"
            })
            .collect()
    }

    #[test]
    fn a_results_line_torn_or_not_text_is_read_past_and_named_and_one_being_written_waits() {
        let (root, folder, state) = folders();
        let core = root.path().join("core.git");
        let path = state.join("net/results.jsonl");
        let failing = run_lines(&[(100, true), (880, false), (940, false)]);
        // Read past, the torn pass at 960 leaves "broke" failing since
        // 880, and the line says which it read past.
        let torn = format!("{failing}{{\"check\":\"broke\",\"at\":960\n");
        let not_text = [failing.as_bytes(), b"\xff\n"].concat();
        for (bytes, why) in [
            (
                torn.into_bytes(),
                "read past results.jsonl line 4, which isn't JSON",
            ),
            (not_text, "read past results.jsonl line 4, which isn't text"),
        ] {
            fs::write(&path, bytes).expect("results that aren't whole");
            let [_, broken] = read_both(&folder, &state, &core, "", 1_000);
            assert_eq!(
                (broken.target, broken.now),
                (Some(0.0), 1.0),
                "results with a bad line weren't judged by the rest: {why}"
            );
            assert!(
                broken.says.contains(why),
                "results with a bad line didn't say {why:?}: {}",
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
        assert!(
            !broken.says.contains("read past"),
            "a line being appended was read past, not left to wait: {}",
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

    #[test]
    fn each_bad_line_is_named_by_its_place_in_the_file_and_a_cut_last_one_waits() {
        let ended = run_lines(&[(100, true), (880, false)]);
        let bad = [ended.as_bytes(), b"\xff\n\xfe\n{torn\n"].concat();
        assert_eq!(
            results(&bad),
            (
                ended.clone(),
                vec![
                    "line 3, which isn't text".to_owned(),
                    "line 4, which isn't text".to_owned(),
                    "line 5, which isn't JSON".to_owned(),
                ]
            ),
            "a bad line was named by its place among the text lines, not the file's"
        );
        // Cut inside a character as it is appended, a last line still
        // waits, and the ended line before it is kept.
        let cut = [ended.as_bytes(), b"{\"log\":\"\xc3"].concat();
        assert_eq!(
            results(&cut),
            (ended, Vec::new()),
            "a last line cut in a character was named, or dropped the line before it"
        );
        assert_eq!(
            results(b"{\"at\":1"),
            (String::new(), Vec::new()),
            "a first line still being appended was read"
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
