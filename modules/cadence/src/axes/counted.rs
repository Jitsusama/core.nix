//! Readings another program already counts, read from what it writes
//! rather than counted again: code-health's regressions, which cadence
//! day judges into its findings ledger under keys starting `code.`; the
//! lines where Joel sounded pained, which health counts in its day file;
//! the turns making one tool call, which the daily measure counts; and
//! the security musts past due, which ISXEG8 files in the ledger.
//! Each stays unread until its program has written a day.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use jiff::Timestamp;
use jiff::tz::TimeZone;

use super::{Bar, Reading, open, shown};
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

/// ISXEG8's musts open past their date: the ledger's open findings on the
/// security axis whose answer was due before `now`.
#[must_use]
pub fn musts(s: &Sources, now: i64) -> Reading {
    let late = s.findings.as_ref().map(|all| {
        all.iter()
            .filter(|f| f.axis == "security" && f.due < now)
            .count()
    });
    Reading::new("{} security musts past due", late.map(count), "", "ISXEG8")
        .bar(Bar::AtMost(0.0))
        .from("cadence day's findings, as ISXEG8 files them")
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

/// The share of the last day's turns that made exactly one tool call, from
/// the newest measure's "Turns Doing One Thing" table, whose first row is
/// the last day: `| ending | turns | one | none | several |`.
#[must_use]
pub fn one_call(measure: Option<&str>) -> Reading {
    let latest = measure.and_then(|text| {
        let (_, section) = text.split_once("## Turns Doing One Thing")?;
        let section = section.split("\n## ").next().unwrap_or_default();
        section
            .lines()
            .filter(|line| line.starts_with("| 20"))
            .find_map(|row| {
                let cells: Vec<f64> = row
                    .split('|')
                    .map(str::trim)
                    .skip(2)
                    .filter_map(|cell| cell.replace(',', "").parse().ok())
                    .collect();
                match cells.as_slice() {
                    [turns, one, ..] if *turns > 0.0 => Some((*one, *turns)),
                    _ => None,
                }
            })
    });
    Reading::new(
        "turns making one tool call {}",
        latest.map(|(one, turns)| one / turns * 100.0),
        "%",
        "QCXZEB",
    )
    .bar(Bar::AtMost(60.0))
    .detail(
        latest.map(|(one, turns)| {
            format!("{} of {} in the last day", shown(one, ""), shown(turns, ""))
        }),
    )
    .from("the measure's turns doing one thing")
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
        on(key, "quality", 0)
    }

    fn on(key: &str, axis: &str, due: i64) -> Entry {
        serde_json::from_str(&format!(
            r#"{{"key":"{key}","first":"2026-11-12","last":"2026-11-12","last_to":0,
            "due":{due},"words":"rose","axis":"{axis}","evidence":"","fell_when":"",
            "open":true,"late":false}}"#
        ))
        .expect("a finding parses")
    }

    #[test]
    fn musts_are_the_security_findings_past_their_date() {
        let s = Sources {
            findings: Some(vec![
                on("must:polkit", "security", 100),
                on("must:asks", "security", 300),
                on("crashes:biome", "quality", 100),
            ]),
            ..Sources::default()
        };
        assert_eq!(
            musts(&s, 200).words(),
            "1 security must past due, at most 0"
        );
        assert_eq!(musts(&Sources::default(), 200).figure(), None);
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
    fn one_call_turns_are_the_last_days_share() {
        let measure = "# Measure\n\n## Turns Doing One Thing\n\n79 of 100 turns made one.\n\n\
            | Day ending | Turns | One call | None | Two or more |\n|---|---|---|---|---|\n\
            | 2026-11-12 | 1000 | 800 | 50 | 150 |\n| 2026-11-11 | 900 | 600 | 0 | 300 |\n\n\
            | Tool | One-call turns | Right after its own |\n|---|---|---|\n| bash | 400 | 200 |\n\n\
            ## Next\n";
        let reading = one_call(Some(measure));
        assert_eq!(
            reading.words(),
            "turns making one tool call 80%, at most 60%"
        );
        assert_eq!(one_call(Some("# Measure\n")).figure(), None);
        assert_eq!(one_call(None).figure(), None);
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
