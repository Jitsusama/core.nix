//! What every source said the moment an allowance went over, kept once
//! each time one does. Over is when a provider may turn pi away, and
//! whether each source still reads then is what the watch is for, so the
//! first pass that finds an allowance over writes down how it stands,
//! how every budget stands, each source's health, what each feed last
//! billed, pi's logs and what was found. It's kept in the history and
//! written beside the weekly reports, and while the allowance is over its
//! row in the collection opens it.

use crate::findings::Finding;
use crate::history::{History, sql as fail};
use crate::picture::Picture;
use crate::report::Format;
use crate::standing::{Allowance, Budget, Standing};
use crate::time::{DAY, Zone};
use rusqlite::params;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// An allowance over: whose, which, since when, and where what every
/// source said is written.
#[derive(Debug, Clone, PartialEq)]
pub struct Crossing {
    pub budget: String,
    pub allowance: String,
    pub at: f64,
    pub path: PathBuf,
}

/// How soon after coming back under an allowance going over again is
/// still the same crossing, since a reading can flicker for a pass as a
/// change of allowance can.
const FLICKER: f64 = crate::caps::HELD;

/// Every crossing still over, the oldest first.
pub fn open(history: &History) -> Result<Vec<Crossing>, String> {
    let mut statement = history
        .db
        .prepare(
            "SELECT budget, allowance, at, path FROM crossings WHERE back IS NULL
             ORDER BY at, budget, allowance",
        )
        .map_err(fail)?;
    statement
        .query_map([], |row| {
            Ok(Crossing {
                budget: row.get(0)?,
                allowance: row.get(1)?,
                at: row.get(2)?,
                path: PathBuf::from(row.get::<_, String>(3)?),
            })
        })
        .map_err(fail)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(fail)
}

/// Keeps what every source says as an allowance goes over, once each time
/// it does, into the history and a report in `reports`; marks each
/// crossing that's back under; and answers those over now.
pub fn keep(
    history: &History,
    picture: &Picture,
    standing: &Standing,
    findings: &[Finding],
    reports: &Path,
) -> Result<Vec<Crossing>, String> {
    let now = picture.now;
    let over: Vec<(&Budget, &Allowance)> = standing
        .budgets
        .iter()
        .flat_map(|budget| {
            budget
                .allowances
                .iter()
                .filter(|allowance| crate::standing::over(allowance))
                .map(move |allowance| (budget, allowance))
        })
        .collect();
    let is = |crossing: &Crossing, (budget, allowance): &(&Budget, &Allowance)| {
        crossing.budget == budget.name && crossing.allowance == allowance.name
    };
    for crossing in open(history)? {
        if !over.iter().any(|pair| is(&crossing, pair)) {
            history
                .db
                .execute(
                    "UPDATE crossings SET back = ?1
                     WHERE budget = ?2 AND allowance = ?3 AND at = ?4",
                    params![now, crossing.budget, crossing.allowance, crossing.at],
                )
                .map_err(fail)?;
        }
    }
    let still = open(history)?;
    for pair in &over {
        let (budget, allowance) = *pair;
        if still.iter().any(|crossing| is(crossing, pair)) || again(history, pair, now)? {
            continue;
        }
        let said = said(history, picture, standing, findings, pair)?;
        let path = reports.join(file_name(&picture.zone, budget, allowance, now));
        // Kept in the history before the report is written, so what was
        // said stays even when the report can't be.
        history
            .db
            .execute(
                "INSERT INTO crossings VALUES (?1, ?2, ?3, NULL, ?4, ?5)",
                params![
                    budget.name,
                    allowance.name,
                    now,
                    path.to_string_lossy(),
                    said
                ],
            )
            .map_err(fail)?;
        crate::status::write_atomic(&path, &said)?;
    }
    open(history)
}

/// Takes up again, as the same crossing, one that came back under within
/// `FLICKER`; answers whether there was one.
fn again(
    history: &History,
    (budget, allowance): &(&Budget, &Allowance),
    now: f64,
) -> Result<bool, String> {
    let taken = history
        .db
        .execute(
            "UPDATE crossings SET back = NULL
             WHERE budget = ?1 AND allowance = ?2 AND back >= ?3",
            params![budget.name, allowance.name, now - FLICKER],
        )
        .map_err(fail)?;
    Ok(taken > 0)
}

/// The report's name: the budget, the allowance and the local minute it
/// went over, so the crossings of one allowance sort by when.
fn file_name(zone: &Zone, budget: &Budget, allowance: &Allowance, at: f64) -> String {
    let plain = |name: &str| -> String {
        name.chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect()
    };
    format!(
        "over-{}-{}-{}-{}.md",
        plain(&budget.name),
        plain(&allowance.name),
        Zone::iso_day(zone.day(at)),
        zone.clock(at, at).replace(':', "")
    )
}

/// Everything the record says, as Markdown.
fn said(
    history: &History,
    picture: &Picture,
    standing: &Standing,
    findings: &[Finding],
    (budget, allowance): &(&Budget, &Allowance),
) -> Result<String, String> {
    let zone = &picture.zone;
    let now = picture.now;
    let label = crate::status::capitalized(&allowance.label);
    let mut out = format!("# Over the {label} Allowance\n\n");
    _ = write!(
        out,
        "{} went over its {label} allowance at {} on {}: {}. This is what each of \
         spend-watch's sources said as it did, kept so it's known whether each \
         still reads while a budget is over.\n\n",
        budget.label,
        zone.clock(now, now),
        Zone::day_name(zone.day(now)),
        allowance.standing,
    );
    out.push_str(&budgets(standing));
    out.push_str(&sources(picture));
    out.push_str(&billed(history, picture)?);
    out.push_str(&logs(picture));
    out.push('\n');
    out.push_str(&crate::report::render(
        &crate::report::summary(picture, findings),
        Format::Markdown,
    ));
    out.push('\n');
    out.push_str(&crate::report::render(
        &crate::report::caps(picture),
        Format::Markdown,
    ));
    Ok(out)
}

/// Every budget's allowances as they were read.
fn budgets(standing: &Standing) -> String {
    let mut out = String::from("## Every Budget\n\n");
    for budget in &standing.budgets {
        _ = writeln!(out, "- {}: {}", budget.label, budget.standing);
        for allowance in &budget.allowances {
            let read = match (&allowance.from, &allowance.at) {
                (Some(from), Some(at)) => format!(", read from {from} at {at}"),
                (None, Some(at)) => format!(", read at {at}"),
                _ => String::new(),
            };
            let state = allowance
                .state
                .as_deref()
                .map_or_else(String::new, |state| format!(", {state}"));
            _ = writeln!(
                out,
                "  - {}: {}{state}{read}",
                crate::status::capitalized(&allowance.label),
                allowance.standing
            );
        }
    }
    out.push('\n');
    out
}

/// Each feed's sources and how each stood.
fn sources(picture: &Picture) -> String {
    let mut out = String::from("## Sources\n\n");
    if picture.sources.is_empty() {
        out.push_str("No feed has said how its sources stand.\n");
    }
    for source in &picture.sources {
        let message = if source.message.is_empty() {
            String::new()
        } else {
            format!(": {}", source.message)
        };
        _ = writeln!(
            out,
            "- {} {}: {} since {}, last said at {}{message}",
            source.feed,
            source.name,
            source.state,
            picture.zone.clock(source.since, picture.now),
            picture.zone.clock(source.at, picture.now),
        );
    }
    out.push('\n');
    out
}

/// The last hour each feed's source billed, and when it last billed
/// anything, so a source that stopped while over shows it.
fn billed(history: &History, picture: &Picture) -> Result<String, String> {
    let mut statement = history
        .db
        .prepare(
            "SELECT feed, source, MAX(hour), MAX(at) FROM billed
             GROUP BY feed, source ORDER BY feed, source",
        )
        .map_err(fail)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, f64>(3)?,
            ))
        })
        .map_err(fail)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(fail)?;
    let mut out = String::from("## What Each Feed Billed\n\n");
    if rows.is_empty() {
        out.push_str("No feed has billed anything.\n");
    }
    let clock = |at: f64| picture.zone.clock(at, picture.now);
    for (feed, source, hour, at) in rows {
        _ = writeln!(
            out,
            "- {feed} {source}: up to the hour from {}, last told at {}",
            clock(crate::time::hour_start(hour)),
            clock(at),
        );
    }
    out.push('\n');
    Ok(out)
}

/// What pi's logs said: when they last grew, and what the provider
/// refused in the last day.
fn logs(picture: &Picture) -> String {
    let now = picture.now;
    let mut out = String::from("## pi's Logs\n\n");
    match picture.logs_at {
        Some(at) => _ = writeln!(out, "- They last grew at {}.", picture.zone.clock(at, now)),
        None => out.push_str("- They hold no turn yet.\n"),
    }
    let refused: Vec<&(f64, String)> = picture
        .refusals
        .iter()
        .filter(|(at, _)| now - at < DAY)
        .collect();
    match refused.last() {
        Some((at, said)) => {
            _ = writeln!(
                out,
                "- {} of pi's requests were refused in the last day, the last at {}: {said}",
                refused.len(),
                picture.zone.clock(*at, now),
            );
        }
        None => out.push_str("- None of pi's requests was refused in the last day.\n"),
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    // 2026-10-14T14:30:00Z.
    const NOW: f64 = 1_791_988_200.0;

    /// A machine with a Claude plan whose five hours a feed reads, and a
    /// key of its own, so the budgets have a row.
    fn config() -> Config {
        serde_json::from_value(serde_json::json!({
            "sessions": "/none", "runs": "/none", "registry": "/none", "quests": "/none",
            "state": "/none", "zone": "UTC",
            "budgets": [
                {"name": "claude", "label": "Claude Pro", "kind": "plan", "whose": "own",
                 "default": true, "price": {"dollars": 20, "per": "month"},
                 "allowances": [{"name": "five-hours", "unit": "share", "windowHours": 5,
                                  "warn": 0.8}]},
                {"name": "key", "label": "My key", "kind": "key", "whose": "own"},
            ],
        }))
        .unwrap()
    }

    /// How things stand once the feed has read the five hours `used`.
    fn read(history: &History, used: f64, at: f64) -> (Picture, Standing) {
        let line = serde_json::json!({
            "v": 1, "kind": "cap", "name": "five-hours", "label": "Five Hours",
            "unit": "share", "budget": "claude", "used": used, "windowHours": 5,
            "warn": 0.8, "state": "ok", "resets": "2026-10-14T18:00:00Z",
            "at": crate::time::rfc3339(at),
        });
        let record = crate::feed::parse(&line.to_string()).unwrap();
        crate::feed::store(history, "claude", &record, at).unwrap();
        let picture = Picture::read(history, &config(), at).unwrap();
        let standing = crate::standing::standing(&picture, &config(), None);
        (picture, standing)
    }

    /// One pass's keeping, answering when each crossing over now began.
    fn pass(history: &History, reports: &Path, used: f64, at: f64) -> Vec<f64> {
        let (picture, standing) = read(history, used, at);
        keep(history, &picture, &standing, &[], reports)
            .unwrap()
            .iter()
            .map(|crossing| crossing.at)
            .collect()
    }

    fn written(reports: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(reports)
            .map(|entries| {
                entries
                    .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                    .filter(|name| !name.starts_with('.'))
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    }

    // Each time an allowance goes over is kept once, however many passes
    // find it over; a reading that flickers back under and over again is
    // the same time; and going over again once it has stood back under
    // is another.
    #[test]
    fn an_allowance_going_over_is_kept_once_each_time() {
        let history = History::memory();
        let dir = tempfile::tempdir().unwrap();
        let reports = dir.path();
        assert_eq!(pass(&history, reports, 0.5, NOW), Vec::<f64>::new());
        assert_eq!(written(reports), Vec::<String>::new());
        let over = NOW + 60.0;
        assert_eq!(pass(&history, reports, 1.0, over), [over]);
        assert_eq!(pass(&history, reports, 1.0, NOW + 120.0), [over]);
        assert_eq!(
            written(reports),
            ["over-claude-five-hours-2026-10-14-1431.md"]
        );
        assert_eq!(pass(&history, reports, 0.4, NOW + 180.0), Vec::<f64>::new());
        assert_eq!(pass(&history, reports, 1.0, NOW + 240.0), [over]);
        assert_eq!(written(reports).len(), 1);
        let back = NOW + 300.0;
        assert_eq!(pass(&history, reports, 0.4, back), Vec::<f64>::new());
        let again = back + FLICKER + 60.0;
        assert_eq!(pass(&history, reports, 1.0, again), [again]);
        assert_eq!(
            written(reports),
            [
                "over-claude-five-hours-2026-10-14-1431.md",
                "over-claude-five-hours-2026-10-14-1446.md",
            ]
        );
        let backs: Vec<Option<f64>> = history
            .db
            .prepare("SELECT back FROM crossings ORDER BY at")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(backs, [Some(back), None]);
    }

    // What's kept says how the allowance and every budget stood, each
    // source's health, what each feed billed and what pi's logs said, and
    // the history holds it as the report says it.
    #[test]
    fn what_is_kept_says_what_every_source_said() {
        let history = History::memory();
        let dir = tempfile::tempdir().unwrap();
        pass(&history, dir.path(), 1.0, NOW);
        let report = dir.path().join("over-claude-five-hours-2026-10-14-1430.md");
        let text = std::fs::read_to_string(report).unwrap();
        assert!(
            text.starts_with(
                "# Over the Five Hours Allowance\n\nClaude Pro went over its Five Hours \
                 allowance at 14:30 on Wed 14 Oct: 100% of 5 h."
            ),
            "{text}"
        );
        for heading in [
            "## Every Budget",
            "## Sources",
            "## What Each Feed Billed",
            "## pi's Logs",
            "## AI spend",
            "## Caps",
        ] {
            assert!(text.contains(heading), "no {heading} in {text}");
        }
        assert!(text.contains("  - Five Hours: 100% of 5 h"), "{text}");
        let kept: String = history
            .db
            .query_row("SELECT said FROM crossings", [], |row| row.get(0))
            .unwrap();
        assert_eq!(kept, text);
    }

    // While it's over, Return on the budget's row opens what was kept,
    // and once it's back under the row copies as it did.
    #[test]
    fn while_over_the_budget_s_row_opens_what_was_kept() {
        let history = History::memory();
        let dir = tempfile::tempdir().unwrap();
        let row = |used: f64, at: f64| {
            let (mut picture, standing) = read(&history, used, at);
            picture.crossings = keep(&history, &picture, &standing, &[], dir.path()).unwrap();
            let collected = crate::collection::collection(&picture, &[], &config());
            let mut rows = collected.now.iter().flat_map(|line| line.rows.iter());
            let found = rows.find(|row| row.key == "budget:claude").cloned();
            found.unwrap()
        };
        let over = row(1.0, NOW);
        let path = dir.path().join("over-claude-five-hours-2026-10-14-1430.md");
        assert_eq!(
            over.run,
            Some(vec!["xdg-open".into(), path.to_string_lossy().into_owned()])
        );
        assert_eq!(over.verb, "open");
        let under = row(0.4, NOW + 60.0);
        assert_eq!(under.run, None);
        assert_eq!(under.verb, "");
    }
}
