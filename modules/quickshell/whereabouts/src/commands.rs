//! location's words as the terminal kit takes them: its help, and the
//! start they ask for. Its unit starts the daemon with its socket first,
//! so the daemon's words are the root's own names, and `set` and `again`
//! are subcommands left out of the tree's listing, the one kind the kit
//! matches before a name (docs/commands.md, Subcommands), so a unit's
//! words keep working and each still answers a help of its own. The root
//! lists them as commands, so its help and commands-agree both find them.

use std::path::PathBuf;

use terminal_kit::{Exit, Failure, Help, flags};

/// What location and its two commands say they are, do and end with, as
/// `--help` shows it.
pub fn help() -> Help {
    Help::new(
        "location",
        "holds where the laptop is, and the clock's zone",
    )
    .example("location location.sock /", "serves only the clock's zone")
    .example(
        "location set pin.json Bergen",
        "pins here to Bergen, by hand",
    )
    .argument("SOCKET", "where it answers, which systemd holds")
    .argument("ROOT", "the root whose etc/localtime is the zone")
    .argument("[KEPT]", "where here is kept through a restart")
    .argument("[PIN]", "where Joel's pin is kept")
    .argument("[MOVED]", "where clock-follows says it moved")
    .option("--words FILE", "the launcher's words, for its rows")
    .command("set PIN WORDS...", "pins here to the town named")
    .command("again PIN", "takes the pin away")
    .subcommand(set().unlisted())
    .subcommand(again().unlisted())
    .exit(Exit::Failed, "it couldn't listen, or hear the pin")
}

fn set() -> Help {
    Help::new("set", "pins here to the town named, found by the web")
        .example(
            "location set pin.json Bergen, Norway",
            "a town and its country",
        )
        .argument("PIN", "the pin's file, in the state folder")
        .argument(
            "WORDS...",
            "the town, as typed, after -- if it opens with -",
        )
        .exit(Exit::NotFound, "no place is called what was typed")
        .exit(Exit::Unavailable, "the place couldn't be asked about")
        .exit(Exit::Failed, "the pin couldn't be kept")
}

fn again() -> Help {
    Help::new("again", "takes the pin away, so GeoClue finds here")
        .example("location again pin.json", "as Find It Again does")
        .argument("PIN", "the pin's file, in the state folder")
        .exit(Exit::Failed, "the pin couldn't be taken away")
}

/// What location's words ask it to do.
#[derive(Debug, PartialEq)]
pub enum Start {
    /// Pin here to the town the words name.
    Set { pin: PathBuf, town: String },
    /// Take the pin away.
    Again { pin: PathBuf },
    /// Answer on the socket: the clock's zone alone, or following the
    /// laptop too, given its kept here, its pin and the clock's move.
    Serve {
        socket: PathBuf,
        root: PathBuf,
        following: Option<[PathBuf; 3]>,
        words: Option<PathBuf>,
    },
}

/// The start location's words ask for.
///
/// # Errors
///
/// Asked wrongly, 64, naming the help of the command the words name: a
/// flag it doesn't take is named with the nearest it does.
pub fn start(args: &[String]) -> Result<Start, Failure> {
    // Words after `--` are the command's own, never flags, so a town
    // opening with a dash can be set.
    let (args, after) = match args.iter().position(|arg| arg == "--") {
        Some(at) => (&args[..at], &args[at + 1..]),
        None => (args, &[][..]),
    };
    // `--words=FILE` is read as `--words FILE`.
    let mut args: Vec<String> = args
        .iter()
        .flat_map(|arg| match arg.split_once('=') {
            Some(("--words", file)) => vec!["--words".to_owned(), file.to_owned()],
            _ => vec![arg.clone()],
        })
        .collect();
    // The launcher's words come last, as the unit gives them. The file
    // mustn't be a flag, which `--words --fast` would take as one, nor
    // empty, as `--words=` leaves it.
    let words = match args.iter().position(|arg| arg == "--words") {
        Some(at) if at + 2 == args.len() => {
            let path = args
                .pop()
                .filter(|file| !file.is_empty() && !file.starts_with("--"));
            args.pop();
            Some(
                path.map(PathBuf::from)
                    .ok_or_else(|| wrongly(&args, "--words needs a value".to_owned()))?,
            )
        }
        Some(_) => {
            return Err(wrongly(
                &args,
                "--words comes last, with the launcher's words after it".to_owned(),
            ));
        }
        None => None,
    };
    if let Some(flag) = args.iter().find(|arg| arg.starts_with('-')) {
        return Err(flags::unknown("option", flag, &["--words"]).try_this(help_for(&args)));
    }
    args.extend_from_slice(after);
    let path = PathBuf::from;
    match args.as_slice() {
        // The launcher's words are the daemon's, so set and again refuse
        // them rather than dropping them unread.
        [command, ..] if words.is_some() && matches!(command.as_str(), "set" | "again") => {
            Err(wrongly(
                &args,
                format!("location {command} takes no --words; they are the daemon's"),
            ))
        }
        [set, pin, town @ ..] if set == "set" && !town.is_empty() => Ok(Start::Set {
            pin: path(pin),
            town: town.join(" "),
        }),
        [again, pin] if again == "again" => Ok(Start::Again { pin: path(pin) }),
        [command, ..] if command == "set" => Err(wrongly(
            &args,
            "location set takes the pin's file and the town".to_owned(),
        )),
        [command, ..] if command == "again" => Err(wrongly(
            &args,
            "location again takes the pin's file alone".to_owned(),
        )),
        [socket, root] => Ok(Start::Serve {
            socket: path(socket),
            root: path(root),
            following: None,
            words,
        }),
        [socket, root, kept, pin, moved] => Ok(Start::Serve {
            socket: path(socket),
            root: path(root),
            following: Some([path(kept), path(pin), path(moved)]),
            words,
        }),
        _ => Err(wrongly(
            &args,
            format!(
                "location takes its socket and root, or those and three files, not {} words",
                args.len()
            ),
        )),
    }
}

/// A start asked wrongly, pointing at the help of the command the words
/// name.
fn wrongly(args: &[String], problem: String) -> Failure {
    Failure::new(Exit::Usage, problem).try_this(help_for(args))
}

/// The help of the command the words name.
fn help_for(args: &[String]) -> String {
    match args.first().map(String::as_str) {
        Some(command @ ("set" | "again")) => format!("location {command} --help"),
        _ => "location --help".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Start, help, start};
    use std::path::{Path, PathBuf};
    use terminal_kit::{Ending, Environment, Exit};

    /// How the kit ends location given `words`, its body reading them as
    /// `main` does and starting nothing, with the start they asked for.
    fn started(words: &[&str]) -> (Ending, Option<Start>) {
        let mut asked = None;
        let ending = terminal_kit::run(
            &help(),
            "0.1.0",
            &Environment::of([false; 3], &[]),
            words.iter().map(|word| (*word).to_owned()).collect(),
            |context| {
                asked = Some(start(&context.args)?);
                Ok(())
            },
        );
        (ending, asked)
    }

    #[test]
    fn the_units_words_start_it_following_with_its_words() {
        let unit = [
            "/run/user/1000/daemons/location.sock",
            "/",
            "/run/user/1000/whereabouts/here.json",
            "/home/joel/.local/state/whereabouts/pin.json",
            "/home/joel/.local/state/clock-follows/moved.json",
            "--words",
            "/nix/store/words.json",
        ];
        let (ending, asked) = started(&unit);
        assert_eq!(
            ending.exit,
            Exit::Done,
            "the unit's words were refused: {}",
            ending.stderr
        );
        let following = [unit[2], unit[3], unit[4]].map(PathBuf::from);
        assert_eq!(
            asked,
            Some(Start::Serve {
                socket: PathBuf::from(unit[0]),
                root: PathBuf::from("/"),
                following: Some(following),
                words: Some(PathBuf::from(unit[6])),
            }),
            "the unit's words asked for another start"
        );
    }

    #[test]
    fn set_and_again_answer_before_a_socket_is_taken_as_their_name() {
        for (words, named) in [
            (&["set", "--help"][..], "location set"),
            (&["again", "-h"][..], "location again"),
            (&["help", "set"][..], "location set"),
        ] {
            let (ending, asked) = started(words);
            assert_eq!(
                ending.exit,
                Exit::Done,
                "{words:?} was refused: {}",
                ending.stderr
            );
            assert!(
                asked.is_none(),
                "{words:?} started location rather than helping"
            );
            assert!(
                ending.stdout.starts_with(named),
                "{words:?} didn't answer {named}'s help: {}",
                ending.stdout
            );
        }
        let (_, asked) = started(&["set", "pin.json", "Bergen,", "Norway"]);
        assert_eq!(
            asked,
            Some(Start::Set {
                pin: PathBuf::from("pin.json"),
                town: "Bergen, Norway".to_owned()
            }),
            "set's town didn't reach it whole"
        );
        let (_, asked) = started(&["again", "pin.json"]);
        assert_eq!(
            asked,
            Some(Start::Again {
                pin: PathBuf::from("pin.json")
            }),
            "again's pin didn't reach it"
        );
    }

    #[test]
    fn a_town_opening_with_a_dash_is_set_after_the_end_of_flags() {
        let (ending, asked) = started(&["set", "pin.json", "--", "-Kelpwater", "--words"]);

        assert_eq!(
            ending.exit,
            Exit::Done,
            "the town after -- was refused: {}",
            ending.stderr
        );
        assert_eq!(
            asked,
            Some(Start::Set {
                pin: PathBuf::from("pin.json"),
                town: "-Kelpwater --words".to_owned()
            }),
            "the words after -- weren't the town, as typed"
        );
    }

    #[test]
    fn the_launchers_words_may_be_joined_to_their_flag() {
        let (ending, asked) = started(&["location.sock", "/", "--words=/nix/w.json"]);

        assert_eq!(
            ending.exit,
            Exit::Done,
            "the joined words were refused: {}",
            ending.stderr
        );
        assert!(
            matches!(asked, Some(Start::Serve { words: Some(words), .. })
                if words == Path::new("/nix/w.json")),
            "the joined words weren't read as written"
        );
    }

    #[test]
    fn a_start_asked_wrongly_ends_64_with_the_help_its_words_name() {
        for (words, named) in [
            (&["set", "pin.json"][..], "location set --help"),
            (&["again"][..], "location again --help"),
            (
                &["again", "pin.json", "now"][..],
                "location again pin.json --help",
            ),
            (&["location.sock"][..], "location --help"),
            (&["location.sock", "/", "kept.json"][..], "location --help"),
            (&["location.sock", "/", "--words"][..], "location --help"),
            (&["location.sock", "/", "--fast"][..], "location --help"),
            (
                &["location.sock", "/", "--words", "--fast"][..],
                "--words needs a value",
            ),
            (
                &["location.sock", "/", "--words="][..],
                "--words needs a value",
            ),
            (
                &["location.sock", "/", "--wrods", "w.json"][..],
                "did you mean --words?",
            ),
            (
                &["set", "pin.json", "Kelpwater", "--words", "w.json"][..],
                "location set takes no --words",
            ),
            (
                &["again", "pin.json", "--words", "w.json"][..],
                "location again takes no --words",
            ),
        ] {
            let (ending, asked) = started(words);
            assert_eq!(ending.exit, Exit::Usage, "{words:?} wasn't refused with 64");
            assert!(asked.is_none(), "{words:?} started {asked:?}");
            assert!(
                ending.stderr.contains(named),
                "{words:?}'s refusal didn't name {named}: {}",
                ending.stderr
            );
        }
    }
}
