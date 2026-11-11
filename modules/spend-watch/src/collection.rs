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
use crate::standing::Standing;
use crate::status::{action_kind, capitalized, unpriced};
use crate::time::Zone;
use serde::Serialize;
use std::collections::BTreeMap;

pub const VERSION: u32 = 2;

/// How many of a cut's rows are shown before the rest are one row.
const TOP: usize = 12;

/// The periods before today, most recent first, each a row of the first
/// screen after the caps, so none is more than a step from it.
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
    /// The theme's mark its glyph is, by name, for a row whose role colours
    /// a mark of its own rather than choosing one: a budget's, a window's,
    /// a day's.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub mark: String,
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
    /// The budget or window it stands for, by its key in budgets.json, so
    /// a cap's row and its window's open one drawer.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub budget: String,
    /// What its drawer says that its row and footer can't: a period's days
    /// and facts, a budget's windows, a window's facts (drawers.rs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drawn: Option<Drawn>,
}

/// A drawer's words, every one ready to draw: what it's of, above its
/// title, and its facts in order. Its title is its row's name unless it
/// says one; `names` is whether the eyebrow names something from the
/// world, so it's set as it's written rather than in Label's capitals.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Drawn {
    pub eyebrow: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub title: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub names: bool,
    pub facts: Vec<Fact>,
}

/// One of a drawer's facts: the vocabulary's id for its name (`spend:` and
/// it, in the launcher's words), or a name from the world, a window's, as
/// its `label`; what it says, its meter where it has one, and whether
/// that's words rather than a name or a figure, so it's set in the words
/// face.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Fact {
    pub id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub label: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub figure: Option<Figure>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub words: bool,
}

/// A fact's meter, as the launcher's Meter draws it: its shape, its
/// values from nothing to full, and whether it's past where it warns.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Figure {
    pub shape: &'static str,
    pub values: Vec<f64>,
    pub warn: bool,
}

impl Fact {
    pub(crate) fn of(id: impl Into<String>, value: impl Into<String>, words: bool) -> Fact {
        Fact {
            id: id.into(),
            label: String::new(),
            value: value.into(),
            figure: None,
            words,
        }
    }
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
    /// What the collection shows while the screen is shared: each finding
    /// that wants a look, how today stands, each cap's share and the rest
    /// of what was found, with no amount in any.
    pub quiet: Vec<Line>,
    /// The keys of the lines the launcher lists, in its order (`ranked`),
    /// so the shell places each row without ranking any.
    pub shown: Vec<String>,
    /// The keys of the rows the summary answers "spend" with, in order.
    pub summary_rows: Vec<String>,
    /// Each report by its id, "<period>/<cut>" or a focused one's name,
    /// for the rows that stand for one and the answers to what's typed.
    pub reports: BTreeMap<String, Shown>,
    /// What each pi session spent today, by its whole id, for the
    /// launcher's Agents to say beside it. While the screen is shared the
    /// shell shows none of it.
    pub sessions: BTreeMap<String, String>,
    /// How today stands on AI spend's own row, wherever it's listed: its
    /// dollars with their period, as All AI Spend says them, and how that
    /// is against usual, "$412 today" (VKCF3Z, 2026-11-08). While the
    /// screen is shared the shell says the first quiet line's instead.
    pub today: String,
    /// What every pi session together spent today, said as each of
    /// `sessions` is and empty when nothing was, for the launcher's Agents
    /// to say on its home row. While the screen is shared the shell shows
    /// none of it.
    pub agents_today: String,
    /// How each pi session's day has gone, by its whole id, for those in
    /// `sessions`: its spend by hour as a spark the shell draws, which
    /// says no amount, and what rebuilding its context cost, which the
    /// shell hides while the screen is shared.
    pub sessions_today: BTreeMap<String, report::SessionDay>,
    /// What each pi session's councils, judges and fleets cost today, by
    /// its whole id, costliest first; part of what it spent.
    pub helpers: BTreeMap<String, Vec<report::Helper>>,
    /// What each quest and the quests under it spent today, by the
    /// quest's id, for the launcher's Agents to say beside an
    /// orchestrator: its whole orchestration's, its own and its
    /// sessions'. While the screen is shared the shell shows none of it.
    pub families: BTreeMap<String, report::Family>,
    pub periods: Vec<Words>,
    pub cuts: Vec<Words>,
    pub examples: Vec<String>,
}

/// Whether a line is a finding that wants a look, which comes first.
fn asks(line: &Line) -> bool {
    line.role == "alert" || line.role == "warning"
}

/// The lines with each that wants a look first, the rest as they were.
fn asking_first(lines: Vec<Line>) -> Vec<Line> {
    let (asking, rest): (Vec<Line>, Vec<Line>) = lines.into_iter().partition(asks);
    asking.into_iter().chain(rest).collect()
}

/// The keys of the lines the launcher lists, in its order: each finding
/// that wants a look first, under the cursor as the collection opens,
/// then how things stand, the rest of what was found and the reports
/// behind no other row. And the summary's rows, which leave out today's,
/// whose words are its headline, and those reports.
fn ranked(now: &[Line], findings: &[Line], more: &[Line]) -> (Vec<String>, Vec<String>) {
    let key = |line: &Line| line.key.clone();
    let asking: Vec<String> = findings.iter().filter(|line| asks(line)).map(key).collect();
    let rest: Vec<String> = findings
        .iter()
        .filter(|line| !asks(line))
        .map(key)
        .collect();
    let stand: Vec<String> = now.iter().map(key).collect();
    let behind: Vec<String> = more.iter().map(key).collect();
    let summary = [&asking[..], stand.get(1..).unwrap_or_default(), &rest[..]].concat();
    ([asking, stand, rest, behind].concat(), summary)
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
        mark: String::new(),
        run: None,
        verb: String::new(),
        expect: None,
        report: String::new(),
        rows: Vec::new(),
        budget: String::new(),
        drawn: None,
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
    let side = if percent(row.share) == "0%" {
        figure.clone()
    } else {
        format!("{figure} · {}", percent(row.share))
    };
    let mut made = line(key, row.name.clone(), side);
    let rest = row
        .shown
        .strip_prefix(&format!("{figure} · "))
        .unwrap_or(&row.shown);
    if rest != figure {
        rest.clone_into(&mut made.note);
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
    id.clone_into(&mut made.report);
    Some(made)
}

/// A period's row: its spend beside it, where that came from below, its
/// cuts behind it, and its drawer.
fn period_line(
    picture: &Picture,
    period: Period,
    reports: &BTreeMap<String, (Report, Shown)>,
    (side, note): (String, String),
) -> Line {
    let mut made = line(
        format!("period:{}", period.id()),
        capitalized(period.name()),
        side,
    );
    made.note = note;
    // A day's row wears a day's mark, so the first screen's glyph column
    // has no hole beside the budgets' (VKCF3Z, 2026-11-07).
    if matches!(period, Period::Today | Period::Yesterday) {
        "date".clone_into(&mut made.mark);
    }
    made.rows = cuts_for(period)
        .filter_map(|cut| {
            let id = format!("{}/{}", period.id(), cut.id());
            cut_line(&id, cut, &reports.get(&id)?.0)
        })
        .collect();
    made.drawn = Some(period_drawn(picture, period, reports, false));
    made
}

/// A period's drawer: its days above its name, then how the day is likely
/// to end while it runs, where its figure was read from, what went on
/// rebuilding context, and the largest of each of its cuts (VKCF3Z,
/// 2026-11-07). Quiet, while the screen is shared, it says no amount and
/// keeps the shares.
fn period_drawn(
    picture: &Picture,
    period: Period,
    reports: &BTreeMap<String, (Report, Shown)>,
    quiet: bool,
) -> Drawn {
    let (from, to) = period.days(&picture.zone, picture.now);
    let pieces = picture.between(from, to);
    let mut facts = Vec::new();
    let ends = if period == Period::Today && !quiet {
        picture.today_ahead()
    } else {
        None
    };
    if let Some((low, mid, high)) = ends {
        let likely = likely(low, high);
        facts.push(Fact::of(
            "ends",
            format!("{} \u{b7} {likely}", whole(mid)),
            false,
        ));
    }
    let read = read_from(pieces);
    if !read.is_empty() {
        facts.push(Fact::of("from", read, true));
    }
    if let Some(share) = picture
        .rebuilt
        .get(period.id())
        .filter(|share| **share >= 0.005)
    {
        let spent = dollars(share * series::total(pieces));
        let said = if quiet {
            percent(*share)
        } else {
            format!("{spent} \u{b7} {}", percent(*share))
        };
        facts.push(Fact::of("rebuilding", said, false));
    }
    facts.extend(cuts_for(period).filter_map(|cut| {
        let (report, _) = reports.get(&format!("{}/{}", period.id(), cut.id()))?;
        most(cut, report)
    }));
    // The eyebrow is where, its days, then what kind of thing it is, as
    // every drawer's is (VKCF3Z, 2026-11-08).
    Drawn {
        eyebrow: joined(&[&days_of(picture, period), period.kind()]),
        title: String::new(),
        names: false,
        facts,
    }
}

/// The largest of a period's cut, by its share: "all" when it's the only
/// one, rather than a hundred percent. A quest's title, a day and a
/// budget's name are words; a model's, a repo's and the rest are names.
fn most(cut: Cut, report: &Report) -> Option<Fact> {
    let top = report.rows.first()?;
    let count = report.rows.len() + report.rest.map_or(0, |(count, _)| count);
    let share = if count == 1 {
        "all".to_owned()
    } else {
        percent(top.share)
    };
    let words = matches!(cut, Cut::Quest | Cut::Day | Cut::Budget);
    Some(Fact::of(
        format!("most-{}", cut.id()),
        format!("{} \u{b7} {share}", top.name),
        words,
    ))
}

/// A period's days as a person says them: one day, or its first to its
/// last.
fn days_of(picture: &Picture, period: Period) -> String {
    let (from, to) = period.days(&picture.zone, picture.now);
    if from == to {
        Zone::day_name(from)
    } else {
        format!("{} to {}", Zone::day_name(from), Zone::day_name(to))
    }
}

/// Where a period's figure was read from, each kind of source it holds:
/// a feed's bill, pi's logs where the bill hasn't come, and pi's logs at
/// list prices where there's no bill at all. Nothing spent, nothing said.
fn read_from(pieces: &[series::Piece]) -> String {
    let held = |sources: &[Source]| {
        pieces
            .iter()
            .any(|piece| piece.dollars > 0.005 && sources.contains(&piece.source))
    };
    let billed = held(&[Source::Warehouse, Source::Meter]);
    let estimated = match (held(&[Source::Estimated]), billed) {
        (false, _) => "",
        (true, true) => "pi's logs where it hasn't come",
        (true, false) => "pi's logs where the bill hasn't come",
    };
    let list = if held(&[Source::List]) {
        "pi's logs at list prices"
    } else {
        ""
    };
    joined(&[if billed { "the bill" } else { "" }, estimated, list])
}

/// How a period's spend reads, and where it came from.
fn figure_of(picture: &Picture, period: Period) -> (String, &'static str) {
    let (from, to) = period.days(&picture.zone, picture.now);
    let pieces = picture.between(from, to);
    (
        maybe_about(dollars(series::total(pieces)), series::estimated(pieces)),
        basis(series::weakest(pieces), picture.fed),
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
    // Its side is its share alone, as a window's is, since its drawer's
    // Standing says it in dollars (VKCF3Z, 2026-11-08).
    let side = match (view.used, cap.allowance) {
        (Some(used), _) => percent(used),
        (None, Some(allowance)) => percent(view.spent / allowance),
        (None, None) => format!("{} spent", whole(view.spent)),
    };
    let mut made = line(format!("cap:{}", cap.name), capitalized(&cap.label), side);
    made.budget = window_key(view);
    // A cap is a window reached from another row, so it wears a window's
    // mark, and its note keeps only what its window's drawer lacks: when
    // it resets and when an override ends are said once, there (VKCF3Z,
    // 2026-11-08).
    CLOCK.clone_into(&mut made.mark);
    made.note = match (view.used, view.resets) {
        (Some(_), Some(_)) => String::new(),
        (Some(_), None) => format!("of {}", window(cap.window_hours)),
        (None, Some(_)) => format!("{} spent", whole(view.spent)),
        (None, None) => format!("{} in {}", whole(view.spent), window(cap.window_hours)),
    };
    made.copy = report::cap_line(picture, view);
    let mut events: Vec<&crate::caps::Event> = picture
        .events
        .iter()
        .filter(|event| {
            event.feed == cap.feed && event.budget == cap.budget && event.name == cap.name
        })
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
                format!(
                    "cap:{}:{}:{}",
                    cap.name,
                    event.kind,
                    crate::time::whole_seconds(event.at)
                ),
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

/// The key in budgets.json of the window a cap is, so its row opens the
/// drawer its window's row under Budgets does.
fn window_key(view: &CapView) -> String {
    format!("budget:{}:{}", view.budget, view.cap.name)
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
            made.note.clone_from(&source.message);
            made.role = "sign-in".into();
            made.run = Some(action.run.clone());
            made.verb = "sign in".into();
            Some(made)
        })
        .collect()
}

fn verb(action: &crate::findings::Action) -> &'static str {
    match action_kind(action) {
        // Return takes Joel to the session, so it says where it goes
        // (VKCF3Z, 2026-11-07).
        "go" => "go to",
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
    made.note.clone_from(&finding.note);
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

/// The drawer of the window a cap is, as its row under Budgets draws it,
/// so either row opens the same one. Quiet, it says no dollars.
fn cap_drawn(standing: &Standing, view: &CapView, quiet: bool) -> Option<Drawn> {
    let budget = standing
        .budgets
        .iter()
        .find(|budget| budget.name == view.budget)?;
    let allowance = budget
        .allowances
        .iter()
        .find(|one| one.name == view.cap.name)?;
    Some(crate::drawers::window(budget, allowance, quiet))
}

/// The theme's marks the budgets' rows wear: AI spend's, for a budget and
/// the whole, and a clock, for a window.
const AGENT: &str = "agent";
const CLOCK: &str = "clock";

/// The Budgets row: the month's whole in billed spend, and behind it that
/// whole, then each budget nearest its allowance first, standing in its
/// own unit, with its allowances behind it when it has more than one.
fn budgets_line(picture: &Picture, standing: &Standing) -> Line {
    let total = &standing.total.periods;
    let month = format!("{} this month", total["this month"].shown);
    let mut made = line("budgets", "Budgets", month);
    made.note = crate::standing::WORDS.into();
    // Every row of the budgets wears AI spend's own mark, a window a clock,
    // its role colouring it (VKCF3Z, 2026-11-07).
    AGENT.clone_into(&mut made.mark);
    // The month's whole is the Budgets row's own figure, so the whole's
    // row behind it says the nearer periods rather than saying it again.
    let mut all = line(
        "budgets/all",
        "All AI Spend",
        format!("{} today", total["today"].shown),
    );
    all.note = joined(&[
        crate::standing::WORDS,
        &format!("{} in 7 days", total["7 days"].shown),
    ]);
    AGENT.clone_into(&mut all.mark);
    let budgets: Vec<Line> = standing
        .budgets
        .iter()
        .map(|budget| budget_line(picture, budget))
        .collect();
    // The row's glyph is its nearest budget's, so the first screen shows a
    // budget past its warning without stepping in.
    made.role = worst_role(budgets.iter().map(|row| row.role.as_str())).into();
    made.rows = std::iter::once(all).chain(budgets).collect();
    made
}

/// A budget's row: its standing beside it, what it is below, and each of
/// its allowances behind it when it has more than one.
fn budget_line(picture: &Picture, budget: &crate::standing::Budget) -> Line {
    let mut made = line(
        format!("budget:{}", budget.name),
        budget.label.clone(),
        budget.standing.clone(),
    );
    made.key.clone_into(&mut made.budget);
    made.drawn = Some(crate::drawers::budget(budget));
    let default = if budget.default { "the default" } else { "" };
    let resets = match budget.allowances.as_slice() {
        [one] => allowance_reset(picture, one),
        _ => String::new(),
    };
    // A note gives way from its end, so it's written most needed first:
    // whether it's the default, what Joel acts on, and when its one
    // window resets. Whose money it is is its drawer's eyebrow, so the
    // note doesn't say it again (VKCF3Z, 2026-11-08).
    made.note = joined(&[default, &resets]);
    AGENT.clone_into(&mut made.mark);
    let nearest = budget
        .allowances
        .iter()
        .find(|allowance| allowance.nearest.is_some());
    made.role = allowance_role(nearest).into();
    if let [one] = budget.allowances.as_slice() {
        opens_record(picture, &budget.name, one, &mut made);
    }
    if budget.allowances.len() > 1 {
        made.rows = budget
            .allowances
            .iter()
            .map(|allowance| {
                let name = capitalized(&allowance.label);
                // Behind its budget a share's row is named for its window,
                // so its figure is the share alone.
                let (standing, read) = match (allowance.unit, allowance.used) {
                    ("share", Some(used)) => (percent(used), true),
                    _ if allowance.standing.is_empty() => ("not read yet".to_owned(), false),
                    _ => (allowance.standing.clone(), true),
                };
                let mut row = line(
                    format!("budget:{}:{}", budget.name, allowance.name),
                    name.clone(),
                    standing,
                );
                if !read {
                    row.copy = format!("{name} has no reading yet, so there's nothing to copy");
                }
                // A window's row says its standing alone: when it resets
                // is its drawer's, so no fact there repeats the row
                // (VKCF3Z, 2026-11-08).
                row.key.clone_into(&mut row.budget);
                row.drawn = Some(crate::drawers::window(budget, allowance, false));
                row.role = allowance_role(Some(allowance)).into();
                CLOCK.clone_into(&mut row.mark);
                opens_record(picture, &budget.name, allowance, &mut row);
                row
            })
            .collect();
    }
    made
}

/// While an allowance is over, Return on its row opens what every source
/// said as it went over, so where to read it is where it's seen.
fn opens_record(
    picture: &Picture,
    budget: &str,
    allowance: &crate::standing::Allowance,
    row: &mut Line,
) {
    if let Some(crossing) = picture
        .crossings
        .iter()
        .find(|crossing| crossing.budget == budget && crossing.allowance == allowance.name)
    {
        row.run = Some(vec![
            "xdg-open".into(),
            crossing.path.to_string_lossy().into_owned(),
        ]);
        row.verb = "open".into();
    }
}

/// When an allowance resets, as its footer says it.
fn allowance_reset(picture: &Picture, allowance: &crate::standing::Allowance) -> String {
    allowance
        .resets
        .as_deref()
        .and_then(crate::time::parse)
        .map_or_else(String::new, |at| {
            format!("resets {}", picture.zone.clock(at, picture.now))
        })
}

/// The most urgent of some rows' roles: an alert over a warning over
/// plain text.
fn worst_role<'a>(roles: impl Iterator<Item = &'a str>) -> &'static str {
    roles.fold("text", |worst, role| match (worst, role) {
        ("alert", _) | (_, "alert") => "alert",
        ("warning", _) | (_, "warning") => "warning",
        _ => "text",
    })
}

/// An allowance's glyph is coloured only past its warning: a warning
/// there, an alert once it's reached or refusing.
fn allowance_role(allowance: Option<&crate::standing::Allowance>) -> &'static str {
    let Some(allowance) = allowance else {
        return "text";
    };
    if crate::standing::over(allowance) {
        "alert"
    } else if allowance.nearest.unwrap_or(0.0) >= allowance.warn {
        "warning"
    } else {
        "text"
    }
}

/// How things stand: today, with a day's caps behind it; each longer cap,
/// the widest first, with the week ahead behind the first, since its days
/// are the week that can stop work; the budgets, where there's more than
/// one; and the periods before, this week among them. Each has its detail
/// behind it, and with no longer cap the week ahead stands on its own.
fn now(
    picture: &Picture,
    reports: &BTreeMap<String, (Report, Shown)>,
    standing: &Standing,
) -> Vec<Line> {
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
        picture,
        Period::Today,
        reports,
        (
            joined(&[&today, word]),
            joined(&[ends.as_deref().unwrap_or(""), today_basis]),
        ),
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
        let mut made = cap_line(picture, view, doors);
        made.drawn = cap_drawn(standing, view, false);
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
    // One budget is the whole, which the periods already say.
    if standing.budgets.len() > 1 {
        now.push(budgets_line(picture, standing));
    }
    let earlier: Vec<Line> = EARLIER
        .iter()
        .map(|period| {
            let (figure, basis) = figure_of(picture, *period);
            let days = days_of(picture, *period);
            let note = joined(&[&days, basis]);
            let mut made = period_line(picture, *period, reports, (figure, note));
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
    // Joel, 2026-10-26: no path deeper than three, so the periods sit in
    // the collection itself rather than behind a row of their own.
    if earlier.iter().any(|row| !row.rows.is_empty()) {
        now.extend(earlier);
    }
    now
}

pub fn collection(
    picture: &Picture,
    findings: &[Finding],
    config: &crate::config::Config,
) -> Collection {
    let reports = reports(picture, config);
    let standing = crate::standing::standing(picture, config, None);
    let now = now(picture, &reports, &standing);
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
    let quiet = asking_first(quiet(picture, &reports, findings, &standing));
    let findings: Vec<Line> = findings.iter().map(finding_line).collect();
    let (shown, summary_rows) = ranked(&now, &findings, &more);
    Collection {
        v: VERSION,
        now,
        findings,
        more,
        quiet,
        shown,
        summary_rows,
        reports: reports
            .into_iter()
            .map(|(id, (_, shown))| (id, shown))
            .collect(),
        sessions: report::by_session(picture, Period::Today),
        today: today_said(picture),
        agents_today: report::by_sessions(picture, Period::Today),
        sessions_today: report::today_by_session(picture),
        helpers: report::helpers_by_session(picture, Period::Today),
        families: report::by_family(picture, Period::Today),
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

/// Today's dollars with their period and how they stand against usual,
/// for AI spend's own row: "$412 today", or "$412 today \u{b7} above
/// usual".
fn today_said(picture: &Picture) -> String {
    let (today, _) = figure_of(picture, Period::Today);
    joined(&[&format!("{today} today"), &picture.standing.word])
}

/// The first screen's rows, since nothing is behind a quiet one: today's
/// standing, each window longer than a day, then the findings, where a
/// day's cap that matters is. Today's drawer and a window's say what they
/// can without an amount.
fn quiet(
    picture: &Picture,
    reports: &BTreeMap<String, (Report, Shown)>,
    findings: &[Finding],
    standing: &Standing,
) -> Vec<Line> {
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
    today.drawn = Some(period_drawn(picture, Period::Today, reports, true));
    let mut quiet = vec![today];
    for view in widest_first(&picture.caps) {
        if view.cap.window_hours <= 24 {
            continue;
        }
        let share = view
            .used
            .or_else(|| view.cap.allowance.map(|allowance| view.spent / allowance));
        if let Some(share) = share {
            let mut made = line(
                format!("quiet:cap:{}", view.cap.name),
                capitalized(&view.cap.label),
                percent(share),
            );
            made.budget = window_key(view);
            made.drawn = cap_drawn(standing, view, true);
            quiet.push(made);
        }
    }
    quiet.extend(findings.iter().map(quiet_finding));
    quiet
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
    let (shown, summary_rows) = ranked(&[], std::slice::from_ref(&made), &[]);
    Collection {
        v: VERSION,
        now: Vec::new(),
        findings: vec![made.clone()],
        more: Vec::new(),
        quiet: vec![made],
        shown,
        summary_rows,
        reports: BTreeMap::from([("summary".to_owned(), summary)]),
        sessions: BTreeMap::new(),
        today: String::new(),
        agents_today: String::new(),
        sessions_today: BTreeMap::new(),
        helpers: BTreeMap::new(),
        families: BTreeMap::new(),
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
        assert_eq!(held.now, [] as [crate::collection::Line; 0]);
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

    fn store(history: &History, line: &serde_json::Value, at: f64) {
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
                &serde_json::json!({
                    "v": 1, "kind": "billed", "source": "warehouse",
                    "hour": crate::time::rfc3339(hour), "tool": "pi",
                    "model": models[turn % 3], "vendor": "v",
                    "session": format!("0199-session-{}", turn % 5),
                    "dollars": 20.0 + crate::stats::count(turn % 7) * 9.0,
                    "tokens": {"input": 0, "cached": 0, "cacheWrite": 0, "output": 0},
                }),
                hour,
            );
            hour += HOUR;
        }
        store(
            &history,
            &serde_json::json!({
                "v": 1, "kind": "settled", "source": "warehouse",
                "through": crate::time::rfc3339(hour),
            }),
            NOW,
        );
        store(
            &history,
            &serde_json::json!({
                "v": 1, "kind": "cap", "name": "weekly", "label": "Week",
                "allowance": 15000, "windowHours": 168, "spent": 14100, "warn": 0.9,
                "state": "notified", "at": "2026-10-14T14:00:00Z",
            }),
            NOW - 1800.0,
        );
        store(
            &history,
            &serde_json::json!({
                "v": 1, "kind": "cap", "name": "weekly", "label": "Week",
                "allowance": 4000, "windowHours": 168, "spent": 14200, "warn": 0.9,
                "fallback": 4000, "state": "refused", "at": "2026-10-14T14:25:00Z",
            }),
            NOW - 300.0,
        );
        store(
            &history,
            &serde_json::json!({
                "v": 1, "kind": "cap", "name": "premium", "label": "Premium Day",
                "allowance": 1000, "windowHours": 24, "spent": 600, "warn": 0.5,
                "state": "notified", "resets": "2026-10-15T04:00:00Z",
                "at": "2026-10-14T14:29:00Z",
            }),
            NOW - 60.0,
        );
        store(
            &history,
            &serde_json::json!({
                "v": 1, "kind": "action", "id": "drive", "name": "Sign in to Drive",
                "run": ["true"], "fits": ["sign-in"],
            }),
            NOW,
        );
        store(
            &history,
            &serde_json::json!({
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
                &serde_json::json!({
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
    // each cap longer than a day and the periods before, then the findings,
    // with every report behind the row it belongs to. A day's cap is behind
    // today, and the week ahead behind the week's cap.
    #[test]
    fn the_first_screen_is_short_and_says_each_thing_once() {
        let (collected, found) = collected();
        let names: Vec<&str> = collected
            .now
            .iter()
            .map(|line| line.name.as_str())
            .collect();
        assert_eq!(
            names,
            [
                "Today",
                "Week",
                "Yesterday",
                "This week",
                "Last week",
                "This month",
                "Last month",
                "The last 30 days"
            ]
        );
        assert_eq!(collected.findings.len(), found.len());
        assert!(collected.more.len() <= 2, "{:?}", collected.more);
        // The cap's figure is its share; its dollars are in the footer.
        let weekly = &collected.now[1];
        assert!(weekly.side.ends_with('%'), "{weekly:?}");
        // Its note says its spend alone, and it wears a window's clock: when
        // it resets or an override ends is its window's drawer's to say.
        assert!(weekly.note.ends_with("in the last 7 days"), "{weekly:?}");
        assert_eq!(weekly.mark, "clock");
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
        assert_eq!(collected.now[0].rows[0].name, "Premium Day");
        // While sharing there's nothing behind a row, so the quiet lines
        // are the first screen's, and a day's cap that matters is a
        // finding among them, after those that want a look.
        let quiet: Vec<&str> = collected
            .quiet
            .iter()
            .skip_while(|line| line.key.starts_with("quiet:finding:"))
            .take_while(|line| !line.key.starts_with("quiet:finding:"))
            .map(|line| line.name.as_str())
            .collect();
        assert_eq!(quiet, ["Today", "Week"]);
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

    // Each pi session that spent today is there by its whole id, even two
    // whose ids start alike, as the report's names cut them, with what it
    // spent today and nothing from before.
    #[test]
    fn each_session_says_what_it_spent_today() {
        use crate::logs::{Cost, Event, Kind, Tokens, Turn};
        let history = History::memory();
        let turns = [
            ("01a1271f-aaaa", NOW - 3600.0, 12.0),
            ("01a1271f-aaaa", NOW - 1800.0, 0.5),
            ("01a1271f-bbbb", NOW - 600.0, 3.25),
            ("01a1271f-cccc", NOW - crate::time::DAY, 40.0),
        ];
        for (session, at, total) in turns {
            history
                .add_event(&Event::Turn(Turn {
                    session: session.into(),
                    at,
                    model: "m".into(),
                    provider: "p".into(),
                    kind: Kind::Agent,
                    tokens: Tokens::default(),
                    cost: Cost {
                        total,
                        ..Cost::default()
                    },
                    quest: None,
                    budget: None,
                    gap: None,
                    rebuilt: 0.0,
                    rebuilt_cost: 0.0,
                    cause: None,
                }))
                .unwrap();
        }
        let config = config();
        let picture = Picture::read(&history, &config, NOW).unwrap();
        let collected = collection(&picture, &[], &config);
        assert_eq!(
            collected.sessions,
            BTreeMap::from([
                ("01a1271f-aaaa".to_owned(), crate::money::dollars(12.5)),
                ("01a1271f-bbbb".to_owned(), crate::money::dollars(3.25)),
            ])
        );
        // Every session's together, from the same sum, so it agrees.
        assert_eq!(collected.agents_today, crate::money::dollars(15.75));
        // AI spend's own row says today's dollars with their period.
        assert!(
            collected
                .today
                .starts_with(&format!("{} today", crate::money::dollars(15.75))),
            "{}",
            collected.today
        );
    }

    // A day no session has spent on says nothing, rather than "$0.00".
    #[test]
    fn no_session_spending_says_nothing() {
        let config = config();
        let picture = Picture::read(&History::memory(), &config, NOW).unwrap();
        let collected = collection(&picture, &[], &config);
        assert_eq!(collected.agents_today, "");
        assert!(collected.sessions_today.is_empty());
    }

    // Each session's day is a spark of its hours since midnight, each over
    // the largest hour any session had, and what it rebuilt: a fresh
    // start rebuilt nothing it had, and less than a cent says nothing.
    #[test]
    fn each_session_s_day_is_its_hours_and_what_it_rebuilt() {
        use crate::logs::{Cause, Cost, Event, Kind, Tokens, Turn};
        let history = History::memory();
        let turns = [
            ("01a1271f-aaaa", NOW - 3600.0, 12.0, 0.0, None),
            ("01a1271f-aaaa", NOW - 1800.0, 0.5, 0.4, Some(Cause::Gap)),
            ("01a1271f-bbbb", NOW - 600.0, 3.25, 0.004, Some(Cause::Gap)),
            ("01a1271f-bbbb", NOW - 300.0, 0.0, 2.0, Some(Cause::Fresh)),
            (
                "01a1271f-cccc",
                NOW - crate::time::DAY,
                40.0,
                9.0,
                Some(Cause::Gap),
            ),
        ];
        for (session, at, total, rebuilt_cost, cause) in turns {
            history
                .add_event(&Event::Turn(Turn {
                    session: session.into(),
                    at,
                    model: "m".into(),
                    provider: "p".into(),
                    kind: Kind::Agent,
                    tokens: Tokens::default(),
                    cost: Cost {
                        total,
                        ..Cost::default()
                    },
                    quest: None,
                    budget: None,
                    gap: None,
                    rebuilt: 0.0,
                    rebuilt_cost,
                    cause,
                }))
                .unwrap();
        }
        let config = config();
        let picture = Picture::read(&history, &config, NOW).unwrap();
        let days = collection(&picture, &[], &config).sessions_today;
        assert_eq!(
            days.keys().collect::<Vec<_>>(),
            ["01a1271f-aaaa", "01a1271f-bbbb"]
        );
        let hours = |rest: &[f64]| {
            let mut values = vec![0.0; 15 - rest.len()];
            values.extend_from_slice(rest);
            values
        };
        let aaaa = &days["01a1271f-aaaa"];
        assert_eq!(aaaa.graph.shape, "spark");
        assert_eq!(aaaa.graph.values, hours(&[1.0, 0.5 / 12.0]));
        assert_eq!(aaaa.rebuilt, crate::money::dollars(0.4));
        let bbbb = &days["01a1271f-bbbb"];
        assert_eq!(bbbb.graph.values, hours(&[3.25 / 12.0]));
        assert_eq!(bbbb.rebuilt, "");
        let json = serde_json::to_value(collection(&picture, &[], &config)).unwrap();
        assert_eq!(
            json["sessionsToday"]["01a1271f-aaaa"]["graph"]["shape"],
            "spark"
        );
    }

    // Late in the day a spark holds its last twenty hours, so a session
    // that spent only before them reads as flat rather than as no graph.
    #[test]
    fn a_session_s_day_holds_its_last_twenty_hours() {
        use crate::logs::{Cost, Event, Kind, Tokens, Turn};
        let history = History::memory();
        let late = NOW + 8.0 * 3600.0; // 22:30
        history
            .add_event(&Event::Turn(Turn {
                session: "01a1271f-aaaa".into(),
                at: NOW - 13.0 * 3600.0, // 01:30
                model: "m".into(),
                provider: "p".into(),
                kind: Kind::Agent,
                tokens: Tokens::default(),
                cost: Cost {
                    total: 5.0,
                    ..Cost::default()
                },
                quest: None,
                budget: None,
                gap: None,
                rebuilt: 0.0,
                rebuilt_cost: 0.0,
                cause: None,
            }))
            .unwrap();
        let config = config();
        let picture = Picture::read(&history, &config, late).unwrap();
        let days = collection(&picture, &[], &config).sessions_today;
        assert_eq!(days["01a1271f-aaaa"].graph.values, vec![0.0; 20]);
    }

    // A quest's family is what it spent today and what every quest under it
    // did, at any depth, so the launcher can say what a whole orchestration
    // cost without adding anything up: yesterday's, another machine's and
    // spend on no quest aren't in any.
    #[test]
    fn each_quest_says_what_it_and_the_quests_under_it_spent_today() {
        use crate::logs::{Cost, Event, Kind, Tokens, Turn};
        let history = History::memory();
        let quest = |id: &str, parent: Option<&str>| crate::quests::Quest {
            id: id.into(),
            title: id.into(),
            kind: "quest".into(),
            parent: parent.map(Into::into),
            status: "active".into(),
            started: None,
            updated: None,
            prs: 0,
            sessions: Vec::new(),
        };
        crate::quests::store(
            &history,
            &[
                quest("QEST-ROOT", None),
                quest("QEST-OWNER", Some("QEST-ROOT")),
                quest("QEST-BUILDER", Some("QEST-OWNER")),
                quest("QEST-QUIET", Some("QEST-ROOT")),
            ],
        )
        .unwrap();
        let turns = [
            ("s-root", NOW - 3600.0, 3.10, Some("QEST-ROOT")),
            ("s-owner", NOW - 1800.0, 6.0, Some("QEST-OWNER")),
            ("s-builder", NOW - 900.0, 5.10, Some("QEST-BUILDER")),
            ("s-owner", NOW - crate::time::DAY, 40.0, Some("QEST-OWNER")),
            ("elsewhere", NOW - 600.0, 7.0, Some("QEST-OWNER")),
            ("s-none", NOW - 600.0, 1.0, None),
        ];
        for (session, at, total, quest) in turns {
            history
                .add_event(&Event::Turn(Turn {
                    session: session.into(),
                    at,
                    model: "m".into(),
                    provider: "p".into(),
                    kind: Kind::Agent,
                    tokens: Tokens::default(),
                    cost: Cost {
                        total,
                        ..Cost::default()
                    },
                    quest: quest.map(Into::into),
                    budget: None,
                    gap: None,
                    rebuilt: 0.0,
                    rebuilt_cost: 0.0,
                    cause: None,
                }))
                .unwrap();
        }
        let config = config();
        let picture = Picture::read(&history, &config, NOW).unwrap();
        let collected = collection(&picture, &[], &config);
        let family = |all: f64, own: f64, under: f64| report::Family {
            all: dollars(all),
            own: dollars(own),
            under: dollars(under),
        };
        assert_eq!(
            collected.families,
            BTreeMap::from([
                ("QEST-BUILDER".to_owned(), family(5.10, 5.10, 0.0)),
                ("QEST-OWNER".to_owned(), family(11.10, 6.0, 5.10)),
                ("QEST-ROOT".to_owned(), family(14.20, 3.10, 11.10)),
            ])
        );
    }

    // A session's councils and fleets today are its helpers, costliest
    // first, by its whole id; yesterday's and its own turns aren't.
    #[test]
    fn each_session_says_what_its_helpers_cost_today() {
        use crate::logs::{Cost, Event, Kind, Tokens, Turn};
        let history = History::memory();
        history
            .add_event(&Event::Turn(Turn {
                session: "01a1271f-aaaa".into(),
                at: NOW - 3600.0,
                model: "m".into(),
                provider: "p".into(),
                kind: Kind::Agent,
                tokens: Tokens::default(),
                cost: Cost {
                    total: 10.0,
                    ..Cost::default()
                },
                quest: None,
                budget: None,
                gap: None,
                rebuilt: 0.0,
                rebuilt_cost: 0.0,
                cause: None,
            }))
            .unwrap();
        let runs = [
            ("c1", "council", NOW - 1800.0, 4.0),
            ("c2", "council", NOW - 900.0, 3.0),
            ("f1", "fleet", NOW - 600.0, 2.5),
            ("f0", "fleet", NOW - crate::time::DAY, 9.0),
        ];
        for (run, kind, started, cost) in runs {
            history
                .db
                .execute(
                    "INSERT INTO runs VALUES (?1, 'a1', ?2, 'opus', 'critic', '01a1271f-aaaa', ?3, NULL, ?4,
                     0, 0, 0, 0, '[]')",
                    rusqlite::params![run, kind, started, cost],
                )
                .unwrap();
        }
        let config = config();
        let picture = Picture::read(&history, &config, NOW).unwrap();
        let collected = collection(&picture, &[], &config);
        let said: Vec<(String, String)> = collected.helpers["01a1271f-aaaa"]
            .iter()
            .map(|helper| (helper.on.clone(), helper.spent.clone()))
            .collect();
        assert_eq!(
            said,
            [
                ("councils".to_owned(), crate::money::dollars(7.0)),
                ("fleets".to_owned(), crate::money::dollars(2.5)),
            ]
        );
        assert_eq!(
            collected.sessions["01a1271f-aaaa"],
            crate::money::dollars(19.5)
        );
    }

    fn depth(lines: &[Line]) -> usize {
        lines
            .iter()
            .map(|line| 1 + depth(&line.rows))
            .max()
            .unwrap_or(0)
    }

    /// What's collected with two budgets declared, the proxy cap the
    /// default and a Claude plan with two windows, the nearer read at 85%.
    fn budgeted() -> (Picture, Collection, crate::config::Config) {
        use crate::logs::{Cost, Event, Kind, Tokens, Turn};
        let config: crate::config::Config = serde_json::from_value(serde_json::json!({
            "sessions": "/none", "runs": "/none", "registry": "/none", "quests": "/none",
            "state": "/none", "zone": "UTC",
            "budgets": [
                {"name": "proxy", "label": "Your proxy cap", "kind": "cap",
                 "whose": "employer", "default": true},
                {"name": "claude", "label": "Claude Pro", "kind": "plan",
                 "whose": "own", "providers": ["claude-code"],
                 "price": {"dollars": 20, "per": "month"},
                 "allowances": [{"name": "five-hours", "unit": "share", "windowHours": 5,
                                 "warn": 0.8},
                                {"name": "week", "unit": "share", "windowHours": 168,
                                 "warn": 0.8}]},
            ],
        }))
        .unwrap();
        let history = History::memory();
        let turns = [("me", "anthropic", 40.0), ("plan", "claude-code", 9.0)];
        for ((session, provider, total), at) in turns
            .into_iter()
            .flat_map(|turn| [(turn, NOW - 600.0), (turn, NOW - crate::time::DAY)])
        {
            history
                .add_event(&Event::Turn(Turn {
                    session: session.into(),
                    at,
                    model: "m".into(),
                    provider: provider.into(),
                    kind: Kind::Agent,
                    tokens: Tokens::default(),
                    cost: Cost {
                        total,
                        ..Cost::default()
                    },
                    quest: None,
                    budget: None,
                    gap: None,
                    rebuilt: 0.0,
                    rebuilt_cost: 0.0,
                    cause: None,
                }))
                .unwrap();
        }
        store(
            &history,
            &serde_json::json!({
                "v": 1, "kind": "cap", "name": "five-hours", "label": "Five Hours",
                "unit": "share", "budget": "claude", "used": 0.85, "windowHours": 5,
                "warn": 0.8, "state": "ok", "resets": "2026-10-14T16:20:00Z",
                "at": "2026-10-14T14:20:00Z",
            }),
            NOW,
        );
        let picture = Picture::read(&history, &config, NOW).unwrap();
        let collected = collection(&picture, &[], &config);
        (picture, collected, config)
    }

    /// The Budgets row of what's collected.
    fn budgets_of(collected: &Collection) -> &Line {
        collected
            .now
            .iter()
            .find(|line| line.name == "Budgets")
            .unwrap()
    }

    // With budgets declared, how things stand holds one more row: the
    // month's whole in billed spend, the whole first behind it, then each
    // budget nearest its allowance, a plan with two windows stepping in to
    // each, and nothing deeper than three.
    #[test]
    fn the_budgets_stand_behind_one_row() {
        let (picture, collected, config) = budgeted();
        let names: Vec<&str> = collected
            .now
            .iter()
            .map(|line| line.name.as_str())
            .collect();
        let at = names.iter().position(|name| *name == "Budgets").unwrap();
        assert_eq!(names[at + 1], "Yesterday", "{names:?}");
        let budgets = &collected.now[at];
        assert_eq!(budgets.side, format!("{} this month", whole(80.0 + 20.0)));
        // The whole's row says today's whole, which its parent doesn't.
        let standing = crate::standing::standing(&picture, &config, None);
        let today = format!("{} today", standing.total.periods["today"].shown);
        assert_ne!(today, budgets.side);
        let behind: Vec<(&str, &str)> = budgets
            .rows
            .iter()
            .map(|row| (row.name.as_str(), row.side.as_str()))
            .collect();
        assert_eq!(
            behind,
            [
                ("All AI Spend", today.as_str()),
                ("Claude Pro", "85% of 5 h"),
                ("Your proxy cap", "$80 this month"),
            ]
        );
        // The Budgets row takes its nearest budget's warning.
        assert_eq!(budgets.role, "warning");
        let plan = &budgets.rows[1];
        assert_eq!(plan.role, "warning");
        // Whose money it is is said by its drawer's eyebrow, not again
        // beside its standing.
        assert_eq!(plan.note, "");
        let windows: Vec<(&str, &str)> = plan
            .rows
            .iter()
            .map(|row| (row.name.as_str(), row.side.as_str()))
            .collect();
        // A window no feed reads yet is named for its length.
        assert_eq!(windows, [("Five Hours", "85%"), ("Week", "not read yet")]);
        assert!(plan.rows[1].copy.contains("nothing to copy"));
        assert_eq!(plan.rows[0].note, "");
        // The budget's drawer and each window's come with their rows.
        let titled = |row: &Line| row.drawn.as_ref().map(|drawn| drawn.title.clone());
        assert_eq!(titled(plan).as_deref(), Some("Claude Pro"));
        assert_eq!(titled(&plan.rows[0]).as_deref(), Some("Five Hours"));
        assert!(depth(&collected.now) <= 3);
    }

    // Each budget's row wears AI spend's mark and each window a clock, and
    // a day's row a day's, a window named as its feed names it.
    #[test]
    fn the_budgets_wear_their_marks() {
        let (_, collected, _) = budgeted();
        let budgets = budgets_of(&collected);
        let plan = &budgets.rows[1];
        let marks: Vec<&str> = std::iter::once(budgets)
            .chain(&budgets.rows)
            .map(|row| row.mark.as_str())
            .collect();
        assert_eq!(marks, ["agent", "agent", "agent", "agent"]);
        let windows: Vec<(&str, &str)> = plan
            .rows
            .iter()
            .map(|row| (row.mark.as_str(), row.name.as_str()))
            .collect();
        assert_eq!(windows, [("clock", "Five Hours"), ("clock", "Week")]);
        assert_eq!(collected.now[0].mark, "date");
    }

    // While the screen is shared nothing says an amount, at any depth.
    #[test]
    fn a_shared_screen_shows_no_amount() {
        let collected = rebuilding();
        assert!(collected.quiet.len() >= 3, "{:?}", collected.quiet);
        for line in every(&collected.quiet) {
            // A drawer's words too, the eyebrow and every fact.
            let drawn = line.drawn.iter().flat_map(|drawn| {
                std::iter::once(&drawn.eyebrow).chain(drawn.facts.iter().map(|fact| &fact.value))
            });
            for said in [&line.name, &line.side, &line.note, &line.copy]
                .into_iter()
                .chain(drawn)
            {
                assert!(!said.contains('$'), "{said}");
            }
        }
        // Today's drawer keeps what went on rebuilding context as a share.
        let today = collected
            .quiet
            .iter()
            .find(|line| line.key == "quiet:today")
            .and_then(|line| line.drawn.as_ref())
            .unwrap();
        assert!(
            today.facts.iter().any(|fact| fact.id == "rebuilding"),
            "{today:?}"
        );
    }

    // A period's drawer says its days above it, then where its figure was
    // read from, what rebuilding context took and its share, and the
    // largest of each cut, "all" for the only one; a cap's row opens its
    // window's drawer in budgets.json.
    #[test]
    fn a_period_and_a_cap_open_a_drawer() {
        let collected = rebuilding();
        let today = &collected.now[0];
        let drawn = today.drawn.as_ref().unwrap();
        // How the day is likely to end comes first, while there's a
        // forecast to say it.
        let facts: Vec<(&str, &str, bool)> = drawn
            .facts
            .iter()
            .map(|fact| (fact.id.as_str(), fact.value.as_str(), fact.words))
            .skip_while(|(id, _, _)| *id == "ends")
            .collect();
        assert_eq!(drawn.eyebrow, "Wed 14 Oct \u{b7} day");
        let ids: Vec<&str> = facts.iter().map(|(id, _, _)| *id).collect();
        assert!(ids.starts_with(&["from", "rebuilding"]), "{ids:?}");
        assert!(ids.contains(&"most-model"), "{ids:?}");
        assert!(
            !ids.contains(&"most-day"),
            "a day has no days to cut by: {ids:?}"
        );
        assert_eq!(facts[0].1, "the bill");
        assert!(facts[0].2, "where it was read from is words");
        let (_, rebuilt, _) = facts[1];
        assert!(
            rebuilt.starts_with('$') && rebuilt.ends_with(" \u{b7} 6%"),
            "{rebuilt}"
        );
        let week = collected
            .now
            .iter()
            .find(|line| line.key == "period:this-week")
            .unwrap();
        let days = week.drawn.as_ref().unwrap();
        assert_eq!(days.eyebrow, "Mon 12 Oct to Wed 14 Oct \u{b7} week");
        assert!(
            days.facts
                .iter()
                .any(|fact| fact.id == "most-day" && fact.words)
        );
        let cap = every(&collected.now)
            .into_iter()
            .find(|line| line.key.starts_with("cap:"))
            .unwrap();
        assert!(cap.budget.starts_with("budget:"), "{}", cap.budget);
        // The cap's row carries its window's drawer, and its quiet row the
        // same, its dollars left out.
        let drawn = cap.drawn.as_ref().expect("a cap's drawer");
        assert!(
            drawn.names && drawn.eyebrow.ends_with("window"),
            "{drawn:?}"
        );
        // A cap longer than a day has a quiet row, which draws the same
        // window; a day's is behind today and has none.
        let (loud, quiet) = collected
            .quiet
            .iter()
            .find_map(|line| {
                let key = line.key.strip_prefix("quiet:")?;
                let loud = every(&collected.now)
                    .into_iter()
                    .find(|one| one.key == key)?;
                Some((loud.drawn.as_ref()?, line.drawn.as_ref()?))
            })
            .expect("a cap longer than a day, quiet and not");
        assert_eq!(quiet.title, loud.title);
    }

    // A finding that wants a look is listed first and in the summary,
    // which leaves out today's row, whose words are its headline, and the
    // reports behind no other row; while shared, it comes first too.
    #[test]
    fn a_finding_that_wants_a_look_is_ranked_first() {
        let made = |key: &str, role: &str| {
            let mut made = line(key, key, "");
            made.role = role.into();
            made
        };
        let now = [made("today", "text"), made("week", "text")];
        let findings = [made("finding:fine", "found"), made("finding:burn", "alert")];
        let (shown, summary) = ranked(&now, &findings, &[made("report:waste", "report")]);
        assert_eq!(
            shown,
            [
                "finding:burn",
                "today",
                "week",
                "finding:fine",
                "report:waste"
            ]
        );
        assert_eq!(summary, ["finding:burn", "week", "finding:fine"]);
        let quiet = asking_first(vec![
            made("quiet:today", "text"),
            made("quiet:finding:burn", "warning"),
        ]);
        let keys: Vec<&str> = quiet.iter().map(|line| line.key.as_str()).collect();
        assert_eq!(keys, ["quiet:finding:burn", "quiet:today"]);
    }

    /// The busy collection, with today's and this week's spend partly
    /// gone on rebuilding context, which its billed hours can't say.
    fn rebuilding() -> Collection {
        let (history, config) = (busy(), config());
        let mut picture = Picture::read(&history, &config, NOW).unwrap();
        picture.rebuilt.insert("today", 0.06);
        picture.rebuilt.insert("this-week", 0.05);
        let found = crate::detect::all(&picture, &history, &config).unwrap();
        collection(&picture, &found, &config)
    }

    // The largest of a cut is said with its share, or "all" when it's the
    // only one.
    #[test]
    fn the_most_of_a_cut_says_its_share() {
        let row = |name: &str, share: f64| report::Row {
            name: name.to_owned(),
            dollars: 1.0,
            share,
            shown: String::new(),
            estimated: false,
        };
        let mut cut = report::Report {
            title: String::new(),
            period: String::new(),
            from: String::new(),
            to: String::new(),
            total: 1.0,
            shown: String::new(),
            source: String::new(),
            rows: vec![row("claude-opus-5-5", 1.0)],
            notes: Vec::new(),
            rest: None,
        };
        assert_eq!(
            most(Cut::Model, &cut).map(|fact| fact.value),
            Some("claude-opus-5-5 \u{b7} all".to_owned())
        );
        cut.rows = vec![row("Build the shell", 0.31), row("Another", 0.2)];
        let quest = most(Cut::Quest, &cut).unwrap();
        assert_eq!(quest.value, "Build the shell \u{b7} 31%");
        assert!(quest.words && quest.id == "most-quest");
    }
}
