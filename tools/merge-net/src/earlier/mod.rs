//! What the earlier commits of a head's change say for it, when the head
//! has no run of its own: `ready` merges a rebased head on one, and
//! `needs` reads what one named moved rather than evaluating again. Beside
//! it, whether a review round read a head or an earlier commit of it
//! (`same`), what the harness's rounds say they read (`seen`), and which
//! heads only pin repos those rounds already review (`pins`).

mod pins;
pub mod same;
mod seen;

use std::cell::LazyCell;
use std::path::Path;
use std::process::Command;

use crate_reach::Crates;

use crate::{candidates, git, gone, lines, patch_id, ready, rules, stands};

/// The record of the run that passed at an earlier commit of `commit`'s
/// change and speaks for it on `main` as `ready` takes one, since main
/// hasn't moved its files or the crates beside them, so `needs` at a
/// rebased head reads what that run named rather than evaluating again.
pub fn pass(commit: &str, main: &str) -> Option<String> {
    let on_main = |base: &str| {
        Command::new("git")
            .args(["merge-base", "--is-ancestor", base, main])
            .status()
            .is_ok_and(|status| status.success())
    };
    let before = judged(commit, main, &on_main).ok()?.before;
    Some(ready::record(before.speaker()?))
}

/// What `ready` knows of a head's change from its earlier commits: where
/// the head stands, whether it's behind main, its files, and which earlier
/// run speaks for it.
pub struct Judged {
    pub stands: String,
    pub behind: bool,
    pub files: Vec<String>,
    pub before: ready::Before,
}

/// What `ready` knows of `commit`, a head on `main` with no run of its
/// own, from the earlier commits of its change, `on_main` saying whether
/// a run's base is on main.
pub fn judged(commit: &str, main: &str, on_main: &dyn Fn(&str) -> bool) -> Result<Judged, String> {
    // The change is what the head adds to the main it branched from, which
    // is main itself unless the head is behind it.
    let branched = git(&["merge-base", main, commit])?;
    let behind = branched != main;
    let stands = stands(commit, main, behind);
    let files = lines(&git(&["diff", "--name-only", &branched, commit])?);
    let renamed = gone(&branched, commit)?;
    let at = |rev: &str, file: &str| git(&["show", &format!("{rev}:{file}")]).unwrap_or_default();
    let at_head = |file: &str| at(commit, file);
    let at_main = |file: &str| at(main, file);
    let change = patch_id(Path::new("."), &branched, commit);
    let base_of = |candidate: &str| git(&["merge-base", candidate, main]).ok();
    // What main did since `base`: the files it changed and the paths it
    // took away.
    let since = |base: &str| {
        let since = lines(&git(&["diff", "--name-only", base, main]).ok()?);
        let taken_away = gone(base, main).ok()?;
        Some((since, taken_away))
    };
    let id_of = |base: &str, candidate: &str| patch_id(Path::new("."), base, candidate);
    // What main moved in a crate the change's crates are in, read or are
    // read by, no run of an earlier base speaks for.
    let crates = Crates::at(Path::new("."), commit);
    // What main moved in a folder one check measures with the change's
    // (`fates.txt`), which the change's own files never show, with those
    // its merge base names, so it can't judge itself by fewer.
    let fates = rules::fates::judging(&at(&branched, rules::fates::FATES_TXT))?;
    let branch = ready::Side {
        changed: &files,
        gone: &renamed,
        text: &at_head,
    };
    // A head behind main is judged apart by what merging it makes of each
    // file, as a rebased head's own files are: its own lack main's moves,
    // so on 16 November 42 heads behind main were sent to build a check
    // for each description main added beside theirs.
    // It merges only once a shared file is judged.
    let merged = LazyCell::new(|| {
        behind
            .then(|| merged_tree(Path::new("."), main, commit))
            .flatten()
    });
    let at_merged = |file: &str| {
        merged
            .as_deref()
            .map_or_else(|| at_head(file), |tree| at(tree, file))
    };
    let rebased = ready::Side {
        changed: &files,
        gone: &renamed,
        text: &at_merged,
    };
    let shared_since = |candidate: &str, base: String| {
        let (since, taken_away) = since(&base)?;
        let main_side = ready::Side {
            changed: &since,
            gone: &taken_away,
            text: &at_main,
        };
        let mut shared = ready::shared(&branch, &main_side);
        let (at_base, checked) = (|f: &str| at(&base, f), |f: &str| at(candidate, f));
        shared.retain(|file| !ready::stands_apart(file, &rebased, &main_side, &at_base, &checked));
        let mut beside = crates.beside(&files, &since);
        for fated in rules::fates::beside(&fates, &branch, &main_side) {
            if !beside.contains(&fated) {
                beside.push(fated);
            }
        }
        shared.retain(|file| !beside.contains(file));
        shared.extend(beside);
        Some(shared)
    };
    let same = |candidate: &str| {
        ready::when_same(candidate, change.as_deref()?, base_of, id_of, |base| {
            shared_since(candidate, base)
        })
    };
    let candidates = ready::speakers(
        commit,
        behind,
        &candidates(commit, main, &files, &ready::recent_records())?,
    );
    let before = ready::checked_before(&candidates, &ready::record, on_main, &same);
    Ok(Judged {
        stands,
        behind,
        files,
        before,
    })
}

/// The tree merging `commit` into `main` in the repository at `folder`
/// makes, which a head behind main lands as; none where they conflict.
fn merged_tree(folder: &Path, main: &str, commit: &str) -> Option<String> {
    let merged = Command::new("git")
        .arg("-C")
        .arg(folder)
        .args(["merge-tree", "--write-tree", main, commit])
        .output()
        .ok()?;
    // A conflict exits 1 and writes a tree with its markers in, which no
    // file of the change would land as. Any other failure reads the same,
    // the head's own copy, and is said, since it looks like a clash.
    if !matches!(merged.status.code(), Some(0 | 1)) {
        eprintln!(
            "merge-net: merging {commit} into {main} to judge it apart failed: {}",
            String::from_utf8_lossy(&merged.stderr).trim()
        );
    }
    let said = String::from_utf8_lossy(&merged.stdout);
    let tree = said.lines().next().unwrap_or_default();
    (merged.status.success() && !tree.is_empty()).then(|| tree.to_owned())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::tests::git_in;

    /// A descriptions file of an entry for each name, each saying "proves".
    fn descriptions(names: &[&str]) -> String {
        let body: String = names
            .iter()
            .flat_map(|name| ["  ", name, " = ''\n    proves\n  '';\n"])
            .collect();
        format!("{{\n{body}}}\n")
    }

    #[test]
    fn a_description_beside_mains_stands_apart_as_merged_not_as_its_own() {
        // 16 November: 42 heads behind main whose only shared file was
        // check-descriptions.nix were sent to build a list's check again.
        let file = "check-descriptions.nix".to_owned();
        let (files, none) = (vec![file.clone()], Vec::new());
        let base = |_: &str| descriptions(&["b", "d", "f"]);
        let own = |_: &str| descriptions(&["b", "d", "e", "f"]);
        let merged = |_: &str| descriptions(&["a", "b", "d", "e", "f"]);
        let main_text = |_: &str| descriptions(&["a", "b", "d", "f"]);
        let main = ready::Side {
            changed: &files,
            gone: &none,
            text: &main_text,
        };
        let as_merged = ready::Side {
            changed: &files,
            gone: &none,
            text: &merged,
        };
        let as_its_own = ready::Side {
            changed: &files,
            gone: &none,
            text: &own,
        };
        assert!(
            ready::stands_apart(&file, &as_merged, &main, &base, &own),
            "a description merged beside main's didn't stand apart from it"
        );
        assert!(
            !ready::stands_apart(&file, &as_its_own, &main, &base, &own),
            "a head's own copy, lacking main's entry, stood apart from it"
        );
    }

    #[test]
    fn a_head_behind_main_is_read_as_merged_into_it() {
        let repo = std::env::temp_dir().join(format!("merge-net-behind-{}", std::process::id()));
        let _ = fs::remove_dir_all(&repo);
        fs::create_dir_all(&repo).unwrap();
        git_in(&repo, &["init", "-q", "-b", "main"]);
        let entry = |name: &str, says: &str| format!("  {name} = ''\n    {says}\n  '';\n");
        let commit = |entries: &[(&str, &str)]| {
            let body: String = entries
                .iter()
                .map(|(name, says)| entry(name, says))
                .collect();
            fs::write(repo.join("d.nix"), format!("{{\n{body}}}\n")).unwrap();
            git_in(&repo, &["add", "d.nix"]);
            git_in(&repo, &["commit", "-qm", "d"]);
            git_in(&repo, &["rev-parse", "HEAD"])
        };
        let base = commit(&[("b", "proves"), ("d", "proves"), ("f", "proves")]);
        let head = commit(&[
            ("b", "proves"),
            ("d", "proves"),
            ("e", "proves"),
            ("f", "proves"),
        ]);
        git_in(&repo, &["checkout", "-q", &base]);
        let main = commit(&[
            ("a", "proves"),
            ("b", "proves"),
            ("d", "proves"),
            ("f", "proves"),
        ]);
        let merged = merged_tree(&repo, &main, &head);
        let said = merged.map(|tree| git_in(&repo, &["show", &format!("{tree}:d.nix")]));
        let both = ["a", "b", "d", "e", "f"]
            .map(|name| entry(name, "proves"))
            .concat();
        assert_eq!(
            said.as_deref(),
            Some(format!("{{\n{both}}}").as_str()),
            "a head behind main wasn't read as merged into it"
        );
        let clash = commit(&[
            ("a", "proves"),
            ("b", "proves"),
            ("d", "holds"),
            ("f", "proves"),
        ]);
        git_in(&repo, &["checkout", "-q", &head]);
        let other = commit(&[
            ("b", "proves"),
            ("d", "fails"),
            ("e", "proves"),
            ("f", "proves"),
        ]);
        assert_eq!(
            merged_tree(&repo, &clash, &other),
            None,
            "a head conflicting with main was read as merged into it"
        );
        fs::remove_dir_all(&repo).unwrap();
    }
}
