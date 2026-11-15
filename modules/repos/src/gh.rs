//! A token from gh's own sign-in, for the few repositories a machine lists
//! that GitHub's App can't see. That token reads whatever its person can,
//! so repos keeps its reader narrow: it asks gh afresh for each refresh,
//! holds the token in memory alone, and never says it, in an error, an
//! answer or the file a refresh keeps (github.rs says where it goes).

use std::ffi::OsStr;
use std::process::{Command, Stdio};

use crate::github::Token;

/// The words gh is asked with, fixed, so nothing a caller says reaches
/// gh's own and the token comes back on its output, never in arguments.
pub const ASKED: [&str; 4] = ["auth", "token", "--hostname", "github.com"];

/// The command asking `gh` for its token.
pub fn command(gh: &OsStr) -> Command {
    let mut command = Command::new(gh);
    command
        .args(ASKED)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    command
}

/// The token `command` answers, or why there's none, said without what it
/// printed, since a gh that fails may still have printed a token.
pub fn token(mut command: Command) -> Result<Token, String> {
    let answer = command
        .output()
        .map_err(|why| format!("gh couldn't be asked for its sign-in: {why}"))?;
    if !answer.status.success() {
        return Err(match answer.status.code() {
            Some(code) => format!("gh has no sign-in for github.com (it exited {code})"),
            None => "gh has no sign-in for github.com (it was stopped)".to_owned(),
        });
    }
    let said = String::from_utf8(answer.stdout).unwrap_or_default();
    let token = said.strip_suffix('\n').unwrap_or(&said);
    if token.is_empty() || !token.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err("gh answered something that isn't a token".to_owned());
    }
    Ok(Token::new(token.to_owned()))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A token planted where a real one would be, to be looked for after.
    pub(crate) const PLANTED: &str = "gho_planted0123456789abcdefghijklmnopqr";

    /// A stand-in gh running `script` in the shell running the tests,
    /// which is the one a build's sandbox has.
    pub(crate) fn gh(script: &str) -> Command {
        let shell = std::env::var_os("BASH").unwrap_or_else(|| "sh".into());
        let mut command = Command::new(shell);
        command
            .args(["-c", script])
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        command
    }

    #[test]
    fn gh_is_asked_in_fixed_words_for_github_alone() {
        let command = command(OsStr::new("gh"));
        let asked: Vec<&OsStr> = command.get_args().collect();
        assert_eq!(asked, ASKED.map(OsStr::new), "gh was asked in other words");
    }

    #[test]
    fn the_token_gh_gives_is_carried_only_in_a_header() {
        let token = token(gh(&format!("printf '{PLANTED}\\n'"))).expect("gh gives one");
        assert_eq!(token.header(), format!("bearer {PLANTED}"));
        let printed = format!("{token:?}");
        assert!(
            !printed.contains(PLANTED),
            "printing the token showed it: {printed}"
        );
    }

    #[test]
    fn a_gh_that_fails_is_said_without_what_it_printed() {
        let why = token(gh(&format!("printf '{PLANTED}\\n'; exit 4"))).expect_err("it failed");
        assert_eq!(why, "gh has no sign-in for github.com (it exited 4)");
    }

    #[test]
    fn a_gh_answering_other_than_a_token_is_refused_unsaid() {
        for script in ["true", &format!("printf 'a {PLANTED}\\n'")] {
            let why = token(gh(script)).expect_err("there's no token");
            assert_eq!(
                why, "gh answered something that isn't a token",
                "{script} was taken"
            );
        }
    }
}
