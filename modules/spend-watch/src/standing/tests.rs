//! The tests of how each budget stands.

use super::*;
use crate::history::History;
use crate::logs::{Cost, Event, Kind, Tokens, Turn};

const NOW: f64 = 1_791_988_200.0; // 2026-10-14T14:30:00Z, a Wednesday

#[test]
fn a_budget_s_words_never_say_what_a_window_s_name_never_says() {
    let said = |kind: &str, whose: &str| {
        let budget: BudgetConfig = serde_json::from_value(serde_json::json!({
            "name": "b", "label": "B", "kind": kind, "whose": whose
        }))
        .unwrap();
        words(Some(&budget))
    };
    assert_eq!(said("cap", "employer"), "the employer's money");
    assert_eq!(said("plan", "own"), "your own plan");
    assert_eq!(said("team", "team"), "a team's token");
    for kind in ["plan", "key", "cap", "team"] {
        for whose in ["own", "employer", "team"] {
            let words = said(kind, whose);
            assert!(
                !words.split_whitespace().any(crate::feed::unsaid),
                "{kind} {whose}: {words}"
            );
        }
    }
}

fn config() -> Config {
    serde_json::from_value(serde_json::json!({
        "sessions": "/none", "runs": "/none", "registry": "/none", "quests": "/none",
        "state": "/none", "zone": "UTC",
        "budgets": [
            {"name": "proxy", "label": "Your proxy cap", "kind": "cap",
             "whose": "employer", "default": true,
             "allowances": [{"name": "weekly", "allowance": 40000, "windowHours": 168,
                             "warn": 0.9}]},
            {"name": "claude", "label": "Claude Pro", "kind": "plan",
             "whose": "own", "providers": ["claude-code"],
             "price": {"dollars": 31, "per": "month"},
             "allowances": [{"name": "five-hours", "unit": "share", "windowHours": 5,
                             "warn": 0.8},
                            {"name": "week", "unit": "share", "windowHours": 168,
                             "warn": 0.8}]},
            {"name": "delta", "label": "Delta team token", "kind": "team", "whose": "team"},
        ],
        "feeds": [{"name": "plan", "label": "Claude", "command": ["f"],
                   "budget": "claude"}],
        "providerNames": {"claude-code": "Claude Code"},
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
        "v": 1, "kind": "cap", "name": "five-hours", "label": "Five Hours", "unit": "share",
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

// Each budget stands in its own unit, the nearest its allowance first,
// and one nobody declared is shown rather than dropped.
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
            // A budget's row isn't named for its window, so a
            // standing in dollars says it.
            ("proxy", "$18,400 of $40,000 this week"),
            ("delta", "$310 this month"),
            ("probe", "$1 this month"),
        ]
    );
    assert_eq!(standing.default, "proxy");
    let probe = &standing.budgets[3];
    assert!(
        probe.undeclared && probe.kind.is_none() && probe.providers.is_empty(),
        "a budget nobody declared wasn't shown bare and undeclared: {:?} {:?} {:?}",
        probe.undeclared,
        probe.kind,
        probe.providers
    );
    // The plan's week has no reading yet, and is still an allowance.
    let claude = &standing.budgets[0];
    let allowances: Vec<&str> = claude
        .allowances
        .iter()
        .map(|allowance| allowance.name.as_str())
        .collect();
    assert_eq!(allowances, ["five-hours", "week"]);
    assert_eq!(claude.allowances[0].from.as_deref(), Some("Claude"));
    assert_eq!(claude.health, None);
    assert_eq!(claude.providers, ["claude-code"]);
}

// The whole is billed dollars: the plan's turns cost nothing but its
// price, a day's share of it today and all of it this month, and
// their worth at list is said beside the plan alone.
#[test]
fn the_whole_is_billed_spend() {
    let standing = standing_now(None);
    let claude = &standing.budgets[0];
    assert!(
        claude.spent["today"].dollars.abs() < 1e-9,
        "a plan's turns today billed {}, not nothing",
        claude.spent["today"].dollars
    );
    assert_eq!(claude.value.as_ref().unwrap()["7 days"].shown, "$84");
    let total = &standing.total;
    assert_eq!(total.words, "billed spend");
    // October has 31 days, so a day's share of $31 is $1.
    for (period, billed) in [
        ("today", 18_400.0 + 310.0 + 1.0 + 1.0),
        ("this month", 18_711.0 + 31.0),
    ] {
        let whole = total.periods[period].dollars;
        assert!(
            (whole - billed).abs() < 1e-9,
            "the whole {period} is {whole}, not ${billed}"
        );
    }
    let json = serde_json::to_value(&standing).unwrap();
    assert_eq!(json["total"]["today"]["shown"], "$18,712");
    assert_eq!(json["budgets"][1]["allowances"][0]["unit"], "dollars");
    let allowance = &json["budgets"][0]["allowances"][0];
    assert!(
        allowance.get("spent").is_none(),
        "a share was turned into dollars spent: {allowance}"
    );
    assert_eq!(json["budgets"][0]["kind"], "plan");
    assert_eq!(json["budgets"][0]["providers"][0], "claude-code");
}

// Each allowance says every fact it holds in words for the launcher's
// drawer, in its own unit, and each budget what it spent, what a
// plan's turns were worth and how its feeds read, so the shell
// formats nothing.
#[test]
fn every_fact_is_said_in_words() {
    let json = serde_json::to_value(standing_now(None)).unwrap();
    let claude = &json["budgets"][0];
    let five = &claude["allowances"][0];
    assert_eq!(five["standing"], "62% of 5 h");
    assert_eq!(five["nearest"], 0.62);
    assert_eq!(
        five["said"],
        serde_json::json!({
            "brief": "62% \u{b7} resets 16:20",
            "window": "5 hours", "warns": "at 80%", "resets": "16:20",
            "ends": "", "changed": "", "read": "Claude at 14:20", "state": ""
        })
    );
    // The week has no reading yet, so no standing, but it still says
    // its window and where it warns.
    let week = &claude["allowances"][1];
    assert!(
        week.get("standing").is_none() && week.get("nearest").is_none(),
        "a week with no reading yet has a standing: {week}"
    );
    assert_eq!(week["said"]["window"], "a week");
    assert_eq!(week["said"]["warns"], "at 80%");
    assert_eq!(
        claude["said"]["worth"],
        "$84 today \u{b7} $84 in 7 days \u{b7} $84 this month"
    );
    assert_eq!(claude["said"]["spent"], "");
    assert_eq!(claude["said"]["what"], "your own \u{b7} plan");
    // Its providers as each names itself, its id kept for the harness.
    assert_eq!(claude["said"]["providers"], "Claude Code");
    assert_eq!(claude["providers"][0], "claude-code");
    assert_eq!(claude["words"], "your own plan");
    assert_eq!(claude["said"]["reading"], "");
    // The watch counts the proxy's week itself, so nobody read it.
    let proxy = &json["budgets"][1];
    assert_eq!(proxy["words"], "the employer's money");
    assert_eq!(proxy["allowances"][0]["said"]["warns"], "at $36,000");
    assert_eq!(proxy["allowances"][0]["said"]["read"], "");
    // A window in dollars has no brief: its budget's drawer says its
    // standing, with its meter, and its reset a step in.
    assert_eq!(proxy["allowances"][0]["said"]["brief"], "");
    assert_eq!(proxy["said"]["what"], "the employer's \u{b7} money");
    assert_eq!(
        proxy["said"]["spent"],
        "$18,400 today \u{b7} $18,400 in 7 days \u{b7} $18,400 this month"
    );
    assert_eq!(proxy["said"]["worth"], "");
}

// An override's end and an allowance's last change are said where
// its feed gives them, what changed first and when after a dot; a
// change of state isn't a change of the allowance.
#[test]
fn an_allowance_says_how_it_last_changed() {
    let config = config();
    let mut picture = Picture::read(&history(), &config, NOW).unwrap();
    let tomorrow = picture.zone.day(NOW) + 1;
    let view = picture
        .caps
        .iter_mut()
        .find(|view| view.cap.name == "five-hours")
        .unwrap();
    view.cap.ends = Some(tomorrow);
    view.cap.ends_known = false;
    let event = |kind: &str, at: f64, before: f64, after: f64| crate::caps::Event {
        feed: view.cap.feed.clone(),
        budget: view.cap.budget.clone(),
        name: view.cap.name.clone(),
        at,
        kind: kind.to_owned(),
        before: Some(before),
        after: Some(after),
        spent: 0.0,
    };
    let events = vec![
        event("lowered", NOW - 7200.0, 20_000.0, 15_000.0),
        event("raised", NOW - 3600.0, 15_000.0, 20_000.0),
        event("refused", NOW - 60.0, 20_000.0, 20_000.0),
    ];
    picture.events = events;
    let said = &standing(&picture, &config, None).budgets[0].allowances[0].said;
    assert_eq!(said.ends, "Thu 15 Oct \u{b7} a guess");
    assert_eq!(said.changed, "raised $15,000 to $20,000 \u{b7} 13:30");
}

// A feed's state and a source's health read as words, saying nothing
// where all's as it should be; a dollar allowance with no amount
// warns at nothing it can say.
#[test]
fn states_read_as_words() {
    assert_eq!(stands("ok"), "");
    assert_eq!(stands("notified"), "warned");
    assert_eq!(stands("refused"), "refusing requests");
    assert_eq!(reading("fine"), "reading fine");
    assert_eq!(reading("signed-out"), "signed out");
    assert_eq!(reading("optional"), "");
    assert_eq!(warns(Unit::Dollars, None, 0.9), "");
    assert_eq!(window_length(48), "2 days");
    assert_eq!(window_length(336), "2 weeks");
}

// The 7 days are left out only where they add nothing: the month
// began within them and they say the same amount.
#[test]
fn the_week_goes_where_it_is_the_month() {
    let amounts = |week: f64| {
        [("today", 12.0), ("7 days", week), ("this month", 80.0)]
            .into_iter()
            .map(|(period, dollars)| (period.to_owned(), Amount::of(dollars, false)))
            .collect::<BTreeMap<_, _>>()
    };
    assert_eq!(
        periods(&amounts(80.0), true),
        "$12 today \u{b7} $80 this month"
    );
    assert_eq!(
        periods(&amounts(80.0), false),
        "$12 today \u{b7} $80 in 7 days \u{b7} $80 this month"
    );
    assert_eq!(
        periods(&amounts(60.0), true),
        "$12 today \u{b7} $60 in 7 days \u{b7} $80 this month"
    );
}

// A session's spend is one amount, all of it, under the budget it
// drew on last, as pi's footer and /budget say it.
#[test]
fn a_session_says_its_spend_under_its_budget() {
    let standing = standing_now(Some("team"));
    let session = standing.session.as_ref().unwrap();
    assert_eq!(session.budget.as_deref(), Some("delta"));
    assert_eq!(session.spent.shown, "$310");
    assert_eq!(standing.sessions["plan"], "claude");
    let json = serde_json::to_value(&standing).unwrap();
    assert_eq!(json["session"]["spent"]["dollars"], 310.0);
}
