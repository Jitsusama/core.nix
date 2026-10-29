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
    fn a_position_is_asked_to_a_hundredth_of_a_degree() {
        assert!(
            place_url(hundredths(43.674_846), hundredths(-79.379_4))
                .contains("lat=43.67&lon=-79.38")
        );
        assert!(zone_url(1.0, 2.5).contains("latitude=1&longitude=2.5&timezone=auto"));
    }
}
