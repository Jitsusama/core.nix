//! What the command line asks for, read from its words.
//!
//! On 12 November `pi-packages hold --help` held every swap with "--help"
//! as its reason, since a hold took any words after it as why. The terminal
//! kit now answers `--help` and `-h` wherever they stand, before a word is
//! read here; `help` as the command, or as a hold's first word, asks for it
//! too. Any other word opening with a dash is refused, never held, unless
//! it comes after `--`.

use std::path::Path;
use std::time::Duration;

use terminal_kit::{Exit, Failure, flags};

use crate::hold;
use crate::run::Pace;

/// What it does, by the first word; a bare line is `status`.
pub const COMMANDS: [&str; 5] = ["status", "update", "sweep", "hold", "release"];

/// The options it reads itself; the kit takes the shared ones first.
const OPTIONS: [&str; 3] = ["--config", "--now", "--for"];

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

/// A line asked wrongly, saying how.
pub fn usage(what: impl Into<String>) -> Failure {
    Failure::new(Exit::Usage, what).try_this("pi-packages --help")
}

/// The options a line gave, and the words left once they're out.
#[derive(Default)]
struct Given<'a> {
    config: Option<&'a str>,
    now: bool,
    length: Option<Duration>,
    words: Vec<&'a str>,
}

fn given<'a>(words: &[&'a str]) -> Result<Given<'a>, Failure> {
    let mut given = Given::default();
    let mut words = words.iter().copied();
    while let Some(word) = words.next() {
        let (flag, attached) = match word.split_once('=') {
            Some((flag, value)) if flag.starts_with("--") => (flag, Some(value)),
            _ => (word, None),
        };
        let mut value = || {
            attached
                .or_else(|| words.next())
                .ok_or_else(|| usage(format!("{flag} needs a value")))
        };
        match flag {
            "--" => given.words.extend(words.by_ref()),
            "--config" => given.config = Some(value()?),
            "--now" => given.now = true,
            "--for" => {
                let said = value()?;
                let length = hold::lasting(said).ok_or_else(|| {
                    usage(format!("{said} is no length")).because("a hold lasts as 90m or 2h")
                })?;
                given.length = Some(length);
            }
            // The kit reads it from the line itself, to say a failure in full.
            "--verbose" | "-v" => {}
            flag if flag.len() > 1 && flag.starts_with('-') => {
                return Err(flags::unknown("option", flag, &OPTIONS));
            }
            _ => given.words.push(word),
        }
    }
    Ok(given)
}

/// What `words`, the arguments after the program's name and the kit's own
/// flags, ask for.
///
/// # Errors
///
/// Words that ask for nothing known, a length that is none, an option
/// another command owns, or a hold with no reason, saying which, each a
/// usage failure.
pub fn asked<'a>(words: &[&'a str]) -> Result<Asked<'a>, Failure> {
    let given = given(words)?;
    match given.words[..] {
        ["help", ..] | ["hold", "help", ..] => return Ok(Asked::Help),
        _ => {}
    }
    let command = given.words.first().copied().unwrap_or("status");
    if given.now && command != "update" {
        return Err(usage("--now is for update alone"));
    }
    if given.length.is_some() && command != "hold" {
        return Err(usage("--for is for hold alone"));
    }
    if !COMMANDS.contains(&command) {
        return Err(flags::unknown("command", command, &COMMANDS));
    }
    let file = Path::new(given.config.ok_or_else(|| {
        usage("no settings were named")
            .because("the account's pi-packages names them with --config FILE")
    })?);
    let rest = given.words.get(1..).unwrap_or_default();
    match (command, rest) {
        ("hold", []) => {
            Err(usage("a hold needs a reason").try_this("pi-packages hold a demo at 2"))
        }
        ("hold", why) => Ok(Asked::Hold {
            file,
            why: why.join(" "),
            length: given.length,
        }),
        (_, [_, ..]) => Err(usage(format!("{command} takes no words"))),
        ("update", []) => Ok(Asked::Update {
            file,
            pace: if given.now { Pace::Now } else { Pace::Spaced },
        }),
        ("sweep", []) => Ok(Asked::Sweep(file)),
        ("release", []) => Ok(Asked::Release(file)),
        _ => Ok(Asked::Status(file)),
    }
}
