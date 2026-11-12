//! The net under main: merges land on their own checks, VM tests aside,
//! and the net runs every check each one changed after it lands.
//!
//! `merge-net watch` follows origin's main from its own checkout. For each
//! new head it asks changed-checks which checks moved since the last head
//! whose checks all passed, and runs each of them, the VM tests last,
//! through `nix-offload there`, which sends it to the builders, twelve at
//! once, as bulk where nix-offload takes it, so sessions' runs go first. A
//! test that fails twice at a commit fails for real: the net finds which
//! merge broke it, holds switching on it (`hold.json`), reverts it through
//! a PR merged on its own checks, and writes a finding for its owner. A
//! held test runs first at each new head, and its pass lifts the hold. One
//! that fails and then passes goes on the quarantine list
//! (`quarantine.json`) instead, and nothing is reverted for it. It starts
//! nothing while the machine is held or paused (`holds.rs`, `state.rs`).
//! `watch.rs` says what a round does and what the quick look beside it runs
//! every five minutes, `watch/settling.rs` how what they found settles
//! beside the next, and `state.rs` what it keeps, and where.
//!
//! `merge-net needs`, on a branch, says whether its change runs VM tests
//! before it merges, rather than after on the net, and which: a change to
//! what can leave a machine unable to boot, unlock or sign in, which a
//! revert after the fact comes too late for (`must-run.txt`). It names
//! too each VM test that owns a file the change touches, whose subtests
//! that reach it run first with `run-subtests --there` (`owners.rs`).
//!
//! `merge-net ready COMMIT`, asked before a branch's head merges, exits 0
//! only when the head sits on origin's main as it is now and changed-checks
//! built what it named there (`ready.rs`), and otherwise says what to run.
//! It waits, too, for a gate that started before the head's own and shares
//! its files, so changes merge in the order their gates started (`turn.rs`),
//! a change run again after a merge left its pass stale keeping the place
//! its tree's first gate had (`places.rs`).
//!
//! `merge-net status` says where the net is.
//!
//! `watch --branch` follows another branch than main, and lands its reverts
//! there, to prove the net on a throwaway branch; `MERGE_NET_STATE` then
//! names a state folder of its own, so its hold stops no switch.
//!
//! `watch --follow PATH` names the installed merge-net: once that is
//! another build than the one running, the watch ends between rounds, for
//! its unit to start the new one, rather than a switch stopping it
//! mid-round and losing every run in flight.
//!
//! Usage: `merge-net watch [--from COMMIT] [--branch NAME] [--once] [--follow PATH]
//! | merge-net needs [--base REF] | merge-net ready COMMIT | merge-net status`

mod apart;
mod holds;
mod lists;
mod movers;
mod owners;
mod places;
mod reach;
mod ready;
mod rules;
mod running;
mod state;
mod subtests;
mod turn;
mod watch;
mod world;

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

use state::{State, short};
use watch::Round;
use world::Real;

const USED: &str = "Usage: merge-net watch [--from COMMIT] [--branch NAME] [--once] \
                    [--follow PATH] | merge-net needs [--base REF] | merge-net ready COMMIT \
                    | merge-net status";

/// What a run asked for is refused with, as sysexits.h numbers it.
const USAGE: u8 = 64;

/// Where the net watches unless `MERGE_NET_REPOSITORY` names another.
const REPOSITORY: &str = "https://github.com/Jitsusama/core.nix";

/// The system whose checks the net runs.
const SYSTEM: &str = "x86_64-linux";

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let words: Vec<&str> = arguments.iter().map(String::as_str).collect();
    let done = match words.as_slice() {
        ["watch", rest @ ..] => watching(rest),
        ["needs"] => needs("refs/remotes/origin/main"),
        ["needs", "--base", base] => needs(base),
        ["ready", commit] => ready(commit),
        ["status"] => Ok(status()),
        ["--help"] => {
            println!("{USED}");
            Ok(ExitCode::SUCCESS)
        }
        _ => {
            eprintln!("{USED}");
            return ExitCode::from(USAGE);
        }
    };
    done.unwrap_or_else(|why| {
        eprintln!("merge-net: {why}");
        ExitCode::FAILURE
    })
}

/// Follow main, a round at a time, until stopped, or once with `--once`.
fn watching(rest: &[&str]) -> Result<ExitCode, String> {
    let (mut from, mut once, mut branch) = (None, false, "main".to_owned());
    let mut follow: Option<PathBuf> = None;
    let mut words = rest.iter();
    while let Some(word) = words.next() {
        match *word {
            "--from" => from = Some((*words.next().ok_or("--from needs a commit")?).to_owned()),
            "--branch" => (*words.next().ok_or("--branch needs a name")?).clone_into(&mut branch),
            "--once" => once = true,
            "--follow" => {
                follow = Some(PathBuf::from(*words.next().ok_or("--follow needs a path")?));
            }
            other => return Err(format!("{other} isn't something watch takes\n{USED}")),
        }
    }
    let state = State::from_env();
    state.make()?;
    let world = Real {
        state: state.root.clone(),
        url: env::var("MERGE_NET_REPOSITORY").unwrap_or_else(|_| REPOSITORY.to_owned()),
        system: SYSTEM.to_owned(),
        booted: holds::booted().unwrap_or(0),
        branch,
        moving: std::sync::Mutex::new(()),
    };
    // With nothing passed yet, the first round starts where it's told, or
    // at main's head as it is now, which it takes as passing. It's written
    // down at once, so a first round cut short, by a hold or a restart,
    // starts from the same place when it runs again.
    let start = match (state.heads().0, from) {
        (Some(passed), _) => passed,
        (None, Some(from)) => from,
        (None, None) => watch::World::head(&world)?,
    };
    if state.heads().0.is_none() {
        state.set_heads(&start, &start)?;
    }
    state.say(&format!(
        "watching {}'s {} from {}",
        world.url,
        world.branch,
        short(&start)
    ));
    // What a round found settles beside the rounds after it, so a merge
    // waits behind the round's tests alone. The net ends only once the
    // settler has finished what it was handed.
    let settler = watch::Settler::new(&world, &state);
    let done = AtomicBool::new(false);
    let ended = std::thread::scope(|scope| {
        settler.start(scope);
        if !once {
            scope.spawn(|| watch::quick(&world, &state, &settler, &done));
        }
        let ended = rounds(&world, &state, &start, &settler, once, follow.as_deref());
        done.store(true, Ordering::Relaxed);
        settler.close();
        ended
    });
    if once && settler.error().is_some() {
        return Ok(ExitCode::FAILURE);
    }
    ended
}

/// Run round after round, or one with `once`, until another merge-net is
/// installed at `follow`.
fn rounds(
    world: &Real,
    state: &State,
    start: &str,
    settler: &watch::Settler,
    once: bool,
    follow: Option<&Path>,
) -> Result<ExitCode, String> {
    loop {
        let round = watch::round_beside(world, state, start, settler);
        let rest = match &round {
            Ok(Round::Passed(_) | Round::Failed(_)) => 0,
            Ok(Round::Idle) => 60,
            Ok(Round::Unfinished(head)) => {
                state.say(&format!(
                    "{} isn't done: a test wasn't sent, so it runs again",
                    short(head)
                ));
                60
            }
            Err(why) => {
                state.say(why);
                120
            }
        };
        if once {
            return Ok(if matches!(round, Ok(Round::Passed(_) | Round::Idle)) {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            });
        }
        if let Some(installed) = follow
            && replaced(fs::canonicalize(installed), env::current_exe())
        {
            state.say(
                "another merge-net is installed, so this one ends between rounds, \
                 leaving what it settles to that one",
            );
            // An exit, since the scope would wait for the searches under way,
            // which the new build takes up from disk.
            return Ok(settler.leave(|| std::process::exit(0)));
        }
        std::thread::sleep(std::time::Duration::from_secs(rest));
    }
}

/// Whether the merge-net installed is another build than the one running.
/// On 7 November three switches in 25 minutes each restarted the net with
/// a new build, and a round of about 20 minutes ran from 16:21 past 17:15.
/// A running build no longer on disk has been replaced; an installed one
/// that can't be read says nothing.
fn replaced(installed: io::Result<PathBuf>, running: io::Result<PathBuf>) -> bool {
    let Ok(installed) = installed else {
        return false;
    };
    !running
        .and_then(fs::canonicalize)
        .is_ok_and(|running| running == installed)
}

/// Say whether the branch's change runs VM tests before merging, and which.
fn needs(base: &str) -> Result<ExitCode, String> {
    let merge_base = git(&["merge-base", "HEAD", base])?;
    let mut files: Vec<String> = git(&["diff", "--name-only", &merge_base])?
        .lines()
        .chain(git(&["ls-files", "--others", "--exclude-standard"])?.lines())
        .map(str::to_owned)
        .collect();
    files.sort();
    files.dedup();
    let areas = rules::areas();
    let known = known_vm_tests();
    let root = git(&["rev-parse", "--show-toplevel"])?;
    let owning = owners::in_tree(Path::new(&root), &files).unwrap_or_else(|| {
        println!("(checks.nix or flake.nix didn't read, so no VM test is named for its files)");
        Vec::new()
    });
    if owning.is_empty()
        && rules::needs(&areas, &files, known.as_deref())
            .touched
            .is_empty()
    {
        println!("no: nothing here needs its VM tests before merging; the net runs them after");
        return Ok(ExitCode::SUCCESS);
    }
    // Only a test whose derivation moved from main can give a result main
    // hasn't, so changed-checks is asked which moved; it costs an
    // evaluation, which only a change touching an area or a VM test's
    // files pays.
    let said = moved_from(base);
    if said.is_none() {
        println!("(changed-checks couldn't say which VM tests moved, so every one named runs)");
    }
    let found = rules::first(&areas, &files, said.as_deref(), known.as_deref());
    let moved = said.as_deref().map(rules::moved);
    let owning: Vec<String> = owning
        .into_iter()
        .filter(|test| moved.as_ref().is_none_or(|moved| moved.contains(test)))
        .collect();
    // The list is read from the base, main, so a branch can't quarantine
    // its own failure.
    let list = git(&["show", &format!("{base}:{}", world::QUARANTINE)])
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    if found.tests.is_empty() && found.there.is_empty() && owning.is_empty() {
        println!(
            "no: none of the VM tests the change touches moved from {base}, so each would \
             give main's result; the net runs the rest after"
        );
        return Ok(ExitCode::SUCCESS);
    }
    println!("yes:");
    said_owners(&owning, base, &list);
    if found.touched.is_empty() {
        return Ok(ExitCode::SUCCESS);
    }
    println!("the change touches");
    for (area, hits) in &found.touched {
        println!("  {area}: {}", hits.join(" "));
    }
    for area in &found.there {
        println!(
            "send one VM test through the changed {area}, end to end: run one VM test's \
             subtests with run-subtests --there, with the change's own {area} first on PATH"
        );
    }
    if found.tests.is_empty() {
        println!("no VM test it runs moved from {base}, so none runs whole first");
        return Ok(ExitCode::SUCCESS);
    }
    println!("run these VM tests whole before merging, since they moved from {base}:");
    for test in &found.tests {
        println!("{}", rules::named(test, &list));
    }
    if known.is_none() {
        println!("(the names are patterns: no list of the checks was found to match them against)");
    }
    Ok(ExitCode::SUCCESS)
}

/// The recent passed runs that could be `commit`'s change: those carrying
/// one of its own subjects and touching one of its `files`, the first read
/// being cheap and the second not. Should git fail to say either, every
/// record goes on, as slow as before but no less right.
fn candidates(commit: &str, main: &str, files: &[String]) -> Result<Vec<String>, String> {
    let records = ready::recent_records();
    let subjects = lines(&git(&["log", "--format=%s", &format!("{main}..{commit}")])?);
    let records = logged(&records, &["--format=%H%x01%s"]).map_or(records.clone(), |said| {
        ready::titled(&records, &said, &subjects)
    });
    Ok(
        logged(&records, &["--format=%x01%H", "--name-only"]).map_or(records.clone(), |touched| {
            ready::likely(&records, &touched, files)
        }),
    )
}

/// Name the VM tests that own a file the change touches, each of which is
/// the one written to catch it, so it runs before the merge rather than
/// on main.
fn said_owners(owning: &[String], base: &str, list: &serde_json::Value) {
    if owning.is_empty() {
        return;
    }
    println!(
        "run the subtests that reach what it touched with run-subtests --there, in each VM \
         test that owns one of its files and moved from {base}:"
    );
    for test in owning {
        println!("  {}", rules::named(test, list));
    }
}

/// Say whether `commit` may merge: on origin's main as it is now, with what
/// changed-checks named there built.
fn ready(commit: &str) -> Result<ExitCode, String> {
    let commit = git(&["rev-parse", "--verify", &format!("{commit}^{{commit}}")])?;
    git(&["fetch", "--quiet", "origin", "main"])?;
    let main = git(&["rev-parse", "refs/remotes/origin/main"])?;
    let holds = |older: &str, newer: &str| {
        Command::new("git")
            .args(["merge-base", "--is-ancestor", older, newer])
            .status()
            .is_ok_and(|status| status.success())
    };
    if !holds(&main, &commit) {
        eprintln!(
            "merge-net: {} doesn't sit on origin's main as it is now, {}, so what it was \
             checked against isn't what it merges onto; rebase onto origin/main, push, and \
             merge again, which needs no new run where main moved none of its files, nor a \
             crate its crates are in, read or are read by",
            short(&commit),
            short(&main)
        );
        return Ok(ExitCode::FAILURE);
    }
    let on_main = |base: &str| holds(base, &main);
    if !ready::built_there(&ready::record(&commit), &on_main) {
        return by_change(&commit, &main, &on_main);
    }
    let files = lines(&git(&["diff", "--name-only", &main, &commit])?);
    Ok(turn::passed(
        &[&commit],
        &files,
        &format!(
            "ready: {} sits on origin's main and changed-checks built what it named there",
            short(&commit)
        ),
    ))
}

/// What ready says of `commit`, the same change that passed at `newest`
/// and `earlier`, where main since moved only `shared`.
fn twice(commit: &str, newest: &str, earlier: &str, shared: &[String]) -> String {
    format!(
        "ready: {} sits on origin's main, and changed-checks built what it named at {} and \
         {}, the same change, so a clean rebase needs no third run though main has since \
         moved {}; the net tests it after it merges",
        short(commit),
        short(newest),
        short(earlier),
        shared.join(", ")
    )
}

/// Whether `commit`, a head on `main` with no run of its own, is ready by
/// an earlier commit of the same change, saying what to do when it isn't.
fn by_change(commit: &str, main: &str, on_main: &dyn Fn(&str) -> bool) -> Result<ExitCode, String> {
    let files = lines(&git(&["diff", "--name-only", main, commit])?);
    let renamed = gone(main, commit)?;
    let at = |rev: &str, file: &str| git(&["show", &format!("{rev}:{file}")]).unwrap_or_default();
    let at_head = |file: &str| at(commit, file);
    let at_main = |file: &str| at(main, file);
    let change = patch_id(Path::new("."), main, commit);
    let base_of = |candidate: &str| git(&["merge-base", candidate, main]).ok();
    // What main did since `base`: the files it changed and the paths it
    // took away.
    let since = |base: &str| {
        let since = lines(&git(&["diff", "--name-only", base, main]).ok()?);
        let taken_away = gone(base, main).ok()?;
        Some((since, taken_away))
    };
    let id_of = |base: &str, candidate: &str| patch_id(Path::new("."), base, candidate);
    // What main moved in a crate the change's crates are in, read or are
    // read by, no run of an earlier base speaks for.
    let crates = reach::Crates::at(commit);
    let branch = ready::Side {
        changed: &files,
        gone: &renamed,
        text: &at_head,
    };
    let shared_since = |candidate: &str, base: String| {
        let (since, taken_away) = since(&base)?;
        let main_side = ready::Side {
            changed: &since,
            gone: &taken_away,
            text: &at_main,
        };
        let mut shared = ready::shared(&branch, &main_side);
        let (at_base, checked) = (|f: &str| at(&base, f), |f: &str| at(candidate, f));
        shared.retain(|file| !ready::stands_apart(file, &branch, &main_side, &at_base, &checked));
        let beside = crates.beside(&files, &since);
        shared.retain(|file| !beside.contains(file));
        shared.extend(beside);
        Some(shared)
    };
    let same = |candidate: &str| {
        if candidate == commit {
            return None;
        }
        ready::when_same(candidate, change.as_deref()?, base_of, id_of, |base| {
            shared_since(candidate, base)
        })
    };
    let names_gone = |candidate: &str| {
        let (since, taken_away) = since(&base_of(candidate)?)?;
        let main_side = ready::Side {
            changed: &since,
            gone: &taken_away,
            text: &at_main,
        };
        let mut naming = ready::names_gone(&branch, &main_side);
        naming.extend(crates.beside(&files, &since));
        Some(naming)
    };
    let candidates = candidates(commit, main, &files)?;
    let mut before = ready::checked_before(&candidates, &ready::record, on_main, &same);
    if let ready::Before::Twice(newest, earlier, shared) = before {
        if names_gone(&newest).is_some_and(|naming| naming.is_empty()) {
            let said = twice(commit, &newest, &earlier, &shared);
            return Ok(turn::passed(&[&newest, &earlier], &files, &said));
        }
        // Main renamed or deleted a path one side names, which neither
        // run saw, so it runs again as a change checked once would.
        before = ready::Before::Shares(newest, shared);
    }
    match before {
        ready::Before::Clean(checked) => {
            let said = format!(
                "ready: {} sits on origin's main, and changed-checks built what it named at \
                 {}, the same change, whose files main hasn't moved since",
                short(commit),
                short(&checked)
            );
            return Ok(turn::passed(&[&checked], &files, &said));
        }
        ready::Before::Shares(checked, shared) => {
            let waits = || turn::waiting(&[commit, &checked], &files);
            if let Some(said) = lists_say(commit, &shared, &waits) {
                return Ok(said);
            }
            // Its run again keeps the place this pass's gate had.
            turn::keep_place(&[commit, &checked]);
            eprintln!(
                "merge-net: the same change passed at {}, but main has since moved {}; run \
                 changed-checks at {}, committed so it's sent, which builds only what the \
                 rebase moved, and merge again",
                short(&checked),
                shared.join(", "),
                short(commit)
            );
        }
        ready::Before::Twice(..) => unreachable!("a change checked twice was judged above"),
        ready::Before::Unchecked => eprintln!(
            "merge-net: no changed-checks run that built what it named passed at {}, or at \
             an earlier commit of the same change; run changed-checks there, committed so \
             it's sent, and merge again",
            short(commit)
        ),
    }
    Ok(ExitCode::FAILURE)
}

/// What the changed-checks it was built with (the profile has none) says
/// moved between `base` and the working copy, or nothing when it couldn't compare.
fn moved_from(base: &str) -> Option<String> {
    let ran = Command::new(option_env!("MERGE_NET_CHANGED_CHECKS").unwrap_or("changed-checks"))
        .args(rules::changed_checks_asks(base, SYSTEM))
        .stderr(Stdio::inherit())
        .output()
        .ok()?;
    let said = String::from_utf8_lossy(&ran.stdout).into_owned();
    // It fails when a check no longer evaluates, and names it.
    (ran.status.success() || said.lines().any(|line| line.starts_with("broken"))).then_some(said)
}

/// The VM tests core has, from the list each new main writes
/// (`~/.local/state/check-names/core.json`), where there is one, since
/// evaluating the flake to name them waits on the machine's memory.
fn known_vm_tests() -> Option<Vec<String>> {
    let path = PathBuf::from(env::var_os("HOME")?).join(".local/state/check-names/core.json");
    let names: serde_json::Value = serde_json::from_str(&fs::read_to_string(path).ok()?).ok()?;
    let tests: Vec<String> = names["checks"][SYSTEM]
        .as_array()?
        .iter()
        .filter_map(|name| name.as_str())
        .filter(|name| name.starts_with("vm-"))
        .map(str::to_owned)
        .collect();
    (!tests.is_empty()).then_some(tests)
}

/// Say where the net is: the head it last passed, what it looked at last,
/// any hold, and how many findings it has written.
fn status() -> ExitCode {
    let state = State::from_env();
    let (passed, seen) = state.heads();
    let name = |head: Option<String>| {
        head.map_or_else(|| "none".to_owned(), |head| short(&head).to_owned())
    };
    println!("passed  {}", name(passed));
    println!("seen    {}", name(seen));
    match state.hold() {
        // One its search couldn't settle leaves switches alone, and says so.
        Some(hold) if hold["unsettled"] == true => println!("unsettled {hold}, switches go on"),
        Some(hold) if hold["already_runs"] == true => {
            println!("hold    {hold}, which the machine already runs, so switches go on");
        }
        Some(hold) => println!("hold    {hold}"),
        None => println!("hold    none"),
    }
    let findings = fs::read_dir(state.root.join("findings")).map_or(0, Iterator::count);
    println!(
        "findings {findings}, in {}",
        state.root.join("findings").display()
    );
    if state.paused() {
        println!("paused");
    }
    ExitCode::SUCCESS
}

/// What lists say of `commit`, when every file it shares with main's moves
/// is a list each change adds to: whether each holds there with its own
/// check passed, saying which way it went, and holding a ready one back
/// while it `waits` its turn. None when some file isn't one.
fn lists_say(commit: &str, shared: &[String], waits: &dyn Fn() -> bool) -> Option<ExitCode> {
    let quarantine = git(&["show", &format!("{commit}:{}", world::QUARANTINE)]).unwrap_or_default();
    match lists::lists(shared, &ready::record(commit), &quarantine) {
        lists::Lists::Need(need) if need.is_empty() => {
            if waits() {
                return Some(ExitCode::FAILURE);
            }
            println!(
                "ready: {} sits on origin's main, and main moved only lists of its ({}), whose \
                 own checks passed there",
                short(commit),
                shared.join(", ")
            );
            return Some(ExitCode::SUCCESS);
        }
        lists::Lists::Need(need) => eprintln!(
            "merge-net: main has since moved only lists of {} ({}); build {} there, committed \
             so it's sent, and merge again",
            short(commit),
            shared.join(", "),
            need.iter()
                .map(|check| format!("`nix-offload there . checks.{SYSTEM}.{check}`"))
                .collect::<Vec<_>>()
                .join(" and ")
        ),
        lists::Lists::Broken(why) => eprintln!(
            "merge-net: {} at {} no longer reads as the net's list: {why}",
            world::QUARANTINE,
            short(commit)
        ),
        lists::Lists::NotAll => return None,
    }
    Some(ExitCode::FAILURE)
}

/// The paths renamed or deleted from `base` to `head`, by their old names.
fn gone(base: &str, head: &str) -> Result<Vec<String>, String> {
    let said = git(&[
        "diff",
        "--name-status",
        "-M",
        "--diff-filter=DR",
        base,
        head,
    ])?;
    Ok(said
        .lines()
        .filter_map(|line| line.split('\t').nth(1))
        .map(str::to_owned)
        .collect())
}

fn lines(text: &str) -> Vec<String> {
    text.lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The stable patch id of the change from `base` to `head`, in the repo
/// at `repo`, which stays the same when a rebase moves the change without
/// changing it. The diff carries no lines of context: with git's three, a
/// rebase over a commit that edits only the lines beside the change gave
/// it a new id, as happened to #2504 in a busy words.json, so its earlier
/// run stopped counting. What main moved in the change's files is caught
/// by the files it shares, not by the id.
fn patch_id(repo: &Path, base: &str, head: &str) -> Option<String> {
    let diff = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["diff", "--unified=0", base, head])
        .output()
        .ok()
        .filter(|ran| ran.status.success())?
        .stdout;
    let mut id = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["patch-id", "--stable"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .ok()?;
    {
        use std::io::Write;
        id.stdin.take()?.write_all(&diff).ok()?;
    }
    let said = id.wait_with_output().ok()?;
    String::from_utf8_lossy(&said.stdout)
        .split_whitespace()
        .next()
        .map(str::to_owned)
}

/// What `git log` says of each of `commits`, in `format`, from one git for
/// them all: its subject for `ready::titled`, or what it changed in its own
/// commit for `ready::likely`. A commit this clone lacks is left out.
fn logged(commits: &[String], format: &[&str]) -> Option<String> {
    let mut log = Command::new("git")
        .args(["log", "--no-walk=unsorted", "--ignore-missing", "--stdin"])
        .args(format)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .ok()?;
    {
        use std::io::Write;
        // git reads every name before it prints, so this can't fill the
        // pipe it writes to.
        log.stdin
            .take()?
            .write_all(commits.join("\n").as_bytes())
            .ok()?;
    }
    let said = log.wait_with_output().ok()?;
    said.status
        .success()
        .then(|| String::from_utf8_lossy(&said.stdout).into_owned())
}

fn git(arguments: &[&str]) -> Result<String, String> {
    let ran = Command::new("git")
        .args(arguments)
        .output()
        .map_err(|error| format!("running git: {error}"))?;
    if !ran.status.success() {
        return Err(format!(
            "git {} failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&ran.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&ran.stdout).trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_watch_ends_once_another_build_is_installed() {
        let dir = env::temp_dir().join(format!("merge-net-follow-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let (old, new) = (dir.join("old"), dir.join("new"));
        fs::write(&old, "").unwrap();
        fs::write(&new, "").unwrap();
        let installed = || fs::canonicalize(&new);
        assert!(
            !replaced(installed(), Ok(new.clone())),
            "the build installed runs"
        );
        assert!(
            replaced(installed(), Ok(old.clone())),
            "an older build runs"
        );
        assert!(
            replaced(installed(), Ok(dir.join("collected"))),
            "the running build is gone from disk"
        );
        assert!(
            !replaced(fs::canonicalize(dir.join("unread")), Ok(old)),
            "nothing can be read of what's installed"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    /// Runs git in `repo` with no user's or system's config, so the test
    /// reads the same wherever it runs, and gives what it printed. The
    /// other modules' tests run their git through it too.
    pub(crate) fn git_in(repo: &Path, arguments: &[&str]) -> String {
        let ran = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(arguments)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .output()
            .unwrap();
        assert!(ran.status.success(), "git {arguments:?}: {ran:?}");
        String::from_utf8_lossy(&ran.stdout).trim().to_owned()
    }

    /// Commits `words` as the file `w` and gives the commit.
    fn commit(repo: &Path, words: &str) -> String {
        fs::write(repo.join("w"), words).unwrap();
        git_in(repo, &["commit", "-qam", "w"]);
        git_in(repo, &["rev-parse", "HEAD"])
    }

    #[test]
    fn a_change_rebased_over_an_edit_beside_it_keeps_its_patch_id() {
        let repo = env::temp_dir().join(format!("merge-net-patch-{}", std::process::id()));
        fs::create_dir_all(&repo).unwrap();
        git_in(&repo, &["init", "-q", "-b", "main"]);
        fs::write(repo.join("w"), "a\nb\nc\nd\ne\n").unwrap();
        git_in(&repo, &["add", "w"]);
        git_in(&repo, &["commit", "-qm", "w"]);
        let base = git_in(&repo, &["rev-parse", "HEAD"]);
        let change = commit(&repo, "a\nb\nC\nd\ne\n");
        git_in(&repo, &["checkout", "-q", &base]);
        let moved = commit(&repo, "a\nB\nc\nd\ne\n");
        let rebased = commit(&repo, "a\nB\nC\nd\ne\n");
        let other = commit(&repo, "a\nB\nc\nD\ne\n");
        assert_eq!(
            patch_id(&repo, &base, &change),
            patch_id(&repo, &moved, &rebased),
            "main's edit beside the change leaves it the same change"
        );
        assert_ne!(
            patch_id(&repo, &base, &change),
            patch_id(&repo, &rebased, &other),
            "a change of other lines is another change"
        );
        fs::remove_dir_all(&repo).unwrap();
    }
}
