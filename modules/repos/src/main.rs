//! repos: every repository on this machine, each once, read from the disk
//! as the command runs (the crate's lib.rs says what it reads, and how).
//!
//! ```text
//! repos                     every repository as a line of a table
//! repos NAME                one in full: its remote, trunk and checkouts
//! repos NAME changes        its open changes, with their checks and review
//! repos NAME trees          every tree cut from it, the newest first
//! repos refresh             asks GitHub after each one's open changes
//! ```
//!
//! It takes the shape every command of ours does (docs/commands.md),
//! through the terminal kit. Where to look comes from `REPOS_ROOTS`, which
//! its Home Manager module sets from `jitsusama.repos.roots`, else ~/src.
//! What GitHub was last seen to hold is kept in the state folder, where
//! `repos refresh`, on a timer, writes it and every reading merges it in
//! (the crate's github.rs says why). The few a machine lists in
//! `REPOS_THROUGH_GH`, from `jitsusama.repos.throughGh`, are read through
//! gh's sign-in instead, the `gh` at `REPOS_GH` (the crate's gh.rs).

mod commands;
mod views;

use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use repos::github::{self, Seen};
use repos::model::Record;
use repos::read::{self, Reading};
use repos::when::Clock;
use repos::{broker, gh};
use serde_json::Value;
use terminal_kit::{Context, Exit, Failure};

fn main() -> ExitCode {
    terminal_kit::main(&commands::help(), env!("CARGO_PKG_VERSION"), run)
}

/// Runs what the words ask, which the kit has walked down repos's tree
/// (commands.rs).
fn run(context: &Context) -> Result<(), Failure> {
    let words: Vec<&str> = context.args.iter().map(String::as_str).collect();
    let home = home();
    let home = home.as_deref();
    let now = now();
    let clock = Clock::here(now);
    match words.as_slice() {
        ["refresh"] => {
            let seen = refresh(home, now)?;
            show(context, &views::seen_json(&seen), || {
                views::seen(&seen, &context.out)
            })
        }
        [] => {
            let record = record(home, now)?;
            show(context, &views::repositories_json(&record), || {
                views::repositories(&record, &clock, home, &context.out, context.room)
            })
        }
        [name] => {
            let record = record(home, now)?;
            let repository = views::find(&record, name)?;
            show(
                context,
                &views::repository_json(&record, repository),
                || {
                    views::repository(
                        &record,
                        repository,
                        &clock,
                        home,
                        &context.out,
                        context.room,
                    )
                },
            )
        }
        [name, "changes" | "change"] => {
            let record = record(home, now)?;
            let repository = views::find(&record, name)?;
            show(context, &views::changes_json(&record, repository), || {
                views::changes(&record, repository, &clock, &context.out, context.room)
            })
        }
        [name, "trees" | "tree"] => {
            let record = record(home, now)?;
            let repository = views::find(&record, name)?;
            show(context, &views::trees_json(&record, repository), || {
                views::trees(repository, &clock, home, &context.out, context.room)
            })
        }
        _ => {
            Err(Failure::new(Exit::Usage, "repos doesn't know those words")
                .try_this("repos --help"))
        }
    }
}

/// The answer as JSON when asked for, and otherwise the view laid out.
fn show(context: &Context, answer: &Value, view: impl FnOnce() -> String) -> Result<(), Failure> {
    let written = if context.format().is_json() {
        context.answer(answer)
    } else {
        context.say(&view())
    };
    written.or_else(|error| match error.kind() {
        // A reader that stopped early, as `head` does, ends the answer.
        io::ErrorKind::BrokenPipe => Ok(()),
        _ => Err(
            Failure::new(Exit::Failed, "repos couldn't write its answer")
                .because(error.to_string()),
        ),
    })
}

/// The time now, in seconds since the epoch, as the record's times are.
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// Home, from the process, since the kit's environment holds only the
/// variables it draws by.
fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .map(PathBuf::from)
}

/// The folders to look under: `REPOS_ROOTS`, colon-separated as PATH is,
/// else ~/src.
fn roots(home: Option<&Path>) -> Result<Vec<PathBuf>, Failure> {
    let said = std::env::var("REPOS_ROOTS").unwrap_or_default();
    let roots: Vec<PathBuf> = said
        .split(':')
        .filter(|root| !root.is_empty())
        .map(PathBuf::from)
        .collect();
    if !roots.is_empty() {
        return Ok(roots);
    }
    home.map(|home| vec![home.join("src")]).ok_or_else(|| {
        Failure::new(Exit::Config, "there's nowhere to look for repositories")
            .because("neither REPOS_ROOTS nor HOME is set")
    })
}

/// The machine's name as it calls itself, read where the kernel keeps it.
fn machine() -> String {
    ["/proc/sys/kernel/hostname", "/etc/hostname"]
        .iter()
        .find_map(|file| {
            std::fs::read_to_string(file)
                .ok()
                .map(|name| name.trim().to_owned())
                .filter(|name| !name.is_empty())
        })
        .or_else(|| {
            std::process::Command::new("hostname")
                .output()
                .ok()
                .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
                .filter(|name| !name.is_empty())
        })
        .unwrap_or_else(|| "this machine".to_owned())
}

/// The record, read from the disk now, with what GitHub was last seen to
/// hold where a refresh has asked it.
fn record(home: Option<&Path>, now: u64) -> Result<Record, Failure> {
    let mut record = disk(home, now)?;
    if let Some(file) = github::seen_file(home) {
        github::merge_kept(&mut record, &file);
    }
    Ok(record)
}

/// The record as the disk alone says it.
fn disk(home: Option<&Path>, now: u64) -> Result<Record, Failure> {
    let roots = roots(home)?;
    let machine = machine();
    Ok(read::read(&Reading {
        roots: &roots,
        home,
        machine: &machine,
        now,
    }))
}

/// Where a refresh keeps what it saw (`github::seen_file`), else why
/// there's nowhere.
fn seen_file(home: Option<&Path>) -> Result<PathBuf, Failure> {
    github::seen_file(home).ok_or_else(|| {
        Failure::new(Exit::Config, "there's nowhere to keep what GitHub holds")
            .because("neither XDG_STATE_HOME nor HOME is set")
    })
}

/// Asks GitHub, through the machine's broker, after each repository's open
/// changes, and keeps what it saw, written whole or not at all.
fn refresh(home: Option<&Path>, now: u64) -> Result<Seen, Failure> {
    let unreached =
        |why: String| Failure::new(Exit::Unavailable, "repos couldn't ask GitHub").because(why);
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|runtime| !runtime.is_empty())
        .ok_or_else(|| {
            Failure::new(Exit::Config, "there's no GitHub broker to ask")
                .because("XDG_RUNTIME_DIR isn't set")
        })?;
    let socket = PathBuf::from(runtime).join("github-broker.sock");
    let token = broker::token(&socket)
        .map_err(|why| unreached(why).try_this("systemctl --user status github-broker.socket"))?;
    let record = disk(home, now)?;
    let listed = through_gh();
    let (by_gh, by_app): (Vec<_>, Vec<_>) = github::asked(&record)
        .into_iter()
        .partition(|one| listed.contains(&one.key));
    let mut seen = github::refresh(&by_app, now, |query| {
        github::post(github::GRAPHQL, &token, query).map_err(|failed| failed.why)
    })
    .map_err(unreached)?;
    let gh = std::env::var_os("REPOS_GH").unwrap_or_else(|| "gh".into());
    seen.add(github::refresh_through_gh(
        &by_gh,
        now,
        || gh::token(gh::command(&gh)),
        |token, query| github::post(github::GRAPHQL, token, query),
    ));
    let file = seen_file(home)?;
    github::keep(&file, &seen).map_err(|why| {
        Failure::new(Exit::Failed, "repos couldn't keep what GitHub holds")
            .because(format!("{}: {why}", file.display()))
    })?;
    Ok(seen)
}

/// The repositories a machine lists to be read through gh's sign-in, by
/// their keys, `REPOS_THROUGH_GH` separated by spaces: the few GitHub's
/// App can't see. None by default, so gh is asked only where listed.
fn through_gh() -> BTreeSet<String> {
    std::env::var("REPOS_THROUGH_GH")
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}
