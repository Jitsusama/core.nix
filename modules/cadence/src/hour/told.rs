//! Where each of the hour's findings goes: to its owner while a session
//! answers as it, or else to the orchestrator, saying whose it was. A
//! finding sent to an owner that has ended is written to the mesh and read
//! by no one: the relay's count of 502s went so on 27 November, and two of
//! Joel's turns that stalled on them were found only by hand.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::Path;

use crate::findings;
use crate::mesh::{self, Mesh, Presence};

/// Sends each of the findings `told` to its owner, or to `to` for an owner
/// no session answers as, as [`told_to`] words it, and answers `keeper`
/// while a session answers as it.
///
/// # Errors
///
/// When a finding can't be written.
pub fn send(
    mesh: &Mesh,
    told: Vec<(String, String)>,
    keeper: Option<String>,
    to: &str,
    now: i64,
) -> io::Result<Option<String>> {
    let asked: Vec<&str> = told
        .iter()
        .map(|(owner, _)| owner.as_str())
        .chain(keeper.as_deref())
        .collect();
    let gone = gone(mesh, &asked, now);
    let answers = |address: &str| !gone.contains(address);
    for (owner, text) in told.into_iter().filter(|(owner, _)| !owner.is_empty()) {
        let (whom, text) = told_to(&owner, text, to, answers);
        mesh.send(&crate::message(&whom, "finding", text))?;
    }
    Ok(keeper.filter(|k| answers(k.as_str())))
}

/// Where Linux says which boot this is.
const BOOT_ID: &str = "/proc/sys/kernel/random/boot_id";

/// Those of `asked` no session answers as, by the rule the day's ledger
/// holds its owners to (`findings::gone::owners`): no presence going by
/// one at all, or only one quiet past half a day that is parked or isn't
/// running here. A session on another machine answers through its
/// presence as the mesh carries it here, which names no pid of this one,
/// so a pid alone can't say it's gone.
#[must_use]
pub fn gone(mesh: &Mesh, asked: &[&str], now: i64) -> BTreeSet<String> {
    let boot = match fs::read_to_string(BOOT_ID) {
        Ok(id) => id.trim().to_owned(),
        Err(err) => {
            eprintln!("cadence: reading {BOOT_ID}: {err}, so no session reads as running here");
            String::new()
        }
    };
    let rosters = mesh.rostered(now);
    let running = |p: &Presence| {
        mesh::running(p, &boot, &rosters, |pid| {
            Path::new(&format!("/proc/{pid}")).exists()
        })
    };
    let owners: BTreeMap<String, String> = asked
        .iter()
        .map(|a| ((*a).to_owned(), (*a).to_owned()))
        .collect();
    findings::gone::owners(&owners, &[], &mesh.all(), running, now)
}

/// The address a finding for `owner` goes to, with its words: `owner`
/// itself while `there` says a session answers as it, or else `to`, told
/// whose it was and asked to pass it on. The ledger still owes it to
/// `owner`, so it closes as its figure comes back in, whoever fixes it.
#[must_use]
pub fn told_to(
    owner: &str,
    text: String,
    to: &str,
    there: impl Fn(&str) -> bool,
) -> (String, String) {
    if there(owner) {
        return (owner.to_owned(), text);
    }
    (
        to.to_owned(),
        format!(
            "For {owner}, which no session answers as now: {text} Please pass it to whoever \
             owns this now, and have cadence's owner map name them, so the next one reaches \
             them."
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_session_on_another_machine_answers_and_one_with_no_presence_is_gone() {
        let root = tempfile::tempdir().expect("a mesh folder");
        let presences = root.path().join("mesh-workflow/presence");
        fs::create_dir_all(&presences).expect("the presence folder");
        let now = 1_795_815_000;
        let at = jiff::Timestamp::from_second(now)
            .expect("a time")
            .to_string();
        // As the mesh carries another machine's: busy, and no pid of here.
        let away = serde_json::json!({
            "session": "s1", "addresses": ["AWAY01"], "state": "busy", "at": at,
        });
        fs::write(presences.join("s1.json"), away.to_string()).expect("a presence");
        let mesh = Mesh {
            root: root.path().to_path_buf(),
        };
        assert_eq!(
            gone(&mesh, &["AWAY01", "ENDED1"], now),
            BTreeSet::from(["ENDED1".to_owned()]),
            "a session on another machine read as gone, or one with no presence as there"
        );
    }

    #[test]
    fn a_finding_whose_owner_has_ended_goes_to_the_orchestrator_saying_whose() {
        let there = |address: &str| address == "LIVE01";
        let said = "cadence hour: the relay is past its ceiling.".to_owned();
        assert_eq!(
            told_to("LIVE01", said.clone(), "ORCH01", there),
            ("LIVE01".to_owned(), said.clone()),
            "a finding for an owner a session answers as went elsewhere, or was reworded"
        );
        let (to, text) = told_to("GONE01", said.clone(), "ORCH01", there);
        assert_eq!(
            to, "ORCH01",
            "a finding for an owner no session answers as wasn't sent to the orchestrator"
        );
        assert!(
            text.starts_with("For GONE01, ") && text.contains(&said),
            "the orchestrator wasn't told whose finding it was, or not the finding: {text}"
        );
    }
}
