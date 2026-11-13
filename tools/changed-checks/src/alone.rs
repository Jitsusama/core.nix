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

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use serde_json::{Map, Value, json};

use crate::checks::{self, Derivations};
use crate::evaluating;
use crate::gathered;
use crate::reads::{self, Reads};
use crate::runs::Run;

/// How many checks are traced at once, each in a process of its own, far
/// lighter than a chunk's.
const AT_ONCE: usize = 16;

/// How long a record of what each check read stands in for every keep
/// after it. Tracing each check alone costs a keep about 400 s, which on
/// every keep would put kept heads 12 to 17 minutes apart, where 3 of 12
/// November's last 13 gates already found no recent kept main; recorded
/// once in this long, every other keep stays as quick as before.
const STANDS_FOR: Duration = Duration::from_mins(20);

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
/// read only because checks.nix gathers them all.
fn record(root: &str, base: &str, system: &str, at: &Path, noted: &mut Run) -> Result<(), String> {
    let derivations = checks::read_kept(&checks::keeping(&crate::cache(), base, system))?
        .ok_or("its derivations aren't kept")?;
    let (tree, source) = reads::source(root, base)?;
    // A check whose derivation takes the whole source in goes unrecorded,
    // so every gate evaluates it.
    let whole = checks::taking_whole(&source, &derivations)?;
    let ordinary: Vec<(usize, &String)> = derivations
        .keys()
        .filter(|name| !name.starts_with("vm-") && !whole.contains(name.as_str()))
        .enumerate()
        .collect();
    let checks = format!("path:{source}#checks.{system}");
    let strace = reads::strace();
    let mut evaluation = Run::evaluating("alone");
    let began = Instant::now();
    let traced = checks::each_at_once(
        &ordinary,
        AT_ONCE,
        || !crate::pressure::memory_stalled(),
        |(number, name)| {
            let trace = at.with_extension(format!("trace{number}"));
            let fresh = at.with_extension(format!("cache{number}"));
            let traced = alone(&checks, &strace, &source, &trace, &fresh, name);
            let _ = fs::remove_file(&trace);
            let _ = fs::remove_dir_all(&fresh);
            Ok::<_, String>(traced)
        },
    )?;
    evaluation.chunk(ordinary.len(), began.elapsed());
    noted.evaluated(evaluation);
    let mut each: BTreeMap<&str, Reads> = ordinary
        .iter()
        .zip(traced)
        .filter_map(|((_, name), traced)| {
            let (said, reads) = traced?;
            (derivations.get(*name) == Some(&said)).then_some((name.as_str(), reads))
        })
        .collect();
    if each.is_empty() {
        return Err("no check's trace could be read".to_owned());
    }
    if let Some(files) = gathered::files(&source, system) {
        gathered::narrow(Path::new(&source), &files, &mut each);
    }
    let writing = at.with_extension("writing");
    let failed = |error: std::io::Error| format!("{}: {error}", at.display());
    fs::write(&writing, kept(&tree, derivations.keys(), &each).to_string()).map_err(failed)?;
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
    let reads = Reads::traced(&fs::read_to_string(trace).ok()?, source)?;
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
    ordinary: Vec<String>,
) -> (Derivations, Vec<String>) {
    let Some((head, known)) = recorded else {
        return (Derivations::new(), ordinary);
    };
    let Some((unreached, left)) = partitioned(root, (head, known), None, system, &ordinary) else {
        return (Derivations::new(), ordinary);
    };
    if !unreached.is_empty() {
        eprintln!(
            "changed-checks: evaluates {} of {system}'s {} other checks, since the change \
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
    let mut asked = left.clone();
    if !vm_kept {
        asked.extend(vm.iter().cloned());
    }
    let mut evaluation = Run::evaluating("base");
    let mut derivations = if asked.is_empty() {
        Derivations::new()
    } else {
        evaluating::evaluated(&flake, system, &asked, &mut evaluation).ok()?
    };
    noted.evaluated(evaluation);
    let wanted: BTreeSet<&String> = asked.iter().collect();
    derivations.retain(|name, _| wanted.contains(name));
    let vm_too = vm.iter().filter(|_| vm_kept);
    derivations.extend(known_of(known, unreached.iter().chain(vm_too)));
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
