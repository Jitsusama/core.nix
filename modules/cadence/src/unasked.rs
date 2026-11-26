//! Git as cadence runs it, unattended on a timer: it never asks anyone
//! for a password or a passphrase. A remote wanting one fails at once,
//! saying so, rather than opening a prompt on Joel's desktop, where a
//! user unit's environment can name a graphical askpass for every program.

use std::process::{Command, Stdio};

/// What tells git and ssh where to ask, each left unset, so nothing is
/// asked; and the config another git passes its children, so none comes in
/// from whoever started cadence.
const UNSET: [&str; 4] = [
    "GIT_ASKPASS",
    "SSH_ASKPASS",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_COUNT",
];

/// Git, ready for its arguments, set never to ask: no terminal prompt, no
/// askpass from the environment or from any config, ssh never asking
/// either, and nothing to read from.
pub fn git() -> Command {
    let mut git = Command::new("git");
    git.args(["-c", "core.askPass="])
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("SSH_ASKPASS_REQUIRE", "never")
        .stdin(Stdio::null());
    for name in UNSET {
        git.env_remove(name);
    }
    git
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    #[test]
    fn git_never_asks_and_takes_no_askpass_from_anywhere() {
        let git = git();
        let envs: Vec<(&OsStr, Option<&OsStr>)> = git.get_envs().collect();
        let set = |name: &str| {
            envs.iter()
                .find(|(key, _)| *key == OsStr::new(name))
                .map(|(_, value)| *value)
        };
        assert_eq!(
            set("GIT_TERMINAL_PROMPT"),
            Some(Some(OsStr::new("0"))),
            "git may prompt on a terminal"
        );
        assert_eq!(
            set("SSH_ASKPASS_REQUIRE"),
            Some(Some(OsStr::new("never"))),
            "ssh may ask through an askpass"
        );
        for name in UNSET {
            assert_eq!(
                set(name),
                Some(None),
                "{name} comes in from whoever started it"
            );
        }
        let args: Vec<&OsStr> = git.get_args().collect();
        assert_eq!(
            args,
            [OsStr::new("-c"), OsStr::new("core.askPass=")],
            "a config's askpass could still be asked"
        );
    }
}
