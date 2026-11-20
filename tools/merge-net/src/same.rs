//! Whether a review round of ours read a head, or an earlier commit of its
//! change that a clean rebase carried to it, and how an earlier commit is
//! told to be the same change. From 17 to 19 November 240 of 429 reviewed
//! pull requests merged at a head no round had read, a fix pushed after
//! the council going to main unread.

use std::collections::HashSet;
use std::path::Path;

use crate::{candidates, git_at, patch_of, ready, seen, short};

/// How a change from a base to a head, in a repo, is told from others.
pub type PatchId = fn(&Path, &str, &str) -> Option<String>;

/// Whether a review round of ours read `commit`, or an earlier commit of
/// the same change that a clean rebase carried here (`seen.rs`). The
/// earlier commits are looked for among those the rounds read, so a
/// reviewed commit with no pass of its own still counts. Why not, when not.
pub fn read_by_a_round(commit: &str, main: &str, files: &[String]) -> Result<(), String> {
    let read = seen::witnessed(&seen::rounds_folder())?;
    read_among(commit, &read, |witnessed| {
        same_change(commit, main, files, witnessed, read_patch_id)
    })
}

/// Whether `commit` is among the commits the rounds `read`, or `earlier`,
/// given those commits, finds one of the same change among them.
fn read_among(
    commit: &str,
    read: &HashSet<String>,
    earlier: impl FnOnce(&[String]) -> Vec<String>,
) -> Result<(), String> {
    if read.contains(commit) {
        return Ok(());
    }
    let mut witnessed: Vec<String> = read.iter().cloned().collect();
    witnessed.sort();
    if earlier(&witnessed).is_empty() {
        Err(seen::unread(short(commit)))
    } else {
        Ok(())
    }
}

/// The earlier commits of `commit`'s change among `records` touching
/// `files` whose own change, against their base on main, is the same patch
/// as `commit`'s, by `id`.
pub fn same_change(
    commit: &str,
    main: &str,
    files: &[String],
    records: &[String],
    id: PatchId,
) -> Vec<String> {
    let candidates = candidates(commit, main, files, records).unwrap_or_default();
    same_patch(Path::new("."), commit, main, candidates, id)
}

/// Those of `candidates`, in the repo at `repo`, whose own change against
/// their base on `main` is the same patch as `commit`'s against its own,
/// by `id`: by `read_patch_id`, a clean rebase of a reviewed commit is one.
fn same_patch(
    repo: &Path,
    commit: &str,
    main: &str,
    candidates: Vec<String>,
    id: PatchId,
) -> Vec<String> {
    let base_of = |at: &str| git_at(repo, &["merge-base", main, at]).ok();
    let base = base_of(commit).unwrap_or_else(|| main.to_owned());
    let Some(change) = id(repo, &base, commit) else {
        return Vec::new();
    };
    candidates
        .into_iter()
        .filter(|candidate| {
            ready::when_same(
                candidate,
                &change,
                base_of,
                |base, at| id(repo, base, at),
                |_| Some(()),
            )
            .is_some()
        })
        .collect()
}

/// The patch id a review carries by, from `base` to `head` in the repo at
/// `repo`: with git's lines of context, so a block moved within its file
/// is another change, and whitespace kept, since it is code in some
/// languages. A rebase over an edit beside the change asks for a round
/// again, which costs little beside a head merging unread.
fn read_patch_id(repo: &Path, base: &str, head: &str) -> Option<String> {
    // Verbatim ids are stable ones too; git takes only one of the two.
    patch_of(repo, base, head, &[], "--verbatim")
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use super::*;
    use crate::tests::{commit, git_in};

    // A head a round read passes; one it didn't passes only when an
    // earlier commit of its change is found among the commits the rounds
    // read, which are what's searched, not the day's passes, so a reviewed
    // commit that never passed alone still speaks for its clean rebase.
    #[test]
    fn a_head_is_read_when_a_round_read_it_or_its_change_before_a_rebase() {
        let read = HashSet::from(["aaa".to_owned(), "bbb".to_owned()]);
        let never = |_: &[String]| -> Vec<String> { panic!("searched for a head a round read") };
        assert_eq!(read_among("aaa", &read, never), Ok(()));
        let mut searched = Vec::new();
        let found = read_among("ccc", &read, |witnessed| {
            searched = witnessed.to_vec();
            vec!["bbb".to_owned()]
        });
        assert_eq!(found, Ok(()), "a clean rebase of a read commit was refused");
        assert_eq!(
            searched,
            ["aaa", "bbb"],
            "the search wasn't over what the rounds read"
        );
        let refused = read_among("ccc", &read, |_| Vec::new()).unwrap_err();
        assert!(
            refused.contains("no review round of ours read"),
            "{refused}"
        );
    }

    // A reviewed commit rebased cleanly onto a main that moved elsewhere
    // is the same patch, so the review carries across with no new round;
    // one rebased with a line of its own added is not, nor one whose read
    // lines moved within their file, and each is refused.
    #[test]
    fn a_clean_rebase_of_a_read_commit_is_read_and_a_changed_one_is_not() {
        let repo = env::temp_dir().join(format!("merge-net-rebase-{}", std::process::id()));
        fs::create_dir_all(&repo).unwrap();
        git_in(&repo, &["init", "-q", "-b", "main"]);
        fs::write(repo.join("w"), "a\nb\nc\n").unwrap();
        fs::write(repo.join("v"), "x\n").unwrap();
        git_in(&repo, &["add", "w", "v"]);
        git_in(&repo, &["commit", "-qm", "base"]);
        let base = git_in(&repo, &["rev-parse", "HEAD"]);
        let reviewed = commit(&repo, "a\nB\nc\n");
        git_in(&repo, &["checkout", "-q", &base]);
        fs::write(repo.join("v"), "y\n").unwrap();
        git_in(&repo, &["commit", "-qam", "elsewhere"]);
        let main = git_in(&repo, &["rev-parse", "HEAD"]);
        git_in(&repo, &["cherry-pick", &reviewed]);
        let rebased = git_in(&repo, &["rev-parse", "HEAD"]);
        git_in(&repo, &["checkout", "-q", &main]);
        let changed = commit(&repo, "a\nB\nc\nd\n");
        git_in(&repo, &["checkout", "-q", &main]);
        // The reviewed commit's very lines, b taken out and B put in, with
        // B moved to the end of the file.
        let moved = commit(&repo, "a\nc\nB\n");
        let range = format!("{base}..{reviewed}");
        let range_diff = git_in(
            &repo,
            &["range-diff", "-s", &range, &format!("{main}..{rebased}")],
        );
        assert!(
            range_diff.contains(" = "),
            "the rebase wasn't clean: {range_diff}"
        );
        let read = HashSet::from([reviewed.clone()]);
        let search = |head: &str| {
            read_among(head, &read, |witnessed| {
                same_patch(&repo, head, &main, witnessed.to_vec(), read_patch_id)
            })
        };
        assert_eq!(search(&rebased), Ok(()), "a clean rebase lost its review");
        assert!(
            search(&changed).is_err(),
            "a changed rebase kept its review"
        );
        assert!(
            search(&moved).is_err(),
            "the read lines moved within their file kept its review"
        );
        fs::remove_dir_all(&repo).unwrap();
    }

    // A review carries by an id read with context and whitespace, so it
    // goes to no edit of its whitespace, nor across an edit beside it.
    #[test]
    fn a_review_goes_to_no_edit_of_whitespace_or_beside_the_change() {
        let repo = env::temp_dir().join(format!("merge-net-read-id-{}", std::process::id()));
        fs::create_dir_all(&repo).unwrap();
        git_in(&repo, &["init", "-q", "-b", "main"]);
        fs::write(repo.join("w"), "a\nb\nc\nd\ne\n").unwrap();
        git_in(&repo, &["add", "w"]);
        git_in(&repo, &["commit", "-qm", "w"]);
        let base = git_in(&repo, &["rev-parse", "HEAD"]);
        let change = commit(&repo, "a\nb\nC\nd\ne\n");
        git_in(&repo, &["checkout", "-q", &base]);
        let spaced = commit(&repo, "a\nb\n  C\nd\ne\n");
        git_in(&repo, &["checkout", "-q", &base]);
        let beside = commit(&repo, "a\nB\nc\nd\ne\n");
        let rebased = commit(&repo, "a\nB\nC\nd\ne\n");
        let read = read_patch_id(&repo, &base, &change);
        assert!(read.is_some(), "git gave no id");
        assert_ne!(
            read,
            read_patch_id(&repo, &base, &spaced),
            "a change to whitespace alone is another change"
        );
        assert_ne!(
            read,
            read_patch_id(&repo, &beside, &rebased),
            "an edit beside the change kept its review"
        );
        fs::remove_dir_all(&repo).unwrap();
    }
}
