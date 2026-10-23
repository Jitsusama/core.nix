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
}

impl Cut {
    pub const ALL: [Cut; 8] = [
        Cut::Quest,
        Cut::Model,
        Cut::Agent,
        Cut::Repo,
        Cut::Session,
        Cut::Tool,
        Cut::Day,
        Cut::Source,
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
    let total = series::total(&pieces);
    // Each name's dollars, whether any are estimated, and its first hour.
    let mut grouped: HashMap<String, (f64, bool, i64)> = HashMap::new();
    for piece in &pieces {
        let entry = grouped
            .entry(key(picture, cut, piece))
            .or_insert((0.0, false, piece.hour));
        entry.0 += piece.dollars;
        entry.1 |= piece.source.estimated() && piece.dollars > 0.005;
        entry.2 = entry.2.min(piece.hour);
    }
    let mut rows: Vec<(i64, Row)> = grouped
        .into_iter()
        .filter(|(_, (spent, _, _))| *spent >= 0.005)
        .map(|(name, (spent, estimated, first))| {
            (
                first,
                Row {
                    name,
                    dollars: spent,
                    share: if total > 0.0 { spent / total } else { 0.0 },
                    shown: crate::money::maybe_about(dollars(spent), estimated),
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
        shown: crate::money::maybe_about(dollars(total), series::estimated(&pieces)),
        source: source_line(&pieces),
        rest: left_out,
        rows,
        notes,
    }
}

/// What each pi session has spent in a period, by its whole id, as a
/// figure ready to draw beside it, so the shell can say what a session it
/// sees costs without reading a report's names. Another tool's spend and
/// spend no log knows have no session, and a session that spent less than
/// a cent says nothing.
pub fn by_session(picture: &Picture, period: Period) -> std::collections::BTreeMap<String, String> {
    let (from, to) = period.days(&picture.zone, picture.now);
    let mut spent: HashMap<&str, (f64, bool)> = HashMap::new();
    let pieces = picture.between(from, to);
    for piece in &pieces {
        if piece.session.is_empty() || piece.session == "elsewhere" {
            continue;
        }
        let entry = spent.entry(piece.session.as_str()).or_insert((0.0, false));
        entry.0 += piece.dollars;
        entry.1 |= piece.source.estimated() && piece.dollars > 0.005;
    }
    spent
        .into_iter()
        .filter(|(_, (dollars_spent, _))| *dollars_spent >= 0.005)
        .map(|(session, (dollars_spent, estimated))| {
            (
                session.to_owned(),
                crate::money::maybe_about(dollars(dollars_spent), estimated),
            )
        })
        .collect()
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
    let pieces = picture.between(from, to);
    for piece in &pieces {
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

/// One kind of run a session started, and what it cost.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Helper {
    pub on: String,
    pub spent: String,
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
        source: source_line(&week),
        rest: None,
        rows,
        notes,
    }
}

pub fn cap_line(picture: &Picture, view: &crate::picture::CapView) -> String {
    let cap = &view.cap;
    let Some(allowance) = cap.allowance else {
        return format!("{}: no limit, {} spent.", cap.label, whole(view.spent));
    };
    let mut line = format!(
        "{}: {} of {} ({}).",
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
        .map(|(cause, spent)| Row {
            name: crate::logs::Cause::words(cause).to_owned(),
            dollars: *spent,
            share: if waste.rebuilt > 0.0 {
                spent / waste.rebuilt
            } else {
                0.0
            },
            shown: dollars(*spent),
            estimated: true,
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
    }
}

/// What each quest has cost, beside where it stands.
pub fn quests(picture: &Picture, period: Period) -> Report {
    let (from, to) = period.days(&picture.zone, picture.now);
    let pieces = picture.between(from, to);
    let mut by_root: HashMap<&str, f64> = HashMap::new();
    for piece in &pieces {
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
            Row {
                name: title,
                dollars: spent,
                share: if total > 0.0 { spent / total } else { 0.0 },
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
        source: source_line(&pieces),
        rest: None,
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
        Format::Text => render_text(report),
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

/// A report as plain text, its rows in columns.
fn render_text(report: &Report) -> String {
    const NAME: usize = 60;
    let mut out = format!("{}\n", report.title);
    if !report.shown.is_empty() {
        _ = writeln!(
            out,
            "{} from {} to {}, {}.",
            report.shown, report.from, report.to, report.source
        );
    }
    out.push('\n');
    let width = report
        .rows
        .iter()
        .map(|row| row.name.chars().count())
        .max()
        .unwrap_or(0)
        .min(NAME);
    // A cut's figures line up on the right; a sentence doesn't.
    let figures = report.rows.iter().all(|row| row.share > 0.0);
    let shown_width = if figures {
        report
            .rows
            .iter()
            .map(|row| row.shown.chars().count())
            .max()
            .unwrap_or(0)
    } else {
        0
    };
    for row in &report.rows {
        let name: String = row.name.chars().take(NAME).collect();
        let share = if row.share > 0.0 {
            format!("  {:>4}", percent(row.share))
        } else {
            String::new()
        };
        _ = writeln!(out, "  {name:<width$}  {:>shown_width$}{share}", row.shown);
    }
    if !report.rows.is_empty() && !report.notes.is_empty() {
        out.push('\n');
    }
    for note in &report.notes {
        _ = writeln!(out, "{note}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn periods_and_cuts_are_found_by_their_words() {
        assert_eq!(Period::parse("last week"), Some(Period::LastWeek));
        assert_eq!(Period::parse("this-month"), Some(Period::ThisMonth));
        assert_eq!(Cut::parse("quests"), Some(Cut::Quest));
        assert_eq!(Cut::parse("nonsense"), None);
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
