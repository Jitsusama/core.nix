//! How many of Joel's pains are still open, from the pain-points hearer's
//! hourly summary. Joel asked on 11 November that we learn from every pain
//! point he has, and the card measured only what our own ledgers held, so a
//! pain he said aloud and nobody took up never reached it.

use serde_json::Value;

use super::{Class, Measure, count};

/// The oldest the hearer's summary may be, in seconds: it writes every five
/// minutes, so three missed writes mean it has stopped.
const STALE_AFTER: i64 = 15 * 60;

/// How far ahead of the hour's clock a summary may be dated, in seconds,
/// for clocks a moment apart; one later than that would read fresh forever.
const AHEAD_AT_MOST: i64 = 60;

/// The most characters of the worst pain's words the card quotes.
const QUOTED: usize = 120;

/// `words` on one line, cut at a word boundary to at most `QUOTED`
/// characters, his own double quotes made single so they can't close the
/// card's.
fn quoted(words: &str) -> String {
    let words = words
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('"', "'");
    if words.chars().count() <= QUOTED {
        return words;
    }
    // One character past the cut says whether it fell inside a word.
    let cut: String = words.chars().take(QUOTED + 1).collect();
    let kept = cut.rsplit_once(' ').map_or(&cut[..0], |(kept, _)| kept);
    format!("{kept} ...")
}

/// How many times, in words.
fn times(hits: u64) -> String {
    match hits {
        1 => "once".to_owned(),
        n => format!("{n} times"),
    }
}

/// The keeper of Joel's ledgers, from the hearer's `routes.json`, who
/// carries a pain into them.
#[must_use]
pub fn keeper(routes: Option<&str>) -> Option<String> {
    let routes: Value = serde_json::from_str(routes?).ok()?;
    routes.get("ledgers")?.as_str().map(str::to_owned)
}

/// Why the summary `read` says nothing at `now`, if it doesn't.
fn unread(read: &Value, now: i64) -> Option<String> {
    if read.get("open").and_then(Value::as_u64).is_none() {
        return Some("the pain-points summary names no count".to_owned());
    }
    match read.get("at").and_then(Value::as_i64).map(|ms| ms / 1000) {
        None => Some("the pain-points summary isn't dated".to_owned()),
        Some(at) if now - at > STALE_AFTER => Some(format!(
            "the pain-points summary is {} min old",
            (now - at) / 60
        )),
        Some(at) if at - now > AHEAD_AT_MOST => Some(format!(
            "the pain-points summary is dated {} min ahead",
            (at - now) / 60
        )),
        Some(_) => None,
    }
}

/// The measure from `summary`, the hearer's `hour.json` (none when it
/// can't be read), at `now` in seconds, steered by `owner`: unjudged when
/// the summary is missing, torn, uncounted, undated, stale or ahead, since
/// a count from a hearer that isn't hearing says nothing. Its own troubles
/// are all of routing, so they're named beside a count that still holds.
/// A pain is open until someone takes it up, however old, unless the
/// ledgers carry it, as the hearer counts it; one it calls lost, past a
/// day with its question's thread never seen, counts with them. Its
/// class comes last, so it takes the drag only once the rest are in: the hearer routes each pain to its owner itself, and a
/// drag every hour would keep broken checks and slow merges from theirs.
#[must_use]
pub fn pains(summary: Option<&str>, now: i64, owner: &str) -> Measure {
    let mut measure = Measure {
        key: "pains-open".to_owned(),
        class: Class::Pains,
        says: "pains Joel said that nobody has taken up".to_owned(),
        now: 0.0,
        target: Some(0.0),
        owner: owner.to_owned(),
        behind: Vec::new(),
    };
    let read: Option<Value> = summary.and_then(|text| serde_json::from_str(text).ok());
    let Some(read) = read else {
        measure.target = None;
        measure
            .says
            .push_str(" (unjudged: the pain-points summary couldn't be read)");
        return measure;
    };
    let open = read.get("open").and_then(Value::as_u64).unwrap_or_default();
    let lost = read.get("lost").and_then(Value::as_u64).unwrap_or_default();
    measure.now = count(usize::try_from(open.saturating_add(lost)).unwrap_or(usize::MAX));
    if lost > 0 {
        measure.behind.push(format!(
            "{lost} lost: past a day, no thread of theirs seen, so only a landing naming its \
             entry closes one"
        ));
    }
    if let Some(why) = unread(&read, now) {
        measure.target = None;
        measure.says = format!("{} (unjudged: {why})", measure.says);
    }
    let troubles: Vec<&str> = read
        .get("troubles")
        .and_then(Value::as_array)
        .map(|all| all.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    if !troubles.is_empty() {
        measure.says = format!(
            "{} (the hearer couldn't route: {})",
            measure.says,
            troubles.join("; ")
        );
    }
    if let Some(worst) = read.get("worst").and_then(Value::as_str) {
        let hits = read.get("worst_hits").and_then(Value::as_u64).unwrap_or(1);
        measure.behind.push(format!(
            "the worst, said {}: \"{}\"",
            times(hits),
            quoted(worst)
        ));
    }
    measure
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::super::drag;
    use super::*;

    const NOW: i64 = 1_794_944_760;

    const WORST: &str = "i am not able to show launcher when screen sharing over tuple. this \
        sucks. we need to make it work so I can show people my UI";

    fn summary(at: i64, troubles: &str) -> String {
        format!(
            r#"{{"at":{},"heard":0,"routed":0,"open":20,"worst":"{WORST}","worst_hits":2,"stalled":0,"troubles":[{troubles}]}}"#,
            at * 1000
        )
    }

    fn worst_of(worst: &str, hits: u64) -> Vec<String> {
        let text = json!({"at": (NOW - 60) * 1000, "open": 1, "worst": worst, "worst_hits": hits})
            .to_string();
        pains(Some(&text), NOW, "WCJQSG").behind
    }

    #[test]
    fn a_broken_main_keeps_the_drag_from_open_pains() {
        let broken = Measure {
            key: "broken".to_owned(),
            class: Class::Functional,
            says: "checks broken on main".to_owned(),
            now: 1.0,
            target: Some(0.0),
            owner: "net".to_owned(),
            behind: Vec::new(),
        };
        let open = pains(Some(&summary(NOW - 60, "")), NOW, "WCJQSG");
        let hour = [open, broken];
        assert_eq!(
            drag(&hour).map(|m| m.key.as_str()),
            Some("broken"),
            "twenty open pains took the drag from a broken main, which the hearer's own \
             routing already moves"
        );
    }

    #[test]
    fn the_count_says_what_open_means() {
        let m = pains(Some(&summary(NOW - 60, "")), NOW, "WCJQSG");
        assert!(
            !m.says.contains("in the last day") && m.says.contains("nobody has taken up"),
            "the row didn't say a pain counts until taken up, however old: {m:?}"
        );
    }

    #[test]
    fn a_lost_pain_counts_open_and_says_how_it_closes() {
        let text = json!({"at": (NOW - 60) * 1000, "open": 2, "lost": 3}).to_string();
        let m = pains(Some(&text), NOW, "WCJQSG");
        assert!(m.now > 4.9 && m.now < 5.1, "open and lost both count: {m:?}");
        assert!(
            m.behind.iter().any(|b| b.starts_with("3 lost") && b.contains("naming its entry")),
            "the lost weren't said with how they close: {m:?}"
        );
    }

    #[test]
    fn the_worst_is_cut_only_inside_a_word_and_said_plainly() {
        let aaa = "aaa ".repeat(29);
        assert_eq!(
            worst_of(&format!("{aaa}bbbb tail"), 2),
            [format!("the worst, said 2 times: \"{aaa}bbbb ...\"")],
            "a word ending at the cut was dropped"
        );
        assert_eq!(
            worst_of(&format!("{aaa}bbbbbb tail"), 2),
            [format!("the worst, said 2 times: \"{aaa}...\"")],
            "a word the cut fell inside was kept in part"
        );
        assert_eq!(
            worst_of("the \"launcher\" hides", 1),
            ["the worst, said once: \"the 'launcher' hides\""],
            "his own quotes closed the card's, or one time read as 1 times"
        );
    }

    #[test]
    fn a_routing_trouble_leaves_the_count_judged_and_names_it() {
        let m = pains(
            Some(&summary(NOW - 60, "\"3 sends failed\"")),
            NOW,
            "WCJQSG",
        );
        assert!(
            m.target.is_some() && m.says.contains("the hearer couldn't route: 3 sends failed"),
            "a hearer that heard but couldn't route hid what it heard: {m:?}"
        );
    }

    #[test]
    fn the_summary_is_fresh_to_fifteen_minutes_and_never_ahead() {
        let fresh = pains(Some(&summary(NOW - STALE_AFTER, "")), NOW, "WCJQSG");
        assert!(
            fresh.target.is_some(),
            "a summary 15 min old went stale: {fresh:?}"
        );
        let stale = pains(Some(&summary(NOW - STALE_AFTER - 1, "")), NOW, "WCJQSG");
        assert!(
            stale.target.is_none(),
            "a summary past 15 min was judged: {stale:?}"
        );
    }

    #[test]
    fn the_ledgers_keeper_comes_from_the_hearers_routes() {
        assert_eq!(
            keeper(Some(r#"{"cells": {}, "ledgers": "KEEPER"}"#)).as_deref(),
            Some("KEEPER"),
            "the drag didn't go to the keeper the hearer names"
        );
        assert_eq!(keeper(Some("{\"cells\": {}}")), None);
        assert_eq!(keeper(None), None);
    }

    #[test]
    fn the_open_pains_count_with_the_worst_quoted() {
        let m = pains(Some(&summary(NOW - 60, "")), NOW, "WCJQSG");
        assert_eq!(
            (m.now, m.target),
            (20.0, Some(0.0)),
            "the open pains weren't judged: {m:?}"
        );
        assert_eq!(
            m.behind,
            [
                "the worst, said 2 times: \"i am not able to show launcher when screen sharing \
              over tuple. this sucks. we need to make it work so I can show people ...\""
            ],
            "the worst pain wasn't quoted whole words, cut: {m:?}"
        );
    }

    #[test]
    fn a_summary_missing_torn_undated_uncounted_stale_or_ahead_leaves_it_unjudged() {
        let cases = [
            (None, "couldn't be read"),
            (Some("{\"at\": 17949".to_owned()), "couldn't be read"),
            (Some(summary(NOW - 20 * 60, "")), "20 min old"),
            (Some(json!({"open": 2}).to_string()), "summary isn't dated"),
            (
                Some(json!({"at": (NOW - 60) * 1000, "open": "many"}).to_string()),
                "names no count",
            ),
            (Some(summary(NOW + 5 * 60, "")), "dated 5 min ahead"),
        ];
        for (text, why) in cases {
            let m = pains(text.as_deref(), NOW, "WCJQSG");
            assert!(
                m.target.is_none() && m.says.contains(why),
                "a hearer whose summary {why} was judged: {m:?}"
            );
        }
    }
}
