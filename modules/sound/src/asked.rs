//! What its unit hands the daemon, read in the terminal kit's shape, and
//! the help that says it.

use std::path::PathBuf;

use sound::naming::read_or_none;
use sound::rows::Words;
use terminal_kit::{Exit, Failure, Help};
use windows::marks::Marks;

/// What `sound --help` says: how its unit runs it, and how it ends.
pub fn help() -> Help {
    Help::new(
        "sound",
        "holds the sound server's devices ready for the shell",
    )
    .example(
        "sound SOCKET --vocabulary WORDS --marks MARKS",
        "as its unit runs it",
    )
    .argument("<socket>", "where the shell asks, unless systemd hands one")
    .option("--vocabulary WORDS", "the launcher's words its rows say")
    .option("--marks MARKS", "the marks its programs wear")
    .exit(Exit::Failed, "its socket couldn't be listened on")
}

/// Its socket, its words and its marks, from what its unit hands it.
pub fn asked(args: &[String]) -> Result<(PathBuf, Words, Marks), Failure> {
    let wrongly = || {
        Failure::new(
            Exit::Usage,
            "sound takes a socket, then --vocabulary and --marks, each with a file",
        )
        .try_this("sound --help")
    };
    let mut args = args.iter();
    let socket = args.next().map(PathBuf::from).ok_or_else(wrongly)?;
    // Without its words, a row says the vocabulary's ids for them, as
    // the launcher's Sound does, and without marks a program wears the
    // launcher's, rather than the daemon not starting.
    let (mut words, mut marks) = (Words::default(), Marks::default());
    while let Some(flag) = args.next() {
        match (flag.as_str(), args.next()) {
            ("--vocabulary", Some(path)) => words = read_or_none(path, Words::from_vocabulary),
            ("--marks", Some(path)) => marks = read_or_none(path, Marks::read),
            _ => return Err(wrongly()),
        }
    }
    Ok((socket, words, marks))
}
