//! `nix-offload ask --quest QUEST [--session ADDRESS] [--on MACHINE]
//! [--longest SECONDS]`: asks the machine that gates to run `there
//! --changed` for this tree's commit, from a machine with no way to the
//! builders, and says what it found.
//!
//! It leaves the commits since origin's main as a bundle, and the request,
//! in the quest's scratch `handover/gates/<commit>/`, shares the quest so
//! mycelium carries them, and has mycelium run `nix-offload answer` in
//! that folder on the machine named, by `--on` or in
//! `nix-offload/gates-on` under XDG's config folder. It then waits for
//! `result.json` to come back beside them, at most `--longest` seconds,
//! and exits as the gate did. An answer that ends without one is told by
//! how it ended once a result has had time to cross back, rather than
//! waited out, and one that couldn't start in its gate's folder, not yet
//! crossed, is started again.

use std::fs;
use std::os::fd::AsFd;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use terminal_kit::{Exit, Failure};

use super::{BUNDLE, Found, LARGEST_BUNDLE, LOG, NOT_NOW, REFUSED, REQUEST, RESULT, Repo, Request};

/// How long `ask` waits for a result unless told: a gate's run is an
/// evaluation of each side and the builds, which took up to an hour under
/// load in November.
const LONGEST: Duration = Duration::from_hours(2);
/// How often it looks for the result.
const LOOKING: Duration = Duration::from_secs(5);
/// How long it still looks once an answer has ended, however it ended,
/// for a result written on another machine to cross back: the answer
/// ends with its gate's own code once the result is written, and that
/// code may be any, 255 included, as a gate's ssh ends.
const CROSSING_BACK: Duration = Duration::from_mins(10);
/// How long a stopped answer is given to end before it's killed, so
/// mycelium can pass the stop on to the machine that runs it.
const STOPPING: Duration = Duration::from_secs(10);
/// How many times an answer that couldn't start in its gate's folder is
/// started: on a Mac's plain share the folder may not have crossed yet,
/// and mycelium then starts it in the home, where its fence refuses it.
const STARTS: u32 = 3;
/// What `mycelium run` ends with when the node refused the run, when it
/// couldn't run it, and when the link dropped while it ran; a gate whose
/// ssh failed ends so too.
const UNRUN: i32 = 255;
/// What the answer's shell ends with when its fence refused to start it.
const UNSTARTED: i32 = 126;

/// The gate's own refusal and not-now, as the table names them, so an ask
/// ends as an answer to it would.
const REFUSES: Exit = exit_of(REFUSED);
const NOT_YET: Exit = exit_of(NOT_NOW);

/// The table's exit for a gate's code, which is always one of its own.
const fn exit_of(code: u8) -> Exit {
    match code {
        65 => Exit::BadInput,
        75 => Exit::TryAgain,
        _ => Exit::Bug,
    }
}

/// What `ask` was told.
#[derive(Debug, PartialEq, Eq)]
struct Asked {
    quest: String,
    session: Option<String>,
    on: Option<String>,
    longest: Duration,
}

/// Reads `ask`'s arguments, or says what's wrong with them.
fn asked(arguments: &[String]) -> Result<Asked, String> {
    let (mut quest, mut session, mut on, mut longest) = (None, None, None, LONGEST);
    let mut words = arguments.iter();
    while let Some(word) = words.next() {
        let mut value = || {
            words
                .next()
                .cloned()
                .ok_or_else(|| format!("{word} needs a value"))
        };
        match word.as_str() {
            "--quest" => quest = Some(value()?).filter(|quest| super::plain_quest(quest)),
            "--session" => {
                let given = value()?;
                if !super::plain_session(&given) {
                    return Err(format!("{given} isn't a mesh address"));
                }
                session = Some(given);
            }
            "--on" => on = Some(value()?),
            "--longest" => {
                let given = value()?;
                longest = Duration::from_secs(
                    given
                        .parse()
                        .map_err(|_| format!("{given} isn't a count of seconds"))?,
                );
            }
            other => return Err(format!("it doesn't take {other}")),
        }
    }
    let quest = quest.ok_or("it needs --quest and a quest's id")?;
    Ok(Asked {
        quest,
        session,
        on,
        longest,
    })
}

/// git in the tree, its output trimmed, or what it said.
fn git(arguments: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(arguments)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("git couldn't start: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

/// The machine that gates, as `nix-offload/gates-on` names it.
fn configured_machine() -> Option<String> {
    let named = super::configured("gates-on")?;
    Some(named.trim().to_owned()).filter(|name| !name.is_empty())
}

/// What went wrong, as docs/commands.md's three parts, ending with `exit`.
fn failed(exit: Exit, what: &str, why: &str, instead: &str) -> Failure {
    Failure::new(exit, what).because(format!("{why}. {instead}"))
}

/// The gate's folder made afresh, holding the bundle and the request.
fn left(request: &Request, gate: &Path) -> Result<(), String> {
    if gate.exists() {
        fs::remove_dir_all(gate).map_err(|error| format!("{}: {error}", gate.display()))?;
    }
    fs::create_dir_all(gate).map_err(|error| format!("{}: {error}", gate.display()))?;
    // Hidden while it's written, since mycelium carries no hidden file.
    let writing = gate.join(format!(".{BUNDLE}.writing"));
    git(&[
        "bundle",
        "create",
        "--quiet",
        &writing.to_string_lossy(),
        "origin/main..HEAD",
    ])?;
    let size = fs::metadata(&writing).map_or(0, |meta| meta.len());
    if size > LARGEST_BUNDLE {
        return Err(format!(
            "the commits since origin's main are {size} bytes, past the {LARGEST_BUNDLE} a shared folder carries"
        ));
    }
    fs::rename(&writing, gate.join(BUNDLE)).map_err(|error| error.to_string())?;
    super::write_whole(gate, REQUEST, request.json().as_bytes())
}

/// Why mycelium didn't do what it was asked.
#[derive(Debug, PartialEq, Eq)]
enum Said {
    /// The node answered no, so asking it again another way would only
    /// get round its no.
    Refused(String),
    /// mycelium couldn't start, or ended without the node's answer.
    Failed(String),
}

/// mycelium, run with `arguments`, what it said passed on to stderr.
fn mycelium(arguments: &[&str]) -> Result<(), Said> {
    let output = Command::new("mycelium")
        .args(arguments)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| Said::Failed(format!("mycelium couldn't start: {error}")))?;
    let said = String::from_utf8_lossy(&output.stdout);
    eprint!("{said}");
    judged(
        output.status.success(),
        &said,
        &String::from_utf8_lossy(&output.stderr),
    )
}

/// What a run of mycelium that `ran` cleanly or not comes to, from what it
/// `said` and its `errors`. The node's reply, where it printed one as
/// `{"Ok": …}` or `{"Err": …}`, decides whatever mycelium's exit: it ends
/// 0 once it has the reply, whatever that says, so a refused start read by
/// its exit alone left ask waiting for a gate that was never started.
fn judged(ran: bool, said: &str, errors: &str) -> Result<(), Said> {
    match reply(said) {
        Some(answer) => answer.map_err(Said::Refused),
        None if ran => Ok(()),
        None => Err(Said::Failed(
            Some(errors.trim())
                .filter(|errors| !errors.is_empty())
                .map_or_else(
                    || "mycelium failed, saying nothing why".to_owned(),
                    str::to_owned,
                ),
        )),
    }
}

/// The node's first reply in `said`: the whole of it read as one, where
/// mycelium printed it across lines, or else its first line that is one.
fn reply(said: &str) -> Option<Result<(), String>> {
    std::iter::once(said)
        .chain(said.lines())
        .filter_map(|text| serde_json::from_str::<serde_json::Value>(text).ok())
        .find_map(|value| {
            let object = value.as_object()?;
            if object.contains_key("Ok") {
                return Some(Ok(()));
            }
            let why = object.get("Err")?;
            let why = why.as_str().map_or_else(|| why.to_string(), str::to_owned);
            Some(Err(if why.trim().is_empty() {
                "the node refused, saying nothing why".to_owned()
            } else {
                why
            }))
        })
}

/// What `ask` tells of mycelium's `said`, as `what` failed: a refusal ends
/// as one, since asking again won't change the node's mind, and isn't
/// sent to check what can be reached, which didn't stop it.
fn failed_by(said: Said, what: &str, instead: &str) -> Failure {
    match said {
        Said::Refused(why) => failed(
            Exit::Refused,
            what,
            &format!("the node refused: {why}"),
            "Asking again won't change its answer",
        ),
        Said::Failed(why) => failed(Exit::Unavailable, what, &why, instead),
    }
}

/// How the answer's run ended.
#[derive(Debug, Clone, PartialEq, Eq)]
enum End {
    /// With this code.
    Exited(i32),
    /// By a signal.
    Signalled,
    /// Unknown, as it couldn't be watched, for this reason.
    Unwatched(String),
}

impl End {
    /// It, in words such as `exited 126`.
    fn said(&self) -> String {
        match self {
            Self::Exited(code) => format!("exited {code}"),
            Self::Signalled => "was ended by a signal".to_owned(),
            Self::Unwatched(why) => format!("couldn't be watched: {why}"),
        }
    }
}

/// The answer's run on the gating machine, as `ask` watches it.
trait Answering {
    /// How it ended, or none while it runs.
    fn ended(&mut self) -> Option<End>;
    /// Stops it, once `ask` waits no longer.
    fn stop(&mut self);
}

impl Answering for Child {
    fn ended(&mut self) -> Option<End> {
        match self.try_wait() {
            Ok(None) => None,
            Ok(Some(status)) => Some(status.code().map_or(End::Signalled, End::Exited)),
            Err(error) => Some(End::Unwatched(error.to_string())),
        }
    }

    fn stop(&mut self) {
        // Asked to stop first, so mycelium can pass the stop on to the
        // answer; killed only once it hasn't ended in `STOPPING`. One
        // already gone leaves nothing to stop, which is no failure.
        if let Ok(pid) = libc::pid_t::try_from(self.id()) {
            // SAFETY: a signal to this process's own child, not yet reaped.
            unsafe { libc::kill(pid, libc::SIGTERM) };
            let since = Instant::now();
            while since.elapsed() < STOPPING {
                if !matches!(self.try_wait(), Ok(None)) {
                    return;
                }
                thread::sleep(Duration::from_millis(100));
            }
        }
        let _ = self.kill();
        let _ = self.wait();
    }
}

/// How waiting for a gate's result went.
#[derive(Debug, PartialEq, Eq)]
enum Waited {
    /// The result came back.
    Found(Found),
    /// The answer ended, so, and no result came back after it.
    Ended(End),
    /// Nothing came back within the longest wait.
    Late,
}

/// How `waited` paces itself: at most `longest` in all, `crossing` after
/// an answer ended, and a look each `looking`.
#[derive(Debug, Clone, Copy)]
struct Pace {
    longest: Duration,
    crossing: Duration,
    looking: Duration,
}

/// The result in `gate` for `commit`, if it's there.
fn result(gate: &Path, commit: &str) -> Option<Found> {
    fs::read_to_string(gate.join(RESULT))
        .ok()
        .and_then(|text| Found::parse(&text))
        .filter(|found| found.commit == commit)
}

/// The result once it's there for `commit`, waited for as `pace` says:
/// while `answer` runs, and once it ended, however it ended, for
/// `crossing`, so a result written as it ended has time to come back.
/// An answer that ended without one, refused by its fence, by the node
/// or finding no gate, is told by how it ended then, rather than waited
/// out. An answer still running once a result is back is left to end
/// within its `--lasts`: the result may be an earlier run's for the same
/// commit, and stopping the run would lose its own.
fn waited(gate: &Path, commit: &str, pace: Pace, answer: &mut dyn Answering) -> Waited {
    let since = Instant::now();
    let mut ended: Option<(End, Instant)> = None;
    loop {
        if let Some(found) = result(gate, commit) {
            return Waited::Found(found);
        }
        if ended.is_none() {
            ended = answer.ended().map(|end| (end, Instant::now()));
        }
        // A result written just before the end was heard is looked for
        // once more, so it isn't told as missing.
        let told = |waited: Waited| result(gate, commit).map_or(waited, Waited::Found);
        if let Some((end, at)) = &ended
            && at.elapsed() >= pace.crossing
        {
            return told(Waited::Ended(end.clone()));
        }
        if since.elapsed() >= pace.longest {
            if ended.is_none() {
                answer.stop();
            }
            return told(Waited::Late);
        }
        thread::sleep(pace.looking);
    }
}

/// The result of `commit`'s gate, from answers `start` starts, each
/// given what's left of `longest`, and waited for in `gate` as `pace`
/// says; or why there's none, already said. An answer that couldn't
/// start, as though its gate's folder hadn't crossed, is started again,
/// `STARTS` times at most while time is left.
fn gated(
    on: &str,
    gate: &Path,
    commit: &str,
    pace: Pace,
    start: &mut dyn FnMut(Duration) -> Result<Box<dyn Answering>, Failure>,
) -> Result<Found, Failure> {
    let since = Instant::now();
    let mut starts = 0;
    loop {
        let left = Pace {
            longest: pace.longest.saturating_sub(since.elapsed()),
            ..pace
        };
        let mut answer = start(left.longest)?;
        starts += 1;
        eprintln!(
            "nix-offload: asked {on} to gate {commit}; waiting for its result in {}",
            gate.display()
        );
        match waited(gate, commit, left, answer.as_mut()) {
            Waited::Found(found) => return Ok(found),
            Waited::Ended(End::Exited(UNSTARTED))
                if starts < STARTS && since.elapsed() < pace.longest =>
            {
                eprintln!(
                    "nix-offload: the answer on {on} couldn't start, as though its gate's \
                     folder hadn't crossed yet, so ask starts it again"
                );
            }
            Waited::Ended(End::Exited(UNRUN)) => {
                return Err(failed(
                    Exit::Unavailable,
                    "ask's gate ended without a result",
                    &format!(
                        "the answer on {on} exited {UNRUN} and wrote none: the node refused \
                         or couldn't run it, or the link to it dropped, as its words above say"
                    ),
                    "Check `mycelium machines` says it's ready, and ask again.",
                ));
            }
            Waited::Ended(end) => {
                return Err(failed(
                    Exit::Unavailable,
                    "ask's gate ended without a result",
                    &format!(
                        "the answer on {on} {} and wrote none, as its words above say",
                        end.said()
                    ),
                    "Check the gate's folder reached that machine, and ask again.",
                ));
            }
            Waited::Late => {
                return Err(failed(
                    NOT_YET,
                    "ask had no result in time",
                    &format!("{on} brought none back within {} s", pace.longest.as_secs()),
                    "Ask again later; a gate that passed meanwhile kept its pass.",
                ));
            }
        }
    }
}

/// `ask`: the gate of this tree's commit, asked of the machine that gates.
pub fn ask(arguments: &[String]) -> Result<(), Failure> {
    let asked = match asked(arguments) {
        Ok(asked) => asked,
        Err(why) => {
            return Err(crate::ended::usage(format!("ask can't read that: {why}"))
                .try_this("nix-offload ask --help"));
        }
    };
    let Some(on) = asked.on.clone().or_else(configured_machine) else {
        return Err(failed(
            Exit::Config,
            "ask doesn't know which machine gates",
            "neither --on nor nix-offload/gates-on names one",
            "Name it with --on, or set jitsusama.nix-offload.gatesOn in the machine's home.",
        ));
    };
    let (commit, repo) = gated_here()?;
    let request = Request {
        repo,
        commit: commit.clone(),
        quest: asked.quest.clone(),
        session: asked.session.clone(),
    };
    let Some(gate) = super::folder(&asked.quest, &commit) else {
        return Err(failed(
            Exit::Config,
            "ask has nowhere to leave the gate's request",
            super::NO_SCRATCH,
            "Set HOME, or XDG_CACHE_HOME, to an absolute folder.",
        ));
    };
    if let Err(why) = left(&request, &gate) {
        return Err(failed(
            Exit::Bug,
            "ask couldn't leave the gate's request",
            &why,
            "Check the quest's scratch folder.",
        ));
    }
    shared(&asked.quest, &on)?;
    let pace = Pace {
        longest: asked.longest,
        crossing: CROSSING_BACK,
        looking: LOOKING,
    };
    let found = gated(&on, &gate, &commit, pace, &mut |longest| {
        answering(&asked.quest, &on, &commit, &gate, longest)
            .map(|child| Box::new(child) as Box<dyn Answering>)
    })?;
    if let Ok(log) = fs::read_to_string(gate.join(LOG)) {
        eprint!("{log}");
    }
    println!("{}", found.said);
    crate::ended::coded(found.exit, || {
        format!("the gate of {commit} didn't pass, as the line above and its log say")
    })
}

/// This tree's commit and its repo on GitHub, once it's committed whole
/// and past origin's main, or why it can't be gated, already said.
fn gated_here() -> Result<(String, Repo), Failure> {
    let commit = git(&["rev-parse", "--verify", "HEAD^{commit}"]).map_err(|why| {
        failed(
            Exit::NotFound,
            "ask needs a git tree with a commit",
            &why,
            "Run it in the tree to gate.",
        )
    })?;
    match git(&["status", "--porcelain", "--untracked-files=no"]) {
        Ok(changes) if changes.is_empty() => {}
        Ok(_) => {
            return Err(failed(
                REFUSES,
                "ask gates a commit",
                "the tree has changes not committed, which the gate wouldn't see",
                "Commit them, or set them aside, and ask again.",
            ));
        }
        Err(why) => {
            return Err(failed(
                Exit::NotFound,
                "ask couldn't read the tree's changes",
                &why,
                "Run it in the tree to gate.",
            ));
        }
    }
    let Some(repo) = git(&["remote", "get-url", "origin"])
        .ok()
        .as_deref()
        .and_then(Repo::of_remote)
    else {
        return Err(failed(
            REFUSES,
            "ask gates a repo on GitHub",
            "origin isn't a GitHub repo",
            "Point origin at the repo on GitHub.",
        ));
    };
    if let Err(why) = git(&["fetch", "--quiet", "origin", "main"]) {
        eprintln!(
            "nix-offload: ask couldn't fetch origin's main, so it bundles from the one it has: {why}"
        );
    }
    if git(&["rev-list", "--count", "origin/main..HEAD"]).as_deref() == Ok("0") {
        return Err(failed(
            REFUSES,
            "ask has nothing to gate",
            "HEAD has no commit past origin's main",
            "Commit the change first.",
        ));
    }
    Ok((commit, repo))
}

/// The quest shared to `on`, or why not, already said.
fn shared(quest: &str, on: &str) -> Result<(), Failure> {
    // Shared to the machine and waited for, where both ends' mycelium
    // can; otherwise shared, and `answer` finds the gate once it crosses.
    // Any no to the narrow share falls back to the plain one, as before:
    // a node whose mycelium can't wait answers it with an `Err` too, and
    // a Mac's gate rests on the plain share.
    let sharing = "Check `mycelium machines` reaches the gate's machine.";
    if mycelium(&["share", quest, "--to", on, "--wait"]).is_err() {
        mycelium(&["share", quest])
            .map_err(|said| failed_by(said, "ask couldn't share the quest", sharing))?;
    }
    Ok(())
}

/// The words `mycelium run` is given to answer the gate of `commit` in
/// `gate`, on `on`, for at most `longest`.
fn answer_run(quest: &str, on: &str, commit: &str, gate: &Path, longest: Duration) -> Vec<String> {
    [
        "run".to_owned(),
        "--need".to_owned(),
        format!("machine={on}"),
        // Started in the gate's own folder, which its fence lets it
        // read: started in the home, which holds git folders, the fence
        // refused it and it ended 126 before it ran.
        "--folder".to_owned(),
        gate.display().to_string(),
        "--lasts".to_owned(),
        format!("{}m", longest.as_secs().div_ceil(60)),
        "--".to_owned(),
        "nix-offload".to_owned(),
        "answer".to_owned(),
        quest.to_owned(),
        commit.to_owned(),
    ]
    .into()
}

/// `answer` run on `on` for `commit` through `mycelium run`, which ends as
/// it ends, so `ask` hears an answer that never wrote a result. What it
/// says goes to stderr, as stdout is the gate's own line.
fn answering(
    quest: &str,
    on: &str,
    commit: &str,
    gate: &Path,
    longest: Duration,
) -> Result<Child, Failure> {
    let unstarted = |why: String| {
        failed(
            Exit::Unavailable,
            "ask couldn't start the gate",
            &why,
            "Check `mycelium machines` says it's ready.",
        )
    };
    let words = std::io::stderr()
        .as_fd()
        .try_clone_to_owned()
        .map_err(|error| unstarted(format!("stderr couldn't be shared: {error}")))?;
    Command::new("mycelium")
        .args(answer_run(quest, on, commit, gate, longest))
        .stdin(Stdio::null())
        .stdout(Stdio::from(words))
        .spawn()
        .map_err(|error| unstarted(format!("mycelium couldn't start: {error}")))
}

#[cfg(test)]
mod tests;
