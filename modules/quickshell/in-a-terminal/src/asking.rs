//! Asking the running shell what the launcher shows, and to do what it
//! offers, through Quickshell's `qs ipc call launcher show` and `doing`, as
//! pi-ask's tool asks it for the catalogue.

use std::io::Read;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

/// How the launcher joins a path's steps (Launcher.qml's catalogueStep), a
/// letter no name holds.
pub const STEP: &str = "\u{203a}";

/// How long the shell has to answer before it's taken as silent: a walk
/// of a place takes milliseconds, and qs itself a hundred or so.
const PATIENCE: Duration = Duration::from_secs(5);

/// How often a call still running is looked at while it's waited for.
const LOOK_AGAIN: Duration = Duration::from_millis(10);

/// What the shell answered.
#[derive(Debug, PartialEq)]
pub enum Answer {
    /// What a place or a row shows, as launcher/Showing.js shapes it, or
    /// what came of doing it.
    Shown(Value),
    /// A step named nothing: the step, the names it might have meant and
    /// the names of the steps before it.
    Missing {
        step: String,
        near: Vec<String>,
        within: Vec<String>,
    },
}

/// Asks the shell through `qs` what the steps show, with each row's drawer
/// where `drawers` asks, since the shell builds them only then. An error is
/// the shell not answering, with what qs said where it said anything.
pub fn show(qs: &str, steps: &[String], drawers: bool) -> Result<Answer, String> {
    call(qs, if drawers { "drawers" } else { "show" }, &path(steps))
}

/// Asks the shell through `qs` to do what the steps name, with the words
/// an act asks for, and past its question once the terminal's sure. It
/// answers with what it asks first (`asks`), the words an act takes
/// (`takes`), or that it's done.
pub fn doing(qs: &str, steps: &[String], words: &str, sure: bool) -> Result<Answer, String> {
    call(qs, "doing", &asked(steps, words, sure))
}

/// What `doing` is given, as JSON, which is never empty as qs needs.
pub fn asked(steps: &[String], words: &str, sure: bool) -> String {
    serde_json::json!({ "steps": steps, "words": words, "sure": sure }).to_string()
}

fn call(qs: &str, function: &str, argument: &str) -> Result<Answer, String> {
    let mut child = Command::new(qs)
        // After `--`, since `show` is also qs ipc's own command, which
        // takes no argument, and qs reads the call as it otherwise.
        .args(["ipc", "call", "--", "launcher", function, argument])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    let began = Instant::now();
    // The answer and qs's own words are each read as they come, so one
    // longer than a pipe holds never stops qs writing it.
    let stdout = child.stdout.take().ok_or_else(String::new)?;
    let stderr = child.stderr.take().ok_or_else(String::new)?;
    let reading = read_apart(stdout);
    let hearing = read_apart(stderr);
    loop {
        if child
            .try_wait()
            .map_err(|error| error.to_string())?
            .is_some()
        {
            break;
        }
        if began.elapsed() > PATIENCE {
            let _ = child.kill();
            let _ = child.wait();
            return Err(String::new());
        }
        thread::sleep(LOOK_AGAIN);
    }
    let printed = reading.join().unwrap_or_default();
    let said = hearing.join().unwrap_or_default();
    answer_of(&printed).ok_or_else(|| {
        plain(if said.trim().is_empty() {
            &printed
        } else {
            &said
        })
    })
}

/// Reads all a pipe says, on a thread of its own.
fn read_apart(mut pipe: impl Read + Send + 'static) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut said = String::new();
        let _ = pipe.read_to_string(&mut said);
        said
    })
}

/// The path the shell is asked, each step after the mark that joins them:
/// qs takes no empty argument, so home is the mark alone.
pub fn path(steps: &[String]) -> String {
    if steps.is_empty() {
        return STEP.to_string();
    }
    let mut path = String::new();
    for step in steps {
        path.push_str(STEP);
        path.push_str(step);
    }
    path
}

/// What the shell's printed answer says, or None where it isn't one:
/// Quickshell's qs prints a failed call's error on stdout, in colour, and
/// still exits 0, so anything that isn't a JSON object is the shell not
/// answering.
pub fn answer_of(printed: &str) -> Option<Answer> {
    let value: Value = serde_json::from_str(printed.trim()).ok()?;
    if !value.is_object() {
        return None;
    }
    if let Some(step) = value.get("missing").and_then(Value::as_str) {
        return Some(Answer::Missing {
            step: step.to_string(),
            near: strings(value.get("near")),
            within: strings(value.get("in")),
        });
    }
    Some(Answer::Shown(value))
}

/// A JSON list's strings, or none.
pub fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|all| {
            all.iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// What qs said, its colours taken away and its lines joined.
fn plain(said: &str) -> String {
    let mut out = String::new();
    let mut escaped = false;
    for letter in said.chars() {
        if escaped {
            escaped = !letter.is_ascii_alphabetic();
        } else if letter == '\u{1b}' {
            escaped = true;
        } else {
            out.push(letter);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_is_asked_as_the_mark_alone_and_a_place_after_it() {
        assert_eq!(path(&[]), "\u{203a}");
        assert_eq!(
            path(&["machine".to_string(), "disk".to_string()]),
            "\u{203a}machine\u{203a}disk"
        );
    }

    #[test]
    fn what_to_do_is_asked_as_json() {
        let steps = ["machine".to_string(), "power".to_string()];
        assert_eq!(
            asked(&steps, "", false),
            r#"{"steps":["machine","power"],"sure":false,"words":""}"#
        );
    }

    #[test]
    fn a_place_is_shown() {
        let answer = answer_of(r#"{"in":["Machine"],"path":["category:computer"],"rows":[]}"#);
        assert!(matches!(answer, Some(Answer::Shown(_))));
    }

    #[test]
    fn a_step_naming_nothing_says_what_it_might_have_meant() {
        let answer = answer_of(r#"{"missing":"mahcine","near":["Machine"],"in":[]}"#);
        assert_eq!(
            answer,
            Some(Answer::Missing {
                step: "mahcine".to_string(),
                near: vec!["Machine".to_string()],
                within: vec![],
            })
        );
    }

    #[test]
    fn qs_saying_it_failed_is_no_answer() {
        assert_eq!(
            answer_of("\u{1b}[31mERROR\u{1b}[0m: no running instance"),
            None
        );
        assert_eq!(answer_of("[]"), None);
        assert_eq!(answer_of(""), None);
    }

    #[test]
    fn what_qs_said_is_said_plainly() {
        assert_eq!(
            plain("\u{1b}[31mERROR\u{1b}[0m:  no\n running"),
            "ERROR: no running"
        );
    }
}
