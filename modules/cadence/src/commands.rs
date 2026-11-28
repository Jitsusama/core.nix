//! cadence's commands, each with help of its own, which the terminal kit
//! answers for `cadence tick --help`, `-h` and `cadence help tick`. Every
//! flag a unit passes is named here with its value, so the kit passes the
//! value over rather than reading it as a command.

use terminal_kit::{Exit, Help};

/// What `cadence --help` says, and each command's help under it.
pub fn help() -> Help {
    Help::new(
        "cadence",
        "watches the delivery cycle and asks when a target slips",
    )
    .example("cadence axes", "the axes the machine is held to")
    .example("cadence axes speed", "one of them, stepped into")
    .subcommand(tick())
    .subcommand(day())
    .subcommand(units())
    .subcommand(axes())
    .subcommand(review())
    .subcommand(hour())
    .subcommand(owe())
    .exit(Exit::Usage, "it was asked wrongly")
    .exit(Exit::Failed, "its records couldn't be read or kept")
}

/// The flags that name who hears of what, for `tick` and `day` alike.
fn telling(help: Help) -> Help {
    help.option("--watcher ADDRESS", "the session keeping the cycle fast")
        .option("--orchestrator ADDRESS", "who hears when the watcher can't")
        .option(
            "--owner PREFIX=ADDRESS",
            "who hears of a finding whose key starts so",
        )
}

fn tick() -> Help {
    telling(Help::new(
        "tick",
        "writes the hour's card, asking of each target missed",
    ))
    .example("cadence tick", "the card alone, asking no one")
    .example(
        "cadence tick --watcher W --orchestrator O",
        "the card, and a question for each slip",
    )
    .option("--restart WORDS", "what starts a died session again")
    .option(
        "--revive ADDRESS",
        "an orchestrator revived, again and again",
    )
    .option("--skip ADDRESS", "a session never revived")
    .option(
        "--realm NAME",
        "a realm whose machines' failed updates it says",
    )
    .exit(Exit::Usage, "a flag it doesn't take, or one with no value")
    .exit(
        Exit::Failed,
        "the card couldn't be written or a question sent",
    )
}

fn day() -> Help {
    telling(Help::new(
        "day",
        "reads the day just ended against the days before it",
    ))
    .example("cadence day", "the day read and kept, telling no one")
    .example(
        "cadence day --orchestrator O",
        "and its note sent to the orchestrator",
    )
    .exit(Exit::Usage, "a flag it doesn't take, or one with no value")
    .exit(Exit::Failed, "the day couldn't be read, kept or sent")
}

fn units() -> Help {
    Help::new("units", "what the unit watch reads and would tell")
        .example("cadence units", "the account's units as they are now")
        .example(
            "cadence units --at 1794600000",
            "as they were then, from the journal",
        )
        .option("--at SECONDS", "a moment past, in seconds since the epoch")
        .exit(Exit::Usage, "--at isn't a number of seconds")
}

fn axes() -> Help {
    Help::new(
        "axes",
        "the axes Joel holds the machine to, each by its bar",
    )
    .example("cadence axes", "every axis, its reading and its owner")
    .example("cadence axes speed", "one, found by the start of its name")
    .example(
        "cadence axes --served words.json",
        "the launcher's Spec rows, as one line of JSON",
    )
    .argument("[axis]", "the one to step into")
    .option("--served WORDS", "Spec's rows, in the launcher's words")
    .exit(Exit::NotFound, "no axis goes by that name")
    .exit(
        Exit::Failed,
        "its records couldn't be read, or held nothing",
    )
}

fn review() -> Help {
    Help::new("review", "the week's noise and blind spots")
        .example("cadence review", "the week's review, sent to no one")
        .example("cadence review --to O", "and sent to O")
        .option("--to ADDRESS", "who it's sent to, again for each")
        .exit(Exit::Usage, "a flag it doesn't take, or one with no value")
        .exit(Exit::Failed, "the week couldn't be read or sent")
}

fn hour() -> Help {
    Help::new("hour", "the hour against its targets, and its drag")
        .example("cadence hour", "the hour's card, a look that keeps nothing")
        .example("cadence hour --to O", "the card sent to O, its drag owed")
        .example(
            "cadence hour --checkout mycelium=~/src/mycelium",
            "and a check named there closes once it merges",
        )
        .option(
            "--to ADDRESS",
            "who the card is sent to; only then is the hour kept",
        )
        .option(
            "--checkout NAME=PATH",
            "a repository whose main says what merged",
        )
        .exit(Exit::Usage, "a flag it doesn't take, or one with no value")
        .exit(Exit::Failed, "the hour couldn't be read, kept or sent")
}

fn owe() -> Help {
    Help::new(
        "owe",
        "files a dated must in the loop's ledger, or says one is met",
    )
    .example(
        "cadence owe K --axis safety --owner A --due 2026-11-20T18:00 --says WORDS",
        "files K, due then",
    )
    .example("cadence owe --done K", "closes the must K once it's met")
    .example(
        "cadence owe --done hour:rulings-unsourced:",
        "closes every open ask about a claim of his rule; only that family closes at once",
    )
    .example(
        "cadence owe --done K 'core #6000'",
        "the finding K fixed by core #6000, closing once its figure is back",
    )
    .argument("[key]", "the must's key")
    .option("--axis AXIS", "the axis it reads beside")
    .option("--owner ADDRESS", "who filed it, and closes it")
    .option("--due WHEN", "when it's due, as 2026-11-20T18:00")
    .option("--says WORDS", "what it says is owed")
    .option("--kind KIND", "what it is; must by default")
    .option(
        "--done KEY",
        "the must to close, or the finding, followed by the change or why it's expected",
    )
    .exit(Exit::Usage, "a key, flag or value missing or wrong")
    .exit(
        Exit::NotFound,
        "no open must or finding is filed as that key, or under it",
    )
    .exit(Exit::Failed, "the ledger couldn't be kept")
}

#[cfg(test)]
mod tests {
    use super::help;
    use std::cell::RefCell;
    use terminal_kit::{Context, Ending, Environment, Exit, Failure};

    /// Runs `words` as cadence would, with `body` for what it does, in a
    /// home that isn't there, so nothing is read or kept.
    fn ran(words: &[&str], body: impl FnOnce(&Context) -> Result<(), Failure>) -> Ending {
        terminal_kit::run(
            &help(),
            "0.1.0",
            &Environment::of([false; 3], &[("HOME", "/nonexistent")]),
            words.iter().map(|word| (*word).to_owned()).collect(),
            body,
        )
    }

    /// The words each unit runs cadence with, as home.nix spells them, and
    /// cadence-spec's ask for Spec's rows (spec/src/main.rs).
    const UNITS: [&[&str]; 4] = [
        &[
            "tick",
            "--watcher",
            "W",
            "--orchestrator",
            "O",
            "--owner",
            "crashes=A",
            "--restart",
            "pi --model provider/model",
            "--revive",
            "R",
            "--skip",
            "S",
            "--realm",
            "work",
        ],
        &[
            "day",
            "--orchestrator",
            "O",
            "--watcher",
            "W",
            "--owner",
            "review=A",
        ],
        &["review", "--to", "O", "--to", "A"],
        &["axes", "--served", "/run/words.json"],
    ];

    #[test]
    fn a_units_words_reach_its_command_as_written() {
        for words in UNITS {
            let reached = RefCell::new(Vec::new());
            let ending = ran(words, |context| {
                reached.replace(context.args.clone());
                Ok(())
            });
            assert_eq!(ending.exit, Exit::Done, "{words:?}: {}", ending.stderr);
            assert_eq!(
                reached.into_inner(),
                words.to_vec(),
                "the kit changed a unit's words, so its command reads others"
            );
        }
    }

    #[test]
    fn a_ticks_unit_words_are_read_as_they_mean() {
        let words = UNITS[0][1..].iter().map(|word| (*word).to_owned());
        let Ok(asked) = crate::ticking(words) else {
            panic!("the tick's unit words were refused");
        };
        assert_eq!(asked.watcher.as_deref(), Some("W"), "--watcher");
        assert_eq!(asked.orchestrator.as_deref(), Some("O"), "--orchestrator");
        assert_eq!(
            asked.restart.as_deref(),
            Some("pi --model provider/model"),
            "--restart"
        );
        assert_eq!(asked.orchestrators, ["R"], "--revive");
        assert_eq!(asked.skip, ["S"], "--skip");
        assert_eq!(asked.realms, ["work"], "--realm");
        assert_eq!(
            asked.owners.get("crashes").map(String::as_str),
            Some("A"),
            "--owner"
        );
    }

    #[test]
    fn a_command_asked_wrongly_ends_64_naming_its_help() {
        for (words, named) in [
            (&[][..], "cadence --help"),
            (&["tock"], "cadence --help"),
            (&["tick", "--sideways", "x"], "cadence tick --help"),
            (&["day", "--to", "O"], "cadence day --help"),
            (&["units", "--at", "soon"], "cadence units --help"),
            (&["axes", "speed", "again"], "cadence axes"),
            (&["review", "--watcher", "W"], "cadence review --help"),
            (&["owe", "K", "--axis"], "cadence owe --help"),
            (&["owe", "--done"], "cadence owe --help"),
        ] {
            let ending = ran(words, crate::asked);
            assert_eq!(ending.exit, Exit::Usage, "{words:?}: {}", ending.stderr);
            assert!(
                ending.stderr.contains(named),
                "{words:?} didn't point at {named}: {}",
                ending.stderr
            );
            assert_eq!(ending.stdout, "", "{words:?} said something on stdout");
        }
    }
}
