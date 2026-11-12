//! The Home Network's own drawer, made from the model the contract draws
//! (home-network.json) in the launcher's words (tests/words.json).

use super::*;

fn model() -> HomeNetwork {
    serde_json::from_str(include_str!("../home-network.json")).expect("the model parses")
}

fn words() -> Words {
    let vocabulary: Value =
        serde_json::from_str(include_str!("../tests/words.json")).expect("the words parse");
    Words::read(&vocabulary).expect("the vocabulary has the Home Network's words")
}

fn drawer(model: &HomeNetwork) -> Value {
    record(model, &words(), &Numbers::default())["drawer"].clone()
}

fn labels(drawer: &Value) -> Vec<&str> {
    drawer["facts"]
        .as_array()
        .expect("facts are a list")
        .iter()
        .filter_map(|fact| fact["name"].as_str())
        .collect()
}

#[test]
fn the_drawer_says_whose_it_is_how_the_home_network_stands_and_its_figures() {
    let made = record(&model(), &words(), &Numbers::default());
    assert_eq!(
        (made["key"].as_str(), made["kind"].as_str()),
        (Some("drawer:eero:place"), Some("drawer")),
        "it's a record of its own, beside the rows"
    );
    let drawer = &made["drawer"];
    assert_eq!(
        drawer["eyebrow"], "via eero \u{b7} network",
        "the way, then the kind"
    );
    assert_eq!(drawer["title"], "Home Network");
    assert_eq!(
        drawer["body"],
        "Connected at 370 Mb/s down, with 3 nodes online and 15 devices on now."
    );
    assert_eq!(
        drawer["facts"],
        json!([
            {"name": "State", "said": "connected"},
            {"name": "Provider", "said": "Example ISP", "identifies": true},
            {"name": "Down", "said": "370 Mb/s", "figure": true},
            {"name": "Up", "said": "11.9 Mb/s", "figure": true},
            {"name": "Speed Test", "at": 1_793_875_935_000_i64, "as": "on"},
            {"name": "Outages", "said": "1 outage in 30 days"},
            {"name": "Nodes", "said": "3 nodes online"},
        ]),
        "its figures, the test's day as a time so it never changes as the day does"
    );
}

#[test]
fn the_lead_says_the_internet_is_down_and_no_speed_where_none_was_tested() {
    let mut model = model();
    model.internet.state = Some("disconnected".into());
    model.internet.speed_test.down_mbps = None;
    model.internet.speed_test.up_mbps = None;
    model.internet.speed_test.at = None;
    let drawer = drawer(&model);
    assert_eq!(
        drawer["body"],
        "Not connected to the internet, with 3 nodes online and 15 devices on now."
    );
    assert_eq!(
        labels(&drawer),
        ["State", "Provider", "Outages", "Nodes"],
        "no test, so no speed and no day"
    );
}

#[test]
fn out_of_eeros_reach_it_says_why_and_how_it_stands_alone() {
    let mut model = model();
    model.cloud.state = CloudState::Unreachable;
    let drawer = drawer(&model);
    assert_eq!(
        drawer["body"],
        "The eero cloud can't be reached, so how the home network stands isn't known."
    );
    assert_eq!(
        drawer["facts"],
        json!([{"name": "State", "said": "eero unreachable"}]),
        "the rest are its last, so they aren't said"
    );
    model.cloud.state = CloudState::SignedOut;
    assert_eq!(
        drawer_body(&model),
        "Signed out of eero, so how the home network stands isn't known."
    );
}

#[test]
fn a_way_of_standing_eero_has_no_sentence_for_leads_with_nothing() {
    let mut model = model();
    model.internet.state = Some("rebooting".into());
    assert_eq!(drawer_body(&model), "", "nothing rather than a guess");
}

fn drawer_body(model: &HomeNetwork) -> String {
    drawer(model)["body"].as_str().unwrap_or("").to_owned()
}
