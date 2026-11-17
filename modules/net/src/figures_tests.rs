use super::rate;
use serde_json::Value;

/// The cases the page's words.ts is held to as well, so the two say every
/// rate alike, a hair under a half and an exact half included.
const SHARED: &str = include_str!("../site/tests/rates.json");

#[test]
fn a_rate_reads_as_the_page_says_it() {
    let cases: Vec<Value> = serde_json::from_str(SHARED).expect("the shared cases parse");
    assert!(!cases.is_empty(), "the shared cases are there");
    for case in cases {
        let mbps = case["mbps"].as_f64().expect("each case has a rate");
        let says = case["says"].as_str().expect("each case says it");
        assert_eq!(rate(mbps), says, "{mbps} Mb/s");
    }
}

#[test]
fn a_rate_of_no_number_is_none() {
    for none in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MIN_POSITIVE / 2.0,
    ] {
        assert_eq!(rate(none), "0 b/s", "{none}");
    }
}
