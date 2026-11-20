//! When each pi host on this machine last resumed its conversations, so
//! the hour can wait out a resume before counting quests no cell runs:
//! the node lists a host's cells again only as each resumes.

use super::host_unit;
use crate::command;

/// How long a host may be starting or stopping before it's taken as stuck
/// rather than resuming: a start's limit is 20 minutes, renewed while it
/// resumes, and a stop's 150 seconds, so an hour is past either.
const STUCK: i64 = 60 * 60;

/// When each pi host on this machine last resumed its conversations, as
/// `resumed` reads it, or `None` when the hosts couldn't be listed.
#[must_use]
pub fn hosts_started(now: i64) -> Option<Vec<i64>> {
    let units = command(
        "systemctl",
        &[
            "--user",
            "list-units",
            "--type=service",
            "--no-legend",
            "--plain",
            "pi-host-*",
        ],
    )
    .ok()?;
    Some(
        units
            .lines()
            .filter_map(|line| {
                line.split_whitespace()
                    .next()
                    .filter(|unit| host_unit(unit))
            })
            .filter_map(|unit| {
                let shown = command(
                    "systemctl",
                    &[
                        "--user",
                        "show",
                        "--timestamp=unix",
                        "-p",
                        "ActiveState",
                        "-p",
                        "ExecMainStartTimestamp",
                        "-p",
                        "ActiveEnterTimestamp",
                        "-p",
                        "StateChangeTimestamp",
                        unit,
                    ],
                )
                .ok()?;
                resumed(&shown, now)
            })
            .collect(),
    )
}

/// When a pi host, from its `systemctl show`, last resumed its
/// conversations, or `None` when it isn't running. A host tells systemd
/// it's ready only once it has resumed each, which under load has taken 8
/// to 9 minutes, so one still activating resumes `now`, and one active
/// resumed when it became so, or when its process began if that was later.
/// One deactivating is parking its conversations, as a restart does before
/// it starts again, so it resumes `now` too. A host starting or stopping
/// for `STUCK` or longer, or with no time it entered that state, is taken
/// as stuck rather than resuming, and so is an active host with no time it
/// became so: each leaves the hour judged as it was.
fn resumed(shown: &str, now: i64) -> Option<i64> {
    let field = |name: &str| {
        shown
            .lines()
            .find_map(|line| line.strip_prefix(name)?.strip_prefix('='))
            .map(str::trim)
    };
    let at = |name: &str| {
        field(name)
            .and_then(|at| at.strip_prefix('@')?.parse::<i64>().ok())
            .filter(|&at| at > 0)
    };
    match field("ActiveState")? {
        "activating" | "reloading" | "deactivating" => at("StateChangeTimestamp")
            .filter(|&changed| now - changed < STUCK)
            .map(|_| now),
        "active" => at("ActiveEnterTimestamp")
            .map(|entered| entered.max(at("ExecMainStartTimestamp").unwrap_or(entered))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_host_resumes_while_it_starts_or_stops_and_from_when_it_became_active() {
        let now = 1_795_200_000;
        let shown = |state: &str, began: &str, entered: &str| {
            format!(
                "ActiveState={state}\nExecMainStartTimestamp={began}\n\
                 ActiveEnterTimestamp={entered}\nStateChangeTimestamp=@1795199500\n"
            )
        };
        for state in ["activating", "deactivating", "reloading"] {
            assert_eq!(
                resumed(&shown(state, "@1795199500", "@1795100000"), now),
                Some(now),
                "a host {state}, its conversations coming or going, was read by its last start"
            );
            let stuck = format!(
                "ActiveState={state}\nStateChangeTimestamp=@{}\n",
                now - STUCK
            );
            assert_eq!(
                resumed(&stuck, now),
                None,
                "a host {state} for an hour was read as resuming"
            );
            assert_eq!(
                resumed(&format!("ActiveState={state}\n"), now),
                None,
                "a host {state} since no one knows when was read as resuming"
            );
        }
        assert_eq!(
            resumed(&shown("active", "@1795199000", "@1795199500"), now),
            Some(1_795_199_500),
            "an active host wasn't read from when it became so"
        );
        assert_eq!(
            resumed(&shown("active", "@1795199700", "@1795199500"), now),
            Some(1_795_199_700),
            "a process begun since the host became active was left out"
        );
        assert_eq!(
            resumed(&shown("active", "@1795199700", ""), now),
            None,
            "an active host with no time it became so was read as just started"
        );
        for state in ["failed", "inactive"] {
            assert_eq!(
                resumed(&shown(state, "@1795199900", "@1795199900"), now),
                None,
                "a host {state} was read as resuming"
            );
        }
        assert_eq!(resumed("", now), None, "nothing shown was read as a host");
    }
}
