//! Whether the loop's fixes stayed fixed, from the ledger `cadence day`
//! keeps as findings.jsonl (findings.rs). A figure finding closes once two
//! days' reads no longer find it; one that falls outside its bar again
//! later is a fix that didn't hold. This reads the whole ledger, closed
//! entries too, where open.rs reads only what's open.

use super::readings::count;
use super::{Bar, Reading};
use crate::findings::Entry;

/// How far back a fix is still watched for coming back: the 28-day reread
/// of plan item 5, which a fix that holds that long has passed.
const WATCHED: i64 = 28 * 86_400;

/// The fixes of the last four weeks that came back, out of all of them,
/// from the ledger's text `ledger`; none read where there's no ledger. A
/// fix is a figure finding the day closed, its kind none, last found in
/// the four weeks; musts and holes close by their owners' word, not by a
/// figure, so they aren't fixes here. It came back when a later line
/// carries its key, since the day opens a new entry for a closed key.
#[must_use]
pub fn came_back(ledger: &str, now: i64) -> Reading {
    let entries: Vec<Entry> = ledger
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    let fixes: Vec<(usize, &Entry)> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| !e.open && e.kind.is_none() && e.last_to > now - WATCHED)
        .collect();
    let back: Vec<&Entry> = fixes
        .iter()
        .filter(|(at, fix)| entries[at + 1..].iter().any(|e| e.key == fix.key))
        .map(|(_, fix)| *fix)
        .collect();
    let detail = if fixes.is_empty() {
        "no fix in the last four weeks yet".to_owned()
    } else {
        let named = back.first().map_or_else(String::new, |fix| {
            let owner = fix.owner.as_deref().unwrap_or("no owner yet");
            format!(" · came back: {} · {owner}", fix.words)
        });
        format!(
            "kept down {} of {}{named}",
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

    #[test]
    fn a_fix_found_again_after_it_closed_came_back() {
        let ledger = [
            entry("held", NOW - 3 * DAY, false, None),
            entry("back", NOW - 5 * DAY, false, None),
            entry("long ago", NOW - 40 * DAY, false, None),
            entry("a must", NOW - 2 * DAY, false, Some("must")),
            entry("never fixed", NOW - DAY, true, None),
            "not an entry".to_owned(),
            entry("back", NOW - DAY, true, None),
        ]
        .join("\n");
        let reading = came_back(&ledger, NOW);
        assert_eq!(reading.figure(), Some(1.0), "one fix came back");
        assert!(reading.out(), "past its bar of none");
        assert_eq!(
            reading.detail.as_deref(),
            Some("kept down 1 of 2 · came back: back rose · H6EDLC"),
            "it says how many held and names what came back"
        );
    }

    #[test]
    fn no_fixes_yet_reads_none_and_no_ledger_reads_nothing() {
        let quiet = came_back(&entry("never fixed", NOW, true, None), NOW);
        assert_eq!(quiet.figure(), Some(0.0), "nothing fixed, nothing back");
        assert_eq!(
            quiet.detail.as_deref(),
            Some("no fix in the last four weeks yet"),
            "it says why"
        );
        assert_eq!(came_back("", NOW).figure(), None, "no ledger, unread");
    }
}
