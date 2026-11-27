//! The tests of spend billed to another key: budgets sessions couldn't draw
//! on, budgets billed under what they logged, and the warehouse feeds whose
//! settling holds the day back or is wrong.

use super::*;
use crate::detect::all;
use crate::detect::tests::{NOW, config};
use crate::logs::{Cost, Event, Kind, Tokens, Turn};
use crate::time::HOUR;

/// A turn of `dollars` at `at` whose log said it drew on `budget`.
fn turn_on(history: &History, at: f64, dollars: f64, budget: Option<&str>) {
    let session = format!("s-{}", budget.unwrap_or("own"));
    turn_as(history, &session, at, dollars, budget);
}

/// A turn of `session`'s of `dollars` at `at`, its log saying `budget`.
fn turn_as(history: &History, session: &str, at: f64, dollars: f64, budget: Option<&str>) {
    history
        .add_event(&Event::Turn(Turn {
            session: session.into(),
            at,
            model: "m".into(),
            provider: "p".into(),
            kind: Kind::Agent,
            tokens: Tokens::default(),
            cost: Cost {
                total: dollars,
                ..Cost::default()
            },
            quest: None,
            budget: budget.map(str::to_owned),
            gap: None,
            rebuilt: 0.0,
            rebuilt_cost: 0.0,
            cause: None,
        }))
        .unwrap();
}

/// What the warehouse billed `budget` at `hour` for pi, the default's
/// when empty.
fn billed_to(history: &History, hour: &str, budget: &str, dollars: f64) {
    billed_for(history, hour, "pi", budget, dollars);
}

/// What the warehouse billed `budget` at `hour` for `tool`.
fn billed_for(history: &History, hour: &str, tool: &str, budget: &str, dollars: f64) {
    billed_on(history, "proxy", hour, tool, budget, dollars);
}

/// What the warehouse billed `budget` at `hour` for `tool`, by `feed`.
fn billed_on(
    history: &History,
    feed: &str,
    hour: &str,
    tool: &str,
    budget: &str,
    dollars: f64,
) {
    let line = serde_json::json!({
        "v": 1, "kind": "billed", "source": "warehouse", "hour": hour, "tool": tool,
        "model": "m", "vendor": "v", "dollars": dollars, "budget": budget,
        "tokens": {"input": 0, "cached": 0, "cacheWrite": 0, "output": 0},
    });
    let record = crate::feed::parse(&line.to_string()).unwrap();
    crate::feed::store(history, feed, &record, NOW).unwrap();
}

/// The warehouse's last settled hour in the tests, 12:00 on 14 October.
const SETTLED: &str = "2026-10-14T12:00:00Z";

/// The hour after it, which the warehouse may still be billing.
const UNSETTLED: &str = "2026-10-14T13:00:00Z";

/// A moment inside the settled hour.
const IN_SETTLED: f64 = NOW - 9000.0 + 60.0;

/// Each budget the history's warehouse billed once, a week before,
/// and the warehouse settled through `SETTLED`.
fn billed_before(history: &History) {
    for budget in ["", "teal", "ochre", "slate"] {
        billed_to(history, "2026-10-07T12:00:00Z", budget, 10.0);
    }
    settled_to(history, "proxy", UNSETTLED);
}

/// The warehouse `feed` settled up to `through`, exclusive.
fn settled_to(history: &History, feed: &str, through: &str) {
    let settled = serde_json::json!({
        "v": 1, "kind": "settled", "source": "warehouse", "through": through,
    });
    let record = crate::feed::parse(&settled.to_string()).unwrap();
    crate::feed::store(history, feed, &record, NOW).unwrap();
}

/// The config with the warehouse's one feed, as the machine runs it.
fn fed_config() -> Config {
    config_running(&["proxy"])
}

/// The config running each of `feeds`.
fn config_running(feeds: &[&str]) -> Config {
    let feeds: Vec<serde_json::Value> = feeds
        .iter()
        .map(|name| serde_json::json!({"name": name, "label": name, "command": ["true"]}))
        .collect();
    serde_json::from_value(serde_json::json!({
        "sessions": "/none", "runs": "/none", "registry": "/none", "quests": "/none",
        "state": "/none", "zone": "UTC", "feeds": feeds,
    }))
    .unwrap()
}

/// A council started from `parent` at `at`, its one agent `agent`, whose
/// cost its log covers.
fn council(history: &History, parent: &str, at: f64, agent: &str, cost: f64) {
    history
        .db
        .execute(
            "INSERT INTO runs VALUES ('council-1', 'a1', 'council', 'm', 'critic', ?1, ?2,
                 NULL, ?3, 0, 0, 0, 0, ?4)",
            rusqlite::params![parent, at, cost, serde_json::to_string(&[agent]).unwrap()],
        )
        .unwrap();
}

fn missed(history: &History, session: &str, ago: f64, budget: &str, missing: &str) {
    history
        .add_event(&crate::logs::Event::Unresolved {
            session: session.into(),
            at: NOW - ago,
            budget: budget.into(),
            missing: missing.into(),
        })
        .unwrap();
}

// A team budget a session couldn't draw on is told the day it starts,
// once per budget however many of its sessions missed it, with the
// last reason the harness gave; a miss over a day old isn't.
#[test]
fn a_budget_sessions_could_not_draw_on_is_told() {
    let history = History::memory();
    // Written out of time order, so the last is told by its time.
    missed(&history, "s2", HOUR, "teal", "its key was refused");
    missed(&history, "s1", 3.0 * HOUR, "teal", "no key for it");
    missed(&history, "s1", 2.0 * HOUR, "teal", "no key for it");
    missed(&history, "s3", DAY + HOUR, "ochre", "no key for it");
    let config = config();
    let picture = Picture::read(&history, &config, NOW).unwrap();
    let found = unresolved(&picture, &history).unwrap();
    let told: Vec<(&str, &str, &str)> = found
        .iter()
        .map(|f| (f.key.as_str(), f.title.as_str(), f.note.as_str()))
        .collect();
    assert_eq!(
        told,
        [(
            "unresolved:teal",
            "2 sessions couldn't draw on teal",
            "drew on another key"
        )]
    );
    let last = picture.zone.clock(NOW - HOUR, picture.now);
    assert_eq!(found[0].badge, format!("last {last}"));
    assert_eq!(
        found[0].severity,
        Severity::Note,
        "a note, so it never takes a notice"
    );
    assert!(
        found[0].detail.contains(&format!(
            "the last at {last}; the harness said: its key was refused."
        )),
        "{}",
        found[0].detail
    );
    assert!(
        all(&picture, &history, &config)
            .unwrap()
            .iter()
            .any(|f| f.key == "unresolved:teal"),
        "every pass looks for it"
    );
}

// A budget the config declares is told by its label, as Joel knows it,
// and keyed by its name.
#[test]
fn a_declared_budget_is_told_by_its_label() {
    let history = History::memory();
    missed(&history, "s1", HOUR, "teal", "no key for it");
    let config: Config = serde_json::from_value(serde_json::json!({
        "sessions": "/none", "runs": "/none", "registry": "/none", "quests": "/none",
        "state": "/none", "zone": "UTC",
        "budgets": [
            {"name": "teal", "label": "Teal team token", "kind": "team", "whose": "team"},
        ],
    }))
    .unwrap();
    let picture = Picture::read(&history, &config, NOW).unwrap();
    let found = unresolved(&picture, &history).unwrap();
    assert_eq!(
        (found[0].key.as_str(), found[0].title.as_str()),
        (
            "unresolved:teal",
            "1 session couldn't draw on Teal team token"
        )
    );
}

fn unbilled_keys(history: &History) -> Vec<String> {
    unbilled_keys_running(history, &fed_config())
}

fn unbilled_keys_running(history: &History, config: &Config) -> Vec<String> {
    let picture = Picture::read(history, config, NOW).unwrap();
    unbilled(&picture, history, config)
        .unwrap()
        .into_iter()
        .map(|f| f.key)
        .collect()
}

// From 23:00 on 17 November to 19:00 on the 18th, the pi host sent its
// own key for every conversation, so teams' work was billed to Joel's;
// another tool on a team's key doesn't hide it.
#[test]
fn a_budget_billed_under_half_what_it_logged_is_told() {
    let history = History::memory();
    billed_before(&history);
    turn_on(&history, IN_SETTLED, 170.55, Some("teal"));
    turn_on(&history, IN_SETTLED, 375.60, Some("ochre"));
    turn_on(&history, IN_SETTLED, 5558.23, None);
    billed_to(&history, SETTLED, "ochre", 2.67);
    billed_for(&history, SETTLED, "river", "teal", 500.0);
    billed_to(&history, SETTLED, "", 6619.50);
    let config = fed_config();
    let picture = Picture::read(&history, &config, NOW).unwrap();
    let found = unbilled(&picture, &history, &config).unwrap();
    let said: Vec<(&str, Severity, &str, &str)> = found
        .iter()
        .map(|f| {
            (
                f.key.as_str(),
                f.severity,
                f.title.as_str(),
                f.badge.as_str(),
            )
        })
        .collect();
    assert_eq!(
        said,
        [
            (
                "unbilled:ochre",
                Severity::Warning,
                "ochre billed $3 of $376 logged",
                "1% billed"
            ),
            (
                "unbilled:teal",
                Severity::Warning,
                "teal billed $0 of $171 logged",
                "0% billed"
            ),
        ]
    );
    assert!(
        found[1]
            .detail
            .starts_with("In the 24 hours to 13:00, pi's logs priced teal's sessions at $171"),
        "the detail says the logged dollars: {}",
        found[1].detail
    );
}

// A team billed near what it logged is as it should be; so is one
// that logged less than the floor, or one no warehouse ever billed,
// whose figures are the logs' own.
#[test]
fn a_budget_billed_as_it_logged_is_quiet() {
    let history = History::memory();
    billed_before(&history);
    turn_on(&history, IN_SETTLED, 100.0, Some("teal"));
    billed_to(&history, SETTLED, "teal", 60.0);
    turn_on(&history, IN_SETTLED, 20.0, Some("slate"));
    turn_on(&history, IN_SETTLED, 300.0, Some("unfed"));
    turn_on(&history, IN_SETTLED, 50.0, None);
    billed_to(&history, SETTLED, "", 400.0);
    assert_eq!(unbilled_keys(&history), [] as [String; 0]);
}

// Only the hours the warehouse has settled are held against it: a
// team's turns since are billed later, and those a day before its
// last settled hour are past the window.
#[test]
fn only_the_warehouses_settled_day_is_held() {
    let history = History::memory();
    billed_before(&history);
    billed_to(&history, SETTLED, "", 400.0);
    turn_on(&history, NOW, 500.0, Some("teal"));
    turn_on(&history, IN_SETTLED - DAY, 500.0, Some("ochre"));
    assert_eq!(unbilled_keys(&history), [] as [String; 0]);
    turn_on(&history, IN_SETTLED - DAY + HOUR, 500.0, Some("ochre"));
    assert_eq!(unbilled_keys(&history), ["unbilled:ochre"]);
}

// An hour the warehouse hasn't settled may still grow, so what it
// bills a team there doesn't cover the team's settled hours.
#[test]
fn an_unsettled_hours_bill_is_left_out() {
    let history = History::memory();
    billed_before(&history);
    turn_on(&history, IN_SETTLED, 100.0, Some("teal"));
    billed_to(&history, UNSETTLED, "teal", 200.0);
    assert_eq!(unbilled_keys(&history), ["unbilled:teal"]);
}

// A council's agent whose log names no budget draws on the budget its
// session drew on, as everywhere else in spend-watch.
#[test]
fn a_runs_agent_counts_on_its_sessions_budget() {
    let history = History::memory();
    billed_before(&history);
    turn_as(&history, "s-teal", IN_SETTLED, 10.0, Some("teal"));
    turn_as(&history, "r1", IN_SETTLED + 60.0, 200.0, None);
    council(&history, "s-teal", IN_SETTLED, "r1", 200.0);
    billed_to(&history, SETTLED, "teal", 5.0);
    billed_to(&history, SETTLED, "", 300.0);
    assert_eq!(unbilled_keys(&history), ["unbilled:teal"]);
}

// A session that last spoke before the day still lends its budget to
// the agents of a run it starts within it, as the picture's pieces do.
#[test]
fn a_runs_agent_keeps_its_sessions_budget_from_before_the_day() {
    let history = History::memory();
    billed_before(&history);
    turn_as(
        &history,
        "s-teal",
        IN_SETTLED - 30.0 * HOUR,
        1.0,
        Some("teal"),
    );
    turn_as(&history, "r1", IN_SETTLED + 60.0, 200.0, None);
    council(&history, "s-teal", IN_SETTLED, "r1", 200.0);
    billed_to(&history, SETTLED, "teal", 5.0);
    billed_to(&history, SETTLED, "", 300.0);
    assert_eq!(unbilled_keys(&history), ["unbilled:teal"]);
}

// A warehouse that stopped settling two days ago is told, in place of
// any budget, rather than holding a day long past against its logs.
#[test]
fn a_warehouse_that_stopped_settling_is_told() {
    let history = History::memory();
    // A feed's settled hour only moves on, so this one is all it said.
    billed_to(&history, "2026-10-07T12:00:00Z", "teal", 10.0);
    settled_to(&history, "proxy", "2026-10-12T12:00:00Z");
    // 11:30 on 12 October, in the last hour it settled.
    turn_on(&history, NOW - 51.0 * HOUR, 100.0, Some("teal"));
    assert_eq!(unbilled_keys(&history), ["unbilled-unsettled:proxy"]);
}

// A feed the config no longer runs keeps its settled row, and doesn't
// hold the day back.
#[test]
fn a_feed_no_longer_run_holds_nothing_back() {
    let history = History::memory();
    billed_before(&history);
    settled_to(&history, "gone", "2026-10-01T00:00:00Z");
    turn_on(&history, IN_SETTLED, 100.0, Some("teal"));
    assert_eq!(unbilled_keys(&history), ["unbilled:teal"]);
}

// A budget a feed first billed within the day hasn't been billed all
// of it, so it isn't judged until a whole day has.
#[test]
fn a_budget_first_billed_within_the_day_is_left_out() {
    let history = History::memory();
    billed_before(&history);
    turn_on(&history, IN_SETTLED, 100.0, Some("umber"));
    billed_to(&history, SETTLED, "umber", 1.0);
    assert_eq!(unbilled_keys(&history), [] as [String; 0]);
}

// A budget only a feed the config no longer runs ever billed has no
// bill to disagree with, so it isn't told billed nothing every day.
#[test]
fn a_budget_only_a_removed_feed_billed_is_left_out() {
    let history = History::memory();
    billed_before(&history);
    billed_on(
        &history,
        "gone",
        "2026-10-07T12:00:00Z",
        "pi",
        "umber",
        10.0,
    );
    turn_on(&history, IN_SETTLED, 100.0, Some("umber"));
    assert_eq!(unbilled_keys(&history), [] as [String; 0]);
}

// The day is the one every feed the config runs has settled, so the
// feed furthest behind holds it back.
#[test]
fn the_feed_furthest_behind_holds_the_day() {
    let history = History::memory();
    billed_before(&history);
    settled_to(&history, "second", "2026-10-14T06:00:00Z");
    let both = config_running(&["proxy", "second"]);
    turn_on(&history, IN_SETTLED, 100.0, Some("teal"));
    assert_eq!(unbilled_keys_running(&history, &both), [] as [String; 0]);
    // 05:30, within the day the second feed has settled.
    turn_on(&history, NOW - 9.0 * HOUR, 100.0, Some("ochre"));
    assert_eq!(unbilled_keys_running(&history, &both), ["unbilled:ochre"]);
}

// The warehouse may have settled every hour before the one under way.
#[test]
fn the_hour_under_way_may_be_settled_to() {
    let history = History::memory();
    billed_before(&history);
    settled_to(&history, "proxy", "2026-10-14T14:00:00Z");
    turn_on(&history, NOW - 2.0 * HOUR, 100.0, Some("teal"));
    assert_eq!(unbilled_keys(&history), ["unbilled:teal"]);
}

// The hour the warehouse has settled to is still being billed, so a
// turn in it isn't held against the bill.
#[test]
fn a_turn_in_the_unsettled_hour_is_not_judged() {
    let history = History::memory();
    billed_before(&history);
    turn_on(&history, NOW - HOUR, 100.0, Some("teal"));
    assert_eq!(unbilled_keys(&history), [] as [String; 0]);
}

// A feed the config runs that bills pi but hasn't said how far it has
// settled may not have billed a day in full, so none is judged, and
// that's told, as a note, so cadence reads the day as unjudged rather
// than as no budget unbilled.
#[test]
fn a_running_feed_never_settled_holds_the_day() {
    let history = History::memory();
    billed_before(&history);
    billed_on(
        &history,
        "second",
        "2026-10-13T12:00:00Z",
        "pi",
        "teal",
        1.0,
    );
    turn_on(&history, IN_SETTLED, 100.0, Some("teal"));
    let both = config_running(&["proxy", "second"]);
    assert_eq!(
        unbilled_keys_running(&history, &both),
        ["unbilled-held:second"]
    );
    let picture = Picture::read(&history, &both, NOW).unwrap();
    let found = unbilled(&picture, &history, &both).unwrap();
    assert_eq!(
        (found[0].severity, found[0].title.as_str()),
        (
            Severity::Note,
            "the second feed hasn't said how far pi's bill is settled"
        )
    );
    assert_eq!(unbilled_keys(&history), ["unbilled:teal"]);
}

// A feed that holds back hides nothing, so a wrong row beside it is
// still told, as a warning: past now, or stopped settling. Nor does
// the row hide the held feed, whose note would clear and count again.
#[test]
fn a_held_feed_hides_no_wrong_row() {
    let wrong = |through: &str| {
        let history = History::memory();
        billed_before(&history);
        billed_on(
            &history,
            "second",
            "2026-10-07T12:00:00Z",
            "pi",
            "teal",
            1.0,
        );
        settled_to(&history, "third", through);
        let all = config_running(&["proxy", "second", "third"]);
        let picture = Picture::read(&history, &all, NOW).unwrap();
        let found = unbilled(&picture, &history, &all).unwrap();
        found
            .into_iter()
            .map(|f| (f.key, f.severity, f.title))
            .collect::<Vec<_>>()
    };
    let told = |title: &str| {
        vec![
            (
                "unbilled-unsettled:third".to_owned(),
                Severity::Warning,
                title.to_owned(),
            ),
            (
                "unbilled-held:second".to_owned(),
                Severity::Note,
                "the second feed hasn't said how far pi's bill is settled".to_owned(),
            ),
        ]
    };
    assert_eq!(
        wrong("2026-10-15T00:00:00Z"),
        told("the third feed's bill is settled past now")
    );
    assert_eq!(
        wrong("2026-10-12T12:00:00Z"),
        told("the third feed's bill stopped settling")
    );
}

// The one feed the config runs, billing pi with no settled hour at
// all, holds the day back as a note rather than saying nothing.
#[test]
fn a_lone_feed_never_settled_is_told() {
    let history = History::memory();
    billed_on(
        &history,
        "proxy",
        "2026-10-13T12:00:00Z",
        "pi",
        "teal",
        10.0,
    );
    turn_on(&history, IN_SETTLED, 100.0, Some("teal"));
    assert_eq!(unbilled_keys(&history), ["unbilled-held:proxy"]);
}

// A held note that has stood for more than a settled hour may lag gains
// a warning beside it, under a key of its own, so marking the note
// expected leaves the warning to be told. It's how long the note has
// stood that counts, not how old the bills are: these are a week old.
#[test]
fn a_feed_held_for_days_is_warned() {
    let told = |stood: f64| {
        let history = History::memory();
        billed_on(
            &history,
            "proxy",
            "2026-10-07T12:00:00Z",
            "pi",
            "teal",
            10.0,
        );
        turn_on(&history, IN_SETTLED, 100.0, Some("teal"));
        let picture = Picture::read(&history, &fed_config(), NOW).unwrap();
        let first = unbilled(&picture, &history, &fed_config()).unwrap();
        crate::findings::settle(&history, &first, NOW - stood).unwrap();
        unbilled(&picture, &history, &fed_config())
            .unwrap()
            .into_iter()
            .map(|f| (f.key, f.severity, f.title))
            .collect::<Vec<_>>()
    };
    let note = (
        "unbilled-held:proxy".to_owned(),
        Severity::Note,
        "the proxy feed hasn't said how far pi's bill is settled".to_owned(),
    );
    let warning = (
        "unbilled-not-settling:proxy".to_owned(),
        Severity::Warning,
        "the proxy feed hasn't said how far pi's bill is settled in 48 hours".to_owned(),
    );
    assert_eq!(told(0.0), std::slice::from_ref(&note), "just told");
    assert_eq!(
        told(48.0 * HOUR + 29.0 * 60.0),
        std::slice::from_ref(&note),
        "told in the hour two days back"
    );
    assert_eq!(
        told(48.5 * HOUR + 60.0),
        [note, warning],
        "told in the hour before that"
    );
}

// Once the feed says how far it has settled, neither the note nor the
// warning is raised, so both clear as any finding no pass raises does.
#[test]
fn a_held_feed_that_settles_is_told_no_more() {
    let history = History::memory();
    billed_on(
        &history,
        "proxy",
        "2026-10-07T12:00:00Z",
        "pi",
        "teal",
        10.0,
    );
    turn_on(&history, IN_SETTLED, 100.0, Some("teal"));
    let picture = Picture::read(&history, &fed_config(), NOW).unwrap();
    let first = unbilled(&picture, &history, &fed_config()).unwrap();
    crate::findings::settle(&history, &first, NOW - 49.0 * HOUR).unwrap();
    settled_to(&history, "proxy", SETTLED);
    let picture = Picture::read(&history, &fed_config(), NOW).unwrap();
    let keys: Vec<String> = unbilled(&picture, &history, &fed_config())
        .unwrap()
        .into_iter()
        .map(|f| f.key)
        .collect();
    assert!(
        keys.iter()
            .all(|key| !key.starts_with("unbilled-held") && !key.starts_with("unbilled-not")),
        "{keys:?}"
    );
}

// Every feed that holds back is told, by name, whichever billed first,
// so a fresh feed's note can't hide one that has stopped settling:
// second's note is fresh and third's has stood two days.
#[test]
fn every_held_feed_is_told_by_name() {
    let history = History::memory();
    billed_before(&history);
    billed_on(&history, "third", "2026-10-07T12:00:00Z", "pi", "teal", 1.0);
    billed_on(
        &history,
        "second",
        "2026-10-07T12:00:00Z",
        "pi",
        "teal",
        1.0,
    );
    let all = config_running(&["proxy", "second", "third"]);
    let picture = Picture::read(&history, &all, NOW).unwrap();
    let first = unbilled(&picture, &history, &all).unwrap();
    let third: Vec<_> = first
        .into_iter()
        .filter(|f| f.key == "unbilled-held:third")
        .collect();
    crate::findings::settle(&history, &third, NOW - 49.0 * HOUR).unwrap();
    let found = unbilled(&picture, &history, &all).unwrap();
    let keys: Vec<_> = found.iter().map(|f| f.key.as_str()).collect();
    assert_eq!(
        keys,
        [
            "unbilled-held:second",
            "unbilled-held:third",
            "unbilled-not-settling:third"
        ]
    );
}

// One feed that stopped settling is told however fresh another is, so
// a stale feed can't hide behind one that settles on time.
#[test]
fn a_stale_feed_beside_a_fresh_one_is_told() {
    let history = History::memory();
    billed_before(&history);
    settled_to(&history, "second", "2026-10-12T12:00:00Z");
    turn_on(&history, IN_SETTLED, 100.0, Some("teal"));
    let both = config_running(&["proxy", "second"]);
    let picture = Picture::read(&history, &both, NOW).unwrap();
    let found = unbilled(&picture, &history, &both).unwrap();
    assert_eq!(
        (found[0].key.as_str(), found[0].title.as_str()),
        (
            "unbilled-unsettled:second",
            "the second feed's bill stopped settling"
        )
    );
}

// A feed whose bill is all the meter's never settles, so it holds no
// day back.
#[test]
fn a_metered_feed_holds_nothing_back() {
    let history = History::memory();
    billed_before(&history);
    let line = serde_json::json!({
        "v": 1, "kind": "billed", "source": "meter", "hour": "2026-10-07T12:00:00Z",
        "tool": "pi", "model": "m", "dollars": 1.0, "budget": "teal",
    });
    let record = crate::feed::parse(&line.to_string()).unwrap();
    crate::feed::store(&history, "second", &record, NOW).unwrap();
    turn_on(&history, IN_SETTLED, 100.0, Some("teal"));
    let both = config_running(&["proxy", "second"]);
    assert_eq!(unbilled_keys_running(&history, &both), ["unbilled:teal"]);
}

// One feed settled past now is wrong however far behind another is, so
// it's told in place of any budget.
#[test]
fn a_second_feed_settled_past_now_is_told() {
    let history = History::memory();
    billed_before(&history);
    settled_to(&history, "second", "2026-10-15T00:00:00Z");
    turn_on(&history, IN_SETTLED, 100.0, Some("teal"));
    let both = config_running(&["proxy", "second"]);
    assert_eq!(
        unbilled_keys_running(&history, &both),
        ["unbilled-unsettled:second"]
    );
    let picture = Picture::read(&history, &both, NOW).unwrap();
    let found = unbilled(&picture, &history, &both).unwrap();
    assert_eq!(found[0].title, "the second feed's bill is settled past now");
}

// The settled hour may be the one under way, and as far back as the
// picture keeps a day before it; an hour either side of those is told.
#[test]
fn the_settled_hour_is_judged_to_its_edges() {
    let now = crate::time::hour_of(NOW);
    let told = |through: i64| {
        let history = History::memory();
        billed_before(&history);
        history
            .db
            .execute(
                "UPDATE settled SET through = ?1 WHERE feed = 'proxy'",
                [through],
            )
            .unwrap();
        unbilled_keys(&history) == ["unbilled-unsettled:proxy"]
    };
    assert!(told(now + 1), "an hour past now");
    assert!(!told(now), "the hour under way");
    assert!(!told(now - UNBILLED_STALE_HOURS), "the oldest it judges");
    assert!(told(now - UNBILLED_STALE_HOURS - 1), "an hour older");
}

// A settled hour a feed gave past now can't be true, so no budget is
// judged under it, and that is told instead.
#[test]
fn a_settled_hour_past_now_is_told_in_place_of_any_budget() {
    let history = History::memory();
    billed_before(&history);
    let future = crate::time::hour_of(NOW) + 34;
    history
        .db
        .execute(
            "UPDATE settled SET through = ?1 WHERE feed = 'proxy'",
            [future],
        )
        .unwrap();
    turn_on(&history, NOW - 2.0 * HOUR, 100.0, Some("teal"));
    turn_on(&history, IN_SETTLED, 100.0, Some("teal"));
    assert_eq!(unbilled_keys(&history), ["unbilled-unsettled:proxy"]);
    let config = fed_config();
    let picture = Picture::read(&history, &config, NOW).unwrap();
    let findings = unbilled(&picture, &history, &config).unwrap();
    assert_eq!(
        findings[0].title,
        "the proxy feed's bill is settled past now"
    );
}

// The floor is on what went unbilled, not on what was logged: $30
// logged and $10 billed is under half, but only $20 short.
#[test]
fn the_floor_is_on_what_went_unbilled() {
    let history = History::memory();
    billed_before(&history);
    turn_on(&history, IN_SETTLED, 30.0, Some("teal"));
    billed_to(&history, SETTLED, "teal", 10.0);
    assert_eq!(unbilled_keys(&history), [] as [String; 0]);
}
