//! A restart across midnight: what the new day carries of the old, and
//! the old day's lines heard after it turned.

use super::*;

/// systemd's line of `service` stopping or started, in `boot`, at `at` on
/// both clocks.
fn job(boot: &str, id: &str, service: &str, at: i64) -> String {
    format!(
        r#"{{"__REALTIME_TIMESTAMP":"{at}","__MONOTONIC_TIMESTAMP":"{at}","_BOOT_ID":"{boot}",
        "PRIORITY":"6","SYSLOG_IDENTIFIER":"systemd",
        "_SYSTEMD_CGROUP":"/user.slice/user-1000.slice/user@1000.service/init.scope",
        "MESSAGE_ID":"{id}","USER_UNIT":"{service}","MESSAGE":"{service}"}}"#
    )
    .replace('\n', "")
}

/// systemd's line of a reload `asker` asked for, in `boot`, at `at` on
/// both clocks.
fn reload(boot: &str, asker: &str, at: i64) -> String {
    format!(
        r#"{{"__REALTIME_TIMESTAMP":"{at}","__MONOTONIC_TIMESTAMP":"{at}","_BOOT_ID":"{boot}",
        "PRIORITY":"5","SYSLOG_IDENTIFIER":"systemd",
        "_SYSTEMD_CGROUP":"/user.slice/user-1000.slice/user@1000.service/init.scope",
        "MESSAGE":"Reload requested from client PID 1 ('{asker}') (unit x.service)..."}}"#
    )
    .replace('\n', "")
}

#[test]
fn a_restart_across_midnight_is_the_new_days() {
    let into = tempfile::tempdir().expect("a folder");
    let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC);
    let midnight = NOON + DAY / 2;
    let second = 1_000_000;
    let stopping = journal::UNIT_STOPPING;
    let started = journal::UNIT_STARTED;
    // Each case is in a boot of its own, so no reload is another's.
    for line in [
        // A restart done before midnight is the old day's alone.
        job("a", stopping, "done.service", midnight - 60 * second),
        job("a", started, "done.service", midnight - 59_900_000),
        job("a", stopping, "google.service", midnight - 30 * second),
        // A tool's reload before midnight still makes a stop after it the
        // tool's.
        reload("b", "systemctl", midnight - 5 * second),
        // Stops before midnight that a tool's and a switch's reloads,
        // heard only after it, take back and give back.
        job("d", stopping, "ended.service", midnight - 3 * second),
        job("e", stopping, "back.service", midnight - 3 * second),
        warning(midnight + 1),
        job("b", stopping, "tooled.service", midnight + second),
        job("b", started, "tooled.service", midnight + 1_100_000),
        job("a", started, "google.service", midnight + 100_000),
        // A stop, and a tool's reload, before midnight, heard from
        // followers that lag behind the day's turn.
        job("a", stopping, "lagged.service", midnight - 20 * second),
        job("a", started, "lagged.service", midnight + 50_000),
        reload("c", "systemctl", midnight - second),
        job("c", stopping, "late.service", midnight + 7 * second),
        job("c", started, "late.service", midnight + 7_100_000),
        // A restart before midnight heard after it, then a start after a
        // crash, which no stop came before.
        job("a", stopping, "early.service", midnight - 40 * second),
        job("a", started, "early.service", midnight - 39_900_000),
        job("a", started, "early.service", midnight + 30 * second),
        // The same, its stop taken back and its start heard before a
        // switch's reload gives it back: the start ended it.
        reload("d", "systemctl", midnight - 8 * second),
        job("d", started, "ended.service", midnight - 2_900_000),
        reload("d", "sd-switch", midnight - 7 * second),
        job("d", started, "ended.service", midnight + 20 * second),
        // A switch's restart across midnight, its stop taken back and
        // given back before its start.
        reload("e", "systemctl", midnight - 8 * second),
        reload("e", "sd-switch", midnight - 7 * second),
        job("e", started, "back.service", midnight + second),
    ] {
        follower.take(&line).expect("a line");
    }
    follower.write().expect("the new day is written");
    assert_eq!(
        (
            read(into.path(), "2026-11-12").launches.restarted,
            read(into.path(), "2026-11-13").launches.restarted,
        ),
        (
            BTreeMap::from([("done.service".to_owned(), vec![100])]),
            BTreeMap::from([
                ("back.service".to_owned(), vec![4_000]),
                ("google.service".to_owned(), vec![30_100]),
                ("lagged.service".to_owned(), vec![20_050]),
            ]),
        ),
        "a stop or reload before midnight was lost to the new day, or a restart before it \
         counted in it"
    );
}
