//! How the cells stand: a session left idle with work it could do, one
//! parked on a quest already done, a quest meant to be moving that no cell
//! runs, the cells the map draws wrongly, and a finished report no one put
//! to Joel. On 16 November a sweep found 111 open quests with no session,
//! 16 of them driving, Joel found cells the map showed alone after the
//! audit said it was done, and he found a review that had waited nine
//! hours for him; the hour saw none of it, since it read only what shipped.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::{Class, Measure, count, seconds, short};
use crate::axes::plans::{field, front};

/// How long a cell may sit idle with a step it could take: a peer silent
/// this long is one the orchestrator is told of, by its overnight rule of
/// 16 November.
const IDLE_AT_MOST: i64 = 20 * 60;

/// The priorities that mean a quest is meant to be moving now; queued,
/// bench and someday quests wait their turn by choice.
const MOVING: [&str; 2] = ["driving", "active"];

/// A quest as its README's frontmatter says it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quest {
    pub id: String,
    pub realm: String,
    pub status: String,
    pub priority: String,
    pub parent: Option<String>,
}

/// The quest a README describes; none without an id.
#[must_use]
pub fn quest(readme: &str) -> Option<Quest> {
    let front = front(readme);
    let text = |key| field(front, key).unwrap_or_default().to_owned();
    Some(Quest {
        id: field(front, "id")?.to_owned(),
        realm: text("realm"),
        status: text("status"),
        priority: text("priority"),
        parent: field(front, "parent")
            .filter(|p| !p.is_empty() && *p != "null")
            .map(str::to_owned),
    })
}

/// A quest's short id, its last part, which is also its cell's address.
fn short_id(id: &str) -> &str {
    id.rsplit('-').next().unwrap_or(id)
}

/// One session as the node says it, with what the measures read of it.
struct Cell<'a> {
    address: &'a str,
    quest: Option<&'a str>,
    /// The session's own state: busy, idle or ended.
    state: &'a str,
    /// Its presence's state: busy, listening or parked.
    presence: Option<&'a str>,
    at: Option<i64>,
    next: Option<&'a str>,
    wanting: bool,
    orchestrator: Option<&'a str>,
    quest_status: Option<&'a str>,
}

fn read(session: &Value) -> Option<Cell<'_>> {
    let state = session["state"].as_str()?;
    let presence = &session["presence"];
    Some(Cell {
        address: session["address"].as_str()?,
        quest: session["quest"].as_str(),
        state,
        presence: presence["state"].as_str(),
        at: seconds(&session["at"]),
        next: session["doing"]["next"]
            .as_str()
            .filter(|next| !next.trim().is_empty()),
        wanting: !presence["wanting"].is_null(),
        orchestrator: session["orchestrated_by"].as_str(),
        quest_status: presence["questInfo"]["status"].as_str(),
    })
}

/// A measure of the cells with no target where no realm's node answered,
/// since a quiet 0 would read as met.
fn judged(key: &str, says: &str, behind: Vec<String>, read: bool, fallback: &str) -> Measure {
    Measure {
        key: key.to_owned(),
        class: Class::Cells,
        says: if read {
            says.to_owned()
        } else {
            format!("{says} (no realm's node named its sessions)")
        },
        now: count(behind.len()),
        target: read.then_some(0.0),
        owner: fallback.to_owned(),
        behind,
    }
}

/// The four measures of the cells, from each realm's `mycelium sessions`
/// answer, by the realm's name, and the quests' READMEs, at `now`. A cell
/// no orchestrator or parent owns goes to `fallback`.
#[must_use]
pub fn cells(
    answers: &[(String, String)],
    quests: &[Quest],
    now: i64,
    fallback: &str,
) -> Vec<Measure> {
    let parsed: Vec<(&str, Value)> = answers
        .iter()
        .filter_map(|(realm, text)| Some((realm.as_str(), serde_json::from_str(text).ok()?)))
        .collect();
    let realms: BTreeSet<&str> = parsed.iter().map(|(realm, _)| *realm).collect();
    let all: Vec<Cell> = parsed
        .iter()
        .flat_map(|(_, answer)| answer["Ok"]["Sessions"].as_array().into_iter().flatten())
        .filter_map(read)
        .collect();
    let readme: BTreeMap<&str, &Quest> = quests.iter().map(|q| (q.id.as_str(), q)).collect();
    // A quest's cells are owned by the orchestration one of them names, else
    // the quest's parent, which is the address of the parent's own cell.
    let owner = |quest: Option<&str>, orchestrator: Option<&str>| -> String {
        orchestrator
            .or_else(|| {
                all.iter()
                    .filter(|c| c.quest.is_some() && c.quest == quest)
                    .find_map(|c| c.orchestrator)
            })
            .or_else(|| {
                quest
                    .and_then(|q| readme.get(q))
                    .and_then(|q| q.parent.as_deref())
                    .map(short_id)
            })
            .unwrap_or(fallback)
            .to_owned()
    };
    // A step that names Joel waits on him, which no orchestrator can move.
    let idle = all
        .iter()
        .filter(|c| {
            c.state == "idle" && !c.wanting && c.at.is_some_and(|at| now - at >= IDLE_AT_MOST)
        })
        .filter_map(|c| {
            let next = c.next.filter(|next| !next.contains("Joel"))?;
            let minutes = (now - c.at?) / 60;
            Some(format!(
                "{} idle {minutes} min: {} · {}",
                c.address,
                short(next),
                owner(c.quest, c.orchestrator)
            ))
        })
        .collect();
    let parked_done = all
        .iter()
        .filter(|c| c.presence == Some("parked") && c.state == "ended")
        .filter_map(|c| {
            let status = c
                .quest_status
                .or_else(|| Some(readme.get(c.quest?)?.status.as_str()))?;
            matches!(status, "concluded" | "retired").then(|| {
                format!(
                    "{} parked, its quest {status} · {}",
                    c.address,
                    owner(c.quest, c.orchestrator)
                )
            })
        })
        .collect();
    let (never, ended) = unrun(quests, &all, &realms, &|q| owner(Some(q), None));
    let read = !parsed.is_empty();
    [
        (
            "cells-idle",
            "cells idle 20 minutes or more with a next step no one else holds",
            idle,
        ),
        (
            "cells-parked-done",
            "cells parked on a quest already concluded or retired",
            parked_done,
        ),
        (
            "quests-without-cells",
            "driving and active quests no cell has taken up",
            never,
        ),
        (
            "quests-cells-ended",
            "driving and active quests whose every cell has ended",
            ended,
        ),
    ]
    .into_iter()
    .map(|(key, says, behind)| judged(key, says, behind, read, fallback))
    .collect()
}

/// The quests meant to be moving, in a realm whose node answered, that no
/// live cell runs: those no cell ever took up, and those whose every cell
/// ended, each with its owner.
fn unrun(
    quests: &[Quest],
    all: &[Cell],
    realms: &BTreeSet<&str>,
    owner: &dyn Fn(&str) -> String,
) -> (Vec<String>, Vec<String>) {
    let (mut never, mut ended) = (Vec::new(), Vec::new());
    for q in quests.iter().filter(|q| {
        q.status == "active"
            && MOVING.contains(&q.priority.as_str())
            && realms.contains(q.realm.as_str())
    }) {
        let of_quest: Vec<&Cell> = all
            .iter()
            .filter(|c| c.quest == Some(q.id.as_str()))
            .collect();
        if of_quest.iter().any(|c| c.state != "ended") {
            continue;
        }
        let named = short_id(&q.id);
        let to = owner(&q.id);
        if of_quest.is_empty() {
            never.push(format!("{named} {}, no cell yet · {to}", q.priority));
        } else {
            ended.push(format!("{named} {}, every cell ended · {to}", q.priority));
        }
    }
    (never, ended)
}

/// A report or review a quest wrote, as its frontmatter says it, and when
/// its file was last written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub id: String,
    pub quest: String,
    pub stage: String,
    pub written: i64,
}

/// The report a document's text describes, written at `written`; none for
/// another kind of document.
#[must_use]
pub fn report(text: &str, written: i64) -> Option<Report> {
    let front = front(text);
    (field(front, "kind")? == "report").then_some(())?;
    Some(Report {
        id: field(front, "id")?.to_owned(),
        quest: field(front, "quest")?.to_owned(),
        stage: field(front, "stage").unwrap_or_default().to_owned(),
        written,
    })
}

/// How long a drafted report may wait for a look that asks Joel about it.
const ASKED_WITHIN: i64 = 2 * 3600;

/// How far back drafted reports are judged: the looks began on 8 November,
/// so older drafts would count misses nothing could have avoided.
const REPORTS_SINCE: i64 = 3 * 24 * 3600;

/// How long before a report's last write a look may come and still be
/// about it, since a session puts the look up as it finishes writing.
const LOOK_BEFORE: i64 = 3600;

/// Reports and reviews still in draft, written at least 2 hours ago and in
/// the last 3 days, by a quest still active, that no look on Joel's list
/// asks him about: 9ZI7PR's review of 16 November sat nine hours behind a
/// look that asked nothing and cleared once seen. `looks` is none where the
/// list couldn't be read.
#[must_use]
pub fn unasked(
    reports: &[Report],
    looks: Option<&[Value]>,
    quests: &[Quest],
    now: i64,
    fallback: &str,
) -> Measure {
    let says = "drafted reports and reviews no look on Joel's list asks him about \
                within 2 hours (a post on the change itself isn't read)";
    let Some(looks) = looks else {
        return Measure {
            key: "reports-unasked".to_owned(),
            class: Class::Cells,
            says: format!("{says} (the look list couldn't be read)"),
            now: 0.0,
            target: None,
            owner: fallback.to_owned(),
            behind: Vec::new(),
        };
    };
    let readme: BTreeMap<&str, &Quest> = quests.iter().map(|q| (q.id.as_str(), q)).collect();
    // A look that asks something, or still waits on the list, puts the
    // report to Joel; one that asked nothing clears once seen.
    let asked = |r: &Report| {
        looks.iter().any(|look| {
            let about = look["quest"].as_str() == Some(r.quest.as_str())
                || look["target"].as_str().is_some_and(|t| t.contains(&r.id));
            let after = seconds(&look["at"]).is_some_and(|at| at >= r.written - LOOK_BEFORE);
            let puts =
                look["asks"].as_bool() == Some(true) || look["state"].as_str() == Some("waiting");
            about && after && puts
        })
    };
    let behind: Vec<String> = reports
        .iter()
        .filter(|r| matches!(r.stage.as_str(), "draft" | "build"))
        .filter(|r| (ASKED_WITHIN..=REPORTS_SINCE).contains(&(now - r.written)))
        .filter_map(|r| {
            let quest = readme.get(r.quest.as_str())?;
            (quest.status == "active" && !asked(r)).then(|| {
                let owner = quest.parent.as_deref().map_or(fallback, short_id);
                format!(
                    "{}'s {}, drafted {} h ago, asked nothing · {owner}",
                    short_id(&r.quest),
                    r.id,
                    (now - r.written) / 3600
                )
            })
        })
        .collect();
    Measure {
        key: "reports-unasked".to_owned(),
        class: Class::Cells,
        says: says.to_owned(),
        now: count(behind.len()),
        target: Some(0.0),
        owner: fallback.to_owned(),
        behind,
    }
}

/// The cells `mycelium-map-check --json` says the map draws wrongly, from
/// its exit and what it printed; unjudged where it couldn't read the node
/// or didn't run.
#[must_use]
pub fn mapped(exit: Option<i32>, out: &str, fallback: &str) -> Measure {
    let says = "cells the map draws wrongly: alone, on an unnamed network or out of reach";
    // Exit 2 is the check failing to read the node, which says nothing of
    // the map: H2VYOB's contract of 16 November.
    let read: Option<Value> = matches!(exit, Some(0 | 1))
        .then(|| serde_json::from_str(out).ok())
        .flatten()
        .filter(|read: &Value| read["count"].is_u64() && read["failures"].is_array());
    let Some(read) = read else {
        return Measure {
            key: "map-cells".to_owned(),
            class: Class::Cells,
            says: format!("{says} (the map's check didn't read the node)"),
            now: 0.0,
            target: None,
            owner: fallback.to_owned(),
            behind: Vec::new(),
        };
    };
    let behind: Vec<String> = read["failures"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|failure| {
            let cell = failure["cell"].as_str()?;
            let why = failure["why"].as_str().unwrap_or("drawn wrongly");
            let within = failure["orchestration"]
                .as_str()
                .map(|o| format!(", in {o}'s orchestration"))
                .unwrap_or_default();
            let owner = failure["owner"].as_str().unwrap_or(fallback);
            Some(format!("{cell} {why}{within} · {owner}"))
        })
        .collect();
    Measure {
        key: "map-cells".to_owned(),
        class: Class::Cells,
        says: says.to_owned(),
        now: count(behind.len()),
        target: Some(0.0),
        owner: fallback.to_owned(),
        behind,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const NOW: i64 = 1_800_000_000;

    fn at(minutes_ago: i64) -> String {
        jiff::Timestamp::from_second(NOW - minutes_ago * 60)
            .expect("a time")
            .to_string()
    }

    fn answer(sessions: &[Value]) -> (String, String) {
        (
            "work".to_owned(),
            json!({"Ok": {"Sessions": sessions}}).to_string(),
        )
    }

    fn quest_of(short: &str, status: &str, priority: &str, parent: Option<&str>) -> Quest {
        Quest {
            id: format!("QEST-20261116-{short}"),
            realm: "work".to_owned(),
            status: status.to_owned(),
            priority: priority.to_owned(),
            parent: parent.map(|p| format!("QEST-20261007-{p}")),
        }
    }

    fn cell(address: &str, state: &str, quest: &str) -> Value {
        json!({
            "address": address,
            "quest": format!("QEST-20261116-{quest}"),
            "state": state,
            "at": at(0),
            "doing": null,
            "orchestrated_by": null,
            "presence": {"state": "busy", "wanting": null,
                         "questInfo": {"status": "active"}},
        })
    }

    fn measure<'a>(all: &'a [Measure], key: &str) -> &'a Measure {
        all.iter()
            .find(|m| m.key == key)
            .unwrap_or_else(|| panic!("no {key} among {all:?}"))
    }

    #[test]
    fn a_readme_says_its_quest_by_its_top_level_fields() {
        let readme = "---\nid: QEST-1\nrealm: work\nkind: subquest\nparent: QEST-0\n\
                      status: active\npriority: driving\nsessions:\n  - id: s\n    \
                      status: detached\n---\n# A Quest\n";
        assert_eq!(
            quest(readme),
            Some(Quest {
                id: "QEST-1".to_owned(),
                realm: "work".to_owned(),
                status: "active".to_owned(),
                priority: "driving".to_owned(),
                parent: Some("QEST-0".to_owned()),
            }),
            "a session's status was read as the quest's, or a field was lost"
        );
        assert_eq!(
            quest("no frontmatter"),
            None,
            "a README with no id said a quest"
        );
    }

    #[test]
    fn an_idle_cell_counts_once_twenty_minutes_pass_with_a_step_nobody_holds() {
        let idle = |address: &str, minutes: i64, next: Option<&str>, wanting: Value| {
            let mut c = cell(address, "idle", address);
            c["at"] = json!(at(minutes));
            c["doing"] = json!({"next": next});
            c["presence"]["wanting"] = wanting;
            c["orchestrated_by"] = json!("ORCH");
            c
        };
        let answers = [answer(&[
            idle("AAAAAA", 30, Some("write the failing test"), Value::Null),
            idle("BBBBBB", 10, Some("write the failing test"), Value::Null),
            idle("CCCCCC", 30, Some("Joel answers A to E"), Value::Null),
            idle("DDDDDD", 30, Some("merge it"), json!({"kind": "approval"})),
            idle("EEEEEE", 30, None, Value::Null),
        ])];
        let all = cells(&answers, &[], NOW, "FALLBACK");
        let idle = measure(&all, "cells-idle");
        assert_eq!(
            idle.behind,
            ["AAAAAA idle 30 min: write the failing test · ORCH"],
            "only the cell idle past 20 minutes with a step waiting on no one counts"
        );
        assert!(
            (idle.now - 1.0).abs() < f64::EPSILON && idle.target == Some(0.0),
            "its count and target: {idle:?}"
        );
    }

    #[test]
    fn a_parked_cell_counts_once_its_quest_is_done() {
        let parked = |address: &str, status: &str| {
            let mut c = cell(address, "ended", address);
            c["presence"]["state"] = json!("parked");
            c["presence"]["questInfo"]["status"] = json!(status);
            c
        };
        let answers = [answer(&[
            parked("AAAAAA", "concluded"),
            parked("BBBBBB", "retired"),
            parked("CCCCCC", "active"),
        ])];
        let quests = [
            quest_of("AAAAAA", "concluded", "someday", Some("PARENT")),
            quest_of("BBBBBB", "retired", "someday", None),
        ];
        let all = cells(&answers, &quests, NOW, "FALLBACK");
        assert_eq!(
            measure(&all, "cells-parked-done").behind,
            [
                "AAAAAA parked, its quest concluded · PARENT",
                "BBBBBB parked, its quest retired · FALLBACK"
            ],
            "a parked cell on a quest still open counted, or an owner was wrong"
        );
    }

    #[test]
    fn a_quest_meant_to_move_with_no_cell_running_counts_once_by_whether_one_ended() {
        let mut ended = cell("BBBBBB", "ended", "BBBBBB");
        ended["orchestrated_by"] = json!("ORCH");
        let answers = [answer(&[
            ended,
            cell("DDDDDD", "busy", "DDDDDD"),
            cell("EEEEEE", "idle", "EEEEEE"),
        ])];
        let mut elsewhere = quest_of("GGGGGG", "active", "driving", None);
        elsewhere.realm = "personal".to_owned();
        let quests = [
            quest_of("AAAAAA", "active", "driving", Some("PARENT")),
            quest_of("BBBBBB", "active", "active", Some("PARENT")),
            quest_of("CCCCCC", "active", "bench", None),
            quest_of("DDDDDD", "active", "driving", None),
            quest_of("EEEEEE", "active", "active", None),
            quest_of("FFFFFF", "concluded", "driving", None),
            elsewhere,
        ];
        let all = cells(&answers, &quests, NOW, "FALLBACK");
        assert_eq!(
            measure(&all, "quests-without-cells").behind,
            ["AAAAAA driving, no cell yet · PARENT"],
            "a quest with no cell was missed, or one benched, running, done or in a \
             realm not read counted"
        );
        assert_eq!(
            measure(&all, "quests-cells-ended").behind,
            ["BBBBBB active, every cell ended · ORCH"],
            "a quest whose cells all ended was missed, or not given its orchestrator"
        );
    }

    #[test]
    fn with_no_realm_answering_the_cells_are_unjudged() {
        let all = cells(
            &[],
            &[quest_of("AAAAAA", "active", "driving", None)],
            NOW,
            "F",
        );
        assert_eq!(all.len(), 4, "a measure went unsaid: {all:?}");
        assert!(
            all.iter()
                .all(|m| m.target.is_none() && m.class == Class::Cells),
            "an hour with no node read judged its cells: {all:?}"
        );
    }

    #[test]
    fn a_document_says_its_report_by_its_kind() {
        let text = "---\nid: RPRT-1\nkind: report\nquest: QEST-20261116-AAAAAA\n\
                    stage: draft\nupdated: 2026-11-16\n---\n# A Review\n";
        assert_eq!(
            report(text, NOW),
            Some(Report {
                id: "RPRT-1".to_owned(),
                quest: "QEST-20261116-AAAAAA".to_owned(),
                stage: "draft".to_owned(),
                written: NOW,
            }),
            "a report's fields were lost"
        );
        assert_eq!(
            report(&text.replace("kind: report", "kind: plan"), NOW),
            None,
            "a plan was read as a report"
        );
    }

    #[test]
    fn a_drafted_report_counts_until_a_look_asks_joel_about_it() {
        let drafted = |short: &str, hours_ago: i64, stage: &str| Report {
            id: format!("RPRT-{short}"),
            quest: format!("QEST-20261116-{short}"),
            stage: stage.to_owned(),
            written: NOW - hours_ago * 3600,
        };
        let look = |quest: &str, target: &str, hours_ago: i64, asks: bool, state: &str| {
            json!({"quest": format!("QEST-20261116-{quest}"), "target": target,
                   "at": at(hours_ago * 60), "asks": asks, "state": state})
        };
        let reports = [
            drafted("AAAAAA", 9, "draft"),
            drafted("BBBBBB", 9, "draft"),
            drafted("CCCCCC", 9, "draft"),
            drafted("DDDDDD", 1, "draft"),
            drafted("EEEEEE", 9, "concluded"),
            drafted("FFFFFF", 9, "draft"),
            drafted("GGGGGG", 100, "draft"),
            drafted("HHHHHH", 9, "draft"),
        ];
        let looks = [
            // Seen, asking nothing: 9ZI7PR's case, which still counts.
            look("AAAAAA", "/r/RPRT-AAAAAA.md", 9, false, "resolved"),
            // Another quest's look that asks about this report's file.
            look("OTHERS", "/r/RPRT-BBBBBB.md", 2, true, "cleared"),
            // Still waiting on the list, though it asks nothing.
            look("CCCCCC", "/r/RPRT-CCCCCC.md", 8, false, "waiting"),
            // An asking look from long before the report was written.
            look("HHHHHH", "https://a/pr/1", 30, true, "waiting"),
        ];
        let quests = [
            quest_of("AAAAAA", "active", "driving", Some("PARENT")),
            quest_of("BBBBBB", "active", "driving", None),
            quest_of("CCCCCC", "active", "driving", None),
            quest_of("DDDDDD", "active", "driving", None),
            quest_of("EEEEEE", "active", "driving", None),
            quest_of("FFFFFF", "concluded", "driving", None),
            quest_of("GGGGGG", "active", "driving", None),
            quest_of("HHHHHH", "active", "driving", None),
        ];
        let unasked = unasked(&reports, Some(&looks), &quests, NOW, "FALLBACK");
        assert_eq!(
            unasked.behind,
            [
                "AAAAAA's RPRT-AAAAAA, drafted 9 h ago, asked nothing · PARENT",
                "HHHHHH's RPRT-HHHHHH, drafted 9 h ago, asked nothing · FALLBACK"
            ],
            "a report no look asks about was missed, or one asked, waiting, too new, \
             concluded, of a done quest or too old counted"
        );
        assert_eq!(unasked.target, Some(0.0), "a read list went unjudged");
        assert_eq!(
            super::unasked(&reports, None, &quests, NOW, "F").target,
            None,
            "an unread list judged the reports"
        );
    }

    #[test]
    fn the_maps_check_counts_each_cell_it_draws_wrongly_and_nothing_when_it_cannot_read() {
        let out = json!({"at": "2026-11-17T05:00:00Z", "cells": 40, "count": 2, "failures": [
            {"cell": "AAAAAA", "orchestration": "ORCH", "owner": "ORCH", "why": "alone"},
            {"cell": "BBBBBB", "orchestration": null, "owner": "BBBBBB",
             "why": "unnamed network"},
        ]})
        .to_string();
        let failing = mapped(Some(1), &out, "F");
        assert_eq!(
            (failing.now, failing.target, failing.behind.clone()),
            (
                2.0,
                Some(0.0),
                vec![
                    "AAAAAA alone, in ORCH's orchestration · ORCH".to_owned(),
                    "BBBBBB unnamed network · BBBBBB".to_owned()
                ]
            ),
            "the check's failures were misread"
        );
        let none = json!({"at": "", "cells": 40, "count": 0, "failures": []}).to_string();
        assert_eq!(mapped(Some(0), &none, "F").now, 0.0, "a clean map counted");
        assert_eq!(
            mapped(Some(0), &none, "F").target,
            Some(0.0),
            "a clean map unjudged"
        );
        for (exit, out) in [(Some(2), out.as_str()), (None, ""), (Some(1), "not json")] {
            assert_eq!(
                mapped(exit, out, "F").target,
                None,
                "a check that couldn't read the node, or didn't run, judged it: {exit:?}"
            );
        }
    }
}
