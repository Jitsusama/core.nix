//! The daemon's command line, in the terminal kit's shape: what
//! `tuple-call --help` says, and the words its unit hands it.

use std::path::PathBuf;
use std::time::Duration;

use terminal_kit::{Exit, Failure, Help};

/// How long an act waits on Tuple's command unless told otherwise. Five
/// seconds is far past what Tuple's app takes to answer one that waits
/// on nobody.
const ANSWERS_WITHIN: Duration = Duration::from_secs(5);

/// How long a command still running holds its act unless told otherwise,
/// so a command stuck on a Tuple since restarted leaves Joel asking again
/// within a minute.
const HELD_FOR: Duration = Duration::from_secs(60);

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
    .option(
        "--answers-within SECONDS",
        &format!(
            "how long an act waits on Tuple before saying it hasn't answered, {} unless given",
            ANSWERS_WITHIN.as_secs()
        ),
    )
    .option(
        "--held-for SECONDS",
        &format!(
            "how long a command still running holds its act, {} unless given",
            HELD_FOR.as_secs()
        ),
    )
    .exit(Exit::Failed, "its socket couldn't be listened on")
}

/// Where it listens, the tuple command, and how long its acts wait.
pub struct Asked {
    pub socket: PathBuf,
    pub tuple: String,
    pub answers_within: Duration,
    pub held_for: Duration,
}

/// Its socket and the tuple command, from the words its unit hands it,
/// and how long its acts wait, where a check gives it its own.
///
/// # Errors
///
/// Asked wrongly, 64, short of its socket or the tuple command, given a
/// word past them, a flag it doesn't take, or a wait that isn't a whole
/// number of seconds past none.
pub fn asked(args: &[String]) -> Result<Asked, Failure> {
    let wrongly = |what: String| Failure::new(Exit::Usage, what).try_this("tuple-call --help");
    let mut answers_within = ANSWERS_WITHIN;
    let mut held_for = HELD_FOR;
    let mut words = Vec::new();
    let mut given = args.iter();
    while let Some(arg) = given.next() {
        let wait = match arg.as_str() {
            "--answers-within" => &mut answers_within,
            "--held-for" => &mut held_for,
            flag if flag.starts_with('-') => {
                return Err(wrongly(format!("{flag} isn't a flag it takes")));
            }
            _ => {
                words.push(arg.clone());
                continue;
            }
        };
        *wait = given
            .next()
            .and_then(|seconds| seconds.parse().ok())
            .filter(|&seconds| seconds > 0)
            .map(Duration::from_secs)
            .ok_or_else(|| wrongly(format!("{arg} takes a whole number of seconds past none")))?;
    }
    match words.as_slice() {
        [socket, tuple] => Ok(Asked {
            socket: PathBuf::from(socket),
            tuple: tuple.clone(),
            answers_within,
            held_for,
        }),
        [_, _, extra, ..] => Err(wrongly(format!(
            "tuple-call takes no word past its TUPLE, and {extra} is one"
        ))),
        [_] => Err(wrongly("tuple-call needs TUPLE after its SOCKET".into())),
        [] => Err(wrongly("tuple-call needs its SOCKET and TUPLE".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The unit hands it no wait, so these are the waits Joel's acts get.
    #[test]
    fn an_act_waits_five_seconds_and_a_command_holds_it_a_minute_unless_told() {
        let Ok(asked) = asked(&["socket".into(), "tuple".into()]) else {
            panic!("its socket and tuple command alone were refused");
        };
        assert_eq!(
            (asked.answers_within, asked.held_for),
            (Duration::from_secs(5), Duration::from_secs(60)),
            "the waits its unit runs with moved"
        );
    }
}
