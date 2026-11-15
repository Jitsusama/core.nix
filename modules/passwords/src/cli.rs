//! The `passwords` command line, in the shape every command of ours takes
//! (docs/commands.md) from the terminal kit, which reads the shared flags,
//! answers `--help`, and says a failure in its three parts with an exit
//! from the table. What's typed is read here ([`parse`]); the keys
//! 1Password's SSH agent holds are read and shown in `keys`.
//!
//! It reaches no daemon, and so no entry and no secret: the daemon answers
//! the shell alone, which runs it as its own child, since every agent runs
//! as Joel and could ask whatever the command could (2026-11-10). Each
//! command that reached an entry before is refused as asked wrongly,
//! naming nothing but itself, and says where passwords are now. A password
//! is filled by the manager's own browser extension, or read in its own
//! app or its own command, each of which asks Joel itself.

mod keys;

use std::io;

use terminal_kit::{Context, Exit, Failure, Help, flags};

/// One thing the command was asked to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// The daemon, which the shell runs as its own child.
    Daemon {
        managers: String,
        /// The launcher's vocabulary, which its rows are said in.
        words: Option<String>,
    },
    /// What a bare run shows, having no state of its own to show.
    Brief,
    Keys,
    /// zsh's completion itself.
    Completion,
}

/// The commands that reached an entry or a secret through the daemon's
/// socket, before the daemon answered the shell alone, each refused by
/// name so whoever types one learns why rather than being offered the
/// nearest command left.
const GONE: [&str; 10] = [
    "find", "list", "details", "show", "copy", "code", "let-in", "health", "audit", "names",
];

/// The command line's help, which the kit answers `--help` with, each
/// command's own help in it, so the kit answers `passwords keys --help`
/// and refuses a word no command answers to before `parse` reads it.
/// The commands that are gone are declared unlisted and taking any
/// words, so the kit hands them to `parse` whole, which refuses each by
/// name: the kit's own refusal of a word too many would say it back.
pub fn help() -> Help {
    let help = Help::new(
        "passwords",
        "1Password's SSH keys; passwords are in the launcher alone",
    )
    .example("passwords keys", "the keys 1Password's SSH agent holds")
    .example(
        "passwords keys --json",
        "the same for a program, as passwords.keys/1",
    )
    .subcommand(
        Help::new(
            "keys",
            "the keys 1Password's SSH agent holds, which ssh and git don't use",
        )
        .example("passwords keys", "each key's type, size and fingerprint")
        .example(
            "passwords keys --json",
            "the same for a program, as passwords.keys/1",
        )
        .exit(Exit::Failed, "the agent didn't list its keys")
        .exit(Exit::Unavailable, "1Password's SSH agent isn't answering"),
    )
    .subcommand(
        Help::new("completion", "zsh's completion")
            .argument("zsh", "the shell it's written for, the only one")
            .example("passwords completion zsh", "the completion zsh loads")
            .exit(Exit::Usage, "asked for a shell but zsh"),
    )
    .subcommand(
        Help::new(
            "daemon",
            "the daemon holding each entry, which the shell runs as its own child",
        )
        .argument("MANAGERS", "the managers it reads entries from")
        .argument(
            "[WORDS]",
            "the launcher's words, which its rows are said in",
        )
        .example(
            "passwords daemon MANAGERS WORDS",
            "as the shell runs it, answering on its own pipe",
        )
        .exit(Exit::Failed, "it couldn't keep its memory to itself")
        .exit(Exit::Config, "the managers can't be read"),
    );
    GONE.iter()
        .fold(help, |help, command| help.subcommand(gone_help(command)))
        .exit(Exit::Failed, "the agent didn't list its keys")
        .exit(
            Exit::Usage,
            "asked wrongly, or for an entry, which no command reaches",
        )
        .exit(Exit::Unavailable, "1Password's SSH agent isn't answering")
        .exit(Exit::Config, "daemon: the managers can't be read")
}

/// The help of a command that reached an entry before, which no command
/// does now.
fn gone_help(command: &str) -> Help {
    Help::new(
        command,
        "reads no entry any more; passwords are in the launcher",
    )
    .argument(
        "WORDS...",
        "read by nothing, since no command reaches an entry",
    )
    .exit(Exit::Usage, "always, since every agent runs as Joel")
    .unlisted()
    .never_done()
}

/// The command line asked wrongly: what was wrong, and where to read how.
fn asked(what: impl Into<String>) -> Failure {
    Failure::new(Exit::Usage, what).try_this("passwords --help")
}

/// A command that reached an entry before, refused naming only itself,
/// never what was typed after it, which may be an entry's title.
fn gone(command: &str) -> Failure {
    Failure::new(
        Exit::Usage,
        format!("passwords {command} reads no entry any more"),
    )
    .because("no command reaches an entry or a secret, since every agent runs as Joel")
    .try_this("the launcher's Passwords place")
}

/// The arguments the kit left after the shared flags, read into a
/// command.
pub fn parse(args: &[String]) -> Result<Command, Failure> {
    let words: Vec<&str> = args
        .iter()
        .map(String::as_str)
        .filter(|word| *word != "--")
        .collect();
    let Some((first, rest)) = words.split_first() else {
        return Ok(Command::Brief);
    };
    let none = |command: Command| {
        if rest.is_empty() {
            Ok(command)
        } else {
            Err(asked(format!("{first} takes nothing after it")))
        }
    };
    let command = match *first {
        "daemon" => match rest {
            [managers, words @ ..] if words.len() <= 1 => Command::Daemon {
                managers: (*managers).to_owned(),
                words: words.first().map(|words| (*words).to_owned()),
            },
            _ => {
                return Err(
                    asked("daemon needs a MANAGERS file, then the launcher's WORDS")
                        .because("the shell runs it"),
                );
            }
        },
        "keys" => none(Command::Keys)?,
        "completion" => match rest {
            ["zsh"] => Command::Completion,
            _ => {
                return Err(asked("completion is written for zsh alone").because("it takes zsh"));
            }
        },
        other if GONE.contains(&other) => return Err(gone(other)),
        other => {
            let help = help();
            return Err(flags::unknown("command", other, &help.command_names())
                .try_this("passwords --help"));
        }
    };
    Ok(command)
}

/// What was written, a stream closed early, as a pipe's reader that has
/// what it wanted closes it, being nothing left to say.
fn written(said: io::Result<()>) -> Result<(), Failure> {
    match said {
        Err(why) if why.kind() != io::ErrorKind::BrokenPipe => {
            Err(Failure::new(Exit::Failed, "couldn't write the answer").because(why.to_string()))
        }
        _ => Ok(()),
    }
}

/// Answers one command but the daemon, which `main` runs itself.
pub fn run(context: &Context, command: &Command) -> Result<(), Failure> {
    match command {
        Command::Brief => written(context.say(&help().brief(&context.out, context.room))),
        Command::Completion => written(context.say(include_str!("completion.zsh"))),
        Command::Keys => keys::run(context),
        Command::Daemon { .. } => Err(Failure::new(
            Exit::Bug,
            "the daemon was asked of the command line",
        )),
    }
}

#[cfg(test)]
mod tests {
    use terminal_kit::layout::MEASURE;
    use terminal_kit::{Pen, Room};

    use super::*;

    fn parsed(words: &str) -> Result<Command, Failure> {
        let args: Vec<String> = words.split_whitespace().map(str::to_owned).collect();
        parse(&args)
    }

    fn command(words: &str) -> Command {
        parsed(words).unwrap_or_else(|failure| panic!("{words:?} wasn't read: {failure}"))
    }

    fn refused(words: &str) -> Failure {
        parsed(words).expect_err(&format!("{words:?} was read"))
    }

    #[test]
    fn each_command_is_read_with_what_it_takes() {
        assert_eq!(command(""), Command::Brief);
        assert_eq!(command("keys"), Command::Keys);
        assert_eq!(command("completion zsh"), Command::Completion);
        assert_eq!(
            command("daemon managers.json"),
            Command::Daemon {
                managers: "managers.json".to_owned(),
                words: None
            }
        );
        assert_eq!(
            command("daemon managers.json words.json"),
            Command::Daemon {
                managers: "managers.json".to_owned(),
                words: Some("words.json".to_owned())
            }
        );
    }

    #[test]
    fn each_entry_command_is_refused_naming_only_itself() {
        for words in [
            "find git",
            "list",
            "details GitHub",
            "show GitHub",
            "copy GitHub password",
            "code GitHub",
            "let-in 1password",
            "health",
            "audit",
            "names entries",
        ] {
            let failure = refused(words);
            let first = words.split_whitespace().next().unwrap_or_default();
            assert_eq!(
                failure.exit,
                Exit::Usage,
                "{words:?} ended {:?}",
                failure.exit
            );
            assert_eq!(
                failure.what,
                format!("passwords {first} reads no entry any more"),
                "{words:?} said what it didn't need to"
            );
            let said = format!("{failure:?}");
            for named in ["GitHub", "git ", "1password", "entries"] {
                assert!(
                    !said.contains(named),
                    "{words:?} said {named:?} back: {said}"
                );
            }
        }
    }

    #[test]
    fn a_mistyped_command_says_what_it_needs() {
        for (words, said) in [
            ("dance", "no command called dance"),
            ("kyes", "no command called kyes"),
            ("keys everything", "keys takes nothing after it"),
            ("completion bash", "zsh alone"),
            ("daemon", "daemon needs"),
            ("daemon a b c", "daemon needs"),
        ] {
            let failure = refused(words);
            assert_eq!(
                failure.exit,
                Exit::Usage,
                "{words:?} ended {:?}",
                failure.exit
            );
            assert!(
                failure.what.contains(said),
                "{words:?} said {:?}",
                failure.what
            );
        }
        assert_eq!(
            refused("kyes").why.as_deref(),
            Some("did you mean keys?"),
            "a near miss wasn't offered the command it missed"
        );
    }

    #[test]
    fn the_help_fits_sixty_columns_and_says_each_exit() {
        let help = help();
        for each in std::iter::once(&help).chain(&help.subs) {
            for width in [60, MEASURE] {
                let said = each.text(&Pen::plain(), Room::Cells(width));
                let wide = said.lines().find(|line| line.chars().count() > width);
                assert_eq!(
                    wide, None,
                    "a line of {}'s help passes {width} columns",
                    each.name
                );
            }
        }
        let exits: Vec<u8> = help
            .every_exit()
            .iter()
            .map(|(exit, _)| exit.code())
            .collect();
        assert_eq!(
            exits,
            [0, 1, 64, 69, 78],
            "the help's exits aren't the ones the command ends with"
        );
        let brief = help.brief(&Pen::plain(), Room::Cells(60));
        assert!(
            brief.contains("passwords keys") && brief.contains("passwords --help"),
            "a bare run doesn't say what to type: {brief}"
        );
    }

    #[test]
    fn a_command_that_reads_no_entry_lists_only_its_refusal() {
        // Each always ends 64, so a 0 among its exits would be untrue.
        let help = help();
        for command in GONE {
            let gone = help
                .subs
                .iter()
                .find(|sub| sub.name == *command)
                .unwrap_or_else(|| panic!("{command} has no help"));
            let exits: Vec<u8> = gone
                .every_exit()
                .iter()
                .map(|(exit, _)| exit.code())
                .collect();
            assert_eq!(
                exits,
                [64],
                "{command}'s help lists an exit it never ends with"
            );
        }
    }
}
