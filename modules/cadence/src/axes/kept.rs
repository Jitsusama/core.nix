//! Whether the loop's fixes stayed fixed, from the ledger `cadence day`
//! keeps as findings.jsonl (findings.rs). A figure finding closes once two
//! days' reads no longer find it; one that falls outside its bar again
//! later is a fix that didn't hold. This reads the whole ledger, closed
//! entries too, where open.rs reads only what's open.

use std::collections::BTreeMap;

use super::readings::count;
use super::{Bar, Reading, shown};
use crate::day;
use crate::findings::Entry;

/// How far back a fix is still watched for coming back: the 28-day reread
/// of plan item 5, which a fix that holds that long has passed.
const WATCHED: i64 = 28 * 86_400;

/// Each kept day's figures by its key, by the day's date.
pub type Dated = BTreeMap<String, BTreeMap<String, f64>>;

/// The fixes of the last four weeks that came back, out of all of them,
/// from the ledger's text `ledger`; none read where there's no ledger. A
/// fix is a figure finding the day closed, its kind none, last found in
/// the four weeks; musts and holes close by their owners' word, not by a
/// figure, so they aren't fixes here. A fix that came back is one the day
/// reopened, its line counting how often; one written before the day
/// reopened lines came back when a later line carries its key.
#[must_use]
pub fn came_back(ledger: &str, dated: &Dated, now: i64) -> Reading {
    let entries: Vec<Entry> = ledger
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    let fixes: Vec<(usize, &Entry)> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            (!e.open || e.reopened > 0) && e.kind.is_none() && e.last_to > now - WATCHED
        })
        .collect();
    let back: Vec<&Entry> = fixes
        .iter()
        .filter(|(at, fix)| fix.reopened > 0 || entries[at + 1..].iter().any(|e| e.key == fix.key))
        .map(|(_, fix)| *fix)
        .collect();
    let detail = if fixes.is_empty() {
        "no fix in the last four weeks yet".to_owned()
    } else {
        let named = back.first().map_or_else(String::new, |fix| {
            let owner = fix.owner.as_deref().unwrap_or("no owner yet");
            format!(" · came back: {} · {owner}", fix.words)
        });
        let moved = fixes
            .iter()
            .filter(|(_, fix)| !back.iter().any(|b| b.key == fix.key))
            .find_map(|(_, fix)| then_and_now(fix, dated))
            .map_or_else(String::new, |said| format!(" · held: {said}"));
        format!(
            "kept down {} of {}{named}{moved}",
            fixes.len() - back.len(),
            fixes.len()
        )
    };
    Reading::new(
        "{} fixes of the last four weeks that came back",
        (!ledger.trim().is_empty()).then(|| count(back.len())),
        "",
        OWNER,
    )
    .bar(Bar::AtMost(0.0))
    .detail((!ledger.trim().is_empty()).then_some(detail))
    .from("cadence day's findings, closed ones too")
}

/// What a fix that held moved: its figure on the day it was last out,
/// and on the newest kept day, in its measure's words and unit; none
/// where either day or the measure isn't kept, so no figure is made up.
/// How often each happens isn't counted anywhere yet, so this stops short
/// of what the fix saves a day.
fn then_and_now(fix: &Entry, dated: &Dated) -> Option<String> {
    let measure = day::measures().find(|m| m.holds(&fix.key))?;
    let then = dated.get(&fix.last)?.get(&fix.key)?;
    let (newest, figures) = dated.last_key_value()?;
    let now = figures.get(&fix.key)?;
    (*newest != fix.last).then(|| {
        format!(
            "{}, {} then, {} now",
            measure.says,
            shown(*then, measure.unit),
            shown(*now, measure.unit)
        )
    })
}

/// Who rereads the fixes, and so moves this reading's owner to each fix's.
const OWNER: &str = "1JQPPC";

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_794_700_000;
    const DAY: i64 = 86_400;

    fn entry(key: &str, last_to: i64, open: bool, kind: Option<&str>) -> String {
        serde_json::json!({
            "key": key, "owner": "H6EDLC", "first": "2026-11-12", "last": "2026-11-12",
            "last_to": last_to, "due": last_to + DAY, "words": format!("{key} rose"),
            "axis": "polish", "evidence": "", "fell_when": "", "open": open,
            "told": "2026-11-12", "late": false, "kind": kind,
        })
        .to_string()
    }

    const DRAWER: &str = "launcher.drawers.presented";

    /// Two kept days' figures: the day the drawer was last out, and today.
    fn dated() -> Dated {
        [("2026-11-12", 66.0), ("2026-11-14", 39.0)]
            .into_iter()
            .map(|(date, ms)| (date.to_owned(), BTreeMap::from([(DRAWER.to_owned(), ms)])))
            .collect()
    }

    #[test]
    fn a_fix_found_again_after_it_closed_came_back() {
        let ledger = [
            entry(DRAWER, NOW - 3 * DAY, false, None),
            entry("back", NOW - 5 * DAY, false, None),
            entry("long ago", NOW - 40 * DAY, false, None),
            entry("a must", NOW - 2 * DAY, false, Some("must")),
            entry("never fixed", NOW - DAY, true, None),
            "not an entry".to_owned(),
            entry("back", NOW - DAY, true, None),
        ]
        .join("\n");
        let reading = came_back(&ledger, &dated(), NOW);
        assert_eq!(reading.figure(), Some(1.0), "one fix came back");
        assert!(reading.out(), "past its bar of none");
        assert_eq!(
            reading.detail.as_deref(),
            Some(
                "kept down 1 of 2 · came back: back rose · H6EDLC · held: a row's details' \
                 median time in the launcher, key to frame, 66 ms then, 39 ms now"
            ),
            "it says how many held, names what came back, and what one that held moved"
        );
    }

    #[test]
    fn a_fix_its_line_reopened_came_back() {
        let mut slipped: serde_json::Value =
            serde_json::from_str(&entry("slipped", NOW - DAY, true, None)).expect("an entry");
        slipped["reopened"] = 1.into();
        let ledger = [
            entry(DRAWER, NOW - 3 * DAY, false, None),
            slipped.to_string(),
        ]
        .join("\n");
        let reading = came_back(&ledger, &dated(), NOW);
        assert_eq!(reading.figure(), Some(1.0), "the reopened fix came back");
        assert_eq!(
            reading.detail.as_deref(),
            Some(
                "kept down 1 of 2 · came back: slipped rose · H6EDLC · held: a row's details' \
                 median time in the launcher, key to frame, 66 ms then, 39 ms now"
            ),
            "a reopened line is a fix, and one that didn't hold"
        );
    }

    #[test]
    fn a_held_fix_with_no_kept_day_is_only_counted() {
        let reading = came_back(&entry(DRAWER, NOW - DAY, false, None), &Dated::new(), NOW);
        assert_eq!(
            reading.detail.as_deref(),
            Some("kept down 1 of 1"),
            "no figures to show, so none are made up"
        );
    }

    #[test]
    fn no_fixes_yet_reads_none_and_no_ledger_reads_nothing() {
        let quiet = came_back(&entry("never fixed", NOW, true, None), &dated(), NOW);
        assert_eq!(quiet.figure(), Some(0.0), "nothing fixed, nothing back");
        assert_eq!(
            quiet.detail.as_deref(),
            Some("no fix in the last four weeks yet"),
            "it says why"
        );
        assert_eq!(
            came_back("", &dated(), NOW).figure(),
            None,
            "no ledger, unread"
        );
    }
}
