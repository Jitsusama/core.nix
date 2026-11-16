//! Where each pain goes, and what it says there. A pain goes to the cell
//! of the session he said it in, which was working on what hurt, as the
//! mycelium's sessions name it; one said to the void, or to a session
//! that has ended or the mycelium doesn't know, goes to the fallback the
//! routes file names, and with none named it waits, counted open. The
//! void is never sent anything. What a finding says stays short and
//! carries links rather than context: the cell and the log entry it was
//! heard at, so whoever reads it opens the rest.

use crate::hear::heard::Heard;
use serde::Deserialize;
use std::collections::BTreeMap;

/// How many pains one finding names; the rest wait for the next run.
pub const NAMED_AT_MOST: usize = 5;

/// His words are cut to this many characters in a finding.
const WORDS_AT_MOST: usize = 280;

/// A session's cell, as the mycelium knows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub address: String,
    pub live: bool,
}

/// What the routes file, beside the listener's state, says: who findings
/// are from, where those no live cell takes go, and who keeps the asks
/// and misses ledgers.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
pub struct Routes {
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub fallback: Option<String>,
    #[serde(default)]
    pub ledgers: Option<String>,
}

/// Each session the mycelium knows, by its id, from what
/// `mycelium sessions` prints.
pub fn cells(printed: &str) -> BTreeMap<String, Cell> {
    let _ = printed;
    BTreeMap::new()
}

/// The address a pain said in `session` goes to, or none for now.
pub fn owner(
    session: &str,
    void: bool,
    cells: &BTreeMap<String, Cell>,
    routes: &Routes,
) -> Option<String> {
    let _ = (session, void, cells, routes);
    None
}

/// The finding a cell is sent for its pains, worst first, each with the
/// link to where it was heard.
pub fn finding(pains: &[&Heard], cells: &BTreeMap<String, Cell>, offset_s: i64) -> String {
    let _ = (pains, cells, offset_s, WORDS_AT_MOST);
    String::new()
}

/// The note the ledgers' keeper is sent of new pains, so it can carry them
/// into asks or misses.
pub fn ledger_note(pains: &[&Heard], cells: &BTreeMap<String, Cell>, offset_s: i64) -> String {
    let _ = (pains, cells, offset_s);
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SESSIONS: &str = r#"{"Ok":{"Sessions":[
        {"id":"s-live","address":"ABC123","state":"busy","quest":"Q1"},
        {"id":"s-ended","address":"DEF456","state":"ended"},
        {"id":"s-idle","address":"GHI789","state":"idle"}]}}"#;

    fn pain(session: &str, entry: &str, words: &str, hits: usize) -> Heard {
        Heard {
            key: format!("{session}/{entry}"),
            session: session.into(),
            entry: entry.into(),
            // 18:55 UTC on 16 November 2026.
            at: 1_794_855_300_000,
            words: words.into(),
            kinds: vec!["corrected".into()],
            looking_at: Some("kitty · mycelium forage".into()),
            tools: vec!["bash".into(), "edit".into()],
            failed: vec!["edit".into()],
            hits,
            carried: false,
            to: None,
            routed_at: None,
        }
    }

    fn routes() -> Routes {
        Routes {
            from: Some("EAR001".into()),
            fallback: Some("ORCH01".into()),
            ledgers: Some("ORCH01".into()),
        }
    }

    #[test]
    fn the_mycelium_names_each_sessions_cell() {
        let cells = cells(SESSIONS);
        assert_eq!(
            cells.get("s-live"),
            Some(&Cell { address: "ABC123".into(), live: true })
        );
        assert_eq!(cells.get("s-ended").map(|c| c.live), Some(false));
        assert_eq!(cells.get("s-idle").map(|c| c.live), Some(true), "idle is alive");
        assert!(super::cells("not json").is_empty());
    }

    #[test]
    fn a_pain_goes_to_its_sessions_cell_else_the_fallback_and_never_the_void() {
        let cells = cells(SESSIONS);
        let routes = routes();
        assert_eq!(owner("s-live", false, &cells, &routes), Some("ABC123".into()));
        assert_eq!(owner("s-ended", false, &cells, &routes), Some("ORCH01".into()));
        assert_eq!(owner("unknown", false, &cells, &routes), Some("ORCH01".into()));
        assert_eq!(
            owner("s-live", true, &cells, &routes),
            Some("ORCH01".into()),
            "the void's pains go to the fallback, never to the void"
        );
        assert_eq!(owner("s-ended", false, &cells, &Routes::default()), None);
    }

    #[test]
    fn a_finding_is_short_carries_links_and_asks_for_a_time() {
        let cells = cells(SESSIONS);
        let worst = pain("s-live", "e1", "the forage command does not fit small terminals", 3);
        let other = pain("s-live", "e2", &"x".repeat(400), 1);
        let said = finding(&[&worst, &other], &cells, -5 * 3600);
        assert!(said.contains("13:55"), "his local time: {said}");
        assert!(said.contains("mycelium:cell/ABC123#e1"), "the link: {said}");
        assert!(said.contains("\"the forage command does not fit small terminals\""), "{said}");
        assert!(said.contains("3 times"), "how often he hit it: {said}");
        assert!(said.contains("kitty · mycelium forage"), "what he looked at: {said}");
        assert!(said.contains("edit failed"), "what failed before it: {said}");
        assert!(said.contains("red case"), "asks for a red case: {said}");
        assert!(said.contains("hours"), "asks for a time in hours: {said}");
        assert!(!said.contains(&"x".repeat(300)), "long words are cut: {said}");
        assert!(said.find("e1").unwrap_or(usize::MAX) < said.find("e2").unwrap_or(0));
    }

    #[test]
    fn a_pain_from_elsewhere_says_where() {
        let cells = cells(SESSIONS);
        let ended = pain("s-ended", "e9", "why did it stop", 1);
        let said = finding(&[&ended], &cells, 0);
        assert!(said.contains("DEF456"), "names the ended cell: {said}");
        assert!(said.contains("ended"), "says it ended: {said}");
    }

    #[test]
    fn the_ledgers_keeper_hears_each_new_pain_in_a_line() {
        let cells = cells(SESSIONS);
        let a = pain("s-live", "e1", "the bar is tight", 1);
        let b = pain("s-idle", "e2", "no window opened", 2);
        let note = ledger_note(&[&a, &b], &cells, 0);
        assert!(note.contains("2 new pains"), "{note}");
        assert!(note.contains("\"the bar is tight\""), "{note}");
        assert!(note.contains("GHI789"), "{note}");
        assert!(note.contains("asks or misses"), "{note}");
    }
}
