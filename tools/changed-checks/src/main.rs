//! Names the checks a change touches, and builds only those.
//!
//! A check whose derivation is the same as at the base proves nothing new,
//! since its result is already what it was. This evaluates every check's
//! derivation at the base, the merge base with origin's main unless told
//! otherwise, and in the working tree, and names each check added, changed
//! or removed, and each that no longer evaluates, then builds those added
//! and changed, from the derivations it evaluated: all but the VM tests,
//! and with `--build` those too unless `--no-vm` says not. A named check
//! proves nothing until it's built, and core's main once broke on a change
//! that named checks without building them. `--name-only` builds nothing.
//! With no `--system` it compares the system with the most checks, this
//! machine's on a tie, since a repository may hold some on one system
//! alone, and a run on a Mac passed over core's Linux-only checks.
//! With `--keep-base` it only evaluates the base
//! and keeps its derivations, comparing nothing, so a run after each merge
//! leaves every later comparison against that commit one evaluation to pay.
//! It goes only to a builder already standing.
//! `changed-checks keep-main` does that for each head of origin's main, once
//! it has stood two minutes, where runs are sent (`keep.rs` says why).
//!
//! Usage: `changed-checks [--base REF] [--system SYSTEM] [--build] [--no-vm] [--name-only]
//! [--keep-base] [--here]`, or `changed-checks keep-main`
//!
//! The working tree is read as Nix reads a git checkout, so a new file
//! counts only once git knows it (`git add -N`). A change that only some
//! checks read, such as a module's, touches only those; one every check
//! reads, such as `flake.lock`, touches them all.
//!
//! The checks are evaluated a few dozen at a time, in a nix eval each, a VM
//! test counting four, two evaluations at once, or up to eight on a builder
//! with the cores and memory for them (`checks.rs` says why),
//! rather than all in one, which held every check at once and reached 24 GB
//! on a machine many sessions share; each evaluation's memory goes when it
//! ends. A run that builds no VM test evaluates the tree's ordinary checks
//! first and builds what they moved while it evaluates its VM tests, which
//! are half the evaluating, so the build waits on none of them. A commit's
//! derivations never change, so the base's are
//! kept under `~/.cache/changed-checks`, and only the working tree is
//! evaluated again. A run that wants a commit another run is evaluating
//! waits for it rather than evaluating it too, and a working tree git
//! calls clean is kept by its commit the same way. git and nix are the person's own, with their settings,
//! so they're found on the path rather than pinned.
//!
//! The whole run goes to another machine where nix-offload can send it
//! (`nix-offload there --changed`): the lent builder, or a machine that
//! evaluates with its builds sent on. Each evaluation here held about
//! 4.8 GB and several cores of the person's machine. The far side runs
//! the commit's own changed-checks with the same arguments and marked so
//! it never sends itself on, a `--base` given as its merge base's commit.
//! A base whose merge base origin's main doesn't hold, which the far side
//! wouldn't have, edits not yet committed, which it would pass over, or
//! nowhere to send it leaves the run unrun, exiting 75 to be asked again;
//! it runs here only with `--here`. nix-offload keeps
//! what passed at each commit, since each builder it lends starts with an
//! empty store, and tells the far side, as `CHANGED_CHECKS_PASSED`, which
//! checks passed at its commit, so it builds none of them again; the same
//! run again at a commit where it passed is answered by nix-offload
//! alone. While nix-offload lends a builder, a build here is sent
//! to it as a command naming a commit origin has, and the builder fetches
//! that commit, evaluates the checks and builds them itself (`there.rs`
//! says why).
//!
//! A check the net holds in quarantine on origin's main
//! (`quarantine.rs` says what that is) is built on its own, after the
//! rest, and its failure is said and blocks nothing.
//!
//! Each run that reaches its base adds a line to `changed-checks/runs.jsonl`
//! under XDG's state folder, saying what each evaluation and the build took
//! (`runs.rs` says what it holds).

mod checks;
mod first;
mod keep;
mod pressure;
mod quarantine;
mod runs;
#[cfg(test)]
mod scratch;
mod there;
mod turn;

use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::thread;
use std::time::{Duration, Instant};

use checks::Derivations;
use quarantine::Held;
use runs::{Evaluation, Run};
use there::Sent;

/// What a run asked for is refused with, as sysexits.h numbers it.
const USAGE: u8 = 64;

/// What a run exits with when it kept nothing since nowhere took it:
/// sysexits.h's temporary failure, as `nix-offload there` gives.
const UNSENT: u8 = 75;

const USED: &str = "Usage: changed-checks [--base REF] [--system SYSTEM] [--build] [--no-vm] \
                    [--name-only] [--keep-base] [--here] [--first]\n       \
                    changed-checks keep-main";

/// Where `keep-main` follows main unless `CHANGED_CHECKS_REPOSITORY` names
/// another.
const REPOSITORY: &str = "https://github.com/Jitsusama/core.nix";

/// The base a run compares with unless told otherwise.
const ORIGIN_MAIN: &str = "refs/remotes/origin/main";

/// What `nix-offload there --changed` exits with when nowhere else took the
/// run, so it runs here.
const NOT_TAKEN: i32 = 75;

/// Set on the far side's run, which runs there and never sends itself on.
const HERE: &str = "CHANGED_CHECKS_HERE";

/// Set on the far side's run to the checks that passed at its commit
/// already, which it builds none of again (`passed_at` says how).
const PASSED: &str = "CHANGED_CHECKS_PASSED";

/// What a run was asked for.
#[expect(
    clippy::struct_excessive_bools,
    reason = "each is a flag of its own on the command line, set apart from the rest"
)]
struct Options {
    /// What the change is compared with, from its merge base.
    base: String,
    /// The checks' system, or this machine's when none is named.
    system: Option<String>,
    /// Whether to build the checks added and changed.
    build: bool,
    /// Whether the VM tests are among those built.
    vm: bool,
    /// Whether to keep the base's derivations and compare nothing.
    keep_base: bool,
    /// Whether to run here even where another machine could.
    here: bool,
    /// Whether a run sent elsewhere goes ahead of those waiting for a
    /// builder's turn, which only the net may ask (`first.rs`).
    first: bool,
    /// The arguments as given, for a run sent elsewhere.
    given: Vec<String>,
}

impl Options {
    /// Read the command line's arguments, or say what's wrong with them.
    fn parse(arguments: impl Iterator<Item = String>) -> Result<Self, String> {
        let given: Vec<String> = arguments.collect();
        let mut arguments = given.clone().into_iter();
        let mut options = Self {
            base: ORIGIN_MAIN.to_owned(),
            system: None,
            build: true,
            vm: false,
            keep_base: false,
            here: false,
            first: false,
            given: Vec::new(),
        };
        let (mut whole, mut no_vm, mut name_only) = (false, false, false);
        while let Some(argument) = arguments.next() {
            let mut value = || {
                arguments
                    .next()
                    .ok_or_else(|| format!("{argument} needs a value"))
            };
            match argument.as_str() {
                "--base" => options.base = value()?,
                "--system" => options.system = Some(value()?),
                "--build" => whole = true,
                "--no-vm" => no_vm = true,
                "--name-only" => name_only = true,
                "--keep-base" => options.keep_base = true,
                "--here" => options.here = true,
                "--first" => options.first = true,
                _ => return Err(format!("{argument} isn't something it takes")),
            }
        }
        if whole && name_only {
            return Err("--build and --name-only ask for opposite things".to_owned());
        }
        options.build = !name_only;
        options.vm = whole && !no_vm;
        options.given = given;
        Ok(options)
    }
}

/// Says how many VM tests a run builds, before it builds them. On 6
/// November one `--build` for a change of the shell's alone sent 27 of
/// them, which held every VM builder for 66 minutes while other changes'
/// runs waited up to 53; the gate before merging needed none of them.
fn say_vm_tests(targets: &[String]) {
    let vm = targets
        .iter()
        .filter(|name| name.starts_with("vm-"))
        .count();
    if vm > 0 {
        let tests = if vm == 1 { "test" } else { "tests" };
        eprintln!(
            "changed-checks: building {vm} VM {tests} whole, as --build asks. Before merging, \
             a change needs only those `merge-net needs` names and `run-subtests --there` \
             on the subtests its files reach; the net runs the rest after the merge."
        );
    }
}

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    if arguments.iter().any(|argument| argument == "--help") {
        println!("{USED}");
        return ExitCode::SUCCESS;
    }
    if let Some(("keep-main", rest)) = arguments
        .split_first()
        .map(|(first, rest)| (first.as_str(), rest))
    {
        return keep_main(rest);
    }
    let options = match Options::parse(arguments.into_iter()) {
        Ok(options) => options,
        Err(why) => {
            eprintln!("changed-checks: {why}\n{USED}");
            return ExitCode::from(USAGE);
        }
    };
    match changed(&options) {
        Ok(code) => code,
        Err(why) => {
            eprintln!("changed-checks: {why}");
            ExitCode::FAILURE
        }
    }
}

/// Name the checks the working tree's change touches, and build them if asked.
fn changed(options: &Options) -> Result<ExitCode, String> {
    let root = run("git", &["rev-parse", "--show-toplevel"])?;
    let Ok(base) = run("git", &["-C", &root, "merge-base", "HEAD", &options.base]) else {
        eprintln!(
            "changed-checks: {} shares no history with HEAD",
            options.base
        );
        return Ok(ExitCode::from(USAGE));
    };
    let flake = format!("git+file://{root}");
    let system = match &options.system {
        Some(system) => system.clone(),
        None => busiest_system(&flake)?,
    };
    let untracked = run(
        "git",
        &["-C", &root, "ls-files", "--others", "--exclude-standard"],
    )?;
    if !untracked.is_empty() {
        eprintln!("changed-checks: Nix doesn't read files git doesn't know:\n{untracked}");
    }
    if let Some(code) = elsewhere(options, &root, &base, &system)? {
        return Ok(code);
    }
    // Asked to, or the far side's run, which runs where it was sent.
    let here = options.here || env::var_os(HERE).is_some();
    // A base kept ahead of time is never worth evaluating beside the
    // person's work, at about 20 GB, unless asked to.
    if options.keep_base && !here {
        eprintln!(
            "changed-checks: no standing builder took the base to keep, so it's left unkept; \
             --here keeps it on this machine"
        );
        return Ok(ExitCode::from(UNSENT));
    }
    // A run that went nowhere else evaluates at several gigabytes and
    // builds what its builders didn't take, beside the person's work. It
    // did so while the machine looked idle, and an idle machine at the
    // start was no promise for the hours after: on 8 November one built
    // openvino here for three hours with Joel at it. So it runs here only
    // when asked, and is otherwise run again later, as nix-offload's 75
    // asks.
    if !here {
        eprintln!(
            "changed-checks: the run went nowhere else, so nothing was run here; --here runs \
             it on this machine"
        );
        return Ok(ExitCode::from(UNSENT));
    }
    // The far side's runs share a builder made for them; this machine's
    // take turns (`turn.rs` says why).
    let _turn = if env::var_os(HERE).is_none() {
        let lock = cache().join("changed-checks/evaluating-here.lock");
        Some(turn::taken(&lock, || {
            eprintln!(
                "changed-checks: another run evaluates on this machine, so this waits its turn"
            );
        })?)
    } else {
        None
    };
    let held = quarantine::on_main(&root, ORIGIN_MAIN);
    let mut noted = Run::new(&base, &system);
    let compared = compare(options, &flake, &base, &system, &held, &mut noted);
    let log = state().join("changed-checks/runs.jsonl");
    if let Err(why) = noted.record(&log) {
        eprintln!("changed-checks: couldn't log the run: {why}");
    }
    compared
}

/// Follow origin's main, keeping each head once it has stood, until stopped.
fn keep_main(rest: &[String]) -> ExitCode {
    if !rest.is_empty() {
        eprintln!("changed-checks: keep-main takes nothing more\n{USED}");
        return ExitCode::from(USAGE);
    }
    let url = env::var("CHANGED_CHECKS_REPOSITORY").unwrap_or_else(|_| REPOSITORY.to_owned());
    let clone = state().join("changed-checks/main");
    let mut keeper = keep::Keeper::default();
    eprintln!(
        "changed-checks: keeping each head of {url}'s main once it has stood {} s",
        keep::STOOD.as_secs()
    );
    loop {
        // The store line names the lent builder by its host key, which is
        // new for each one lent, so it says when what's kept is gone.
        let builder = std::fs::read_to_string(offload_dir().join("store")).ok();
        let builder = builder
            .as_deref()
            .map(str::trim)
            .filter(|line| !line.is_empty());
        match keep::head(&url) {
            Ok(head) => {
                if let (Some(head), Some(builder)) =
                    (keeper.due(&head, builder, Instant::now()), builder)
                {
                    let short = head.get(..12).unwrap_or(&head);
                    match keep_head(&url, &clone, &head) {
                        Ok(true) => keeper.kept(&head, builder),
                        Ok(false) => {
                            eprintln!("changed-checks: nothing took {short}, so it waits");
                            keeper.failed(&head, builder, Instant::now());
                        }
                        Err(why) => {
                            eprintln!("changed-checks: couldn't keep {short}: {why}");
                            keeper.failed(&head, builder, Instant::now());
                        }
                    }
                }
            }
            Err(why) => eprintln!("changed-checks: couldn't read main's head: {why}"),
        }
        std::thread::sleep(keep::LOOK_EVERY);
    }
}

/// Keep `head`'s checks where runs are sent, from the keeper's own clone,
/// as a run on a branch cut from it would send them. Nothing that took it
/// says false, since it's never kept on this machine.
fn keep_head(url: &str, clone: &Path, head: &str) -> Result<bool, String> {
    keep::check_out(url, clone, head)?;
    env::set_current_dir(clone)
        .map_err(|error| format!("going to {}: {error}", clone.display()))?;
    let root = clone.to_string_lossy().into_owned();
    let system = busiest_system(&format!("git+file://{root}"))?;
    let options = Options::parse(std::iter::once("--keep-base".to_owned()))?;
    Ok(match elsewhere(&options, &root, head, &system)? {
        Some(code) if code == ExitCode::SUCCESS => true,
        Some(_) => return Err("the far side's run failed".to_owned()),
        None => false,
    })
}

/// The system whose checks are the most, `checks::busiest` says why. Only
/// the names are counted, which took 1.1 s and 158 MB for core's 580 checks
/// on 30 October, against 4.8 GB for evaluating them. They're counted
/// through the flake's own `checks`, as the rest is evaluated, since
/// `builtins.getFlake` on a flake not yet locked needs `--impure`.
fn busiest_system(flake: &str) -> Result<String, String> {
    let here = this_system();
    let checks = format!("{flake}#checks");
    let counting = "builtins.mapAttrs (_: checks: builtins.length (builtins.attrNames checks))";
    let counted = run(
        "nix",
        &pure(&["eval", "--json", &checks, "--apply", counting]),
    )?;
    let counts = serde_json::from_str(&counted)
        .map_err(|error| format!("reading how many checks each system has: {error}"))?;
    Ok(checks::busiest(&counts, &here))
}

/// The Nix system this program was built for, which is this machine's:
/// read without asking Nix, whose evaluation of `builtins.currentSystem`
/// waited 30 minutes at the jobs door on 10 November before a run it began
/// could be sent anywhere.
fn this_system() -> String {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        os => os,
    };
    format!("{}-{os}", std::env::consts::ARCH)
}

/// The run sent to another machine by `nix-offload there --changed`, its
/// exit, or none when it runs here: asked to, already the far side's, a
/// base whose merge base origin's main doesn't hold, a nix-offload that
/// doesn't take it, edits not yet in a commit, or nowhere that took it. A
/// run that only keeps a base is never worth a builder of its own, so it
/// goes only to one that stands, and needs a nix-offload that takes
/// `--never-ask`.
fn elsewhere(
    options: &Options,
    root: &str,
    base: &str,
    system: &str,
) -> Result<Option<ExitCode>, String> {
    if options.here || env::var_os(HERE).is_some() {
        return Ok(None);
    }
    // The far side fetches origin's main, so it has any commit main holds,
    // and the merge base is what the comparison starts from either way.
    let holds = Command::new("git")
        .args(["-C", root, "merge-base", "--is-ancestor", base, ORIGIN_MAIN])
        .status()
        .is_ok_and(|status| status.success());
    if options.base != ORIGIN_MAIN && !holds {
        eprintln!(
            "changed-checks: origin's main doesn't hold {}, so the run isn't sent",
            base.get(..12).unwrap_or(base)
        );
        return Ok(None);
    }
    let takes = Command::new("nix-offload")
        .args(["there", "--takes"])
        .stderr(std::process::Stdio::null())
        .output();
    let wanted: &[&str] = if options.keep_base {
        &["changed", "never-ask"]
    } else {
        &["changed"]
    };
    let words = match &takes {
        Ok(takes) if takes.status.success() => String::from_utf8_lossy(&takes.stdout).into_owned(),
        _ => String::new(),
    };
    let takes_it = wanted
        .iter()
        .all(|word| words.lines().any(|line| line == *word));
    if !takes_it {
        eprintln!("changed-checks: this machine's nix-offload doesn't take the run");
        return Ok(None);
    }
    // The far side checks a commit, so edits not yet in one would be passed
    // over for the last commit's checks.
    if clean_commit()?.is_none() {
        eprintln!("changed-checks: uncommitted changes aren't sent; commit to send the run");
        return Ok(None);
    }
    // A base is given as its merge base's commit, which a ref named only
    // here, such as a branch, wouldn't be on the far side.
    let mut given = Vec::new();
    let mut each = options.given.iter();
    while let Some(word) = each.next() {
        // Going first is asked of nix-offload, never of the far side's run.
        if word == "--first" {
            continue;
        }
        given.push(word.clone());
        if word == "--base" {
            each.next();
            given.push(base.to_owned());
        }
    }
    let cgroup = std::fs::read_to_string("/proc/self/cgroup").unwrap_or_default();
    let first = match first::first(options.first, &words, &cgroup) {
        first::First::Yes => true,
        first::First::No => false,
        first::First::Refused(why) => {
            eprintln!("changed-checks: {why}");
            false
        }
    };
    // The far side may be another system, which would name its own.
    if options.system.is_none() {
        given.extend(["--system".to_owned(), system.to_owned()]);
    }
    eprintln!("changed-checks: sending the run elsewhere; --here runs it on this machine");
    let status = Command::new("nix-offload")
        .arg("there")
        .args(options.keep_base.then_some("--never-ask"))
        .args(first.then_some("--first"))
        .arg("--changed")
        .args(&given)
        .current_dir(root)
        .status()
        .map_err(|error| format!("running nix-offload: {error}"))?;
    Ok(match status.code() {
        Some(NOT_TAKEN) => {
            eprintln!("changed-checks: no builder took the run");
            None
        }
        Some(code) => Some(ExitCode::from(u8::try_from(code).unwrap_or(1))),
        None => Some(ExitCode::FAILURE),
    })
}

/// Compare the working tree's checks with the base's, noting in the run
/// what each step took, and build what changed if asked.
fn compare(
    options: &Options,
    flake: &str,
    base: &str,
    system: &str,
    held: &[Held],
    noted: &mut Run,
) -> Result<ExitCode, String> {
    let short = base.get(..12).unwrap_or(base);
    let kept = |noted: &mut Run| {
        checks::kept(&checks::keeping(&cache(), base, system), || {
            derivations(&format!("{flake}?rev={base}"), system, "base", noted)
        })
    };
    if options.keep_base {
        kept(noted)?;
        eprintln!("kept {system}'s checks at {short}");
        return Ok(ExitCode::SUCCESS);
    }
    eprintln!("comparing {system}'s checks at {short} with the working tree");
    let before = kept(noted)?;
    // A working tree git calls clean is its commit, whose derivations never
    // change, so they're kept by it as a base's are, and a run again on the
    // same commit, or one whose commit is a base kept already, evaluates
    // nothing.
    let commit = clean_commit()?;
    let passed = passed_at(commit.as_deref());
    let unbuilt = |comparison: &checks::Comparison| {
        unpassed(
            &comparison.to_build(options.vm),
            &passed,
            commit.as_deref(),
            options.build,
        )
    };
    // A run that builds no VM test, as the gate before a merge, builds what
    // the ordinary checks moved while it evaluates the VM tests, rather than
    // after: on 11 November the 90 VM tests were 11% of core's checks but
    // half of each run's evaluating, which every build waited on.
    let beside = options.build && !options.vm;
    let (after, building) = thread::scope(|scope| {
        // Once the ordinary checks are compared, the build they began, if
        // they moved any; none for a tree kept already.
        let mut building = None;
        let mut evaluate = || -> Result<Derivations, String> {
            if !beside {
                return derivations(flake, system, "tree", noted);
            }
            let mut evaluation = Run::evaluating("tree");
            let (vm, ordinary): (Vec<String>, Vec<String>) = names(flake, system)?
                .into_iter()
                .partition(|name| name.starts_with("vm-"));
            let mut after = evaluated(flake, system, &ordinary, &mut evaluation)?;
            after.retain(|name, _| !name.starts_with("vm-"));
            let moved = checks::compare(&before, &after);
            let targets = if moved.broken.is_empty() {
                unbuilt(&moved)
            } else {
                Vec::new()
            };
            building = Some((!targets.is_empty()).then(|| {
                let derivations = after.clone();
                scope.spawn(move || built(flake, system, held, &targets, &derivations))
            }));
            after.extend(evaluated(flake, system, &vm, &mut evaluation)?);
            noted.evaluated(evaluation);
            Ok(after)
        };
        let after = match &commit {
            Some(commit) => checks::kept(&checks::keeping(&cache(), commit, system), evaluate),
            None => evaluate(),
        };
        let building = building.map(|began| {
            began.map(|build| {
                build
                    .join()
                    .unwrap_or_else(|_| Err("the build beside the evaluation panicked".to_owned()))
            })
        });
        (after, building)
    });
    let after = after?;
    let comparison = checks::compare(&before, &after);
    let mut named = 0;
    for (kind, names) in comparison.kinds() {
        for name in names {
            println!("{kind:<8} {name}");
            named += 1;
        }
    }
    noted.named(named);
    let whole = comparison.broken.is_empty();
    let build = match building {
        Some(began) => began.transpose()?,
        None if whole && options.build => {
            let targets = unbuilt(&comparison);
            if targets.is_empty() {
                None
            } else {
                say_vm_tests(&targets);
                Some(built(flake, system, held, &targets, &after)?)
            }
        }
        None => None,
    };
    if let Some(build) = &build {
        noted.built(build.checks, build.took, build.sent.passed);
        if build.sent.there {
            noted.lent(build.sent.bytes);
        }
    }
    Ok(
        if whole && build.as_ref().is_none_or(|build| build.sent.passed) {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        },
    )
}

/// Those of the checks a comparison moved that `passed` doesn't say passed
/// at the commit already, saying of each it leaves out, when building, that
/// it isn't built again.
fn unpassed(
    moved: &[&str],
    passed: &[String],
    commit: Option<&str>,
    building: bool,
) -> Vec<String> {
    moved
        .iter()
        .copied()
        .filter(|name| {
            let already = passed.iter().any(|check| check == name);
            if already && building {
                let short = commit.and_then(|commit| commit.get(..12));
                eprintln!(
                    "changed-checks: {name} passed at {} already, so it isn't built again",
                    short.unwrap_or_default()
                );
            }
            !already
        })
        .map(str::to_owned)
        .collect()
}

/// A build of the checks a run named: how many, how long it took, and how
/// it ended.
struct Built {
    checks: usize,
    took: Duration,
    sent: Sent,
}

/// Build `targets`, each that main quarantines alone after the rest, from
/// the derivations `after` evaluated for them.
fn built(
    flake: &str,
    system: &str,
    held: &[Held],
    targets: &[String],
    after: &Derivations,
) -> Result<Built, String> {
    let began = Instant::now();
    let (quarantined, blocking): (Vec<&str>, Vec<&str>) = targets
        .iter()
        .map(String::as_str)
        .partition(|name| quarantine::find(held, name).is_some());
    let mut sent = Sent {
        passed: true,
        there: false,
        bytes: None,
    };
    if !blocking.is_empty() {
        sent = build(flake, system, &blocking, after)?;
    }
    // Each is built alone, so a failure is known for the check it was.
    for name in quarantined {
        let alone = build(flake, system, &[name], after)?;
        if !alone.passed {
            let why = quarantine::find(held, name)
                .map(Held::said)
                .unwrap_or_default();
            eprintln!("changed-checks: {name} failed, but {why}, so it blocks nothing");
        }
        sent.there |= alone.there;
        // Each build there sent its own, so the run sent them all.
        sent.bytes = match (sent.bytes, alone.bytes) {
            (Some(before), Some(now)) => Some(before + now),
            (before, now) => before.or(now),
        };
    }
    Ok(Built {
        checks: targets.len(),
        took: began.elapsed(),
        sent,
    })
}

/// Build `targets` on the lent builder while one stands, and otherwise
/// here, each from the derivation `after` evaluated for it.
fn build(flake: &str, system: &str, targets: &[&str], after: &Derivations) -> Result<Sent, String> {
    if there::lent(&offload_dir()) {
        let journal = state().join("nix-offload/there.jsonl");
        match published().and_then(|fetched| there::build(&fetched, system, targets, &journal)) {
            Ok(sent) => return Ok(sent),
            Err(why) => eprintln!("changed-checks: building here, since {why}"),
        }
    }
    // Each check is built from the derivation just evaluated, so no
    // evaluator runs beside the builds: one holds its whole heap until its
    // last build ends, 1.4 to 4 GiB through a VM test. A derivation the
    // store has collected since is built from the flake, as before.
    let built = Command::new("nix")
        .args(["build", "--no-link", "--keep-going", "-L"])
        .args(PURE)
        .args(standing())
        .args(
            targets
                .iter()
                .map(|name| match after.get(*name).and_then(Option::as_deref) {
                    Some(derivation) if Path::new(derivation).exists() => format!("{derivation}^*"),
                    _ => format!("{flake}#checks.{system}.{name}"),
                }),
        )
        .status()
        .map_err(|error| format!("running nix: {error}"))?;
    Ok(Sent {
        passed: built.success(),
        there: false,
        bytes: None,
    })
}

/// Where nix-offload says the machine's builders: `NIX_OFFLOAD_DIR`, or
/// /run/nix-offload. In it, machines-with-standing names the standing
/// builders beside any lent one, for a build that asks for them, and store
/// the lent builder's store, while one stands.
fn offload_dir() -> PathBuf {
    env::var_os("NIX_OFFLOAD_DIR").map_or_else(|| PathBuf::from("/run/nix-offload"), PathBuf::from)
}

/// What a build here asks for: the standing builders, where the machine
/// has them. Those are never offered to every build, since they may be
/// machines only public code may reach; core.nix's checks are public code
/// with public inputs alone, which `stands-alone` holds, so they may go
/// there.
fn standing() -> Vec<String> {
    let with_standing = offload_dir().join("machines-with-standing");
    if with_standing.exists() {
        vec![
            "--builders".to_owned(),
            format!("@{}", with_standing.display()),
        ]
    } else {
        Vec::new()
    }
}

/// The flake a builder can fetch the working tree from by itself: its
/// commit, from origin, when the tree is that commit and origin has it.
/// Otherwise why not, since then only this machine has what's built.
fn published() -> Result<String, String> {
    let Some(commit) = clean_commit()? else {
        return Err("the working tree differs from its commit".to_owned());
    };
    let short = commit.get(..12).unwrap_or(&commit);
    let on = run(
        "git",
        &["branch", "-r", "--contains", &commit, "--list", "origin/*"],
    )?;
    if on.is_empty() {
        return Err(format!(
            "origin doesn't have {short}; push it to build there"
        ));
    }
    let url = run("git", &["remote", "get-url", "origin"])?;
    Ok(there::fetched(&url, &commit))
}

/// Which checks, by name, `CHANGED_CHECKS_PASSED` says passed at `commit`,
/// the working tree's when it's clean: its commit, then each name. A
/// check that passed at a commit passes there again, so nix-offload, which
/// keeps what passed wherever it was built, says so when it sends a run;
/// told of another commit, or of a tree with changes, it names none.
fn passed_at(commit: Option<&str>) -> Vec<String> {
    let told = env::var(PASSED).unwrap_or_default();
    let mut words = told.split_whitespace();
    match (words.next(), commit) {
        (Some(told), Some(commit)) if told == commit => words.map(str::to_owned).collect(),
        _ => Vec::new(),
    }
}

/// The working tree's commit, when nothing in it differs from the commit
/// in what Nix reads: no file git tracks, or has been told of, changed.
fn clean_commit() -> Result<Option<String>, String> {
    let changed = run("git", &["status", "--porcelain", "--untracked-files=no"])?;
    if !changed.is_empty() {
        return Ok(None);
    }
    run("git", &["rev-parse", "HEAD"]).map(Some)
}

/// Each evaluation goes past nix's evaluation cache. Evaluated through
/// `--apply`, a result can't be kept there, so the cache saved nothing (one
/// chunk took 115 s with it and 108 to 127 s without), while two evaluations
/// at once, or two runs, found its database busy.
const UNCACHED: &str = "--no-eval-cache";

/// Every evaluation is pure, and no flake's own config is taken. A fenced
/// session's run goes to a lent builder with a flake it chose, which may
/// read only what the commit holds there (ISXEG8, 11 November): an impure
/// evaluation reads the machine's files and environment, and a flake's
/// `nixConfig`, once taken, could turn purity off again.
const PURE: [&str; 6] = [
    "--option",
    "pure-eval",
    "true",
    "--option",
    "accept-flake-config",
    "false",
];

/// Nix's words `asked`, then those that keep its evaluation [`PURE`].
fn pure<'a>(asked: &[&'a str]) -> Vec<&'a str> {
    asked.iter().copied().chain(PURE).collect()
}

/// Evaluate each of a flake's checks for a system to its derivation's path,
/// noting in the run what each chunk of them took, as the side named.
fn derivations(
    flake: &str,
    system: &str,
    side: &'static str,
    noted: &mut Run,
) -> Result<Derivations, String> {
    let mut evaluation = Run::evaluating(side);
    let found = evaluated(flake, system, &names(flake, system)?, &mut evaluation)?;
    noted.evaluated(evaluation);
    Ok(found)
}

/// The names of a flake's checks for a system.
fn names(flake: &str, system: &str) -> Result<Vec<String>, String> {
    let checks = format!("{flake}#checks.{system}");
    let naming: [&str; 6] = [
        "eval",
        "--json",
        &checks,
        "--apply",
        "builtins.attrNames",
        UNCACHED,
    ];
    let listed = run("nix", &pure(&naming))?;
    serde_json::from_str(&listed).map_err(|error| format!("nix named the checks oddly: {error}"))
}

/// Evaluate these of a flake's checks for a system to their derivations'
/// paths, noting in the evaluation what each chunk of them took.
fn evaluated(
    flake: &str,
    system: &str,
    names: &[String],
    evaluation: &mut Evaluation,
) -> Result<Derivations, String> {
    let checks = format!("{flake}#checks.{system}");
    let chunks = checks::chunks(names);
    // As many go at once as a builder made for the run has room for, and two
    // on the person's own machine (`checks::at_once` says why). Either way a
    // further chunk begins only while memory isn't stalling: two at once
    // while it was full stalled Joel's launcher on 29 October.
    let evaluated = checks::each_at_once(
        &chunks,
        checks::at_once_here(env::var_os(HERE).is_some()),
        || !pressure::memory_stalled(),
        |chunk| {
            let selecting = checks::selecting(chunk)?;
            let began = Instant::now();
            let asked: [&str; 6] = ["eval", "--json", &checks, "--apply", &selecting, UNCACHED];
            let evaluated = run("nix", &pure(&asked))?;
            let took = began.elapsed();
            let derivations: Derivations = serde_json::from_str(&evaluated)
                .map_err(|error| format!("nix gave the checks' derivations oddly: {error}"))?;
            Ok::<_, String>((chunk.len(), took, derivations))
        },
    )?;
    let mut found = Derivations::new();
    for (checks, took, derivations) in evaluated {
        evaluation.chunk(checks, took);
        found.extend(derivations);
    }
    Ok(found)
}

/// Run a program on the path, and give what it printed, trimmed; what it
/// says of itself goes to this one's standard error.
fn run(program: &str, arguments: &[&str]) -> Result<String, String> {
    let ran = Command::new(program)
        .args(arguments)
        .stderr(std::process::Stdio::inherit())
        .output()
        .map_err(|error| format!("running {program}: {error}"))?;
    if !ran.status.success() {
        return Err(format!(
            "{program} {} failed: {}",
            arguments.join(" "),
            ran.status
        ));
    }
    String::from_utf8(ran.stdout)
        .map(|said| said.trim().to_owned())
        .map_err(|_| format!("{program} {} printed what isn't text", arguments.join(" ")))
}

/// Give the cache folder: XDG's, or `~/.cache` without one.
fn cache() -> PathBuf {
    xdg("XDG_CACHE_HOME", ".cache")
}

/// Give the state folder: XDG's, or `~/.local/state` without one.
fn state() -> PathBuf {
    xdg("XDG_STATE_HOME", ".local/state")
}

/// Give the folder XDG's variable names, or the one under home without it.
fn xdg(variable: &str, under_home: &str) -> PathBuf {
    env::var_os(variable)
        .filter(|folder| !folder.is_empty())
        .map_or_else(
            || PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(under_home),
            PathBuf::from,
        )
}
