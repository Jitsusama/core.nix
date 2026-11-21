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
//! that reach it run first with `run-subtests --there` (`owners/`).
//! Which moved it reads from a run that passed at a clean head, and asks
//! changed-checks only where none did (`gated.rs`, `needs.rs`).
//!
//! `merge-net ready COMMIT`, asked before a branch's head merges, exits 0
//! only when the head sits on origin's main as it is now and changed-checks
//! built what it named there (`ready.rs`), and otherwise says what to run.
//! It waits, too, for a gate that started before the head's own and shares
//! its files, so changes merge in the order their gates started (`turn.rs`),
//! a change run again after a merge left its pass stale keeping the place
//! its tree's first gate had (`places.rs`). A change that passed, and that
//! main then moved beside, boards the merge train, `merge-net ride`, which
//! merges every change aboard to main together on one run (`train.rs`).
//! Where it can't, or one fell off, `ready` starts its run again itself,
//! in the asking tree, where nothing else goes there (`rerun.rs`).
//!
//! `merge-net status` says where the net is.
//!
//! `watch --branch` follows another branch than main, and lands its reverts
//! there, to prove the net on a throwaway branch; `MERGE_NET_STATE` then
//! names a state folder of its own, so its hold stops no switch.
//!
//! `watch --follow PATH` names the installed merge-net: once that is
//! another build than the one running, the watch finishes its round and
//! ends, for its unit to start the new one, which takes up what the searches
//! left from what was recorded, rather than a switch stopping it and losing
//! every run in flight.
//!
//! `merge-net --help` says how each is asked (`command.rs`).

mod aboard;
mod answers;
mod apart;
mod boarding;
mod candidates;
mod command;
mod comments;
mod earlier;
mod gated;
mod holds;
mod kit;
mod lists;
mod macs;
mod movers;
mod needs;
mod owners;
mod places;
mod quarantine;
mod ready;
mod rerun;
mod rules;
mod running;
mod sharing;
mod stall;
mod state;
mod subtests;
mod train;
mod turn;
mod waited;
mod watch;
mod whole;
mod world;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

use command::Asked;
use state::{State, short};
use watch::Round;
use world::Real;

/// Where the net watches unless `MERGE_NET_REPOSITORY` names another.
const REPOSITORY: &str = "https://github.com/Jitsusama/core.nix";

/// The system whose checks the net runs.
const SYSTEM: &str = "x86_64-linux";

fn main() -> ExitCode {
    // The kit reads the words, answering help and refusing what's asked
    // wrongly. What was asked then runs outside it, since each subcommand
    // says its own lines and ends 0 or 1, which the merge gate and every
    // session's gate read as they are.
    let mut asked = None;
    let read = terminal_kit::main(&command::help(), env!("CARGO_PKG_VERSION"), |context| {
        asked = Some(command::asked(&context.args)?);
        Ok(())
    });
    let Some(asked) = asked else {
        return read;
    };
    let done = match asked {
        Asked::Watch {
            from,
            branch,
            once,
            follow,
        } => watching(from, branch, once, follow),
        Asked::Needs { base } => needs::needs(&base),
        Asked::Ready { commit } => ready(&commit),
        Asked::Status => Ok(status()),
        Asked::RunAgain { commit } => rerun::again(&commit),
        Asked::Ride => {
            train::ride(&env::var("MERGE_NET_REPOSITORY").unwrap_or_else(|_| REPOSITORY.to_owned()))
        }
    };
    done.unwrap_or_else(|why| {
        eprintln!("merge-net: {why}");
        ExitCode::FAILURE
    })
}

/// Follow `branch`, a round at a time, until stopped, or once with `once`.
fn watching(
    from: Option<String>,
    branch: String,
    once: bool,
    follow: Option<PathBuf>,
) -> Result<ExitCode, String> {
    let state = State::from_env();
    state.make()?;
    // The train starts as, and follows, the build the watch follows, not
    // whichever build asked for it. A watch that can't say so still
    // watches: the train then starts as the asker's build, as before.
    if let Some(installed) = &follow
        && let Err(error) = train::install(&state.root, installed)
    {
        eprintln!("merge-net: couldn't say where merge-net is installed: {error}");
    }
    let world = Real {
        state: state.root.clone(),
        url: env::var("MERGE_NET_REPOSITORY").unwrap_or_else(|_| REPOSITORY.to_owned()),
        system: SYSTEM.to_owned(),
        booted: holds::booted().unwrap_or(0),
        branch,
        moving: std::sync::Mutex::new(()),
        dropped: std::sync::Mutex::default(),
        follow,
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
            // Never joined: it ends with the net, which it aborts if hung.
            let guarded = State::at(state.root.clone());
            std::thread::spawn(move || stall::guard(&guarded));
        }
        let ended = rounds(&world, &state, &start, &settler, once);
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
/// installed where the watch follows.
fn rounds(
    world: &Real,
    state: &State,
    start: &str,
    settler: &watch::Settler,
    once: bool,
) -> Result<ExitCode, String> {
    loop {
        let round = watch::round_beside(world, state, start, settler);
        // A round another build replaced runs to its end, and what it
        // settles is the new build's, taken up from what was recorded.
        if !once && watch::World::superseded(world) {
            state.say(
                "another merge-net is installed, so this one ends with its round, leaving \
                 what it settles to that one",
            );
            // An end at once, since the scope would wait for the searches
            // under way, which the new build takes up from disk. It's the
            // kit's, so the code is the table's 0 as any other end is.
            return Ok(settler.leave(|| terminal_kit::Exit::Done.end_now()));
        }
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
        std::thread::sleep(std::time::Duration::from_secs(rest));
    }
}

/// The recent passed runs that could be `commit`'s change, as
/// `candidates::could_be` chooses them from what git says of each.
fn candidates(
    commit: &str,
    main: &str,
    files: &[String],
    records: &[String],
) -> Result<Vec<String>, String> {
    let subjects = lines(&git(&["log", "--format=%s", &format!("{main}..{commit}")])?);
    let said = logged(records, &["--format=%H%x01%s"]);
    let touched = |records: &[String]| logged(records, &["--format=%x01%H", "--name-only"]);
    Ok(candidates::could_be(
        records,
        said.as_deref(),
        &subjects,
        touched,
        files,
    ))
}

/// Say whether `commit` may merge: on origin's main as it is now, with what
/// changed-checks named there built.
fn ready(commit: &str) -> Result<ExitCode, String> {
    let commit = git(&["rev-parse", "--verify", &format!("{commit}^{{commit}}")])?;
    if let Some(said) = train::riding(&commit) {
        eprintln!(
            "merge-net: {} rides the merge train: {said}",
            short(&commit)
        );
        return Ok(ExitCode::FAILURE);
    }
    git(&["fetch", "--quiet", "origin", "main"])?;
    let main = git(&["rev-parse", "refs/remotes/origin/main"])?;
    let holds = |older: &str, newer: &str| {
        Command::new("git")
            .args(["merge-base", "--is-ancestor", older, newer])
            .status()
            .is_ok_and(|status| status.success())
    };
    let on_main = |base: &str| holds(base, &main);
    if !holds(&main, &commit) {
        // On 15 November #4791 rebased four times in 20 minutes as main
        // moved eight times, none of them in its files, and no rebase ran
        // anything again. A head behind main is judged as a rebased one
        // is, by its own run first, unless its base lacks main's newest
        // rules for which VM tests run, which its needs answer can't know.
        let rules = newest_rules(&main);
        if rules.is_empty() || holds(&rules, &commit) {
            return by_change(&commit, &main, &on_main);
        }
        eprintln!(
            "merge-net: {} doesn't sit on origin's main as it is now, {}, and its base \
             lacks main's newest rules for which VM tests run (must-run.txt, lock-after.txt), \
             so what \
             needs said of it can't speak for them; rebase onto origin/main, push, and \
             merge again",
            short(&commit),
            short(&main)
        );
        return Ok(ExitCode::FAILURE);
    }
    if !ready::built_there(&ready::record(&commit), &on_main) {
        return by_change(&commit, &main, &on_main);
    }
    let files = lines(&git(&["diff", "--name-only", &main, &commit])?);
    Ok(may_merge(
        &commit,
        &main,
        &[&commit],
        &files,
        &format!(
            "ready: {} sits on origin's main and changed-checks built what it named there",
            short(&commit)
        ),
    ))
}

/// Say `ready` of `commit`, whose checks passed at `passes`, once `needs`
/// was asked of it, and refuse it until then.
fn may_merge(commit: &str, main: &str, passes: &[&str], files: &[String], said: &str) -> ExitCode {
    if needs_heard(commit, main, passes, files) {
        mark_skipped_macs(commit, passes);
        // A change held for an earlier gate sharing its files stays held,
        // rather than board the train and land ahead of it.
        if turn::waiting(commit, passes, files) {
            return ExitCode::FAILURE;
        }
        // A draft is its owner's word that it isn't to land yet, and
        // GitHub merges none, so it neither boards the train nor is told
        // it may merge.
        let branch = git(&["symbolic-ref", "--quiet", "--short", "HEAD"]).unwrap_or_default();
        if let Some(said) = (!branch.is_empty())
            .then(|| train::drafted(&branch))
            .flatten()
        {
            eprintln!("merge-net: {said}");
            return ExitCode::FAILURE;
        }
        // Before the right of way, which boards the train itself, so a
        // head no round read neither boards it nor merges.
        if let Err(said) = earlier::same::read_by_a_round(commit, main, files) {
            eprintln!("merge-net: {said}");
            return ExitCode::FAILURE;
        }
        if let Some(said) = train::gives_way(commit) {
            eprintln!("merge-net: {said}");
            return ExitCode::FAILURE;
        }
        return turn::passed(commit, passes, files, said);
    }
    eprintln!(
        "merge-net: merge-net needs wasn't asked at {}, nor at an earlier commit of the same \
         change on a base holding main's rules for it (must-run.txt, lock-after.txt), so no \
         one has heard \
         which VM tests run before it merges; run merge-net needs there, run what it names, \
         and merge again",
        short(commit)
    );
    ExitCode::FAILURE
}

/// Mark `commit` for the net to compare a Mac's checks at its merge, when
/// a record speaking for it, its own or one of `passes`', kept one skipped
/// (`macs.rs`). A mark for a head that never merges is never read.
fn mark_skipped_macs(commit: &str, passes: &[&str]) {
    let records: Vec<String> = passes
        .iter()
        .chain([&commit])
        .map(|at| ready::record(at))
        .collect();
    let root = state::State::from_env().root;
    if let Err(error) = macs::keep(&root, commit, &records) {
        eprintln!("merge-net: the Mac's skipped comparison at {commit} wasn't marked: {error}");
    }
}

/// Whether `needs` was asked of `commit`'s change, at it, at one of
/// `passes` or at an earlier commit of the same change, on a base holding
/// main's newest `must-run.txt`. On 15 November #4653 merged at a rebase
/// onto #4770, which added the rule that its initrd moved, after needs had
/// been asked only at commits whose base lacked it.
fn needs_heard(commit: &str, main: &str, passes: &[&str], files: &[String]) -> bool {
    let rules = newest_rules(main);
    let knows_rules = |asked: &str| {
        rules.is_empty()
            || Command::new("git")
                .args(["merge-base", "--is-ancestor", &rules, asked])
                .status()
                .is_ok_and(|status| status.success())
    };
    let mut asked: Vec<&str> = passes.to_vec();
    asked.push(commit);
    asked.retain(|at| knows_rules(at));
    if gated::asked(&asked) {
        return true;
    }
    let records = ready::recent_records();
    let earlier: Vec<String> = earlier::same::same_change(commit, main, files, &records, patch_id)
        .into_iter()
        .filter(|candidate| knows_rules(candidate))
        .collect();
    gated::asked(&earlier.iter().map(String::as_str).collect::<Vec<_>>())
}

/// What to do about `commit`, which must run again: wait for the run
/// `ready` started for it in this tree, or run it, where it was left to
/// its author (`rerun.rs`), once no reader of the kit owed its turn goes
/// first (`kit.rs`).
fn run_again(commit: &str) -> String {
    if let Some(said) = turn::waits_to_run(commit) {
        return said.trim_start_matches("merge-net: ").to_owned();
    }
    let gates = turn::folder()
        .map(|folder| turn::gates(&folder))
        .unwrap_or_default();
    env::current_dir()
        .ok()
        .and_then(|here| rerun::offer(&here, commit, &gates))
        .unwrap_or_else(|| {
            format!(
                "run changed-checks at {}, committed so it's sent, which builds only what \
                 the rebase moved, and merge again",
                short(commit)
            )
        })
}

/// The newest commit on `main` to change the rules for which VM tests run
/// before a merge, the inputs a lock may move alone among them, or nothing
/// when git can't say.
fn newest_rules(main: &str) -> String {
    git(&[
        "log",
        "-1",
        "--format=%H",
        main,
        "--",
        "tools/merge-net/must-run.txt",
        rules::LOCK_AFTER,
    ])
    .unwrap_or_default()
}

/// Where `commit` stands against origin's main at `main`, as `ready` says
/// it: on it, or behind it and merging as it stands.
fn stands(commit: &str, main: &str, behind: bool) -> String {
    if behind {
        format!(
            "{} is behind origin's main, {}, and merges as it stands,",
            short(commit),
            short(main)
        )
    } else {
        format!("{} sits on origin's main,", short(commit))
    }
}

/// What ready says of a head standing as `stands` says, the same change
/// that passed at `newest` and `earlier`, where main since moved only
/// `shared`.
fn twice(stands: &str, newest: &str, earlier: &str, shared: &[String]) -> String {
    format!(
        "ready: {stands} and changed-checks built what it named at {} and {}, the same \
         change, so a clean rebase needs no third run though main has since moved {}; the \
         net tests it after it merges",
        short(newest),
        short(earlier),
        shared.join(", ")
    )
}

/// Whether `commit`, a head on `main` with no run of its own, is ready by
/// an earlier commit of the same change, saying what to do when it isn't.
fn by_change(commit: &str, main: &str, on_main: &dyn Fn(&str) -> bool) -> Result<ExitCode, String> {
    let earlier::Judged {
        stands,
        behind,
        files,
        before,
    } = earlier::judged(commit, main, on_main)?;
    match before {
        ready::Before::Twice(newest, earlier, shared) => {
            let said = twice(&stands, &newest, &earlier, &shared);
            return Ok(may_merge(commit, main, &[&newest, &earlier], &files, &said));
        }
        ready::Before::Clean(checked) => {
            let said = format!(
                "ready: {stands} and changed-checks built what it named at {}, the same \
                 change, whose files main hasn't moved since",
                short(&checked)
            );
            return Ok(may_merge(commit, main, &[&checked], &files, &said));
        }
        ready::Before::Shares(checked, shared) => {
            let passes = |said: &str| may_merge(commit, main, &[commit, &checked], &files, said);
            let quarantine = git(&["show", &format!("{commit}:{}", world::QUARANTINE)]);
            let quarantine = quarantine.unwrap_or_default();
            if let Some(said) = lists::say(commit, &shared, &quarantine, &passes) {
                return Ok(said);
            }
            // The train merges what it carries, so a head no round read
            // boards no more than it merges through may_merge.
            if let Err(said) = earlier::same::read_by_a_round(commit, main, &files) {
                eprintln!("merge-net: {said}");
                return Ok(ExitCode::FAILURE);
            }
            // A head behind main boards the train as one on it does
            // (`boarding.rs`), and one that can't is told why.
            if behind {
                let boarded = aboard::boards(commit, main, &checked, &files);
                if boarded.is_ok() {
                    aboard::keep_regated(commit, &boarded, &shared);
                }
                let said = boarding::behind_said(commit, main, &checked, &shared, &boarded);
                eprintln!("merge-net: {said}");
                return Ok(ExitCode::FAILURE);
            }
            // Its run again keeps the place this pass's gate had.
            turn::keep_place(&[commit, &checked]);
            let boarded = aboard::boards(commit, main, &checked, &files);
            aboard::keep_regated(commit, &boarded, &shared);
            eprintln!(
                "merge-net: the same change passed at {}, but main has since moved {}; {}",
                short(&checked),
                shared.join(", "),
                aboard::ride_or_run_again(commit, &boarded)
            );
        }
        ready::Before::Unchecked => eprintln!(
            "merge-net: no changed-checks run that built what it named passed at {}, or at \
             an earlier commit of the same change; run changed-checks there, committed so \
             it's sent, and merge again",
            short(commit)
        ),
    }
    Ok(ExitCode::FAILURE)
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
    patch_of(repo, base, head, &["--unified=0"], "--stable")
}

/// The id `git patch-id` gives, taken `kind`, of the diff from `base` to
/// `head` with `options`.
fn patch_of(repo: &Path, base: &str, head: &str, options: &[&str], kind: &str) -> Option<String> {
    let diff = Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("diff")
        .args(options)
        .args([base, head])
        .output()
        .ok()
        .filter(|ran| ran.status.success())?
        .stdout;
    let mut id = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["patch-id", kind])
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
/// them all: its subject, or what it changed in its own commit, for
/// `candidates::could_be`. A commit this clone lacks is left out.
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
    git_at(Path::new("."), arguments)
}

/// What git says, run in the repo at `repo`.
fn git_at(repo: &Path, arguments: &[&str]) -> Result<String, String> {
    let ran = Command::new("git")
        .arg("-C")
        .arg(repo)
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
    pub(crate) fn commit(repo: &Path, words: &str) -> String {
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
