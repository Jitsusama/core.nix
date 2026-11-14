//! What a change did to each check's derivation, and how the checks are
//! evaluated and kept to find out.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};
use std::thread;

/// Each check's derivation path by its name, or nothing for a check that
/// doesn't evaluate.
pub type Derivations = BTreeMap<String, Option<String>>;

/// How much one nix eval takes, by its checks' weight. Nix's evaluator
/// never gives memory back, so one holds everything its checks share and
/// each machine they build. Measured on core's 277 checks, an evaluation
/// holds about 2 GiB of its own, an ordinary check about 75 MB more and a
/// VM test about 300 MB, so a VM test weighs four: forty VM tests in one
/// evaluation peaked at 13.8 GiB, ten at 5.6 GiB, forty ordinary checks at
/// 4.9 GiB. Smaller chunks than these only add evaluations' own cost.
pub const WEIGHT: usize = 40;

/// What a VM test weighs against an ordinary check.
const VM_TEST: usize = 4;

/// How many chunks are evaluated at once on a machine with little to
/// spare. Measured on core's 305 checks, a comparison's evaluations kept
/// about 1.6 of the machine's 16 cores busy and peaked at 5.8 GB one at a
/// time, so two at once take about half as long and hold about twice that,
/// well short of the 24 GB that evaluating every check in one held. Each
/// further one begins only while memory isn't stalling (see pressure.rs).
pub const AT_ONCE: usize = 2;

/// The most chunks one evaluation takes at once on any machine; a keep's
/// two halves side by side take this each. The lent builder, with
/// 32 cores and 125 GB nearly all free, evaluated two at a time for a
/// median of 8 minutes a side on 5 November, four fifths of a whole run,
/// while its build took under 2; core's 21 chunks a side go in three turns
/// of eight.
const AT_ONCE_MOST: usize = 8;

/// The cores one evaluation is given room for. Each keeps about one busy,
/// and the rest are left to the builds beside it.
const CORES_EACH: usize = 4;

/// The memory one evaluation is given room for: the heaviest chunk, ten VM
/// tests, peaked at 5.6 GiB.
const MEMORY_EACH: u64 = 6 << 30;

/// How many chunks to evaluate at once on a machine with these cores and
/// this much memory available. On a builder made for the run, as many as
/// both have room for, between `AT_ONCE` and `AT_ONCE_MOST`: the lent
/// builder takes eight. On the person's own machine, `AT_ONCE` however much
/// is free, since its cores are shared with everything else there: on
/// 7 November a run given room for four on a 16-core machine with 35 GB
/// free had three evaluating as its CPU pressure reached 40%, nearly all of
/// it theirs.
pub fn at_once(cores: usize, available: u64, builder: bool) -> usize {
    if !builder {
        return AT_ONCE;
    }
    let by_memory = usize::try_from(available / MEMORY_EACH).unwrap_or(usize::MAX);
    (cores / CORES_EACH)
        .min(by_memory)
        .clamp(AT_ONCE, AT_ONCE_MOST)
}

/// How many chunks this machine evaluates at once, by `at_once`, from its
/// cores and the memory `/proc/meminfo` says is available, as a builder
/// made for the run when `builder` says so. A machine that says neither gets
/// `AT_ONCE`.
pub fn at_once_here(builder: bool) -> usize {
    let (cores, available) = room_here();
    at_once(cores, available, builder)
}

/// How many chunks each of two evaluations side by side on a builder made
/// for the run takes at once: as many as half its cores and half its
/// memory available have room for, so together they hold no more than the
/// builder has room for. The lent builder, with 64 cores and 406 GB free
/// beside the day's gates on 12 November, takes eight each.
pub fn at_once_each_of_two_here() -> usize {
    let (cores, available) = room_here();
    at_once(cores / 2, available / 2, true)
}

/// This machine's cores, and the bytes of memory `/proc/meminfo` says are
/// available, or none.
pub fn room_here() -> (usize, u64) {
    let cores = thread::available_parallelism().map_or(0, std::num::NonZero::get);
    let available = fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|text| available(&text))
        .unwrap_or(0);
    (cores, available)
}

/// The bytes `/proc/meminfo`'s `MemAvailable` line says, or none.
fn available(meminfo: &str) -> Option<u64> {
    let line = meminfo
        .lines()
        .find_map(|line| line.strip_prefix("MemAvailable:"))?;
    let kib: u64 = line.trim().strip_suffix("kB")?.trim().parse().ok()?;
    kib.checked_mul(1024)
}

/// How long a chunk that may not begin yet waits before asking again.
const WAIT_TO_BEGIN: std::time::Duration = std::time::Duration::from_secs(1);

/// What a check's name may hold beside letters and digits, so a name is
/// never read as Nix code.
const NAMED: [char; 4] = ['_', '.', '+', '-'];

/// How each check's derivation moved from the base to the working tree.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Comparison {
    pub added: Vec<String>,
    pub changed: Vec<String>,
    pub removed: Vec<String>,
    pub broken: Vec<String>,
}

impl Comparison {
    /// Give each kind of move, named as a line of the report names it,
    /// with the checks that moved that way.
    pub fn kinds(&self) -> [(&'static str, &[String]); 4] {
        [
            ("added", &self.added),
            ("changed", &self.changed),
            ("removed", &self.removed),
            ("broken", &self.broken),
        ]
    }

    /// Give the checks worth building, added then changed, with or without
    /// the VM tests.
    pub fn to_build(&self, vm: bool) -> Vec<&str> {
        self.added
            .iter()
            .chain(&self.changed)
            .map(String::as_str)
            .filter(|name| vm || !is_vm_test(name))
            .collect()
    }
}

/// Compare each check's derivation at the base with the working tree's.
///
/// A check that stops evaluating is broken, not changed; one that starts
/// to evaluate again, or evaluates for the first time, has changed or been
/// added. One that never evaluated is named nowhere.
pub fn compare(before: &Derivations, after: &Derivations) -> Comparison {
    let mut comparison = Comparison::default();
    for (name, now) in after {
        match (before.get(name), now) {
            (None, Some(_)) => comparison.added.push(name.clone()),
            (Some(Some(_)), None) => comparison.broken.push(name.clone()),
            (Some(then), Some(_)) if then != now => comparison.changed.push(name.clone()),
            _ => {}
        }
    }
    comparison.removed = before
        .keys()
        .filter(|name| !after.contains_key(*name))
        .cloned()
        .collect();
    comparison
}

/// Whether the check named so is a VM test, as the flake names each one.
pub fn is_vm_test(name: &str) -> bool {
    name.starts_with("vm-")
}

/// Split the names, in order, into chunks weighing at most `WEIGHT` each.
pub fn chunks(names: &[String]) -> Vec<&[String]> {
    let weight = |name: &String| if is_vm_test(name) { VM_TEST } else { 1 };
    let mut chunks = Vec::new();
    let (mut start, mut held) = (0, 0);
    for (at, name) in names.iter().enumerate() {
        if held + weight(name) > WEIGHT {
            chunks.push(&names[start..at]);
            (start, held) = (at, 0);
        }
        held += weight(name);
    }
    if start < names.len() {
        chunks.push(&names[start..]);
    }
    chunks
}

/// The order to begin these chunks in: those holding the most VM tests
/// first, the rest in their order. A chunk of ten VM tests took 105 to
/// 218 s to evaluate against 20 to 90 s for one of forty other checks
/// (13 November), yet the VM tests' names, and so their chunks, come last,
/// so each side waited on them after everything else. Begun first, a
/// side's median over that day's runs would have been 237 s, not 261.
/// Their weight can't tell them apart: ten VM tests weigh as forty checks.
pub fn begin_order(chunks: &[&[String]]) -> Vec<usize> {
    let vm_tests = |chunk: &[String]| chunk.iter().filter(|name| is_vm_test(name)).count();
    let mut order: Vec<usize> = (0..chunks.len()).collect();
    order.sort_by_key(|&at| std::cmp::Reverse(vm_tests(chunks[at])));
    order
}

/// Do `work` on each item, up to `at_once` at a time, answering in the
/// items' order, as [`each_at_once_in`] does beginning them in it.
pub fn each_at_once<T, R, E>(
    items: &[T],
    at_once: usize,
    may_begin: impl Fn() -> bool + Sync,
    work: impl Fn(&T) -> Result<R, E> + Sync,
) -> Result<Vec<R>, E>
where
    T: Sync,
    R: Send,
    E: Send,
{
    let begin: Vec<usize> = (0..items.len()).collect();
    each_at_once_in(items, &begin, at_once, may_begin, work)
}

/// Do `work` on each item, up to `at_once` at a time, beginning them in
/// `begin`'s order, which names each item's place once, and answering in
/// the items' order. Once one fails, no item not yet begun is begun, and
/// the first failure in the items' order among those begun is what the
/// run reports: an item earlier by order may never have begun.
///
/// Only one item at a time is sure to go: each further one begins only
/// while `may_begin` says so, and otherwise waits and asks again. So a run
/// always makes progress, one item at a time while memory stalls.
pub fn each_at_once_in<T, R, E>(
    items: &[T],
    begin: &[usize],
    at_once: usize,
    may_begin: impl Fn() -> bool + Sync,
    work: impl Fn(&T) -> Result<R, E> + Sync,
) -> Result<Vec<R>, E>
where
    T: Sync,
    R: Send,
    E: Send,
{
    debug_assert_eq!(begin.len(), items.len(), "begin names each item once");
    let next = AtomicUsize::new(0);
    let failed = AtomicBool::new(false);
    let done: Mutex<Vec<Option<Result<R, E>>>> = Mutex::new(items.iter().map(|_| None).collect());
    thread::scope(|scope| {
        for worker in 0..at_once.clamp(1, items.len().max(1)) {
            let (next, failed, done, work, may_begin) = (&next, &failed, &done, &work, &may_begin);
            scope.spawn(move || {
                while !failed.load(Ordering::Relaxed) {
                    // Every worker but the first waits for room to begin, and
                    // stops waiting once nothing is left for it.
                    if worker > 0 && !may_begin() {
                        if next.load(Ordering::Relaxed) >= items.len() {
                            break;
                        }
                        thread::sleep(WAIT_TO_BEGIN);
                        continue;
                    }
                    let Some(&at) = begin.get(next.fetch_add(1, Ordering::Relaxed)) else {
                        break;
                    };
                    let Some(item) = items.get(at) else {
                        break;
                    };
                    let answer = work(item);
                    failed.fetch_or(answer.is_err(), Ordering::Relaxed);
                    done.lock().unwrap_or_else(PoisonError::into_inner)[at] = Some(answer);
                }
            });
        }
    });
    // An item left unbegun after a failure has no answer, and the failure
    // before it is returned first.
    done.into_inner()
        .unwrap_or_else(PoisonError::into_inner)
        .into_iter()
        .flatten()
        .collect()
}

/// Give the Nix function taking a flake's checks to these ones'
/// derivations, or null for one that doesn't evaluate, so one broken check
/// is named rather than stopping the whole evaluation.
///
/// A name that could be read as Nix code rather than a name is refused.
pub fn selecting(names: &[String]) -> Result<String, String> {
    if let Some(name) = names.iter().find(|name| !is_name(name)) {
        return Err(format!("{name:?} isn't a check's name"));
    }
    let listed: Vec<String> = names.iter().map(|name| format!("\"{name}\"")).collect();
    Ok(format!(
        "checks: builtins.mapAttrs (_: check: let tried = builtins.tryEval check.drvPath; \
         in if tried.success then tried.value else null) (builtins.intersectAttrs \
         (builtins.listToAttrs (map (name: {{ inherit name; value = null; }}) [{}])) checks)",
        listed.join(" ")
    ))
}

/// Whether the text could be a check's name and nothing more.
fn is_name(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|letter| letter.is_ascii_alphanumeric() || NAMED.contains(&letter))
}

/// Give the derivations kept at the path, or evaluate and keep them.
///
/// They're written beside the path and moved onto it, so a run stopped
/// halfway keeps nothing rather than half. Evaluating holds a lock beside
/// them, so a run that wants a commit another run is evaluating waits for
/// its result rather than evaluating it too: main moves between most
/// runs, and each new base cost every run that met it a whole evaluation.
pub fn kept(
    path: &Path,
    evaluate: impl FnOnce() -> Result<Derivations, String>,
) -> Result<Derivations, String> {
    let unreadable = |error: &dyn std::fmt::Display| format!("{}: {error}", path.display());
    if let Some(found) = read_kept(path)? {
        return Ok(found);
    }
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder).map_err(|error| unreadable(&error))?;
    }
    let lock = fs::File::create(path.with_extension("lock")).map_err(|error| unreadable(&error))?;
    lock.lock().map_err(|error| unreadable(&error))?;
    if let Some(found) = read_kept(path)? {
        return Ok(found);
    }
    let found = evaluate()?;
    let writing = path.with_extension("writing");
    let text = serde_json::to_string(&found).map_err(|error| unreadable(&error))?;
    fs::write(&writing, text).map_err(|error| unreadable(&error))?;
    fs::rename(&writing, path).map_err(|error| unreadable(&error))?;
    Ok(found)
}

/// The checks among `derivations` whose derivation takes `source`, the
/// flake's whole source, in, so its build may read any file though no trace
/// saw it read: `formatting`'s evaluation read 408 files on 12 November.
/// No record of reads may stand for one of these. The closure is transitive
/// on purpose: `formatting` takes the source in through treefmt's check, a
/// derivation it depends on, which `--referrers` alone wouldn't name.
/// A check naming the source's root by a path takes a copy of it in, as
/// [`copied`] says, which counts the same.
pub fn taking_whole<'a>(
    source: &str,
    derivations: &'a Derivations,
) -> Result<BTreeSet<&'a str>, String> {
    let copy = copied(source)?;
    let mut asked = vec!["--query", "--referrers-closure", source];
    // Nix refuses a path it doesn't hold, and none took a copy not made.
    if Path::new(&copy).exists() {
        asked.push(&copy);
    }
    let referring = crate::run("nix-store", &asked)?;
    let referring: BTreeSet<&str> = referring.lines().collect();
    Ok(derivations
        .iter()
        .filter(|(_, derivation)| {
            derivation
                .as_deref()
                .is_some_and(|it| referring.contains(it))
        })
        .map(|(name, _)| name.as_str())
        .collect())
}

/// The path Nix copies the source's root to when a file names it by a
/// path, as `src = ../..` in a folder's `checks.nix` does: one of its own,
/// named after the source's, made without reading a file a trace sees.
/// every-long-file-says-why took it so on 13 November, and its record said
/// it read no file, so a gate kept its base's derivation through a change.
fn copied(source: &str) -> Result<String, String> {
    let name = Path::new(source)
        .file_name()
        .map(|it| it.to_string_lossy().into_owned())
        .ok_or_else(|| format!("{source} names no file"))?;
    let asked = ["store", "add", "--dry-run", "--name", &name, source];
    crate::run("nix", &crate::evaluating::pure(&asked))
}

/// Give the derivations kept at the path, or none when nothing is kept.
pub fn read_kept(path: &Path) -> Result<Option<Derivations>, String> {
    let unreadable = |error: &dyn std::fmt::Display| format!("{}: {error}", path.display());
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map(Some)
            .map_err(|error| unreadable(&error)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(unreadable(&error)),
    }
}

/// Give where a commit's derivations for a system are kept, under the
/// cache folder given.
pub fn keeping(cache: &Path, base: &str, system: &str) -> PathBuf {
    cache
        .join("changed-checks")
        .join(format!("{base}-{system}.json"))
}

/// The system to compare when none is named, from how many checks each
/// system has: the one with the most, since a repository may hold some on
/// one system alone, as core holds its checks about checks on Linux, and a
/// run on another machine compared only its own and passed over them.
/// This machine's own wins a tie, and stands when the flake names none.
pub fn busiest(counts: &BTreeMap<String, u64>, here: &str) -> String {
    let own = counts.get(here).copied().unwrap_or(0);
    counts
        .iter()
        .filter(|&(_, &count)| count > own)
        .max_by_key(|&(_, &count)| count)
        .map_or_else(|| here.to_owned(), |(system, _)| system.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scratch::Scratch;

    #[test]
    fn the_system_with_the_most_checks_is_compared_and_this_one_wins_a_tie() {
        let counts = |pairs: &[(&str, u64)]| {
            pairs
                .iter()
                .map(|&(system, count)| (system.to_owned(), count))
                .collect::<BTreeMap<_, _>>()
        };
        let core = counts(&[("aarch64-darwin", 136), ("x86_64-linux", 444)]);
        assert_eq!(busiest(&core, "aarch64-darwin"), "x86_64-linux");
        assert_eq!(busiest(&core, "x86_64-linux"), "x86_64-linux");
        let even = counts(&[("aarch64-darwin", 3), ("x86_64-linux", 3)]);
        assert_eq!(busiest(&even, "aarch64-darwin"), "aarch64-darwin");
        assert_eq!(busiest(&BTreeMap::new(), "riscv64-linux"), "riscv64-linux");
    }

    fn derivations(pairs: &[(&str, Option<&str>)]) -> Derivations {
        pairs
            .iter()
            .map(|(name, path)| ((*name).to_owned(), path.map(str::to_owned)))
            .collect()
    }

    fn names(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn a_check_whose_derivation_differs_has_changed() {
        let moved = compare(
            &derivations(&[("a", Some("a.drv")), ("b", Some("b.drv"))]),
            &derivations(&[("a", Some("a2.drv")), ("b", Some("b.drv"))]),
        );
        assert_eq!(moved.changed, names(&["a"]));
    }

    #[test]
    fn a_check_only_in_the_working_tree_is_added() {
        let moved = compare(&derivations(&[]), &derivations(&[("new", Some("new.drv"))]));
        assert_eq!(moved.added, names(&["new"]));
    }

    #[test]
    fn a_check_only_at_the_base_is_removed() {
        let moved = compare(&derivations(&[("old", Some("old.drv"))]), &derivations(&[]));
        assert_eq!(moved.removed, names(&["old"]));
    }

    #[test]
    fn a_check_that_stops_evaluating_is_broken_not_changed() {
        let moved = compare(
            &derivations(&[("a", Some("a.drv"))]),
            &derivations(&[("a", None)]),
        );
        assert_eq!(
            moved,
            Comparison {
                broken: names(&["a"]),
                ..Comparison::default()
            }
        );
    }

    #[test]
    fn a_check_that_evaluates_again_has_changed() {
        let moved = compare(
            &derivations(&[("a", None)]),
            &derivations(&[("a", Some("a.drv"))]),
        );
        assert_eq!(moved.changed, names(&["a"]));
    }

    #[test]
    fn a_check_that_never_evaluated_is_named_nowhere() {
        let moved = compare(
            &derivations(&[("a", None)]),
            &derivations(&[("a", None), ("b", None)]),
        );
        assert_eq!(moved, Comparison::default());
    }

    #[test]
    fn the_vm_tests_are_left_out_when_asked() {
        let moved = compare(
            &derivations(&[("vm-a", Some("1")), ("b", Some("1"))]),
            &derivations(&[("vm-a", Some("2")), ("b", Some("2")), ("c", Some("1"))]),
        );
        assert_eq!(moved.to_build(true), ["c", "b", "vm-a"]);
        assert_eq!(moved.to_build(false), ["c", "b"]);
    }

    #[test]
    fn every_name_is_in_one_chunk_in_its_order() {
        let all: Vec<String> = (0..85).map(|at| format!("check-{at}")).collect();
        let chunks = chunks(&all);
        assert_eq!(sizes(&chunks), [40, 40, 5]);
        assert_eq!(chunks.concat(), all);
    }

    fn sizes(chunks: &[&[String]]) -> Vec<usize> {
        chunks.iter().map(|chunk| chunk.len()).collect()
    }

    #[test]
    fn a_vm_test_weighs_four_ordinary_checks() {
        let all: Vec<String> = (0..12).map(|at| format!("vm-{at}")).collect();
        assert_eq!(sizes(&chunks(&all)), [10, 2]);
    }

    #[test]
    fn ordinary_checks_and_vm_tests_share_a_chunk_by_weight() {
        let mut all: Vec<String> = (0..30).map(|at| format!("check-{at}")).collect();
        all.extend((0..5).map(|at| format!("vm-{at}")));
        // Thirty and two VM tests weigh 38; a third would bring it to 42.
        assert_eq!(sizes(&chunks(&all)), [32, 3]);
    }

    #[test]
    fn a_builder_with_little_memory_free_evaluates_two_at_once() {
        assert_eq!(at_once(16, 11 << 30, true), 2);
    }

    #[test]
    fn a_big_idle_builder_evaluates_eight_at_once() {
        assert_eq!(at_once(32, 122 << 30, true), 8);
    }

    #[test]
    fn the_scarcer_of_cores_and_memory_sets_how_many() {
        assert_eq!(at_once(32, 24 << 30, true), 4);
        assert_eq!(at_once(12, 122 << 30, true), 3);
    }

    #[test]
    fn a_machine_that_says_nothing_still_evaluates_two_at_once() {
        assert_eq!(at_once(0, 0, true), AT_ONCE);
    }

    #[test]
    fn the_person_s_own_machine_evaluates_two_however_much_is_free() {
        assert_eq!(at_once(16, 35 << 30, false), AT_ONCE);
        assert_eq!(at_once(32, 122 << 30, false), AT_ONCE);
    }

    #[test]
    fn the_available_memory_is_read_from_meminfo() {
        let meminfo = "MemTotal:       131072000 kB\nMemAvailable:   127926272 kB\n";
        assert_eq!(available(meminfo), Some(127_926_272 * 1024));
        assert_eq!(available("MemTotal: 1 kB\n"), None);
    }

    #[test]
    fn work_done_at_once_is_answered_in_its_items_order() {
        let items: Vec<usize> = (0..7).collect();
        let answered = each_at_once(&items, AT_ONCE, || true, |item| Ok::<_, ()>(item * 10));
        assert_eq!(answered, Ok(vec![0, 10, 20, 30, 40, 50, 60]));
    }

    #[test]
    fn while_memory_stalls_one_item_goes_at_a_time_and_every_one_is_done() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let items: Vec<usize> = (0..5).collect();
        let (running, most) = (AtomicUsize::new(0), AtomicUsize::new(0));
        let answered = each_at_once(
            &items,
            AT_ONCE,
            || false,
            |item| {
                let now = running.fetch_add(1, Ordering::SeqCst) + 1;
                most.fetch_max(now, Ordering::SeqCst);
                thread::sleep(std::time::Duration::from_millis(20));
                running.fetch_sub(1, Ordering::SeqCst);
                Ok::<_, ()>(*item)
            },
        );
        assert_eq!(answered, Ok(vec![0, 1, 2, 3, 4]));
        assert_eq!(most.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn with_room_the_items_go_at_once() {
        use std::sync::Barrier;
        // Both items wait on each other, so this ends only if both run at once.
        let together = Barrier::new(2);
        let answered = each_at_once(
            &[0, 1],
            AT_ONCE,
            || true,
            |item| {
                together.wait();
                Ok::<_, ()>(*item)
            },
        );
        assert_eq!(answered, Ok(vec![0, 1]));
    }

    #[test]
    fn the_first_failure_in_order_is_answered() {
        let items: Vec<usize> = (0..7).collect();
        let answered = each_at_once(
            &items,
            1,
            || true,
            |item| {
                if *item >= 3 { Err(*item) } else { Ok(*item) }
            },
        );
        assert_eq!(answered, Err(3));
    }

    #[test]
    fn work_begun_in_another_order_is_answered_in_its_items_order() {
        let begun = Mutex::new(Vec::new());
        let answered = each_at_once_in(
            &[0, 1, 2],
            &[2, 0, 1],
            1,
            || true,
            |item| {
                begun.lock().expect("not poisoned").push(*item);
                Ok::<_, ()>(item * 10)
            },
        );
        assert_eq!(answered, Ok(vec![0, 10, 20]));
        assert_eq!(begun.into_inner().expect("not poisoned"), [2, 0, 1]);
    }

    #[test]
    fn chunks_holding_the_most_vm_tests_begin_first() {
        let mut all: Vec<String> = (0..45).map(|at| format!("check-{at}")).collect();
        all.extend((0..12).map(|at| format!("vm-{at}")));
        let chunks = chunks(&all);
        // Forty checks; five checks and eight VM tests, weighing 37; four
        // VM tests.
        assert_eq!(sizes(&chunks), [40, 13, 4]);
        assert_eq!(begin_order(&chunks), [1, 2, 0]);
    }

    #[test]
    fn no_work_is_no_answers() {
        let answered = each_at_once(&[] as &[usize], AT_ONCE, || true, |item| Ok::<_, ()>(*item));
        assert_eq!(answered, Ok(vec![]));
    }

    #[test]
    fn no_checks_need_no_evaluation() {
        assert_eq!(chunks(&[]), [] as [&[std::string::String]; 0]);
    }

    #[test]
    fn a_chunk_names_each_of_its_checks() {
        let function = selecting(&names(&["a", "vm-b.c+d_e"])).expect("good names");
        assert!(function.contains(r#"["a" "vm-b.c+d_e"]"#), "{function}");
    }

    #[test]
    fn a_name_nix_would_read_as_code_is_refused() {
        for name in ["a\" ++ builtins.abort \"", "", "a b"] {
            assert!(selecting(&names(&["fine", name])).is_err(), "{name:?}");
        }
    }

    #[test]
    fn the_first_run_evaluates_and_keeps_what_it_found() {
        let scratch = Scratch::new("first");
        let path = scratch.join("kept").join("abc-x86_64-linux.json");
        let found = kept(&path, || {
            Ok(derivations(&[("a", Some("a.drv")), ("broken", None)]))
        });
        assert_eq!(
            found,
            Ok(derivations(&[("a", Some("a.drv")), ("broken", None)]))
        );
        assert!(path.exists());
        assert!(!path.with_extension("writing").exists());
    }

    #[test]
    fn a_later_run_reads_what_was_kept_without_evaluating() {
        let scratch = Scratch::new("later");
        let path = scratch.join("abc-x86_64-linux.json");
        kept(&path, || Ok(derivations(&[("a", Some("a.drv"))]))).expect("kept");
        let found = kept(&path, || panic!("it evaluated again"));
        assert_eq!(found, Ok(derivations(&[("a", Some("a.drv"))])));
    }

    #[test]
    fn runs_wanting_one_commit_at_once_evaluate_it_once() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let scratch = Scratch::new("at-once");
        let path = scratch.join("abc-x86_64-linux.json");
        let evaluations = AtomicUsize::new(0);
        let found: Vec<_> = thread::scope(|scope| {
            let runs: Vec<_> = (0..4)
                .map(|_| {
                    scope.spawn(|| {
                        kept(&path, || {
                            evaluations.fetch_add(1, Ordering::SeqCst);
                            // Long enough that the other runs arrive while
                            // this one holds the lock.
                            thread::sleep(std::time::Duration::from_millis(200));
                            Ok(derivations(&[("a", Some("a.drv"))]))
                        })
                    })
                })
                .collect();
            runs.into_iter()
                .map(|run| run.join().expect("ran"))
                .collect()
        });
        assert_eq!(evaluations.load(Ordering::SeqCst), 1);
        assert!(
            found
                .iter()
                .all(|found| *found == Ok(derivations(&[("a", Some("a.drv"))])))
        );
    }

    #[test]
    fn an_evaluation_that_fails_keeps_nothing() {
        let scratch = Scratch::new("failed");
        let path = scratch.join("abc-x86_64-linux.json");
        assert_eq!(kept(&path, || Err("no".to_owned())), Err("no".to_owned()));
        assert!(!path.exists());
    }

    #[test]
    fn a_commits_derivations_are_kept_by_commit_and_system() {
        assert_eq!(
            keeping(Path::new("/cache"), "abc", "x86_64-linux"),
            Path::new("/cache/changed-checks/abc-x86_64-linux.json")
        );
    }
}
