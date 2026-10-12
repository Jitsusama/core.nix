//! wifid joins Wi-Fi networks and finds the ones that want a sign-in.
//!
//! It starts by giving networkd a file for every network the launcher
//! saved, then gives wpa_supplicant every saved network on every card and
//! lets it choose which to join. wpa_supplicant starts again whenever a card
//! comes or goes, so wifid adds the cards again each time it does.
//!
//! Once a card's network reaches beyond itself, as networkd sees it, wifid
//! checks whether it really does or wants a sign-in first, and checks again
//! whenever the card moves to another network or is handed other DNS
//! servers.

mod address;
mod card;
mod check;
mod link;
mod lookup;
mod network;
mod secret;
mod store;
mod supplicant;
mod syntax;

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use futures_util::StreamExt;
use zbus::fdo::DBusProxy;
use zbus::zvariant::{self, ObjectPath};
use zbus::{Connection, MatchRule, MessageStream};

use address::AddressKey;
use card::Card;
use check::Verdict;
use link::Link;
use secret::Credentials;
use store::{Saved, Store};
use supplicant::{InterfaceProxy, JoinableProxy, SupplicantProxy};

/// Where wifid keeps what it must remember between starts.
const STATE: &str = "/var/lib/wifid";

/// What every card wpa_supplicant is given starts from.
const CARD_SETTINGS: &str = "/etc/wpa_supplicant/interface.conf";

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("<3>{error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let store = Store {
        declared: PathBuf::from("/etc/systemd/network"),
        saved: Path::new(STATE).join("networks"),
        runtime: PathBuf::from("/run/systemd/network"),
        defaults: PathBuf::from("/etc/systemd/network/90-wifi.network"),
    };
    fs::create_dir_all(&store.saved)?;
    fs::create_dir_all(&store.runtime)?;
    store.give_networkd_all()?;
    let key = AddressKey::kept_in(&Path::new(STATE).join("address-key"))?;
    let credentials = Credentials::system();

    let bus = Connection::system().await?;
    reload_networkd(&bus).await?;

    let owners = DBusProxy::new(&bus).await?;
    let mut changes = owners.receive_name_owner_changed().await?;
    let supplicant = SupplicantProxy::new(&bus).await?;
    let wifid = Wifid {
        bus: &bus,
        supplicant: &supplicant,
        store: &store,
        key: &key,
        credentials: &credentials,
    };
    wifid.add_cards().await;

    let rule = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface("org.freedesktop.DBus.Properties")?
        .member("PropertiesChanged")?
        .path_namespace("/org/freedesktop/network1/link")?
        .build();
    let mut links = MessageStream::for_match_rule(rule, &bus, None).await?;
    let mut checked = HashMap::new();
    for link in all_links(&bus).await {
        consider(link, &mut checked);
    }
    loop {
        tokio::select! {
            Some(change) = changes.next() => {
                let Ok(args) = change.args() else { continue };
                if args.new_owner().is_none() {
                    continue;
                }
                match args.name().as_str() {
                    "fi.w1.wpa_supplicant1" => wifid.add_cards().await,
                    "org.freedesktop.network1" => {
                        for link in all_links(&bus).await {
                            consider(link, &mut checked);
                        }
                    }
                    _ => {}
                }
            }
            Some(Ok(message)) = links.next() => {
                let header = message.header();
                let Some(path) = header.path() else { continue };
                if let Some(link) = one_link(&bus, path).await {
                    consider(link, &mut checked);
                }
            }
            else => return Ok(()),
        }
    }
}

/// What a network was last checked as: its name and DNS servers.
type Checked = (Option<String>, Vec<IpAddr>);

/// Checks the link's network when it reaches beyond itself and wasn't
/// checked as it stands, without holding up anything else.
fn consider(link: Link, checked: &mut HashMap<i32, Checked>) {
    if !link.routable {
        checked.remove(&link.index);
        return;
    }
    let standing = (link.ssid.clone(), link.dns.clone());
    if checked.get(&link.index) == Some(&standing) {
        return;
    }
    checked.insert(link.index, standing);
    tokio::spawn(async move {
        let verdict = check::check(&link.name, &link.dns).await;
        let network = link.ssid.as_deref().unwrap_or("The network");
        let card = &link.name;
        match verdict {
            Verdict::Online => println!("<6>{network} on {card} reaches beyond itself"),
            Verdict::SignIn { page } => {
                println!("<5>{network} on {card} wants a sign-in at {page}")
            }
            Verdict::Limited => println!("<4>{network} on {card} reaches no further"),
        }
    });
}

/// Every Wi-Fi card's link, as networkd describes them.
async fn all_links(bus: &Connection) -> Vec<Link> {
    let described = bus
        .call_method(
            Some("org.freedesktop.network1"),
            "/org/freedesktop/network1",
            Some("org.freedesktop.network1.Manager"),
            "Describe",
            &(),
        )
        .await;
    let Some(described) = json(described) else {
        return Vec::new();
    };
    let links = described["Interfaces"].as_array().into_iter().flatten();
    links.filter_map(Link::described).collect()
}

/// One link, as networkd describes it, if it's a Wi-Fi card's.
async fn one_link(bus: &Connection, path: &ObjectPath<'_>) -> Option<Link> {
    let described = bus
        .call_method(
            Some("org.freedesktop.network1"),
            path,
            Some("org.freedesktop.network1.Link"),
            "Describe",
            &(),
        )
        .await;
    Link::described(&json(described)?)
}

/// The JSON a Describe call answered with; none, said in the journal,
/// when it failed.
fn json(answer: zbus::Result<zbus::Message>) -> Option<serde_json::Value> {
    let read = answer.and_then(|message| message.body().deserialize::<String>());
    match read.map(|text| serde_json::from_str(&text)) {
        Ok(Ok(described)) => Some(described),
        Ok(Err(error)) => {
            eprintln!("<4>networkd described a link oddly: {error}");
            None
        }
        Err(error) => {
            eprintln!("<4>networkd didn't describe its links: {error}");
            None
        }
    }
}

/// Asks networkd to read its files again, so it has wifid's.
async fn reload_networkd(bus: &Connection) -> zbus::Result<()> {
    bus.call_method(
        Some("org.freedesktop.network1"),
        "/org/freedesktop/network1",
        Some("org.freedesktop.network1.Manager"),
        "Reload",
        &(),
    )
    .await
    .map(|_| ())
}

/// What wifid works with once it has started.
struct Wifid<'a> {
    bus: &'a Connection,
    supplicant: &'a SupplicantProxy<'a>,
    store: &'a Store,
    key: &'a AddressKey,
    credentials: &'a Credentials,
}

impl Wifid<'_> {
    /// Gives wpa_supplicant every card, each with every saved network. A card
    /// or network that can't be added is said in the journal and passed over,
    /// so one bad file never keeps the machine off every network.
    async fn add_cards(&self) {
        let cards = match card::all(Path::new("/sys")) {
            Ok(cards) => cards,
            Err(error) => return eprintln!("<3>can't list the Wi-Fi cards: {error}"),
        };
        let networks = match self.store.networks() {
            Ok(listing) => {
                for (file, reason) in listing.unreadable {
                    eprintln!("<4>{}: {reason}", file.display());
                }
                listing.saved
            }
            Err(error) => return eprintln!("<3>can't read the saved networks: {error}"),
        };
        for card in cards {
            if let Err(error) = self.add_card(&card, &networks).await {
                eprintln!("<3>can't add {}: {error}", card.name);
            }
        }
    }

    async fn add_card(&self, card: &Card, networks: &[Saved]) -> zbus::Result<()> {
        let (bus, supplicant) = (self.bus, self.supplicant);
        let path = match supplicant.get_interface(&card.name).await {
            Ok(path) => path,
            Err(_) => {
                let args = HashMap::from([
                    ("Ifname", zvariant::Value::from(card.name.as_str())),
                    ("ConfigFile", zvariant::Value::from(CARD_SETTINGS)),
                ]);
                supplicant.create_interface(args).await?
            }
        };
        let interface = InterfaceProxy::builder(bus).path(path)?.build().await?;
        interface.remove_all_networks().await?;
        let mut added: Vec<&str> = Vec::new();
        for saved in networks {
            let name = saved.network.name.as_str();
            // Nix's networks come first, and win over the launcher's of the
            // same name.
            if added.contains(&name) {
                continue;
            }
            let password = match secret::password(&saved.network, self.credentials).await {
                Ok(password) => password,
                Err(error) => {
                    eprintln!("<4>{name}: can't read its password: {error}");
                    continue;
                }
            };
            let address = self.key.address(card.address, name);
            let fields = match supplicant::properties(&saved.network, password.as_deref(), address)
            {
                Ok(fields) => fields,
                Err(unjoinable) => {
                    eprintln!("<4>{name}: {}", unjoinable.0);
                    continue;
                }
            };
            let path = interface
                .add_network(supplicant::dictionary(fields))
                .await?;
            JoinableProxy::builder(bus)
                .path(path)?
                .build()
                .await?
                .set_enabled(true)
                .await?;
            added.push(name);
        }
        Ok(())
    }
}
