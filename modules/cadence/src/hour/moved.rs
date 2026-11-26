//! Whether a change moved on after a council round read it, asked of git,
//! so the hour can tell a queued fix made before the merge from one left as
//! a follow-up.

use std::fs;
use std::io;
use std::path::Path;
use std::process::{Command, Output};

/// Where the harness's history is fetched from.
pub(super) const HARNESS: &str = "https://github.com/Jitsusama/agentic-harness.pi";

/// Whether the change `merge` merged, its second parent, changed after a
/// round read `witness`, asked of a `clone` cadence keeps, made from
/// `seed` when it has none and fetched from `origin`. `None` where git
/// can't say, which goes to the journal with why. Only full commit names
/// reach git, so none reads as an option.
pub(super) fn moved(
    clone: &Path,
    seed: &str,
    origin: &str,
    merge: &str,
    witness: &str,
) -> Option<bool> {
    let sha = |name: &str| name.len() == 40 && name.bytes().all(|b| b.is_ascii_hexdigit());
    if !sha(merge) || !sha(witness) {
        eprintln!("cadence: rounds: {merge} or {witness} is no full commit name");
        return None;
    }
    changed_since(clone, seed, origin, merge, witness)
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
/// or a merge, changes it. So does an edit that only fits it to main's
/// later changes, which git can't tell from a fix. Where that merge
/// conflicts, how the conflict was settled says nothing git can read, so
/// it's an error, and the fix is unread.
///
/// A rebase leaves the head a round read on no branch, so a clone that
/// keeps only refs lacks it: what's missing is asked of `origin` by its
/// name, as GitHub gives a commit it holds, with main beside it so the
/// clone's own refs keep up. Main goes to a ref of cadence's own, as the
/// tick fetches core's main into the same clone and the two can't then
/// meet on one lock.
fn changed_since(
    clone: &Path,
    seed: &str,
    origin: &str,
    merge: &str,
    witness: &str,
) -> io::Result<bool> {
    // A clone is made beside its place and moved in whole, so one there
    // is one finished.
    if !clone.exists() {
        made(clone, seed)?;
    }
    let kept = |name: &str| {
        let commit = format!("{name}^{{commit}}");
        run(clone, &["cat-file", "-e", &commit]).is_ok_and(|out| out.status.success())
    };
    let lacking: Vec<&str> = [merge, witness]
        .into_iter()
        .filter(|name| !kept(name))
        .collect();
    if !lacking.is_empty() {
        let mut args = vec!["fetch", "--quiet", "--no-write-fetch-head", origin];
        args.push(MAIN);
        args.extend_from_slice(&lacking);
        git(clone, "fetching what a round read", &args)?;
    }
    let head = git(
        clone,
        "reading its head",
        &["rev-parse", "--verify", &format!("{merge}^2")],
    )?;
    if head.trim().eq_ignore_ascii_case(witness) {
        return Ok(false);
    }
    let main = format!("{merge}^1");
    // merge-tree finds where the change left main itself, as a rebase does.
    let onto = run(
        clone,
        &[
            "merge-tree",
            "--write-tree",
            "--no-messages",
            &main,
            witness,
        ],
    )?;
    match onto.status.code() {
        Some(0) => {
            let merged = git(
                clone,
                "reading what merged",
                &["rev-parse", &format!("{merge}^{{tree}}")],
            )?;
            Ok(String::from_utf8_lossy(&onto.stdout).trim() != merged.trim())
        }
        Some(1) => Err(io::Error::other(
            "it met main's edits, so git can't say what settling them made of it",
        )),
        _ => Err(failed("merging it onto main", &onto)),
    }
}

/// Where a fetch puts its origin's main in a clone.
pub(super) const MAIN: &str = "+refs/heads/main:refs/remotes/cadence/main";

/// Clones `seed` bare into `clone`, by way of a folder beside it, so a
/// clone killed part way leaves nothing that reads as one, and the next
/// clears it. A seed on this machine copies in seconds what a fetch would
/// bring over the network.
fn made(clone: &Path, seed: &str) -> io::Result<()> {
    let making = clone.with_extension("making");
    if making.exists() {
        fs::remove_dir_all(&making)?;
    }
    let at = clone.parent().unwrap_or(Path::new("."));
    let into = making.to_string_lossy();
    git(
        at,
        "cloning",
        &["clone", "--bare", "--quiet", "--no-tags", seed, &into],
    )?;
    fs::rename(&making, clone)
}

/// Runs git in `dir` with no account or system config, so no setting of
/// Joel's, a rename limit or a diff driver, turns what it judges, and a
/// link that stalls gives up in half a minute rather than holding the
/// card. Upkeep is git's own, as cadence owns its clones.
fn run(dir: &Path, args: &[&str]) -> io::Result<Output> {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "http.lowSpeedLimit=1000",
            "-c",
            "http.lowSpeedTime=30",
        ])
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .map_err(|err| io::Error::other(format!("git didn't start: {err}")))
}

/// [`run`], giving what git printed, or an error naming `step`.
pub(super) fn git(dir: &Path, step: &str, args: &[&str]) -> io::Result<String> {
    let out = run(dir, args).map_err(|err| io::Error::other(format!("{step}: {err}")))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(failed(step, &out))
    }
}

/// What a git run that failed at `step` said.
fn failed(step: &str, out: &Output) -> io::Error {
    io::Error::other(format!(
        "{step}: git {}: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr).trim()
    ))
}

#[cfg(test)]
pub(super) mod tests {
    use std::path::PathBuf;

    use super::*;

    /// Runs git in `at` with a config of its own, so the test reads and
    /// signs with nobody's, giving what it printed.
    pub(in crate::hour) fn git(at: &Path, args: &[&str]) -> String {
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
    pub(in crate::hour) fn commit(at: &Path, file: &str, text: &str) -> String {
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
                "an empty commit",
                own("empty", &|| {
                    git(dir, &["commit", "-q", "--allow-empty", "-m", "nothing"]);
                }),
            ),
        ];
        for (case, (merge, read)) in &cases {
            assert_eq!(moved(dir, url, url, merge, read), Some(false), "{case}");
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
        ];
        for (case, (merge, read)) in &cases {
            assert_eq!(moved(dir, url, url, merge, read), Some(true), "{case}");
        }
    }

    #[test]
    fn a_change_that_met_mains_edits_is_unread_however_it_was_settled() {
        let root = tempfile::tempdir().expect("a scratch folder");
        let (dir, url) = &history(root.path());
        let settled = |change, after: &dyn Fn()| {
            merged(dir, change, (change, "C\n"), (change, "B\n"), &|| {
                git(dir, &["rebase", "-q", "-X", "theirs", "main"]);
                after();
            })
        };
        let cases = [
            ("settled as it was read", settled("settled", &|| ())),
            (
                "fixed as it was settled",
                settled("kept", &|| drop(commit(dir, "kept", "D\n"))),
            ),
        ];
        for (case, (merge, read)) in &cases {
            assert_eq!(moved(dir, url, url, merge, read), None, "{case}");
        }
    }

    #[test]
    fn a_head_rebased_away_is_fetched_and_a_name_is_never_an_option() {
        let root = tempfile::tempdir().expect("a scratch folder");
        let (dir, url) = &history(root.path());
        let rebase = || drop(git(dir, &["rebase", "-q", "main"]));
        // A clone made before the merge lacks it and main as it is now.
        let early = &root.path().join("early");
        let at = early.to_string_lossy();
        git(root.path(), &["clone", "-q", "--bare", url, &at]);
        let (merge, read) = &merged(dir, "re", ("re", "re"), ("main", "re"), &rebase);
        assert_eq!(
            moved(early, url, url, merge, read),
            Some(false),
            "a merge fetched"
        );
        assert_eq!(
            git(early, &["rev-parse", "refs/remotes/cadence/main"]),
            *merge,
            "main wasn't fetched with it"
        );
        // A clone of main alone lacks the head the rebased change was read
        // at, and one not made yet, or killed part way, lacks everything,
        // until each is fetched.
        let there = &root.path().join("there");
        let at = there.to_string_lossy();
        git(
            root.path(),
            &["clone", "-q", "--bare", "--single-branch", url, &at],
        );
        assert_eq!(
            moved(there, url, url, merge, read),
            Some(false),
            "a witness fetched"
        );
        let fresh = &root.path().join("fresh");
        assert_eq!(
            moved(fresh, url, url, merge, read),
            Some(false),
            "a clone made"
        );
        // Made from a seed that lacks the witness, it's fetched from origin.
        let seeded = &root.path().join("seeded");
        let seed = format!("file://{}", there.display());
        assert_eq!(
            moved(seeded, &seed, url, merge, read),
            Some(false),
            "a clone made from its seed and fetched from its origin"
        );
        let half = &root.path().join("half");
        let left = half.with_extension("making");
        std::fs::create_dir(&left).expect("a folder");
        std::fs::write(left.join("objects"), "").expect("a stray file");
        assert_eq!(
            moved(half, url, url, merge, read),
            Some(false),
            "a clone killed part way made again"
        );
        // Its origin answers, and has no such commit.
        assert_eq!(
            moved(there, url, url, merge, &"0".repeat(40)),
            None,
            "a witness nobody has"
        );
        // main reads as a commit, so only the guard keeps either from git.
        let last = git(dir, &["rev-parse", "main^2"]);
        assert_eq!(
            moved(dir, url, url, merge, "main"),
            None,
            "a witness that's no name"
        );
        assert_eq!(
            moved(dir, url, url, "main", &last),
            None,
            "a merge that's no name"
        );
    }
}
