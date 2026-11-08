//! `launcher`, the launcher in a terminal: any place, row or drawer the
//! running shell's launcher holds, printed as text or as JSON, so every
//! integration the launcher has gets a command line with it (docs/design.md,
//! "As deep as the deepest"). It knows nothing a provider knows: it asks the
//! shell (`qs ipc call launcher show`, launcher/Showing.js), which walks the
//! same rows Return does, and only lays out what comes back.
//!
//!     launcher show [step ...] [--drawers] [--json]
//!
//! Each step is a row's name, in any case, a hyphen for a space, or the
//! start of only one: `launcher show machine disk`. With none it shows
//! home. Text is set as the launcher sets a row, in the theme's roles;
//! `--drawers` sets each row's drawer after the rows, and `--json` prints
//! the shell's answer as it came, every drawer in it.
//!
//! For the shell: `launcher completion zsh` writes zsh's completion, and
//! `launcher names [step ...]` the rows a place holds as it offers them.
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
mod names;
mod words;

use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

use serde_json::Value;
use unicode_width::UnicodeWidthStr;

use crate::asking::Answer;
use crate::layout::Room;
use crate::look::{Ink, Look};
use crate::words::Words;

/// What a command line asked for.
#[derive(Debug, PartialEq, Eq)]
enum Asked {
    /// A place or a row, by its steps, as text or JSON, and with every
    /// row's drawer or only its own.
    Show {
        steps: Vec<String>,
        json: bool,
        drawers: bool,
    },
    /// The rows a place holds, as zsh's completion offers them.
    Names { steps: Vec<String> },
    /// zsh's completion itself.
    Completion,
    /// How it's used.
    Help,
    /// Something it doesn't take, said back.
    Unknown(String),
}

/// The flags `show` takes, anywhere among its steps.
const SHOW_FLAGS: [&str; 2] = ["--json", "--drawers"];

/// Reads the arguments after the program's name.
fn asked(arguments: &[String]) -> Asked {
    let rest = arguments.get(1..).unwrap_or_default();
    match arguments.first().map(String::as_str) {
        None | Some("-h" | "--help" | "help") => Asked::Help,
        Some("show") => {
            // After `--`, every argument is a step, so one starting with a
            // hyphen can be named too.
            let ends = rest.iter().position(|one| one == "--");
            let (flagged, named) = match ends {
                Some(at) => (&rest[..at], &rest[at + 1..]),
                None => (rest, &rest[rest.len()..]),
            };
            if let Some(flag) = flagged
                .iter()
                .find(|one| one.starts_with('-') && !SHOW_FLAGS.contains(&one.as_str()))
            {
                return Asked::Unknown(flag.clone());
            }
            Asked::Show {
                steps: flagged
                    .iter()
                    .filter(|one| !SHOW_FLAGS.contains(&one.as_str()))
                    .chain(named)
                    .cloned()
                    .collect(),
                json: flagged.iter().any(|one| one == "--json"),
                drawers: flagged.iter().any(|one| one == "--drawers"),
            }
        }
        Some("names") => Asked::Names {
            steps: rest.to_vec(),
        },
        Some("completion") if rest == ["zsh"] => Asked::Completion,
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
            fail(&errors(), &words.unknown(&what));
            eprint!("{}", words.usage());
            ExitCode::from(MISUSED)
        }
        Asked::Completion => {
            print!("{}", include_str!("completion.zsh"));
            ExitCode::SUCCESS
        }
        Asked::Names { steps } => match ask(&steps, &words) {
            Ok(shown) => {
                print!("{}", names::for_zsh(&shown));
                ExitCode::SUCCESS
            }
            Err(code) => code,
        },
        Asked::Show {
            steps,
            json,
            drawers,
        } => match ask(&steps, &words) {
            Ok(shown) => {
                let out = if json {
                    format!(
                        "{}\n",
                        serde_json::to_string_pretty(&shown).unwrap_or_default()
                    )
                } else {
                    let look =
                        Look::read(env::var_os("LAUNCHER_LOOK").as_deref(), Ink::for_stdout());
                    layout::shown(&shown, &look, Room::of_stdout(), &words, drawers)
                };
                // A pipe closed early, as `| head` does, isn't a failure.
                let _ = io::stdout().write_all(out.as_bytes());
                ExitCode::SUCCESS
            }
            Err(code) => code,
        },
    }
}

/// How what goes to stderr is inked.
fn errors() -> Look {
    Look::read(env::var_os("LAUNCHER_LOOK").as_deref(), Ink::for_stderr())
}

/// What the shell shows at the steps, or how to exit, said on stderr,
/// where a step names nothing or the shell doesn't answer.
fn ask(steps: &[String], words: &Words) -> Result<Value, ExitCode> {
    let qs = env::var("LAUNCHER_QS").unwrap_or_else(|_| "qs".to_string());
    match asking::show(&qs, steps) {
        Ok(Answer::Shown(shown)) => Ok(shown),
        Ok(Answer::Missing { step, near, within }) => {
            fail(&errors(), &words.missing(&step, &within, &near));
            Err(ExitCode::from(NOTHING_CALLED))
        }
        Err(why) => {
            fail(&errors(), &words.silent());
            if !why.is_empty() {
                eprintln!("{why}");
            }
            Err(ExitCode::from(SHELL_SILENT))
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
    fn show_takes_its_steps_and_flags_anywhere() {
        assert_eq!(
            asked(&words(&["show", "machine", "--json", "disk"])),
            Asked::Show {
                steps: words(&["machine", "disk"]),
                json: true,
                drawers: false,
            }
        );
        assert_eq!(
            asked(&words(&["show", "--drawers", "machine"])),
            Asked::Show {
                steps: words(&["machine"]),
                json: false,
                drawers: true,
            }
        );
        assert_eq!(
            asked(&words(&["show"])),
            Asked::Show {
                steps: vec![],
                json: false,
                drawers: false,
            }
        );
    }

    #[test]
    fn after_two_hyphens_every_word_is_a_step() {
        assert_eq!(
            asked(&words(&["show", "--json", "--", "-5 dB", "--json"])),
            Asked::Show {
                steps: words(&["-5 dB", "--json"]),
                json: true,
                drawers: false,
            }
        );
    }

    #[test]
    fn the_shell_is_given_names_and_its_completion() {
        assert_eq!(
            asked(&words(&["names", "machine"])),
            Asked::Names {
                steps: words(&["machine"])
            }
        );
        assert_eq!(asked(&words(&["completion", "zsh"])), Asked::Completion);
        assert_eq!(
            asked(&words(&["completion", "bash"])),
            Asked::Unknown("completion".to_string())
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

    #[test]
    fn the_completion_is_zshs_for_launcher() {
        assert!(include_str!("completion.zsh").starts_with("#compdef launcher\n"));
    }
}
