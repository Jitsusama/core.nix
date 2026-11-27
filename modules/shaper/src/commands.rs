//! The shaper's commands, each with help of its own, which the terminal
//! kit answers for `shaper run --help`, `-h` and `shaper help run`. Every
//! flag a unit passes is named here with its value, so the kit passes the
//! value over rather than reading it as a command. `proof-call` is the
//! call `shaper proof` starts in a slice of its own, so it answers without
//! being offered.

use terminal_kit::{Exit, Help};

/// What `shaper --help` says, and each command's help under it.
pub fn help() -> Help {
    Help::new(
        "shaper",
        "keeps the network's queue here, sorted by the slices' classes",
    )
    .example("shaper status", "how each link is shaped now")
    .example("shaper day", "today's and yesterday's records summed")
    .subcommand(run())
    .subcommand(status())
    .subcommand(stop())
    .subcommand(record())
    .subcommand(replay())
    .subcommand(day())
    .subcommand(proof())
    .subcommand(proof_call())
    .exit(Exit::Usage, "it was asked wrongly")
}

/// What a flag it doesn't take, or a value it can't read, ends in.
const WRONGLY: &str = "a flag it doesn't take, or a value it can't read";

fn run() -> Help {
    Help::new("run", "shapes every uplink until it's stopped")
        .example("shaper run", "as its unit runs it, with no VPNs")
        .example(
            "shaper run --vpns wg0",
            "riding wg0, rated from each direction's delay",
        )
        .option("--hosts HOSTS", "the distant hosts it pings, by commas")
        .option("--vpns VPNS", "the tunnels it shapes too, by commas")
        .option("--record DIR", "where the recorder writes, which it reads")
        .option("--trial FILE", "a trial of how the upload finds its line")
        .option(
            "--networks FILE",
            "each network's upload rate, to guard it by",
        )
        .option("--state DIR", "where what it heard each second is kept")
        .option("--runtime DIR", "where its status is written")
        .option("--cgroups DIR", "where the slices' cgroups are")
        .option("--proc DIR", "where processes are listed")
        .option("--users DIR", "where the accounts' runtime folders are")
        .exit(Exit::Usage, WRONGLY)
}

fn status() -> Help {
    Help::new("status", "how each link is shaped, and what it carries")
        .example("shaper status", "each uplink's rates and its flows")
        .option("--runtime DIR", "where the shaper writes its status")
        .exit(Exit::Usage, WRONGLY)
        .exit(Exit::Unavailable, "the shaper isn't running")
}

fn stop() -> Help {
    Help::new("stop", "takes every queue the shaper made away")
        .example("shaper stop", "takes them away now")
        .example(
            "shaper stop --unless-restarting shaper.service",
            "as its unit does once the shaper ends",
        )
        .option(
            "--unless-restarting UNIT",
            "leaves them while systemd restarts UNIT, for the shaper it starts",
        )
        .exit(Exit::Usage, "it takes no words")
}

fn record() -> Help {
    Help::new("record", "records the uplink's signal until it's stopped")
        .example(
            "shaper record --state /var/lib/shaper-record",
            "as its unit runs it",
        )
        .option("--state DIR", "where the record is written")
        .exit(Exit::Usage, WRONGLY)
}

fn replay() -> Help {
    Help::new("replay", "replays recorded days through a controller")
        .example("shaper replay day.jsonl", "the summary, by the rebuild")
        .example(
            "shaper replay --controller none --json day.jsonl",
            "unshaped, as JSON",
        )
        .example(
            "shaper replay --controller held --heard /var/lib/shaper/heard \
             --anchor 00:00:5e:00:53:01=12000 *.jsonl",
            "what the shaper held, against the line",
        )
        .argument("<files>...", "the recorded days")
        .option("--base UP,DOWN", "the rates it starts from, in kbit/s")
        .option("--controller CONTROLLER", "one-way, none, or held")
        .option("--heard DIR", "the live shaper's trace, put back in")
        .option(
            "--anchor NETWORK=UP",
            "the line a network carries up, in kbit/s",
        )
        .option(
            "--most NETWORK=UP",
            "the most a network's upload is held to, in kbit/s",
        )
        .option("--windows FILE", "NAME FROM TO lines, judged apart")
        .exit(Exit::Usage, WRONGLY)
        .exit(Exit::Failed, "a file couldn't be read")
}

fn day() -> Help {
    Help::new("day", "sums a day's record into one file")
        .example("shaper day", "today's, and yesterday's until it's whole")
        .example("shaper day 2026-11-13", "that day's alone")
        .argument("[date]...", "the days, as 2026-11-13")
        .option("--record DIR", "where the recorder writes")
        .option("--out DIR", "where each day's file goes")
        .exit(Exit::Usage, WRONGLY)
        .exit(Exit::Failed, "a day couldn't be written")
}

fn proof() -> Help {
    Help::new("proof", "measures whether the shaper beats the router")
        .example("shaper proof", "twelve pairs, while Joel is away")
        .example(
            "shaper proof --pairs 2 --out proof.json",
            "two, with the report kept",
        )
        .option("--server URL", "the speed server it loads the link from")
        .option("--pairs COUNT", "how many pairs of runs it measures")
        .option("--seconds SECONDS", "how long each run is")
        .option("--uploads COUNT", "how many uploads load the link")
        .option("--wait SECONDS", "how long it waits for Joel to be away")
        .option("--out FILE", "where the report is written")
        .exit(Exit::Usage, WRONGLY)
        .exit(Exit::Failed, "the shaper didn't beat the router")
        .exit(
            Exit::Unavailable,
            "no default route, no swayidle, no lock, or the shaper couldn't be put back",
        )
        .exit(
            Exit::TryAgain,
            "Joel wasn't away long enough, the shaper's state was unknown, or a proof is running",
        )
        .exit(Exit::Config, "what it waits on isn't a list of commands")
        .exit(Exit::Interrupted, "stopped, the shaper back as it was")
}

fn proof_call() -> Help {
    Help::new("proof-call", "a call's round trips, one each line")
        .example("shaper proof-call 30", "for thirty seconds")
        .argument("<seconds>", "how long the call lasts")
        .exit(Exit::Usage, "it takes a number of seconds")
        .exit(Exit::Unavailable, "the resolver couldn't be reached")
        .unlisted()
}

#[cfg(test)]
mod tests {
    use super::help;
    use std::cell::RefCell;
    use terminal_kit::{Context, Ending, Environment, Exit, Failure};

    /// Runs `words` as the shaper would, with `body` for what it does.
    fn ran(words: &[&str], body: impl FnOnce(&Context) -> Result<(), Failure>) -> Ending {
        terminal_kit::run(
            &help(),
            "0.1.0",
            &Environment::of([false; 3], &[("HOME", "/nonexistent")]),
            owned(words),
            body,
        )
    }

    fn owned(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| (*word).to_owned()).collect()
    }

    /// The words each unit runs the shaper with, as nixos.nix spells them.
    const UNITS: [&[&str]; 5] = [
        &["run"],
        &[
            "run",
            "--vpns",
            "wg0,wg1",
            "--networks",
            "/etc/shaper-networks.json",
        ],
        &["stop", "--unless-restarting", "shaper.service"],
        &["record", "--state", "/var/lib/shaper-record"],
        &["day"],
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
                owned(words),
                "the kit changed a unit's words, so its command reads others"
            );
        }
    }

    #[test]
    fn a_runs_unit_words_are_read_as_they_mean() {
        let Ok(asked) = crate::options("run", &owned(&UNITS[1][1..])) else {
            panic!("the run's unit words were refused");
        };
        assert_eq!(asked.vpns, ["wg0", "wg1"], "--vpns");
        assert_eq!(
            asked.record,
            std::path::Path::new(UNITS[3][2]),
            "the folder the run reads isn't the one the recorder's unit writes"
        );
        assert_eq!(
            asked.networks.as_deref(),
            Some(std::path::Path::new("/etc/shaper-networks.json")),
            "--networks"
        );
        let Ok(recorder) = crate::options("record", &owned(&UNITS[3][1..])) else {
            panic!("the recorder's unit words were refused");
        };
        assert_eq!(
            recorder.state,
            std::path::Path::new("/var/lib/shaper-record"),
            "--state"
        );
    }

    #[test]
    fn a_command_asked_wrongly_ends_64_naming_its_help() {
        for (words, named) in [
            (&[][..], "shaper --help"),
            (&["shape"], "shaper --help"),
            (&["run", "--sideways"], "shaper run --help"),
            (&["run", "--controller", "sideways"], "shaper run --help"),
            // One controller is left, so there is none to choose.
            (&["run", "--controller", "round-trip"], "shaper run --help"),
            (&["run", "--controller", "one-way"], "shaper run --help"),
            (&["run", "--vpns"], "shaper run --help"),
            (&["status", "--runtime"], "shaper status --help"),
            (&["stop", "--now"], "shaper stop --help"),
            (&["stop", "--unless-restarting"], "shaper stop --help"),
            (&["record", "--sideways"], "shaper record --help"),
            (&["replay"], "shaper replay --help"),
            (&["day", "--sideways"], "shaper day --help"),
            (&["proof", "--pairs", "many"], "shaper proof --help"),
            (&["proof-call"], "shaper proof-call --help"),
            (&["proof-call", "soon"], "shaper proof-call --help"),
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

    #[test]
    fn status_with_no_shaper_running_says_so_as_unavailable() {
        let ending = ran(
            &["status", "--runtime", "/nonexistent/shaper"],
            crate::asked,
        );
        assert_eq!(ending.exit, Exit::Unavailable, "{}", ending.stderr);
        assert!(
            ending.stderr.contains("isn't running"),
            "status didn't say the shaper isn't running: {}",
            ending.stderr
        );
    }
}
