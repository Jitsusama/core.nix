//! A period's spend cut one way: the periods and cuts a report is asked
//! by, its rows, and the cut that names and sums them.

use crate::money::{dollars, percent};
use crate::picture::Picture;
use crate::series::{self, Piece, Source};
use crate::time::Zone;
use serde::Serialize;
use std::collections::HashMap;
use terminal_kit::Align;

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
    Machine,
}

impl Cut {
    pub const ALL: [Cut; 10] = [
        Cut::Quest,
        Cut::Model,
        Cut::Agent,
        Cut::Repo,
        Cut::Session,
        Cut::Tool,
        Cut::Day,
        Cut::Source,
        Cut::Budget,
        Cut::Machine,
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
            Cut::Machine => "machine",
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
            Cut::Machine => &["machine", "machines", "host", "hosts"],
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
pub(super) fn spend_columns(noun: &'static str) -> Vec<(&'static str, Align)> {
    vec![
        (noun, Align::Left),
        ("spent", Align::Right),
        ("share", Align::Right),
    ]
}

/// A spend row's cells under those columns; a share of nothing says
/// nothing.
pub(super) fn spend_cells(shown: &str, share: f64) -> Vec<String> {
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
        Cut::Machine => ([&piece.machine, &piece.session, &piece.tool, ""], 0),
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
        Cut::Machine => machine_name(picture, piece),
    }
}

/// The machine a piece's work was done on: another machine by its name,
/// pi's spend no log here knows as elsewhere, another tool's as no
/// machine, since its bill doesn't say, and the rest this machine's.
fn machine_name(picture: &Picture, piece: &Piece) -> String {
    if !piece.machine.is_empty() {
        piece.machine.clone()
    } else if piece.session == crate::config::ELSEWHERE {
        crate::config::ELSEWHERE.into()
    } else if piece.tool != "pi" {
        format!("{}{}", piece.tool, crate::config::NO_MACHINE)
    } else {
        picture.machine.clone()
    }
}

pub(super) fn source_line(pieces: &[Piece]) -> String {
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
    // reports on 16 November. Groups are kept in the order of their first
    // piece, so a name's dollars are summed in the same order every pass.
    let mut at: HashMap<Fields, usize> = HashMap::new();
    let mut groups: Vec<(f64, bool, i64, &Piece)> = Vec::new();
    for piece in pieces {
        let group = *at.entry(fields(cut, piece)).or_insert_with(|| {
            groups.push((0.0, false, piece.hour, piece));
            groups.len() - 1
        });
        let entry = &mut groups[group];
        entry.0 += piece.dollars;
        entry.1 |= piece.source.estimated() && piece.dollars > 0.005;
        entry.2 = entry.2.min(piece.hour);
    }
    let mut grouped: HashMap<String, (f64, bool, i64)> = HashMap::new();
    for (spent, estimated, first, piece) in groups {
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
        // A day's hours are groups of their own that meet under its name.
        assert_eq!(
            rows(Cut::Day)
                .iter()
                .map(|(_, spent)| *spent)
                .collect::<Vec<_>>(),
            [3.0],
            "a day's hours weren't one row"
        );
    }

    // Each machine's sessions are its own under the machine cut: this
    // machine's by its name, another's by that machine's, and spend no
    // log explains as elsewhere.
    #[test]
    fn each_machine_s_sessions_are_its_own() {
        use crate::logs::Event;
        let history = crate::history::History::memory();
        let config: crate::config::Config = serde_json::from_value(serde_json::json!({
            "sessions": "/none", "runs": "/none", "registry": "/none", "quests": "/none",
            "state": "/none", "zone": "UTC", "machine": "slate",
        }))
        .unwrap();
        let sentinel = crate::config::ELSEWHERE;
        for (id, machine) in [
            ("here", None),
            ("there", Some("teal")),
            (sentinel, Some("umber")),
        ] {
            history
                .add_event(&Event::Session {
                    id: id.into(),
                    cwd: "/x".into(),
                    at: NOW - 4.0 * 3600.0,
                    kind: None,
                    machine: machine.map(str::to_owned),
                })
                .unwrap();
        }
        turn(&history, "here", None, NOW - 3.0 * 3600.0);
        turn(&history, "there", None, NOW - 2.0 * 3600.0);
        turn(&history, "there", None, NOW - 3600.0);
        turn(&history, sentinel, None, NOW - 3.5 * 3600.0);
        let picture = crate::picture::Picture::read(&history, &config, NOW).unwrap();
        let mut rows: Vec<(String, f64)> = cut(&picture, Period::Today, Cut::Machine, 10)
            .rows
            .into_iter()
            .map(|row| (row.name, (row.dollars * 100.0).round() / 100.0))
            .collect();
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(
            rows,
            [
                ("elsewhere".to_owned(), 1.0),
                ("slate".to_owned(), 1.0),
                ("teal".to_owned(), 2.0)
            ],
            "a machine's sessions weren't its own, or a brought log took elsewhere's spend"
        );
        // Each way a piece's machine is named: another machine's by its
        // name, spend no log knows as elsewhere, another tool's as its
        // tool's, and the rest this machine's.
        let mut piece = picture.pieces[0].clone();
        let named = |piece: &Piece| machine_name(&picture, piece);
        piece.session = "here".into();
        piece.machine = String::new();
        assert_eq!(named(&piece), "slate", "this machine's work wasn't its");
        "teal".clone_into(&mut piece.machine);
        assert_eq!(named(&piece), "teal", "another machine's work wasn't its");
        piece.machine = String::new();
        piece.session = sentinel.into();
        assert_eq!(
            named(&piece),
            "elsewhere",
            "unexplained spend wasn't elsewhere's"
        );
        piece.session = String::new();
        "codex".clone_into(&mut piece.tool);
        assert_eq!(
            named(&piece),
            "codex (no machine)",
            "another tool's bill was put on a machine"
        );
        assert_eq!(Cut::parse("hosts"), Some(Cut::Machine));
        // Through the cut, another tool's bill beside this machine's work
        // in the same hour and session is still its own row.
        let mut billed = picture.clone();
        let mut bill = billed
            .pieces
            .iter()
            .find(|piece| piece.session == "here")
            .unwrap()
            .clone();
        "codex".clone_into(&mut bill.tool);
        billed.pieces.push(bill);
        let rows: Vec<String> = cut(&billed, Period::Today, Cut::Machine, 10)
            .rows
            .into_iter()
            .map(|row| row.name)
            .collect();
        assert!(
            rows.iter().any(|name| name == "codex (no machine)")
                && rows.iter().any(|name| name == "slate"),
            "another tool's bill was cut together with this machine's work: {rows:?}"
        );
    }

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
