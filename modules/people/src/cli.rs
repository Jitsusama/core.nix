//! The daemon's command line, in the terminal kit's shape: what
//! `people --help` says, and the words its unit hands it.

use std::path::PathBuf;

use terminal_kit::{Exit, Failure, Help, flags};

/// The flags it takes, all of them the kit's own, which the kit takes
/// before these words reach it: a flag left here was mistyped, and the
/// nearest of these is what was meant.
const FLAGS: [&str; 5] = ["--help", "--version", "--json", "--format", "--color"];

/// What `people --help` says: how its unit runs it, and how it ends.
pub fn help() -> Help {
    Help::new("people", "holds everyone Joel deals with for the shell")
        .example(
            "people SOCKET SOURCES WORDS",
            "as its unit runs it, systemd handing the socket",
        )
        .argument("SOCKET", "where the shell asks, unless systemd hands one")
        .argument("SOURCES", "the sources it runs, as home.nix writes them")
        .argument("[WORDS]", "the launcher's words, which stands are said in")
        .exit(Exit::Failed, "its socket couldn't be listened on")
        .exit(Exit::Config, "its sources couldn't be read")
}

/// Where it listens, the file its sources are in, and the launcher's
/// words, if it's handed them.
pub struct Asked<'a> {
    pub socket: PathBuf,
    pub sources: PathBuf,
    pub words: Option<&'a str>,
}

/// Its socket, its sources and its words, from the words its unit hands
/// it.
///
/// # Errors
///
/// Asked wrongly, 64, short of its socket or sources, given a word past
/// its words, or given a flag.
pub fn asked(args: &[String]) -> Result<Asked<'_>, Failure> {
    let wrongly = |what: String| Failure::new(Exit::Usage, what).try_this("people --help");
    if let Some(flag) = args.iter().find(|arg| arg.starts_with('-')) {
        return Err(flags::unknown("option", flag, &FLAGS).try_this("people --help"));
    }
    match args {
        [socket, sources, words @ ..] if words.len() < 2 => Ok(Asked {
            socket: PathBuf::from(socket),
            sources: PathBuf::from(sources),
            words: words.first().map(String::as_str),
        }),
        [_, _, _, extra, ..] => Err(wrongly(format!(
            "people takes no word past its WORDS, and {extra} is one"
        ))),
        [_] => Err(wrongly("people needs SOURCES after its SOCKET".into())),
        _ => Err(wrongly("people needs its SOCKET and SOURCES".into())),
    }
}
