//! A subcommand that hands its words on: a wrapper of nix or cargo, as
//! `nix-offload there` is, whose every word after its name is the
//! program's, so `-v`, `--json` and `--color` there are nix's and never
//! the kit's. Only `-h` and `--help` are still the kit's, before `--` or
//! a word the subcommand names as its end, past which even help is the
//! program's (docs/commands.md, A stand-in passes through).

use crate::Help;
use crate::tree;

/// The words split where a subcommand that hands them on is named: the
/// kit's own, up to and with its name, and the program's after it, with
/// whether those ask for the subcommand's help. Words that reach no such
/// subcommand are all the kit's.
pub(crate) fn split(help: &Help, mut args: Vec<String>) -> (Vec<String>, Vec<String>, bool) {
    // Walked as the words come, flags and all, since the walk passes over
    // a flag as the shared flags are taken later: a word it refuses here
    // is refused again, said, once they have been.
    if help.subs.is_empty() {
        return (args, Vec::new(), false);
    }
    let Ok(walked) = tree::walk(help, &args) else {
        return (args, Vec::new(), false);
    };
    let (Some(at), Some(handing)) = (walked.handed, walked.chain.last()) else {
        return (args, Vec::new(), false);
    };
    let theirs = args.split_off(at + 1);
    let ends = handing.hands_on.as_deref().unwrap_or_default();
    let helped = theirs
        .iter()
        .take_while(|word| *word != "--" && !ends.contains(word))
        .any(|word| word == "-h" || word == "--help");
    (args, theirs, helped)
}

#[cfg(test)]
mod tests {
    use crate::{Context, Ending, Environment, Exit, Failure, Format, Help, run};

    fn offload() -> Help {
        Help::new("offload", "sends builds elsewhere")
            .subcommand(
                Help::new("there", "builds a flake's attributes on a builder")
                    .option("--keep-going", "builds every attribute, whatever fails")
                    .hands_words_on(&["--changed"]),
            )
            .subcommand(Help::new("lines", "copies the lender's lines"))
    }

    fn ran(line: &str, body: impl FnOnce(&Context) -> Result<(), Failure>) -> Ending {
        let args = line.split_whitespace().map(str::to_owned).collect();
        run(
            &offload(),
            "1.0",
            &Environment::of([false; 3], &[]),
            args,
            body,
        )
    }

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn the_shared_flags_after_a_handing_subcommand_are_the_programs() {
        let mut seen = None;
        let ending = ran(
            "there --keep-going -v --json --color always . a",
            |context| {
                seen = Some((context.args.clone(), context.shared.format.clone()));
                Ok(())
            },
        );
        assert_eq!(ending.exit, Exit::Done, "{}", ending.stderr);
        assert_eq!(
            seen,
            Some((
                words("there --keep-going -v --json --color always . a"),
                Format::Text
            )),
            "the kit took a word the program was handed"
        );
    }

    #[test]
    fn the_shared_flags_before_it_are_still_the_kits() {
        let mut seen = None;
        let ending = ran("--color never there -v", |context| {
            seen = Some((context.args.clone(), context.out.inks()));
            Ok(())
        });
        assert_eq!(ending.exit, Exit::Done, "{}", ending.stderr);
        assert_eq!(seen, Some((words("there -v"), false)));
    }

    #[test]
    fn help_after_it_is_still_its_own() {
        for line in ["there --help", "there --keep-going -h", "help there"] {
            let ending = ran(line, |_| panic!("{line} started the command"));
            assert_eq!(ending.exit, Exit::Done, "{line}: {}", ending.stderr);
            assert!(
                ending.stdout.starts_with("offload there  builds"),
                "{line} didn't answer there's help: {}",
                ending.stdout
            );
        }
    }

    #[test]
    fn help_past_its_end_is_the_programs() {
        for line in ["there . -- nix --help", "there --changed --help"] {
            let mut seen = None;
            let ending = ran(line, |context| {
                seen = Some(context.args.clone());
                Ok(())
            });
            assert_eq!(ending.exit, Exit::Done, "{line}: {}", ending.stderr);
            assert_eq!(seen, Some(words(line)), "{line} wasn't handed on whole");
        }
    }

    #[test]
    fn a_programs_v_doesnt_make_a_failure_verbose() {
        let ending = ran("there -v .", |_| {
            Err(Failure::new(Exit::Failed, "the build failed").caused_by("a cause"))
        });
        assert_eq!(ending.exit, Exit::Failed);
        assert!(
            !ending.stderr.contains("a cause"),
            "nix's -v made the kit's failure verbose: {}",
            ending.stderr
        );
    }

    #[test]
    fn a_subcommand_that_hands_nothing_on_still_has_its_flags_taken() {
        let mut seen = None;
        let ending = ran("lines --json", |context| {
            seen = Some((context.args.clone(), context.shared.format.clone()));
            Ok(())
        });
        assert_eq!(ending.exit, Exit::Done, "{}", ending.stderr);
        assert_eq!(seen, Some((words("lines"), Format::Json)));
    }
}
