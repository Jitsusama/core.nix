//! Every case in sayings.json is judged, and reached, as it says.

use pineapple::{Context, Joel, Platform, Saying, Target, judge, reach};
use serde_json::Value;

const TABLE: &str = include_str!("../sayings.json");

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn said(targets: &[Target]) -> Vec<String> {
    targets.iter().map(ToString::to_string).collect()
}

fn joel_from(value: &Value) -> Joel {
    Joel {
        slack_ids: strings(&value["slack_ids"]),
        logins: strings(&value["logins"]),
        emails: strings(&value["emails"]),
    }
}

fn context_from(value: &Value) -> Context {
    Context {
        pr: value["pr"].as_u64(),
        repo: value["repo"].as_str().map(str::to_owned),
        branch: value["branch"].as_str().map(str::to_owned),
        quest: value["quest"].as_str().map(str::to_owned),
        lanes: strings(&value["lanes"]),
    }
}

/// What a case got wrong, or nothing when it holds.
fn wrong_in(case: &Value, joel: &Joel) -> Vec<String> {
    let name = case["name"].as_str().unwrap_or("unnamed");
    let platform = match case["platform"].as_str() {
        Some("slack") => Platform::Slack,
        _ => Platform::Markdown,
    };
    let saying = Saying {
        platform,
        author: case["author"].as_str().unwrap_or(""),
        by_bot: case["bot"].as_bool().unwrap_or(false),
        text: case["text"].as_str().unwrap_or(""),
    };
    let judged = judge(&saying, joel);
    let mut wrong = Vec::new();
    let verdict = judged.verdict.to_string();
    if Some(verdict.as_str()) != case["verdict"].as_str() {
        wrong.push(format!("{name}: judged {verdict}, not {}", case["verdict"]));
    }
    if !case["targets"].is_null() && said(&judged.targets) != strings(&case["targets"]) {
        wrong.push(format!(
            "{name}: named {:?}, not {}",
            said(&judged.targets),
            case["targets"]
        ));
    }
    if !case["reach"].is_null() {
        let reached = reach(&judged.targets, &context_from(&case["context"]));
        let want = &case["reach"];
        if said(&reached.targets) != strings(&want["targets"])
            || Some(reached.fell_back) != want["fell_back"].as_bool()
        {
            wrong.push(format!(
                "{name}: reached {:?} (fell back {}), not {want}",
                said(&reached.targets),
                reached.fell_back
            ));
        }
    }
    wrong
}

#[test]
fn every_saying_is_judged_as_the_table_says() {
    let table: Value = serde_json::from_str(TABLE).expect("sayings.json is JSON");
    let joel = joel_from(&table["joel"]);
    let cases = table["sayings"]
        .as_array()
        .expect("sayings.json lists its sayings");
    assert!(cases.len() >= 40, "the table lost cases: {}", cases.len());
    let wrong: Vec<String> = cases
        .iter()
        .flat_map(|case| wrong_in(case, &joel))
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_stop_with_nothing_named_anywhere_says_it_fell_back() {
    let reached = reach(&[], &Context::default());
    assert_eq!(said(&reached.targets), ["everything"]);
    assert!(
        reached.fell_back,
        "a stop naming nothing anywhere says it fell back"
    );
}

#[test]
fn two_stops_reach_what_both_named() {
    let named = [
        Target::Pr {
            repo: None,
            number: 1,
        },
        Target::Quest("QEST-20261113-OX4WFV".to_owned()),
    ];
    let context = Context {
        repo: Some("Jitsusama/core.nix".to_owned()),
        ..Context::default()
    };
    let reached = reach(&named, &context);
    assert_eq!(
        said(&reached.targets),
        ["pr:Jitsusama/core.nix#1", "quest:QEST-20261113-OX4WFV"]
    );
    assert!(!reached.fell_back, "two named stops are no fallback");
}
