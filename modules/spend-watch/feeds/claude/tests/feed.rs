//! What the feed says of a Claude plan after one read: its sign-in read from
//! a recorded `.credentials.json`, and the usage endpoint's answer from a
//! recorded response, so no test reaches the network.

use std::time::Duration;

use claude_plan_feed::{Answer, Endpoint, Feed, Outcome, Unreached};
use jiff::Timestamp;
use serde_json::{Value, json};

/// When every read here happens, unless a test says otherwise: an hour
/// after the recorded sign-ins expire and five before the others do.
const NOW: &str = "2026-11-06T15:00:00Z";

#[test]
fn a_plan_s_two_windows_are_its_caps_as_the_contract_example_says() {
    let mut feed = Feed::new(login("pro"));
    let mut endpoint = Recorded::answering(200, &answer("pro"));

    let said: Vec<String> = feed
        .introduction()
        .into_iter()
        .chain(feed.read(&mut endpoint, at(NOW)).lines)
        .collect();

    let example = include_str!("claude-plan.jsonl");
    assert_eq!(records(&said), records_of(example));
}

#[test]
fn the_endpoint_is_asked_with_the_sign_in_s_access_token() {
    let mut feed = Feed::new(login("pro"));
    let mut endpoint = Recorded::answering(200, &answer("pro"));

    feed.read(&mut endpoint, at(NOW));

    assert_eq!(endpoint.tokens, ["sk-ant-oat01-recorded-for-tests"]);
}

#[test]
fn a_plan_without_five_hours_says_only_its_week() {
    let mut feed = Feed::new(login("ultra"));
    let mut endpoint = Recorded::answering(200, &answer("ultra"));

    let read = feed.read(&mut endpoint, at(NOW));

    let caps = kinds(&read.lines, "cap");
    assert_eq!(caps.len(), 1, "only the week is said: {caps:?}");
    assert_eq!(caps[0]["name"], "week");
    assert_eq!(caps[0]["label"], "Claude Ultra weekly allowance");
    assert_eq!(caps[0]["used"], 0.0);
}

#[test]
fn a_window_used_up_refuses_and_one_past_its_warning_is_notified() {
    let mut feed = Feed::new(login("pro"));
    let mut endpoint = Recorded::answering(200, &answer("used-up"));

    let read = feed.read(&mut endpoint, at(NOW));

    let states: Vec<(Value, Value)> = kinds(&read.lines, "cap")
        .into_iter()
        .map(|cap| (cap["name"].clone(), cap["state"].clone()))
        .collect();
    assert_eq!(
        states,
        [
            (json!("five-hours"), json!("refused")),
            (json!("week"), json!("notified"))
        ]
    );
}

#[test]
fn a_window_whose_reset_has_passed_is_not_said() {
    let mut feed = Feed::new(login("pro"));
    let mut endpoint = Recorded::answering(200, &answer("pro"));

    let read = feed.read(&mut endpoint, at("2026-11-06T18:30:00Z"));

    let names: Vec<Value> = kinds(&read.lines, "cap")
        .into_iter()
        .map(|cap| cap["name"].clone())
        .collect();
    assert_eq!(names, [json!("week")]);
}

#[test]
fn nothing_signed_in_is_no_fault_and_offers_a_sign_in() {
    let mut feed = Feed::new(login("none"));
    let mut endpoint = Recorded::answering(200, &answer("pro"));

    let read = feed.read(&mut endpoint, at(NOW));

    assert_eq!(health(&read.lines)["state"], "optional");
    assert_eq!(health(&read.lines)["signIn"], "sign-in");
    assert_eq!(read.outcome, Outcome::Unasked);
    assert!(
        endpoint.tokens.is_empty(),
        "nothing was signed in to ask with"
    );
}

#[test]
fn the_sign_in_it_offers_runs_pi_claude_login() {
    let feed = Feed::new(login("none"));

    let action = kinds(&feed.introduction(), "action").remove(0);

    assert_eq!(action["id"], "sign-in");
    assert_eq!(
        action["run"],
        json!(["kitty", "--hold", "pi-claude", "login"])
    );
}

#[test]
fn a_login_with_no_plan_in_it_is_nothing_signed_in() {
    let mut feed = Feed::new(login("mcp-only"));
    let mut endpoint = Recorded::answering(200, &answer("pro"));

    let read = feed.read(&mut endpoint, at(NOW));

    assert_eq!(health(&read.lines)["state"], "optional");
    assert!(
        endpoint.tokens.is_empty(),
        "nothing was signed in to ask with"
    );
}

#[test]
fn an_expired_sign_in_stalls_until_claude_code_renews_it() {
    let mut feed = Feed::new(login("expired"));
    let mut endpoint = Recorded::answering(200, &answer("pro"));

    let read = feed.read(&mut endpoint, at(NOW));

    assert_eq!(health(&read.lines)["state"], "stalled");
    assert_eq!(read.outcome, Outcome::Unasked);
    assert!(endpoint.tokens.is_empty(), "an expired token isn't sent");
}

#[test]
fn a_sign_in_that_cannot_read_usage_is_an_error_without_asking() {
    let mut feed = Feed::new(login("no-profile-scope"));
    let mut endpoint = Recorded::answering(200, &answer("pro"));

    let read = feed.read(&mut endpoint, at(NOW));

    assert_eq!(health(&read.lines)["state"], "error");
    assert!(
        endpoint.tokens.is_empty(),
        "a token without the scope isn't sent"
    );
}

#[test]
fn a_sign_in_that_does_not_read_is_an_error() {
    let mut feed = Feed::new(login("garbled"));
    let mut endpoint = Recorded::answering(200, &answer("pro"));

    let read = feed.read(&mut endpoint, at(NOW));

    assert_eq!(health(&read.lines)["state"], "error");
    assert_eq!(read.outcome, Outcome::Failed);
}

#[test]
fn an_answer_with_no_usable_window_is_an_error_and_no_cap() {
    let mut feed = Feed::new(login("pro"));
    let mut endpoint = Recorded::answering(200, &answer("no-window"));

    let read = feed.read(&mut endpoint, at(NOW));

    assert!(kinds(&read.lines, "cap").is_empty(), "nothing is guessed");
    assert_eq!(health(&read.lines)["state"], "error");
    assert_eq!(read.outcome, Outcome::Failed);
}

#[test]
fn an_answer_that_is_not_json_is_an_error() {
    let mut feed = Feed::new(login("pro"));
    let mut endpoint = Recorded::answering(200, "<html>Service Unavailable</html>");

    let read = feed.read(&mut endpoint, at(NOW));

    assert_eq!(health(&read.lines)["state"], "error");
    assert_eq!(read.outcome, Outcome::Failed);
}

#[test]
fn a_refused_token_is_signed_out() {
    let mut feed = Feed::new(login("pro"));
    let mut endpoint = Recorded::answering(401, r#"{"type":"error"}"#);

    let read = feed.read(&mut endpoint, at(NOW));

    assert_eq!(health(&read.lines)["state"], "signed-out");
    assert_eq!(health(&read.lines)["signIn"], "sign-in");
}

#[test]
fn a_server_s_error_is_an_error_with_its_status() {
    let mut feed = Feed::new(login("pro"));
    let mut endpoint = Recorded::answering(503, "");

    let read = feed.read(&mut endpoint, at(NOW));

    let message = health(&read.lines)["message"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    assert!(message.contains("503"), "the status is said: {message}");
    assert_eq!(read.outcome, Outcome::Failed);
}

#[test]
fn an_endpoint_out_of_reach_is_an_error_and_its_cost_is_said() {
    let mut feed = Feed::new(login("pro"));
    let mut endpoint = Recorded {
        answer: Err(Unreached {
            why: "dns failed".into(),
            millis: 30,
        }),
        tokens: Vec::new(),
    };

    let read = feed.read(&mut endpoint, at(NOW));

    assert_eq!(health(&read.lines)["state"], "error");
    assert_eq!(kinds(&read.lines, "read")[0]["millis"], 30);
}

#[test]
fn being_asked_to_wait_stalls_and_asks_nothing_until_the_wait_is_over() {
    let mut feed = Feed::new(login("pro"));
    let mut endpoint = Recorded::answering(429, "");
    if let Ok(answer) = &mut endpoint.answer {
        answer.retry_after = Some(Duration::from_secs(300));
    }

    let first = feed.read(&mut endpoint, at(NOW));
    let meanwhile = feed.read(&mut endpoint, at("2026-11-06T15:01:00Z"));

    assert_eq!(first.outcome, Outcome::Paused(at("2026-11-06T15:05:00Z")));
    assert_eq!(health(&meanwhile.lines)["state"], "stalled");
    assert_eq!(meanwhile.outcome, first.outcome);
    assert_eq!(endpoint.tokens.len(), 1, "it waited as it was asked to");
}

#[test]
fn a_wait_with_no_length_given_is_five_minutes() {
    let mut feed = Feed::new(login("pro"));
    let mut endpoint = Recorded::answering(429, "");

    let read = feed.read(&mut endpoint, at(NOW));

    assert_eq!(read.outcome, Outcome::Paused(at("2026-11-06T15:05:00Z")));
}

/// An endpoint giving one recorded answer to every request, and keeping the
/// tokens it was asked with.
struct Recorded {
    answer: Result<Answer, Unreached>,
    tokens: Vec<String>,
}

impl Recorded {
    fn answering(status: u16, body: &str) -> Recorded {
        Recorded {
            answer: Ok(Answer {
                status,
                body: body.into(),
                retry_after: None,
                millis: 120,
            }),
            tokens: Vec::new(),
        }
    }
}

impl Endpoint for Recorded {
    fn usage(&mut self, token: &str) -> Result<Answer, Unreached> {
        self.tokens.push(token.into());
        self.answer.clone()
    }
}

fn login(name: &str) -> std::path::PathBuf {
    [
        env!("CARGO_MANIFEST_DIR"),
        "tests",
        "logins",
        &format!("{name}.json"),
    ]
    .iter()
    .collect()
}

fn answer(name: &str) -> String {
    let path = format!("{}/tests/answers/{name}.json", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
}

fn at(instant: &str) -> Timestamp {
    instant
        .parse()
        .unwrap_or_else(|error| panic!("{instant}: {error}"))
}

fn records(lines: &[String]) -> Vec<Value> {
    lines
        .iter()
        .map(|line| serde_json::from_str(line).unwrap_or_else(|error| panic!("{line}: {error}")))
        .collect()
}

fn records_of(text: &str) -> Vec<Value> {
    records(&text.lines().map(String::from).collect::<Vec<_>>())
}

fn kinds(lines: &[String], kind: &str) -> Vec<Value> {
    records(lines)
        .into_iter()
        .filter(|record| record["kind"] == kind)
        .collect()
}

fn health(lines: &[String]) -> Value {
    let mut said = kinds(lines, "health");
    assert_eq!(said.len(), 1, "one read says one health: {said:?}");
    said.remove(0)
}
