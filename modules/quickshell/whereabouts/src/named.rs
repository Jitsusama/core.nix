//! What a position is called and the time zone it keeps, asked of the web
//! once for each new here, so the launcher says a place, "Kelpwater, Westmark",
//! rather than its degrees, and can say when the clock's zone isn't the
//! place's.
//!
//! The name comes from OpenStreetMap's Nominatim, to a town's detail, and
//! the zone from Open-Meteo, which the weather asks already. Each is asked
//! with the position to a hundredth of a degree, about a kilometre, which
//! is all either needs. Nominatim asks that a program say who it is, and
//! ask no more than once a second; this asks once each time here moves.
//!
//! The zone is asked at the edge of where the laptop may be too, eight
//! points round the fix's circle in the same asking, so the clock moves
//! only where the whole circle keeps the zone's time: a fix to a region's
//! width near a zone's line could name either side, and a clock moved to
//! the wrong one is worse than one left (clock-follows/).
//!
//! A place Joel names himself is found the other way, from its name, by
//! Open-Meteo's geocoding, which says its town, province and zone at once.

use std::time::Duration;

use serde_json::Value;

/// Who's asking, as Nominatim's policy wants every program to say.
const USER_AGENT: &str = "whereabouts (core.nix, the launcher's here)";

/// How long either may take before here goes unnamed for now.
const PATIENCE: Duration = Duration::from_secs(10);

/// How many points round a fix's circle its zone is asked at besides its
/// middle, a bearing apart each: eight, so a zone's line has to cut off
/// less than an eighth of the circle to pass between them.
const EDGE: u32 = 8;

/// The Earth's mean radius, in metres, for the points on a fix's edge.
const EARTH: f64 = 6_371_000.0;

/// What the oceans' zones are named under, "Etc/GMT+4": a point at sea is
/// nowhere Joel's laptop is, so it says nothing of whether the circle
/// keeps the zone's time.
const AT_SEA: &str = "Etc/";

/// The name, the country and the zone of a position, each missing when its
/// service couldn't be reached or didn't know, and whether every point on
/// its circle that's on land keeps the zone's time now, missing likewise.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Named {
    pub place: Option<String>,
    pub country: Option<String>,
    pub zone: Option<String>,
    pub zone_holds: Option<bool>,
}

/// Asks both services about the position, on a thread of its own, since
/// each waits on the network.
pub async fn name(latitude: f64, longitude: f64, accuracy: f64) -> Named {
    let points = circle(latitude, longitude, accuracy);
    let (latitude, longitude) = points[0];
    tokio::task::spawn_blocking(move || {
        let agent = agent();
        // The place and its country are one answer of Nominatim's.
        let placed = asked(&agent, &place_url(latitude, longitude));
        // Should the asking round the circle be refused, the middle's zone
        // alone is asked, so here's still named, nothing said of the edge.
        let (zone, zone_holds) = match asked(&agent, &zone_url(&points)) {
            Some(said) => held(&said, points.len()),
            None => (
                asked(&agent, &zone_url(&points[..1])).and_then(|said| zone(&said)),
                None,
            ),
        };
        Named {
            place: placed.as_ref().and_then(place),
            country: placed.as_ref().and_then(country),
            zone,
            zone_holds,
        }
    })
    .await
    .unwrap_or_default()
}

/// The fix's middle, then the points on its edge, `accuracy` metres from it
/// a bearing apart, each to a hundredth of a degree.
fn circle(latitude: f64, longitude: f64, accuracy: f64) -> Vec<(f64, f64)> {
    let angle = accuracy.max(0.0) / EARTH;
    let (from, along) = (latitude.to_radians(), longitude.to_radians());
    let edge = (0..EDGE).map(|at| {
        let bearing = f64::from(at) * std::f64::consts::TAU / f64::from(EDGE);
        let to = (from.sin() * angle.cos() + from.cos() * angle.sin() * bearing.cos()).asin();
        let across = along
            + (bearing.sin() * angle.sin() * from.cos()).atan2(angle.cos() - from.sin() * to.sin());
        // Back within a half turn of Greenwich, as a longitude is said.
        let across = (across + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU)
            - std::f64::consts::PI;
        (hundredths(to.to_degrees()), hundredths(across.to_degrees()))
    });
    std::iter::once((hundredths(latitude), hundredths(longitude)))
        .chain(edge)
        .collect()
}

/// The zone Open-Meteo says the middle keeps, and whether every point on
/// the edge it places on land keeps the same time now, of the `asked`
/// points. It answers one point alone, not a list of one, and two zones
/// may keep the same time by other names, so it's the offset from UTC
/// that's held alike. It holds only where every point was answered and
/// all those on land agree: an answer short of a point, or a point with
/// no zone, leaves it unknown, unless another already keeps another time.
fn held(said: &Value, asked: usize) -> (Option<String>, Option<bool>) {
    let answers: Vec<&Value> = match said {
        Value::Array(each) => each.iter().collect(),
        one => vec![one],
    };
    let Some((middle, edge)) = answers.split_first() else {
        return (None, None);
    };
    let Some(zone) = zone(middle) else {
        return (None, None);
    };
    let offset = |said: &Value| said.get("utc_offset_seconds").and_then(Value::as_i64);
    let Some(kept) = offset(middle) else {
        return (Some(zone), None);
    };
    // Each point on the edge: Some(whether it keeps the time) on land,
    // Some(true) at sea, which says nothing against it, None unknown.
    let keeps = |said: &&Value| match said.get("timezone").and_then(Value::as_str) {
        Some(at) if at.starts_with(AT_SEA) => Some(true),
        Some(at) if at.contains('/') => offset(said).map(|their| their == kept),
        _ => None,
    };
    let each: Vec<Option<bool>> = edge.iter().map(keeps).collect();
    let holds = if each.contains(&Some(false)) {
        Some(false)
    } else if answers.len() == asked && each.iter().all(Option::is_some) {
        Some(true)
    } else {
        None
    };
    (Some(zone), holds)
}

/// A place found by its name: where it is, what it's called and its zone.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub latitude: f64,
    pub longitude: f64,
    pub place: String,
    pub country: Option<String>,
    pub zone: Option<String>,
}

/// Why a place named couldn't be found.
#[derive(Debug, PartialEq)]
pub enum Unfound {
    /// The geocoding couldn't be asked, as with no network.
    Unasked,
    /// It was asked, and knows no place by that name.
    Nowhere,
}

/// Finds the place named, the likeliest first, waiting on the web, so
/// it's asked from a thread that may wait: the one serving the connection
/// that asked, or `set` run by hand.
pub fn find(words: &str) -> Result<Found, Unfound> {
    let said = agent()
        .get("https://geocoding-api.open-meteo.com/v1/search")
        .query("name", words)
        .query("count", "1")
        .query("language", "en")
        .query("format", "json")
        .call()
        .ok()
        .and_then(|mut answer| answer.body_mut().read_to_string().ok())
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .ok_or(Unfound::Unasked)?;
    found(&said).ok_or(Unfound::Nowhere)
}

/// The first place a geocoding answer gives, by its town and province as
/// `place` says them, or nothing when it gives none.
pub fn found(said: &Value) -> Option<Found> {
    let first = said.get("results")?.get(0)?;
    let text = |key: &str| {
        first
            .get(key)
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
    };
    let town = text("name")?;
    Some(Found {
        latitude: first.get("latitude")?.as_f64()?,
        longitude: first.get("longitude")?.as_f64()?,
        place: match text("admin1") {
            Some(state) if state != town => format!("{town}, {state}"),
            _ => town.to_owned(),
        },
        country: text("country").map(str::to_owned),
        zone: zone(first),
    })
}

fn hundredths(degrees: f64) -> f64 {
    (degrees * 100.0).round() / 100.0
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(PATIENCE))
        .user_agent(USER_AGENT)
        .build()
        .into()
}

fn place_url(latitude: f64, longitude: f64) -> String {
    format!(
        "https://nominatim.openstreetmap.org/reverse?format=jsonv2&zoom=10\
         &accept-language=en&lat={latitude}&lon={longitude}"
    )
}

/// Open-Meteo's forecast at each point, asked at once, for the zone it
/// says each keeps.
fn zone_url(points: &[(f64, f64)]) -> String {
    let listed = |each: fn(&(f64, f64)) -> f64| {
        points
            .iter()
            .map(|point| each(point).to_string())
            .collect::<Vec<_>>()
            .join(",")
    };
    format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}\
         &timezone=auto&forecast_days=1",
        listed(|point| point.0),
        listed(|point| point.1)
    )
}

/// What a service said, as JSON, or nothing when it couldn't be asked.
fn asked(agent: &ureq::Agent, url: &str) -> Option<Value> {
    let text = agent
        .get(url)
        .call()
        .ok()?
        .body_mut()
        .read_to_string()
        .ok()?;
    serde_json::from_str(&text).ok()
}

/// The place Nominatim says, as a person names it: its town and the
/// province or state it's in, "Kelpwater, Westmark", or the town alone where
/// there's no such part.
pub fn place(said: &Value) -> Option<String> {
    let address = said.get("address")?;
    let part = |keys: &[&str]| {
        keys.iter()
            .find_map(|key| address.get(*key).and_then(Value::as_str))
            .filter(|name| !name.is_empty())
    };
    let town = part(&[
        "city",
        "town",
        "village",
        "municipality",
        "hamlet",
        "county",
    ])?;
    Some(match part(&["state", "province", "region"]) {
        Some(state) if state != town => format!("{town}, {state}"),
        _ => town.to_owned(),
    })
}

/// The country Nominatim says the position is in, "Canada", as English
/// names it, since the place is asked in English.
pub fn country(said: &Value) -> Option<String> {
    said.get("address")?
        .get("country")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
}

/// The zone Open-Meteo says the position keeps, "America/Toronto".
pub fn zone(said: &Value) -> Option<String> {
    said.get("timezone")
        .and_then(Value::as_str)
        .filter(|zone| zone.contains('/'))
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_place_is_its_town_and_its_province() {
        let said =
            json!({"address": {"city": "Kelpwater", "state": "Westmark", "country": "Canada"}});
        assert_eq!(place(&said).as_deref(), Some("Kelpwater, Westmark"));
    }

    #[test]
    fn a_place_with_no_city_is_its_town_or_village() {
        let said = json!({"address": {"village": "Fjord", "state": "Lowmark"}});
        assert_eq!(place(&said).as_deref(), Some("Fjord, Lowmark"));
        let said = json!({"address": {"town": "Cove"}});
        assert_eq!(place(&said).as_deref(), Some("Cove"));
    }

    #[test]
    fn a_city_that_is_its_own_state_is_said_once() {
        let said = json!({"address": {"city": "Quaystead", "state": "Quaystead"}});
        assert_eq!(place(&said).as_deref(), Some("Quaystead"));
    }

    #[test]
    fn a_place_is_in_the_country_nominatim_says() {
        let said =
            json!({"address": {"city": "Kelpwater", "state": "Westmark", "country": "Canada"}});
        assert_eq!(country(&said).as_deref(), Some("Canada"));
        assert_eq!(country(&json!({"address": {"city": "Cove"}})), None);
        assert_eq!(country(&json!({"address": {"country": ""}})), None);
    }

    #[test]
    fn nowhere_named_is_no_place() {
        assert_eq!(place(&json!({"error": "Unable to geocode"})), None);
        assert_eq!(place(&json!({"address": {"country": "Canada"}})), None);
        assert_eq!(place(&json!({"address": {"city": ""}})), None);
    }

    #[test]
    fn a_zone_is_the_one_open_meteo_says() {
        assert_eq!(
            zone(&json!({"timezone": "America/Toronto", "utc_offset_seconds": -14400})).as_deref(),
            Some("America/Toronto")
        );
        // GMT is what it says where it couldn't tell, which is no zone.
        assert_eq!(zone(&json!({"timezone": "GMT"})), None);
        assert_eq!(zone(&json!({})), None);
    }

    #[test]
    fn a_place_named_is_its_first_answer() {
        let said = json!({"results": [
            {"name": "Kelpwater", "latitude": 52.34, "longitude": -115.67,
             "admin1": "Highreach", "country": "Canada", "timezone": "America/Edmonton"},
            {"name": "Kelpwater", "latitude": 1.0, "longitude": 2.0, "admin1": "Elsewhere"}
        ]});
        assert_eq!(
            found(&said),
            Some(Found {
                latitude: 52.34,
                longitude: -115.67,
                place: "Kelpwater, Highreach".into(),
                country: Some("Canada".into()),
                zone: Some("America/Edmonton".into()),
            })
        );
    }

    #[test]
    fn a_name_nowhere_has_is_no_place() {
        assert_eq!(found(&json!({"generationtime_ms": 0.5})), None);
        assert_eq!(found(&json!({"results": []})), None);
        assert_eq!(found(&json!({"results": [{"name": "Cove"}]})), None);
        let said = json!({"results": [{"name": "Quaystead", "admin1": "Quaystead",
                                       "latitude": 12.35, "longitude": -98.76}]});
        assert_eq!(
            found(&said).map(|place| place.place).as_deref(),
            Some("Quaystead")
        );
    }

    #[test]
    fn a_position_is_asked_to_a_hundredth_of_a_degree() {
        assert!(
            place_url(hundredths(12.345_6), hundredths(-98.765_4)).contains("lat=12.35&lon=-98.77")
        );
        assert!(zone_url(&[(1.0, 2.5)]).contains("latitude=1&longitude=2.5&timezone=auto"));
    }

    #[test]
    fn the_zone_is_asked_at_the_middle_and_round_the_edge() {
        let points = circle(12.345_6, -98.765_4, 26_000.0);
        assert_eq!(points.len(), 9, "the middle and eight on the edge");
        assert_eq!(points[0], (12.35, -98.77), "the middle comes first");
        // 26 km north is about 0.23 degrees of latitude; east, at this
        // latitude, about 0.24 of longitude.
        assert_eq!(points[1], (12.58, -98.77), "north first");
        assert_eq!(points[3], (12.35, -98.53), "east a quarter turn on");
        let url = zone_url(&points);
        assert!(
            url.contains("latitude=12.35,12.58,"),
            "every point in one asking: {url}"
        );
    }

    #[test]
    fn an_edge_across_the_date_line_stays_a_longitude() {
        let points = circle(0.0, 179.9, 50_000.0);
        assert!(
            points
                .iter()
                .all(|point| (-180.0..=180.0).contains(&point.1)),
            "each longitude within a half turn: {points:?}"
        );
    }

    fn at(zone: &str, offset: i64) -> Value {
        json!({"timezone": zone, "utc_offset_seconds": offset})
    }

    #[test]
    fn a_zone_holds_where_every_point_keeps_its_time() {
        let said = json!([
            at("America/Toronto", -18_000),
            at("America/Toronto", -18_000),
            at("America/New_York", -18_000),
            at("Etc/GMT+4", -14_400),
        ]);
        assert_eq!(
            held(&said, 4),
            (Some("America/Toronto".into()), Some(true)),
            "another zone's name at the same time holds, and a point at sea says nothing"
        );
    }

    #[test]
    fn a_zone_doesnt_hold_where_its_edge_keeps_another_time() {
        let said = json!([
            at("America/Toronto", -18_000),
            json!({}),
            at("America/Winnipeg", -21_600),
        ]);
        assert_eq!(
            held(&said, 9),
            (Some("America/Toronto".into()), Some(false)),
            "a circle a zone's line crosses, however much else is unknown"
        );
    }

    #[test]
    fn a_zone_is_unknown_round_a_circle_not_wholly_answered() {
        let toronto = (Some("America/Toronto".into()), None);
        assert_eq!(
            held(&at("America/Toronto", -18_000), 9),
            toronto,
            "one point answered of nine"
        );
        let short = json!([
            at("America/Toronto", -18_000),
            at("America/Toronto", -18_000)
        ]);
        assert_eq!(held(&short, 9), toronto, "a list short of the points");
        let zoneless = json!([
            at("America/Toronto", -18_000),
            json!({"utc_offset_seconds": 0})
        ]);
        assert_eq!(held(&zoneless, 2), toronto, "an edge point with no zone");
        assert_eq!(
            held(&at("America/Halifax", -14_400), 1),
            (Some("America/Halifax".into()), Some(true)),
            "a fix with no edge to ask, its zone holding"
        );
        assert_eq!(held(&json!([]), 9), (None, None), "nothing answered");
        assert_eq!(
            held(&json!({"timezone": "America/Halifax"}), 1),
            (Some("America/Halifax".into()), None),
            "a zone without its offset can't be held alike"
        );
    }
}
