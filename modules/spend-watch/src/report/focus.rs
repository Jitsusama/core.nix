//! The focused reports: the week at a glance, a cap's line, waste, quests,
//! outliers, caps and the forecast.

use super::cuts::{short_session, source_line, spend_cells, spend_columns};
use super::{Period, Report, Row};
use crate::findings::Finding;
use crate::money::{dollars, percent, whole};
use crate::picture::Picture;
use crate::time::Zone;
use std::collections::HashMap;
use std::fmt::Write as _;
use terminal_kit::Align;

/// The week at a glance: where it stands, the forecast, the caps, the
/// findings and where money went to waste.
pub fn summary(picture: &Picture, findings: &[Finding]) -> Report {
    let mut notes = Vec::new();
    let figure = |label: &str, figure: &crate::picture::Figure| {
        format!(
            "{label}: {}",
            crate::money::maybe_about(dollars(figure.dollars), figure.estimated)
        )
    };
    notes.push(figure("Today", &picture.today));
    if !picture.standing.word.is_empty() {
        let stands = match picture.standing.word.as_str() {
            "quiet" => "quieter than usual",
            "ahead" => "ahead of usual",
            _ => "usual",
        };
        notes.push(format!(
            "Today is {stands}: a usual day by this hour is {}.",
            dollars(picture.standing.usual)
        ));
    }
    notes.push(figure("Yesterday", &picture.yesterday));
    notes.push(figure("The last 7 days", &picture.week));
    notes.push(figure("The last 30 days", &picture.month));
    if let Some((low, mid, high)) = picture.today_ahead() {
        notes.push(format!(
            "Today should end near {} ({} to {}, 80%).",
            whole(mid),
            whole(low),
            whole(high)
        ));
    }
    if let Some((low, mid, high)) = picture.week_ahead() {
        notes.push(format!(
            "The next 7 days: {} ({} to {}, 80%).",
            whole(mid),
            whole(low),
            whole(high)
        ));
    }
    for view in &picture.caps {
        notes.push(cap_line(picture, view));
    }
    let waste = &picture.waste;
    if waste.spent > 0.0 {
        notes.push(format!(
            "Cache writes in the last 7 days: {} of {} at list prices, {} of it rebuilding \
             context.",
            dollars(waste.cache_write),
            dollars(waste.spent),
            dollars(waste.rebuilt)
        ));
    }
    let rows = findings
        .iter()
        .filter(|finding| !finding.marked)
        .map(|finding| Row {
            name: format!("{}: {}", finding.severity.name(), finding.title),
            dollars: finding.dollars,
            share: 0.0,
            shown: finding.detail.clone(),
            estimated: false,
            cells: Vec::new(),
        })
        .collect();
    let week = picture.between(
        picture.zone.day(picture.now) - 6,
        picture.zone.day(picture.now),
    );
    Report {
        title: "AI spend".into(),
        period: "summary".into(),
        from: Zone::iso_day(picture.zone.day(picture.now) - 6),
        to: Zone::iso_day(picture.zone.day(picture.now)),
        total: picture.week.dollars,
        shown: crate::money::maybe_about(dollars(picture.week.dollars), picture.week.estimated),
        source: source_line(week),
        rest: None,
        rows,
        notes,
        columns: Vec::new(),
    }
}

pub fn cap_line(picture: &Picture, view: &crate::picture::CapView) -> String {
    let cap = &view.cap;
    if let Some(used) = view.used {
        let resets = view.resets.map_or_else(String::new, |at| {
            format!(" It resets at {}.", picture.zone.clock(at, picture.now))
        });
        return format!("{}: {} used.{resets}", cap.label, percent(used));
    }
    let Some(allowance) = cap.allowance else {
        return format!("{}: no limit, {} spent.", cap.label, whole(view.spent));
    };
    // A feed's figure is the provider's own count at its last run, so the
    // time it ran is said with it; a machine's own cap is counted here.
    let by = if cap.feed.is_empty() {
        String::new()
    } else {
        format!(" by the {} count", picture.zone.clock(cap.at, picture.now))
    };
    let mut line = format!(
        "{}: {} of {} ({}){by}.",
        cap.label,
        whole(view.spent),
        whole(allowance),
        percent(view.spent / allowance)
    );
    // Writing to a String can't fail.
    if let Some(at) = view.resets {
        _ = write!(
            line,
            " It resets at {}.",
            picture.zone.clock(at, picture.now)
        );
    }
    if let Some(clock) = &view.clock {
        match (clock.frees_at, view.resets) {
            // Back under when it resets, which is said already.
            (Some(_), Some(_)) => {}
            (Some(at), None) => {
                _ = write!(
                    line,
                    " Back under by {}.",
                    picture.zone.clock(at, picture.now)
                );
            }
            (None, _) => {
                _ = write!(
                    line,
                    " A {} chance of reaching it within a day.",
                    percent(clock.cap_within_day)
                );
            }
        }
        if let (Some(ends), Some(after)) = (cap.ends, clock.over_after_end) {
            _ = write!(
                line,
                " The override ends {}, with a {} chance of being over what follows.",
                Zone::day_name(ends),
                percent(after)
            );
        }
    }
    if let (Some(ends), false) = (cap.ends, cap.ends_known) {
        _ = write!(line, " Its end, {}, is a guess.", Zone::day_name(ends));
    }
    line
}

/// Where money bought nothing in the last seven days.
pub fn waste(picture: &Picture) -> Report {
    let waste = &picture.waste;
    let rows = waste
        .causes
        .iter()
        .map(|(cause, spent)| {
            let share = if waste.rebuilt > 0.0 {
                spent / waste.rebuilt
            } else {
                0.0
            };
            Row {
                name: crate::logs::Cause::words(cause).to_owned(),
                dollars: *spent,
                share,
                shown: dollars(*spent),
                estimated: true,
                cells: spend_cells(&dollars(*spent), share),
            }
        })
        .collect();
    let mut notes = vec![format!(
        "Of {} spent at list prices, {} was cache written and {} of that rebuilt context a \
         session already had.",
        dollars(waste.spent),
        dollars(waste.cache_write),
        dollars(waste.rebuilt)
    )];
    if let Some((session, spent)) = &waste.worst {
        notes.push(format!(
            "The most after pauses was {} in session {}.",
            dollars(*spent),
            short_session(session)
        ));
    }
    let today = picture.zone.day(picture.now);
    Report {
        title: "Context rebuilt in 7 days".into(),
        period: "waste".into(),
        from: Zone::iso_day(today - 6),
        to: Zone::iso_day(today),
        total: waste.rebuilt,
        shown: dollars(waste.rebuilt),
        source: "pi's logs at list prices".into(),
        rest: None,
        rows,
        notes,
        columns: spend_columns("cause"),
    }
}

/// What each quest has cost, beside where it stands.
pub fn quests(picture: &Picture, period: Period) -> Report {
    let (from, to) = period.days(&picture.zone, picture.now);
    let pieces = picture.between(from, to);
    let mut by_root: HashMap<&str, f64> = HashMap::new();
    for piece in pieces {
        if let Some(quest) = &piece.quest {
            *by_root.entry(picture.root(quest)).or_default() += piece.dollars;
        }
    }
    let total: f64 = by_root.values().sum();
    let mut rows: Vec<Row> = by_root
        .into_iter()
        .map(|(id, spent)| {
            let quest = picture.quests.iter().find(|quest| quest.id == id);
            let (title, status, prs) = quest
                .map_or((id.to_owned(), "unknown".to_owned(), 0), |quest| {
                    (quest.title.clone(), quest.status.clone(), quest.prs)
                });
            let share = if total > 0.0 { spent / total } else { 0.0 };
            let mut cells = spend_cells(&dollars(spent), share);
            cells.extend([status.clone(), prs.to_string()]);
            Row {
                name: title,
                dollars: spent,
                share,
                cells,
                shown: format!(
                    "{} · {status} · {}",
                    dollars(spent),
                    if prs == 1 {
                        "1 pull request".to_owned()
                    } else {
                        format!("{prs} pull requests")
                    }
                ),
                estimated: false,
            }
        })
        .collect();
    rows.sort_by(|one, other| other.dollars.total_cmp(&one.dollars));
    rows.truncate(25);
    let concluded: Vec<&Row> = rows
        .iter()
        .filter(|row| row.shown.contains("concluded"))
        .collect();
    let mut notes = Vec::new();
    if !concluded.is_empty() {
        let spent: f64 = concluded.iter().map(|row| row.dollars).sum();
        notes.push(format!(
            "{} concluded quests cost {} together.",
            concluded.len(),
            dollars(spent)
        ));
    }
    Report {
        title: format!("Quests {}", period.name()),
        period: period.id().into(),
        from: Zone::iso_day(from),
        to: Zone::iso_day(to),
        total,
        shown: dollars(total),
        source: source_line(pieces),
        rest: None,
        columns: [
            spend_columns("quest"),
            vec![("status", Align::Left), ("pull requests", Align::Right)],
        ]
        .concat(),
        rows,
        notes,
    }
}

/// Sessions costing far more per turn than their peers, the last 30 days.
pub fn outliers(picture: &Picture, false_discovery: f64, floor: f64) -> Report {
    let found = crate::waste::outliers(&picture.work, false_discovery, floor);
    let rows = found
        .iter()
        .map(|outlier| {
            let name = outlier
                .work
                .quest
                .as_ref()
                .and_then(|id| picture.quests.iter().find(|quest| &quest.id == id))
                .map_or_else(
                    || outlier.work.session.clone(),
                    |quest| format!("{} · {}", quest.title, short_session(&outlier.work.session)),
                );
            Row {
                name,
                dollars: outlier.excess,
                share: 0.0,
                shown: format!(
                    "{} beyond the usual: {:.1}\u{d7} per turn, {} turns",
                    dollars(outlier.excess),
                    outlier.times,
                    outlier.work.turns
                ),
                estimated: true,
                cells: Vec::new(),
            }
        })
        .collect();
    let today = picture.zone.day(picture.now);
    Report {
        title: "Costly sessions, the last 30 days".into(),
        period: "outliers".into(),
        from: Zone::iso_day(today - 29),
        to: Zone::iso_day(today),
        total: found.iter().map(|o| o.excess).sum(),
        shown: dollars(found.iter().map(|o| o.excess).sum()),
        source: "pi's logs at list prices".into(),
        rest: None,
        columns: Vec::new(),
        rows,
        notes: vec![format!(
            "{} sessions compared, each shrunk toward the usual by its turns, held to a {} false \
             discovery rate.",
            picture.work.len(),
            percent(false_discovery)
        )],
    }
}

/// The caps and their history of changes.
pub fn caps(picture: &Picture) -> Report {
    let mut notes: Vec<String> = picture
        .caps
        .iter()
        .map(|view| cap_line(picture, view))
        .collect();
    if picture.caps.is_empty() {
        notes.push("No cap is known: no feed gives one and none is set.".into());
    }
    let rows = picture
        .events
        .iter()
        .map(|event| Row {
            name: format!(
                "{} {}",
                picture.zone.clock(event.at, picture.now),
                event.kind
            ),
            dollars: event.after.unwrap_or(0.0),
            share: 0.0,
            shown: format!(
                "{} to {} with {} spent",
                event.before.map_or("no limit".into(), whole),
                event.after.map_or("no limit".into(), whole),
                whole(event.spent)
            ),
            estimated: false,
            cells: Vec::new(),
        })
        .collect();
    let refused = picture
        .refusals
        .iter()
        .filter(|(at, _)| picture.now - at < 30.0 * crate::time::DAY)
        .count();
    if refused > 0 {
        notes.push(format!(
            "{refused} of pi's requests were refused for a cap in the last 30 days."
        ));
    }
    Report {
        title: "Caps".into(),
        period: "caps".into(),
        from: String::new(),
        to: String::new(),
        total: 0.0,
        shown: String::new(),
        source: String::new(),
        rest: None,
        columns: Vec::new(),
        rows,
        notes,
    }
}

pub fn forecast(picture: &Picture) -> Report {
    let mut notes = Vec::new();
    match (picture.today_ahead(), picture.week_ahead()) {
        (Some(today), Some(week)) => {
            notes.push(format!(
                "Today: {} ({} to {}, 80%).",
                whole(today.1),
                whole(today.0),
                whole(today.2)
            ));
            notes.push(format!(
                "The next 7 days: {} ({} to {}, 80%).",
                whole(week.1),
                whole(week.0),
                whole(week.2)
            ));
            notes.push(
                "A blend of a smoothing model on log daily spend and the same days of the last \
                 eight weeks."
                    .into(),
            );
        }
        _ => notes.push("Too few days to forecast: two weeks are needed.".into()),
    }
    let today = picture.zone.day(picture.now);
    let rows = (0..7)
        .filter(|_| !picture.paths.is_empty())
        .map(|ahead| {
            let mut day: Vec<f64> = picture.paths.iter().map(|path| path[ahead]).collect();
            day.sort_by(f64::total_cmp);
            let q = |p| crate::stats::quantile_sorted(&day, p);
            Row {
                name: Zone::day_name(today + crate::stats::signed(ahead)),
                dollars: q(0.5),
                share: 0.0,
                shown: format!("{} ({} to {})", whole(q(0.5)), whole(q(0.1)), whole(q(0.9))),
                estimated: true,
                cells: Vec::new(),
            }
        })
        .collect();
    Report {
        title: "The week ahead".into(),
        period: "forecast".into(),
        from: Zone::iso_day(today),
        to: Zone::iso_day(today + 6),
        total: picture.week_ahead().map_or(0.0, |w| w.1),
        shown: picture.week_ahead().map_or(String::new(), |w| whole(w.1)),
        source: "forecast".into(),
        rest: None,
        columns: Vec::new(),
        rows,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::caps::{Cap, Clock, Unit};
    use crate::picture::CapView;

    const NOW: f64 = 1_791_988_200.0; // 2026-10-14T14:30:00Z
    const HOUR: f64 = crate::time::HOUR;

    /// A picture of nothing spent, in UTC at `NOW`, for what a report says
    /// over figures set by hand.
    fn an_empty_picture() -> Picture {
        let config: crate::config::Config = serde_json::from_value(serde_json::json!({
            "sessions": "/none", "runs": "/none", "registry": "/none", "quests": "/none",
            "state": "/none", "zone": "UTC",
        }))
        .unwrap();
        Picture::read(&crate::history::History::memory(), &config, NOW).unwrap()
    }

    // The session that rebuilt most is named by its first eight
    // characters, not bytes, since a brought log sets its id.
    #[test]
    fn the_worst_session_is_cut_at_a_whole_character() {
        let mut picture = an_empty_picture();
        picture.waste.worst = Some(("abcdefg\u{e9}-on".into(), 12.0));
        assert_eq!(
            waste(&picture).notes.last().map(String::as_str),
            Some("The most after pauses was $12 in session abcdefg\u{e9}."),
            "the worst session wasn't named by its first eight characters"
        );
    }

    /// A dollar cap named `label`, a machine's own where `feed` is empty,
    /// last read half an hour before `NOW`.
    fn a_cap(label: &str, feed: &str, allowance: Option<f64>) -> Cap {
        Cap {
            feed: feed.into(),
            budget: String::new(),
            name: "week".into(),
            label: label.into(),
            unit: Unit::Dollars,
            allowance,
            window_hours: 168,
            spent: 0.0,
            warn: 0.8,
            fallback: None,
            state: "ok".into(),
            ends: None,
            ends_known: false,
            resets: None,
            used: None,
            at: NOW - 0.5 * HOUR,
        }
    }

    /// How `cap` stands with `spent` in its window, read as nothing more.
    fn standing_at(cap: Cap, spent: f64) -> CapView {
        CapView {
            cap,
            budget: "default".into(),
            owner: "default".into(),
            spent,
            used: None,
            clock: None,
            ends_at: None,
            resets: None,
        }
    }

    // A share of a plan's window says how much is used, and when it resets
    // where it does, never as dollars.
    #[test]
    fn a_cap_used_says_its_share() {
        let picture = an_empty_picture();
        let mut view = standing_at(a_cap("Five Hours", "plan", None), 0.0);
        view.cap.unit = Unit::Share;
        view.used = Some(0.42);
        assert_eq!(
            cap_line(&picture, &view),
            "Five Hours: 42% used.",
            "a share used wasn't said as its share"
        );
        view.resets = Some(NOW + 2.0 * HOUR);
        assert_eq!(
            cap_line(&picture, &view),
            "Five Hours: 42% used. It resets at 16:30.",
            "a share used didn't say when it resets"
        );
    }

    // A cap with no allowance limits nothing, so it says only what's spent.
    #[test]
    fn a_cap_with_no_allowance_says_what_is_spent() {
        let picture = an_empty_picture();
        let view = standing_at(a_cap("Team", "provider", None), 37.4);
        assert_eq!(
            cap_line(&picture, &view),
            "Team: no limit, $37 spent.",
            "a cap with no allowance said more than what's spent"
        );
    }

    // A feed's figure is the provider's count when it last ran, so its time
    // is said; a machine's own cap is counted here, so nothing is.
    #[test]
    fn a_feed_s_cap_says_when_it_was_counted_and_the_machine_s_does_not() {
        let picture = an_empty_picture();
        let fed = standing_at(a_cap("Provider Week", "provider", Some(400.0)), 100.0);
        assert_eq!(
            cap_line(&picture, &fed),
            "Provider Week: $100 of $400 (25%) by the 14:00 count.",
            "a feed's cap didn't say when the provider counted it"
        );
        let own = standing_at(a_cap("Own Week", "", Some(400.0)), 100.0);
        assert_eq!(
            cap_line(&picture, &own),
            "Own Week: $100 of $400 (25%).",
            "the machine's own cap was said as a provider's count"
        );
    }

    // A cap that resets says when, with a clock or without one; over, it's
    // back under when it resets, so that isn't said twice, and with no
    // reset it says when the window frees it.
    #[test]
    fn a_cap_says_when_it_resets_with_a_clock_and_without() {
        let picture = an_empty_picture();
        let mut view = standing_at(a_cap("Own Week", "", Some(400.0)), 100.0);
        view.resets = Some(NOW + 24.0 * HOUR);
        assert_eq!(
            cap_line(&picture, &view),
            "Own Week: $100 of $400 (25%). It resets at 14:30 tomorrow.",
            "a cap with no clock didn't say when it resets"
        );
        view.clock = Some(Clock {
            cap_within_day: 0.3,
            ..Clock::default()
        });
        assert_eq!(
            cap_line(&picture, &view),
            "Own Week: $100 of $400 (25%). It resets at 14:30 tomorrow. A 30% chance of \
             reaching it within a day.",
            "a cap under its allowance didn't say its chance of reaching it"
        );
        view.clock = Some(Clock {
            frees_at: Some(NOW + 3.0 * HOUR),
            ..Clock::default()
        });
        assert_eq!(
            cap_line(&picture, &view),
            "Own Week: $100 of $400 (25%). It resets at 14:30 tomorrow.",
            "a cap over until it resets said when it's back under twice"
        );
        view.resets = None;
        assert_eq!(
            cap_line(&picture, &view),
            "Own Week: $100 of $400 (25%). Back under by 17:30.",
            "a cap over with no reset didn't say when it's back under"
        );
    }
}
