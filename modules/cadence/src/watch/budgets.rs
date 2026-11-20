//! Each budget's standing against its allowances, as spend-watch writes it
//! to `budgets.json`. On 20 November the default budget stood at 90% of its
//! week by about 13:00 and nobody was told: spend-watch says so in pi's
//! footer and the launcher, which no one reads while the cells work alone.

use super::{Breach, Snapshot};

/// The shares of an allowance, in percent, past which the orchestrator is
/// told: once past the first and again past the second, as it asked on 20
/// November. They're its levels, not spend-watch's own `warn`, which is
/// where Joel's footer warns him.
pub const LEVELS: [u32; 2] = [80, 90];

/// How far from now spend-watch's file may have been written and still be
/// read, in seconds: its daemon rewrites it every few minutes, so an older
/// one is a stopped daemon's, whose shares say nothing of now, and one
/// from ahead of the clock is no more a sign of life.
pub const FRESH_S: i64 = 30 * 60;

/// The key of the alarm that no budget is watched.
const UNWATCHED: &str = "budget:unwatched";

/// One allowance of one budget, as spend-watch last stood it.
#[derive(Debug, Clone, PartialEq)]
pub struct Allowance {
    /// The budget's name, and its label as spend-watch shows it.
    pub budget: String,
    pub label: String,
    /// The allowance's name, and its label, such as `Week`.
    pub name: String,
    pub window: String,
    /// How near it is, as a share of one: spend-watch's own `nearest`, for
    /// an allowance in dollars and a plan's share alike.
    pub share: f64,
    /// spend-watch's own words for it, such as `$36,261 of $40,000`.
    pub standing: String,
    /// When its window resets, for one that resets at a set time, so each
    /// window's crossing is told once.
    pub resets: Option<String>,
}

/// Each allowance in spend-watch's `budgets.json`, as it stood at `now`,
/// or `None` for a file that can't be read or wasn't written within
/// [`FRESH_S`] of now, so no budget is watched. An allowance with no
/// `nearest`, one in dollars with nothing allowed, is left out.
#[must_use]
pub fn read(text: &str, now: i64) -> Option<Vec<Allowance>> {
    let file = serde_json::from_str::<serde_json::Value>(text).ok()?;
    let written = file["at"]
        .as_str()
        .and_then(|at| at.parse::<jiff::Timestamp>().ok())?
        .as_second();
    if (now - written).abs() > FRESH_S {
        return None;
    }
    let text = |value: &serde_json::Value| value.as_str().unwrap_or_default().to_owned();
    Some(
        file["budgets"]
            .as_array()?
            .iter()
            .flat_map(|budget| {
                budget["allowances"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(move |allowance| {
                        Some(Allowance {
                            budget: text(&budget["name"]),
                            label: text(&budget["label"]),
                            name: text(&allowance["name"]),
                            window: text(&allowance["label"]),
                            share: allowance["nearest"].as_f64()?,
                            standing: text(&allowance["standing"]),
                            resets: allowance["resets"].as_str().map(Into::into),
                        })
                    })
            })
            .collect(),
    )
}

/// Each allowance past one of [`LEVELS`], keyed by the highest it has
/// passed in its window, and that no budget is watched when spend-watch's
/// file couldn't be read. The asking side remembers a level while it
/// stands, with those beneath it, so each is told once.
#[must_use]
pub fn standing(s: &Snapshot) -> Vec<Breach> {
    if s.budgets_unwatched {
        return vec![Breach {
            key: UNWATCHED.to_owned(),
            text: "spend-watch's budgets weren't written in the last 30 min, so no \
                   budget's allowance is watched"
                .to_owned(),
        }];
    }
    s.budgets
        .iter()
        .filter_map(|a| {
            let level = *LEVELS
                .iter()
                .rev()
                .find(|&&level| a.share * 100.0 >= f64::from(level))?;
            let next = LEVELS.iter().find(|&&above| above > level).map_or_else(
                || "the last level it's told".to_owned(),
                |above| format!("told again past {above}%"),
            );
            Some(Breach {
                key: key(a, level),
                // Rounded, as spend-watch shows Joel the same share.
                text: format!(
                    "{}'s {} allowance stands at {}, {:.0}%: past {level}%, {next}",
                    a.label,
                    a.window,
                    a.standing,
                    (a.share * 100.0).round()
                ),
            })
        })
        .collect()
}

/// The keys an allowance's breach at `key` stands for: its own and each
/// level beneath it in the same window, so one told past 90% isn't told
/// as new past 80% as it eases. None for a key not an allowance's.
#[must_use]
pub fn stands_for(key: &str) -> Vec<String> {
    // Read from the left, as a reset time holds colons of its own.
    let mut parts = key
        .strip_prefix("budget:")
        .unwrap_or_default()
        .splitn(4, ':');
    let (Some(budget), Some(name), Some(level), Some(resets)) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Vec::new();
    };
    let Ok(level) = level.parse::<u32>() else {
        return Vec::new();
    };
    LEVELS
        .iter()
        .filter(|&&beneath| beneath <= level)
        .map(|beneath| format!("budget:{budget}:{name}:{beneath}:{resets}"))
        .collect()
}

fn key(a: &Allowance, level: u32) -> String {
    format!(
        "budget:{}:{}:{level}:{}",
        a.budget,
        a.name,
        a.resets.as_deref().unwrap_or_default()
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const NOW: i64 = 1_795_000_000;

    /// A file as spend-watch writes one: a dollar allowance carries its
    /// `allowance`, `spent` and `nearest`, and a plan's share only `used`
    /// and `nearest`.
    fn budgets(at: &str, allowances: &[(&str, Option<f64>, Option<&str>)]) -> String {
        let allowances: Vec<_> = allowances
            .iter()
            .map(|(name, nearest, resets)| {
                json!({
                    "name": name,
                    "label": name.to_uppercase(),
                    "unit": "share",
                    "used": nearest,
                    "nearest": nearest,
                    "resets": resets,
                    "standing": format!("{:.0}% of week", nearest.unwrap_or_default() * 100.0),
                })
            })
            .collect();
        json!({
            "v": 1,
            "at": at,
            "budgets": [
                {"name": "proxy", "label": "Your AI proxy token", "allowances": allowances},
                {"name": "plain", "label": "No allowance", "allowances": []},
            ],
        })
        .to_string()
    }

    fn at(seconds: i64) -> String {
        jiff::Timestamp::from_second(seconds)
            .expect("a time")
            .to_string()
    }

    fn said(allowances: &[(&str, Option<f64>, Option<&str>)]) -> Vec<Breach> {
        let s = Snapshot {
            now: NOW,
            budgets: read(&budgets(&at(NOW - 60), allowances), NOW).expect("a fresh file"),
            ..Snapshot::default()
        };
        standing(&s)
    }

    #[test]
    fn a_budget_past_eighty_and_ninety_percent_of_an_allowance_is_told_at_each() {
        // At about 13:00 on 20 November the default budget stood at 90%
        // of its week, $36,261 of $40,000, and nothing said so.
        assert_eq!(
            said(&[("weekly", Some(0.9065), None)]),
            [Breach {
                key: "budget:proxy:weekly:90:".to_owned(),
                text: "Your AI proxy token's WEEKLY allowance stands at 91% of week, 91%: \
                       past 90%, the last level it's told"
                    .to_owned(),
            }],
            "a budget past 90% of its week wasn't told, or was told wrong"
        );
        assert_eq!(
            said(&[("weekly", Some(0.82), None)]),
            [Breach {
                key: "budget:proxy:weekly:80:".to_owned(),
                text: "Your AI proxy token's WEEKLY allowance stands at 82% of week, 82%: \
                       past 80%, told again past 90%"
                    .to_owned(),
            }],
            "a budget past 80% wasn't told at its first level, or read as past both"
        );
        assert_eq!(
            said(&[("weekly", Some(0.7999), None)]),
            [],
            "a budget short of 80% was told"
        );
        assert_eq!(
            said(&[("day", Some(0.95), Some("2026-11-21T05:00:00Z"))])
                .iter()
                .map(|b| b.key.as_str())
                .collect::<Vec<_>>(),
            ["budget:proxy:day:90:2026-11-21T05:00:00Z"],
            "an allowance that resets wasn't keyed by its window, so a later \
             day's crossing would go untold"
        );
        assert_eq!(
            said(&[("spent", None, None)]),
            [],
            "an allowance with nothing allowed was read as passed"
        );
    }

    #[test]
    fn a_level_stands_for_those_beneath_it_in_its_window() {
        assert_eq!(
            stands_for("budget:proxy:weekly:90:"),
            ["budget:proxy:weekly:80:", "budget:proxy:weekly:90:"],
            "a level told past 90% would be told as new past 80% as it eased"
        );
        assert_eq!(
            stands_for("budget:proxy:day:90:2026-11-21T05:00:00Z"),
            [
                "budget:proxy:day:80:2026-11-21T05:00:00Z",
                "budget:proxy:day:90:2026-11-21T05:00:00Z"
            ],
            "a window's level wasn't kept to its window"
        );
        assert_eq!(
            stands_for(UNWATCHED),
            Vec::<String>::new(),
            "a key not an allowance's stood for one"
        );
        assert_eq!(
            stands_for("netheld:1:AAAAAA"),
            Vec::<String>::new(),
            "another alarm's key stood for a budget's"
        );
    }

    #[test]
    fn a_file_stale_ahead_or_unreadable_leaves_no_budget_watched() {
        let at_95 = |at: &str| budgets(at, &[("weekly", Some(0.95), None)]);
        assert_eq!(
            read(&at_95(&at(NOW - 60)), NOW).map(|all| all.len()),
            Some(1),
            "a fresh file wasn't read"
        );
        for (written, why) in [
            (at(NOW - FRESH_S - 1), "a stopped daemon's file was read"),
            (
                at(NOW + FRESH_S + 1),
                "a file from ahead of the clock was read",
            ),
            ("not a time".to_owned(), "a file with no time was read"),
        ] {
            assert_eq!(read(&at_95(&written), NOW), None, "{why}");
        }
        assert_eq!(read("", NOW), None, "an empty file was read");
        let s = Snapshot {
            budgets_unwatched: true,
            ..Snapshot::default()
        };
        assert_eq!(
            standing(&s)
                .iter()
                .map(|b| b.key.as_str())
                .collect::<Vec<_>>(),
            [UNWATCHED],
            "no budget watched read the same as every budget being fine"
        );
    }
}
