//! The two measures the merge net feeds the hour: the merges it reverted,
//! from cadence's own `reverted.json`, and the checks broken on main,
//! from the net's results. Each file is read whole or the measure is
//! said unread, since read as empty a file torn or barred would count none
//! reverted and none broken, and close both lines as fixed while nothing
//! was looked at. A file not there is one never written, and holds nothing:
//! the tick writes `reverted.json` only where a watcher is set, and the net
//! writes its results only with its first run, as its own journal reads.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use super::measures::{self, Measure};
use super::{NET, NET_RULES};

/// The file at `path` whole, `None` when it was never written, or why it
/// can't be read.
fn read(path: &Path) -> Result<Option<String>, String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
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
    let kept = read(&folder.join("reverted.json")).and_then(|text| match text {
        Some(text) => serde_json::from_str::<serde_json::Value>(&text)
            .map(|_| text)
            .map_err(|err| format!("reverted.json isn't JSON: {err}")),
        None => Ok(String::new()),
    });
    let reverts = match kept {
        Ok(text) => measures::reverts(&text, now, NET_RULES),
        Err(why) => measures::unread(measures::reverts("", now, NET_RULES), &why),
    };
    let broken = |text: &str, main: &measures::Main| {
        measures::broken(text, now, NET, main, &measures::failed_in)
    };
    let broken = match read(&state_home.join("net/results.jsonl")) {
        Ok(text) => broken(
            &text.unwrap_or_default(),
            &measures::Main::read(core, quarantine),
        ),
        Err(why) => measures::unread(broken("", &measures::Main::default()), &why),
    };
    [reverts, broken]
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

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
        let results = [(100, true), (880, false), (940, false)]
            .map(|(at, passed)| {
                serde_json::json!({"check": "broke", "commit": "", "at": at, "passed": passed,
                    "stalled": false, "log": ""})
                .to_string()
            })
            .join("\n");
        fs::write(state.join("net/results.jsonl"), results).expect("whole results");
        let [reverts, broken] = read_both(&folder, &state, &core, "", 1_000);
        assert_eq!(
            (reverts.now, reverts.behind, broken.now, broken.behind),
            (1.0, vec!["#4083".to_owned()], 1.0, vec!["broke".to_owned()]),
            "whole files' contents didn't reach their measures"
        );
    }
}
