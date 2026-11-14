//! `launcher`, the launcher in a terminal: any place, row or drawer the
//! running shell's launcher holds, printed as text or as JSON, so every
//! integration the launcher has gets a command line with it (docs/design.md,
//! "As deep as the deepest"). It knows nothing a provider knows: it asks the
//! shell (`qs ipc call launcher show`, launcher/Showing.js), which walks the
//! same rows Return does, and only lays out what comes back.
//!
//!     launcher show [--drawers] [--json] [step ...] [-- step ...]
//!
//! Each step is a row's name, in any case, a hyphen for a space, or the
//! start of only one: `launcher show machine disk`. With none it shows
//! home. Text is set as the launcher sets a row, in the theme's roles;
//! `--drawers` asks the shell for each row's drawer too (`launcher
//! drawers` over its IPC, since it builds them only then) and sets them
//! after the rows, and `--json` prints the shell's answer as it came.
//!
//!     launcher do step ... [--yes] [-- words]
//!
//! `do` does what a row's Return does, or any act a place offers among
//! its rows, as the launcher's `:` offers it (Launcher.qml's
//! doneFromTerminal). One the launcher would ask about first, or that
//! ends something, asks its question on the terminal, y doing it; with
//! no terminal to answer on, it's done only with `--yes`. The y does the
//! row asked about, by its steady ids, never another its names now find.
//! An act that asks for words takes them after `--`. A blank step names
//! no row, so it's refused. It prints nothing once it's done.
//!
//! For the shell: `launcher completion zsh` writes zsh's completion, and
//! `launcher names [step ...]` the rows a place holds as it offers them.
//!
//! Where it's installed (../home.nix) says what it asks, how it inks and
//! its words: `LAUNCHER_QS`, Quickshell's `qs`; `LAUNCHER_LOOK`, the
//! theme's roles as escapes and its marks' stand-ins; `LAUNCHER_WORDS`, the
//! launcher's words.json, whose `terminal` section holds what it says of
//! its own. A step that names nothing, or an act not done since the
//! answer was no, exits 1; a shell that doesn't answer exits 2; an act
//! with no terminal to ask on and no `--yes` exits 3; a place the launcher
//! didn't get into, as Passwords refuses a walk from outside, exits 4; each
//! said on stderr but the no, which the terminal already shows.

mod asking;
mod layout;
mod look;
mod names;
mod words;

use std::env;
use std::io::{self, BufRead, IsTerminal, Write};
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
    /// What a row does, or an act a place offers, by its steps, with the
    /// words it asks for and whether it's done unasked.
    Do {
        steps: Vec<String>,
        words: String,
        yes: bool,
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
        Some("do") => {
            let (before, after) = match rest.iter().position(|one| one == "--") {
                Some(at) => (&rest[..at], &rest[at + 1..]),
                None => (rest, &rest[rest.len()..]),
            };
            if let Some(flag) = before
                .iter()
                .find(|one| one.starts_with('-') && *one != "--yes")
            {
                return Asked::Unknown(flag.clone());
            }
            Asked::Do {
                steps: before
                    .iter()
                    .filter(|one| *one != "--yes")
                    .cloned()
                    .collect(),
                words: after.join(" "),
                yes: before.iter().any(|one| one == "--yes"),
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
const NOT_SURE: u8 = 3;
const WONT_OPEN: u8 = 4;
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
            print!("{}", names::completion(&words));
            ExitCode::SUCCESS
        }
        Asked::Do {
            steps,
            words: said,
            yes,
        } => {
            if steps.is_empty() {
                fail(&errors(), &words.nothing_to_do());
                eprint!("{}", words.usage());
                return ExitCode::from(MISUSED);
            }
            if steps.iter().any(|one| one.trim().is_empty()) {
                fail(&errors(), &words.blank_step());
                return ExitCode::from(MISUSED);
            }
            done(&qs(), &steps, &said, yes, &words)
        }
        // Completion wants only names, so it doesn't wait for figures.
        Asked::Names { steps } => match ask(&steps, Ask::Names, &words) {
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
        } => match ask(&steps, Ask::Settled { drawers }, &words) {
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

/// Does what the steps name: asked once unsure, and where the shell asks
/// first, its question put on the terminal, y asking again, sure; with
/// nothing to answer on, refused but for `--yes`, which was sure from the
/// start. Once asked, it's the row asked about that's done, by the
/// steady ids the shell gave with its question, so rows moving between
/// the question and the y can't put another in its place.
fn done(qs: &str, steps: &[String], said: &str, yes: bool, words: &Words) -> ExitCode {
    // A row its provider's daemon makes, as Power's Charge Limit, is there
    // only once the provider is seen, and with the launcher closed only a
    // terminal's show sees it (Launcher.qml's terminalSees). So the place
    // the act is in is shown first, settled as `show` is, and the act
    // walks rows already read.
    if let Some((_, place)) = steps.split_last()
        && !place.is_empty()
    {
        let _ = asking::settled(qs, place, false);
    }
    let mut sure = yes;
    let mut path = Vec::new();
    loop {
        let answer = match heard(asking::doing(qs, steps, &path, said, sure), words) {
            Ok(answer) => answer,
            Err(code) => return code,
        };
        let text = |key: &str| answer.get(key).and_then(Value::as_str).map(str::to_string);
        if let Some(question) = text("asks") {
            if sure {
                // The shell asking again once told it's sure is its own
                // failure, never a second ask.
                fail(&errors(), &question);
                return ExitCode::from(SHELL_SILENT);
            }
            if !(io::stdin().is_terminal() && io::stderr().is_terminal()) {
                fail(&errors(), &words.unsure(&question));
                return ExitCode::from(NOT_SURE);
            }
            if !answered_yes(&question, &mut io::stdin().lock(), &mut io::stderr()) {
                return ExitCode::from(NOTHING_CALLED);
            }
            path = asking::strings(answer.get("path"));
            sure = true;
            continue;
        }
        if answer.get("nothing").and_then(Value::as_bool) == Some(true) {
            fail(&errors(), &words.nothing_to_do());
            return ExitCode::from(MISUSED);
        }
        if let Some(ask) = text("takes") {
            let names = asking::strings(answer.get("in"));
            let name = names.last().map_or("", String::as_str);
            fail(&errors(), &words.takes(name, &ask));
            return ExitCode::from(MISUSED);
        }
        if answer.get("unopened").and_then(Value::as_bool) == Some(true) {
            let names = asking::strings(answer.get("in"));
            let name = names.last().map_or("", String::as_str);
            fail(&errors(), &words.unopened(name));
            return ExitCode::from(WONT_OPEN);
        }
        if answer.get("failed").and_then(Value::as_bool) == Some(true) {
            let name = steps.last().map_or("", String::as_str);
            fail(&errors(), &words.failed(name));
            return ExitCode::from(SHELL_SILENT);
        }
        return ExitCode::SUCCESS;
    }
}

/// Puts the launcher's question where it's answered, as its line does,
/// and whether the line read back was y; nothing read, as at the end of
/// what's given, is no.
fn answered_yes(question: &str, input: &mut impl BufRead, out: &mut impl Write) -> bool {
    let _ = write!(out, "{question} ");
    let _ = out.flush();
    let mut line = String::new();
    match input.read_line(&mut line) {
        Ok(0) | Err(_) => false,
        Ok(_) => is_yes(&line),
    }
}

/// Whether a line typed back says yes: y alone, as the launcher takes it
/// (Confirm.js's answer), so neither " Y " nor "yes" does it.
fn is_yes(line: &str) -> bool {
    let typed = line.strip_suffix('\n').unwrap_or(line);
    typed.strip_suffix('\r').unwrap_or(typed) == "y"
}

/// How what goes to stderr is inked.
fn errors() -> Look {
    Look::read(env::var_os("LAUNCHER_LOOK").as_deref(), Ink::for_stderr())
}

/// How a place is asked: for its names alone, or settled, with every
/// row's drawer where `drawers` says.
#[derive(Clone, Copy)]
enum Ask {
    Names,
    Settled { drawers: bool },
}

/// What the shell shows at the steps, or how to exit, said on stderr,
/// where a step names nothing or the shell doesn't answer.
fn ask(steps: &[String], how: Ask, words: &Words) -> Result<Value, ExitCode> {
    let qs = qs();
    let answer = match how {
        Ask::Names => asking::names(&qs, steps),
        Ask::Settled { drawers } => asking::settled(&qs, steps, drawers),
    };
    heard(answer, words)
}

/// The qs that asks the shell: Quickshell's, or a stand-in's in a check.
fn qs() -> String {
    env::var("LAUNCHER_QS").unwrap_or_else(|_| "qs".to_string())
}

/// What the shell answered, or how to exit, said on stderr, where a step
/// names nothing or the shell doesn't answer.
fn heard(answer: Result<Answer, String>, words: &Words) -> Result<Value, ExitCode> {
    match answer {
        Ok(Answer::Shown(shown)) => Ok(shown),
        Ok(Answer::Missing {
            step, near, within, ..
        }) => {
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
    fn a_do_the_shell_failed_at_exits_as_the_shell_silent() {
        let said = Words::from_value(&serde_json::json!({"terminal": {
            "failed": "{name} not done \u{b7} the shell failed"}}));
        assert_eq!(
            done(
                &asking::stand_in::qs("failed"),
                &words(&["Reboot"]),
                "",
                true,
                &said
            ),
            ExitCode::from(SHELL_SILENT)
        );
    }

    #[test]
    fn a_place_that_wouldnt_open_exits_apart_from_one_done() {
        let said = Words::from_value(&serde_json::json!({"terminal": {
            "unopened": "{name} won't open from a terminal"}}));
        assert_eq!(
            done(
                &asking::stand_in::qs("unopened"),
                &words(&["Passwords"]),
                "",
                false,
                &said
            ),
            ExitCode::from(WONT_OPEN)
        );
        assert_eq!(
            said.unopened("Passwords"),
            "Passwords won't open from a terminal"
        );
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
    fn do_takes_its_steps_its_yes_and_words_after_the_dashes() {
        assert_eq!(
            asked(&words(&["do", "machine", "--yes", "power", "reboot"])),
            Asked::Do {
                steps: words(&["machine", "power", "reboot"]),
                words: String::new(),
                yes: true,
            }
        );
        assert_eq!(
            asked(&words(&["do", "people", "joel", "--", "on", "--my", "way"])),
            Asked::Do {
                steps: words(&["people", "joel"]),
                words: "on --my way".to_string(),
                yes: false,
            }
        );
        assert_eq!(
            asked(&words(&["do", "machine", "-y"])),
            Asked::Unknown("-y".to_string())
        );
    }

    #[test]
    fn only_y_is_yes() {
        assert!(is_yes("y\n"));
        assert!(is_yes("y\r\n"));
        assert!(is_yes("y"));
        assert!(!is_yes(" Y \n"));
        assert!(!is_yes("Y\n"));
        assert!(!is_yes(" y\n"));
        assert!(!is_yes("yes please\n"));
        assert!(!is_yes("n\n"));
        assert!(!is_yes(""));
    }

    /// What answering the question with this gives, and what was put before it.
    fn answering(typed: &str) -> (bool, String) {
        let mut out = Vec::new();
        let yes = answered_yes(
            "Close Kestrel Notes? (y/n)",
            &mut io::Cursor::new(typed),
            &mut out,
        );
        (yes, String::from_utf8(out).unwrap_or_default())
    }

    #[test]
    fn the_question_is_put_and_only_a_y_does_it() {
        assert_eq!(
            answering("y\n"),
            (true, "Close Kestrel Notes? (y/n) ".to_string())
        );
        assert!(!answering("n\n").0);
        assert!(!answering(" Y \n").0);
        // The end of what's given, as </dev/null is, before any answer.
        assert!(!answering("").0);
    }
}
