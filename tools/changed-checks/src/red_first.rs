//! Red first: each check whose tests a change adds or edits is built again
//! with those tests laid over the base's code, and a change none of whose
//! checks fails there is refused, since its tests would pass without it.
//!
//! On 26 November, across 18 sessions and 42 changes, the most common
//! cause behind a review's findings was a test that passed with its fix
//! undone, or couldn't fail at all, each found and fixed change by change.
//! A test proves a change only if it fails without it, so the gate asks
//! that of every one.
//!
//! Tests are told from code by their paths alone ([`is_test`]): a file in
//! a `tests`, `test` or `checks` folder, a `checks.nix`, or one whose name
//! starts `test_` or ends `_test`, `-test`, `.test` or `tests` before its
//! extension. The root's `tests/lib` folder and `checks.nix` are the
//! harness every test runs in, with no test written in them, so they're
//! code to red first. The change's added and edited test files are laid over the base's tree as a
//! commit of their own, never checked out, and each check the gate built
//! is evaluated there. Which checks those tests reach follows from the
//! derivations, with no list of which test is whose:
//!
//! - one whose derivation over the base is the base's reads none of them;
//! - one whose derivation over the base is the tree's reads none of the
//!   change's code, so it passes there whatever its tests say, as a scan
//!   counting every VM test's subtests does, and it's said, not built;
//! - one that doesn't evaluate there fails there, as it should, which
//!   proves the tests;
//! - one that isn't there over the base can't be asked there, so it's
//!   said, and since it might be the one running them, nothing is refused;
//! - a lint, named `-passes-clippy`, `-passes-qmllint` or
//!   `-compiles-for-macos` as core's are, reads its folder's tests with
//!   its code but runs none of them, so it passes over the base by design,
//!   and it's said, not built there;
//! - one whose derivation moves too with a file nothing names at the root
//!   reads the whole tree, as a formatter or a scan of every file does, so
//!   no file of the change is its test, and it's said, not refused;
//! - each other is built over the base, the example systems and VM tests
//!   last, until one a builder fails, red as the change's tests should
//!   be. A check may carry tests without running
//!   them, as a system holding a package whose source holds its tests
//!   does, and passes there by design, so one red proves the change and
//!   the rest go unbuilt. The change is refused when every one passes.
//!   One that failed with no builder failing it, as when one is lost or a
//!   download fails, says nothing of its tests, is said, and refuses
//!   nothing. Nix's log of these builds is kept out of the run's, where
//!   merge-net would read a failed build as the change's.
//!
//! A change of tests alone is said and not built, a test file moved
//! whole is said and not laid over the base, a test file the change only
//! takes lines from isn't laid there, nor is a new Rust test file among a
//! crate's sources that only the change's code declares (one a laid test
//! file declares is laid with it), and tests added inside a code file,
//! which no path tells apart, are said. A candidate of merge-net's train,
//! all merges of changes judged at their own heads, isn't judged again.
//! Red first judges on Linux alone: a Mac's checks that a change's tests
//! reach often only evaluate a system, while the check running them may
//! be Linux's alone, so a Mac's run would refuse what Linux's proved.
//! When red first can't judge at all, as when nix can't evaluate the
//! laid tests or git can't lay them, the run says so and refuses nothing.
//!
//! A test that pins behaviour that was already right passes on the base
//! by design: a commit of the change saying `Red-first: none, <why>`
//! excuses it, and the run says so. A
//! revert, as the net lands one, restores old tests over code that passed
//! them, and is excused too.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Stdio;

use laying::{git, laid_over};

use crate::building;
use crate::checks::Derivations;
use crate::evaluating::evaluated;
use crate::runs::Run;

/// The trailer a commit excuses its change's tests from failing on the
/// base with, as `Red-first: none, <why>`.
const TRAILER: &str = "Red-first";

/// How a lint's name ends: it reads a folder's tests with its code and runs
/// none of them, so it can't fail for a test's sake.
const LINTS: [&str; 3] = ["-passes-clippy", "-passes-qmllint", "-compiles-for-macos"];

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
    if path.starts_with("tests/lib/") || path == "checks.nix" {
        return false;
    }
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

/// Whether a test file a change adds is one only the change's code can
/// declare: a Rust file among a crate's sources is compiled only where a
/// module names it, and a base naming a file it lacks wouldn't build, so
/// laid over the base it's never compiled, and its check passes there.
fn declared_by_code(path: &str) -> bool {
    let folders = path.rsplit_once('/').map_or("", |(folders, _)| folders);
    Path::new(path)
        .extension()
        .is_some_and(|extension| extension == "rs")
        && folders.split('/').any(|folder| folder == "src")
}

/// The files that could declare the Rust module file at `path`: its
/// parent module's, beside its folder or in it, or its crate's root.
fn declarers(path: &str) -> Vec<String> {
    let Some((folder, _)) = path.rsplit_once('/') else {
        return Vec::new();
    };
    let mut found = vec![format!("{folder}.rs"), format!("{folder}/mod.rs")];
    if folder == "src" || folder.ends_with("/src") {
        found.extend([format!("{folder}/lib.rs"), format!("{folder}/main.rs")]);
    }
    found.retain(|declarer| declarer != path);
    found
}

/// The files a change touched, by what they are.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Change {
    /// Test files added or edited, which are laid over the base.
    pub tests: Vec<String>,
    /// Test files moved, from and to, which pass on the base as before.
    pub moved: Vec<(String, String)>,
    /// Test files added that only the change's code declares, which
    /// would never run over the base.
    pub declared: Vec<String>,
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
            // Only a file moved whole passed on the base as it is; one
            // moved with lines of its own is as a test edited.
            let whole = status.get(1..) == Some("100");
            match (status.chars().next(), fields.next()) {
                (Some('R' | 'C'), Some(to)) if is_test(path) && is_test(to) && whole => {
                    change.moved.push((path.to_owned(), to.to_owned()));
                }
                (Some('R' | 'C'), Some(to)) if is_test(path) && is_test(to) => {
                    change.tests.push(to.to_owned());
                }
                (Some('R' | 'C'), Some(to)) => {
                    change.code.extend([path.to_owned(), to.to_owned()]);
                }
                // A test removed takes nothing from the base's tests.
                (Some('D'), _) if is_test(path) => {}
                (Some('A'), _) if is_test(path) && declared_by_code(path) => {
                    change.declared.push(path.to_owned());
                }
                _ if is_test(path) => change.tests.push(path.to_owned()),
                _ => change.code.push(path.to_owned()),
            }
        }
        // A new file a laid test file declares is laid with it, or the
        // laid one names a module the base lacks, and fails to build
        // there whatever its tests say.
        let (laid, declared): (Vec<String>, Vec<String>) = std::mem::take(&mut change.declared)
            .into_iter()
            .partition(|file| {
                declarers(file)
                    .iter()
                    .any(|declarer| change.tests.contains(declarer))
            });
        change.tests.extend(laid);
        change.declared = declared;
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

/// Whether a check the gate built, `asked`, fails with the change's
/// tests laid over the base's code, saying what it found; false when
/// each built there passes, as it would without the change. A run that builds
/// nothing asks nothing, and a change not yet `committed` has no commit
/// to take its tests from, so is said and passes. Red first failing for
/// its own reasons is said and refuses nothing: its checks have passed.
pub fn judged(
    at: &At,
    asked: Option<Vec<&str>>,
    committed: bool,
    (before, after): (&Derivations, &Derivations),
    noted: &mut Run,
) -> bool {
    judging(at, asked, committed, (before, after), noted).unwrap_or_else(|why| {
        eprintln!(
            "changed-checks: red first couldn't judge the change ({why}), so its tests \
             aren't asked to fail on the base"
        );
        true
    })
}

/// Whether the change's tests fail on the base, as `judged` says.
fn judging(
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
    if !at.system.ends_with("-linux") {
        eprintln!(
            "changed-checks: the change's tests are asked to fail on the base on Linux, where \
             its checks run them, not on {}",
            at.system
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
    // A test reading what the change adds stops nix's whole evaluation
    // over the base, as a checks file naming a new package does, and so
    // does nix failing for its own reasons, which no exit tells apart; so
    // it's said, and refuses nothing.
    let on_base = evaluated_as(&flake, at.system, built, noted).map_err(|why| {
        format!(
            "the checks don't evaluate with its tests laid over the base, as when a test reads \
             what the change adds: {why}"
        )
    })?;
    let mut moved = Vec::new();
    let mut only_tests = Vec::new();
    let mut undefined = false;
    for name in built {
        match over(name, before, after, &on_base) {
            Over::Moved => moved.push(name.clone()),
            // One reading none of the change's code is the very build that
            // just passed at the head, so its passing says nothing.
            Over::OnlyTests => only_tests.push(name.as_str()),
            Over::Undefined => {
                eprintln!(
                    "changed-checks: {name} isn't a check over the base, so it can't be asked \
                     to fail there, and the change isn't refused"
                );
                undefined = true;
            }
            Over::Unevaluated => {
                eprintln!(
                    "changed-checks: {name} doesn't evaluate with the change's tests laid over \
                     the base, so it fails there, as it should"
                );
                return Ok(true);
            }
            Over::Unreached => {}
        }
    }
    if !only_tests.is_empty() {
        eprintln!(
            "changed-checks: {} read the change's tests but none of its code, so they pass on \
             the base whatever the tests say, and aren't asked to fail there",
            only_tests.join(", ")
        );
    }
    if moved.is_empty() && only_tests.is_empty() && !undefined {
        eprintln!(
            "changed-checks: no check built reads the change's tests ({}), so none is asked \
             to fail on the base",
            tests.join(", ")
        );
    }
    // One not there over the base might have been the check running them.
    if undefined {
        return Ok(true);
    }
    let mut moved = by_their_files(at, tests, &on_base, moved, noted)?;
    // The systems carrying a package, and the VM tests, cost most to build,
    // and a system only carrying the tests passes there by design, so the
    // checks that run tests go first.
    moved.sort_by_key(|name| name.starts_with("example-") || name.starts_with("vm-"));
    // A check that only carries the tests, as a system holding a package
    // whose source holds them does, passes over the base by design, so
    // one failing there proves them, and the rest needn't be built.
    let mut passing = Vec::new();
    let mut unbuilt = false;
    for (done, name) in moved.iter().enumerate() {
        // A build's log is held back, so the run says what it waits on.
        eprintln!(
            "changed-checks: building {name} over the base ({} of {})",
            done + 1,
            moved.len()
        );
        match built_over(&flake, at.system, name, &on_base)? {
            Built::Red => {
                eprintln!(
                    "changed-checks: {name} fails with the change's tests laid over the base, \
                     as it should"
                );
                return Ok(true);
            }
            Built::Passed => passing.push(name.as_str()),
            Built::Unbuilt => unbuilt = true,
        }
    }
    // One unbuilt might have been the check that runs the tests.
    if unbuilt || passing.is_empty() {
        return Ok(true);
    }
    say_refused(&passing, tests);
    Ok(false)
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
        // Runs judging the same check at once each keep their own.
        let kept = folder.join(format!("red-first-{}-{name}.log", std::process::id()));
        let _ = fs::create_dir_all(&folder).and_then(|()| fs::write(&kept, &*log));
        eprintln!(
            "changed-checks: {name} couldn't be built over the base, though no builder failed \
             it (nix's words are in {}), so its tests weren't asked to fail there",
            kept.display()
        );
    }
    Ok(ended)
}

/// Say that each check built over the base passed there, none failing,
/// and so refuse the change's `tests`.
fn say_refused(passing: &[&str], tests: &[String]) {
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
    eprintln!("\n    {TRAILER}: none, <why the tests pin what was already right>\n");
}

/// Say each test file the change moved, or added for its code alone to
/// declare, which isn't laid over the base, and each code file it added a
/// test among the code of.
fn say_apart(at: &At, change: &Change) -> Result<(), String> {
    for (from, to) in &change.moved {
        eprintln!(
            "changed-checks: {from} moved to {to}, so it isn't asked to fail on the base, \
             where it passed as it was"
        );
    }
    for file in &change.declared {
        eprintln!(
            "changed-checks: {file} is a new test file only the change's code declares, so \
             it wouldn't run on the base, and isn't asked to fail there"
        );
    }
    if change.code.is_empty() {
        return Ok(());
    }
    // The prefixes are named, since a person's git may be set to leave
    // them out, and the files are read from after them.
    let mut asked = vec![
        "diff",
        "-U0",
        "--no-color",
        "--src-prefix=a/",
        "--dst-prefix=b/",
        at.base,
        "HEAD",
        "--",
    ];
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

mod laying;

#[cfg(test)]
mod tests;
