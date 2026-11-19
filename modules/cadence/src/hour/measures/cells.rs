//! How the cells stand: a session left idle with work it could do, one
//! parked on a quest already done, a quest meant to be moving that no cell
//! runs, and the cells the map draws wrongly. On 16 November a sweep found
//! 111 open quests with no session, 16 of them driving, and Joel found
//! cells the map showed alone after the audit said it was done; the hour
//! saw none of it, since it read only what shipped.

use std::collections::{BTreeMap, BTreeSet};

use jiff::tz::TimeZone;
use serde_json::Value;

use super::{Class, Measure, count, seconds, short};
use crate::axes::plans::{field, front};
use crate::idle::{dated, routes};

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
    /// Whether it routes, its cell idle by design (see `idle::routes`).
    pub routes: bool,
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
        routes: routes(readme),
    })
}

/// A quest's short id, its last part, which is also its cell's address.
pub(super) fn short_id(id: &str) -> &str {
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
    /// The machine it runs on, as the node names it.
    machine: Option<&'a str>,
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
        orchestrator: session["orchestrated_by"]
            .as_str()
            .filter(|o| !o.is_empty()),
        quest_status: presence["questInfo"]["status"].as_str(),
        machine: session["machine"].as_str().filter(|m| !m.is_empty()),
    })
}

/// The cells the node names, each once: it has listed a session twice,
/// and a live listing then outranks an ended one.
fn once<'a>(cells: impl Iterator<Item = Cell<'a>>) -> Vec<Cell<'a>> {
    let mut kept: Vec<Cell> = Vec::new();
    for cell in cells {
        match kept.iter_mut().find(|c| c.address == cell.address) {
            Some(seen) if seen.state == "ended" && cell.state != "ended" => *seen = cell,
            Some(_) => {}
            None => kept.push(cell),
        }
    }
    kept
}

/// A measure of the cells with no target where what it reads couldn't be
/// read, named by `unread`, since a quiet 0 would read as met.
fn judged(
    key: &str,
    says: &str,
    behind: Vec<String>,
    unread: Option<&str>,
    fallback: &str,
) -> Measure {
    let read = unread.is_none();
    Measure {
        key: key.to_owned(),
        class: Class::Cells,
        says: match unread {
            None => says.to_owned(),
            Some(why) => format!("{says} ({why})"),
        },
        now: count(behind.len()),
        target: read.then_some(0.0),
        owner: fallback.to_owned(),
        behind,
    }
}

/// The four measures of the cells, from each realm's `mycelium sessions`
/// answer, by the realm's name, and the quests' READMEs, at `now`; the
/// quests are none where their store couldn't be read. A cell no
/// orchestrator or parent owns goes to `fallback`. A node names the
/// sessions of every machine it hears from, so a cell on a machine other
/// than `here` says which: on 19 November A210QS was counted idle from
/// another machine through the personal realm's node, and its owner
/// looked for it in vain among this machine's cells. A step's date is
/// read in `zone`. A router is known by its README in this machine's
/// store, which holds those of quests whose cells run elsewhere too.
#[must_use]
pub fn cells(
    answers: &[(String, String)],
    quests: Option<&[Quest]>,
    here: &str,
    zone: &TimeZone,
    now: i64,
    fallback: &str,
) -> Vec<Measure> {
    // A realm counts as read only where its node named its sessions; an
    // error from it says nothing of them.
    let parsed: Vec<(&str, Value)> = answers
        .iter()
        .filter_map(|(realm, text)| Some((realm.as_str(), serde_json::from_str(text).ok()?)))
        .filter(|(_, answer): &(&str, Value)| answer["Ok"]["Sessions"].is_array())
        .collect();
    let realms: BTreeSet<&str> = parsed.iter().map(|(realm, _)| *realm).collect();
    let all: Vec<Cell> = once(
        parsed
            .iter()
            .flat_map(|(_, answer)| answer["Ok"]["Sessions"].as_array().into_iter().flatten())
            .filter_map(read),
    );
    let readme: BTreeMap<&str, &Quest> = quests
        .unwrap_or_default()
        .iter()
        .map(|q| (q.id.as_str(), q))
        .collect();
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
    // A step that names Joel waits on him, which no orchestrator can move;
    // a quest that routes is idle by design; and a step dated ahead counts
    // its idle time from its start, as idle.rs wakes it.
    let idle = all
        .iter()
        .filter(|c| c.state == "idle" && !c.wanting)
        .filter(|c| {
            !c.quest
                .and_then(|q| readme.get(q))
                .is_some_and(|q| q.routes)
        })
        .filter_map(|c| {
            let next = c.next.filter(|next| !next.contains("Joel"))?;
            let at = c.at?;
            let since = dated::starts(next, now, zone).map_or(at, |s| s.max(at));
            if now - since < IDLE_AT_MOST {
                return None;
            }
            let minutes = (now - since) / 60;
            let on = c
                .machine
                .filter(|m| *m != here)
                .map_or_else(String::new, |m| format!(" on {m}"));
            Some(format!(
                "{}{on} idle {minutes} min: {} · {}",
                c.address,
                short(next),
                owner(c.quest, c.orchestrator)
            ))
        })
        .collect();
    let parked_done = parked_done(&all, &readme, &owner);
    let (never, ended) = unrun(quests.unwrap_or_default(), &all, &realms, &|q| {
        owner(Some(q), None)
    });
    // A realm that didn't answer would read as one whose cells all went,
    // closing their owners' drags; and without the READMEs owners fall
    // back and quests' statuses go stale. Either leaves every measure
    // unjudged.
    let unread = if parsed.is_empty() || parsed.len() < answers.len() {
        Some("a realm's node didn't name its sessions")
    } else if quests.is_none() {
        Some("the quests' READMEs couldn't be read")
    } else {
        None
    };
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
    .map(|(key, says, behind)| judged(key, says, behind, unread, fallback))
    .collect()
}

/// The ended cells parked on a quest already concluded or retired, each
/// with its owner.
fn parked_done(
    all: &[Cell],
    readme: &BTreeMap<&str, &Quest>,
    owner: &dyn Fn(Option<&str>, Option<&str>) -> String,
) -> Vec<String> {
    all.iter()
        .filter(|c| c.presence == Some("parked") && c.state == "ended")
        .filter_map(|c| {
            // The README is written as the quest ends; the cell's word of
            // its quest dates from when it parked.
            let status = c
                .quest
                .and_then(|q| readme.get(q))
                .map(|q| q.status.as_str())
                .or(c.quest_status)?;
            matches!(status, "concluded" | "retired").then(|| {
                format!(
                    "{} parked, its quest {status} · {}",
                    c.address,
                    owner(c.quest, c.orchestrator)
                )
            })
        })
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
            // The node forgets sessions long ended, so this says only that
            // it knows of none, not that none ever ran.
            never.push(format!(
                "{named} {}, no cell the node knows · {to}",
                q.priority
            ));
        } else {
            ended.push(format!("{named} {}, every cell ended · {to}", q.priority));
        }
    }
    (never, ended)
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
            says: format!(
                "{says} ({})",
                if exit.is_none() {
                    "the map's check isn't installed"
                } else {
                    "the map's check didn't read the node"
                }
            ),
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
pub(super) mod tests {
    use serde_json::json;

    use super::*;

    pub(in crate::hour::measures) const NOW: i64 = 1_800_000_000;

    /// The machine the hour runs on.
    const HERE: &str = "here";

    /// The zone the hour reads dates in: Toronto's, in winter.
    fn zone() -> TimeZone {
        TimeZone::fixed(jiff::tz::offset(-5))
    }

    pub(in crate::hour::measures) fn at(minutes_ago: i64) -> String {
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

    pub(in crate::hour::measures) fn quest_of(
        short: &str,
        status: &str,
        priority: &str,
        parent: Option<&str>,
    ) -> Quest {
        Quest {
            id: format!("QEST-20261116-{short}"),
            realm: "work".to_owned(),
            status: status.to_owned(),
            priority: priority.to_owned(),
            parent: parent.map(|p| format!("QEST-20261007-{p}")),
            routes: false,
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
                      status: active\npriority: driving\nhears: addressed\nsessions:\n  \
                      - id: s\n    status: detached\n---\n# A Quest\n";
        assert_eq!(
            quest(readme),
            Some(Quest {
                id: "QEST-1".to_owned(),
                realm: "work".to_owned(),
                status: "active".to_owned(),
                priority: "driving".to_owned(),
                parent: Some("QEST-0".to_owned()),
                routes: true,
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
        let all = cells(&answers, Some(&[]), HERE, &zone(), NOW, "FALLBACK");
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
    fn a_router_or_a_step_dated_ahead_isnt_idle_and_another_machines_cell_says_so() {
        let idle = |address: &str, next: &str| {
            let mut c = cell(address, "idle", address);
            c["at"] = json!(at(90));
            c["doing"] = json!({"next": next});
            c["orchestrated_by"] = json!("ORCH");
            c
        };
        let mut far = idle("FFFFFF", "spawn the streams");
        far["machine"] = json!("yonder");
        let mut near = idle("NNNNNN", "merge it");
        near["machine"] = json!(HERE);
        // NOW is 03:00 on Friday 15 January 2027 in Toronto. One step
        // started half an hour ago, one ten minutes ago.
        let answers = [answer(&[
            idle("RRRRRR", "hear the next line"),
            idle("AAAAAA", "Judge it at Fri 15 23:00, over the spell"),
            idle("PPPPPP", "Judge it at Thu 14 09:00, over the spell"),
            idle("SSSSSS", "Judge it at Fri 15 02:30, over the spell"),
            idle("TTTTTT", "Judge it at Fri 15 02:50, over the spell"),
            idle("DDDDDD", "Land the hubs by Fri 15 23:00"),
            far,
            near,
        ])];
        let mut router = quest_of("RRRRRR", "active", "driving", None);
        router.routes = true;
        let all = cells(&answers, Some(&[router]), HERE, &zone(), NOW, "F");
        assert_eq!(
            measure(&all, "cells-idle").behind,
            [
                "PPPPPP idle 90 min: Judge it at Thu 14 09:00, over the spell · ORCH",
                "SSSSSS idle 30 min: Judge it at Fri 15 02:30, over the spell · ORCH",
                "DDDDDD idle 90 min: Land the hubs by Fri 15 23:00 · ORCH",
                "FFFFFF on yonder idle 90 min: spawn the streams · ORCH",
                "NNNNNN idle 90 min: merge it · ORCH",
            ],
            "a router counted, a step dated ahead counted, a due date or a start passed \
             left out, or a cell's machine misnamed"
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
        let all = cells(&answers, Some(&quests), HERE, &zone(), NOW, "FALLBACK");
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
        let all = cells(&answers, Some(&quests), HERE, &zone(), NOW, "FALLBACK");
        assert_eq!(
            measure(&all, "quests-without-cells").behind,
            ["AAAAAA driving, no cell the node knows · PARENT"],
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
    fn the_readme_outranks_a_parked_cells_old_word_of_its_quest() {
        let mut parked = cell("AAAAAA", "ended", "AAAAAA");
        parked["presence"]["state"] = json!("parked");
        // A quest with no README here, which only its cell can speak for.
        let mut elsewhere = cell("BBBBBB", "ended", "BBBBBB");
        elsewhere["presence"]["state"] = json!("parked");
        elsewhere["presence"]["questInfo"]["status"] = json!("retired");
        let quests = [quest_of("AAAAAA", "concluded", "someday", None)];
        let all = cells(
            &[answer(&[parked, elsewhere])],
            Some(&quests),
            HERE,
            &zone(),
            NOW,
            "F",
        );
        assert_eq!(
            measure(&all, "cells-parked-done").behind,
            [
                "AAAAAA parked, its quest concluded · F",
                "BBBBBB parked, its quest retired · F"
            ],
            "a quest concluded after its cell parked was read from the cell's old status, \
             or a cell's word went unheard where no README speaks"
        );
    }

    #[test]
    fn a_cell_the_node_lists_twice_counts_once_and_an_empty_orchestrator_names_no_one() {
        let idle = || {
            let mut c = cell("AAAAAA", "idle", "AAAAAA");
            c["at"] = json!(at(30));
            c["doing"] = json!({"next": "write it"});
            c["orchestrated_by"] = json!("");
            c
        };
        let quests = [quest_of("AAAAAA", "active", "driving", Some("PARENT"))];
        let all = cells(
            &[answer(&[idle(), idle()])],
            Some(&quests),
            HERE,
            &zone(),
            NOW,
            "F",
        );
        assert_eq!(
            measure(&all, "cells-idle").behind,
            ["AAAAAA idle 30 min: write it · PARENT"],
            "a cell listed twice counted twice, or an empty orchestrator was taken as one"
        );
        // Listed ended first and live after, the live listing is the cell.
        let all = cells(
            &[answer(&[cell("AAAAAA", "ended", "AAAAAA"), idle()])],
            Some(&quests),
            HERE,
            &zone(),
            NOW,
            "F",
        );
        assert_eq!(
            (
                measure(&all, "cells-idle").now,
                measure(&all, "quests-cells-ended").now
            ),
            (1.0, 0.0),
            "an ended listing outranked a live one"
        );
    }

    #[test]
    fn an_answer_with_no_sessions_or_an_unread_quest_store_is_unjudged() {
        let quests = [quest_of("AAAAAA", "active", "driving", None)];
        let err = ("work".to_owned(), json!({"Err": "no node"}).to_string());
        assert!(
            cells(&[err], Some(&quests), HERE, &zone(), NOW, "F")
                .iter()
                .all(|m| m.target.is_none()),
            "a node's error was read as a realm with no cells"
        );
        // One realm answering and another not: the missing realm's cells
        // would read as gone, and close their owners' drags.
        let silent = ("personal".to_owned(), String::new());
        assert!(
            cells(
                &[answer(&[]), silent],
                Some(&quests),
                HERE,
                &zone(),
                NOW,
                "F"
            )
            .iter()
            .all(|m| m.target.is_none()),
            "one realm's silence was read as that realm having no cells"
        );
        // With no READMEs, owners fall back and statuses go stale.
        assert!(
            cells(&[answer(&[])], None, HERE, &zone(), NOW, "F")
                .iter()
                .all(|m| m.target.is_none()),
            "with no quest store a cells measure read judged"
        );
    }

    #[test]
    fn with_no_realm_answering_the_cells_are_unjudged() {
        let all = cells(
            &[],
            Some(&[quest_of("AAAAAA", "active", "driving", None)]),
            HERE,
            &zone(),
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
        assert!(
            mapped(None, "", "F")
                .says
                .contains("(the map's check isn't installed)"),
            "a missing check was said to have failed to read the node"
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
