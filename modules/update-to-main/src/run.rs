//! Running the programs an update needs, with nothing of the environment
//! root was started in: no git configuration but what's given here, no
//! prompts, and a fixed path, so what runs depends only on the config.

use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Stdio};

/// How many of a failed command's last lines an answer carries.
pub const TAIL: usize = 20;

/// A command that didn't succeed: what it was, and the end of what it said.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    pub why: String,
    pub lines: Vec<String>,
}

impl Failure {
    pub fn new(why: impl Into<String>) -> Failure {
        Failure {
            why: why.into(),
            lines: Vec::new(),
        }
    }
}

/// A command with a clean environment: `home` for programs that want one,
/// and a path that finds the system's own programs.
pub fn command(program: &Path, home: &Path) -> Command {
    let mut command = Command::new(program);
    command
        .env_clear()
        .env("HOME", home)
        .env(
            "PATH",
            "/run/current-system/sw/bin:/usr/bin:/bin:/usr/sbin:/sbin",
        )
        .env("LANG", "C.UTF-8")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null());
    // The system's certificates, where both NixOS and nix-darwin keep
    // them, for git's and Nix's fetches over HTTPS.
    if Path::new(CERTIFICATES).exists() {
        command
            .env("SSL_CERT_FILE", CERTIFICATES)
            .env("NIX_SSL_CERT_FILE", CERTIFICATES);
    }
    command
}

const CERTIFICATES: &str = "/etc/ssl/certs/ca-certificates.crt";

/// Runs a command to its end, giving what it printed, or a failure naming
/// `what` with the last lines of everything it said.
pub fn output<I, S>(mut command: Command, what: &str, args: I) -> Result<String, Failure>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let out = command
        .args(args)
        .output()
        .map_err(|e| Failure::new(format!("{what}: {e}")))?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned());
    }
    let mut said = String::from_utf8_lossy(&out.stdout).into_owned();
    said.push_str(&String::from_utf8_lossy(&out.stderr));
    Err(Failure {
        why: format!("{what} failed ({})", out.status),
        lines: tail(&said),
    })
}

/// The last lines of what a command said, blank ones left out.
pub fn tail(said: &str) -> Vec<String> {
    let lines: Vec<&str> = said.lines().filter(|l| !l.trim().is_empty()).collect();
    let from = lines.len().saturating_sub(TAIL);
    lines[from..].iter().map(|l| (*l).to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tail_keeps_the_last_lines_that_say_something() {
        let said: String = (1..=30).map(|n| format!("{n}\n\n")).collect();
        let kept = tail(&said);
        assert_eq!(kept.len(), TAIL);
        assert_eq!(kept.first().map(String::as_str), Some("11"));
        assert_eq!(kept.last().map(String::as_str), Some("30"));
    }
}
