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
//! evaluates it, as a gate does each check the record lacks.
//!
//! A keep past a recorded head copies what each check main's change from
//! it reaches nothing of read there, as a gate keeps its derivation, and
//! traces only the rest, so each kept head is recorded in about the time
//! its change takes to trace. Recording only once in 20 minutes, as keeps
//! did, left gates on 13 November judging from a record 25 merges behind,
//! and 21 of 63 since noon paid a median 281 s evaluating main's side.
//! Every few hours a keep traces every check again, so a copy misjudged
//! is carried no further than that.
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
//! While a gate runs beside it on the builder, a keep traces them one at
//! a time, and keeps the record after each: keeps that traced none beside
//! a gate recorded no VM test all evening on 13 November, as a gate nearly
//! always ran when they reached them, and copying carried none on. Beside
//! a gate it begins none after five minutes but the first, so the next
//! head's record isn't held back: that evening gates judged from records
//! 50 to 160 files behind main and evaluated 400 to 650 checks. The record
//! says since when each it left has waited, and the next keep traces those
//! first, the longest waiting first, so a busy main can't spend every keep
//! on the same few. One it didn't trace, as one the record lacks, is
//! evaluated by every gate the union doesn't spare.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime};

use serde_json::{Map, Value, json};

use crate::checks::{self, Derivations};
use crate::evaluating;
use crate::gathered;
use crate::gating;
use crate::reads::{self, Reads};
use crate::runs::{Evaluation, Run};

/// How many checks are traced at once, each in a process of its own, far
/// lighter than a chunk's.
pub const AT_ONCE: usize = 16;

/// How long after every check was last traced alone a record may still be
/// copied forward: tracing every one costs a keep about 400 s.
const COPIED_FOR: Duration = Duration::from_hours(4);

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
/// The record says how many keeps in a row `carried` to the base's
/// derivations (`carry.rs`).
pub fn keep(root: &str, base: &str, system: &str, carried: u64, noted: &mut Run) {
    let at = keeping(&crate::cache(), base, system);
    if at.exists() {
        return;
    }
    if let Err(why) = record(root, (base, carried), system, &at, noted) {
        eprintln!(
            "changed-checks: couldn't record what {system}'s checks each read at {}: {why}",
            short(base)
        );
    }
}

/// What the record of `head`, a head of main `base` came after, with its
/// derivations `known`, gives a keep of `base`: what each check main's
/// change from it reaches nothing of read there, where its derivation is
/// the base's too, and when every check was last traced. Nothing when it
/// was made from another tree, or every check was last traced
/// `COPIED_FOR` ago.
fn copied(
    root: &str,
    base: &str,
    system: &str,
    derivations: &Derivations,
    (head, known): &(String, Derivations),
) -> Option<(BTreeMap<String, Reads>, u64)> {
    let text = fs::read_to_string(keeping(&crate::cache(), head, system)).ok()?;
    let whole = serde_json::from_str::<Value>(&text)
        .ok()?
        .get("whole")?
        .as_u64()?;
    if now().saturating_sub(whole) >= COPIED_FOR.as_secs() {
        return None;
    }
    let mut each = from_kept(&text, &reads::tree(root, head).ok()?)?;
    let names: Vec<String> = derivations.keys().cloned().collect();
    let (unreached, _) = partitioned(root, (head.as_str(), known), Some(base), system, &names)?;
    let copied: BTreeMap<String, Reads> = unreached
        .into_iter()
        .filter(|name| known.get(name) == derivations.get(name))
        .filter_map(|name| each.remove(&name).map(|reads| (name, reads)))
        .collect();
    eprintln!(
        "changed-checks: copies what {} of {system}'s {} checks read at {}, since main from \
         it reaches nothing they read, and traces the rest alone",
        copied.len(),
        names.len(),
        short(head)
    );
    Some((copied, whole))
}

/// Since when, in seconds since 1970, each VM test the record of `head`
/// left untraced has waited to be: none when it says nothing of it.
fn waiting(head: &str, system: &str) -> BTreeMap<String, u64> {
    let text = fs::read_to_string(keeping(&crate::cache(), head, system)).unwrap_or_default();
    serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|record| {
            let waiting = record.get("waiting")?.as_object()?;
            let since = |(name, at): (&String, &Value)| Some((name.clone(), at.as_u64()?));
            Some(waiting.iter().filter_map(since).collect())
        })
        .unwrap_or_default()
}

/// Now, in seconds since 1970.
fn now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// Trace each of the base's ordinary checks alone and keep at `at` what
/// each one whose derivation is git's read, but the folders' checks.nix it
/// read only because checks.nix gathers them all; then its VM tests, kept
/// with them after each, the longest waiting first, and one at a time while
/// a gate runs beside the keep, which begins none past a bound but the
/// first (`gating.rs`).
fn record(
    root: &str,
    (base, carried): (&str, u64),
    system: &str,
    at: &Path,
    noted: &mut Run,
) -> Result<(), String> {
    let derivations = checks::read_kept(&checks::keeping(&crate::cache(), base, system))?
        .ok_or("its derivations aren't kept")?;
    let (tree, source) = reads::source(root, base)?;
    // A check whose derivation takes the whole source in goes unrecorded,
    // so every gate evaluates it.
    let whole = checks::taking_whole(&source, &derivations)?;
    let last = recorded(root, base, system).filter(|(head, _)| head != base);
    let (copied, traced_whole) = last
        .as_ref()
        .and_then(|last| copied(root, base, system, &derivations, last))
        .unwrap_or_default();
    let waited = last
        .as_ref()
        .map(|(head, _)| waiting(head, system))
        .unwrap_or_default();
    let traced_whole = Some(traced_whole).filter(|&at| at != 0).unwrap_or_else(now);
    let (mut vm, ordinary): (Vec<&String>, Vec<&String>) = derivations
        .keys()
        .filter(|name| !whole.contains(name.as_str()) && !copied.contains_key(*name))
        .partition(|name| checks::is_vm_test(name));
    // Those a keep left go first, the longest waiting first, then the rest
    // by name.
    vm.sort_by_key(|name| waited.get(name.as_str()).copied().unwrap_or(u64::MAX));
    let started = now();
    let left = |each: &BTreeMap<&str, Reads>| -> BTreeMap<&str, u64> {
        vm.iter()
            .filter(|name| !each.contains_key(name.as_str()))
            .map(|name| {
                let since = waited.get(name.as_str()).copied().unwrap_or(started);
                (name.as_str(), since)
            })
            .collect()
    };
    let copied: BTreeMap<&str, Reads> = copied
        .iter()
        .map(|(name, reads)| (name.as_str(), reads.clone()))
        .collect();
    let tracing = (source.as_str(), system, at);
    // The floor is what every check read, so the record is narrowed whole.
    let gathering = gathered::files(&source, system);
    let ungathered = |each| narrowed(&source, gathering.as_ref(), each);
    let mut evaluation = Run::evaluating("alone");
    let each = if ordinary.is_empty() {
        BTreeMap::new()
    } else {
        let may_begin = || !crate::pressure::memory_stalled();
        traced(
            &ordinary,
            &derivations,
            tracing,
            &mut evaluation,
            (may_begin, || false, |_, _| ()),
        )
    };
    if each.is_empty() && !ordinary.is_empty() {
        noted.evaluated(evaluation);
        return Err("no check's trace could be read".to_owned());
    }
    let mut each = ungathered(each);
    each.extend(copied);
    let record = |each: &BTreeMap<&str, Reads>| {
        written(
            at,
            (&tree, traced_whole, carried),
            &derivations,
            (each, &left(each)),
        )
    };
    record(&each)?;
    if !vm.is_empty() {
        let paced = gating::Paced::beside_gates();
        // Kept after each, so a keep stopped part way loses one trace.
        let vm_each = Mutex::new(BTreeMap::new());
        let keep_each = |name, reads: &Reads| {
            let mut vm_each = vm_each.lock().unwrap_or_else(PoisonError::into_inner);
            vm_each.insert(name, reads.clone());
            let mut whole = each.clone();
            whole.extend(ungathered(vm_each.clone()));
            if let Err(why) = record(&whole) {
                eprintln!("changed-checks: couldn't keep what {name} read alone: {why}");
            }
        };
        traced(
            &vm,
            &derivations,
            tracing,
            &mut evaluation,
            (|| paced.may_begin(), || paced.stops(), keep_each),
        );
        let kept = vm_each
            .into_inner()
            .unwrap_or_else(PoisonError::into_inner)
            .len();
        eprintln!(
            "changed-checks: traced {kept} of the {} VM tests {system} had to trace alone at {}, \
             leaving {} for a later keep",
            vm.len(),
            short(base),
            vm.len() - kept
        );
    }
    noted.evaluated(evaluation);
    Ok(())
}

/// Trace each of `names` alone from `source`'s checks for `system`, its
/// traces and caches named after `at`, noting what it took as a chunk of
/// `evaluation`, and give what each whose derivation is `derivations`'
/// read, handing each to `each_read` as it's traced. One is traced at a
/// time, and more only while `may_begin` says so; none is begun once
/// `stop` says to.
fn traced<'a>(
    names: &[&'a String],
    derivations: &Derivations,
    (source, system, at): (&str, &str, &Path),
    evaluation: &mut Evaluation,
    (may_begin, stop, each_read): (
        impl Fn() -> bool + Sync,
        impl Fn() -> bool + Sync,
        impl Fn(&'a str, &Reads) + Sync,
    ),
) -> BTreeMap<&'a str, Reads> {
    let checks = format!("path:{source}#checks.{system}");
    let strace = reads::strace();
    let numbered: Vec<(usize, &String)> = names.iter().copied().enumerate().collect();
    let began = Instant::now();
    let traced = checks::each_at_once(&numbered, AT_ONCE, may_begin, |&(number, name)| {
        if stop() {
            return Ok(None);
        }
        let trace = at.with_extension(format!("trace{number}"));
        let fresh = at.with_extension(format!("cache{number}"));
        let traced = alone(&checks, &strace, source, &trace, &fresh, name);
        let _ = fs::remove_file(&trace);
        let _ = fs::remove_dir_all(&fresh);
        let reads = traced
            .and_then(|(said, reads)| (derivations.get(name) == Some(&said)).then_some(reads));
        if let Some(reads) = &reads {
            each_read(name.as_str(), reads);
        }
        Ok::<_, String>(reads)
    })
    .unwrap_or_default();
    evaluation.chunk(names.len(), began.elapsed());
    names
        .iter()
        .zip(traced)
        .filter_map(|(name, reads)| Some((name.as_str(), reads?)))
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

/// Keep at `at` the record of what `each` read of `tree`, whole, by rename,
/// saying when every check was last traced alone, how many keeps in a row
/// carried to the derivations, and since when each VM test `left` names has
/// waited to be.
fn written(
    at: &Path,
    (tree, traced_whole, carried): (&str, u64, u64),
    derivations: &Derivations,
    (each, left): (&BTreeMap<&str, Reads>, &BTreeMap<&str, u64>),
) -> Result<(), String> {
    let writing = at.with_extension("writing");
    let failed = |error: std::io::Error| format!("{}: {error}", at.display());
    let mut record = kept(tree, derivations.keys(), each);
    record["whole"] = json!(traced_whole);
    record["carried"] = json!(carried);
    record["waiting"] = json!(left);
    fs::write(&writing, record.to_string()).map_err(failed)?;
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

/// What a base past a recorded head takes from it: the head's derivation
/// of each check main reached nothing of since, and the checks left to
/// evaluate at the base.
pub struct Carry {
    /// The head's derivation of each check main reached nothing of.
    pub known: Derivations,
    /// The base's ordinary checks left to evaluate, and how many it has.
    pub left: Vec<String>,
    pub ordinary: usize,
    /// The base's VM tests left to evaluate, and how many it has.
    pub vm_left: Vec<String>,
    pub vm: usize,
    /// Whether main reached nothing the VM tests read together, so each
    /// keeps the head's derivation.
    pub vm_kept: bool,
}

impl Carry {
    /// The base's derivations, `flake`'s: those carried, and the rest
    /// evaluated.
    pub fn evaluated(
        &self,
        flake: &str,
        system: &str,
        noted: &mut Run,
    ) -> Result<Derivations, String> {
        let mut asked = self.left.clone();
        asked.extend(self.vm_left.iter().cloned());
        let mut evaluation = Run::evaluating("base");
        let mut derivations = if asked.is_empty() {
            Derivations::new()
        } else {
            evaluating::evaluated(flake, system, &asked, &mut evaluation)?
        };
        noted.evaluated(evaluation);
        let wanted: BTreeSet<&String> = asked.iter().collect();
        derivations.retain(|name, _| wanted.contains(name));
        derivations.extend(self.known.clone());
        Ok(derivations)
    }
}

/// What `base` takes from the record of `head`, a head of main it came
/// after, with its derivations `known`: each check main reached nothing of
/// since keeps the head's, the VM tests too when main reached nothing they
/// read, and only the rest are evaluated at the base. Nothing when main's
/// change can't be judged, so the whole base is evaluated.
pub fn carry(
    root: &str,
    base: &str,
    system: &str,
    (head, known): (&str, &Derivations),
) -> Option<Carry> {
    let flake = format!("git+file://{root}?rev={base}");
    let named = evaluating::names(&flake, system).ok()?;
    let (vm, ordinary): (Vec<String>, Vec<String>) = named
        .iter()
        .cloned()
        .partition(|name| checks::is_vm_test(name));
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
    Some(Carry {
        known: known_of(known, unreached.iter().chain(&vm_unreached)).collect(),
        left,
        ordinary: ordinary.len(),
        vm: vm_left.len() + vm_unreached.len(),
        vm_left,
        vm_kept,
    })
}

/// The base's derivations when it came after the recorded head, as `carry`
/// gives them. Nothing when the base is the head, or main's change can't
/// be judged, so the whole base is evaluated.
pub fn past(
    root: &str,
    base: &str,
    system: &str,
    recorded: Option<&(String, Derivations)>,
    noted: &mut Run,
) -> Option<Derivations> {
    let (head, known) = recorded.filter(|(head, _)| head != base)?;
    let carry = carry(root, base, system, (head, known))?;
    let flake = format!("git+file://{root}?rev={base}");
    let derivations = carry.evaluated(&flake, system, noted).ok()?;
    if !carry.vm_kept {
        eprintln!(
            "changed-checks: evaluates {} of {system}'s {} VM tests at the base {}, since main \
             from {} reaches nothing the other {} read",
            carry.vm_left.len(),
            carry.vm,
            short(base),
            short(head),
            carry.vm - carry.vm_left.len()
        );
    }
    eprintln!(
        "changed-checks: evaluates {} of {system}'s {} other checks at the base {}, since main \
         from {} reaches nothing the other {} read",
        carry.left.len(),
        carry.ordinary,
        short(base),
        short(head),
        carry.ordinary - carry.left.len()
    );
    Some(derivations)
}

/// A commit by its first 12 characters, as changed-checks names one.
fn short(commit: &str) -> &str {
    commit.get(..12).unwrap_or(commit)
}
