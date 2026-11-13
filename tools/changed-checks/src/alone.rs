//! What each of a base's ordinary checks read of the repository, traced
//! alone, kept beside its derivations, so a gate evaluates only the checks
//! its change reaches and gives the rest the base's derivations. On 12
//! November a gate naming about 20 checks still spent 300 to 900 s
//! evaluating every one of the tree's, and a merge among the last 100
//! reached on average 9 of 24 checks sampled.
//!
//! Nix reads a file once in a process, so checks evaluated together say
//! nothing of which read what: each is evaluated in a process of its own,
//! as a plain path under strace with a cache of its own, as `reads.rs`
//! traces the VM tests, and read by the same rules. Traced alone, eight at
//! a time, those 24 took 39 s on the lent builder, against 69 s untraced in
//! one process. A check whose trace can't be read, or whose derivation
//! from the plain path isn't git's, goes unrecorded, so every gate
//! evaluates it, as a gate does each check the record lacks. A keep
//! records this only when none was recorded in the last 20 minutes, so
//! most keep as quickly as before.
//!
//! A gate's base is most often a head of main past the newest recorded one:
//! in four hours on 12 November, 80 of 183 runs evaluated their base, 50 of
//! them within three minutes of it landing. So a gate judges reach from the
//! newest recorded head the base came after: at the base, it evaluates only
//! the checks main's change from that head reaches, and in the tree only
//! those the change from that head to the tree reaches. Each other check
//! keeps the head's derivation, which is the same wherever nothing it read
//! differs.
//!
//! Every check reads each file the flake gathers its checks from, and
//! `gathered.rs` leaves out of its record those it reads only for that.
//!
//! The VM tests are traced alone too, once the others are recorded. A gate
//! whose change reaches what they read together (`reads.rs`) evaluates
//! only those whose own reads it reaches: on 13 November 46 of 49 gates
//! evaluated all 101, which set each tree's evaluating at about 300 s
//! against 110 without, while a merge among the last 300 reached a mean of
//! 37 traced alone, and every merge the net found broke one reached it.
//! Traced alone, the 101 took 299 s, six at a time, a median of 11 s each.
//! A keep traces none while a gate runs beside it on the builder, and one
//! it didn't trace, as one the record lacks, is evaluated by every gate
//! the union doesn't spare.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime};

use serde_json::{Map, Value, json};

use crate::checks::{self, Derivations};
use crate::evaluating;
use crate::gathered;
use crate::reads::{self, Reads};
use crate::runs::{Evaluation, Run};

/// How many checks are traced at once, each in a process of its own, far
/// lighter than a chunk's.
const AT_ONCE: usize = 16;

/// How long a record of what each check read stands in for every keep
/// after it. Tracing each check alone costs a keep about 400 s, which on
/// every keep would put kept heads 12 to 17 minutes apart, where 3 of 12
/// November's last 13 gates already found no recent kept main; recorded
/// once in this long, every other keep stays as quick as before.
const STANDS_FOR: Duration = Duration::from_mins(20);

/// Where a gate holds a lock, shared, for as long as it runs, so a keep
/// beside it begins no VM test's trace.
pub fn gating_lock() -> PathBuf {
    crate::cache().join("changed-checks/gating.lock")
}

/// Say that a gate runs here until what this gives is dropped; nothing
/// when the lock can't be had, and the gate runs regardless.
pub fn gating(lock: &Path) -> Option<fs::File> {
    fs::create_dir_all(lock.parent()?).ok()?;
    let held = fs::File::create(lock).ok()?;
    held.lock_shared().ok()?;
    Some(held)
}

/// Whether a gate runs here, as its lock says: a keep tries it whole and
/// gives it back at once, so a gate starting then waits only that long.
/// A lock tried whole holds against every other open of it, this
/// process's too, so a keep's threads ask one at a time: asking together,
/// each took another's try for a gate and the keep stopped. The try is let
/// go of before the file closes: a program another thread starts holds a
/// copy of the file until it runs, and the lock with it, which closing
/// leaves held and letting go frees.
fn a_gate_runs(lock: &Path) -> bool {
    static ASKING: Mutex<()> = Mutex::new(());
    let _alone = ASKING.lock().unwrap_or_else(PoisonError::into_inner);
    let Ok(tried) = fs::File::create(lock) else {
        return false;
    };
    match tried.try_lock() {
        Ok(()) => {
            let _ = tried.unlock();
            false
        }
        Err(fs::TryLockError::WouldBlock) => true,
        Err(fs::TryLockError::Error(_)) => false,
    }
}

/// Where what each of a base's ordinary checks read is kept, beside its
/// derivations; its traces and caches are named so a keep's sweep takes
/// them when a keep stopped left them.
pub fn keeping(cache: &Path, base: &str, system: &str) -> PathBuf {
    cache
        .join("changed-checks")
        .join(format!("{base}-{system}.alone.reads.json"))
}

/// Keep what each of a base's ordinary checks read, traced alone once its
/// derivations are kept, unless it's kept already, saying why when it
/// can't be: every gate on the base then evaluates them all, as before.
pub fn keep(root: &str, base: &str, system: &str, noted: &mut Run) {
    let at = keeping(&crate::cache(), base, system);
    if at.exists() {
        return;
    }
    let folder = crate::cache().join("changed-checks");
    if recently_recorded(&folder, system, SystemTime::now()) {
        eprintln!(
            "changed-checks: what {system}'s checks each read was recorded under \
             {} minutes ago, so this keep records it again later",
            STANDS_FOR.as_secs() / 60
        );
        return;
    }
    if let Err(why) = record(root, base, system, &at, noted) {
        eprintln!(
            "changed-checks: couldn't record what {system}'s checks each read at {}: {why}",
            short(base)
        );
    }
}

/// Whether what each of `system`'s checks read, at any base, was recorded
/// in `folder` less than `STANDS_FOR` before `now`.
fn recently_recorded(folder: &Path, system: &str, now: SystemTime) -> bool {
    let named = format!("-{system}.alone.reads.json");
    let Ok(entries) = fs::read_dir(folder) else {
        return false;
    };
    entries.flatten().any(|entry| {
        entry.file_name().to_string_lossy().ends_with(&named)
            && entry
                .metadata()
                .and_then(|kept| kept.modified())
                .is_ok_and(|at| now.duration_since(at).unwrap_or_default() < STANDS_FOR)
    })
}

/// Trace each of the base's ordinary checks alone and keep at `at` what
/// each one whose derivation is git's read, but the folders' checks.nix it
/// read only because checks.nix gathers them all; then its VM tests, kept
/// with them, unless a gate runs beside the keep.
fn record(root: &str, base: &str, system: &str, at: &Path, noted: &mut Run) -> Result<(), String> {
    let derivations = checks::read_kept(&checks::keeping(&crate::cache(), base, system))?
        .ok_or("its derivations aren't kept")?;
    let (tree, source) = reads::source(root, base)?;
    // A check whose derivation takes the whole source in goes unrecorded,
    // so every gate evaluates it.
    let whole = checks::taking_whole(&source, &derivations)?;
    let (vm, ordinary): (Vec<&String>, Vec<&String>) = derivations
        .keys()
        .filter(|name| !whole.contains(name.as_str()))
        .partition(|name| name.starts_with("vm-"));
    let tracing = (source.as_str(), system, at);
    // The floor is what every check read, so the record is narrowed whole.
    let gathering = gathered::files(&source, system);
    let ungathered = |each| narrowed(&source, gathering.as_ref(), each);
    let mut evaluation = Run::evaluating("alone");
    let each = traced(&ordinary, &derivations, tracing, &mut evaluation, || false);
    if each.is_empty() {
        noted.evaluated(evaluation);
        return Err("no check's trace could be read".to_owned());
    }
    written(at, &tree, &derivations, &ungathered(each.clone()))?;
    let lock = gating_lock();
    let stopped = AtomicBool::new(false);
    let stop = || {
        let runs = a_gate_runs(&lock);
        if runs && !stopped.swap(true, Ordering::Relaxed) {
            eprintln!(
                "changed-checks: a gate runs here, so this keep traces no more VM tests alone"
            );
        }
        runs
    };
    if vm.is_empty() || stop() {
        noted.evaluated(evaluation);
        return Ok(());
    }
    let vm_each = traced(&vm, &derivations, tracing, &mut evaluation, stop);
    noted.evaluated(evaluation);
    if vm_each.is_empty() {
        return Ok(());
    }
    let mut all = each;
    all.extend(vm_each);
    written(at, &tree, &derivations, &ungathered(all))
}

/// Trace each of `names` alone from `source`'s checks for `system`, its
/// traces and caches named after `at`, noting what it took as a chunk of
/// `evaluation`, and give what each whose derivation is `derivations`'
/// read. Once `stop` says so, none is begun.
fn traced<'a>(
    names: &[&'a String],
    derivations: &Derivations,
    (source, system, at): (&str, &str, &Path),
    evaluation: &mut Evaluation,
    stop: impl Fn() -> bool + Sync,
) -> BTreeMap<&'a str, Reads> {
    let checks = format!("path:{source}#checks.{system}");
    let strace = reads::strace();
    let numbered: Vec<(usize, &String)> = names.iter().copied().enumerate().collect();
    let began = Instant::now();
    let traced = checks::each_at_once(
        &numbered,
        AT_ONCE,
        || !crate::pressure::memory_stalled(),
        |(number, name)| {
            if stop() {
                return Ok::<_, String>(None);
            }
            let trace = at.with_extension(format!("trace{number}"));
            let fresh = at.with_extension(format!("cache{number}"));
            let traced = alone(&checks, &strace, source, &trace, &fresh, name);
            let _ = fs::remove_file(&trace);
            let _ = fs::remove_dir_all(&fresh);
            Ok(traced)
        },
    )
    .unwrap_or_default();
    evaluation.chunk(names.len(), began.elapsed());
    names
        .iter()
        .zip(traced)
        .filter_map(|(name, traced)| {
            let (said, reads) = traced?;
            (derivations.get(*name) == Some(&said)).then_some((name.as_str(), reads))
        })
        .collect()
}

/// What `each` read, less what `gathered.rs` says each read only because
/// checks.nix gathers every folder's checks, as `files` names them.
fn narrowed<'a>(
    source: &str,
    files: Option<&BTreeMap<String, String>>,
    mut each: BTreeMap<&'a str, Reads>,
) -> BTreeMap<&'a str, Reads> {
    if let Some(files) = files {
        gathered::narrow(Path::new(source), files, &mut each);
    }
    each
}

/// Keep at `at` the record of what `each` read of `tree`, whole, by rename.
fn written(
    at: &Path,
    tree: &str,
    derivations: &Derivations,
    each: &BTreeMap<&str, Reads>,
) -> Result<(), String> {
    let writing = at.with_extension("writing");
    let failed = |error: std::io::Error| format!("{}: {error}", at.display());
    fs::write(&writing, kept(tree, derivations.keys(), each).to_string()).map_err(failed)?;
    fs::rename(&writing, at).map_err(failed)
}

/// Evaluate the check `name` alone from `checks`, a plain path's, under
/// strace, writing its trace to `trace` with a cache at `fresh` that
/// starts empty, giving its derivation and what it read; none when either
/// can't be said.
fn alone(
    checks: &str,
    strace: &str,
    source: &str,
    trace: &Path,
    fresh: &Path,
    name: &str,
) -> Option<(Option<String>, Reads)> {
    let _ = fs::remove_dir_all(fresh);
    fs::create_dir_all(fresh).ok()?;
    let selecting = checks::selecting(&[name.to_owned()]).ok()?;
    let trace_name = trace.to_string_lossy();
    let mut asked = vec![
        "-f",
        "-qq",
        "-y",
        "-e",
        reads::TRACED,
        "-o",
        &trace_name,
        "nix",
    ];
    asked.extend(evaluating::pure(&[
        "eval",
        "--json",
        checks,
        "--apply",
        &selecting,
        evaluating::UNCACHED,
    ]));
    let ran = reads::uncached(fresh, strace, &asked).ok()?;
    let mut said: Derivations = serde_json::from_str(&ran).ok()?;
    let reads = Reads::traced(&fs::read_to_string(trace).ok()?, source).ok()?;
    // Every evaluation of the flake reads its flake.nix, so a trace
    // without it didn't see the evaluation.
    reads.read.contains("flake.nix").then_some(())?;
    Some((said.remove(name)?, reads))
}

/// The record as kept: the tree it was read from, the base's checks by
/// name, each path read once, and what each check read by its paths'
/// places among them, since every check reads hundreds of the same.
fn kept<'a>(
    tree: &str,
    names: impl Iterator<Item = &'a String>,
    each: &'a BTreeMap<&'a str, Reads>,
) -> Value {
    let mut paths: Vec<&str> = Vec::new();
    let mut found_at: HashMap<&str, usize> = HashMap::new();
    let mut placed = |set: &'a BTreeSet<String>| -> Vec<usize> {
        set.iter()
            .map(|path| {
                *found_at.entry(path.as_str()).or_insert_with(|| {
                    paths.push(path);
                    paths.len() - 1
                })
            })
            .collect()
    };
    let checks: Map<String, Value> = each
        .iter()
        .map(|(name, reads)| {
            let read = json!({
                "read": placed(&reads.read),
                "missing": placed(&reads.missing),
                "listed": placed(&reads.listed),
            });
            ((*name).to_owned(), read)
        })
        .collect();
    json!({
        "tree": tree,
        "names": names.collect::<Vec<_>>(),
        "paths": paths,
        "checks": checks,
    })
}

/// What each check recorded read, as kept, when it was made from `tree`.
fn from_kept(text: &str, tree: &str) -> Option<BTreeMap<String, Reads>> {
    let kept: Value = serde_json::from_str(text).ok()?;
    if kept.get("tree")?.as_str()? != tree {
        return None;
    }
    let texts = |value: &Value| -> Option<Vec<String>> {
        value
            .as_array()?
            .iter()
            .map(|text| text.as_str().map(str::to_owned))
            .collect()
    };
    let paths = texts(kept.get("paths")?)?;
    let placed = |value: &Value| -> Option<BTreeSet<String>> {
        value
            .as_array()?
            .iter()
            .map(|place| paths.get(usize::try_from(place.as_u64()?).ok()?).cloned())
            .collect()
    };
    let each = kept
        .get("checks")?
        .as_object()?
        .iter()
        .map(|(name, read)| {
            let reads = Reads {
                read: placed(read.get("read")?)?,
                missing: placed(read.get("missing")?)?,
                listed: placed(read.get("listed")?)?,
            };
            Some((name.clone(), reads))
        })
        .collect::<Option<_>>()?;
    Some(each)
}

/// How many of the newest records a gate asks git about: one further back
/// than these has more of main between it and the base, so reaches no fewer.
const NEWEST: usize = 8;

/// The newest head of main whose checks' reads were recorded alone, and
/// its derivations: the base itself, or a head it came after.
pub fn recorded(root: &str, base: &str, system: &str) -> Option<(String, Derivations)> {
    let named = format!("-{system}.alone.reads.json");
    let mut found: Vec<(SystemTime, String)> = fs::read_dir(crate::cache().join("changed-checks"))
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let head = entry
                .file_name()
                .to_string_lossy()
                .strip_suffix(&named)?
                .to_owned();
            Some((entry.metadata().ok()?.modified().ok()?, head))
        })
        .collect();
    found.sort_unstable_by(|one, other| other.cmp(one));
    found.into_iter().take(NEWEST).find_map(|(_, head)| {
        let asked = ["-C", root, "merge-base", "--is-ancestor", &head, base];
        if head != base && crate::run("git", &asked).is_err() {
            return None;
        }
        let derivations = checks::read_kept(&checks::keeping(&crate::cache(), &head, system));
        Some((head, derivations.ok().flatten()?))
    })
}

/// Note in the run the head `recorded` judges from and how many files moved
/// from it to the base and to the working tree, so a gate that evaluated
/// many checks can be traced to a record main had left behind.
pub fn note(root: &str, base: &str, recorded: Option<&(String, Derivations)>, noted: &mut Run) {
    let Some((head, _)) = recorded else {
        return;
    };
    let moved = |to| reads::touched_since(root, head, to).map(|touched| touched.len());
    let main = if head == base {
        Some(0)
    } else {
        moved(Some(base))
    };
    noted.recorded(head, main, moved(None));
}

/// The checks of `ordinary` whose reads at `head` the change from it to
/// `to`, a commit or, when none, the working tree, reaches none of, and the
/// rest; nothing when there's no record, it was made from another tree, or
/// git can't say what changed. A check the record lacks is among the rest,
/// and one it holds that `ordinary` doesn't name is left out. That holds
/// because the change is judged from `head` itself, not the gate's base: a
/// check added since is one the record lacks, and one whose set of checks
/// moved read the file that added or removed one.
fn partitioned(
    root: &str,
    (head, known): (&str, &Derivations),
    to: Option<&str>,
    system: &str,
    ordinary: &[String],
) -> Option<(Vec<String>, Vec<String>)> {
    let text = fs::read_to_string(keeping(&crate::cache(), head, system)).ok()?;
    let each = from_kept(&text, &reads::tree(root, head).ok()?)?;
    let touched = reads::touched_since(root, head, to)?;
    // Each check asks of the same folders, so git is asked of each once.
    let seen: RefCell<HashMap<String, bool>> = RefCell::new(HashMap::new());
    let came_or_went = |folder: &str| {
        if let Some(said) = seen.borrow().get(folder) {
            return *said;
        }
        let said = reads::came_or_went(root, head, to, folder);
        seen.borrow_mut().insert(folder.to_owned(), said);
        said
    };
    Some(ordinary.iter().cloned().partition(|name| {
        known.contains_key(name)
            && each.get(name).is_some_and(|reads| {
                touched
                    .iter()
                    .all(|(file, how)| !reads.reached_by(file, *how, &came_or_went))
            })
    }))
}

/// The derivations `known` gives of each of `names`.
fn known_of<'a>(
    known: &Derivations,
    names: impl Iterator<Item = &'a String>,
) -> impl Iterator<Item = (String, Option<String>)> {
    names.filter_map(|name| Some((name.clone(), known.get(name)?.clone())))
}

/// The recorded head's derivation of each of the tree's ordinary checks,
/// `ordinary`, whose reads there the change from it reaches none of, and the
/// checks left to evaluate: all of them when there's no record, it was made
/// from another tree, or git can't say what changed.
pub fn unreached(
    root: &str,
    recorded: Option<&(String, Derivations)>,
    system: &str,
    (ordinary, what): (Vec<String>, &str),
) -> (Derivations, Vec<String>) {
    let Some((head, known)) = recorded else {
        return (Derivations::new(), ordinary);
    };
    let Some((unreached, left)) = partitioned(root, (head, known), None, system, &ordinary) else {
        return (Derivations::new(), ordinary);
    };
    if !unreached.is_empty() {
        eprintln!(
            "changed-checks: evaluates {} of {system}'s {} {what}, since the change \
             reaches nothing the other {} read at {}, which keep their derivations there",
            left.len(),
            unreached.len() + left.len(),
            unreached.len(),
            short(head)
        );
    }
    (known_of(known, unreached.iter()).collect(), left)
}

/// The base's derivations when it came after the recorded head: each check
/// main reached nothing of since keeps the head's, the VM tests too when
/// main reached nothing they read, and only the rest are evaluated at the
/// base. Nothing when the base is the head, or main's change can't be
/// judged, so the whole base is evaluated.
pub fn past(
    root: &str,
    base: &str,
    system: &str,
    recorded: Option<&(String, Derivations)>,
    noted: &mut Run,
) -> Option<Derivations> {
    let (head, known) = recorded.filter(|(head, _)| head != base)?;
    let flake = format!("git+file://{root}?rev={base}");
    let named = evaluating::names(&flake, system).ok()?;
    let (vm, ordinary): (Vec<String>, Vec<String>) = named
        .iter()
        .cloned()
        .partition(|name| name.starts_with("vm-"));
    let (unreached, left) = partitioned(root, (head, known), Some(base), system, &ordinary)?;
    let vm_read = reads::keeping(&crate::cache(), head, system);
    let vm_kept = vm.iter().all(|name| known.contains_key(name))
        && reads::untouched(&vm_read, root, head, Some(base));
    // Short of all of them, each VM test main reached nothing it read alone
    // of keeps the head's derivation too.
    let (vm_unreached, vm_left) = if vm_kept {
        (vm, Vec::new())
    } else {
        partitioned(root, (head, known), Some(base), system, &vm).unwrap_or((Vec::new(), vm))
    };
    let mut asked = left.clone();
    asked.extend(vm_left.iter().cloned());
    let mut evaluation = Run::evaluating("base");
    let mut derivations = if asked.is_empty() {
        Derivations::new()
    } else {
        evaluating::evaluated(&flake, system, &asked, &mut evaluation).ok()?
    };
    noted.evaluated(evaluation);
    let wanted: BTreeSet<&String> = asked.iter().collect();
    derivations.retain(|name, _| wanted.contains(name));
    derivations.extend(known_of(known, unreached.iter().chain(&vm_unreached)));
    if !vm_kept {
        eprintln!(
            "changed-checks: evaluates {} of {system}'s {} VM tests at the base {}, since main \
             from {} reaches nothing the other {} read",
            vm_left.len(),
            vm_left.len() + vm_unreached.len(),
            short(base),
            short(head),
            vm_unreached.len()
        );
    }
    eprintln!(
        "changed-checks: evaluates {} of {system}'s {} other checks at the base {}, since main \
         from {} reaches nothing the other {} read",
        left.len(),
        ordinary.len(),
        short(base),
        short(head),
        unreached.len()
    );
    Some(derivations)
}

/// A commit by its first 12 characters, as changed-checks names one.
fn short(commit: &str) -> &str {
    commit.get(..12).unwrap_or(commit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scratch::Scratch;

    #[test]
    fn a_gate_holding_its_lock_is_seen_running_until_every_gate_lets_go() {
        let scratch = Scratch::new("gating");
        let lock = scratch.join("changed-checks/gating.lock");
        assert!(!a_gate_runs(&lock), "no gate ran, yet one was seen");
        let gate = gating(&lock).expect("the gate's lock");
        assert!(a_gate_runs(&lock), "a gate ran unseen");
        let other = gating(&lock).expect("a second gate shares it");
        drop(gate);
        assert!(a_gate_runs(&lock), "the second gate ran unseen");
        drop(other);
        assert!(!a_gate_runs(&lock), "a gate was seen after both let go");
    }

    #[test]
    fn a_keeps_threads_asking_together_never_take_one_another_for_a_gate() {
        // Each of a keep's 16 tracing threads asks before it begins a VM
        // test, and a lock tried whole is held, for that instant, against
        // every other open of it, this process's too. Meanwhile the others
        // start nix, and a program started holds a copy of each file open
        // until it runs, with any lock on it.
        let scratch = Scratch::new("gating-together");
        let lock = scratch.join("changed-checks/gating.lock");
        fs::create_dir_all(lock.parent().expect("its folder")).expect("made");
        let asked = std::sync::atomic::AtomicBool::new(false);
        let seen = std::thread::scope(|scope| {
            scope.spawn(|| {
                while !asked.load(std::sync::atomic::Ordering::Relaxed) {
                    let _ = std::process::Command::new("changed-checks-starts-nothing").status();
                }
            });
            let asking: Vec<_> = (0..AT_ONCE)
                .map(|_| scope.spawn(|| (0..200).filter(|_| a_gate_runs(&lock)).count()))
                .collect();
            let seen = asking
                .into_iter()
                .map(|thread| thread.join().expect("asked"))
                .sum::<usize>();
            asked.store(true, std::sync::atomic::Ordering::Relaxed);
            seen
        });
        assert_eq!(
            seen, 0,
            "with no gate, a keep's threads saw one {seen} times"
        );
    }
}
