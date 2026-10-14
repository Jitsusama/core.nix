//! The access points wpa_supplicant hears, each as it last described it.
//! wpa_supplicant says everything of one when it first hears it and only
//! what changed after that, so each is kept whole here.

use std::collections::HashMap;

use zbus::zvariant::{OwnedValue, Value};

use crate::radio::Radio;
use crate::state::Security;

/// One access point, with everything wpa_supplicant has said of it.
#[derive(Debug, Default)]
pub struct AccessPoint {
    properties: HashMap<String, OwnedValue>,
}

/// What the shell is told of an access point it could join.
#[derive(Clone, Debug, PartialEq)]
pub struct Heard {
    pub name: String,
    pub security: Security,
    /// How strongly it's heard, in dBm.
    pub signal: i16,
    /// Its own address, as 02:00:00:00:01:00.
    pub bssid: String,
    /// Its channel, by frequency in MHz; 0 where wpa_supplicant doesn't say.
    pub frequency: u32,
    /// How many seconds ago it was last heard.
    pub age: u32,
    pub radio: Radio,
}

impl AccessPoint {
    pub fn new(properties: HashMap<String, OwnedValue>) -> AccessPoint {
        AccessPoint { properties }
    }

    /// Takes in what wpa_supplicant says changed.
    pub fn changed(&mut self, changed: HashMap<String, OwnedValue>) {
        self.properties.extend(changed);
    }

    /// What the shell is told of it, unless it hides its name or is joined
    /// in a way wifid doesn't know.
    pub fn heard(&self) -> Option<Heard> {
        let name = String::from_utf8(self.bytes("SSID")?).ok()?;
        if name.is_empty() {
            return None;
        }
        let Value::I16(signal) = *self.value("Signal")? else {
            return None;
        };
        let Value::Bool(privacy) = *self.value("Privacy")? else {
            return None;
        };
        let mut ways = self.key_management("RSN")?;
        ways.extend(self.key_management("WPA")?);
        let any = |known: &[&str]| ways.iter().any(|way| known.contains(&way.as_str()));
        // A password is asked for more simply than an account, so a network
        // taking either is joined by password.
        let security = if any(PASSWORD) {
            Security::Personal
        } else if any(ACCOUNT) {
            Security::Enterprise
        } else if ways.is_empty() && !privacy {
            Security::Open
        } else {
            return None;
        };
        let frequency = self.frequency().unwrap_or_default();
        let bssid = self.bytes("BSSID").unwrap_or_default();
        let bssid = bssid.iter().map(|byte| format!("{byte:02x}"));
        let age = match self.value("Age") {
            Some(Value::U32(age)) => *age,
            _ => 0,
        };
        Some(Heard {
            name,
            security,
            signal,
            bssid: bssid.collect::<Vec<_>>().join(":"),
            frequency,
            age,
            radio: Radio::read(&self.bytes("IEs").unwrap_or_default(), frequency),
        })
    }

    /// The channel it's heard on, by frequency in MHz.
    pub fn frequency(&self) -> Option<u32> {
        match self.value("Frequency")? {
            Value::U16(frequency) => Some(u32::from(*frequency)),
            _ => None,
        }
    }

    fn value(&self, property: &str) -> Option<&Value<'static>> {
        self.properties.get(property).map(|value| &**value)
    }

    fn bytes(&self, property: &str) -> Option<Vec<u8>> {
        let Value::Array(bytes) = self.value(property)? else {
            return None;
        };
        let byte = |value: &Value| match value {
            Value::U8(byte) => Some(*byte),
            _ => None,
        };
        bytes.iter().map(byte).collect()
    }

    /// The ways of joining an RSN or WPA element offers; none when the
    /// access point sends no such element.
    fn key_management(&self, element: &str) -> Option<Vec<String>> {
        let Value::Dict(element) = self.value(element)? else {
            return None;
        };
        let ways = element.iter().find_map(|(key, value)| match key {
            Value::Str(key) if key.as_str() == "KeyMgmt" => Some(value),
            _ => None,
        });
        let Some(ways) = ways else {
            return Some(Vec::new());
        };
        let ways = match ways {
            Value::Value(ways) => ways,
            ways => ways,
        };
        let Value::Array(ways) = ways else {
            return None;
        };
        let way = |value: &Value| match value {
            Value::Str(way) => Some(way.to_string()),
            _ => None,
        };
        ways.iter().map(way).collect()
    }
}

/// What wpa_supplicant calls the ways of joining with a password, WPA2's and
/// WPA3's.
const PASSWORD: &[&str] = &[
    "wpa-psk",
    "wpa-ft-psk",
    "wpa-psk-sha256",
    "sae",
    "sae-ext-key",
    "ft-sae",
    "ft-sae-ext-key",
];

/// What it calls the ways of joining with an account.
const ACCOUNT: &[&str] = &[
    "wpa-eap",
    "wpa-ft-eap",
    "wpa-eap-sha256",
    "wpa-eap-sha384",
    "wpa-eap-suite-b",
    "wpa-eap-suite-b-192",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn owned(value: Value<'_>) -> OwnedValue {
        value.try_to_owned().unwrap()
    }

    fn key_management(ways: &[&str]) -> OwnedValue {
        let dictionary = HashMap::from([("KeyMgmt", Value::from(ways.to_vec()))]);
        owned(Value::from(dictionary))
    }

    fn access_point(ssid: &[u8], rsn: &[&str], privacy: bool, signal: i16) -> AccessPoint {
        AccessPoint::new(HashMap::from([
            ("SSID".into(), owned(Value::from(ssid.to_vec()))),
            ("WPA".into(), key_management(&[])),
            ("RSN".into(), key_management(rsn)),
            ("Privacy".into(), owned(Value::from(privacy))),
            ("Signal".into(), owned(Value::from(signal))),
        ]))
    }

    #[test]
    fn a_network_with_a_password_is_heard_by_its_name_and_signal() {
        let lantern = access_point(b"Lantern", &["wpa-psk", "sae"], true, -58);

        let heard = lantern.heard();

        let heard = heard.unwrap();
        assert_eq!(
            (heard.name.as_str(), heard.security, heard.signal),
            ("Lantern", Security::Personal, -58)
        );
    }

    #[test]
    fn an_access_point_is_heard_with_its_address_age_and_radio() {
        let mut lantern = access_point(b"Lantern", &["wpa-psk"], true, -58);
        // A BSS Load element: 3 on it, its channel busy half the time.
        let said = HashMap::from([
            ("BSSID".into(), owned(Value::from(vec![2u8, 0, 0, 0, 1, 0]))),
            ("Frequency".into(), owned(Value::from(2437u16))),
            ("Age".into(), owned(Value::from(4u32))),
            (
                "IEs".into(),
                owned(Value::from(vec![11u8, 5, 3, 0, 128, 0, 0])),
            ),
        ]);

        lantern.changed(said);
        let heard = lantern.heard().unwrap();

        assert_eq!(heard.bssid, "02:00:00:00:01:00");
        assert_eq!((heard.frequency, heard.age), (2437, 4));
        let load = heard.radio.load.unwrap();
        assert_eq!((load.stations, load.busy), (3, 50));
    }

    #[test]
    fn a_network_asking_wpa3_alone_still_takes_a_password() {
        let lantern = access_point(b"Lantern", &["sae"], true, -58);

        let heard = lantern.heard().unwrap();

        assert_eq!(heard.security, Security::Personal);
    }

    #[test]
    fn a_network_asking_nothing_is_open() {
        let inn = access_point(b"Roadside Inn", &[], false, -70);

        let heard = inn.heard().unwrap();

        assert_eq!(heard.security, Security::Open);
    }

    #[test]
    fn a_network_asking_an_account_is_enterprise() {
        let office = access_point(b"Office", &["wpa-eap", "wpa-eap-sha256"], true, -61);

        let heard = office.heard().unwrap();

        assert_eq!(heard.security, Security::Enterprise);
    }

    #[test]
    fn a_network_taking_a_password_or_an_account_is_joined_by_password() {
        let both = access_point(b"Both", &["wpa-eap", "wpa-psk"], true, -61);

        let heard = both.heard().unwrap();

        assert_eq!(heard.security, Security::Personal);
    }

    #[test]
    fn an_access_point_is_heard_on_a_channel() {
        let mut lantern = access_point(b"Lantern", &["wpa-psk"], true, -58);
        let on = HashMap::from([("Frequency".into(), owned(Value::from(5180u16)))]);

        lantern.changed(on);

        assert_eq!(lantern.frequency(), Some(5180));
    }

    #[test]
    fn an_access_point_described_without_its_channel_is_heard_on_none() {
        let lantern = access_point(b"Lantern", &["wpa-psk"], true, -58);

        assert_eq!(lantern.frequency(), None);
    }

    #[test]
    fn signal_changing_is_taken_in_and_the_rest_kept() {
        let mut lantern = access_point(b"Lantern", &["wpa-psk"], true, -58);

        lantern.changed(HashMap::from([(
            "Signal".into(),
            owned(Value::from(-77i16)),
        )]));
        let heard = lantern.heard().unwrap();

        assert_eq!(heard.signal, -77);
        assert_eq!(heard.name, "Lantern");
    }

    #[test]
    fn an_access_point_hiding_its_name_is_not_offered() {
        let hidden = access_point(b"", &["wpa-psk"], true, -58);

        assert_eq!(hidden.heard(), None);
    }

    #[test]
    fn a_name_that_isn_t_text_is_not_offered() {
        let garbled = access_point(&[0xff, 0xfe, 0x41], &["wpa-psk"], true, -58);

        assert_eq!(garbled.heard(), None);
    }

    #[test]
    fn wep_is_not_offered() {
        let wep = access_point(b"Old Router", &[], true, -58);

        assert_eq!(wep.heard(), None);
    }

    #[test]
    fn a_network_asking_only_opportunistic_encryption_is_not_offered() {
        let owe = access_point(b"Cafe", &["owe"], true, -58);

        assert_eq!(owe.heard(), None);
    }

    #[test]
    fn an_access_point_described_in_part_is_not_offered() {
        let partial = AccessPoint::new(HashMap::from([(
            "SSID".into(),
            owned(Value::from(b"Lantern".to_vec())),
        )]));

        assert_eq!(partial.heard(), None);
    }
}
