//! Merges in the order their gates started. Two changes sharing a file
//! each pass `nix-offload there --changed` against main as it stood, and
//! whichever merges first leaves the other's pass stale, so it runs again
//! whole. When the later gate's change merges first, the earlier one,
//! which may be most of the way through, is the one that pays. So `ready`
//! holds a change back while a gate that started before its own, and
//! shares one of its files, still runs, and for [`GRACE`] after it passes,
//! while its change merges (`finished`).
//!
//! nix-offload says each gate going in `nix-offload/running` under XDG's
//! state folder, and keeps when each passed gate started in its record of
//! what passed (`passed.rs` and `running.rs` there). Ordering by start
//! rather than by whether a gate still runs means the earlier change never
//! waits on the later one, so neither waits on the other forever. A gate
//! whose process is gone, or is another process now, started at another
//! time than the gate's, or that ran past [`LONGEST`], holds nothing
//! back; nor does one in the asking tree, or one sharing only lists each
//! change adds a line to (`lists.rs`), which never make a pass stale, or
//! lists of modules its change and the asking one gave lines apart, or a
//! pytest file they gave cases apart (`apart.rs`), which `ready` will
//! judge apart once it's rebased. A change in a crate the gate's crates
//! are in, read or are read by shares with it too, as `ready` counts
//! (modules/crate-reach, `shares`). A
//! gate counts as having started when its tree's first gate did
//! (`places.rs`), so a change that runs again after a merge left its pass
//! stale keeps its place rather than going behind every gate since.
//!
//! A change that passed waits so for at most [`PATIENCE`] from when it was
//! first held, across rebases (`waited.rs`), then goes first.
//! Waiting saves the other gate's run only by costing this change one of its
//! own once the other merges, so it's a turn, not a right: on 11 November a
//! change that passed at 22:40 and again at 23:14 was refused six times in
//! an hour, each for another tree's gate placed before it on words.json,
//! since trees that run again keep their places for good.
//!
//! Between a change to the terminal kit and a reader of it, the kit's
//! queue decides rather than when their gates started (`kit.rs`).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use crate::kit;
use crate::places::Places;
use crate_reach::Crates;
use crate::waited::Waited;

/// The longest a gate is waited on. Gates have run past an hour on
/// saturated lenders, so this leaves room, while a gate that hangs still
/// lets merges go within a few hours.
pub const LONGEST: u64 = 3 * 60 * 60;

/// The longest a change that passed waits on earlier gates, from when it
/// was first held back (`waited.rs`): about the time one gate takes, so an earlier gate nearly
/// done still ends first, and one far from done doesn't hold a pass back.
pub const PATIENCE: u64 = 15 * 60;

/// How long a change whose gate passed keeps its turn, for its author to
/// merge it. Holding back only while a gate ran let a change waiting on it
/// go the moment it ended: on 14 November a change sharing Launcher.qml
/// passed at 22:50:57, one placed after it merged 32 seconds later, and
/// the first's pass went stale for the second time that evening. Ten
/// minutes leaves room for its author's merge, while a pass nobody merges
/// holds others back only so long.
pub const GRACE: u64 = 10 * 60;

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

/// The gates of `trees`, each a folder with its head, that ended with a
/// pass at that head under [`GRACE`] before `now` and that main doesn't
/// hold yet, as gates with no process (a `pid` of 0). `kept` says when a
/// head's passing gate started and when its pass was kept; `merged`
/// whether main holds a head, or nothing where that can't be told, which
/// holds nothing back.
pub fn finished(
    trees: &[(PathBuf, String)],
    now: u64,
    kept: &dyn Fn(&str) -> Option<(u64, u64)>,
    merged: &dyn Fn(&str) -> Option<bool>,
) -> Vec<Gate> {
    trees
        .iter()
        .filter_map(|(folder, head)| {
            let (started, kept) = kept(head)?;
            let fresh = now.saturating_sub(kept) < GRACE;
            (fresh && merged(head) == Some(false)).then(|| Gate {
                pid: 0,
                ticks: 0,
                commit: head.clone(),
                folder: folder.clone(),
                started,
            })
        })
        .collect()
}

/// Whether origin's main holds `commit`, or each of its change's commits
/// by patch, as a change rebased before it merged has them: on 15 November
/// a tree moved on to new work from its change's head before the rebase
/// that merged, and that head held others back as a pass still to merge.
/// As git says in the asking tree, which shares its store with the others;
/// nothing where git can't tell, as of a commit it hasn't got.
fn on_main(commit: &str) -> Option<bool> {
    on_main_in(Path::new("."), commit)
}

fn on_main_in(repo: &Path, commit: &str) -> Option<bool> {
    if holds(repo, commit)? {
        return Some(true);
    }
    let cherry = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["cherry", "refs/remotes/origin/main", commit])
        .output()
        .ok()?;
    let said = String::from_utf8_lossy(&cherry.stdout);
    let mut lines = said.lines().peekable();
    let any = lines.peek().is_some();
    Some(cherry.status.success() && any && lines.all(|line| line.starts_with('-')))
}

/// Whether origin's main holds `commit` itself.
fn holds(repo: &Path, commit: &str) -> Option<bool> {
    let ran = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "merge-base",
            "--is-ancestor",
            commit,
            "refs/remotes/origin/main",
        ])
        .output()
        .ok()?;
    match ran.status.code() {
        Some(0) => Some(true),
        Some(1) => Some(false),
        _ => None,
    }
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
    /// When it was first held back, which [`PATIENCE`] counts from
    /// (`waited.rs`).
    pub waits_from: u64,
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
    if now.saturating_sub(asking.waits_from) >= PATIENCE {
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

/// Why the change at `commit` changing `ours` onto origin's main waits for
/// another gate before it merges, and the files they share, or nothing
/// when it needn't, given `passes`, the commits whose runs it merges on,
/// newest first. A change whose runs kept no start, from before starts
/// were, waits on nothing.
pub fn waits(commit: &str, passes: &[&str], ours: &[String]) -> Option<(String, Vec<String>)> {
    let its_gate = start_of(passes, &crate::ready::record)?;
    let gates = gates(&folder()?);
    let here = std::env::current_dir().ok()?;
    let now = crate::state::now();
    let places = placed(&gates, &here, Some(its_gate), now);
    let patch = crate::waited::patch_of(commit);
    let file = crate::waited::file();
    let mut waited = Waited::read(&file);
    let kept = kept_at(passes, &crate::ready::record);
    let waits_from = waited.since(patch.as_deref(), &here, kept, now);
    let asking = Asking {
        started: places.start(&here, its_gate, now),
        waits_from,
        here: &here,
        ours,
    };
    let place = |gate: &Gate| places.start(&gate.folder, gate.started, now);
    let crates = Crates::at(Path::new("."), commit);
    let mut gates = gates;
    gates.extend(finished(&heads(&places), now, &passed_at, &on_main));
    let queue = Queue::of(&here, patch.as_deref(), ours, &gates, &crates, now);
    let waited_for = now.saturating_sub(waits_from);
    if let Some((said, shared)) = queue.holds(patch.as_deref(), ours, &crates, now, waited_for) {
        keep_held(&mut waited, patch.as_deref(), &here, &file, now);
        let left = PATIENCE.saturating_sub(waited_for).div_ceil(60);
        let said = format!("{said}; this one waits at most {left} more minutes, then goes first");
        return Some((said, shared));
    }
    let touched = |gate: &Gate| {
        if !kit::by_start(queue.ours, queue.role_of(gate)) {
            return Some(Vec::new());
        }
        let files = files_of(gate)?;
        let apart = files
            .into_iter()
            .filter(|file| !will_stand_apart(gate, commit, file))
            .collect();
        Some(shares(apart, ours, &crates))
    };
    let going = |gate: &Gate| gate.pid == 0 || alive(gate);
    let held = ahead(&gates, &asking, now, &place, &going, &touched)?;
    keep_held(&mut waited, patch.as_deref(), &here, &file, now);
    let state = if held.gate.pid == 0 {
        "passed and keeps its turn while it merges"
    } else {
        "still runs"
    };
    let theirs = files_of(held.gate).unwrap_or_default();
    let said = format!(
        "merge-net: the gate in {} at {}, which counts as started {} minutes before this \
         one's, as its tree's first gate did, {state} and {}; merging first \
         would leave its pass stale, so merge again once it ends, which needs no new run \
         here unless it merges first and moves these; this one waits at most {} more \
         minutes, then goes first",
        held.gate.folder.display(),
        crate::state::short(&held.gate.commit),
        (asking.started - held.started) / 60,
        crate::sharing::said(&theirs, &held.shared),
        (PATIENCE.saturating_sub(now.saturating_sub(waits_from))).div_ceil(60)
    );
    Some((said, held.shared))
}

/// The gates going, or keeping their turn after a pass, as the kit's queue
/// sees them (`kit.rs`), and the role of the change asking.
struct Queue {
    ours: kit::Role,
    going: Vec<Gate>,
    roles: BTreeMap<String, kit::Role>,
}

impl Queue {
    /// The queue for the change `patch` changing `ours`, in the tree at
    /// `here`, among the other trees' `gates`.
    fn of(
        here: &Path,
        patch: Option<&str>,
        ours: &[String],
        gates: &[Gate],
        crates: &Crates,
        now: u64,
    ) -> Self {
        let owed = kit::Owed::read(&kit::file());
        let merged = |commit: &str| on_main(commit) == Some(true);
        let is_owed =
            |patch: Option<&str>| patch.is_some_and(|patch| owed.owes(patch, now, &merged));
        let going: Vec<Gate> = gates
            .iter()
            .filter(|gate| gate.folder != here && now.saturating_sub(gate.started) < LONGEST)
            .filter(|gate| gate.pid == 0 || alive(gate))
            .cloned()
            .collect();
        let roles = going
            .iter()
            .map(|gate| {
                let files = files_of(gate).unwrap_or_default();
                let theirs = crate::waited::patch_of(&gate.commit);
                (
                    gate.commit.clone(),
                    kit::role(&files, crates, is_owed(theirs.as_deref())),
                )
            })
            .collect();
        Self {
            ours: kit::role(ours, crates, is_owed(patch)),
            going,
            roles,
        }
    }

    /// The role of `gate`, apart where it isn't going.
    fn role_of(&self, gate: &Gate) -> kit::Role {
        self.roles
            .get(&gate.commit)
            .copied()
            .unwrap_or(kit::Role::Apart)
    }

    /// Why the change `patch` changing `ours` waits in the kit's queue,
    /// and the files of ours that read or change the kit, or nothing when
    /// it needn't, having `waited` since its first hold. A reader held so
    /// keeps which change it waits for, so it's owed its turn once that one
    /// merges.
    fn holds(
        &self,
        patch: Option<&str>,
        ours: &[String],
        crates: &Crates,
        now: u64,
        waited: u64,
    ) -> Option<(String, Vec<String>)> {
        let held = kit::behind(self.ours, &self.going, &|gate| self.role_of(gate), waited)?;
        let kit_files = [format!("{}Cargo.toml", kit::KIT)];
        let shared = if self.ours == kit::Role::Kit {
            ours.iter()
                .filter(|file| file.starts_with(kit::KIT))
                .cloned()
                .collect()
        } else {
            crates.beside(&kit_files, ours)
        };
        let said = if self.ours == kit::Role::Kit {
            format!(
                "merge-net: the change in {} at {} reads the terminal kit and waited for a \
                 change to it that has since merged, so it's owed its turn before this \
                 change to the kit merges or runs again; merge again once it has merged",
                held.folder.display(),
                crate::state::short(&held.commit)
            )
        } else {
            if let Some(patch) = patch {
                let file = kit::file();
                let mut owed = kit::Owed::read(&file);
                owed.waits(patch, &held.commit, now, &|kit| on_main(kit) == Some(true));
                owed.tidy(now);
                if let Err(error) = owed.write(&file) {
                    eprintln!("merge-net: this wait for the kit wasn't kept: {error}");
                }
            }
            format!(
                "merge-net: the change to the terminal kit in {} at {} goes first, since \
                 this one reads the kit ({}), and merging first would send that one round \
                 again; merge again once it has merged, after which no change to the kit \
                 holds this one",
                held.folder.display(),
                crate::state::short(&held.commit),
                shared.join(", ")
            )
        };
        Some((said, shared))
    }
}

/// Why the change at `commit`, which must run again, waits before that
/// run starts: a change to the terminal kit waits for a reader owed its
/// turn (`kit.rs`), whose pass its run would leave stale once it merges.
pub fn waits_to_run(commit: &str) -> Option<String> {
    let base = git_in(
        Path::new("."),
        &["merge-base", commit, "refs/remotes/origin/main"],
    )?;
    let listed = git_in(
        Path::new("."),
        &["diff", "--name-only", base.trim(), commit],
    )?;
    let ours: Vec<String> = listed.lines().map(str::to_owned).collect();
    let crates = Crates::at(Path::new("."), commit);
    let now = crate::state::now();
    let mut gates = gates(&folder()?);
    let places = crate::places::Places::read(&crate::places::file());
    gates.extend(finished(&heads(&places), now, &passed_at, &on_main));
    let patch = crate::waited::patch_of(commit);
    let here = std::env::current_dir().ok()?;
    let queue = Queue::of(&here, patch.as_deref(), &ours, &gates, &crates, now);
    if queue.ours != kit::Role::Kit {
        return None;
    }
    let waits_from = Waited::read(&crate::waited::file()).since(patch.as_deref(), &here, None, now);
    queue
        .holds(
            patch.as_deref(),
            &ours,
            &crates,
            now,
            now.saturating_sub(waits_from),
        )
        .map(|(said, _)| said)
}

/// Keep `now` in `waited`, written to `file`, as when the change `patch`
/// in `tree` was first held, so its [`PATIENCE`] counts from its first hold.
fn keep_held(waited: &mut Waited, patch: Option<&str>, tree: &Path, file: &Path, now: u64) {
    waited.held(patch, tree, now);
    waited.tidy(now);
    if let Err(error) = waited.write(file) {
        eprintln!("merge-net: this hold wasn't kept, so its wait starts again: {error}");
    }
}

/// What a gate changing `theirs` shares with a change to `ours`: the files
/// they both change, and those of ours in a crate one of theirs is in,
/// reads or is read by, which `ready` counts as moved under the gate once
/// ours merges, so its pass would go stale for them all the same.
fn shares(theirs: Vec<String>, ours: &[String], crates: &Crates) -> Vec<String> {
    let beside = crates.beside(&theirs, ours);
    let shared: BTreeSet<String> = theirs.into_iter().chain(beside).collect();
    shared.into_iter().collect()
}

/// Each tree a place is kept for, with its head, where git can read it.
fn heads(places: &Places) -> Vec<(PathBuf, String)> {
    places
        .trees()
        .filter_map(|folder| {
            let head = git_in(folder, &["rev-parse", "HEAD"])?;
            Some((folder.to_path_buf(), head.trim().to_owned()))
        })
        .collect()
}

/// When the gate that passed at `commit` started, and when its pass was
/// kept, from nix-offload's record of it.
fn passed_at(commit: &str) -> Option<(u64, u64)> {
    let started = started(&crate::ready::record(commit))?;
    Some((started, kept_at(&[commit], &crate::ready::record)?))
}

/// Whether `gate`'s change and the one at `head` gave `file`, one of the
/// lists of modules, a pytest file or a note in docs/, apart, as `ready`
/// will judge them once the gate's change is rebased onto `head`: then
/// its pass won't go stale for the file, and holding `head` back for it
/// saves nothing. `head` is read in the asking tree, `gate`'s change in
/// its own.
fn will_stand_apart(gate: &Gate, head: &str, file: &str) -> bool {
    let judged = [crate::apart::is_cases, crate::apart::is_prose];
    if !crate::apart::LISTS.contains(&file) && !judged.iter().any(|is| is(file)) {
        return false;
    }
    let Some(base) = git_in(
        &gate.folder,
        &["merge-base", &gate.commit, "refs/remotes/origin/main"],
    ) else {
        return false;
    };
    let at = |folder: &Path, commit: &str| git_in(folder, &["show", &format!("{commit}:{file}")]);
    match (
        at(&gate.folder, base.trim()),
        at(&gate.folder, &gate.commit),
        at(Path::new("."), head),
    ) {
        (Some(base), Some(theirs), Some(ours)) => {
            crate::apart::would_stand_apart(file, &base, &theirs, &ours)
        }
        _ => false,
    }
}

/// Whether the change at `head`, merging on the runs at `passes`, newest
/// first, waits for an earlier gate sharing one of its `files`, which it
/// says and keeps.
pub fn waiting(head: &str, passes: &[&str], files: &[String]) -> bool {
    let Some((why, shared)) = waits(head, passes, files) else {
        return false;
    };
    eprintln!("{why}");
    crate::answers::keep(head, crate::answers::Verdict::Hold, &shared);
    true
}

/// The change at `head`, ready on the runs at `passes`, as `ready` says,
/// once no earlier gate sharing its `files` still runs.
pub fn passed(head: &str, passes: &[&str], files: &[String], ready: &str) -> ExitCode {
    if waiting(head, passes, files) {
        return ExitCode::FAILURE;
    }
    println!("{ready}");
    crate::answers::keep(head, crate::answers::Verdict::Ok, files);
    ExitCode::SUCCESS
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
    let git = |arguments: &[&str]| git_in(&gate.folder, arguments);
    let base = git(&["merge-base", &gate.commit, "refs/remotes/origin/main"])?;
    let files = git(&["diff", "--name-only", base.trim(), &gate.commit])?;
    Some(files.lines().map(str::to_owned).collect())
}

/// What git says in `folder`, or nothing when it fails.
fn git_in(folder: &Path, arguments: &[&str]) -> Option<String> {
    let ran = Command::new("git")
        .arg("-C")
        .arg(folder)
        .args(arguments)
        .output()
        .ok()?;
    ran.status
        .success()
        .then(|| String::from_utf8_lossy(&ran.stdout).into_owned())
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
mod tests;
