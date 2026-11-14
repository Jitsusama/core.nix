//! wl-copy and wl-paste as a fenced command runs them, asking the door.
//!
//! They take the options agents use, as wl-clipboard spells them, and
//! exit as it does: 0 when done, 1 on anything else. An option they
//! don't take is refused by name, never passed over, so a command never
//! believes it did something it didn't: `--watch`, `--seat` and
//! `--foreground` among them, which a door that answers a request at a
//! time has no meaning for.
//!
//! The terminal kit says their help and their refusals (docs/commands.md),
//! so they take its shared flags as well; `--format` takes only text,
//! since what's pasted is written as it is.

use std::io::{self, Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

use terminal_kit::{Ending, Environment, Exit, Failure, Format, Help};

use crate::kinds;
use crate::wire::{Answer, LONGEST_LINE, Request, Selection, Wanted, plain_kind};

/// The version said, for whatever asks.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Where the door's socket is in the runtime folder.
const DOOR: &str = "pi-clipboard/door.sock";

/// What overrides where the door is, for tests.
const DOOR_VARIABLE: &str = "PI_CLIPBOARD_DOOR";

/// Which of the two the shim was run as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Program {
    Copy,
    Paste,
}

impl Program {
    /// The program a path runs as, by its last part.
    pub fn named(path: &str) -> Option<Self> {
        match Path::new(path).file_name()?.to_str()? {
            "wl-copy" => Some(Program::Copy),
            "wl-paste" => Some(Program::Paste),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Program::Copy => "wl-copy",
            Program::Paste => "wl-paste",
        }
    }
}

/// What a command line asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asked {
    Copy(Copying),
    Paste(Pasting),
    Help,
    Version,
}

/// A copy, or a clearing, as wl-copy's options say it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Copying {
    pub selection: Selection,
    pub kind: Option<String>,
    /// Leave out the trailing newline, when there is one.
    pub trim_newline: bool,
    pub once: bool,
    pub clear: bool,
    /// The text to copy, from the arguments, or stdin when none.
    pub text: Option<String>,
}

/// A paste, or a listing, as wl-paste's options say it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pasting {
    pub selection: Selection,
    pub wanted: Wanted,
    pub no_newline: bool,
    pub list: bool,
}

/// An option as wl-clipboard names it: short, long, and whether it takes
/// a value.
struct Known {
    short: char,
    long: &'static str,
    value: bool,
}

const COPY_OPTIONS: [Known; 7] = [
    Known {
        short: 'p',
        long: "primary",
        value: false,
    },
    Known {
        short: 'n',
        long: "trim-newline",
        value: false,
    },
    Known {
        short: 't',
        long: "type",
        value: true,
    },
    Known {
        short: 'c',
        long: "clear",
        value: false,
    },
    Known {
        short: 'o',
        long: "paste-once",
        value: false,
    },
    Known {
        short: 'h',
        long: "help",
        value: false,
    },
    Known {
        short: 'v',
        long: "version",
        value: false,
    },
];

const PASTE_OPTIONS: [Known; 6] = [
    Known {
        short: 'p',
        long: "primary",
        value: false,
    },
    Known {
        short: 'n',
        long: "no-newline",
        value: false,
    },
    Known {
        short: 't',
        long: "type",
        value: true,
    },
    Known {
        short: 'l',
        long: "list-types",
        value: false,
    },
    Known {
        short: 'h',
        long: "help",
        value: false,
    },
    Known {
        short: 'v',
        long: "version",
        value: false,
    },
];

/// How a run of the shim as `name` with `args` ends: wl-copy's or
/// wl-paste's, its help, or, run by any other name, a refusal. What it
/// pastes is written to `stdout` as it comes; what the kit says is in the
/// ending, to be written after.
pub fn ending(
    name: &str,
    args: Vec<String>,
    environment: &Environment,
    door: Option<&Path>,
    stdin: &mut impl Read,
    stdout: &mut impl Write,
) -> Ending {
    let Some(program) = Program::named(name) else {
        return terminal_kit::run(&own_help(), VERSION, environment, args, |_| {
            Err(Failure::new(Exit::Usage, "it's run as wl-copy or wl-paste")
                .because("it stands in for both in pi's fence, told apart by the name it's run as")
                .try_this("wl-paste --help"))
        });
    };
    let help = help(program);
    let name = program.name();
    terminal_kit::run(&help, VERSION, environment, args, |context| {
        if *context.format() != Format::Text {
            return Err(
                Failure::new(Exit::Failed, format!("{name} answers only as text"))
                    .because("what's copied or pasted is written as it is")
                    .try_this(format!("{name} --help")),
            );
        }
        let asked = parse(program, &context.args)
            .map_err(|said| Failure::new(Exit::Failed, said).try_this(format!("{name} --help")))?;
        match asked {
            Asked::Help => write!(stdout, "{}", help.text(&context.out, context.room)),
            Asked::Version => writeln!(stdout, "{name} {VERSION}"),
            asked => {
                let door = door.ok_or_else(|| {
                    Failure::new(Exit::Failed, "the fence's clipboard door can't be found")
                        .because("there's no runtime folder to find it in")
                })?;
                return run(&asked, door, stdin, stdout)
                    .map_err(|said| Failure::new(Exit::Failed, said));
            }
        }
        .map_err(|error| {
            Failure::new(Exit::Failed, "stdout couldn't be written").because(error.to_string())
        })
    })
}

/// The help of the program itself, run by its own name.
fn own_help() -> Help {
    Help::new(
        "pi-clipboard-shim",
        "wl-copy and wl-paste for pi's fenced commands, run by those names",
    )
    .example("wl-copy --help", "what wl-copy takes")
    .example("wl-paste --help", "what wl-paste takes")
}

/// The help of `program`, as the kit sets it.
fn help(program: Program) -> Help {
    let failed = "the door refused, or couldn't be reached; it says why";
    match program {
        Program::Copy => Help::new(
            "wl-copy",
            "copies the text given, or else stdin, through the fence's clipboard door",
        )
        .option("-p, --primary", "the primary selection, not the clipboard")
        .option("-n, --trim-newline", "leave out a trailing newline")
        .option(
            "-t, --type TYPE",
            "offer it as TYPE, not what it looks like",
        )
        .option("-c, --clear", "empty it instead")
        .option("-o, --paste-once", "serve one paste, then empty it")
        .option("-v", "its version")
        .example(
            "wl-copy -p -n < note",
            "copies the note to the primary selection",
        )
        .exit(Exit::Failed, failed),
        Program::Paste => Help::new(
            "wl-paste",
            "writes what's copied to stdout, through the fence's clipboard door",
        )
        .option("-p, --primary", "the primary selection, not the clipboard")
        .option("-n, --no-newline", "add no newline after text")
        .option(
            "-t, --type TYPE",
            "read it as TYPE, or any of a family, as text",
        )
        .option("-l, --list-types", "list the types it's offered as")
        .option("-v", "its version")
        .example("wl-paste -p -n", "writes the primary selection as it is")
        .exit(Exit::Failed, failed),
    }
}

/// What a command line asks of `program`, or what's wrong with it.
pub fn parse(program: Program, args: &[String]) -> Result<Asked, String> {
    let known: &[Known] = match program {
        Program::Copy => &COPY_OPTIONS,
        Program::Paste => &PASTE_OPTIONS,
    };
    let (options, words) = options(known, args)?;
    if options.iter().any(|(short, _)| *short == 'h') {
        return Ok(Asked::Help);
    }
    if options.iter().any(|(short, _)| *short == 'v') {
        return Ok(Asked::Version);
    }
    let given = |short: char| options.iter().any(|(option, _)| *option == short);
    let selection = if given('p') {
        Selection::Primary
    } else {
        Selection::Clipboard
    };
    let kind = options
        .iter()
        .rev()
        .find_map(|(short, value)| (*short == 't').then(|| value.clone()))
        .flatten();
    if let Some(kind) = &kind
        && plain_kind(kind).is_none()
    {
        return Err(format!("{kind:?} isn't a type the clipboard door takes"));
    }
    match program {
        Program::Copy => Ok(Asked::Copy(Copying {
            selection,
            kind,
            trim_newline: given('n'),
            once: given('o'),
            clear: given('c'),
            text: (!words.is_empty()).then(|| words.join(" ")),
        })),
        Program::Paste => {
            if let Some(word) = words.first() {
                return Err(format!("it takes no arguments, and was given {word:?}"));
            }
            Ok(Asked::Paste(Pasting {
                selection,
                wanted: kind.map_or(Wanted::Any, Wanted::Kind),
                no_newline: given('n'),
                list: given('l'),
            }))
        }
    }
}

/// The options given, by their short names with their values, and the
/// other words, in getopt's way: options anywhere until `--`, short ones
/// run together, a value after `=` or in the next word.
#[expect(clippy::type_complexity, reason = "a pair of lists, read once")]
fn options(
    known: &[Known],
    args: &[String],
) -> Result<(Vec<(char, Option<String>)>, Vec<String>), String> {
    let mut options = Vec::new();
    let mut words = Vec::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if arg == "--" {
            words.extend(args.by_ref().cloned());
        } else if let Some(long) = arg.strip_prefix("--") {
            let (name, inline) = match long.split_once('=') {
                Some((name, value)) => (name, Some(value.to_owned())),
                None => (long, None),
            };
            let option = known
                .iter()
                .find(|option| option.long == name)
                .ok_or_else(|| refused(&format!("--{name}")))?;
            let value = match (option.value, inline) {
                (true, Some(value)) => Some(value),
                (true, None) => Some(
                    args.next()
                        .ok_or_else(|| format!("--{name} needs a value"))?
                        .clone(),
                ),
                (false, Some(_)) => return Err(format!("--{name} takes no value")),
                (false, None) => None,
            };
            options.push((option.short, value));
        } else if let Some(shorts) = arg.strip_prefix('-').filter(|shorts| !shorts.is_empty()) {
            for (at, short) in shorts.char_indices() {
                let option = known
                    .iter()
                    .find(|option| option.short == short)
                    .ok_or_else(|| refused(&format!("-{short}")))?;
                if option.value {
                    let rest = &shorts[at + short.len_utf8()..];
                    let value = if rest.is_empty() {
                        args.next()
                            .ok_or_else(|| format!("-{short} needs a value"))?
                            .clone()
                    } else {
                        rest.to_owned()
                    };
                    options.push((short, Some(value)));
                    break;
                }
                options.push((short, None));
            }
        } else {
            words.push(arg.clone());
        }
    }
    Ok((options, words))
}

fn refused(option: &str) -> String {
    format!("{option} isn't an option the fence's clipboard door takes")
}

/// Where the door's socket is: as `PI_CLIPBOARD_DOOR` says, or in the
/// runtime folder.
pub fn door() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os(DOOR_VARIABLE) {
        return Some(PathBuf::from(path));
    }
    std::env::var_os("XDG_RUNTIME_DIR").map(|runtime| PathBuf::from(runtime).join(DOOR))
}

/// A copy or a paste, done through the door at `door`, or what was wrong.
fn run(
    asked: &Asked,
    door: &Path,
    stdin: &mut impl Read,
    stdout: &mut impl Write,
) -> Result<(), String> {
    match asked {
        Asked::Copy(copying) => copied(copying, door, stdin),
        Asked::Paste(pasting) => pasted(pasting, door, stdout),
        Asked::Help | Asked::Version => Ok(()),
    }
}

/// What wl-copy does: the door asked to copy, or to clear.
fn copied(copying: &Copying, door: &Path, stdin: &mut impl Read) -> Result<(), String> {
    if copying.clear {
        let mut stream = asking(
            door,
            &Request::Clear {
                selection: copying.selection,
            },
            &[],
        )?;
        return answered(&mut stream).map(drop);
    }
    let mut bytes = if let Some(text) = &copying.text {
        text.clone().into_bytes()
    } else {
        let mut bytes = Vec::new();
        stdin
            .read_to_end(&mut bytes)
            .map_err(|error| format!("stdin couldn't be read: {error}"))?;
        bytes
    };
    if copying.trim_newline && bytes.last() == Some(&b'\n') {
        bytes.pop();
    }
    let request = Request::Copy {
        selection: copying.selection,
        kind: copying.kind.clone(),
        once: copying.once,
    };
    let mut stream = asking(door, &request, &bytes)?;
    answered(&mut stream).map(drop)
}

/// What wl-paste does: the door asked for a selection, or its types.
fn pasted(pasting: &Pasting, door: &Path, stdout: &mut impl Write) -> Result<(), String> {
    let request = if pasting.list {
        Request::List {
            selection: pasting.selection,
        }
    } else {
        Request::Paste {
            selection: pasting.selection,
            wanted: pasting.wanted.clone(),
        }
    };
    let mut stream = asking(door, &request, &[])?;
    let kind = answered(&mut stream)?;
    let written = |error: io::Error| format!("stdout couldn't be written: {error}");
    io::copy(&mut stream, stdout).map_err(written)?;
    let text = kind.as_deref().is_some_and(kinds::is_text);
    if !pasting.list && !pasting.no_newline && text {
        stdout.write_all(b"\n").map_err(written)?;
    }
    stdout.flush().map_err(written)
}

/// The door at `door`, sent `request` and `bytes` after it, its half
/// shut so it knows all is sent.
fn asking(door: &Path, request: &Request, bytes: &[u8]) -> Result<UnixStream, String> {
    let mut stream = UnixStream::connect(door).map_err(|error| {
        format!(
            "the fence's clipboard door can't be reached at {}: {error}",
            door.display()
        )
    })?;
    let lost = |error: io::Error| format!("the fence's clipboard door stopped listening: {error}");
    stream
        .write_all(format!("{}\n", request.line()).as_bytes())
        .map_err(lost)?;
    stream.write_all(bytes).map_err(lost)?;
    stream.shutdown(Shutdown::Write).map_err(lost)?;
    Ok(stream)
}

/// The door's answer: the type it read, when it read one, or what it
/// said in refusing.
fn answered(stream: &mut UnixStream) -> Result<Option<String>, String> {
    let mut line = Vec::new();
    let mut byte = [0];
    let unanswered = || "the fence's clipboard door didn't answer".to_owned();
    loop {
        match stream.read(&mut byte) {
            Ok(1) if byte[0] == b'\n' => break,
            Ok(1) if line.len() < LONGEST_LINE => line.push(byte[0]),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            _ => return Err(unanswered()),
        }
    }
    let line = String::from_utf8(line).map_err(|_| unanswered())?;
    match Answer::read(&line).ok_or_else(unanswered)? {
        Answer::Ok(kind) => Ok(kind),
        Answer::No(why) => Err(why.sentence().to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;
    use std::thread;

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    fn copying(line: &str) -> Copying {
        match parse(Program::Copy, &words(line)) {
            Ok(Asked::Copy(copying)) => copying,
            other => panic!("{line}: {other:?}"),
        }
    }

    fn pasting(line: &str) -> Pasting {
        match parse(Program::Paste, &words(line)) {
            Ok(Asked::Paste(pasting)) => pasting,
            other => panic!("{line}: {other:?}"),
        }
    }

    #[test]
    fn it_is_told_apart_by_the_name_it_runs_as() {
        assert_eq!(Program::named("/fence/bin/wl-copy"), Some(Program::Copy));
        assert_eq!(Program::named("wl-paste"), Some(Program::Paste));
        assert_eq!(Program::named("pi-clipboard-shim"), None);
    }

    #[test]
    fn wl_copys_options_are_read_as_getopt_reads_them() {
        let plain = copying("");
        assert_eq!(plain.selection, Selection::Clipboard);
        assert_eq!(plain.text, None);
        let all = copying("-pnoc -t text/html");
        assert_eq!(
            all,
            Copying {
                selection: Selection::Primary,
                kind: Some("text/html".into()),
                trim_newline: true,
                once: true,
                clear: true,
                text: None,
            }
        );
        assert_eq!(copying("-ttext/html").kind.as_deref(), Some("text/html"));
        assert_eq!(
            copying("--type=image/png").kind.as_deref(),
            Some("image/png")
        );
        assert_eq!(
            copying("--primary --trim-newline --paste-once").selection,
            Selection::Primary
        );
        assert_eq!(
            copying("hello -p there").text.as_deref(),
            Some("hello there")
        );
        assert_eq!(copying("-- -p -n").text.as_deref(), Some("-p -n"));
    }

    #[test]
    fn wl_pastes_options_are_read_as_getopt_reads_them() {
        assert_eq!(
            pasting("-pn --type text"),
            Pasting {
                selection: Selection::Primary,
                wanted: Wanted::Kind("text".into()),
                no_newline: true,
                list: false,
            }
        );
        assert!(pasting("-l").list, "-l didn't list the types");
        assert_eq!(pasting("").wanted, Wanted::Any);
    }

    #[test]
    fn an_option_it_doesnt_take_is_refused_by_name() {
        for (program, line, named) in [
            (Program::Paste, "--watch cat", "--watch"),
            (Program::Paste, "-w cat", "-w"),
            (Program::Paste, "-s seat0", "-s"),
            (Program::Copy, "--foreground", "--foreground"),
            (Program::Copy, "-pf", "-f"),
            (Program::Copy, "--regular", "--regular"),
            (Program::Copy, "--no-newline", "--no-newline"),
            (Program::Paste, "--trim-newline", "--trim-newline"),
        ] {
            let said = parse(program, &words(line)).unwrap_err();
            assert!(said.starts_with(named), "{line}: {said}");
        }
    }

    #[test]
    fn a_command_line_wrong_in_shape_is_refused() {
        for (program, line) in [
            (Program::Copy, words("-t")),
            (Program::Copy, words("--clear=yes")),
            (Program::Paste, words("stray")),
            (Program::Paste, vec!["-t".into(), "a b".into()]),
        ] {
            assert!(parse(program, &line).is_err(), "{line:?} was taken");
        }
    }

    /// How a run as `name` with the words of `line` ends, the door at
    /// `door`: its exit code, what it wrote to stdout, and what to stderr.
    fn ran_as(name: &str, line: &str, door: &Path, stdin: &[u8]) -> (u8, String, String) {
        let mut out = Vec::new();
        let ending = ending(
            name,
            words(line),
            &Environment::of([false; 3], &[]),
            Some(door),
            &mut &stdin[..],
            &mut out,
        );
        (
            ending.exit.code(),
            String::from_utf8_lossy(&out).into_owned() + &ending.stdout,
            ending.stderr,
        )
    }

    fn ran(program: Program, line: &str, door: &Path, stdin: &[u8]) -> (u8, String, String) {
        ran_as(program.name(), line, door, stdin)
    }

    #[test]
    fn help_and_version_are_answered_without_the_door() {
        assert_eq!(parse(Program::Copy, &words("-h -x")), Err(refused("-x")));
        assert_eq!(parse(Program::Copy, &words("-p --help")), Ok(Asked::Help));
        assert_eq!(parse(Program::Paste, &words("-v")), Ok(Asked::Version));
        let nowhere = Path::new("/nonexistent");
        for line in ["--help", "-ph", "-p --help"] {
            let (code, out, err) = ran(Program::Paste, line, nowhere, b"");
            assert_eq!((code, err.as_str()), (0, ""), "{line}");
            assert!(out.contains("--list-types"), "{line}: {out}");
        }
        assert_eq!(
            ran(Program::Copy, "-v", nowhere, b""),
            (0, format!("wl-copy {VERSION}\n"), String::new())
        );
        assert_eq!(ran(Program::Copy, "--version", nowhere, b"").0, 0);
    }

    #[test]
    fn run_by_its_own_name_it_says_what_it_stands_in_for() {
        let nowhere = Path::new("/nonexistent");
        let (code, out, _) = ran_as("pi-clipboard-shim", "--help", nowhere, b"");
        assert_eq!(code, 0);
        assert!(out.contains("wl-paste --help"), "{out}");
        let (code, _, err) = ran_as("pi-clipboard-shim", "", nowhere, b"");
        assert_eq!(code, Exit::Usage.code());
        assert!(err.contains("wl-copy or wl-paste"), "{err}");
    }

    #[test]
    fn an_option_it_doesnt_take_exits_one_and_says_which() {
        let (code, out, err) = ran(Program::Paste, "--watch cat", Path::new("/x"), b"");
        assert_eq!((code, out.as_str()), (1, ""));
        assert!(err.contains("--watch isn't an option"), "{err}");
        assert!(err.contains("wl-paste --help"), "{err}");
    }

    #[test]
    fn a_form_other_than_text_is_refused() {
        let (code, out, err) = ran(Program::Paste, "--json", Path::new("/x"), b"");
        assert_eq!((code, out.as_str()), (1, ""));
        assert!(err.contains("only as text"), "{err}");
    }

    /// A door that hears one request and gives one answer, handing back
    /// what it heard.
    fn door(name: &str, answer: &'static [u8]) -> (PathBuf, thread::JoinHandle<Vec<u8>>) {
        let path =
            std::env::temp_dir().join(format!("pi-clipboard-shim-{name}-{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).unwrap();
        let heard = thread::spawn(move || {
            let (mut client, _) = listener.accept().unwrap();
            let mut heard = Vec::new();
            client.read_to_end(&mut heard).unwrap();
            client.write_all(answer).unwrap();
            heard
        });
        (path, heard)
    }

    #[test]
    fn a_copy_sends_stdin_less_its_newline_when_asked() {
        let (path, heard) = door("copy", b"ok\n");
        assert_eq!(
            ran(Program::Copy, "-p -n", &path, b"line\n"),
            (0, String::new(), String::new())
        );
        assert_eq!(heard.join().unwrap(), b"copy primary - always\nline");
    }

    #[test]
    fn a_copy_of_arguments_sends_them_joined() {
        let (path, heard) = door("words", b"ok\n");
        assert_eq!(ran(Program::Copy, "-o two words", &path, b"").0, 0);
        assert_eq!(heard.join().unwrap(), b"copy clipboard - once\ntwo words");
    }

    #[test]
    fn a_paste_of_text_ends_in_a_newline_unless_asked_not_to() {
        let (path, heard) = door("paste", b"ok text/plain\nhello");
        assert_eq!(
            ran(Program::Paste, "-p", &path, b""),
            (0, "hello\n".into(), String::new())
        );
        assert_eq!(heard.join().unwrap(), b"paste primary -\n");
        let (path, _) = door("paste-n", b"ok text/plain\nhello");
        assert_eq!(ran(Program::Paste, "-n", &path, b"").1, "hello");
        let (path, _) = door("paste-png", b"ok image/png\n\x89PNG");
        assert_eq!(ran(Program::Paste, "-t image", &path, b"").1, "\u{fffd}PNG");
    }

    #[test]
    fn a_listing_is_written_as_the_door_gives_it() {
        let (path, heard) = door("list", b"ok\ntext/plain\nimage/png\n");
        assert_eq!(
            ran(Program::Paste, "-l", &path, b"").1,
            "text/plain\nimage/png\n"
        );
        assert_eq!(heard.join().unwrap(), b"list clipboard\n");
    }

    #[test]
    fn a_refusal_is_said_and_exits_one() {
        let (path, _) = door("secret", b"no secret\n");
        let (status, out, err) = ran(Program::Paste, "", &path, b"");
        assert_eq!((status, out.as_str()), (1, ""));
        assert!(err.contains("password manager"), "{err}");
        let (path, _) = door("empty", b"no nothing-copied\n");
        let (status, _, err) = ran(Program::Paste, "", &path, b"");
        assert_eq!(status, 1);
        assert!(err.contains("nothing is copied"), "{err}");
    }

    #[test]
    fn a_door_not_there_exits_one() {
        let (status, _, err) = ran(Program::Copy, "x", Path::new("/nonexistent/door.sock"), b"");
        assert_eq!(status, 1);
        assert!(err.contains("can't be reached"), "{err}");
    }

    #[test]
    fn a_door_answering_nonsense_exits_one() {
        let (path, _) = door("nonsense", b"maybe\n");
        let (status, _, err) = ran(Program::Copy, "-c", &path, b"");
        assert_eq!(status, 1);
        assert!(err.contains("didn't answer"), "{err}");
    }
}
