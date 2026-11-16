//! Reports: a period's spend cut one way, or a focused look at waste,
//! caps, quests, outliers or the forecast, rendered as text, markdown,
//! JSON or CSV. The collection's reports are these same ones, written
//! ahead, so what the launcher shows is what the command line says.

use crate::findings::Finding;
use crate::money::{dollars, percent, whole};
use crate::picture::Picture;
use crate::series::{self, Piece, Source};
use crate::time::Zone;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Write as _;
use terminal_kit::{Align, Pen, Room, layout};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    Today,
    Yesterday,
    ThisWeek,
    LastWeek,
    ThisMonth,
    LastMonth,
    Days30,
}

impl Period {
    pub const ALL: [Period; 7] = [
        Period::Today,
        Period::Yesterday,
        Period::ThisWeek,
        Period::LastWeek,
        Period::ThisMonth,
        Period::LastMonth,
        Period::Days30,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Period::Today => "today",
            Period::Yesterday => "yesterday",
            Period::ThisWeek => "this-week",
            Period::LastWeek => "last-week",
            Period::ThisMonth => "this-month",
            Period::LastMonth => "last-month",
            Period::Days30 => "30-days",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Period::Today => "today",
            Period::Yesterday => "yesterday",
            Period::ThisWeek => "this week",
            Period::LastWeek => "last week",
            Period::ThisMonth => "this month",
            Period::LastMonth => "last month",
            Period::Days30 => "the last 30 days",
        }
    }

    /// What kind of span it is, as its drawer's eyebrow says after its
    /// days.
    pub fn kind(self) -> &'static str {
        match self {
            Period::Today | Period::Yesterday => "day",
            Period::ThisWeek | Period::LastWeek => "week",
            Period::ThisMonth | Period::LastMonth => "month",
            Period::Days30 => "30 days",
        }
    }

    /// The ways a person might type it.
    pub fn words(self) -> &'static [&'static str] {
        match self {
            Period::Today => &["today", "day"],
            Period::Yesterday => &["yesterday"],
            Period::ThisWeek => &["this week", "week"],
            Period::LastWeek => &["last week"],
            Period::ThisMonth => &["this month", "month"],
            Period::LastMonth => &["last month"],
            Period::Days30 => &["30 days", "last 30 days", "30d"],
        }
    }

    pub fn parse(text: &str) -> Option<Period> {
        let text = text.trim().to_lowercase().replace('-', " ");
        Period::ALL.into_iter().find(|period| {
            period.id().replace('-', " ") == text || period.words().iter().any(|word| *word == text)
        })
    }

    /// Its local days, first and last.
    pub fn days(self, zone: &Zone, now: f64) -> (i64, i64) {
        let today = zone.day(now);
        let monday = Zone::week_start(today);
        let month_first = |day: i64| {
            let iso = Zone::iso_day(day);
            crate::time::parse_day(&format!("{}-01", &iso[..7])).unwrap_or(day)
        };
        match self {
            Period::Today => (today, today),
            Period::Yesterday => (today - 1, today - 1),
            Period::ThisWeek => (monday, today),
            Period::LastWeek => (monday - 7, monday - 1),
            Period::ThisMonth => (month_first(today), today),
            Period::LastMonth => {
                let first = month_first(today);
                (month_first(first - 1), first - 1)
            }
            Period::Days30 => (today - 29, today),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cut {
    Quest,
    Model,
    Agent,
    Repo,
    Session,
    Tool,
    Day,
    Source,
    Budget,
}

impl Cut {
    pub const ALL: [Cut; 9] = [
        Cut::Quest,
        Cut::Model,
        Cut::Agent,
        Cut::Repo,
        Cut::Session,
        Cut::Tool,
        Cut::Day,
        Cut::Source,
        Cut::Budget,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Cut::Quest => "quest",
            Cut::Model => "model",
            Cut::Agent => "agent",
            Cut::Repo => "repo",
            Cut::Session => "session",
            Cut::Tool => "tool",
            Cut::Day => "day",
            Cut::Source => "source",
            Cut::Budget => "budget",
        }
    }

    pub fn words(self) -> &'static [&'static str] {
        match self {
            Cut::Quest => &["quest", "quests"],
            Cut::Model => &["model", "models"],
            Cut::Agent => &["agent", "agents", "kind", "persona"],
            Cut::Repo => &["repo", "repos", "repository"],
            Cut::Session => &["session", "sessions"],
            Cut::Tool => &["tool", "tools"],
            Cut::Day => &["day", "days", "daily"],
            Cut::Source => &["source", "sources"],
            Cut::Budget => &["budget", "budgets"],
        }
    }

    pub fn parse(text: &str) -> Option<Cut> {
        let text = text.trim().to_lowercase();
        Cut::ALL
            .into_iter()
            .find(|cut| cut.words().contains(&text.as_str()))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Row {
    pub name: String,
    pub dollars: f64,
    pub share: f64,
    pub shown: String,
    pub estimated: bool,
    /// What it says under its report's columns after its name, in a
    /// terminal; a row of a report with none says `shown` beside it.
    #[serde(skip)]
    pub cells: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub title: String,
    pub period: String,
    pub from: String,
    pub to: String,
    pub total: f64,
    pub shown: String,
    pub source: String,
    pub rows: Vec<Row>,
    pub notes: Vec<String>,
    /// How many rows were left out past the top, and their dollars
    /// together, which a note says too.
    #[serde(skip)]
    pub rest: Option<(usize, f64)>,
    /// The columns its rows are set in, in a terminal, its names' first;
    /// none where each row says a sentence, set as facts.
    #[serde(skip)]
    pub columns: Vec<(&'static str, Align)>,
}

/// The columns of a report of spend: what each row names, what it spent
/// and its share of the whole.
fn spend_columns(noun: &'static str) -> Vec<(&'static str, Align)> {
    vec![
        (noun, Align::Left),
        ("spent", Align::Right),
        ("share", Align::Right),
    ]
}

/// A spend row's cells under those columns; a share of nothing says
/// nothing.
fn spend_cells(shown: &str, share: f64) -> Vec<String> {
    let share = if share > 0.0 {
        percent(share)
    } else {
        String::new()
    };
    vec![shown.to_owned(), share]
}

/// What an agent's work is called in a report.
pub fn agent_name(piece: &Piece) -> String {
    match piece.kind.as_str() {
        "agent" => "pi sessions".into(),
        "compaction" => "compaction".into(),
        "elsewhere" => "elsewhere".into(),
        kind if kind.starts_with("side:") => format!("{} (extension)", &kind[5..]),
        kind if kind.starts_with("subagent:") => {
            let rest = &kind[9..];
            if piece.persona.is_empty() {
                format!("{rest} subagents")
            } else {
                format!("{} ({rest})", piece.persona)
            }
        }
        kind => kind.into(),
    }
}

/// The fields of a piece its name under a cut is made of, as [`key`]
/// reads them: pieces alike in these have the same name.
type Fields<'a> = ([&'a str; 4], i64);

/// The fields [`key`] reads of `piece` under `cut`, borrowed, with a
/// number for what isn't text: whether it has a quest, or its hour for a
/// day's name.
fn fields(cut: Cut, piece: &Piece) -> Fields<'_> {
    let quest = piece.quest.as_deref();
    let has_quest = i64::from(quest.is_some());
    match cut {
        Cut::Quest => ([quest.unwrap_or(""), &piece.tool, "", ""], has_quest),
        Cut::Model => ([&piece.model, &piece.kind, "", ""], 0),
        Cut::Agent => ([&piece.kind, &piece.persona, "", ""], 0),
        Cut::Repo => ([&piece.repo, &piece.tool, "", ""], 0),
        Cut::Session => (
            [
                &piece.session,
                quest.unwrap_or(""),
                &piece.repo,
                &piece.tool,
            ],
            has_quest,
        ),
        Cut::Tool => ([&piece.tool, "", "", ""], 0),
        Cut::Day => ([""; 4], piece.hour),
        Cut::Source => ([piece.source.name(), "", "", ""], 0),
        Cut::Budget => ([&piece.budget, "", "", ""], 0),
    }
}

fn key(picture: &Picture, cut: Cut, piece: &Piece) -> String {
    let title = |id: &str| picture.title(id);
    match cut {
        Cut::Quest => match &piece.quest {
            Some(quest) => title(picture.root(quest)),
            None if piece.tool != "pi" => format!("{} (no quest)", piece.tool),
            None => "no quest".into(),
        },
        Cut::Model => {
            if piece.model.is_empty() {
                format!(
                    "not recorded ({})",
                    piece.kind.strip_prefix("subagent:").unwrap_or(&piece.kind)
                )
            } else {
                piece.model.clone()
            }
        }
        Cut::Agent => agent_name(piece),
        Cut::Repo => {
            if piece.repo.is_empty() {
                format!("no repo ({})", piece.tool)
            } else {
                piece.repo.clone()
            }
        }
        Cut::Session => match (piece.session.as_str(), &piece.quest) {
            ("", _) => format!("{} (no session)", piece.tool),
            ("elsewhere", _) => "elsewhere".into(),
            (session, Some(quest)) => {
                format!("{} · {}", title(quest), &session[..session.len().min(8)])
            }
            (session, None) => format!("{} · {}", piece.repo, &session[..session.len().min(8)]),
        },
        Cut::Tool => piece.tool.clone(),
        Cut::Day => Zone::day_name(picture.zone.day(crate::time::hour_start(piece.hour))),
        Cut::Source => piece.source.name().into(),
        // By the name the harness and pi's footer know it by.
        Cut::Budget => piece.budget.clone(),
    }
}

fn source_line(pieces: &[Piece]) -> String {
    match series::weakest(pieces) {
        Some(Source::Warehouse) => "billed".into(),
        Some(Source::Meter) => "billed, the latest hours from the live meter".into(),
        Some(Source::Estimated) => {
            "billed where settled, estimated from pi's logs where not".into()
        }
        Some(Source::List) => "pi's logs at list prices, with no bill to read".into(),
        None => "nothing spent".into(),
    }
}

/// A period's spend, cut one way.
pub fn cut(picture: &Picture, period: Period, cut: Cut, top: usize) -> Report {
    let (from, to) = period.days(&picture.zone, picture.now);
    let pieces = picture.between(from, to);
    let total = series::total(pieces);
    // Each name's dollars, whether any are estimated, and its first hour,
    // gathered first by the fields its name is made of, so a name is made
    // once a group rather than once a piece: made for each of a month's
    // pieces under every cut, names were most of what a pass spent on its
    // reports on 16 November.
    let mut by_fields: HashMap<Fields, (f64, bool, i64, &Piece)> = HashMap::new();
    for piece in pieces {
        let entry = by_fields
            .entry(fields(cut, piece))
            .or_insert((0.0, false, piece.hour, piece));
        entry.0 += piece.dollars;
        entry.1 |= piece.source.estimated() && piece.dollars > 0.005;
        entry.2 = entry.2.min(piece.hour);
    }
    let mut grouped: HashMap<String, (f64, bool, i64)> = HashMap::new();
    for (spent, estimated, first, piece) in by_fields.into_values() {
        let entry = grouped
            .entry(key(picture, cut, piece))
            .or_insert((0.0, false, first));
        entry.0 += spent;
        entry.1 |= estimated;
        entry.2 = entry.2.min(first);
    }
    let mut rows: Vec<(i64, Row)> = grouped
        .into_iter()
        .filter(|(_, (spent, _, _))| *spent >= 0.005)
        .map(|(name, (spent, estimated, first))| {
            let share = if total > 0.0 { spent / total } else { 0.0 };
            let shown = crate::money::maybe_about(dollars(spent), estimated);
            (
                first,
                Row {
                    name,
                    dollars: spent,
                    share,
                    cells: spend_cells(&shown, share),
                    shown,
                    estimated,
                },
            )
        })
        .collect();
    if cut == Cut::Day {
        rows.sort_by_key(|(first, _)| *first);
    } else {
        rows.sort_by(|(_, one), (_, other)| {
            other
                .dollars
                .total_cmp(&one.dollars)
                .then(one.name.cmp(&other.name))
        });
    }
    let mut rows: Vec<Row> = rows.into_iter().map(|(_, row)| row).collect();
    let mut notes = Vec::new();
    let mut left_out = None;
    if rows.len() > top {
        let rest: Vec<Row> = rows.split_off(top);
        let spent: f64 = rest.iter().map(|row| row.dollars).sum();
        notes.push(format!(
            "{} more under {} together.",
            rest.len(),
            dollars(spent)
        ));
        left_out = Some((rest.len(), spent));
    }
    Report {
        title: format!("Spend {} by {}", period.name(), cut.id()),
        period: period.id().into(),
        from: Zone::iso_day(from),
        to: Zone::iso_day(to),
        total,
        shown: crate::money::maybe_about(dollars(total), series::estimated(pieces)),
        source: source_line(pieces),
        rest: left_out,
        rows,
        notes,
        columns: spend_columns(cut.id()),
    }
}

/// What each pi session has spent in a period, by its whole id, as a
/// figure ready to draw beside it, so the shell can say what a session it
/// sees costs without reading a report's names. Another tool's spend and
/// spend no log knows have no session, and a session that spent less than
/// a cent says nothing.
pub fn by_session(picture: &Picture, period: Period) -> std::collections::BTreeMap<String, String> {
    spent_by_session(picture, period)
        .into_iter()
        .map(|(session, spent)| {
            (
                session,
                crate::money::maybe_about(dollars(spent.dollars), spent.estimated),
            )
        })
        .collect()
}

/// What a pi session spent, as a figure rather than words: its dollars
/// and whether any of them was estimated.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Spent {
    pub dollars: f64,
    pub estimated: bool,
}

/// What each pi session has spent in a period, by its whole id, as
/// `by_session` says it but as figures, so the launcher's Agents can add
/// up the sessions of a network, each once, and say the sum as these
/// are said (ready-rows' money), rather than adding up words. A session
/// that spent less than a cent is left out, as there.
pub fn spent_by_session(
    picture: &Picture,
    period: Period,
) -> std::collections::BTreeMap<String, Spent> {
    sessions_spent(picture, period)
        .into_iter()
        .filter(|(_, (dollars_spent, _))| *dollars_spent >= 0.005)
        .map(|(session, (dollars, estimated))| (session, Spent { dollars, estimated }))
        .collect()
}

/// What every pi session together spent in a period, said as each of
/// `by_session`'s is, and empty when it's under a cent: the same sum, so
/// the launcher's Agents says it without adding up strings, and it agrees
/// with its rows.
pub fn by_sessions(picture: &Picture, period: Period) -> String {
    let (total, estimated) = sessions_spent(picture, period)
        .into_values()
        .fold((0.0, false), |(total, estimated), (dollars_spent, its)| {
            (total + dollars_spent, estimated || its)
        });
    if total < 0.005 {
        return String::new();
    }
    crate::money::maybe_about(dollars(total), estimated)
}

/// Each pi session's spend in a period, by its whole id, and whether any
/// of it was estimated; spend that came from no session, or from another
/// machine, is no session's.
fn sessions_spent(picture: &Picture, period: Period) -> HashMap<String, (f64, bool)> {
    let (from, to) = period.days(&picture.zone, picture.now);
    let mut spent: HashMap<String, (f64, bool)> = HashMap::new();
    for piece in picture.between(from, to) {
        if piece.session.is_empty() || piece.session == "elsewhere" {
            continue;
        }
        let entry = spent.entry(piece.session.clone()).or_insert((0.0, false));
        entry.0 += piece.dollars;
        entry.1 |= piece.source.estimated() && piece.dollars > 0.005;
    }
    spent
}

/// What a pi session's helpers cost in a period: each kind of run it
/// started (its councils, judges, fleets) by its plural, costliest first,
/// by the session's whole id. A session whose spend was all its own has
/// none, and a kind under a cent is left out.
pub fn helpers_by_session(
    picture: &Picture,
    period: Period,
) -> std::collections::BTreeMap<String, Vec<Helper>> {
    let (from, to) = period.days(&picture.zone, picture.now);
    let mut spent: HashMap<(&str, &str), (f64, bool)> = HashMap::new();
    for piece in picture.between(from, to) {
        let Some(kind) = piece.kind.strip_prefix("subagent:") else {
            continue;
        };
        if piece.session.is_empty() || piece.session == "elsewhere" {
            continue;
        }
        let entry = spent
            .entry((piece.session.as_str(), kind))
            .or_insert((0.0, false));
        entry.0 += piece.dollars;
        entry.1 |= piece.source.estimated() && piece.dollars > 0.005;
    }
    let mut by: std::collections::BTreeMap<String, Vec<(f64, Helper)>> =
        std::collections::BTreeMap::default();
    for ((session, kind), (dollars_spent, estimated)) in spent {
        if dollars_spent < 0.005 {
            continue;
        }
        by.entry(session.to_owned()).or_default().push((
            dollars_spent,
            Helper {
                on: format!("{kind}s"),
                spent: crate::money::maybe_about(dollars(dollars_spent), estimated),
            },
        ));
    }
    by.into_iter()
        .map(|(session, mut helpers)| {
            helpers.sort_by(|(one, a), (other, b)| other.total_cmp(one).then(a.on.cmp(&b.on)));
            (
                session,
                helpers.into_iter().map(|(_, helper)| helper).collect(),
            )
        })
        .collect()
}

/// What each quest spent in a period with every quest under it, at any
/// depth, by the quest's id: a quest's family, as an orchestrator's row
/// says it, since a quest's sessions orchestrate the quests under it.
/// Only this machine's: spend no log here knows, and spend on no quest,
/// is in none. A family that spent less than a cent says nothing.
pub fn by_family(picture: &Picture, period: Period) -> std::collections::BTreeMap<String, Family> {
    let (from, to) = period.days(&picture.zone, picture.now);
    let parents: HashMap<&str, &str> = picture
        .quests
        .iter()
        .filter_map(|quest| Some((quest.id.as_str(), quest.parent.as_deref()?)))
        .collect();
    // Each quest's own and under, and whether either was estimated.
    let mut spent: HashMap<&str, (f64, f64, bool, bool)> = HashMap::new();
    for piece in picture.between(from, to) {
        let Some(quest) = piece.quest.as_deref() else {
            continue;
        };
        if piece.session == "elsewhere" {
            continue;
        }
        let estimated = piece.source.estimated() && piece.dollars > 0.005;
        let own = spent.entry(quest).or_default();
        own.0 += piece.dollars;
        own.2 |= estimated;
        // Up through its parents, each once, so a family whose record names
        // itself above itself still ends.
        let mut seen = vec![quest];
        let mut at = quest;
        while let Some(&parent) = parents.get(at) {
            if seen.contains(&parent) {
                break;
            }
            seen.push(parent);
            let above = spent.entry(parent).or_default();
            above.1 += piece.dollars;
            above.3 |= estimated;
            at = parent;
        }
    }
    let figure = |amount: f64, estimated: bool| {
        crate::money::maybe_about(dollars(amount), estimated && amount >= 0.005)
    };
    spent
        .into_iter()
        .filter(|(_, (own, under, _, _))| own + under >= 0.005)
        .map(|(quest, (own, under, own_estimated, under_estimated))| {
            (
                quest.to_owned(),
                Family {
                    all: figure(own + under, own_estimated || under_estimated),
                    own: figure(own, own_estimated),
                    under: figure(under, under_estimated),
                },
            )
        })
        .collect()
}

/// A quest's family's spend: all of it, the quest's own, and what the
/// quests under it spent, each a figure ready to draw.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Family {
    pub all: String,
    pub own: String,
    pub under: String,
}

/// One kind of run a session started, and what it cost.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Helper {
    pub on: String,
    pub spent: String,
}

/// How a pi session's day has gone, for the launcher's Agents: its spend
/// hour by hour as a graph the shell draws, and what rebuilding its
/// context cost.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SessionDay {
    pub graph: Spark,
    /// At list, empty when under a cent.
    pub rebuilt: String,
}

/// A history as the shell's Graph draws it: values from nothing to one,
/// the oldest first.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Spark {
    pub shape: &'static str,
    pub values: Vec<f64>,
}

/// The most hours a spark holds, which is what the shell's Graph shows.
const SPARKED: usize = 20;

/// Each pi session's day so far, by its whole id, for those that spent a
/// cent or more today. Each hour is said over the largest hour any session
/// had today, so sparks side by side compare, and none says an amount.
pub fn today_by_session(picture: &Picture) -> std::collections::BTreeMap<String, SessionDay> {
    let today = picture.zone.day(picture.now);
    let first = crate::time::hour_of(picture.zone.day_start(today));
    let last = crate::time::hour_of(picture.now).max(first);
    let first = first.max(last + 1 - i64::try_from(SPARKED).unwrap_or(i64::MAX));
    let shown = usize::try_from(last + 1 - first).unwrap_or(0);
    let mut hours: HashMap<&str, Vec<f64>> = HashMap::new();
    for piece in picture.pieces_today() {
        if piece.session.is_empty() || piece.session == "elsewhere" {
            continue;
        }
        let Ok(at) = usize::try_from(piece.hour - first) else {
            continue;
        };
        let spent = hours
            .entry(piece.session.as_str())
            .or_insert_with(|| vec![0.0; shown]);
        if let Some(hour) = spent.get_mut(at) {
            *hour += piece.dollars;
        }
    }
    let spent = by_session(picture, Period::Today);
    let top = hours.values().flatten().copied().fold(0.0_f64, f64::max);
    spent
        .into_keys()
        .map(|session| {
            // A session whose spend today came before the hours shown reads
            // as flat, not as no graph.
            let values = hours.get(session.as_str()).map_or_else(
                || vec![0.0; shown],
                |spent| {
                    spent
                        .iter()
                        .map(|hour| {
                            if top > 0.0 {
                                (hour / top).clamp(0.0, 1.0)
                            } else {
                                0.0
                            }
                        })
                        .collect()
                },
            );
            let rebuilt = picture
                .rebuilt_today
                .get(&session)
                .copied()
                .filter(|rebuilt| *rebuilt >= 0.005)
                .map_or_else(String::new, dollars);
            let day = SessionDay {
                graph: Spark {
                    shape: "spark",
                    values,
                },
                rebuilt,
            };
            (session, day)
        })
        .collect()
}

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
            &session[..session.len().min(8)]
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
                    |quest| {
                        format!(
                            "{} · {}",
                            quest.title,
                            &outlier.work.session[..8.min(outlier.work.session.len())]
                        )
                    },
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Text,
    Markdown,
    Json,
    Csv,
}

impl Format {
    pub fn parse(text: &str) -> Option<Format> {
        match text {
            "text" => Some(Format::Text),
            "markdown" | "md" => Some(Format::Markdown),
            "json" => Some(Format::Json),
            "csv" => Some(Format::Csv),
            _ => None,
        }
    }
}

fn csv_field(text: &str) -> String {
    if text.contains([',', '"', '\n']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_owned()
    }
}

pub fn render(report: &Report, format: Format) -> String {
    match format {
        Format::Json => serde_json::to_string_pretty(report).unwrap_or_default() + "\n",
        Format::Csv => render_csv(report),
        Format::Markdown => render_markdown(report),
        Format::Text => render_text_in(report, &Pen::plain(), Room::Unbounded),
    }
}

// Writing to a String can't fail, so what `write!` returns below is let go.

/// A report's rows as CSV, with a header.
fn render_csv(report: &Report) -> String {
    let mut out = String::from("name,dollars,share,estimated\n");
    for row in &report.rows {
        _ = writeln!(
            out,
            "{},{:.2},{:.4},{}",
            csv_field(&row.name),
            row.dollars,
            row.share,
            row.estimated
        );
    }
    out
}

/// A report as Markdown: its title, what it covers, a table and its notes.
fn render_markdown(report: &Report) -> String {
    let mut out = format!("## {}\n\n", report.title);
    if !report.shown.is_empty() {
        _ = write!(
            out,
            "{} from {} to {}, {}.\n\n",
            report.shown, report.from, report.to, report.source
        );
    }
    // The caps' rows are what changed and when, not spend.
    if !report.rows.is_empty() && report.period == "caps" {
        out.push_str("| When | What changed |\n|---|---|\n");
        for row in &report.rows {
            _ = writeln!(
                out,
                "| {} | {} |",
                row.name.replace('|', "/"),
                row.shown.replace('|', "/")
            );
        }
        out.push('\n');
    } else if !report.rows.is_empty() {
        out.push_str("| | Spend | Share |\n|---|---:|---:|\n");
        for row in &report.rows {
            let share = if row.share > 0.0 {
                percent(row.share)
            } else {
                String::new()
            };
            _ = writeln!(
                out,
                "| {} | {} | {share} |",
                row.name.replace('|', "/"),
                row.shown.replace('|', "/")
            );
        }
        out.push('\n');
    }
    for note in &report.notes {
        _ = writeln!(out, "- {note}");
    }
    out
}

/// The inset of a report's rows under its title.
const INSET: &str = "  ";

/// A report as plain text, set to `room`: its rows in the kit's table
/// where it has columns, and as the kit's facts where each says a
/// sentence. In a terminal its prose wraps, a name too long for its
/// column is cut at a whole word, or shortened in its middle where it's
/// one word, and a sentence hangs under itself; into a pipe nothing
/// gives way.
pub fn render_text_in(report: &Report, pen: &Pen, room: Room) -> String {
    let prose = room.for_prose();
    let mut out = format!("{}\n", report.title);
    if !report.shown.is_empty() {
        let covers = format!(
            "{} from {} to {}, {}.",
            report.shown, report.from, report.to, report.source
        );
        for line in layout::paragraph(&covers, 0, prose) {
            _ = writeln!(out, "{line}");
        }
    }
    out.push('\n');
    let inside = match room.cells() {
        Some(cells) => Room::Cells(cells.saturating_sub(INSET.len())),
        None => Room::Unbounded,
    };
    let lines = if report.columns.is_empty() {
        sentences(report, pen, inside)
    } else {
        table(report, pen, inside)
    };
    for line in lines {
        _ = writeln!(out, "{INSET}{line}");
    }
    if !report.rows.is_empty() && !report.notes.is_empty() {
        out.push('\n');
    }
    for note in &report.notes {
        for line in layout::paragraph(note, 0, prose) {
            _ = writeln!(out, "{line}");
        }
    }
    out
}

/// A report's rows in its columns, each name cut to the room the other
/// columns leave, so a long quest's title never moves its figures.
fn table(report: &Report, pen: &Pen, room: Room) -> Vec<String> {
    // The fewest cells a name is cut to, so it still tells rows apart.
    const FEWEST: usize = 24;
    if report.rows.is_empty() {
        return Vec::new();
    }
    let others: usize = report
        .columns
        .iter()
        .skip(1)
        .enumerate()
        .map(|(at, (label, _))| {
            report
                .rows
                .iter()
                .filter_map(|row| row.cells.get(at))
                .map(|cell| layout::width(cell))
                .chain([layout::width(label)])
                .max()
                .unwrap_or(0)
                + layout::GAP
        })
        .sum();
    let rows: Vec<Vec<String>> = report
        .rows
        .iter()
        .map(|row| {
            let name = room.cells().map_or_else(
                || row.name.clone(),
                |cells| layout::cut(&row.name, cells.saturating_sub(others).max(FEWEST)),
            );
            std::iter::once(name)
                .chain(row.cells.iter().cloned())
                .collect()
        })
        .collect();
    // Where even a name cut short leaves too little room, the columns
    // past what it spent go, the last first.
    let gives_way: Vec<usize> = (2..report.columns.len()).rev().collect();
    layout::table_in(&report.columns, &rows, pen, room, &gives_way)
}

/// A report whose rows each say a sentence, as facts: its name, and the
/// sentence beside it or under it.
fn sentences(report: &Report, pen: &Pen, room: Room) -> Vec<String> {
    let facts: Vec<(&str, Option<&str>)> = report
        .rows
        .iter()
        .map(|row| (row.name.as_str(), Some(row.shown.as_str())))
        .collect();
    layout::facts(&facts, "unknown", pen, room)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: f64 = 1_791_988_200.0; // 2026-10-14T14:30:00Z

    fn turn(history: &crate::history::History, session: &str, quest: Option<&str>, at: f64) {
        use crate::logs::{Cost, Event, Kind, Tokens, Turn};
        history
            .add_event(&Event::Turn(Turn {
                session: session.into(),
                at,
                model: "m".into(),
                provider: "anthropic".into(),
                kind: Kind::Agent,
                tokens: Tokens::default(),
                cost: Cost {
                    total: 1.0,
                    ..Cost::default()
                },
                quest: quest.map(str::to_owned),
                budget: None,
                gap: None,
                rebuilt: 0.0,
                rebuilt_cost: 0.0,
                cause: None,
            }))
            .unwrap();
    }

    // Names are made once for each group of pieces alike, so pieces alike
    // in different hours must still come out as one row, and pieces unlike
    // as rows apart.
    #[test]
    fn pieces_alike_under_a_cut_are_one_row_whatever_their_hours() {
        let history = crate::history::History::memory();
        let config: crate::config::Config = serde_json::from_value(serde_json::json!({
            "sessions": "/none", "runs": "/none", "registry": "/none", "quests": "/none",
            "state": "/none", "zone": "UTC",
        }))
        .unwrap();
        turn(&history, "one", Some("QEST-A"), NOW - 3.0 * 3600.0);
        turn(&history, "two", Some("QEST-A"), NOW - 3600.0);
        turn(&history, "three", None, NOW - 2.0 * 3600.0);
        let picture = crate::picture::Picture::read(&history, &config, NOW).unwrap();
        let rows = |by: Cut| -> Vec<(String, f64)> {
            cut(&picture, Period::Today, by, 10)
                .rows
                .into_iter()
                .map(|row| (row.name, (row.dollars * 100.0).round() / 100.0))
                .collect()
        };
        assert_eq!(
            rows(Cut::Quest),
            [("QEST-A".to_owned(), 2.0), ("no quest".to_owned(), 1.0)],
            "a quest's pieces weren't one row, or no quest's joined it"
        );
        assert_eq!(
            rows(Cut::Session).len(),
            3,
            "sessions alike in their quest were taken as one"
        );
        assert_eq!(
            rows(Cut::Tool),
            [("pi".to_owned(), 3.0)],
            "one tool's pieces were split"
        );
    }

    #[test]
    fn periods_and_cuts_are_found_by_their_words() {
        assert_eq!(Period::parse("last week"), Some(Period::LastWeek));
        assert_eq!(Period::parse("this-month"), Some(Period::ThisMonth));
        assert_eq!(Cut::parse("quests"), Some(Cut::Quest));
        assert_eq!(Cut::parse("nonsense"), None);
    }

    /// A report with a name too long for a narrow terminal, and a note
    /// longer than any line: a cut where its rows have a share, sentences
    /// where they don't.
    fn a_wide_report(period: &str, share: f64, shown: &str) -> Report {
        let cut = share > 0.0;
        let row = |name: &str| Row {
            name: name.into(),
            dollars: 12.5,
            share,
            shown: shown.into(),
            estimated: false,
            cells: if cut {
                spend_cells(shown, share)
            } else {
                Vec::new()
            },
        };
        Report {
            title: "Spend by quest".into(),
            period: period.into(),
            from: "2026-11-07".into(),
            to: "2026-11-13".into(),
            total: 25.0,
            shown: "$25.00".into(),
            source: "pi's logs at list prices".into(),
            rest: None,
            columns: if cut {
                spend_columns("quest")
            } else {
                Vec::new()
            },
            rows: vec![
                row("QEST-20261112-0TUD09 Exquisite Command Lines for People and Machines"),
                row("short"),
            ],
            notes: vec![
                "Of $25.00 spent at list prices, $3.00 was cache written and $1.00 of that \
                 rebuilt context a session already had."
                    .into(),
            ],
        }
    }

    fn past(said: &str, cells: usize) -> Vec<&str> {
        said.lines()
            .filter(|line| layout::width(line) > cells)
            .collect()
    }

    #[test]
    fn a_cut_fits_a_narrow_terminal() {
        let said = render_text_in(
            &a_wide_report("last week", 0.5, "$12.50"),
            &Pen::plain(),
            Room::Cells(60),
        );
        assert!(past(&said, 60).is_empty(), "lines ran past 60:\n{said}");
        assert!(
            said.contains("  QEST-20261112-0TUD09 Exquisite Command") && said.contains('…'),
            "the long name wasn't cut at a whole word:\n{said}"
        );
    }

    #[test]
    fn a_cut_heads_its_columns_and_lines_its_figures_up_under_them() {
        let said = render_text_in(
            &a_wide_report("last week", 0.5, "$12.50"),
            &Pen::plain(),
            Room::Cells(60),
        );
        let head = said
            .lines()
            .find(|line| line.contains("SPENT"))
            .unwrap_or_else(|| panic!("the cut's columns weren't headed:\n{said}"));
        assert_eq!(
            head.split_whitespace().collect::<Vec<_>>(),
            ["QUEST", "SPENT", "SHARE"],
            "the cut was headed otherwise:\n{said}"
        );
        assert!(head.starts_with("  QUEST"), "the head isn't inset:\n{said}");
        let ends = |line: &str, word: &str| {
            line.find(word)
                .map(|at| layout::width(&line[..at + word.len()]))
        };
        for line in said.lines().filter(|line| line.contains("$12.50")) {
            assert_eq!(
                ends(line, "$12.50"),
                ends(head, "SPENT"),
                "a figure didn't end under its label:\n{said}"
            );
            assert_eq!(
                ends(line, "50%"),
                ends(head, "SHARE"),
                "a share didn't end under its label:\n{said}"
            );
        }
    }

    #[test]
    fn a_sentence_row_hangs_under_itself() {
        let sentence = "the default budget's cap rose from $100 to $200 at the start of the month";
        let said = render_text_in(
            &a_wide_report("caps", 0.0, sentence),
            &Pen::plain(),
            Room::Cells(60),
        );
        assert!(past(&said, 60).is_empty(), "lines ran past 60:\n{said}");
        let words = said.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            words.contains("$200 at the start of the month short"),
            "a sentence row lost its words:\n{said}"
        );
    }

    #[test]
    fn into_a_pipe_a_report_is_set_as_its_files_are() {
        let report = a_wide_report("last week", 0.5, "$12.50");
        let said = render_text_in(&report, &Pen::plain(), Room::Unbounded);
        assert_eq!(
            said,
            render(&report, Format::Text),
            "a report into a pipe differs from the one the daemon keeps"
        );
        // Its title, what it covers, a gap, the head and two rows, a gap
        // and its note.
        assert_eq!(
            said.lines().count(),
            8,
            "a pipe's report gave way somewhere:\n{said}"
        );
        assert!(
            said.contains("for People and Machines  $12.50"),
            "a pipe's name was cut:\n{said}"
        );
    }

    #[test]
    fn last_month_is_a_whole_month() {
        let zone = Zone::utc();
        // 2026-10-14 12:00 UTC.
        let now = crate::time::parse("2026-10-14T12:00:00Z").unwrap();
        let (from, to) = Period::LastMonth.days(&zone, now);
        assert_eq!(Zone::iso_day(from), "2026-09-01");
        assert_eq!(Zone::iso_day(to), "2026-09-30");
        let (monday, _) = Period::ThisWeek.days(&zone, now);
        assert_eq!(Zone::iso_day(monday), "2026-10-12");
    }
}
