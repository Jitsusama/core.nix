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
/// subcommand are all the kit's, and a command that hands on every word
/// keeps them all, reading even its subcommands its own way.
pub(crate) fn split(help: &Help, mut args: Vec<String>) -> (Vec<String>, Vec<String>, bool) {
    if let Some(ends) = &help.hands_on {
        let helped = asks_help(&args, ends);
        return (Vec::new(), args, helped);
    }
    if help.subs.is_empty() {
        return (args, Vec::new(), false);
    }
    // Walked as the words come, flags and all, since the walk passes over
    // a flag as the shared flags are taken later: a word it refuses here
    // is refused again, said, once they have been.
    let Ok(walked) = tree::walk(help, &args) else {
        return (args, Vec::new(), false);
    };
    let (Some(at), Some(handing)) = (walked.handed, walked.chain.last()) else {
        return (args, Vec::new(), false);
    };
    let theirs = args.split_off(at + 1);
    let helped = asks_help(&theirs, handing.hands_on.as_deref().unwrap_or_default());
    (args, theirs, helped)
}

/// Whether handed words ask for help: `-h` or `--help` before `--` or any
/// of `ends`.
fn asks_help(words: &[String], ends: &[String]) -> bool {
    words
        .iter()
        .take_while(|word| *word != "--" && !ends.contains(word))
        .any(|word| word == "-h" || word == "--help")
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
    fn a_command_that_hands_every_word_on_keeps_them_all_and_ends_through_the_kit() {
        let wrapper = offload().hands_words_on(&[]);
        let mut seen = None;
        let ending = run(
            &wrapper,
            "1.0",
            &Environment::of([false; 3], &[]),
            words("--helper h --json --prefix p unheard"),
            |context| {
                seen = Some(context.args.clone());
                Err(Failure::passed(3, "the build failed"))
            },
        );
        assert_eq!(
            seen,
            Some(words("--helper h --json --prefix p unheard")),
            "the kit took or refused a word the command reads its own way"
        );
        assert_eq!(ending.code(), 3, "{}", ending.stderr);
        assert_eq!(ending.stderr, "\u{2715} the build failed\n");
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
