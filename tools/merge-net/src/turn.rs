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
//! change adds a line to (`lists.rs`), which never make a pass stale. A
//! gate counts as having started when its tree's first gate did
//! (`places.rs`), so a change that runs again after a merge left its pass
//! stale keeps its place rather than going behind every gate since.
//!
//! A change that passed waits so for at most [`PATIENCE`], then goes first.
//! Waiting saves the other gate's run only by costing this change one of its
//! own once the other merges, so it's a turn, not a right: on 11 November a
//! change that passed at 22:40 and again at 23:14 was refused six times in
//! an hour, each for another tree's gate placed before it on words.json,
//! since trees that run again keep their places for good.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::places::Places;

/// The longest a gate is waited on. Gates have run past an hour on
/// saturated lenders, so this leaves room, while a gate that hangs still
/// lets merges go within a few hours.
pub const LONGEST: u64 = 3 * 60 * 60;

/// The longest a change that passed waits on earlier gates, from when its
/// pass was kept: about the time one gate takes, so an earlier gate nearly
/// done still ends first, and one far from done doesn't hold a pass back.
pub const PATIENCE: u64 = 15 * 60;

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
    /// When it counts as having started, its tree's place.
    pub started: u64,
}

/// The change asking to merge: when its gate counts as having started,
/// the tree it's in, and the files it changes.
pub struct Asking<'a> {
    pub started: u64,
    /// When its pass was kept, which [`PATIENCE`] counts from.
    pub passed: u64,
    pub here: &'a Path,
    pub ours: &'a [String],
}

/// The gate among `gates` that holds back the change `asking`: the
/// earliest that counts, as `place` says, as started before it, still runs
/// as `alive` says of it, hasn't run past [`LONGEST`] by `now` since it
/// really started, and changes, as `files_of` says, a file of its that's no
/// list. A change that passed [`PATIENCE`] ago waits on none.
pub fn ahead<'a>(
    gates: &'a [Gate],
    asking: &Asking,
    now: u64,
    place: &dyn Fn(&Gate) -> u64,
    alive: &dyn Fn(&Gate) -> bool,
    files_of: &dyn Fn(&Gate) -> Option<Vec<String>>,
) -> Option<Ahead<'a>> {
    if now.saturating_sub(asking.passed) >= PATIENCE {
        return None;
    }
    let ours: BTreeSet<&str> = asking
        .ours
        .iter()
        .map(String::as_str)
        .filter(|file| crate::lists::list(file).is_none())
        .collect();
    let mut earlier: Vec<(u64, &Gate)> = gates
        .iter()
        .filter(|gate| now.saturating_sub(gate.started) < LONGEST && gate.folder != asking.here)
        .map(|gate| (place(gate), gate))
        .filter(|(started, _)| *started < asking.started)
        .collect();
    earlier.sort_by_key(|(started, _)| *started);
    earlier.into_iter().find_map(|(started, gate)| {
        if !alive(gate) {
            return None;
        }
        let shared: Vec<String> = files_of(gate)?
            .into_iter()
            .filter(|file| ours.contains(file.as_str()))
            .collect();
        (!shared.is_empty()).then_some(Ahead {
            gate,
            shared,
            started,
        })
    })
}

/// When the gate a change merges on started: the first of `passes`, the
/// commits whose runs `ready` passes it on, newest first, whose record
/// (`record`) kept a start. A head rebased onto main after its gate has no
/// run of its own, and passes on an earlier commit of the same change, so
/// that commit's start is the change's: on 10 November the lease read only
/// the head's, found none on nearly every merge, and held nothing, so
/// #2981 lost seven passes to changes sharing its files.
pub fn start_of(passes: &[&str], record: &dyn Fn(&str) -> String) -> Option<u64> {
    passes.iter().find_map(|commit| started(&record(commit)))
}

/// Why a change changing `ours` onto origin's main waits for another gate
/// before it merges, or nothing when it needn't, given `passes`, the
/// commits whose runs it merges on, newest first. A change whose runs
/// kept no start, from before starts were, waits on nothing.
pub fn waits(passes: &[&str], ours: &[String]) -> Option<String> {
    let its_gate = start_of(passes, &crate::ready::record)?;
    let gates = gates(&folder()?);
    let here = std::env::current_dir().ok()?;
    let now = crate::state::now();
    let places = placed(&gates, &here, Some(its_gate), now);
    let kept = kept_at(passes, &crate::ready::record).unwrap_or(now);
    let asking = Asking {
        started: places.start(&here, its_gate, now),
        passed: kept,
        here: &here,
        ours,
    };
    let place = |gate: &Gate| places.start(&gate.folder, gate.started, now);
    let held = ahead(&gates, &asking, now, &place, &alive, &files_of)?;
    Some(format!(
        "merge-net: the gate in {} at {}, which counts as started {} minutes before this \
         one's, as its tree's first gate did, still runs and changes {} too; merging first \
         would leave its pass stale, so merge again once it ends, which needs no new run \
         here unless it merges first and moves these; this one waits at most {} more \
         minutes, then goes first",
        held.gate.folder.display(),
        crate::state::short(&held.gate.commit),
        (asking.started - held.started) / 60,
        held.shared.join(", "),
        (PATIENCE.saturating_sub(now.saturating_sub(kept))).div_ceil(60)
    ))
}

/// When the pass a change merges on was kept, in seconds since the epoch:
/// when the record of the first of `passes` that kept a start was last
/// written, which nix-offload does as its gate passes.
fn kept_at(passes: &[&str], record: &dyn Fn(&str) -> String) -> Option<u64> {
    let commit = passes
        .iter()
        .find(|commit| started(&record(commit)).is_some())?;
    let written = fs::metadata(crate::ready::record_of(commit)?)
        .ok()?
        .modified()
        .ok()?;
    Some(
        written
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs(),
    )
}

/// Keep the place of the tree asking, whose pass at the first of `passes`
/// that kept a start a move of main has left stale, so its run again goes
/// where that pass's gate did. Main may have moved by a merge made by hand,
/// which no `ready` saw the gate through.
pub fn keep_place(passes: &[&str]) {
    let (Some(folder), Ok(here)) = (folder(), std::env::current_dir()) else {
        return;
    };
    let its_gate = start_of(passes, &crate::ready::record);
    placed(&gates(&folder), &here, its_gate, crate::state::now());
}

/// The places kept, with the start of each gate still going, and the
/// asking tree's at `here` of its pass at `passed`, kept too, and those
/// past keeping given up. Failing to write them only loses a place, which
/// the next `ready` to see the gate keeps again.
fn placed(gates: &[Gate], here: &Path, passed: Option<u64>, now: u64) -> Places {
    let file = crate::places::file();
    let mut places = Places::read(&file);
    for gate in gates.iter().filter(|gate| alive(gate)) {
        places.keep(&gate.folder, gate.started);
    }
    if let Some(passed) = passed {
        places.keep(here, passed);
    }
    places.tidy(now, &|folder| folder.exists());
    if let Err(why) = places.write(&file) {
        eprintln!("merge-net: {why}");
    }
    places
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
    Some(crate::ready::offload_state()?.join("running"))
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

    /// Each gate counted as started when it did, as with no places kept.
    fn own(gate: &Gate) -> u64 {
        gate.started
    }

    fn asking<'a>(started: u64, here: &'a str, ours: &'a [String]) -> Asking<'a> {
        Asking {
            started,
            passed: NOW,
            here: Path::new(here),
            ours,
        }
    }

    #[test]
    fn a_pass_waits_on_an_earlier_gate_only_so_long() {
        let gates = [gate(1, "/a", 100)];
        let ours = owned(&["y.rs"]);
        let touch = touching(&["y.rs"]);
        let mut waiting = asking(200, "/b", &ours);
        waiting.passed = NOW - PATIENCE + 1;
        assert!(
            ahead(&gates, &waiting, NOW, &own, &everyone, &touch).is_some(),
            "a pass short of its patience went ahead of an earlier gate"
        );
        waiting.passed = NOW - PATIENCE;
        assert_eq!(
            ahead(&gates, &waiting, NOW, &own, &everyone, &touch),
            None,
            "a pass that waited its patience was still held back"
        );
    }

    #[test]
    fn an_earlier_gate_sharing_a_file_holds_a_later_change_back() {
        let gates = [gate(1, "/a", 100)];
        let ours = owned(&["y.rs", "z.rs"]);
        let touch = touching(&["x.rs", "y.rs"]);
        let held = ahead(
            &gates,
            &asking(200, "/b", &ours),
            NOW,
            &own,
            &everyone,
            &touch,
        );
        assert_eq!(
            held,
            Some(Ahead {
                gate: &gates[0],
                shared: owned(&["y.rs"]),
                started: 100,
            }),
            "an earlier gate sharing y.rs didn't hold the change back"
        );
    }

    #[test]
    fn a_later_gate_holds_an_earlier_change_back_never() {
        let gates = [gate(1, "/a", 300)];
        let ours = owned(&["y.rs"]);
        let touch = touching(&["y.rs"]);
        let held = ahead(
            &gates,
            &asking(200, "/b", &ours),
            NOW,
            &own,
            &everyone,
            &touch,
        );
        assert_eq!(held, None, "a gate started later held the change back");
    }

    #[test]
    fn a_rerun_keeps_its_trees_place_ahead_of_gates_started_since_its_first() {
        // The tree at /a passed a gate at 100, a merge left it stale, and
        // it runs again from 300; /b's gate started at 200, between them.
        let mut places = Places::default();
        places.keep(Path::new("/a"), 100);
        let place = |gate: &Gate| places.start(&gate.folder, gate.started, NOW);
        let rerun = [gate(1, "/a", 300)];
        let ours = owned(&["y.rs"]);
        let touch = touching(&["y.rs"]);
        let held = ahead(
            &rerun,
            &asking(200, "/b", &ours),
            NOW,
            &place,
            &everyone,
            &touch,
        );
        assert_eq!(
            held.map(|held| held.started),
            Some(100),
            "a rerun went behind a gate started after its tree's first"
        );
        let first = [gate(2, "/c", 50)];
        let held = ahead(
            &first,
            &asking(100, "/a", &ours),
            NOW,
            &place,
            &everyone,
            &touch,
        );
        assert_eq!(
            held.map(|held| held.gate.pid),
            Some(2),
            "a rerun in its tree's place went ahead of a gate started before it"
        );
    }

    #[test]
    fn a_gate_gone_here_too_long_or_sharing_only_lists_holds_nothing_back() {
        let ours = owned(&["y.rs", "check-descriptions.nix"]);
        let gone = [gate(1, "/a", 100)];
        let nobody = |_: &Gate| false;
        let touch = touching(&["y.rs"]);
        let at = asking(200, "/b", &ours);
        assert_eq!(
            ahead(&gone, &at, NOW, &own, &nobody, &touch),
            None,
            "a gate whose process is gone held the change back"
        );
        let here = [gate(1, "/b", 100)];
        assert_eq!(
            ahead(&here, &at, NOW, &own, &everyone, &touch),
            None,
            "a gate in the asking tree held its own change back"
        );
        let old = [gate(1, "/a", 100)];
        let later = 100 + LONGEST;
        let placed_early = |_: &Gate| 10;
        assert_eq!(
            ahead(&old, &at, later, &placed_early, &everyone, &touch),
            None,
            "a gate past the longest wait held the change back"
        );
        let lists = touching(&["check-descriptions.nix"]);
        assert_eq!(
            ahead(&old, &at, NOW, &own, &everyone, &lists),
            None,
            "a gate sharing only a list held the change back"
        );
    }

    #[test]
    fn a_rebased_head_takes_the_start_of_the_pass_it_merges_on() {
        let kept = |started: u64| format!(r#"{{"run":["x"],"started":{started}}}"#);
        let record = |commit: &str| match commit {
            "newer" => kept(300),
            "older" => kept(100),
            _ => String::new(),
        };
        assert_eq!(
            start_of(&["head", "older"], &record),
            Some(100),
            "a rebased head with no run of its own took no start"
        );
        assert_eq!(start_of(&["newer", "older"], &record), Some(300));
        assert_eq!(start_of(&["head"], &record), None);
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
