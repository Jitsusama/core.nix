use std::fmt::Write as _;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::{env, fs};

use super::*;
use crate::tests::{commit, git_in};

/// A repo of its own for a test named `name`, with `main` at one commit,
/// and that commit.
fn repo(name: &str) -> (PathBuf, String) {
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

/// What folding says of `commit` in `repo` by `rounds`, with only `base`
/// a range's base on main.
fn said(repo: &Path, commit: &str, rounds: &[seen::Read], base: &str) -> Folded {
    let rounds: HashSet<seen::Read> = rounds.iter().cloned().collect();
    read(repo, commit, "main", &rounds, |from| from == base)
}

/// Whether `folded` reads the head, its words holding `words`.
fn reads(folded: &Folded, words: &str) -> bool {
    matches!(folded, Folded::Read(said) if said.contains(words))
}

/// Whether `folded` refuses the head, its words holding each of `words`.
fn refuses(folded: &Folded, words: &[&str]) -> bool {
    matches!(folded, Folded::Refused(why) if words.iter().all(|word| why.contains(word)))
}

// A change below a stack's tip is read by the stack round that read the
// tip, but not by a council at the tip, nor by a stack round over a range
// from a base off main.
#[test]
fn a_change_below_a_stacks_tip_is_read_by_its_stack_round() {
    let (repo, base) = repo("stack");
    let lower = commit(&repo, "lower\n");
    let tip = commit_file(&repo, "upper", "tip\n");
    let read = said(&repo, &lower, &[round(&tip, None, true)], &base);
    assert!(
        reads(&read, "stack round"),
        "a stack round of the tip didn't read the change below it: {read:?}"
    );
    assert_eq!(
        said(&repo, &lower, &[round(&tip, None, false)], &base),
        Folded::Silent,
        "a council of the tip read the change below it"
    );
    assert_eq!(
        said(&repo, &lower, &[round(&tip, Some(&lower), true)], &base),
        Folded::Silent,
        "a stack round from a base off main read the change below it"
    );
    assert!(
        !reads(&said(&repo, &tip, &[round(&lower, None, true)], &base), ""),
        "a stack round of a lower change read the tip above it"
    );
    fs::remove_dir_all(&repo).unwrap();
}

// A lower change pointed back at its first commit, which put in a bug its
// second took out, isn't what a stack round of the tip read of that file,
// so only the second is read.
#[test]
fn a_lower_commit_a_later_one_changed_isnt_read_by_the_stack_round() {
    let (repo, base) = repo("stack-back");
    let buggy = commit(&repo, "bug\n");
    let fixed = commit(&repo, "fixed\n");
    let tip = commit_file(&repo, "upper", "tip\n");
    let rounds = [round(&tip, None, true)];
    let refused = said(&repo, &buggy, &rounds, &base);
    assert!(
        refuses(&refused, &["changes above it", "w,", short(&fixed)]),
        "a lower commit whose file a later one changed was read, or not said why: {refused:?}"
    );
    let read = said(&repo, &fixed, &rounds, &base);
    assert!(
        reads(&read, "stack round"),
        "a lower commit nothing above changed was left unread: {read:?}"
    );
    fs::remove_dir_all(&repo).unwrap();
}

// Asking after a base is a git, and the rounds hold thousands, so only a
// stack tip git finds holding the head has its base put to `on_main`: an
// unread head among many rounds asks none, and a stack's lower change one.
#[test]
fn only_a_tip_holding_the_head_has_its_base_asked_after() {
    let (repo, base) = repo("asks");
    let lower = commit(&repo, "lower\n");
    let tip = commit_file(&repo, "upper", "tip\n");
    git_in(&repo, &["checkout", "-qb", "elsewhere", &base]);
    let mut rounds: HashSet<seen::Read> = (0..20)
        .map(|n| {
            let other = commit_file(&repo, "other", &format!("{n}\n"));
            round(&other, Some(&format!("base-{n}")), n % 2 == 0)
        })
        .collect();
    let gone = "0123456789abcdef0123456789abcdef01234567";
    rounds.insert(round(gone, Some("base-gone"), true));
    git_in(&repo, &["checkout", "-qb", "unread", &base]);
    let unread = commit_file(&repo, "unread", "x\n");
    let mut asked = Vec::new();
    let folded = read(&repo, &unread, "main", &rounds, |from| {
        asked.push(from.to_owned());
        false
    });
    assert_eq!(folded, Folded::Silent, "an unread head was said of");
    assert!(
        asked.is_empty(),
        "an unread head asked after bases: {asked:?}"
    );
    rounds.insert(round(&tip, Some(&base), true));
    let folded = read(&repo, &lower, "main", &rounds, |from| {
        asked.push(from.to_owned());
        from == base
    });
    assert!(reads(&folded, "stack round"), "{folded:?}");
    assert_eq!(
        asked,
        [base.as_str()],
        "bases beside the tip's were asked after"
    );
    fs::remove_dir_all(&repo).unwrap();
}

// A head is read when what came after its read commit touches only docs
// and tests, and isn't once a commit touches anything else.
#[test]
fn docs_and_tests_after_a_read_commit_keep_the_head_read() {
    let (repo, base) = repo("docs");
    let reviewed = commit(&repo, "code\n");
    let rounds = [round(&reviewed, None, false)];
    commit_file(&repo, "docs/guide.md", "words\n");
    commit_file(&repo, "modules/x/README.md", "words\n");
    let tested = commit_file(&repo, "src/a.test.ts", "case\n");
    let read = said(&repo, &tested, &rounds, &base);
    assert!(
        reads(&read, "only docs and tests"),
        "docs and tests after a read commit left the head unread: {read:?}"
    );
    let coded = commit_file(&repo, "modules/x/tests.nix", "check\n");
    let refused = said(&repo, &coded, &rounds, &base);
    assert!(
        refuses(&refused, &[short(&coded), "modules/x/tests.nix"]),
        "a code change after the read commit was taken as read, or not named: {refused:?}"
    );
    fs::remove_dir_all(&repo).unwrap();
}

/// What folding says of `file` committed alone after `reviewed`, which a
/// change's round read, on a branch of its own.
fn alone_after(repo: &Path, reviewed: &str, file: &str) -> Folded {
    git_in(repo, &["checkout", "-q", "-B", "alone", reviewed]);
    let head = commit_file(repo, file, "words\n");
    said(repo, &head, &[round(reviewed, None, false)], "")
}

// A doc something reads as an input, a skill, an agents' guide, or the
// flake's public tests, is no doc, each on its own after a read commit;
// a doc without `.md` under docs/ is one, and so is an upper-case `.MD`.
#[test]
fn only_a_doc_nothing_reads_as_an_input_keeps_the_head_read() {
    let (repo, _) = repo("inputs");
    commit_file(
        &repo,
        "modules/cadence/src/hour/good.rs",
        "const GOOD: &str = include_str!(\"../../good.md\");\n",
    );
    let reviewed = commit_file(&repo, "modules/cadence/home.nix", "{ x = ./SKILL.md; }\n");
    for (file, why) in [
        (
            "modules/cadence/good.md",
            "modules/cadence/src/hour/good.rs reads",
        ),
        ("modules/cadence/SKILL.md", "agents read"),
        ("modules/x/SKILL.md", "agents read"),
        ("AGENTS.md", "agents read"),
        (".agents/skills/x/steps.md", "agents read"),
        ("modules/theme/web/skill/notes.md", "agents read"),
        ("tests/rules/x.nix", "neither a doc nor a test"),
    ] {
        let refused = alone_after(&repo, &reviewed, file);
        assert!(
            refuses(&refused, &[file, why]),
            "{file} after a read commit was taken as a doc, or not said why: {refused:?}"
        );
    }
    for file in ["docs/diagram.svg", "README.MD", "modules/cadence/notes.md"] {
        let read = alone_after(&repo, &reviewed, file);
        assert!(
            reads(&read, "only docs and tests"),
            "{file} alone after a read commit left the head unread: {read:?}"
        );
    }
    fs::remove_dir_all(&repo).unwrap();
}

// A path is resolved from the folder of the file naming it, and one that
// climbs out of the repo names nothing in it.
#[test]
fn a_named_path_is_resolved_from_its_files_folder() {
    for (file, path, wanted) in [
        ("a/b/c.rs", "../../d.md", Some("d.md")),
        ("a/b/c.rs", "./d.md", Some("a/b/d.md")),
        ("a/c.rs", ".././a/./d.md", Some("a/d.md")),
        ("c.rs", "../d.md", None),
    ] {
        assert_eq!(
            resolved(file, path).as_deref(),
            wanted,
            "{path} from {file}"
        );
    }
}

// A range round counts for what follows its commit only from a base on
// main: one from an unread commit below it reads none of that commit's
// change below it.
#[test]
fn a_range_round_off_main_reads_nothing_after_its_commit() {
    let (repo, base) = repo("range");
    let lower = commit(&repo, "unread\n");
    let reviewed = commit(&repo, "code\n");
    let head = commit_file(&repo, "docs/guide.md", "words\n");
    let refused = said(
        &repo,
        &head,
        &[round(&reviewed, Some(&lower), false)],
        &base,
    );
    assert!(
        refuses(&refused, &["base off main"]),
        "a range round off main read the docs after it: {refused:?}"
    );
    let read = said(&repo, &head, &[round(&reviewed, Some(&base), false)], &base);
    assert!(
        reads(&read, "only docs and tests"),
        "a range round from main didn't read the docs after it: {read:?}"
    );
    fs::remove_dir_all(&repo).unwrap();
}

// A docs commit doesn't carry a read past a code commit before it, nor
// past a merge of main edited by hand, and the refusal names that one.
#[test]
fn docs_after_code_or_an_edited_merge_keep_the_head_unread() {
    let (repo, base) = repo("docs-after");
    let reviewed = commit(&repo, "code\n");
    let rounds = [round(&reviewed, None, false)];
    let coded = commit(&repo, "more code\n");
    let head = commit_file(&repo, "docs/guide.md", "words\n");
    let refused = said(&repo, &head, &rounds, &base);
    assert!(
        refuses(&refused, &[short(&coded), "touches w"]),
        "docs after a code commit kept the head read: {refused:?}"
    );
    git_in(&repo, &["checkout", "-q", "main"]);
    commit_file(&repo, "elsewhere", "main's\n");
    git_in(&repo, &["checkout", "-q", "-B", "change", &reviewed]);
    git_in(&repo, &["merge", "-q", "--no-edit", "main"]);
    fs::write(repo.join("w"), "code and more\n").unwrap();
    git_in(&repo, &["commit", "-q", "-a", "--amend", "--no-edit"]);
    let edited = git_in(&repo, &["rev-parse", "HEAD"]);
    let head = commit_file(&repo, "docs/guide.md", "words\n");
    let refused = said(&repo, &head, &rounds, &base);
    assert!(
        refuses(&refused, &[short(&edited), "doesn't make"]),
        "docs after a hand-edited merge kept the head read: {refused:?}"
    );
    fs::remove_dir_all(&repo).unwrap();
}

// A clean merge of main after a read commit keeps the head read; a merge
// with a hand-made line doesn't.
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
        reads(&read, "merged main"),
        "a clean merge of main left the head unread: {read:?}"
    );
    fs::write(repo.join("w"), "code and more\n").unwrap();
    git_in(&repo, &["commit", "-q", "-a", "--amend", "--no-edit"]);
    let edited = git_in(&repo, &["rev-parse", "HEAD"]);
    assert!(
        refuses(&said(&repo, &edited, &rounds, &base), &["doesn't make"]),
        "a merge with a hand-made line was taken as read"
    );
    fs::remove_dir_all(&repo).unwrap();
}

// A merge of a branch that isn't main is refused even when that branch
// brings only a doc. The walk judges each commit such a branch brings in
// on its own, so the check of main is defence in depth: it keeps a clean
// merge meaning main's, and this is the one case only it refuses.
#[test]
fn a_merge_of_another_branch_is_refused_as_defence_in_depth_even_with_only_docs() {
    let (repo, base) = repo("merge-other");
    let reviewed = commit(&repo, "code\n");
    git_in(&repo, &["checkout", "-qb", "other", &base]);
    commit_file(&repo, "docs/theirs.md", "words\n");
    git_in(&repo, &["checkout", "-q", "change"]);
    git_in(&repo, &["merge", "-q", "--no-edit", "other"]);
    let other = git_in(&repo, &["rev-parse", "HEAD"]);
    let refused = said(&repo, &other, &[round(&reviewed, None, false)], &base);
    assert!(
        refuses(&refused, &[short(&other), "isn't main"]),
        "a merge of a branch that isn't main was taken as read: {refused:?}"
    );
    fs::remove_dir_all(&repo).unwrap();
}

/// Writes `commits` linear commits on the branch `long` from `base` in
/// `repo` by one git, the first changing `w` and each after it a doc of
/// its own, and gives them, oldest first.
fn long_change(repo: &Path, base: &str, commits: usize) -> Vec<String> {
    let mut stream = String::new();
    for n in 1..=commits {
        let (file, words) = if n == 1 {
            ("w".to_owned(), "code\n".to_owned())
        } else {
            (format!("docs/{n}.md"), format!("{n}\n"))
        };
        let from = if n == 1 {
            base.to_owned()
        } else {
            format!(":{}", n - 1)
        };
        write!(
            stream,
            "commit refs/heads/long\nmark :{n}\ncommitter t <t@t> 0 +0000\ndata 1\n{}\nfrom \
             {from}\nM 100644 inline {file}\ndata {}\n{words}\n",
            n % 10,
            words.len()
        )
        .unwrap();
    }
    let mut import = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["fast-import", "--quiet"])
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    import
        .stdin
        .take()
        .unwrap()
        .write_all(stream.as_bytes())
        .unwrap();
    assert!(import.wait().unwrap().success(), "git fast-import failed");
    git_in(repo, &["rev-list", "--reverse", "long", "^main"])
        .lines()
        .map(str::to_owned)
        .collect()
}

// A change of as many commits as are walked is walked whole, and one past
// it is refused saying so, since git stops listing there.
#[test]
fn a_change_past_the_most_walked_is_refused_saying_so() {
    let (repo, base) = repo("long");
    let change = long_change(&repo, &base, MOST_WALKED + 1);
    let rounds = [round(&change[0], None, false)];
    let read = said(&repo, &change[MOST_WALKED - 1], &rounds, &base);
    assert!(
        reads(&read, "only docs and tests"),
        "a change of {MOST_WALKED} commits wasn't walked whole: {read:?}"
    );
    let refused = said(&repo, &change[MOST_WALKED], &rounds, &base);
    assert!(
        refuses(&refused, &["more than 500"]),
        "a change past {MOST_WALKED} commits was walked: {refused:?}"
    );
    fs::remove_dir_all(&repo).unwrap();
}
