//! repos: every repository on this machine, each once, read from the disk
//! as the command runs (the crate's lib.rs says what it reads, and how).
//!
//! ```text
//! repos                     every repository as a line of a table
//! repos NAME                one in full: its remote, trunk and checkouts
//! repos NAME trees          every tree cut from it, the newest first
//! ```
//!
//! It takes the shape every command of ours does (docs/commands.md),
//! through the terminal kit. Where to look comes from `REPOS_ROOTS`, which
//! its Home Manager module sets from `jitsusama.repos.roots`, else ~/src.

mod commands;
mod views;

use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use repos::model::Record;
use repos::read::{self, Reading};
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
    match words.as_slice() {
        [] => {
            let record = record(home, now)?;
            show(context, &views::repositories_json(&record), || {
                views::repositories(&record, now, home, &context.out, context.room)
            })
        }
        [name] => {
            let record = record(home, now)?;
            let repository = views::find(&record, name)?;
            show(
                context,
                &views::repository_json(&record, repository),
                || views::repository(repository, now, home, &context.out, context.room),
            )
        }
        [name, "trees" | "tree"] => {
            let record = record(home, now)?;
            let repository = views::find(&record, name)?;
            show(context, &views::trees_json(&record, repository), || {
                views::trees(repository, now, home, &context.out, context.room)
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

/// The record, read from the disk now.
fn record(home: Option<&Path>, now: u64) -> Result<Record, Failure> {
    let roots = roots(home)?;
    let machine = machine();
    Ok(read::read(&Reading {
        roots: &roots,
        home,
        machine: &machine,
        now,
    }))
}
