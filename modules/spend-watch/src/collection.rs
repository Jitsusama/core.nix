//! What the launcher's spend collection draws, `collection.json`, written
//! only when it changes. It's laid out as the launcher's other collections
//! are (docs/design.md, Kinds): each row a short name with a few muted
//! words beside it, what else is known of it in the footer while it's
//! picked, and the rest a right arrow away, behind the row it belongs to:
//! a period's cuts behind the period, a cut's rows behind the cut, a cap's
//! changes behind the cap. So each fact has one home and the first screen
//! holds only how things stand and what was found.
//!
//! Every string is ready to draw, so the shell does no arithmetic and
//! formats no money, and the quiet lines, drawn while the screen is
//! shared, hold no amount.

use crate::findings::{Finding, Severity};
use crate::money::{dollars, maybe_about, percent, whole};
use crate::picture::{CapView, Picture};
use crate::report::{self, Cut, Format, Period, Report};
use crate::series::{self, Source};
use crate::status::{action_kind, capitalized, unpriced};
use crate::time::Zone;
use serde::Serialize;
use std::collections::BTreeMap;

pub const VERSION: u32 = 2;

/// How many of a cut's rows are shown before the rest are one row.
const TOP: usize = 12;

/// The periods behind Earlier, most recent first.
const EARLIER: [Period; 6] = [
    Period::Yesterday,
    Period::ThisWeek,
    Period::LastWeek,
    Period::ThisMonth,
    Period::LastMonth,
    Period::Days30,
];

/// One row, as the launcher draws it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    pub key: String,
    pub name: String,
    /// A few words beside the name: a figure, a share, a time.
    pub side: String,
    /// What the footer says while it's picked.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub note: String,
    /// What Return copies.
    pub copy: String,
    /// What its glyph shows: text, a report, a sign-in, or a finding that's
    /// found, a warning, an alert or expected.
    pub role: String,
    /// What Return runs instead of copying, and the footer's word for it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run: Option<Vec<String>>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub verb: String,
    /// What marks a finding expected, so it never asks again.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expect: Option<Vec<String>>,
    /// The report it stands for, by id: the right arrow steps into that
    /// report's rows, and Return copies it whole.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub report: String,
    /// What else the right arrow steps into.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<Line>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shown {
    pub id: String,
    pub title: String,
    pub headline: String,
    pub rows: Vec<Line>,
    /// The whole report, for Return to copy.
    pub markdown: String,
    /// What else finds it when it's typed for: its period's and cut's
    /// words, or what it's about.
    pub words: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Words {
    pub id: String,
    pub words: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub v: u32,
    /// How things stand: today, each cap longer than a day, and the
    /// periods before.
    pub now: Vec<Line>,
    /// What's been found, best first.
    pub findings: Vec<Line>,
    /// Reports that belong behind no other row.
    pub more: Vec<Line>,
    /// What the collection shows while the screen is shared: how today
    /// stands, each cap's share and each finding, with no amount in any.
    pub quiet: Vec<Line>,
    /// Each report by its id, "<period>/<cut>" or a focused one's name,
    /// for the rows that stand for one and the answers to what's typed.
    pub reports: BTreeMap<String, Shown>,
    pub periods: Vec<Words>,
    pub cuts: Vec<Words>,
    pub examples: Vec<String>,
}

fn line(key: impl Into<String>, name: impl Into<String>, side: impl Into<String>) -> Line {
    let (name, side) = (name.into(), side.into());
    Line {
        key: key.into(),
        copy: if side.is_empty() {
            name.clone()
        } else {
            format!("{name}: {side}")
        },
        name,
        side,
        note: String::new(),
        role: "text".into(),
        run: None,
        verb: String::new(),
        expect: None,
        report: String::new(),
        rows: Vec::new(),
    }
}

/// Where a figure came from, in the few words a footer has, when it's
/// less than what's expected: a feed's bill where there's a feed, and
/// pi's list prices where there isn't. Saying what's expected would be
/// words on every row that tell nothing.
fn basis(source: Option<Source>, fed: bool) -> &'static str {
    match source {
        Some(Source::Estimated) => "estimated where the bill hasn't come",
        Some(Source::List) if fed => "at list prices: no bill read lately",
        _ => "",
    }
}

/// Parts of a footer, those that say something, between dots.
fn joined(parts: &[&str]) -> String {
    parts
        .iter()
        .filter(|part| !part.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" · ")
}

/// A range the forecast gives, as its footer says it.
fn likely(low: f64, high: f64) -> String {
    format!("likely {} to {}", whole(low), whole(high))
}

/// One of a report's rows: its figure and share beside it, and whatever
/// else the report says of it in the footer.
fn row_line(key: String, row: &report::Row) -> Line {
    let figure = maybe_about(dollars(row.dollars), row.estimated);
    // A share too small to round to a percent says nothing the figure
    // doesn't.
    let side = if percent(row.share) != "0%" {
        format!("{figure} · {}", percent(row.share))
    } else {
        figure.clone()
    };
    let mut made = line(key, row.name.clone(), side);
    let rest = row
        .shown
        .strip_prefix(&format!("{figure} · "))
        .unwrap_or(&row.shown);
    if rest != figure {
        made.note = rest.to_owned();
    }
    made
}

/// A report's rows, with those left out past its top as one more.
fn rows_of(id: &str, report: &Report) -> Vec<Line> {
    let mut rows: Vec<Line> = report
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| row_line(format!("{id}/{index}"), row))
        .collect();
    if let Some((count, spent)) = report.rest {
        let mut more = line(
            format!("{id}/more"),
            format!("{count} more"),
            dollars(spent),
        );
        more.note = "together".into();
        rows.push(more);
    }
    rows
}

fn shown(id: &str, words: &str, report: &Report) -> Shown {
    Shown {
        id: id.to_owned(),
        title: report.title.clone(),
        headline: if report.shown.is_empty() {
            report.notes.first().cloned().unwrap_or_default()
        } else {
            report.shown.clone()
        },
        rows: rows_of(id, report),
        markdown: report::render(report, Format::Markdown),
        words: words.to_owned(),
    }
}

/// A period's day reports cut by day say nothing a period of a day
/// doesn't.
fn cuts_for(period: Period) -> impl Iterator<Item = Cut> {
    let one_day = matches!(period, Period::Today | Period::Yesterday);
    Cut::ALL
        .into_iter()
        .filter(move |cut| !(one_day && *cut == Cut::Day))
}

/// The row standing for a period's spend cut one way: how many it's cut
/// into, and the largest. A cut into one says nothing, so it's none.
fn cut_line(id: &str, cut: Cut, report: &Report) -> Option<Line> {
    let count = report.rows.len() + report.rest.map_or(0, |(count, _)| count);
    let top = report.rows.first()?;
    if count < 2 {
        return None;
    }
    let plural = cut.words().get(1).copied().unwrap_or(cut.id());
    let mut made = line(id, format!("By {}", cut.id()), format!("{count} {plural}"));
    made.note = if cut == Cut::Day {
        format!("the most on {}", top.name)
    } else {
        format!("{} to {}", percent(top.share), top.name)
    };
    made.role = "report".into();
    made.report = id.to_owned();
    Some(made)
}

/// A period's row: its spend beside it, where that came from below, and
/// its cuts behind it.
fn period_line(
    period: Period,
    reports: &BTreeMap<String, (Report, Shown)>,
    side: String,
    note: String,
) -> Line {
    let mut made = line(
        format!("period:{}", period.id()),
        capitalized(period.name()),
        side,
    );
    made.note = note;
    made.rows = cuts_for(period)
        .filter_map(|cut| {
            let id = format!("{}/{}", period.id(), cut.id());
            cut_line(&id, cut, &reports.get(&id)?.0)
        })
        .collect();
    made
}

/// How a period's spend reads, and where it came from.
fn figure_of(picture: &Picture, period: Period) -> (String, &'static str) {
    let (from, to) = period.days(&picture.zone, picture.now);
    let pieces = picture.between(from, to);
    (
        maybe_about(dollars(series::total(&pieces)), series::estimated(&pieces)),
        basis(series::weakest(&pieces), picture.fed),
    )
}

/// A window of hours as a person says it.
fn window(hours: u32) -> String {
    match hours {
        24 => "the last day".into(),
        hours if hours % 24 == 0 => format!("the last {} days", hours / 24),
        hours => format!("the last {hours} hours"),
    }
}

/// A cap's row: its share beside it, what's counted and when it frees in
/// the footer, and its changes behind it, with the sign-ins its feed
/// offers for what it can't read without one.
fn cap_line(picture: &Picture, view: &CapView, sign_ins: Vec<Line>) -> Line {
    let cap = &view.cap;
    let now = picture.now;
    let side = match cap.allowance {
        Some(allowance) => format!(
            "{} of {}",
            percent(view.spent / allowance),
            whole(allowance)
        ),
        None => format!("{} spent", whole(view.spent)),
    };
    let mut made = line(format!("cap:{}", cap.name), capitalized(&cap.label), side);
    let counted = match view.resets {
        Some(at) => format!(
            "{} spent, resets {}",
            whole(view.spent),
            picture.zone.clock(at, now)
        ),
        None => format!("{} in {}", whole(view.spent), window(cap.window_hours)),
    };
    let ends = cap.ends.map(|day| {
        let said = format!("override ends {}", Zone::day_name(day));
        if cap.ends_known {
            said
        } else {
            format!("{said}, a guess")
        }
    });
    made.note = joined(&[&counted, ends.as_deref().unwrap_or("")]);
    made.copy = report::cap_line(picture, view);
    let mut events: Vec<&crate::caps::Event> = picture
        .events
        .iter()
        .filter(|event| event.feed == cap.feed && event.name == cap.name)
        .collect();
    events.sort_by(|one, other| other.at.total_cmp(&one.at));
    made.rows = events
        .into_iter()
        .map(|event| {
            let name = match event.kind.as_str() {
                "lowered" => "Lowered".to_owned(),
                "raised" => "Raised".to_owned(),
                "lapsed" => "Override lapsed".to_owned(),
                "refused" => "Refusing".to_owned(),
                "notified" => "Warned".to_owned(),
                "ok" => "Back under".to_owned(),
                kind => capitalized(kind),
            };
            let limit = |amount: Option<f64>| amount.map_or("no limit".into(), whole);
            let mut row = line(
                format!("cap:{}:{}:{}", cap.name, event.kind, event.at as i64),
                name,
                picture.zone.clock(event.at, now),
            );
            let spent = format!("with {} spent", whole(event.spent));
            // A change of state says only what was spent; a change of
            // allowance, made at the same moment, says what it went to.
            let changed = matches!(event.kind.as_str(), "lowered" | "raised" | "lapsed");
            row.note = if !changed || event.before == event.after {
                spent
            } else {
                format!("{} to {}, {spent}", limit(event.before), limit(event.after))
            };
            row.copy = format!("{}: {}", row.name, row.note);
            row
        })
        .chain(sign_ins)
        .collect();
    made
}

/// The sign-ins a feed offers for sources it can do without, each to sit
/// behind the first of that feed's caps.
fn sign_ins(picture: &Picture, feed: &str) -> Vec<Line> {
    picture
        .sources
        .iter()
        .filter(|source| source.feed == feed && source.state == "optional")
        .filter_map(|source| {
            let id = source.sign_in.as_ref()?;
            let action = picture
                .actions
                .iter()
                .find(|action| &action.id == id && action.feed == feed)?;
            let mut made = line(
                format!("sign-in:{feed}:{}", source.name),
                action.name.clone(),
                format!("for {}", source.name),
            );
            made.note = source.message.clone();
            made.role = "sign-in".into();
            made.run = Some(action.run.clone());
            made.verb = "sign in".into();
            Some(made)
        })
        .collect()
}

fn verb(action: &crate::findings::Action) -> &'static str {
    match action_kind(action) {
        "go" => "go",
        "sign-in" => "sign in",
        _ => "ask",
    }
}

fn role(finding: &Finding) -> &'static str {
    match (finding.marked, finding.severity) {
        (true, _) => "expected",
        (false, Severity::Alert) => "alert",
        (false, Severity::Warning) => "warning",
        (false, Severity::Note) => "found",
    }
}

fn finding_line(finding: &Finding) -> Line {
    let name = capitalized(&finding.title);
    let mut made = line(
        format!("finding:{}", finding.key),
        name.clone(),
        if finding.marked {
            "expected".to_owned()
        } else {
            finding.badge.clone()
        },
    );
    made.note = finding.note.clone();
    made.copy = format!("{name}. {}", finding.detail);
    made.role = role(finding).into();
    if let Some(action) = &finding.action {
        made.run = Some(action.run.clone());
        made.verb = verb(action).into();
    }
    made.expect = (!finding.marked).then(|| {
        ["spend-watch", "findings", "expect", &finding.key]
            .map(String::from)
            .to_vec()
    });
    // Where the money went to waste is the finding's own detail.
    if finding.detector == "waste" {
        made.report = "waste".into();
    }
    made
}

/// The same, saying no amount.
fn quiet_finding(finding: &Finding) -> Line {
    let name =
        unpriced(&capitalized(&finding.title)).unwrap_or_else(|| "Something wants a look".into());
    let mut made = line(
        format!("quiet:finding:{}", finding.key),
        name.clone(),
        if finding.marked {
            "expected".to_owned()
        } else {
            unpriced(&finding.badge).unwrap_or_default()
        },
    );
    made.note = unpriced(&finding.note).unwrap_or_default();
    made.copy = name;
    made.role = role(finding).into();
    if let Some(action) = &finding.action {
        made.run = Some(action.run.clone());
        made.verb = verb(action).into();
    }
    made
}

/// Every report, by its id, as the collection keeps it.
fn reports(picture: &Picture, config: &crate::config::Config) -> BTreeMap<String, (Report, Shown)> {
    let mut reports = BTreeMap::new();
    let mut keep = |id: String, words: &str, report: Report| {
        let made = shown(&id, words, &report);
        reports.insert(id, (report, made));
    };
    for period in Period::ALL {
        for cut in Cut::ALL {
            keep(
                format!("{}/{}", period.id(), cut.id()),
                &format!("{} by {}", period.words().join(" "), cut.words().join(" ")),
                report::cut(picture, period, cut, TOP),
            );
        }
    }
    keep(
        "summary".into(),
        "today summary",
        report::summary(picture, &[]),
    );
    keep(
        "waste".into(),
        "waste cache context rebuilt",
        report::waste(picture),
    );
    keep("caps".into(), "cap caps limit", report::caps(picture));
    keep(
        "forecast".into(),
        "forecast ahead",
        report::forecast(picture),
    );
    keep(
        "quests".into(),
        "quests outcomes pull requests",
        report::quests(picture, Period::Days30),
    );
    keep(
        "outliers".into(),
        "outliers costly sessions",
        report::outliers(
            picture,
            config.detectors.false_discovery,
            config.detectors.floor,
        ),
    );
    reports
}

/// The widest cap first: it's the one a week's work runs into.
fn widest_first(caps: &[CapView]) -> Vec<&CapView> {
    let mut caps: Vec<&CapView> = caps.iter().collect();
    caps.sort_by_key(|view| std::cmp::Reverse(view.cap.window_hours));
    caps
}

/// How things stand: today, with a day's caps behind it; each longer cap,
/// the widest first, with the week ahead behind the first, since its days
/// are the week that can stop work; and the periods before, this week
/// among them. Each has its detail behind it, and with no longer cap the
/// week ahead stands on its own.
fn now(picture: &Picture, reports: &BTreeMap<String, (Report, Shown)>) -> Vec<Line> {
    let word = picture.standing.word.as_str();
    let mut now = Vec::new();
    let (today, today_basis) = figure_of(picture, Period::Today);
    let ends = picture.today_ahead().map(|(low, mid, high)| {
        format!(
            "ends near {} ({} to {})",
            whole(mid),
            whole(low),
            whole(high)
        )
    });
    let mut made = period_line(
        Period::Today,
        reports,
        joined(&[&today, word]),
        joined(&[ends.as_deref().unwrap_or(""), today_basis]),
    );
    // What's copied leads the answer to "spend" too, so it says only the
    // likeliest end of the day; its range is in the footer.
    let likeliest = picture
        .today_ahead()
        .map(|(_, mid, _)| format!(", ends near {}", whole(mid)));
    made.copy = format!(
        "Today: {}{}",
        joined(&[&today, word]),
        likeliest.unwrap_or_default()
    );
    now.push(made);
    let mut offered: Vec<&str> = Vec::new();
    let mut daily = Vec::new();
    for view in widest_first(&picture.caps) {
        let feed = view.cap.feed.as_str();
        let doors = if offered.contains(&feed) {
            Vec::new()
        } else {
            offered.push(feed);
            sign_ins(picture, feed)
        };
        let made = cap_line(picture, view, doors);
        if view.cap.window_hours <= 24 {
            daily.push(made);
        } else {
            now.push(made);
        }
    }
    now[0].rows.splice(0..0, daily);
    if let Some((low, mid, high)) = picture.week_ahead() {
        let mut made = line("ahead", "Next 7 days", format!("near {}", whole(mid)));
        made.note = likely(low, high);
        made.report = "forecast".into();
        match now.get_mut(1) {
            Some(cap) => cap.rows.insert(0, made),
            None => now.push(made),
        }
    }
    let mut earlier = line("earlier", "Earlier", "");
    earlier.note = "yesterday, this week and the months before".into();
    earlier.rows = EARLIER
        .iter()
        .map(|period| {
            let (figure, basis) = figure_of(picture, *period);
            let (from, to) = period.days(&picture.zone, picture.now);
            let days = if from == to {
                Zone::day_name(from)
            } else {
                format!("{} to {}", Zone::day_name(from), Zone::day_name(to))
            };
            let mut made = period_line(*period, reports, figure, joined(&[&days, basis]));
            // A month's quests are better told with how each turned out.
            if *period == Period::Days30
                && let Some(quest) = made
                    .rows
                    .iter_mut()
                    .find(|row| row.report.ends_with("/quest"))
            {
                quest.report = "quests".into();
                quest.note = "with how each turned out".into();
            }
            made
        })
        .collect();
    if earlier.rows.iter().any(|row| !row.rows.is_empty()) {
        now.push(earlier);
    }
    now
}

pub fn collection(
    picture: &Picture,
    findings: &[Finding],
    config: &crate::config::Config,
) -> Collection {
    let reports = reports(picture, config);
    let now = now(picture, &reports);
    let mut more = Vec::new();
    let waste_found = findings.iter().any(|finding| finding.detector == "waste");
    if let Some((report, _)) = reports.get("waste")
        && !waste_found
        && !report.rows.is_empty()
    {
        let mut made = line(
            "report:waste",
            "Context rebuilt in 7 days",
            report.shown.clone(),
        );
        made.note = "where money bought nothing".into();
        made.role = "report".into();
        made.report = "waste".into();
        more.push(made);
    }
    if let Some((report, _)) = reports.get("outliers")
        && !report.rows.is_empty()
    {
        let count = report.rows.len();
        let mut made = line(
            "report:outliers",
            // Named as the findings drawn from it are, so it reads as
            // the rest of them.
            "Heavy turns",
            if count == 1 {
                "1 session".to_owned()
            } else {
                format!("{count} sessions")
            },
        );
        made.note = format!("{} beyond the usual in 30 days", report.shown);
        made.role = "report".into();
        made.report = "outliers".into();
        more.push(made);
    }
    let mut today = line(
        "quiet:today",
        "Today",
        if picture.standing.word.is_empty() {
            "amounts hidden".to_owned()
        } else {
            picture.standing.word.clone()
        },
    );
    today.note = "amounts are hidden while the screen is shared".into();
    let mut quiet = vec![today];
    for view in widest_first(&picture.caps) {
        if let Some(allowance) = view.cap.allowance {
            quiet.push(line(
                format!("quiet:cap:{}", view.cap.name),
                capitalized(&view.cap.label),
                percent(view.spent / allowance),
            ));
        }
    }
    quiet.extend(findings.iter().map(quiet_finding));
    Collection {
        v: VERSION,
        now,
        findings: findings.iter().map(finding_line).collect(),
        more,
        quiet,
        reports: reports
            .into_iter()
            .map(|(id, (_, shown))| (id, shown))
            .collect(),
        periods: Period::ALL
            .iter()
            .map(|period| Words {
                id: period.id().into(),
                words: period.words().iter().map(|w| (*w).to_owned()).collect(),
            })
            .collect(),
        cuts: Cut::ALL
            .iter()
            .map(|cut| Words {
                id: cut.id().into(),
                words: cut.words().iter().map(|w| (*w).to_owned()).collect(),
            })
            .collect(),
        examples: [
            "spend last week by quest",
            "spend today by model",
            "spend this month by agent",
            "spend waste",
            "spend caps",
        ]
        .map(str::to_owned)
        .to_vec(),
    }
}

/// The collection while a newer spend-watch keeps the history: one row
/// saying so, the same while the screen is shared, since it holds no
/// amount, and no reports to step into, since none were read, but the
/// summary that answers "spend" at home, which says so too.
pub fn held(shape: i64) -> Collection {
    let mut made = line("held", "Watch on hold", "newer history");
    made.note = "a newer spend-watch keeps it; run that one".into();
    made.copy = format!(
        "spend-watch is on hold: a newer one keeps its history, at shape {shape} \
         where this one reads up to {}",
        crate::history::SCHEMA
    );
    made.role = "warning".into();
    let summary = Shown {
        id: "summary".into(),
        title: "AI spend".into(),
        headline: "On hold: a newer spend-watch keeps the history".into(),
        rows: Vec::new(),
        markdown: format!("{}\n", made.copy),
        words: String::new(),
    };
    Collection {
        v: VERSION,
        now: Vec::new(),
        findings: vec![made.clone()],
        more: Vec::new(),
        quiet: vec![made],
        reports: BTreeMap::from([("summary".to_owned(), summary)]),
        periods: Vec::new(),
        cuts: Vec::new(),
        examples: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::History;
    use crate::time::HOUR;

    #[test]
    fn a_held_watch_says_why_and_nothing_else() {
        let held = held(crate::history::SCHEMA + 1);
        assert_eq!(held.findings.len(), 1);
        assert_eq!(held.quiet, held.findings);
        let row = &held.findings[0];
        assert_eq!(
            (row.role.as_str(), row.name.as_str()),
            ("warning", "Watch on hold")
        );
        assert!(row.note.len() <= 45, "{}", row.note);
        assert!(row.copy.contains("a newer one"), "{}", row.copy);
        assert!(held.now.is_empty());
        let ids: Vec<_> = held.reports.keys().collect();
        assert_eq!(ids, ["summary"]);
        assert!(!held.reports["summary"].headline.contains('$'));
    }

    fn config() -> crate::config::Config {
        serde_json::from_value(serde_json::json!({
            "sessions": "/none", "runs": "/none", "registry": "/none", "quests": "/none",
            "state": "/none", "zone": "UTC",
        }))
        .unwrap()
    }

    const NOW: f64 = 1_791_988_200.0; // 2026-10-14T14:30:00Z, a Wednesday

    fn store(history: &History, line: serde_json::Value, at: f64) {
        let record = crate::feed::parse(&line.to_string()).unwrap();
        crate::feed::store(history, "proxy", &record, at).unwrap();
    }

    /// A machine's history as the proxy leaves it: a month billed hour by
    /// hour across models, a weekly cap lowered, a day's cap that resets,
    /// an optional source with its sign-in and one that's been down a
    /// while, with names long enough to crowd a row.
    fn busy() -> History {
        let history = History::memory();
        let models = [
            "claude-opus-4-5-a-very-long-model-name-indeed",
            "gpt-5-codex",
            "gemini-2.5-pro",
        ];
        let mut hour = crate::time::hour_start(crate::time::hour_of(NOW - 40.0 * 86400.0));
        let mut turn = 0;
        while hour < NOW {
            turn += 1;
            store(
                &history,
                serde_json::json!({
                    "v": 1, "kind": "billed", "source": "warehouse",
                    "hour": crate::time::rfc3339(hour), "tool": "pi",
                    "model": models[turn % 3], "vendor": "v",
                    "session": format!("0199-session-{}", turn % 5),
                    "dollars": 20.0 + (turn % 7) as f64 * 9.0,
                    "tokens": {"input": 0, "cached": 0, "cacheWrite": 0, "output": 0},
                }),
                hour,
            );
            hour += HOUR;
        }
        store(
            &history,
            serde_json::json!({
                "v": 1, "kind": "settled", "source": "warehouse",
                "through": crate::time::rfc3339(hour),
            }),
            NOW,
        );
        store(
            &history,
            serde_json::json!({
                "v": 1, "kind": "cap", "name": "weekly", "label": "weekly cap",
                "allowance": 15000, "windowHours": 168, "spent": 14100, "warn": 0.9,
                "state": "notified", "at": "2026-10-14T14:00:00Z",
            }),
            NOW - 1800.0,
        );
        store(
            &history,
            serde_json::json!({
                "v": 1, "kind": "cap", "name": "weekly", "label": "weekly cap",
                "allowance": 4000, "windowHours": 168, "spent": 14200, "warn": 0.9,
                "fallback": 4000, "state": "refused", "at": "2026-10-14T14:25:00Z",
            }),
            NOW - 300.0,
        );
        store(
            &history,
            serde_json::json!({
                "v": 1, "kind": "cap", "name": "premium", "label": "premium day",
                "allowance": 1000, "windowHours": 24, "spent": 600, "warn": 0.5,
                "state": "notified", "resets": "2026-10-15T04:00:00Z",
                "at": "2026-10-14T14:29:00Z",
            }),
            NOW - 60.0,
        );
        store(
            &history,
            serde_json::json!({
                "v": 1, "kind": "action", "id": "drive", "name": "Sign in to Drive",
                "run": ["true"], "fits": ["sign-in"],
            }),
            NOW,
        );
        store(
            &history,
            serde_json::json!({
                "v": 1, "kind": "health", "source": "overrides", "state": "optional",
                "message": "needs Drive access to read an override's end", "signIn": "drive",
            }),
            NOW,
        );
        let failed = "the warehouse couldn't be reached: io: failed to lookup address \
                      information: Name or service not known";
        for at in [NOW - 3600.0, NOW] {
            store(
                &history,
                serde_json::json!({
                    "v": 1, "kind": "health", "source": "warehouse", "state": "error",
                    "message": failed,
                }),
                at,
            );
        }
        history
    }

    fn every(lines: &[Line]) -> Vec<&Line> {
        let mut all = Vec::new();
        for line in lines {
            all.push(line);
            all.extend(every(&line.rows));
        }
        all
    }

    fn collected() -> (Collection, Vec<Finding>) {
        let history = busy();
        let config = config();
        let picture = Picture::read(&history, &config, NOW).unwrap();
        let mut found = crate::detect::all(&picture, &history, &config).unwrap();
        crate::findings::sort(&mut found);
        (collection(&picture, &found, &config), found)
    }

    // A row's side is a few words, never a sentence: past about a third of
    // a row it would run into the name. That's every row the collection
    // has, at every depth, and the rows of every report.
    #[test]
    fn every_side_is_a_few_words() {
        let (collected, _) = collected();
        let mut lines = every(&collected.now);
        lines.extend(every(&collected.findings));
        lines.extend(every(&collected.more));
        lines.extend(every(&collected.quiet));
        for shown in collected.reports.values() {
            lines.extend(every(&shown.rows));
        }
        assert!(lines.len() > 100, "{}", lines.len());
        for line in lines {
            assert!(
                line.side.chars().count() <= 32,
                "{}: {:?} beside {:?}",
                line.key,
                line.side,
                line.name
            );
        }
    }

    // The first screen holds how things stand and what was found: today,
    // each cap longer than a day and Earlier, then the findings, with every
    // report behind the row it belongs to. A day's cap is behind today, the
    // week ahead behind the week's cap, and this week among the periods
    // before, since the cap's own seven days are the week that matters.
    #[test]
    fn the_first_screen_is_short_and_says_each_thing_once() {
        let (collected, found) = collected();
        let names: Vec<&str> = collected
            .now
            .iter()
            .map(|line| line.name.as_str())
            .collect();
        assert_eq!(names, ["Today", "Weekly cap", "Earlier"]);
        assert_eq!(collected.findings.len(), found.len());
        assert!(collected.more.len() <= 2, "{:?}", collected.more);
        // The cap's figure is its share; its dollars are in the footer.
        let weekly = &collected.now[1];
        assert!(weekly.side.ends_with("of $4,000"), "{weekly:?}");
        assert!(weekly.note.contains("the last 7 days"), "{weekly:?}");
        // The week ahead is behind it, then its change, and the optional
        // sign-in too.
        let behind: Vec<&str> = weekly.rows.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(
            behind,
            [
                "Next 7 days",
                "Override lapsed",
                "Refusing",
                "Sign in to Drive"
            ]
        );
        assert_eq!(weekly.rows[2].note, "with $14,200 spent");
        // The premium day leads what's behind today, before its cuts.
        assert_eq!(collected.now[0].rows[0].name, "Premium day");
        // This week is the first period after yesterday.
        let earlier: Vec<&str> = collected.now[2]
            .rows
            .iter()
            .map(|row| row.name.as_str())
            .collect();
        assert_eq!(earlier[..2], ["Yesterday", "This week"]);
        // A period's cuts are behind it, after a day's caps, each standing
        // for its report.
        let today = &collected.now[0];
        assert!(today.rows.iter().any(|row| row.name == "By model"));
        assert!(
            today
                .rows
                .iter()
                .filter(|row| !row.key.starts_with("cap:"))
                .all(|row| collected.reports.contains_key(&row.report))
        );
        assert!(!today.rows.iter().any(|row| row.name == "By day"));
        // No row says where its figures came from beside them.
        for line in every(&collected.now) {
            assert!(!line.side.contains("billed"), "{line:?}");
        }
    }

    // A source down an hour is a note saying since when, in a few words;
    // the error underneath it is in what's copied.
    #[test]
    fn a_source_down_is_one_quiet_row() {
        let (collected, _) = collected();
        let down = collected
            .findings
            .iter()
            .find(|line| line.key == "finding:health:proxy:warehouse")
            .expect("the warehouse's row");
        assert_eq!(down.name, "The warehouse can't be read");
        assert_eq!(down.side, "since 13:30");
        assert_eq!(down.note, "the warehouse couldn't be reached");
        assert_eq!(down.role, "found");
        assert!(down.copy.contains("Name or service not known"));
    }

    // While the screen is shared nothing says an amount, at any depth.
    #[test]
    fn a_shared_screen_shows_no_amount() {
        let (collected, _) = collected();
        assert!(collected.quiet.len() >= 3, "{:?}", collected.quiet);
        for line in every(&collected.quiet) {
            for said in [&line.name, &line.side, &line.note, &line.copy] {
                assert!(!said.contains('$'), "{said}");
            }
        }
    }
}
