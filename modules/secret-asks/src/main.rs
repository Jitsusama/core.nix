//! secret-asks: the release line's first experiment, one throwaway item
//! released to one fixed checker on Joel's yes in the launcher.
//!
//! - `secret-asks serve --socket SOCKET --asks DIR --vocabulary WORDS
//!   --item REF --op OP --git GIT --loginctl LOGINCTL [--account ACCOUNT]
//!   [--journal SOCKET] [--forge URL]... [--gh GH]` serves each ask in DIR
//!   as a row in Asks, and answers release and refuse (src/daemon.rs says
//!   how), telling the journal at SOCKET, journald's own by default, how
//!   each ended. A yes stands only for repositories whose origin is under
//!   a forge URL given, on that forge's main, GitHub asked with GH's token.
//! - `secret-asks ask --item REF --tree PATH --why WORDS [--asks DIR]
//!   [--given NAME] [--session NAME] [--quest-id ID --quest-title TITLE]`
//!   puts up one ask for the checker, as the harness will for an agent.
//! - `secret-asks received` is the checker: it says how many characters
//!   each variable it was given holds, and never what they hold.
//!
//! It takes the shape every command of ours does (docs/commands.md): its
//! help, usage that says what to try, and failures with named exits. What
//! it prints for a pipe is as it was, since the daemon reads the checker's
//! lines, and no help or error says an item, its vault or a reason.

use std::path::PathBuf;
use std::process::ExitCode;

use secret_asks::ask::Ask;
use secret_asks::daemon::{self, Config};
use secret_asks::reading;
use secret_asks::words::Words;
use terminal_kit::{Context, Exit, Failure, Help, flags};

/// What it does, by the first word.
const COMMANDS: [&str; 3] = ["serve", "ask", "received"];

/// The named flags each command reads, every one taking a value; the
/// first seven of serve's and the first three of ask's it can't go
/// without.
const SERVE_FLAGS: [&str; 11] = [
    "--socket",
    "--asks",
    "--vocabulary",
    "--item",
    "--op",
    "--git",
    "--loginctl",
    "--account",
    "--journal",
    "--forge",
    "--gh",
];
const ASK_FLAGS: [&str; 8] = [
    "--item",
    "--tree",
    "--why",
    "--asks",
    "--given",
    "--session",
    "--quest-id",
    "--quest-title",
];

/// What it says of itself, set as help and read by the check.
fn help() -> Help {
    Help::new(
        "secret-asks",
        "asks Joel in the launcher before releasing one throwaway secret to a checker",
    )
    .subcommand(serve_help())
    .subcommand(ask_help())
    .subcommand(received_help())
    .example(
        "secret-asks ask --item REF --tree . --why WORDS",
        "put up an ask for the checker",
    )
    .exit(
        Exit::Failed,
        "the daemon stopped, or the ask couldn't be put up",
    )
}

fn serve_help() -> Help {
    Help::new(
        "serve",
        "serves each ask as a row in Asks, releasing on a yes",
    )
    .example(
        "secret-asks serve --socket S --asks D --vocabulary W --item REF --op OP \
         --git GIT --loginctl L",
        "answer S with the asks in D",
    )
    .option("--socket PATH", "the socket to answer")
    .option("--asks DIR", "the folder the asks are put in")
    .option(
        "--vocabulary FILE",
        "the launcher's words, its rows said in",
    )
    .option("--item REF", "the one item a yes may release")
    .option("--op PATH", "1Password's command, through its wrapper")
    .option("--git PATH", "git, to read an ask's tree")
    .option("--loginctl PATH", "loginctl, to hear the screen lock")
    .option("--account NAME", "1Password's account, if not its first")
    .option("--journal PATH", "the journal's socket, if not journald's")
    .option("--forge URL", "a forge a yes may stand for, again for each")
    .option("--gh PATH", "the gh whose token asks GitHub of a main")
    .words_unsaid()
    .exit(
        Exit::Failed,
        "the vocabulary couldn't be read, or it stopped serving",
    )
}

fn ask_help() -> Help {
    Help::new("ask", "puts up one ask for the checker, and prints its id")
        .example(
            "secret-asks ask --item REF --tree . --why WORDS",
            "ask for REF to the checker, for this tree",
        )
        .option("--item REF", "the item asked for")
        .option("--tree PATH", "the tree the checker would run in")
        .option("--why WORDS", "the reason the row says")
        .option("--asks DIR", "the asks' folder, if not the runtime one")
        .option(
            "--given NAME",
            "the variable it's given in, RELEASED if not",
        )
        .option("--session NAME", "who asks, you if not")
        .option("--quest-id ID", "the quest it's for, with --quest-title")
        .option("--quest-title TITLE", "that quest's title")
        .words_unsaid()
        .exit(Exit::Failed, "the ask couldn't be put up")
}

fn received_help() -> Help {
    Help::new(
        "received",
        "the checker: each variable it's given, by length alone",
    )
    .example("secret-asks received", "what a yes runs")
    .words_unsaid()
    .exit(Exit::Failed, "it couldn't print")
}

/// Every value each named flag was given, in `known`'s order, or a usage
/// failure for a word it doesn't read or a flag left without a value.
fn given(command: &str, args: &[String], known: &[&str]) -> Result<Vec<Vec<String>>, Failure> {
    let mut given = vec![Vec::new(); known.len()];
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let Some(at) = known.iter().position(|name| name == arg) else {
            return Err(unread(command, arg, known));
        };
        let value = args.next().ok_or_else(|| {
            Failure::new(Exit::Usage, format!("{arg} needs a value"))
                .try_this(format!("secret-asks {command} --help"))
        })?;
        given[at].push(value.clone());
    }
    Ok(given)
}

/// The usage failure for a word a command doesn't read. It's named only
/// where it reads as a flag: any other may be a word of a reason left
/// unquoted, or an item given with `=`, neither of which an error says.
fn unread(command: &str, word: &str, known: &[&str]) -> Failure {
    let named = word.strip_prefix("--").is_some_and(|name| {
        !name.is_empty()
            && name
                .chars()
                .all(|letter| letter.is_ascii_lowercase() || letter == '-')
    });
    let failure = if known.is_empty() {
        Failure::new(Exit::Usage, format!("{command} takes no words"))
    } else if named {
        flags::unknown("flag", word, known)
    } else {
        Failure::new(
            Exit::Usage,
            format!("{command} takes only flags, each with its value"),
        )
        .because("a word was left over, not said here, since it may be part of a reason")
    };
    failure.try_this(format!("secret-asks {command} --help"))
}

/// The first value a flag was given, as the command has always read it.
fn first(values: &[String]) -> Option<String> {
    values.first().cloned()
}

/// The usage failure for a flag a command can't go without.
fn missing(command: &str, flag: &str) -> Failure {
    Failure::new(Exit::Usage, format!("{command} needs {flag}"))
        .try_this(format!("secret-asks {command} --help"))
}

/// The asks folder in the runtime directory, where the daemon watches.
fn default_asks() -> Option<PathBuf> {
    std::env::var_os("XDG_RUNTIME_DIR").map(|runtime| PathBuf::from(runtime).join("secret-asks"))
}

fn serve(args: &[String]) -> Result<(), Failure> {
    let given = given("serve", args, &SERVE_FLAGS)?;
    let take = |at: usize| first(&given[at]).ok_or_else(|| missing("serve", SERVE_FLAGS[at]));
    let (socket, asks, vocabulary, item) = (take(0)?, take(1)?, take(2)?, take(3)?);
    let (op, git, loginctl) = (take(4)?, take(5)?, take(6)?);
    let read = std::fs::read_to_string(&vocabulary).map_err(|why| why.to_string());
    let words = read
        .and_then(|text| Words::from_vocabulary(&text).map_err(|why| why.to_string()))
        .map_err(|why| {
            Failure::new(Exit::Failed, "it can't read the vocabulary")
                .because(format!("{vocabulary}: {why}"))
        })?;
    let checker = std::env::current_exe().map_err(|why| {
        Failure::new(Exit::Failed, "it can't say where it runs from").because(why.to_string())
    })?;
    // The daemon serves until it can't, saying why on stderr as it stops,
    // so it only ever ends as a failure, with the code it always had.
    let _stopped: ExitCode = daemon::run(Config {
        socket: socket.into(),
        asks: asks.into(),
        item,
        account: first(&given[7]),
        op: op.into(),
        git: git.into(),
        loginctl: loginctl.into(),
        checker,
        home: std::env::var("HOME").unwrap_or_default(),
        words,
        journal: first(&given[8]).map_or_else(|| reading::JOURNAL.into(), PathBuf::from),
        forges: given[9].clone(),
        gh: first(&given[10]).map(PathBuf::from),
    });
    Err(Failure::new(Exit::Failed, "it stopped serving")
        .try_this("journalctl --user -u secret-asks"))
}

/// Sixteen random bytes as 32 lowercase hex characters.
fn fresh_id() -> std::io::Result<String> {
    use std::fmt::Write;
    use std::io::Read;
    let mut bytes = [0_u8; 16];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes.iter().fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    }))
}

fn ask(context: &Context, args: &[String]) -> Result<(), Failure> {
    let given = given("ask", args, &ASK_FLAGS)?;
    let need = |at: usize| first(&given[at]).ok_or_else(|| missing("ask", ASK_FLAGS[at]));
    let (item, tree, why) = (need(0)?, need(1)?, need(2)?);
    let asks = first(&given[3])
        .map(PathBuf::from)
        .or_else(default_asks)
        .ok_or_else(|| {
            Failure::new(
                Exit::Usage,
                "ask needs --asks, since XDG_RUNTIME_DIR isn't set",
            )
            .try_this("secret-asks ask --help")
        })?;
    let quest = first(&given[6]).zip(first(&given[7]));
    let tree = std::path::absolute(&tree).unwrap_or_else(|_| tree.into());
    let now_ms = daemon::now_ms();
    let made = fresh_id().map(|id| Ask {
        id,
        asked_ms: now_ms,
        session: first(&given[5]).unwrap_or_else(|| "you".to_owned()),
        quest,
        item,
        why,
        argv: daemon::CHECKER
            .iter()
            .map(|word| (*word).to_owned())
            .collect(),
        tree: tree.display().to_string(),
        given: first(&given[4]).unwrap_or_else(|| "RELEASED".to_owned()),
        minutes: 1,
    });
    let id = made
        .and_then(|ask| {
            daemon::write_whole(&asks.join(format!("{}.ask.json", ask.id)), &ask.said())?;
            Ok(ask.id)
        })
        .map_err(|why| {
            Failure::new(Exit::Failed, "it can't put the ask up")
                .because(format!("{}: {why}", asks.display()))
        })?;
    context.say(&format!("{id}\n")).map_err(|why| {
        Failure::new(Exit::Failed, "it couldn't print the ask's id").because(why.to_string())
    })
}

/// What the checker says of its variables: each but the few every command
/// is given, by name and length only, or that it was given nothing.
fn lengths(variables: impl Iterator<Item = (std::ffi::OsString, std::ffi::OsString)>) -> String {
    use std::fmt::Write;
    let said = variables
        .filter(|(name, _)| !["PATH", "HOME", "LANG"].iter().any(|kept| name == *kept))
        .fold(String::new(), |mut said, (name, value)| {
            let length = value.to_string_lossy().chars().count();
            // Writing to a String can't fail.
            let _ = writeln!(
                said,
                "{}: received {length} characters",
                name.to_string_lossy()
            );
            said
        });
    if said.is_empty() {
        "received nothing\n".to_owned()
    } else {
        said
    }
}

/// The checker, saying its variables' lengths and never what they hold.
fn received(context: &Context, args: &[String]) -> Result<(), Failure> {
    if let Some(word) = args.first() {
        return Err(unread("received", word, &[]));
    }
    context
        .say(&lengths(std::env::vars_os()))
        .map_err(|why| Failure::new(Exit::Failed, "it couldn't print").because(why.to_string()))
}

/// The command's body, by its first word.
fn body(context: &Context) -> Result<(), Failure> {
    match context.args.split_first() {
        Some((command, rest)) if command == "serve" => serve(rest),
        Some((command, rest)) if command == "ask" => ask(context, rest),
        Some((command, rest)) if command == "received" => received(context, rest),
        Some((command, _)) => Err(flags::unknown("command", command, &COMMANDS)),
        None => Err(Failure::new(Exit::Usage, "it takes serve, ask or received")
            .try_this("secret-asks --help")),
    }
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use terminal_kit::{Ending, Environment};

    /// A line run through the command's own body, as main runs it. Each
    /// case here is refused before anything is written or said.
    fn ran(line: &[&str]) -> Ending {
        ran_in(&Environment::of([false; 3], &[]), line)
    }

    fn ran_in(environment: &Environment, line: &[&str]) -> Ending {
        terminal_kit::run(
            &help(),
            "0.1.0",
            environment,
            line.iter().map(|word| (*word).to_owned()).collect(),
            body,
        )
    }

    /// The same line refused into each place its error can land: a pipe, a
    /// terminal, the journal as a unit's stderr, and JSON for a machine.
    fn refused_everywhere(line: &[&str]) -> Vec<(&'static str, Ending)> {
        let mut journal = Environment::of([false; 3], &[]);
        journal.journal = true;
        let json = [line, &["--format", "json"]].concat();
        vec![
            ("a pipe", ran(line)),
            (
                "a terminal",
                ran_in(&Environment::of([true; 3], &[("TERM", "xterm")]), line),
            ),
            ("the journal", ran_in(&journal, line)),
            ("JSON", ran(&json)),
        ]
    }

    #[test]
    fn no_refusal_says_a_word_it_was_given_wherever_it_lands() {
        let item = "op://Vault/Hidden/password";
        for line in [
            &[
                "ask",
                "--item",
                item,
                "--tree",
                ".",
                "--why",
                "To",
                "lighthouse",
            ][..],
            &["serve", "--item", item, "lighthouse"],
            &["received", "lighthouse"],
            &["ask", "--item=op://Vault/Hidden/password"],
        ] {
            for (place, ending) in refused_everywhere(line) {
                assert_eq!(
                    ending.exit,
                    Exit::Usage,
                    "{line:?} into {place} wasn't refused"
                );
                for word in ["lighthouse", "Hidden", "Vault"] {
                    assert!(
                        !ending.stderr.contains(word) && !ending.stdout.contains(word),
                        "{line:?}'s refusal into {place} said {word}: {}",
                        ending.stderr
                    );
                }
            }
        }
    }

    #[test]
    fn a_mistyped_flag_is_named_with_the_one_meant() {
        let ending = ran(&["ask", "--itme", "x"]);
        assert_eq!(ending.exit, Exit::Usage, "a mistyped flag wasn't refused");
        assert!(
            ending.stderr.contains("--itme") && ending.stderr.contains("--item"),
            "the error didn't name the flag and the one meant: {}",
            ending.stderr
        );
    }

    #[test]
    fn the_checker_says_each_length_and_never_a_value() {
        let given = [
            ("PATH", "/bin"),
            ("RELEASED", "hunter2"),
            ("HOME", "/home/someone"),
            ("LANG", "C"),
        ]
        .map(|(name, value)| (OsString::from(name), OsString::from(value)));
        assert_eq!(
            lengths(given.into_iter()),
            "RELEASED: received 7 characters\n",
            "the checker didn't say the one length alone"
        );
        assert_eq!(
            lengths(std::iter::empty()),
            "received nothing\n",
            "the checker given nothing didn't say so"
        );
    }

    #[test]
    fn no_help_says_an_item_or_a_vault() {
        for line in [
            &["--help", "--format", "json"][..],
            &["serve", "--help"],
            &["ask", "--help"],
            &["received", "--help"],
        ] {
            let ending = ran(line);
            assert_eq!(ending.exit, Exit::Done, "{line:?} wasn't answered");
            assert!(
                !ending.stdout.contains("op://"),
                "{line:?} named an item: {}",
                ending.stdout
            );
        }
    }
}
