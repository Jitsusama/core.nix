//! The `there --changed` runs going now, each a file in
//! `nix-offload/running` under XDG's state folder, named by its process
//! and holding its commit, its tree's folder, when it started, and when
//! its process started in clock ticks since boot.
//!
//! Two changes sharing a file each pass their gate against main as it
//! stood, and the first to merge makes the other's pass stale, so it runs
//! again whole, on builders other gates are waiting for. merge-net's
//! `ready` reads these files and the start kept with each pass
//! (`passed.rs`) to merge such changes in the order their gates started,
//! holding a later one back while an earlier gate sharing its files still
//! runs.
//!
//! The file goes when the run ends, however its function returns. A run
//! killed by a signal leaves it behind, so a reader counts a file as ended
//! unless its process is there and started when the file says. A process
//! id comes back to another program, and about twenty nix-offloads run
//! all day on a machine that lends, so its name alone would be fooled.
//!
//! A run that finds its twin going, one in the same folder at the same
//! commit for the same system, waits for it to end rather than sending
//! the same work again (`twin`). From 15 November merge-net's `ready`
//! starts a change's run again itself in its author's tree, and an
//! author's loop that starts one too would otherwise double it on the
//! builders. A tree with changes not committed sends a commit of its own
//! making, so it never matches one without them.

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

/// A run's file, removed when this is dropped.
pub struct Running {
    file: PathBuf,
}

impl Running {
    /// Says that the run `pid` started at `started`, in seconds since
    /// 1970, for `commit` in the tree at `folder`. None, and nothing said,
    /// when its process's start can't be read or the file can't be
    /// written; the run goes on without it, and a merge waits on nothing
    /// for it.
    pub fn start(
        state: &Path,
        pid: u32,
        at: &Twin,
        started: u64,
    ) -> Option<Self> {
        let Twin {
            commit,
            folder,
            system,
        } = at;
        let running = state.join("nix-offload/running");
        let file = running.join(format!("{pid}.json"));
        let Some(ticks) = ticks(pid) else {
            eprintln!("nix-offload: couldn't read when process {pid} started, so it goes unsaid");
            return None;
        };
        let line = serde_json::json!({
            "pid": pid,
            "ticks": ticks,
            "commit": commit,
            "folder": folder.to_string_lossy(),
            "started": started,
            "system": system,
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

/// What makes two runs the same work: the commit sent, the tree it was
/// sent from, and the system its checks are for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Twin<'a> {
    pub commit: &'a str,
    pub folder: &'a Path,
    pub system: Option<&'a str>,
}

/// A run going now that does the same work as another would: its
/// process, and when it started, in seconds since 1970.
#[derive(Debug, PartialEq, Eq)]
pub struct Going {
    pub pid: u32,
    pub started: u64,
}

/// The run going under `state`, other than process `me`, that does the
/// work `at` names, if one does.
pub fn twin(state: &Path, me: u32, at: &Twin) -> Option<Going> {
    let folder = at.folder.to_string_lossy();
    fs::read_dir(state.join("nix-offload/running"))
        .ok()?
        .filter_map(|entry| fs::read_to_string(entry.ok()?.path()).ok())
        .filter_map(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .find_map(|said| {
            let pid = u32::try_from(said["pid"].as_u64()?).ok()?;
            let same = pid != me
                && said["commit"].as_str() == Some(at.commit)
                && said["folder"].as_str() == Some(&folder)
                && said["system"].as_str() == at.system;
            // A process id comes back to another program, so the run is
            // there only while its process started when the file says.
            (same && ticks(pid) == said["ticks"].as_u64()).then(|| Going {
                pid,
                started: said["started"].as_u64().unwrap_or_default(),
            })
        })
}

/// Wait while a twin of the run `at` goes, saying so once: whether one
/// went, so the caller looks again at what it passed.
pub fn waited_for_twin(state: &Path, me: u32, at: &Twin, now: impl Fn() -> u64) -> bool {
    let Some(going) = twin(state, me, at) else {
        return false;
    };
    eprintln!(
        "nix-offload: run {} already runs these changed-checks in this tree at {}, for {} s so \
         far, so this waits for it to end and then reads what it passed",
        going.pid,
        at.commit.get(..12).unwrap_or(at.commit),
        now().saturating_sub(going.started)
    );
    while twin(state, me, at).is_some() {
        // A run goes for minutes, so looking every two seconds costs
        // nothing and loses at most two of them.
        thread::sleep(Duration::from_secs(2));
    }
    true
}

/// When process `pid` started, in clock ticks since boot: the 22nd field
/// of its `stat`, counted after the name, which may hold spaces.
pub fn ticks(pid: u32) -> Option<u64> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let (_, after) = stat.rsplit_once(')')?;
    after.split_whitespace().nth(19)?.parse().ok()
}

/// A signal skips this, leaving the file behind; a reader checks the
/// process's start against the file's, so no handler is needed for it.
impl Drop for Running {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.file);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at<'a>(commit: &'a str, folder: &'a str, system: &'a str) -> Twin<'a> {
        Twin {
            commit,
            folder: Path::new(folder),
            system: Some(system),
        }
    }

    #[test]
    fn a_run_finds_its_twin_going_and_nothing_else() {
        let state = tempfile::tempdir().unwrap();
        let pid = std::process::id();
        let first = at("abc", "/t", "x86_64-linux");
        let _going = Running::start(state.path(), pid, &first, 7).expect("the run was said");
        assert_eq!(
            twin(state.path(), pid + 1, &first),
            Some(Going { pid, started: 7 }),
            "a run in the same tree at the same commit wasn't found"
        );
        for (other, why) in [
            (at("abd", "/t", "x86_64-linux"), "another commit"),
            (at("abc", "/u", "x86_64-linux"), "another tree"),
            (at("abc", "/t", "aarch64-darwin"), "another system"),
        ] {
            assert_eq!(
                twin(state.path(), pid + 1, &other),
                None,
                "a run for {why} was taken for a twin"
            );
        }
        assert_eq!(
            twin(state.path(), pid, &first),
            None,
            "a run was taken for its own twin"
        );
    }

    #[test]
    fn a_twin_whose_process_is_gone_or_another_is_not_waited_for() {
        let state = tempfile::tempdir().unwrap();
        let running = state.path().join("nix-offload/running");
        fs::create_dir_all(&running).unwrap();
        let pid = std::process::id();
        let line = serde_json::json!({
            "pid": pid, "ticks": 0, "commit": "abc", "folder": "/t", "started": 7,
            "system": "x86_64-linux"
        });
        fs::write(running.join(format!("{pid}.json")), line.to_string()).unwrap();
        assert_eq!(
            twin(state.path(), pid + 1, &at("abc", "/t", "x86_64-linux")),
            None,
            "a run left behind by a process since reused was waited for"
        );
    }

    #[test]
    fn a_second_run_waits_for_its_twin_to_end() {
        let state = tempfile::tempdir().unwrap();
        let mut child = std::process::Command::new("sh")
            .args(["-c", "sleep 2"])
            .spawn()
            .expect("a stand-in run started");
        let first = at("abc", "/t", "x86_64-linux");
        let going = Running::start(state.path(), child.id(), &first, 7);
        assert!(going.is_some(), "the stand-in run wasn't said");
        let reaper = thread::spawn(move || {
            let _ = child.wait();
            drop(going);
        });
        let begun = std::time::Instant::now();
        let waited = waited_for_twin(state.path(), std::process::id(), &first, || 10);
        assert!(waited, "a second run in the same tree didn't wait for the first");
        assert!(
            begun.elapsed() >= Duration::from_secs(1),
            "the second run went on while the first still ran: {:?}",
            begun.elapsed()
        );
        reaper.join().unwrap();
        assert!(
            !waited_for_twin(state.path(), std::process::id(), &first, || 10),
            "a run waited where nothing else went"
        );
    }

    #[test]
    fn a_run_is_said_while_it_goes_and_not_after() {
        let state = tempfile::tempdir().unwrap();
        let pid = std::process::id();
        let ticks = ticks(pid).expect("this process's start is read");
        let file = state.path().join(format!("nix-offload/running/{pid}.json"));
        {
            let running = Running::start(state.path(), pid, &at("abc", "/t", "x"), 7);
            assert!(running.is_some(), "the run wasn't said");
            let said: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
            assert_eq!(
                said,
                serde_json::json!({
                    "pid": pid, "ticks": ticks, "commit": "abc", "folder": "/t", "started": 7,
                    "system": "x"
                }),
                "the run was said wrongly"
            );
        }
        assert!(!file.exists(), "the run's file outlived it");
    }

    #[test]
    fn a_run_that_cant_be_said_goes_on_unsaid() {
        let state = tempfile::tempdir().unwrap();
        fs::write(state.path().join("nix-offload"), "a file, not a folder").unwrap();
        let pid = std::process::id();
        assert!(
            Running::start(state.path(), pid, &at("abc", "/t", "x"), 7).is_none(),
            "a run was said where nothing could be written"
        );
    }

    #[test]
    fn a_process_s_start_is_read_and_none_for_one_gone() {
        let pid = std::process::id();
        assert!(ticks(pid).is_some(), "this process's start wasn't read");
        assert_eq!(
            ticks(u32::MAX),
            None,
            "a process that isn't there had a start"
        );
    }
}
