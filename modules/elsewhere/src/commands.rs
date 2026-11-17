//! elsewhere's three commands as the terminal kit takes them: their help,
//! and a start asked wrongly ending 64 with the help to read. The kit
//! reads nothing from `--` on, which is the job's own.

use terminal_kit::{Exit, Failure, Help};

/// What elsewhere and its commands say they are, do and end with, as
/// `--help` shows it.
pub fn help() -> Help {
    Help::new("elsewhere", "runs a heavy job wherever mycelium places it")
        .example(
            "elsewhere run model-check -- tla-rs -workers 8 Spec.tla",
            "here or on a far machine, as mycelium places it",
        )
        .example(
            "elsewhere where model-check",
            "where it would run now, and why",
        )
        .example("elsewhere kinds", "the kinds of job it may send")
        .subcommand(run())
        .subcommand(place())
        .subcommand(kinds())
}

fn run() -> Help {
    Help::new(
        "run",
        "runs a job here, or by mycelium where it's placed, its tracked files carried by PI_QUEST's handover; one that can't go runs here",
    )
    .example(
        "elsewhere run model-check -- tla-rs Spec.tla",
        "wherever mycelium places it",
    )
    .example(
        "elsewhere run model-check --here -- tla-rs Spec.tla",
        "here, wherever it's placed",
    )
    .argument("KIND", "the kind of job, as its settings name it")
    .argument("COMMAND...", "the job, after --")
    .option("--here", "runs it here, without asking mycelium")
    .option(
        "--there",
        "sends it to the first machine, without asking mycelium",
    )
    .exit(
        Exit::Failed,
        "the job couldn't be left in the quest's handover, or didn't start",
    )
    .exit(
        Exit::Usage,
        "no kind, no command after --, or an option it doesn't take",
    )
    .exit(Exit::NotFound, "no kind goes by that name")
    .exit(
        Exit::Refused,
        "the job's program isn't one its kind may start",
    )
    .exit(
        Exit::TryAgain,
        "the job's files didn't cross, or what it made didn't come home",
    )
    .exit(
        Exit::Config,
        "its settings can't be read, or its kind brings back what can't come home",
    )
    .passes("the job's own code, when it fails, here or there")
}

fn place() -> Help {
    Help::new("where", "where a kind of job would run now, and why")
        .example(
            "elsewhere where model-check",
            "here, or the machine it'd go to",
        )
        .argument("KIND", "the kind of job, as its settings name it")
        .exit(Exit::NotFound, "no kind goes by that name")
        .exit(Exit::Config, "its settings can't be read")
}

fn kinds() -> Help {
    Help::new(
        "kinds",
        "the kinds of job it may send, and what each starts",
    )
    .alias("kind")
    .example("elsewhere kinds --json", "as JSON")
    .exit(Exit::Config, "its settings can't be read")
}

/// A start asked wrongly, as the parse says, pointing at the help of the
/// command the words name.
pub fn wrongly(args: &[String], problem: impl Into<String>) -> Failure {
    let named = match args.first().map(String::as_str) {
        Some(command @ ("run" | "where" | "kinds")) => format!("elsewhere {command} --help"),
        Some("kind") => "elsewhere kinds --help".to_owned(),
        _ => "elsewhere --help".to_owned(),
    };
    Failure::new(Exit::Usage, problem).try_this(named)
}
