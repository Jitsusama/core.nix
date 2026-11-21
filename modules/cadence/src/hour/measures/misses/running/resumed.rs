//! When each pi host on this machine last resumed its conversations, so
//! the hour can wait out a resume before counting quests no cell runs:
//! the node lists a host's cells again only as each resumes.

use super::host_unit;
use crate::command;

/// How long a host may be reloading before it's taken as stuck rather than
/// resuming: a reload starts nothing again, so an hour is well past one.
const STUCK: i64 = 60 * 60;

/// How long a host may be starting or stopping, counted from its first try,
/// before it's taken as stuck. Under modules/pi/host.nix's limits a try
/// that fails takes up to 20 minutes to time out, 150 seconds to stop and
/// 5 to restart, so three tries take about 68 minutes, and a third that
/// succeeds resumes in 8 to 9 more. A machine that raises the start's
/// limit needs this raised with it.
const LOOPING: i64 = 90 * 60;

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
                        "-p",
                        "ActiveExitTimestamp",
                        "-p",
                        "InactiveExitTimestamp",
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
/// for `LOOPING` or longer, or reloading for `STUCK`, or with no time it
/// began to, is taken as stuck rather than resuming, and so is an active
/// host with no time it became so: each leaves the hour judged as it was.
///
/// A start or stop is timed from when the host last left active, or left
/// inactive if that was later, rather than from its last change of state.
/// A start that fails is restarted at once, its unit going straight back
/// to activating (`RestartMode = direct` in modules/pi/host.nix) and never
/// through inactive, so a host that fails to start for good is taken as
/// stuck `LOOPING` after it first tried, where its last change of state is
/// never minutes old. On systemd 262 a unit that never said it was ready
/// kept the `InactiveExitTimestamp` of its first try through three direct
/// restarts, while its `StateChangeTimestamp` moved with each. A host that
/// crashes once up is timed from leaving active, its
/// `InactiveExitTimestamp` as old as its last clean start. A reload doesn't restart, so it keeps
/// its last change of state.
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
    let lately =
        |since: Option<i64>, bound: i64| since.filter(|&since| now - since < bound).map(|_| now);
    match field("ActiveState")? {
        "activating" | "deactivating" => lately(
            at("ActiveExitTimestamp").max(at("InactiveExitTimestamp")),
            LOOPING,
        ),
        "reloading" => lately(at("StateChangeTimestamp"), STUCK),
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
                 ActiveEnterTimestamp={entered}\nStateChangeTimestamp=@1795199500\n\
                 ActiveExitTimestamp=@1795199400\nInactiveExitTimestamp=@1795199500\n"
            )
        };
        for (state, bound) in [
            ("activating", LOOPING),
            ("deactivating", LOOPING),
            ("reloading", STUCK),
        ] {
            assert_eq!(
                resumed(&shown(state, "@1795199500", "@1795100000"), now),
                Some(now),
                "a host {state}, its conversations coming or going, was read by its last start"
            );
            let since = |at: i64| {
                format!(
                    "ActiveState={state}\nStateChangeTimestamp=@{at}\n\
                     ActiveExitTimestamp=@{at}\nInactiveExitTimestamp=@{at}\n"
                )
            };
            assert_eq!(
                resumed(&since(now - bound), now),
                None,
                "a host {state} past its bound was read as resuming"
            );
            assert_eq!(
                resumed(&since(now - bound + 60), now),
                Some(now),
                "a host {state} a minute inside its bound was read as stuck"
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

    #[test]
    fn a_host_failing_to_start_is_timed_from_its_first_try_and_a_crash_from_leaving_active() {
        let now = 1_795_200_000;
        // A start that fails is restarted straight into activating, so its
        // last change of state is minutes old however long it has failed.
        let looping = |left_active: &str, left_inactive: i64| {
            format!(
                "ActiveState=activating\nStateChangeTimestamp=@{}\n\
                 ActiveExitTimestamp={left_active}\nInactiveExitTimestamp=@{left_inactive}\n",
                now - 300
            )
        };
        assert_eq!(
            resumed(
                &looping(&format!("@{}", now - LOOPING), now - 9 * LOOPING),
                now
            ),
            None,
            "a host failing to start since it last left active long ago was read as resuming"
        );
        assert_eq!(
            resumed(&looping("", now - LOOPING), now),
            None,
            "a host failing to start since its first try long ago was read as resuming"
        );
        assert_eq!(
            resumed(&looping("", now - 70 * 60), now),
            Some(now),
            "a host on its third try at the unit's limits was read as stuck"
        );
        assert_eq!(
            resumed(&looping(&format!("@{}", now - 9 * LOOPING), now - 600), now),
            Some(now),
            "a host started ten minutes ago after a long stop was read as stuck"
        );
        // A host up for days that crashes goes straight back to activating,
        // so only its leaving active is fresh.
        assert_eq!(
            resumed(&looping(&format!("@{}", now - 300), now - 9 * LOOPING), now),
            Some(now),
            "a host restarting after a crash, long after its last clean start, was read as stuck"
        );
        assert_eq!(
            resumed(&looping("", now - 600), now),
            Some(now),
            "a host's first start ten minutes ago was read as stuck"
        );
    }
}
