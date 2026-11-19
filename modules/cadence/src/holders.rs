//! Who holds a unit, from its owner lines and its own name: the unit
//! watch, the day's failed units and health's units with no ready line
//! all ask `owner`, so a unit is held the same way wherever it's found,
//! except that the day reads a line for the finding's own key first. On
//! 17 November 66 open findings had no holder, among them units a
//! session had named for itself and runs whose starter reads their end.

use std::collections::{BTreeMap, BTreeSet};

use crate::findings::owner_of;

/// The owner of `unit`: whoever `owners` names for `unit:<unit>`, or,
/// failing that, for `unstamped:<unit>`, since the session that owns a
/// unit's readiness owns the unit; or else the session its name carries,
/// one of `known`.
#[must_use]
pub fn owner(
    unit: &str,
    owners: &BTreeMap<String, String>,
    known: &BTreeSet<String>,
) -> Option<String> {
    owner_of(&format!("unit:{unit}"), owners)
        .or_else(|| owner_of(&format!("unstamped:{unit}"), owners))
        .or_else(|| address(unit, known))
}

/// The session address a unit's name carries, as a session names the
/// units it starts for itself (`garbage-bound-mon-2af2dh.service`): a
/// dash-separated part that is one of the `known` sessions' addresses.
/// The mesh has to know it, since device names such as `virbr0` and
/// `tpmrm0` look like addresses too, and a plain word such as `camera`
/// would if an address were all letters.
pub fn address(unit: &str, known: &BTreeSet<String>) -> Option<String> {
    let stem = unit.rsplit_once('.').map_or(unit, |(stem, _)| stem);
    stem.split('-')
        .filter(|part| {
            part.len() == 6
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
        .map(str::to_ascii_uppercase)
        .find(|part| known.contains(part))
}

/// Whether `unit` is one run started for whoever ran it, which reads its
/// end itself: merge-net's reruns of a check, and an unnamed transient
/// unit, which systemd-run calls `run-p<pid>-i<id>.service` and pi's
/// commands are. An unnamed one says nobody's name, so no owner could be
/// told; whoever ran it has its exit. On 17 November these were among
/// the failed units nobody held, so they're never told.
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

/// What systemd-run starts a unit's description with when it's given
/// none of its own: `[systemd-run] ` and the command.
const SYSTEMD_RUN: &str = "[systemd-run] ";

/// Whether a unit described as `description` is one run that systemd-run
/// started, named or not, which whoever started it reads the end of, as
/// the sessions' `systemd-run --unit=NAME` runs are. On 18 November 9 of
/// the 13 failed units nobody held were such runs, all told to the watcher.
/// A run given a description of its own says what it's for, so it's told.
pub fn started_by_systemd_run(description: &str) -> bool {
    description.starts_with(SYSTEMD_RUN)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The sessions the mesh knows, as `owner` is given them.
    pub(crate) fn known() -> BTreeSet<String> {
        ["2AF2DH", "B8RBNW", "S99IA2", "KQZWVB"]
            .map(str::to_owned)
            .into()
    }

    #[test]
    fn a_run_is_the_nets_rerun_or_a_command_pi_ran() {
        for run in [
            "merge-net-rerun-worktree-github-Jitsusama-core-nix-jitsusama-repos-serve.service",
            "run-p178036-i50400995.service",
        ] {
            assert!(a_run(run), "{run}: a run, never told");
        }
        for unit in [
            "merge-net.service",
            "run-parts.service",
            "run-p12-ix.service",
            "run-px-i12.service",
            "run-p-i12.service",
        ] {
            assert!(!a_run(unit), "{unit}: not a run, so it's told");
        }
    }

    #[test]
    fn a_unit_systemd_run_described_is_a_run() {
        assert!(
            started_by_systemd_run("[systemd-run] /run/current-system/sw/bin/bash -c \"x\""),
            "systemd-run's own description wasn't a run"
        );
        for description in ["Measures the hour's shipping", "systemd-run", ""] {
            assert!(
                !started_by_systemd_run(description),
                "{description:?}: a unit that says what it's for was taken for a run"
            );
        }
    }

    #[test]
    fn a_unit_named_for_a_session_is_that_sessions() {
        let owners: BTreeMap<String, String> = [(
            "unit:world-cadence-mt-2af2dh.service".to_owned(),
            "80ECO2".to_owned(),
        )]
        .into_iter()
        .collect();
        let known = known();
        assert_eq!(
            owner("garbage-bound-mon-2af2dh.service", &owners, &known).as_deref(),
            Some("2AF2DH"),
            "its address last"
        );
        assert_eq!(
            owner("b8rbnw-guard.service", &owners, &known).as_deref(),
            Some("B8RBNW"),
            "its address first"
        );
        assert_eq!(
            owner("world-cadence-mt-2af2dh.service", &owners, &known).as_deref(),
            Some("80ECO2"),
            "an owner line comes before the name"
        );
        assert_eq!(
            owner("relink-kqzwvb.service", &owners, &known).as_deref(),
            Some("KQZWVB"),
            "an address of letters alone"
        );
        for plain in [
            "camera.service",
            "system.service",
            "code-site.service",
            "t-crash-probe.service",
            "libvirt-virbr0.service",
            "tpm-tpmrm0.service",
            "guard-2af2dhx.service",
            "guard-2AF2DH.service",
        ] {
            assert_eq!(
                owner(plain, &owners, &known),
                None,
                "{plain}: no address the mesh knows in it"
            );
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
            owner("google.service", &owners, &known()).as_deref(),
            Some("RTOIMQ"),
            "its own"
        );
        assert_eq!(
            owner("slack-unreads.service", &owners, &known()).as_deref(),
            Some("RCTRF8"),
            "its readiness owner"
        );
        assert_eq!(
            owner("people.service", &owners, &known()),
            None,
            "none named"
        );
    }
}
