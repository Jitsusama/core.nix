//! The two measures the merge net feeds the hour: the merges it reverted,
//! from cadence's own `reverted.json`, and the checks broken on main,
//! from the net's results. Each file is read whole or the measure is
//! said unread, since read as empty a file lost or torn would count none
//! reverted and none broken, and close both lines as fixed while nothing
//! was looked at.

use std::fs;
use std::path::Path;

use super::measures::{self, Measure};
use super::{NET, NET_RULES};

/// The file at `path` whole, or why it can't be read.
fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|err| format!("{} can't be read: {err}", path.display()))
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
    let kept = read(&folder.join("reverted.json")).and_then(|text| {
        serde_json::from_str::<serde_json::Value>(&text)
            .map(|_| text)
            .map_err(|err| format!("reverted.json isn't JSON: {err}"))
    });
    let reverts = match kept {
        Ok(text) => measures::reverts(&text, now, NET_RULES),
        Err(why) => measures::unread(measures::reverts("", now, NET_RULES), &why),
    };
    let broken = |text: &str, main: &measures::Main| {
        measures::broken(text, now, NET, main, &measures::failed_in)
    };
    let broken = match read(&state_home.join("net/results.jsonl")) {
        Ok(text) => broken(&text, &measures::Main::read(core, quarantine)),
        Err(why) => measures::unread(broken("", &measures::Main::default()), &why),
    };
    [reverts, broken]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nets_files_lost_or_torn_leave_their_lines_unjudged_not_fixed() {
        let root = tempfile::tempdir().expect("a scratch folder");
        let (folder, state) = (root.path().join("cadence"), root.path().join("state"));
        fs::create_dir_all(state.join("net")).expect("the net's folder");
        fs::create_dir_all(&folder).expect("cadence's folder");
        let core = root.path().join("core.git");
        let [reverts, broken] = read_both(&folder, &state, &core, "", 1_000);
        assert_eq!(
            reverts.target, None,
            "a lost reverted.json was judged: {}",
            reverts.says
        );
        assert_eq!(
            broken.target, None,
            "lost results were judged: {}",
            broken.says
        );
        assert!(
            reverts.says.contains("unread"),
            "a lost file went unsaid: {}",
            reverts.says
        );
        fs::write(folder.join("reverted.json"), "{\"reverts\":").expect("a torn file");
        let [reverts, _] = read_both(&folder, &state, &core, "", 1_000);
        assert_eq!(
            reverts.target, None,
            "a torn reverted.json was judged: {}",
            reverts.says
        );
        fs::write(folder.join("reverted.json"), "{}").expect("a whole file");
        fs::write(state.join("net/results.jsonl"), "").expect("no results yet");
        let [reverts, broken] = read_both(&folder, &state, &core, "", 1_000);
        assert_eq!(
            reverts.target,
            Some(0.0),
            "a whole reverted.json went unjudged"
        );
        assert_eq!(broken.target, Some(0.0), "whole results went unjudged");
    }
}
