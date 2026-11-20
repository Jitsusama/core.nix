//! What systemd says of a failed unit, and which failed units are runs
//! systemd-run started, whose starter reads their end rather than cadence.

use super::Failed;
use crate::holders::{red, started_by_systemd_run};

/// What the unit watch asks systemd of a failed unit, before its name.
pub const SHOW: [&str; 7] = [
    "--user",
    "show",
    "--timestamp=unix",
    "-p",
    "StateChangeTimestamp",
    "-p",
    "Transient,Description",
];

/// From what `systemctl show` says of `unit`: when it last changed state,
/// which for one failed is when it failed, and whether systemd-run started
/// it. A run needs systemd's word that it's transient, made over the bus
/// as systemd-run makes it, since a unit file may say any description.
/// Each key is read only when it's said once, so a description can't
/// carry a second.
pub fn shown_from(unit: &str, said: &str) -> (Option<i64>, bool) {
    let value = |key: &str| {
        let mut values = said
            .lines()
            .filter_map(|line| line.strip_prefix(key)?.strip_prefix('='));
        let first = values.next();
        first.filter(|_| values.next().is_none())
    };
    let since =
        value("StateChangeTimestamp").and_then(|at| at.trim().strip_prefix('@')?.parse().ok());
    let run = value("Transient") == Some("yes")
        && value("Description").is_some_and(|d| started_by_systemd_run(unit, d));
    (since, run)
}

/// Leaves out of `failed` each run systemd-run started that nobody
/// `held`: whoever started it reads its end, and a session that named it
/// for itself, or holds it by an owner line, is told as for any unit,
/// but for a red run, which has to fail.
pub fn leave_runs(failed: &mut Vec<Failed>, held: impl Fn(&str) -> bool) {
    failed.retain(|f| !f.run || (held(&f.unit) && !red(&f.unit)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn systemd_says_when_a_unit_failed_and_whether_systemd_run_started_it() {
        let at = "StateChangeTimestamp=@1795031296";
        let run = "Description=[systemd-run] /bin/bash -c \"a=b\"";
        let shown = |lines: &[&str]| shown_from("q16-rc47-build.service", &lines.join("\n"));
        assert_eq!(
            shown(&["Transient=yes", at, run]),
            (Some(1_795_031_296), true),
            "a transient run systemd-run described wasn't read as one, or lost when it failed"
        );
        assert_eq!(
            shown(&[run, "Transient=yes", at]),
            (Some(1_795_031_296), true),
            "the order systemd says its keys in mattered"
        );
        for (lines, why) in [
            (
                vec!["Transient=no", at, run],
                "a unit file saying systemd-run's description",
            ),
            (vec![at, run], "a unit systemd didn't say was transient"),
            (
                vec!["Transient=yes", at, "Description=Camera"],
                "a run that says what it's for",
            ),
        ] {
            assert_eq!(
                shown(&lines),
                (Some(1_795_031_296), false),
                "{why} was a run"
            );
        }
        assert_eq!(
            shown(&["Transient=yes", "StateChangeTimestamp=", run, at]),
            (None, true),
            "a moment said twice, one of them maybe in a description, was believed"
        );
        assert_eq!(shown(&[]), (None, false), "nothing said was a run");
    }

    #[test]
    fn a_run_systemd_run_started_is_left_out_unless_someone_holds_it() {
        let failed = |unit: &str, run: bool| Failed {
            unit: unit.into(),
            since: Some(50),
            run,
        };
        let mut read = vec![
            failed("q16-rc47-build.service", true),
            failed("y2ybts-race-timers-2044.service", true),
            failed("y2ybts-first-red2-1236.service", true),
            failed("camera.service", false),
            failed("red-light.service", false),
        ];
        leave_runs(&mut read, |unit| unit.starts_with("y2ybts-"));
        let units: Vec<&str> = read.iter().map(|f| f.unit.as_str()).collect();
        assert_eq!(
            units,
            [
                "y2ybts-race-timers-2044.service",
                "camera.service",
                "red-light.service"
            ],
            "a run nobody holds, or a red run, stayed, or one its session holds, or a \
             service, was left out"
        );
    }
}
