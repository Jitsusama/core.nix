//! What eero's cloud says, read into what the machine needs to know.
//!
//! eero publishes no API. Its app talks JSON to api-user.e2ro.com, under
//! /2.2, and these shapes are the ones the eero app's traffic shows and that
//! Home Assistant's eero integration has read for years. Every answer comes
//! wrapped as `{"meta": {"code": .., "error": ..}, "data": ..}`. A field
//! missing from an answer is read as unknown rather than refused, so a field
//! eero drops costs one line of the status, not all of it.

use serde_json::Value;

/// Where eero's cloud answers.
pub const ORIGIN: &str = "https://api-user.e2ro.com";

/// The account's own record, which lists its networks.
pub const ACCOUNT: &str = "/2.2/account";

/// A network, as the status shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct Network {
    pub name: String,
    /// The path its own record is at, such as /2.2/networks/1234.
    pub url: String,
    /// Whether the line to the internet is up, as the eero sees it.
    pub online: Option<bool>,
    /// The last speed test the eero ran, in Mbit/s, and when.
    pub down_mbps: Option<f64>,
    pub up_mbps: Option<f64>,
    pub tested: Option<String>,
    /// "Optimize for conferencing and gaming", which the app calls SQM.
    pub sqm: Option<bool>,
    /// Where its connected devices are listed.
    pub devices_url: Option<String>,
    /// Where its settings are changed.
    pub settings_url: Option<String>,
}

/// A device on a network.
#[derive(Clone, Debug, PartialEq)]
pub struct Device {
    pub name: String,
    pub ip: Option<String>,
    pub wired: bool,
}

/// The data of an answer, or why there's none.
pub fn data(body: &Value) -> Result<&Value, String> {
    match body.get("data") {
        Some(data) if !data.is_null() => Ok(data),
        _ => Err(format!(
            "eero answered {} without data",
            body.get("meta")
                .map_or_else(|| "nothing".into(), Value::to_string)
        )),
    }
}

/// Whether an answer says the session has to be refreshed before asking
/// again, as eero says once a session token has aged.
pub fn session_expired(status: u16, body: &Value) -> bool {
    let error = body.pointer("/meta/error").and_then(Value::as_str);
    status == 401
        && matches!(
            error,
            Some("error.session.invalid" | "error.session.refresh")
        )
}

/// The paths of the account's networks, from its record's data.
pub fn networks(account: &Value) -> Vec<String> {
    account
        .pointer("/networks/data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|network| network.get("url").and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}

/// A network, from its record's data.
pub fn network(url: &str, data: &Value) -> Network {
    let text = |pointer: &str| {
        data.pointer(pointer)
            .and_then(Value::as_str)
            .map(str::to_owned)
    };
    let online = data
        .pointer("/health/internet/isp_up")
        .and_then(Value::as_bool)
        .or_else(|| text("/status").map(|status| status == "connected"));
    Network {
        name: text("/name").unwrap_or_else(|| "eero".into()),
        url: url.to_owned(),
        online,
        down_mbps: speed(data.pointer("/speed/down")),
        up_mbps: speed(data.pointer("/speed/up")),
        tested: text("/speed/date"),
        sqm: data.get("sqm").and_then(Value::as_bool),
        devices_url: text("/resources/devices"),
        settings_url: text("/resources/settings"),
    }
}

/// What a network's settings are sent to turn "Optimize for conferencing
/// and gaming" on or off: a bare boolean, since eero ignores it nested.
pub fn conferencing(on: bool) -> Value {
    serde_json::json!({ "sqm": on })
}

/// Whether the network, read again, says the setting was taken. eero answers
/// a change it ignores as gladly as one it takes, so only reading it back
/// tells.
pub fn taken(asked: bool, read: Option<bool>) -> Result<(), String> {
    let word = |on: bool| if on { "on" } else { "off" };
    match read {
        Some(read) if read == asked => Ok(()),
        Some(read) => Err(format!(
            "eero took the change but still says conferencing {}",
            word(read)
        )),
        None => Err("eero no longer says whether conferencing is on".into()),
    }
}

/// A speed eero gives as a value and its units, in Mbit/s.
fn speed(reading: Option<&Value>) -> Option<f64> {
    let reading = reading?;
    let value = reading.get("value")?.as_f64()?;
    let per_mbit = match reading.get("units").and_then(Value::as_str) {
        Some("Kbps") => 0.001,
        Some("Gbps") => 1000.0,
        _ => 1.0,
    };
    Some(value * per_mbit)
}

/// The devices connected now, from a network's devices list.
pub fn devices(list: &Value) -> Vec<Device> {
    list.as_array()
        .into_iter()
        .flatten()
        .filter(|device| device.get("connected").and_then(Value::as_bool) == Some(true))
        .map(|device| {
            let text = |key: &str| {
                device
                    .get(key)
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
            };
            Device {
                name: text("nickname")
                    .or_else(|| text("hostname"))
                    .or_else(|| text("mac"))
                    .unwrap_or_else(|| "unknown".into()),
                ip: text("ip"),
                wired: device.get("wireless").and_then(Value::as_bool) == Some(false),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_network_reads_its_line_speed_and_setting() {
        let data = json!({
            "name": "home",
            "status": "connected",
            "health": {"internet": {"isp_up": true, "status": "connected"}},
            "speed": {
                "down": {"value": 76.8, "units": "Mbps"},
                "up": {"value": 12_400, "units": "Kbps"},
                "date": "2026-10-25T18:00:00.000Z"
            },
            "sqm": false,
            "resources": {
                "devices": "/2.2/networks/1234/devices",
                "settings": "/2.2/networks/1234/settings"
            }
        });
        let network = network("/2.2/networks/1234", &data);
        assert_eq!(network.name, "home");
        assert_eq!(network.online, Some(true));
        assert_eq!(network.down_mbps, Some(76.8));
        assert!((network.up_mbps.unwrap() - 12.4).abs() < 1e-9);
        assert_eq!(network.tested.as_deref(), Some("2026-10-25T18:00:00.000Z"));
        assert_eq!(network.sqm, Some(false));
        assert_eq!(
            network.devices_url.as_deref(),
            Some("/2.2/networks/1234/devices")
        );
        assert_eq!(
            network.settings_url.as_deref(),
            Some("/2.2/networks/1234/settings")
        );
    }

    #[test]
    fn a_network_missing_fields_reads_as_unknown() {
        let network = network("/2.2/networks/1", &json!({}));
        assert_eq!(network.name, "eero");
        assert_eq!(network.online, None);
        assert_eq!(network.down_mbps, None);
        assert_eq!(network.sqm, None);
    }

    #[test]
    fn conferencing_is_set_as_a_bare_boolean() {
        assert_eq!(conferencing(true), json!({"sqm": true}));
        assert_eq!(conferencing(false), json!({"sqm": false}));
    }

    #[test]
    fn a_setting_counts_as_taken_only_once_eero_reads_it_back() {
        assert!(taken(true, Some(true)).is_ok());
        assert_eq!(
            taken(true, Some(false)).unwrap_err(),
            "eero took the change but still says conferencing off"
        );
        assert!(taken(false, None).is_err());
    }

    #[test]
    fn without_health_the_status_says_whether_it_is_online() {
        let network = network("/2.2/networks/1", &json!({"status": "disconnected"}));
        assert_eq!(network.online, Some(false));
    }

    #[test]
    fn an_account_lists_its_networks_paths() {
        let account = json!({"networks": {"count": 2, "data": [
            {"url": "/2.2/networks/1", "name": "home"},
            {"url": "/2.2/networks/2", "name": "cottage"}
        ]}});
        assert_eq!(networks(&account), ["/2.2/networks/1", "/2.2/networks/2"]);
        assert!(networks(&json!({})).is_empty());
    }

    #[test]
    fn only_connected_devices_are_listed_by_their_best_name() {
        let list = json!([
            {"connected": true, "nickname": "laptop", "hostname": "nixos", "ip": "192.168.4.20", "wireless": true},
            {"connected": true, "nickname": null, "hostname": "dock", "ip": "192.168.4.21", "wireless": false},
            {"connected": true, "nickname": "", "hostname": null, "mac": "6c:ae:f6:00:00:01"},
            {"connected": false, "nickname": "old phone"}
        ]);
        let names: Vec<_> = devices(&list)
            .into_iter()
            .map(|d| (d.name, d.wired))
            .collect();
        assert_eq!(
            names,
            [
                ("laptop".to_owned(), false),
                ("dock".to_owned(), true),
                ("6c:ae:f6:00:00:01".to_owned(), false)
            ]
        );
    }

    #[test]
    fn an_aged_session_is_told_from_other_refusals() {
        let aged = json!({"meta": {"code": 401, "error": "error.session.refresh"}});
        let invalid = json!({"meta": {"code": 401, "error": "error.session.invalid"}});
        let other = json!({"meta": {"code": 401, "error": "error.auth.forbidden"}});
        assert!(session_expired(401, &aged));
        assert!(session_expired(401, &invalid));
        assert!(!session_expired(401, &other));
        assert!(!session_expired(500, &aged));
    }

    #[test]
    fn an_answer_without_data_says_what_eero_said() {
        let refused =
            json!({"meta": {"code": 404, "error": "error.network.missing"}, "data": null});
        assert!(
            data(&refused)
                .unwrap_err()
                .contains("error.network.missing")
        );
        assert_eq!(data(&json!({"data": {"a": 1}})).unwrap(), &json!({"a": 1}));
    }
}
