//! A finding: something a detector saw, explained, ranked and answerable.
//! Each has a key naming its cause, so one cause is one finding however
//! many passes see it; it clears only once a pass no longer raises it and
//! an hour has passed since it last did, so a figure hovering at a
//! threshold doesn't come and go. Marked expected, it stays in the
//! collection but never asks for the person again.

use crate::history::History;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

/// How long a finding no pass raises stays before it clears.
const SETTLE: f64 = 3600.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Note,
    Warning,
    Alert,
}

impl Severity {
    pub fn name(self) -> &'static str {
        match self {
            Severity::Note => "note",
            Severity::Warning => "warning",
            Severity::Alert => "alert",
        }
    }

    pub fn parse(text: &str) -> Severity {
        match text {
            "alert" => Severity::Alert,
            "warning" => Severity::Warning,
            _ => Severity::Note,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Action {
    pub name: String,
    pub run: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    pub key: String,
    pub detector: String,
    pub severity: Severity,
    /// What it is, in a line: "a burn in graph-bridge".
    pub title: String,
    /// What drove it and how it was judged.
    pub detail: String,
    /// The money it's about: an excess, a cost, what's at stake.
    pub dollars: f64,
    /// How unlikely, as -log10 of its chance, so 2 is one in a hundred.
    pub surprise: f64,
    /// The margin's short form: "$180/h", "91%".
    pub badge: String,
    /// Why, in a few words, for the launcher's footer: "usual $40/h for
    /// this hour". The detail says it whole.
    #[serde(default)]
    pub note: String,
    pub session: Option<String>,
    pub quest: Option<String>,
    pub action: Option<Action>,
    /// Whether it may ask for the person: a cap's events always may,
    /// others within the notice budget.
    pub cap: bool,
    #[serde(default)]
    pub started: f64,
    #[serde(default)]
    pub marked: bool,
}

impl Finding {
    /// How it ranks against the rest: its severity first, then how
    /// surprising and how much money, together.
    pub fn rank(&self) -> (Severity, f64) {
        (
            self.severity,
            (self.surprise.max(0.5)) * self.dollars.max(1.0).sqrt(),
        )
    }
}

pub fn sort(findings: &mut [Finding]) {
    findings.sort_by(|one, other| {
        let (a, b) = (one.rank(), other.rank());
        b.0.cmp(&a.0).then(b.1.total_cmp(&a.1))
    });
}

fn fail(error: rusqlite::Error) -> String {
    error.to_string()
}

/// Keeps what this pass raised, clears what's settled, and says what holds
/// now with when each started and whether it's marked.
pub fn settle(history: &History, raised: Vec<Finding>, now: f64) -> Result<Vec<Finding>, String> {
    let db = &history.db;
    for finding in &raised {
        let started: Option<f64> = db
            .query_row(
                "SELECT started FROM findings WHERE key = ?1 AND cleared IS NULL",
                [&finding.key],
                |row| row.get(0),
            )
            .optional()
            .map_err(fail)?;
        db.execute(
            "INSERT OR REPLACE INTO findings VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,NULL,?10,?11,?12)",
            params![
                finding.key,
                finding.detector,
                finding.severity.name(),
                finding.title,
                serde_json::to_string(finding).unwrap_or_default(),
                finding.dollars,
                finding.surprise,
                started.unwrap_or(now),
                now,
                finding
                    .action
                    .as_ref()
                    .map(|action| serde_json::to_string(action).unwrap_or_default()),
                finding.session,
                finding.quest,
            ],
        )
        .map_err(fail)?;
    }
    // A source is read again or it isn't, with no noise to settle, so its
    // finding clears on the first pass that doesn't raise it.
    db.execute(
        "UPDATE findings SET cleared = ?1 WHERE cleared IS NULL
         AND (updated < ?2 OR (detector = 'health' AND updated < ?1))",
        params![now, now - SETTLE],
    )
    .map_err(fail)?;
    // A mark is for one episode: a cap near again next week asks again.
    db.execute(
        "DELETE FROM marks WHERE key IN (SELECT key FROM findings WHERE cleared IS NOT NULL)",
        [],
    )
    .map_err(fail)?;
    standing(history)
}

/// What holds now, best first.
pub fn standing(history: &History) -> Result<Vec<Finding>, String> {
    let mut statement = history
        .db
        .prepare(
            "SELECT f.detail, f.started, m.mark FROM findings f LEFT JOIN marks m ON m.key = f.key
             WHERE f.cleared IS NULL",
        )
        .map_err(fail)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(fail)?;
    let mut out = Vec::new();
    for row in rows {
        let (detail, started, mark) = row.map_err(fail)?;
        if let Ok(mut finding) = serde_json::from_str::<Finding>(&detail) {
            finding.started = started;
            finding.marked = mark.as_deref() == Some("expected");
            out.push(finding);
        }
    }
    sort(&mut out);
    Ok(out)
}

/// Marks a finding expected, by its key or the start of it: the whole key
/// when one holds by that name, as the shell gives it, else the one key
/// that starts so.
pub fn mark(history: &History, key: &str, now: f64) -> Result<String, String> {
    let holding = |query: &str| -> Result<Vec<String>, String> {
        let mut statement = history.db.prepare(query).map_err(fail)?;
        statement
            .query_map([key], |row| row.get(0))
            .map_err(fail)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(fail)
    };
    let mut found = holding("SELECT key FROM findings WHERE key = ?1 AND cleared IS NULL")?;
    if found.is_empty() {
        found = holding(
            "SELECT key FROM findings WHERE substr(key, 1, length(?1)) = ?1 AND cleared IS NULL",
        )?;
    }
    match found.as_slice() {
        [one] => {
            history
                .db
                .execute(
                    "INSERT OR REPLACE INTO marks VALUES (?1, 'expected', ?2)",
                    params![one, now],
                )
                .map_err(fail)?;
            Ok(one.clone())
        }
        [] => Err(format!("no finding holds whose key starts {key}")),
        many => Err(format!(
            "{} findings' keys start {key}: {}",
            many.len(),
            many.join(", ")
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(key: &str, severity: Severity, dollars: f64) -> Finding {
        Finding {
            key: key.into(),
            detector: "test".into(),
            severity,
            title: key.into(),
            detail: String::new(),
            dollars,
            surprise: 2.0,
            badge: String::new(),
            note: String::new(),
            session: None,
            quest: None,
            action: None,
            cap: false,
            started: 0.0,
            marked: false,
        }
    }

    // A source that can be read again is said to be fine at once: its
    // finding doesn't wait out the hour a burn's does.
    #[test]
    fn a_source_back_clears_at_once() {
        let history = History::memory();
        let mut down = finding("health:proxy:meter", Severity::Note, 0.0);
        down.detector = "health".into();
        settle(&history, vec![down], 0.0).unwrap();
        assert!(settle(&history, vec![], 30.0).unwrap().is_empty());
    }

    #[test]
    fn one_cause_is_one_finding_and_it_settles_before_clearing() {
        let history = History::memory();
        settle(
            &history,
            vec![finding("burn:s1", Severity::Alert, 100.0)],
            0.0,
        )
        .unwrap();
        let held = settle(
            &history,
            vec![finding("burn:s1", Severity::Alert, 120.0)],
            60.0,
        )
        .unwrap();
        assert_eq!(held.len(), 1);
        assert_eq!(held[0].started, 0.0);
        assert_eq!(
            settle(&history, vec![], 600.0).unwrap().len(),
            1,
            "not yet settled"
        );
        assert!(
            settle(&history, vec![], 60.0 + SETTLE + 1.0)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn an_alert_outranks_a_larger_note_and_a_mark_is_kept() {
        let history = History::memory();
        let held = settle(
            &history,
            vec![
                finding("note", Severity::Note, 5000.0),
                finding("cap", Severity::Alert, 10.0),
            ],
            0.0,
        )
        .unwrap();
        assert_eq!(held[0].key, "cap");
        mark(&history, "no", 1.0).unwrap();
        assert!(
            standing(&history)
                .unwrap()
                .iter()
                .any(|f| f.key == "note" && f.marked)
        );
    }

    // The shell marks by the whole key, which may start another's, and a
    // key's _ is a character like any other.
    #[test]
    fn a_whole_key_is_marked_though_it_starts_another() {
        let history = History::memory();
        let held = vec![
            finding("new:model:opus", Severity::Warning, 5.0),
            finding("new:model:opus-4", Severity::Warning, 5.0),
            finding("new:tool:a_b", Severity::Warning, 5.0),
        ];
        settle(&history, held, 0.0).unwrap();
        assert_eq!(
            mark(&history, "new:model:opus", 1.0).unwrap(),
            "new:model:opus"
        );
        assert!(mark(&history, "new:tool:a%", 1.0).is_err());
        assert!(mark(&history, "new:model:op", 1.0).is_err(), "two start so");
    }

    // Marking this week's near cap leaves next week's to ask.
    #[test]
    fn a_mark_lasts_one_episode() {
        let history = History::memory();
        settle(
            &history,
            vec![finding("cap:near", Severity::Warning, 5.0)],
            0.0,
        )
        .unwrap();
        mark(&history, "cap:near", 1.0).unwrap();
        settle(&history, vec![], 2.0 * SETTLE).unwrap();
        let back = settle(
            &history,
            vec![finding("cap:near", Severity::Warning, 5.0)],
            7.0 * 86400.0,
        )
        .unwrap();
        assert!(!back[0].marked);
        assert_eq!(back[0].started, 7.0 * 86400.0);
    }
}
