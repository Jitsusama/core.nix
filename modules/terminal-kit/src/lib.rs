//! The terminal kit: the shape every command of ours takes, for Joel in a
//! terminal and for a machine through a pipe, so learning one command
//! teaches them all (docs/commands.md). A command hands `main` its help
//! and its body; the kit takes the shared flags, answers `--help` and
//! `--version` without starting anything, gives the body a `Context`
//! holding the answer's form and a pen for each stream, and ends the
//! process with the exit its body chose: a failure said in its three
//! parts, as text or as JSON, and a panic as a bug, 70. A command whose
//! help declares subcommands of its own has its words walked down them
//! first (tree.rs): each answers its own help, and a word none answers to
//! is refused before the body starts. A command that asks yes or no hands
//! `ask` its body instead, which answers 0 for yes and 1 for no, saying
//! nothing for no, so a no never reads as broken.
//!
//! It depends on `serde_json` and `unicode-width` alone, since a parser crate
//! would be most of a small command's dependencies: the command reads
//! what's left of its arguments its own way. Where a command says a local
//! time, `local` reads it through the C library, the one copy of its
//! `struct tm` every command shares.
//!
//! Long because the one path every command runs, from its words to its
//! exit, stays readable in one place, and its tests drive that whole path
//! through `run` beside it.

pub mod environment;
pub mod failure;
pub mod flags;
mod handing;
pub mod help;
pub mod ink;
pub mod layout;
pub mod local;
pub mod screen;
pub mod time;
pub mod tree;
pub mod wait;

use std::cell::{Cell, RefCell};
use std::ffi::OsString;
use std::fmt::Write as _;
use std::io::{self, Write};
use std::panic::{self, AssertUnwindSafe};
use std::process::ExitCode;
use std::sync::Once;

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
    /// How many of `args` are the command's own, the rest handed on to
    /// the program a subcommand runs (`Help::hands_words_on`).
    own: usize,
}

impl Context {
    /// A context for a command line, in an environment.
    pub fn new(
        environment: Environment,
        args: Vec<String>,
        forms: &[&str],
    ) -> Result<Self, Failure> {
        Self::saying(environment, args, forms, true)
    }

    /// As `new`, saying a shared flag's wrong value only where `says` is
    /// set, as a command whose words may be private doesn't.
    fn saying(
        environment: Environment,
        args: Vec<String>,
        forms: &[&str],
        says: bool,
    ) -> Result<Self, Failure> {
        let (shared, args) = flags::take_saying(args, forms, says)?;
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
            own: args.len(),
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
        self.answer_to(&mut io::stdout().lock(), value)
    }

    /// The answer, as `answer` writes it, written to `out`: for a command
    /// that writes its answer where it's told, as its tests tell it.
    pub fn answer_to(&self, out: &mut dyn Write, value: &Value) -> io::Result<()> {
        debug_assert!(
            value.get("schema").is_some(),
            "every answer names its schema"
        );
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
        self.line_to(&mut io::stdout().lock(), kind, fields)
    }

    /// One line of a `jsonl` stream, as `line` writes it, written to `out`.
    pub fn line_to(&self, out: &mut dyn Write, kind: &str, fields: &Value) -> io::Result<()> {
        let mut line = json!({"type": kind});
        if let (Some(line), Some(fields)) = (line.as_object_mut(), fields.as_object()) {
            line.extend(fields.clone());
        }
        writeln!(out, "{line}")
    }

    /// Text for a person on stdout, as it's laid out.
    pub fn say(&self, text: &str) -> io::Result<()> {
        write!(io::stdout().lock(), "{text}")
    }

    /// A note on stderr: what changed, a warning, never the answer.
    pub fn tell(&self, mark: &str, role: &str, words: &str) -> io::Result<()> {
        writeln!(
            io::stderr().lock(),
            "{}",
            told(&self.err, self.err_room, mark, role, words)
        )
    }

    /// A wait said on stderr.
    pub fn wait(&self, words: &str) -> io::Result<Wait<'_, io::Stderr>> {
        Wait::begin(io::stderr(), &self.err, self.err_room, words)
    }
}

/// A note as `tell` draws it: its mark, then the words hung under it,
/// spaced as a failure's words are, so none reads as a try line.
fn told(pen: &Pen, room: Room, mark: &str, role: &str, words: &str) -> String {
    format!(
        "{}{}",
        pen.marked(mark, role),
        pen.role(
            "text",
            &layout::hung(&layout::spaced(words), layout::MARK, room)
        )
    )
}

/// How a command's run ended, and what the kit itself wrote: help, the
/// version, the error, and a stream's `end` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ending {
    pub exit: Exit,
    pub stdout: String,
    pub stderr: String,
    /// The code of the program it ran as its stand-in, where it failed
    /// with one (`Failure::passed`).
    pub passed: Option<u8>,
}

impl Ending {
    /// The number the process ends with.
    pub fn code(&self) -> u8 {
        self.passed.unwrap_or_else(|| self.exit.code())
    }
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
    run_question(help, version, environment, args, |context| {
        body(context).map(|()| true)
    })
}

/// Runs a question's body as `run` runs a command's: yes ends 0, no ends
/// 1 saying nothing but a stream's `end` line, whose outcome is `no`, and
/// a failure is said as any command's is (docs/commands.md).
pub fn run_question(
    help: &Help,
    version: &str,
    environment: &Environment,
    args: Vec<String>,
    body: impl FnOnce(&Context) -> Result<bool, Failure>,
) -> Ending {
    let forms: Vec<&str> = help.forms.iter().map(String::as_str).collect();
    let (ours, theirs, helped) = handing::split(help, args);
    let mut context =
        match Context::saying(environment.clone(), ours, &forms, !help.keeps_any_unsaid()) {
            Ok(context) => context,
            Err(failure) => return unread(environment, &failure),
        };
    context.args.extend(theirs);
    context.shared.help |= helped;
    // A command that hands every word on reads its subcommands its own
    // way, so its words aren't walked.
    let walked = if help.subs.is_empty() || help.hands_on.is_some() {
        None
    } else {
        asked_for_help(help, &mut context);
        match tree::walk(help, &context.args) {
            Ok(walked) => Some(walked),
            Err(failure) => return failed(&context, environment, &failure),
        }
    };
    if context.shared.help {
        let shown = walked.as_ref().map(tree::Walked::shown);
        let shown = shown.as_ref().unwrap_or(help);
        let stdout = if context.shared.format.is_json() {
            format!("{}\n", shown.json())
        } else {
            shown.text(&context.out, context.room)
        };
        return Ending {
            exit: Exit::Done,
            stdout,
            stderr: String::new(),
            passed: None,
        };
    }
    if context.shared.version {
        return Ending {
            exit: Exit::Done,
            stdout: format!("{} {version}\n", help.name),
            stderr: String::new(),
            passed: None,
        };
    }
    if let Some(walked) = walked {
        context.args = walked.words;
    }
    match caught(|| body(&context)) {
        Ok(true) => ended(&context, (Exit::Done, None), "done", String::new()),
        Ok(false) => ended(&context, (Exit::Failed, None), "no", String::new()),
        Err(failure) => failed(&context, environment, &failure),
    }
}

/// Takes `help` as the first word, as `eero help devices`, for `--help`
/// on what follows it, unless a subcommand is called help.
fn asked_for_help(help: &Help, context: &mut Context) {
    let first = context.args.iter().position(|arg| !arg.starts_with('-'));
    let own = help.subs.iter().any(|sub| sub.name == "help");
    if let Some(at) = first
        && context.args[at] == "help"
        && !own
    {
        context.args.remove(at);
        context.shared.help = true;
    }
}

/// How a run ends that failed: the failure said on stderr in the answer's
/// form, marked for the journal where stderr is the journal's.
fn failed(context: &Context, environment: &Environment, failure: &Failure) -> Ending {
    let verbose = context
        .args
        .iter()
        .take(context.own)
        .any(|arg| arg == "--verbose" || arg == "-v");
    // A failure said already was said in words for people; a reader of
    // JSON still gets its object, the only form it reads.
    let said = if context.shared.format.is_json() {
        format!("{}\n", failure.json(verbose))
    } else if failure.said {
        String::new()
    } else {
        for_the_journal(
            failure.text(&context.err, context.err_room, verbose),
            environment,
        )
    };
    ended(context, (failure.exit, failure.passed), "failed", said)
}

/// How a run ends, with its exit and any stand-in's code, and a stream's
/// `end` line under `jsonl`.
fn ended(
    context: &Context,
    (exit, passed): (Exit, Option<u8>),
    outcome: &str,
    stderr: String,
) -> Ending {
    let ending = Ending {
        exit,
        stdout: String::new(),
        stderr,
        passed,
    };
    let stdout = if context.shared.format == Format::Jsonl {
        format!(
            "{}\n",
            json!({"type": "end", "outcome": outcome, "exit": ending.code()})
        )
    } else {
        String::new()
    };
    Ending { stdout, ..ending }
}

/// A failure's text as the journal reads an error, where stderr is the
/// journal's: each line opens with `<3>`, sd-daemon(3)'s mark for one, so
/// `journalctl -p err` finds a daemon's failures. Elsewhere it's as it was.
fn for_the_journal(said: String, environment: &Environment) -> String {
    if !environment.journal {
        return said;
    }
    said.lines().fold(String::new(), |mut marked, line| {
        let _ = writeln!(marked, "<3>{line}");
        marked
    })
}

/// How a run ends whose line couldn't be read, and so neither could its
/// form: the failure is said as text, marked for the journal where stderr
/// is the journal's.
fn unread(environment: &Environment, failure: &Failure) -> Ending {
    let pen = Pen::new(
        Ink::decide(environment.stderr, Colour::Auto, environment),
        None,
    );
    Ending {
        exit: failure.exit,
        passed: failure.passed,
        stdout: String::new(),
        stderr: for_the_journal(
            failure.text(&pen, Room::of_stderr(environment), false),
            environment,
        ),
    }
}

/// As `run`, over the arguments as the system gives them. An argument
/// that isn't UTF-8 is refused as asked wrongly, where reading it as
/// `String`s would panic before the body could say anything.
pub fn run_os(
    help: &Help,
    version: &str,
    environment: &Environment,
    args: Vec<OsString>,
    body: impl FnOnce(&Context) -> Result<(), Failure>,
) -> Ending {
    match texts(args, !help.keeps_any_unsaid()) {
        Ok(args) => run(help, version, environment, args, body),
        Err(failure) => unread(environment, &failure),
    }
}

/// The arguments as text, or the failure of one that isn't, saying it
/// only where `says` is set.
fn texts(args: Vec<OsString>, says: bool) -> Result<Vec<String>, Failure> {
    args.into_iter()
        .map(OsString::into_string)
        .collect::<Result<_, _>>()
        .map_err(|word| {
            let failure = Failure::new(Exit::Usage, "an argument isn't UTF-8 text");
            if says {
                failure.because(format!("{} can't be read as text", word.to_string_lossy()))
            } else {
                failure.because("it isn't said here, since it may be private")
            }
        })
}

thread_local! {
    /// Whether this thread is in a body `caught` runs, and what its panic
    /// said once it has.
    static CATCHING: Cell<bool> = const { Cell::new(false) };
    static SAID: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// The body's outcome, a panic in it a bug whose words are the panic's.
/// The panic hook is the whole process's, so it's set once and keeps the
/// words only of a thread in a body, handing every other panic to the
/// hook it found: swapped in and out around each body, two at once on
/// two threads would put back each other's.
fn caught<T>(body: impl FnOnce() -> Result<T, Failure>) -> Result<T, Failure> {
    static HOOK: Once = Once::new();
    HOOK.call_once(|| {
        let before = panic::take_hook();
        // The panic is said as a failure, so the default hook's lines,
        // which would break stderr's JSON, are kept back.
        panic::set_hook(Box::new(move |info| {
            if !CATCHING.get() {
                return before(info);
            }
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
            SAID.set(Some(format!("{words}{place}")));
        }));
    });
    SAID.set(None);
    CATCHING.set(true);
    let outcome = panic::catch_unwind(AssertUnwindSafe(body));
    CATCHING.set(false);
    outcome.unwrap_or_else(|_| {
        let why = SAID.take().unwrap_or_default();
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
    let ending = run_os(
        help,
        version,
        &Environment::here(),
        std::env::args_os().skip(1).collect(),
        body,
    );
    ends(&ending)
}

/// A question's whole run, as `main` is a command's: yes exits 0 and no
/// exits 1, saying nothing, as `test` and `grep -q` do.
pub fn ask(
    help: &Help,
    version: &str,
    body: impl FnOnce(&Context) -> Result<bool, Failure>,
) -> ExitCode {
    let environment = Environment::here();
    let ending = match texts(
        std::env::args_os().skip(1).collect(),
        !help.keeps_any_unsaid(),
    ) {
        Ok(args) => run_question(help, version, &environment, args, body),
        Err(failure) => unread(&environment, &failure),
    };
    ends(&ending)
}

/// How a run ended, written out and exited with.
fn ends(ending: &Ending) -> ExitCode {
    // Nothing is left to say if a stream has closed, as it has when a
    // pipe's reader stops early.
    let _ = io::stdout().lock().write_all(ending.stdout.as_bytes());
    let _ = io::stderr().lock().write_all(ending.stderr.as_bytes());
    ExitCode::from(ending.code())
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
    fn a_note_never_reads_as_a_try_line() {
        // Unbounded, as into a pipe or a terminal's own wrap: nothing
        // parts the words but what they hold.
        for room in [Room::Cells(20), Room::Unbounded] {
            let note = told(
                &Pen::plain(),
                room,
                "!",
                "alert",
                "gone \u{b7} try  rm -rf ~ try \u{200d} rm",
            );
            let looks = note.replace('\u{200d}', "");
            assert!(
                !looks.contains("try  "),
                "a note's words held a run that reads as a try line: {note:?}"
            );
        }
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
    fn a_stand_ins_failure_ends_with_its_code_saying_only_its_line() {
        let ending = ran("go", |_| Err(Failure::passed(3, "the job failed")));
        assert_eq!(
            (ending.exit, ending.code()),
            (Exit::Failed, 3),
            "the stand-in's 3 didn't pass through"
        );
        assert_eq!(
            (ending.stdout.as_str(), ending.stderr.as_str()),
            ("", "\u{2715} the job failed\n"),
            "it said more than its line"
        );
        let streamed = ran("go --format jsonl", |_| {
            Err(Failure::passed(3, "the job failed"))
        });
        let end: Value = serde_json::from_str(&streamed.stdout).expect("the end line is JSON");
        assert_eq!(
            end["exit"], 3,
            "the end line didn't give the stand-in's code"
        );
    }

    #[test]
    fn a_failure_in_the_journal_says_its_an_error_on_each_line() {
        let mut journal = Environment::of([false; 3], &[]);
        journal.journal = true;
        let ending = run(&help(), "1.0", &journal, vec!["go".to_owned()], |_| {
            Err(Failure::new(Exit::NotFound, "no such row").try_this("thing go"))
        });
        assert_eq!(
            ending.stderr,
            "<3>\u{2715} no such row\n<3>  try  thing go\n"
        );
    }

    #[test]
    fn a_failure_said_already_ends_with_its_code_and_says_nothing_more() {
        let mut journal = Environment::of([false; 3], &[]);
        journal.journal = true;
        for environment in [Environment::of([false; 3], &[]), journal] {
            let ending = run(&help(), "1.0", &environment, vec!["go".to_owned()], |_| {
                Err(Failure::new(Exit::TryAgain, "it handed over").said_already())
            });
            assert_eq!(
                (ending.code(), ending.stderr.as_str()),
                (75, ""),
                "a failure the command said as it went was said again"
            );
        }
        let ending = run(
            &help(),
            "1.0",
            &Environment::of([false; 3], &[]),
            vec!["--json".to_owned(), "go".to_owned()],
            |_| Err(Failure::new(Exit::TryAgain, "it handed over").said_already()),
        );
        assert_eq!(ending.code(), 75, "its code is kept under --json");
        assert!(
            ending.stderr.contains("\"it handed over\""),
            "a reader of JSON gets the failure's object: {}",
            ending.stderr
        );
    }

    #[test]
    fn a_line_it_cant_read_is_asked_wrongly() {
        assert_eq!(ran("--format yaml", |_| Ok(())).exit, Exit::Usage);
    }

    #[test]
    fn an_argument_that_isnt_text_is_asked_wrongly_before_the_body() {
        use std::os::unix::ffi::OsStringExt;
        let ending = run_os(
            &help(),
            "1.0",
            &Environment::of([false; 3], &[]),
            vec![OsString::from("go"), OsString::from_vec(vec![b'a', 0xff])],
            |_| panic!("the body ran"),
        );
        assert_eq!(ending.exit, Exit::Usage);
        assert!(ending.stderr.contains("a\u{fffd}"), "{}", ending.stderr);
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

    fn asked(line: &str, body: impl FnOnce(&Context) -> Result<bool, Failure>) -> Ending {
        let args = line.split_whitespace().map(str::to_owned).collect();
        run_question(
            &help(),
            "1.0",
            &Environment::of([false; 3], &[]),
            args,
            body,
        )
    }

    #[test]
    fn a_question_answered_yes_ends_done_saying_nothing() {
        let ending = asked("go", |_| Ok(true));
        assert_eq!(ending.exit, Exit::Done);
        assert_eq!(
            (ending.stdout, ending.stderr),
            (String::new(), String::new())
        );
    }

    #[test]
    fn a_question_answered_no_ends_1_saying_nothing_but_its_streams_end() {
        let ending = asked("go", |_| Ok(false));
        assert_eq!(ending.exit, Exit::Failed);
        assert_eq!(
            (ending.stdout, ending.stderr),
            (String::new(), String::new())
        );
        let ending = asked("go --format jsonl", |_| Ok(false));
        assert_eq!(
            ending.stdout,
            "{\"exit\":1,\"outcome\":\"no\",\"type\":\"end\"}\n"
        );
        assert_eq!(ending.stderr, "", "a no said something on stderr");
    }

    #[test]
    fn a_question_that_cant_be_answered_fails_as_any_command_does() {
        let ending = asked("go", |_| {
            Err(Failure::new(Exit::Unavailable, "nothing answered"))
        });
        assert_eq!(ending.exit, Exit::Unavailable);
        assert_eq!(ending.stderr, "\u{2715} nothing answered\n");
    }

    #[test]
    fn panics_on_two_threads_each_keep_their_words() {
        let ran: Vec<_> = (0..8)
            .map(|row| {
                std::thread::spawn(move || {
                    let ending = ran("go", |_| panic!("row {row} was gone"));
                    (row, ending)
                })
            })
            .collect();
        for done in ran {
            let (row, ending) = done
                .join()
                .expect("a body's panic was caught on its thread");
            assert_eq!(ending.exit, Exit::Bug);
            assert!(
                ending.stderr.contains(&format!("row {row} was gone")),
                "row {row} lost its words: {}",
                ending.stderr
            );
        }
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

    fn tree() -> Help {
        Help::new("thing", "does a thing")
            .subcommand(
                Help::new("rows", "every row, or one")
                    .alias("row")
                    .argument("<name>", "a row's name")
                    .example("thing rows disk", "the disk's row"),
            )
            .exit(Exit::NotFound, "no such row")
    }

    fn ran_tree(line: &str, body: impl FnOnce(&Context) -> Result<(), Failure>) -> Ending {
        let args = line.split_whitespace().map(str::to_owned).collect();
        run(
            &tree(),
            "1.0",
            &Environment::of([false; 3], &[]),
            args,
            body,
        )
    }

    #[test]
    fn each_way_of_asking_for_a_subcommands_help_answers_it_alone() {
        for line in ["rows --help", "rows -h", "help rows", "row disk --help"] {
            let ending = ran_tree(line, |_| panic!("{line} started the command"));
            assert_eq!(ending.exit, Exit::Done, "{line}: {}", ending.stderr);
            assert!(
                ending.stdout.starts_with("thing rows  every row, or one\n"),
                "{line} didn't answer rows' help: {}",
                ending.stdout
            );
            assert!(
                ending.stdout.contains("66 no such row"),
                "{line} lost its parent's exits: {}",
                ending.stdout
            );
        }
        let ending = ran_tree("help rows --json", |_| Ok(()));
        let value: Value = serde_json::from_str(&ending.stdout).expect("help is JSON");
        assert_eq!(
            value["name"], "thing rows",
            "the JSON named the wrong command"
        );
    }

    #[test]
    fn a_word_no_subcommand_answers_to_is_refused_before_the_body() {
        for line in ["rwos", "rwos --help"] {
            let ending = ran_tree(line, |_| panic!("{line} started the command"));
            assert_eq!(ending.exit, Exit::Usage, "{line} wasn't refused");
            assert!(
                ending.stderr.contains("did you mean rows?"),
                "{line} didn't name the nearest: {}",
                ending.stderr
            );
        }
    }

    #[test]
    fn the_body_reads_an_alias_as_the_name_it_stands_for() {
        let ending = ran_tree("row disk", |context| {
            assert_eq!(context.args, ["rows", "disk"], "the alias reached the body");
            Ok(())
        });
        assert_eq!(ending.exit, Exit::Done, "{}", ending.stderr);
    }

    #[test]
    fn the_body_reads_what_the_shared_flags_left() {
        let ending = ran("go --color never far", |context| {
            assert_eq!(context.args, ["go", "far"]);
            assert!(!context.out.inks(), "--color never inks nothing");
            Ok(())
        });
        assert_eq!(ending.exit, Exit::Done);
    }
}
