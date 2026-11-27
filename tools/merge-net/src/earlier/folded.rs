//! Heads a review round read without naming them. Two kinds:
//!
//! - A change below the tip of a stack that one stack round read. The
//!   round records only the tip, which holds every commit of the changes
//!   below it, so each lower change asked a round of its own. On 27
//!   November that undid folding rounds into stacks of about five, and the
//!   start line held 61 asks.
//! - A head pushed after a round read an earlier commit of its change,
//!   where what came after touches only docs and tests, or merges main in
//!   cleanly. No reviewer's question turns on those, and the gate's checks
//!   at the head still have to pass.

use std::collections::HashSet;
use std::path::Path;

use super::seen;
use crate::{git_at, short};

/// The most commits of a change walked for what came after its read one,
/// so a head on a branch that never shared main's history isn't walked
/// through whole.
const MOST_WALKED: usize = 500;

/// Why `commit`, which no round named, was read all the same, in the repo
/// at `repo`: within a stack whose tip a stack round of `rounds` read, or
/// after a commit a round read with only docs, tests and clean merges of
/// `main` since. `on_main` says whether a range round's base is on main,
/// as for any round, so only rounds that read their commit in full count.
pub fn read(
    repo: &Path,
    commit: &str,
    main: &str,
    rounds: &HashSet<seen::Read>,
    mut on_main: impl FnMut(&str) -> bool,
) -> Option<String> {
    let mut full: Vec<&seen::Read> = rounds
        .iter()
        .filter(|read| read.from.as_deref().is_none_or(&mut on_main))
        .collect();
    full.sort_by(|one, other| one.commit.cmp(&other.commit));
    within_a_stack(repo, commit, &full).or_else(|| {
        let read: HashSet<&str> = full.iter().map(|read| read.commit.as_str()).collect();
        after_a_read(repo, commit, main, &read)
    })
}

/// Whether a stack round of `full` read a tip that holds `commit`.
fn within_a_stack(repo: &Path, commit: &str, full: &[&seen::Read]) -> Option<String> {
    let tip = full.iter().find(|read| {
        read.stack
            && read.commit != commit
            && git_at(repo, &["merge-base", "--is-ancestor", commit, &read.commit]).is_ok()
    })?;
    Some(format!(
        "{} is read: a stack round read {}, which holds it, and so read every commit of its \
         change",
        short(commit),
        short(&tip.commit)
    ))
}

/// Whether a commit of `commit`'s change that a round read, among `read`,
/// is followed only by commits touching docs and tests, and merges of
/// `main` that git made cleanly. The newest read commit is the one asked
/// after, since what follows it is the least.
fn after_a_read(repo: &Path, commit: &str, main: &str, read: &HashSet<&str>) -> Option<String> {
    let not_main = format!("^{main}");
    let change = git_at(repo, &["rev-list", "--topo-order", commit, &not_main]).ok()?;
    let change: Vec<&str> = change.lines().take(MOST_WALKED + 1).collect();
    if change.len() > MOST_WALKED {
        return None;
    }
    let reviewed = change.iter().find(|one| read.contains(*one))?;
    let after = git_at(
        repo,
        &[
            "rev-list",
            "--parents",
            commit,
            &format!("^{reviewed}"),
            &not_main,
        ],
    )
    .ok()?;
    for line in after.lines() {
        let mut words = line.split(' ');
        let one = words.next()?;
        let parents: Vec<&str> = words.collect();
        let fine = match parents.as_slice() {
            [_] => only_docs_and_tests(repo, one),
            [first, second] => merged_main_cleanly(repo, one, [first, second], main),
            _ => false,
        };
        if !fine {
            return None;
        }
    }
    Some(format!(
        "{} is read: a round read {}, and what came after touches only docs and tests, or \
         merged main in cleanly",
        short(commit),
        short(reviewed)
    ))
}

/// Whether the commit `one`, of one parent, changes only docs and tests:
/// a file under `docs/` or `tests/` at the top, any `.md`, or a
/// `.test.ts`.
fn only_docs_and_tests(repo: &Path, one: &str) -> bool {
    let Ok(files) = git_at(
        repo,
        &["diff-tree", "--no-commit-id", "--name-only", "-r", one],
    ) else {
        return false;
    };
    files.lines().all(|file| {
        file.starts_with("docs/")
            || file.starts_with("tests/")
            || Path::new(file)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
            || file.ends_with(".test.ts")
    })
}

/// Whether the merge `one` of `parents` took one of them from `main` and
/// holds just what git merges of the two without a conflict, so no line
/// of it was written by hand.
fn merged_main_cleanly(repo: &Path, one: &str, parents: [&str; 2], main: &str) -> bool {
    let from_main = parents
        .iter()
        .any(|parent| git_at(repo, &["merge-base", "--is-ancestor", parent, main]).is_ok());
    if !from_main {
        return false;
    }
    let Ok(merged) = git_at(
        repo,
        &["merge-tree", "--write-tree", parents[0], parents[1]],
    ) else {
        return false;
    };
    let tree = git_at(repo, &["rev-parse", &format!("{one}^{{tree}}")]);
    merged
        .lines()
        .next()
        .is_some_and(|merged| tree.as_deref() == Ok(merged))
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use super::*;
    use crate::tests::{commit, git_in};

    /// A repo of its own for a test named `name`, with `main` at one
    /// commit, and that commit.
    fn repo(name: &str) -> (std::path::PathBuf, String) {
        let repo = env::temp_dir().join(format!("merge-net-folded-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&repo);
        fs::create_dir_all(&repo).unwrap();
        git_in(&repo, &["init", "-q", "-b", "main"]);
        fs::write(repo.join("w"), "a\n").unwrap();
        git_in(&repo, &["add", "w"]);
        git_in(&repo, &["commit", "-qm", "w"]);
        let base = git_in(&repo, &["rev-parse", "HEAD"]);
        git_in(&repo, &["checkout", "-qb", "change"]);
        (repo, base)
    }

    /// Commits `words` as the file `file` in `repo` and gives the commit.
    fn commit_file(repo: &Path, file: &str, words: &str) -> String {
        let path = repo.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, words).unwrap();
        git_in(repo, &["add", file]);
        git_in(repo, &["commit", "-qm", file]);
        git_in(repo, &["rev-parse", "HEAD"])
    }

    fn round(commit: &str, from: Option<&str>, stack: bool) -> seen::Read {
        seen::Read {
            commit: commit.to_owned(),
            from: from.map(str::to_owned),
            stack,
        }
    }

    /// Whether `commit` is read in `repo` by `rounds`, with only `base`
    /// a range's base on main.
    fn said(repo: &Path, commit: &str, rounds: &[seen::Read], base: &str) -> Option<String> {
        let rounds: HashSet<seen::Read> = rounds.iter().cloned().collect();
        read(repo, commit, "main", &rounds, |from| from == base)
    }

    // A change below a stack's tip is read by the stack round that read the
    // tip, but not by a council at the tip, nor by a stack round over a
    // range from a base off main.
    #[test]
    fn a_change_below_a_stacks_tip_is_read_by_its_stack_round() {
        let (repo, base) = repo("stack");
        let lower = commit(&repo, "lower\n");
        let tip = commit(&repo, "tip\n");
        let read = said(&repo, &lower, &[round(&tip, None, true)], &base);
        assert!(
            read.as_deref()
                .is_some_and(|read| read.contains("stack round")),
            "a stack round of the tip didn't read the change below it: {read:?}"
        );
        assert_eq!(
            said(&repo, &lower, &[round(&tip, None, false)], &base),
            None,
            "a council of the tip read the change below it"
        );
        assert_eq!(
            said(&repo, &lower, &[round(&tip, Some(&lower), true)], &base),
            None,
            "a stack round from a base off main read the change below it"
        );
        assert_eq!(
            said(&repo, &tip, &[round(&lower, None, true)], &base),
            None,
            "a stack round of a lower change read the tip above it"
        );
        fs::remove_dir_all(&repo).unwrap();
    }

    // A head is read when what came after its read commit touches only
    // docs and tests, and isn't once a commit touches anything else.
    #[test]
    fn docs_and_tests_after_a_read_commit_keep_the_head_read() {
        let (repo, base) = repo("docs");
        let reviewed = commit(&repo, "code\n");
        let rounds = [round(&reviewed, None, false)];
        commit_file(&repo, "docs/guide.md", "words\n");
        commit_file(&repo, "modules/x/AGENTS.md", "words\n");
        commit_file(&repo, "tests/case.py", "case\n");
        let tested = commit_file(&repo, "src/a.test.ts", "case\n");
        let read = said(&repo, &tested, &rounds, &base);
        assert!(
            read.as_deref()
                .is_some_and(|read| read.contains("only docs and tests")),
            "docs and tests after a read commit left the head unread: {read:?}"
        );
        let coded = commit_file(&repo, "modules/x/tests.nix", "check\n");
        assert_eq!(
            said(&repo, &coded, &rounds, &base),
            None,
            "a code change after the read commit was taken as read"
        );
        fs::remove_dir_all(&repo).unwrap();
    }

    // A clean merge of main after a read commit keeps the head read; a
    // merge with a hand-made line, or of a branch that isn't main, doesn't.
    #[test]
    fn only_a_clean_merge_of_main_keeps_the_head_read() {
        let (repo, base) = repo("merge");
        let reviewed = commit(&repo, "code\n");
        let rounds = [round(&reviewed, None, false)];
        git_in(&repo, &["checkout", "-q", "main"]);
        commit_file(&repo, "elsewhere", "main's\n");
        git_in(&repo, &["checkout", "-q", "change"]);
        git_in(&repo, &["merge", "-q", "--no-edit", "main"]);
        let merged = git_in(&repo, &["rev-parse", "HEAD"]);
        let read = said(&repo, &merged, &rounds, &base);
        assert!(
            read.as_deref()
                .is_some_and(|read| read.contains("merged main")),
            "a clean merge of main left the head unread: {read:?}"
        );
        fs::write(repo.join("w"), "code and more\n").unwrap();
        git_in(&repo, &["commit", "-q", "-a", "--amend", "--no-edit"]);
        let edited = git_in(&repo, &["rev-parse", "HEAD"]);
        assert_eq!(
            said(&repo, &edited, &rounds, &base),
            None,
            "a merge with a hand-made line was taken as read"
        );
        git_in(&repo, &["reset", "-q", "--hard", &reviewed]);
        git_in(&repo, &["checkout", "-qb", "other", &base]);
        commit_file(&repo, "theirs", "unreviewed\n");
        git_in(&repo, &["checkout", "-q", "change"]);
        git_in(&repo, &["merge", "-q", "--no-edit", "other"]);
        let other = git_in(&repo, &["rev-parse", "HEAD"]);
        assert_eq!(
            said(&repo, &other, &rounds, &base),
            None,
            "a merge of a branch that isn't main was taken as read"
        );
        fs::remove_dir_all(&repo).unwrap();
    }
}
