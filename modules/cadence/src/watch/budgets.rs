//! Each budget's standing against its allowances, as spend-watch writes it
//! to `budgets.json`. On 20 November the default budget stood at 90% of its
//! week by about 13:00 and nobody was told: spend-watch says so in pi's
//! footer and the launcher, which no one reads while the cells work alone.

use super::{Breach, Snapshot};

/// The shares of an allowance, in percent, past which the orchestrator is
/// told: once past the first and again past the second, as it asked on 20
/// November.
pub const LEVELS: [u32; 2] = [80, 90];

/// How old spend-watch's file may be and still be read, in seconds: its
/// daemon rewrites it every few minutes, so an older one is a stopped
/// daemon's, whose shares say nothing of now.
pub const FRESH_S: i64 = 30 * 60;

/// One allowance of one budget, as spend-watch last stood it.
#[derive(Debug, Clone, PartialEq)]
pub struct Allowance {
    /// The budget's name, and its label as spend-watch shows it.
    pub budget: String,
    pub label: String,
    /// The allowance's name, and its label, such as `Week`.
    pub name: String,
    pub window: String,
    /// How much of it is spent, as a share of one.
    pub share: f64,
    /// spend-watch's own words for it, such as `$36,261 of $40,000`.
    pub standing: String,
    /// When its window resets, for one that resets at a set time, so each
    /// window's crossing is told once.
    pub resets: Option<String>,
}

/// Each allowance in spend-watch's `budgets.json`, as it stood at `now`;
/// none from a file that can't be read or is older than [`FRESH_S`].
#[must_use]
pub fn read(text: &str, now: i64) -> Vec<Allowance> {
    let Ok(file) = serde_json::from_str::<serde_json::Value>(text) else {
        return Vec::new();
    };
    let written = file["at"]
        .as_str()
        .and_then(|at| at.parse::<jiff::Timestamp>().ok())
        .map(jiff::Timestamp::as_second);
    if written.is_none_or(|at| now - at > FRESH_S) {
        return Vec::new();
    }
    let text = |value: &serde_json::Value| value.as_str().unwrap_or_default().to_owned();
    file["budgets"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|budget| {
            budget["allowances"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(move |allowance| {
                    let of = allowance["allowance"].as_f64().filter(|&of| of > 0.0)?;
                    Some(Allowance {
                        budget: text(&budget["name"]),
                        label: text(&budget["label"]),
                        name: text(&allowance["name"]),
                        window: text(&allowance["label"]),
                        share: allowance["spent"].as_f64()? / of,
                        standing: text(&allowance["standing"]),
                        resets: allowance["resets"].as_str().map(str::to_owned),
                    })
                })
        })
        .collect()
}

/// Each allowance past one of [`LEVELS`], said once for the highest level
/// it has passed in its window, so a budget that climbs past both is told
/// twice and one that sits there is told once.
#[must_use]
pub fn standing(s: &Snapshot) -> Vec<Breach> {
    s.budgets
        .iter()
        .filter_map(|a| {
            // Floored, so 79.99% isn't said as past 80.
            let percent = (a.share * 100.0).floor();
            let level = LEVELS
                .iter()
                .rev()
                .find(|&&level| percent >= f64::from(level))?;
            Some(Breach {
                key: format!(
                    "budget:{}:{}:{level}:{}",
                    a.budget,
                    a.name,
                    a.resets.as_deref().unwrap_or_default()
                ),
                text: format!(
                    "{}'s {} allowance stands at {}, {percent:.0}% (told past {}% and {}%)",
                    a.label, a.window, a.standing, LEVELS[0], LEVELS[1]
                ),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const NOW: i64 = 1_795_000_000;

    fn budgets(at: i64, allowances: &[(&str, f64, f64, Option<&str>)]) -> String {
        let at = jiff::Timestamp::from_second(at).expect("a time");
        let allowances: Vec<_> = allowances
            .iter()
            .map(|(name, allowance, spent, resets)| {
                json!({
                    "name": name,
                    "label": name.to_uppercase(),
                    "unit": "dollars",
                    "allowance": allowance,
                    "spent": spent,
                    "resets": resets,
                    "standing": format!("${spent} of ${allowance}"),
                })
            })
            .collect();
        json!({
            "v": 1,
            "at": at.to_string(),
            "budgets": [
                {"name": "proxy", "label": "Your AI proxy token", "allowances": allowances},
                {"name": "plain", "label": "No allowance", "allowances": []},
            ],
        })
        .to_string()
    }

    fn said(allowances: &[(&str, f64, f64, Option<&str>)]) -> Vec<Breach> {
        let s = Snapshot {
            now: NOW,
            budgets: read(&budgets(NOW - 60, allowances), NOW),
            ..Snapshot::default()
        };
        standing(&s)
    }

    #[test]
    fn a_budget_past_eighty_and_ninety_percent_of_an_allowance_is_told_at_each() {
        // At about 13:00 on 20 November the default budget stood at 90%
        // of its week, $36,261 of $40,000, and nothing said so.
        assert_eq!(
            said(&[("weekly", 40_000.0, 36_261.0, None)]),
            [Breach {
                key: "budget:proxy:weekly:90:".to_owned(),
                text: "Your AI proxy token's WEEKLY allowance stands at $36261 of $40000, \
                       90% (told past 80% and 90%)"
                    .to_owned(),
            }],
            "a budget at 90% of its week wasn't told, or was told wrong"
        );
        assert_eq!(
            said(&[("weekly", 40_000.0, 32_000.0, None)])
                .iter()
                .map(|b| b.key.as_str())
                .collect::<Vec<_>>(),
            ["budget:proxy:weekly:80:"],
            "a budget at 80% wasn't told once, at its first level"
        );
        assert_eq!(
            said(&[("weekly", 40_000.0, 31_999.0, None)]),
            [],
            "a budget short of 80% was told"
        );
        assert_eq!(
            said(&[("day", 300.0, 290.0, Some("2026-11-21T05:00:00Z"))])
                .iter()
                .map(|b| b.key.as_str())
                .collect::<Vec<_>>(),
            ["budget:proxy:day:90:2026-11-21T05:00:00Z"],
            "an allowance that resets wasn't keyed by its window, so a later \
             day's crossing would go untold"
        );
        assert_eq!(
            said(&[("broken", 0.0, 10.0, None)]),
            [],
            "an allowance of nothing was read as passed"
        );
    }

    #[test]
    fn a_file_stale_or_unreadable_says_nothing_of_now() {
        let fresh = budgets(NOW - 60, &[("weekly", 100.0, 95.0, None)]);
        assert_eq!(read(&fresh, NOW).len(), 1, "a fresh file wasn't read");
        let stale = budgets(NOW - FRESH_S - 1, &[("weekly", 100.0, 95.0, None)]);
        assert_eq!(read(&stale, NOW), [], "a stopped daemon's file was read");
        assert_eq!(read("", NOW), [], "an empty file said something");
        assert_eq!(
            read("{\"v\":1}", NOW),
            [],
            "a file with no time said something"
        );
    }
}
