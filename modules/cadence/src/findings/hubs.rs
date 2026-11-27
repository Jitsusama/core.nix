//! A finding still open past its date goes back to its subject's hub as a
//! need, to the holder of that subject who holds the fewest findings,
//! rather than as a line to the orchestrator, which made the orchestrator
//! every late finding's funnel. The node names a hub for each subject a
//! cell declares, read through subject-hubs, which pain-points reads them
//! through too; a finding whose key and axis take in every word of a
//! subject is that subject's. Each holder is given it once; one no hub
//! stands for, or with no live holder left to give it to, is told to the
//! orchestrator as before. Only cells this machine's mesh runs count as
//! live, since those are the cells cadence's message reaches. Each need is
//! kept as one line in the mesh's day file, in the harness's shape, as
//! subject-hubs keeps every sender's.

use std::collections::{BTreeMap, BTreeSet};

pub use subject_hubs::{Hub, Need, keep, listed, socket};
use subject_hubs::{taking, words};

use super::Entry;
use crate::DAY;

/// A late finding put to a hub: whose it was, the subject it went by, and
/// the holder it went to through which hub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Put {
    pub key: String,
    pub words: Vec<String>,
    pub hub: String,
    pub fit: String,
}

/// The hub whose subject `entry` is of, and the live holder it names
/// that hasn't had it, nor holds it now, holding the fewest of `held`:
/// the subject of most words first, then the order the node names, so
/// `crashes:chrome` takes a chrome crash before `crashes` does.
fn holder_for<'a>(
    entry: &Entry,
    hubs: &'a [Hub],
    live: &impl Fn(&str) -> bool,
    held: &BTreeMap<String, usize>,
) -> Option<(&'a Hub, &'a str, Vec<String>)> {
    let mut said = words(&entry.key);
    said.extend(words(&entry.axis));
    let had = |f: &str| entry.owner.as_deref() == Some(f) || entry.put_to.iter().any(|p| p == f);
    taking(&said, hubs, live)
        .into_iter()
        .find_map(|(hub, subject)| {
            let fit = hub
                .fits
                .iter()
                .filter(|f| !had(f) && live(f))
                .min_by_key(|f| held.get(f.as_str()).copied().unwrap_or_default())?;
            Some((hub, fit.as_str(), subject))
        })
}

/// Puts each open finding past its date at `now`, not yet put or told, to
/// its subject's hub: its holder takes it with a day to answer and is told
/// with the day's findings. Those no hub takes are marked late and given
/// back as the orchestrator's lines, as before. A finding put to a hub
/// that goes late again goes to the next holder that hasn't had it, and
/// once each has, to the orchestrator.
pub fn put_late(
    ledger: &mut [Entry],
    hubs: &[Hub],
    live: impl Fn(&str) -> bool,
    now: i64,
) -> (Vec<Put>, Vec<String>) {
    let mut held: BTreeMap<String, usize> = BTreeMap::new();
    for owner in ledger
        .iter()
        .filter(|e| e.open)
        .filter_map(|e| e.owner.clone())
    {
        *held.entry(owner).or_default() += 1;
    }
    let mut put = Vec::new();
    let mut lines = Vec::new();
    let mut drags: BTreeSet<String> = BTreeSet::new();
    let mut dragged = 0;
    // One back, its figure in, waits only on its owner's word of the fix,
    // which it was asked for, so it goes to nobody else; nor does one
    // waiting only on the change its owner named going live here, which
    // is the switch's to do.
    for entry in ledger.iter_mut().filter(|e| {
        e.open && !e.late && e.due < now && e.back.is_none() && !e.shown.awaits_switch(now)
    }) {
        // A must its owner filed stays its owner's, so it's only ever late.
        let found = if entry.kind.is_none() {
            holder_for(entry, hubs, &live, &held)
        } else {
            None
        };
        if let Some((hub, fit, words)) = found {
            if let Some(before) = entry.owner.as_ref().and_then(|o| held.get_mut(o)) {
                *before = before.saturating_sub(1);
            }
            *held.entry(fit.to_owned()).or_default() += 1;
            put.push(Put {
                key: entry.key.clone(),
                words,
                hub: hub.cell.clone(),
                fit: fit.to_owned(),
            });
            if let Some(owner) = entry.owner.take()
                && !entry.put_to.contains(&owner)
            {
                entry.put_to.push(owner);
            }
            entry.put_to.push(fit.to_owned());
            entry.owner = Some(fit.to_owned());
            entry.hub = Some(hub.subject.clone());
            entry.due = now + DAY;
            entry.told = None;
            continue;
        }
        entry.late = true;
        // The hour files every measure past its target, so its red lines
        // go late together; each is said as one count, which would
        // otherwise crowd the day's worse and better off its note. Which
        // keep a line each is listed once, at the hour's `owing::red`.
        if entry.red {
            dragged += 1;
            // An owner is a peer's word, so only one in an address's
            // letters is said in the note, never one carrying words.
            drags.insert(match entry.owner.as_deref() {
                None => "no owner".to_owned(),
                Some(o) if !o.is_empty() && o.bytes().all(|b| b.is_ascii_alphanumeric()) => {
                    o.to_owned()
                }
                Some(_) => "an owner that reads as no address".to_owned(),
            });
            continue;
        }
        lines.push(format!(
            "Late: {}, open since {}, with {}",
            entry.words,
            entry.first,
            entry.owner.as_deref().unwrap_or("no owner")
        ));
    }
    if dragged > 0 {
        let with: Vec<String> = drags.into_iter().collect();
        lines.push(format!(
            "Late: {dragged} of the hour's red lines asked of their owners, with {}",
            with.join(", ")
        ));
    }
    (put, lines)
}

/// Each of `put` as the need the mesh keeps, asked by `asker` at `at_ms`
/// in a realm of `cells`.
#[must_use]
pub fn needs(put: &[Put], asker: &str, at_ms: i64, cells: usize) -> Vec<Need> {
    put.iter()
        .enumerate()
        .map(|(n, p)| Need::put(asker, n, at_ms, p.words.clone(), (&p.hub, &p.fit), cells))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::findings::tests::{day, found};
    use crate::findings::{tell, update};

    fn owners() -> BTreeMap<String, String> {
        [("crashes".to_owned(), "A".to_owned())].into()
    }

    /// A ledger of one open finding a day found on `d1`, held by A and due
    /// two days in.
    fn ledger(key: &str) -> Vec<Entry> {
        let mut ledger = Vec::new();
        update(
            &mut ledger,
            &day("d1", DAY),
            &[found(key)],
            &owners(),
            &BTreeSet::new(),
        );
        ledger
    }

    #[test]
    fn a_late_finding_goes_to_its_narrowest_subjects_least_held_live_holder() {
        let mut ledger = ledger("crashes:chrome");
        ledger.extend(self::ledger("crashes:tec"));
        ledger[1].owner = Some("C".into());
        ledger[1].due = 10 * DAY;
        let hubs = [
            Hub::new("crashes", "H1", &["B"]),
            Hub::new("zzz:chrome", "H8", &["B"]),
            Hub::new("x.y:chrome", "H9", &["B"]),
            Hub::new("crashes:chrome", "H2", &["A", "E", "C", "D"]),
        ];
        let live = |a: &str| a != "E";
        let (put, lines) = put_late(&mut ledger, &hubs, live, 2 * DAY + 1);
        assert_eq!(
            lines,
            Vec::<String>::new(),
            "a finding a hub took was said late"
        );
        assert_eq!(
            put,
            vec![Put {
                key: "crashes:chrome".into(),
                words: vec!["crashes".into(), "chrome".into()],
                hub: "H2".into(),
                fit: "D".into(),
            }],
            "not its owner A, not E gone though it holds none, not C holding one \
             already, nor a hub of fewer words or one with a word it lacks: D"
        );
        let moved = &ledger[0];
        assert_eq!(
            (moved.owner.as_deref(), moved.hub.as_deref(), moved.due),
            (Some("D"), Some("crashes:chrome"), 3 * DAY + 1),
            "its holder wasn't given it with a day to answer"
        );
        assert!(!moved.late && moved.told.is_none(), "{moved:?}");
        let told = tell(&mut ledger, "d3", None);
        assert!(
            told.iter()
                .any(|(to, text)| to == "D"
                    && text.contains("put to you as a holder of crashes:chrome")),
            "its new holder wasn't told why it has it: {told:?}"
        );
        let mut given = Vec::new();
        for at in [4, 6, 8] {
            let (again, lines) = put_late(&mut ledger, &hubs, live, at * DAY);
            given.extend(again.into_iter().map(|p| p.fit));
            given.extend(lines);
        }
        assert_eq!(
            given,
            [
                "C",
                "B",
                "Late: crashes:chrome got worse, open since d1, with B"
            ],
            "late again, it goes to each holder once, the broader hub's last, and then \
             to the orchestrator rather than round them again"
        );
        assert_eq!(ledger[0].put_to, ["A", "D", "C", "B"], "{:?}", ledger[0]);
    }

    #[test]
    fn one_no_other_live_holder_takes_is_late_for_the_orchestrator() {
        let late = "Late: crashes:chrome got worse, open since d1, with A";
        for (hubs, why) in [
            (vec![], "no hub stands"),
            (
                vec![Hub::new("crashes:chrome", "H2", &["A"])],
                "its owner the only holder",
            ),
            (vec![Hub::new("crashes", "GONE", &["B"])], "its hub gone"),
            (
                vec![Hub::new("switch", "H3", &["B"])],
                "another subject's hub",
            ),
        ] {
            let mut ledger = ledger("crashes:chrome");
            let (put, lines) = put_late(&mut ledger, &hubs, |a| a != "GONE", 2 * DAY + 1);
            assert!(put.is_empty(), "{why}: put {put:?}");
            assert_eq!(lines, [late], "{why}: not said late as before");
            assert!(ledger[0].late, "{why}: not marked late");
        }
    }

    #[test]
    fn a_must_is_never_put_to_a_hub() {
        let mut ledger = ledger("crashes:chrome");
        ledger[0].kind = Some("hole".into());
        let hubs = [Hub::new("crashes:chrome", "H2", &["D"])];
        let (put, lines) = put_late(&mut ledger, &hubs, |_| true, 2 * DAY + 1);
        assert!(
            put.is_empty(),
            "a must left the quest that filed it: {put:?}"
        );
        assert_eq!(lines.len(), 1, "a must past its date wasn't said late");
    }

    #[test]
    fn the_hours_red_lines_gone_late_are_said_as_one_count() {
        let mut ledger = ledger("crashes:chrome");
        ledger[0].kind = Some("hole".into());
        for (key, owner, red) in [
            ("hour:a", "B", true),
            ("hour:b", "C", true),
            ("hour:c", "B", true),
            ("hour:miss:the host crashed", "D", false),
        ] {
            let mut drag = self::ledger(key).remove(0);
            drag.kind = Some("drag".into());
            drag.red = red;
            drag.owner = Some(owner.into());
            drag.words = key.to_owned();
            ledger.push(drag);
        }
        let (_, lines) = put_late(&mut ledger, &[], |_| true, 2 * DAY + 1);
        assert_eq!(
            lines,
            [
                "Late: crashes:chrome got worse, open since d1, with A",
                "Late: hour:miss:the host crashed, open since d1, with D",
                "Late: 3 of the hour's red lines asked of their owners, with B, C"
            ],
            "the hour's red lines took a line each, or the must or Joel's miss lost its own"
        );
        assert!(ledger.iter().all(|e| e.late), "one wasn't marked late");
    }

    #[test]
    fn a_put_finding_stays_with_its_holder_until_it_reopens() {
        let mut ledger = ledger("crashes:chrome");
        let hubs = [Hub::new("crashes:chrome", "H2", &["D"])];
        let _ = put_late(&mut ledger, &hubs, |_| true, 2 * DAY + 1);
        let found_again = [found("crashes:chrome")];
        update(
            &mut ledger,
            &day("d2", 2 * DAY + 2),
            &found_again,
            &owners(),
            &BTreeSet::new(),
        );
        assert_eq!(
            ledger[0].owner.as_deref(),
            Some("D"),
            "the owner lines took back a finding put to a hub"
        );
        ledger[0].open = false;
        update(
            &mut ledger,
            &day("d5", 5 * DAY),
            &found_again,
            &owners(),
            &BTreeSet::new(),
        );
        assert_eq!(
            (ledger[0].owner.as_deref(), ledger[0].hub.as_deref()),
            (Some("A"), None),
            "a reopened finding went to its hub's holder rather than its owner"
        );
        assert!(
            ledger[0].put_to.is_empty(),
            "reopened, it may go to each again"
        );
    }

    #[test]
    fn one_whose_holder_went_goes_back_to_its_owner_and_names_no_flag() {
        let mut ledger = ledger("crashes:chrome");
        let hubs = [Hub::new("crashes:chrome", "H2", &["D", "E"])];
        let _ = put_late(&mut ledger, &hubs, |_| true, 2 * DAY + 1);
        let gone: BTreeSet<String> = ["D".to_owned()].into();
        assert_eq!(
            super::super::gone::to_flag(&owners(), &ledger, &gone),
            BTreeSet::new(),
            "a hub's holder that went was named as a flag to change"
        );
        super::super::gone::update_without(
            &mut ledger,
            &day("d2", 2 * DAY + 2),
            &[found("crashes:chrome")],
            &owners(),
            &BTreeSet::new(),
            &gone,
        );
        let back = &ledger[0];
        assert_eq!(
            (back.owner.as_deref(), back.hub.as_deref()),
            (Some("A"), None),
            "it stayed with a holder that went: {back:?}"
        );
        let (again, _) = put_late(&mut ledger, &hubs, |a| a != "D", 5 * DAY);
        assert_eq!(
            again.iter().map(|p| p.fit.as_str()).collect::<Vec<_>>(),
            ["E"],
            "late again, it went back to one that had it"
        );
    }

    #[test]
    fn each_late_finding_put_is_a_need_of_cadences() {
        let put = [
            Put {
                key: "crashes:chrome".into(),
                words: vec!["chrome".into()],
                hub: "H2".into(),
                fit: "D".into(),
            },
            Put {
                key: "cadence".into(),
                words: vec!["cadence".into()],
                hub: "D".into(),
                fit: "D".into(),
            },
        ];
        // 01:25:46 on 19 November 2026, UTC.
        let needs = needs(&put, "CADENCE", 1_795_051_546_000, 130);
        assert_eq!(
            needs
                .iter()
                .map(|n| (n.id.as_str(), n.path.clone(), n.words.clone()))
                .collect::<Vec<_>>(),
            [
                (
                    "CADENCE/mwmvcdcg.0",
                    vec!["H2".to_owned(), "D".to_owned()],
                    vec!["chrome".to_owned()]
                ),
                (
                    "CADENCE/mwmvcdcg.1",
                    vec!["D".to_owned()],
                    vec!["cadence".to_owned()]
                ),
            ],
            "a put wasn't its need, by its place, its hub and holder and its subject's words"
        );
    }
}
