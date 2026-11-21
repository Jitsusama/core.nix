//! When each pi host on this machine last resumed its conversations, so
//! the hour can wait out a resume before counting quests no cell runs:
//! the node lists a host's cells again only as each resumes.

use super::hosts::host_unit;
use crate::command;

/// How long a host may be reloading before it's taken as stuck rather than
/// resuming: a reload starts nothing again, so an hour is well past one.
const STUCK: i64 = 60 * 60;

/// How many tries a host may take to start before it's taken as stuck.
const TRIES: i64 = 3;

/// What a host's starting or stopping may take beside its tries, for what
/// its unit's limits leave out, such as the jobs a start waits on, and a
/// start renewing its ask past its limit, which host.nix allows a host
/// still resuming conversations. Starts have taken 8 to 9 minutes.
const SLACK: i64 = 15 * 60;

/// How long a host may be starting or stopping when its unit's limits
/// can't be read, as when one has none: three tries at modules/pi/host.nix's
/// own, each up to 20 minutes to time out, 150 seconds to stop and 5 to
/// restart, take about 68 minutes.
const LOOPING: i64 = 90 * 60;

/// What `hosts_started` asks systemd of each host, and all `resumed`
/// reads: its tests read nothing else, so a name dropped here or spelt
/// two ways fails them rather than reading as never set.
const PROPERTIES: [&str; 9] = [
    "ActiveState",
    "ExecMainStartTimestamp",
    "ActiveEnterTimestamp",
    "StateChangeTimestamp",
    "ActiveExitTimestamp",
    "InactiveExitTimestamp",
    "TimeoutStartUSec",
    "TimeoutStopUSec",
    "RestartUSec",
];

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
                let mut asked = vec!["--user", "show", "--timestamp=unix"];
                for property in PROPERTIES {
                    asked.extend(["-p", property]);
                }
                asked.push(unit);
                resumed(&command("systemctl", &asked).ok()?, now)
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
/// for longer than `TRIES` tries at its unit's limits and `SLACK`, or
/// `LOOPING` when those can't be read, or reloading for `STUCK`, or with no
/// time it began to, is taken as stuck rather than resuming, and so is an
/// active host with no time it became so: each leaves the hour judged as it
/// was.
///
/// A start or stop is timed from when the host last left active, or left
/// inactive if that was later, rather than from its last change of state.
/// A start that fails is restarted at once, its unit going straight back
/// to activating (`RestartMode = direct` in modules/pi/host.nix) and never
/// through inactive, so a host that fails to start for good is taken as
/// stuck once its tries are spent, where its last change of state is never
/// minutes old. On systemd 262 a unit that never said it was ready kept
/// the `InactiveExitTimestamp` of its first try through three direct
/// restarts, while its `StateChangeTimestamp` moved with each. A host that
/// crashes once up is timed from leaving active, its
/// `InactiveExitTimestamp` as old as its last clean start. A reload doesn't
/// restart, so it keeps its last change of state.
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
    // A machine may give its hosts longer to start than host.nix does, so
    // each is held to its own unit's limits.
    let one_try: Option<i64> = ["TimeoutStartUSec", "TimeoutStopUSec", "RestartUSec"]
        .into_iter()
        .map(|name| field(name).and_then(span))
        .sum();
    let looping = one_try.map_or(LOOPING, |one| TRIES * one + SLACK);
    match field("ActiveState")? {
        "activating" | "deactivating" => lately(
            at("ActiveExitTimestamp").max(at("InactiveExitTimestamp")),
            looping,
        ),
        "reloading" => lately(at("StateChangeTimestamp"), STUCK),
        "active" => at("ActiveEnterTimestamp")
            .map(|entered| entered.max(at("ExecMainStartTimestamp").unwrap_or(entered))),
        _ => None,
    }
}

/// A span as `systemctl show` prints one, such as `2min 30s`, `3month` or
/// `2min 30.500000s`, in whole seconds rounded up, or `None` for one it
/// doesn't read, as `infinity`. Zero it prints as a bare `0`.
fn span(text: &str) -> Option<i64> {
    if text == "0" {
        return Some(0);
    }
    let mut micros: i64 = 0;
    let mut parts = 0;
    for part in text.split_whitespace() {
        let digits = part.find(|c: char| !c.is_ascii_digit() && c != '.')?;
        let (count, unit) = part.split_at(digits);
        // As systemd's own table has them, a month and a year in days of
        // 24 hours.
        let each: i64 = match unit {
            "us" | "µs" => 1,
            "ms" => 1_000,
            "s" => 1_000_000,
            "min" => 60_000_000,
            "h" => 3_600_000_000,
            "d" => 86_400_000_000,
            "w" => 604_800_000_000,
            "month" => 2_629_800_000_000,
            "y" => 31_557_600_000_000,
            _ => return None,
        };
        let (whole, fraction) = count.split_once('.').unwrap_or((count, ""));
        let mut part_micros = whole.parse::<i64>().ok()?.checked_mul(each)?;
        if !fraction.is_empty() {
            let scale = 10_i64.checked_pow(u32::try_from(fraction.len()).ok()?)?;
            let numerator = fraction.parse::<i64>().ok()?;
            part_micros = part_micros.checked_add(numerator.checked_mul(each)? / scale)?;
        }
        micros = micros.checked_add(part_micros)?;
        parts += 1;
    }
    (parts > 0).then(|| (micros + 999_999) / 1_000_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `resumed` of `shown`, which holds only what `hosts_started` asks.
    fn read(shown: &str, now: i64) -> Option<i64> {
        for line in shown.lines() {
            let name = line.split('=').next().unwrap_or_default();
            assert!(
                PROPERTIES.contains(&name),
                "{name} is read but never asked of systemd"
            );
        }
        resumed(shown, now)
    }

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
                read(&shown(state, "@1795199500", "@1795100000"), now),
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
                read(&since(now - bound), now),
                None,
                "a host {state} past its bound was read as resuming"
            );
            assert_eq!(
                read(&since(now - bound + 60), now),
                Some(now),
                "a host {state} a minute inside its bound was read as stuck"
            );
            assert_eq!(
                read(&format!("ActiveState={state}\n"), now),
                None,
                "a host {state} since no one knows when was read as resuming"
            );
        }
        assert_eq!(
            read(&shown("active", "@1795199000", "@1795199500"), now),
            Some(1_795_199_500),
            "an active host wasn't read from when it became so"
        );
        assert_eq!(
            read(&shown("active", "@1795199700", "@1795199500"), now),
            Some(1_795_199_700),
            "a process begun since the host became active was left out"
        );
        assert_eq!(
            read(&shown("active", "@1795199700", ""), now),
            None,
            "an active host with no time it became so was read as just started"
        );
        for state in ["failed", "inactive"] {
            assert_eq!(
                read(&shown(state, "@1795199900", "@1795199900"), now),
                None,
                "a host {state} was read as resuming"
            );
        }
        assert_eq!(read("", now), None, "nothing shown was read as a host");
    }

    /// A host restarted straight into activating, its last change of state
    /// minutes old however long it has failed, with its unit's limits.
    fn restarting(now: i64, left_active: &str, left_inactive: i64, limits: &str) -> String {
        format!(
            "ActiveState=activating\nStateChangeTimestamp=@{}\n\
             ActiveExitTimestamp={left_active}\nInactiveExitTimestamp=@{left_inactive}\n{limits}",
            now - 300
        )
    }

    #[test]
    fn a_host_failing_to_start_is_timed_from_its_first_try_and_a_crash_from_leaving_active() {
        let now = 1_795_200_000;
        let looping =
            |left_active: &str, left_inactive: i64| restarting(now, left_active, left_inactive, "");
        assert_eq!(
            read(
                &looping(&format!("@{}", now - LOOPING), now - 9 * LOOPING),
                now
            ),
            None,
            "a host failing to start since it last left active long ago was read as resuming"
        );
        assert_eq!(
            read(&looping("", now - LOOPING), now),
            None,
            "a host failing to start since its first try long ago was read as resuming"
        );
        assert_eq!(
            read(&looping(&format!("@{}", now - 9 * LOOPING), now - 600), now),
            Some(now),
            "a host started ten minutes ago after a long stop was read as stuck"
        );
        // A host up for days that crashes goes straight back to activating,
        // so only its leaving active is fresh.
        assert_eq!(
            read(&looping(&format!("@{}", now - 300), now - 9 * LOOPING), now),
            Some(now),
            "a host restarting after a crash, long after its last clean start, was read as stuck"
        );
        assert_eq!(
            read(&looping("", now - 600), now),
            Some(now),
            "a host's first start ten minutes ago was read as stuck"
        );
    }

    #[test]
    fn a_host_is_given_its_own_units_tries_to_start() {
        let now = 1_795_200_000;
        // host.nix's limits: three tries end about 68 minutes after the
        // first, at 22 min 35 s each.
        let given = |start: &str| {
            format!("TimeoutStartUSec={start}\nTimeoutStopUSec=2min 30s\nRestartUSec=5s\n")
        };
        for (start, one) in [("20min", 1_355), ("40min", 2_555)] {
            let bound = TRIES * one + SLACK;
            assert_eq!(
                read(
                    &restarting(now, "", now - TRIES * one + 60, &given(start)),
                    now
                ),
                Some(now),
                "a host on the last of its tries at a {start} start limit was read as stuck"
            );
            assert_eq!(
                read(&restarting(now, "", now - bound + 60, &given(start)), now),
                Some(now),
                "a host inside its tries and their slack at a {start} limit was read as stuck"
            );
            assert_eq!(
                read(&restarting(now, "", now - bound, &given(start)), now),
                None,
                "a host past its tries at a {start} start limit was read as resuming"
            );
        }
        assert_eq!(
            read(
                &restarting(now, "", now - LOOPING + 60, &given("infinity")),
                now
            ),
            Some(now),
            "a unit with no start limit wasn't held to the default"
        );
    }

    #[test]
    fn a_span_is_read_as_systemctl_prints_it() {
        for (text, seconds) in [
            ("20min", Some(1_200)),
            ("2min 30s", Some(150)),
            ("5s", Some(5)),
            ("100ms", Some(1)),
            ("1h 30min", Some(5_400)),
            ("0", Some(0)),
            ("2min 30.500000s", Some(151)),
            ("1.5s", Some(2)),
            ("2w", Some(1_209_600)),
            ("3month", Some(7_889_400)),
            ("1y", Some(31_557_600)),
            ("infinity", None),
            ("", None),
            ("5", None),
            (".s", None),
            ("5x", None),
        ] {
            assert_eq!(span(text), seconds, "{text:?} was misread");
        }
    }
}
