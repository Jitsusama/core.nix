//! The terminal kit: the shape every command of ours takes, for Joel in a
//! terminal and for a machine through a pipe, so learning one command
//! teaches them all (docs/commands.md). A command hands `main` its help
//! and its body; the kit takes the shared flags, answers `--help` and
//! `--version` without starting anything, gives the body a `Context`
//! holding the answer's form and a pen for each stream, and ends the
//! process with the exit its body chose: a failure said in its three
//! parts, as text or as JSON, and a panic as a bug, 70.
//!
//! It depends on `serde_json` and `unicode-width` alone, since a parser crate
//! would be most of a small command's dependencies: the command reads
//! what's left of its arguments its own way.

pub mod environment;
pub mod failure;
pub mod flags;
pub mod help;
pub mod ink;
pub mod layout;
pub mod wait;

use std::io::{self, Write};
use std::panic::{self, AssertUnwindSafe};
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

pub use environment::Environment;
pub use failure::{Exit, Failure};
pub use flags::{Colour, Format, Shared};
pub use help::Help;
pub use ink::{Ink, Pen};
pub use layout::{Align, Room, Row};
pub use wait::Wait;

/// What a command's body runs with.
pub struct Context {
    pub environment: Environment,
    pub shared: Shared,
    /// The arguments the shared flags were taken from, for the command
    /// to read its own way.
    pub args: Vec<String>,
    /// The pen for stdout, the answer's.
    pub out: Pen,
    /// The pen for stderr, the conversation's.
    pub err: Pen,
    /// How wide stdout is.
    pub room: Room,
    /// How wide stderr is.
    pub err_room: Room,
}

impl Context {
    /// A context for a command line, in an environment.
    pub fn new(
        environment: Environment,
        args: Vec<String>,
        forms: &[&str],
    ) -> Result<Self, Failure> {
        let (shared, args) = flags::take(args, forms)?;
        let theme = ink::theme(&environment);
        let out = Pen::new(
            Ink::decide(environment.stdout, shared.colour, &environment),
            theme.as_ref(),
        );
        let err = Pen::new(
            Ink::decide(environment.stderr, shared.colour, &environment),
            theme.as_ref(),
        );
        Ok(Self {
            room: Room::of_stdout(&environment),
            err_room: Room::of_stderr(&environment),
            environment,
            shared,
            args,
            out,
            err,
        })
    }

    /// The answer's form.
    pub fn format(&self) -> &Format {
        &self.shared.format
    }

    /// Whether a question can be asked: stdin is a terminal.
    pub fn can_ask(&self) -> bool {
        self.environment.stdin
    }

    /// The answer as one JSON object: pretty into a terminal, one line
    /// into a pipe. Under `jsonl` it's the one line before `end`.
    pub fn answer(&self, value: &Value) -> io::Result<()> {
        debug_assert!(
            value.get("schema").is_some(),
            "every answer names its schema"
        );
        let mut out = io::stdout().lock();
        if self.shared.format == Format::Jsonl {
            let mut line = json!({"type": "answer"});
            if let (Some(line), Some(fields)) = (line.as_object_mut(), value.as_object()) {
                line.extend(fields.clone());
            }
            return writeln!(out, "{line}");
        }
        if self.environment.stdout {
            writeln!(out, "{value:#}")
        } else {
            writeln!(out, "{value}")
        }
    }

    /// One line of a `jsonl` stream, typed.
    pub fn line(&self, kind: &str, fields: &Value) -> io::Result<()> {
        let mut line = json!({"type": kind});
        if let (Some(line), Some(fields)) = (line.as_object_mut(), fields.as_object()) {
            line.extend(fields.clone());
        }
        writeln!(io::stdout().lock(), "{line}")
    }

    /// Text for a person on stdout, as it's laid out.
    pub fn say(&self, text: &str) -> io::Result<()> {
        write!(io::stdout().lock(), "{text}")
    }

    /// A note on stderr: what changed, a warning, never the answer.
    pub fn tell(&self, mark: &str, role: &str, words: &str) -> io::Result<()> {
        writeln!(
            io::stderr().lock(),
            "{}{}",
            self.err.marked(mark, role),
            self.err
                .role("text", &layout::hung(words, layout::MARK, self.err_room))
        )
    }

    /// A wait said on stderr.
    pub fn wait(&self, words: &str) -> io::Result<Wait<'_, io::Stderr>> {
        Wait::begin(io::stderr(), &self.err, self.err_room, words)
    }
}

/// How a command's run ended, and what the kit itself wrote: help, the
/// version, the error, and a stream's `end` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ending {
    pub exit: Exit,
    pub stdout: String,
    pub stderr: String,
}

/// Runs a command's body over a command line in an environment, catching
/// what it ends with: what `main` does, apart from the process, so a test
/// reads a command's whole surface.
pub fn run(
    help: &Help,
    version: &str,
    environment: &Environment,
    args: Vec<String>,
    body: impl FnOnce(&Context) -> Result<(), Failure>,
) -> Ending {
    let forms: Vec<&str> = help.forms.iter().map(String::as_str).collect();
    let mut stdout = String::new();
    let context = match Context::new(environment.clone(), args, &forms) {
        Ok(context) => context,
        Err(failure) => {
            // The line couldn't be read, so neither could its form: it's
            // said as text, unless JSON was plainly asked for.
            let pen = Pen::new(
                Ink::decide(environment.stderr, Colour::Auto, environment),
                None,
            );
            let stderr = failure.text(&pen, Room::of_stderr(environment), false);
            return Ending {
                exit: failure.exit,
                stdout,
                stderr,
            };
        }
    };
    if context.shared.help {
        stdout = if context.shared.format.is_json() {
            format!("{}\n", help.json())
        } else {
            help.text(&context.out, context.room)
        };
        return Ending {
            exit: Exit::Done,
            stdout,
            stderr: String::new(),
        };
    }
    if context.shared.version {
        return Ending {
            exit: Exit::Done,
            stdout: format!("{} {version}\n", help.name),
            stderr: String::new(),
        };
    }
    let outcome = caught(|| body(&context));
    let (exit, stderr) = match outcome {
        Ok(()) => (Exit::Done, String::new()),
        Err(failure) => {
            let verbose = context
                .args
                .iter()
                .any(|arg| arg == "--verbose" || arg == "-v");
            let said = if context.shared.format.is_json() {
                format!("{}\n", failure.json(verbose))
            } else {
                failure.text(&context.err, context.err_room, verbose)
            };
            (failure.exit, said)
        }
    };
    if context.shared.format == Format::Jsonl {
        let outcome = if exit == Exit::Done { "done" } else { "failed" };
        stdout = format!(
            "{}\n",
            json!({"type": "end", "outcome": outcome, "exit": exit.code()})
        );
    }
    Ending {
        exit,
        stdout,
        stderr,
    }
}

/// The body's outcome, a panic in it a bug whose words are the panic's.
fn caught(body: impl FnOnce() -> Result<(), Failure>) -> Result<(), Failure> {
    let said: Arc<Mutex<Option<String>>> = Arc::default();
    let into = Arc::clone(&said);
    let before = panic::take_hook();
    // The panic is said as a failure, so the default hook's lines, which
    // would break stderr's JSON, are kept back.
    panic::set_hook(Box::new(move |info| {
        let words = info
            .payload()
            .downcast_ref::<&str>()
            .map(|words| (*words).to_owned())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_default();
        let place = info
            .location()
            .map(|place| format!(" at {}:{}", place.file(), place.line()))
            .unwrap_or_default();
        if let Ok(mut said) = into.lock() {
            *said = Some(format!("{words}{place}"));
        }
    }));
    let outcome = panic::catch_unwind(AssertUnwindSafe(body));
    panic::set_hook(before);
    outcome.unwrap_or_else(|_| {
        let why = said
            .lock()
            .ok()
            .and_then(|said| said.clone())
            .unwrap_or_default();
        Err(Failure::new(Exit::Bug, "it broke where it shouldn't have").because(why))
    })
}

/// A command's whole run: the shared flags taken from this process's
/// arguments, help and version answered, its body run, and how it ended
/// written out and exited with.
pub fn main(
    help: &Help,
    version: &str,
    body: impl FnOnce(&Context) -> Result<(), Failure>,
) -> ExitCode {
    let ending = run(
        help,
        version,
        &Environment::here(),
        std::env::args().skip(1).collect(),
        body,
    );
    // Nothing is left to say if a stream has closed, as it has when a
    // pipe's reader stops early.
    let _ = io::stdout().lock().write_all(ending.stdout.as_bytes());
    let _ = io::stderr().lock().write_all(ending.stderr.as_bytes());
    ending.exit.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn help() -> Help {
        Help::new("thing", "does a thing").command("go", "does it")
    }

    fn ran(line: &str, body: impl FnOnce(&Context) -> Result<(), Failure>) -> Ending {
        let args = line.split_whitespace().map(str::to_owned).collect();
        run(
            &help(),
            "1.0",
            &Environment::of([false; 3], &[]),
            args,
            body,
        )
    }

    #[test]
    fn help_is_answered_and_nothing_starts() {
        let ending = ran("go --help", |_| panic!("help started the command"));
        assert_eq!(ending.exit, Exit::Done);
        assert!(
            ending.stdout.starts_with("thing  does a thing\n"),
            "{}",
            ending.stdout
        );
        assert_eq!(ending.stderr, "");
    }

    #[test]
    fn help_as_json_is_its_declaration() {
        let ending = ran("--help --json", |_| Ok(()));
        let value: Value = serde_json::from_str(&ending.stdout).expect("help is JSON");
        assert_eq!(value["schema"], "help/1");
    }

    #[test]
    fn version_is_one_line() {
        assert_eq!(ran("--version", |_| Ok(())).stdout, "thing 1.0\n");
    }

    #[test]
    fn a_failure_exits_with_its_code_and_says_itself() {
        let ending = ran("go", |_| Err(Failure::new(Exit::NotFound, "no such row")));
        assert_eq!(ending.exit, Exit::NotFound);
        assert_eq!(ending.stderr, "\u{2715} no such row\n");
        let ending = ran("go --json", |_| {
            Err(Failure::new(Exit::NotFound, "no such row"))
        });
        let value: Value = serde_json::from_str(&ending.stderr).expect("the error is JSON");
        assert_eq!(value["kind"], "not-found");
    }

    #[test]
    fn a_line_it_cant_read_is_asked_wrongly() {
        assert_eq!(ran("--format yaml", |_| Ok(())).exit, Exit::Usage);
    }

    #[test]
    fn a_stream_ends_with_its_outcome() {
        let ending = ran("go --format jsonl", |_| {
            Err(Failure::new(Exit::TryAgain, "busy"))
        });
        assert_eq!(
            ending.stdout,
            "{\"exit\":75,\"outcome\":\"failed\",\"type\":\"end\"}\n"
        );
    }

    #[test]
    fn a_panic_is_a_bug() {
        let ending = ran("go", |_| panic!("the row was gone"));
        assert_eq!(ending.exit, Exit::Bug);
        assert!(
            ending.stderr.contains("the row was gone"),
            "{}",
            ending.stderr
        );
    }

    #[test]
    fn the_body_reads_what_the_shared_flags_left() {
        let ending = ran("go --color never far", |context| {
            assert_eq!(context.args, ["go", "far"]);
            assert!(!context.out.inks());
            Ok(())
        });
        assert_eq!(ending.exit, Exit::Done);
    }
}
