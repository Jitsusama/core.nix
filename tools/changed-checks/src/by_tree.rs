//! A commit's derivations kept by its tree too, so a commit holding a tree
//! some run already evaluated at another commit evaluates nothing.
//!
//! The merge train evaluates its candidate, a merge of the riders onto
//! main, and then lands it on main as another commit holding the same
//! tree. The next candidate's base is that commit, which nothing had kept,
//! so the train's next run evaluated it again: on 28 November each of the
//! last three landings, d224e9e85e80, 53c193275324 and 79b8e1087fb9, held
//! the tree of the candidate just evaluated. The run after d224e9e85e80
//! spent 408 s of its 2556 evaluating that base's 1279 checks, and the run
//! after 53c193275324 spent 508 of its 1222 s outside its own evaluating
//! while a keep of main, begun with it, evaluated that base for 518 s.
//!
//! A derivation is made from the files the flake reads, never from the
//! commit naming them: no check reads the flake's own `rev`,
//! `lastModified` or the like, as the records of what each check read
//! (`alone.rs`) already take for granted. So the derivations kept at one
//! commit are the derivations of every commit holding its tree.

use std::fs;
use std::path::{Path, PathBuf};

use crate::checks::{self, Derivations};

/// Where the derivations of a tree's checks for a system are kept, under
/// the cache folder given, beside those kept by commit.
pub fn keeping(cache: &Path, tree: &str, system: &str) -> PathBuf {
    cache
        .join("changed-checks/trees")
        .join(format!("{tree}-{system}.json"))
}

/// Give `commit`'s derivations kept by it or by its tree, or evaluate and
/// keep them by both, as `checks::kept` does by commit alone.
pub fn kept(
    root: &str,
    commit: &str,
    system: &str,
    evaluate: impl FnOnce() -> Result<Derivations, String>,
) -> Result<Derivations, String> {
    let tree = tree_of(root, commit).ok();
    kept_in(&crate::cache(), (commit, tree.as_deref()), system, evaluate)
}

/// Keep `commit`'s derivations by its tree too, once they're kept by it.
pub fn index(root: &str, commit: &str, system: &str) {
    let cache = crate::cache();
    let kept = checks::read_kept(&checks::keeping(&cache, commit, system));
    if let (Ok(tree), Ok(Some(found))) = (tree_of(root, commit), kept) {
        indexed(&cache, &tree, system, &found);
    }
}

/// Keep `commit`'s derivations by it from those kept by its tree, when
/// they're kept by its tree alone, so whatever reads them by commit, as a
/// keep of main does, finds them.
pub fn seed(root: &str, commit: &str, system: &str) {
    if let Ok(tree) = tree_of(root, commit) {
        seeded(&crate::cache(), commit, &tree, system);
    }
}

/// The tree `commit` holds, as git gives it when asked to verify it: one
/// object's name or a failure, so a commit it can't find keeps nothing by
/// a tree.
fn tree_of(root: &str, commit: &str) -> Result<String, String> {
    let tree = format!("{commit}^{{tree}}");
    crate::run("git", &["-C", root, "rev-parse", "--verify", &tree])
}

/// `kept` under the cache folder given, for `commit` holding `tree` when
/// git could say.
fn kept_in(
    cache: &Path,
    (commit, tree): (&str, Option<&str>),
    system: &str,
    evaluate: impl FnOnce() -> Result<Derivations, String>,
) -> Result<Derivations, String> {
    if let Some(tree) = tree {
        seeded(cache, commit, tree, system);
    }
    let found = checks::kept(&checks::keeping(cache, commit, system), evaluate)?;
    if let Some(tree) = tree {
        indexed(cache, tree, system, &found);
    }
    Ok(found)
}

/// Keep `commit`'s derivations by it from those kept by `tree`, unless
/// they're kept by it already or `tree`'s aren't, saying so when it does.
/// One that can't be kept is said and left for the run to evaluate.
fn seeded(cache: &Path, commit: &str, tree: &str, system: &str) -> bool {
    let path = checks::keeping(cache, commit, system);
    if path.exists() {
        return false;
    }
    let found = match checks::read_kept(&keeping(cache, tree, system)) {
        Ok(Some(found)) => found,
        Ok(None) => return false,
        Err(why) => {
            eprintln!("changed-checks: couldn't read what was kept by a tree: {why}");
            return false;
        }
    };
    match written(&path, &found) {
        Ok(()) => {
            eprintln!(
                "changed-checks: {} holds the tree {} a run evaluated at another commit, so \
                 it takes that one's derivations and evaluates none",
                short(commit),
                short(tree)
            );
            true
        }
        Err(why) => {
            eprintln!("changed-checks: couldn't keep what a tree had kept: {why}");
            false
        }
    }
}

/// Keep `found` by `tree` unless they're kept by it already, saying why
/// when they can't be: a later commit holding it then evaluates them.
fn indexed(cache: &Path, tree: &str, system: &str, found: &Derivations) {
    let path = keeping(cache, tree, system);
    if path.exists() {
        return;
    }
    if let Err(why) = written(&path, found) {
        eprintln!("changed-checks: couldn't keep the derivations by their tree: {why}");
    }
}

/// Write `found` at `path` whole, beside it first and moved onto it, so a
/// run stopped halfway leaves nothing rather than half.
fn written(path: &Path, found: &Derivations) -> Result<(), String> {
    let failed = |error: &dyn std::fmt::Display| format!("{}: {error}", path.display());
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder).map_err(|error| failed(&error))?;
    }
    let text = serde_json::to_string(found).map_err(|error| failed(&error))?;
    let writing = path.with_extension(format!("writing-{}", std::process::id()));
    fs::write(&writing, text).map_err(|error| failed(&error))?;
    fs::rename(&writing, path).map_err(|error| failed(&error))
}

/// A commit or tree, shortened as git would show it.
fn short(name: &str) -> &str {
    name.get(..12).unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scratch::Scratch;

    fn derivations(pairs: &[(&str, &str)]) -> Derivations {
        pairs
            .iter()
            .map(|&(name, drv)| (name.to_owned(), Some(drv.to_owned())))
            .collect()
    }

    #[test]
    fn a_commit_holding_a_tree_kept_at_another_evaluates_nothing() {
        let cache = Scratch::new("by-tree-landed");
        let candidate = derivations(&[("a", "a.drv"), ("vm-b", "b.drv")]);
        let first = kept_in(&cache, ("candidate", Some("t1")), "x", || {
            Ok(candidate.clone())
        });
        assert_eq!(first, Ok(candidate.clone()));
        let landed = kept_in(&cache, ("landed", Some("t1")), "x", || {
            panic!("the landed commit was evaluated again")
        });
        assert_eq!(landed, Ok(candidate));
        assert!(
            checks::keeping(&cache, "landed", "x").exists(),
            "the landed commit's derivations weren't kept by it, where a keep reads them"
        );
    }

    #[test]
    fn a_commit_holding_another_tree_is_evaluated() {
        let cache = Scratch::new("by-tree-other");
        kept_in(&cache, ("one", Some("t1")), "x", || {
            Ok(derivations(&[("a", "a.drv")]))
        })
        .expect("kept");
        let other = kept_in(&cache, ("two", Some("t2")), "x", || {
            Ok(derivations(&[("a", "a2.drv")]))
        });
        assert_eq!(other, Ok(derivations(&[("a", "a2.drv")])));
    }

    #[test]
    fn a_tree_is_kept_for_each_system_apart() {
        let cache = Scratch::new("by-tree-system");
        kept_in(&cache, ("one", Some("t1")), "x", || {
            Ok(derivations(&[("a", "a.drv")]))
        })
        .expect("kept");
        let other = kept_in(&cache, ("two", Some("t1")), "y", || {
            Ok(derivations(&[("a", "a-y.drv")]))
        });
        assert_eq!(other, Ok(derivations(&[("a", "a-y.drv")])));
    }

    #[test]
    fn a_commit_kept_already_keeps_its_own() {
        let cache = Scratch::new("by-tree-own");
        let own = derivations(&[("a", "own.drv")]);
        written(&checks::keeping(&cache, "c", "x"), &own).expect("written");
        written(&keeping(&cache, "t1", "x"), &derivations(&[("a", "t.drv")])).expect("written");
        assert!(!seeded(&cache, "c", "t1", "x"));
        assert_eq!(
            checks::read_kept(&checks::keeping(&cache, "c", "x")),
            Ok(Some(own))
        );
    }

    #[test]
    fn a_commit_git_gave_no_tree_for_is_kept_by_commit_alone() {
        let cache = Scratch::new("by-tree-none");
        let found = kept_in(&cache, ("c", None), "x", || {
            Ok(derivations(&[("a", "a.drv")]))
        });
        assert_eq!(found, Ok(derivations(&[("a", "a.drv")])));
        assert!(checks::keeping(&cache, "c", "x").exists());
        assert!(!cache.join("changed-checks/trees").exists());
    }

    #[test]
    fn a_failed_evaluation_keeps_nothing_by_its_tree() {
        let cache = Scratch::new("by-tree-failed");
        let failed = kept_in(&cache, ("c", Some("t1")), "x", || Err("no".to_owned()));
        assert_eq!(failed, Err("no".to_owned()));
        assert!(!keeping(&cache, "t1", "x").exists());
    }
}
