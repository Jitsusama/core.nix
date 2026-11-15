//! night-shift's two commands as the terminal kit takes them: their help,
//! and a start asked wrongly ending 64 with the help to read. Each valued
//! flag is declared with its value, so the kit's walk passes over it, and
//! the kit reads nothing from `--` on, which is the job's own.

use terminal_kit::{Exit, Failure, Help};

/// What night-shift and its two commands say they are, do and end with,
/// as `--help` shows it.
pub fn help() -> Help {
    Help::new(
        "night-shift",
        "runs a heavy job at night, once, only while memory is to spare",
    )
    .example(
        "night-shift run repack -- git maintenance run",
        "as a night job's unit runs it",
    )
    .example(
        "night-shift turn eval -- nix flake check",
        "a heavy job by day, waiting its turn",
    )
    .subcommand(run())
    .subcommand(turn())
}

fn run() -> Help {
    Help::new("run", "runs a night job while memory is to spare")
        .example(
            "night-shift run repack --rests 12 -- git gc",
            "once in 12 hours at most",
        )
        .argument("NAME", "the job's name, for its marks")
        .argument("COMMAND...", "the job, after --")
        .option("--start-under PCT", "starts under this pressure; 3")
        .option("--stop SLICE=PCT", "stops past a slice's pressure")
        .option("--stop-machine PCT", "stops past the machine's; 25")
        .option("--rests HOURS", "does nothing this long after; 12")
        .option("--wait-at-most SECONDS", "waits this long for the turn")
        .option("--every SECONDS", "how often it looks; 5")
        .option("--grace SECONDS", "how long a stop waits; 60")
        .option("--memory-high SIZE", "the job's memory.high")
        .option("--memory-max SIZE", "the job's memory.max")
        .option("--swap-max SIZE", "the job's memory.swap.max")
        .option("--reads-at-most DISK=RATE", "the job's reads from a disk")
        .exit(
            Exit::Failed,
            "its folders, lock or cgroup failed it, or the job didn't start",
        )
        .exit(
            Exit::TryAgain,
            "it waited for the turn as long as it was let",
        )
        .passes("the job's own code, when it fails")
}

fn turn() -> Help {
    Help::new("turn", "runs a heavy job once no other is running")
        .example(
            "night-shift turn eval --wait-at-most 600 -- make",
            "gives up after ten minutes",
        )
        .argument("NAME", "the job's name, as others see it")
        .argument("COMMAND...", "the job, after --")
        .option("--wait-at-most SECONDS", "gives up waiting after this")
        .exit(Exit::Failed, "its lock failed it, or the job didn't start")
        .exit(Exit::TryAgain, "it waited as long as it was let, before the job")
        .passes("the job's own code once it runs, even 1 or 75")
}

/// A start asked wrongly, as the parse says, pointing at the help of the
/// command the words name.
pub fn wrongly(args: &[String], problem: String) -> Failure {
    let named = match args.first().map(String::as_str) {
        Some(command @ ("run" | "turn")) => format!("night-shift {command} --help"),
        _ => "night-shift --help".to_owned(),
    };
    Failure::new(Exit::Usage, problem).try_this(named)
}

#[cfg(test)]
mod tests {
    use super::{help, wrongly};
    use terminal_kit::{Environment, Exit};

    /// How the kit ends night-shift given `words`, its body parsing them
    /// as `main`'s does and starting nothing, with the job's command it
    /// read.
    fn parsed(words: &str) -> (Exit, String, Vec<String>) {
        let mut command = Vec::new();
        let ending = terminal_kit::run(
            &help(),
            "0.1.0",
            &Environment::of([false; 3], &[]),
            words.split(' ').map(str::to_owned).collect(),
            |context| {
                let args = &context.args;
                command = if args.first().map(String::as_str) == Some("turn") {
                    crate::turn::parse(args).map(|turn| turn.command)
                } else {
                    crate::parse(args).map(|job| job.command)
                }
                .map_err(|problem| wrongly(args, problem))?;
                Ok(())
            },
        );
        (ending.exit, ending.stderr, command)
    }

    #[test]
    fn a_units_words_reach_the_job_as_written() {
        let unit = "run repack --start-under 3 --stop app.slice=1 --stop-machine 25 \
                    --rests 12 --wait-at-most 1800 --memory-high 12G --reads-at-most /dev/nvme0n1=50M \
                    -- git -C /srv maintenance run --help";
        let (exit, said, command) = parsed(unit);
        assert_eq!(exit, Exit::Done, "the unit's words were refused: {said}");
        assert_eq!(
            command,
            ["git", "-C", "/srv", "maintenance", "run", "--help"],
            "the job's command didn't reach it whole"
        );
        let (exit, said, command) = parsed("turn eval --wait-at-most 5 -- nix --version");
        assert_eq!(exit, Exit::Done, "a turn's words were refused: {said}");
        assert_eq!(
            command,
            ["nix", "--version"],
            "the kit took the turn's command's flags"
        );
    }

    #[test]
    fn a_wrong_start_ends_64_naming_its_commands_help() {
        for (words, named) in [
            ("run repack", "night-shift run --help"),
            ("run repack --rests lots -- true", "night-shift run --help"),
            ("run repack --what 1 -- true", "night-shift run --help"),
            ("turn eval --", "night-shift turn --help"),
            ("turn", "night-shift turn --help"),
            ("walk repack -- true", "night-shift --help"),
        ] {
            let (exit, said, _) = parsed(words);
            assert_eq!(exit, Exit::Usage, "{words} wasn't refused with 64: {said}");
            assert!(
                said.contains(named),
                "{words}'s refusal didn't name {named}: {said}"
            );
        }
    }
}
