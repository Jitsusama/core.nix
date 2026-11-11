//! Where the laptop is, said as Location says it: the cases
//! tests/answers.qml held launcher/Here.js to, each kept as it read there.

use super::*;
use crate::words::fixture;

/// A fix where the cases put it, to `accuracy` metres.
fn at(latitude: f64, longitude: f64, accuracy: f64) -> Fix {
    let mut fix = Fix::new(latitude, longitude, 26_000.0, 1).expect("a place on the Earth");
    fix.accuracy = accuracy;
    fix
}

/// A fix GPS found, with as many of its three figures as given.
fn gps(altitude: Option<f64>, speed: Option<f64>, heading: Option<f64>) -> Fix {
    let mut fix = at(12.3457, -98.7654, 5.0);
    fix.source = Some("gps".into());
    fix.altitude = altitude;
    fix.speed = speed;
    fix.heading = heading;
    fix
}

/// A fix in another zone, by its width and whether its zone holds.
fn unfollowed_by(accuracy: f64, holds: Option<bool>) -> String {
    let mut fix = at(1.0, 1.0, accuracy);
    fix.zone_holds = holds;
    unfollowed(&fix, &fixture())
}

/// The facts' ids and what each says, for the facts after the first two.
fn said(facts: &[Value]) -> Vec<String> {
    facts
        .iter()
        .map(|fact| format!("{} {}", fact["id"].as_str().unwrap(), fact["said"]))
        .collect()
}

#[test]
fn a_position_claims_the_places_its_accuracy_knows() {
    let words = fixture();
    let cases = [
        ((12.345_678, -98.765_4, 5.0), "12.3457° N · 98.7654° W"),
        ((12.345_678, -98.765_4, 80.0), "12.346° N · 98.765° W"),
        ((12.345_678, -98.765_4, 100.0), "12.346° N · 98.765° W"),
        ((-12.345_6, 123.456_7, 5000.0), "12.3° S · 123.5° E"),
        ((12.35, -98.76, 500_000.0), "12° N · 99° W"),
        ((-0.04, 0.04, 26_000.0), "0.0° · 0.0°"),
    ];
    for ((latitude, longitude, accuracy), expected) in cases {
        assert_eq!(
            coordinates(&at(latitude, longitude, accuracy), &words),
            expected,
            "a position to {accuracy} m"
        );
    }
}

#[test]
fn how_near_is_said_to_three_figures_rounded_up() {
    let words = fixture();
    let cases = [
        (80.0, "within 80 m"),
        (999.5, "within 1 km"),
        (1001.0, "within 1.01 km"),
        (26_000.0, "within 26 km"),
        (100.2, "within 101 m"),
        (1_234_000.0, "within 1,240 km"),
    ];
    for (metres, expected) in cases {
        assert_eq!(within(metres, &words), expected, "{metres} m");
    }
}

#[test]
fn when_it_was_found_is_ago_within_the_day_and_a_moment_after() {
    let fix = Fix::new(1.0, 1.0, 5.0, 1000).unwrap();
    assert_eq!(
        located(&fix, 1_000_000 + 720_000),
        json!({"at": 1_000_000, "as": "ago"}),
        "within the day"
    );
    assert_eq!(
        located(&fix, 1_000_000 + 3 * DAY_MS),
        json!({"at": 1_000_000, "as": "moment"}),
        "days ago"
    );
    assert_eq!(
        turns(&fix, 1_000_000),
        Some(1_000_000 + DAY_MS),
        "it turns a day on"
    );
    assert_eq!(turns(&fix, 1_000_000 + DAY_MS), None, "and once only");
}

#[test]
fn the_clock_not_following_says_why() {
    assert_eq!(
        unfollowed_by(3000.0, Some(false)),
        "near a time zone's edge",
        "a circle a zone's line crosses isn't followed"
    );
    assert_eq!(
        unfollowed_by(80_000.0, Some(true)),
        "found within 80 km",
        "one wider than a region says its width"
    );
    assert_eq!(
        unfollowed_by(26_000.0, Some(true)),
        "",
        "one that's followed says nothing more"
    );
    assert_eq!(
        (unfollowed_by(9000.0, None), unfollowed_by(26_000.0, None)),
        (String::new(), "found within 26 km".to_owned()),
        "one unsaid round its circle is followed only within 10 km"
    );
    assert_eq!(
        unfollowed_by(f64::NAN, Some(true)),
        "",
        "one of no width says nothing"
    );
}

#[test]
fn the_eyebrow_says_what_found_it() {
    let words = fixture();
    let found_by = |source: Option<&str>, hand: bool| {
        let mut fix = at(1.0, 1.0, 5.0);
        fix.source = source.map(str::to_owned);
        fix.hand = hand;
        eyebrow(&fix, &words)
    };
    assert_eq!(found_by(Some("wifi"), false), "via Wi-Fi · location");
    assert_eq!(
        found_by(Some("galileo"), false),
        "found · location",
        "a source unknown"
    );
    assert_eq!(found_by(None, false), "found · location", "none named");
    assert_eq!(
        found_by(Some("wifi"), true),
        "set by hand · location",
        "set by hand, though a source is named"
    );
}

#[test]
fn gpss_figures_follow_where_on_a_map() {
    let words = fixture();
    let plate = facts(
        &gps(Some(212.4), Some(4.26), Some(274.6)),
        1000,
        &words,
        true,
    );
    assert_eq!(
        said(&plate[1..]),
        [
            r#"coordinates "12.3457° N · 98.7654° W""#,
            r#"altitude "212 m""#,
            r#"speed "15.3 km/h""#,
            r#"heading "275°""#,
        ],
        "speed in km/h, and how near left to the note"
    );
    assert_eq!(plate[0]["as"], "ago", "found first, a time");
    let speeds: Vec<String> = [0.0, 1.2, 27.8, 10.0]
        .iter()
        .map(|speed| {
            let plate = facts(&gps(None, Some(*speed), None), 1000, &words, true);
            let speed = plate.iter().find(|fact| fact["id"] == "speed").unwrap();
            speed["said"].as_str().unwrap().to_owned()
        })
        .collect();
    assert_eq!(
        speeds,
        ["0 km/h", "4.32 km/h", "100 km/h", "36.0 km/h"],
        "none is held and said, a walk keeps three figures, a highway none past them, \
         a round one its zeros"
    );
    let wide = facts(&at(1.0, 1.0, 26_000.0), 1000, &words, false);
    assert_eq!(
        wide[1]["said"], "within 26 km",
        "how near stays while the note says the zone instead"
    );
}

#[test]
fn what_its_source_couldnt_give_is_said_once() {
    let words = fixture();
    let wifi = {
        let mut fix = at(1.0, 1.0, 5.0);
        fix.source = Some("wifi".into());
        fix
    };
    let mut hand = at(1.0, 1.0, 5.0);
    hand.hand = true;
    let cases = [
        (wifi, "Wi-Fi gives no altitude, speed or heading."),
        (
            gps(Some(212.0), None, None),
            "GPS gave no speed or heading.",
        ),
        (gps(Some(212.0), Some(4.0), None), "GPS gave no heading."),
        (gps(Some(212.0), Some(4.0), Some(275.0)), ""),
        (
            at(1.0, 1.0, 5.0),
            "Its source gives no altitude, speed or heading.",
        ),
        (hand, ""),
    ];
    for (fix, expected) in cases {
        assert_eq!(left_out(&fix, &words), expected, "of {fix:?}");
    }
}

#[test]
fn a_number_is_grouped_by_thousands() {
    assert_eq!(number(1240.0, 0), "1,240");
    assert_eq!(number(1_234_567.25, 2), "1,234,567.25");
    assert_eq!(number(-12.0, 0), "-12", "below the sea");
    assert_eq!(number(-0.2, 0), "0", "none has no sign");
}
