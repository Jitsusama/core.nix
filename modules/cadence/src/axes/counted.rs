//! Readings another program already counts, read from what it writes
//! rather than counted again: code-health's regressions, which cadence
//! day judges into its findings ledger under keys starting `code.`, and
//! the lines where Joel sounded pained, which health counts in its day
//! file. Either stays unread until its program has written a day.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use jiff::Timestamp;
use jiff::tz::TimeZone;

use super::{Bar, Reading, open};
use crate::findings::Entry;

/// Everything these readings are read from.
#[derive(Debug, Default)]
pub struct Sources {
    /// The ledger's open findings, when cadence day has kept one.
    pub findings: Option<Vec<Entry>>,
    /// Whether code-health has written a day yet.
    pub code_health: bool,
    /// Today's figures from health's day file, when it wrote one.
    pub health: Option<BTreeMap<String, f64>>,
}

/// The code-health regressions found and not yet fixed: the open
/// findings cadence day keyed `code.<repo>.<measure>`.
#[must_use]
pub fn regressions(s: &Sources) -> Reading {
    let open = s
        .findings
        .as_ref()
        .filter(|_| s.code_health)
        .map(|all| all.iter().filter(|f| f.key.starts_with("code.")).count());
    Reading::new("{} code-health regressions", open.map(count), "", "Y9XISP")
        .bar(Bar::AtMost(0.0))
        .detail(Some(
            "found worse than the days before and still open".to_owned(),
        ))
        .from("cadence day's findings, from code-health's figures")
}

/// The lines Joel typed today that sounded pained, as health counts them.
/// Each is a pain point the loop learns from, so the bar is none.
#[must_use]
pub fn pained(s: &Sources) -> Reading {
    Reading::new(
        "{} lines where Joel sounded pained today",
        s.health
            .as_ref()
            .and_then(|figures| figures.get("health.words.errors").copied()),
        "",
        "1JQPPC",
    )
    .bar(Bar::AtMost(0.0))
    .from("health's day file")
    .his()
}

fn count(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

/// Reads the ledger in `folder`, whether code-health has written, and
/// health's file for the day `now` falls in: files only.
#[must_use]
pub fn gather(folder: &Path, state_home: &Path, now: i64) -> Sources {
    let date = Timestamp::from_second(now)
        .map(|t| {
            t.to_zoned(TimeZone::system())
                .strftime("%Y-%m-%d")
                .to_string()
        })
        .unwrap_or_default();
    Sources {
        findings: folder
            .join("findings.jsonl")
            .exists()
            .then(|| open::gather(folder)),
        code_health: fs::read_dir(state_home.join("code-health"))
            .into_iter()
            .flatten()
            .flatten()
            .any(|e| e.path().extension().is_some_and(|x| x == "json")),
        health: fs::read_to_string(state_home.join(format!("health/{date}.json")))
            .ok()
            .map(|json| crate::machine::figures(&json)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(key: &str) -> Entry {
        serde_json::from_str(&format!(
            r#"{{"key":"{key}","first":"2026-11-12","last":"2026-11-12","last_to":0,
            "due":0,"words":"rose","axis":"quality","evidence":"","fell_when":"",
            "open":true,"late":false}}"#
        ))
        .expect("a finding parses")
    }

    #[test]
    fn regressions_are_the_open_code_findings() {
        let s = Sources {
            findings: Some(vec![
                finding("code.core.nix.long-files"),
                finding("crashes:biome"),
                finding("code.mycelium.suppressions"),
            ]),
            code_health: true,
            health: None,
        };
        assert_eq!(
            regressions(&s).words(),
            "2 code-health regressions, at most 0"
        );
    }

    #[test]
    fn regressions_wait_for_code_health_and_the_ledger() {
        let before = Sources {
            findings: Some(vec![]),
            code_health: false,
            health: None,
        };
        assert_eq!(regressions(&before).figure(), None);
        let no_ledger = Sources {
            findings: None,
            code_health: true,
            health: None,
        };
        assert_eq!(regressions(&no_ledger).figure(), None);
    }

    #[test]
    fn pained_lines_come_from_healths_figure() {
        let s = Sources {
            health: Some(BTreeMap::from([("health.words.errors".to_owned(), 1.0)])),
            ..Sources::default()
        };
        assert_eq!(
            pained(&s).words(),
            "1 line where Joel sounded pained today, at most 0"
        );
        assert_eq!(pained(&Sources::default()).figure(), None);
    }
}
