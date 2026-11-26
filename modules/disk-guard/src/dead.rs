//! Which store paths are dead, as Nix's garbage collector would call
//! them, worked out without its lock where Nix's settings let a closure
//! say so, and asked of the collector itself where they don't.

use crate::garbage::{CHUNK, LIST_TIMEOUT, NIX_COMMAND, ran};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::Path;
use std::time::Duration;

/// Every dead path in the store at `store`, as `nix` and `nix-store`
/// beside it say, with `idle` at idle priority: without the lock where
/// Nix's `keep-outputs` is off (`unlocked_dead`), and through Nix's own
/// `--print-dead` where it's on or Nix can't say. Nix's own temporary
/// files in the store's folder may be among them.
///
/// # Errors
///
/// When the store's folder can't be read or Nix fails a question.
pub fn dead(nix: &Path, store: &Path, idle: bool) -> Result<Vec<String>, String> {
    if said_off(nix, "keep-outputs", idle) {
        let derivers = !said_off(nix, "keep-derivations", idle);
        unlocked_dead(nix, store, derivers, idle)
    } else {
        // Said, since builds then wait on the lock for as long as it lists.
        say!("disk-guard: listing under Nix's lock, since keep-outputs is on or Nix couldn't say");
        locked_dead(nix, idle)
    }
}

/// Whether Nix says its `setting` is off. With `keep-outputs` it keeps
/// every output of a live derivation, even one no longer built from it,
/// which only its own collector follows; with `keep-derivations`, its
/// default, the derivation that built a live path. Where Nix can't say,
/// as though it were on, which keeps the more.
fn said_off(nix: &Path, setting: &str, idle: bool) -> bool {
    let argv: Vec<&OsStr> = std::iter::once(nix.as_os_str())
        .chain(NIX_COMMAND.iter().map(OsStr::new))
        .chain(["config", "show", setting].map(OsStr::new))
        .collect();
    ran(&argv, Duration::from_secs(30), idle).is_ok_and(|said| said.trim() == "false")
}

/// The dead as Nix's own collector lists them, under its lock.
fn locked_dead(nix: &Path, idle: bool) -> Result<Vec<String>, String> {
    let store = nix.with_file_name("nix-store");
    let listed = ran(
        &[
            store.as_os_str(),
            "--option".as_ref(),
            "experimental-features".as_ref(),
            "".as_ref(),
            "--gc".as_ref(),
            "--print-dead".as_ref(),
        ],
        LIST_TIMEOUT,
        idle,
    )?;
    Ok(listed.lines().map(str::to_owned).collect())
}

/// The dead worked out with no lock: every entry in the store at `store`,
/// read first, less what Nix's roots keep, read after. The roots come
/// from `nix-store --gc --print-roots`, which reads them as the collector
/// does, its temporary and running processes' roots included, without
/// its lock; what they keep is their closure, from `nix-store --query
/// --requisites`, and with `derivers` the derivation Nix recorded as
/// building each path kept, and its closure in turn, as Nix's collector
/// keeps them under `keep-derivations`. Reading the entries first means
/// a path added between the two is either rooted by then or not listed.
/// A reading that names no root in the store fails the listing, since
/// every entry would read dead: a running machine always has its own.
fn unlocked_dead(
    nix: &Path,
    store: &Path,
    derivers: bool,
    idle: bool,
) -> Result<Vec<String>, String> {
    let mut entries: Vec<String> = std::fs::read_dir(store)
        .map_err(|error| format!("reading {}: {error}", store.display()))?
        .filter_map(Result::ok)
        .filter_map(|entry| entry.path().into_os_string().into_string().ok())
        .collect();
    entries.sort_unstable();
    let nix_store = nix.with_file_name("nix-store");
    let asked = |words: &[&str], paths: &[&String]| -> Result<String, String> {
        let argv: Vec<&OsStr> = std::iter::once(nix_store.as_os_str())
            .chain(words.iter().map(OsStr::new))
            .chain(paths.iter().map(OsStr::new))
            .collect();
        ran(&argv, LIST_TIMEOUT, idle)
    };
    let roots: HashSet<String> = asked(&["--gc", "--print-roots"], &[])?
        .lines()
        .filter_map(|line| line.rsplit_once(" -> "))
        .filter_map(|(_, path)| store_path(store, path))
        .collect();
    // Only a root among what was read counts: one naming a path already
    // collected, or one still building, keeps none of it.
    let rooted = roots.iter().any(|root| entries.binary_search(root).is_ok());
    if !rooted && !entries.is_empty() {
        return Err(format!(
            "nix-store --gc --print-roots named no root in {}, so all of it would read dead",
            store.display()
        ));
    }
    let mut live: HashSet<String> = roots.clone();
    let mut asking: Vec<String> = roots.into_iter().collect();
    asking.sort_unstable();
    // Each path kept is asked for its deriver once, and each deriver kept
    // for what it keeps in turn, until a round finds nothing new.
    let mut walked: HashSet<String> = HashSet::new();
    loop {
        let mut fresh: Vec<String> = Vec::new();
        let asking_refs: Vec<&String> = asking.iter().collect();
        for chunk in asking_refs.chunks(CHUNK) {
            for kept in asked_of_valid(&asked, &["--query", "--requisites"], chunk)? {
                if walked.insert(kept.clone()) {
                    fresh.push(kept.clone());
                }
                live.insert(kept);
            }
        }
        if !derivers || fresh.is_empty() {
            break;
        }
        fresh.sort_unstable();
        let fresh_refs: Vec<&String> = fresh.iter().collect();
        let mut next: Vec<String> = Vec::new();
        for chunk in fresh_refs.chunks(CHUNK) {
            // A path with none recorded is said `unknown-deriver`.
            next.extend(
                asked_of_valid(&asked, &["--query", "--deriver"], chunk)?
                    .iter()
                    .filter(|line| line.starts_with('/') && !walked.contains(*line))
                    .cloned(),
            );
        }
        next.sort_unstable();
        next.dedup();
        if next.is_empty() {
            break;
        }
        asking = next;
    }
    Ok(entries
        .into_iter()
        .filter(|entry| !live.contains(entry))
        .collect())
}

/// A question put to `nix-store`: its words, then the paths it's about.
type Asked<'a> = dyn Fn(&[&str], &[&String]) -> Result<String, String> + 'a;

/// What `nix-store` answers `question` of the valid paths among `paths`,
/// through `asked`, a line each. Nix refuses a whole question naming a
/// path not valid: a root a build or a fetch is still making, which keeps
/// only itself, or a path a collection deleted since it was found, which
/// no lock now stops. One gone between asking which are valid and the
/// question itself is dropped as the question is asked once more.
fn asked_of_valid(
    asked: &Asked,
    question: &[&str],
    paths: &[&String],
) -> Result<Vec<String>, String> {
    let mut tried = 0;
    loop {
        let invalid: HashSet<String> = asked(&["--check-validity", "--print-invalid"], paths)?
            .lines()
            .map(str::to_owned)
            .collect();
        let valid: Vec<&String> = paths
            .iter()
            .filter(|path| !invalid.contains(path.as_str()))
            .copied()
            .collect();
        if valid.is_empty() {
            return Ok(Vec::new());
        }
        match asked(question, &valid) {
            Ok(kept) => return Ok(kept.lines().map(str::to_owned).collect()),
            Err(_) if tried == 0 => tried += 1,
            Err(why) => return Err(why),
        }
    }
}

/// The store path `path` lies in under `store`, as a root may name a file
/// within one; None for a path outside the store.
fn store_path(store: &Path, path: &str) -> Option<String> {
    let within = Path::new(path).strip_prefix(store).ok()?;
    let name = within.components().next()?;
    store.join(name).into_os_string().into_string().ok()
}
