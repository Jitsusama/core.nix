//! What the command line asks for, read from its words.
//!
//! On 12 November `pi-packages hold --help` held every swap with "--help"
//! as its reason, since a hold took any words after it as why. So help is
//! asked for first or straight after `hold`, and a reason that opens with
//! a flag is refused, never held.

use std::path::Path;
use std::time::Duration;

use crate::hold;
use crate::run::Pace;

/// The usage, for help and for words that ask for nothing known.
pub const USAGE: &str = "usage: pi-packages update --config FILE [--now] | sweep --config FILE | \
                         status --config FILE | hold --config FILE [--for 90m|2h] WHY... | \
                         release --config FILE";

/// One thing the command line asks for, naming the settings file it reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asked<'a> {
    Help,
    Update {
        file: &'a Path,
        pace: Pace,
    },
    Sweep(&'a Path),
    Status(&'a Path),
    Hold {
        file: &'a Path,
        why: String,
        length: Option<Duration>,
    },
    Release(&'a Path),
}

/// What `words`, the arguments after the program's name, ask for.
///
/// # Errors
///
/// Words that ask for nothing known, a length that is none, or a hold
/// with no reason or one opening with a flag, saying which.
pub fn asked<'a>(words: &[&'a str]) -> Result<Asked<'a>, String> {
    let file = |name: &&'a str| Path::new(*name);
    match words {
        [first, ..] if is_help(first) => Ok(Asked::Help),
        ["hold", second, ..] if is_help(second) => Ok(Asked::Help),
        ["update", "--config", name] => Ok(Asked::Update {
            file: file(name),
            pace: Pace::Spaced,
        }),
        ["update", "--config", name, "--now"] => Ok(Asked::Update {
            file: file(name),
            pace: Pace::Now,
        }),
        ["sweep", "--config", name] => Ok(Asked::Sweep(file(name))),
        ["status", "--config", name] => Ok(Asked::Status(file(name))),
        ["release", "--config", name] => Ok(Asked::Release(file(name))),
        ["hold", "--config", name, "--for", length, why @ ..] => {
            let length = hold::lasting(length)
                .ok_or_else(|| format!("{length} is no length; say 90m or 2h"))?;
            held(file(name), why, Some(length))
        }
        ["hold", "--config", name, why @ ..] => held(file(name), why, None),
        _ => Err(USAGE.into()),
    }
}

/// Whether a word asks for help.
fn is_help(word: &str) -> bool {
    matches!(word, "--help" | "-h" | "help")
}

/// A hold for `why`, which says something and opens with no flag.
fn held<'a>(file: &'a Path, why: &[&str], length: Option<Duration>) -> Result<Asked<'a>, String> {
    match why.first() {
        None => Err(USAGE.into()),
        Some(word) if is_help(word) => Ok(Asked::Help),
        Some(word) if word.starts_with('-') => Err(format!(
            "{word} reads as a flag, not a reason, so nothing is held; {USAGE}"
        )),
        Some(_) => Ok(Asked::Hold {
            file,
            why: why.join(" "),
            length,
        }),
    }
}
