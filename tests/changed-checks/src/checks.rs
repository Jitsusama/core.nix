//! What a change did to each check's derivation, and how the checks are
//! evaluated and kept to find out.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Each check's derivation path by its name, or nothing for a check that
/// doesn't evaluate.
pub type Derivations = BTreeMap<String, Option<String>>;

/// How many checks one nix eval takes. Nix's evaluator never gives memory
/// back, so one holds everything its checks share and each machine they
/// build: a quarter of core's 231 checks held 11.3 GB at their peak and
/// took 77 s at a load of 59, and every check at once about 24 GB.
pub const CHUNK: usize = 40;

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
/// halfway keeps nothing rather than half.
pub fn kept(
    path: &Path,
    evaluate: impl FnOnce() -> Result<Derivations, String>,
) -> Result<Derivations, String> {
    let unreadable = |error: &dyn std::fmt::Display| format!("{}: {error}", path.display());
    match fs::read_to_string(path) {
        Ok(text) => return serde_json::from_str(&text).map_err(|error| unreadable(&error)),
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            return Err(unreadable(&error));
        }
        Err(_) => {}
    }
    let found = evaluate()?;
    let writing = path.with_extension("writing");
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder).map_err(|error| unreadable(&error))?;
    }
    let text = serde_json::to_string(&found).map_err(|error| unreadable(&error))?;
    fs::write(&writing, text).map_err(|error| unreadable(&error))?;
    fs::rename(&writing, path).map_err(|error| unreadable(&error))?;
    Ok(found)
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
        let chunks: Vec<&[String]> = all.chunks(CHUNK).collect();
        assert_eq!(
            chunks.iter().map(|chunk| chunk.len()).collect::<Vec<_>>(),
            [40, 40, 5]
        );
        assert_eq!(chunks.concat(), all);
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
