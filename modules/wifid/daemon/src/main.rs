//! wifid joins Wi-Fi networks and finds the ones that want a sign-in.
//!
//! It starts by giving networkd a file for every network the launcher
//! saved, then gives wpa_supplicant every saved network on every card and
//! lets it choose which to join. It gives a card networkd finds later the
//! same, and every card again whenever wpa_supplicant starts again. A card
//! that already holds a network as wanted keeps it, so wifid starting
//! again never takes a card off the network it's on.
//!
//! Once a card's network reaches beyond itself, as networkd sees it, wifid
//! checks whether it really does or wants a sign-in first. It checks again
//! whenever the card moves to another network or is handed other DNS
//! servers, as the machine wakes, when asked, and on a timer the verdict
//! sets.
//!
//! It answers the shell over varlink, as io.jitsusama.Wifi (service.rs),
//! with what it knows of each card, every saved network and the networks in
//! the air, which wpa_supplicant says as it hears them.

mod address;
mod air;
mod card;
mod check;
mod link;
mod lookup;
mod network;
mod page;
mod secret;
mod service;
mod state;
mod store;
mod supplicant;
mod syntax;

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::future;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use futures_util::StreamExt;
use tokio::sync::{mpsc, watch};
use tokio::time::{Duration, Instant, sleep_until};
use zbus::fdo::{DBusProxy, PropertiesProxy};
use zbus::names::InterfaceName;
use zbus::proxy::CacheProperties;
use zbus::zvariant::{self, ObjectPath, OwnedObjectPath, OwnedValue};
use zbus::{Connection, MatchRule, MessageStream, proxy};
use zlink::Server;

use address::AddressKey;
use air::AccessPoint;
use card::Card;
use check::{Checker, Settings, Verdict};
use link::Link;
use network::{Address, Network, Security};
use secret::Credentials;
use service::{Ask, Refused, Stage, Wifi};
use state::{Progress, Reach, State, Trouble};
use store::{Saved, Store};
use supplicant::{InterfaceProxy, JoinableProxy, Step, SupplicantProxy};

/// Where wifid keeps what it must remember between starts.
const STATE: &str = "/var/lib/wifid";

/// What every card wpa_supplicant is given starts from.
const CARD_SETTINGS: &str = "/etc/wpa_supplicant/interface.conf";

/// How long a card has to join a network the shell asked for.
const JOINING: Duration = Duration::from_secs(45);

/// Which pages the check asks.
const SETTINGS: &str = "/etc/wifid/wifid.conf";

/// The certificate authorities the machine trusts.
const CERTIFICATES: &str = "/etc/ssl/certs/ca-certificates.crt";

/// logind, which says when the machine sleeps and wakes.
#[proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait Login {
    #[zbus(signal)]
    fn prepare_for_sleep(&self, start: bool) -> zbus::Result<()>;
}

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
    // Nothing wifid can do without stops it: a network it can't give
    // networkd a file for still joins, under the Wi-Fi defaults.
    if let Err(error) = store.give_networkd_all() {
        eprintln!("<3>can't give networkd the saved networks' files: {error}");
    }
    let key = AddressKey::kept_in(&Path::new(STATE).join("address-key")).or_else(|error| {
        eprintln!("<3>{error}; every network sees a new address until wifid starts again");
        AddressKey::for_now()
    })?;
    let credentials = Credentials::system();

    let bus = Connection::system().await?;
    // networkd reads the files anyway when it next starts.
    if let Err(error) = reload_networkd(&bus).await {
        eprintln!("<4>can't ask networkd to read its files again: {error}");
    }

    let owners = DBusProxy::new(&bus).await?;
    let mut changes = owners.receive_name_owner_changed().await?;
    let supplicant = SupplicantProxy::new(&bus).await?;
    let state = watch::Sender::new(State::default());
    let wifid = Wifid {
        bus: &bus,
        supplicant: &supplicant,
        store: &store,
        key: &key,
        credentials: &credentials,
        state: &state,
    };
    wifid.add_cards().await;

    // Followed before the access points are first read, so none heard
    // meanwhile is missed.
    let rule = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .path_namespace("/fi/w1/wpa_supplicant1")?
        .build();
    let mut heard = MessageStream::for_match_rule(rule, &bus, None).await?;
    let mut air = Air {
        points: HashMap::new(),
        state: state.clone(),
    };
    air.hear_all(&bus, &supplicant).await;

    let rule = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface("org.freedesktop.DBus.Properties")?
        .member("PropertiesChanged")?
        .path_namespace("/org/freedesktop/network1/link")?
        .build();
    let mut links = MessageStream::for_match_rule(rule, &bus, None).await?;
    let mut sleeps = LoginProxy::new(&bus)
        .await?
        .receive_prepare_for_sleep()
        .await?;
    let mut networks = Networks {
        checker: Arc::new(Checker {
            settings: settings(),
            tls: trusted(),
        }),
        asked: watch::Sender::new(0),
        state: state.clone(),
        followed: HashMap::new(),
    };
    let (asks, mut taken) = mpsc::channel(1);
    let wifi = Wifi {
        state: state.subscribe(),
        asked: networks.asked.clone(),
        asks,
    };
    // Every card there is now was given wpa_supplicant above.
    networks.follow_all(all_links(&bus).await);
    let mut joining: Option<Joining> = None;
    let mut watched = state.subscribe();
    let following = async {
        loop {
            let deadline = joining.as_ref().map(|joining| joining.deadline);
            tokio::select! {
                Some(change) = changes.next() => {
                    let Ok(args) = change.args() else { continue };
                    if args.new_owner().is_none() {
                        continue;
                    }
                    match args.name().as_str() {
                        // It starts again hearing nothing, and says each
                        // access point as it hears it.
                        "fi.w1.wpa_supplicant1" => {
                            air.gone_under("");
                            wifid.add_cards().await;
                        }
                        "org.freedesktop.network1" => {
                            for card in networks.follow_all(all_links(&bus).await) {
                                wifid.add_card_named(&card).await;
                            }
                        }
                        _ => {}
                    }
                }
                Some(Ok(message)) = links.next() => {
                    let header = message.header();
                    let Some(path) = header.path() else { continue };
                    match one_link(&bus, path).await {
                        Some(link) => {
                            let card = link.name.clone();
                            if networks.follow(link) {
                                wifid.add_card_named(&card).await;
                            }
                        }
                        // Not a card, or one going: whichever, networkd's
                        // own list says which cards are left.
                        None => {
                            for card in networks.follow_all(all_links(&bus).await) {
                                wifid.add_card_named(&card).await;
                            }
                        }
                    }
                }
                Some(Ok(message)) = heard.next() => air.take_in(&bus, &message).await,
                Some(ask) = taken.recv() => match ask {
                    Ask::Join { name, password, progress } => {
                        if let Some(before) = joining.take() {
                            let instead = Refused::Failed {
                                reason: format!("joining {name} instead"),
                            };
                            wifid.settle(before, Err(instead)).await;
                        }
                        joining = wifid.join(name, password, progress, &air).await;
                    }
                    ask => wifid.answer(ask).await,
                },
                Ok(()) = watched.changed(), if joining.is_some() => {
                    if let Some(ongoing) = joining.take() {
                        joining = wifid.advance(ongoing).await;
                    }
                }
                () = until(deadline), if joining.is_some() => {
                    if let Some(late) = joining.take() {
                        wifid.settle(late, Err(Refused::TimedOut)).await;
                    }
                }
                Some(sleep) = sleeps.next() => {
                    if sleep.args().is_ok_and(|args| !args.start) {
                        networks.asked.send_modify(|asked| *asked += 1);
                    }
                }
                else => return Ok(()),
            }
        }
    };
    tokio::select! {
        result = following => result,
        result = answer(wifi) => result,
    }
}

/// Answers on the socket systemd holds, for as long as wifid runs. Run by
/// hand, with no socket, it answers nobody.
async fn answer(wifi: Wifi) -> Result<(), Box<dyn Error>> {
    let Some(listener) = service::listener()? else {
        eprintln!("<4>systemd passed no socket, so nobody can ask wifid anything");
        return future::pending().await;
    };
    // Ending wifid has systemd start it again, which leaves every card on
    // its network and gives the shell its answers back.
    Server::new(listener, wifi).run().await?;
    Err("stopped answering on the socket".into())
}

/// The pages wifid.conf names; Google's, said in the journal, when it
/// can't be read.
fn settings() -> Settings {
    let read = fs::read_to_string(SETTINGS).map_err(|error| error.to_string());
    match read.and_then(|text| Settings::read(&text)) {
        Ok(settings) => settings,
        Err(reason) => {
            let missing = fs::exists(SETTINGS).is_ok_and(|exists| !exists);
            if !missing {
                eprintln!("<4>{SETTINGS}: {reason}; asking Google's pages instead");
            }
            Settings::default()
        }
    }
}

/// What TLS trusts; nothing, said in the journal, when the certificates
/// can't be read, which leaves only the plain check.
fn trusted() -> Option<Arc<rustls::ClientConfig>> {
    page::trusting(Path::new(CERTIFICATES))
        .inspect_err(|error: &io::Error| {
            eprintln!("<4>{CERTIFICATES}: {error}; only the plain check will run")
        })
        .ok()
}

/// Every Wi-Fi card's network, each followed by a task of its own.
struct Networks {
    checker: Arc<Checker>,
    /// Counted up as the machine wakes and when someone asks, for every
    /// task to check its network again.
    asked: watch::Sender<u64>,
    state: watch::Sender<State>,
    followed: HashMap<i32, watch::Sender<Link>>,
}

impl Networks {
    /// Hands the link to its card's task, starting one for a card new to
    /// wifid, and says whether it was.
    fn follow(&mut self, link: Link) -> bool {
        change(&self.state, |state| state.follow(&link));
        if let Some(followed) = self.followed.get(&link.index) {
            followed.send_replace(link);
            return false;
        }
        let index = link.index;
        let (followed, link) = watch::channel(link);
        let checker = Arc::clone(&self.checker);
        let (asked, state) = (self.asked.subscribe(), self.state.clone());
        tokio::spawn(check_network(link, asked, checker, state));
        self.followed.insert(index, followed);
        true
    }

    /// Follows these, every Wi-Fi card there is, and stops following any
    /// other, whose task ends with it. Gives the names of the cards new to
    /// wifid.
    fn follow_all(&mut self, links: Vec<Link>) -> Vec<String> {
        let there: Vec<i32> = links.iter().map(|link| link.index).collect();
        self.followed.retain(|index, _| there.contains(index));
        change(&self.state, |state| state.keep_only(&there));
        let mut new = Vec::new();
        for link in links {
            let card = link.name.clone();
            if self.follow(link) {
                new.push(card);
            }
        }
        new
    }
}

/// Waits until then, or forever.
async fn until(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => sleep_until(deadline).await,
        None => future::pending().await,
    }
}

/// A join the shell asked for, followed until the network's reach is
/// known or the card couldn't join it.
struct Joining {
    name: String,
    card: String,
    /// Whether the join saved the network, so it's forgotten again if the
    /// card couldn't join it.
    new: bool,
    progress: mpsc::Sender<Result<Stage, Refused>>,
    said: Option<Stage>,
    deadline: Instant,
}

/// The access points every card hears, kept as wpa_supplicant last
/// described each.
struct Air {
    /// By the access point's path.
    points: HashMap<String, AccessPoint>,
    state: watch::Sender<State>,
}

impl Air {
    /// Reads every access point wpa_supplicant already hears, as it does
    /// when wifid starts while it runs.
    async fn hear_all(&mut self, bus: &Connection, supplicant: &SupplicantProxy<'_>) {
        let cards = match supplicant.interfaces().await {
            Ok(cards) => cards,
            Err(error) => return eprintln!("<4>wpa_supplicant didn't list its cards: {error}"),
        };
        for card in cards {
            let points = match interface(bus, card).await {
                Ok(interface) => interface.access_points().await,
                Err(error) => Err(error),
            };
            for point in points.into_iter().flatten() {
                // One gone since it was listed needs no saying.
                if let Ok(properties) = described(bus, &point).await {
                    self.heard(point.as_str(), properties);
                }
            }
        }
    }

    /// The card hearing the network most strongly, as wpa_supplicant's path
    /// for it.
    fn strongest(&self, name: &str) -> Option<String> {
        let heard = self.points.iter().filter_map(|(path, point)| {
            let heard = point.heard().filter(|heard| heard.name == name)?;
            let (card, _) = path.split_once("/BSSs/")?;
            Some((heard.signal, card))
        });
        heard.max().map(|(_, card)| card.to_string())
    }

    /// Takes in a signal of wpa_supplicant's: an access point heard, heard
    /// differently or no longer heard, a card going, or a card's join going
    /// wrong.
    async fn take_in(&mut self, bus: &Connection, message: &zbus::Message) {
        let header = message.header();
        let (Some(path), Some(member)) = (header.path(), header.member()) else {
            return;
        };
        let body = message.body();
        match member.as_str() {
            "BSSAdded" => {
                if let Ok((point, properties)) = body.deserialize::<(OwnedObjectPath, _)>() {
                    self.heard(point.as_str(), properties);
                }
            }
            "BSSRemoved" => {
                if let Ok(point) = body.deserialize::<OwnedObjectPath>() {
                    self.points.remove(point.as_str());
                    change(&self.state, |state| state.heard(point.as_str(), None));
                }
            }
            "InterfaceRemoved" => {
                if let Ok(card) = body.deserialize::<OwnedObjectPath>() {
                    self.gone_under(&format!("{}/", card.as_str()));
                }
            }
            "PskMismatch" => self.troubled(bus, path, Trouble::WrongPassword).await,
            // Each object also says its changes under its own interface, in
            // a shape of its own, which this passes over.
            "PropertiesChanged"
                if header
                    .interface()
                    .is_some_and(|interface| interface == "org.freedesktop.DBus.Properties") =>
            {
                let changed =
                    body.deserialize::<(String, HashMap<String, OwnedValue>, Vec<String>)>();
                let Ok((interface, changed, _)) = changed else {
                    return;
                };
                match interface.as_str() {
                    "fi.w1.wpa_supplicant1.BSS" => {
                        let Some(point) = self.points.get_mut(path.as_str()) else {
                            return;
                        };
                        point.changed(changed);
                        let heard = point.heard();
                        change(&self.state, |state| state.heard(path.as_str(), heard));
                    }
                    "fi.w1.wpa_supplicant1.Interface" => {
                        let steps = [
                            ("AuthStatusCode", Step::Authentication),
                            ("AssocStatusCode", Step::Association),
                        ];
                        for (property, step) in steps {
                            let code = changed.get(property).map(|code| &**code);
                            let Some(&zvariant::Value::I32(code)) = code else {
                                continue;
                            };
                            // Only a status that ends a join is trouble,
                            // and the network's kind only matters to WPA3.
                            let said =
                                supplicant::Trouble::from_status(step, code, Security::Personal);
                            let trouble = match said {
                                Some(supplicant::Trouble::WrongPassword) => Trouble::WrongPassword,
                                Some(supplicant::Trouble::Refused(_)) => Trouble::TurnedAway,
                                None => continue,
                            };
                            self.troubled(bus, path, trouble).await;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    /// Says what went wrong on the card wpa_supplicant has at this path.
    async fn troubled(&self, bus: &Connection, card: &ObjectPath<'_>, trouble: Trouble) {
        let named = match interface(bus, card.to_owned().into()).await {
            Ok(interface) => interface.ifname().await,
            Err(error) => Err(error),
        };
        match named {
            Ok(name) => change(&self.state, |state| state.troubled(&name, Some(trouble))),
            Err(error) => eprintln!("<4>can't tell which card {card} is: {error}"),
        }
    }

    fn heard(&mut self, path: &str, properties: HashMap<String, OwnedValue>) {
        let point = AccessPoint::new(properties);
        let heard = point.heard();
        self.points.insert(path.into(), point);
        change(&self.state, |state| state.heard(path, heard));
    }

    /// Forgets every access point whose path starts so.
    fn gone_under(&mut self, path: &str) {
        self.points.retain(|point, _| !point.starts_with(path));
        change(&self.state, |state| state.unheard_under(path));
    }
}

/// Everything wpa_supplicant says of an access point.
async fn described(
    bus: &Connection,
    point: &OwnedObjectPath,
) -> zbus::Result<HashMap<String, OwnedValue>> {
    let properties = PropertiesProxy::builder(bus)
        .destination("fi.w1.wpa_supplicant1")?
        .path(point.clone())?
        .cache_properties(CacheProperties::No)
        .build()
        .await?;
    let bss = InterfaceName::from_static_str_unchecked("fi.w1.wpa_supplicant1.BSS");
    Ok(properties.get_all(bss).await?)
}

/// One card, as wpa_supplicant drives it.
async fn interface(bus: &Connection, card: OwnedObjectPath) -> zbus::Result<InterfaceProxy<'_>> {
    InterfaceProxy::builder(bus)
        .path(card)?
        .cache_properties(CacheProperties::No)
        .build()
        .await
}

/// What the shell is told of a change to the saved networks failing.
fn refused(failure: store::Failure) -> Refused {
    match failure {
        store::Failure::Declared(_) => Refused::Declared,
        store::Failure::NotSaved => Refused::NotSaved,
        store::Failure::Io(error) => Refused::Failed {
            reason: error.to_string(),
        },
    }
}

/// Tells everyone listening what changed, if anything did.
fn change(state: &watch::Sender<State>, change: impl FnOnce(&mut State)) {
    state.send_if_modified(|state| {
        let before = state.clone();
        change(state);
        *state != before
    });
}

/// Checks a card's network whenever it newly reaches beyond itself, then
/// again when the last verdict says to, the machine wakes or someone asks.
/// A verdict is said in the journal when it's news.
async fn check_network(
    mut link: watch::Receiver<Link>,
    mut asked: watch::Receiver<u64>,
    checker: Arc<Checker>,
    state: watch::Sender<State>,
) {
    let mut checked = None;
    let mut said = None;
    let mut due = None;
    loop {
        let now = link.borrow_and_update().clone();
        if now.routable {
            let standing = (now.ssid.clone(), now.dns.clone(), now.portal.clone());
            let reached = checked.as_ref() != Some(&standing);
            if reached || due.is_some_and(|due| Instant::now() >= due) {
                change(&state, |state| state.checking(now.index));
                let (verdict, after) = checker
                    .check(&now.name, &now.dns, now.portal.as_deref(), reached)
                    .await;
                if reached || said.as_ref() != Some(&verdict) {
                    say(&now, &verdict);
                }
                change(&state, |state| state.judged(now.index, &verdict));
                said = Some(verdict);
                checked = Some(standing);
                due = Some(Instant::now() + after);
            }
        } else {
            (checked, said, due) = (None, None, None);
        }
        let timer = async {
            match due {
                Some(due) => sleep_until(due).await,
                None => future::pending().await,
            }
        };
        tokio::select! {
            changed = link.changed() => if changed.is_err() { return },
            Ok(()) = asked.changed() => due = due.map(|_| Instant::now()),
            () = timer => {}
        }
    }
}

fn say(link: &Link, verdict: &Verdict) {
    let network = link.ssid.as_deref().unwrap_or("The network");
    let card = &link.name;
    match verdict {
        Verdict::Online => println!("<6>{network} on {card} reaches beyond itself"),
        Verdict::SignIn { page } => println!("<5>{network} on {card} wants a sign-in at {page}"),
        Verdict::Limited => println!("<4>{network} on {card} reaches no further"),
    }
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
/// A link that has gone can't be described, which needs no saying.
async fn one_link(bus: &Connection, path: &ObjectPath<'_>) -> Option<Link> {
    let described = bus
        .call_method(
            Some("org.freedesktop.network1"),
            path,
            Some("org.freedesktop.network1.Link"),
            "Describe",
            &(),
        )
        .await
        .ok()?;
    Link::described(&json(Ok(described))?)
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
    state: &'a watch::Sender<State>,
}

impl Wifid<'_> {
    /// Gives wpa_supplicant every card, each with every saved network. A card
    /// or network that can't be added is said in the journal and passed over,
    /// so one bad file never keeps the machine off every network.
    async fn add_cards(&self) {
        self.add(|_| true).await
    }

    /// Gives wpa_supplicant the card of that name, as add_cards does.
    async fn add_card_named(&self, name: &str) {
        self.add(|card| card.name == name).await
    }

    async fn add(&self, wanted: impl Fn(&Card) -> bool) {
        let cards = match card::all(Path::new("/sys")) {
            Ok(cards) => cards,
            Err(error) => return eprintln!("<3>can't list the Wi-Fi cards: {error}"),
        };
        let networks = match self.store.networks() {
            Ok(listing) => {
                for (file, reason) in listing.unreadable {
                    eprintln!("<4>{}: {reason}", file.display());
                }
                change(self.state, |state| state.saved(&listing.saved));
                listing.saved
            }
            Err(error) => return eprintln!("<3>can't read the saved networks: {error}"),
        };
        for card in cards.into_iter().filter(wanted) {
            if let Err(error) = self.add_card(&card, &networks).await {
                eprintln!("<3>can't add {}: {error}", card.name);
            }
        }
    }

    /// Gives wpa_supplicant the card with every saved network, and says
    /// where it holds each, by name.
    async fn add_card(
        &self,
        card: &Card,
        networks: &[Saved],
    ) -> zbus::Result<Vec<(String, OwnedObjectPath)>> {
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
        let interface = interface(bus, path).await?;
        let mut added: Vec<&str> = Vec::new();
        let mut wanted = Vec::new();
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
            wanted.push(supplicant::marked(fields, self.key));
            added.push(name);
        }
        let marks: Vec<String> = wanted.iter().map(|(mark, _)| mark.clone()).collect();
        let named = |mark: &str| {
            let at = marks.iter().position(|wanted| wanted == mark)?;
            Some(added[at].to_string())
        };

        let mut held = Vec::new();
        for network in interface.networks().await? {
            let fields = self.joinable(&network).await?.properties().await?;
            let mark = fields
                .get("id_str")
                .and_then(|field| <&str>::try_from(field).ok());
            let mark = mark.and_then(supplicant::unquoted).map(str::to_string);
            held.push((network, mark));
        }
        let (gone, new) = supplicant::changes(&held, &marks);
        for network in &gone {
            interface.remove_network(&network.as_ref()).await?;
        }
        let mut joinable: Vec<(String, OwnedObjectPath)> = held
            .iter()
            .filter(|(network, _)| !gone.contains(&network))
            .filter_map(|(network, mark)| Some((named(mark.as_deref()?)?, network.clone())))
            .collect();
        for i in new {
            let (mark, fields) = std::mem::take(&mut wanted[i]);
            let network = interface
                .add_network(supplicant::dictionary(fields))
                .await?;
            joinable.extend(named(&mark).map(|name| (name, network)));
        }
        // Turning on one already on changes nothing. One left off, as a
        // join from the launcher leaves the others for a moment, is on again.
        for (_, network) in &joinable {
            self.joinable(network).await?.set_enabled(true).await?;
        }
        Ok(joinable)
    }

    /// Starts joining the network as the shell asked, saving it first when
    /// it's new or given a password, and says so through progress. Gives
    /// back the join to follow, or nothing once it's said why it couldn't.
    async fn join(
        &self,
        name: String,
        password: Option<String>,
        progress: mpsc::Sender<Result<Stage, Refused>>,
        air: &Air,
    ) -> Option<Joining> {
        match self.start(&name, password, air).await {
            Ok((card, new)) => {
                let joining = Joining {
                    name,
                    card,
                    new,
                    progress,
                    said: None,
                    deadline: Instant::now() + JOINING,
                };
                self.advance(joining).await
            }
            Err(refused) => {
                let _ = progress.send(Err(refused)).await;
                None
            }
        }
    }

    /// Saves the network as the join needs and has a card join it. Gives
    /// the card's name, and whether the network is new.
    async fn start(
        &self,
        name: &str,
        password: Option<String>,
        air: &Air,
    ) -> Result<(String, bool), Refused> {
        let failed = |reason: String| Refused::Failed { reason };
        let listing = self
            .store
            .networks()
            .map_err(|error| failed(error.to_string()))?;
        let saved = listing
            .saved
            .into_iter()
            .find(|saved| saved.network.name == name);
        let heard = self.state.borrow().described().nearby;
        let heard = heard.into_iter().find(|nearby| nearby.name == name);
        let new = saved.is_none();
        let mut network = match (&saved, heard) {
            (Some(saved), _) => saved.network.clone(),
            (None, Some(heard)) => Network {
                name: name.to_string(),
                security: match heard.security {
                    state::Security::Open => Security::Open,
                    state::Security::Personal => Security::Personal,
                    state::Security::Enterprise => {
                        return Err(failed("wifid can't join with an account yet".into()));
                    }
                },
                secret: None,
                priority: 0,
                hidden: false,
                address: Address::Network,
                trusted: false,
            },
            (None, None) => return Err(Refused::NotNearby),
        };
        let declared = saved.as_ref().is_some_and(|saved| saved.declared);
        let given = match (network.security, password) {
            (Security::Personal, Some(_)) if declared => return Err(Refused::Declared),
            (Security::Personal, Some(password)) => Some(password),
            (Security::Personal, None) if network.secret.is_none() => {
                return Err(Refused::NeedsPassword);
            }
            // An open network has no use for a password it's given.
            _ => None,
        };
        if let Some(password) = &given {
            supplicant::psk(password).map_err(|unjoinable| failed(unjoinable.0.into()))?;
            let file = self.store.password_file(name).map_err(refused)?;
            let sealed_as = file.file_stem().unwrap_or_default().to_string_lossy();
            let sealed = self.credentials.seal(&sealed_as, password).await;
            let sealed =
                sealed.map_err(|error| failed(format!("can't seal the password: {error}")))?;
            self.store
                .keep_password(&file, &sealed)
                .map_err(|error| failed(error.to_string()))?;
            network.secret = Some(file);
        }
        if new || given.is_some() {
            self.store.save(&network).map_err(refused)?;
        }
        self.given_everything().await?;

        let card = self.card_for(name, air).await?;
        change(self.state, |state| state.troubled(&card.name, None));
        let networks = self
            .store
            .networks()
            .map_err(|error| failed(error.to_string()))?;
        let joinable = self.add_card(&card, &networks.saved).await;
        let joinable = joinable.map_err(|error| failed(error.to_string()))?;
        let Some((_, path)) = joinable.into_iter().find(|(held, _)| held == name) else {
            return Err(failed(format!("wpa_supplicant wasn't given {name}")));
        };
        let selected = match self.supplicant.get_interface(&card.name).await {
            Ok(at) => match interface(self.bus, at).await {
                Ok(interface) => interface.select_network(&path.as_ref()).await,
                Err(error) => Err(error),
            },
            Err(error) => Err(error),
        };
        selected.map_err(|error| failed(format!("wpa_supplicant didn't join it: {error}")))?;
        println!("<6>joining {name} on {}, as asked", card.name);
        Ok((card.name, new))
    }

    /// The card hearing the network most strongly, or the first there is
    /// when none hears it, as when it hides its name.
    async fn card_for(&self, name: &str, air: &Air) -> Result<Card, Refused> {
        let failed = |reason: String| Refused::Failed { reason };
        let cards = card::all(Path::new("/sys")).map_err(|error| failed(error.to_string()))?;
        if let Some(at) = air.strongest(name) {
            let at = OwnedObjectPath::try_from(at).map_err(|error| failed(error.to_string()))?;
            let named = match interface(self.bus, at).await {
                Ok(interface) => interface.ifname().await.ok(),
                Err(_) => None,
            };
            if let Some(card) = cards.iter().find(|card| Some(&card.name) == named.as_ref()) {
                return Ok(card.clone());
            }
        }
        cards
            .into_iter()
            .next()
            .ok_or_else(|| failed("there's no Wi-Fi card".into()))
    }

    /// Says how far the join has got, if that's news, and settles it once
    /// the card is done. Gives back a join still going.
    async fn advance(&self, mut joining: Joining) -> Option<Joining> {
        let progress = self.state.borrow().progress(&joining.card, &joining.name);
        let stage = match progress {
            Progress::Joining => Stage::Joining,
            Progress::GettingAddress => Stage::GettingAddress,
            Progress::Checking | Progress::Reached(Reach::Checking) => Stage::Checking,
            Progress::Reached(Reach::Online) => Stage::Online,
            Progress::Reached(Reach::SignIn) => Stage::SignIn,
            Progress::Reached(Reach::Limited) => Stage::Limited,
            Progress::Troubled(Trouble::WrongPassword) => {
                self.settle(joining, Err(Refused::WrongPassword)).await;
                return None;
            }
            Progress::Troubled(Trouble::TurnedAway) => {
                self.settle(joining, Err(Refused::TurnedAway)).await;
                return None;
            }
            Progress::Gone => {
                let gone = Refused::Failed {
                    reason: format!("{} has gone", joining.card),
                };
                self.settle(joining, Err(gone)).await;
                return None;
            }
        };
        if joining.said != Some(stage) {
            // Whoever asked may have stopped listening, which changes nothing.
            let _ = joining.progress.send(Ok(stage)).await;
            joining.said = Some(stage);
        }
        if stage.is_final() {
            self.settle(joining, Ok(())).await;
            return None;
        }
        Some(joining)
    }

    /// Ends the join: says why it couldn't, forgets a network it saved that
    /// the card couldn't join, and turns the card's other networks back on.
    async fn settle(&self, joining: Joining, outcome: Result<(), Refused>) {
        let Joining {
            name, card, new, ..
        } = &joining;
        match outcome {
            Ok(()) => println!("<6>joined {name} on {card}"),
            Err(refused) => {
                println!("<5>{card} couldn't join {name}: {refused:?}");
                let _ = joining.progress.send(Err(refused)).await;
                if *new {
                    if let Err(error) = self.store.forget(name) {
                        eprintln!("<4>can't forget {name} again: {error:?}");
                    }
                    // Taking it from networkd and wpa_supplicant turns the
                    // others on with it.
                    let _ = self.given_everything().await;
                    return;
                }
            }
        }
        self.add_card_named(card).await;
    }

    /// Changes the saved networks as asked, and says how it went once
    /// networkd and wpa_supplicant have every network as it now stands.
    async fn answer(&self, ask: Ask) {
        let (changed, done) = match ask {
            Ask::Join { .. } => unreachable!("the main loop follows each join itself"),
            Ask::Scan { done } => {
                let _ = done.send(self.scan().await);
                return;
            }
            Ask::Forget { name, done } => (self.store.forget(&name).map(|_| ()), done),
            Ask::Trust {
                name,
                trusted,
                done,
            } => (self.store.trust(&name, trusted), done),
        };
        let answer = match changed {
            Ok(()) => self.given_everything().await,
            Err(failure) => Err(refused(failure)),
        };
        // Whoever asked may have stopped waiting, which changes nothing.
        let _ = done.send(answer);
    }

    /// Has every card wpa_supplicant drives look for networks now.
    async fn scan(&self) -> Result<(), Refused> {
        let failed = |error: zbus::Error| Refused::Failed {
            reason: format!("wpa_supplicant didn't list its cards: {error}"),
        };
        for card in self.supplicant.interfaces().await.map_err(failed)? {
            let looking = HashMap::from([("Type", zvariant::Value::from("active"))]);
            let asked = match interface(self.bus, card.clone()).await {
                Ok(interface) => interface.scan(looking).await,
                Err(error) => Err(error),
            };
            // A card already looking, or turned off, is refused, which
            // needs no more than a line for whoever is debugging.
            if let Err(error) = asked {
                println!("<7>{} didn't look for networks: {error}", card.as_str());
            }
        }
        Ok(())
    }

    /// Has networkd read its files again and gives wpa_supplicant every
    /// card's networks again, as the store now holds them.
    async fn given_everything(&self) -> Result<(), Refused> {
        let reloaded = reload_networkd(self.bus).await;
        self.add_cards().await;
        reloaded.map_err(|error| Refused::Failed {
            reason: format!("networkd didn't read its files again: {error}"),
        })
    }

    async fn joinable(&self, network: &OwnedObjectPath) -> zbus::Result<JoinableProxy<'_>> {
        JoinableProxy::builder(self.bus)
            .path(network.clone())?
            .cache_properties(CacheProperties::No)
            .build()
            .await
    }
}
