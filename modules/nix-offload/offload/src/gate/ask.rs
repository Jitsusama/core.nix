//! `nix-offload ask --quest QUEST [--session ADDRESS] [--on MACHINE]
//! [--longest SECONDS]`: asks the machine that gates to run `there
//! --changed` for this tree's commit, from a machine with no way to the
//! builders, and says what it found.
//!
//! It leaves the commits since origin's main as a bundle, and the request,
//! in the quest's scratch `handover/gates/<commit>/`, shares the quest so
//! mycelium carries them, and has mycelium start `nix-offload answer` on
//! the machine named, by `--on` or in `nix-offload/gates-on` under XDG's
//! config folder. It then waits for `result.json` to come back beside
//! them, at most `--longest` seconds, and exits as the gate did.

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
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

/// mycelium, run with `arguments`, what it said passed on to stderr.
fn mycelium(arguments: &[&str]) -> Result<(), String> {
    let output = Command::new("mycelium")
        .args(arguments)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("mycelium couldn't start: {error}"))?;
    eprint!("{}", String::from_utf8_lossy(&output.stdout));
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

/// The result once it's there for `commit`, waited for at most `longest`.
fn waited(gate: &Path, commit: &str, longest: Duration) -> Option<Found> {
    let since = Instant::now();
    loop {
        let found = fs::read_to_string(gate.join(RESULT))
            .ok()
            .and_then(|text| Found::parse(&text))
            .filter(|found| found.commit == commit);
        if found.is_some() {
            return found;
        }
        if since.elapsed() >= longest {
            return None;
        }
        thread::sleep(LOOKING);
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
    let gate = super::folder(&asked.quest, &commit);
    if let Err(why) = left(&request, &gate) {
        return Err(failed(
            Exit::Bug,
            "ask couldn't leave the gate's request",
            &why,
            "Check the quest's scratch folder.",
        ));
    }
    started(&asked.quest, &on, &commit)?;
    eprintln!(
        "nix-offload: asked {on} to gate {commit}; waiting for its result in {}",
        gate.display()
    );
    let Some(found) = waited(&gate, &commit, asked.longest) else {
        return Err(failed(
            NOT_YET,
            "ask had no result in time",
            &format!(
                "{} brought none back within {} s",
                on,
                asked.longest.as_secs()
            ),
            "Ask again later; a gate that passed meanwhile kept its pass.",
        ));
    };
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

/// The quest shared to `on` and `answer` started there for `commit`, or
/// why not, already said.
fn started(quest: &str, on: &str, commit: &str) -> Result<(), Failure> {
    // Shared to the machine and waited for, where both ends' mycelium
    // can; otherwise shared, and `answer` finds the gate once it crosses.
    mycelium(&["share", quest, "--to", on, "--wait"])
        .or_else(|_| mycelium(&["share", quest]))
        .map_err(|why| {
            failed(
                Exit::Unavailable,
                "ask couldn't share the quest",
                &why,
                "Check `mycelium machines` reaches the gate's machine.",
            )
        })?;
    let machine = format!("machine={on}");
    let answering = [
        "start",
        "--need",
        &machine,
        "--",
        "nix-offload",
        "answer",
        quest,
        commit,
    ];
    mycelium(&answering).map_err(|why| {
        failed(
            Exit::Unavailable,
            "ask couldn't start the gate",
            &why,
            "Check `mycelium machines` says it's ready.",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<String> {
        text.split(' ').map(str::to_owned).collect()
    }

    #[test]
    fn an_ask_ends_with_the_gates_own_codes() {
        assert_eq!(REFUSES.code(), REFUSED);
        assert_eq!(NOT_YET.code(), NOT_NOW);
    }

    #[test]
    fn an_ask_names_its_quest_and_may_name_the_rest() {
        assert_eq!(
            asked(&words(
                "--quest QEST-20261115-ABC123 --session SK1FF0 --on kestrel --longest 60"
            )),
            Ok(Asked {
                quest: "QEST-20261115-ABC123".to_owned(),
                session: Some("SK1FF0".to_owned()),
                on: Some("kestrel".to_owned()),
                longest: Duration::from_secs(60),
            }),
            "every option should be read"
        );
        assert!(
            asked(&words("--session SK1FF0")).is_err(),
            "an ask without a quest should be refused"
        );
        assert!(
            asked(&words("--quest ../x")).is_err(),
            "a quest that isn't an id should be refused"
        );
        assert!(
            asked(&words("--quest QEST-20261115-ABC123 --session $(x)")).is_err(),
            "a session that isn't an address should be refused"
        );
    }
}
