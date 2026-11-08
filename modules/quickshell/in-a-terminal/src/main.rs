//! `launcher`, the launcher in a terminal: any place, row or drawer the
//! running shell's launcher holds, printed as text or as JSON, so every
//! integration the launcher has gets a command line with it (docs/design.md,
//! "As deep as the deepest"). It knows nothing a provider knows: it asks the
//! shell (`qs ipc call launcher show`, launcher/Showing.js), which walks the
//! same rows Return does, and only lays out what comes back.
//!
//!     launcher show [step ...] [--json]
//!
//! Each step is a row's name, in any case, a hyphen for a space, or the
//! start of only one: `launcher show machine disk`. With none it shows
//! home. Text is set as the launcher sets a row, in the theme's roles, and
//! `--json` prints the shell's answer as it came.
//!
//! Where it's installed (../home.nix) says what it asks, how it inks and
//! its words: `LAUNCHER_QS`, Quickshell's `qs`; `LAUNCHER_LOOK`, the
//! theme's roles as escapes and its marks' stand-ins; `LAUNCHER_WORDS`, the
//! launcher's words.json, whose `terminal` section holds what it says of
//! its own. A step that names nothing exits 1, and a shell that doesn't
//! answer exits 2, each said on stderr.

mod asking;
mod layout;
mod look;
mod words;

use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

use unicode_width::UnicodeWidthStr;

use crate::asking::Answer;
use crate::layout::Room;
use crate::look::{Ink, Look};
use crate::words::Words;

/// What a command line asked for.
#[derive(Debug, PartialEq, Eq)]
enum Asked {
    /// A place or a row, by its steps, as text or JSON.
    Show { steps: Vec<String>, json: bool },
    /// How it's used.
    Help,
    /// Something it doesn't take, said back.
    Unknown(String),
}

/// Reads the arguments after the program's name.
fn asked(arguments: &[String]) -> Asked {
    match arguments.first().map(String::as_str) {
        None | Some("-h" | "--help" | "help") => Asked::Help,
        Some("show") => {
            // After `--`, every argument is a step, so one starting with a
            // hyphen can be named too.
            let rest = &arguments[1..];
            let ends = rest.iter().position(|one| one == "--");
            let (flagged, named) = match ends {
                Some(at) => (&rest[..at], &rest[at + 1..]),
                None => (rest, &rest[rest.len()..]),
            };
            if let Some(flag) = flagged
                .iter()
                .find(|one| one.starts_with('-') && *one != "--json")
            {
                return Asked::Unknown(flag.clone());
            }
            Asked::Show {
                steps: flagged
                    .iter()
                    .filter(|one| *one != "--json")
                    .chain(named)
                    .cloned()
                    .collect(),
                json: flagged.iter().any(|one| one == "--json"),
            }
        }
        Some(other) => Asked::Unknown(other.to_string()),
    }
}

/// How the program exits, by what went wrong.
const NOTHING_CALLED: u8 = 1;
const SHELL_SILENT: u8 = 2;
const MISUSED: u8 = 64;

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let words = Words::read(env::var_os("LAUNCHER_WORDS").as_deref());
    match asked(&arguments) {
        Asked::Help => {
            print!("{}", words.usage());
            ExitCode::SUCCESS
        }
        Asked::Unknown(what) => {
            let look = Look::read(env::var_os("LAUNCHER_LOOK").as_deref(), Ink::for_stderr());
            fail(&look, &words.unknown(&what));
            eprint!("{}", words.usage());
            ExitCode::from(MISUSED)
        }
        Asked::Show { steps, json } => show(&steps, json, &words),
    }
}

/// Shows a place or a row, as text or as the shell's JSON.
fn show(steps: &[String], json: bool, words: &Words) -> ExitCode {
    let qs = env::var("LAUNCHER_QS").unwrap_or_else(|_| "qs".to_string());
    let errors = Look::read(env::var_os("LAUNCHER_LOOK").as_deref(), Ink::for_stderr());
    let answer = match asking::show(&qs, steps) {
        Ok(answer) => answer,
        Err(why) => {
            fail(&errors, &words.silent());
            if !why.is_empty() {
                eprintln!("{why}");
            }
            return ExitCode::from(SHELL_SILENT);
        }
    };
    match answer {
        Answer::Missing { step, near, within } => {
            fail(&errors, &words.missing(&step, &within, &near));
            ExitCode::from(NOTHING_CALLED)
        }
        Answer::Shown(shown) => {
            let out = if json {
                format!(
                    "{}\n",
                    serde_json::to_string_pretty(&shown).unwrap_or_default()
                )
            } else {
                let look = Look::read(env::var_os("LAUNCHER_LOOK").as_deref(), Ink::for_stdout());
                layout::shown(&shown, &look, Room::of_stdout(), words)
            };
            // A pipe closed early, as `| head` does, isn't a failure.
            let _ = io::stdout().write_all(out.as_bytes());
            ExitCode::SUCCESS
        }
    }
}

/// Says what went wrong on stderr, after the theme's failure mark in alert,
/// fitted to the terminal by whole words as everything it shows is.
fn fail(look: &Look, said: &str) {
    let mark = look.failure();
    let room = Room::of_stderr();
    let line = if mark.is_empty() {
        layout::hung(said, 0, room)
    } else {
        let lead = UnicodeWidthStr::width(mark.as_str()) + 1;
        format!(
            "{} {}",
            look.ink("alert", &mark),
            layout::hung(said, lead, room)
        )
    };
    eprintln!("{line}");
    let _ = io::stderr().flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(of: &[&str]) -> Vec<String> {
        of.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn nothing_asked_is_help() {
        assert_eq!(asked(&[]), Asked::Help);
        assert_eq!(asked(&words(&["--help"])), Asked::Help);
    }

    #[test]
    fn show_takes_its_steps_and_json_anywhere() {
        assert_eq!(
            asked(&words(&["show", "machine", "--json", "disk"])),
            Asked::Show {
                steps: words(&["machine", "disk"]),
                json: true
            }
        );
        assert_eq!(
            asked(&words(&["show"])),
            Asked::Show {
                steps: vec![],
                json: false
            }
        );
    }

    #[test]
    fn after_two_hyphens_every_word_is_a_step() {
        assert_eq!(
            asked(&words(&["show", "--json", "--", "-5 dB", "--json"])),
            Asked::Show {
                steps: words(&["-5 dB", "--json"]),
                json: true
            }
        );
    }

    #[test]
    fn what_it_doesnt_take_is_said_back() {
        assert_eq!(asked(&words(&["shwo"])), Asked::Unknown("shwo".to_string()));
        assert_eq!(
            asked(&words(&["show", "--jsno"])),
            Asked::Unknown("--jsno".to_string())
        );
    }
}
