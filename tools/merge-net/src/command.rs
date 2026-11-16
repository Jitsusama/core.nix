//! What merge-net and each of its subcommands say they are, take and end
//! with, as `--help` shows it (docs/commands.md), and what it was asked,
//! read from the words before any of it runs, so a word asked wrongly is
//! refused before git or the net's state is touched.

use std::path::PathBuf;

use terminal_kit::{Exit, Failure, Help, flags};

/// The base `needs` compares with unless `--base` names another.
const ORIGIN_MAIN: &str = "refs/remotes/origin/main";

/// The flags every command takes, which the kit takes before these words
/// reach a subcommand: one left in the words of a subcommand that takes
/// none was mistyped, and the nearest of these is what was meant.
const SHARED: [&str; 5] = ["--help", "--version", "--json", "--format", "--color"];

/// What it was asked to do.
pub enum Asked {
    /// Follow a branch of origin, a round at a time.
    Watch {
        from: Option<String>,
        branch: String,
        once: bool,
        follow: Option<PathBuf>,
    },
    /// Say whether the change runs VM tests before it merges.
    Needs { base: String },
    /// Say whether a commit may merge.
    Ready { commit: String },
    /// Say where the net is.
    Status,
    /// Run changed-checks again at a commit, as `ready` starts it.
    RunAgain { commit: String },
}

/// merge-net's help, with each subcommand's under it.
pub fn help() -> Help {
    Help::new(
        "merge-net",
        "the net under main: runs the checks each merge moved, and reverts one that breaks",
    )
    .example(
        "merge-net needs",
        "says whether this change runs VM tests before it merges",
    )
    .example(
        "merge-net ready $(git rev-parse HEAD)",
        "says whether this head may merge, or what to run first",
    )
    .example("merge-net status", "says where the net is")
    .exit(
        Exit::Failed,
        "not so, or it couldn't find out; stderr says which",
    )
    .subcommand(watch())
    .subcommand(needs())
    .subcommand(ready())
    .subcommand(status())
    .subcommand(run_again())
}

fn watch() -> Help {
    Help::new(
        "watch",
        "follows origin's main, running what each merge moved",
    )
    .example(
        "merge-net watch --follow ~/.nix-profile/bin/merge-net",
        "as its unit runs it, ending once another build is installed there",
    )
    .example("merge-net watch --once", "runs one round, then ends")
    .option(
        "--from COMMIT",
        "starts at COMMIT when nothing has passed yet",
    )
    .option(
        "--branch NAME",
        "follows NAME rather than main, landing reverts there",
    )
    .option("--once", "runs one round and ends")
    .option(
        "--follow PATH",
        "ends after its round once PATH is another merge-net",
    )
    .exit(
        Exit::Failed,
        "a round with --once failed, or the net couldn't start",
    )
}

fn needs() -> Help {
    Help::new(
        "needs",
        "says whether this change runs VM tests before it merges, and which",
    )
    .example("merge-net needs", "against origin's main")
    .example("merge-net needs --base origin/next", "against another base")
    .option(
        "--base REF",
        "compares with REF's merge base; origin's main unless named",
    )
    .exit(Exit::Failed, "it couldn't find out; stderr says why")
}

fn ready() -> Help {
    Help::new(
        "ready",
        "says whether COMMIT may merge, or what to run first",
    )
    .example(
        "merge-net ready $(git rev-parse HEAD)",
        "asked before the head merges",
    )
    .argument("COMMIT", "the head about to merge")
    .exit(
        Exit::Failed,
        "it may not merge yet; stderr says what to run",
    )
}

fn status() -> Help {
    Help::new("status", "says where the net is: its heads, holds and runs")
        .example("merge-net status", "what it's running and what it holds")
}

fn run_again() -> Help {
    Help::new(
        "run-again",
        "runs changed-checks again at COMMIT, as ready starts it in a tree",
    )
    .example(
        "merge-net run-again $(git rev-parse HEAD)",
        "what ready runs when main moved under a pass",
    )
    .argument("COMMIT", "the head whose pass went stale")
    .exit(Exit::Failed, "changed-checks failed at COMMIT")
}

/// What the words, past the kit's own flags, ask it to do.
///
/// # Errors
///
/// Asked wrongly, 64: no subcommand, a flag it doesn't take, a flag short
/// of its value, or a subcommand short of its word.
pub fn asked(words: &[String]) -> Result<Asked, Failure> {
    let Some((first, rest)) = words.split_first() else {
        return Err(Failure::new(Exit::Usage, "merge-net needs a subcommand")
            .because("watch, needs, ready, status or run-again")
            .try_this("merge-net --help"));
    };
    let rest: Vec<&str> = rest.iter().map(String::as_str).collect();
    match first.as_str() {
        "watch" => watching(&rest),
        "needs" => match rest.as_slice() {
            [] => Ok(Asked::Needs {
                base: ORIGIN_MAIN.to_owned(),
            }),
            ["--base", base] => Ok(Asked::Needs {
                base: (*base).to_owned(),
            }),
            ["--base"] => Err(short_of("--base needs a ref", "needs")),
            [word, ..] => Err(unknown(word, &["--base"], "needs")),
        },
        "ready" => commit(&rest, "ready").map(|commit| Asked::Ready { commit }),
        "run-again" => commit(&rest, "run-again").map(|commit| Asked::RunAgain { commit }),
        // The kit refuses a word a subcommand with no arguments doesn't
        // take, so only a flag reaches here.
        "status" => match rest.as_slice() {
            [] => Ok(Asked::Status),
            [word, ..] => Err(unknown(word, &SHARED, "status")),
        },
        // The kit refuses a word no subcommand answers to before this.
        other => {
            Err(flags::unknown("command", other, &help().command_names())
                .try_this("merge-net --help"))
        }
    }
}

/// What `watch` was asked to follow, from the words after it.
fn watching(words: &[&str]) -> Result<Asked, Failure> {
    let (mut from, mut branch, mut once, mut follow) = (None, "main".to_owned(), false, None);
    let mut words = words.iter();
    while let Some(word) = words.next() {
        let mut value = |what: &str| {
            words
                .next()
                .map(|value| (*value).to_owned())
                .ok_or_else(|| short_of(&format!("{word} needs {what}"), "watch"))
        };
        match *word {
            "--from" => from = Some(value("a commit")?),
            "--branch" => branch = value("a name")?,
            "--follow" => follow = Some(PathBuf::from(value("a path")?)),
            "--once" => once = true,
            other => {
                return Err(unknown(
                    other,
                    &["--from", "--branch", "--once", "--follow"],
                    "watch",
                ));
            }
        }
    }
    Ok(Asked::Watch {
        from,
        branch,
        once,
        follow,
    })
}

/// The one commit a subcommand is asked about.
fn commit(words: &[&str], sub: &str) -> Result<String, Failure> {
    match words {
        [commit] if !commit.starts_with('-') => Ok((*commit).to_owned()),
        [] => Err(short_of(&format!("{sub} needs the COMMIT to judge"), sub)),
        [word, ..] => Err(unknown(word, &SHARED, sub)),
    }
}

fn short_of(what: &str, sub: &str) -> Failure {
    Failure::new(Exit::Usage, what).try_this(format!("merge-net {sub} --help"))
}

fn unknown(word: &str, known: &[&str], sub: &str) -> Failure {
    flags::unknown("option", word, known).try_this(format!("merge-net {sub} --help"))
}
