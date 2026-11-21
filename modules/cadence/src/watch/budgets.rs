//! Each budget's standing against its allowances, as spend-watch writes it
//! to `budgets.json`. On 20 November the default budget stood at 90% of its
//! week by about 13:00 and nobody was told: spend-watch says so in pi's
//! footer and the launcher, which no one reads while the cells work alone.

use std::path::Path;

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

/// The version of spend-watch's standing cadence reads, its `v`.
const VERSION: u32 = 1;

/// The key of the alarm that no budget is watched.
pub const UNWATCHED: &str = "budget:unwatched";

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

/// Each allowance spend-watch's daemon keeps in its runtime folder, or
/// why none is watched, in words that follow "budgets.json".
///
/// # Errors
///
/// When there's no runtime folder, or the file is missing, as while
/// spend-watch's watch is held and it deletes the file, or can't be read
/// as [`read`] says.
pub fn gather(runtime: Option<&Path>, now: i64) -> Result<Vec<Allowance>, String> {
    let runtime = runtime.ok_or("has no runtime folder to be read from")?;
    match std::fs::read_to_string(runtime.join("spend-watch/budgets.json")) {
        Ok(text) => read(&text, now),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Err("is missing, as while spend-watch's watch is held or it isn't running".into())
        }
        Err(error) => Err(format!("can't be read: {error}")),
    }
}

/// Each allowance in spend-watch's `budgets.json`, as it stood at `now`.
/// An allowance with no `nearest`, one in dollars with nothing allowed, is
/// left out.
///
/// # Errors
///
/// For a file that isn't spend-watch's standing, or wasn't written within
/// [`FRESH_S`] of now, saying which.
pub fn read(text: &str, now: i64) -> Result<Vec<Allowance>, String> {
    let file = serde_json::from_str::<serde_json::Value>(text)
        .map_err(|_| "isn't spend-watch's standing".to_owned())?;
    // A version marks a change in what a field means, which no pin on its
    // name sees, so one cadence wasn't written for is told as unwatched
    // rather than read as false levels.
    if file["v"] != VERSION {
        return Err("is of a version cadence doesn't read".to_owned());
    }
    let written = file["at"]
        .as_str()
        .and_then(|at| at.parse::<jiff::Timestamp>().ok())
        .ok_or("says no time it was written")?
        .as_second();
    if now - written > FRESH_S {
        return Err(format!("was last written {} min ago", (now - written) / 60));
    }
    if written - now > FRESH_S {
        return Err("is dated ahead of the clock".to_owned());
    }
    let text = |value: &serde_json::Value| value.as_str().unwrap_or_default().to_owned();
    Ok(file["budgets"]
        .as_array()
        .ok_or("holds no budgets")?
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
        .collect())
}

/// Each allowance past one of [`LEVELS`], keyed by the highest it has
/// passed in its window, and that no budget is watched when spend-watch's
/// file couldn't be read. The asking side remembers a level while it
/// stands, with those beneath it, so each is told once.
#[must_use]
pub fn standing(s: &Snapshot) -> Vec<Breach> {
    if let Some(why) = &s.budgets_unwatched {
        return vec![Breach {
            key: UNWATCHED.to_owned(),
            // A spend-watch whose every pass fails still reads as running, so
            // the alarm says where its reason is.
            text: format!(
                "no budget's allowance is watched: spend-watch's budgets.json {why} \
                 (journalctl --user -u spend-watch says why)"
            ),
        }];
    }
    s.budgets
        .iter()
        .filter_map(|a| {
            // Judged on the percent shown, rounded as spend-watch shows
            // Joel the same share, so the text never names a level it
            // says isn't passed.
            let percent = (a.share * 100.0).round();
            let level = *LEVELS
                .iter()
                .rev()
                .find(|&&level| percent >= f64::from(level))?;
            // A plan's share stands as its percent already; a dollar
            // allowance's doesn't.
            let shown = if a.standing.contains('%') {
                a.standing.clone()
            } else {
                format!("{}, {percent:.0}%", a.standing)
            };
            let next = LEVELS.iter().find(|&&above| above > level).map_or_else(
                || "the last level it's told".to_owned(),
                |above| format!("told again past {above}%"),
            );
            Some(Breach {
                key: key(a, level),
                text: format!(
                    "{}'s {} allowance stands at {shown}: past {level}%, {next}",
                    a.label, a.window
                ),
            })
        })
        .collect()
}

/// The keys a breach at `key` stands for: an allowance's own and each
/// level beneath it in the same window, so one told past 90% isn't told
/// as new past 80% as it eases, and the unwatched alarm's own, so it's
/// told once however long it stands. None for another alarm's key.
#[must_use]
pub fn stands_for(key: &str) -> Vec<String> {
    if key == UNWATCHED {
        return vec![UNWATCHED.to_owned()];
    }
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

/// An allowance's key at a level. Its window is named by the hour it
/// resets in, as a plan's feed reports a reset to the second, worked out
/// at each read, and a second's drift would read as a new window. A colon
/// in a budget's or an allowance's name reads as a dot, as `stands_for`
/// splits the key on colons.
fn key(a: &Allowance, level: u32) -> String {
    let window = a.resets.as_deref().map_or_else(String::new, |resets| {
        resets
            .parse::<jiff::Timestamp>()
            .and_then(|at| at.round(jiff::Unit::Hour))
            .map_or_else(|_| resets.to_owned(), |at| at.to_string())
    });
    let bare = |name: &str| name.replace(':', ".");
    format!(
        "budget:{}:{}:{level}:{window}",
        bare(&a.budget),
        bare(&a.name)
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const NOW: i64 = 1_795_000_000;

    type Shown<'a> = (&'a str, Option<f64>, Option<&'a str>);

    /// A file as spend-watch writes one: a dollar allowance carries its
    /// `allowance`, `spent` and `nearest`, and a plan's share only `used`
    /// and `nearest`.
    fn budgets(at: &str, allowances: &[Shown]) -> String {
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

    fn said(allowances: &[Shown]) -> Vec<Breach> {
        let s = Snapshot {
            now: NOW,
            budgets: read(&budgets(&at(NOW - 60), allowances), NOW).expect("a fresh file"),
            ..Snapshot::default()
        };
        standing(&s)
    }

    fn keys(breaches: &[Breach]) -> Vec<&str> {
        breaches.iter().map(|b| b.key.as_str()).collect()
    }

    #[test]
    fn a_budget_past_eighty_and_ninety_percent_of_an_allowance_is_told_at_each() {
        // At about 13:00 on 20 November the default budget stood at 90%
        // of its week, $36,261 of $40,000, and nothing said so.
        assert_eq!(
            said(&[("weekly", Some(0.9065), None)]),
            [Breach {
                key: "budget:proxy:weekly:90:".to_owned(),
                text: "Your AI proxy token's WEEKLY allowance stands at 91% of week: \
                       past 90%, the last level it's told"
                    .to_owned(),
            }],
            "a budget past 90% of its week wasn't told, or was told wrong"
        );
        assert_eq!(
            said(&[("weekly", Some(0.82), None)]),
            [Breach {
                key: "budget:proxy:weekly:80:".to_owned(),
                text: "Your AI proxy token's WEEKLY allowance stands at 82% of week: \
                       past 80%, told again past 90%"
                    .to_owned(),
            }],
            "a budget past 80% wasn't told at its first level, or read as past both"
        );
        // Joel's footer shows 0.897 as 90%, so the orchestrator is told it
        // passed 90% rather than "90%: past 80%".
        assert_eq!(
            keys(&said(&[("weekly", Some(0.897), None)])),
            ["budget:proxy:weekly:90:"],
            "a share shown as 90% was judged short of it"
        );
        assert_eq!(
            said(&[("weekly", Some(0.7949), None)]),
            [],
            "a budget short of 80% was told"
        );
        assert_eq!(
            said(&[("spent", None, None)]),
            [],
            "an allowance with nothing allowed was read as passed"
        );
    }

    #[test]
    fn a_window_is_keyed_by_the_hour_it_resets_in() {
        for resets in [
            "2026-11-21T05:00:00Z",
            "2026-11-21T04:59:59.902Z",
            "2026-11-21T05:00:01Z",
        ] {
            assert_eq!(
                keys(&said(&[("day", Some(0.95), Some(resets))])),
                ["budget:proxy:day:90:2026-11-21T05:00:00Z"],
                "a reset at {resets} wasn't keyed by its hour, so a second's drift \
                 would tell a window again"
            );
        }
        assert_eq!(
            keys(&said(&[("day", Some(0.95), Some("not a time"))])),
            ["budget:proxy:day:90:not a time"],
            "a reset that isn't a time lost its window"
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
            [UNWATCHED],
            "no budget watched wasn't remembered while it stood"
        );
        assert_eq!(
            stands_for("netheld:1:AAAAAA"),
            Vec::<String>::new(),
            "another alarm's key stood for a budget's"
        );
        let coloned = Allowance {
            budget: "team:one".into(),
            label: "Team One".into(),
            name: "plan:week".into(),
            window: "Week".into(),
            share: 0.95,
            standing: String::new(),
            resets: None,
        };
        assert_eq!(
            stands_for(&key(&coloned, 90)),
            [
                "budget:team.one:plan.week:80:",
                "budget:team.one:plan.week:90:"
            ],
            "a name holding a colon left its level standing for nothing"
        );
    }

    #[test]
    fn a_file_missing_stale_ahead_or_unreadable_says_why_no_budget_is_watched() {
        let at_95 = |at: &str| budgets(at, &[("weekly", Some(0.95), None)]);
        assert_eq!(
            read(&at_95(&at(NOW - 60)), NOW).map(|all| all.len()),
            Ok(1),
            "a fresh file wasn't read"
        );
        for (file, why) in [
            (
                at_95(&at(NOW - FRESH_S - 60)),
                "was last written 31 min ago",
            ),
            (at_95(&at(NOW + FRESH_S + 1)), "is dated ahead of the clock"),
            (at_95("not a time"), "says no time it was written"),
            (String::new(), "isn't spend-watch's standing"),
            (
                json!({"v": 1, "at": at(NOW)}).to_string(),
                "holds no budgets",
            ),
            (
                json!({"v": 2, "at": at(NOW), "budgets": []}).to_string(),
                "is of a version cadence doesn't read",
            ),
        ] {
            assert_eq!(read(&file, NOW), Err(why.to_owned()), "{file}");
        }
        let runtime = std::env::temp_dir().join(format!("cadence-budgets-{}", std::process::id()));
        assert_eq!(
            gather(Some(&runtime), NOW),
            Err("is missing, as while spend-watch's watch is held or it isn't running".to_owned()),
            "a file spend-watch deleted wasn't told as missing"
        );
        assert_eq!(
            gather(None, NOW),
            Err("has no runtime folder to be read from".to_owned()),
            "no runtime folder wasn't told"
        );
        let s = Snapshot {
            budgets_unwatched: Some("is missing".to_owned()),
            ..Snapshot::default()
        };
        assert_eq!(
            standing(&s),
            [Breach {
                key: UNWATCHED.to_owned(),
                text: "no budget's allowance is watched: spend-watch's budgets.json is missing \
                       (journalctl --user -u spend-watch says why)"
                    .to_owned(),
            }],
            "no budget watched read the same as every budget being fine"
        );
    }
}
