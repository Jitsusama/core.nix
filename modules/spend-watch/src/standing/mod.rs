//! How each budget stands, as `spend-watch budgets --json` says it and the
//! collection's Budgets row shows it: what each spent today, in the last
//! seven days and this month; each of its allowances in its own unit, the
//! nearest first; what a plan's turns were worth at list; and the whole,
//! in billed dollars only, which is metered spend plus each plan's price
//! over the days it covers. A share is never turned into dollars, so it's
//! never in the whole.

use crate::budgets::{Budgets, window_label, window_name};
use crate::caps::Unit;
use crate::config::{BudgetConfig, Config};
use crate::money::{maybe_about, percent, whole};
use crate::picture::{CapView, Picture};
use crate::series::Piece;
use crate::time::Zone;
use serde::Serialize;
use std::collections::BTreeMap;

pub const VERSION: u32 = 1;

/// What the whole says it is, since it holds no plan's use.
pub const WORDS: &str = "billed spend";

/// The periods each budget's spend is said for, by the names they're
/// keyed by.
const PERIODS: [&str; 3] = ["today", "7 days", "this month"];

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Amount {
    pub dollars: f64,
    pub shown: String,
    /// Whether any of it is pi's own estimate rather than a bill, which
    /// `shown` says as "about".
    #[serde(skip)]
    pub estimated: bool,
}

impl Amount {
    fn of(dollars: f64, estimated: bool) -> Amount {
        Amount {
            // An empty sum is negative zero, which would read as -0.
            dollars: dollars + 0.0,
            shown: maybe_about(whole(dollars), estimated),
            estimated,
        }
    }

    /// What some pieces cost together, an estimate if any piece that
    /// counts for anything is one.
    fn of_pieces<'a>(pieces: impl IntoIterator<Item = &'a Piece>) -> Amount {
        let (dollars, estimated) =
            pieces
                .into_iter()
                .fold((0.0, false), |(dollars, estimated), piece| {
                    (
                        dollars + piece.dollars,
                        estimated || (piece.source.estimated() && piece.dollars > 0.005),
                    )
                });
        Amount::of(dollars, estimated)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Total {
    #[serde(flatten)]
    pub periods: BTreeMap<String, Amount>,
    pub words: String,
}

/// One allowance as it stands, in its own unit.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Allowance {
    pub name: String,
    pub label: String,
    pub unit: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used: Option<f64>,
    /// The dollars it allows, for one in dollars.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowance: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spent: Option<f64>,
    pub window_hours: u32,
    pub warn: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resets: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// Whose reading it is: a feed's label, for an allowance a feed reads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    /// Its standing in its own unit, and how near it is, as a share.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub standing: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nearest: Option<f64>,
    /// Each of its facts as a person reads them, for the launcher's
    /// drawer, which formats nothing itself.
    pub said: AllowanceSaid,
}

/// An allowance's facts in words, each empty where it isn't known: how
/// long its window is, where it warns in its own unit, when it resets,
/// when an override of it ends and how it last changed, whose reading it
/// is and when that was taken, and what its feed says of it. `brief` is
/// how a share stands as its budget's drawer lists it beside its name,
/// where the window is already named: its figure alone and when it
/// resets, "85% \u{b7} resets 16:40", so it says what the budget's row
/// doesn't (VKCF3Z, 2026-11-08). A window in dollars has none: its
/// standing is too long to carry a reset and keep its meter beside it,
/// so its reset is said a step in, in its own drawer.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct AllowanceSaid {
    pub brief: String,
    pub window: String,
    pub warns: String,
    pub resets: String,
    pub ends: String,
    pub changed: String,
    pub read: String,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Budget {
    pub name: String,
    pub label: String,
    pub kind: Option<&'static str>,
    pub whose: Option<&'static str>,
    pub default: bool,
    /// A budget the logs or a feed named that the config doesn't declare:
    /// shown, never dropped, so the budgets sum to the whole.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub undeclared: bool,
    /// pi's providers whose turns are always this budget's, so pi can
    /// tell which budget a session on one of them draws on.
    pub providers: Vec<String>,
    pub spent: BTreeMap<String, Amount>,
    /// What a plan's turns were worth at list prices.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<BTreeMap<String, Amount>>,
    pub allowances: Vec<Allowance>,
    pub standing: String,
    pub nearest: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<String>,
    /// What it is, in a few words, as its row says it.
    pub words: String,
    /// Its facts in words, for the launcher's drawer.
    pub said: BudgetSaid,
}

/// A budget's facts in words, each empty where there's nothing to say:
/// what it is, as its drawer's eyebrow says it, whose money and what kind
/// after a dot as every eyebrow's where and kind are ("your own \u{b7}
/// plan"); what it spent over each period, what a plan's turns were worth
/// at list over each, how its feeds read, and the providers it always
/// draws for, each as it names itself ("Claude Code").
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct BudgetSaid {
    pub what: String,
    pub spent: String,
    pub worth: String,
    pub reading: String,
    pub providers: String,
}

/// One session's spend and the budget it draws on, when asked for.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Session {
    pub id: String,
    pub budget: Option<String>,
    /// All it has spent, over every day it ran.
    pub spent: Amount,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Standing {
    pub v: u32,
    pub at: String,
    pub default: String,
    pub total: Total,
    pub budgets: Vec<Budget>,
    /// Each session that spent today, by its whole id, to the budget it
    /// drew on last.
    pub sessions: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<Session>,
}

/// Whether an allowance is over: reached, or refusing as its feed says.
pub fn over(allowance: &Allowance) -> bool {
    allowance.nearest.unwrap_or(0.0) >= 1.0 || allowance.state.as_deref() == Some("refused")
}

/// How every budget stands now, and one session's spend when asked.
pub fn standing(picture: &Picture, config: &Config, session: Option<&str>) -> Standing {
    let budgets = &picture.budgets;
    let spans = spans(picture);
    let mut all: Vec<Budget> = names(picture)
        .into_iter()
        .map(|name| budget(picture, config, &spans, &name))
        .collect();
    // Nearest its allowance first; those with none after, as they're
    // declared.
    all.sort_by(|one, other| {
        other
            .nearest
            .unwrap_or(-1.0)
            .total_cmp(&one.nearest.unwrap_or(-1.0))
    });
    let today = picture.zone.day(picture.now);
    let mut sessions = last_budgets(picture.between(today, today));
    let session = session.map(|id| {
        let pieces: Vec<Piece> = picture
            .pieces
            .iter()
            .filter(|piece| piece.session == id)
            .cloned()
            .collect();
        let budget = last_budgets(&pieces).remove(id);
        if let Some(budget) = &budget {
            sessions.insert(id.to_owned(), budget.clone());
        }
        Session {
            id: id.to_owned(),
            budget,
            spent: Amount::of_pieces(&pieces),
        }
    });
    Standing {
        v: VERSION,
        at: crate::time::rfc3339(picture.now),
        default: budgets.default.clone(),
        total: Total {
            periods: total(picture, &spans),
            words: WORDS.into(),
        },
        budgets: all,
        sessions,
        session,
    }
}

/// Each period's local days, first and last, by its name.
type Spans<'a> = Vec<(&'static str, (i64, i64), &'a [Piece])>;

fn spans(picture: &Picture) -> Spans<'_> {
    let today = picture.zone.day(picture.now);
    let month = crate::report::Period::ThisMonth.days(&picture.zone, picture.now);
    [(today, today), (today - 6, today), month]
        .into_iter()
        .zip(PERIODS)
        .map(|((from, to), name)| (name, (from, to), picture.between(from, to)))
        .collect()
}

/// Every budget, declared or found in what was spent.
fn names(picture: &Picture) -> Vec<String> {
    let mut names: Vec<String> = picture
        .budgets
        .all
        .iter()
        .map(|budget| budget.name.clone())
        .collect();
    let found = picture
        .pieces
        .iter()
        .map(|piece| &piece.budget)
        .chain(picture.caps.iter().map(|view| &view.budget));
    for name in found {
        if !names.contains(name) {
            names.push(name.clone());
        }
    }
    names
}

fn spent(spans: &Spans, belongs: impl Fn(&Piece) -> bool) -> BTreeMap<String, Amount> {
    spans
        .iter()
        .map(|(name, _, pieces)| {
            let mine = pieces.iter().filter(|piece| belongs(piece));
            ((*name).to_owned(), Amount::of_pieces(mine))
        })
        .collect()
}

fn budget(picture: &Picture, config: &Config, spans: &Spans, name: &str) -> Budget {
    let declared = picture.budgets.declared(name);
    let mut allowances = allowances(picture, config, name, declared);
    allowances.sort_by(|one, other| {
        other
            .nearest
            .unwrap_or(-1.0)
            .total_cmp(&one.nearest.unwrap_or(-1.0))
    });
    let spent = spent(spans, |piece| piece.budget == name);
    let value = picture.budgets.is_plan(name).then(|| {
        spans
            .iter()
            .map(|(period, _, pieces)| {
                let list = pieces
                    .iter()
                    .filter(|piece| piece.budget == name)
                    .map(|piece| piece.list)
                    .sum();
                ((*period).to_owned(), Amount::of(list, false))
            })
            .collect::<BTreeMap<_, _>>()
    });
    let read = allowances
        .iter()
        .find(|allowance| allowance.nearest.is_some());
    let standing = match (read, &value) {
        (Some(allowance), _) => with_window(allowance),
        (None, Some(value)) => format!("{} at list this month", value["this month"].shown),
        (None, None) => format!("{} this month", spent["this month"].shown),
    };
    let health = health(picture, &picture.budgets, name);
    let fresh = month_in_week(spans);
    // A plan's turns are billed nothing but its price, so what they were
    // worth at list says its use and its spend says nothing.
    let said = BudgetSaid {
        what: what(declared),
        spent: if value.is_some() {
            String::new()
        } else {
            periods(&spent, fresh)
        },
        worth: value
            .as_ref()
            .map_or_else(String::new, |value| periods(value, fresh)),
        reading: health.as_deref().map_or_else(String::new, reading),
        providers: declared.map_or_else(String::new, |budget| {
            budget
                .providers
                .iter()
                .map(|id| config.provider_names.get(id).unwrap_or(id).as_str())
                .collect::<Vec<_>>()
                .join(" \u{b7} ")
        }),
    };
    Budget {
        name: name.to_owned(),
        label: declared.map_or_else(|| name.to_owned(), |budget| budget.label.clone()),
        kind: declared.map(|budget| budget.kind.name()),
        whose: declared.map(|budget| budget.whose.name()),
        default: picture.budgets.default == name,
        undeclared: declared.is_none(),
        providers: declared.map_or_else(Vec::new, |budget| budget.providers.clone()),
        nearest: allowances.iter().find_map(|allowance| allowance.nearest),
        health,
        words: words(declared),
        spent,
        value,
        allowances,
        standing,
        said,
    }
}

/// Whether this month began within the last 7 days, so all of it lies
/// inside them.
fn month_in_week(spans: &Spans) -> bool {
    let first = |period: &str| {
        spans
            .iter()
            .find(|(name, _, _)| *name == period)
            .map(|(_, (from, _), _)| *from)
    };
    matches!((first("this month"), first("7 days")), (Some(month), Some(week)) if month >= week)
}

/// Some amounts over each period, as one line: "$12 today \u{b7} $80 in
/// 7 days \u{b7} $140 this month". When the month began within the 7
/// days and they say the same, the 7 days add nothing, so only the
/// month is said (VKCF3Z, 2026-11-07).
fn periods(amounts: &BTreeMap<String, Amount>, month_in_week: bool) -> String {
    let shown = |period: &str| amounts.get(period).map(|amount| amount.shown.as_str());
    let redundant =
        month_in_week && shown("7 days").is_some() && shown("7 days") == shown("this month");
    PERIODS
        .iter()
        .filter(|period| !(redundant && **period == "7 days"))
        .filter_map(|period| {
            let amount = amounts.get(*period)?;
            Some(match *period {
                "7 days" => format!("{} in 7 days", amount.shown),
                _ => format!("{} {period}", amount.shown),
            })
        })
        .collect::<Vec<_>>()
        .join(" \u{b7} ")
}

/// How a budget's feeds read, as its drawer says it; nothing for one
/// whose only feed is optional and not signed in for, since nothing's
/// wrong with it.
fn reading(health: &str) -> String {
    match health {
        "fine" => "reading fine",
        "stalled" => "stalled",
        "signed-out" => "signed out",
        "error" => "can't be read",
        _ => "",
    }
    .into()
}

/// A budget's standing, its nearest window's, with that window said
/// where it counts dollars, since a budget's row isn't named for its
/// window as a window's own row is: "$250 of $300 today" (VKCF3Z,
/// 2026-11-08). A share's standing already says its window ("85% of
/// 5 h").
fn with_window(allowance: &Allowance) -> String {
    if allowance.unit == Unit::Share.name() {
        return allowance.standing.clone();
    }
    let over = match allowance.window_hours {
        24 => "today".to_owned(),
        168 => "this week".to_owned(),
        hours => format!("in {}", window_length(hours)),
    };
    format!("{} {over}", allowance.standing)
}

/// What a budget is, in a few words: whose money and what kind. An
/// employer's cap says only who pays, since its label already names the
/// credential, and the kind's code name never reaches a row (VKCF3Z,
/// 2026-11-07).
fn words(declared: Option<&BudgetConfig>) -> String {
    whose_and_kind(declared).map_or_else(
        || "not declared".into(),
        |(whose, kind)| format!("{whose} {kind}"),
    )
}

/// The same as its drawer's eyebrow says it, whose and what kind after
/// a dot, as every eyebrow's where and kind are (VKCF3Z, 2026-11-08).
fn what(declared: Option<&BudgetConfig>) -> String {
    whose_and_kind(declared).map_or_else(
        || "not declared".into(),
        |(whose, kind)| format!("{whose} \u{b7} {kind}"),
    )
}

fn whose_and_kind(declared: Option<&BudgetConfig>) -> Option<(&'static str, &'static str)> {
    use crate::config::{BudgetKind, Whose};
    let budget = declared?;
    let whose = match budget.whose {
        Whose::Own => "your own",
        Whose::Employer => "the employer's",
        Whose::Team => "a team's",
    };
    let kind = match budget.kind {
        BudgetKind::Plan => "plan",
        BudgetKind::Key => "key",
        BudgetKind::Cap => "money",
        BudgetKind::Team => "token",
    };
    Some((whose, kind))
}

/// A budget's allowances: each one read or counted, and each declared
/// share no feed has read yet.
fn allowances(
    picture: &Picture,
    config: &Config,
    name: &str,
    declared: Option<&BudgetConfig>,
) -> Vec<Allowance> {
    let mut allowances: Vec<Allowance> = picture
        .caps
        .iter()
        .filter(|view| view.budget == name)
        .map(|view| allowance(picture, config, view))
        .collect();
    let unread = declared
        .into_iter()
        .flat_map(|budget| &budget.allowances)
        .filter(|declared| !allowances.iter().any(|read| read.name == declared.name))
        .filter(|declared| !declared.stands_in)
        .map(|declared| Allowance {
            name: declared.name.clone(),
            label: window_name(declared.window_hours),
            unit: declared.unit.name(),
            used: None,
            allowance: declared.allowance,
            spent: None,
            window_hours: declared.window_hours,
            warn: declared.warn,
            resets: None,
            state: None,
            from: None,
            at: None,
            standing: String::new(),
            nearest: None,
            said: AllowanceSaid {
                window: window_length(declared.window_hours),
                warns: warns(declared.unit, declared.allowance, declared.warn),
                ..AllowanceSaid::default()
            },
        })
        .collect::<Vec<_>>();
    allowances.extend(unread);
    allowances
}

fn allowance(picture: &Picture, config: &Config, view: &CapView) -> Allowance {
    let cap = &view.cap;
    // Its name is its feed's, or its window's for one of the watch's own;
    // its figure says the window short ("62% of 5 h"), whatever it's named.
    let label = cap.label.clone();
    let fed = !cap.feed.is_empty();
    let (standing, nearest) = match (cap.unit, view.used, cap.allowance) {
        (Unit::Share, Some(used), _) => (
            format!("{} of {}", percent(used), window_label(cap.window_hours)),
            Some(used),
        ),
        (Unit::Dollars, _, Some(allowance)) => (
            format!("{} of {}", whole(view.spent), whole(allowance)),
            Some(view.spent / allowance),
        ),
        _ => (format!("{} spent", whole(view.spent)), None),
    };
    let from = config
        .feeds
        .iter()
        .find(|feed| feed.name == cap.feed)
        .map_or_else(|| cap.feed.clone(), |feed| feed.label.clone());
    let share = cap.unit == Unit::Share;
    let clock = |at: f64| picture.zone.clock(at, picture.now);
    let brief = match (cap.unit, view.used) {
        (Unit::Share, Some(used)) => view.resets.map_or_else(
            || percent(used),
            |at| format!("{} \u{b7} resets {}", percent(used), clock(at)),
        ),
        _ => String::new(),
    };
    let said = AllowanceSaid {
        brief,
        window: window_length(cap.window_hours),
        warns: warns(cap.unit, cap.allowance, cap.warn),
        resets: view.resets.map_or_else(String::new, clock),
        ends: override_ends(cap),
        changed: last_changed(picture, cap),
        read: if fed {
            format!("{from} at {}", clock(cap.at))
        } else {
            String::new()
        },
        state: if fed {
            stands(&cap.state)
        } else {
            String::new()
        },
    };
    Allowance {
        name: cap.name.clone(),
        label,
        unit: cap.unit.name(),
        used: view.used,
        allowance: cap.allowance,
        spent: (!share).then_some(view.spent),
        window_hours: cap.window_hours,
        warn: cap.warn,
        resets: view.resets.map(crate::time::rfc3339),
        state: Some(cap.state.clone()),
        from: fed.then_some(from),
        at: fed.then(|| crate::time::rfc3339(cap.at)),
        standing,
        nearest,
        said,
    }
}

/// The day an override of an allowance ends, a qualifier after a dot when
/// its feed didn't say and it's guessed (VKCF3Z, 2026-11-07).
fn override_ends(cap: &crate::caps::Cap) -> String {
    cap.ends.map_or_else(String::new, |day| {
        let said = Zone::day_name(day);
        if cap.ends_known {
            said
        } else {
            format!("{said} \u{b7} a guess")
        }
    })
}

/// How an allowance last changed, what changed leading and when after it:
/// "raised $30,000 to $40,000 \u{b7} Thu 11:31" (VKCF3Z, 2026-11-07).
/// A change of state isn't a change of the allowance, so it's not one.
fn last_changed(picture: &Picture, cap: &crate::caps::Cap) -> String {
    let limit = |amount: Option<f64>| amount.map_or_else(|| "no limit".to_owned(), whole);
    picture
        .events
        .iter()
        .filter(|event| {
            event.feed == cap.feed
                && event.budget == cap.budget
                && event.name == cap.name
                && matches!(event.kind.as_str(), "lowered" | "raised" | "lapsed")
                && event.before != event.after
        })
        .max_by(|one, other| one.at.total_cmp(&other.at))
        .map_or_else(String::new, |event| {
            format!(
                "{} {} to {} \u{b7} {}",
                event.kind,
                limit(event.before),
                limit(event.after),
                picture.zone.clock(event.at, picture.now)
            )
        })
}

/// How long a window is, as a person says it: "5 hours", "a day", "a
/// week".
fn window_length(hours: u32) -> String {
    match hours {
        1 => "an hour".into(),
        24 => "a day".into(),
        168 => "a week".into(),
        hours if hours % 168 == 0 => format!("{} weeks", hours / 168),
        hours if hours % 24 == 0 => format!("{} days", hours / 24),
        hours => format!("{hours} hours"),
    }
}

/// Where an allowance warns, in its own unit, so one in dollars never
/// reads as a share: "at $36,000", or "at 90%" of a share. One in
/// dollars with no amount warns at nothing it can say.
fn warns(unit: Unit, allowance: Option<f64>, warn: f64) -> String {
    match (unit, allowance) {
        (Unit::Share, _) => format!("at {}", percent(warn)),
        (Unit::Dollars, Some(allowance)) => format!("at {}", whole(allowance * warn)),
        (Unit::Dollars, None) => String::new(),
    }
}

/// What a feed says of its allowance, where it says more than that it's
/// within it, which its standing already shows.
fn stands(state: &str) -> String {
    match state {
        "notified" => "warned",
        "refused" => "refusing requests",
        _ => "",
    }
    .into()
}

/// How the sources of the feeds for a budget are: the first that isn't
/// fine, or fine; none for a budget no feed is for.
fn health(picture: &Picture, budgets: &Budgets, name: &str) -> Option<String> {
    let feeds = budgets.feeds_of(name);
    let sources: Vec<&str> = picture
        .sources
        .iter()
        .filter(|source| feeds.contains(&source.feed))
        .map(|source| source.state.as_str())
        .collect();
    if sources.is_empty() {
        return None;
    }
    let down = sources
        .into_iter()
        .find(|state| !matches!(*state, "fine" | "optional"));
    Some(down.unwrap_or("fine").to_owned())
}

/// The whole in billed dollars: what every budget spent, which counts a
/// plan's turns as nothing, and each plan's price over the period, its
/// month's price in the month and a day's share of it in a day.
fn total(picture: &Picture, spans: &Spans) -> BTreeMap<String, Amount> {
    let metered = spent(spans, |_| true);
    let monthly: f64 = picture
        .budgets
        .all
        .iter()
        .filter_map(|budget| budget.price.map(|price| price.dollars))
        .sum();
    spans
        .iter()
        .map(|(name, (from, to), _)| {
            let plans: f64 = if *name == "this month" {
                monthly
            } else {
                (*from..=*to).map(|day| monthly / days_in_month(day)).sum()
            };
            let spent = &metered[*name];
            (
                (*name).to_owned(),
                Amount::of(spent.dollars + plans, spent.estimated),
            )
        })
        .collect()
}

/// How many days the month a day is in has.
#[expect(
    clippy::cast_precision_loss,
    reason = "a month has at most 31 days, which f64 holds exactly"
)]
fn days_in_month(day: i64) -> f64 {
    let first = |day: i64| {
        let iso = Zone::iso_day(day);
        crate::time::parse_day(&format!("{}-01", &iso[..7])).unwrap_or(day)
    };
    let this = first(day);
    (first(this + 31) - this) as f64
}

/// Each session's budget as it drew on it last, by its whole id.
fn last_budgets(pieces: &[Piece]) -> BTreeMap<String, String> {
    pieces
        .iter()
        .filter(|piece| !piece.session.is_empty() && piece.session != "elsewhere")
        .map(|piece| (piece.session.clone(), piece.budget.clone()))
        .collect()
}

#[cfg(test)]
mod tests;
