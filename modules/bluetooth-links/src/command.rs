//! bluetooth-links as a command, in the terminal kit's shape: what
//! `bluetooth-links --help` says, and which file the words it's given name.

use std::path::PathBuf;

use terminal_kit::{Exit, Failure, Help};

/// Where it writes unless its unit says otherwise.
const FILE: &str = "/run/bluetooth-links/links.json";

/// What `bluetooth-links --help` says: the file it keeps, and that it
/// runs until it's stopped.
pub fn help() -> Help {
    Help::new("bluetooth-links", "says how each Bluetooth link is doing")
        .example(
            "bluetooth-links",
            "writes its usual file every five seconds",
        )
        .example("bluetooth-links /tmp/links.json", "writes there instead")
        .argument("[file]", "where it writes; /run/bluetooth-links/links.json")
        .exit(Exit::Done, "never, as it runs until it's stopped")
}

/// The file `args` name, or its usual one when they name none.
pub fn file(args: &[String]) -> Result<PathBuf, Failure> {
    match args {
        [] => Ok(PathBuf::from(FILE)),
        [file] => Ok(PathBuf::from(file)),
        _ => Err(Failure::new(Exit::Usage, "bluetooth-links keeps one file")
            .because("it was given more than one")
            .try_this("bluetooth-links --help")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_keeps_the_file_it_is_given_or_its_usual_one() {
        assert_eq!(
            file(&[]).ok(),
            Some(PathBuf::from(FILE)),
            "no word, the usual file"
        );
        let given = ["/tmp/links.json".to_owned()];
        assert_eq!(
            file(&given).ok(),
            Some(PathBuf::from("/tmp/links.json")),
            "one word is the file"
        );
        let two = ["a".to_owned(), "b".to_owned()];
        assert!(file(&two).is_err(), "two words are asked wrongly");
    }
}
