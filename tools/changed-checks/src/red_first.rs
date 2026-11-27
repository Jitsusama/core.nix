//! Red first: each check whose tests a change adds or edits is built again
//! with those tests laid over the base's code, and a change none of whose
//! checks fails there would be refused, since its tests would pass without
//! it. For now it only says so, and notes its verdict in the run's line:
//! each review of it found another edge where a path's being a test or code
//! refused a tested change, so refusing waits for a day of gated runs whose
//! verdicts show it refuses none wrongly.
//!
//! On 26 November, across 18 sessions and 42 changes, the most common
//! cause behind a review's findings was a test that passed with its fix
//! undone, or couldn't fail at all, each found and fixed change by change.
//! A test proves a change only if it fails without it, so the gate asks
//! that of every one.
//!
//! Tests are told from code by their paths alone (`telling.rs`): a file in
//! a `tests`, `test` or `checks` folder, a `checks.nix`, or one whose name
//! starts `test_` or ends `_test`, `-test`, `.test`, `tests` or `checks`
//! before its extension, as `host-checks.nix` does. The root's
//! `tests/lib` folder and `checks.nix` are the harness every test runs
//! in, with no test written in them, so they're code to red first. The
//! change's added and edited test files are laid over the base's tree as
//! a commit of their own, never checked out, and each check the gate built
//! is evaluated there. Which checks those tests
//! reach follows from the derivations, with no list of which test is
//! whose:
//!
//! - one whose derivation over the base is the base's reads none of them;
//! - one whose derivation over the base is the tree's comes out as the
//!   build that just passed at the head, as a scan counting every VM
//!   test's subtests does, reading none of the change's code, or as a
//!   check nix decides as it evaluates does, so it's said, not built; a
//!   passing `holds` of the root's `checks.nix` is one, so a weak one goes
//!   unrefused;
//! - one that doesn't evaluate there fails there, as it should, which
//!   proves the tests, and checks that stop the whole evaluation there,
//!   as one naming a file the change adds does, refuse nothing;
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
//!   A check main's quarantine holds fails there for its own reasons, so
//!   it isn't asked.
//!   One that failed with no builder failing it, as when one is lost or a
//!   download fails, says nothing of its tests, is said, and refuses
//!   nothing. Nix's log of these builds is kept out of the run's, where
//!   merge-net would read a failed build as the change's.
//!
//! Red first judges a change only when it can lay the change's whole test
//! side over the base as the head has it, since a check reading every test
//! of a kind would otherwise be judged by a tree that is neither the
//! base's nor the head's, and the tests left out may be the ones failing
//! there. A test file removed or moved, a new Rust test file only the
//! change's code declares (one a laid test file declares is laid with it;
//! one directly in a crate's `tests` folder, which cargo finds alone, is
//! laid too), a path git quotes, and a test added inside a Rust or Python
//! code file, which no path tells apart, are each said, and such a change
//! isn't judged. So a new module's `tests.rs`, the usual home of a unit
//! test, leaves its change unjudged. A change of tests alone, or one only
//! taking lines from its tests, is said and not built. A candidate of
//! merge-net's train, which the train marks with
//! `CHANGED_CHECKS_CANDIDATE=1`, all merges of changes judged at their own
//! heads, isn't judged again. Red first judges the commit the run
//! evaluated, not whatever `HEAD` is by the time it starts.
//! Red first judges on Linux alone: a Mac's checks that a change's tests
//! reach often only evaluate a system, while the check running them may
//! be Linux's alone, so a Mac's run would refuse what Linux's proved.
//! When red first can't judge at all, as when git can't lay the tests,
//! the run says so and refuses nothing.
//!
//! A test that pins behaviour that was already right passes on the base
//! by design: a commit of the change saying `Red-first: none, <why>`
//! excuses it, and the run says so. A
//! revert, as the net lands one, restores old tests over code that passed
//! them, and is excused too.

use std::collections::BTreeSet;
use std::fs;
use std::process::Stdio;

use laying::{git, laid_over};
use serde_json::json;
pub use telling::{Apart, Change, adding, inline_tests};

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

/// What merge-net's train sets in a run gating its candidate, whose
/// merges are each of a change judged at its own head.
const CANDIDATE: &str = "CHANGED_CHECKS_CANDIDATE";

/// Where a run judges red first: the repository, its base, the commit the
/// run evaluated and the checks' system.
pub struct At<'a> {
    pub root: &'a str,
    pub base: &'a str,
    pub head: &'a str,
    pub system: &'a str,
}

impl<'a> At<'a> {
    /// Where a run at `commit`, or at the working tree's `HEAD` where it
    /// evaluated no commit, judges red first.
    pub fn of(root: &'a str, base: &'a str, commit: Option<&'a str>, system: &'a str) -> Self {
        let head = commit.unwrap_or("HEAD");
        Self {
            root,
            base,
            head,
            system,
        }
    }
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

/// Judge whether a check the gate built, `asked`, fails with the change's
/// tests laid over the base's code, saying what it found and noting its
/// verdict in the run's line. A run that builds nothing asks nothing, and
/// a change not yet `committed` has no commit to take its tests from, so
/// is said. Red first failing for its own reasons is said. A change whose
/// every check passes there is said as one red first would refuse, and
/// for now isn't refused.
pub fn judged(
    at: &At,
    asked: Option<Vec<&str>>,
    committed: bool,
    (before, after): (&Derivations, &Derivations),
    noted: &mut Run,
) {
    let verdict = match judging(at, asked, committed, (before, after), noted) {
        Ok(verdict) => verdict,
        Err(why) => {
            eprintln!(
                "changed-checks: red first couldn't judge the change ({why}), so its tests \
                 aren't asked to fail on the base"
            );
            Verdict::Unjudged("couldn't judge")
        }
    };
    if let Verdict::Refused { .. } = verdict {
        eprintln!(
            "changed-checks: red first only says so for now and passes the change, until a \
             day of gated runs shows it refuses none wrongly"
        );
    }
    noted.red_first(verdict.noted(at.head));
}

/// What red first found of a change, as the run's line notes it.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    /// It wasn't judged, and why, in a word or two.
    Unjudged(&'static str),
    /// Its tests can't be laid over the base as its head has them.
    Apart,
    /// A commit excuses its tests, or it reverts.
    Excused,
    /// This check fails with its tests laid over the base, as it should.
    Red { check: String, tests: Vec<String> },
    /// Each check built over the base passed, so its tests would pass
    /// without it.
    Refused {
        checks: Vec<String>,
        tests: Vec<String>,
    },
}

impl Verdict {
    /// The verdict as the run's line holds it, of the commit `head`.
    pub fn noted(&self, head: &str) -> serde_json::Value {
        match self {
            Self::Unjudged(why) => json!({ "head": head, "verdict": "unjudged", "why": why }),
            Self::Apart => json!({ "head": head, "verdict": "apart" }),
            Self::Excused => json!({ "head": head, "verdict": "excused" }),
            Self::Red { check, tests } => {
                json!({ "head": head, "verdict": "red", "check": check, "tests": tests })
            }
            Self::Refused { checks, tests } => json!({
                "head": head, "verdict": "would refuse", "checks": checks, "tests": tests,
            }),
        }
    }
}

/// Whether the change's tests fail on the base, as `judged` says.
fn judging(
    at: &At,
    asked: Option<Vec<&str>>,
    committed: bool,
    (before, after): (&Derivations, &Derivations),
    noted: &mut Run,
) -> Result<Verdict, String> {
    let built = asked.unwrap_or_default();
    if built.is_empty() {
        return Ok(Verdict::Unjudged("nothing built"));
    }
    if !committed {
        eprintln!(
            "changed-checks: the change's tests are asked to fail on the base once it's \
             committed"
        );
        return Ok(Verdict::Unjudged("uncommitted"));
    }
    if !at.system.ends_with("-linux") {
        eprintln!(
            "changed-checks: the change's tests are asked to fail on the base on Linux, where \
             its checks run them, not on {}",
            at.system
        );
        return Ok(Verdict::Unjudged("not linux"));
    }
    let built: Vec<String> = built.iter().map(|name| (*name).to_owned()).collect();
    let range = format!("{}..{}", at.base, at.head);
    // merge-net's train cuts a candidate of merges alone, each of a change
    // whose own head was judged, so judging them as one would refuse a
    // rider of tests alone beside another's code. The train says so, since
    // any branch may be only merges.
    if std::env::var_os(CANDIDATE).is_some_and(|set| set == "1") {
        eprintln!(
            "changed-checks: the train's candidate merges changes each judged at its own \
             head, so its tests aren't asked to fail on the base again"
        );
        return Ok(Verdict::Unjudged("train candidate"));
    }
    // Only the change's own commits speak for it, not a branch it merged.
    let format = format!("--format=%s%x1f%(trailers:key={TRAILER},valueonly)%x1e");
    let trailers = ["log", "--first-parent", "--no-merges", &format, &range];
    let log = git(at.root, &trailers, None, &[])?;
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
            return Ok(Verdict::Excused);
        }
        Some(Excused::Reverts) => {
            eprintln!(
                "changed-checks: the change reverts, so its tests aren't asked to fail on the base"
            );
            return Ok(Verdict::Excused);
        }
        None => {}
    }
    let asked = ["diff", "--find-renames", "--name-status", at.base, at.head];
    let change = Change::read(&git(at.root, &asked, None, &[])?);
    if said_apart(at, &change)? {
        return Ok(Verdict::Apart);
    }
    if change.tests.is_empty() {
        return Ok(Verdict::Unjudged("no tests"));
    }
    // Every test file is laid as the head has it, so a check reading only
    // tests comes out there as at the head; only one gaining lines adds a
    // test to ask. The paths are added or edited, so renames are off.
    let mut counting = vec!["diff", "--numstat", "--no-renames", at.base, at.head, "--"];
    counting.extend(change.tests.iter().map(String::as_str));
    let added = adding(&git(at.root, &counting, None, &[])?);
    if !change.tests.iter().any(|test| added.contains(test)) {
        eprintln!(
            "changed-checks: the change only takes lines from its tests, so none is asked to \
             fail on the base"
        );
        return Ok(Verdict::Unjudged("lines only taken"));
    }
    if change.code.is_empty() {
        eprintln!(
            "changed-checks: the change touches only tests, so none is asked to fail on the base"
        );
        return Ok(Verdict::Unjudged("only tests"));
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
        return Ok(Verdict::Unjudged("only lints"));
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
) -> Result<Verdict, String> {
    let red = |check: &str| Verdict::Red {
        check: check.to_owned(),
        tests: tests.to_vec(),
    };
    let laid = laid_over(at, tests, &[], "the change's tests over its base")?;
    let flake = format!("git+file://{}?rev={laid}", at.root);
    // A test reading what the change adds stops nix's whole evaluation
    // over the base, as a checks file naming a new file does, and so does
    // nix failing for its own reasons, which no exit tells apart; so it's
    // said, and refuses nothing.
    let on_base = match evaluated_as(&flake, at.system, built, noted) {
        Ok(found) => found,
        Err(why) => {
            eprintln!(
                "changed-checks: the checks don't evaluate with the change's tests laid over the \
                 base ({why}), as when a test reads a file the change adds, so the change isn't \
                 refused"
            );
            return Ok(Verdict::Unjudged("unevaluated"));
        }
    };
    let mut moved = Vec::new();
    let mut only_tests = Vec::new();
    let mut undefined = false;
    for name in built {
        match over(name, before, after, &on_base) {
            Over::Moved => moved.push(name.clone()),
            // One coming out as at the head is the very build that just
            // passed there, so its passing says nothing.
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
                return Ok(red(name));
            }
            Over::Unreached => {}
        }
    }
    if !only_tests.is_empty() {
        eprintln!(
            "changed-checks: {} come out the same with the change's tests laid over the base \
             as at its head, as a check reading none of its code or one nix decides as it \
             evaluates does, so building them there proves nothing, and they aren't asked to \
             fail there",
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
        return Ok(Verdict::Unjudged("undefined"));
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
                return Ok(red(name));
            }
            Built::Passed => passing.push(name.as_str()),
            Built::Unbuilt => unbuilt = true,
        }
    }
    // One unbuilt might have been the check that runs the tests.
    if unbuilt {
        return Ok(Verdict::Unjudged("unbuilt"));
    }
    if passing.is_empty() {
        return Ok(Verdict::Unjudged("none moved"));
    }
    say_refused(&passing, tests);
    Ok(Verdict::Refused {
        checks: passing.iter().map(|name| (*name).to_owned()).collect(),
        tests: tests.to_vec(),
    })
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
             why it pins what was already right with a trailer of one line, ending a commit \
             message of the change:"
        );
    }
    // The trailer goes on a line of its own, so it's copied whole into a
    // commit message rather than retyped from inside a sentence.
    eprintln!("\n    {TRAILER}: none, <why the tests pin what was already right>\n");
}

/// Whether the change holds tests that can't be laid over the base as its
/// head has them, saying each: a test file removed, moved, declared only
/// by its code or quoted, or a test added among code. Laying the rest
/// alone would judge a check reading every test of a kind by a tree that
/// is neither the base's nor the head's, and the tests set aside may be
/// the ones that fail there, so such a change isn't judged.
fn said_apart(at: &At, change: &Change) -> Result<bool, String> {
    let mut apart: Vec<String> = change.apart.iter().map(Apart::said).collect();
    apart.extend(inline_in(at, change)?.into_iter().map(|file| {
        format!("{file} adds a test among its code, which no path tells apart from it")
    }));
    for said in &apart {
        eprintln!("changed-checks: {said}");
    }
    if !apart.is_empty() {
        eprintln!(
            "changed-checks: so the change's tests can't be laid over the base as its head has \
             them, and none is asked to fail there"
        );
    }
    Ok(!apart.is_empty())
}

/// The code files the change adds a test among the code of.
fn inline_in(at: &At, change: &Change) -> Result<Vec<String>, String> {
    if change.code.is_empty() {
        return Ok(Vec::new());
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
        at.head,
        "--",
    ];
    asked.extend(change.code.iter().map(String::as_str));
    Ok(inline_tests(&git(at.root, &asked, None, &[])?))
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
mod telling;

#[cfg(test)]
mod tests;
