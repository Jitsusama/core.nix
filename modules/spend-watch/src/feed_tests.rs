use super::*;

const FIXTURES: [(&str, &str); 3] = [
    ("everything", include_str!("../contract/everything.jsonl")),
    ("meter-only", include_str!("../contract/meter-only.jsonl")),
    ("signed-out", include_str!("../contract/signed-out.jsonl")),
];

#[test]
fn every_fixture_keeps_the_contract() {
    for (name, text) in FIXTURES {
        for (number, line) in text
            .lines()
            .enumerate()
            .filter(|(_, line)| !line.is_empty())
        {
            parse(line).unwrap_or_else(|error| panic!("{name}:{}: {error}", number + 1));
        }
    }
}

// A window is named for its length in words, in Title Case, with what
// it's for in front, and in the glossary's words; a feed that names
// one otherwise has its cap refused, so every feed's names agree.
#[test]
fn a_window_is_named_as_the_contract_says() {
    let named = |label: &str| {
        parse(
            &serde_json::json!({
                "v": 1, "kind": "cap", "name": "w", "label": label,
                "allowance": 300, "windowHours": 24, "spent": 0, "warn": 0.8,
                "state": "ok", "at": "2026-10-14T05:00:00Z",
            })
            .to_string(),
        )
    };
    for label in [
        "Week",
        "Five Hours",
        "Premium Day",
        "Forty-Five Hours",
        "Delta's Week",
    ] {
        assert!(named(label).is_ok(), "{label}: {:?}", named(label));
    }
    for (label, why) in [
        ("", "is named"),
        ("5 h", "Title Case"),
        ("the week", "Title Case"),
        ("Premium day", "Title Case"),
        ("Forty-five Hours", "Title Case"),
        ("Weekly Cap", "says \"Cap\""),
        ("Daily Limits", "says \"Limits\""),
        ("Week Allowance", "says \"Allowance\""),
        ("Quota", "says \"Quota\""),
    ] {
        let refused = named(label).unwrap_err();
        assert!(refused.contains(why), "{label}: {refused}");
    }
}

#[test]
fn a_line_without_a_version_is_refused() {
    assert!(parse(r#"{"kind":"said","severity":"info","text":"x"}"#).is_err());
    assert!(parse(r#"{"v":2,"kind":"said","severity":"info","text":"x"}"#).is_err());
}

#[test]
fn an_hour_must_start_an_hour() {
    let line = concat!(
        r#"{"v":1,"kind":"billed","source":"meter","hour":"2026-10-14T05:30:00Z","#,
        r#""tool":"all","model":"m","dollars":1}"#
    );
    assert!(parse(line).unwrap_err().contains("doesn't start an hour"));
}

#[test]
fn a_cap_of_nothing_is_refused() {
    let line = serde_json::json!({
        "v": 1, "kind": "cap", "name": "weekly", "label": "Week",
        "allowance": 0, "windowHours": 168, "spent": 1, "warn": 0.9,
        "state": "refused", "at": "2026-10-14T05:00:00Z",
    });
    assert!(
        parse(&line.to_string())
            .unwrap_err()
            .contains("more than nothing")
    );
}

// A day's cap that resets at the next midnight is kept with its reset;
// one claiming to reset before it was read can't be judged.
#[test]
fn a_period_s_reset_is_kept_after_it_was_read() {
    let day = |resets: &str| {
        serde_json::json!({
            "v": 1, "kind": "cap", "name": "premium", "label": "Premium Day",
            "allowance": 1000, "windowHours": 24, "spent": 421.43, "warn": 0.5,
            "state": "ok", "resets": resets, "at": "2026-10-14T05:40:00Z",
        })
        .to_string()
    };
    assert!(
        parse(&day("2026-10-14T05:00:00Z"))
            .unwrap_err()
            .contains("isn't after")
    );
    let history = History::memory();
    store(
        &history,
        "proxy",
        &parse(&day("2026-10-15T04:00:00Z")).unwrap(),
        0.0,
    )
    .unwrap();
    let kept = crate::caps::latest(&history).unwrap();
    assert_eq!(kept[0].resets, crate::time::parse("2026-10-15T04:00:00Z"));
}

// A share of a plan's window says what's used and when it
// resets, and never an amount; a cap in dollars says what's spent.
#[test]
fn a_share_says_what_is_used_and_when_it_resets() {
    let share = |extra: serde_json::Value| {
        let mut line = serde_json::json!({
            "v": 1, "kind": "cap", "name": "five-hours", "label": "Five Hours",
            "budget": "claude", "unit": "share", "windowHours": 5, "warn": 0.8,
            "state": "ok", "at": "2026-11-07T10:00:00Z",
        });
        for (key, value) in extra.as_object().unwrap() {
            line[key] = value.clone();
        }
        parse(&line.to_string())
    };
    let resets = serde_json::json!("2026-11-07T14:20:00Z");
    let read = share(serde_json::json!({"used": 0.62, "resets": resets})).unwrap();
    assert!(matches!(
        read,
        Record::Cap(Cap {
            used: Some(0.62),
            ..
        })
    ));
    assert!(share(serde_json::json!({"used": 1.2, "resets": resets})).is_err());
    assert!(share(serde_json::json!({"used": 0.62})).is_err());
    let priced = serde_json::json!({"used": 0.6, "resets": resets, "allowance": 20});
    assert!(share(priced).is_err());
    let unspent = serde_json::json!({
        "v": 1, "kind": "cap", "name": "weekly", "label": "Week",
        "allowance": 15000, "windowHours": 168, "warn": 0.9, "state": "ok",
        "at": "2026-11-07T10:00:00Z",
    });
    assert!(parse(&unspent.to_string()).is_err());
}

// Two budgets' bills alike from one feed are two figures, not one
// replacing the other.
#[test]
fn a_bill_for_another_budget_is_its_own() {
    let history = History::memory();
    for budget in [None, Some("delta")] {
        let line = serde_json::json!({
            "v": 1, "kind": "billed", "source": "meter", "hour": "2026-10-14T05:00:00Z",
            "tool": "all", "model": "m", "budget": budget, "dollars": 10.0,
        });
        store(&history, "proxy", &parse(&line.to_string()).unwrap(), 1.0).unwrap();
    }
    let kept: Vec<String> = history
        .db
        .prepare("SELECT budget FROM billed ORDER BY budget")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(kept, ["", "delta"]);
}

#[test]
fn an_unknown_field_is_refused_rather_than_ignored() {
    let line = r#"{"v":1,"kind":"said","severity":"info","text":"x","colour":"red"}"#;
    assert!(parse(line).is_err());
}

fn cap(allowance: Option<f64>, at: &str, state: &str) -> Record {
    let line = serde_json::json!({
        "v": 1, "kind": "cap", "name": "weekly", "label": "Week",
        "allowance": allowance, "windowHours": 168, "spent": 4995, "warn": 0.9,
        "fallback": 4000, "state": state, "at": at,
    });
    parse(&line.to_string()).unwrap()
}

fn events(history: &History) -> Vec<String> {
    let mut statement = history
        .db
        .prepare("SELECT kind FROM cap_events ORDER BY at, rowid")
        .unwrap();
    statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

#[test]
fn a_lapse_a_refusal_and_a_raise_are_events() {
    let history = History::memory();
    store(
        &history,
        "proxy",
        &cap(Some(8000.0), "2026-09-08T23:04:00Z", "ok"),
        0.0,
    )
    .unwrap();
    store(
        &history,
        "proxy",
        &cap(Some(4000.0), "2026-09-09T00:04:00Z", "refused"),
        0.0,
    )
    .unwrap();
    store(
        &history,
        "proxy",
        &cap(Some(8000.0), "2026-09-09T00:28:00Z", "ok"),
        0.0,
    )
    .unwrap();
    store(
        &history,
        "proxy",
        &cap(None, "2026-09-10T00:28:00Z", "ok"),
        0.0,
    )
    .unwrap();
    store(
        &history,
        "proxy",
        &cap(Some(10000.0), "2026-09-11T00:28:00Z", "ok"),
        0.0,
    )
    .unwrap();
    assert_eq!(
        events(&history),
        ["lapsed", "refused", "raised", "raised", "lowered"]
    );
}

#[test]
fn a_provisional_hour_is_replaced_as_it_grows() {
    let history = History::memory();
    let billed = |dollars: f64| {
        let line = serde_json::json!({
            "v": 1, "kind": "billed", "source": "meter", "hour": "2026-10-14T05:00:00Z",
            "tool": "all", "model": "m", "dollars": dollars, "provisional": true,
        });
        parse(&line.to_string()).unwrap()
    };
    store(&history, "proxy", &billed(29.19), 1.0).unwrap();
    store(&history, "proxy", &billed(29.83), 2.0).unwrap();
    let total: f64 = history
        .db
        .query_row("SELECT SUM(dollars) FROM billed", [], |row| row.get(0))
        .unwrap();
    assert_eq!(total, 29.83);
}

// A history built afresh keeps nothing of a feed's, and says so, so
// the feed tells its bill again from the start; one that keeps some
// says how far, source by source.
#[test]
fn a_feed_is_told_what_the_watch_keeps_of_its_own() {
    let history = History::memory();
    let said = |feed: &str| -> Value {
        serde_json::from_str(&kept_line(&history, feed).unwrap()).unwrap()
    };
    assert_eq!(
        said("proxy"),
        serde_json::json!({ "v": 1, "kept": { "billed": {}, "caps": {} } })
    );
    for (source, hour) in [
        ("warehouse", "2026-10-14T12:00:00Z"),
        ("warehouse", "2026-10-14T13:00:00Z"),
        ("meter", "2026-10-14T16:00:00Z"),
    ] {
        let line = serde_json::json!({
            "v": 1, "kind": "billed", "source": source, "hour": hour,
            "tool": "all", "model": "m", "dollars": 1.0,
        });
        store(&history, "proxy", &parse(&line.to_string()).unwrap(), 1.0).unwrap();
    }
    // Each cap by its own newest, since one a feed reads more often
    // mustn't hide a gap in another's.
    for (name, label, at) in [
        ("weekly", "Week", "2026-10-14T15:04:12Z"),
        ("weekly", "Week", "2026-10-14T16:04:29Z"),
        ("premium", "Premium Day", "2026-10-14T16:50:00Z"),
    ] {
        let cap = serde_json::json!({
            "v": 1, "kind": "cap", "name": name, "label": label,
            "allowance": 15000, "windowHours": 168, "spent": 4262, "warn": 0.9,
            "state": "ok", "at": at,
        });
        store(&history, "proxy", &parse(&cap.to_string()).unwrap(), 1.0).unwrap();
    }
    assert_eq!(
        said("proxy"),
        serde_json::json!({ "v": 1, "kept": {
            "billed": {
                "meter": "2026-10-14T16:00:00Z",
                "warehouse": "2026-10-14T13:00:00Z",
            },
            "caps": {
                "premium": "2026-10-14T16:50:00Z",
                "weekly": "2026-10-14T16:04:29Z",
            },
        } })
    );
    assert_eq!(said("other")["kept"]["billed"], serde_json::json!({}));
}
