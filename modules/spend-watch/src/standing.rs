//! How each budget stands, as `spend-watch budgets --json` says it and the
//! collection's Budgets row shows it: what each spent today, in the last
//! seven days and this month; each of its limits in its own unit, the
//! nearest first; what a subscription's turns were worth at list; and the
//! whole, in billed dollars only, which is metered spend plus each
//! subscription's price over the days it covers. A share is never turned
//! into dollars, so it's never in the whole.

use crate::budgets::{Budgets, window_label};
use crate::caps::Unit;
use crate::config::{BudgetConfig, Config};
use crate::money::{maybe_about, percent, whole};
use crate::picture::{CapView, Picture};
use crate::series::Piece;
use crate::time::Zone;
use serde::Serialize;
use std::collections::BTreeMap;

pub const VERSION: u32 = 1;

/// What the whole says it is, since it holds no subscription's use.
pub const WORDS: &str = "billed spend";

/// The periods each budget's spend is said for, by the names they're
/// keyed by.
const PERIODS: [&str; 3] = ["today", "7 days", "this month"];

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Amount {
    pub dollars: f64,
    pub shown: String,
}

impl Amount {
    fn of(dollars: f64, estimated: bool) -> Amount {
        Amount {
            // An empty sum is negative zero, which would read as -0.
            dollars: dollars + 0.0,
            shown: maybe_about(whole(dollars), estimated),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Total {
    #[serde(flatten)]
    pub periods: BTreeMap<String, Amount>,
    pub words: String,
}

/// One limit as it stands, in its own unit.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Limit {
    pub name: String,
    pub label: String,
    pub unit: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used: Option<f64>,
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
    /// Whose reading it is: a feed's label, for a limit a feed reads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    /// Its standing in its own unit, and how near it is, as a share.
    #[serde(skip)]
    pub standing: String,
    #[serde(skip)]
    pub nearest: Option<f64>,
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
    pub spent: BTreeMap<String, Amount>,
    /// What a subscription's turns were worth at list prices.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<BTreeMap<String, Amount>>,
    pub limits: Vec<Limit>,
    pub standing: String,
    pub nearest: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<String>,
    /// What it is, in a few words, for the collection's row.
    #[serde(skip)]
    pub words: String,
}

/// One session's spend and the budget it draws on, when asked for.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Session {
    pub id: String,
    pub budget: Option<String>,
    pub spent: BTreeMap<String, Amount>,
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

/// How every budget stands now, and one session's spend when asked.
pub fn standing(picture: &Picture, config: &Config, session: Option<&str>) -> Standing {
    let budgets = &picture.budgets;
    let spans = spans(picture);
    let mut all: Vec<Budget> = names(picture)
        .into_iter()
        .map(|name| budget(picture, config, &spans, &name))
        .collect();
    // Nearest its limit first; those with none after, as they're declared.
    all.sort_by(|one, other| {
        other
            .nearest
            .unwrap_or(-1.0)
            .total_cmp(&one.nearest.unwrap_or(-1.0))
    });
    let today = picture.zone.day(picture.now);
    let mut sessions = last_budgets(&picture.between(today, today));
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
            spent: spent(&spans, |piece| piece.session == id),
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
type Spans<'a> = Vec<(&'static str, (i64, i64), Vec<Piece>)>;

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
            let mine: Vec<&Piece> = pieces.iter().filter(|piece| belongs(piece)).collect();
            let dollars = mine.iter().map(|piece| piece.dollars).sum();
            let estimated = mine
                .iter()
                .any(|piece| piece.source.estimated() && piece.dollars > 0.005);
            ((*name).to_owned(), Amount::of(dollars, estimated))
        })
        .collect()
}

fn budget(picture: &Picture, config: &Config, spans: &Spans, name: &str) -> Budget {
    let declared = picture.budgets.declared(name);
    let mut limits = limits(picture, config, name, declared);
    limits.sort_by(|one, other| {
        other
            .nearest
            .unwrap_or(-1.0)
            .total_cmp(&one.nearest.unwrap_or(-1.0))
    });
    let spent = spent(spans, |piece| piece.budget == name);
    let subscription = picture.budgets.subscription(name);
    let value = subscription.then(|| {
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
    let standing = match (limits.iter().find(|limit| limit.nearest.is_some()), &value) {
        (Some(limit), _) => limit.standing.clone(),
        (None, Some(value)) => format!("{} at list this month", value["this month"].shown),
        (None, None) => format!("{} this month", spent["this month"].shown),
    };
    Budget {
        name: name.to_owned(),
        label: declared.map_or_else(|| name.to_owned(), |budget| budget.label.clone()),
        kind: declared.map(|budget| budget.kind.name()),
        whose: declared.map(|budget| budget.whose.name()),
        default: picture.budgets.default == name,
        undeclared: declared.is_none(),
        nearest: limits.iter().find_map(|limit| limit.nearest),
        health: health(picture, &picture.budgets, name),
        words: words(declared),
        spent,
        value,
        limits,
        standing,
    }
}

/// What a budget is, in a few words: whose money and what kind.
fn words(declared: Option<&BudgetConfig>) -> String {
    use crate::config::{BudgetKind, Whose};
    let Some(budget) = declared else {
        return "not declared".into();
    };
    let whose = match budget.whose {
        Whose::Own => "your own",
        Whose::Employer => "the employer's",
        Whose::Team => "a team's",
    };
    let kind = match budget.kind {
        BudgetKind::Subscription => "subscription",
        BudgetKind::Key => "key",
        BudgetKind::Cap => "cap",
        BudgetKind::Team => "token",
    };
    format!("{whose} {kind}")
}

/// A budget's limits: each one read or counted, and each declared share
/// no feed has read yet.
fn limits(
    picture: &Picture,
    config: &Config,
    name: &str,
    declared: Option<&BudgetConfig>,
) -> Vec<Limit> {
    let mut limits: Vec<Limit> = picture
        .caps
        .iter()
        .filter(|view| view.budget == name)
        .map(|view| limit(config, view))
        .collect();
    let unread = declared
        .into_iter()
        .flat_map(|budget| &budget.limits)
        .filter(|declared| !limits.iter().any(|limit| limit.name == declared.name))
        .filter(|declared| !declared.stands_in)
        .map(|declared| Limit {
            name: declared.name.clone(),
            label: window_label(declared.window_hours),
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
        })
        .collect::<Vec<_>>();
    limits.extend(unread);
    limits
}

fn limit(config: &Config, view: &CapView) -> Limit {
    let cap = &view.cap;
    let fed = !cap.feed.is_empty();
    let label = if fed {
        cap.label.clone()
    } else {
        window_label(cap.window_hours)
    };
    let (standing, nearest) = match (cap.unit, view.used, cap.allowance) {
        (Unit::Share, Some(used), _) => (format!("{} of {label}", percent(used)), Some(used)),
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
    Limit {
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
    }
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
/// subscription's turns as nothing, and each subscription's price over
/// the period, its month's price in the month and a day's share of it in
/// a day.
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
            let estimated = spent.shown.starts_with("about");
            (
                (*name).to_owned(),
                Amount::of(spent.dollars + plans, estimated),
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
mod tests {
    use super::*;
    use crate::history::History;
    use crate::logs::{Cost, Event, Kind, Tokens, Turn};

    const NOW: f64 = 1_791_988_200.0; // 2026-10-14T14:30:00Z, a Wednesday

    fn config() -> Config {
        serde_json::from_value(serde_json::json!({
            "sessions": "/none", "runs": "/none", "registry": "/none", "quests": "/none",
            "state": "/none", "zone": "UTC",
            "budgets": [
                {"name": "proxy", "label": "Your proxy cap", "kind": "cap",
                 "whose": "employer", "default": true,
                 "limits": [{"name": "weekly", "allowance": 40000, "windowHours": 168,
                             "warn": 0.9}]},
                {"name": "claude", "label": "Claude Pro", "kind": "subscription",
                 "whose": "own", "providers": ["claude-code"],
                 "price": {"dollars": 31, "per": "month"},
                 "limits": [{"name": "five-hours", "unit": "share", "windowHours": 5,
                             "warn": 0.8},
                            {"name": "week", "unit": "share", "windowHours": 168,
                             "warn": 0.8}]},
                {"name": "delta", "label": "Delta team token", "kind": "team", "whose": "team"},
            ],
            "feeds": [{"name": "plan", "label": "Claude", "command": ["f"],
                       "budget": "claude"}],
        }))
        .unwrap()
    }

    fn turn(history: &History, session: &str, provider: &str, budget: &str, cost: f64) {
        history
            .add_event(&Event::Turn(Turn {
                session: session.into(),
                at: NOW - 600.0,
                model: "m".into(),
                provider: provider.into(),
                kind: Kind::Agent,
                tokens: Tokens::default(),
                cost: Cost {
                    total: cost,
                    ..Cost::default()
                },
                quest: None,
                budget: Some(budget.into()),
                gap: None,
                rebuilt: 0.0,
                rebuilt_cost: 0.0,
                cause: None,
            }))
            .unwrap();
    }

    fn history() -> History {
        let history = History::memory();
        turn(&history, "me", "anthropic", "", 18_400.0);
        turn(&history, "team", "anthropic", "delta", 310.0);
        turn(&history, "plan", "claude-code", "", 84.0);
        turn(&history, "probe", "anthropic", "probe", 1.0);
        let share = serde_json::json!({
            "v": 1, "kind": "cap", "name": "five-hours", "label": "5 h", "unit": "share",
            "used": 0.62, "windowHours": 5, "warn": 0.8, "state": "ok",
            "resets": "2026-10-14T16:20:00Z", "at": "2026-10-14T14:20:00Z",
        });
        let record = crate::feed::parse(&share.to_string()).unwrap();
        crate::feed::store(&history, "plan", &record, NOW).unwrap();
        history
    }

    fn standing_now(session: Option<&str>) -> Standing {
        let config = config();
        let picture = Picture::read(&history(), &config, NOW).unwrap();
        standing(&picture, &config, session)
    }

    // Each budget stands in its own unit, the nearest its limit first, and
    // one nobody declared is shown rather than dropped.
    #[test]
    fn each_budget_stands_in_its_own_unit_nearest_first() {
        let standing = standing_now(None);
        let order: Vec<(&str, &str)> = standing
            .budgets
            .iter()
            .map(|budget| (budget.name.as_str(), budget.standing.as_str()))
            .collect();
        assert_eq!(
            order,
            [
                ("claude", "62% of 5 h"),
                ("proxy", "$18,400 of $40,000"),
                ("delta", "$310 this month"),
                ("probe", "$1 this month"),
            ]
        );
        assert_eq!(standing.default, "proxy");
        let probe = &standing.budgets[3];
        assert!(probe.undeclared && probe.kind.is_none());
        // The plan's week has no reading yet, and is still a limit.
        let claude = &standing.budgets[0];
        let limits: Vec<&str> = claude
            .limits
            .iter()
            .map(|limit| limit.name.as_str())
            .collect();
        assert_eq!(limits, ["five-hours", "week"]);
        assert_eq!(claude.limits[0].from.as_deref(), Some("Claude"));
        assert_eq!(claude.health, None);
    }

    // The whole is billed dollars: the plan's turns cost nothing but its
    // price, a day's share of it today and all of it this month, and
    // their worth at list is said beside the plan alone.
    #[test]
    fn the_whole_is_billed_spend() {
        let standing = standing_now(None);
        let claude = &standing.budgets[0];
        assert!(claude.spent["today"].dollars.abs() < 1e-9);
        assert_eq!(claude.value.as_ref().unwrap()["7 days"].shown, "$84");
        let total = &standing.total;
        assert_eq!(total.words, "billed spend");
        // October has 31 days, so a day's share of $31 is $1.
        assert!((total.periods["today"].dollars - (18_400.0 + 310.0 + 1.0 + 1.0)).abs() < 1e-9);
        assert!((total.periods["this month"].dollars - (18_711.0 + 31.0)).abs() < 1e-9);
        let json = serde_json::to_value(&standing).unwrap();
        assert_eq!(json["total"]["today"]["shown"], "$18,712");
        assert_eq!(json["budgets"][1]["limits"][0]["unit"], "dollars");
        assert!(json["budgets"][0]["limits"][0].get("spent").is_none());
    }

    #[test]
    fn a_session_says_its_spend_under_its_budget() {
        let standing = standing_now(Some("team"));
        let session = standing.session.unwrap();
        assert_eq!(session.budget.as_deref(), Some("delta"));
        assert_eq!(session.spent["today"].shown, "$310");
        assert_eq!(standing.sessions["plan"], "claude");
    }
}
