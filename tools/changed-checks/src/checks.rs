//! What a change did to each check's derivation, and how the checks are
//! evaluated and kept to find out.

use std::collections::BTreeMap;
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

/// How many chunks are evaluated at once. Measured on core's 305 checks, a
/// comparison's evaluations kept about 1.6 of the machine's 16 cores busy
/// and peaked at 5.8 GB one at a time, so two at once take about half as
/// long and hold about twice that, well short of the 24 GB that evaluating
/// every check in one held.
pub const AT_ONCE: usize = 2;

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
            .filter(|name| vm || !name.starts_with("vm-"))
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

/// Split the names, in order, into chunks weighing at most `WEIGHT` each.
pub fn chunks(names: &[String]) -> Vec<&[String]> {
    let weight = |name: &String| if name.starts_with("vm-") { VM_TEST } else { 1 };
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

/// Do `work` on each item, `at_once` at a time, answering in the items'
/// order. Once one fails, no item not yet begun is begun, since the first
/// failure in order is what the run reports.
pub fn each_at_once<T, R, E>(
    items: &[T],
    at_once: usize,
    work: impl Fn(&T) -> Result<R, E> + Sync,
) -> Result<Vec<R>, E>
where
    T: Sync,
    R: Send,
    E: Send,
{
    let next = AtomicUsize::new(0);
    let failed = AtomicBool::new(false);
    let done: Mutex<Vec<Option<Result<R, E>>>> = Mutex::new(items.iter().map(|_| None).collect());
    thread::scope(|scope| {
        for _ in 0..at_once.clamp(1, items.len().max(1)) {
            scope.spawn(|| {
                while !failed.load(Ordering::Relaxed) {
                    let at = next.fetch_add(1, Ordering::Relaxed);
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

/// Give the derivations kept at the path, or none when nothing is kept.
fn read_kept(path: &Path) -> Result<Option<Derivations>, String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn derivations(pairs: &[(&str, Option<&str>)]) -> Derivations {
        pairs
            .iter()
            .map(|(name, path)| ((*name).to_owned(), path.map(str::to_owned)))
            .collect()
    }

    fn names(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    /// A folder of the test's own, emptied first.
    fn folder(test: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("changed-checks-{}-{test}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        folder
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
    fn work_done_at_once_is_answered_in_its_items_order() {
        let items: Vec<usize> = (0..7).collect();
        let answered = each_at_once(&items, AT_ONCE, |item| Ok::<_, ()>(item * 10));
        assert_eq!(answered, Ok(vec![0, 10, 20, 30, 40, 50, 60]));
    }

    #[test]
    fn the_first_failure_in_order_is_answered() {
        let items: Vec<usize> = (0..7).collect();
        let answered = each_at_once(
            &items,
            1,
            |item| {
                if *item >= 3 { Err(*item) } else { Ok(*item) }
            },
        );
        assert_eq!(answered, Err(3));
    }

    #[test]
    fn no_work_is_no_answers() {
        let answered = each_at_once(&[] as &[usize], AT_ONCE, |item| Ok::<_, ()>(*item));
        assert_eq!(answered, Ok(vec![]));
    }

    #[test]
    fn no_checks_need_no_evaluation() {
        assert!(chunks(&[]).is_empty());
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
        let path = folder("first").join("kept").join("abc-x86_64-linux.json");
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
        let path = folder("later").join("abc-x86_64-linux.json");
        kept(&path, || Ok(derivations(&[("a", Some("a.drv"))]))).expect("kept");
        let found = kept(&path, || panic!("it evaluated again"));
        assert_eq!(found, Ok(derivations(&[("a", Some("a.drv"))])));
    }

    #[test]
    fn runs_wanting_one_commit_at_once_evaluate_it_once() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let path = folder("at-once").join("abc-x86_64-linux.json");
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
        let path = folder("failed").join("abc-x86_64-linux.json");
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
