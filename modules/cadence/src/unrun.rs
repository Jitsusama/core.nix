//! Telling a merge's owner that no Mac compared its Mac checks. The
//! net compares a Mac's checks at each merge whose gate passed without a
//! Mac, and keeps a `mac-unrun` finding naming the merge's PR and lets it
//! go when none compared them for a day, or at once when a Mac took them
//! twice and neither run finished (#4925). Nothing read that
//! finding, so a Mac check the change may have broken stayed unchecked and
//! no one heard. Cadence tells the session that landed the merge, as it
//! does of reverts, and asks the watcher only when no landing names one.

use std::path::Path;

use crate::reverted::{self, Tell, Told};

/// How long a told merge is remembered: past the hour of findings read.
const REMEMBER: i64 = 24 * 3600;

/// What the net names the check comparing a Mac's checks at a merge,
/// before the merge's first parent it compares against.
const CHECK: &str = "a-macs-checks-since-";

/// A merge whose Mac checks no Mac compared: none took them in a day,
/// or one that took them stalled twice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unrun {
    /// The PR the merge's subject names, when it names one.
    pub owner_pr: Option<u64>,
    pub commit: String,
    /// The merge's first parent, which its Mac checks compare against.
    pub base: String,
    /// Whether a Mac took them and they stalled twice, which may be a hang
    /// the change made, rather than no Mac taking them.
    pub stalled: bool,
    /// The log of a run a Mac failed before the ones that compared
    /// nothing, when one did.
    pub failed_log: Option<String>,
    /// The log of the last run that stalled, when they stalled.
    pub stalled_log: Option<String>,
}

/// The merges the net let go unrun on a Mac since `since`, oldest first,
/// read from its findings.
#[must_use]
pub fn unrun(findings: &Path, since: i64) -> Vec<Unrun> {
    let mut kept = crate::net::kept_since(findings, since);
    kept.sort_by_key(|finding| finding["at"].as_i64());
    kept.iter()
        .filter(|finding| finding["kind"] == "mac-unrun")
        .filter_map(|finding| {
            Some(Unrun {
                owner_pr: finding["owner_pr"].as_u64(),
                commit: finding["commit"].as_str()?.to_owned(),
                base: finding["test"].as_str()?.strip_prefix(CHECK)?.to_owned(),
                stalled: finding["stalled"] == true,
                failed_log: finding["failed_log"].as_str().map(str::to_owned),
                stalled_log: finding["log"].as_str().map(str::to_owned),
            })
        })
        .collect()
}

/// The unrun merges not yet told, each once, remembered in `told`.
#[must_use]
pub fn due(unrun: &[Unrun], landings: &str, told: &mut Told, now: i64) -> Vec<Tell> {
    told.unrun.retain(|_, at| now - *at < REMEMBER);
    let mut out = Vec::new();
    for merge in unrun {
        let key = format!("mac:{}", merge.commit);
        if told.unrun.contains_key(&key) {
            continue;
        }
        told.unrun.insert(key.clone(), now);
        let to = merge.owner_pr.and_then(|pr| reverted::owner(landings, pr));
        let text = said_to(merge, to.is_some());
        out.push(Tell { to, key, text });
    }
    out
}

/// What the owner is told of `merge`, or the watcher when no landing
/// `named` the owner.
fn said_to(merge: &Unrun, named: bool) -> String {
    let short = merge.commit.get(..12).unwrap_or(&merge.commit);
    // The net knows a stalled run was taken and didn't end by itself, not
    // why: past its two hours, gone silent, or stopped by a signal from
    // outside, so each is said as what may have been.
    let how = match (merge.stalled, &merge.failed_log) {
        (true, _) => format!(
            "a Mac took its Mac checks at {short} twice and both runs stopped unfinished: they \
             may hang, may take longer than the net's two hours, or something else stopped them"
        ),
        (false, Some(_)) => {
            format!("a Mac failed its Mac checks at {short} once, and no Mac compared them again")
        }
        (false, None) => format!("no Mac compared its Mac checks at {short}"),
    };
    let failed = match (merge.stalled, &merge.failed_log) {
        (true, Some(log)) => format!(" A Mac failed them once before that; its log is {log}."),
        (false, Some(log)) => format!(" The failure's log is {log}."),
        (_, None) => String::new(),
    };
    let failed = match (merge.stalled, &merge.stalled_log) {
        (true, Some(log)) => format!(" The last run's log is {log}.{failed}"),
        _ => failed,
    };
    let what = format!(
        "{how}. The net let them go unchecked.{failed} Build them on a Mac, in a tree at \
         {}: nix-offload there --changed --build --no-vm --base {} --system aarch64-darwin, \
         and fix forward if any fail.",
        merge.commit, merge.base
    );
    match (merge.owner_pr, named) {
        (Some(pr), true) => format!("Cadence: your #{pr} merged, but {what}"),
        (Some(pr), false) => format!(
            "Cadence: no landing names who merged #{pr}, so tell its owner: it merged, but {what}"
        ),
        (None, _) => format!(
            "Cadence: no PR is named by the merge {short}, so tell its owner: it merged, but \
             {what}"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn an_unrun_merge_is_read_with_its_pr_and_base() {
        let findings = std::env::temp_dir().join(format!("cadence-unrun-{}", std::process::id()));
        fs::create_dir_all(&findings).expect("the folder wasn't made");
        let keep = |name: &str, finding: serde_json::Value| {
            fs::write(findings.join(name), finding.to_string()).expect("a finding wasn't kept");
        };
        let unrun_at = |at: i64, commit: &str, owner_pr: serde_json::Value| {
            serde_json::json!({
                "kind": "mac-unrun", "test": "a-macs-checks-since-bbbb", "commit": commit,
                "owner_pr": owner_pr, "since": at - 86_400, "at": at,
            })
        };
        keep(
            "90-aaaa.json",
            unrun_at(90, "aaaa", serde_json::json!(4900)),
        );
        keep(
            "210-cccc.json",
            unrun_at(210, "cccc", serde_json::Value::Null),
        );
        let mut stalled = unrun_at(200, "dddd", serde_json::json!(4925));
        stalled["stalled"] = serde_json::json!(true);
        stalled["failed_log"] = serde_json::json!("/net/logs/dddd-1.log");
        stalled["log"] = serde_json::json!("/net/logs/dddd-2.log");
        keep("200-dddd.json", stalled);
        keep(
            "220-eeee.json",
            serde_json::json!({"kind": "failed", "test": "vm-a", "owner_pr": 4930, "at": 220}),
        );
        let read = unrun(&findings, 100);
        fs::remove_dir_all(&findings).expect("the folder wasn't cleared");
        assert_eq!(
            read,
            [
                Unrun {
                    owner_pr: Some(4925),
                    commit: "dddd".to_owned(),
                    base: "bbbb".to_owned(),
                    stalled: true,
                    failed_log: Some("/net/logs/dddd-1.log".to_owned()),
                    stalled_log: Some("/net/logs/dddd-2.log".to_owned()),
                },
                Unrun {
                    owner_pr: None,
                    commit: "cccc".to_owned(),
                    base: "bbbb".to_owned(),
                    stalled: false,
                    failed_log: None,
                    stalled_log: None,
                },
            ],
            "an unrun merge since then wasn't read, oldest first with its PR, base, stall \
             and failed run, or an older one or another kind of finding was"
        );
    }

    const LANDINGS: &str = r#"{"v":1,"landings":[
        {"address":"Y2YBTS","at":"2026-11-15T10:26:30Z","ref":"core.nix#4925"}
    ]}"#;

    fn merge(owner_pr: Option<u64>) -> Unrun {
        Unrun {
            owner_pr,
            commit: "fb4676a1c0ffee0123456789abcdef0123456789".to_owned(),
            base: "e279db1b".to_owned(),
            stalled: false,
            failed_log: None,
            stalled_log: None,
        }
    }

    #[test]
    fn a_failure_a_mac_saw_without_a_stall_is_told_as_a_failure() {
        let failed = said_to(
            &Unrun {
                failed_log: Some("/net/logs/fb46-1.log".to_owned()),
                ..merge(Some(4925))
            },
            true,
        );
        assert!(
            failed.contains("a Mac failed its Mac checks at fb4676a1c0ff once")
                && failed.contains("The failure's log is /net/logs/fb46-1.log.")
                && !failed.contains("no Mac compared its")
                && !failed.contains("may hang"),
            "a failure a Mac saw was told as no Mac comparing, or as a hang: {failed}"
        );
    }

    #[test]
    fn a_stall_and_a_failure_a_mac_saw_are_told_as_they_were() {
        let plain = said_to(&merge(Some(4925)), true);
        let stalled = said_to(
            &Unrun {
                stalled: true,
                failed_log: Some("/net/logs/fb46-1.log".to_owned()),
                stalled_log: Some("/net/logs/fb46-2.log".to_owned()),
                ..merge(Some(4925))
            },
            true,
        );
        assert!(
            plain.contains("no Mac compared")
                && !plain.contains("stalled")
                && !plain.contains("failed them"),
            "a merge no Mac took was told as more than that: {plain}"
        );
        assert!(
            stalled.contains("both runs stopped unfinished")
                && stalled.contains(
                    "may hang, may take longer than the net's two hours, or something else \
                     stopped them"
                )
                && !stalled.contains("no Mac compared")
                && stalled.contains("The last run's log is /net/logs/fb46-2.log.")
                && stalled.contains("its log is /net/logs/fb46-1.log"),
            "a stall or a failure a Mac saw was told as no Mac comparing: {stalled}"
        );
    }

    #[test]
    fn each_unrun_merge_is_told_to_its_owner_once_with_how_to_run_it() {
        let mut told = Told::default();
        let tells = due(
            &[merge(Some(4925)), merge(Some(1))],
            LANDINGS,
            &mut told,
            1000,
        );
        assert_eq!(
            tells.iter().map(|t| t.to.as_deref()).collect::<Vec<_>>(),
            [Some("Y2YBTS")],
            "an unrun merge wasn't told to its owner, or a second merge at the same commit was"
        );
        assert!(
            tells[0].text.contains("#4925")
                && tells[0].text.contains("at fb4676a1c0ff.")
                && tells[0].text.contains(
                    "nix-offload there --changed --build --no-vm --base e279db1b --system \
                     aarch64-darwin"
                ),
            "the owner wasn't told which merge went unrun and how to run its Mac checks: {}",
            tells[0].text
        );
        assert!(
            due(&[merge(Some(4925))], LANDINGS, &mut told, 1300).is_empty(),
            "an unrun merge was told twice"
        );
        assert_eq!(
            due(&[merge(Some(4925))], LANDINGS, &mut told, 1000 + REMEMBER).len(),
            1,
            "a merge told a day ago was still remembered"
        );
    }

    #[test]
    fn an_unrun_merge_no_landing_names_goes_to_the_watcher() {
        let mut told = Told::default();
        let tells = due(&[merge(Some(1)), merge(None)], "", &mut told, 1000);
        assert!(
            tells.len() == 1
                && tells[0].to.is_none()
                && tells[0].text.contains("so tell its owner"),
            "an unrun merge no landing names wasn't asked of the watcher: {tells:?}"
        );
    }
}
