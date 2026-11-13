//! Telling a merge's owner that the net reverted it, or quarantined a test
//! it broke when the revert wouldn't apply. The net says so only in its
//! findings, so on 13 November #3972's owner learned of its revert in #4083
//! from no one for half an hour, and of the week's 16 reverts the owners
//! heard of each only when a session happened to notice. A quarantine went
//! to the watcher as an alarm, which only passed it on. Cadence knows who
//! landed each merge from the mesh's landings, so it tells that session
//! itself, and asks the watcher only when no landing names one.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// How long a told merge is remembered: past the hour of findings read.
const REMEMBER: i64 = 24 * 3600;

/// A merge the net reverted or quarantined a test for, with the tests it
/// broke.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revert {
    pub owner_pr: u64,
    /// The PR that reverted it, or that listed its tests in quarantine.
    pub ended_in: u64,
    /// Whether the revert wouldn't apply, so its tests were quarantined and
    /// the owner fixes it forward.
    pub quarantined: bool,
    pub tests: Vec<String>,
    /// The log of the first run that failed, for the owner to read.
    pub log: Option<String>,
}

/// The merges the net reverted or quarantined for since `since`, one for
/// each PR that did it however many tests it was for, read from its
/// findings.
#[must_use]
pub fn reverts(findings: &Path, since: i64) -> Vec<Revert> {
    let mut read: Vec<Revert> = Vec::new();
    let mut kept = crate::net::kept_since(findings, since);
    kept.sort_by_key(|finding| finding["at"].as_i64());
    for finding in kept.iter().filter(|finding| finding["kind"] == "failed") {
        let (Some(owner_pr), Some(test)) = (finding["owner_pr"].as_u64(), finding["test"].as_str())
        else {
            continue;
        };
        let (ended_in, quarantined) = match (
            finding["revert_pr"].as_u64(),
            finding["quarantine_pr"].as_u64(),
        ) {
            (Some(pr), _) => (pr, false),
            (None, Some(pr)) => (pr, true),
            (None, None) => continue,
        };
        let log = finding["log"].as_str().map(str::to_owned);
        match read
            .iter_mut()
            .find(|one| one.ended_in == ended_in && one.owner_pr == owner_pr)
        {
            Some(one) => one.tests.push(test.to_owned()),
            None => read.push(Revert {
                owner_pr,
                ended_in,
                quarantined,
                tests: vec![test.to_owned()],
                log,
            }),
        }
    }
    read
}

/// The session that landed core's `pr`, by the newest landing naming it.
#[must_use]
pub fn owner(landings: &str, pr: u64) -> Option<String> {
    #[derive(Deserialize)]
    struct Landing {
        address: String,
        at: String,
        #[serde(rename = "ref")]
        names: String,
    }
    #[derive(Deserialize)]
    struct Landings {
        landings: Vec<Landing>,
    }
    let named = format!("core.nix#{pr}");
    serde_json::from_str::<Landings>(landings)
        .ok()?
        .landings
        .into_iter()
        .filter(|landing| landing.names == named)
        // Each `at` is an RFC 3339 moment in UTC, so they sort as text.
        .max_by(|a, b| a.at.cmp(&b.at))
        .map(|landing| landing.address)
}

/// The merges already told, by the PR that ended each and its owner's, and
/// when.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Told {
    #[serde(default)]
    pub reverts: BTreeMap<String, i64>,
}

/// A line to send: to the owner's session, or to the watcher when no
/// landing names who merged it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tell {
    pub to: Option<String>,
    /// What it is remembered under in [`Told`].
    pub key: String,
    pub text: String,
}

/// The merges not yet told, each once, remembered in `told`.
#[must_use]
pub fn due(reverts: &[Revert], landings: &str, told: &mut Told, now: i64) -> Vec<Tell> {
    told.reverts.retain(|_, at| now - *at < REMEMBER);
    let mut out = Vec::new();
    for revert in reverts {
        let key = format!("{}:{}", revert.ended_in, revert.owner_pr);
        if told.reverts.contains_key(&key) {
            continue;
        }
        told.reverts.insert(key.clone(), now);
        let to = owner(landings, revert.owner_pr);
        let text = said_to(revert, to.is_some());
        out.push(Tell { to, key, text });
    }
    out
}

/// What the owner is told of `revert`, or the watcher when no landing
/// `named` the owner.
fn said_to(revert: &Revert, named: bool) -> String {
    let owner = revert.owner_pr;
    let ended = revert.ended_in;
    let tests = said(&revert.tests);
    let log = revert
        .log
        .as_ref()
        .map(|log| format!(" The first failing run's log is {log}."))
        .unwrap_or_default();
    let what = if revert.quarantined {
        format!(
            "couldn't revert it, so it quarantined {tests} in #{ended} naming #{owner} its \
             owner.{log} Fix it forward, passing under run-subtests --there, and take its \
             subtests off the entry in tools/merge-net/quarantine.json."
        )
    } else {
        format!(
            "reverted it in #{ended}.{log} Reland it once they pass under run-subtests --there."
        )
    };
    if named {
        format!("Cadence: the net found your #{owner} broke {tests} and {what}")
    } else {
        format!(
            "Cadence: no landing names who merged #{owner}, so tell its owner: the net found it \
             broke {tests} and {what}"
        )
    }
}

/// Names read as a list: "a", "a and b", "a, b and c".
fn said(names: &[String]) -> String {
    match names.split_last() {
        None => String::new(),
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn a_revert_or_quarantine_is_read_once_with_every_test_it_was_for() {
        let findings = std::env::temp_dir().join(format!("cadence-reverts-{}", std::process::id()));
        fs::create_dir_all(&findings).expect("the folder wasn't made");
        let keep = |name: &str, finding: serde_json::Value| {
            fs::write(findings.join(name), finding.to_string()).expect("a finding wasn't kept");
        };
        let failed =
            |at: i64, test: &str, revert_pr: serde_json::Value, held: serde_json::Value| {
                serde_json::json!({
                    "kind": "failed", "at": at, "test": test, "owner_pr": 3972,
                    "revert_pr": revert_pr, "quarantine_pr": held,
                    "log": format!("/logs/{test}-2.log"),
                })
            };
        let none = serde_json::Value::Null;
        keep(
            "90-aaaa-vm-a.json",
            failed(90, "vm-a", serde_json::json!(4000), none.clone()),
        );
        keep(
            "200-aaaa-vm-b.json",
            failed(200, "vm-b", serde_json::json!(4083), none.clone()),
        );
        keep(
            "210-aaaa-vm-c.json",
            failed(210, "vm-c", serde_json::json!(4083), none.clone()),
        );
        keep(
            "220-aaaa-vm-d.json",
            failed(220, "vm-d", none.clone(), serde_json::json!(4093)),
        );
        keep(
            "230-aaaa-vm-e.json",
            failed(230, "vm-e", none.clone(), none),
        );
        let read = reverts(&findings, 100);
        fs::remove_dir_all(&findings).expect("the folder wasn't cleared");
        assert_eq!(
            read,
            [
                Revert {
                    owner_pr: 3972,
                    ended_in: 4083,
                    quarantined: false,
                    tests: vec!["vm-b".to_owned(), "vm-c".to_owned()],
                    log: Some("/logs/vm-b-2.log".to_owned()),
                },
                Revert {
                    owner_pr: 3972,
                    ended_in: 4093,
                    quarantined: true,
                    tests: vec!["vm-d".to_owned()],
                    log: Some("/logs/vm-d-2.log".to_owned()),
                }
            ],
            "a revert or quarantine since then wasn't read once with its tests, or an older one, \
             or a finding that ended in neither, was"
        );
    }

    const LANDINGS: &str = r#"{"v":1,"landings":[
        {"address":"OLDONE","at":"2026-11-12T10:00:00Z","ref":"core.nix#3972"},
        {"address":"Y2YBTS","at":"2026-11-13T10:26:30Z","ref":"core.nix#3972"},
        {"address":"OTHERS","at":"2026-11-13T11:00:00Z","ref":"agentic-harness.pi#3972"}
    ]}"#;

    #[test]
    fn a_merges_owner_is_the_session_that_landed_it_last() {
        assert_eq!(
            owner(LANDINGS, 3972).as_deref(),
            Some("Y2YBTS"),
            "the newest landing of core's PR didn't name its owner"
        );
        assert_eq!(
            owner(LANDINGS, 1),
            None,
            "a PR no landing names had an owner"
        );
    }

    fn revert(owner_pr: u64, ended_in: u64, quarantined: bool) -> Revert {
        Revert {
            owner_pr,
            ended_in,
            quarantined,
            tests: vec!["vm-b".to_owned(), "vm-c".to_owned()],
            log: Some("/logs/vm-b-2.log".to_owned()),
        }
    }

    #[test]
    fn each_revert_is_told_to_its_owner_once() {
        let mut told = Told::default();
        let tells = due(
            &[revert(3972, 4083, false), revert(1, 4090, false)],
            LANDINGS,
            &mut told,
            1000,
        );
        assert_eq!(
            tells.iter().map(|t| t.to.as_deref()).collect::<Vec<_>>(),
            [Some("Y2YBTS"), None],
            "a revert wasn't told to its owner, or to the watcher when none landed it"
        );
        assert!(
            tells[0].text.contains("#3972")
                && tells[0].text.contains("#4083")
                && tells[0].text.contains("vm-b and vm-c")
                && tells[0].text.contains("/logs/vm-b-2.log"),
            "the owner wasn't told what was reverted, for what, and where to read it: {}",
            tells[0].text
        );
        assert!(
            due(&[revert(3972, 4083, false)], LANDINGS, &mut told, 1300).is_empty(),
            "a revert was told twice"
        );
    }

    #[test]
    fn a_quarantine_asks_its_owner_to_fix_forward() {
        let mut told = Told::default();
        let tells = due(&[revert(3972, 4093, true)], LANDINGS, &mut told, 1000);
        assert!(
            tells.len() == 1
                && tells[0].text.contains("quarantined vm-b and vm-c in #4093")
                && tells[0].text.contains("Fix it forward")
                && tells[0].text.contains("quarantine.json"),
            "the owner wasn't asked to fix its quarantined tests forward: {tells:?}"
        );
    }
}
