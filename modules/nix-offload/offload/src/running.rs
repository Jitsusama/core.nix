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
use std::path::{Path, PathBuf};
use std::time::Duration;

/// How often a run waiting on its twin says it still waits, so one that
/// hangs behind a hung first shows it rather than going quiet.
const SAY_EVERY: Duration = Duration::from_secs(300);

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
    /// Another does it, and this was told to wait for nothing.
    Busy,
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

/// FNV-1a over `parts`, a zero byte between each, as every build of this
/// program reckons it. std's `DefaultHasher` leaves its algorithm free to
/// change between Rust releases, so the installed nix-offload and a dev
/// shell's could name one piece of work differently and each do it.
pub fn stable_digest(parts: &[&[u8]]) -> u64 {
    const BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0100_0000_01b3;
    let mut digest = BASIS;
    for (index, part) in parts.iter().enumerate() {
        let between: &[u8] = if index == 0 { &[] } else { &[0] };
        for &byte in between.iter().chain(part.iter()) {
            digest = (digest ^ u64::from(byte)).wrapping_mul(PRIME);
        }
    }
    digest
}

/// Take the work `at` names for the run `me`, or, where it `waits`, wait
/// for the run doing it to end, saying so with the one going's process
/// and how long it has gone by `now`, in seconds since 1970.
pub fn alone(state: &Path, me: u32, at: &Twin, now: impl Fn() -> u64, waits: bool) -> Alone {
    alone_saying(state, me, at, now, waits, SAY_EVERY, &mut |said| {
        eprintln!("{said}");
    })
}

/// [`alone`], saying through `say` that it waits, and again every `every`
/// while it still does.
fn alone_saying(
    state: &Path,
    me: u32,
    at: &Twin,
    now: impl Fn() -> u64,
    waits: bool,
    every: Duration,
    say: &mut dyn FnMut(String),
) -> Alone {
    let twins = state.join("nix-offload/twins");
    let work = stable_digest(&[
        at.folder.as_os_str().as_encoded_bytes(),
        at.asked.as_bytes(),
    ]);
    let file = twins.join(format!("{}-{work:016x}.lock", at.commit));
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
        Ok(()) => {
            take_away_left(&twins, &file);
            return Alone::Held(Held { file, _lock: lock });
        }
        Err(fs::TryLockError::WouldBlock) => {}
        Err(fs::TryLockError::Error(_)) => return Alone::Unheld,
    }
    let going = || {
        twin(state, me, at).map_or_else(
            || "another run".to_owned(),
            |going| {
                format!(
                    "run {}, going {} s so far,",
                    going.pid,
                    now().saturating_sub(going.started)
                )
            },
        )
    };
    let commit = at.commit.get(..12).unwrap_or(at.commit);
    if !waits {
        say(format!(
            "nix-offload: {} already runs these changed-checks in this tree at {commit}, and \
             --never-ask waits for nothing, so this ends; stop that one to go on",
            going()
        ));
        return Alone::Busy;
    }
    say(format!(
        "nix-offload: {} already runs these changed-checks in this tree at {commit}, so this \
         waits for it to end and then reads what it passed",
        going()
    ));
    // Looked at a few times a second rather than blocked on, so a wait
    // behind a first that hangs keeps saying so; a first that dies frees
    // its lock with it.
    let pause = every.min(Duration::from_millis(250));
    let mut since = Duration::ZERO;
    loop {
        match lock.try_lock() {
            Ok(()) => return Alone::Waited,
            Err(fs::TryLockError::WouldBlock) => {}
            Err(fs::TryLockError::Error(_)) => return Alone::Unheld,
        }
        std::thread::sleep(pause);
        since += pause;
        if since >= every {
            since = Duration::ZERO;
            say(format!(
                "nix-offload: this still waits on {} at {commit}; stop that one to go on",
                going()
            ));
        }
    }
}

/// How long a lock no run holds is kept: a day, far past any run's, so
/// one taken away is one a run killed with SIGKILL left behind.
const LEFT_FOR: Duration = Duration::from_hours(24);

/// Take away each lock in `twins` but `mine` that's older than
/// [`LEFT_FOR`] and that no run holds, since a killed run never removes
/// its own and they gather one per commit.
fn take_away_left(twins: &Path, mine: &Path) {
    let Ok(entries) = fs::read_dir(twins) else {
        return;
    };
    for path in entries.filter_map(Result::ok).map(|entry| entry.path()) {
        let old = fs::metadata(&path)
            .and_then(|meta| meta.modified())
            .is_ok_and(|at| at.elapsed().is_ok_and(|age| age > LEFT_FOR));
        if path == mine || !old {
            continue;
        }
        let unheld = fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .is_ok_and(|left| left.try_lock().is_ok());
        if unheld {
            let _ = fs::remove_file(&path);
        }
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
    fn a_twins_lock_is_named_the_same_by_every_build() {
        let state = tempfile::tempdir().unwrap();
        let _held = alone(state.path(), 1, &at("abc", "/t", ASKED), || 10, true);
        let named = state
            .path()
            .join("nix-offload/twins/abc-806d23c24afc7eb5.lock");
        assert!(
            named.exists(),
            "the lock wasn't named by FNV-1a of its tree and ask, which every build agrees on"
        );
    }

    // A process's start is read from /proc, which only Linux has, and so
    // are these three.
    #[cfg(target_os = "linux")]
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
            (
                at("abc", "/t", "--build --system x86_64-linux"),
                "another ask",
            ),
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
        let held = alone(state.path(), 1, &first, || 10, true);
        assert!(
            matches!(held, Alone::Held(_)),
            "the first run didn't hold its work"
        );
        assert!(
            matches!(
                alone(state.path(), 2, &at("abc", "/t", "--build"), || 10, true),
                Alone::Held(_)
            ),
            "a run asked something else waited on the first"
        );
        let begun = Instant::now();
        let ender = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(1));
            drop(held);
        });
        let waited = alone(state.path(), 2, &first, || 10, true);
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
            matches!(alone(state.path(), 2, &first, || 10, true), Alone::Held(_)),
            "a run waited where nothing else went"
        );
    }

    #[test]
    fn a_lock_a_killed_run_left_a_day_ago_is_taken_away() {
        use std::time::SystemTime;
        let state = tempfile::tempdir().unwrap();
        let twins = state.path().join("nix-offload/twins");
        fs::create_dir_all(&twins).unwrap();
        let two_days_ago = SystemTime::now() - Duration::from_hours(48);
        let left = |name: &str, when: SystemTime| {
            let file = fs::File::create(twins.join(name)).unwrap();
            file.set_modified(when).unwrap();
            file
        };
        drop(left("old-0000000000000000.lock", two_days_ago));
        drop(left("new-0000000000000000.lock", SystemTime::now()));
        let going = left("going-0000000000000000.lock", two_days_ago);
        going.lock().unwrap();
        let _held = alone(state.path(), 1, &at("abc", "/t", ASKED), || 10, true);
        assert!(
            !twins.join("old-0000000000000000.lock").exists(),
            "a lock no run has held for a day was left to gather"
        );
        assert!(
            twins.join("new-0000000000000000.lock").exists(),
            "a lock left within the day was taken away"
        );
        assert!(
            twins.join("going-0000000000000000.lock").exists(),
            "a lock a run still holds was taken away"
        );
    }

    #[test]
    fn a_run_waiting_on_its_twin_says_it_still_waits() {
        let state = tempfile::tempdir().unwrap();
        let first = at("abc", "/t", ASKED);
        let held = alone(state.path(), 1, &first, || 10, true);
        // The first lets go once the second has said twice that it still
        // waits, rather than after a fixed sleep: on 26 November a loaded
        // builder's sleeps ran long, the lock fell before the second line,
        // and the test failed an update with nothing wrong. Half a minute
        // bounds a wait that never speaks, so it fails rather than hangs.
        let (tell, heard) = std::sync::mpsc::channel::<String>();
        let ender = std::thread::spawn(move || {
            let mut said = Vec::new();
            while said
                .iter()
                .filter(|line: &&String| line.contains("still waits"))
                .count()
                < 2
            {
                match heard.recv_timeout(Duration::from_secs(30)) {
                    Ok(line) => said.push(line),
                    Err(_) => break,
                }
            }
            drop(held);
            said.extend(heard.iter());
            said
        });
        let mut say = move |line: String| {
            let _ = tell.send(line);
        };
        let waited = alone_saying(
            state.path(),
            2,
            &first,
            || 10,
            true,
            Duration::from_millis(300),
            &mut say,
        );
        drop(say);
        let said = ender.join().unwrap();
        assert!(
            matches!(waited, Alone::Waited),
            "a second run didn't wait for the first"
        );
        assert!(
            said.iter()
                .filter(|line| line.contains("still waits"))
                .count()
                >= 2,
            "a run waiting on a twin went quiet while it waited: {said:?}"
        );
    }

    #[test]
    fn a_run_that_never_asks_leaves_its_twin_at_once() {
        use std::time::Instant;
        let state = tempfile::tempdir().unwrap();
        let first = at("abc", "/t", ASKED);
        let held = alone(state.path(), 1, &first, || 10, true);
        let ender = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(2));
            drop(held);
        });
        let begun = Instant::now();
        let mut said = Vec::new();
        let left = alone_saying(
            state.path(),
            2,
            &first,
            || 10,
            false,
            Duration::from_millis(300),
            &mut |line| said.push(line),
        );
        let took = begun.elapsed();
        ender.join().unwrap();
        assert!(
            matches!(left, Alone::Busy) && took < Duration::from_secs(1),
            "a run told to wait for nothing waited {took:?} on its twin"
        );
        assert!(
            said.iter()
                .any(|line| line.contains("--never-ask") && line.contains("stop that one")),
            "a run leaving its twin be didn't say why: {said:?}"
        );
    }

    #[cfg(target_os = "linux")]
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

    #[cfg(target_os = "linux")]
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
