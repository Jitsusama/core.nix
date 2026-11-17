//! The finished reports and reviews no one put to Joel: on 16 November
//! 9ZI7PR's review sat nine hours behind a look that asked nothing and
//! cleared once seen, and the hour saw none of it.

use std::collections::BTreeMap;

use serde_json::Value;

use super::cells::{Quest, short_id};
use super::{Class, Measure, count, seconds};
use crate::axes::plans::{field, front};

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
pub const REPORTS_SINCE: i64 = 3 * 24 * 3600;

/// How long before a report's last write a look may come and still be
/// about it, since a session puts the look up as it finishes writing.
const LOOK_BEFORE: i64 = 3600;

/// Reports and reviews still in draft, written at least 2 hours ago and in
/// the last 3 days, by a quest still active, that no look on Joel's list
/// asks him about. `looks` and `quests` are none where the look list or
/// the quests' store couldn't be read.
#[must_use]
pub fn unasked(
    reports: &[Report],
    looks: Option<&[Value]>,
    quests: Option<&[Quest]>,
    now: i64,
    fallback: &str,
) -> Measure {
    let says = "drafted reports and reviews no look on Joel's list asks him about \
                within 2 hours (a post on the change itself isn't read)";
    let unread = |why: &str| Measure {
        key: "reports-unasked".to_owned(),
        class: Class::Cells,
        says: format!("{says} ({why})"),
        now: 0.0,
        target: None,
        owner: fallback.to_owned(),
        behind: Vec::new(),
    };
    let Some(looks) = looks else {
        return unread("the look list couldn't be read");
    };
    let Some(quests) = quests else {
        return unread("the quests' READMEs couldn't be read");
    };
    let readme: BTreeMap<&str, &Quest> = quests.iter().map(|q| (q.id.as_str(), q)).collect();
    // A look that asks something, or still waits on the list unexpired,
    // puts the report to Joel; one that asked nothing clears once seen. A
    // look written before `asks` was asks something. One naming the report
    // is about it whenever it came; one only of its quest must come about
    // when the report was written.
    let asked = |r: &Report| {
        looks.iter().any(|look| {
            let named = look["target"].as_str().is_some_and(|t| t.contains(&r.id));
            let of_quest = look["quest"].as_str() == Some(r.quest.as_str())
                && seconds(&look["at"]).is_some_and(|at| at >= r.written - LOOK_BEFORE);
            let waiting = look["state"].as_str() == Some("waiting")
                && seconds(&look["expiresAt"]).is_none_or(|until| until > now);
            let puts = look["asks"].as_bool() != Some(false) || waiting;
            (named || of_quest) && puts
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::super::cells::tests::{NOW, at, quest_of};
    use super::*;

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
            drafted("IIIIII", 9, "draft"),
            drafted("JJJJJJ", 9, "draft"),
            drafted("KKKKKK", 9, "draft"),
            drafted("LLLLLL", 9, "draft"),
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
            // Asked about this file, before the report's last edit.
            look("IIIIII", "/r/RPRT-IIIIII.md", 30, true, "resolved"),
            // Waiting, but past its time on the list.
            json!({"quest": "QEST-20261116-JJJJJJ", "target": "/r/RPRT-JJJJJJ.md",
                   "at": at(8 * 60), "asks": false, "state": "waiting",
                   "expiresAt": at(60)}),
        ];
        let mut older = look("KKKKKK", "/r/RPRT-KKKKKK.md", 8, false, "resolved");
        // A look from before `asks` was written asks something.
        older.as_object_mut().expect("an object").remove("asks");
        // The quest's look about its pull request, after the report.
        let pr = look("LLLLLL", "https://a/pr/2", 8, true, "resolved");
        let looks = [looks.as_slice(), &[older, pr]].concat();
        let quests = [
            quest_of("AAAAAA", "active", "driving", Some("PARENT")),
            quest_of("BBBBBB", "active", "driving", None),
            quest_of("CCCCCC", "active", "driving", None),
            quest_of("DDDDDD", "active", "driving", None),
            quest_of("EEEEEE", "active", "driving", None),
            quest_of("FFFFFF", "concluded", "driving", None),
            quest_of("GGGGGG", "active", "driving", None),
            quest_of("HHHHHH", "active", "driving", None),
            quest_of("IIIIII", "active", "driving", None),
            quest_of("JJJJJJ", "active", "driving", None),
            quest_of("KKKKKK", "active", "driving", None),
            quest_of("LLLLLL", "active", "driving", None),
        ];
        let unasked = unasked(&reports, Some(&looks), Some(&quests), NOW, "FALLBACK");
        assert_eq!(
            unasked.behind,
            [
                "AAAAAA's RPRT-AAAAAA, drafted 9 h ago, asked nothing · PARENT",
                "HHHHHH's RPRT-HHHHHH, drafted 9 h ago, asked nothing · FALLBACK",
                "JJJJJJ's RPRT-JJJJJJ, drafted 9 h ago, asked nothing · FALLBACK"
            ],
            "a report no look asks about was missed, or one asked, waiting, too new, \
             concluded, of a done quest or too old counted"
        );
        assert_eq!(unasked.target, Some(0.0), "a read list went unjudged");
        assert_eq!(
            super::unasked(&reports, None, Some(&quests), NOW, "F").target,
            None,
            "an unread list judged the reports"
        );
        assert_eq!(
            super::unasked(&reports, Some(&looks), None, NOW, "F").target,
            None,
            "with no quest store the reports read a judged 0"
        );
    }
}
