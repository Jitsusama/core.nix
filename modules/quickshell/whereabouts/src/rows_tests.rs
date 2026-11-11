//! Location's rows as the launcher's Now drew them before the daemon
//! served them (tests/now-moment.qml's cases), each kept as it read there.

use super::*;
use crate::words::fixture;

/// Kelpwater, found by Wi-Fi to 26 km, in a zone the clock keeps too.
fn kelpwater() -> Fix {
    let mut fix = Fix::new(12.3457, -98.7654, 26_000.0, 1_790_000_000)
        .unwrap()
        .named(
            Some("Kelpwater, Westmark".into()),
            Some("America/Toronto".into()),
        )
        .countried(Some("Canada".into()));
    fix.source = Some("wifi".into());
    fix
}

/// Twelve minutes after Kelpwater was found, in milliseconds.
const NOW: u64 = 1_790_000_000_000 + 720_000;

fn by_key<'a>(rows: &'a [Value], key: &str) -> &'a Value {
    rows.iter()
        .find(|row| row["key"] == key)
        .unwrap_or_else(|| panic!("no row {key} among {rows:?}"))
}

fn keys(rows: &[Value]) -> Vec<&str> {
    rows.iter()
        .map(|row| row["key"].as_str().unwrap())
        .collect()
}

#[test]
fn here_says_its_town_and_how_near_it_is() {
    let rows = rows(Some(&kelpwater()), Some("America/Toronto"), &fixture(), NOW);
    let on = by_key(&rows, ON);
    assert_eq!(
        (&on["name"], &on["side"], &on["note"], &on["hue"]),
        (
            &json!("Location"),
            &json!([{"said": "Kelpwater, Westmark"}]),
            &json!("within 26 km"),
            &json!("voice")
        ),
        "its own row: {on}"
    );
    assert_eq!(
        (&on["drawer"]["eyebrow"], &on["drawer"]["title"]),
        (&json!("via Wi-Fi \u{b7} location"), &json!("Kelpwater")),
        "its plate's eyebrow and title"
    );
    let plate: Vec<&str> = on["drawer"]["facts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|fact| fact["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        plate,
        ["located", "coordinates", "country", "zone"],
        "how near left to the note"
    );
    assert_eq!(
        on["drawer"]["body"], "Wi-Fi gives no altitude, speed or heading.",
        "what Wi-Fi couldn't give"
    );
    assert_eq!(
        keys(&rows[1..]),
        [
            "row:location:town",
            "row:location:country",
            "row:location:knows",
            "row:location:located",
            "row:location:set",
            "row:location:copy",
        ],
        "inside, in the design's order, with no Find It Again unpinned"
    );
    assert_eq!(
        by_key(&rows, "row:location:knows")["side"],
        json!([{"said": "found, within 26 km"}])
    );
    let town = by_key(&rows, "row:location:town");
    assert_eq!(
        (&town["thing"], &town["acts"]),
        (
            &json!("location:town"),
            &json!([{"key": "verb:copy", "copy": "Kelpwater, Westmark"}])
        ),
        "a figure copies what it says"
    );
    assert!(
        rows.iter().all(|row| row.get("order").is_none()),
        "Kept places them, in the order handed over: {rows:?}"
    );
    assert_eq!(
        by_key(&rows, "row:location:located")["side"],
        json!([{"at": 1_790_000_000_000_u64, "as": "ago"}]),
        "when, a time"
    );
    let copy = by_key(&rows, "row:location:copy");
    assert_eq!(
        (&copy["side"], &copy["acts"]),
        (
            &json!([{"said": "12.35, -98.77"}]),
            &json!([{"key": "verb:location:copy", "copy": "12.35, -98.77"}])
        ),
        "Copy, of its position"
    );
    let set = by_key(&rows, "row:location:set");
    assert_eq!(
        (&set["thing"], &set["name"], &set["acts"]),
        (
            &json!("verb:location:set"),
            &json!("Set It by Hand"),
            &json!([{"key": "verb:location:set", "ask": {"ask": "act", "act": "set"}}])
        ),
        "Set It by Hand asks this daemon, its verb and question the shell's by its key"
    );
}

#[test]
fn another_zone_than_the_clocks_is_said_strongly_with_why_it_isnt_followed() {
    let mut wide = kelpwater();
    wide.accuracy = 80_000.0;
    wide.zone_holds = Some(true);
    let rows = rows(Some(&wide), Some("America/Halifax"), &fixture(), NOW);
    let on = by_key(&rows, ON);
    assert_eq!(
        (&on["note"], &on["hue"]),
        (
            &json!("keeps America/Toronto \u{b7} not following \u{b7} found within 80 km"),
            &json!("strong")
        ),
        "{on}"
    );
    assert_eq!(
        on["drawer"]["facts"][1]["id"], "accuracy",
        "how near stays on the plate while the note says the zone"
    );
}

#[test]
fn nowhere_known_is_said_plainly_and_set_by_hand_can_be_found_again() {
    let rows_unknown = rows(None, Some("America/Halifax"), &fixture(), NOW);
    let on = by_key(&rows_unknown, ON);
    assert_eq!(
        (&on["side"], &on["note"], on.get("drawer")),
        (&json!([{"said": "unknown"}]), &json!("not found yet"), None),
        "never guessed: {on}"
    );
    assert_eq!(
        keys(&rows_unknown[1..]),
        [
            "row:location:town",
            "row:location:knows",
            "row:location:set"
        ],
        "only what's known, and setting it by hand"
    );
    let mut pinned = kelpwater().by_hand();
    pinned.source = None;
    let rows_pinned = rows(Some(&pinned), Some("America/Toronto"), &fixture(), NOW);
    assert_eq!(
        by_key(&rows_pinned, ON)["note"],
        "set by hand",
        "a place Joel set says so"
    );
    assert_eq!(
        by_key(&rows_pinned, "row:location:again")["acts"],
        json!([{"key": "verb:location:again", "ask": {"ask": "act", "act": "again"}}]),
        "and can be found again"
    );
}

#[test]
fn a_day_on_its_time_says_its_day() {
    let fix = kelpwater();
    let later = NOW + 86_400_000;
    let rows = rows(Some(&fix), None, &fixture(), later);
    assert_eq!(
        by_key(&rows, "row:location:located")["side"][0]["as"],
        "moment"
    );
    assert_eq!(turns(Some(&fix), NOW), Some(1_790_086_400_000));
    assert_eq!(turns(Some(&fix), later), None, "turned once only");
}
