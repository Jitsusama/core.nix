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
        // A spend-watch whose every pass fails still reads as running, so a
        // cause on its side says where its reason is; one on cadence's
        // side, as a version it doesn't read, has nothing in that journal.
        let theirs = ["is missing", "was last written", "can't be read"]
            .iter()
            .any(|cause| why.starts_with(cause));
        let look = if theirs {
            " (journalctl --user -u spend-watch says why)"
        } else {
            ""
        };
        return vec![Breach {
            key: UNWATCHED.to_owned(),
            text: format!(
                "no budget's allowance is watched: spend-watch's budgets.json {why}{look}"
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

/// A key for each allowance read in `s`, at its lowest level, so what's
/// remembered of its window follows its reset on every read, not only
/// while a level stands. None while no budget is watched.
#[must_use]
pub fn windows(s: &Snapshot) -> Vec<String> {
    if s.budgets_unwatched.is_some() {
        return Vec::new();
    }
    s.budgets.iter().map(|a| key(a, LEVELS[0])).collect()
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
    let Some((allowance, level, resets)) = parts(key) else {
        return Vec::new();
    };
    LEVELS
        .iter()
        .filter(|&&beneath| beneath <= level)
        .map(|beneath| format!("budget:{allowance}{beneath}:{resets}"))
        .collect()
}

/// How far one read of an allowance's reset may stand from the last and
/// still name one window: a plan's feed works out each reset at its read,
/// so one window's drifts by seconds, while an allowance's windows are
/// hours apart.
const SAME_WINDOW_S: i64 = 5 * 60;
/// How far a reset may stand from a remembered whole hour and name its
/// window: keys written before resets were kept to the second named a
/// window by its reset rounded to the hour, so one standing at an upgrade
/// keeps its window told. A reset that rounded to the hour stood within
/// half an hour of it, and the next read within one drift of that reset,
/// still hours short of an allowance's next window.
const ROUNDED_TO_HOUR_S: i64 = 30 * 60 + SAME_WINDOW_S;

/// Where `key`'s window is remembered under resets read earlier, each key
/// of that window in `remembered`, under every reset that matches, beside
/// the one it is remembered as now, so a window told once isn't told again
/// as its reset drifts. Each read is matched to the last rather than the
/// first, so a reset that creeps one way over a window's life stays its
/// window while it is read; through a stretch nothing is read, as a
/// sleep, a creep past `SAME_WINDOW_S` is told again, which a plan's feed,
/// whose reset jitters by under a second, never does. No rounding keeps
/// every drift to one side of an edge, wherever the edge falls, so the
/// reset is matched instead. Empty where it is remembered as it reads, or
/// isn't remembered.
#[must_use]
pub fn moved<'a>(
    key: &str,
    remembered: impl IntoIterator<Item = &'a str>,
) -> Vec<(String, String)> {
    let Some((allowance, _, resets)) = parts(key) else {
        return Vec::new();
    };
    let Some(at) = instant(resets) else {
        return Vec::new();
    };
    let windows: Vec<(&str, u32, &str)> = remembered
        .into_iter()
        .filter_map(parts)
        .filter(|(other, _, _)| *other == allowance)
        .collect();
    if windows.iter().any(|(_, _, was)| *was == resets) {
        return Vec::new();
    }
    let same = |was: &&str| {
        instant(was).is_some_and(|was| {
            let apart = (was - at).abs();
            apart <= SAME_WINDOW_S || (was % 3600 == 0 && apart <= ROUNDED_TO_HOUR_S)
        })
    };
    windows
        .iter()
        .filter(|(_, _, was)| same(was))
        .map(|(_, level, was)| {
            (
                format!("budget:{allowance}{level}:{was}"),
                format!("budget:{allowance}{level}:{resets}"),
            )
        })
        .collect()
}

/// A budget's key read from the left, as a reset time holds colons of its
/// own: its budget and allowance, as `budget:allowance:`, its level and its
/// reset. None for another alarm's key.
fn parts(key: &str) -> Option<(&str, u32, &str)> {
    let rest = key.strip_prefix("budget:")?;
    let mut colons = rest.match_indices(':').map(|(at, _)| at);
    let (_, named) = (colons.next()?, colons.next()?);
    let (allowance, rest) = rest.split_at(named + 1);
    let (level, resets) = rest.split_once(':')?;
    Some((allowance, level.parse().ok()?, resets))
}

/// A reset as seconds since the epoch, when it's a time.
fn instant(resets: &str) -> Option<i64> {
    resets
        .parse::<jiff::Timestamp>()
        .ok()
        .map(jiff::Timestamp::as_second)
}

/// An allowance's key at a level. Its window is named by when it resets,
/// to the second, which `moved` matches to the window already told
/// as a plan's feed works it out afresh at each read. A colon in a
/// budget's or an allowance's name reads as a dot, as `stands_for` splits
/// the key on colons.
fn key(a: &Allowance, level: u32) -> String {
    let window = a.resets.as_deref().map_or_else(String::new, |resets| {
        resets
            .parse::<jiff::Timestamp>()
            .and_then(|at| at.round(jiff::Unit::Second))
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
    fn a_window_is_keyed_by_its_reset_and_matched_to_the_one_told() {
        assert_eq!(
            keys(&said(&[(
                "day",
                Some(0.95),
                Some("2026-11-21T04:59:58.6Z")
            )])),
            ["budget:proxy:day:90:2026-11-21T04:59:59Z"],
            "a reset wasn't keyed to its nearest second"
        );
        let told = [
            "budget:proxy:day:80:2026-11-21T04:59:59Z",
            "budget:proxy:day:90:2026-11-21T04:59:59Z",
            "budget:proxy:five:80:2026-11-21T16:29:59Z",
            "budget:proxy:edge:80:2026-11-21T10:00:30Z",
            // As keys were written before resets were kept to the second.
            "budget:proxy:hour:80:2026-11-21T17:00:00Z",
            // A window split across two hours by the old rounding.
            "budget:proxy:split:80:2026-11-21T16:00:00Z",
            "budget:proxy:split:90:2026-11-21T17:00:00Z",
            "budget:proxy:rim:90:2026-11-21T16:00:00Z",
            "budget:proxy:late:80:2026-11-21T17:00:00Z",
            "budget:proxy:torn:80:2026-11-21T16:00:00Z",
            "budget:proxy:torn:90:2026-11-21T17:00:00Z",
        ];
        let moves = |pairs: &[(&str, &str)]| -> Vec<(String, String)> {
            pairs
                .iter()
                .map(|(was, now)| ((*was).to_owned(), (*now).to_owned()))
                .collect()
        };
        let none = moves(&[]);
        for (key, moved_as) in [
            // A reset drifting across the hour or the half hour moves its
            // window's every level, a level past the one told included.
            (
                "budget:proxy:day:80:2026-11-21T05:00:01Z",
                moves(&[
                    (
                        "budget:proxy:day:80:2026-11-21T04:59:59Z",
                        "budget:proxy:day:80:2026-11-21T05:00:01Z",
                    ),
                    (
                        "budget:proxy:day:90:2026-11-21T04:59:59Z",
                        "budget:proxy:day:90:2026-11-21T05:00:01Z",
                    ),
                ]),
            ),
            (
                "budget:proxy:five:90:2026-11-21T16:30:01Z",
                moves(&[(
                    "budget:proxy:five:80:2026-11-21T16:29:59Z",
                    "budget:proxy:five:80:2026-11-21T16:30:01Z",
                )]),
            ),
            ("budget:proxy:day:80:2026-11-21T04:59:59Z", none.clone()),
            // Five minutes is one window, a second past it the next.
            (
                "budget:proxy:edge:80:2026-11-21T10:05:30Z",
                moves(&[(
                    "budget:proxy:edge:80:2026-11-21T10:00:30Z",
                    "budget:proxy:edge:80:2026-11-21T10:05:30Z",
                )]),
            ),
            ("budget:proxy:edge:80:2026-11-21T10:05:31Z", none.clone()),
            // A window remembered by its hour stays told within half an
            // hour and one drift of it, either side.
            (
                "budget:proxy:rim:90:2026-11-21T16:30:01Z",
                moves(&[(
                    "budget:proxy:rim:90:2026-11-21T16:00:00Z",
                    "budget:proxy:rim:90:2026-11-21T16:30:01Z",
                )]),
            ),
            (
                "budget:proxy:late:80:2026-11-21T16:29:59Z",
                moves(&[(
                    "budget:proxy:late:80:2026-11-21T17:00:00Z",
                    "budget:proxy:late:80:2026-11-21T16:29:59Z",
                )]),
            ),
            // A window split across two hours moves whole from a read a
            // second past the half hour.
            (
                "budget:proxy:torn:90:2026-11-21T16:30:01Z",
                moves(&[
                    (
                        "budget:proxy:torn:80:2026-11-21T16:00:00Z",
                        "budget:proxy:torn:80:2026-11-21T16:30:01Z",
                    ),
                    (
                        "budget:proxy:torn:90:2026-11-21T17:00:00Z",
                        "budget:proxy:torn:90:2026-11-21T16:30:01Z",
                    ),
                ]),
            ),
            (
                "budget:proxy:hour:90:2026-11-21T16:30:01Z",
                moves(&[(
                    "budget:proxy:hour:80:2026-11-21T17:00:00Z",
                    "budget:proxy:hour:80:2026-11-21T16:30:01Z",
                )]),
            ),
            ("budget:proxy:hour:80:2026-11-21T17:35:01Z", none.clone()),
            // Half an hour from each of its two hours exactly, every level
            // under each moves.
            (
                "budget:proxy:split:80:2026-11-21T16:30:00Z",
                moves(&[
                    (
                        "budget:proxy:split:80:2026-11-21T16:00:00Z",
                        "budget:proxy:split:80:2026-11-21T16:30:00Z",
                    ),
                    (
                        "budget:proxy:split:90:2026-11-21T17:00:00Z",
                        "budget:proxy:split:90:2026-11-21T16:30:00Z",
                    ),
                ]),
            ),
            ("budget:proxy:day:80:2026-11-22T05:00:00Z", none.clone()),
            ("budget:other:day:80:2026-11-21T05:00:00Z", none.clone()),
            ("budget:proxy:weekly:90:", none.clone()),
            (UNWATCHED, none.clone()),
        ] {
            assert_eq!(
                moved(key, told),
                moved_as,
                "{key} was matched to the wrong window: a drift would tell one again, or \
                 the next window or another budget's would go untold"
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
        let s = Snapshot {
            budgets_unwatched: Some("is of a version cadence doesn't read".to_owned()),
            ..Snapshot::default()
        };
        assert_eq!(
            standing(&s)[0].text,
            "no budget's allowance is watched: spend-watch's budgets.json \
             is of a version cadence doesn't read",
            "a cause on cadence's side sent the orchestrator to spend-watch's journal"
        );
    }
}
