//! Whether a change moved on after a council round read it, asked of git,
//! so the hour can tell a queued fix made before the merge from one left as
//! a follow-up.

use std::io;
use std::path::Path;
use std::process::Command;

use crate::command;

/// Where the harness's history is fetched from.
pub(super) const HARNESS: &str = "https://github.com/Jitsusama/agentic-harness.pi";

/// Whether the change `merge` merged, its second parent, changed after a
/// round read `witness`, asked of a `clone` cadence keeps of `origin`,
/// `None` where git can't say, which goes to the journal with why. Only
/// full commit names reach git, so none reads as an option.
pub(super) fn moved(clone: &Path, origin: &str, merge: &str, witness: &str) -> Option<bool> {
    let sha = |name: &str| name.len() == 40 && name.bytes().all(|b| b.is_ascii_hexdigit());
    if !sha(merge) || !sha(witness) {
        eprintln!("cadence: rounds: {merge} or {witness} is no full commit name");
        return None;
    }
    changed_since(clone, origin, merge, witness)
        .inspect_err(|err| {
            eprintln!("cadence: rounds: can't tell whether {merge} moved after {witness}: {err}");
        })
        .ok()
}

/// Whether what the change `merge` merged does differs from what it did at
/// `witness`. The change as the round read it is merged onto main as it
/// stood, as a rebase would, and the tree that makes is held against the
/// tree that merged. So a rebase, beside main's edits or past work main
/// took another way, a merge from main and an empty commit leave it as it
/// was, and any fix, to a line's place, a binary or a rename, in a rebase
/// or a merge, changes it. Where that merge conflicts, as main's edit met
/// the change's, only the lines each adds are held against each other, so
/// a conflict settled as the round read it leaves it too.
///
/// A rebase leaves the head a round read on no branch, so a clone that
/// keeps only refs lacks it: what's missing is asked of `origin` by its
/// name, as GitHub gives a commit it holds. Like the hour's other fetch, a
/// link that stalls gives up in half a minute, and git starts no upkeep.
fn changed_since(clone: &Path, origin: &str, merge: &str, witness: &str) -> io::Result<bool> {
    let dir = clone.to_string_lossy();
    let quiet = [
        "-c",
        "http.lowSpeedLimit=1000",
        "-c",
        "http.lowSpeedTime=30",
        "-c",
        "maintenance.auto=false",
        "-c",
        "gc.auto=0",
    ];
    // Each step names itself, as git's own word is often none.
    let git = |step: &str, args: &[&str]| {
        let mut all = vec!["-C", &dir];
        all.extend_from_slice(&quiet);
        all.extend_from_slice(args);
        command("git", &all).map_err(|err| io::Error::other(format!("{step}: {err}")))
    };
    if !clone.exists() {
        let mut all = quiet.to_vec();
        all.extend_from_slice(&["clone", "--bare", "--quiet", "--no-tags", origin, &dir]);
        command("git", &all).map_err(|err| io::Error::other(format!("cloning: {err}")))?;
    }
    let kept = |name: &str| git("", &["cat-file", "-e", &format!("{name}^{{commit}}")]).is_ok();
    let lacking: Vec<&str> = [merge, witness]
        .into_iter()
        .filter(|name| !kept(name))
        .collect();
    if !lacking.is_empty() {
        let mut args = vec!["fetch", "--quiet", "--no-write-fetch-head", origin];
        args.extend_from_slice(&lacking);
        git("fetching what a round read", &args)?;
    }
    let head = git(
        "reading its head",
        &["rev-parse", "--verify", &format!("{merge}^2")],
    )?;
    if head.trim().eq_ignore_ascii_case(witness) {
        return Ok(false);
    }
    let main = format!("{merge}^1");
    let base = git(
        "finding where it left main",
        &["merge-base", witness, &main],
    )?;
    let onto = Command::new("git")
        .args(["-C", &dir, "merge-tree", "--write-tree", "--no-messages"])
        .arg(format!("--merge-base={}", base.trim()))
        .args([&main, witness])
        .output()?;
    let rebased = String::from_utf8_lossy(&onto.stdout);
    match onto.status.code() {
        Some(0) => {
            let merged = git(
                "reading what merged",
                &["rev-parse", &format!("{merge}^{{tree}}")],
            )?;
            Ok(rebased.trim() != merged.trim())
        }
        Some(1) => {
            let added = |from: &str, to: &str| {
                let diff = git(
                    "reading what it adds",
                    &[
                        "diff",
                        "-U0",
                        "--no-color",
                        "--no-ext-diff",
                        "--no-textconv",
                        "--no-renames",
                        from,
                        to,
                    ],
                )?;
                io::Result::Ok(
                    diff.lines()
                        .filter(|line| line.starts_with('+'))
                        .map(str::to_owned)
                        .collect::<Vec<_>>(),
                )
            };
            Ok(added(base.trim(), witness)? != added(&main, merge)?)
        }
        _ => Err(io::Error::other(format!(
            "merging it onto main: {}",
            String::from_utf8_lossy(&onto.stderr).trim()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// Runs git in `at` with a config of its own, so the test reads and
    /// signs with nobody's, giving what it printed.
    fn git(at: &Path, args: &[&str]) -> String {
        let out = Command::new("git")
            .arg("-C")
            .arg(at)
            .args(args)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "here")
            .env("GIT_AUTHOR_EMAIL", "here@example.org")
            .env("GIT_COMMITTER_NAME", "here")
            .env("GIT_COMMITTER_EMAIL", "here@example.org")
            .output()
            .expect("git runs");
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
    }

    /// Commits `text` as `file` in `at`, giving the commit. Its message
    /// names the branch, so one edit made on two in the same second is two
    /// commits rather than one.
    fn commit(at: &Path, file: &str, text: &str) -> String {
        std::fs::write(at.join(file), text).expect("a file written");
        git(at, &["add", file]);
        let on = git(at, &["branch", "--show-current"]);
        git(at, &["commit", "-q", "-m", &format!("{file} on {on}")]);
        git(at, &["rev-parse", "HEAD"])
    }

    /// Ten lines, enough that a rename beside an edit is still found as one.
    const LONG: &str = "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\n";

    /// A repository `here` in `root` whose main holds a file for each case
    /// to edit, and which gives a commit on no branch by its name, as GitHub
    /// does, with its address to fetch from.
    fn history(root: &Path) -> (PathBuf, String) {
        let dir = root.join("here");
        std::fs::create_dir(&dir).expect("a repository folder");
        git(&dir, &["init", "-q", "-b", "main"]);
        git(&dir, &["config", "uploadpack.allowAnySHA1InWant", "true"]);
        for (file, text) in [
            ("shared", "one\ntwo\nthree\nfour\n"),
            ("lines", "1\n2\n3\n4\n"),
            ("order", "b\nc\n"),
            ("long", LONG),
            ("settled", "a\n"),
            ("kept", "a\n"),
        ] {
            commit(&dir, file, text);
        }
        let url = format!("file://{}", dir.display());
        (dir, url)
    }

    /// A change making `edit` in `dir`, which a round read, then given
    /// `after` and merged to main past main's own `beside`, giving the merge
    /// and the head read.
    fn merged(
        dir: &Path,
        change: &str,
        edit: (&str, &str),
        beside: (&str, &str),
        after: &dyn Fn(),
    ) -> (String, String) {
        git(dir, &["checkout", "-q", "-b", change, "main"]);
        let read = commit(dir, edit.0, edit.1);
        git(dir, &["checkout", "-q", "main"]);
        commit(dir, beside.0, beside.1);
        git(dir, &["checkout", "-q", change]);
        after();
        git(dir, &["checkout", "-q", "main"]);
        git(dir, &["merge", "-q", "--no-ff", "-m", change, change]);
        (git(dir, &["rev-parse", "HEAD"]), read)
    }

    /// A change that renames a file and edits it, read there, rebased past
    /// main's edit to another line of it, and merged.
    fn renamed(dir: &Path) -> (String, String) {
        git(dir, &["checkout", "-q", "-b", "renamed", "main"]);
        git(dir, &["mv", "long", "moved"]);
        let read = commit(dir, "moved", &LONG.replace("ten", "TEN"));
        git(dir, &["checkout", "-q", "main"]);
        commit(dir, "long", &LONG.replace("one", "ONE"));
        git(dir, &["checkout", "-q", "renamed"]);
        git(dir, &["rebase", "-q", "main"]);
        git(dir, &["checkout", "-q", "main"]);
        git(dir, &["merge", "-q", "--no-ff", "-m", "renamed", "renamed"]);
        (git(dir, &["rev-parse", "HEAD"]), read)
    }

    #[test]
    fn a_change_that_still_does_what_its_round_read_has_not_moved() {
        let root = tempfile::tempdir().expect("a scratch folder");
        let (dir, url) = &history(root.path());
        let own = |change, after: &dyn Fn()| {
            merged(dir, change, (change, change), ("main", change), after)
        };
        let rebase = || drop(git(dir, &["rebase", "-q", "main"]));
        let cases = [
            ("as read", own("as-read", &|| ())),
            (
                "merged from main",
                own("in", &|| {
                    git(dir, &["merge", "-q", "--no-ff", "-m", "in", "main"]);
                }),
            ),
            ("rebased", own("re", &rebase)),
            (
                "rebased beside main's edit",
                merged(
                    dir,
                    "near",
                    ("shared", "one\ntwo\nthree\nFOUR\n"),
                    ("shared", "ONE\ntwo\nthree\nfour\n"),
                    &rebase,
                ),
            ),
            (
                "rebased past lines main added above it",
                merged(
                    dir,
                    "below",
                    ("lines", "1\n2\n3\nFOUR\n"),
                    ("lines", "0\n1\n2\n3\n4\n"),
                    &rebase,
                ),
            ),
            ("renamed and rebased beside main's edit", renamed(dir)),
            (
                "its work taken by main another way",
                merged(dir, "taken", ("same", "x\n"), ("same", "x\n"), &|| {
                    git(dir, &["merge", "-q", "--no-ff", "-m", "taken", "main"]);
                }),
            ),
            (
                "a conflict settled as it was read",
                merged(
                    dir,
                    "settled",
                    ("settled", "C\n"),
                    ("settled", "B\n"),
                    &|| {
                        git(dir, &["rebase", "-q", "-X", "theirs", "main"]);
                    },
                ),
            ),
            (
                "an empty commit",
                own("empty", &|| {
                    git(dir, &["commit", "-q", "--allow-empty", "-m", "nothing"]);
                }),
            ),
        ];
        for (case, (merge, read)) in &cases {
            assert_eq!(moved(dir, url, merge, read), Some(false), "{case}");
        }
    }

    #[test]
    fn a_fix_made_after_its_round_moved_the_change() {
        let root = tempfile::tempdir().expect("a scratch folder");
        let (dir, url) = &history(root.path());
        let own = |change, after: &dyn Fn()| {
            merged(dir, change, (change, change), ("main", change), after)
        };
        let cases = [
            (
                "fixed",
                own("fixed", &|| drop(commit(dir, "a-fix", "a fix"))),
            ),
            (
                "fixed in a merge from main",
                own("evil", &|| {
                    git(dir, &["merge", "-q", "--no-ff", "--no-commit", "main"]);
                    commit(dir, "in-the-merge", "a fix");
                }),
            ),
            (
                "a line it added moved",
                merged(
                    dir,
                    "order",
                    ("order", "b\nc\na\n"),
                    ("main", "order"),
                    &|| {
                        commit(dir, "order", "a\nb\nc\n");
                    },
                ),
            ),
            (
                "a binary it added fixed",
                merged(dir, "blob", ("blob", "\0one"), ("main", "blob"), &|| {
                    commit(dir, "blob", "\0two");
                }),
            ),
            (
                "fixed where main's edit met it",
                merged(dir, "kept", ("kept", "C\n"), ("kept", "B\n"), &|| {
                    git(dir, &["rebase", "-q", "-X", "theirs", "main"]);
                    commit(dir, "kept", "D\n");
                }),
            ),
        ];
        for (case, (merge, read)) in &cases {
            assert_eq!(moved(dir, url, merge, read), Some(true), "{case}");
        }
    }

    #[test]
    fn a_head_rebased_away_is_fetched_and_a_name_is_never_an_option() {
        let root = tempfile::tempdir().expect("a scratch folder");
        let (dir, url) = &history(root.path());
        let rebase = || drop(git(dir, &["rebase", "-q", "main"]));
        let (merge, read) = &merged(dir, "re", ("re", "re"), ("main", "re"), &rebase);
        // A clone of main alone lacks the head the rebased change was read
        // at, and one not made yet lacks everything, until each is fetched.
        let there = &root.path().join("there");
        let at = there.to_string_lossy();
        git(
            root.path(),
            &["clone", "-q", "--bare", "--single-branch", url, &at],
        );
        assert_eq!(
            moved(there, url, merge, read),
            Some(false),
            "a witness fetched"
        );
        let fresh = &root.path().join("fresh");
        assert_eq!(moved(fresh, url, merge, read), Some(false), "a clone made");
        assert!(fresh.join("HEAD").exists(), "no clone was made");
        // Its origin answers, and has no such commit.
        assert_eq!(
            moved(there, url, merge, &"0".repeat(40)),
            None,
            "a witness nobody has"
        );
        assert_eq!(
            moved(dir, url, merge, "as-read"),
            None,
            "a witness that's no name"
        );
        // main's second parent is a real witness, so only the guard keeps
        // the ref from reading as a merge.
        let last = git(dir, &["rev-parse", "main^2"]);
        assert_eq!(
            moved(dir, url, "main", &last),
            None,
            "a merge that's no name"
        );
    }
}
