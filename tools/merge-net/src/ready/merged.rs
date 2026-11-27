//! A pass at a merge of a head over main, which speaks for the head when
//! main moved its files since its own run. On 26 November owners merged
//! 14 changes alone by hand this way in four hours, while `ready` boarded
//! each on a train main's red kept sinking: the merge `git merge-tree
//! --write-tree MAIN HEAD` makes, committed as `git commit-tree TREE -p
//! MAIN -p HEAD`, is what GitHub lands, so changed-checks and `needs`
//! passing there say what a run at the head can't once main has moved.
//!
//! A merge speaks for the head while its first parent is on main, its
//! second is the head, its tree is the one merging the head onto that
//! parent makes, changed-checks built what it named there, and main has
//! since moved none of the change's files nor the crates beside them,
//! the append-only descriptions file aside while the head still merges
//! onto main cleanly. The commits are found among nix-offload's records,
//! each its own merge, since a signed commit's sha can't be made again.

use std::path::Path;

use crate_reach::Crates;

use crate::{earlier, git, lines, ready};

/// The file every check adds a line to, which a merge onto a newer main
/// takes beside the change's own lines (`apart.rs`).
const DESCRIPTIONS: &str = "check-descriptions.nix";

/// A commit, as `git log` says of it: itself, its tree and its parents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    pub sha: String,
    pub tree: String,
    pub parents: Vec<String>,
}

/// How a merge is judged, apart from git.
pub struct Judge<'a> {
    /// Whether changed-checks built what it named at a commit.
    pub built: &'a dyn Fn(&str) -> bool,
    /// Whether main holds a commit.
    pub on_main: &'a dyn Fn(&str) -> bool,
    /// The tree merging the head onto a base makes, none where they clash.
    pub merged: &'a dyn Fn(&str) -> Option<String>,
    /// The change's files, or crates beside them, main moved since a base.
    pub moved: &'a dyn Fn(&str) -> Vec<String>,
}

/// The newest of `merges` that speaks for `head`, as [`Judge`] says.
#[must_use]
pub fn speaking(head: &str, merges: &[Commit], judge: &Judge) -> Option<String> {
    merges
        .iter()
        .find(|merge| {
            let [base, second] = merge.parents.as_slice() else {
                return false;
            };
            second == head
                && (judge.on_main)(base)
                && (judge.merged)(base).as_deref() == Some(merge.tree.as_str())
                && (judge.moved)(base).is_empty()
                && (judge.built)(&merge.sha)
        })
        .map(|merge| merge.sha.clone())
}

/// The commits of `said`, `git log`'s words in `%H %T %P`, one a line.
#[must_use]
pub fn commits(said: &str) -> Vec<Commit> {
    said.lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace().map(str::to_owned);
            Some(Commit {
                sha: words.next()?,
                tree: words.next()?,
                parents: words.collect(),
            })
        })
        .collect()
}

/// Of `files`, those `since` holds, `beside` the crates main moved around
/// them, the descriptions file left out when `clean`, as the head still
/// merges onto main as it is now.
#[must_use]
pub fn shared(files: &[String], since: &[String], beside: Vec<String>, clean: bool) -> Vec<String> {
    let mut shared: Vec<String> = files
        .iter()
        .filter(|file| since.contains(file))
        .filter(|file| !(clean && file.as_str() == DESCRIPTIONS))
        .cloned()
        .collect();
    for file in beside {
        if !shared.contains(&file) {
            shared.push(file);
        }
    }
    shared
}

/// The merge of `head` over main at `main` that passed and speaks for the
/// head, whose change holds `files`, `on_main` saying what main holds.
#[must_use]
pub fn pass(
    head: &str,
    main: &str,
    files: &[String],
    on_main: &dyn Fn(&str) -> bool,
) -> Option<String> {
    let records = ready::recent_records();
    if records.is_empty() {
        return None;
    }
    let said = crate::logged(Path::new("."), &records, &["--format=%H %T %P"])?;
    let merges: Vec<Commit> = commits(&said)
        .into_iter()
        .filter(|merge| merge.parents.get(1).is_some_and(|second| second == head))
        .collect();
    if merges.is_empty() {
        return None;
    }
    let here = Path::new(".");
    let clean = earlier::merged_tree(here, main, head).is_some();
    let crates = Crates::at(here, head);
    let moved = |base: &str| {
        let since = lines(&git(&["diff", "--name-only", base, main]).unwrap_or_default());
        shared(files, &since, crates.beside(files, &since), clean)
    };
    let built = |commit: &str| ready::built_there(&ready::record(commit), on_main);
    let merged = |base: &str| earlier::merged_tree(here, base, head);
    let judge = Judge {
        built: &built,
        on_main,
        merged: &merged,
        moved: &moved,
    };
    speaking(head, &merges, &judge)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(sha: &str, tree: &str, parents: &[&str]) -> Commit {
        Commit {
            sha: sha.to_owned(),
            tree: tree.to_owned(),
            parents: parents.iter().map(|&parent| parent.to_owned()).collect(),
        }
    }

    fn owned(names: &[&str]) -> Vec<String> {
        names.iter().map(|&name| name.to_owned()).collect()
    }

    /// A judge for which merges onto `base` make tree `t`, main holds
    /// `base`, every merge built, and main moved nothing.
    fn speaks_for(head: &str, merges: &[Commit]) -> Option<String> {
        let judge = Judge {
            built: &|_| true,
            on_main: &|base| base == "m1" || base == "m2",
            merged: &|base| Some(format!("t-{base}")),
            moved: &|_| Vec::new(),
        };
        speaking(head, merges, &judge)
    }

    #[test]
    fn the_newest_merge_of_the_head_over_main_speaks_for_it() {
        let merges = [
            commit("c2", "t-m2", &["m2", "h"]),
            commit("c1", "t-m1", &["m1", "h"]),
        ];
        assert_eq!(
            speaks_for("h", &merges).as_deref(),
            Some("c2"),
            "the newest merge that passed should speak"
        );
    }

    #[test]
    fn a_merge_of_another_head_or_a_plain_commit_speaks_for_nothing() {
        let merges = [
            commit("c3", "t-m1", &["m1", "other"]),
            commit("c4", "t-m1", &["h"]),
            commit("c5", "t-m1", &["m1", "h", "third"]),
        ];
        assert_eq!(
            speaks_for("h", &merges),
            None,
            "only a two-parent merge whose second parent is the head speaks"
        );
    }

    #[test]
    fn a_merge_whose_tree_isnt_the_heads_merge_speaks_for_nothing() {
        // Its commit was made over another tree, so what passed there isn't
        // what GitHub lands.
        let merges = [commit("c1", "t-edited", &["m1", "h"])];
        assert_eq!(
            speaks_for("h", &merges),
            None,
            "a merge commit over an edited tree must not speak"
        );
    }

    #[test]
    fn a_merge_over_a_base_main_lacks_speaks_for_nothing() {
        let merges = [commit("c1", "t-x", &["x", "h"])];
        assert_eq!(
            speaks_for("h", &merges),
            None,
            "a merge over a base main doesn't hold must not speak"
        );
    }

    #[test]
    fn a_merge_main_moved_the_files_of_since_gives_way_to_an_older_one() {
        let merges = [
            commit("c2", "t-m2", &["m2", "h"]),
            commit("c1", "t-m1", &["m1", "h"]),
        ];
        let judge = Judge {
            built: &|sha| sha == "c2",
            on_main: &|_| true,
            merged: &|base| Some(format!("t-{base}")),
            moved: &|base| {
                if base == "m1" {
                    owned(&["a.rs"])
                } else {
                    Vec::new()
                }
            },
        };
        assert_eq!(
            speaking("h", &merges, &judge).as_deref(),
            Some("c2"),
            "the merge main hasn't moved past should speak"
        );
        let unbuilt = Judge {
            built: &|_| false,
            ..judge
        };
        assert_eq!(
            speaking("h", &merges, &unbuilt),
            None,
            "a merge whose run built nothing must not speak"
        );
    }

    #[test]
    fn git_logs_words_read_as_commits() {
        let said = "c1 t1 m1 h\nc2 t2 h\n\n";
        assert_eq!(
            commits(said),
            vec![commit("c1", "t1", &["m1", "h"]), commit("c2", "t2", &["h"])],
            "each line should read as a commit, its tree and parents"
        );
    }

    #[test]
    fn the_descriptions_file_is_shared_only_when_the_head_clashes_with_main() {
        let files = owned(&["a.rs", DESCRIPTIONS]);
        let since = owned(&[DESCRIPTIONS, "b.rs"]);
        assert!(
            shared(&files, &since, Vec::new(), true).is_empty(),
            "a descriptions line beside main's is no shared file while it merges cleanly"
        );
        assert_eq!(
            shared(&files, &since, Vec::new(), false),
            owned(&[DESCRIPTIONS]),
            "a head that clashes with main shares the descriptions file"
        );
        assert_eq!(
            shared(
                &files,
                &owned(&["a.rs"]),
                owned(&["c/lib.rs", "a.rs"]),
                true
            ),
            owned(&["a.rs", "c/lib.rs"]),
            "crates beside the change count once each"
        );
    }
}
