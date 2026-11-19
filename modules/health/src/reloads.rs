//! Who asked systemd to reload, from the line it says as each ask comes:
//! a switch, or anything else. A restart is timed as a switch's
//! (`launches.rs`) only where a switch could have made it: on 14 November
//! a tool that updates itself had systemd reload, then restarted its own
//! service within a second, 12 times in the day, and each was counted as a
//! switch's restart that said no ready line.

/// What systemd says, as information, when a program asks it to reload:
/// "Reload requested from client PID 1520947 ('systemctl') (unit x.service)...".
const REQUESTED: &str = "Reload requested from client PID ";

/// The programs a switch has systemd reload from, as systemd names them,
/// cut to 15 characters: NixOS's switch-to-configuration, as the machine's
/// own switch and as the follower's, and home-manager's sd-switch. The
/// name is the one a program gives itself, so any program of the user's
/// can pass for a switch, a reload heard late included, and one passing
/// for a tool, heard late, can take back a restart already timed, or crowd
/// a stop taken back out past the most kept: each skews no more than
/// health's figures, by something that already runs as
/// the user, and only at the moment of the stop, since journald stamps
/// both clocks.
const SWITCHES: [&str; 3] = ["switch-to-confi", ".switch-to-conf", "sd-switch"];

/// Who asked for a reload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asker {
    /// A switch, whose restarts follow.
    Switch,
    /// Anything else, such as a tool restarting what it just updated.
    Other,
}

/// Who asked for the reload `message` says, when it says one.
#[must_use]
pub fn asker(message: &str) -> Option<Asker> {
    let asked = message.strip_prefix(REQUESTED)?;
    let (_, rest) = asked.split_once(" ('")?;
    let (program, _) = rest.split_once("')")?;
    Some(if SWITCHES.contains(&program) {
        Asker::Switch
    } else {
        Asker::Other
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::day::Day;
    use crate::day::tests::entry;
    use crate::journal::{Entry, Job, Origin};

    // The machine's systemd and each account's reloading apart.
    mod managers;

    fn requested(program: &str, unit: &str) -> String {
        format!("Reload requested from client PID 1520947 ('{program}') (unit {unit})...")
    }

    #[test]
    fn a_reload_is_a_switchs_only_when_a_switch_asked_for_it() {
        for program in SWITCHES {
            assert_eq!(
                asker(&requested(program, "home-manager-jitsusama.service")),
                Some(Asker::Switch),
                "a reload {program} asked for wasn't a switch's"
            );
        }
        assert_eq!(
            asker(&requested("systemctl", "janitor.service")),
            Some(Asker::Other),
            "a reload a tool asked for was a switch's"
        );
        assert_eq!(
            asker("Reloading finished in 412 ms."),
            None,
            "a line that asks for no reload was read as one"
        );
    }

    #[test]
    fn a_service_a_tool_restarted_after_its_own_reload_is_no_switchs_restart() {
        let mut day = Day::new("2026-11-14");
        let job = |unit: &str, at, job| Entry {
            at,
            since_boot: at,
            job: Some(job),
            ..entry(unit, 6, "")
        };
        let reload = |at, program: &str| Entry {
            at,
            since_boot: at,
            origin: Origin::Systemd,
            ..entry("systemd", 5, &requested(program, "x.service"))
        };
        day.take(&reload(1_000_000, "systemctl"));
        day.take(&job("backend.service", 1_800_000, Job::Stopping));
        day.take(&job("backend.service", 1_900_000, Job::Started));
        day.take(&reload(5_000_000, "sd-switch"));
        day.take(&job("google.service", 6_000_000, Job::Stopping));
        day.take(&job("google.service", 6_064_000, Job::Started));
        day.take(&reload(300_000_000, "systemctl"));
        day.take(&reload(301_000_000, "sd-switch"));
        day.take(&job("notices.service", 302_000_000, Job::Stopping));
        day.take(&job("notices.service", 302_100_000, Job::Started));
        assert_eq!(
            day.launches.restarted,
            BTreeMap::from([
                ("google.service".to_owned(), vec![64]),
                ("notices.service".to_owned(), vec![100]),
            ]),
            "a restart a tool made after its own reload was counted as a switch's, or a \
             switch's was left out"
        );
        assert!(
            day.things.is_empty(),
            "a reload's lines were counted as warnings"
        );
    }

    /// The backend's job, in `boot`, with both clocks at `at`.
    fn backend(boot: &str, at: i64, job: Job) -> Entry {
        Entry {
            at,
            since_boot: at,
            job: Some(job),
            boot: boot.to_owned(),
            ..entry("backend.service", 6, "")
        }
    }

    /// A reload `program` asked for, in `boot`, with both clocks at `at`.
    fn reload_in(boot: &str, at: i64, program: &str) -> Entry {
        Entry {
            at,
            since_boot: at,
            boot: boot.to_owned(),
            origin: Origin::Systemd,
            ..entry("systemd", 5, &requested(program, "x.service"))
        }
    }

    /// An entry with its clocks set apart: the boot's `since_boot` and the
    /// wall clock's `at`.
    fn clocked(line: Entry, since_boot: i64, at: i64) -> Entry {
        Entry {
            at,
            since_boot,
            ..line
        }
    }

    #[test]
    fn a_reload_counts_only_for_a_stop_in_its_own_boot() {
        // A tool's reload just before a reboot, and a restart seconds into
        // the next boot: that reload went with its boot, so it can't make
        // the restart the tool's. A tool's own reload and restart later in
        // that boot is still its own.
        let mut after_a_tools_reload = Day::new("2026-11-17");
        let day = &mut after_a_tools_reload;
        day.take(&reload_in("before", 30_000_000, "systemctl"));
        day.take(&backend("after", 38_000_000, Job::Stopping));
        day.take(&backend("after", 38_064_000, Job::Started));
        day.take(&reload_in("after", 200_000_000, "systemctl"));
        day.take(&backend("after", 200_800_000, Job::Stopping));
        day.take(&backend("after", 200_900_000, Job::Started));
        // A switch just before a quick reboot, then a tool's reload and
        // restart of its own service in the next boot, under a minute later:
        // the switch's reload went with its boot, so the restart is the
        // tool's own.
        let mut after_a_switch = Day::new("2026-11-17");
        let day = &mut after_a_switch;
        day.take(&reload_in("before", 1_000_000, "sd-switch"));
        day.take(&reload_in("after", 40_000_000, "systemctl"));
        day.take(&backend("after", 40_800_000, Job::Stopping));
        day.take(&backend("after", 40_900_000, Job::Started));
        assert_eq!(
            [
                &after_a_tools_reload.launches.restarted,
                &after_a_switch.launches.restarted,
            ],
            [
                &BTreeMap::from([("backend.service".to_owned(), vec![64])]),
                &BTreeMap::new(),
            ],
            "a reload from the boot before decided whose a restart was"
        );
    }

    #[test]
    fn a_reload_counts_before_a_stop_on_the_boots_clock_and_near_it_on_the_walls() {
        let days: Vec<(&str, Vec<Entry>, Vec<u64>)> = vec![
            // A switch's reload while the clock ran an hour fast, which then
            // stepped back: five minutes on by the boot's clock, a tool's own
            // reload and restart is its own, though the wall clock puts the
            // switch's reload after it.
            (
                "an hour stepped back after a switch",
                vec![
                    clocked(reload_in("b", 0, "sd-switch"), 100_000_000, 3_700_000_000),
                    clocked(reload_in("b", 0, "systemctl"), 400_000_000, 100_000_000),
                    clocked(backend("b", 0, Job::Stopping), 400_800_000, 100_800_000),
                    clocked(backend("b", 0, Job::Started), 400_900_000, 100_900_000),
                ],
                vec![],
            ),
            // The same step, but the switch's own restart thirty seconds on by
            // the boot's clock, with a tool's reload just before it: the wall
            // clock puts the switch's reload an hour after the stop, which any
            // step back may, so the restart is still the switch's.
            (
                "a switch's restart after an hour stepped back",
                vec![
                    clocked(reload_in("b", 0, "sd-switch"), 100_000_000, 3_700_000_000),
                    clocked(reload_in("b", 0, "systemctl"), 125_000_000, 125_000_000),
                    clocked(backend("b", 0, Job::Stopping), 130_000_000, 130_000_000),
                    clocked(backend("b", 0, Job::Started), 130_100_000, 130_100_000),
                ],
                vec![100],
            ),
            // A switch, then two hours asleep, and a tool's own reload and
            // restart seconds after the resume: the boot's clock stood still
            // through the sleep, but the switch is hours before, so the restart
            // is the tool's own.
            (
                "a sleep after a switch",
                vec![
                    reload_in("b", 1_000_000, "sd-switch"),
                    clocked(reload_in("b", 0, "systemctl"), 5_000_000, 7_201_000_000),
                    clocked(backend("b", 0, Job::Stopping), 5_800_000, 7_201_800_000),
                    clocked(backend("b", 0, Job::Started), 5_900_000, 7_201_900_000),
                ],
                vec![],
            ),
            // A tool's reload, then the clock stepped back under a second as it
            // syncs, then the tool's restart of its own service: the wall clock
            // puts the stop before the reload, the boot's clock just after, so
            // the restart is still the tool's own.
            (
                "a sync stepping back",
                vec![
                    reload_in("b", 10_000_000, "systemctl"),
                    clocked(backend("b", 0, Job::Stopping), 10_500_000, 9_600_000),
                    clocked(backend("b", 0, Job::Started), 10_600_000, 9_700_000),
                ],
                vec![],
            ),
            // A tool's reload, then the laptop asleep for two hours within the
            // second, and a service restarted just after the resume: the
            // boot's clock puts the reload under a second before, but the wall
            // clock two hours, so the restart isn't the tool's.
            (
                "a sleep after a tool's reload",
                vec![
                    reload_in("b", 1_000_000, "systemctl"),
                    clocked(backend("b", 0, Job::Stopping), 1_800_000, 7_201_800_000),
                    clocked(backend("b", 0, Job::Started), 1_900_000, 7_201_900_000),
                ],
                vec![100],
            ),
        ];
        each_restarted(days);
    }

    /// Take each case's entries in a day of its own, in the order given,
    /// and check the backend's restarts it times.
    fn each_restarted(days: Vec<(&str, Vec<Entry>, Vec<u64>)>) {
        for (what, entries, restarted) in days {
            let mut day = Day::new("2026-11-17");
            for entry in &entries {
                day.take(entry);
            }
            let expected = if restarted.is_empty() {
                BTreeMap::new()
            } else {
                BTreeMap::from([("backend.service".to_owned(), restarted)])
            };
            assert_eq!(
                day.launches.restarted, expected,
                "after {what}, the wrong reload decided whose a restart was"
            );
        }
    }

    #[test]
    fn a_reload_heard_out_of_order_live_still_finds_its_stop() {
        // Heard live, the reloads' follower can run ahead of the jobs'.
        let days: Vec<(&str, Vec<Entry>, Vec<u64>)> = vec![
            // A tool's reload heard before a stop systemd wrote just before it,
            // as the live followers can: the reload came after the stop, so it
            // can't make that restart the tool's own.
            (
                "a reload heard before an earlier stop",
                vec![
                    reload_in("b", 10_500_000, "systemctl"),
                    backend("b", 10_000_000, Job::Stopping),
                    backend("b", 10_100_000, Job::Started),
                ],
                vec![100],
            ),
            // A tool's reload and its own restart, with a second reload just
            // after the stop heard before it: the first reload is still the
            // stop's, so the restart is still the tool's own.
            (
                "a second reload heard before the stop",
                vec![
                    reload_in("b", 10_000_000, "systemctl"),
                    reload_in("b", 11_000_000, "systemctl"),
                    backend("b", 10_800_000, Job::Stopping),
                    backend("b", 10_900_000, Job::Started),
                ],
                vec![],
            ),
            // A tool's reload and its own restart, with a switch's reload just
            // after the stop heard before it: the switch's comes after the
            // stop, so the restart is still the tool's own.
            (
                "a switch's reload heard before an earlier tool's stop",
                vec![
                    reload_in("b", 10_000_000, "systemctl"),
                    reload_in("b", 11_000_000, "sd-switch"),
                    backend("b", 10_800_000, Job::Stopping),
                    backend("b", 10_900_000, Job::Started),
                ],
                vec![],
            ),
            // A tool's reload, then a reload from the next boot merged before
            // the tool's stop, as a replay ordered by the wall clock can. The
            // next boot's is further into its boot than ten minutes past the
            // tool's, yet the tool's reload is still its boot's, so the
            // restart is its own.
            (
                "a reload of the next boot among the boot before's lines",
                vec![
                    reload_in("a", 10_000_000, "systemctl"),
                    clocked(reload_in("z", 0, "systemctl"), 700_000_000, 10_200_000),
                    backend("a", 10_800_000, Job::Stopping),
                    backend("a", 10_900_000, Job::Started),
                ],
                vec![],
            ),
            // A switch's reload, a tool's reload late in its window and a stop,
            // then a second switch's reload more than its window after the
            // first, heard before the stop: the first is still the stop's, so
            // the restart is still the switch's.
            (
                "a switch's reload a window on heard before the stop",
                vec![
                    reload_in("b", 1_000_000, "sd-switch"),
                    reload_in("b", 112_000_000, "systemctl"),
                    reload_in("b", 200_000_000, "sd-switch"),
                    backend("b", 115_000_000, Job::Stopping),
                    backend("b", 115_100_000, Job::Started),
                ],
                vec![100],
            ),
        ];
        each_restarted(days);
    }

    #[test]
    fn a_reload_heard_after_its_stop_still_makes_the_restart_the_tools() {
        // Heard live, the jobs' follower can run ahead of the reloads'.
        let days: Vec<(&str, Vec<Entry>, Vec<u64>)> = vec![
            // A tool's reload heard after the stop it caused, before the
            // start: the restart is the tool's own.
            (
                "a tool's reload heard after its stop",
                vec![
                    backend("b", 10_800_000, Job::Stopping),
                    reload_in("b", 10_000_000, "systemctl"),
                    backend("b", 10_900_000, Job::Started),
                ],
                vec![],
            ),
            // Heard after the start too: the restart already timed is the
            // tool's own, so it goes.
            (
                "a tool's reload heard after its restart",
                vec![
                    backend("b", 10_800_000, Job::Stopping),
                    backend("b", 10_900_000, Job::Started),
                    reload_in("b", 10_000_000, "systemctl"),
                ],
                vec![],
            ),
            // A switch's reload before both: the restart is still the
            // switch's, whenever the tool's is heard.
            (
                "a tool's reload heard late in a switch",
                vec![
                    reload_in("b", 1_000_000, "sd-switch"),
                    backend("b", 10_800_000, Job::Stopping),
                    backend("b", 10_900_000, Job::Started),
                    reload_in("b", 10_000_000, "systemctl"),
                ],
                vec![100],
            ),
            // A tool's reload more than ten seconds before the stop, or
            // after it, or in another boot, heard late: none made it.
            (
                "a tool's reload heard late, too early for the stop",
                vec![
                    backend("b", 30_000_000, Job::Stopping),
                    backend("b", 30_100_000, Job::Started),
                    reload_in("b", 10_000_000, "systemctl"),
                ],
                vec![100],
            ),
            (
                "a tool's reload heard late, after the stop",
                vec![
                    backend("b", 30_000_000, Job::Stopping),
                    backend("b", 30_100_000, Job::Started),
                    reload_in("b", 30_050_000, "systemctl"),
                ],
                vec![100],
            ),
            (
                "a tool's reload heard late from another boot",
                vec![
                    backend("b", 10_800_000, Job::Stopping),
                    backend("b", 10_900_000, Job::Started),
                    reload_in("z", 10_000_000, "systemctl"),
                ],
                vec![100],
            ),
            // Before the stop on the boot's clock, but a sleep between them
            // on the wall's: the reload can't have made it.
            (
                "a tool's reload heard late, a sleep before the stop",
                vec![
                    clocked(backend("b", 0, Job::Stopping), 10_800_000, 7_210_800_000),
                    clocked(backend("b", 0, Job::Started), 10_900_000, 7_210_900_000),
                    reload_in("b", 10_000_000, "systemctl"),
                ],
                vec![100],
            ),
            // Two restarts of the backend, the second the tool's: only the
            // second goes when its reload is heard late.
            (
                "a tool's reload heard late after a switch's restart before",
                vec![
                    reload_in("b", 1_000_000, "sd-switch"),
                    backend("b", 2_000_000, Job::Stopping),
                    backend("b", 2_064_000, Job::Started),
                    backend("b", 300_800_000, Job::Stopping),
                    backend("b", 300_900_000, Job::Started),
                    reload_in("b", 300_000_000, "systemctl"),
                ],
                vec![64],
            ),
        ];
        each_restarted(days);
    }

    #[test]
    fn a_switchs_reload_heard_after_a_tools_late_one_keeps_its_restart() {
        let days: Vec<(&str, Vec<Entry>, Vec<u64>)> = vec![
            // A tool's reload, then a switch's, both before the stop and
            // heard after it, in the order written: the switch's makes the
            // restart its own again, whether the tool's reload took back
            // the stop, the restart its start then timed, or the restart
            // already timed.
            (
                "a switch's reload heard after a tool's took the stop",
                vec![
                    backend("b", 12_000_000, Job::Stopping),
                    reload_in("b", 10_000_000, "systemctl"),
                    reload_in("b", 11_000_000, "sd-switch"),
                    backend("b", 12_100_000, Job::Started),
                ],
                vec![100],
            ),
            (
                "a switch's reload heard after a tool's took the stop, then its start",
                vec![
                    backend("b", 12_000_000, Job::Stopping),
                    reload_in("b", 10_000_000, "systemctl"),
                    backend("b", 12_100_000, Job::Started),
                    reload_in("b", 11_000_000, "sd-switch"),
                ],
                vec![100],
            ),
            (
                "a switch's reload heard after a tool's took the restart",
                vec![
                    backend("b", 12_000_000, Job::Stopping),
                    backend("b", 12_100_000, Job::Started),
                    reload_in("b", 10_000_000, "systemctl"),
                    reload_in("b", 11_000_000, "sd-switch"),
                ],
                vec![100],
            ),
            // A switch's reload written after the stop, heard after a
            // tool's that took it: it didn't make the stop, so the restart
            // stays the tool's.
            (
                "a switch's reload after the stop a tool's took",
                vec![
                    backend("b", 12_000_000, Job::Stopping),
                    backend("b", 12_100_000, Job::Started),
                    reload_in("b", 10_000_000, "systemctl"),
                    reload_in("b", 13_000_000, "sd-switch"),
                ],
                vec![],
            ),
            // A tool's reload heard after the restart's ready line: too
            // late, so the restart, timed to its ready line, stays.
            (
                "a tool's reload heard after the restart's ready line",
                vec![
                    backend("b", 10_800_000, Job::Stopping),
                    backend("b", 10_900_000, Job::Started),
                    Entry {
                        at: 11_000_000,
                        since_boot: 11_000_000,
                        boot: "b".to_owned(),
                        ..entry("backend", 6, "backend: ready in 5 ms")
                    },
                    reload_in("b", 10_000_000, "systemctl"),
                ],
                vec![200],
            ),
            // A tool's reload heard after a newer restart of the service
            // was timed: too late for the older, which stays a switch's,
            // though the newer is taken back.
            (
                "a tool's reload heard after a newer restart",
                vec![
                    backend("b", 10_800_000, Job::Stopping),
                    backend("b", 10_900_000, Job::Started),
                    backend("b", 11_000_000, Job::Stopping),
                    backend("b", 11_100_000, Job::Started),
                    reload_in("b", 10_000_000, "systemctl"),
                ],
                vec![100],
            ),
        ];
        each_restarted(days);
    }

    #[test]
    fn a_restart_given_back_keeps_its_place_and_its_ready_line() {
        let ready = |at| Entry {
            at,
            since_boot: at,
            boot: "b".to_owned(),
            ..entry("backend", 6, "backend: ready in 5 ms")
        };
        let served_while_taken = vec![
            backend("b", 12_000_000, Job::Stopping),
            backend("b", 12_100_000, Job::Started),
            reload_in("b", 10_000_000, "systemctl"),
            ready(12_300_000),
            reload_in("b", 11_000_000, "sd-switch"),
        ];
        let mut day = Day::new("2026-11-17");
        for entry in &served_while_taken {
            day.take(entry);
        }
        assert_eq!(
            day.launches.unstamped(),
            BTreeMap::new(),
            "a restart given back timed to its ready line was left without its stamp"
        );
        // Another service's jobs and ready line while the backend's restart
        // is taken back are its own, not the backend's.
        let other = |at, job| Entry {
            at,
            since_boot: at,
            job: Some(job),
            boot: "b".to_owned(),
            ..entry("other.service", 6, "")
        };
        let mut day = Day::new("2026-11-17");
        for entry in [
            backend("b", 12_000_000, Job::Stopping),
            backend("b", 12_100_000, Job::Started),
            reload_in("b", 10_000_000, "systemctl"),
            other(12_200_000, Job::Stopping),
            other(12_250_000, Job::Started),
            Entry {
                at: 12_280_000,
                since_boot: 12_280_000,
                boot: "b".to_owned(),
                ..entry("other", 6, "other: ready in 5 ms")
            },
            ready(12_300_000),
            reload_in("b", 11_000_000, "sd-switch"),
        ] {
            day.take(&entry);
        }
        assert_eq!(
            day.launches.restarted.get("backend.service"),
            Some(&vec![300]),
            "another service's job or ready line was the backend's while it was taken back"
        );
        let days: Vec<(&str, Vec<Entry>, Vec<u64>)> = vec![
            // A restart taken back, then a newer one timed before a
            // switch's reload gives the first back: it goes before the
            // newer, which still waits for its ready line.
            (
                "a restart given back after a newer one",
                vec![
                    backend("b", 12_000_000, Job::Stopping),
                    backend("b", 12_100_000, Job::Started),
                    reload_in("b", 10_000_000, "systemctl"),
                    backend("b", 50_000_000, Job::Stopping),
                    backend("b", 50_200_000, Job::Started),
                    reload_in("b", 11_000_000, "sd-switch"),
                    ready(50_500_000),
                ],
                vec![100, 500],
            ),
            // A restart taken back still hears its ready line, and is
            // given back timed to it.
            (
                "a ready line heard while the restart was taken back",
                served_while_taken,
                vec![300],
            ),
            // A job after it ends the restart's ready lines while it is
            // taken back too: the next program's ready isn't its.
            (
                "a ready line after the next job while the restart was taken back",
                vec![
                    backend("b", 12_000_000, Job::Stopping),
                    backend("b", 12_100_000, Job::Started),
                    reload_in("b", 10_000_000, "systemctl"),
                    backend("b", 25_000_000, Job::Stopping),
                    ready(25_500_000),
                    reload_in("b", 11_000_000, "sd-switch"),
                ],
                vec![100],
            ),
        ];
        each_restarted(days);
    }

    #[test]
    fn a_stop_a_tools_reload_made_as_it_was_heard_is_given_back() {
        let days: Vec<(&str, Vec<Entry>, Vec<u64>)> = vec![
            // A stop a tool's reload heard before it made its own, then a
            // switch's reload written before the stop and heard after it:
            // the restart is the switch's.
            (
                "a switch's reload heard after a stop a tool's made",
                vec![
                    reload_in("b", 10_000_000, "systemctl"),
                    backend("b", 12_000_000, Job::Stopping),
                    reload_in("b", 11_000_000, "sd-switch"),
                    backend("b", 12_100_000, Job::Started),
                ],
                vec![100],
            ),
            // A newer stop ends the wait of one a tool's reload made, so a
            // switch's reload heard later can't give the older back for a
            // start after a crash to time.
            (
                "a switch's reload heard after a newer stop and its start",
                vec![
                    reload_in("b", 10_000_000, "systemctl"),
                    backend("b", 12_000_000, Job::Stopping),
                    backend("b", 30_000_000, Job::Stopping),
                    backend("b", 30_100_000, Job::Started),
                    reload_in("b", 11_000_000, "sd-switch"),
                    backend("b", 40_000_000, Job::Started),
                ],
                vec![100],
            ),
        ];
        each_restarted(days);
    }

    #[test]
    fn a_start_written_before_its_stop_ends_nothing() {
        // As the midnight cases fenced and regiven hold for a day's turn,
        // a start written before its stop and heard after it was the run
        // before the stop's, within a day too: the start after times it.
        let days: Vec<(&str, Vec<Entry>, Vec<u64>)> = vec![
            (
                "a start written before its stop",
                vec![
                    backend("b", 12_000_000, Job::Stopping),
                    backend("b", 10_000_000, Job::Started),
                    backend("b", 16_000_000, Job::Started),
                ],
                vec![4_000],
            ),
            (
                "a start written before its stop while a tool's reload took it",
                vec![
                    backend("b", 13_000_000, Job::Stopping),
                    reload_in("b", 8_000_000, "systemctl"),
                    backend("b", 10_000_000, Job::Started),
                    reload_in("b", 9_000_000, "sd-switch"),
                    backend("b", 17_000_000, Job::Started),
                ],
                vec![4_000],
            ),
            // One without the boot's stamp says nothing of when, so it
            // ends the stop it can't time, and the start after times
            // nothing.
            (
                "a start without the boot's stamp",
                vec![
                    backend("b", 12_000_000, Job::Stopping),
                    clocked(backend("b", 0, Job::Started), 0, 12_050_000),
                    backend("b", 16_000_000, Job::Started),
                ],
                vec![],
            ),
            (
                "a start without the boot's stamp while a tool's reload took it",
                vec![
                    backend("b", 13_000_000, Job::Stopping),
                    reload_in("b", 8_000_000, "systemctl"),
                    clocked(backend("b", 0, Job::Started), 0, 13_050_000),
                    reload_in("b", 9_000_000, "sd-switch"),
                    backend("b", 17_000_000, Job::Started),
                ],
                vec![],
            ),
        ];
        each_restarted(days);
    }

    #[test]
    fn a_reload_only_systemd_tells() {
        let mut day = Day::new("2026-11-17");
        let job = |at, job| Entry {
            at,
            since_boot: at,
            job: Some(job),
            ..entry("google.service", 6, "")
        };
        // Said under systemd's name by anything but systemd's manager: the
        // timed work's run of a test, say.
        day.take(&Entry {
            at: 1_000_000,
            since_boot: 1_000_000,
            origin: Origin::Elsewhere,
            ..entry("systemd", 5, &requested("systemctl", "x.service"))
        });
        day.take(&job(1_800_000, Job::Stopping));
        day.take(&job(1_864_000, Job::Started));
        assert_eq!(
            day.launches.restarted,
            BTreeMap::from([("google.service".to_owned(), vec![64])]),
            "a job's made-up reload hid a restart as a tool's own"
        );
    }
}
