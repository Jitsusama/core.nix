//! What of core.nix's merges the running system carries, so a landing of
//! core's waits to be tried until a switch brings it (WCJQSG, 2026-11-13:
//! Joel tried a line at 20:10 that no switch had carried yet, and it
//! failed, as clock-follows had at 19:35).
//!
//! update-to-main says the core it followed in its last ask's answer,
//! `followed.core`, which is what's running once that ask's `step` is
//! `done`; a failed ask rewrites it, and the folder it's in goes at a
//! boot, so the daemon keeps the last done one in a file of its own.
//! Which merges that commit holds is read from a clone of core's
//! (cadence keeps one, fetched each tick) by the subjects on its first
//! parents, `Merge pull request #N from ...`, as main merges each change.
//! Where any of it can't be told, nothing is held back, as before.

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::{Value, json};

use crate::file;

/// What of core's merges the running system carries, by their numbers,
/// or none where that can't be told.
#[derive(Debug, Default)]
pub struct Carried {
    merged: Option<HashSet<u64>>,
}

impl Carried {
    /// What the system carries, as update-to-main's answer at `switched`
    /// says it followed once done, else as the daemon kept it at `kept`,
    /// its merges read from core's clone at `clone`. Each done ask's core
    /// is kept for the next start.
    pub fn read(switched: &Path, kept: &Path, clone: &Path) -> Self {
        let done = file::read(switched).and_then(|said| followed(&said).map(str::to_owned));
        let was = file::read(kept).and_then(|said| said["core"].as_str().map(str::to_owned));
        if let Some(core) = &done
            && was.as_ref() != Some(core)
            && let Err(why) = keep(kept, core)
        {
            eprintln!("agents: can't keep {}: {why}", kept.display());
        }
        let merged = done.or(was).and_then(|core| {
            let log = Command::new("git")
                .arg("-C")
                .arg(clone)
                .args(["log", "--first-parent", "--format=%s", &core])
                .output()
                .ok()
                .filter(|ran| ran.status.success())?;
            Some(merged(&String::from_utf8_lossy(&log.stdout)))
        });
        Self { merged }
    }

    /// Whether landing `one` is on the running system: one of core's only
    /// once its merge is carried, and any other, or any where that can't
    /// be told, as it lands.
    pub fn carries(&self, one: &Value) -> bool {
        let repo = one["repo"].as_str().unwrap_or("");
        let number = one["ref"]
            .as_str()
            .and_then(|said| said.strip_prefix("core.nix#"))
            .and_then(|number| number.parse::<u64>().ok());
        match (&self.merged, number) {
            (Some(merged), Some(number)) if repo.rsplit('/').next() == Some("core.nix") => {
                merged.contains(&number)
            }
            _ => true,
        }
    }

    /// One carrying exactly `merged`, for a test.
    #[cfg(test)]
    pub fn of(merged: &[u64]) -> Self {
        Self {
            merged: Some(merged.iter().copied().collect()),
        }
    }
}

/// The core update-to-main followed, from its last answer `said`, where
/// that ask is done; a failed or stopped one says nothing of what runs.
pub fn followed(said: &Value) -> Option<&str> {
    let core = said["followed"]["core"].as_str()?;
    (said["step"] == "done" && core.len() == 40 && core.chars().all(|c| c.is_ascii_hexdigit()))
        .then_some(core)
}

/// The changes merged on first parents whose subjects are `log`, a line
/// each, by their numbers.
pub fn merged(log: &str) -> HashSet<u64> {
    log.lines()
        .filter_map(|line| line.strip_prefix("Merge pull request #"))
        .filter_map(|rest| rest.split(' ').next()?.parse().ok())
        .collect()
}

/// Keeps `core` at `kept`, written whole and renamed into place.
fn keep(kept: &Path, core: &str) -> std::io::Result<()> {
    if let Some(folder) = kept.parent() {
        fs::create_dir_all(folder)?;
    }
    let writing = kept.with_extension("json.new");
    fs::write(&writing, json!({ "core": core }).to_string())?;
    fs::rename(writing, kept)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORE: &str = "7aab2c55ccf240baa727bd4ac09b82b1079ed695";

    #[test]
    fn only_a_done_ask_says_which_core_runs() {
        let done = json!({"step": "done", "followed": {"core": CORE}});
        assert_eq!(followed(&done), Some(CORE), "a done ask's core runs");
        let failed = json!({"step": "failed", "followed": {"core": CORE}});
        assert_eq!(followed(&failed), None, "a failed ask's core may not run");
        let short = json!({"step": "done", "followed": {"core": "7aab2c55"}});
        assert_eq!(followed(&short), None, "only a whole commit names one");
    }

    #[test]
    fn a_merge_is_read_from_its_subject_on_main() {
        let log = "Merge pull request #4199 from Jitsusama/jitsusama/host-down\n\
                   fix(pi-host): stay quiet\n\
                   Merge pull request #4178 from Jitsusama/eero-narrow\n";
        assert_eq!(
            merged(log),
            HashSet::from([4199, 4178]),
            "each merge by its number, the commits beneath none"
        );
    }

    #[test]
    fn a_landing_of_cores_is_carried_only_once_its_merge_runs() {
        let carried = Carried::of(&[4177]);
        let landing = |repo: &str, at: &str| json!({"repo": repo, "ref": at});
        for (one, carries, why) in [
            (
                landing("Jitsusama/core.nix", "core.nix#4177"),
                true,
                "its merge runs",
            ),
            (
                landing("Jitsusama/core.nix", "core.nix#4178"),
                false,
                "no switch brought it",
            ),
            (
                landing("core.nix", "core.nix#4178"),
                false,
                "core named alone is core",
            ),
            (
                landing("Jitsusama/mycelium", "mycelium#154"),
                true,
                "another's isn't told",
            ),
            (
                landing("", "PLAN-20261113-KRINS0#3"),
                true,
                "a plan's item isn't a merge",
            ),
        ] {
            assert_eq!(carried.carries(&one), carries, "{why}");
        }
        assert!(
            Carried::default().carries(&landing("Jitsusama/core.nix", "core.nix#4178")),
            "with nothing known, nothing waits"
        );
    }

    #[test]
    fn a_done_asks_core_is_kept_for_when_the_answer_is_gone() {
        let folder = std::env::temp_dir().join(format!("agents-carried-{}", std::process::id()));
        let switched = folder.join("last.json");
        let kept = folder.join("agents").join("carried.json");
        fs::create_dir_all(&folder).expect("a folder for the test");
        fs::write(
            &switched,
            json!({"step": "done", "followed": {"core": CORE}}).to_string(),
        )
        .expect("an answer to read");
        let _ = Carried::read(&switched, &kept, &folder.join("core.git"));
        fs::remove_file(&switched).expect("the answer gone, as at a boot");
        let said = file::read(&kept).expect("the core kept");
        assert_eq!(said["core"], CORE, "the done ask's core is kept");
        fs::remove_dir_all(&folder).expect("the test's folder gone");
    }
}
