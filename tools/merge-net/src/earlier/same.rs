//! Whether a review round of ours read a head, or an earlier commit of its
//! change that a clean rebase carried to it, and how an earlier commit is
//! told to be the same change. From 17 to 19 November 240 of 429 reviewed
//! pull requests merged at a head no round had read, a fix pushed after
//! the council going to main unread.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::seen;
use crate::{candidates, git_at, patch_of, ready, short};

/// How a change from a base to a head, in a repo, is told from others.
pub type PatchId = fn(&Path, &str, &str) -> Option<String>;

/// Whether a review round of ours read `commit`, or an earlier commit of
/// the same change that a clean rebase carried here (`same_patch`). The
/// earlier commits are looked for among those the rounds read, so a
/// reviewed commit with no pass of its own still counts. Why not, when not.
///
/// Only the rounds of `commit` and of the earlier commits that could be its
/// change have their base asked after, one git each: on 21 November the
/// rounds held about 2,300 bases, and asking after every one held each
/// `ready` past 15 minutes while four ran at once.
///
/// The rounds mirrored from other machines are asked only when this
/// machine's read nothing for the head, so a mirror that's broken or slow
/// never holds back a head reviewed here.
pub fn read_by_a_round(commit: &str, main: &str, files: &[String]) -> Result<(), String> {
    let repo = Path::new(".");
    here_or_mirrored(
        seen::witnessed(&seen::rounds_folder())?,
        || seen::mirrored(&seen::mirrors_folder()),
        |rounds| {
            read_in(
                commit,
                rounds,
                on_main_at(repo, main),
                |witnessed| candidates(commit, main, files, witnessed).unwrap_or_default(),
                |read| same_patch(repo, commit, main, read, read_patch_id),
            )
        },
    )
}

/// Whether `ask` finds the head read among this machine's `rounds`, or,
/// only when it doesn't, among them and those `mirrors` reads. A refusal
/// says which mirrors were asked and which couldn't be.
fn here_or_mirrored(
    mut rounds: HashSet<seen::Read>,
    mirrors: impl FnOnce() -> seen::Mirrored,
    mut ask: impl FnMut(&HashSet<seen::Read>) -> Result<(), String>,
) -> Result<(), String> {
    let Err(unread) = ask(&rounds) else {
        return Ok(());
    };
    let mirrored = mirrors();
    // No mirror read means nothing new to find the head among, so the
    // head isn't asked after again, only told with why.
    if mirrored.asked.is_empty() {
        return Err(seen::unread_with_mirrors(unread, &mirrored));
    }
    rounds.extend(mirrored.rounds.iter().cloned());
    ask(&rounds).map_err(|why| seen::unread_with_mirrors(why, &mirrored))
}

/// Whether `rounds` read `commit` in full, or an earlier commit of its
/// change: `could_be`, given every commit the rounds read, sorted, names
/// those that might be its change, and `same` keeps those of the ones read
/// in full that are. A base is put to `on_main` only for the rounds of
/// `commit` and of what `could_be` names, which keeps the gits to a few.
fn read_in(
    commit: &str,
    rounds: &HashSet<seen::Read>,
    mut on_main: impl FnMut(&str) -> bool,
    could_be: impl FnOnce(&[String]) -> Vec<String>,
    same: impl FnOnce(Vec<String>) -> Vec<String>,
) -> Result<(), String> {
    let mut full = |among: &HashSet<&str>| {
        let of_them = rounds
            .iter()
            .filter(|read| among.contains(read.commit.as_str()));
        in_full(of_them, &mut on_main)
    };
    if !full(&HashSet::from([commit])).is_empty() {
        return Ok(());
    }
    let mut witnessed: Vec<String> = rounds
        .iter()
        .map(|read| read.commit.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    witnessed.sort();
    let could_be = could_be(&witnessed);
    let mut read: Vec<String> = full(&could_be.iter().map(String::as_str).collect())
        .into_iter()
        .collect();
    read.sort();
    if same(read).is_empty() {
        Err(seen::unread(short(commit)))
    } else {
        Ok(())
    }
}

/// The commits of `rounds` read in full: each a change's round read, and
/// each a range's whose base `on_main` finds on main, so a round over only
/// a head's newest commit doesn't stand for the commits under it.
fn in_full<'a>(
    rounds: impl Iterator<Item = &'a seen::Read>,
    on_main: &mut impl FnMut(&str) -> bool,
) -> HashSet<String> {
    rounds
        .filter(|read| read.from.as_deref().is_none_or(&mut *on_main))
        .map(|read| read.commit.clone())
        .collect()
}

/// Whether a base is on `main`, in the repo at `repo`, one git a base.
fn on_main_at<'a>(repo: &'a Path, main: &'a str) -> impl FnMut(&str) -> bool + 'a {
    let mut known = HashMap::new();
    move |base: &str| {
        *known.entry(base.to_owned()).or_insert_with(|| {
            // One git would read as an option is no base.
            !base.is_empty()
                && !base.starts_with('-')
                && git_at(repo, &["merge-base", "--is-ancestor", base, main]).is_ok()
        })
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
/// again, which costs little beside a head merging unread. The diff's
/// shape is named rather than left to git's config, where
/// `diff.context=0` would drop the context the id is read with.
fn read_patch_id(repo: &Path, base: &str, head: &str) -> Option<String> {
    let shape = [
        "--unified=3",
        "--no-color",
        "--no-ext-diff",
        "--no-textconv",
    ];
    // Verbatim ids are stable ones too; git takes only one of the two.
    patch_of(repo, base, head, &shape, "--verbatim")
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use super::*;
    use crate::tests::{commit, git_in};

    fn round(commit: &str, from: Option<&str>) -> seen::Read {
        seen::Read {
            commit: commit.to_owned(),
            from: from.map(str::to_owned),
        }
    }

    /// Asks whether `head` is among the rounds, as `read_in` would.
    fn asks(head: &str) -> impl FnMut(&HashSet<seen::Read>) -> Result<(), String> + '_ {
        move |rounds| {
            if rounds.iter().any(|read| read.commit == head) {
                Ok(())
            } else {
                Err(format!("unread {head}"))
            }
        }
    }

    /// What a mirror read: the rounds of `commits`, from the nodes `asked`,
    /// with `trouble`.
    fn mirror(commits: &[&str], asked: &[&str], trouble: &[&str]) -> seen::Mirrored {
        seen::Mirrored {
            rounds: commits.iter().map(|one| round(one, None)).collect(),
            asked: asked.iter().map(|one| (*one).to_owned()).collect(),
            trouble: trouble.iter().map(|one| (*one).to_owned()).collect(),
        }
    }

    // A head this machine's rounds read passes without a mirror asked, so
    // a mirror that's broken or slow never holds back a head reviewed here.
    #[test]
    fn a_head_read_here_asks_no_mirror() {
        let here = HashSet::from([round("aaa", None)]);
        let found = here_or_mirrored(
            here,
            || panic!("a mirror was asked for a head read here"),
            asks("aaa"),
        );
        assert_eq!(found, Ok(()), "a head read here was refused");
    }

    // A head only another machine's round read passes once its mirror is
    // asked, and one none read is refused naming the mirrors asked and
    // those that couldn't be.
    #[test]
    fn a_head_unread_here_is_read_from_the_mirrors() {
        let found = here_or_mirrored(
            HashSet::new(),
            || mirror(&["bbb"], &["equilibrius"], &[]),
            asks("bbb"),
        );
        assert_eq!(found, Ok(()), "a head a mirrored round read was refused");
        let said = here_or_mirrored(
            HashSet::from([round("aaa", None)]),
            || {
                mirror(
                    &["bbb"],
                    &["equilibrius"],
                    &["the mirror from penelope can't be read"],
                )
            },
            asks("ccc"),
        )
        .unwrap_err();
        assert!(
            said.starts_with("unread ccc")
                && said.contains("mirrored from equilibrius were asked too")
                && said.contains("penelope can't be read"),
            "the refusal didn't say what was asked: {said}"
        );
    }

    // With no mirror on the machine, a refusal reads as it always did; with
    // one that couldn't be read, it says why, and the head isn't asked
    // after a second time over the same rounds.
    #[test]
    fn no_mirror_read_refuses_as_before_and_says_why() {
        let said = here_or_mirrored(HashSet::new(), seen::Mirrored::default, asks("ccc"));
        assert_eq!(
            said,
            Err("unread ccc".to_owned()),
            "no mirror changed the refusal"
        );
        let mut times = 0;
        let said = here_or_mirrored(
            HashSet::new(),
            || mirror(&[], &[], &["the rounds mirrored under /x can't be listed"]),
            |rounds| {
                times += 1;
                asks("ccc")(rounds)
            },
        );
        assert_eq!(
            said,
            Err("unread ccc; the rounds mirrored under /x can't be listed".to_owned()),
            "an unlisted mirror wasn't said"
        );
        assert_eq!(times, 1, "the head was asked after again with nothing new");
    }

    // A head a round read in full passes with nothing searched; one it
    // didn't passes only when an earlier commit of its change is found
    // among every commit the rounds read, and that commit was itself read
    // in full, so a range over a head's newest commit alone stands for no
    // rebase of it.
    #[test]
    fn a_head_is_read_when_a_round_read_it_or_its_change_in_full() {
        let rounds = HashSet::from([
            round("aaa", None),
            round("bbb", None),
            round("bbb", Some("main")),
            round("eee", Some("off")),
        ]);
        let on_main = |base: &str| base == "main";
        let never = |_: &[String]| -> Vec<String> { panic!("searched for a head a round read") };
        assert_eq!(read_in("aaa", &rounds, on_main, never, |read| read), Ok(()));
        let mut searched = Vec::new();
        let mut matched = Vec::new();
        let found = read_in(
            "ccc",
            &rounds,
            on_main,
            |witnessed| {
                searched = witnessed.to_vec();
                vec!["bbb".to_owned(), "eee".to_owned()]
            },
            |read| {
                matched.clone_from(&read);
                read
            },
        );
        assert_eq!(found, Ok(()), "a clean rebase of a read commit was refused");
        assert_eq!(
            searched,
            ["aaa", "bbb", "eee"],
            "the search wasn't over every commit the rounds read, once each"
        );
        assert_eq!(matched, ["bbb"], "a commit read only off main was matched");
        let refused = read_in(
            "ccc",
            &rounds,
            on_main,
            |_| vec!["eee".to_owned()],
            |read| read,
        )
        .unwrap_err();
        assert!(
            refused.contains("no review round of ours read"),
            "{refused}"
        );
    }

    // Each base is a git, and on 21 November the rounds held about 2,300,
    // so only the head's own rounds and those of what could be its change
    // have theirs asked after.
    #[test]
    fn only_the_heads_and_its_candidates_bases_are_asked_after() {
        let rounds = HashSet::from([
            round("aaa", Some("for-aaa")),
            round("bbb", Some("for-bbb")),
            round("fff", Some("for-fff")),
        ]);
        let mut asked = Vec::new();
        let _ = read_in(
            "aaa",
            &rounds,
            |base: &str| {
                asked.push(base.to_owned());
                false
            },
            |_| vec!["bbb".to_owned()],
            |read| read,
        );
        asked.sort();
        assert_eq!(asked, ["for-aaa", "for-bbb"]);
    }

    // A reviewed commit rebased cleanly onto a main that moved elsewhere
    // is the same patch, so the review carries across with no new round;
    // one rebased with a line of its own added is not, nor one whose read
    // lines moved within their file, and each is refused.
    #[test]
    fn a_clean_rebase_of_a_read_commit_is_read_and_a_changed_one_is_not() {
        let repo = env::temp_dir().join(format!("merge-net-rebase-{}", std::process::id()));
        let _ = fs::remove_dir_all(&repo);
        fs::create_dir_all(&repo).unwrap();
        git_in(&repo, &["init", "-q", "-b", "main"]);
        // A config dropping the diff's context must not make a moved block
        // the same change.
        git_in(&repo, &["config", "diff.context", "0"]);
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
        let rounds = HashSet::from([round(&reviewed, None)]);
        let search = |head: &str| {
            read_in(
                head,
                &rounds,
                |_| true,
                <[String]>::to_vec,
                |read| same_patch(&repo, head, &main, read, read_patch_id),
            )
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

    // A round over a range read only what lies between its ends, so its
    // head counts as read in full only where the range starts on main: one
    // over a head's newest commit alone doesn't stand for the one under it.
    #[test]
    fn a_range_round_reads_its_head_in_full_only_from_a_base_on_main() {
        let repo = env::temp_dir().join(format!("merge-net-in-full-{}", std::process::id()));
        let _ = fs::remove_dir_all(&repo);
        fs::create_dir_all(&repo).unwrap();
        git_in(&repo, &["init", "-q", "-b", "main"]);
        fs::write(repo.join("w"), "a\n").unwrap();
        git_in(&repo, &["add", "w"]);
        git_in(&repo, &["commit", "-qm", "base"]);
        let main = git_in(&repo, &["rev-parse", "HEAD"]);
        git_in(&repo, &["tag", "trunk"]);
        let unread = commit(&repo, "b\n");
        let head = commit(&repo, "c\n");
        let read = |from: Option<&str>| round(&head, from);
        let full = |read: seen::Read| in_full([&read].into_iter(), &mut on_main_at(&repo, &main));
        assert_eq!(
            full(read(None)),
            HashSet::from([head.clone()]),
            "a change's round"
        );
        assert_eq!(
            full(read(Some(&main))),
            HashSet::from([head.clone()]),
            "from main"
        );
        assert_eq!(
            full(read(Some("trunk"))),
            HashSet::from([head.clone()]),
            "from a name"
        );
        for base in [unread.as_str(), "", "--all", "gone"] {
            assert!(full(read(Some(base))).is_empty(), "from {base:?}");
        }
        fs::remove_dir_all(&repo).unwrap();
    }

    // A review carries by an id read with context and whitespace, so it
    // goes to no edit of its whitespace, nor across an edit beside it.
    #[test]
    fn a_review_goes_to_no_edit_of_whitespace_or_beside_the_change() {
        let repo = env::temp_dir().join(format!("merge-net-read-id-{}", std::process::id()));
        let _ = fs::remove_dir_all(&repo);
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
