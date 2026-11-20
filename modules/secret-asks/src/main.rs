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
//!
//! Long because its tests try each way a word given could be said back,
//! into each of the four places a refusal lands, beside the line reading
//! they guard.

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
    .words_unsaid()
    .exit(
        Exit::Failed,
        "the daemon stopped, or the ask couldn't be put up or its id printed",
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
        .example(
            "secret-asks ask -- --item REF --tree . --why --help",
            "the same, for a reason that is a flag every command reads",
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
        .exit(
            Exit::Failed,
            "the ask couldn't be put up, or its id printed",
        )
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
        // `--` only ends the kit's shared flags, so a reason that is
        // exactly one, as `--why --help`, can be given after it.
        if arg == "--" {
            continue;
        }
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

/// The usage failure for a word a command doesn't read, which it never
/// says: it may be a word of a reason left unquoted, or an item given
/// with `=`. A dash word is answered with the flag nearest it.
fn unread(command: &str, word: &str, known: &[&str]) -> Failure {
    let failure = if known.is_empty() {
        Failure::new(Exit::Usage, format!("{command} takes no words"))
    } else if let Some(flag) = word
        .split_once('=')
        .and_then(|(flag, _)| known.iter().find(|known| **known == flag))
    {
        Failure::new(
            Exit::Usage,
            format!("{flag} takes its value as the next word"),
        )
        .because("a value after = isn't read, and isn't said here")
    } else if word.starts_with('-') {
        flags::unknown_unsaid("flag", word, known)
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

/// What serve was told, each flag in its own field.
#[derive(Debug, PartialEq)]
struct Served {
    socket: String,
    asks: String,
    vocabulary: String,
    item: String,
    op: String,
    git: String,
    loginctl: String,
    account: Option<String>,
    journal: Option<String>,
    forges: Vec<String>,
    gh: Option<String>,
}

/// serve's flags, each read from its place in `SERVE_FLAGS`.
fn served(args: &[String]) -> Result<Served, Failure> {
    let given = given("serve", args, &SERVE_FLAGS)?;
    let take = |at: usize| first(&given[at]).ok_or_else(|| missing("serve", SERVE_FLAGS[at]));
    Ok(Served {
        socket: take(0)?,
        asks: take(1)?,
        vocabulary: take(2)?,
        item: take(3)?,
        op: take(4)?,
        git: take(5)?,
        loginctl: take(6)?,
        account: first(&given[7]),
        journal: first(&given[8]),
        forges: given[9].clone(),
        gh: first(&given[10]),
    })
}

fn serve(args: &[String]) -> Result<(), Failure> {
    let served = served(args)?;
    let vocabulary = &served.vocabulary;
    let read = std::fs::read_to_string(vocabulary).map_err(|why| why.to_string());
    let words = read
        .and_then(|text| Words::from_vocabulary(&text).map_err(|why| why.to_string()))
        .map_err(|why| {
            Failure::new(Exit::Failed, "it can't read the vocabulary")
                .because(format!("{vocabulary}: {why}"))
        })?;
    let checker = std::env::current_exe().map_err(|why| {
        Failure::new(Exit::Failed, "it can't say where it runs from").because(why.to_string())
    })?;
    // The daemon serves until it can't, so it only ever ends as a failure,
    // with the code it always had. It says why on stderr where it can,
    // and nothing where its watch on the folder simply ends.
    let _stopped: ExitCode = daemon::run(Config {
        socket: served.socket.into(),
        asks: served.asks.into(),
        item: served.item,
        account: served.account,
        op: served.op.into(),
        git: served.git.into(),
        loginctl: served.loginctl.into(),
        checker,
        home: std::env::var("HOME").unwrap_or_default(),
        words,
        journal: served
            .journal
            .map_or_else(|| reading::JOURNAL.into(), PathBuf::from),
        forges: served.forges,
        gh: served.gh.map(PathBuf::from),
    });
    Err(Failure::new(Exit::Failed, "it stopped serving")
        .because("its earlier lines in the journal say why, or its watch on the asks folder ended"))
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

fn ask(context: &Context, args: &[String], out: &mut dyn std::io::Write) -> Result<(), Failure> {
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
    let json = serde_json::json!({ "schema": "secret-asks.asked/1", "id": id });
    answered(context, out, &format!("{id}\n"), &json).map_err(|why| {
        Failure::new(Exit::Failed, "it couldn't print the ask's id").because(why.to_string())
    })
}

/// The answer in the form asked for: the text a pipe has always had, or
/// JSON under `--json` and `--format jsonl`.
fn answered(
    context: &Context,
    out: &mut dyn std::io::Write,
    text: &str,
    json: &serde_json::Value,
) -> std::io::Result<()> {
    if context.format().is_json() {
        context.answer_to(out, json)
    } else {
        write!(out, "{text}")
    }
}

/// Each variable the checker was given but the few every command is, by
/// name and length only.
fn counted(
    variables: impl Iterator<Item = (std::ffi::OsString, std::ffi::OsString)>,
) -> Vec<(String, usize)> {
    variables
        .filter(|(name, _)| !["PATH", "HOME", "LANG"].iter().any(|kept| name == *kept))
        .map(|(name, value)| {
            let length = value.to_string_lossy().chars().count();
            (name.to_string_lossy().into_owned(), length)
        })
        .collect()
}

/// What the checker says of its variables as text, or that it was given
/// nothing.
fn lengths(counted: &[(String, usize)]) -> String {
    use std::fmt::Write;
    if counted.is_empty() {
        return "received nothing\n".to_owned();
    }
    counted
        .iter()
        .fold(String::new(), |mut said, (name, length)| {
            // Writing to a String can't fail.
            let _ = writeln!(said, "{name}: received {length} characters");
            said
        })
}

/// What the checker says of its variables as JSON: a list, in the order
/// it was given them, of each one's name and length.
fn lengths_json(counted: &[(String, usize)]) -> serde_json::Value {
    let received: Vec<serde_json::Value> = counted
        .iter()
        .map(|(name, length)| serde_json::json!({ "name": name, "characters": length }))
        .collect();
    serde_json::json!({ "schema": "secret-asks.received/1", "received": received })
}

/// The checker, saying the lengths of the variables it was given and
/// never what they hold.
fn received(
    context: &Context,
    args: &[String],
    variables: impl Iterator<Item = (std::ffi::OsString, std::ffi::OsString)>,
    out: &mut dyn std::io::Write,
) -> Result<(), Failure> {
    if let Some(word) = args.first() {
        return Err(unread("received", word, &[]));
    }
    let counted = counted(variables);
    answered(context, out, &lengths(&counted), &lengths_json(&counted))
        .map_err(|why| Failure::new(Exit::Failed, "it couldn't print").because(why.to_string()))
}

/// The command's body, by its first word, answering on stdout. Unlocked,
/// since serve never returns and a lock held for it would stop any later
/// write there.
fn body(context: &Context) -> Result<(), Failure> {
    body_to(context, &mut std::io::stdout())
}

/// As `body`, answering into `out`, as the tests read what each command
/// really answers.
fn body_to(context: &Context, out: &mut dyn std::io::Write) -> Result<(), Failure> {
    match context.args.split_first() {
        Some((command, rest)) if command == "serve" => serve(rest),
        Some((command, rest)) if command == "ask" => ask(context, rest, out),
        Some((command, rest)) if command == "received" => {
            received(context, rest, std::env::vars_os(), out)
        }
        Some((command, _)) => {
            Err(flags::unknown_unsaid("command", command, &COMMANDS).try_this("secret-asks --help"))
        }
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

    /// A line run through the command's own body, as main runs it, into a
    /// pipe. What the body prints goes to the test's own stdout, so an
    /// ask's id is found by the file it put up.
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
        // First, since a shared flag after `--` is the command's.
        let json = [&["--format", "json"][..], line].concat();
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
            // A first word that's no command, read by the walk and by the
            // body, as an item given with `=` before any command is.
            &["lighthouse"],
            &["--item=op://Vault/Hidden/password"],
            // A reason's dash word, read by the walk before a word left
            // over, and by the body as the last word.
            &["ask", "--why", "To", "--lighthouse", "Hidden"],
            &["ask", "--item", item, "--lighthouse"],
            // A reason's word read as a shared flag's value.
            &["ask", "--why", "To", "--format", "lighthouse"],
            &["ask", "--why", "To", "--color=lighthouse"],
            // A word left over after `--`, which the body reads past.
            &["ask", "--", "--why", "lighthouse", "Hidden"],
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
    fn a_mistyped_flag_is_answered_with_the_one_meant_and_never_said() {
        // The walk reads a dash word before a word left over, and the body
        // one at the end; both offer the flag meant and never say the one
        // given.
        for line in [&["ask", "--itme", "x"][..], &["ask", "--itme"]] {
            let ending = ran(line);
            assert_eq!(ending.exit, Exit::Usage, "{line:?} wasn't refused");
            assert!(
                ending.stderr.contains("did you mean --item?") && !ending.stderr.contains("--itme"),
                "{line:?} didn't offer the flag meant, or said the one given: {}",
                ending.stderr
            );
        }
    }

    /// An ask's line run into a folder of its own, with the ask it put up
    /// there, when it put up one that reads back.
    fn put_up(purpose: &str, line: &[&str]) -> (Ending, Option<Ask>) {
        let asks =
            std::env::temp_dir().join(format!("secret-asks-{purpose}-{}", std::process::id()));
        // A folder an earlier run left is cleared, so only this run's ask
        // is found.
        let _ = std::fs::remove_dir_all(&asks);
        std::fs::create_dir_all(&asks).expect("the asks folder couldn't be made");
        let folder = asks.display().to_string();
        let ending = ran(&[line, &["--asks", folder.as_str()]].concat());
        let put: Vec<_> = std::fs::read_dir(&asks)
            .expect("the asks folder couldn't be read")
            .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
            .collect();
        let read = |name: &String| {
            let id = secret_asks::ask::id_of(name)?.to_owned();
            let text = std::fs::read_to_string(asks.join(name)).ok()?;
            Ask::read(&id, &text).ok()
        };
        let ask = match put.as_slice() {
            [name] => read(name),
            _ => None,
        };
        let _ = std::fs::remove_dir_all(&asks);
        (ending, ask)
    }

    #[test]
    fn an_ask_keeps_each_flag_in_its_own_field() {
        let (ending, ask) = put_up(
            "fields",
            &[
                "ask",
                "--quest-title",
                "a quest",
                "--item",
                "op://V/I/p",
                "--session",
                "a session",
                "--tree",
                "/t",
                "--given",
                "GIVEN",
                "--why",
                "a reason",
                "--quest-id",
                "Q1",
            ],
        );
        assert_eq!(
            ending.exit,
            Exit::Done,
            "the ask wasn't put up: {}",
            ending.stderr
        );
        let ask = ask.expect("not one ask that reads back was put up");
        assert_eq!(
            [&ask.item, &ask.tree, &ask.why, &ask.given, &ask.session],
            ["op://V/I/p", "/t", "a reason", "GIVEN", "a session"],
            "a flag landed in another's field"
        );
        assert_eq!(
            ask.quest,
            Some(("Q1".to_owned(), "a quest".to_owned())),
            "the quest's id and title were swapped or lost"
        );
    }

    #[test]
    fn a_reason_that_is_a_shared_flag_is_kept_after_the_line_end() {
        let (ending, ask) = put_up(
            "after-the-end",
            &[
                "ask",
                "--",
                "--item",
                "op://V/I/p",
                "--tree",
                "/t",
                "--why",
                "--help",
            ],
        );
        assert_eq!(
            ending.exit,
            Exit::Done,
            "the ask wasn't put up: {}",
            ending.stderr
        );
        assert_eq!(
            ask.map(|ask| ask.why).as_deref(),
            Some("--help"),
            "the reason after -- wasn't kept whole"
        );
    }

    #[test]
    fn serve_keeps_each_flag_in_its_own_field() {
        let line = [
            "--gh",
            "/gh",
            "--forge",
            "f1",
            "--account",
            "acct",
            "--loginctl",
            "/lc",
            "--journal",
            "/j",
            "--git",
            "/git",
            "--op",
            "/op",
            "--item",
            "op://V/I/p",
            "--vocabulary",
            "/v",
            "--asks",
            "/a",
            "--forge",
            "f2",
            "--socket",
            "/s",
        ]
        .map(str::to_owned);
        let owned = str::to_owned;
        assert_eq!(
            served(&line).ok(),
            Some(Served {
                socket: owned("/s"),
                asks: owned("/a"),
                vocabulary: owned("/v"),
                item: owned("op://V/I/p"),
                op: owned("/op"),
                git: owned("/git"),
                loginctl: owned("/lc"),
                account: Some(owned("acct")),
                journal: Some(owned("/j")),
                forges: vec![owned("f1"), owned("f2")],
                gh: Some(owned("/gh")),
            }),
            "a flag landed in another's field"
        );
    }

    #[test]
    fn a_line_missing_a_flag_or_its_value_ends_as_asked_wrongly() {
        for line in [
            &["ask", "--item"][..],
            &["ask", "--tree", "/t", "--why", "w"],
            &["serve", "--socket", "/s"],
            &["serve"],
        ] {
            let ending = ran(line);
            assert_eq!(
                ending.code(),
                64,
                "{line:?} didn't end as asked wrongly: {}",
                ending.stderr
            );
        }
    }

    #[test]
    fn an_item_given_with_an_equals_is_told_how_flags_take_values() {
        let ending = ran(&["ask", "--item=op://Vault/Hidden/password"]);
        assert!(
            ending
                .stderr
                .contains("--item takes its value as the next word")
                && !ending.stderr.contains("Hidden"),
            "an item given with = wasn't told what's wrong, or was said: {}",
            ending.stderr
        );
    }

    #[test]
    fn an_ask_answers_the_id_of_the_ask_it_put_up_in_each_form() {
        // Each form puts up its ask in a folder of its own, so the id it
        // answers can only be that one ask's.
        let asked = |form: &str, shared: &[&str]| -> (String, Vec<String>) {
            let asks = std::env::temp_dir()
                .join(format!("secret-asks-answer-{form}-{}", std::process::id()));
            // A folder an earlier run left is cleared, so only this ask
            // is found.
            let _ = std::fs::remove_dir_all(&asks);
            std::fs::create_dir_all(&asks).expect("the asks folder couldn't be made");
            let folder = asks.display().to_string();
            let line = ["ask", "--item", "op://V/I/p", "--tree", "/t", "--why", "w"];
            let line = [shared, &line[..], &["--asks", folder.as_str()]].concat();
            let mut out = Vec::new();
            body_to(&context_of(&line), &mut out).expect("the ask wasn't put up");
            let ids = std::fs::read_dir(&asks)
                .expect("the asks folder couldn't be read")
                .filter_map(|entry| {
                    let name = entry.ok()?.file_name().into_string().ok()?;
                    secret_asks::ask::id_of(&name).map(str::to_owned)
                })
                .collect();
            let _ = std::fs::remove_dir_all(&asks);
            (String::from_utf8(out).expect("the answer wasn't text"), ids)
        };
        let (piped, ids) = asked("pipe", &[]);
        let [id] = ids.as_slice() else {
            panic!("not one ask was put up into a pipe: {ids:?}");
        };
        assert_eq!(piped, format!("{id}\n"), "a pipe didn't get the id alone");
        for (form, shared, kind) in [
            ("json", &["--json"][..], None),
            ("jsonl", &["--format", "jsonl"][..], Some("answer")),
        ] {
            let (said, ids) = asked(form, shared);
            let [id] = ids.as_slice() else {
                panic!("not one ask was put up under {form}: {ids:?}");
            };
            let mut meant = serde_json::json!({ "schema": "secret-asks.asked/1", "id": id });
            if let Some(kind) = kind {
                meant["type"] = kind.into();
            }
            let said: serde_json::Value =
                serde_json::from_str(&said).expect("the answer wasn't one JSON line");
            assert_eq!(said, meant, "{form} didn't answer the ask's id alone");
        }
    }

    /// A context for `line`, its shared flags taken as main takes them.
    fn context_of(line: &[&str]) -> Context {
        let args = line.iter().map(|word| (*word).to_owned()).collect();
        Context::new(Environment::of([false; 3], &[]), args, &[])
            .expect("the shared flags weren't taken")
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
        let answered = |line: &[&str]| {
            let mut out = Vec::new();
            received(&context_of(line), &[], given.clone().into_iter(), &mut out)
                .expect("the checker couldn't answer");
            String::from_utf8(out).expect("the answer wasn't text")
        };
        assert_eq!(
            answered(&[]),
            "RELEASED: received 7 characters\n",
            "the checker didn't say the one length alone"
        );
        let said = answered(&["--json"]);
        assert!(
            !said.contains("hunter2"),
            "the checker said a value: {said}"
        );
        assert_eq!(
            lengths(&[]),
            "received nothing\n",
            "the checker given nothing didn't say so"
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&said).expect("the answer wasn't JSON"),
            serde_json::json!({
                "schema": "secret-asks.received/1",
                "received": [{ "name": "RELEASED", "characters": 7 }],
            }),
            "the checker's JSON isn't a list of names and lengths"
        );
    }

    #[test]
    fn each_command_reads_the_flags_its_help_names() {
        for (help, flags) in [
            (serve_help(), &SERVE_FLAGS[..]),
            (ask_help(), &ASK_FLAGS[..]),
        ] {
            let named: Vec<&str> = help
                .options
                .iter()
                .filter_map(|option| option.name.split_whitespace().next())
                .collect();
            assert_eq!(named, flags, "{}'s help and its flags disagree", help.name);
            // The walk takes a flag's next word as its value only when its
            // help names one, so each must, as every flag here takes one.
            for option in &help.options {
                assert!(
                    option.name.split_whitespace().nth(1).is_some(),
                    "{}'s help gives {} no value",
                    help.name,
                    option.name
                );
            }
        }
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
