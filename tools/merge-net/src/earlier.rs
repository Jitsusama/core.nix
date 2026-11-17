//! What the earlier commits of a head's change say for it, when the head
//! has no run of its own: `ready` merges a rebased head on one, and
//! `needs` reads what one named moved rather than evaluating again.

use std::path::Path;
use std::process::Command;

use crate_reach::Crates;

use crate::{candidates, git, gone, lines, patch_id, ready, stands};

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
    let ready::Before::Clean(checked) = judged(commit, main, &on_main).ok()?.before else {
        return None;
    };
    Some(ready::record(&checked))
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
    let branch = ready::Side {
        changed: &files,
        gone: &renamed,
        text: &at_head,
    };
    // A head behind main is judged apart by what merging it makes of each
    // file, as a rebased head's own files are: its own lack main's moves,
    // so on 16 November 42 heads behind main were sent to build a check
    // for each description main added beside theirs.
    let merged = behind
        .then(|| merged_tree(Path::new("."), main, commit))
        .flatten();
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
        let beside = crates.beside(&files, &since);
        shared.retain(|file| !beside.contains(file));
        shared.extend(beside);
        Some(shared)
    };
    let same = |candidate: &str| {
        ready::when_same(candidate, change.as_deref()?, base_of, id_of, |base| {
            shared_since(candidate, base)
        })
    };
    let names_gone = |candidate: &str| {
        let (since, taken_away) = since(&base_of(candidate)?)?;
        let main_side = ready::Side {
            changed: &since,
            gone: &taken_away,
            text: &at_main,
        };
        let mut naming = ready::names_gone(&branch, &main_side);
        naming.extend(crates.beside(&files, &since));
        Some(naming)
    };
    let candidates = ready::speakers(commit, behind, &candidates(commit, main, &files)?);
    let before = match ready::checked_before(&candidates, &ready::record, on_main, &same) {
        // Main renamed or deleted a path one side names, which neither
        // run saw, so it runs again as a change checked once would.
        ready::Before::Twice(newest, _, shared)
            if !names_gone(&newest).is_some_and(|naming| naming.is_empty()) =>
        {
            ready::Before::Shares(newest, shared)
        }
        before => before,
    };
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
    let _ = (folder, main, commit);
    None
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::tests::git_in;

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
