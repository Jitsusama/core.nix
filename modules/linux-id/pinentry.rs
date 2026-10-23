//! A pinentry that asks through the shell. Each question a program puts to
//! it over pinentry's protocol goes to `SSH_ASKPASS`, the shell's prompt, the
//! way askpass is asked: a yes for CONFIRM and MESSAGE, a secret for GETPIN.
//! linux-id asks it whether Joel is there before its key signs.
//!
//! What's asked is the description the program set, else its prompt, else
//! its title, after an error it set for this PIN. A refusal answers as
//! pinentry's cancel does, and an askpass that can't be run as pinentry
//! missing.
//!
//! It needs only the standard library.

use std::env;
use std::ffi::OsStr;
use std::io::{self, BufRead, Write};
use std::process::{Command, ExitCode, Stdio};

const CANCELLED: &str = "ERR 83886179 Operation cancelled <Pinentry>";
const UNREACHABLE: &str = "ERR 83886165 No pinentry <Pinentry>";
const UNKNOWN: &str = "ERR 83886355 Unknown IPC command <Pinentry>";
const UNASKABLE: &str = "ERR 83886360 IPC parameter error <Pinentry>";

/// What a question wants: a yes, or a secret typed.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Yes,
    Secret,
}

/// How a question was answered.
#[derive(Debug, PartialEq)]
enum Answer {
    /// What was typed, or nothing for a yes.
    Given(String),
    Refused,
    /// No askpass could be run to ask.
    Unreachable,
}

/// What the program has said so far about the next question.
#[derive(Default)]
struct Session {
    description: String,
    prompt: String,
    title: String,
    error: String,
}

/// The lines a command is answered with, and whether that ends it.
#[derive(Debug, PartialEq)]
struct Reply {
    lines: Vec<String>,
    done: bool,
}

impl Reply {
    fn of(lines: &[&str]) -> Self {
        Reply {
            lines: lines.iter().map(|line| (*line).to_owned()).collect(),
            done: false,
        }
    }

    fn data(data: &str) -> Self {
        Reply {
            lines: vec![format!("D {}", escaped(data)), "OK".to_owned()],
            done: false,
        }
    }
}

impl Session {
    /// The words askpass is given: what the program set, most specific first.
    fn asking(&self) -> String {
        let words = [&self.description, &self.prompt, &self.title]
            .into_iter()
            .find(|words| !words.is_empty())
            .map_or("", String::as_str);
        if self.error.is_empty() {
            words.to_owned()
        } else {
            format!("{}\n{words}", self.error)
        }
    }

    /// Answers one line of the protocol, asking through `ask` where it asks.
    fn answer(&mut self, line: &str, ask: &mut impl FnMut(Kind, &str) -> Answer) -> Reply {
        if line.is_empty() || line.starts_with('#') {
            return Reply::of(&[]);
        }
        let (verb, argument) = line.split_once(' ').unwrap_or((line, ""));
        let argument = decoded(argument);
        match verb.to_ascii_uppercase().as_str() {
            "SETDESC" => self.description = argument,
            "SETPROMPT" => self.prompt = argument,
            "SETTITLE" => self.title = argument,
            "SETERROR" => self.error = argument,
            "RESET" => *self = Session::default(),
            verb if verb.starts_with("SET") || verb == "OPTION" || verb == "NOP" => {}
            "GETINFO" => {
                return match argument.as_str() {
                    "flavor" => Reply::data("askpass"),
                    "version" => Reply::data("1"),
                    "pid" => Reply::data(&std::process::id().to_string()),
                    "ttyinfo" => Reply::data("- - -"),
                    _ => Reply::of(&[UNASKABLE]),
                };
            }
            "CONFIRM" | "MESSAGE" => {
                // One button only tells, so whatever's pressed carries on.
                let telling = verb.eq_ignore_ascii_case("MESSAGE")
                    || argument
                        .split_whitespace()
                        .any(|word| word == "--one-button");
                return match ask(Kind::Yes, &self.asking()) {
                    Answer::Unreachable => Reply::of(&[UNREACHABLE]),
                    Answer::Refused if !telling => Reply::of(&[CANCELLED]),
                    _ => Reply::of(&["OK"]),
                };
            }
            "GETPIN" => {
                let asked = ask(Kind::Secret, &self.asking());
                self.error.clear();
                return match asked {
                    Answer::Given(secret) => Reply::data(&secret),
                    Answer::Refused => Reply::of(&[CANCELLED]),
                    Answer::Unreachable => Reply::of(&[UNREACHABLE]),
                };
            }
            "BYE" => {
                return Reply {
                    lines: vec!["OK closing connection".to_owned()],
                    done: true,
                };
            }
            _ => return Reply::of(&[UNKNOWN]),
        }
        Reply::of(&["OK"])
    }
}

/// An argument with the protocol's %XX escapes undone.
fn decoded(argument: &str) -> String {
    let bytes = argument.as_bytes();
    let mut plain = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let byte = bytes
            .get(at + 1..at + 3)
            .filter(|_| bytes[at] == b'%')
            .and_then(|hex| u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok());
        if let Some(byte) = byte {
            plain.push(byte);
            at += 3;
        } else {
            plain.push(bytes[at]);
            at += 1;
        }
    }
    String::from_utf8_lossy(&plain).into_owned()
}

/// Data with what would end or break its line escaped.
fn escaped(data: &str) -> String {
    data.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

/// Asks through `SSH_ASKPASS`.
fn ask(kind: Kind, asking: &str) -> Answer {
    if let Some(askpass) = env::var_os("SSH_ASKPASS").filter(|askpass| !askpass.is_empty()) {
        ask_through(&askpass, kind, asking)
    } else {
        eprintln!("pinentry-askpass: SSH_ASKPASS isn't set, so there's nothing to ask");
        Answer::Unreachable
    }
}

/// Asks through an askpass, which answers a secret on its output and says
/// yes by succeeding.
fn ask_through(askpass: &OsStr, kind: Kind, asking: &str) -> Answer {
    let mut command = Command::new(askpass);
    command
        .arg(asking)
        .stdin(Stdio::null())
        .stderr(Stdio::inherit());
    match kind {
        Kind::Yes => command
            .env("SSH_ASKPASS_PROMPT", "confirm")
            .stdout(Stdio::null()),
        Kind::Secret => command
            .env_remove("SSH_ASKPASS_PROMPT")
            .stdout(Stdio::piped()),
    };
    match command.output() {
        Err(error) => {
            eprintln!(
                "pinentry-askpass: can't run {}: {error}",
                askpass.to_string_lossy()
            );
            Answer::Unreachable
        }
        Ok(output) if output.status.success() => {
            let typed = String::from_utf8_lossy(&output.stdout);
            Answer::Given(typed.strip_suffix('\n').unwrap_or(&typed).to_owned())
        }
        Ok(_) => Answer::Refused,
    }
}

fn main() -> ExitCode {
    let mut session = Session::default();
    let mut out = io::stdout().lock();
    let said = |out: &mut io::StdoutLock, lines: &[String]| -> io::Result<()> {
        for line in lines {
            writeln!(out, "{line}")?;
        }
        out.flush()
    };
    if said(&mut out, &["OK Pleased to meet you".to_owned()]).is_err() {
        return ExitCode::FAILURE;
    }
    for line in io::stdin().lock().lines() {
        let Ok(line) = line else {
            return ExitCode::FAILURE;
        };
        let reply = session.answer(line.trim_end_matches('\r'), &mut ask);
        if said(&mut out, &reply.lines).is_err() {
            return ExitCode::FAILURE;
        }
        if reply.done {
            break;
        }
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs the lines through a session, answering every question with
    /// `answer`, and gives back what it replied and what it asked.
    fn run(lines: &[&str], answer: fn(Kind) -> Answer) -> (Vec<String>, Vec<(Kind, String)>) {
        let mut session = Session::default();
        let mut asked = Vec::new();
        let mut ask = |kind: Kind, words: &str| {
            let given = answer(kind);
            asked.push((kind, words.to_owned()));
            given
        };
        let mut replies = Vec::new();
        for line in lines {
            let reply = session.answer(line, &mut ask);
            replies.extend(reply.lines);
            if reply.done {
                break;
            }
        }
        (replies, asked)
    }

    fn yes(_: Kind) -> Answer {
        Answer::Given(String::new())
    }

    fn no(_: Kind) -> Answer {
        Answer::Refused
    }

    #[test]
    fn asks_linux_id_for_a_yes_in_its_own_words() {
        // As go-pinentry sends them, CONFIRM carrying the words unescaped.
        let lines = [
            "OPTION ttyname=/dev/pts/1",
            "SETDESC FIDO2 Register: example.com",
            "SETTITLE LINUX-ID",
            "SETPROMPT LINUX-ID",
            "CONFIRM FIDO2 Register: example.com",
            "BYE",
        ];
        let (replies, asked) = run(&lines, yes);
        assert_eq!(
            replies,
            ["OK", "OK", "OK", "OK", "OK", "OK closing connection"]
        );
        assert_eq!(
            asked,
            [(Kind::Yes, "FIDO2 Register: example.com".to_owned())]
        );
    }

    #[test]
    fn says_a_refusal_was_cancelled() {
        let (replies, _) = run(&["SETDESC Sign?", "CONFIRM", "GETPIN"], no);
        assert_eq!(replies, ["OK", CANCELLED, CANCELLED]);
    }

    #[test]
    fn carries_on_from_a_message_however_its_answered() {
        let (replies, asked) = run(&["SETDESC Done.", "CONFIRM --one-button", "MESSAGE"], no);
        assert_eq!(replies, ["OK", "OK", "OK"]);
        assert_eq!(asked.len(), 2);
    }

    #[test]
    fn gives_a_typed_pin_escaped() {
        let (replies, asked) = run(&["SETDESC Unlock%0Athe key", "GETPIN"], |_| {
            Answer::Given("50%\nof it".to_owned())
        });
        assert_eq!(replies, ["OK", "D 50%25%0Aof it", "OK"]);
        assert_eq!(asked, [(Kind::Secret, "Unlock\nthe key".to_owned())]);
    }

    #[test]
    fn says_an_error_once_before_the_pin_its_about() {
        let lines = ["SETPROMPT PIN:", "SETERROR Bad PIN", "GETPIN", "GETPIN"];
        let (_, asked) = run(&lines, |_| Answer::Given("1234".to_owned()));
        let words: Vec<_> = asked.into_iter().map(|(_, words)| words).collect();
        assert_eq!(words, ["Bad PIN\nPIN:", "PIN:"]);
    }

    #[test]
    fn asks_in_the_title_when_nothing_else_is_set() {
        let (_, asked) = run(&["SETTITLE Unlock", "CONFIRM"], yes);
        assert_eq!(asked[0].1, "Unlock");
    }

    #[test]
    fn forgets_everything_on_a_reset() {
        let (_, asked) = run(&["SETDESC Old", "RESET", "SETTITLE New", "CONFIRM"], yes);
        assert_eq!(asked[0].1, "New");
    }

    #[test]
    fn says_pinentry_is_missing_when_askpass_cant_be_run() {
        let (replies, _) = run(&["CONFIRM", "GETPIN"], |_| Answer::Unreachable);
        assert_eq!(replies, [UNREACHABLE, UNREACHABLE]);
    }

    #[test]
    fn says_what_it_is_and_refuses_what_it_doesnt_know() {
        let (replies, asked) = run(
            &[
                "getinfo flavor",
                "GETINFO ttyinfo",
                "GETINFO nothing",
                "SCD SERIALNO",
                "",
            ],
            yes,
        );
        assert_eq!(
            replies,
            ["D askpass", "OK", "D - - -", "OK", UNASKABLE, UNKNOWN]
        );
        assert!(asked.is_empty());
    }

    #[test]
    fn stops_at_bye() {
        let (replies, asked) = run(&["BYE", "CONFIRM"], yes);
        assert_eq!(replies, ["OK closing connection"]);
        assert!(asked.is_empty());
    }

    /// An askpass that writes down how it was asked beside itself, types
    /// "typed" and a newline, and refuses when asked "no".
    fn askpass(name: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let dir = env::temp_dir().join(format!("pinentry-askpass-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("askpass");
        std::fs::write(
            &script,
            "#!/bin/sh\nprintf '%s|%s' \"${SSH_ASKPASS_PROMPT-unset}\" \"$1\" > \"$0.asked\"\n\
             [ \"$1\" != no ] && echo typed\n",
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        script
    }

    fn asked_of(script: &std::path::Path) -> String {
        std::fs::read_to_string(script.with_extension("asked")).unwrap()
    }

    #[test]
    fn asks_askpass_for_a_secret_and_takes_what_it_typed() {
        let script = askpass("secret");
        let answer = ask_through(script.as_os_str(), Kind::Secret, "PIN:");
        assert_eq!(answer, Answer::Given("typed".to_owned()));
        assert_eq!(asked_of(&script), "unset|PIN:");
    }

    #[test]
    fn asks_askpass_for_a_yes_as_a_confirmation() {
        let script = askpass("yes");
        assert_eq!(
            ask_through(script.as_os_str(), Kind::Yes, "Sign?"),
            Answer::Given(String::new())
        );
        assert_eq!(asked_of(&script), "confirm|Sign?");
        assert_eq!(
            ask_through(script.as_os_str(), Kind::Yes, "no"),
            Answer::Refused
        );
        assert_eq!(
            ask_through(script.as_os_str(), Kind::Secret, "no"),
            Answer::Refused
        );
    }

    #[test]
    fn says_an_askpass_that_cant_run_is_unreachable() {
        let missing = OsStr::new("/nonexistent/askpass");
        assert_eq!(
            ask_through(missing, Kind::Yes, "Sign?"),
            Answer::Unreachable
        );
    }

    #[test]
    fn leaves_a_stray_percent_as_it_is() {
        assert_eq!(decoded("100% %zz %4"), "100% %zz %4");
        assert_eq!(decoded("%25%41"), "%A");
    }
}
