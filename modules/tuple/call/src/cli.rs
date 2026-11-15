//! The daemon's command line, in the terminal kit's shape: what
//! `tuple-call --help` says, and the words its unit hands it.

use std::path::PathBuf;

use terminal_kit::{Exit, Failure, Help};

/// What `tuple-call --help` says: how its unit runs it, and how it ends.
pub fn help() -> Help {
    Help::new(
        "tuple-call",
        "holds the Tuple call Joel's in for the shell, and does its acts",
    )
    .example(
        "tuple-call SOCKET TUPLE",
        "as its unit runs it, systemd handing the socket",
    )
    .argument("SOCKET", "where the shell asks, unless systemd hands one")
    .argument("TUPLE", "the tuple command it follows and acts through")
    .exit(Exit::Failed, "its socket couldn't be listened on")
}

/// Where it listens, and the tuple command.
pub struct Asked {
    pub socket: PathBuf,
    pub tuple: String,
}

/// Its socket and the tuple command, from the words its unit hands it.
///
/// # Errors
///
/// Asked wrongly, 64, short of its socket or the tuple command, given a
/// word past them, or given a flag.
pub fn asked(args: &[String]) -> Result<Asked, Failure> {
    let wrongly = |what: String| Failure::new(Exit::Usage, what).try_this("tuple-call --help");
    if let Some(flag) = args.iter().find(|arg| arg.starts_with('-')) {
        return Err(wrongly(format!("{flag} isn't a flag it takes")));
    }
    match args {
        [socket, tuple] => Ok(Asked {
            socket: PathBuf::from(socket),
            tuple: tuple.clone(),
        }),
        [_, _, extra, ..] => Err(wrongly(format!(
            "tuple-call takes no word past its TUPLE, and {extra} is one"
        ))),
        [_] => Err(wrongly("tuple-call needs TUPLE after its SOCKET".into())),
        [] => Err(wrongly("tuple-call needs its SOCKET and TUPLE".into())),
    }
}
