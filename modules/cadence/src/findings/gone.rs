//! Owners the flags name that no session answers as: which they are, how
//! their findings stay with the watcher meanwhile, and the line that tells
//! the orchestrator which flags to change.

use std::collections::{BTreeMap, BTreeSet};

use super::{Entry, update};
use crate::DAY;
use crate::day::Day;
use crate::judge::Finding;
use crate::mesh::Presence;

/// How long an owner's session may say nothing, parked or not running,
/// before its findings go to the watcher: half a day, so one a crash or a
/// reboot took down is first given back by its holder, and one the day's
/// run finds still reaches a session that can answer before it falls due.
pub const QUIET_AT_MOST: i64 = DAY / 2;

/// The owners with no session to answer them, of those `owners` names and
/// those holding an open finding of the ledger's, found by a unit's name:
/// by each address's newest presence, none at all, or one quiet past
/// `QUIET_AT_MOST` that is parked or isn't `running`. A parked session is
/// told its findings as findings, which start no turn, so one parked all
/// day reads none of them: 7QEY1S's, parked from 00:31 on 18 November,
/// went late.
pub fn owners(
    owners: &BTreeMap<String, String>,
    ledger: &[Entry],
    presences: &[Presence],
    running: impl Fn(&Presence) -> bool,
    now: i64,
) -> BTreeSet<String> {
    let said_at = |p: &Presence| {
        p.at.as_str()
            .and_then(|t| t.parse::<jiff::Timestamp>().ok())
            .map(jiff::Timestamp::as_second)
    };
    let holding = ledger
        .iter()
        .filter(|e| e.open && e.kind.is_none())
        .filter_map(|e| e.owner.as_ref());
    owners
        .values()
        .chain(holding)
        .filter(|address| {
            // One with no readable time ranks below every dated one.
            let newest = presences
                .iter()
                .filter(|p| p.addresses.iter().any(|a| a.eq_ignore_ascii_case(address)))
                .max_by_key(|p| said_at(p));
            newest.is_none_or(|p| {
                let quiet = said_at(p).is_none_or(|at| now - at > QUIET_AT_MOST);
                quiet && (p.state == "parked" || !running(p))
            })
        })
        .cloned()
        .collect()
}

/// Brings `ledger` up to `day` as `update` does, as though the owners in
/// `gone` weren't named: their open findings go back to nobody, keeping
/// their dates, so the watcher is told them once and one already late
/// isn't given a fresh day, and stay with it until a flag names a session
/// that answers. A must keeps the quest that filed it. One put to a hub
/// whose holder went goes back to its owner line first, the holders it
/// was put to kept, so the next time it's late it goes to one not yet
/// given it.
pub fn update_without(
    ledger: &mut Vec<Entry>,
    day: &Day,
    worse: &[Finding],
    owners: &BTreeMap<String, String>,
    known: &BTreeSet<String>,
    gone: &BTreeSet<String>,
) {
    let is_gone = |address: &str| gone.iter().any(|g| g.eq_ignore_ascii_case(address));
    let there: BTreeMap<String, String> = owners
        .iter()
        .filter(|(_, address)| !is_gone(address))
        .map(|(key, address)| (key.clone(), address.clone()))
        .collect();
    let known: BTreeSet<String> = known.iter().filter(|k| !is_gone(k)).cloned().collect();
    for entry in ledger.iter_mut().filter(|e| e.open && e.hub.is_some()) {
        if entry.owner.as_deref().is_some_and(is_gone) {
            entry.hub = None;
            entry.owner = None;
            entry.told = None;
        }
    }
    update(ledger, day, worse, &there, &known);
    for entry in ledger.iter_mut().filter(|e| e.open && e.kind.is_none()) {
        if entry.owner.as_deref().is_some_and(is_gone) {
            entry.owner = None;
            entry.told = None;
        }
    }
}

/// The owners in `gone` a flag names, or that hold an open finding by a
/// unit's name: a hub's holder that went holds none once `update_without`
/// gives its findings back, and no flag names it, so there's none to
/// change. Read from the ledger as it stood before.
#[must_use]
pub fn to_flag(
    owners: &BTreeMap<String, String>,
    ledger: &[Entry],
    gone: &BTreeSet<String>,
) -> BTreeSet<String> {
    gone.iter()
        .filter(|address| {
            owners.values().any(|a| a.eq_ignore_ascii_case(address))
                || ledger.iter().any(|e| {
                    e.open
                        && e.kind.is_none()
                        && e.hub.is_none()
                        && e.owner
                            .as_deref()
                            .is_some_and(|o| o.eq_ignore_ascii_case(address))
                })
        })
        .cloned()
        .collect()
}

/// What the orchestrator is told of the owners in `gone`, in one line
/// whatever their number: the flags that name each, so they can be
/// changed, and who has their findings meanwhile.
pub fn flags(
    owners: &BTreeMap<String, String>,
    gone: &BTreeSet<String>,
    watcher: Option<&str>,
) -> Option<String> {
    if gone.is_empty() {
        return None;
    }
    let each: Vec<String> = gone
        .iter()
        .map(|address| {
            let keys: Vec<&str> = owners
                .iter()
                .filter(|(_, a)| a.eq_ignore_ascii_case(address))
                .map(|(key, _)| key.as_str())
                .collect();
            if keys.is_empty() {
                format!("{address} (by a unit's name)")
            } else {
                format!("{address} ({})", keys.join(", "))
            }
        })
        .collect();
    let told = watcher.map_or_else(
        || "nobody is told their findings".to_owned(),
        |w| format!("{w} has their findings"),
    );
    Some(format!(
        "Owner flags name no live session for {}; {told} until the flags change",
        each.join(", ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::tell;
    use crate::findings::tests::{day, found};
    use crate::holders;

    /// A session's presence as the mesh keeps it, last said at `at`.
    fn presence(address: &str, state: &str, at: i64, pid: u32) -> Presence {
        let at = jiff::Timestamp::from_second(at).expect("a time in range");
        serde_json::from_value(serde_json::json!({
            "addresses": [address],
            "state": state,
            "at": at.to_string(),
            "owner": {"pid": pid},
        }))
        .expect("a presence is read from its record")
    }

    fn named() -> BTreeMap<String, String> {
        [
            ("checks", "DEAD01"),
            ("code", "CRASHD"),
            ("crashes", "GONE01"),
            ("crashes:chrome", "LIVE01"),
            ("health", "RESTED"),
            ("switch", "PARKED"),
        ]
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .into()
    }

    #[test]
    fn an_owner_with_no_session_or_quiet_past_half_a_day_is_gone() {
        let now = 10 * DAY;
        let quiet = now - QUIET_AT_MOST - 1;
        let undated: Presence = serde_json::from_value(serde_json::json!({
            "addresses": ["LIVE01"],
            "state": "parked",
        }))
        .expect("a presence is read from its record");
        let presences = [
            // The newest of an address's presences counts, whatever the
            // order they're read in, and one with no time never does.
            presence("PARKED", "parked", quiet, 3),
            presence("PARKED", "busy", now - DAY, 1),
            undated,
            presence("LIVE01", "busy", now - DAY, 1),
            presence("DEAD01", "busy", quiet, 2),
            // Down an hour, as a crash or a reboot leaves it: its holder
            // gives it back first.
            presence("CRASHD", "busy", now - 3600, 2),
            presence("RESTED", "parked", now - QUIET_AT_MOST, 3),
        ];
        let running = |p: &Presence| p.owner.as_ref().and_then(|o| o.pid) == Some(1);
        let mut ledger = Vec::new();
        update(
            &mut ledger,
            &day("d1", DAY),
            &[found("failed:camera.service")],
            &named(),
            &holders::tests::known(),
        );
        ledger[0].owner = Some("UNIT01".to_owned());
        let gone = owners(&named(), &ledger, &presences, running, now);
        assert_eq!(
            gone.iter().map(String::as_str).collect::<Vec<_>>(),
            ["DEAD01", "GONE01", "PARKED", "UNIT01"],
            "an owner that can't answer was kept, or one that can was taken"
        );
    }

    #[test]
    fn a_gone_owners_findings_go_to_the_watcher_until_it_answers_again() {
        let mut ledger = Vec::new();
        let unit = "failed:garbage-bound-mon-2af2dh.service";
        let worse = [found("crashes:tec"), found("crashes:chrome"), found(unit)];
        let known = holders::tests::known();
        // An address found from a unit's name is matched whatever its case.
        let gone: BTreeSet<String> = ["GONE01".to_owned(), "2af2dh".to_owned()].into();
        let none = BTreeSet::new();
        let on = |ledger: &mut Vec<Entry>, date: &str, at: i64, gone: &BTreeSet<String>| {
            update_without(ledger, &day(date, at * DAY), &worse, &named(), &known, gone);
            let told = tell(ledger, date, Some("WATCH1"));
            told.into_iter().map(|(to, _)| to).collect::<Vec<String>>()
        };
        on(&mut ledger, "d1", 1, &none);
        ledger.push(Entry {
            key: "security:s2".to_owned(),
            kind: Some("hole".to_owned()),
            ..ledger[0].clone()
        });
        ledger[0].late = true;
        assert_eq!(
            on(&mut ledger, "d2", 2, &gone),
            ["WATCH1"],
            "the watcher wasn't told"
        );
        let owners: Vec<Option<&str>> = ledger.iter().map(|e| e.owner.as_deref()).collect();
        assert_eq!(
            (owners, ledger[0].due, ledger[0].late),
            (
                vec![None, Some("LIVE01"), None, Some("GONE01")],
                2 * DAY,
                true
            ),
            "a gone owner kept its finding, a late one got a fresh day, or a must moved"
        );
        assert!(
            on(&mut ledger, "d3", 3, &gone).is_empty(),
            "the watcher was told twice"
        );
        assert_eq!(
            on(&mut ledger, "d4", 4, &none),
            ["2AF2DH", "GONE01"],
            "an owner back on the mesh didn't take its findings again"
        );
    }

    #[test]
    fn the_orchestrator_is_told_in_a_line_which_flags_name_a_gone_owner() {
        let gone: BTreeSet<String> = [
            "GONE01".to_owned(),
            "PARKED".to_owned(),
            "UNIT01".to_owned(),
        ]
        .into();
        assert_eq!(
            [
                flags(&named(), &gone, Some("WATCH1")),
                flags(&named(), &["GONE01".to_owned()].into(), None),
                flags(&named(), &BTreeSet::new(), Some("WATCH1")),
            ],
            [
                Some(
                    "Owner flags name no live session for GONE01 (crashes), PARKED (switch), \
                     UNIT01 (by a unit's name); WATCH1 has their findings until the flags change"
                        .to_owned()
                ),
                Some(
                    "Owner flags name no live session for GONE01 (crashes); nobody is told \
                     their findings until the flags change"
                        .to_owned()
                ),
                None,
            ],
            "the orchestrator wasn't told which flags to change, or who has their findings"
        );
    }
}
