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
    /// Merge the changes waiting on the train, as `ready` starts it.
    Ride,
    /// Run `ready` for a pull request at its head from a clone at its
    /// branch, for a room that can't reach the train itself.
    Door { pr: u64, head: String },
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
    .subcommand(ride())
    .subcommand(door())
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

fn ride() -> Help {
    Help::new(
        "ride",
        "merges the changes waiting on the train to main on one run, as ready starts it",
    )
    .example(
        "merge-net ride",
        "what ready starts when main moved beside a pass",
    )
    .exit(Exit::Failed, "its checkout couldn't be made or moved")
}

fn door() -> Help {
    Help::new(
        "door",
        "runs ready for PR at HEAD from a clone at its branch, so the train can take it",
    )
    .example(
        "merge-net door 7361 $(git rev-parse HEAD)",
        "what a room asks through mycelium run, for a PR it can't board itself",
    )
    .argument("PR", "the pull request's number")
    .argument("HEAD", "the head GitHub has for it, all 40 characters")
    .passes("ready's own code")
    .exit(
        Exit::Failed,
        "GitHub couldn't be asked or the clone couldn't be made",
    )
    .exit(
        Exit::Refused,
        "the PR isn't one the door takes, or its branch moved",
    )
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
            .because("watch, needs, ready, status, run-again, ride or door")
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
        "ride" => match rest.as_slice() {
            [] => Ok(Asked::Ride),
            [word, ..] => Err(unknown(word, &SHARED, "ride")),
        },
        "door" => door_asked(&rest),
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

/// The pull request and head `door` was asked about. The head is taken
/// only whole, so it names one commit and no ref a branch could move.
fn door_asked(words: &[&str]) -> Result<Asked, Failure> {
    match words {
        [pr, head] => {
            let pr = pr_number(pr).ok_or_else(|| {
                short_of(&format!("door's PR is a number from 1, not {pr}"), "door")
            })?;
            if !whole_commit(head) {
                return Err(short_of(
                    &format!("door's HEAD is a commit's 40 lowercase hex digits, not {head}"),
                    "door",
                ));
            }
            Ok(Asked::Door {
                pr,
                head: (*head).to_owned(),
            })
        }
        [word, ..] if word.starts_with('-') => Err(unknown(word, &SHARED, "door")),
        _ => Err(short_of("door needs the PR and the HEAD to ready", "door")),
    }
}

/// A pull request's number, from its digits alone, so `+7` or `0x7` is no
/// number of one.
fn pr_number(word: &str) -> Option<u64> {
    if word.is_empty() || !word.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    word.parse().ok().filter(|number| *number > 0)
}

/// Whether `word` is a commit's whole name as GitHub gives it: 40 lowercase
/// hex digits.
fn whole_commit(word: &str) -> bool {
    word.len() == 40
        && word
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn short_of(what: &str, sub: &str) -> Failure {
    Failure::new(Exit::Usage, what).try_this(format!("merge-net {sub} --help"))
}

fn unknown(word: &str, known: &[&str], sub: &str) -> Failure {
    flags::unknown("option", word, known).try_this(format!("merge-net {sub} --help"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asking(words: &[&str]) -> Asked {
        let words: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
        let Ok(asked) = asked(&words) else {
            panic!("{words:?} was refused, though a caller asks it so");
        };
        asked
    }

    /// The words the net's unit, the merge gate and ready's rerun ask with
    /// today, which each must keep taking as they are.
    #[test]
    fn every_callers_words_are_taken_as_they_were() {
        let watched = asking(&["watch", "--follow", "/profile/bin/merge-net"]);
        assert!(
            matches!(&watched, Asked::Watch { from: None, branch, once: false, follow: Some(path) }
                if branch == "main" && path == &PathBuf::from("/profile/bin/merge-net")),
            "the unit's watch --follow PATH wasn't taken as following main from PATH"
        );
        let once = asking(&["watch", "--once", "--from", "c1", "--branch", "next"]);
        assert!(
            matches!(&once, Asked::Watch { from: Some(from), branch, once: true, follow: None }
                if from == "c1" && branch == "next"),
            "watch --once --from c1 --branch next lost one of its words"
        );
        assert!(
            matches!(asking(&["needs"]), Asked::Needs { base } if base == ORIGIN_MAIN),
            "a bare needs didn't compare with origin's main"
        );
        assert!(
            matches!(asking(&["needs", "--base", "r1"]), Asked::Needs { base } if base == "r1"),
            "needs --base r1 didn't compare with r1"
        );
        assert!(
            matches!(asking(&["ready", "abc123"]), Asked::Ready { commit } if commit == "abc123"),
            "ready COMMIT didn't judge COMMIT"
        );
        assert!(
            matches!(asking(&["run-again", "c1"]), Asked::RunAgain { commit } if commit == "c1"),
            "run-again COMMIT, as ready starts it, didn't run COMMIT"
        );
        assert!(
            matches!(asking(&["status"]), Asked::Status),
            "status wasn't taken"
        );
        assert!(
            matches!(asking(&["ride"]), Asked::Ride),
            "ride, as ready starts the train, wasn't taken"
        );
        let head = "08c885690815f5b468316df71fe33c79bf174603";
        assert!(
            matches!(asking(&["door", "7361", head]), Asked::Door { pr: 7361, head: asked }
                if asked == head),
            "door PR HEAD, as a room asks it through mycelium run, didn't ready PR at HEAD"
        );
    }

    /// A door asked of anything but a positive number and a whole commit
    /// is refused as asked wrongly, before GitHub or git is asked.
    #[test]
    fn a_door_asked_of_no_number_or_no_whole_commit_is_refused() {
        let head = "08c885690815f5b468316df71fe33c79bf174603";
        for words in [
            &["door"][..],
            &["door", "7361"],
            &["door", "7361", head, "more"],
            &["door", "0", head],
            &["door", "+7361", head],
            &["door", "-7361", head],
            &["door", "73a1", head],
            &["door", "99999999999999999999999", head],
            &["door", "7361", "08c88569"],
            &["door", "7361", "08C885690815F5B468316DF71FE33C79BF174603"],
            &["door", "7361", "08c885690815f5b468316df71fe33c79bf17460g"],
            &["door", "7361", "08c885690815f5b468316df71fe33c79bf1746030"],
            &["door", "7361", "main"],
        ] {
            let words: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
            match asked(&words) {
                Ok(_) => panic!("{words:?} was taken, though it names no PR at a whole head"),
                Err(failure) => assert_eq!(failure.exit, Exit::Usage, "{words:?}"),
            }
        }
    }
}
