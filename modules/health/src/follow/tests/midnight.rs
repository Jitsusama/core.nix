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

const SECOND: i64 = 1_000_000;
const MIDNIGHT: i64 = NOON + DAY / 2;

#[test]
fn a_restart_across_midnight_is_the_new_days() {
    let into = tempfile::tempdir().expect("a folder");
    let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC);
    let m = MIDNIGHT;
    let (stopping, started) = (journal::UNIT_STOPPING, journal::UNIT_STARTED);
    // Each case with a reload is in a boot of its own, so no reload is
    // another's, and boot a holds those with none. The day turns once, at
    // the warning; what comes before it is heard in the old day, what
    // comes after in the new.
    let heard_before_the_turn = [
        // done: restarted before midnight, the old day's alone.
        job("a", stopping, "done.service", m - 60 * SECOND),
        job("a", started, "done.service", m - 59_900_000),
        // google: its stop carried, so its start times it.
        job("a", stopping, "google.service", m - 30 * SECOND),
        // tooled: a tool's reload carried still makes the stop after it
        // the tool's.
        reload("b", "systemctl", m - 5 * SECOND),
        // ended, back, fenced, regiven: stops carried, for what follows.
        job("d", stopping, "ended.service", m - 3 * SECOND),
        job("e", stopping, "back.service", m - 3 * SECOND),
        job("h", stopping, "fenced.service", m - 3 * SECOND),
        job("i", stopping, "regiven.service", m - 3 * SECOND),
        // turned: its stop taken back before the turn, still carried.
        job("f", stopping, "turned.service", m - 3 * SECOND),
        reload("f", "systemctl", m - 8 * SECOND),
        // lost: restarted before midnight and taken back by a tool's
        // reload before the turn, which writes the old day without it.
        job("j", stopping, "lost.service", m - 3 * SECOND),
        job("j", started, "lost.service", m - 2_900_000),
        reload("j", "systemctl", m - 8 * SECOND),
    ];
    let heard_after_the_turn = [
        job("b", stopping, "tooled.service", m + SECOND),
        job("b", started, "tooled.service", m + 1_100_000),
        job("a", started, "google.service", m + 100_000),
        // lagged: a stop before midnight heard after it, still timed.
        job("a", stopping, "lagged.service", m - 20 * SECOND),
        job("a", started, "lagged.service", m + 50_000),
        // late: a tool's reload before midnight heard after it, alone in
        // its boot, makes the stop after it the tool's.
        reload("c", "systemctl", m - SECOND),
        job("c", stopping, "late.service", m + 7 * SECOND),
        job("c", started, "late.service", m + 7_100_000),
        // early: restarted before midnight, heard after it, then started
        // after a crash, which no stop came before: none.
        job("a", stopping, "early.service", m - 40 * SECOND),
        job("a", started, "early.service", m - 39_900_000),
        job("a", started, "early.service", m + 30 * SECOND),
        // ended: the same, its stop taken back and its start heard before
        // a switch's reload gives it back: the start ended it, so none.
        reload("d", "systemctl", m - 8 * SECOND),
        job("d", started, "ended.service", m - 2_900_000),
        reload("d", "sd-switch", m - 7 * SECOND),
        job("d", started, "ended.service", m + 20 * SECOND),
        // back: taken back and given back before its start: 4 s.
        reload("e", "systemctl", m - 8 * SECOND),
        reload("e", "sd-switch", m - 7 * SECOND),
        job("e", started, "back.service", m + SECOND),
        // turned: given back after the turn: 4 s.
        reload("f", "sd-switch", m - 7 * SECOND),
        job("f", started, "turned.service", m + SECOND),
        // fenced: a start written before its stop, the run before's, ends
        // nothing: 4 s.
        job("h", started, "fenced.service", m - 5 * SECOND),
        job("h", started, "fenced.service", m + SECOND),
        // regiven: the same while the stop is taken back: 4 s.
        reload("i", "systemctl", m - 8 * SECOND),
        job("i", started, "regiven.service", m - 5 * SECOND),
        reload("i", "sd-switch", m - 7 * SECOND),
        job("i", started, "regiven.service", m + SECOND),
        // lost: a switch's reload heard after the turn has nothing left to
        // give back, so it is in neither day.
        reload("j", "sd-switch", m - 7 * SECOND),
    ];
    let turn = [warning(m + 1)];
    for line in heard_before_the_turn
        .iter()
        .chain(&turn)
        .chain(&heard_after_the_turn)
    {
        follower.take(line).expect("a line");
    }
    follower.write().expect("the new day is written");
    let new_days = ["back", "fenced", "regiven", "turned"]
        .map(|unit| (format!("{unit}.service"), vec![4_000]))
        .into_iter()
        .chain([
            ("google.service".to_owned(), vec![30_100]),
            ("lagged.service".to_owned(), vec![20_050]),
        ]);
    assert_eq!(
        (
            read(into.path(), "2026-11-12").launches.restarted,
            read(into.path(), "2026-11-13").launches.restarted,
        ),
        (
            BTreeMap::from([("done.service".to_owned(), vec![100])]),
            new_days.collect::<BTreeMap<_, _>>(),
        ),
        "a stop or reload before midnight was lost to the new day, or a restart before it \
         counted in it"
    );
}

/// The same `line` without its boot's clock, as journald writes a line it
/// couldn't stamp.
fn unstamped(line: &str) -> String {
    let at = line
        .split(r#""__MONOTONIC_TIMESTAMP":""#)
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("a stamped line");
    line.replace(
        &format!(r#""__MONOTONIC_TIMESTAMP":"{at}""#),
        r#""__MONOTONIC_TIMESTAMP":"0""#,
    )
}

#[test]
fn a_start_of_the_day_before_without_its_stamp_ends_the_stop_it_cant_time() {
    let into = tempfile::tempdir().expect("a folder");
    let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC);
    let m = MIDNIGHT;
    let (stopping, started) = (journal::UNIT_STOPPING, journal::UNIT_STARTED);
    // carried: a stop carried, then a start of the day before with no
    // stamp, heard after the turn, ends it, so the start after it times
    // nothing. taken: the same while a tool's reload has the stop, given
    // back after the start.
    for line in [
        job("k", stopping, "carried.service", m - 3 * SECOND),
        job("l", stopping, "taken.service", m - 3 * SECOND),
        reload("l", "systemctl", m - 8 * SECOND),
        warning(m + 1),
        unstamped(&job("k", started, "carried.service", m - 2_900_000)),
        job("k", started, "carried.service", m + SECOND),
        unstamped(&job("l", started, "taken.service", m - 2_900_000)),
        reload("l", "sd-switch", m - 7 * SECOND),
        job("l", started, "taken.service", m + SECOND),
    ] {
        follower.take(&line).expect("a line");
    }
    follower.write().expect("the new day is written");
    assert_eq!(
        read(into.path(), "2026-11-13").launches.restarted,
        BTreeMap::new(),
        "a start without its stamp left the stop waiting for the next start"
    );
}

#[test]
fn a_switchs_reload_carried_keeps_a_restart_after_midnight_the_switchs() {
    let into = tempfile::tempdir().expect("a folder");
    let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC);
    let m = MIDNIGHT;
    let (stopping, started) = (journal::UNIT_STOPPING, journal::UNIT_STARTED);
    // A switch reloads, then something it runs that calls itself a tool,
    // both before the turn: the stop after midnight is still the switch's.
    for line in [
        reload("g", "sd-switch", m - 20 * SECOND),
        reload("g", "systemctl", m - 8 * SECOND),
        warning(m + 1),
        job("g", stopping, "switched.service", m + SECOND),
        job("g", started, "switched.service", m + 1_100_000),
    ] {
        follower.take(&line).expect("a line");
    }
    follower.write().expect("the new day is written");
    assert_eq!(
        read(into.path(), "2026-11-13").launches.restarted,
        BTreeMap::from([("switched.service".to_owned(), vec![100])]),
        "the switch's reload was left in the old day, so the tool's took its restart"
    );
}

#[test]
fn the_new_day_carries_from_midnight_and_a_late_line_changes_it() {
    let into = tempfile::tempdir().expect("a folder");
    let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC);
    let m = MIDNIGHT;
    let (stopping, started) = (journal::UNIT_STOPPING, journal::UNIT_STARTED);
    // The day turns well after midnight: slow's stop is within the restart
    // window of midnight, though not of the line that turned the day.
    for line in [
        job("a", stopping, "slow.service", m - 50 * SECOND),
        job("b", stopping, "tooled.service", m - 3 * SECOND),
        warning(m + 100 * SECOND),
        job("a", started, "slow.service", m + 60 * SECOND),
        job("b", started, "tooled.service", m + SECOND),
    ] {
        follower.take(&line).expect("a line");
    }
    follower.write().expect("the new day is written");
    // A tool's reload of the day before, heard only now, makes tooled's
    // restart its own: the day it changes is written again.
    follower
        .take(&reload("b", "systemctl", m - 8 * SECOND))
        .expect("a late line");
    follower.write().expect("the new day is written again");
    assert_eq!(
        read(into.path(), "2026-11-13").launches.restarted,
        BTreeMap::from([("slow.service".to_owned(), vec![110_000])]),
        "the new day carried from the line that turned it, not midnight, or a late line that \
         changed it left it unwritten"
    );
}
