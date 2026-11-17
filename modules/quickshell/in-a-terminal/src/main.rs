//! `launcher`, the launcher in a terminal: any place, row or drawer the
//! running shell's launcher holds, printed as text or as JSON, so every
//! integration the launcher has gets a command line with it (docs/design.md,
//! "As deep as the deepest"). It knows nothing a provider knows: it asks the
//! shell (`qs ipc call launcher show`, launcher/Showing.js), which walks the
//! same rows Return does, and only lays out what comes back.
//!
//!     launcher [show] [--drawers] [--json] [step ...] [-- step ...]
//!
//! Each step is a row's name, in any case, a hyphen for a space, or the
//! start of only one: `launcher show machine disk`. With none it shows
//! home, as a bare `launcher` does. Text is set as the launcher sets a row, in the theme's roles;
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
//! its words: `LAUNCHER_QS`, Quickshell's `qs`; `LAUNCHER_WORDS`, the
//! launcher's words.json, whose `terminal` section holds what it says of
//! its own. It inks with the terminal kit's pen, from the account's theme
//! (look.rs). Its help, its failures and its exits take the terminal kit's
//! shape (`help()`), `--json` among the kit's flags: a step that names
//! nothing exits 66; a shell that doesn't answer 69; an act asked
//! about and answered no, or with no terminal to ask on and no `--yes`,
//! or a place the launcher didn't get into, as Passwords refuses a walk
//! from outside, or an act the shell refused as it ran it, as a yes to a
//! secret ask without Joel's own key press, 77; an act whose daemon had
//! no answer to give 69 too; an act the shell failed at 1; an answer it
//! doesn't know, never done, 70.

mod asking;
mod layout;
mod look;
mod names;
mod words;

use std::env;
use std::io::{self, BufRead, IsTerminal, Write};
use std::process::ExitCode;

use serde_json::Value;
use terminal_kit::{Exit, Failure, Help};

use crate::asking::Answer;
use crate::layout::Room;
use crate::look::Look;
use crate::words::Words;

/// What a command line asked for.
#[derive(Debug, PartialEq, Eq)]
enum Asked {
    /// A place or a row, by its steps, as text or JSON, and with every
    /// row's drawer or only its own.
    Show { steps: Vec<String>, drawers: bool },
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
    /// `completion` with no shell named.
    NoShell,
    /// Something it doesn't take, said back.
    Unknown(String),
}

/// The flag `show` takes of its own, anywhere among its steps; `--json`
/// is the kit's.
const SHOW_FLAGS: [&str; 1] = ["--drawers"];

/// What `launcher --help` says, and each command's own.
fn help() -> Help {
    let step = "a row's name, a hyphen for a space, or its start";
    Help::new("launcher", "the launcher's places, in a terminal")
        .example("launcher", "home, as the launcher opens on it")
        .example(
            "launcher show machine disk",
            "the disk's row and what it holds",
        )
        .example(
            "launcher do programs kitty",
            "what Return does on kitty's row",
        )
        .subcommand(
            Help::new("show", "a place or a row, as the launcher shows it")
                .example(
                    "launcher show machine disk",
                    "the disk's row and what it holds",
                )
                .example("launcher show --drawers", "home, with every row's drawer")
                .example(
                    "launcher show --json network",
                    "what the shell says, as JSON",
                )
                .argument("<step>...", step)
                .option("--drawers", "every row's drawer, after the rows")
                .exit(Exit::NotFound, "a step named nothing")
                .exit(Exit::Unavailable, "the shell isn't answering"),
        )
        .subcommand(
            Help::new("do", "what a row does, or an act a place offers")
                .example(
                    "launcher do programs kitty",
                    "what Return does on kitty's row",
                )
                .example(
                    "launcher do machine power reboot --yes",
                    "unasked, with no one to answer",
                )
                .argument("<step>...", step)
                .option("--yes", "does an act that asks first, unasked")
                .exit(Exit::Failed, "the shell failed at it")
                .exit(Exit::NotFound, "a step named nothing")
                .exit(
                    Exit::Unavailable,
                    "the shell isn't answering, or the act's daemon had no answer",
                )
                .exit(
                    Exit::Bug,
                    "the shell asked again once told yes, or answered oddly",
                )
                .exit(
                    Exit::Refused,
                    "answered no, no one to ask, or the shell refused it",
                ),
        )
        .subcommand(
            Help::new("completion", "writes a shell's completion")
                .example("launcher completion zsh", "offers each place's rows")
                .argument("<shell>", "zsh, the one it writes")
                .exit(Exit::Done, "it always writes it"),
        )
        .subcommand(
            Help::new("names", "the rows a place holds, for completion")
                .example("launcher names machine", "Machine's rows, one a line")
                .argument("<step>...", step)
                .exit(Exit::NotFound, "a step named nothing")
                .exit(Exit::Unavailable, "the shell isn't answering")
                .unlisted(),
        )
        .exit(Exit::NotFound, "a step named nothing")
        .exit(Exit::Unavailable, "the shell isn't answering")
}

/// Reads the arguments after the program's name, the kit's flags taken.
fn asked(arguments: &[String]) -> Asked {
    let rest = arguments.get(1..).unwrap_or_default();
    match arguments.first().map(String::as_str) {
        // Bare, it shows what's true now, home, as docs/commands.md says.
        None => Asked::Show {
            steps: Vec::new(),
            drawers: false,
        },
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
        Some("completion") => match rest {
            [shell] if shell == "zsh" => Asked::Completion,
            [shell, ..] => Asked::Unknown(shell.clone()),
            [] => Asked::NoShell,
        },
        Some(other) => Asked::Unknown(other.to_string()),
    }
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let words = Words::read(env::var_os("LAUNCHER_WORDS").as_deref());
        let look = Look::new(
            context.out.clone(),
            terminal_kit::ink::theme(&context.environment).as_ref(),
        );
        run(
            asked(&context.args),
            context.format().is_json(),
            &words,
            &look,
        )
    })
}

/// Asked wrongly, in the launcher's words, pointing at the help.
fn misused(said: String) -> Failure {
    Failure::new(Exit::Usage, said).try_this("launcher --help")
}

/// Does what was asked: shows, as JSON where `json` says, or does.
fn run(asked: Asked, json: bool, words: &Words, look: &Look) -> Result<(), Failure> {
    match asked {
        Asked::Unknown(what) => Err(misused(words.unknown(&what))),
        Asked::NoShell => Err(misused(words.no_shell())),
        Asked::Completion => {
            print!("{}", names::completion(words));
            Ok(())
        }
        Asked::Do {
            steps,
            words: said,
            yes,
        } => {
            if steps.is_empty() {
                return Err(misused(words.nothing_to_do()));
            }
            if steps.iter().any(|one| one.trim().is_empty()) {
                return Err(Failure::new(Exit::Usage, words.blank_step()));
            }
            done(&qs(), &steps, &said, yes, words)
        }
        // Completion wants only names, so it doesn't wait for figures.
        Asked::Names { steps } => {
            let shown = ask(&steps, Ask::Names, words)?;
            print!("{}", names::for_zsh(&shown));
            Ok(())
        }
        Asked::Show { steps, drawers } => {
            let shown = ask(&steps, Ask::Settled { drawers }, words)?;
            let out = if json {
                format!(
                    "{}\n",
                    serde_json::to_string_pretty(&shown).unwrap_or_default()
                )
            } else {
                layout::shown(&shown, look, Room::of_stdout(), words, drawers)
            };
            // A pipe closed early, as `| head` does, isn't a failure.
            let _ = io::stdout().write_all(out.as_bytes());
            Ok(())
        }
    }
}

/// Does what the steps name: asked once unsure, and where the shell asks
/// first, its question put on the terminal, y asking again, sure; with
/// nothing to answer on, refused but for `--yes`, which was sure from the
/// start. Once asked, it's the row asked about that's done, by the
/// steady ids the shell gave with its question, so rows moving between
/// the question and the y can't put another in its place.
fn done(qs: &str, steps: &[String], said: &str, yes: bool, words: &Words) -> Result<(), Failure> {
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
        let answer = heard(asking::doing(qs, steps, &path, said, sure), words)?;
        let text = |key: &str| answer.get(key).and_then(Value::as_str).map(str::to_string);
        if let Some(question) = text("asks") {
            if sure {
                // The shell asking again once told it's sure is its own
                // failure, never a second ask.
                return Err(Failure::new(Exit::Bug, question));
            }
            if !(io::stdin().is_terminal() && io::stderr().is_terminal()) {
                return Err(Failure::new(Exit::Refused, words.unsure(&question)));
            }
            if !answered_yes(
                &words.asks(&question),
                &mut io::stdin().lock(),
                &mut io::stderr(),
            ) {
                return Err(Failure::new(Exit::Refused, words.answered_no(&question)));
            }
            path = asking::strings(answer.get("path"));
            sure = true;
            continue;
        }
        if answer.get("nothing").and_then(Value::as_bool) == Some(true) {
            return Err(misused(words.nothing_to_do()));
        }
        if let Some(ask) = text("takes") {
            let names = asking::strings(answer.get("in"));
            let name = names.last().map_or("", String::as_str);
            return Err(Failure::new(Exit::Usage, words.takes(name, &ask)));
        }
        if answer.get("unopened").and_then(Value::as_bool) == Some(true) {
            let names = asking::strings(answer.get("in"));
            let name = names.last().map_or("", String::as_str);
            return Err(Failure::new(Exit::Refused, words.unopened(name)));
        }
        // An act the shell refused as it ran it, as a yes to a secret ask
        // it withholds, refused again however often it's asked; or one whose
        // daemon had no answer to give, which an agent may ask again. Each
        // names the whole path walked, since the act is often a verb, as
        // Yes, said alone (5GLTPH, 2026-11-16).
        let walked = || asking::strings(answer.get("in")).join(layout::CRUMB);
        if let Some(why) = text("refused") {
            return Err(Failure::new(Exit::Refused, words.refused(&walked(), &why)));
        }
        if let Some(why) = text("unavailable") {
            let said = words.unanswered(&walked(), &why);
            return Err(Failure::new(Exit::Unavailable, said));
        }
        if answer.get("failed").and_then(Value::as_bool) == Some(true) {
            let name = steps.last().map_or("", String::as_str);
            return Err(Failure::new(Exit::Failed, words.failed(name)));
        }
        // Only an answer saying done is done: one this program doesn't know,
        // as a refusal the shell learnt to give before this did, exited 0
        // as done, which is how a withheld walk was said done.
        if answer.get("done").and_then(Value::as_bool) == Some(true) {
            return Ok(());
        }
        return Err(Failure::new(Exit::Bug, words.strange()).because(answer.to_string()));
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

/// Whether a line typed back says yes: y alone, so neither " Y " nor
/// "yes" does it.
fn is_yes(line: &str) -> bool {
    let typed = line.strip_suffix('\n').unwrap_or(line);
    typed.strip_suffix('\r').unwrap_or(typed) == "y"
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
fn ask(steps: &[String], how: Ask, words: &Words) -> Result<Value, Failure> {
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
fn heard(answer: Result<Answer, String>, words: &Words) -> Result<Value, Failure> {
    match answer {
        Ok(Answer::Shown(shown)) => Ok(shown),
        Ok(Answer::Missing {
            step, near, within, ..
        }) => Err(Failure::new(
            Exit::NotFound,
            words.missing(&step, &within, &near),
        )),
        Err(why) => {
            let failure = Failure::new(Exit::Unavailable, words.silent());
            Err(if why.is_empty() {
                failure
            } else {
                failure.because(why)
            })
        }
    }
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
            )
            .map_err(|failure| failure.exit),
            Err(Exit::Failed)
        );
    }

    #[test]
    fn an_act_the_shell_withheld_exits_refused_saying_why() {
        let said = Words::from_value(&serde_json::json!({"terminal": {
            "refused": "{name} not done \u{b7} {why}"}}));
        // Typed as an agent types them, so the path said is the shell's.
        let failure = done(
            &asking::stand_in::qs("refused"),
            &words(&["asks", "kil", "y"]),
            "",
            true,
            &said,
        )
        .expect_err("a withheld act isn't done");
        assert_eq!(failure.exit, Exit::Refused);
        assert_eq!(
            failure.what,
            "Asks \u{203a} Kiln \u{203a} Yes not done \u{b7} not without your own key press"
        );
    }

    #[test]
    fn an_act_whose_daemon_had_no_answer_exits_unavailable_saying_ask_again() {
        let said = Words::from_value(&serde_json::json!({"terminal": {
            "refused": "{name} not done \u{b7} {why}",
            "unanswered": "{name} not done \u{b7} {why} \u{b7} ask again"}}));
        let failure = done(
            &asking::stand_in::qs("unavailable"),
            &words(&["net", "whim", "join"]),
            "",
            true,
            &said,
        )
        .expect_err("an act nothing answered isn't done");
        assert_eq!(failure.exit, Exit::Unavailable);
        assert_eq!(
            failure.what,
            "Network \u{203a} Whimbrel \u{203a} Join not done \u{b7} no answer \u{b7} ask again"
        );
    }

    #[test]
    fn only_an_answer_saying_done_is_done() {
        let said = Words::from_value(&serde_json::json!({"terminal": {
            "strange": "an answer launcher doesn't know"}}));
        assert_eq!(
            done(
                &asking::stand_in::qs("done"),
                &words(&["programs", "godwit"]),
                "",
                true,
                &said
            )
            .map_err(|failure| failure.exit),
            Ok(())
        );
        let failure = done(
            &asking::stand_in::qs("strange"),
            &words(&["asks"]),
            "",
            true,
            &said,
        )
        .expect_err("an answer it doesn't know is never done");
        assert_eq!(failure.exit, Exit::Bug);
        assert_eq!(failure.what, "an answer launcher doesn't know");
        assert!(
            failure.why.is_some_and(|why| why.contains("withheld")),
            "the answer itself should be said, for whoever reads why"
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
            )
            .map_err(|failure| failure.exit),
            Err(Exit::Refused)
        );
        assert_eq!(
            said.unopened("Passwords"),
            "Passwords won't open from a terminal"
        );
    }

    #[test]
    fn nothing_asked_shows_home() {
        assert_eq!(
            asked(&[]),
            Asked::Show {
                steps: vec![],
                drawers: false,
            }
        );
    }

    #[test]
    fn show_takes_its_steps_and_flags_anywhere() {
        assert_eq!(
            asked(&words(&["show", "machine", "--drawers", "disk"])),
            Asked::Show {
                steps: words(&["machine", "disk"]),
                drawers: true,
            }
        );
        assert_eq!(
            asked(&words(&["show", "--drawers", "machine"])),
            Asked::Show {
                steps: words(&["machine"]),
                drawers: true,
            }
        );
        assert_eq!(
            asked(&words(&["show"])),
            Asked::Show {
                steps: vec![],
                drawers: false,
            }
        );
    }

    #[test]
    fn after_two_hyphens_every_word_is_a_step() {
        assert_eq!(
            asked(&words(&["show", "--drawers", "--", "-5 dB", "--drawers"])),
            Asked::Show {
                steps: words(&["-5 dB", "--drawers"]),
                drawers: true,
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
            Asked::Unknown("bash".to_string())
        );
        assert_eq!(asked(&words(&["completion"])), Asked::NoShell);
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
