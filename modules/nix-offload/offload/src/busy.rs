//! `nix-offload busy`: whether this Mac is past its room for more work,
//! by the rule a gate's comparisons keep to before sending it any
//! ([`there::Load::over`]), so work it starts for itself, such as
//! rebuilding its system after each merge, can wait on the same rule.
//!
//! A question, so it answers as `test` does: 0 while the Mac is past its
//! room, 1 while it has room, and 69 when it can't say its load, which a
//! caller holding work back counts as busy. It says the load it read, or
//! why it read none, on standard error either way.

use crate::there;
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};

/// What `-h` and `--help` print.
const HELP: &str = "Usage: nix-offload busy

Says whether this Mac is past its room for more work: either its last
minute's or last five minutes' load past three a processor. A caller
holding work back counts anything but 1 as busy.

EXIT
  0   it's past its room
  1   it has room
  64  asked wrongly
  69  it couldn't say its load";

/// Where a Mac keeps `sysctl`, run by its path so a launchd job's `PATH`,
/// or a `sysctl` earlier on it, never decides which runs.
const SYSCTL: &str = "/usr/sbin/sysctl";

/// Answers `busy` given `arguments`, the words after it.
pub fn busy(arguments: &[String]) -> ExitCode {
    if arguments
        .iter()
        .any(|word| word == "-h" || word == "--help")
    {
        println!("{HELP}");
        return ExitCode::SUCCESS;
    }
    if !arguments.is_empty() {
        eprintln!("nix-offload: busy takes no arguments\n{HELP}");
        return ExitCode::from(64);
    }
    let sysctl = if Path::new(SYSCTL).exists() {
        SYSCTL
    } else {
        "sysctl"
    };
    let (code, why) = judged(said_by(sysctl));
    eprintln!("nix-offload: {why}");
    ExitCode::from(code)
}

/// What `sysctl -n vm.loadavg hw.ncpu` says, run as `sysctl`, or why it
/// said nothing: it couldn't be run, or it failed, as on a machine
/// without those names, with what it said of why.
fn said_by(sysctl: &str) -> Result<String, String> {
    let answered = Command::new(sysctl)
        .args(["-n", "vm.loadavg", "hw.ncpu"])
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("{sysctl} couldn't be run: {error}"))?;
    if answered.status.success() {
        Ok(String::from_utf8_lossy(&answered.stdout).into_owned())
    } else {
        Err(format!(
            "{sysctl} ended {}: {}",
            answered.status,
            String::from_utf8_lossy(&answered.stderr).trim()
        ))
    }
}

/// The exit code `busy` answers with, and the line it says, given what
/// `sysctl` said or why it said nothing.
fn judged(said: Result<String, String>) -> (u8, String) {
    let said = match said {
        Ok(said) => said,
        Err(why) => return (69, format!("couldn't say its load, since {why}")),
    };
    match there::Load::parse(&said) {
        Some(load) if load.over() => (0, format!("past its room at {}", load.said())),
        Some(load) => (1, format!("room at {}", load.said())),
        None => (
            69,
            format!(
                "couldn't say its load, since sysctl said {:?}, not vm.loadavg and hw.ncpu",
                said.trim()
            ),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn judged_of(said: &str) -> (u8, String) {
        judged(Ok(said.to_owned()))
    }

    #[test]
    fn a_mac_past_three_a_processor_either_minute_or_five_is_busy() {
        assert_eq!(
            judged_of("{ 36.17 41.53 36.98 }\n12\n"),
            (
                0,
                "past its room at load 36.2 over a minute and 41.5 over five on 12 processors"
                    .to_owned()
            ),
            "a Mac at 41.5 over five on 12 processors wasn't busy"
        );
        assert_eq!(
            judged_of("{ 36.10 20.00 10.00 }\n12\n"),
            (
                0,
                "past its room at load 36.1 over a minute and 20.0 over five on 12 processors"
                    .to_owned()
            ),
            "a Mac at 36.1 over a minute on 12 processors wasn't busy"
        );
    }

    #[test]
    fn a_mac_at_three_a_processor_has_room() {
        assert_eq!(
            judged_of("{ 36.00 36.00 50.00 }\n12\n"),
            (
                1,
                "room at load 36.0 over a minute and 36.0 over five on 12 processors".to_owned()
            ),
            "a Mac at its room, the fifteen minutes' load aside, wasn't given room"
        );
    }

    #[test]
    fn a_load_that_cant_be_read_is_unavailable_not_room() {
        for said in ["", "{ 1.00 2.00 }\n12\n", "{ 1.00 2.00 3.00 }\n0\n"] {
            assert_eq!(
                judged_of(said).0,
                69,
                "an unread load wasn't said to be unknown: {said:?}"
            );
        }
        assert_eq!(
            judged(Err("sysctl ended exit status: 1: unknown oid".to_owned())),
            (
                69,
                "couldn't say its load, since sysctl ended exit status: 1: unknown oid".to_owned()
            ),
            "a sysctl that failed wasn't said to be why"
        );
    }

    #[test]
    fn sysctl_is_asked_for_both_names_and_its_failure_is_said() {
        let folder = tempfile::tempdir().unwrap();
        let sysctl = folder.path().join("sysctl");
        let path = sysctl.to_str().unwrap();
        crate::testing::executable(
            &sysctl,
            r#"#!/bin/sh
[ "$*" = '-n vm.loadavg hw.ncpu' ] || exit 2
printf '{ 1.00 2.00 3.00 }\n12\n'
"#,
        );
        assert_eq!(
            said_by(path),
            Ok("{ 1.00 2.00 3.00 }\n12\n".to_owned()),
            "sysctl wasn't asked for the load and processors"
        );
        crate::testing::executable(&sysctl, "#!/bin/sh\necho 'unknown oid' >&2\nexit 1\n");
        let failed = said_by(path).unwrap_err();
        assert!(
            failed.ends_with(": unknown oid"),
            "what a failed sysctl said was lost: {failed}"
        );
        let missing = folder.path().join("none");
        assert!(
            said_by(missing.to_str().unwrap()).is_err(),
            "a sysctl that isn't there was read as an answer"
        );
    }
}
