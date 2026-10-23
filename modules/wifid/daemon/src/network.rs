//! A saved network: one networkd .network file that names the network in
//! `[Match] SSID=` and says how to join it in an `[X-WiFi]` section, which
//! networkd skips, as it skips every section whose name starts with X-.
//!
//! ```ini
//! [Match]
//! SSID="Roadside Inn"
//!
//! [X-WiFi]
//! Security=personal
//! Secret=/var/lib/wifid/networks/roadside-inn.sealed
//! ```
//!
//! | Key        | Values                              | Default |
//! | ---------- | ----------------------------------- | ------- |
//! | `Security` | open or personal                    | open    |
//! | `Secret`   | the file holding the password, a    | none    |
//! |            | .cred file of it sealed by systemd, |         |
//! |            | or a .sealed one the launcher kept  |         |
//! | `Priority` | a number, highest joined first      | 0       |
//! | `Hidden`   | yes when it doesn't announce itself | no      |
//! | `Address`  | network, card or random             | network |
//!
//! A network asking for an account says whose, and which server it proves
//! itself to, since a look-alike access point would gladly take the
//! password otherwise:
//!
//! | Key                 | Values                               | Default |
//! | ------------------- | ------------------------------------ | ------- |
//! | `Security`          | enterprise                           |         |
//! | `Method`            | peap or ttls                         |         |
//! | `Identity`          | the account's name                   |         |
//! | `AnonymousIdentity` | the name said outside the tunnel     | none    |
//! | `ServerDomain`      | the domain the server's certificate  |         |
//! |                     | must name                            |         |
//! | `Authority`         | the authorities' certificates file   | the     |
//! |                     |                                      | system's|
//! | `Inner`             | mschapv2 or pap                      | mschapv2|
//! |                     |                                      | for peap|
//! |                     |                                      | pap for |
//! |                     |                                      | ttls    |
//!
//! A network is trusted when its own file turns on `MulticastDNS`.

use std::path::PathBuf;

use crate::syntax::Document;

#[derive(Clone, Debug, PartialEq)]
pub struct Network {
    pub name: String,
    pub security: Security,
    pub secret: Option<PathBuf>,
    pub priority: i32,
    pub hidden: bool,
    pub address: Address,
    pub trusted: bool,
    /// Whose account an enterprise network is joined with, and how.
    pub account: Option<Account>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Security {
    Open,
    /// WPA2 or WPA3 with a password: wpa_supplicant takes whichever the
    /// access point offers.
    Personal,
    /// WPA2 or WPA3 with an account, the password sent inside TLS to a
    /// server that proves it's the network's.
    Enterprise,
}

/// An account an enterprise network is joined with.
#[derive(Clone, Debug, PartialEq)]
pub struct Account {
    pub method: Method,
    pub identity: String,
    pub anonymous_identity: Option<String>,
    /// The domain the server's certificate must name.
    pub server_domain: String,
    /// The authorities the server's certificate must come from; the
    /// machine's when none.
    pub authority: Option<PathBuf>,
    pub inner: Inner,
}

/// The tunnel the password goes through.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Method {
    Peap,
    Ttls,
}

/// How the password is sent inside the tunnel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Inner {
    Mschapv2,
    Pap,
}

/// The MAC address the card shows the network.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Address {
    /// One of its own, the same every time, so a portal knows the machine
    /// again and no two networks can tell it's the same one.
    Network,
    /// The card's own.
    Card,
    /// A new one each time it joins.
    Random,
}

/// Why a file that says it's a saved network can't be joined.
#[derive(Debug, PartialEq)]
pub struct Unreadable(pub String);

const SECTION: &str = "X-WiFi";

impl Network {
    /// The saved network a .network file describes, or none for a file that
    /// only gives networkd settings, such as one for every network whose
    /// name starts with Hilton.
    pub fn read(file: &Document) -> Result<Option<Network>, Unreadable> {
        if !file.has(SECTION) {
            return Ok(None);
        }
        let wifi = |key| file.value(SECTION, key);
        let name = file
            .value("Match", "SSID")
            .ok_or_else(|| Unreadable("it names no network in [Match] SSID=".to_string()))?;
        let security = wifi("Security").map_or(Ok(Security::Open), Security::parse)?;
        let account = match security {
            Security::Enterprise => Some(Account::read(file)?),
            _ => None,
        };
        Ok(Some(Network {
            name: unquoted(name)?,
            security,
            secret: wifi("Secret").map(PathBuf::from),
            priority: wifi("Priority").map_or(Ok(0), |p| {
                p.parse()
                    .map_err(|_| Unreadable(format!("Priority={p} isn't a number")))
            })?,
            hidden: wifi("Hidden").map_or(Ok(false), boolean)?,
            address: wifi("Address").map_or(Ok(Address::Network), Address::parse)?,
            // networkd also takes MulticastDNS=resolve, which asks but
            // never answers, so only a yes is trust.
            trusted: file
                .value("Network", "MulticastDNS")
                .is_some_and(|value| boolean(value) == Ok(true)),
            account,
        }))
    }
}

/// The keys an account is kept under.
const ACCOUNT: [&str; 6] = [
    "Method",
    "Identity",
    "AnonymousIdentity",
    "ServerDomain",
    "Authority",
    "Inner",
];

impl Account {
    fn read(file: &Document) -> Result<Account, Unreadable> {
        let wifi = |key| file.value(SECTION, key);
        let needed = |key| {
            wifi(key).ok_or_else(|| Unreadable(format!("an enterprise network needs {key}=")))
        };
        let method = match needed("Method")? {
            "peap" => Method::Peap,
            "ttls" => Method::Ttls,
            other => {
                return Err(Unreadable(format!(
                    "Method={other} is neither peap nor ttls"
                )));
            }
        };
        let inner = match wifi("Inner") {
            None if method == Method::Peap => Inner::Mschapv2,
            Some("mschapv2") => Inner::Mschapv2,
            None | Some("pap") => Inner::Pap,
            Some(other) => {
                return Err(Unreadable(format!(
                    "Inner={other} is neither mschapv2 nor pap"
                )));
            }
        };
        Ok(Account {
            method,
            identity: needed("Identity")?.to_string(),
            anonymous_identity: wifi("AnonymousIdentity").map(str::to_string),
            server_domain: needed("ServerDomain")?.to_string(),
            authority: wifi("Authority").map(PathBuf::from),
            inner,
        })
    }

    fn write_into(&self, file: &mut Document) {
        let method = match self.method {
            Method::Peap => "peap",
            Method::Ttls => "ttls",
        };
        let inner = match self.inner {
            Inner::Mschapv2 => "mschapv2",
            Inner::Pap => "pap",
        };
        file.set(SECTION, "Method", method);
        file.set(SECTION, "Identity", &self.identity);
        match &self.anonymous_identity {
            Some(name) => file.set(SECTION, "AnonymousIdentity", name),
            None => file.remove(SECTION, "AnonymousIdentity"),
        }
        file.set(SECTION, "ServerDomain", &self.server_domain);
        match &self.authority {
            Some(authority) => file.set(SECTION, "Authority", &authority.to_string_lossy()),
            None => file.remove(SECTION, "Authority"),
        }
        file.set(SECTION, "Inner", inner);
    }
}

impl Network {
    /// Writes the network into its file, leaving every other setting in it
    /// as it was.
    pub fn write_into(&self, file: &mut Document) {
        file.set("Match", "SSID", &quoted(&self.name));
        file.set(SECTION, "Security", self.security.name());
        match &self.secret {
            Some(secret) => file.set(SECTION, "Secret", &secret.to_string_lossy()),
            None => file.remove(SECTION, "Secret"),
        }
        file.set(SECTION, "Priority", &self.priority.to_string());
        file.set(SECTION, "Hidden", if self.hidden { "yes" } else { "no" });
        file.set(SECTION, "Address", self.address.name());
        match &self.account {
            Some(account) => account.write_into(file),
            None => {
                for key in ACCOUNT {
                    file.remove(SECTION, key);
                }
            }
        }
        // The defaults leave multicast DNS off on every network, so trust
        // is the network's own file turning it on.
        if self.trusted {
            file.set("Network", "MulticastDNS", "yes");
        } else {
            file.remove("Network", "MulticastDNS");
        }
    }
}

/// What networkd's patterns give a meaning to, and so a backslash escapes.
const PATTERN: [char; 4] = ['\\', '*', '?', '['];

/// A network's name as SSID= matches only it. networkd splits the value
/// into words at spaces outside quotes, either kind, and keeps backslashes
/// for the pattern match, so a backslash can't keep a quote from ending
/// one: the name goes in double quotes, and any double quote in it in
/// single quotes of its own.
fn quoted(name: &str) -> String {
    let mut value = String::from('"');
    for c in name.chars() {
        match c {
            '"' => value.push_str(r#""'"'""#),
            _ if PATTERN.contains(&c) => value.extend(['\\', c]),
            _ => value.push(c),
        }
    }
    value.push('"');
    value
}

/// The name SSID= matches, read as networkd reads it, or why it doesn't
/// name just one network.
fn unquoted(value: &str) -> Result<String, Unreadable> {
    let refused = |why| Err(Unreadable(format!("SSID={value} {why}")));
    // networkd reads a leading ! as every network but the ones named.
    if value.starts_with('!') {
        return refused("is a pattern, not one network's name");
    }
    let mut name = String::new();
    let mut quote = None;
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        match (c, quote) {
            ('"' | '\'', None) => quote = Some(c),
            (_, Some(open)) if c == open => quote = None,
            ('\\', _) => name.extend(chars.next()),
            (' ' | '\t', None) => return refused("names more than one network"),
            _ if PATTERN.contains(&c) => return refused("is a pattern, not one network's name"),
            _ => name.push(c),
        }
    }
    Ok(name)
}

impl Security {
    fn name(self) -> &'static str {
        match self {
            Security::Open => "open",
            Security::Personal => "personal",
            Security::Enterprise => "enterprise",
        }
    }

    fn parse(value: &str) -> Result<Security, Unreadable> {
        match value {
            "open" => Ok(Security::Open),
            "personal" => Ok(Security::Personal),
            "enterprise" => Ok(Security::Enterprise),
            _ => Err(Unreadable(format!(
                "Security={value} is none of open, personal or enterprise"
            ))),
        }
    }
}

impl Address {
    fn name(self) -> &'static str {
        match self {
            Address::Network => "network",
            Address::Card => "card",
            Address::Random => "random",
        }
    }

    fn parse(value: &str) -> Result<Address, Unreadable> {
        match value {
            "network" => Ok(Address::Network),
            "card" => Ok(Address::Card),
            "random" => Ok(Address::Random),
            _ => Err(Unreadable(format!(
                "Address={value} is none of network, card or random"
            ))),
        }
    }
}

/// A yes or no, spelled any of the ways systemd takes one.
fn boolean(value: &str) -> Result<bool, Unreadable> {
    match value {
        "1" | "yes" | "y" | "true" | "t" | "on" => Ok(true),
        "0" | "no" | "n" | "false" | "f" | "off" => Ok(false),
        _ => Err(Unreadable(format!("{value} is neither yes nor no"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> Result<Option<Network>, Unreadable> {
        Network::read(&Document::parse(text).unwrap())
    }

    #[test]
    fn a_saved_network_says_how_to_join_it() {
        let network = read(concat!(
            "[Match]\nSSID=Lantern\n\n",
            "[X-WiFi]\nSecurity=personal\nSecret=/run/keys/lantern\n",
            "Priority=10\nHidden=yes\nAddress=card\n",
        ));

        assert_eq!(
            network,
            Ok(Some(Network {
                name: "Lantern".to_string(),
                security: Security::Personal,
                secret: Some(PathBuf::from("/run/keys/lantern")),
                priority: 10,
                hidden: true,
                address: Address::Card,
                trusted: false,
                account: None,
            }))
        );
    }

    #[test]
    fn what_a_saved_network_leaves_out_takes_its_default() {
        let network = read("[Match]\nSSID=Lantern\n\n[X-WiFi]\n")
            .unwrap()
            .unwrap();

        assert_eq!(network.security, Security::Open);
        assert_eq!(network.secret, None);
        assert_eq!(network.priority, 0);
        assert!(!network.hidden);
        assert_eq!(network.address, Address::Network);
    }

    fn network(name: &str) -> Network {
        Network {
            name: name.to_string(),
            security: Security::Personal,
            secret: Some(PathBuf::from("/var/lib/wifid/secrets/inn")),
            priority: 0,
            hidden: false,
            address: Address::Network,
            trusted: false,
            account: None,
        }
    }

    #[test]
    fn a_network_is_written_so_networkd_matches_its_name_exactly() {
        let mut file = Document::default();

        network(r#"Free*WiFi "5G""#).write_into(&mut file);

        assert_eq!(
            file.render(),
            concat!(
                r#"[Match]"#,
                "\n",
                r#"SSID="Free\*WiFi "'"'"5G"'"'"""#,
                "\n\n",
                "[X-WiFi]\nSecurity=personal\nSecret=/var/lib/wifid/secrets/inn\n",
                "Priority=0\nHidden=no\nAddress=network\n",
            )
        );
    }

    #[test]
    fn a_network_written_reads_back_the_same() {
        let mut file = Document::default();
        let written = network(r#" back\slash [and] it's "quoted"? "#);

        written.write_into(&mut file);

        assert_eq!(Network::read(&file), Ok(Some(written)));
    }

    fn office() -> Account {
        Account {
            method: Method::Peap,
            identity: "joel".into(),
            anonymous_identity: Some("anonymous".into()),
            server_domain: "radius.office.test".into(),
            authority: Some(PathBuf::from("/etc/office/authority.pem")),
            inner: Inner::Mschapv2,
        }
    }

    #[test]
    fn an_enterprise_network_says_whose_account_and_which_server() {
        let network = read(concat!(
            "[Match]\nSSID=Office\n\n",
            "[X-WiFi]\nSecurity=enterprise\nSecret=/run/keys/office\n",
            "Method=peap\nIdentity=joel\nAnonymousIdentity=anonymous\n",
            "ServerDomain=radius.office.test\nAuthority=/etc/office/authority.pem\n",
        ));

        let network = network.unwrap().unwrap();

        assert_eq!(network.security, Security::Enterprise);
        assert_eq!(network.account, Some(office()));
    }

    #[test]
    fn the_password_goes_through_the_tunnel_as_each_method_usually_sends_it() {
        let account = |method| {
            let text = format!(
                "[Match]\nSSID=Office\n\n[X-WiFi]\nSecurity=enterprise\nMethod={method}\n\
                 Identity=joel\nServerDomain=radius.office.test\n"
            );
            read(&text).unwrap().unwrap().account.unwrap()
        };

        assert_eq!(account("peap").inner, Inner::Mschapv2);
        assert_eq!(account("ttls").inner, Inner::Pap);
        assert_eq!(account("ttls").authority, None);
    }

    #[test]
    fn an_enterprise_network_naming_no_server_to_prove_itself_is_unreadable() {
        let network = read(concat!(
            "[Match]\nSSID=Office\n\n",
            "[X-WiFi]\nSecurity=enterprise\nMethod=peap\nIdentity=joel\n",
        ));

        assert!(network.is_err(), "{network:?}");
    }

    #[test]
    fn an_enterprise_network_written_reads_back_the_same() {
        let mut file = Document::default();
        let written = Network {
            security: Security::Enterprise,
            account: Some(office()),
            ..network("Office")
        };

        written.write_into(&mut file);

        assert_eq!(Network::read(&file), Ok(Some(written)));
    }

    #[test]
    fn a_network_no_longer_enterprise_keeps_no_account() {
        let mut file = Document::default();
        let enterprise = Network {
            security: Security::Enterprise,
            account: Some(office()),
            ..network("Office")
        };
        enterprise.write_into(&mut file);

        network("Office").write_into(&mut file);

        assert_eq!(file.value("X-WiFi", "Identity"), None);
        assert_eq!(file.value("X-WiFi", "ServerDomain"), None);
    }

    #[test]
    fn a_network_whose_file_turns_on_multicast_dns_is_trusted() {
        let network = read("[Match]\nSSID=Home\n\n[Network]\nMulticastDNS=yes\n\n[X-WiFi]\n");

        assert!(network.unwrap().unwrap().trusted);
    }

    #[test]
    fn trusting_a_network_turns_on_multicast_dns_and_untrusting_leaves_it_to_the_defaults() {
        let mut file = Document::default();
        let mut home = network("Home");

        home.trusted = true;
        home.write_into(&mut file);
        let trusted = file.value("Network", "MulticastDNS").map(str::to_string);
        home.trusted = false;
        home.write_into(&mut file);

        assert_eq!(trusted.as_deref(), Some("yes"));
        assert_eq!(file.value("Network", "MulticastDNS"), None);
    }

    #[test]
    fn a_saved_network_named_by_a_pattern_is_refused() {
        let refused = read("[Match]\nSSID=Hilton*\n\n[X-WiFi]\n");

        assert_eq!(
            refused,
            Err(Unreadable(
                "SSID=Hilton* is a pattern, not one network's name".to_string()
            ))
        );
        assert_eq!(
            read("[Match]\nSSID=!Lantern\n\n[X-WiFi]\n"),
            Err(Unreadable(
                "SSID=!Lantern is a pattern, not one network's name".to_string()
            ))
        );
    }

    #[test]
    fn a_saved_network_named_twice_is_refused() {
        let refused = read("[Match]\nSSID=Lantern \"Roadside Inn\"\n\n[X-WiFi]\n");

        assert_eq!(
            refused,
            Err(Unreadable(
                "SSID=Lantern \"Roadside Inn\" names more than one network".to_string()
            ))
        );
    }

    #[test]
    fn a_file_without_wifi_settings_is_not_a_saved_network() {
        assert_eq!(
            read("[Match]\nSSID=Hilton*\n\n[Network]\nDHCP=yes\n"),
            Ok(None)
        );
    }
}
