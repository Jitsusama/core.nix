//! Merges in the order their gates started. Two changes sharing a file
//! each pass `nix-offload there --changed` against main as it stood, and
//! whichever merges first leaves the other's pass stale, so it runs again
//! whole. When the later gate's change merges first, the earlier one,
//! which may be most of the way through, is the one that pays. So `ready`
//! holds a change back while a gate that started before its own, and
//! shares one of its files, still runs.
//!
//! nix-offload says each gate going in `nix-offload/running` under XDG's
//! state folder, and keeps when each passed gate started in its record of
//! what passed (`passed.rs` and `running.rs` there). Ordering by start
//! rather than by whether a gate still runs means the earlier change never
//! waits on the later one, so neither waits on the other forever. A gate
//! whose process is gone, or is another process now, started at another
//! time than the gate's, or that ran past [`LONGEST`], holds nothing
//! back; nor does one in the asking tree, or one sharing only lists each
//! change adds a line to (`lists.rs`), which never make a pass stale.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The longest a gate is waited on. Gates have run past an hour on
/// saturated lenders, so this leaves room, while a gate that hangs still
/// lets merges go within a few hours.
pub const LONGEST: u64 = 3 * 60 * 60;

/// A gate going now, as nix-offload said it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gate {
    pub pid: u32,
    /// When its process started, in clock ticks since boot.
    pub ticks: u64,
    pub commit: String,
    pub folder: PathBuf,
    pub started: u64,
}

/// The gates nix-offload says are going, in `folder`.
pub fn gates(folder: &Path) -> Vec<Gate> {
    fs::read_dir(folder)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|e| e == "json"))
        .filter_map(|entry| fs::read_to_string(entry.path()).ok())
        .filter_map(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .filter_map(|said| {
            Some(Gate {
                pid: u32::try_from(said["pid"].as_u64()?).ok()?,
                ticks: said["ticks"].as_u64()?,
                commit: said["commit"].as_str()?.to_owned(),
                folder: PathBuf::from(said["folder"].as_str()?),
                started: said["started"].as_u64()?,
            })
        })
        .collect()
}

/// When the earliest gate kept in a commit's record of what passed
/// started, of those that built what they named at the top of the
/// repository; none for a record kept before starts were.
pub fn started(record: &str) -> Option<u64> {
    record
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|line| line["dir"].is_null() && line["run"].is_array())
        .filter_map(|line| line["started"].as_u64())
        .min()
}

/// What a gate holding a change back shares with it.
#[derive(Debug, PartialEq, Eq)]
pub struct Ahead<'a> {
    pub gate: &'a Gate,
    pub shared: Vec<String>,
}

/// The gate among `gates` that holds back a change whose own gate started
/// at `started`, in the tree at `here`, changing `ours`: the earliest that
/// started before it, still runs as `alive` says of it, hasn't
/// run past [`LONGEST`] by `now`, and changes, as `files_of` says, a file
/// of `ours` that's no list.
pub fn ahead<'a>(
    gates: &'a [Gate],
    started: u64,
    here: &Path,
    now: u64,
    alive: &dyn Fn(&Gate) -> bool,
    files_of: &dyn Fn(&Gate) -> Option<Vec<String>>,
    ours: &[String],
) -> Option<Ahead<'a>> {
    let ours: BTreeSet<&str> = ours
        .iter()
        .map(String::as_str)
        .filter(|file| crate::lists::list(file).is_none())
        .collect();
    let mut earlier: Vec<&Gate> = gates
        .iter()
        .filter(|gate| gate.started < started && now.saturating_sub(gate.started) < LONGEST)
        .filter(|gate| gate.folder != here)
        .collect();
    earlier.sort_by_key(|gate| gate.started);
    earlier.into_iter().find_map(|gate| {
        if !alive(gate) {
            return None;
        }
        let shared: Vec<String> = files_of(gate)?
            .into_iter()
            .filter(|file| ours.contains(file.as_str()))
            .collect();
        (!shared.is_empty()).then_some(Ahead { gate, shared })
    })
}

/// Why `commit`, changing `ours` onto origin's main, waits for another
/// gate before it merges, or nothing when it needn't. Its start is the
/// one kept with its own pass; a head with none, passed only at an
/// earlier commit of the same change, waits on nothing.
pub fn waits(commit: &str, ours: &[String]) -> Option<String> {
    let started = started(&crate::ready::record(commit))?;
    let gates = gates(&folder()?);
    let here = std::env::current_dir().ok()?;
    let now = crate::state::now();
    let held = ahead(&gates, started, &here, now, &alive, &files_of, ours)?;
    Some(format!(
        "merge-net: the gate in {} at {}, which started {} minutes before this one's, \
         still runs and changes {} too; merging first would leave its pass stale, so \
         merge again once it ends, which needs no new run here unless it merges first \
         and moves these",
        held.gate.folder.display(),
        crate::state::short(&held.gate.commit),
        (started - held.gate.started) / 60,
        held.shared.join(", ")
    ))
}

/// The files a gate's commit changes from where it left origin's main, as
/// its own tree's repository says.
fn files_of(gate: &Gate) -> Option<Vec<String>> {
    let git = |arguments: &[&str]| {
        let ran = Command::new("git")
            .arg("-C")
            .arg(&gate.folder)
            .args(arguments)
            .output()
            .ok()?;
        ran.status
            .success()
            .then(|| String::from_utf8_lossy(&ran.stdout).into_owned())
    };
    let base = git(&["merge-base", &gate.commit, "refs/remotes/origin/main"])?;
    let files = git(&["diff", "--name-only", base.trim(), &gate.commit])?;
    Some(files.lines().map(str::to_owned).collect())
}

/// Whether `gate`'s process still runs: one has its id and started when
/// the gate said, so a process given the id since doesn't count.
pub fn alive(gate: &Gate) -> bool {
    ticks(gate.pid) == Some(gate.ticks)
}

/// When process `pid` started, in clock ticks since boot: the 22nd field
/// of its `stat`, counted after the name, which may hold spaces.
fn ticks(pid: u32) -> Option<u64> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let (_, after) = stat.rsplit_once(')')?;
    after.split_whitespace().nth(19)?.parse().ok()
}

/// The folder nix-offload says its gates going in.
pub fn folder() -> Option<PathBuf> {
    Some(crate::ready::record_of("")?.parent()?.join("running"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 10_000;

    fn gate(pid: u32, folder: &str, started: u64) -> Gate {
        Gate {
            pid,
            ticks: 1,
            commit: format!("{pid:040}"),
            folder: PathBuf::from(folder),
            started,
        }
    }

    fn owned(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    fn everyone(_: &Gate) -> bool {
        true
    }

    fn touching(files: &[&str]) -> impl Fn(&Gate) -> Option<Vec<String>> {
        let files = owned(files);
        move |_| Some(files.clone())
    }

    #[test]
    fn an_earlier_gate_sharing_a_file_holds_a_later_change_back() {
        let gates = [gate(1, "/a", 100)];
        let held = ahead(
            &gates,
            200,
            Path::new("/b"),
            NOW,
            &everyone,
            &touching(&["x.rs", "y.rs"]),
            &owned(&["y.rs", "z.rs"]),
        );
        assert_eq!(
            held,
            Some(Ahead {
                gate: &gates[0],
                shared: owned(&["y.rs"])
            }),
            "an earlier gate sharing y.rs didn't hold the change back"
        );
    }

    #[test]
    fn a_later_gate_holds_an_earlier_change_back_never() {
        let gates = [gate(1, "/a", 300)];
        let held = ahead(
            &gates,
            200,
            Path::new("/b"),
            NOW,
            &everyone,
            &touching(&["y.rs"]),
            &owned(&["y.rs"]),
        );
        assert_eq!(held, None, "a gate started later held the change back");
    }

    #[test]
    fn a_gate_gone_here_too_long_or_sharing_only_lists_holds_nothing_back() {
        let ours = owned(&["y.rs", "check-descriptions.nix"]);
        let gone = [gate(1, "/a", 100)];
        let nobody = |_: &Gate| false;
        let touch = touching(&["y.rs"]);
        let at = Path::new("/b");
        assert_eq!(
            ahead(&gone, 200, at, NOW, &nobody, &touch, &ours),
            None,
            "a gate whose process is gone held the change back"
        );
        let here = [gate(1, "/b", 100)];
        assert_eq!(
            ahead(&here, 200, at, NOW, &everyone, &touch, &ours),
            None,
            "a gate in the asking tree held its own change back"
        );
        let old = [gate(1, "/a", 100)];
        let later = 100 + LONGEST;
        assert_eq!(
            ahead(&old, 200, at, later, &everyone, &touch, &ours),
            None,
            "a gate past the longest wait held the change back"
        );
        let lists = touching(&["check-descriptions.nix"]);
        assert_eq!(
            ahead(&old, 200, at, NOW, &everyone, &lists, &ours),
            None,
            "a gate sharing only a list held the change back"
        );
    }

    #[test]
    fn a_gates_start_is_its_earliest_kept_and_none_before_starts_were() {
        let record = "{\"dir\":null,\"run\":[\"--build\"],\"printed\":\"\",\"started\":300}\n\
                      {\"dir\":null,\"check\":\"checks.x.a\"}\n\
                      {\"dir\":null,\"run\":[\"--build\",\"--no-vm\"],\"started\":200}\n\
                      {\"dir\":\"sub\",\"run\":[\"--build\"],\"started\":100}\n";
        assert_eq!(started(record), Some(200), "the wrong start was read");
        let old = "{\"dir\":null,\"run\":[\"--build\"],\"printed\":\"\"}\n";
        assert_eq!(started(old), None, "a record with no start gave one");
    }

    #[test]
    fn a_gate_is_alive_only_while_a_process_started_when_it_said_has_its_id() {
        let pid = std::process::id();
        let ticks = ticks(pid).expect("this process's start is read");
        let mut running = gate(pid, "/a", 100);
        running.ticks = ticks;
        assert!(
            alive(&running),
            "a gate whose process runs was taken as ended"
        );
        running.ticks = ticks + 1;
        assert!(
            !alive(&running),
            "a gate was taken as running by another process given its id"
        );
        assert!(
            !alive(&gate(u32::MAX, "/a", 100)),
            "a gate with no process ran"
        );
    }

    #[test]
    fn the_gates_going_are_read_beside_what_passed() {
        assert!(
            folder().is_some_and(|folder| folder.ends_with("nix-offload/running")),
            "the gates going were looked for outside nix-offload's state"
        );
    }

    #[test]
    fn gates_are_read_as_nix_offload_says_them() {
        let folder = std::env::temp_dir().join(format!("merge-net-turn-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        let said = r#"{"pid":7,"ticks":9,"commit":"abc","folder":"/t","started":5}"#;
        fs::write(folder.join("7.json"), said).unwrap();
        fs::write(folder.join("8.json.new"), said).unwrap();
        fs::write(folder.join("9.json"), "not yet whole").unwrap();
        let read = gates(&folder);
        let _ = fs::remove_dir_all(&folder);
        assert_eq!(
            read,
            [Gate {
                pid: 7,
                ticks: 9,
                commit: "abc".to_owned(),
                folder: PathBuf::from("/t"),
                started: 5
            }],
            "the gates going were misread"
        );
    }
}
