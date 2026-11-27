//! Red first: each check whose tests a change adds or edits is built again
//! with those tests laid over the base's code, and a check that still
//! passes there is refused, since its tests would pass without the change.
//!
//! On 26 November, across 18 sessions and 42 changes, the most common
//! cause behind a review's findings was a test that passed with its fix
//! undone, or couldn't fail at all, each found and fixed change by change.
//! A test proves a change only if it fails without it, so the gate asks
//! that of every one.
//!
//! Tests are told from code by their paths alone ([`is_test`]): a file in
//! a `tests`, `test` or `checks` folder, a `checks.nix`, or one whose name
//! ends `_test`, `-test`, `.test` or `tests` before its extension. The
//! change's added and edited test files are laid over the base's tree as a
//! commit of their own, never checked out, and each check the gate built
//! is evaluated there. Which checks those tests reach follows from the
//! derivations, with no list of which test is whose:
//!
//! - one whose derivation over the base is the base's reads none of them;
//! - one whose derivation over the base is the tree's reads none of the
//!   change's code, so its tests can't fail without it; it's built there
//!   as the rest are, which the store answers at once;
//! - one that isn't there over the base, or doesn't evaluate there, can't
//!   pass there, and is said;
//! - a lint, named `-passes-clippy` or `-passes-qmllint` as core's are,
//!   reads its folder's tests with its code but runs none of them, so it
//!   passes over the base by design, and it's said, not built there;
//! - one whose derivation moves too with a file nothing names at the root
//!   reads the whole tree, as a formatter or a scan of every file does, so
//!   no file of the change is its test, and it's said, not refused;
//! - each other is built over the base, and refused if it passes. One a
//!   builder failed is red, as it should be; one that failed with no
//!   builder failing it, as when one is lost or a download fails, says
//!   nothing of its tests, and is said. Nix's log of these builds is kept
//!   out of the run's, where merge-net would read a failed build as the
//!   change's.
//!
//! A change of tests alone is said and not built, a moved test file is
//! said and not laid over the base, a test file the change only takes
//! lines from isn't laid there, and tests added inside a code file, which
//! no path tells apart, are said. A candidate of merge-net's train, all
//! merges of changes judged at their own heads, isn't judged again.
//!
//! A test that pins behaviour that was already right passes on the base
//! by design: a commit of the change
//! saying `Red-first: none, <why>` excuses it, and the run says so. A
//! revert, as the net lands one, restores old tests over code that passed
//! them, and is excused too.

use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::building;
use crate::checks::Derivations;
use crate::evaluating::evaluated;
use crate::runs::Run;

/// The trailer a commit excuses its change's tests from failing on the
/// base with, as `Red-first: none, <why>`.
const TRAILER: &str = "Red-first";

/// How a lint's name ends: it reads a folder's tests with its code and runs
/// none of them, so it can't fail for a test's sake.
const LINTS: [&str; 2] = ["-passes-clippy", "-passes-qmllint"];

/// What an added line holds where it adds a test inside a code file.
const INLINE: [&str; 4] = ["#[test]", "#[cfg(test)]", "#[tokio::test]", "def test_"];

/// Where a run judges red first: the repository, its base and the
/// checks' system.
pub struct At<'a> {
    pub root: &'a str,
    pub base: &'a str,
    pub system: &'a str,
}

/// Whether a path is a test's, by the path alone.
pub fn is_test(path: &str) -> bool {
    let (folders, file) = path.rsplit_once('/').unwrap_or(("", path));
    if folders
        .split('/')
        .any(|folder| matches!(folder, "tests" | "test" | "checks"))
        || file == "checks.nix"
    {
        return true;
    }
    let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
    let marked = ["_test", "-test", ".test", "_tests", "-tests", ".tests"];
    matches!(stem, "test" | "tests")
        || stem.starts_with("test_")
        || marked.iter().any(|end| stem.ends_with(end))
}

/// The files a change touched, by what they are.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Change {
    /// Test files added or edited, which are laid over the base.
    pub tests: Vec<String>,
    /// Test files moved, from and to, which pass on the base as before.
    pub moved: Vec<(String, String)>,
    /// Every other file touched, a moved one by both its paths.
    pub code: Vec<String>,
}

impl Change {
    /// The change `git diff --find-renames --name-status` says.
    pub fn read(said: &str) -> Self {
        let mut change = Self::default();
        for line in said.lines().filter(|line| !line.is_empty()) {
            let mut fields = line.split('\t');
            let (Some(status), Some(path)) = (fields.next(), fields.next()) else {
                continue;
            };
            match (status.chars().next(), fields.next()) {
                (Some('R' | 'C'), Some(to)) if is_test(path) && is_test(to) => {
                    change.moved.push((path.to_owned(), to.to_owned()));
                }
                (Some('R' | 'C'), Some(to)) => {
                    change.code.extend([path.to_owned(), to.to_owned()]);
                }
                // A test removed takes nothing from the base's tests.
                (Some('D'), _) if is_test(path) => {}
                _ if is_test(path) => change.tests.push(path.to_owned()),
                _ => change.code.push(path.to_owned()),
            }
        }
        change
    }
}

/// Of the files `git diff --numstat` says, those it adds lines to, or
/// can't count the lines of: a test file the change only takes lines
/// from, as one dropping the cases of behaviour it takes out, adds no
/// test to fail on the base.
pub fn adding(said: &str) -> BTreeSet<String> {
    said.lines()
        .filter_map(|line| {
            let mut fields = line.splitn(3, '\t');
            let added = fields.next()?;
            let path = fields.nth(1)?;
            (added != "0").then(|| path.to_owned())
        })
        .collect()
}

/// How a check's build over the base ended, from its exit and nix's log.
#[derive(Debug, PartialEq, Eq)]
pub enum Built {
    /// It built, so its tests pass there.
    Passed,
    /// A builder ran it and it failed, as a test failing does.
    Red,
    /// It failed without a builder failing, as when one is lost, a
    /// download fails or the disk fills, which says nothing of its tests.
    Unbuilt,
}

/// How a build over the base that `passed` or not, saying `log`, ended.
pub fn built(passed: bool, log: &str) -> Built {
    if passed {
        Built::Passed
    } else if log.contains("failed with exit code") {
        Built::Red
    } else {
        Built::Unbuilt
    }
}

/// The code files whose added lines, as `git diff -U0` gives them, add a
/// test among their code.
pub fn inline_tests(diff: &str) -> Vec<String> {
    let mut found = BTreeSet::new();
    let mut file = None;
    for line in diff.lines() {
        if let Some(path) = line.strip_prefix("+++ ") {
            file = path.strip_prefix("b/");
        } else if line.starts_with('+') && INLINE.iter().any(|marker| line.contains(marker)) {
            found.extend(file.map(str::to_owned));
        }
    }
    found.into_iter().collect()
}

/// Why a change's tests needn't fail on the base.
#[derive(Debug, PartialEq, Eq)]
pub enum Excused {
    /// A commit says why, as `Red-first: none, <why>`.
    Pinned(String),
    /// Every commit reverts.
    Reverts,
}

/// Whether the change's commits, as `git log --no-merges` gives each's
/// subject, a unit separator and its Red-first trailers, one to a line,
/// ended by a record separator, excuse it; and each trailer that doesn't
/// read, to say.
pub fn excused(log: &str) -> (Option<Excused>, Vec<String>) {
    let commits: Vec<(&str, &str)> = log
        .split('\x1e')
        .map(|commit| commit.trim_start_matches('\n'))
        .filter(|commit| !commit.is_empty())
        .map(|commit| commit.split_once('\x1f').unwrap_or((commit, "")))
        .collect();
    let mut unread = Vec::new();
    for (_, trailers) in &commits {
        for trailer in trailers
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
        {
            let why = trailer
                .strip_prefix("none,")
                .map(str::trim)
                .filter(|why| !why.is_empty());
            match why {
                Some(why) => return (Some(Excused::Pinned(why.to_owned())), unread),
                None => unread.push(trailer.to_owned()),
            }
        }
    }
    let reverts = !commits.is_empty()
        && commits
            .iter()
            .all(|(subject, _)| subject.to_lowercase().starts_with("revert"));
    (reverts.then_some(Excused::Reverts), unread)
}

/// What a check is over the base, its derivation there beside the base's
/// and the tree's.
#[derive(Debug, PartialEq, Eq)]
pub enum Over {
    /// Its derivation is the base's: the change's tests aren't its.
    Unreached,
    /// Its derivation is the tree's: the change's code isn't read by it.
    OnlyTests,
    /// It isn't a check over the base.
    Undefined,
    /// It doesn't evaluate over the base, so it fails there.
    Unevaluated,
    /// Neither, so it's built over the base.
    Moved,
}

/// What the check named is over the base, from each side's derivations.
pub fn over(name: &str, before: &Derivations, after: &Derivations, laid: &Derivations) -> Over {
    match laid.get(name) {
        None => Over::Undefined,
        Some(None) => Over::Unevaluated,
        Some(there) if after.get(name) == Some(there) => Over::OnlyTests,
        Some(there) if before.get(name) == Some(there) => Over::Unreached,
        Some(_) => Over::Moved,
    }
}

/// Whether each check the gate built, `asked`, fails with the change's
/// tests laid over the base's code, saying what it found; false when one
/// passes there, as it would without the change. A run that builds
/// nothing asks nothing, and a change not yet `committed` has no commit
/// to take its tests from, so is said and passes.
pub fn judged(
    at: &At,
    asked: Option<Vec<&str>>,
    committed: bool,
    (before, after): (&Derivations, &Derivations),
    noted: &mut Run,
) -> Result<bool, String> {
    let built = asked.unwrap_or_default();
    if built.is_empty() {
        return Ok(true);
    }
    if !committed {
        eprintln!(
            "changed-checks: the change's tests are asked to fail on the base once it's \
             committed"
        );
        return Ok(true);
    }
    let built: Vec<String> = built.iter().map(|name| (*name).to_owned()).collect();
    let range = format!("{}..HEAD", at.base);
    // merge-net's train cuts a candidate of merges alone, each of a change
    // whose own head was judged, so judging them as one would refuse a
    // rider of tests alone beside another's code.
    let own = ["rev-list", "--first-parent", "--no-merges", &range];
    if git(at.root, &own, None, &[])?.is_empty() {
        eprintln!(
            "changed-checks: every commit since the base merges a change judged at its own \
             head, so its tests aren't asked to fail on the base again"
        );
        return Ok(true);
    }
    let format = format!("--format=%s%x1f%(trailers:key={TRAILER},valueonly)%x1e");
    let log = git(at.root, &["log", "--no-merges", &format, &range], None, &[])?;
    let (excuse, unread) = excused(&log);
    for trailer in unread {
        eprintln!(
            "changed-checks: \"{TRAILER}: {trailer}\" doesn't read; a test that pins what was \
             right is excused by \"{TRAILER}: none, <why>\""
        );
    }
    match excuse {
        Some(Excused::Pinned(why)) => {
            eprintln!(
                "changed-checks: {TRAILER}: none, {why}; so no check's tests are asked to fail \
                 on the base"
            );
            return Ok(true);
        }
        Some(Excused::Reverts) => {
            eprintln!(
                "changed-checks: the change reverts, so its tests aren't asked to fail on the base"
            );
            return Ok(true);
        }
        None => {}
    }
    let asked = ["diff", "--find-renames", "--name-status", at.base, "HEAD"];
    let mut change = Change::read(&git(at.root, &asked, None, &[])?);
    say_apart(at, &change)?;
    if change.tests.is_empty() {
        return Ok(true);
    }
    let mut counting = vec!["diff", "--numstat", at.base, "HEAD", "--"];
    counting.extend(change.tests.iter().map(String::as_str));
    let added = adding(&git(at.root, &counting, None, &[])?);
    change.tests.retain(|test| added.contains(test));
    if change.tests.is_empty() {
        eprintln!(
            "changed-checks: the change only takes lines from its tests, so none is asked to \
             fail on the base"
        );
        return Ok(true);
    }
    if change.code.is_empty() {
        eprintln!(
            "changed-checks: the change touches only tests, so none is asked to fail on the base"
        );
        return Ok(true);
    }
    let (lints, built): (Vec<String>, Vec<String>) = built
        .into_iter()
        .partition(|name| LINTS.iter().any(|end| name.ends_with(end)));
    if !lints.is_empty() {
        eprintln!(
            "changed-checks: {} lint the change's tests without running them, so they aren't \
             asked to fail on the base",
            lints.join(", ")
        );
    }
    if built.is_empty() {
        return Ok(true);
    }
    over_the_base(at, &change.tests, &built, (before, after), noted)
}

/// Whether each of the checks `built` fails with `tests` laid over the
/// base's code, saying what it found.
fn over_the_base(
    at: &At,
    tests: &[String],
    built: &[String],
    (before, after): (&Derivations, &Derivations),
    noted: &mut Run,
) -> Result<bool, String> {
    let laid = laid_over(at, tests, &[], "the change's tests over its base")?;
    let flake = format!("git+file://{}?rev={laid}", at.root);
    // A test that reads what the change adds can't even be evaluated over
    // the base, as a checks file naming a new package can't, so it fails
    // there as it should.
    let on_base = match evaluated_as(&flake, at.system, built, noted) {
        Ok(on_base) => on_base,
        Err(why) => {
            eprintln!(
                "changed-checks: the change's tests don't evaluate laid over the base ({why}), \
                 so {} fail there, as they should",
                built.join(", ")
            );
            return Ok(true);
        }
    };
    let mut moved = Vec::new();
    for name in built {
        match over(name, before, after, &on_base) {
            // One reading none of the change's code is the very build that
            // just passed at the head, so it's built over the base too,
            // costing nothing where the store still holds it.
            Over::Moved | Over::OnlyTests => moved.push(name.clone()),
            Over::Undefined => eprintln!(
                "changed-checks: {name} isn't a check over the base, so its tests aren't run there"
            ),
            Over::Unevaluated => eprintln!(
                "changed-checks: {name} doesn't evaluate with the change's tests laid over the \
                 base, so it fails there, as it should"
            ),
            Over::Unreached => {}
        }
    }
    let moved = by_their_files(at, tests, &on_base, moved, noted)?;
    let mut passing = Vec::new();
    let mut red = Vec::new();
    for name in &moved {
        match built_over(&flake, at.system, name, &on_base)? {
            Built::Passed => passing.push(name.as_str()),
            Built::Red => red.push(name.clone()),
            Built::Unbuilt => {}
        }
    }
    say_verdict(&passing, &red, tests);
    Ok(passing.is_empty())
}

/// Build the check `name` over the base, keeping nix's log out of the
/// run's: a test failing there as it should prints nix's words for a
/// failed build, which merge-net reads as a check of the change failing.
/// The log is kept in the cache, and named when it says nothing of the
/// tests.
fn built_over(
    flake: &str,
    system: &str,
    name: &str,
    on_base: &Derivations,
) -> Result<Built, String> {
    let ran = building::building_here(flake, system, &[name], on_base)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("running nix: {error}"))?;
    let log = String::from_utf8_lossy(&ran.stderr);
    let ended = built(ran.status.success(), &log);
    if ended == Built::Unbuilt {
        let folder = crate::cache().join("changed-checks");
        let kept = folder.join(format!("red-first-{name}.log"));
        let _ = fs::create_dir_all(&folder).and_then(|()| fs::write(&kept, &*log));
        eprintln!(
            "changed-checks: {name} couldn't be built over the base, though no builder failed \
             it (nix's words are in {}), so its tests weren't asked to fail there",
            kept.display()
        );
    }
    Ok(ended)
}

/// Say which of the checks built over the base passed there, and so
/// refuse the change's `tests`, or which failed there, `red`, as they
/// should.
fn say_verdict(passing: &[&str], red: &[String], tests: &[String]) {
    let tests = tests.join(", ");
    for name in passing {
        eprintln!(
            "changed-checks: {name} passes with the change's tests laid over the base's code \
             ({tests}), so they'd pass without the change. Make a test fail without it, or say \
             why it pins what was already right with a trailer on a commit of the change:"
        );
    }
    // The trailer goes on a line of its own, so it's copied whole into a
    // commit message rather than retyped from inside a sentence.
    if !passing.is_empty() {
        eprintln!("\n    {TRAILER}: none, <why the tests pin what was already right>\n");
    } else if !red.is_empty() {
        eprintln!(
            "changed-checks: {} fails with the change's tests laid over the base, as it should",
            red.join(", ")
        );
    }
}

/// Say each test file the change moved, which isn't laid over the base,
/// and each code file it added a test among the code of.
fn say_apart(at: &At, change: &Change) -> Result<(), String> {
    for (from, to) in &change.moved {
        eprintln!(
            "changed-checks: {from} moved to {to}, so it isn't asked to fail on the base, \
             where it passed as it was"
        );
    }
    if change.code.is_empty() {
        return Ok(());
    }
    let mut asked = vec!["diff", "-U0", "--no-color", at.base, "HEAD", "--"];
    asked.extend(change.code.iter().map(String::as_str));
    for file in inline_tests(&git(at.root, &asked, None, &[])?) {
        eprintln!(
            "changed-checks: {file} adds a test among its code, which no path tells apart \
             from it, so that test isn't asked to fail on the base"
        );
    }
    Ok(())
}

/// Of `moved`, those that read the change's test files rather than the
/// whole tree: each whose derivation over the base moves again with a
/// file nothing names at the root reads every file, and is said.
fn by_their_files(
    at: &At,
    tests: &[String],
    on_base: &Derivations,
    moved: Vec<String>,
    noted: &mut Run,
) -> Result<Vec<String>, String> {
    if moved.is_empty() {
        return Ok(moved);
    }
    let file = |test: &'_ String| -> String {
        test.rsplit_once('/')
            .map_or(test.as_str(), |(_, file)| file)
            .to_owned()
    };
    let kinds: BTreeSet<String> = tests
        .iter()
        .filter_map(|test| Some(file(test).rsplit_once('.')?.1.to_owned()))
        .collect();
    let probes: Vec<String> = std::iter::once("changed-checks-probe".to_owned())
        .chain(
            kinds
                .iter()
                .map(|kind| format!("changed-checks-probe.{kind}")),
        )
        .collect();
    let message = "the change's tests over its base, with files nothing names";
    let laid = laid_over(at, tests, &probes, message)?;
    let flake = format!("git+file://{}?rev={laid}", at.root);
    let with_probes = evaluated_as(&flake, at.system, &moved, noted)?;
    let (theirs, whole): (Vec<String>, Vec<String>) = moved
        .into_iter()
        .partition(|name| with_probes.get(name) == on_base.get(name));
    if !whole.is_empty() {
        eprintln!(
            "changed-checks: {} read the whole tree, so no file of the change is told apart as \
             their test, and they aren't asked to fail on the base",
            whole.join(", ")
        );
    }
    Ok(theirs)
}

/// Evaluate these checks of the flake, noting it in the run as over the
/// base.
fn evaluated_as(
    flake: &str,
    system: &str,
    names: &[String],
    noted: &mut Run,
) -> Result<Derivations, String> {
    let mut evaluation = Run::evaluating("over base");
    let found = evaluated(flake, system, names, &mut evaluation)?;
    noted.evaluated(evaluation);
    Ok(found)
}

/// A commit holding the base's tree with HEAD's `files` laid over it, and
/// an empty file at each of `empty`, made with git's plumbing alone, so
/// nothing is checked out, no ref moves and nothing is signed. It takes
/// the base's author and committer, so it differs from the base only in
/// its files and its revision.
fn laid_over(at: &At, files: &[String], empty: &[String], message: &str) -> Result<String, String> {
    let index = Index::new()?;
    let indexed = [("GIT_INDEX_FILE", index.0.as_os_str())];
    git(at.root, &["read-tree", at.base], None, &indexed)?;
    let mut listing = Vec::new();
    if !files.is_empty() {
        let mut asked = vec!["ls-tree", "-z", "HEAD", "--"];
        asked.extend(files.iter().map(String::as_str));
        listing.extend(git(at.root, &asked, None, &[])?.into_bytes());
        if !listing.ends_with(b"\0") {
            listing.push(0);
        }
    }
    if !empty.is_empty() {
        let blob = git(at.root, &["hash-object", "-w", "--stdin"], Some(b""), &[])?;
        for path in empty {
            listing.extend(format!("100644 blob {blob}\t{path}\0").into_bytes());
        }
    }
    let info = ["update-index", "-z", "--index-info"];
    git(at.root, &info, Some(&listing), &indexed)?;
    let tree = git(at.root, &["write-tree"], None, &indexed)?;
    let base = git(at.root, &["cat-file", "commit", at.base], None, &[])?;
    let people: String = base
        .lines()
        .filter(|line| line.starts_with("author ") || line.starts_with("committer "))
        .flat_map(|line| [line, "\n"])
        .collect();
    let commit = format!(
        "tree {tree}\nparent {}\n{people}\nchanged-checks: {message}\n",
        at.base
    );
    let asked = ["hash-object", "-t", "commit", "-w", "--stdin"];
    git(at.root, &asked, Some(commit.as_bytes()), &[])
}

/// An index of the run's own, gone once dropped.
struct Index(PathBuf);

impl Index {
    fn new() -> Result<Self, String> {
        let folder = crate::cache().join("changed-checks");
        fs::create_dir_all(&folder)
            .map_err(|error| format!("making {}: {error}", folder.display()))?;
        let path = folder.join(format!("red-first-{}.index", std::process::id()));
        let _ = fs::remove_file(&path);
        Ok(Self(path))
    }
}

impl Drop for Index {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// Run git in the repository at `root`, given `input` and these variables,
/// and give what it printed, trimmed of its last line's end.
fn git(
    root: &str,
    arguments: &[&str],
    input: Option<&[u8]>,
    told: &[(&str, &std::ffi::OsStr)],
) -> Result<String, String> {
    let failed = |error: std::io::Error| format!("running git: {error}");
    let mut child = Command::new("git")
        .arg("-C")
        .arg(Path::new(root))
        .args(arguments)
        .envs(told.iter().copied())
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(failed)?;
    if let (Some(input), Some(mut stdin)) = (input, child.stdin.take()) {
        stdin.write_all(input).map_err(failed)?;
    }
    let ran = child.wait_with_output().map_err(failed)?;
    if !ran.status.success() {
        return Err(format!(
            "git {} failed: {}",
            arguments.join(" "),
            ran.status
        ));
    }
    let said = String::from_utf8(ran.stdout)
        .map_err(|_| format!("git {} printed what isn't text", arguments.join(" ")))?;
    Ok(said.trim_end_matches('\n').to_owned())
}

#[cfg(test)]
mod tests;
