//! Git as cadence runs it, unattended on a timer: it never asks anyone
//! for a password or a passphrase. A remote wanting one fails at once,
//! saying so, rather than opening a prompt on Joel's desktop, where a
//! user unit's environment can name a graphical askpass for every program.
//! Nor does anything git hands a key or a token to: the ssh agent asks for
//! a key's PIN on the desktop from its own environment, and a credential
//! helper backed by a keyring can open an unlock dialog, so git reaches
//! neither. The remotes it fetches are public, needing no key or token.

use std::process::{Command, Stdio};

/// What tells git and ssh where to ask, each left unset, so nothing is
/// asked; the ssh agent and the ssh another git would run, so no key is
/// asked for; and the config another git passes its children, so none
/// comes in from whoever started cadence.
const UNSET: [&str; 7] = [
    "GIT_ASKPASS",
    "SSH_ASKPASS",
    "SSH_AUTH_SOCK",
    "GIT_SSH",
    "GIT_SSH_COMMAND",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_COUNT",
];

/// Ssh as git runs it here: it never asks, and reaches no agent, since an
/// account's `IdentityAgent` wins over an unset `SSH_AUTH_SOCK`. A caller
/// setting its own `core.sshCommand` keeps both options.
pub const SSH: &str = "ssh -o BatchMode=yes -o IdentityAgent=none";

/// The settings that keep git from asking: no askpass from any config, no
/// credential helper (an empty one clears the list config gave), and ssh
/// as [`SSH`] runs it.
const SETTINGS: [&str; 3] = [
    "core.askPass=",
    "credential.helper=",
    "core.sshCommand=ssh -o BatchMode=yes -o IdentityAgent=none",
];

/// Git, ready for its arguments, set never to ask: no terminal prompt, no
/// askpass or credential helper from the environment or from any config,
/// ssh never asking nor reaching an agent, and nothing to read from.
pub fn git() -> Command {
    let mut git = Command::new("git");
    for setting in SETTINGS {
        git.args(["-c", setting]);
    }
    git.env("GIT_TERMINAL_PROMPT", "0")
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
        let given: Vec<&OsStr> = SETTINGS
            .iter()
            .flat_map(|setting| [OsStr::new("-c"), OsStr::new(setting)])
            .collect();
        assert_eq!(
            args, given,
            "a config's askpass, credential helper or ssh could still be asked"
        );
        assert_eq!(
            SETTINGS
                .last()
                .and_then(|s| s.strip_prefix("core.sshCommand=")),
            Some(SSH),
            "git's ssh isn't the one callers are told it is"
        );
        for option in ["BatchMode=yes", "IdentityAgent=none"] {
            assert!(SSH.contains(option), "ssh lacks {option}");
        }
    }
}
