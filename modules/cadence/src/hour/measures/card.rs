//! What the hour reads off the tick's card: whether it is fresh enough to
//! be the truth, the medians its lines print, and merge to on from them.
//! The card changes with the tick's wording, not with any one measure.

use super::{Class, Measure, median};

/// How long a card stays the truth: three of the tick's five-minute runs.
pub const CARD_FRESH_FOR: i64 = 15 * 60;

/// The tick's `card`, drawn at `drawn` in seconds, if drawn within
/// [`CARD_FRESH_FOR`] of `now`, else nothing, so the hour leaves out what
/// it measures rather than repeat it. On 18 November, the tick stopped,
/// the hour said at 05:10 that three merges a 04:21 switch carried weren't on.
#[must_use]
pub fn card_if_fresh(card: String, drawn: Option<i64>, now: i64) -> String {
    match drawn {
        Some(at) if (0..=CARD_FRESH_FOR).contains(&(now - at)) => card,
        _ => String::new(),
    }
}

/// The median the tick's card gives on the line opening with `label`,
/// such as `Merge to on (target median 18): median 37.9, ...`, and the
/// target it names.
#[must_use]
pub fn card_median(card: &str, label: &str) -> Option<(f64, f64)> {
    let line = card.lines().find(|line| line.starts_with(label))?;
    Some((
        card_number(line, "): median ")?,
        card_number(line, "target median ")?,
    ))
}

/// The card's median minutes from a change's last commit to its merge,
/// held to the card's own target and owned by `owner`, or none when the
/// card has no such row.
#[must_use]
pub fn commit_to_merge(card: &str, owner: &str) -> Option<Measure> {
    let (median, target) = card_median(card, "Last commit to merge")?;
    Some(Measure {
        key: "commit-to-merge".to_owned(),
        class: Class::Ship,
        says: "median minutes from last commit to merge".to_owned(),
        now: median,
        target: Some(target),
        owner: owner.to_owned(),
        behind: Vec::new(),
    })
}

/// The number a card's `line` prints right after `after`.
fn card_number(line: &str, after: &str) -> Option<f64> {
    let rest = &line[line.find(after)? + after.len()..];
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// Merge to on from the card, as a measure owned by `owner`: the card's
/// median where a switch landed in its hour, else the median of the merges
/// not on yet, each at its wait so far, which each has taken at least; so
/// an hour with no switch reads its waits rather than nothing. The three
/// longest waits are named behind it either way.
#[must_use]
pub fn merge_to_on(card: &str, owner: &str) -> Option<Measure> {
    let label = "Merge to on";
    let line = card.lines().find(|line| line.starts_with(label))?;
    let target = card_number(line, "target median ")?;
    // The card prints `#4534 2 min, #4533 14 min, ...`, shortest first.
    let mut waits: Vec<(&str, f64)> = card
        .lines()
        .find_map(|line| line.trim_start().strip_prefix("not on yet: "))
        .into_iter()
        .flat_map(|list| list.split(", "))
        .filter_map(|wait| {
            let (pr, minutes) = wait.split_once(' ')?;
            Some((pr, minutes.strip_suffix(" min")?.parse().ok()?))
        })
        .collect();
    waits.sort_by(|a, b| b.1.total_cmp(&a.1));
    let now = match card_median(card, label) {
        Some((median_on, _)) => median_on,
        None if waits.is_empty() => return None,
        None => {
            let mut minutes: Vec<f64> = waits.iter().map(|(_, m)| *m).collect();
            minutes.sort_by(f64::total_cmp);
            median(&minutes)
        }
    };
    Some(Measure {
        key: "merge-to-on".to_owned(),
        class: Class::Ship,
        says: "median minutes from merge to on his machine, those not on yet at their wait so far"
            .to_owned(),
        now,
        target: Some(target),
        owner: owner.to_owned(),
        behind: waits
            .iter()
            .take(3)
            .map(|(pr, minutes)| format!("{pr} waiting {minutes} min"))
            .collect(),
    })
}
