//! wifid joins Wi-Fi networks and finds the ones that want a sign-in.
//!
//! It starts by giving networkd a file for every network the launcher
//! saved, then gives wpa_supplicant every saved network on every card and
//! lets it choose which to join. It gives a card networkd finds later the
//! same, and every card again whenever wpa_supplicant starts again. A card
//! that already holds a network as wanted keeps it, so wifid starting
//! again never takes a card off the network it's on. A card that's been
//! looking for networks too long with a saved one in range is told to join
//! it (stall.rs).
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
mod detail;
mod link;
mod lookup;
mod network;
mod page;
mod radio;
mod rfkill;
mod secret;
mod seen;
mod service;
mod stall;
mod state;
mod store;
mod supplicant;
mod syntax;
mod vault;

use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::future;
use std::io::{self, Read, Write};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use futures_util::StreamExt;
use tokio::sync::{mpsc, watch};
use tokio::time::{Duration, Instant, MissedTickBehavior, sleep_until};
use zbus::fdo::{DBusProxy, NameOwnerChanged, PropertiesProxy};
use zbus::names::InterfaceName;
use zbus::proxy::CacheProperties;
use zbus::zvariant::{self, ObjectPath, OwnedObjectPath, OwnedValue};
use zbus::{Connection, MatchRule, MessageStream, proxy};
use zlink::Server;

use address::AddressKey;
use air::AccessPoint;
use card::Card;
use check::{Checker, Settings, Verdict};
use detail::Polled;
use link::Link;
use network::{Account, Address, Inner, Method, Network, Security};
use secret::{Credentials, Secrets};
use seen::Seen;
use service::{Ask, Refused, Stage, Wifi};
use stall::Stalls;
use state::{Progress, Reach, State, Trouble};
use store::{Saved, Store};
use supplicant::{InterfaceProxy, JoinableProxy, Step, SupplicantProxy};

/// Where wifid keeps what it must remember between starts.
const STATE: &str = "/var/lib/wifid";

/// What every card wpa_supplicant is given starts from.
const CARD_SETTINGS: &str = "/etc/wpa_supplicant/interface.conf";

/// The bits of the card's own address a look for networks keeps, laid over
/// it by the driver: only that the address is one card's, as the kernel
/// insists, and how it's given out. The rest is new for every look.
const LOOKING_MASK: [u8; 6] = [0x03, 0, 0, 0, 0, 0];

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

/// A stream read on a task of its own, so zbus always has room for what it
/// hears. zbus stops reading the bus while any stream it feeds is full, and
/// so stops handing back replies, so a loop waiting on a reply while its own
/// streams go unread would wait forever.
fn drained<S>(stream: S) -> mpsc::UnboundedReceiver<S::Item>
where
    S: futures_util::Stream + Send + 'static,
    S::Item: Send + 'static,
{
    let (sending, drained) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        let mut stream = std::pin::pin!(stream);
        while let Some(item) = stream.next().await {
            // Nobody reads any more once wifid is stopping.
            if sending.send(item).is_err() {
                return;
            }
        }
    });
    drained
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
    let secrets = Secrets::new(Credentials::system(), &Path::new(STATE).join("vault.cred"));
    let seen = RefCell::new(Seen::kept_in(&Path::new(STATE).join("seen")));

    let bus = Connection::system().await?;
    // networkd reads the files anyway when it next starts.
    if let Err(error) = reload_networkd(&bus).await {
        eprintln!("<4>can't ask networkd to read its files again: {error}");
    }

    let owners = DBusProxy::new(&bus).await?;
    let changes = drained(owners.receive_name_owner_changed().await?);
    let supplicant = SupplicantProxy::new(&bus).await?;
    let state = watch::Sender::new(State::default());
    let wifid = Wifid {
        bus: &bus,
        supplicant: &supplicant,
        store: &store,
        key: &key,
        secrets: &secrets,
        seen: &seen,
        state: &state,
    };
    wifid.add_cards().await;

    // Followed before the access points are first read, so none heard
    // meanwhile is missed.
    let rule = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .path_namespace("/fi/w1/wpa_supplicant1")?
        .build();
    let signals = drained(MessageStream::for_match_rule(rule, &bus, None).await?);
    let mut air = Air {
        points: HashMap::new(),
        state: state.clone(),
        cards: HashMap::new(),
        seen: &seen,
        stalls: Stalls::default(),
    };
    air.hear_all(&bus, &supplicant).await;

    let rule = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface("org.freedesktop.DBus.Properties")?
        .member("PropertiesChanged")?
        .path_namespace("/org/freedesktop/network1/link")?
        .build();
    let links = drained(MessageStream::for_match_rule(rule, &bus, None).await?);
    let sleeps = drained(
        LoginProxy::new(&bus)
            .await?
            .receive_prepare_for_sleep()
            .await?,
    );
    let mut networks = Networks {
        checker: Arc::new(Checker {
            settings: settings(),
            tls: trusted(),
        }),
        asked: watch::Sender::new(0),
        state: state.clone(),
        followed: HashMap::new(),
    };
    let (asks, taken) = mpsc::channel(1);
    let watching = watch::Sender::new(0);
    let wifi = Wifi {
        state: state.subscribe(),
        asked: networks.asked.clone(),
        asks,
        watching: watching.clone(),
    };
    // Every card there is now was given wpa_supplicant above.
    networks.follow_all(all_links(&bus).await);
    let heard = Heard {
        owners: changes,
        signals,
        links,
        sleeps,
        asks: taken,
        watching: watching.subscribe(),
    };
    // The vault's key is the one thing opened through the TPM, however
    // many networks are saved, and it's opened as wifid starts, beside
    // everything else.
    let following = async {
        let following = follow(&wifid, air, networks, heard);
        let ((), followed) = tokio::join!(secrets.ready(&store), following);
        followed
    };
    tokio::select! {
        result = following => result,
        result = answer(wifi) => result,
    }
}

/// What wifid hears, each read on a task of its own, and what the shell
/// asks of it.
struct Heard {
    /// Names on the bus changing hands, as wpa_supplicant or networkd
    /// starts again.
    owners: mpsc::UnboundedReceiver<NameOwnerChanged>,
    /// Every signal of wpa_supplicant's.
    signals: mpsc::UnboundedReceiver<zbus::Result<zbus::Message>>,
    /// networkd's links changing.
    links: mpsc::UnboundedReceiver<zbus::Result<zbus::Message>>,
    /// The machine going to sleep and waking.
    sleeps: mpsc::UnboundedReceiver<PrepareForSleep>,
    /// What only wifid's main loop can do for the shell.
    asks: mpsc::Receiver<Ask>,
    /// How many are asking for a network in detail.
    watching: watch::Receiver<u32>,
}

/// Follows everything wifid hears and is asked until there's nothing left
/// to hear: the cards, their networks, the radios and the shell's joins.
async fn follow(
    wifid: &Wifid<'_>,
    mut air: Air<'_>,
    mut networks: Networks,
    heard: Heard,
) -> Result<(), Box<dyn Error>> {
    let Heard {
        mut owners,
        mut signals,
        mut links,
        mut sleeps,
        mut asks,
        mut watching,
    } = heard;
    let (bus, state) = (wifid.bus, wifid.state);
    let mut joining: Option<Joining> = None;
    let mut watched = state.subscribe();
    let mut switched = switches();
    let mut radios = rfkill::Switches::default();
    let mut polling = tokio::time::interval(POLLING);
    polling.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        let deadline = joining.as_ref().map(|joining| joining.deadline);
        let stalled = air.stalls.due();
        tokio::select! {
            Some(change) = owners.recv() => {
                wifid.owner_changed(&change, &mut air, &mut networks).await;
            }
            Some(Ok(message)) = links.recv() => wifid.link_changed(&message, &mut networks).await,
            Some(Ok(message)) = signals.recv() => air.take_in(bus, &message).await,
            Some(event) = switched.recv() => {
                let was = radios.on();
                radios.take_in(event);
                change(state, |state| state.radio(radios.on()));
                // The kernel takes every card's link down as its radio
                // goes off, and wpa_supplicant brings them back up only
                // when it can trace every Wi-Fi switch to its radio, so
                // networkd sets each up again however the radios came
                // back on.
                if !was && radios.on() {
                    for card in &all_links(bus).await.cards {
                        if let Err(error) = reconfigure(bus, card.index).await {
                            eprintln!("<4>can't have networkd bring {} up: {error}", card.name);
                        }
                    }
                }
            }
            Some(ask) = asks.recv() => wifid.take(ask, &mut joining, &networks, &air).await,
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
            // A join the shell asked for is followed on its own.
            () = until(stalled), if stalled.is_some() => {
                for (card, looked) in air.stalls.nudge(Instant::now()) {
                    if joining.is_none() {
                        wifid.nudge(&card, looked, &air).await;
                    }
                }
            }
            _ = polling.tick(), if *watching.borrow() > 0 => {
                poll(bus, wifid.supplicant, state).await;
            }
            Ok(()) = watching.changed() => {
                if *watching.borrow_and_update() == 0 {
                    change(state, State::unpolled);
                } else {
                    polling.reset_immediately();
                }
            }
            Some(sleep) = sleeps.recv() => {
                if sleep.args().is_ok_and(|args| !args.start) {
                    networks.asked.send_modify(|asked| *asked += 1);
                }
            }
            else => return Ok(()),
        }
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
            eprintln!("<4>{CERTIFICATES}: {error}; only the plain check will run");
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

    /// The DNS servers the network of the card of that name handed out,
    /// unless wifid follows no such card.
    fn servers(&self, card: &str) -> Option<Vec<IpAddr>> {
        let links = self.followed.values().map(|followed| followed.borrow());
        links
            .filter(|link| link.name == card)
            .map(|link| link.dns.clone())
            .next()
    }

    /// Follows these, every Wi-Fi card there is, and stops following any
    /// other, whose task ends with it. Gives the names of the cards new to
    /// wifid.
    fn follow_all(&mut self, links: Links) -> Vec<String> {
        let Links {
            cards: links,
            wired,
        } = links;
        change(&self.state, |state| state.wired(wired));
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

/// A name's addresses from the first of the network's servers to answer.
async fn look_up(
    card: &str,
    servers: Option<Vec<IpAddr>>,
    name: &str,
) -> Result<Vec<IpAddr>, Refused> {
    let servers = servers.ok_or(Refused::NotJoined)?;
    let mut reason = format!("{card}'s network handed out no DNS servers");
    for server in servers {
        match lookup::addresses(card, server, name).await {
            Ok(found) => return Ok(found),
            Err(error) => reason = error.to_string(),
        }
    }
    Err(Refused::Failed { reason })
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
    /// How many attempts the card had made as the join began, so trouble
    /// from those is never taken for its own.
    since: u64,
}

/// What's asked of a network to keep: anything not given stays as it was,
/// or as a new network starts.
struct Wanted {
    name: String,
    security: Option<Security>,
    password: Option<String>,
    hidden: Option<bool>,
    priority: Option<i32>,
    address: Option<Address>,
    account: Option<Account>,
}

/// How wifid joins a network said to be joined so.
fn joined_by(security: state::Security) -> Security {
    match security {
        state::Security::Open => Security::Open,
        state::Security::Personal => Security::Personal,
        state::Security::Enterprise => Security::Enterprise,
    }
}

/// An account as the shell gives it: the machine's authorities prove the
/// server, and the password goes through the tunnel as the method usually
/// sends it unless the shell says otherwise.
fn given(account: service::Account) -> Account {
    let method = match account.method {
        service::Method::Peap => Method::Peap,
        service::Method::Ttls => Method::Ttls,
    };
    let inner = match (account.inner, method) {
        (Some(service::Inner::Mschapv2), _) | (None, Method::Peap) => Inner::Mschapv2,
        (Some(service::Inner::Pap), _) | (None, Method::Ttls) => Inner::Pap,
    };
    Account {
        method,
        identity: account.identity,
        anonymous_identity: account.anonymous_identity,
        server_domain: account.server_domain,
        authority: None,
        inner,
    }
}

/// The access points every card hears, kept as wpa_supplicant last
/// described each.
struct Air<'a> {
    /// By the access point's path.
    points: HashMap<String, AccessPoint>,
    state: watch::Sender<State>,
    /// Each card's name, and how far wpa_supplicant last said it had got
    /// joining a network, by its path.
    cards: HashMap<String, (String, String)>,
    seen: &'a RefCell<Seen>,
    /// Which cards have been looking for networks too long.
    stalls: Stalls,
}

impl Air<'_> {
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
            // What a card heard is said before it says it's done looking.
            "ScanDone" => {
                let Some(card) = self.named(bus, path).await else {
                    return;
                };
                if body.deserialize::<bool>().unwrap_or(false) {
                    println!("<7>{card} is done looking");
                    change(&self.state, |state| state.looked(&card));
                } else {
                    println!("<7>{card}'s look failed");
                }
            }
            // How an enterprise network's join went: the server proving
            // itself, then the account taken or not.
            "EAP" => {
                let Ok((status, parameter)) = body.deserialize::<(String, String)>() else {
                    return;
                };
                let trouble = match (status.as_str(), parameter.as_str()) {
                    ("remote certificate verification", "success") => return,
                    ("remote certificate verification", _) => Trouble::ServerUnproven,
                    ("completion", "failure") => Trouble::WrongPassword,
                    _ => return,
                };
                self.troubled(bus, path, trouble).await;
            }
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
                        self.card_changed(bus, path, &changed).await;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    /// Takes in what wpa_supplicant says changed of the card at this path:
    /// how far it's got joining a network, the access point it's at, and
    /// any status that ends its join.
    async fn card_changed(
        &mut self,
        bus: &Connection,
        path: &ObjectPath<'_>,
        changed: &HashMap<String, OwnedValue>,
    ) {
        // An attempt at a network begins as the card starts authenticating
        // with it, or associating where it skips that.
        let step = changed.get("State").map(|step| &**step);
        if let Some(zvariant::Value::Str(step)) = step {
            let step = step.to_string();
            if let Some(card) = self.named(bus, path).await {
                let before = self.cards.get(path.as_str());
                let before = before.map_or("", |(_, step)| step.as_str());
                let begins = step == "authenticating"
                    || (step == "associating" && before != "authenticating");
                if begins {
                    change(&self.state, |state| state.attempting(&card));
                }
                if step == "completed" && before != "completed" {
                    self.joined(bus, path).await;
                }
                // Each step said, so a join that dawdles shows where.
                if step != before {
                    println!("<7>{card} is {step}");
                }
                self.stalls.step(&card, &step, Instant::now());
                self.cards.insert(path.to_string(), (card, step));
            }
        }
        let point = changed.get("CurrentBSS").map(|point| &**point);
        if let Some(zvariant::Value::ObjectPath(point)) = point {
            let point = joined_to(point);
            if let Some(card) = self.named(bus, path).await {
                change(&self.state, |state| state.at(&card, point));
            }
        }
        let steps = [
            ("AuthStatusCode", Step::Authentication),
            ("AssocStatusCode", Step::Association),
        ];
        for (property, step) in steps {
            let code = changed.get(property).map(|code| &**code);
            let Some(&zvariant::Value::I32(code)) = code else {
                continue;
            };
            // Only a status that ends a join is trouble, and the network's
            // kind only matters to WPA3.
            let said = supplicant::Trouble::from_status(step, code, Security::Personal);
            let trouble = match said {
                Some(supplicant::Trouble::WrongPassword) => Trouble::WrongPassword,
                Some(supplicant::Trouble::Refused(_)) => Trouble::TurnedAway,
                None => continue,
            };
            self.troubled(bus, path, trouble).await;
        }
    }

    /// Says what went wrong on the card wpa_supplicant has at this path.
    async fn troubled(&mut self, bus: &Connection, card: &ObjectPath<'_>, trouble: Trouble) {
        if let Some(name) = self.named(bus, card).await {
            change(&self.state, |state| state.troubled(&name, Some(trouble)));
        }
    }

    /// Notes the channel the card joined its network on, so a card looks
    /// there first when it next has no network.
    async fn joined(&self, bus: &Connection, card: &ObjectPath<'_>) {
        let point = match interface(bus, card.to_owned().into()).await {
            Ok(interface) => interface.current_access_point().await,
            Err(error) => Err(error),
        };
        let Some(point) = point.ok().and_then(|point| self.points.get(point.as_str())) else {
            return;
        };
        let (Some(heard), Some(frequency)) = (point.heard(), point.frequency()) else {
            return;
        };
        if let Err(error) = self.seen.borrow_mut().joined(&heard.name, frequency) {
            eprintln!("<4>can't keep where {} was joined: {error}", heard.name);
        }
    }

    /// The name of the card wpa_supplicant has at this path.
    async fn named(&self, bus: &Connection, card: &ObjectPath<'_>) -> Option<String> {
        if let Some((name, _)) = self.cards.get(card.as_str()) {
            return Some(name.clone());
        }
        let named = match interface(bus, card.to_owned().into()).await {
            Ok(interface) => interface.ifname().await,
            Err(error) => Err(error),
        };
        named
            .inspect_err(|error| eprintln!("<4>can't tell which card {card} is: {error}"))
            .ok()
    }

    fn heard(&mut self, path: &str, properties: HashMap<String, OwnedValue>) {
        let point = AccessPoint::new(properties);
        let heard = point.heard();
        self.points.insert(path.into(), point);
        change(&self.state, |state| state.heard(path, heard));
    }

    /// The saved network of these the card hears most strongly, if it hears
    /// any.
    fn best_heard(&self, card: &str, saved: &[&str]) -> Option<String> {
        let (at, _) = self.cards.iter().find(|(_, (named, _))| named == card)?;
        let under = format!("{at}/BSSs/");
        let heard = self
            .points
            .iter()
            .filter(|(path, _)| path.starts_with(&under));
        let heard = heard.filter_map(|(_, point)| point.heard());
        let heard = heard.filter(|heard| saved.contains(&heard.name.as_str()));
        heard
            .max_by_key(|heard| heard.signal)
            .map(|heard| heard.name)
    }

    /// Forgets every access point whose path starts so.
    fn gone_under(&mut self, path: &str) {
        for (card, (name, _)) in &self.cards {
            if card.starts_with(path) {
                self.stalls.gone(name);
            }
        }
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

/// The access point a card is joined to, from wpa_supplicant's path for
/// it, which is the root while it's joined to none.
fn joined_to(point: &ObjectPath<'_>) -> Option<String> {
    (point.as_str() != "/").then(|| point.to_string())
}

/// Turns every Wi-Fi radio on or off.
fn radio(on: bool) -> Result<(), Refused> {
    let turned = OpenOptions::new()
        .write(true)
        .open(rfkill::DEVICE)
        .and_then(|mut device| device.write_all(&rfkill::turning(on)));
    turned.map_err(|error| Refused::Failed {
        reason: format!("{}: {error}", rfkill::DEVICE),
    })
}

/// Every event of the radios' switches, read on a thread of its own, since
/// the device is read by waiting on it. Without the device, Wi-Fi is shown
/// on, as nothing could turn it off.
fn switches() -> mpsc::UnboundedReceiver<rfkill::Event> {
    let (sender, events) = mpsc::unbounded_channel();
    std::thread::spawn(move || {
        let mut device = match fs::File::open(rfkill::DEVICE) {
            Ok(device) => device,
            Err(error) => return eprintln!("<4>{}: {error}", rfkill::DEVICE),
        };
        let mut event = [0; 8];
        // Each read gives one event, as much of it as was asked for.
        while device.read_exact(&mut event).is_ok() && sender.send(event).is_ok() {}
    });
    events
}

/// What the shell is told of a change to the saved networks failing.
fn refused(failure: store::Failure) -> Refused {
    match failure {
        store::Failure::Declared => Refused::Declared,
        store::Failure::NotSaved => Refused::NotSaved,
        store::Failure::Io(error) => Refused::Failed {
            reason: error.to_string(),
        },
    }
}

/// How often a card on a network reads its link while someone asks for a
/// network in detail.
const POLLING: Duration = Duration::from_secs(2);

/// Reads the link of each card on a network. A card wpa_supplicant can't
/// answer for in a second is left unread rather than holding wifid up.
async fn poll(bus: &Connection, supplicant: &SupplicantProxy<'_>, state: &watch::Sender<State>) {
    let cards = state.borrow().on_networks();
    for card in cards {
        let reading = async {
            let interface = interface(bus, supplicant.get_interface(&card).await?).await?;
            let point = interface.current_access_point().await?;
            let said = interface.signal_poll().await?;
            let said = match &*said {
                zvariant::Value::Value(inside) => inside.try_to_owned()?,
                _ => said,
            };
            let said: HashMap<String, OwnedValue> = said.try_into()?;
            zbus::Result::Ok(Polled::read(point.to_string(), &said))
        };
        let polled = match tokio::time::timeout(Duration::from_secs(1), reading).await {
            Ok(Ok(polled)) => Some(polled),
            Ok(Err(error)) => {
                eprintln!("<7>can't read {card}'s link: {error}");
                None
            }
            Err(_) => {
                eprintln!("<7>can't read {card}'s link: wpa_supplicant didn't answer in time");
                None
            }
        };
        change(state, |state| state.polled(&card, polled));
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
    let mut last_standing = None;
    let mut said = None;
    let mut due = None;
    loop {
        let now = link.borrow_and_update().clone();
        if now.routable {
            let standing = (now.ssid.clone(), now.dns.clone(), now.portal.clone());
            let reached = last_standing.as_ref() != Some(&standing);
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
                last_standing = Some(standing);
                due = Some(Instant::now() + after);
            }
        } else {
            (last_standing, said, due) = (None, None, None);
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

/// What networkd describes: every Wi-Fi card's link, and whether a cable
/// reaches beyond the machine.
struct Links {
    cards: Vec<Link>,
    wired: bool,
}

async fn all_links(bus: &Connection) -> Links {
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
        return Links {
            cards: Vec::new(),
            wired: false,
        };
    };
    let links = described["Interfaces"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    Links {
        cards: links.iter().filter_map(Link::described).collect(),
        wired: link::wired(links),
    }
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

/// Has networkd set a link up again, as its files say.
async fn reconfigure(bus: &Connection, index: i32) -> zbus::Result<()> {
    bus.call_method(
        Some("org.freedesktop.network1"),
        "/org/freedesktop/network1",
        Some("org.freedesktop.network1.Manager"),
        "ReconfigureLink",
        &(index,),
    )
    .await
    .map(|_| ())
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
    secrets: &'a Secrets,
    seen: &'a RefCell<Seen>,
    state: &'a watch::Sender<State>,
}

impl Wifid<'_> {
    /// Gives wpa_supplicant every card, each with every saved network. A card
    /// or network that can't be added is said in the journal and passed over,
    /// so one bad file never keeps the machine off every network.
    async fn add_cards(&self) {
        self.add(|_| true).await;
    }

    /// Gives wpa_supplicant the card of that name, as `add_cards` does.
    async fn add_card_named(&self, name: &str) {
        self.add(|card| card.name == name).await;
    }

    /// Each card is handed over and listening first, and given its
    /// networks once their passwords will open, so a card that appears as
    /// wifid starts is heard from while the vault's key is opened.
    async fn add(&self, wanted: impl Fn(&Card) -> bool) {
        let cards = match card::all(Path::new("/sys")) {
            Ok(cards) => cards,
            Err(error) => return eprintln!("<3>can't list the Wi-Fi cards: {error}"),
        };
        let mut handed = Vec::new();
        for card in cards.into_iter().filter(wanted) {
            match self.hand_over(&card).await {
                Ok(interface) => handed.push((card, interface)),
                Err(error) => eprintln!("<3>can't add {}: {error}", card.name),
            }
        }
        // With no card to give them to, the networks are read for the
        // shell without waiting for the vault.
        if !handed.is_empty() {
            self.secrets.ready(self.store).await;
        }
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
        let names = networks.iter().map(|saved| saved.network.name.as_str());
        if let Err(error) = self.seen.borrow_mut().keep_only(names) {
            eprintln!("<4>can't forget where forgotten networks were joined: {error}");
        }
        for (card, interface) in handed {
            if let Err(error) = self.give_networks(&card, &interface, &networks).await {
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
        let interface = self.hand_over(card).await?;
        self.secrets.ready(self.store).await;
        self.give_networks(card, &interface, networks).await
    }

    /// Gives wpa_supplicant the card, if it hasn't it already, and has a
    /// card on no network look for one, so the networks given it a moment
    /// later are joined from what it heard rather than after another look.
    /// Changes none of the networks it holds, so a card on one stays.
    async fn hand_over(&self, card: &Card) -> zbus::Result<InterfaceProxy<'_>> {
        let supplicant = self.supplicant;
        let path = if let Ok(path) = supplicant.get_interface(&card.name).await {
            path
        } else {
            let args = HashMap::from([
                ("Ifname", zvariant::Value::from(card.name.as_str())),
                ("ConfigFile", zvariant::Value::from(CARD_SETTINGS)),
            ]);
            supplicant.create_interface(args).await?
        };
        let interface = interface(self.bus, path).await?;
        // A card kept on its network as wifid restarts says nothing of the
        // access point it's at until it moves, so it's asked.
        let point = interface.current_access_point().await.ok();
        let point = point.and_then(|point| joined_to(&point));
        change(self.state, |state| state.at(&card.name, point));
        // The driver makes up each address, so the card's own link stays up
        // as it looks, as it wouldn't were its address changed for a look.
        let masks = HashMap::from([("scan", LOOKING_MASK.to_vec())]);
        if let Err(error) = interface.set_looking_masks(masks).await {
            eprintln!(
                "<4>{} looks for networks from its own address: {error}",
                card.name
            );
        }
        if matches!(
            interface.state().await?.as_str(),
            "disconnected" | "inactive"
        ) {
            let mut looking = HashMap::from([("Type", zvariant::Value::from("active"))]);
            // Only where a saved network was joined: should none answer,
            // wpa_supplicant looks everywhere as each network is given it.
            let channels = self.seen.borrow().channels();
            if !channels.is_empty() {
                let channels: Vec<(u32, u32)> = channels
                    .into_iter()
                    .map(|frequency| (frequency, 20))
                    .collect();
                looking.insert("Channels", zvariant::Value::from(channels));
            }
            // A card already looking, or whose radio is off, is refused,
            // and either way it looks once its networks are given it.
            if let Err(error) = interface.scan(looking).await {
                println!("<7>{} didn't look for networks: {error}", card.name);
            }
        }
        Ok(interface)
    }

    /// Gives the card every saved network it hasn't, takes away any it holds
    /// that aren't, and says where it holds each, by name.
    async fn give_networks(
        &self,
        card: &Card,
        interface: &InterfaceProxy<'_>,
        networks: &[Saved],
    ) -> zbus::Result<Vec<(String, OwnedObjectPath)>> {
        let mut added: Vec<&str> = Vec::new();
        let mut wanted = Vec::new();
        for saved in networks {
            let name = saved.network.name.as_str();
            // Nix's networks come first, and win over the launcher's of the
            // same name.
            if added.contains(&name) {
                continue;
            }
            let password = match self.secrets.password(&saved.network).await {
                Ok(password) => password,
                Err(error) => {
                    eprintln!("<4>{name}: can't read its password: {error}");
                    continue;
                }
            };
            let address = self.key.address(card.address, name);
            let password = password.as_deref().map(String::as_str);
            let fields = match supplicant::properties(&saved.network, password, address) {
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

    /// Starts over with wpa_supplicant or networkd as either starts again.
    async fn owner_changed(
        &self,
        change: &NameOwnerChanged,
        air: &mut Air<'_>,
        networks: &mut Networks,
    ) {
        let Ok(args) = change.args() else { return };
        if args.new_owner().is_none() {
            return;
        }
        match args.name().as_str() {
            // It starts again hearing nothing, and says each access point
            // as it hears it.
            "fi.w1.wpa_supplicant1" => {
                air.gone_under("");
                self.add_cards().await;
            }
            "org.freedesktop.network1" => {
                for card in networks.follow_all(all_links(self.bus).await) {
                    self.add_card_named(&card).await;
                }
            }
            _ => {}
        }
    }

    /// Follows the link networkd says changed, giving wpa_supplicant its
    /// card when it's new to wifid.
    async fn link_changed(&self, message: &zbus::Message, networks: &mut Networks) {
        let header = message.header();
        let Some(path) = header.path() else { return };
        match one_link(self.bus, path).await {
            Some(link) => {
                let card = link.name.clone();
                if networks.follow(link) {
                    self.add_card_named(&card).await;
                }
            }
            // Not a card, or one going: whichever, networkd's own list says
            // which cards are left.
            None => {
                for card in networks.follow_all(all_links(self.bus).await) {
                    self.add_card_named(&card).await;
                }
            }
        }
    }

    /// Does what the shell asked of wifid's main loop. A join asked for
    /// takes the place of the one before it, if that's still going.
    async fn take(
        &self,
        ask: Ask,
        joining: &mut Option<Joining>,
        networks: &Networks,
        air: &Air<'_>,
    ) {
        match ask {
            Ask::Join {
                name,
                password,
                account,
                progress,
            } => {
                if let Some(before) = joining.take() {
                    let instead = Refused::Failed {
                        reason: format!("joining {name} instead"),
                    };
                    self.settle(before, Err(instead)).await;
                }
                let account = account.map(given);
                *joining = self.join(name, password, account, progress, air).await;
            }
            // A server slow to answer mustn't hold anything else up.
            Ask::Lookup { card, name, done } => {
                let servers = networks.servers(&card);
                tokio::spawn(async move {
                    let found = look_up(&card, servers, &name).await;
                    if let Err(refused) = &found {
                        eprintln!("<4>can't look {name} up through {card}: {refused:?}");
                    }
                    let _ = done.send(found);
                });
            }
            ask => self.answer(ask).await,
        }
    }

    /// Starts joining the network as the shell asked, saving it first when
    /// it's new or given a password, and says so through progress. Gives
    /// back the join to follow, or nothing once it's said why it couldn't.
    async fn join(
        &self,
        name: String,
        password: Option<String>,
        account: Option<Account>,
        progress: mpsc::Sender<Result<Stage, Refused>>,
        air: &Air<'_>,
    ) -> Option<Joining> {
        match self.start(&name, password, account, air).await {
            Ok((card, new, since)) => {
                let joining = Joining {
                    name,
                    card,
                    new,
                    progress,
                    said: None,
                    deadline: Instant::now() + JOINING,
                    since,
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
        account: Option<Account>,
        air: &Air<'_>,
    ) -> Result<(String, bool, u64), Refused> {
        let failed = |reason: String| Refused::Failed { reason };
        let listing = self
            .store
            .networks()
            .map_err(|error| failed(error.to_string()))?;
        let saved = listing.saved.iter().any(|saved| saved.network.name == name);
        // A network not saved is joined the way the air says it's joined.
        let security = if saved {
            None
        } else {
            let heard = self.state.borrow().described().nearby;
            let heard = heard.into_iter().find(|nearby| nearby.name == name);
            Some(joined_by(heard.ok_or(Refused::NotNearby)?.security))
        };
        let new = self
            .keep(Wanted {
                name: name.to_string(),
                security,
                password,
                hidden: None,
                priority: None,
                address: None,
                account,
            })
            .await?;

        let card = self.card_for(name, air).await?;
        let since = self.state.borrow().attempts(&card.name);
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
        Ok((card.name, new, since))
    }

    /// Keeps the network as asked, sealing a password it's given beside its
    /// file, and gives networkd and every card the networks as they now
    /// stand. Anything not asked for stays as it was, or as a new network
    /// starts. Says whether the network is new.
    async fn keep(&self, wanted: Wanted) -> Result<bool, Refused> {
        let failed = |reason: String| Refused::Failed { reason };
        // The networks are read once the vault's open, and any password
        // moved into it, so a password is sealed there and nowhere else.
        self.secrets.ready(self.store).await;
        let listing = self
            .store
            .networks()
            .map_err(|error| failed(error.to_string()))?;
        let saved = listing
            .saved
            .into_iter()
            .find(|saved| saved.network.name == wanted.name);
        let declared = saved.as_ref().is_some_and(|saved| saved.declared);
        let new = saved.is_none();
        let mut network = match saved {
            Some(saved) => saved.network,
            None => Network {
                name: wanted.name.clone(),
                security: wanted.security.ok_or(Refused::NotNearby)?,
                secret: None,
                priority: 0,
                hidden: false,
                address: Address::Network,
                trusted: false,
                account: None,
            },
        };
        if let Some(security) = wanted.security {
            network.security = security;
        }
        // An open network has no use for a password it's given, nor any but
        // an enterprise network for an account.
        let password = wanted
            .password
            .filter(|_| network.security != Security::Open);
        let account = wanted
            .account
            .filter(|_| network.security == Security::Enterprise);
        let changing = wanted.security.is_some()
            || password.is_some()
            || account.is_some()
            || wanted.hidden.is_some()
            || wanted.priority.is_some()
            || wanted.address.is_some();
        if declared && changing {
            return Err(Refused::Declared);
        }
        network.hidden = wanted.hidden.unwrap_or(network.hidden);
        network.priority = wanted.priority.unwrap_or(network.priority);
        network.address = wanted.address.unwrap_or(network.address);
        if network.security == Security::Enterprise {
            network.account = account.or(network.account);
            if network.account.is_none() {
                return Err(Refused::NeedsAccount);
            }
        } else {
            network.account = None;
        }
        let before = network.secret.clone();
        match (network.security, password) {
            (Security::Personal | Security::Enterprise, Some(password)) => {
                // Only WPA2's and WPA3's passwords have a shape to keep to.
                if network.security == Security::Personal {
                    supplicant::psk(&password).map_err(|unjoinable| failed(unjoinable.0.into()))?;
                }
                let file = self.store.password_file(&network.name).map_err(refused)?;
                let sealed_as = file.file_stem().unwrap_or_default().to_string_lossy();
                let sealed = self.secrets.seal(&sealed_as, &password);
                let sealed =
                    sealed.map_err(|error| failed(format!("can't seal the password: {error}")))?;
                Store::keep_password(&file, &sealed).map_err(|error| failed(error.to_string()))?;
                network.secret = Some(file);
            }
            (Security::Personal | Security::Enterprise, None) if network.secret.is_none() => {
                return Err(Refused::NeedsPassword);
            }
            (Security::Open, _) => network.secret = None,
            _ => {}
        }
        if new || changing {
            self.store.save(&network).map_err(refused)?;
        }
        // A password the network no longer names, since it's open now or
        // its password is kept another way, is taken away.
        if let Some(before) = before.filter(|before| network.secret.as_ref() != Some(before))
            && let Err(error) = self.store.let_go_of_password(&before)
        {
            let name = &network.name;
            eprintln!("<4>{name}: can't take away its old password: {error}");
        }
        self.given_everything().await?;
        Ok(new)
    }

    /// The card hearing the network most strongly, or the first there is
    /// when none hears it, as when it hides its name.
    async fn card_for(&self, name: &str, air: &Air<'_>) -> Result<Card, Refused> {
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

    /// Has a card that's been looking for networks too long, with a saved
    /// one in range, join what it can: wpa_supplicant is asked to join as a
    /// join from the launcher asks, which got a card on a laptop going again
    /// once Joel joined by hand, from what it heard last if that's fresh and
    /// from a new look if not. It picks among every saved network, all turned on
    /// first, rather than this naming one: a network still heard of may
    /// have gone, and joining only it would hold the card there.
    async fn nudge(&self, name: &str, looked: Duration, air: &Air<'_>) {
        let listing = match self.store.networks() {
            Ok(listing) => listing,
            Err(error) => return eprintln!("<4>can't list the saved networks: {error}"),
        };
        let names: Vec<&str> = listing
            .saved
            .iter()
            .map(|saved| saved.network.name.as_str())
            .collect();
        let Some(network) = air.best_heard(name, &names) else {
            return;
        };
        let cards = card::all(Path::new("/sys")).unwrap_or_default();
        let Some(card) = cards.into_iter().find(|card| card.name == name) else {
            return;
        };
        if let Err(error) = self.add_card(&card, &listing.saved).await {
            return eprintln!("<4>can't give {name} its networks: {error}");
        }
        let asked = match self.supplicant.get_interface(name).await {
            Ok(at) => match interface(self.bus, at).await {
                Ok(interface) => interface.reassociate().await,
                Err(error) => Err(error),
            },
            Err(error) => Err(error),
        };
        match asked {
            Ok(()) => println!(
                "<5>{name} looked for networks for {} s with {network} in range; asking it to join",
                looked.as_secs()
            ),
            Err(error) => eprintln!("<4>{name} wasn't asked to join: {error}"),
        }
    }

    /// Says how far the join has got, if that's news, and settles it once
    /// the card is done. Gives back a join still going.
    async fn advance(&self, mut joining: Joining) -> Option<Joining> {
        let progress = (self.state.borrow()).progress(&joining.card, &joining.name, joining.since);
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
            Progress::Troubled(Trouble::ServerUnproven) => {
                self.settle(joining, Err(Refused::ServerUnproven)).await;
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
            Ask::Join { .. } | Ask::Lookup { .. } => {
                unreachable!("the main loop answers joins and lookups itself")
            }
            Ask::Scan { done } => {
                let _ = done.send(self.scan().await);
                return;
            }
            Ask::Leave { name, done } => {
                let _ = done.send(self.leave(&name).await);
                return;
            }
            Ask::Save { saving, done } => {
                let wanted = Wanted {
                    name: saving.name,
                    security: Some(joined_by(saving.security)),
                    password: saving.password,
                    hidden: saving.hidden,
                    priority: saving.priority,
                    address: saving.address.map(|address| match address {
                        service::Address::Network => Address::Network,
                        service::Address::Card => Address::Card,
                        service::Address::Random => Address::Random,
                    }),
                    account: saving.account.map(given),
                };
                let _ = done.send(self.keep(wanted).await.map(|_| ()));
                return;
            }
            Ask::Radio { on, done } => {
                let _ = done.send(radio(on));
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

    /// Takes every card on the network off it.
    async fn leave(&self, name: &str) -> Result<(), Refused> {
        let cards: Vec<String> = self
            .state
            .borrow()
            .described()
            .cards
            .into_iter()
            .filter(|card| card.network.as_deref() == Some(name))
            .map(|card| card.name)
            .collect();
        if cards.is_empty() {
            return Err(Refused::NotJoined);
        }
        for card in cards {
            let left = match self.supplicant.get_interface(&card).await {
                Ok(at) => match interface(self.bus, at).await {
                    Ok(interface) => interface.disconnect().await,
                    Err(error) => Err(error),
                },
                Err(error) => Err(error),
            };
            left.map_err(|error| Refused::Failed {
                reason: format!("{card} didn't leave {name}: {error}"),
            })?;
            println!("<6>{card} left {name}, as asked");
        }
        Ok(())
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
            // A card already looking is refused, and the shell hears what
            // that look finds anyway, as often as every ten seconds, so
            // only something else is worth a line for whoever is debugging.
            match asked {
                Err(zbus::Error::MethodError(name, _, _))
                    if name.as_str() == "fi.w1.wpa_supplicant1.Interface.ScanError" => {}
                Err(error) => println!("<7>{} didn't look for networks: {error}", card.as_str()),
                Ok(()) => {}
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

#[cfg(test)]
mod tests {
    use super::*;

    // zbus hands a stream each message only once the one before has been
    // taken, as a channel with room for one does, so nothing it says after
    // that gets through, replies included, until somebody reads.
    #[tokio::test]
    async fn a_drained_stream_is_read_while_nobody_reads_what_it_gave() {
        let (feeding, fed) = mpsc::channel(1);
        let stream = futures_util::stream::unfold(fed, |mut fed| async move {
            fed.recv().await.map(|said| (said, fed))
        });
        let mut drained = drained(stream);

        let feeder = async {
            for said in 0..100 {
                feeding.send(said).await.unwrap();
            }
        };
        let kept_up = tokio::time::timeout(Duration::from_secs(5), feeder).await;

        assert!(kept_up.is_ok(), "the stream stopped taking messages");
        drop(feeding);
        let mut heard = Vec::new();
        while let Some(said) = drained.recv().await {
            heard.push(said);
        }
        assert_eq!(heard, (0..100).collect::<Vec<_>>());
    }
}
