//! What wifid is asked on its command line: its help, and that its unit
//! starts it with no words, so anything else is refused before a card, a
//! bus or a file is touched.

use terminal_kit::{Exit, Failure, Help};

/// What `wifid --help` says.
pub fn help() -> Help {
    Help::new(
        "wifid",
        "joins Wi-Fi networks and finds the ones that want a sign-in",
    )
    .example("wifid", "as its unit runs it, as root")
    .exit(Exit::Failed, "it couldn't start, or stopped, and says why")
}

/// That wifid was started with no words, as its unit starts it.
///
/// # Errors
///
/// Asked wrongly, 64, given any word or flag.
pub fn words(args: &[String]) -> Result<(), Failure> {
    match args.first() {
        None => Ok(()),
        Some(word) => {
            let what = if word.starts_with('-') {
                format!("{word} isn't a flag wifid takes")
            } else {
                format!("wifid takes no words, and {word} is one")
            };
            Err(Failure::new(Exit::Usage, what).try_this("wifid --help"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{help, words};
    use terminal_kit::{Environment, Exit};

    /// How the kit ends wifid started with `said`, its body reading only
    /// its words.
    fn started(said: &[&str]) -> (Exit, String) {
        let ending = terminal_kit::run(
            &help(),
            "0.1.0",
            &Environment::of([false; 3], &[]),
            said.iter().map(|word| (*word).to_owned()).collect(),
            |context| words(&context.args),
        );
        (ending.exit, ending.stderr)
    }

    #[test]
    fn its_unit_starts_it_with_no_words() {
        assert_eq!(
            started(&[]).0,
            Exit::Done,
            "the kit refused its unit's start"
        );
    }

    #[test]
    fn a_start_with_any_word_ends_64_naming_its_help() {
        for said in [&["start"][..], &["--debug"], &["-x"], &["wlan0", "more"]] {
            let (exit, refused) = started(said);
            assert_eq!(
                exit,
                Exit::Usage,
                "{said:?} wasn't refused as asked wrongly: {refused}"
            );
            assert!(
                refused.contains("wifid --help"),
                "{said:?}'s refusal didn't name its help: {refused}"
            );
        }
    }
}
