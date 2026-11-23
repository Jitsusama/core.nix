//! What nix-offload and each of its subcommands say they are and take, as
//! `--help` shows it (docs/commands.md), answered by the terminal kit.
//! Help is the kit's alone; every other call's words are nix-offload's
//! own to read, as its units, its helpers and every gate call them, and
//! only its ending goes through the kit, as a code from its table.

use std::process::ExitCode;

use terminal_kit::{Exit, Failure, Help};

/// The kit's answer to a call [`asked`] takes. The kit answers every such
/// call itself, so reaching the body means the two disagree: a bug, said
/// as one, never a quiet success.
pub fn answered() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |_| {
        Err(Failure::new(
            Exit::Bug,
            "nix-offload took this for help, but its help didn't answer it",
        ))
    })
}

/// Whether the words ask for help or the version, which the kit answers:
/// `-h` or `--help` among nix-offload's own words, or `--version` alone.
/// The words after `--` are a command's, and after `there --changed`
/// changed-checks', so a `--help` there is theirs, as it was before.
pub fn asked(arguments: &[String]) -> bool {
    if arguments == ["--version"] {
        return true;
    }
    let ours = arguments
        .iter()
        .position(|word| word == "--" || word == "--changed")
        .unwrap_or(arguments.len());
    // `help` as the first word asks for what follows it, as `nix-offload
    // help there`, a word no call of nix-offload's opens with.
    arguments[..ours]
        .iter()
        .find(|word| !word.starts_with('-'))
        .is_some_and(|word| word == "help")
        || arguments[..ours]
            .iter()
            .any(|word| word == "--help" || word == "-h")
}

/// nix-offload's help, with each subcommand's under it.
pub fn help() -> Help {
    Help::new(
        "nix-offload",
        "lends builders from the cloud and sends builds to them, or to a machine nearby",
    )
    .example(
        "nix-offload there . checks.x86_64-linux.a",
        "builds a check on a builder",
    )
    .example(
        "nix-offload there --changed",
        "builds every check this change moved",
    )
    .example(
        "nix-offload there --run -- cargo test",
        "runs a command in a checkout of this commit, on a builder",
    )
    .exit(Exit::Failed, "what it ran failed, or it couldn't run it")
    .exit(Exit::Usage, "asked wrongly; it says how it's asked")
    .subcommand(there())
    .subcommand(outside())
    .subcommand(ask())
    .subcommand(answer())
    .subcommand(passed())
    .subcommand(progress())
    .subcommand(report())
    .subcommand(cargo())
    .subcommand(scoped())
    .subcommand(serve())
    .subcommand(stand())
    .subcommand(relay())
    .subcommand(lines())
    .subcommand(prove_standing())
    .subcommand(keep())
    .subcommand(busy())
}

fn there() -> Help {
    Help::new(
        "there",
        "builds on a lent builder or a machine nearby, keeping each pass",
    )
    .argument(
        "TARGET...",
        "a flake and its attributes, FLAKE#ATTR, or derivations",
    )
    .example(
        "nix-offload there . checks.x86_64-linux.a",
        "builds one check of the tree's flake",
    )
    .example(
        "nix-offload there --here-when-idle . checks.x86_64-linux.a",
        "builds it here instead while this machine is idle",
    )
    .example(
        "nix-offload there --changed",
        "builds what changed-checks names for this change",
    )
    .example(
        "nix-offload there --run --copy \"$(readlink -f result)\" -- \"$(readlink -f result)/bin/a\"",
        "runs a program built here on a builder, its store path sent first",
    )
    .option(
        "--never-ask",
        "waits only for a builder already standing, first of all",
    )
    .option("-L, --print-build-logs", "shows each build's log")
    .option("--keep-going", "builds what it can past a failure")
    .option(
        "--apply FUNCTION",
        "applies FUNCTION to the one attribute, as nix eval does",
    )
    .option("--out DIR", "copies the one attribute's output to DIR")
    .option("--print-out-paths", "names each output it built")
    .option("--here", "builds on this machine")
    .option(
        "--here-when-idle",
        "builds on this machine while it's idle, else sends it",
    )
    .option(
        "--outlive",
        "goes on to its end on the builder if its caller leaves",
    )
    .option(
        "--bulk",
        "one of many sent at once, letting other VM tests go first",
    )
    .option(
        "--first",
        "goes ahead of others' runs; the net's alone keep it",
    )
    .option(
        "--lent-only",
        "keeps the run, and all it starts, to the lent builders",
    )
    .option(
        "--changed",
        "builds what changed-checks names; first, or after --never-ask, \
         and the words after are changed-checks'",
    )
    .option(
        "--run",
        "runs the command after -- in a checkout of this commit; first, \
         or after --never-ask",
    )
    .option(
        "--copy PATH",
        "with --run, sends the store path PATH's closure first",
    )
    .option("--longest SECONDS", "with --run, gives it SECONDS at most")
    .option(
        "--alone",
        "with --run, waits for a VM builder with nothing sent to it and holds it, \
         VM tests and other --alone runs kept off it while it goes",
    )
    .option("--takes", "names a word each for what this there takes")
    .exit(Exit::Failed, "nix failed, as an evaluation does")
    .exit(
        Exit::Unavailable,
        "part of it was left unbuilt, owed for when a machine that builds it \
         answers; with --changed no builder took the checks named, or a busy \
         Mac holds a run for its system",
    )
    .exit(
        Exit::TryAgain,
        "no builder took it, so nothing was built: with --changed, \
         changed-checks builds it here, and otherwise ask again once one is lent; \
         or a nearby Mac's part waits for the Mac, the rest built",
    )
    .exit(Exit::Bug, "it couldn't start itself again, marked for its run")
    .passes(
        "nix's own code, 100 for a failed build; changed-checks' with --changed; \
         the command's with --run",
    )
}

fn outside() -> Help {
    Help::new(
        "outside",
        "runs a there call in the environment the agent fence's exemption gives, \
         granting nothing: run from inside the fence, the call stays fenced",
    )
    .argument("-- there WORD...", "the call, as there takes it")
    .example(
        "nix-offload outside --git-dir /src/a/.git --work-tree /src/a -- there --changed",
        "gates a tree from outside the fence",
    )
    .option(
        "--git-dir DIR",
        "the git folder of the tree whose commit goes",
    )
    .option("--work-tree DIR", "that tree, where the call runs")
    .option(
        "--lent-only",
        "keeps the call, and all it starts, to the lent builders",
    )
    .exit(
        Exit::Refused,
        "from 2026-11-28, when the exemption ends, it runs nothing",
    )
    .exit(Exit::Config, "HOME isn't set, so it can't run the call")
    .exit(Exit::Bug, "it couldn't start the call")
    .passes("the there call's own code, once it runs")
}

fn ask() -> Help {
    Help::new(
        "ask",
        "asks the machine that gates for the gate of this tree's commit, ending as the gate did",
    )
    .example(
        "nix-offload ask --quest QEST-20261112-0TUD09",
        "gates this commit for a quest",
    )
    .example(
        "nix-offload ask --quest QEST-20261112-0TUD09 --session SK1FF0 --on kestrel",
        "gates it on a named machine, for a session's journal",
    )
    .option("--quest QUEST", "the quest whose scratch carries the gate")
    .option(
        "--session ADDRESS",
        "the asking session, for the gate's journal",
    )
    .option(
        "--on MACHINE",
        "the machine that gates, else nix-offload/gates-on in XDG's config",
    )
    .option(
        "--longest SECONDS",
        "how long it waits for the result, else 7200",
    )
    .exit(Exit::BadInput, "what it was asked can't be gated")
    .exit(Exit::NotFound, "it isn't in a git tree with a commit")
    .exit(
        Exit::Unavailable,
        "mycelium couldn't share the quest or start the gate",
    )
    .exit(Exit::Bug, "it couldn't leave the gate's request")
    .exit(Exit::TryAgain, "no result came in time")
    .exit(Exit::Config, "neither --on nor gates-on names a machine")
    .passes("the gate's own code, once the gate ran")
}

fn answer() -> Help {
    Help::new(
        "answer",
        "runs the gate a quest asked for at a commit, leaving its result beside the asking",
    )
    .argument("QUEST", "the quest that asked")
    .argument("COMMIT", "the commit it's gated at")
    .example(
        "nix-offload answer QEST-20261112-0TUD09 $(git rev-parse HEAD)",
        "as mycelium starts it on the machine that gates",
    )
    .exit(
        Exit::Usage,
        "it wasn't given a quest's id and a full commit hash",
    )
    .exit(Exit::BadInput, "what was asked can't be gated")
    .exit(
        Exit::NotFound,
        "no gate was left there, or a link is on the way to it",
    )
    .exit(
        Exit::Unavailable,
        "the forge or git didn't answer, or it couldn't fence its git",
    )
    .exit(Exit::TryAgain, "the gate couldn't run now")
    .exit(
        Exit::Refused,
        "its own state can't be trusted, holding a link it won't follow",
    )
    .passes("the gate's own code, once the gate ran")
}

fn passed() -> Help {
    Help::new("passed", "what's kept as passed at each commit")
        .example(
            "nix-offload passed ran $(git rev-parse HEAD)",
            "names each there --run that passed there",
        )
        .subcommand(
            Help::new("forget", "has a commit's checks built again")
                .example(
                    "nix-offload passed forget $(git rev-parse HEAD)",
                    "builds the head's checks again",
                )
                .argument("COMMIT", "the commit whose passes go, whole")
                .exit(Exit::Usage, "the commit isn't whole")
                .exit(Exit::Failed, "its passes couldn't be forgotten"),
        )
        .subcommand(
            Help::new("import", "hands the keeper the passes kept before it stood")
                .example("nix-offload passed import", "hands over what's kept here")
                .exit(Exit::Unavailable, "no keeper runs on this machine")
                .exit(Exit::Failed, "a commit's records couldn't be handed over"),
        )
        .subcommand(
            Help::new(
                "ran",
                "names each there --run that passed at a commit, a JSON line each",
            )
            .example(
                "nix-offload passed ran $(git rev-parse HEAD)",
                "the runs that passed at the head",
            )
            .argument("COMMIT", "the commit, whole")
            .exit(Exit::Usage, "the commit isn't whole")
            .exit(Exit::Unavailable, "no keeper runs on this machine"),
        )
}

fn progress() -> Help {
    Help::new(
        "progress",
        "says how far the builds on a lent builder have got",
    )
    .argument("HOST", "the builder, as ssh calls it")
    .example("nix-offload progress nix-offload", "for the first builder")
    .exit(Exit::Unavailable, "ssh couldn't reach the builder")
    .exit(Exit::TryAgain, "no lender lends a builder by that name")
}

fn report() -> Help {
    Help::new("report", "says what today's lent builders cost")
        .example("nix-offload report --rate 0.68", "at 0.68 dollars an hour")
        .option("--rate DOLLARS", "a builder's price an hour")
        .option(
            "--dir DIR",
            "the lender's folder, /run/nix-offload unless named",
        )
}

fn cargo() -> Help {
    Help::new(
        "cargo",
        "runs cargo on a lent builder, or here when none stands",
    )
    .argument("-- WORD...", "cargo's words")
    .example(
        "nix-offload cargo --cargo $(command -v cargo) -- test",
        "runs a crate's tests",
    )
    .option("--cargo PATH", "the cargo to run")
    .exit(Exit::Unavailable, "cargo couldn't start")
    .passes("cargo's own code, wherever it ran")
}

fn scoped() -> Help {
    Help::new(
        "scoped",
        "runs a program in a Landlock domain that reaches and signals nothing outside it",
    )
    .argument("-- PROGRAM WORD...", "the program and its words")
    .example("nix-offload scoped -- git status", "as a gate's git runs")
    .exit(Exit::Unavailable, "Landlock isn't here, so it ran nothing")
    .exit(Exit::Bug, "it couldn't start the program")
}

fn serve() -> Help {
    Help::new(
        "serve",
        "lends builders while the queue wants one, as its unit runs it",
    )
    .example(
        "nix-offload serve --helper PATH --prefix nix-offload --rate 0.68 \
         --first-start 06:00 --last-start 22:00 --deadline 23:59",
        "lends from 06:00 to 22:00",
    )
    .option(
        "--helper PATH",
        "the program that makes and gives back a builder",
    )
    .option("--prefix NAME", "what the builders it makes are named for")
    .option("--rate DOLLARS", "a builder's price an hour")
    .option("--first-start HH:MM", "the first hour a builder may start")
    .option("--last-start HH:MM", "the last hour a builder may start")
    .option("--deadline HH:MM", "when every builder is given back")
    .option(
        "--longest HH:MM",
        "the longest a builder lives, in place of the deadline",
    )
    .option(
        "--idle HH:MM",
        "how long one sits unused before it's given back",
    )
    .option(
        "--standing PATH",
        "the standing builders, which it lends first",
    )
    .option("--beside PATH", "the builders beside them")
    .option("--dir DIR", "its folder, /run/nix-offload unless named")
    .option("--only-there", "lends only while a there build waits")
    .option("--warm WORDS", "what each builder builds before VM tests")
    .option(
        "--outlasting PROGRAM",
        "what each helper runs through, to outlast it",
    )
    .option(
        "--tell PROGRAM",
        "told when no builder answers, and again once one does",
    )
    .option(
        "--freeing PATH",
        "names what frees each builder beside once it's full",
    )
    .exit(
        Exit::Failed,
        "another serve is running, or it couldn't make its lock",
    )
    .exit(
        Exit::TryAgain,
        "it handed over to its successor, which its unit counts done",
    )
}

fn stand() -> Help {
    Help::new(
        "stand",
        "names the standing builders that answer, lending none",
    )
    .example("nix-offload stand --standing PATH", "as its unit runs it")
    .option("--standing PATH", "the standing builders")
    .option("--beside PATH", "the builders beside them")
    .exit(
        Exit::TryAgain,
        "it handed over to its successor, which its unit counts done",
    )
}

fn relay() -> Help {
    Help::new(
        "relay",
        "relays connections to a builder, as the tunnel it stands in for",
    )
    .example(
        "nix-offload relay --listen localhost:2222 --to 10.0.0.2:22",
        "as a helper starts it",
    )
    .option("--listen ADDRESS", "where it listens")
    .option("--to ADDRESS", "where it relays to")
    .exit(Exit::Failed, "it couldn't listen, or gave up relaying")
}

fn lines() -> Help {
    Help::new(
        "lines",
        "copies the lender's lines into root's, for as long as it runs",
    )
    .example(
        "nix-offload lines --config /etc/nix-offload/lines.json",
        "as its unit runs it",
    )
    .option("--config PATH", "its settings")
    .exit(Exit::Config, "its settings can't be read")
}

fn prove_standing() -> Help {
    Help::new(
        "prove-standing",
        "proves, as root, the standing builders the lines' settings name",
    )
    .example(
        "nix-offload prove-standing --config /etc/nix-offload/lines.json",
        "as its unit runs it",
    )
    .option("--config PATH", "its settings")
    .exit(Exit::Config, "its settings can't be read")
}

fn keep() -> Help {
    Help::new(
        "keep",
        "keeps the passes, answering on its socket until idle",
    )
    .example(
        "nix-offload keep --folder /var/lib/nix-offload-keep",
        "as its socket starts it",
    )
    .option("--folder DIR", "where the passes are kept")
}

fn busy() -> Help {
    Help::new(
        "busy",
        "says whether this Mac is past its room for more work, by its load alone, \
         as a person's own part is judged",
    )
    .example("nix-offload busy", "before rebuilding a Mac's system")
    .exit(Exit::Done, "it's past its room")
    .exit(Exit::Failed, "it has room")
    .exit(
        Exit::Unavailable,
        "it couldn't say its load, which a caller counts as busy",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn help_is_asked_among_its_own_words_alone() {
        for line in [
            "--help",
            "-h",
            "there --help",
            "there --never-ask . checks.x86_64-linux.a -h",
            "passed ran --help",
            "outside --help",
            "help there",
            "help passed ran",
        ] {
            assert!(asked(&words(line)), "{line} asks for help");
        }
        for line in [
            "there . checks.x86_64-linux.a",
            "there --run -- cargo --help",
            "there --changed --help",
            "outside -- there --help",
            "cargo --cargo cargo -- --help",
            "there --version",
        ] {
            assert!(!asked(&words(line)), "{line} is a call, not help");
        }
        assert!(asked(&words("--version")), "--version alone is the version");
        assert!(
            !asked(&words("-- help")),
            "a help after -- is the command's"
        );
    }

    #[test]
    fn the_kit_answers_each_call_taken_for_help() {
        let environment = terminal_kit::Environment::of([false; 3], &[]);
        for line in [
            "--help",
            "-h",
            "--version",
            "there --help",
            "there --never-ask . checks.x86_64-linux.a -h",
            "there --run --copy /nix/store/a --longest 60 --help",
            "ask --quest Q --on kestrel --help",
            "answer Q 0123 --help",
            "passed ran --help",
            "passed ran COMMIT -h",
            "outside --git-dir G --work-tree W --help",
            "serve --helper P --rate 0.68 --help",
            "help there",
            "help passed ran",
        ] {
            assert!(asked(&words(line)), "{line} isn't taken for help");
            let ending = terminal_kit::run(&help(), "0", &environment, words(line), |_| {
                panic!("{line} reached the body, not the kit's help")
            });
            assert_eq!(
                ending.code(),
                0,
                "{line} wasn't answered with help: {}",
                ending.stderr
            );
            assert!(!ending.stdout.is_empty(), "{line} said nothing");
        }
    }

    #[test]
    fn each_subcommand_the_words_take_has_its_help() {
        let help = help();
        for name in [
            "there",
            "outside",
            "ask",
            "answer",
            "passed",
            "progress",
            "report",
            "cargo",
            "scoped",
            "serve",
            "stand",
            "relay",
            "lines",
            "prove-standing",
            "keep",
            "busy",
        ] {
            assert!(
                help.command_names().contains(&name),
                "{name} has no help of its own"
            );
        }
    }
}
