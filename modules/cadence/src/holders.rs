//! Who holds a unit, from its owner lines and its own name: the unit
//! watch, the day's failed units and health's units with no ready line
//! all ask, so a unit is held the same way wherever it's found. On 17
//! November 66 open findings had no holder, among them units a session
//! had named for itself and runs whose starter already reads their end.

use std::collections::BTreeMap;

use crate::findings::owner_of;

/// The owner of `unit`: whoever `owners` names for `unit:<unit>`, or,
/// failing that, for `unstamped:<unit>`, since the session that owns a
/// unit's readiness owns the unit; or else the session its name carries.
#[must_use]
pub fn owner(unit: &str, owners: &BTreeMap<String, String>) -> Option<String> {
    owner_of(&format!("unit:{unit}"), owners)
        .or_else(|| owner_of(&format!("unstamped:{unit}"), owners))
        .or_else(|| address(unit))
}

/// The session address a unit's name carries, as a session names the
/// units it starts for itself (`garbage-bound-mon-2af2dh.service`): a
/// part of six letters and digits, holding both, so a plain word such as
/// `camera` is never taken for one.
pub fn address(unit: &str) -> Option<String> {
    let stem = unit.rsplit_once('.').map_or(unit, |(stem, _)| stem);
    stem.split('-')
        .find(|part| {
            part.len() == 6
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
                && part.bytes().any(|b| b.is_ascii_digit())
                && part.bytes().any(|b| b.is_ascii_lowercase())
        })
        .map(str::to_ascii_uppercase)
}

/// Whether `unit` is one run started for whoever ran it, which reads its
/// end itself: merge-net's reruns of a check, and pi's commands run as a
/// transient unit, `run-p<pid>-i<id>.service`. On 17 November these
/// were among the failed units nobody held, so they're never told.
pub fn a_run(unit: &str) -> bool {
    if unit.starts_with("merge-net-rerun-") {
        return true;
    }
    let stem = unit.strip_suffix(".service").unwrap_or(unit);
    stem.strip_prefix("run-p")
        .and_then(|rest| rest.split_once("-i"))
        .is_some_and(|(pid, id)| {
            !pid.is_empty()
                && !id.is_empty()
                && pid.bytes().all(|b| b.is_ascii_digit())
                && id.bytes().all(|b| b.is_ascii_digit())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_is_the_nets_rerun_or_a_command_pi_ran() {
        assert!(
            a_run(
                "merge-net-rerun-worktree-github-Jitsusama-core-nix-jitsusama-repos-serve.service"
            ) && a_run("run-p178036-i50400995.service"),
            "a rerun of the net's and a command pi ran are runs"
        );
        assert!(
            !a_run("merge-net.service") && !a_run("run-parts.service"),
            "the net itself, and a unit only beginning with run-p"
        );
    }

    #[test]
    fn a_unit_named_for_a_session_is_that_sessions() {
        let owners: BTreeMap<String, String> = [(
            "unit:world-cadence-mt-2af2dh.service".to_owned(),
            "80ECO2".to_owned(),
        )]
        .into_iter()
        .collect();
        assert_eq!(
            owner("garbage-bound-mon-2af2dh.service", &owners).as_deref(),
            Some("2AF2DH"),
            "its address last"
        );
        assert_eq!(
            owner("b8rbnw-guard.service", &owners).as_deref(),
            Some("B8RBNW"),
            "its address first"
        );
        assert_eq!(
            owner("world-cadence-mt-2af2dh.service", &owners).as_deref(),
            Some("80ECO2"),
            "an owner line comes before the name"
        );
        for plain in [
            "camera.service",
            "system.service",
            "code-site.service",
            "t-crash-probe.service",
        ] {
            assert_eq!(owner(plain, &owners), None, "{plain}: no address in it");
        }
    }

    #[test]
    fn a_units_owner_is_its_own_or_its_readiness_owner() {
        let owners: BTreeMap<String, String> = [
            ("unit:google.service", "RTOIMQ"),
            ("unstamped:google.service", "OTHER1"),
            ("unstamped:slack-unreads.service", "RCTRF8"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
        assert_eq!(
            owner("google.service", &owners).as_deref(),
            Some("RTOIMQ"),
            "its own"
        );
        assert_eq!(
            owner("slack-unreads.service", &owners).as_deref(),
            Some("RCTRF8"),
            "its readiness owner"
        );
        assert_eq!(owner("people.service", &owners), None, "none named");
    }
}
