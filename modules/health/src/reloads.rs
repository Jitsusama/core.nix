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
/// own switch and as the follower's, and home-manager's sd-switch.
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
    use crate::journal::{Entry, Job};

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
            job: Some(job),
            ..entry(unit, 6, "")
        };
        let reload = |at, program: &str| Entry {
            at,
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
}
