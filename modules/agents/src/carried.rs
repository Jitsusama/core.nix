//! What of core.nix's, the harness's and mycelium's merges the running
//! system carries, so a landing waits to be tried until it is on the
//! laptop (WCJQSG, 2026-11-13: Joel tried a line at 20:10 that no switch
//! had carried yet, and it failed, as clock-follows had at 19:35).
//!
//! update-to-main says the core it followed in its last ask's answer,
//! `followed.core`, which is what's running once that ask's `step` is
//! `done`; a failed ask rewrites it, and the folder it's in goes at a
//! boot, so the daemon keeps the last done one in a file of its own.
//! Which merges that commit holds is read from a clone of core's
//! (cadence keeps one, fetched each tick) by the subjects on its first
//! parents, `Merge pull request #N from ...`, as main merges each change.
//!
//! The harness runs as pi-packages installed it: its `current` link names
//! the commit, whose merges its mirror's history says the same way, read
//! again as the link moves. Mycelium runs as that core's flake.lock pins
//! it, but no clone of mycelium's is kept here, so its pin is told by the
//! time of the commit it pins. That's most often the merge of the
//! change it was bumped for, as on 13 November, when core pinned #152's
//! and the harness wrote #152's landing 7.5 s later; the harness writes
//! one 3 to 34 s after GitHub merges it, so a landing written within
//! 34 s of the pin, or before it, is the pin's own or older. Only a merge
//! within 31 s of the one pinned would be taken as carried when it isn't;
//! mycelium's closest that day were 52 s apart.
//! Where any of it can't be told, nothing is held back, as before.

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::{Value, json};

use crate::{clock, file};

/// How long after its merge the harness writes a landing at the most,
/// in ms: 34 s, the most seen on 13 November.
const RECORDED_WITHIN: i64 = 34_000;

/// What of each repository's merges the running system carries, by their
/// numbers, or of mycelium's by the time of its pin, each none where that
/// can't be told.
#[derive(Debug, Default)]
pub struct Carried {
    core: Option<HashSet<u64>>,
    harness: Option<HashSet<u64>>,
    mycelium: Option<i64>,
}

/// Where what runs is said: update-to-main's last answer, the core the
/// daemon kept, core's clone, and where pi-packages installs the harness.
#[derive(Debug, Clone, Copy)]
pub struct Said<'a> {
    pub switched: Option<&'a Path>,
    pub kept: &'a Path,
    pub clone: Option<&'a Path>,
    pub harness: Option<&'a Path>,
}

impl Carried {
    /// What the system carries, as `said` tells it: core as update-to-main
    /// says it followed once done, else as the daemon kept it, its merges
    /// and mycelium's pin read from core's clone; the harness as installed.
    /// Each done ask's core is kept for the next start.
    pub fn read(said: Said) -> Self {
        let done = said
            .switched
            .and_then(file::read)
            .and_then(|answer| followed(&answer).map(str::to_owned));
        let was = file::read(said.kept).and_then(|kept| kept["core"].as_str().map(str::to_owned));
        if let Some(core) = &done
            && was.as_ref() != Some(core)
            && let Err(why) = keep(said.kept, core)
        {
            eprintln!("agents: can't keep {}: {why}", said.kept.display());
        }
        let core = done.or(was);
        let (core, mycelium) = match (said.clone, core) {
            (Some(clone), Some(core)) => (
                git(clone, &["log", "--first-parent", "--format=%s", &core])
                    .map(|log| merged(&log)),
                git(clone, &["show", &format!("{core}:flake.lock")])
                    .and_then(|lock| pinned(&serde_json::from_str(&lock).ok()?)),
            ),
            _ => (None, None),
        };
        let harness = said.harness.and_then(|folder| {
            let commit = installed(&fs::read_link(folder.join("current")).ok()?)?;
            let log = git(
                &folder.join("mirror.git"),
                &["log", "--first-parent", "--format=%s", &commit],
            )?;
            Some(merged(&log))
        });
        Self {
            core,
            harness,
            mycelium,
        }
    }

    /// Whether landing `one` is on the running system: one of core's or
    /// the harness's once its merge is carried, one of mycelium's once its
    /// pin is surely past it, and any other, or any where that can't be
    /// told, as it lands.
    pub fn carries(&self, one: &Value) -> bool {
        let repo = one["repo"].as_str().unwrap_or("");
        let name = repo.rsplit('/').next().unwrap_or("");
        let number = one["ref"]
            .as_str()
            .and_then(|said| said.strip_prefix(name)?.strip_prefix('#'))
            .and_then(|number| number.parse::<u64>().ok());
        let holds = |merged: &Option<HashSet<u64>>| match (merged, number) {
            (Some(merged), Some(number)) => merged.contains(&number),
            _ => true,
        };
        match name {
            "core.nix" => holds(&self.core),
            "agentic-harness.pi" => holds(&self.harness),
            "mycelium" => match (self.mycelium, clock::ms_of(&one["at"]), number) {
                (Some(pin), Some(at), Some(_)) => at - RECORDED_WITHIN <= pin,
                _ => true,
            },
            _ => true,
        }
    }

    /// One carrying exactly `merged` of core's, for a test.
    #[cfg(test)]
    pub fn of(merged: &[u64]) -> Self {
        Self {
            core: Some(merged.iter().copied().collect()),
            ..Self::default()
        }
    }
}

/// What git says in the repository at `folder`, run with `args`, where it
/// says it without failing.
fn git(folder: &Path, args: &[&str]) -> Option<String> {
    let ran = Command::new("git")
        .arg("-C")
        .arg(folder)
        .args(args)
        .output()
        .ok()
        .filter(|ran| ran.status.success())?;
    Some(String::from_utf8_lossy(&ran.stdout).into_owned())
}

/// The commit pi-packages installed, from where its `current` link
/// points, `versions/<commit>`.
fn installed(points: &Path) -> Option<String> {
    let commit = points.file_name()?.to_str()?;
    whole(commit).then(|| commit.to_owned())
}

/// When the commit core's `lock` pins mycelium at was made, in ms.
fn pinned(lock: &Value) -> Option<i64> {
    lock["nodes"]["mycelium"]["locked"]["lastModified"]
        .as_i64()
        .map(|seconds| seconds * 1000)
}

/// Whether `commit` names one whole, as forty hexadecimal digits.
fn whole(commit: &str) -> bool {
    commit.len() == 40 && commit.chars().all(|c| c.is_ascii_hexdigit())
}

/// The core update-to-main followed, from its last answer `said`, where
/// that ask is done; a failed or stopped one says nothing of what runs.
pub fn followed(said: &Value) -> Option<&str> {
    let core = said["followed"]["core"].as_str()?;
    (said["step"] == "done" && whole(core)).then_some(core)
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
                "mycelium's pin isn't told here",
            ),
            (
                landing("Jitsusama/agentic-harness.pi", "agentic-harness.pi#1521"),
                true,
                "the harness's install isn't told here",
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
    fn a_landing_of_the_harnesss_is_carried_once_its_install_holds_its_merge() {
        let carried = Carried {
            harness: Some(HashSet::from([1521])),
            ..Carried::default()
        };
        let landing = |number: u64| {
            json!({"repo": "Jitsusama/agentic-harness.pi",
                   "ref": format!("agentic-harness.pi#{number}")})
        };
        assert!(carried.carries(&landing(1521)), "its merge is installed");
        assert!(
            !carried.carries(&landing(1523)),
            "pi-packages hasn't moved to it"
        );
        assert!(
            carried.carries(&json!({"repo": "Jitsusama/core.nix", "ref": "core.nix#4178"})),
            "core's isn't told by the harness's install"
        );
    }

    #[test]
    fn the_harness_install_is_the_commit_its_link_names() {
        let commit = "5e6e94320e16624795aa25b62713ce13557335b8";
        assert_eq!(
            installed(Path::new(&format!("versions/{commit}"))).as_deref(),
            Some(commit),
            "the version's folder is named by its commit"
        );
        assert_eq!(
            installed(Path::new("versions/next")),
            None,
            "only a commit names one"
        );
    }

    #[test]
    fn a_landing_of_myceliums_is_carried_once_cores_pin_is_its_merge_or_after() {
        // The pin, 2026-11-13T20:50:01Z, and landings written about it.
        let pin = 1_794_603_001_000;
        let carried = Carried {
            mycelium: Some(pin),
            ..Carried::default()
        };
        let landing =
            |at: &str| json!({"repo": "Jitsusama/mycelium", "ref": "mycelium#155", "at": at});
        for (at, carries, why) in [
            (
                "2026-11-13T20:40:00.000Z",
                true,
                "written well before the pin",
            ),
            (
                "2026-11-13T20:50:08.500Z",
                true,
                "written 7.5 s after, so it's the pin's own merge",
            ),
            (
                "2026-11-13T20:50:36.000Z",
                false,
                "written 35 s after, so merged after it",
            ),
            ("2026-11-13T22:26:00.000Z", false, "merged after the pin"),
        ] {
            assert_eq!(carried.carries(&landing(at)), carries, "{why}");
        }
        assert_eq!(
            pinned(&json!({"nodes": {"mycelium": {"locked": {"lastModified": 1_794_603_001}}}})),
            Some(pin),
            "the pin's time is its commit's, in ms"
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
        let clone = folder.join("core.git");
        let _ = Carried::read(Said {
            switched: Some(&switched),
            kept: &kept,
            clone: Some(&clone),
            harness: None,
        });
        fs::remove_file(&switched).expect("the answer gone, as at a boot");
        let said = file::read(&kept).expect("the core kept");
        assert_eq!(said["core"], CORE, "the done ask's core is kept");
        fs::remove_dir_all(&folder).expect("the test's folder gone");
    }
}
