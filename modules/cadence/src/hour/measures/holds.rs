//! Live holds on the mesh with no dated fix to end them. On 24 November
//! holds stopped agents working all evening while nothing said when the
//! fix that ends each would land, and the hour saw none of it.
//!
//! A hold lives as the harness keeps it: until an answer to it ends it,
//! a later hold replaces it, or two hours pass. It's dated when the asks
//! list's keeper names it by its short id in an ask's `hold` with an
//! `expected` time, each ask's latest line deciding, since the keeper
//! moves `hold` to a hold's replacement.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use jiff::Timestamp;

use super::{Class, Measure, count, lines, seconds, short};
use crate::mesh::{Mesh, Message};

/// The key the count of undated holds takes.
const KEY: &str = "holds-undated";

/// How long the harness keeps a hold no one ends or replaces.
const LASTS: i64 = 2 * 3600;

/// Who answers for the holds when none is undated: velocity sends them.
const OWNER: &str = "1OGSE7";

/// The measure, from the mesh's messages of the last two hours under
/// `state_home`'s harness folder and the asks list `asks`.
#[must_use]
pub fn read(state_home: &Path, asks: &str, now: i64) -> Measure {
    let mesh = Mesh {
        root: state_home.join("pi/agentic-harness.pi"),
    };
    holds(&mesh.sent_since((now - LASTS) * 1000), asks, now)
}

/// The holds in `sent` still live at `now` that no ask in `asks` dates,
/// oldest first, each named by its short id, its opening words and its
/// holder, who answers for the measure while it holds the oldest.
#[must_use]
pub fn holds(sent: &[Message], asks: &str, now: i64) -> Measure {
    let ended: BTreeSet<&str> = sent
        .iter()
        .flat_map(|m| {
            let answered = m.re.as_deref().filter(|_| m.ends_hold);
            answered.into_iter().chain(m.supersedes.as_deref())
        })
        .map(short_id)
        .collect();
    let dated = dated(asks);
    let mut undated: Vec<(i64, &Message)> = sent
        .iter()
        .filter(|m| m.intent == "hold" && !ended.contains(short_id(&m.id)))
        .filter(|m| !dated.contains(short_id(&m.id)))
        .filter_map(|m| Some((m.at.parse::<Timestamp>().ok()?.as_second(), m)))
        .filter(|&(at, _)| at > now - LASTS && at <= now)
        .collect();
    undated.sort_by_key(|&(at, _)| at);
    Measure {
        key: KEY.to_owned(),
        class: Class::Slip,
        says: "live holds with no dated fix to end them".to_owned(),
        now: count(undated.len()),
        target: Some(0.0),
        owner: undated
            .first()
            .map_or(OWNER, |(_, m)| m.from.as_str())
            .to_owned(),
        behind: undated
            .iter()
            .map(|(_, m)| {
                let words = short(&m.text.replace(char::is_control, " "));
                format!("[{}] {words} · {}", short_id(&m.id), m.from)
            })
            .collect(),
    }
}

/// The short id the mesh shows a message by: the last part of its id.
fn short_id(id: &str) -> &str {
    id.rsplit('-').next().unwrap_or(id)
}

/// The holds the asks list dates, by short id: each named in `hold` on
/// an ask's latest line, keyed by when and what was asked, that gives
/// an `expected` time.
fn dated(asks: &str) -> BTreeSet<String> {
    let mut latest = BTreeMap::new();
    for ask in lines(asks) {
        let asked = [ask["at"].to_string(), ask["words"].to_string()];
        let hold = ask["hold"]
            .as_str()
            .filter(|_| seconds(&ask["expected"]).is_some())
            .map(|hold| short_id(hold).to_owned());
        latest.insert(asked, hold);
    }
    latest.into_values().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_795_600_000;

    fn at(seconds: i64) -> String {
        Timestamp::from_second(seconds)
            .map(|t| t.to_string())
            .unwrap_or_default()
    }

    fn sent(short: &str, ago: i64, from: &str, intent: &str) -> Message {
        Message {
            v: 1,
            id: format!("{}-{from}-1-{short}", (NOW - ago) * 1000),
            at: at(NOW - ago),
            from: from.to_owned(),
            to: vec!["#machine".to_owned()],
            intent: intent.to_owned(),
            text: format!("{from}: hold {short}\nuntil its fix is live"),
            pid: 1,
            re: None,
            supersedes: None,
            ends_hold: false,
        }
    }

    fn hold(short: &str, ago: i64) -> Message {
        sent(short, ago, "1OGSE7", "hold")
    }

    /// On 24 November a hold on tree cuts held for hours with no ask
    /// dating the fix that would end it, while another was dated.
    #[test]
    fn a_live_hold_no_ask_dates_is_past_its_target_and_named() {
        let asks = serde_json::json!({
            "at": at(NOW - 600), "words": "end the cargo hold", "owner": "1OGSE7",
            "hold": "a86d656e", "expected": at(NOW + 1200),
        })
        .to_string();
        let m = holds(&[hold("8ea9d31e", 3000), hold("a86d656e", 600)], &asks, NOW);
        assert!((m.now - 1.0).abs() < 0.5, "one undated, got {}", m.now);
        assert!(
            m.past().is_some(),
            "an undated hold isn't past its target of 0"
        );
        assert_eq!(
            m.behind,
            ["[8ea9d31e] 1OGSE7: hold 8ea9d31e until its fix is live · 1OGSE7"],
            "the undated hold wasn't named by its id, words and holder"
        );
        assert_eq!(m.owner, "1OGSE7", "the oldest undated hold's holder");
    }

    #[test]
    fn a_hold_ended_replaced_or_two_hours_old_is_not_counted() {
        let mut ending = sent("e0000001", 60, "WCJQSG", "fyi");
        ending.re = Some(format!("{}-1OGSE7-1-0000000a", (NOW - 900) * 1000));
        ending.ends_hold = true;
        let mut answered = sent("e0000002", 60, "16V6F4", "fyi");
        answered.re = Some("0000000b".to_owned());
        let mut replacing = hold("0000000d", 300);
        replacing.supersedes = Some(format!("{}-1OGSE7-1-0000000c", (NOW - 1800) * 1000));
        let all = [
            hold("0000000a", 900),
            hold("0000000b", 900),
            hold("0000000c", 1800),
            replacing,
            hold("0000000e", LASTS + 60),
            ending,
            answered,
        ];
        let m = holds(&all, "", NOW);
        assert_eq!(
            m.behind.iter().map(|line| &line[1..9]).collect::<Vec<_>>(),
            ["0000000b", "0000000d"],
            "an answer without endsHold ended a hold, or an ended, replaced or lapsed one counted"
        );
    }

    #[test]
    fn only_an_asks_latest_line_with_a_date_dates_its_hold() {
        let line = |hold: &str, expected: Option<i64>| {
            serde_json::json!({
                "at": at(NOW - 3000), "words": "end it", "owner": "1OGSE7",
                "hold": hold, "expected": expected.map(at),
            })
            .to_string()
        };
        let moved = [line("0000000a", Some(NOW)), line("0000000b", Some(NOW))].join("\n");
        let m = holds(&[hold("0000000a", 600), hold("0000000b", 60)], &moved, NOW);
        assert_eq!(
            m.behind.len(),
            1,
            "the hold the ask moved off still read as dated: {:?}",
            m.behind
        );
        let m = holds(&[hold("0000000a", 600)], &line("0000000a", None), NOW);
        assert!(
            (m.now - 1.0).abs() < 0.5,
            "an ask naming the hold with no date dated it"
        );
        assert_eq!(
            holds(&[], "", NOW).owner,
            OWNER,
            "with no undated hold, velocity answers for the measure"
        );
    }
}
