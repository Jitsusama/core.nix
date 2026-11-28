//! What the work spends for little: context rebuilt past its usual share,
//! turns far heavier than their peers', and a quest spending while it
//! doesn't move.

use super::{finding, go_to};
use crate::config::Config;
use crate::findings::{Finding, Severity};
use crate::money::{dollars, grouped, percent, whole};
use crate::picture::Picture;
use crate::stats;
use std::collections::HashMap;
use std::fmt::Write as _;

pub fn waste(picture: &Picture, config: &Config) -> Vec<Finding> {
    let waste = &picture.waste;
    let ratio = if picture.fed {
        picture.ratios.overall.as_ref().map_or(1.0, |r| r.ratio)
    } else {
        1.0
    };
    let rebuilt = waste.rebuilt * ratio;
    if rebuilt < config.detectors.floor.max(100.0) {
        return Vec::new();
    }
    let Some((cause, amount)) = waste.causes.first() else {
        return Vec::new();
    };
    // Some rebuilding is how an account works, so it's news only past half
    // again the usual share of the eight weeks before, and four of them are
    // needed to say what's usual.
    let share = rebuilt / (waste.spent * ratio).max(rebuilt);
    if picture.waste_before.len() < 4 {
        return Vec::new();
    }
    let usual = stats::median(&picture.waste_before);
    if share <= 1.5 * usual {
        return Vec::new();
    }
    let mut detail = format!(
        "{} of the last 7 days' {} went on rebuilding context a session already had, the most \
         of it ({}) {}.",
        dollars(rebuilt),
        dollars(waste.spent * ratio),
        dollars(amount * ratio),
        crate::logs::Cause::words(cause)
    );
    let mut f = finding(
        "waste:week".into(),
        "waste",
        Severity::Note,
        "context rebuilt in 7 days".into(),
        String::new(),
    );
    f.badge = whole(rebuilt);
    f.note = format!(
        "{} of all spent, usually {}",
        percent(share),
        percent(usual)
    );
    if let Some((session, worst)) = &waste.worst
        && worst * ratio >= config.detectors.floor
    {
        // Writing to a String can't fail.
        _ = write!(
            detail,
            " {} of it after pauses in one session.",
            dollars(worst * ratio)
        );
        f.session = Some(session.clone());
        f.action = go_to(picture, session);
    }
    f.detail = detail;
    f.dollars = rebuilt;
    f.surprise = 1.0;
    vec![f]
}

pub fn peers(picture: &Picture, config: &Config) -> Vec<Finding> {
    let ratio = if picture.fed {
        picture.ratios.overall.as_ref().map_or(1.0, |r| r.ratio)
    } else {
        1.0
    };
    let recent = &picture.work;
    let found = crate::waste::outliers(
        recent,
        config.detectors.false_discovery,
        config.detectors.floor / ratio.max(0.01),
    );
    // A quest's sessions are one piece of work, so however many of them
    // stand out it's one finding; a session no quest holds stands alone.
    let mut groups: Vec<(String, Option<&str>, Vec<&crate::waste::Outlier>)> = Vec::new();
    for outlier in &found {
        let root = outlier
            .work
            .quest
            .as_deref()
            .map(|quest| picture.root(quest));
        let key = match root {
            Some(root) => format!("peer:quest:{root}"),
            None => format!("peer:{}", outlier.work.session),
        };
        match groups.iter_mut().find(|(held, _, _)| *held == key) {
            Some((_, _, members)) => members.push(outlier),
            None => groups.push((key, root, vec![outlier])),
        }
    }
    groups
        .into_iter()
        .take(3)
        .filter_map(|(key, root, members)| {
            let worst = *members.iter().max_by(|a, b| a.times.total_cmp(&b.times))?;
            let spent: f64 = members.iter().map(|o| o.work.dollars).sum::<f64>() * ratio;
            let turns: f64 = members.iter().map(|o| o.work.turns).sum();
            let sessions = match members.len() {
                1 => "one session".to_owned(),
                n => format!("{n} sessions"),
            };
            let mut f = finding(
                key,
                "peers",
                Severity::Note,
                match root {
                    Some(root) => format!("heavy turns in {}", picture.title(root)),
                    None => "heavy turns in a session".into(),
                },
                format!(
                    "{} at up to {:.1} times the usual per turn, {} over {} turns; the worst \
                     took {} of its input from cache against {} usual. Held to a {} false \
                     discovery rate across {} sessions.",
                    capitalized(&sessions),
                    worst.times,
                    dollars(spent),
                    turns_shown(turns),
                    percent(worst.work.cached),
                    percent(worst.usual_cached),
                    percent(config.detectors.false_discovery),
                    recent.len()
                ),
            );
            f.badge = format!("{:.1}\u{d7} usual", worst.times);
            f.note = format!(
                "{sessions}, {} over {} turns",
                whole(spent),
                turns_shown(turns)
            );
            f.dollars = members.iter().map(|o| o.excess).sum::<f64>() * ratio;
            f.surprise = members
                .iter()
                .map(|o| -o.p.max(1e-12).log10())
                .fold(0.0, f64::max);
            f.session = Some(worst.work.session.clone());
            f.quest = root.map(str::to_owned);
            f.action = go_to(picture, &worst.work.session);
            Some(f)
        })
        .collect()
}

/// A count of turns, grouped by thousands.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "turns are a sum of whole counts of at least zero, far inside u64"
)]
fn turns_shown(turns: f64) -> String {
    grouped(turns as u64)
}

fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// A quest spending while it doesn't move: real money over the last week
/// and no update to its record in five days.
pub fn stalled(picture: &Picture, config: &Config) -> Vec<Finding> {
    let mut by_root: HashMap<&str, f64> = HashMap::new();
    for piece in picture.pieces_week() {
        if let Some(quest) = &piece.quest {
            *by_root.entry(picture.root(quest)).or_default() += piece.dollars;
        }
    }
    // A subquest's spend is its root's, so its changes are too: a root's
    // record can sit still while the work moves in a subquest.
    let mut changed: HashMap<&str, i64> = HashMap::new();
    for quest in &picture.quests {
        if let Some(day) = quest.updated.as_deref().and_then(crate::time::parse_day) {
            let latest = changed.entry(picture.root(&quest.id)).or_insert(day);
            *latest = (*latest).max(day);
        }
    }
    let today = picture.zone.day(picture.now);
    let mut out = Vec::new();
    for quest in &picture.quests {
        if quest.status != "active" {
            continue;
        }
        let spent = by_root.get(quest.id.as_str()).copied().unwrap_or(0.0);
        let updated = changed.get(quest.id.as_str());
        let quiet_days = updated.map_or(0, |day| today - day);
        if spent >= config.detectors.floor.max(100.0) && quiet_days >= 5 {
            let mut f = finding(
                format!("stalled:{}", quest.id),
                "outcomes",
                Severity::Note,
                format!("still spending on {}", quest.title),
                format!(
                    "{} in the last 7 days; its records last changed {quiet_days} days ago, with \
                     {} pull requests.",
                    dollars(spent),
                    quest.prs
                ),
            );
            f.badge = whole(spent);
            f.note = format!("in 7 days; unchanged for {quiet_days} days");
            f.dollars = spent;
            f.quest = Some(quest.id.clone());
            out.push(f);
        }
    }
    out
}

#[cfg(test)]
#[path = "work_tests.rs"]
mod tests;
