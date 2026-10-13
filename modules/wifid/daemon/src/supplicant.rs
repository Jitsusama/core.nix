//! How wifid asks wpa_supplicant to join a network.

use std::collections::HashMap;

use zbus::proxy;
use zbus::zvariant::{self, ObjectPath, OwnedObjectPath, OwnedValue};

use crate::address::AddressKey;
use crate::network::{Address, Inner, Method, Network, Security};

/// A value for one of a network's fields, in the type wpa_supplicant's
/// AddNetwork reads it as.
#[derive(Debug, PartialEq)]
pub enum Value {
    Bytes(Vec<u8>),
    Text(String),
    Number(i32),
}

/// Why a saved network can't be handed to wpa_supplicant.
#[derive(Debug, PartialEq)]
pub struct Unjoinable(pub &'static str);

/// What went wrong joining a network, as the shell warns of it.
#[derive(Debug, PartialEq)]
pub enum Trouble {
    /// The password didn't work, which on a network joined before means it
    /// has changed.
    WrongPassword,
    /// The access point turned the machine away, with the 802.11 status
    /// code it gave.
    Refused(i32),
}

/// The certificate authorities the machine trusts, which prove an
/// enterprise network's server unless it names its own.
const AUTHORITIES: &str = "/etc/ssl/certs/ca-certificates.crt";

/// The 802.11 status an access point answers a wrong WPA3 password with.
const CHALLENGE_FAILED: i32 = 15;

/// The statuses SAE answers with while agreeing how to join: a token to
/// send back, a group to use instead, and the newer ways of deriving the
/// key. wpa_supplicant goes on with the join after each.
const NEGOTIATING: [i32; 4] = [76, 77, 126, 127];

/// Which of a join's steps a status code came from.
#[derive(Clone, Copy, Debug)]
pub enum Step {
    Authentication,
    Association,
}

impl Trouble {
    /// The trouble a status code from wpa_supplicant's AuthStatusCode or
    /// AssocStatusCode means. A wrong WPA2 password comes as a PskMismatch
    /// signal instead, but a wrong WPA3 one only as a failed challenge,
    /// which is how hostapd answers it.
    pub fn from_status(step: Step, code: i32, security: Security) -> Option<Trouble> {
        match (step, code, security) {
            (_, 0, _) => None,
            (Step::Authentication, CHALLENGE_FAILED, Security::Personal) => {
                Some(Trouble::WrongPassword)
            }
            (Step::Authentication, code, _) if NEGOTIATING.contains(&code) => None,
            (_, code, _) => Some(Trouble::Refused(code)),
        }
    }
}

/// The fields wpa_supplicant joins this network with, shown this address.
pub fn properties(
    network: &Network,
    password: Option<&str>,
    address: [u8; 6],
) -> Result<Vec<(&'static str, Value)>, Unjoinable> {
    // Given as bytes, wpa_supplicant takes the name as it is, whatever
    // characters it holds; as text it would be quoted.
    let mut fields = vec![("ssid", Value::Bytes(network.name.as_bytes().to_vec()))];
    match network.security {
        Security::Open => fields.push(("key_mgmt", Value::Text("NONE".to_string()))),
        Security::Personal => {
            // WPA2 and WPA3 take the same password, and wpa_supplicant
            // picks the best the access point offers.
            let key_mgmt = "WPA-PSK WPA-PSK-SHA256 SAE".to_string();
            fields.push(("key_mgmt", Value::Text(key_mgmt)));
            let password = password.ok_or(Unjoinable("the network's password is missing"))?;
            fields.push(("psk", psk(password)?));
        }
        Security::Enterprise => {
            let account = network.account.as_ref();
            let account = account.ok_or(Unjoinable("the network's account is missing"))?;
            let password = password.ok_or(Unjoinable("the network's password is missing"))?;
            let text = |value: &str| Value::Text(value.to_string());
            fields.push(("key_mgmt", text("WPA-EAP WPA-EAP-SHA256")));
            let method = match account.method {
                Method::Peap => "PEAP",
                Method::Ttls => "TTLS",
            };
            fields.push(("eap", text(method)));
            fields.push(("identity", text(&account.identity)));
            if let Some(name) = &account.anonymous_identity {
                fields.push(("anonymous_identity", text(name)));
            }
            fields.push(("password", text(password)));
            // The server proves itself with a certificate from these
            // authorities that names this domain, so an access point
            // only pretending to be the network never sees the password.
            let authorities = account
                .authority
                .as_ref()
                .map(|file| file.to_string_lossy());
            fields.push((
                "ca_cert",
                text(authorities.as_deref().unwrap_or(AUTHORITIES)),
            ));
            fields.push(("domain_suffix_match", text(&account.server_domain)));
            let inner = match account.inner {
                Inner::Mschapv2 => "auth=MSCHAPV2",
                Inner::Pap => "auth=PAP",
            };
            fields.push(("phase2", text(inner)));
        }
    }
    if network.hidden {
        fields.push(("scan_ssid", Value::Number(1)));
    }
    fields.push(("priority", Value::Number(network.priority)));
    match network.address {
        Address::Network => {
            fields.push(("mac_addr", Value::Number(3)));
            let shown = address.map(|byte| format!("{byte:02x}")).join(":");
            fields.push(("mac_value", Value::Text(shown)));
        }
        Address::Card => fields.push(("mac_addr", Value::Number(0))),
        Address::Random => fields.push(("mac_addr", Value::Number(1))),
    }
    Ok(fields)
}

/// A password as wpa_supplicant takes it. Sixty-four hex digits are the
/// key itself, which it reads as bytes; anything else it reads as text
/// and works the key out from.
pub fn psk(password: &str) -> Result<Value, Unjoinable> {
    if password.len() == 64 && password.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        let pairs = password.as_bytes().chunks(2);
        let key = pairs.map(|pair| hex(pair[0]) << 4 | hex(pair[1])).collect();
        Ok(Value::Bytes(key))
    } else if (8..=63).contains(&password.len()) {
        Ok(Value::Text(password.to_string()))
    } else {
        Err(Unjoinable(
            "a password is 8 to 63 characters, or 64 hex digits",
        ))
    }
}

fn hex(digit: u8) -> u8 {
    match digit {
        b'0'..=b'9' => digit - b'0',
        _ => (digit | 0x20) - b'a' + 10,
    }
}

/// A network's fields as AddNetwork takes them over D-Bus.
pub fn dictionary(
    fields: Vec<(&'static str, Value)>,
) -> HashMap<&'static str, zvariant::Value<'static>> {
    let value = |value| match value {
        Value::Bytes(bytes) => zvariant::Value::from(bytes),
        Value::Text(text) => zvariant::Value::from(text),
        Value::Number(number) => zvariant::Value::from(number),
    };
    fields
        .into_iter()
        .map(|(key, field)| (key, value(field)))
        .collect()
}

/// A mark of the fields, and the fields with it added as the network's
/// id_str. It's made under
/// the machine's key, so it says nothing of the password. wifid knows by
/// it, next time it starts, whether a network a card holds is still as
/// wanted.
pub fn marked(
    fields: Vec<(&'static str, Value)>,
    key: &AddressKey,
) -> (String, Vec<(&'static str, Value)>) {
    let mut given = Vec::new();
    for (name, value) in &fields {
        let (kind, bytes) = match value {
            Value::Bytes(bytes) => (b'b', bytes.clone()),
            Value::Text(text) => (b't', text.as_bytes().to_vec()),
            Value::Number(number) => (b'n', number.to_string().into_bytes()),
        };
        // Each part led by its length, so no two lists of fields read alike.
        for part in [name.as_bytes(), &[kind], &bytes] {
            given.extend_from_slice(&(part.len() as u32).to_be_bytes());
            given.extend_from_slice(part);
        }
    }
    let mark = key.mark(&given);
    let mut fields = fields;
    fields.push(("id_str", Value::Text(mark.clone())));
    (mark, fields)
}

/// A text field as wpa_supplicant gives it back, quoted.
pub fn unquoted(field: &str) -> Option<&str> {
    field.strip_prefix('"')?.strip_suffix('"')
}

/// What to change so a card holds just the networks of these marks, from
/// the marks of those it holds: the ones to take away, and which of the
/// wanted to add. Anything it holds as wanted stays, and with it the
/// connection.
pub fn changes<'a, P>(
    held: &'a [(P, Option<String>)],
    wanted: &[String],
) -> (Vec<&'a P>, Vec<usize>) {
    let mut kept: Vec<&str> = Vec::new();
    let mut gone = Vec::new();
    for (network, mark) in held {
        match mark.as_deref() {
            Some(mark) if wanted.iter().any(|w| w == mark) && !kept.contains(&mark) => {
                kept.push(mark)
            }
            _ => gone.push(network),
        }
    }
    let new = (0..wanted.len())
        .filter(|&i| !kept.contains(&wanted[i].as_str()))
        .collect();
    (gone, new)
}

/// wpa_supplicant itself, which adds and takes away cards.
#[proxy(
    interface = "fi.w1.wpa_supplicant1",
    default_service = "fi.w1.wpa_supplicant1",
    default_path = "/fi/w1/wpa_supplicant1"
)]
pub trait Supplicant {
    fn create_interface(
        &self,
        args: HashMap<&str, zvariant::Value<'_>>,
    ) -> zbus::Result<OwnedObjectPath>;

    fn get_interface(&self, ifname: &str) -> zbus::Result<OwnedObjectPath>;

    #[zbus(property)]
    fn interfaces(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
}

/// One card, as wpa_supplicant drives it.
#[proxy(
    interface = "fi.w1.wpa_supplicant1.Interface",
    default_service = "fi.w1.wpa_supplicant1"
)]
pub trait Interface {
    fn add_network(
        &self,
        args: HashMap<&str, zvariant::Value<'_>>,
    ) -> zbus::Result<OwnedObjectPath>;

    fn remove_network(&self, path: &ObjectPath<'_>) -> zbus::Result<()>;

    fn scan(&self, args: HashMap<&str, zvariant::Value<'_>>) -> zbus::Result<()>;

    /// Leaves the network it's on, and joins none until asked.
    fn disconnect(&self) -> zbus::Result<()>;

    /// Joins this network now, turning every other the card holds off.
    fn select_network(&self, path: &ObjectPath<'_>) -> zbus::Result<()>;

    #[zbus(property)]
    fn ifname(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn networks(&self) -> zbus::Result<Vec<OwnedObjectPath>>;

    /// The access points it hears.
    #[zbus(property, name = "BSSs")]
    fn access_points(&self) -> zbus::Result<Vec<OwnedObjectPath>>;

    #[zbus(property)]
    fn state(&self) -> zbus::Result<String>;
}

/// A network wpa_supplicant has been given for one card.
#[proxy(
    interface = "fi.w1.wpa_supplicant1.Network",
    default_service = "fi.w1.wpa_supplicant1"
)]
pub trait Joinable {
    #[zbus(property)]
    fn set_enabled(&self, enabled: bool) -> zbus::Result<()>;

    /// Every field but the keys, each as text.
    #[zbus(property)]
    fn properties(&self) -> zbus::Result<HashMap<String, OwnedValue>>;
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::address::AddressKey;
    use crate::network::Account;
    use crate::network::{Address, Security};

    const SHOWN: [u8; 6] = [0x5a, 0x01, 0x02, 0xa0, 0xb0, 0xff];

    fn inn() -> Network {
        Network {
            name: "Roadside Inn".to_string(),
            security: Security::Open,
            secret: None,
            priority: 0,
            hidden: false,
            address: Address::Network,
            trusted: false,
            account: None,
        }
    }

    fn marked_inn(key: &AddressKey, priority: i32) -> (String, Vec<(&'static str, Value)>) {
        let network = Network { priority, ..inn() };
        marked(properties(&network, None, SHOWN).unwrap(), key)
    }

    fn mark_of((mark, fields): (String, Vec<(&'static str, Value)>)) -> String {
        assert_eq!(fields.last(), Some(&("id_str", Value::Text(mark.clone()))));
        mark
    }

    #[test]
    fn a_network_is_marked_with_everything_it_was_given() {
        let key = AddressKey::from([7; 32]);

        let first = marked_inn(&key, 0);
        let again = marked_inn(&key, 0);

        assert_eq!(first, again);
        assert_eq!(
            first.1.len(),
            properties(&inn(), None, SHOWN).unwrap().len() + 1
        );
        let mark = mark_of(first);
        assert_eq!(mark.len(), 64);
        assert!(mark.bytes().all(|byte| byte.is_ascii_hexdigit()), "{mark}");
    }

    #[test]
    fn the_mark_changes_when_anything_the_network_is_given_does() {
        let key = AddressKey::from([7; 32]);
        let lantern = Network {
            name: "Lantern".to_string(),
            security: Security::Personal,
            ..inn()
        };
        let with = |password| marked(properties(&lantern, Some(password), SHOWN).unwrap(), &key);

        let first = mark_of(with("glowing in the dark"));
        let new_password = mark_of(with("dark in the glowing"));
        let inn = mark_of(marked_inn(&key, 0));
        let new_priority = mark_of(marked_inn(&key, 5));
        let other_machine = mark_of(marked_inn(&AddressKey::from([8; 32]), 0));

        assert_ne!(first, new_password);
        assert_ne!(inn, new_priority);
        assert_ne!(inn, other_machine);
    }

    #[test]
    fn the_mark_is_read_back_as_wpa_supplicant_quotes_it() {
        assert_eq!(unquoted("\"0a1b\""), Some("0a1b"));
        assert_eq!(unquoted("0a1b"), None);
        assert_eq!(unquoted("\""), None);
    }

    #[test]
    fn networks_a_card_already_holds_as_wanted_are_left_alone() {
        let held = [("/0", Some("a".to_string())), ("/1", Some("b".to_string()))];
        let wanted = ["b".to_string(), "a".to_string()];

        let (gone, new) = changes(&held, &wanted);

        assert_eq!(gone, Vec::<&&str>::new());
        assert_eq!(new, Vec::<usize>::new());
    }

    #[test]
    fn networks_changed_or_no_longer_wanted_are_taken_away_and_the_rest_added() {
        let held = [
            ("/0", Some("kept".to_string())),
            ("/1", Some("changed".to_string())),
            ("/2", None),
            ("/3", Some("kept".to_string())),
        ];
        let wanted = [
            "kept".to_string(),
            "new".to_string(),
            "changed now".to_string(),
        ];

        let (gone, new) = changes(&held, &wanted);

        assert_eq!(gone, vec![&"/1", &"/2", &"/3"]);
        assert_eq!(new, vec![1, 2]);
    }

    #[test]
    fn an_open_network_is_joined_without_a_key() {
        let network = inn();

        let fields = properties(&network, None, SHOWN).unwrap();

        assert_eq!(
            fields,
            vec![
                ("ssid", Value::Bytes(b"Roadside Inn".to_vec())),
                ("key_mgmt", Value::Text("NONE".to_string())),
                ("priority", Value::Number(0)),
                ("mac_addr", Value::Number(3)),
                ("mac_value", Value::Text("5a:01:02:a0:b0:ff".to_string())),
            ]
        );
    }

    #[test]
    fn a_personal_network_is_joined_by_wpa2_or_wpa3_with_its_password() {
        let mut network = inn();
        network.security = Security::Personal;
        network.priority = 5;

        let fields = properties(&network, Some("glowing in the dark"), SHOWN).unwrap();

        assert_eq!(
            fields[1],
            (
                "key_mgmt",
                Value::Text("WPA-PSK WPA-PSK-SHA256 SAE".to_string())
            )
        );
        assert_eq!(
            fields[2],
            ("psk", Value::Text("glowing in the dark".to_string()))
        );
        assert_eq!(fields[3], ("priority", Value::Number(5)));
    }

    #[test]
    fn a_password_of_64_hex_digits_is_the_key_itself() {
        let mut network = inn();
        network.security = Security::Personal;
        let key = "00112233445566778899aabbccddeeff00112233445566778899AABBCCDDEEFF";

        let fields = properties(&network, Some(key), SHOWN).unwrap();

        let mut bytes: Vec<u8> = (0..16).map(|n| n * 0x11).collect();
        bytes.extend_from_within(..);
        assert_eq!(fields[2], ("psk", Value::Bytes(bytes)));
    }

    #[test]
    fn a_password_wpa_supplicant_would_refuse_is_refused() {
        let mut network = inn();
        network.security = Security::Personal;
        let too_long = "x".repeat(64);

        let refusals = [None, Some("seven 7"), Some(too_long.as_str())]
            .map(|password| properties(&network, password, SHOWN).err());

        let wrong_length = Unjoinable("a password is 8 to 63 characters, or 64 hex digits");
        assert_eq!(
            refusals,
            [
                Some(Unjoinable("the network's password is missing")),
                Some(wrong_length),
                Some(Unjoinable(
                    "a password is 8 to 63 characters, or 64 hex digits"
                )),
            ]
        );
    }

    #[test]
    fn a_challenge_failing_on_a_personal_network_is_a_wrong_password() {
        let trouble = Trouble::from_status(Step::Authentication, 15, Security::Personal);

        assert_eq!(trouble, Some(Trouble::WrongPassword));
    }

    #[test]
    fn what_sae_says_to_agree_on_how_to_join_is_no_trouble() {
        let codes = [0, 76, 77, 126, 127];

        let troubles =
            codes.map(|code| Trouble::from_status(Step::Authentication, code, Security::Personal));

        assert_eq!(troubles, [None, None, None, None, None]);
    }

    #[test]
    fn any_other_status_is_the_network_refusing_the_machine() {
        let open = Trouble::from_status(Step::Authentication, 15, Security::Open);
        let full = Trouble::from_status(Step::Association, 17, Security::Personal);
        let negotiating = Trouble::from_status(Step::Association, 76, Security::Personal);
        let joined = Trouble::from_status(Step::Association, 0, Security::Personal);

        assert_eq!(open, Some(Trouble::Refused(15)));
        assert_eq!(full, Some(Trouble::Refused(17)));
        assert_eq!(negotiating, Some(Trouble::Refused(76)));
        assert_eq!(joined, None);
    }

    fn office(authority: Option<&str>) -> Network {
        Network {
            name: "Office".to_string(),
            security: Security::Enterprise,
            account: Some(Account {
                method: Method::Peap,
                identity: "joel".into(),
                anonymous_identity: Some("anonymous".into()),
                server_domain: "radius.office.test".into(),
                authority: authority.map(PathBuf::from),
                inner: Inner::Mschapv2,
            }),
            ..inn()
        }
    }

    fn field<'a>(fields: &'a [(&'static str, Value)], name: &str) -> Option<&'a Value> {
        fields
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value)
    }

    fn text(value: &str) -> Option<Value> {
        Some(Value::Text(value.to_string()))
    }

    #[test]
    fn an_enterprise_network_is_joined_by_its_account_proving_the_server_by_its_domain() {
        let office = office(Some("/etc/office/authority.pem"));

        let fields = properties(&office, Some("hunter22"), [2, 0, 0, 0, 0, 1]).unwrap();

        for (name, value) in [
            ("key_mgmt", "WPA-EAP WPA-EAP-SHA256"),
            ("eap", "PEAP"),
            ("identity", "joel"),
            ("anonymous_identity", "anonymous"),
            ("password", "hunter22"),
            ("ca_cert", "/etc/office/authority.pem"),
            ("domain_suffix_match", "radius.office.test"),
            ("phase2", "auth=MSCHAPV2"),
        ] {
            assert_eq!(field(&fields, name), text(value).as_ref(), "{name}");
        }
    }

    #[test]
    fn an_enterprise_network_s_server_is_proved_by_the_machine_s_authorities_by_default() {
        let fields = properties(&office(None), Some("hunter22"), [2, 0, 0, 0, 0, 1]).unwrap();

        assert_eq!(
            field(&fields, "ca_cert"),
            text("/etc/ssl/certs/ca-certificates.crt").as_ref()
        );
    }

    #[test]
    fn an_enterprise_network_without_its_account_or_password_is_unjoinable() {
        let no_account = Network {
            account: None,
            ..office(None)
        };

        assert!(properties(&no_account, Some("hunter22"), [2, 0, 0, 0, 0, 1]).is_err());
        assert!(properties(&office(None), None, [2, 0, 0, 0, 0, 1]).is_err());
    }

    #[test]
    fn a_hidden_network_is_looked_for_by_name() {
        let mut network = inn();
        network.hidden = true;

        let fields = properties(&network, None, SHOWN).unwrap();

        assert!(fields.contains(&("scan_ssid", Value::Number(1))));
    }

    #[test]
    fn a_network_can_see_the_card_s_own_address_or_a_new_one_each_time() {
        let mut card = inn();
        card.address = Address::Card;
        let mut random = inn();
        random.address = Address::Random;

        let shown_the_card = properties(&card, None, SHOWN).unwrap();
        let shown_another = properties(&random, None, SHOWN).unwrap();

        assert_eq!(shown_the_card.last(), Some(&("mac_addr", Value::Number(0))));
        assert_eq!(shown_another.last(), Some(&("mac_addr", Value::Number(1))));
    }
}
