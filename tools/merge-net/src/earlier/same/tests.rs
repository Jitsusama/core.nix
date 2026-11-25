use std::{env, fs};

use super::*;
use crate::tests::{commit, git_in};

/// What the review gate says of `head` in `repo`, given the rounds of
/// `read` and no mirror, asking `asks` about a pin.
fn gated(repo: &Path, head: &str, read: &[&str], asks: &pins::tests::Fake) -> Result<(), String> {
    let rounds: HashSet<seen::Read> = read.iter().map(|one| round(one, None)).collect();
    read_in_repo(
        repo,
        (head, "main", &[]),
        || Ok(rounds),
        || mirror(&[], &[], &[]),
        asks,
    )
}

/// What the review gate says of `head` in `repo`, given `rounds` here
/// and a mirror that read `mirrored`, asking `asks` about a pin.
fn gated_with(
    repo: &Path,
    head: &str,
    rounds: Result<HashSet<seen::Read>, String>,
    mirrored: &[&str],
    asks: &pins::tests::Fake,
) -> Result<(), String> {
    read_in_repo(
        repo,
        (head, "main", &[]),
        || rounds,
        || mirror(mirrored, &["peer"], &[]),
        asks,
    )
}

/// A fake that finds no pull request for any commit.
fn unmerged() -> pins::tests::Fake {
    let mut asks = pins::tests::Fake::vouching();
    asks.pulls.clear();
    asks
}

// A head only a mirror's round read passes on that round, asking
// GitHub nothing, whatever its pin would have been told.
#[test]
fn a_pin_a_mirrored_round_read_passes_asking_nothing() {
    let (repo, head) = pins::tests::pinned_to_b("gate-mirrored", None);
    let asks = unmerged();
    assert_eq!(
        gated_with(&repo, &head, Ok(HashSet::new()), &[&head], &asks),
        Ok(()),
        "a pin a mirrored round read was refused"
    );
    assert!(
        asks.asked().is_empty(),
        "a pin a mirrored round read was asked after"
    );
    fs::remove_dir_all(&repo).unwrap();
}

// Rounds that can't be read leave a pin to be asked after: one vouched
// for passes, and one turned down is refused saying both reasons.
#[test]
fn a_pin_beside_rounds_that_cant_be_read_is_asked_after() {
    let (repo, head) = pins::tests::pinned_to_b("gate-unread-rounds", None);
    let broken = || Err("the rounds folder can't be read".to_owned());
    assert_eq!(
        gated_with(&repo, &head, broken(), &[], &pins::tests::Fake::vouching()),
        Ok(()),
        "a vouched pin was refused because the rounds couldn't be read"
    );
    let refused = gated_with(&repo, &head, broken(), &[], &unmerged());
    assert!(
        refused
            .as_ref()
            .is_err_and(|why| why.contains("the rounds folder can't be read")
                && why.contains("moves only flake.lock, but no pull request")),
        "a pin turned down beside unread rounds didn't say both why: {refused:?}"
    );
    fs::remove_dir_all(&repo).unwrap();
}

// A pin the pass vouches for needs no round at all.
#[test]
fn a_pin_of_ours_passes_the_review_gate_without_a_round() {
    let (repo, head) = pins::tests::pinned_to_b("gate", None);
    assert_eq!(
        gated(&repo, &head, &[], &pins::tests::Fake::vouching()),
        Ok(()),
        "a lock-only pin of a merged mycelium commit was refused at the review gate"
    );
    fs::remove_dir_all(&repo).unwrap();
}

// The rounds are asked first, so a head a round read passes without
// GitHub or Nix asked, and only an unread one is asked about as a pin.
#[test]
fn a_pin_a_round_read_passes_asking_nothing_and_a_turned_down_one_is_refused() {
    let (repo, head) = pins::tests::pinned_to_b("gate-declined", None);
    let read = unmerged();
    assert_eq!(
        gated(&repo, &head, &[&head], &read),
        Ok(()),
        "a pin a round read was refused"
    );
    assert!(
        read.asked.borrow().is_empty(),
        "a pin a round read was asked after"
    );
    let refused = gated(&repo, &head, &[], &unmerged());
    assert!(
        refused
            .as_ref()
            .is_err_and(|why| why.contains(&seen::unread(short(&head)))
                && why.contains("moves only flake.lock, but no pull request")),
        "an unread pin turned down wasn't refused, saying why: {refused:?}"
    );
    fs::remove_dir_all(&repo).unwrap();
    let (repo, head) = pins::tests::pinned_to_b("gate-beside", Some("module.nix"));
    assert_eq!(
        gated(&repo, &head, &[], &pins::tests::Fake::vouching()),
        Err(seen::unread(short(&head))),
        "an unread head beside a pin wasn't refused as unread"
    );
    fs::remove_dir_all(&repo).unwrap();
}

// A clean rebase of a pin a round read passes on that round, its
// candidates read from git in the pin's own repo, with GitHub and Nix
// asked nothing.
#[test]
fn a_rebase_of_a_pin_a_round_read_passes_asking_nothing() {
    let (repo, read) = pins::tests::pinned_to_b("gate-rebased", None);
    git_in(&repo, &["checkout", "-q", "main"]);
    fs::write(repo.join("module.nix"), "{ y = 2; }").unwrap();
    git_in(&repo, &["commit", "-qam", "main moves on"]);
    git_in(&repo, &["checkout", "-q", "pin"]);
    git_in(&repo, &["rebase", "-q", "main"]);
    let rebased = git_in(&repo, &["rev-parse", "HEAD"]);
    assert_ne!(rebased, read, "the rebase made no new commit");
    let asks = unmerged();
    assert_eq!(
        gated(&repo, &rebased, &[&read], &asks),
        Ok(()),
        "a clean rebase of a pin a round read was refused"
    );
    assert!(
        asks.asked().is_empty(),
        "a rebase of a pin a round read was asked after"
    );
    fs::remove_dir_all(&repo).unwrap();
}

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
        || mirror(&["bbb"], &["peer"], &[]),
        asks("bbb"),
    );
    assert_eq!(found, Ok(()), "a head a mirrored round read was refused");
    let said = here_or_mirrored(
        HashSet::from([round("aaa", None)]),
        || {
            mirror(
                &["bbb"],
                &["peer"],
                &["the mirror from spare can't be read"],
            )
        },
        asks("ccc"),
    )
    .unwrap_err();
    assert!(
        said.starts_with("unread ccc")
            && said.contains("mirrored from peer were asked too")
            && said.contains("spare can't be read"),
        "the refusal didn't say what was asked: {said}"
    );
}

// With no mirror on the machine, a refusal reads as it always did; with
// one that couldn't be read, it says why; and with no mirrored round
// the rounds here lack, the head isn't asked after a second time over
// the same rounds, while a round still mirroring is said to count once
// it lands.
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
    let mut again = 0;
    let said = here_or_mirrored(
        HashSet::from([round("aaa", None)]),
        || mirror(&["aaa"], &["peer"], &[]),
        |rounds| {
            again += 1;
            asks("ccc")(rounds)
        },
    )
    .unwrap_err();
    assert_eq!(
        again, 1,
        "a mirror adding no round had the head asked after again"
    );
    assert!(
        said.contains("from peer were asked too")
            && said.contains("counts here once its mirror holds it"),
        "the refusal didn't say a round run there counts once mirrored: {said}"
    );
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

// A head that only range rounds read, from a base off main or none at
// all, is refused saying so, since asking that range again reads from
// the same base; one no round read is refused as unread.
#[test]
fn a_head_read_only_from_no_base_is_told_apart_from_one_unread() {
    let off = "0123456789abcdef0123456789abcdef01234567";
    // Another base whose short form is the same, which the fixture's
    // own set can't fold into the first, so the refusal's folding is
    // what names it once.
    let beside = "0123456789ab0000000000000000000000000000";
    let rounds = HashSet::from([
        round("ddd", Some(off)),
        round("ddd", Some(beside)),
        round("ddd", Some("")),
        round("ddd", Some("origin/release-2026")),
        round("eee", Some("elsewhere")),
    ]);
    let on_main = |base: &str| base == "main";
    let refused = read_in("ddd", &rounds, on_main, |_| Vec::new(), |read| read).unwrap_err();
    // A commit id is named short and once, a ref whole, a base that
    // was no commit id as such, and another commit's base not at all.
    assert!(
        refused.contains("read from: 0123456789ab, no commit id recorded, origin/release-2026)"),
        "the bases it read from weren't named: {refused}"
    );
    assert!(!refused.contains("elsewhere"), "{refused}");
    let unread = read_in("ccc", &rounds, on_main, |_| Vec::new(), |read| read).unwrap_err();
    assert_eq!(unread, seen::unread("ccc"));
}

// A base git can't check, as in a repo it can't open, counts as one off
// main, as the gate must, and git's own failure is kept apart from its
// no, so it can be said.
#[test]
fn a_base_git_cannot_check_counts_as_off_main_and_keeps_why() {
    let repo = env::temp_dir().join(format!("merge-net-no-repo-{}", std::process::id()));
    let _ = fs::remove_dir_all(&repo);
    let base = "0123456789abcdef0123456789abcdef01234567";
    let rounds = HashSet::from([round("ddd", Some(base))]);
    let refused = read_in(
        "ddd",
        &rounds,
        on_main_at(&repo, "main"),
        |_| Vec::new(),
        |read| read,
    )
    .unwrap_err();
    assert!(
        refused.contains("couldn't check (read from: 0123456789ab)"),
        "a base git couldn't check wasn't named: {refused}"
    );
    let said = is_ancestor(&repo, base, "main").unwrap_err();
    assert!(!said.is_empty(), "git's failure lost its words");
    let told = couldnt_check(base, &said);
    assert!(
        told.contains("the base 0123456789ab is on main") && told.ends_with(&said),
        "{told}"
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
