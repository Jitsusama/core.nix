//! The account's spend changing: a new regime in its daily spend, something
//! new billed under its name, a day on course past any of eight weeks', and
//! a model the bill now prices apart from the month before.

use super::{NEW_NOTE, digest, elsewhere_note, finding};
use crate::config::Config;
use crate::findings::{Finding, Severity};
use crate::history::{History, sql as fail};
use crate::money::{dollars, percent, whole};
use crate::picture::Picture;
use crate::stats;
use crate::time::{DAY, Zone};
use augurs_changepoint::{Detector, NormalGammaDetector};

/// A change of regime in daily spend: Bayesian online changepoints on the
/// standardized log of each day, a change counted once a week has passed
/// after it and the days since differ from the days before by a quarter.
pub fn regimes(picture: &Picture) -> Vec<Finding> {
    let daily = &picture.days.dollars;
    let n = daily.len().saturating_sub(1); // today is still going
    if n < 56 {
        return Vec::new();
    }
    let logs: Vec<f64> = daily[..n].iter().map(|d| d.ln_1p()).collect();
    let centre = stats::mean(&logs);
    let spread = (logs.iter().map(|x| (x - centre).powi(2)).sum::<f64>() / stats::count(n))
        .sqrt()
        .max(1e-6);
    let z: Vec<f64> = logs.iter().map(|x| (x - centre) / spread).collect();
    let changes = NormalGammaDetector::default().detect_changepoints(&z);
    let Some(&at) = changes.iter().rev().find(|&&at| at + 7 <= n && at > 0) else {
        return Vec::new();
    };
    if n - at > 21 {
        return Vec::new();
    }
    let before_from = changes
        .iter()
        .rev()
        .find(|&&c| c < at)
        .copied()
        .unwrap_or(0);
    let usual = |days: &[f64]| {
        stats::median(
            &days
                .iter()
                .copied()
                .filter(|d| *d >= 5.0)
                .collect::<Vec<_>>(),
        )
    };
    let (before, after) = (usual(&daily[before_from..at]), usual(&daily[at..n]));
    if !before.is_finite() || !after.is_finite() || (after / before - 1.0).abs() < 0.25 {
        return Vec::new();
    }
    let day = picture.days.first + stats::signed(at);
    let mut f = finding(
        format!("regime:{day}"),
        "regimes",
        Severity::Note,
        format!(
            "a usual day is {} since {}",
            if after > before { "up" } else { "down" },
            Zone::day_name(day)
        ),
        format!(
            "{} from {}; found by online changepoints on daily spend.",
            if after > before { "up" } else { "down" },
            whole(before)
        ),
    );
    f.badge = whole(after);
    f.note = format!("from {} before", whole(before));
    f.dollars = (after - before).abs() * 7.0;
    f.surprise = 2.0;
    vec![f]
}

/// Something new spending under the account's name: a tool, model or
/// credential billed in the last day that wasn't in the thirty before. A
/// new model or tool is one someone chose, so it's listed; a credential
/// nobody knew may be a leaked key, so it's told.
pub fn novelty(
    picture: &Picture,
    history: &History,
    config: &Config,
) -> Result<Vec<Finding>, String> {
    let now = picture.now;
    let recent_from = crate::time::hour_of(now - DAY);
    let before_from = crate::time::hour_of(now - 31.0 * DAY);
    let mut out = Vec::new();
    for (column, what) in [
        ("tool", "tool"),
        ("model", "model"),
        ("token", "credential"),
    ] {
        let query = format!(
            "SELECT {column}, SUM(dollars) FROM billed
             WHERE hour >= ?1 AND hour <= ?3 AND {column} != '' AND {column} != 'all'
             AND {column} NOT IN (SELECT {column} FROM billed WHERE hour >= ?2 AND hour < ?1)
             GROUP BY 1 HAVING SUM(dollars) >= 1"
        );
        let seen_before: bool = history
            .db
            .query_row(
                &format!("SELECT EXISTS (SELECT 1 FROM billed WHERE hour < ?1 AND {column} != '')"),
                [recent_from],
                |row| row.get(0),
            )
            .map_err(fail)?;
        if !seen_before {
            continue;
        }
        let mut statement = history.db.prepare(&query).map_err(fail)?;
        let rows = statement
            .query_map(
                [recent_from, before_from, crate::time::hour_of(now)],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?)),
            )
            .map_err(fail)?;
        for row in rows {
            let (name, spent) = row.map_err(fail)?;
            let shown = if what == "credential" {
                format!("{}...", name.chars().take(8).collect::<String>())
            } else {
                name.clone()
            };
            // Something new that's spent little isn't worth a look: a
            // model tried once for a few dollars.
            if spent < config.detectors.floor {
                continue;
            }
            // A credential's key is its digest, never the credential.
            let named = if what == "credential" {
                digest(&name)
            } else {
                name.clone()
            };
            let mut f = finding(
                format!("new:{column}:{named}"),
                "novelty",
                if what == "credential" {
                    Severity::Warning
                } else {
                    Severity::Note
                },
                format!("a new {what}: {shown}"),
                format!(
                    "{} in the last day; not billed in the thirty days before. Expected, or worth \
                     a look?",
                    dollars(spent)
                ),
            );
            f.badge = whole(spent);
            f.note = NEW_NOTE.into();
            f.dollars = spent;
            f.surprise = 2.0;
            out.push(f);
        }
    }
    out.extend(elsewhere(picture));
    Ok(out)
}

/// Spend billed for sessions no log here knows, past its usual share.
fn elsewhere(picture: &Picture) -> Option<Finding> {
    let share = |pieces: &[crate::series::Piece]| {
        let pi: f64 = pieces
            .iter()
            .filter(|p| p.tool == "pi")
            .map(|p| p.dollars)
            .sum();
        let elsewhere: f64 = pieces
            .iter()
            .filter(|p| p.session == "elsewhere")
            .map(|p| p.dollars)
            .sum();
        (elsewhere, if pi > 0.0 { elsewhere / pi } else { 0.0 })
    };
    let (week, week_share) = share(picture.pieces_week());
    let (_, month_share) = share(picture.pieces_month());
    let past_usual = week >= 50.0 && week_share > (2.0 * month_share).max(0.1);
    if !past_usual {
        return None;
    }
    let mut f = finding(
        "new:elsewhere".into(),
        "novelty",
        Severity::Warning,
        "pi spending somewhere this machine doesn't know".into(),
        format!(
            "{} in the last 7 days from sessions with no log here, {} of pi's spend against \
             {} usual.",
            dollars(week),
            percent(week_share),
            percent(month_share)
        ),
    );
    f.badge = whole(week);
    f.note = elsewhere_note(week_share, month_share);
    f.dollars = week;
    f.surprise = 2.0;
    Some(f)
}

/// On course for a day far past any of the last eight weeks'.
pub fn spikes(picture: &Picture) -> Vec<Finding> {
    let Some((_, median, _)) = picture.today_ahead() else {
        return Vec::new();
    };
    let recent: Vec<f64> = picture
        .days
        .dollars
        .iter()
        .rev()
        .skip(1)
        .take(56)
        .copied()
        .collect();
    if recent.len() < 28 {
        return Vec::new();
    }
    let costliest = recent.iter().copied().fold(0.0, f64::max);
    if median <= costliest || median < 300.0 {
        return Vec::new();
    }
    let mut f = finding(
        format!("spike:{}", picture.zone.day(picture.now)),
        "spikes",
        // A long day is a note unless the money is large.
        if median >= 2.0 * costliest {
            Severity::Warning
        } else {
            Severity::Note
        },
        "on course for the costliest day in eight weeks".into(),
        format!(
            "On course for {} today; the most of any day in eight weeks was {}.",
            whole(median),
            whole(costliest)
        ),
    );
    f.badge = whole(median);
    f.note = format!("the most before was {}", whole(costliest));
    f.dollars = median - costliest;
    f.surprise = 1.5;
    vec![f]
}

/// A model the bill now prices apart from the month before, which the
/// estimates have already taken up: a note, since nothing is to be done but
/// knowing figures from the logs moved with it.
pub fn prices(picture: &Picture) -> Vec<Finding> {
    picture
        .shifts
        .iter()
        .map(|shift| {
            let mut f = finding(
                format!("price:{}", shift.model),
                "prices",
                Severity::Note,
                // Opened on its own words, since a title's first letter is
                // capitalized and a model's name is written as it's named.
                format!("a new price for {}", shift.model),
                format!(
                    "{} has billed at {} of its list price since {}, against {} in the \
                     month before; estimates from the logs use the new price.",
                    shift.model,
                    percent(shift.now),
                    Zone::day_name(shift.since),
                    percent(shift.before)
                ),
            );
            f.badge = percent(shift.now);
            f.note = format!("of list, from {} before", percent(shift.before));
            f.dollars = (shift.now - shift.before).abs() * shift.list_a_day * 7.0;
            f.surprise = 1.0;
            f
        })
        .collect()
}

#[cfg(test)]
#[path = "changes_tests.rs"]
mod tests;
