//! What a position is called and the time zone it keeps, asked of the web
//! once for each new here, so the launcher says a place, "London, Ontario",
//! rather than its degrees, and can say when the clock's zone isn't the
//! place's.
//!
//! The name comes from OpenStreetMap's Nominatim, to a town's detail, and
//! the zone from Open-Meteo, which the weather asks already. Each is asked
//! with the position to a hundredth of a degree, about a kilometre, which
//! is all either needs. Nominatim asks that a program say who it is, and
//! ask no more than once a second; this asks once each time here moves.
//!
//! A place Joel names himself is found the other way, from its name, by
//! Open-Meteo's geocoding, which says its town, province and zone at once.

use std::time::Duration;

use serde_json::Value;

/// Who's asking, as Nominatim's policy wants every program to say.
const USER_AGENT: &str = "whereabouts (core.nix, the launcher's here)";

/// How long either may take before here goes unnamed for now.
const PATIENCE: Duration = Duration::from_secs(10);

/// The name and the zone of a position, either missing when its service
/// couldn't be reached or didn't know.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Named {
    pub place: Option<String>,
    pub zone: Option<String>,
}

/// Asks both services about the position, on a thread of its own, since
/// each waits on the network.
pub async fn name(latitude: f64, longitude: f64) -> Named {
    let (latitude, longitude) = (hundredths(latitude), hundredths(longitude));
    tokio::task::spawn_blocking(move || {
        let agent = agent();
        Named {
            place: asked(&agent, &place_url(latitude, longitude)).and_then(|said| place(&said)),
            zone: asked(&agent, &zone_url(latitude, longitude)).and_then(|said| zone(&said)),
        }
    })
    .await
    .unwrap_or_default()
}

/// A place found by its name: where it is, what it's called and its zone.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub latitude: f64,
    pub longitude: f64,
    pub place: String,
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

/// Finds the place named, the likeliest first, on a thread of its own.
pub async fn find(words: String) -> Result<Found, Unfound> {
    tokio::task::spawn_blocking(move || {
        let said = agent()
            .get("https://geocoding-api.open-meteo.com/v1/search")
            .query("name", &words)
            .query("count", "1")
            .query("language", "en")
            .query("format", "json")
            .call()
            .ok()
            .and_then(|mut answer| answer.body_mut().read_to_string().ok())
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .ok_or(Unfound::Unasked)?;
        found(&said).ok_or(Unfound::Nowhere)
    })
    .await
    .unwrap_or(Err(Unfound::Unasked))
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

fn zone_url(latitude: f64, longitude: f64) -> String {
    format!(
        "https://api.open-meteo.com/v1/forecast?latitude={latitude}&longitude={longitude}\
         &timezone=auto&forecast_days=1"
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
/// province or state it's in, "London, Ontario", or the town alone where
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
            json!({"address": {"city": "Lighthouse", "state": "Ontario", "country": "Canada"}});
        assert_eq!(place(&said).as_deref(), Some("Lighthouse, Ontario"));
    }

    #[test]
    fn a_place_with_no_city_is_its_town_or_village() {
        let said = json!({"address": {"village": "Fjord", "state": "Nova Scotia"}});
        assert_eq!(place(&said).as_deref(), Some("Fjord, Nova Scotia"));
        let said = json!({"address": {"town": "Cove"}});
        assert_eq!(place(&said).as_deref(), Some("Cove"));
    }

    #[test]
    fn a_city_that_is_its_own_state_is_said_once() {
        let said = json!({"address": {"city": "Berlin", "state": "Berlin"}});
        assert_eq!(place(&said).as_deref(), Some("Berlin"));
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
            {"name": "Lighthouse", "latitude": 51.05, "longitude": -114.07,
             "admin1": "Alberta", "timezone": "America/Edmonton"},
            {"name": "Lighthouse", "latitude": 1.0, "longitude": 2.0, "admin1": "Elsewhere"}
        ]});
        assert_eq!(
            found(&said),
            Some(Found {
                latitude: 51.05,
                longitude: -114.07,
                place: "Lighthouse, Alberta".into(),
                zone: Some("America/Edmonton".into()),
            })
        );
    }

    #[test]
    fn a_name_nowhere_has_is_no_place() {
        assert_eq!(found(&json!({"generationtime_ms": 0.5})), None);
        assert_eq!(found(&json!({"results": []})), None);
        assert_eq!(found(&json!({"results": [{"name": "Cove"}]})), None);
        let said = json!({"results": [{"name": "Berlin", "admin1": "Berlin",
                                       "latitude": 52.5, "longitude": 13.4}]});
        assert_eq!(
            found(&said).map(|place| place.place).as_deref(),
            Some("Berlin")
        );
    }

    #[test]
    fn a_position_is_asked_to_a_hundredth_of_a_degree() {
        assert!(
            place_url(hundredths(43.674_846), hundredths(-79.379_4))
                .contains("lat=43.67&lon=-79.38")
        );
        assert!(zone_url(1.0, 2.5).contains("latitude=1&longitude=2.5&timezone=auto"));
    }
}
