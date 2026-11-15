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
//! commit asked the same, waits for it to end rather than sending
//! the same work again (`alone`). Each takes a lock named for its work in
//! `nix-offload/twins`, so of two starting together exactly one goes,
//! however their steps fall. From 15 November merge-net's `ready`
//! starts a change's run again itself in its author's tree, and an
//! author's loop that starts one too would otherwise double it on the
//! builders. Runs match only when they send the same commit, and a run
//! from a tree with changes not committed sends a commit it makes then
//! (`there.rs`), so those changes keep it apart from the run before.

use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

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
    pub fn start(state: &Path, pid: u32, at: &Twin, started: u64) -> Option<Self> {
        let Twin {
            commit,
            folder,
            asked,
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
            "asked": asked,
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
/// sent from, and what changed-checks was asked, its system included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Twin<'a> {
    pub commit: &'a str,
    pub folder: &'a Path,
    pub asked: String,
}

impl<'a> Twin<'a> {
    pub fn new(commit: &'a str, folder: &'a Path, arguments: &[String]) -> Self {
        Self {
            commit,
            folder,
            asked: arguments.join(" "),
        }
    }
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
                && said["asked"].as_str() == Some(&at.asked);
            // A process id comes back to another program, so the run is
            // there only while its process started when the file says.
            (same && ticks(pid) == said["ticks"].as_u64()).then(|| Going {
                pid,
                started: said["started"].as_u64().unwrap_or_default(),
            })
        })
}

/// Whether a run does its work alone, from [`alone`].
pub enum Alone {
    /// None else does it, and none will while this is held.
    Held(Held),
    /// Another did it, and this waited for that one to end.
    Waited,
    /// The lock couldn't be had, so the run goes on as before it was.
    Unheld,
}

/// The lock on a run's work, its file removed when this is dropped. A
/// run waiting on the file as it goes wakes holding a lock no other sees,
/// so it looks again (`Alone::Waited`) rather than going on with it.
pub struct Held {
    file: PathBuf,
    _lock: fs::File,
}

impl Drop for Held {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.file);
    }
}

/// Take the work `at` names for the run `me`, or wait for the run doing
/// it to end, saying so with the one going's process and how long it has
/// gone by `now`, in seconds since 1970.
pub fn alone(state: &Path, me: u32, at: &Twin, now: impl Fn() -> u64) -> Alone {
    let twins = state.join("nix-offload/twins");
    let mut work = DefaultHasher::new();
    (at.folder, &at.asked).hash(&mut work);
    let file = twins.join(format!("{}-{:016x}.lock", at.commit, work.finish()));
    let opened = fs::create_dir_all(&twins).and_then(|()| {
        fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&file)
    });
    let Ok(lock) = opened else {
        return Alone::Unheld;
    };
    match lock.try_lock() {
        Ok(()) => return Alone::Held(Held { file, _lock: lock }),
        Err(fs::TryLockError::WouldBlock) => {}
        Err(fs::TryLockError::Error(_)) => return Alone::Unheld,
    }
    let going = twin(state, me, at).map_or_else(
        || "another run".to_owned(),
        |going| {
            format!(
                "run {}, going {} s so far,",
                going.pid,
                now().saturating_sub(going.started)
            )
        },
    );
    eprintln!(
        "nix-offload: {going} already runs these changed-checks in this tree at {}, so this \
         waits for it to end and then reads what it passed",
        at.commit.get(..12).unwrap_or(at.commit),
    );
    match lock.lock() {
        Ok(()) => Alone::Waited,
        Err(_) => Alone::Unheld,
    }
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

    fn at<'a>(commit: &'a str, folder: &'a str, asked: &str) -> Twin<'a> {
        Twin {
            commit,
            folder: Path::new(folder),
            asked: asked.to_owned(),
        }
    }

    const ASKED: &str = "--system x86_64-linux";

    #[test]
    fn a_run_finds_its_twin_going_and_nothing_else() {
        let state = tempfile::tempdir().unwrap();
        let pid = std::process::id();
        let first = at("abc", "/t", ASKED);
        let _going = Running::start(state.path(), pid, &first, 7).expect("the run was said");
        assert_eq!(
            twin(state.path(), u32::MAX, &first),
            Some(Going { pid, started: 7 }),
            "a run in the same tree at the same commit wasn't found"
        );
        for (other, why) in [
            (at("abd", "/t", ASKED), "another commit"),
            (at("abc", "/u", ASKED), "another tree"),
            (at("abc", "/t", "--system aarch64-darwin"), "another system"),
            (at("abc", "/t", "--build --system x86_64-linux"), "another ask"),
        ] {
            assert_eq!(
                twin(state.path(), u32::MAX, &other),
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
            "asked": ASKED
        });
        fs::write(running.join(format!("{pid}.json")), line.to_string()).unwrap();
        assert_eq!(
            twin(state.path(), u32::MAX, &at("abc", "/t", ASKED)),
            None,
            "a run left behind by a process since reused was waited for"
        );
    }

    #[test]
    fn a_second_run_waits_for_its_twin_to_end() {
        use std::time::{Duration, Instant};
        let state = tempfile::tempdir().unwrap();
        let first = at("abc", "/t", ASKED);
        let held = alone(state.path(), 1, &first, || 10);
        assert!(
            matches!(held, Alone::Held(_)),
            "the first run didn't hold its work"
        );
        assert!(
            matches!(
                alone(state.path(), 2, &at("abc", "/t", "--build"), || 10),
                Alone::Held(_)
            ),
            "a run asked something else waited on the first"
        );
        let begun = Instant::now();
        let ender = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(1));
            drop(held);
        });
        let waited = alone(state.path(), 2, &first, || 10);
        assert!(
            matches!(waited, Alone::Waited),
            "a second run in the same tree didn't wait for the first"
        );
        assert!(
            begun.elapsed() >= Duration::from_secs(1),
            "the second run went on while the first still ran: {:?}",
            begun.elapsed()
        );
        ender.join().unwrap();
        assert!(
            matches!(alone(state.path(), 2, &first, || 10), Alone::Held(_)),
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
                    "asked": "x"
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
