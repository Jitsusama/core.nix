//! pi-keys' command line: what `pi-keys --help` says, and the settings a
//! run of `sample` is asked for. Its timer runs `pi-keys sample
//! --call-check PROGRAM`, so that line reads as it always has.

use std::path::PathBuf;

use terminal_kit::{Exit, Failure, Help, flags};

/// The flags `sample` takes, each with a value.
const FLAGS: [&str; 3] = ["--minutes", "--wait", "--call-check"];

/// What one sample is asked to do.
#[derive(Debug, PartialEq)]
pub struct Settings {
    /// How many minutes at pi windows it samples.
    pub minutes: f64,
    /// How many minutes it waits for a pi window to hold the focus.
    pub wait: f64,
    /// A program that exits 0 while a call is on.
    pub call_check: Option<PathBuf>,
}

/// What `pi-keys --help` says: `sample`, which its timer runs.
pub fn help() -> Help {
    Help::new("pi-keys", "samples what a key typed into pi waits on")
        .example(
            "pi-keys sample",
            "ten minutes at pi windows, kept in today's file",
        )
        .subcommand(
            Help::new("sample", "samples the keys at pi windows, once")
                .example(
                    "pi-keys sample --call-check PROGRAM",
                    "as its timer runs it, stopping for a call",
                )
                .option("--minutes N", "minutes at pi windows to sample; 10")
                .option("--wait N", "minutes to wait for a pi window; 60")
                .option(
                    "--call-check PROGRAM",
                    "exits 0 during a call, which stops it",
                )
                .exit(Exit::Failed, "the sample couldn't be kept"),
        )
}

/// The settings `args` ask a sample for, or none for a bare `pi-keys`,
/// which shows its help's first lines. The kit has walked them, so the
/// first word is `sample` and nothing past its flags is left.
pub fn asked(args: &[String]) -> Result<Option<Settings>, Failure> {
    let Some((_, flagged)) = args.split_first() else {
        return Ok(None);
    };
    let mut settings = Settings {
        minutes: 10.0,
        wait: 60.0,
        call_check: None,
    };
    let mut flagged = flagged.iter();
    while let Some(arg) = flagged.next() {
        let (flag, value) = match arg.split_once('=') {
            Some((flag, value)) => (flag, Some(value)),
            None => (arg.as_str(), None),
        };
        if !FLAGS.contains(&flag) {
            return Err(flags::unknown("option", flag, &FLAGS));
        }
        let value = value
            .or_else(|| flagged.next().map(String::as_str))
            .ok_or_else(|| {
                Failure::new(Exit::Usage, format!("{flag} needs a value"))
                    .try_this("pi-keys sample --help")
            })?;
        match flag {
            "--minutes" => settings.minutes = minutes(flag, value)?,
            "--wait" => settings.wait = minutes(flag, value)?,
            _ => settings.call_check = Some(PathBuf::from(value)),
        }
    }
    Ok(Some(settings))
}

/// A count of minutes past none, as `--minutes` and `--wait` take.
fn minutes(flag: &str, value: &str) -> Result<f64, Failure> {
    value
        .parse::<f64>()
        .ok()
        .filter(|minutes| minutes.is_finite() && *minutes > 0.0)
        .ok_or_else(|| {
            Failure::new(Exit::Usage, format!("{flag} takes minutes, as 10"))
                .because(format!("{value} isn't a count of minutes past none"))
                .try_this("pi-keys sample --help")
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use terminal_kit::{Pen, Room};

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    fn refused(line: &str) -> Failure {
        match asked(&words(line)) {
            Err(failure) => failure,
            Ok(settings) => panic!("{line:?} was taken, as {settings:?}"),
        }
    }

    fn said(failure: &Failure) -> String {
        failure.text(&Pen::plain(), Room::Cells(80), false)
    }

    #[test]
    fn its_timers_line_reads_as_it_always_has() {
        let settings = asked(&words("sample --call-check /run/calls")).expect("its line is taken");
        assert_eq!(
            settings,
            Some(Settings {
                minutes: 10.0,
                wait: 60.0,
                call_check: Some(PathBuf::from("/run/calls")),
            }),
            "its timer's line should sample ten minutes, waiting an hour"
        );
    }

    #[test]
    fn minutes_and_wait_are_taken_either_way_theyre_written() {
        let settings = asked(&words("sample --minutes 2 --wait=0.5")).expect("both are taken");
        assert_eq!(
            settings.map(|settings| (settings.minutes, settings.wait)),
            Some((2.0, 0.5)),
            "--minutes 2 and --wait=0.5 should both be read"
        );
    }

    #[test]
    fn a_bare_pi_keys_samples_nothing() {
        assert_eq!(
            asked(&[]).expect("bare is taken"),
            None,
            "a bare pi-keys should only show"
        );
    }

    #[test]
    fn a_flag_it_doesnt_know_is_refused_with_the_nearest() {
        let failure = refused("sample --minute 2");
        assert_eq!(failure.exit, Exit::Usage, "an unknown flag should end 64");
        assert!(
            said(&failure).contains("--minutes"),
            "it should name --minutes: {}",
            said(&failure)
        );
    }

    #[test]
    fn a_number_that_isnt_one_is_refused_rather_than_read_as_the_default() {
        for line in [
            "sample --minutes ten",
            "sample --wait -1",
            "sample --minutes 0",
        ] {
            let failure = refused(line);
            assert_eq!(failure.exit, Exit::Usage, "{line:?} should end 64");
        }
    }

    #[test]
    fn a_flag_with_no_value_is_refused() {
        let failure = refused("sample --call-check");
        assert_eq!(
            failure.exit,
            Exit::Usage,
            "a flag with no value should end 64"
        );
        assert!(
            said(&failure).contains("--call-check"),
            "it should name the flag: {}",
            said(&failure)
        );
    }

    #[test]
    fn its_help_names_sample_and_what_its_timer_hands_it() {
        let text = help().text(&Pen::plain(), Room::Cells(60));
        assert!(
            text.starts_with("pi-keys"),
            "help should open with its name: {text}"
        );
        let help = help();
        let sample = help.subs.iter().find(|sub| sub.name == "sample");
        let sample = sample.expect("help should name sample");
        let flags: Vec<&str> = sample
            .options
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();
        assert!(
            FLAGS
                .iter()
                .all(|flag| flags.iter().any(|named| named.starts_with(flag))),
            "sample's help should name each flag it takes: {flags:?}"
        );
    }
}
