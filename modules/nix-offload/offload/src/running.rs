//! The `there --changed` runs going now, each a file in
//! `nix-offload/running` under XDG's state folder, named by its process
//! and holding its commit, its tree's folder and when it started.
//!
//! Two changes sharing a file each pass their gate against main as it
//! stood, and the first to merge makes the other's pass stale, so it runs
//! again whole, on builders other gates are waiting for. merge-net's
//! `ready` reads these files and the start kept with each pass
//! (`passed.rs`) to merge such changes in the order their gates started, holding a later one back while an earlier gate
//! sharing its files still runs.
//!
//! The file goes when the run ends, however its function returns. A run
//! killed by a signal leaves it behind, so a reader counts a file whose
//! process is gone, or is another program now, as ended.

use std::fs;
use std::path::{Path, PathBuf};

/// A run's file, removed when this is dropped.
pub struct Running {
    file: PathBuf,
}

impl Running {
    /// Says that the run `pid` started at `started`, in seconds since
    /// 1970, for `commit` in the tree at `folder`. None, and nothing said,
    /// when the file can't be written; the run goes on without it, and a
    /// merge waits on nothing for it.
    pub fn start(
        state: &Path,
        pid: u32,
        commit: &str,
        folder: &Path,
        started: u64,
    ) -> Option<Self> {
        let running = state.join("nix-offload/running");
        let file = running.join(format!("{pid}.json"));
        let line = serde_json::json!({
            "pid": pid,
            "commit": commit,
            "folder": folder.to_string_lossy(),
            "started": started,
        });
        let partial = running.join(format!("{pid}.json.new"));
        let written = fs::create_dir_all(&running)
            .and_then(|()| fs::write(&partial, format!("{line}\n")))
            .and_then(|()| fs::rename(&partial, &file));
        match written {
            Ok(()) => Some(Self { file }),
            Err(error) => {
                eprintln!(
                    "nix-offload: couldn't say this run is going in {}: {error}",
                    file.display()
                );
                None
            }
        }
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.file);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_is_said_while_it_goes_and_not_after() {
        let state = tempfile::tempdir().unwrap();
        let file = state.path().join("nix-offload/running/42.json");
        {
            let running = Running::start(state.path(), 42, "abc", Path::new("/t"), 7);
            assert!(running.is_some(), "the run wasn't said");
            let said: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
            assert_eq!(
                said,
                serde_json::json!({"pid": 42, "commit": "abc", "folder": "/t", "started": 7}),
                "the run was said wrongly"
            );
        }
        assert!(!file.exists(), "the run's file outlived it");
    }

    #[test]
    fn a_run_that_cant_be_said_goes_on_unsaid() {
        let state = tempfile::tempdir().unwrap();
        fs::write(state.path().join("nix-offload"), "a file, not a folder").unwrap();
        assert!(
            Running::start(state.path(), 42, "abc", Path::new("/t"), 7).is_none(),
            "a run was said where nothing could be written"
        );
    }
}
