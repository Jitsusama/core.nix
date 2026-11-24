//! wifid joins Wi-Fi networks and finds the ones that want a sign-in.
//!
//! It starts by giving networkd a file for every network the launcher
//! saved, then gives wpa_supplicant every saved network on every card and
//! lets it choose which to join. It gives a card networkd finds later the
//! same, and every card again whenever wpa_supplicant starts again. A card
//! that already holds a network as wanted keeps it, so wifid starting
//! again never takes a card off the network it's on. A card that's been
//! looking for networks too long with a saved one in range is told to join
//! it (stall.rs). A card moved between a network's access points is
//! followed until it's there, somewhere else or lost, and how long that took
//! said (arrive.rs).
//!
//! Once a card's network reaches beyond itself, as networkd sees it, wifid
//! checks whether it really does or wants a sign-in first. It checks again
//! whenever the card moves to another network or is handed other DNS
//! servers, as the machine wakes, when asked, and on a timer the verdict
//! sets.
//!
//! It answers the shell over varlink, as io.jitsusama.Wifi (service.rs),
//! with what it knows of each card, every saved network and the networks in
//! the air, which wpa_supplicant says as it hears them. It serves the
//! launcher its Wi-Fi rows ready, worded in the launcher's own vocabulary,
//! on a second socket (face.rs, rows.rs), so the shell only draws them.

mod acts;
mod address;
mod air;
mod arrive;
mod calling;
mod card;
mod check;
mod choose;
mod cli;
mod detail;
mod echo;
mod face;
mod frequencies;
mod learn;
mod link;
mod lookup;
mod measure;
mod moves;
mod network;
mod nl80211;
mod others;
mod page;
mod parts;
mod radio;
mod rfkill;
mod rows;
mod secret;
mod seen;
mod service;
mod stall;
mod state;
mod store;
mod supplicant;
mod syntax;
mod vault;
mod words;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
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
use zbus::names::{InterfaceName, WellKnownName};
use zbus::proxy::CacheProperties;
use zbus::zvariant::{self, ObjectPath, OwnedObjectPath, OwnedValue};
use zbus::{Connection, MatchRule, MessageStream, proxy};
use zlink::Server;

use address::AddressKey;
use air::AccessPoint;
use arrive::{Arrivals, Short, Unreached};
use card::Card;
use check::{Checker, Settings, Verdict};
use detail::Polled;
use face::Face;
use frequencies::Avoided;
use learn::Learned;
use link::Link;
use network::{Account, Address, Inner, Method, Network, Security};
use others::Others;
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

/// How many accounts that let wpa_supplicant's name go are passed over,
/// the newest kept: their queued signals drain within a tick or two.
const DEPARTED: usize = 4;

/// What wpa_supplicant answers a disconnect with for a card it isn't
/// joining or on (`dbus_new_handlers.c`, 2.12).
const NOT_CONNECTED: &str = "fi.w1.wpa_supplicant1.NotConnected";

/// wpa_supplicant's name on the bus.
const SUPPLICANT: WellKnownName<'static> =
    WellKnownName::from_static_str_unchecked("fi.w1.wpa_supplicant1");

/// Which pages the check asks.
const SETTINGS: &str = "/etc/wifid/wifid.conf";

/// The frequencies each network's cards are kept off (frequencies.rs).
const AVOIDED: &str = "/etc/wifid/avoid.json";

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

fn main() -> ExitCode {
    terminal_kit::main(&cli::help(), env!("CARGO_PKG_VERSION"), |context| {
        cli::words(&context.args)?;
        // The runtime #[tokio::main(flavor = "current_thread")] builds,
        // which the work main.rs spawns leans on.
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| stopped(&error))?
            .block_on(run())
            .map_err(|error| stopped(&*error))
    })
}

/// How a run that stopped ends: 1, saying why.
fn stopped(error: &dyn std::error::Error) -> terminal_kit::Failure {
    terminal_kit::Failure::new(terminal_kit::Exit::Failed, "wifid stopped")
        .because(error.to_string())
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
    // The vault's key is the one thing opened through the TPM, however
    // many networks are saved. The TPM takes a second or two over each
    // sealed secret and takes them in turns, so two asked at once both
    // finish late, and the login keyring asks for its own as the session
    // starts. So wifid asks the moment it starts, beside everything else it
    // sets up, rather than once it has a card to give networks to.
    let ((), served) = tokio::join!(secrets.ready(&store), serve(&store, &key, &secrets, &seen));
    served
}

/// The account holding wpa_supplicant's name, asked of the bus itself,
/// which starts nothing; none while it isn't running, or where the bus
/// can't say, which is said.
async fn supplicant_owner(owners: &DBusProxy<'_>) -> Option<String> {
    match owners.get_name_owner(SUPPLICANT.into()).await {
        Ok(owner) => Some(owner.to_string()),
        Err(zbus::fdo::Error::NameHasNoOwner(_)) => None,
        Err(error) => {
            eprintln!("<4>can't ask the bus who runs wpa_supplicant: {error}");
            None
        }
    }
}

/// Joins the bus, gives wpa_supplicant the cards and follows them, and
/// answers the shell, until there's nothing left to hear.
async fn serve(
    store: &Store,
    key: &AddressKey,
    secrets: &Secrets,
    seen: &RefCell<Seen>,
) -> Result<(), Box<dyn Error>> {
    let bus = Connection::system().await?;
    // networkd reads the files anyway when it next starts.
    if let Err(error) = reload_networkd(&bus).await {
        eprintln!("<4>can't ask networkd to read its files again: {error}");
    }

    let owners = DBusProxy::new(&bus).await?;
    let changes = drained(owners.receive_name_owner_changed().await?);
    let supplicant = SupplicantProxy::new(&bus).await?;
    let state = watch::Sender::new(State::default());
    // Whether Joel's on a call, as his session tells the rows socket.
    let calling = watch::Sender::<calling::Calling>::default();
    let avoided = avoided();
    let arrivals = RefCell::default();
    let wifid = Wifid {
        bus: &bus,
        supplicant: &supplicant,
        store,
        key,
        secrets,
        avoided: &avoided,
        seen,
        arrivals: &arrivals,
        state: &state,
        picked: RefCell::default(),
        held: RefCell::default(),
        astray: RefCell::default(),
        alone: RefCell::default(),
        stopped: RefCell::default(),
        leaving: RefCell::default(),
    };
    wifid.add_cards().await;

    // Followed before the access points are first read, so none heard
    // meanwhile is missed. Any account on the laptop may send a signal
    // under wpa_supplicant's path, as the system bus lets any broadcast
    // one through, so each is taken only from wpa_supplicant as it runs
    // now (`Air::take_in`), but for a card beginning to look, since the VM
    // test says a card is scanning so, no radio there ever hanging a look
    // (2026-11-20). That's taken only of a card holding no network and in
    // no move, so such an account can have a card it says has looked too
    // long told to join a network it saved, and nothing more: not one Joel
    // took off its network, which `nudge` leaves be until it joins again.
    let rule = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .path_namespace("/fi/w1/wpa_supplicant1")?
        .build();
    let signals = drained(MessageStream::for_match_rule(rule, &bus, None).await?);
    let owner = supplicant_owner(&owners).await;
    let mut air = Air::new(state.clone(), seen, &arrivals, owner);
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
    if let Err(error) = learn::let_go_of_before(Path::new(STATE)) {
        eprintln!("<4>can't let go of the hours learned before video was weighed: {error}");
    }
    // How well each access point carries, measured from where the machine
    // is, and the card moved to one carrying better (moves.rs), with what
    // each carried at each hour kept for a trusted network (learn.rs).
    tokio::spawn(measure::follow(
        state.subscribe(),
        calling.subscribe(),
        asks.clone(),
        Learned::kept_in(&Path::new(STATE).join(learn::FILE)),
    ));
    let watching = watch::Sender::new(0);
    let wifi = Wifi {
        state: state.subscribe(),
        asked: networks.asked.clone(),
        asks,
        watching: watching.clone(),
        calling,
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
    tokio::select! {
        result = follow(&wifid, air, networks, heard) => result,
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
    let mut kernel: Option<nl80211::Nl80211> = None;
    loop {
        let deadline = joining.as_ref().map(|joining| joining.deadline);
        let stalled = air.stalls.due();
        let unreached = wifid.arrivals.borrow().due();
        tokio::select! {
            Some(change) = owners.recv() => {
                wifid.owner_changed(&change, &mut air, &mut networks).await;
            }
            Some(Ok(message)) = links.recv() => wifid.link_changed(&message, &mut networks).await,
            Some(Ok(message)) = signals.recv() => {
                take_signal(wifid, &mut air, joining.as_ref(), &message).await;
            }
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
                        wifid.nudge(&card, looked, &mut air).await;
                    }
                }
            }
            () = until(unreached), if unreached.is_some() => {
                // What wpa_supplicant has said is taken in before a move's
                // time is judged: a move that completed as its time ran
                // out, the signal still queued, would otherwise be sent
                // back off the point it reached. Only what's queued now is,
                // so signals coming without end can't hold the judging off.
                // One still on its way across the bus isn't, so `overdue`
                // asks wpa_supplicant where each card is before acting.
                tokio::task::yield_now().await;
                for _ in 0..signals.len() {
                    if let Ok(Ok(message)) = signals.try_recv() {
                        take_signal(wifid, &mut air, joining.as_ref(), &message).await;
                    }
                }
                let joining = joining.as_ref().map(|joining| joining.card.as_str());
                wifid.overdue(joining, Instant::now()).await;
            }
            _ = polling.tick(), if *watching.borrow() > 0 => {
                poll(bus, wifid.supplicant, &mut kernel, state).await;
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

/// Takes in a signal of wpa_supplicant's, then each card it says came off
/// its network, settled on one or went. A card in a join the shell asked
/// for isn't free, as the join turns its networks on as it ends.
async fn take_signal(
    wifid: &Wifid<'_>,
    air: &mut Air<'_>,
    joining: Option<&Joining>,
    message: &zbus::Message,
) {
    air.take_in(wifid.bus, message).await;
    let free = |card: &str| joining.is_none_or(|joining| joining.card != card);
    for card in std::mem::take(&mut air.tried) {
        wifid.take_off(&card, free(&card)).await;
    }
    for card in std::mem::take(&mut air.left) {
        wifid.came_off(&card, free(&card)).await;
    }
    for card in std::mem::take(&mut air.settled) {
        wifid.settled(&card, free(&card)).await;
    }
    for card in std::mem::take(&mut air.gone) {
        wifid.gone(&card);
    }
}

/// Answers on the sockets systemd holds, for as long as wifid runs: varlink
/// on one, the launcher's rows on the other. Run by hand, with no socket,
/// it answers nobody.
async fn answer(wifi: Wifi) -> Result<(), Box<dyn Error>> {
    let service::Handed { varlink, rows } = service::handed()?;
    let face = Face::new(
        wifi.state.clone(),
        wifi.asks.clone(),
        wifi.watching.clone(),
        wifi.calling.clone(),
    );
    let rows = async move {
        let Some(listener) = rows else {
            eprintln!("<4>systemd passed no rows socket, so the launcher draws no Wi-Fi");
            return future::pending().await;
        };
        face.serve(listener).await
    };
    let Some(listener) = varlink else {
        eprintln!("<4>systemd passed no socket, so nobody can ask wifid anything");
        rows.await?;
        return Err("stopped serving rows on the socket".into());
    };
    // Ending wifid has systemd start it again, which leaves every card on
    // its network and gives the shell its answers back.
    tokio::select! {
        served = Server::new(listener, wifi).run() => served?,
        served = rows => served?,
    }
    Err("stopped answering on a socket".into())
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

/// The frequencies avoid.json keeps each network's cards off; none, said
/// in the journal when it's there but can't be read, so every network
/// keeps every channel rather than one being lost.
fn avoided() -> Avoided {
    match fs::read_to_string(AVOIDED) {
        Ok(text) => Avoided::read(&text).unwrap_or_else(|reason| {
            eprintln!("<4>{AVOIDED}: {reason}; keeping every network on every channel");
            Avoided::default()
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Avoided::default(),
        Err(error) => {
            eprintln!("<4>{AVOIDED}: {error}; keeping every network on every channel");
            Avoided::default()
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
    /// The moves on their way, followed to where they end.
    arrivals: &'a RefCell<Arrivals>,
    /// Which cards have been looking for networks too long.
    stalls: Stalls,
    /// The cards that have just come off a network, by name, for their
    /// other networks to be turned on again.
    left: Vec<String>,
    /// The cards that have just joined a network, by name, for one that
    /// went astray as it was let go to look again.
    settled: Vec<String>,
    /// The cards that have just begun a try at a network, by name, for one
    /// Joel left between tries to be taken off it.
    tried: Vec<String>,
    /// The cards wpa_supplicant has just let go of, by name, for what a
    /// move kept of them to be forgotten.
    gone: Vec<String>,
    /// The access point each card was last said to be at, by the card's
    /// path and the point's, as wpa_supplicant said them in turn. A join
    /// is credited to the point it was said at, since by the time wifid
    /// reads its signal the card can be on its way to another: one sent
    /// back said it reached a point it was still authenticating with
    /// (2026-11-20).
    at: HashMap<String, String>,
    /// The account holding wpa_supplicant's name now, from the bus as
    /// wifid started and from each change of hands since, so its own
    /// signals are told from another's without asking the bus each time.
    owner: Option<String>,
    /// The last few accounts that held wpa_supplicant's name and let it
    /// go, whose signals still queued are passed over: they name paths a
    /// new wpa_supplicant numbers again from nought. Those drain within a
    /// tick or two, so older ones needn't be kept.
    departed: VecDeque<String>,
    /// The cards a look another account said of was passed over for, at
    /// the step each was last said at, so a flood of such looks says one
    /// line rather than crowding wifid's own out of the journal.
    passed: HashSet<String>,
}

impl<'a> Air<'a> {
    /// Hears nothing yet, wpa_supplicant held by its owner as wifid starts,
    /// if it runs.
    fn new(
        state: watch::Sender<State>,
        seen: &'a RefCell<Seen>,
        arrivals: &'a RefCell<Arrivals>,
        owner: Option<String>,
    ) -> Self {
        Air {
            points: HashMap::new(),
            state,
            cards: HashMap::new(),
            seen,
            arrivals,
            stalls: Stalls::default(),
            left: Vec::new(),
            settled: Vec::new(),
            tried: Vec::new(),
            gone: Vec::new(),
            at: HashMap::new(),
            owner,
            departed: VecDeque::new(),
            passed: HashSet::new(),
        }
    }

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
        let sender = header.sender().map(zbus::names::UniqueName::as_str);
        if sender.is_some_and(|sender| self.departed.iter().any(|gone| gone == sender)) {
            return;
        }
        if !self.own(bus, sender).await {
            return self.said_by_another(bus, message).await;
        }
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
                let Ok(card) = body.deserialize::<OwnedObjectPath>() else {
                    return;
                };
                self.gone_under(&format!("{}/", card.as_str()));
            }
            "PskMismatch" => self.troubled(bus, path, Trouble::WrongPassword).await,
            // What a card heard is said before it says it's done looking.
            "ScanDone" => {
                let Some(card) = self.named(bus, path).await else {
                    return;
                };
                self.arrivals.borrow_mut().looked(&card, Instant::now());
                if body.deserialize::<bool>().unwrap_or(false) {
                    println!("<7>{card} is done looking");
                    change(&self.state, |state| state.looked(&card));
                } else {
                    println!("<7>{card}'s look failed");
                    change(&self.state, State::look_ended);
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

    /// Takes in another account's signal only as a card beginning to look
    /// (`serve`), and only its step, of a card holding no network and in
    /// no move, so it can't end a move, lift a hold or let a card's other
    /// networks on.
    async fn said_by_another(&mut self, bus: &Connection, message: &zbus::Message) {
        let header = message.header();
        let Some(path) = header.path() else { return };
        let changed = message
            .body()
            .deserialize::<(String, HashMap<String, OwnedValue>, Vec<String>)>();
        if header
            .member()
            .is_some_and(|member| member == "PropertiesChanged")
            && header
                .interface()
                .is_some_and(|interface| interface == "org.freedesktop.DBus.Properties")
            && let Ok((interface, changed, _)) = changed
            && begins_looking(&interface, &changed)
        {
            // A path that's none of the cards' is anyone's to make up, so
            // nothing is said of it.
            let Some((card, step)) = self.cards.get(path.as_str()) else {
                return;
            };
            let moving = self.arrivals.borrow().moving(card);
            let new = says_a_stranger_looked(step, self.stalls.counting(card));
            if let Some(changed) = stranger_look(changed, step, moving) {
                // Said apart, so the journal shows a look wpa_supplicant
                // never said, as the stall it can set off is read there.
                if new {
                    println!("<7>another account says {card} is scanning");
                }
                self.card_changed(bus, path, &changed).await;
            } else if self.passed.insert(card.clone()) {
                // Said once a step, so a look passed over as its card still
                // held a network, or was moving, shows why nothing followed.
                println!("<7>passed over a look another account said of {card}");
            }
        }
    }

    /// Whether a signal from this sender is wpa_supplicant's own. Knowing
    /// nobody, as where the ask failed as wifid started, it asks the bus
    /// again. Knowing the owner, it never does: one started since is
    /// read in its change of hands, which forgets all its signals said
    /// before, and asking for each of another account's signals would let
    /// any account slow the loop judging a move.
    async fn own(&mut self, bus: &Connection, sender: Option<&str>) -> bool {
        if said_by(sender, self.owner.as_deref()) {
            return true;
        }
        if sender.is_none() || self.owner.is_some() {
            return false;
        }
        let Ok(owners) = DBusProxy::new(bus).await else {
            return false;
        };
        if let Some(owner) = supplicant_owner(&owners).await {
            self.owner = Some(owner);
        }
        said_by(sender, self.owner.as_deref())
    }

    /// Takes in what wpa_supplicant says changed of the card at this path:
    /// how far it's got joining a network, the access point it's at, and
    /// any status that ends its join. The access point is taken only as
    /// one of the card's own.
    async fn card_changed(
        &mut self,
        bus: &Connection,
        path: &ObjectPath<'_>,
        changed: &HashMap<String, OwnedValue>,
    ) {
        let point = changed
            .get("CurrentBSS")
            .map(|point| &**point)
            .filter(|point| of_card(path.as_str(), point));
        // Taken as wifid reads the signal, before it asks the bus anything,
        // so the time a move took isn't the time wifid took to ask what it
        // reached. Anything wifid was busy with as the signal came still
        // counts in it.
        let now = Instant::now();
        // An attempt at a network begins as the card starts authenticating
        // with it, or associating where it skips that.
        let step = changed.get("State").map(|step| &**step);
        // Whether this signal's step ended a join, so a point it names in
        // the same signal doesn't end the move again.
        let mut landed = false;
        if let Some(zvariant::Value::Str(step)) = step {
            let step = step.to_string();
            if let Some(card) = self.named(bus, path).await {
                let before = self.cards.get(path.as_str());
                let before = before.map_or("", |(_, step)| step.as_str());
                let begins = step == "authenticating"
                    || (step == "associating" && before != "authenticating");
                let left = supplicant::left_its_network(before, &step);
                if begins {
                    change(&self.state, |state| state.attempting(&card));
                    self.tried.push(card.clone());
                }
                if step == "completed" && before != "completed" {
                    landed = true;
                    let said = credited(point, self.at.get(path.as_str()).map(String::as_str));
                    let on = self.joined(bus, path, said.as_deref()).await;
                    let arrived = self.arrivals.borrow_mut().joined(&card, on.as_deref(), now);
                    if let Some(arrived) = arrived {
                        println!("{}", arrived.said(&card));
                    }
                    self.settled.push(card.clone());
                } else if step != before
                    && let Some(lost) = self.arrivals.borrow_mut().step(&card, &step, now)
                {
                    println!("{}", lost.said(&card));
                }
                // Each step said, so a join that dawdles shows where.
                if step != before {
                    println!("<7>{card} is {step}");
                    self.passed.remove(&card);
                }
                self.stalls.step(&card, &step, Instant::now());
                if left {
                    self.left.push(card.clone());
                }
                self.cards.insert(path.to_string(), (card, step));
            }
        }
        if let Some(zvariant::Value::ObjectPath(said)) = point {
            // wpa_supplicant says only where a card's steps have got to when
            // it sends, so a move can come as a new point alone, the card
            // joined throughout.
            let bssid = self
                .points
                .get(said.as_str())
                .and_then(air::AccessPoint::heard);
            let point = joined_to(said);
            if let Some(card) = self.named(bus, path).await {
                // Kept only for a card, so a path no card has adds nothing.
                self.at.insert(path.to_string(), said.to_string());
                change(&self.state, |state| state.at(&card, point));
                let joined = self.cards.get(path.as_str());
                let joined = !landed && joined.is_some_and(|(_, step)| step == "completed");
                let arrived = bssid.filter(|_| joined).and_then(|heard| {
                    self.arrivals
                        .borrow_mut()
                        .joined(&card, Some(&heard.bssid), now)
                });
                // A move landing so ends as one landing on a completed
                // step does, its other networks on again.
                if let Some(arrived) = arrived {
                    println!("{}", arrived.said(&card));
                    self.settled.push(card.clone());
                }
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
    /// there first when it next has no network, and says the access point
    /// it joined, where it's heard: the one wpa_supplicant said, by its
    /// path, or where it said none or one not heard, the one it's at now.
    async fn joined(
        &self,
        bus: &Connection,
        card: &ObjectPath<'_>,
        said: Option<&str>,
    ) -> Option<String> {
        // A point said but not yet heard, or since forgotten, is asked for
        // again rather than leave the join unnamed.
        let said = said.and_then(|point| self.points.get(point));
        let point = if let Some(point) = said {
            point
        } else {
            let current = match interface(bus, card.to_owned().into()).await {
                Ok(interface) => interface.current_access_point().await.ok(),
                Err(_) => None,
            };
            current.and_then(|point| self.points.get(point.as_str()))?
        };
        let heard = point.heard()?;
        if let Some(frequency) = point.frequency()
            && let Err(error) = self.seen.borrow_mut().joined(&heard.name, frequency)
        {
            eprintln!("<4>can't keep where {} was joined: {error}", heard.name);
        }
        Some(heard.bssid)
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

    /// Forgets every card and access point whose path starts so. A card's
    /// own path is matched with a slash after it, as the paths under it are,
    /// so one gone takes its moves, its steps and where it was with it.
    fn gone_under(&mut self, path: &str) {
        for (card, (name, _)) in &self.cards {
            if under(card, path) {
                self.stalls.gone(name);
                self.arrivals.borrow_mut().forget(name);
                self.gone.push(name.clone());
            }
        }
        self.cards.retain(|card, _| !under(card, path));
        self.at.retain(|card, _| !under(card, path));
        self.points.retain(|point, _| !point.starts_with(path));
        change(&self.state, |state| state.unheard_under(path));
    }
}

/// Whether a signal is wpa_supplicant's own: its sender is the account
/// that holds wpa_supplicant's name now. With no sender named, or nobody
/// holding the name, it isn't.
fn said_by(sender: Option<&str>, owner: Option<&str>) -> bool {
    sender
        .zip(owner)
        .is_some_and(|(sender, owner)| sender == owner)
}

/// Whether another account may say a card at this step, in a move or not,
/// began to look: only one holding no network and in no move, for whom a
/// look made up can't end a join or a move, or let its networks on.
fn may_be_said_looking(step: &str, moving: bool) -> bool {
    !supplicant::holds_a_network(step) && !moving
}

/// What wifid takes of another account's look at a card at this step, in a
/// move or not: its step alone, so it can't name a point to credit a join
/// to or end one, and only of a card that may be said looking.
fn stranger_look(
    mut changed: HashMap<String, OwnedValue>,
    step: &str,
    moving: bool,
) -> Option<HashMap<String, OwnedValue>> {
    may_be_said_looking(step, moving).then(|| {
        changed.retain(|property, _| property == "State");
        changed
    })
}

/// Whether another account's look taken at a card at this step, its stall
/// counted or not, is said: only as it changes the card's step or starts a
/// stall's count, as one taken again changes nothing, so a flood can't
/// crowd wifid's own lines out. A card at rest after a nudge keeps
/// "scanning" as its step, so a look that starts its count again is new.
fn says_a_stranger_looked(step: &str, counting: bool) -> bool {
    step != "scanning" || !counting
}

/// Whether another account's signal says only what wifid takes from any:
/// a card beginning to look, among whatever else it says.
fn begins_looking(interface: &str, changed: &HashMap<String, OwnedValue>) -> bool {
    interface == "fi.w1.wpa_supplicant1.Interface"
        && matches!(
            changed.get("State").map(|step| &**step),
            Some(zvariant::Value::Str(step)) if step.as_str() == "scanning"
        )
}

/// Whether a point wpa_supplicant says a card is at is one of that card's,
/// under its path, or none, the root.
fn of_card(card: &str, point: &zvariant::Value<'_>) -> bool {
    match point {
        zvariant::Value::ObjectPath(point) => {
            point.as_str() == "/" || point.as_str().starts_with(&format!("{card}/BSSs/"))
        }
        _ => false,
    }
}

/// The path of the access point a card's completed join is credited to:
/// the one the same signal names, or else the last one said, or where
/// neither is, none, so wpa_supplicant is asked.
fn credited(said: Option<&zvariant::Value<'_>>, at: Option<&str>) -> Option<String> {
    match said {
        Some(zvariant::Value::ObjectPath(point)) => Some(point.to_string()),
        _ => at.map(str::to_owned),
    }
}

/// Whether a card's path is the one given, with a slash after it, or lies
/// under it.
fn under(card: &str, path: &str) -> bool {
    format!("{card}/").starts_with(path)
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

/// Reads the link of each card on a network, as wpa_supplicant polls it
/// and as the kernel says the rest. A card wpa_supplicant can't answer for
/// in a second is left unread, and one the kernel can't is read without
/// it, rather than holding wifid up.
async fn poll(
    bus: &Connection,
    supplicant: &SupplicantProxy<'_>,
    kernel: &mut Option<nl80211::Nl80211>,
    state: &watch::Sender<State>,
) {
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
            Ok(Ok(polled)) => {
                let (station, tuning) = station(kernel, &card).await;
                Some(Polled {
                    station,
                    tuning,
                    ..polled
                })
            }
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

/// What the kernel says of a card's station and tuning, opening its socket
/// first if none is open. A socket that errs or doesn't answer in a second
/// is let go, to be opened again at the next poll.
async fn station(
    kernel: &mut Option<nl80211::Nl80211>,
    card: &str,
) -> (Option<nl80211::Station>, Option<nl80211::Tuning>) {
    let reading = async {
        if kernel.is_none() {
            *kernel = Some(nl80211::Nl80211::open().await?);
        }
        let Some(socket) = kernel.as_mut() else {
            return Ok((None, None));
        };
        let station = socket.station(card).await?;
        io::Result::Ok((station, Some(socket.tuning(card).await?)))
    };
    match tokio::time::timeout(Duration::from_secs(1), reading).await {
        Ok(Ok(read)) => read,
        Ok(Err(error)) => {
            eprintln!("<7>can't read {card}'s station from the kernel: {error}");
            *kernel = None;
            (None, None)
        }
        Err(_) => {
            eprintln!("<7>can't read {card}'s station: the kernel didn't answer in time");
            *kernel = None;
            (None, None)
        }
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
        if let Some(standing) = now.standing() {
            let reached = last_standing.as_ref() != Some(&standing);
            if reached || due.is_some_and(|due| Instant::now() >= due) {
                change(&state, |state| state.checking(now.index));
                let (verdict, after) = checker
                    .check(&now.name, &now.dns, now.portal.as_deref(), reached)
                    .await;
                // A card can move to another network while the check is
                // out, and then the verdict is of the one it left: taken
                // in, it would end a join to the new one as that one's
                // reach. It goes unsaid, and the next turn checks afresh.
                if link.borrow().standing().as_ref() != Some(&standing) {
                    continue;
                }
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
    avoided: &'a Avoided,
    seen: &'a RefCell<Seen>,
    /// The moves on their way, followed to where they end.
    arrivals: &'a RefCell<Arrivals>,
    state: &'a watch::Sender<State>,
    /// The cards on a network Joel picked in the launcher, which keep
    /// their others off until they come off it.
    picked: RefCell<HashSet<String>>,
    /// The cards moved to an access point, each with the network it's
    /// held to that point on, until the card comes off it.
    held: RefCell<HashMap<String, OwnedObjectPath>>,
    /// The cards that took another network as they were let go, each with
    /// the network they were held on, to look again once joined.
    astray: RefCell<HashMap<String, OwnedObjectPath>>,
    /// The cards moving with their other networks off, until they've
    /// joined again, or are let join whatever they hear.
    alone: RefCell<HashSet<String>>,
    /// The cards Joel took off their network, never nudged to join one:
    /// until they've joined again, he's asked one to join, or
    /// wpa_supplicant has let them go, as a new one starts every card with
    /// every network on. It's held in memory alone, so a wifid started
    /// again forgets it: a look another account says then can have such a
    /// card nudged onto a saved network.
    stopped: RefCell<HashSet<String>>,
    /// The cards Joel left between tries, which wpa_supplicant wouldn't
    /// disconnect as they held no network, each with when he left it, to
    /// be taken off the next try they begin. wpa_supplicant names the
    /// network of a try before it says the try's first step, so a
    /// disconnect then holds, and keeps the card off as one Joel asked
    /// does. Its networks stay as they were until then, so a way back's
    /// next try is to the network he left rather than any it hears; a card
    /// with every network on, as one that fell off with no move, can still
    /// reach another for the moment before the disconnect lands. One left
    /// with no network on, as when a failed join forgets the one it was
    /// trying, begins no try and stays off. Held in memory alone, so a
    /// wifid started again before that try forgets it, and the card
    /// rejoins.
    leaving: RefCell<HashMap<String, Instant>>,
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
        let point = point.filter(|point| joined_to(point).is_some());
        let joined = point.as_ref().map(ToString::to_string);
        change(self.state, |state| state.at(&card.name, joined));
        // A card in a move, or on its way back, keeps the hold the move
        // gave it, as networks saved meanwhile hand every card over again:
        // held where it is mid-way, it could join any point of its network.
        if !self.arrivals.borrow().moving(&card.name) {
            self.hold_where_it_is(&card.name, &interface, point).await;
        }
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

    /// Holds a card on its network as it's handed over to the access point
    /// it's on, and lets go of any other of its networks held before.
    /// wifid forgets what it held as it stops: 10 s after a switch
    /// restarted it on 15 Nov 2026, wpa_supplicant moved the card on its
    /// own, the gateway silent 291 ms, and a hold kept from before was
    /// never let go, so once its point went the card couldn't rejoin its
    /// network elsewhere.
    async fn hold_where_it_is(
        &self,
        card: &str,
        interface: &InterfaceProxy<'_>,
        point: Option<OwnedObjectPath>,
    ) {
        let held = async {
            let on = match point {
                Some(point) if interface.state().await? == "completed" => {
                    let heard = AccessPoint::new(described(self.bus, &point).await?).heard();
                    let network = interface.current_network().await?;
                    heard.map(|heard| (network, heard.bssid))
                }
                _ => None,
            };
            for network in interface.networks().await? {
                if on.as_ref().is_some_and(|(held, _)| *held == network) {
                    continue;
                }
                let fields = self.joinable(&network).await?.properties().await?;
                if fields.contains_key("bssid") {
                    self.hold(&network, "any").await?;
                    println!("<7>{card} may join a network held before at any access point");
                }
            }
            if let Some((network, bssid)) = on {
                self.hold(&network, &bssid).await?;
                self.held.borrow_mut().insert(card.to_owned(), network);
                println!("<7>{card} keeps to {bssid}, which it was on as wifid started");
            }
            zbus::Result::Ok(())
        };
        if let Err(error) = held.await {
            eprintln!("<4>{card} isn't held where it is: {error}");
        }
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
            let mut fields = match supplicant::properties(&saved.network, password, address) {
                Ok(fields) => fields,
                Err(unjoinable) => {
                    eprintln!("<4>{name}: {}", unjoinable.0);
                    continue;
                }
            };
            // Part of the fields, so a network given another list is taken
            // away and given again as wifid next starts.
            if let Some(kept) = self.avoided.list(name) {
                fields.push(("freq_list", supplicant::Value::Text(kept)));
            }
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
        // join from the launcher leaves the others, is on again once the
        // card is off the network Joel picked; while it's on it, they stay
        // off, or its next look could move it (supplicant::holds_a_network).
        // wpa_supplicant adds a network off, so one newly saved is on.
        // A card in a move keeps them off until the move ends, whatever its
        // step, as a Keep, Forget or Trust can come meanwhile. One Joel left
        // between tries keeps them as they are until its next try is taken
        // off, and is marked alone so they come on then (`leaving`).
        let picked = self.picked.borrow().contains(&card.name);
        let alone = self.alone.borrow().contains(&card.name);
        let leaving = self.leaving.borrow().contains_key(&card.name);
        // The step is read only where it can change what's done.
        let step = if picked && !alone && !leaving {
            interface.state().await?
        } else {
            String::new()
        };
        match others::others_given(alone, leaving, picked, &step) {
            Others::Kept => {}
            Others::KeptTillTakenOff => {
                self.alone.borrow_mut().insert(card.name.clone());
            }
            // Each is turned on alone, as in turn_on_networks, so one that
            // won't doesn't leave the rest off.
            Others::On => {
                for (_, network) in &joinable {
                    let on = async { self.joinable(network).await?.set_enabled(true).await };
                    if let Err(error) = on.await {
                        let (card, network) = (&card.name, network.as_str());
                        eprintln!("<4>can't turn {card}'s {network} on again: {error}");
                    }
                }
            }
        }
        Ok(joinable)
    }

    /// Turns on every network the card holds, the others a join from the
    /// launcher left off among them, so a card come off its network can
    /// join whichever it hears.
    /// Each is turned on alone, so one that won't doesn't leave the rest
    /// off. One that won't is said and not tried again: one forgotten
    /// meanwhile needn't be, and every one is turned on again as the card
    /// comes off the network it's on (`free`), or is handed its networks
    /// (`give_networks`) at a change to what's saved or by a stall's nudge
    /// 20 s into looking off every network.
    async fn turn_on_networks(&self, card: &str) {
        let listed = async {
            let at = self.supplicant.get_interface(card).await?;
            interface(self.bus, at).await?.networks().await
        };
        let networks = match listed.await {
            Ok(networks) => networks,
            Err(error) => {
                eprintln!("<4>can't turn {card}'s networks on again: {error}");
                return;
            }
        };
        for network in networks {
            let on = async { self.joinable(&network).await?.set_enabled(true).await };
            if let Err(error) = on.await {
                let network = network.as_str();
                eprintln!(
                    "<4>can't turn {card}'s {network} on again, so it may join only what's \
                     on until it next comes off its network or is handed its networks: \
                     {error}"
                );
            }
        }
    }

    /// Starts over with wpa_supplicant or networkd as either starts again.
    async fn owner_changed(
        &self,
        change: &NameOwnerChanged,
        air: &mut Air<'_>,
        networks: &mut Networks,
    ) {
        let Ok(args) = change.args() else { return };
        let started = args.new_owner().is_some();
        match args.name().as_str() {
            // Stopped, it holds none of the cards, networks or points it
            // had, whether or not it comes back; started again, it hears
            // nothing yet, and says each access point as it hears it.
            "fi.w1.wpa_supplicant1" => {
                if let Some(old) = Option::as_ref(args.old_owner()) {
                    air.departed.push_back(old.to_string());
                    if air.departed.len() > DEPARTED {
                        air.departed.pop_front();
                    }
                }
                air.owner = Option::as_ref(args.new_owner()).map(ToString::to_string);
                air.gone_under("");
                // Forgotten now, not as the next signal comes, which would
                // take the holds the cards handed over below have since.
                for card in std::mem::take(&mut air.gone) {
                    self.gone(&card);
                }
                self.forget_all();
                if started {
                    self.add_cards().await;
                }
            }
            "org.freedesktop.network1" if started => {
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
            ask => {
                let joining = joining.as_ref().map(|joining| joining.card.as_str());
                self.answer(ask, joining).await;
            }
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
                // The join turns the card's networks on as it ends,
                // however it ends.
                self.alone.borrow_mut().remove(&card);
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
        // Joel's pick ends any move or way back the card was on, which,
        // stranded later, would let go of the join he asked for, and lets
        // go of its hold first, so the join isn't aimed at a point gone.
        self.arrivals.borrow_mut().forget(&card.name);
        let was_stopped = self.stopped.borrow_mut().remove(&card.name);
        self.let_go(&card.name, false).await;
        let selected = match self.supplicant.get_interface(&card.name).await {
            Ok(at) => match interface(self.bus, at).await {
                Ok(interface) => interface.select_network(&path.as_ref()).await,
                Err(error) => Err(error),
            },
            Err(error) => Err(error),
        };
        if let Err(error) = selected {
            // Left off its network still, a card Joel left stays so.
            if was_stopped {
                self.stopped.borrow_mut().insert(card.name.clone());
            }
            // The move it ended kept the card's other networks off, and
            // with no join to turn them on as it ends, they come on now,
            // unless Joel left it between tries (`together`).
            self.together(&card.name, true).await;
            return Err(failed(format!("wpa_supplicant didn't join it: {error}")));
        }
        // Joel's join ends his Leave: wpa_supplicant tries the network
        // he picked, which isn't to be taken off.
        self.leaving.borrow_mut().remove(&card.name);
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
    async fn nudge(&self, name: &str, looked: Duration, air: &mut Air<'_>) {
        // A card Joel left looks only as another account says it does, as
        // it has nothing to join, so that's no stall to end. It's no
        // longer counted as looking, as wpa_supplicant says no step to end
        // it, but keeps its wait, so a stream of such looks is said no
        // more often than a card nudged.
        if self.stopped.borrow().contains(name) {
            air.stalls.rest(name);
            return println!("<7>{name} was taken off its network, so isn't asked to join one");
        }
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
    /// the card couldn't join, and turns the card's other networks back on
    /// when it couldn't. Joined, they stay off while the card is on it, and
    /// come on again as it leaves.
    async fn settle(&self, joining: Joining, outcome: Result<(), Refused>) {
        let Joining {
            name, card, new, ..
        } = &joining;
        match outcome {
            Ok(()) => {
                println!("<6>joined {name} on {card}");
                self.picked.borrow_mut().insert(card.clone());
                self.add_card_named(card).await;
            }
            Err(refused) => {
                println!("<5>{card} couldn't join {name}: {refused:?}");
                self.picked.borrow_mut().remove(card);
                let _ = joining.progress.send(Err(refused)).await;
                if *new {
                    if let Err(error) = self.store.forget(name) {
                        eprintln!("<4>can't forget {name} again: {error:?}");
                    }
                    let _ = self.given_everything().await;
                } else {
                    self.add_card_named(card).await;
                }
                // wpa_supplicant may still be trying the network it was
                // asked to join, so the others are turned on whatever step
                // the card is at, as it's no longer picked. A move begun
                // during the join keeps them off till it ends, and a card
                // Joel left between the join's tries till its next is taken
                // off.
                let alone = self.alone.borrow().contains(card);
                let leaving = self.leaving.borrow().contains_key(card);
                match others::others_given(alone, leaving, false, "") {
                    Others::Kept => {}
                    Others::KeptTillTakenOff => {
                        self.alone.borrow_mut().insert(card.clone());
                    }
                    Others::On => self.turn_on_networks(card).await,
                }
            }
        }
    }

    /// Changes the saved networks as asked, and says how it went once
    /// networkd and wpa_supplicant have every network as it now stands.
    async fn answer(&self, ask: Ask, joining: Option<&str>) {
        let (changed, done) = match ask {
            Ask::Join { .. } | Ask::Lookup { .. } => {
                unreachable!("the main loop answers joins and lookups itself")
            }
            Ask::Scan { done } => {
                let _ = done.send(self.scan().await);
                return;
            }
            Ask::Ready { bssid, done } => {
                let _ = done.send(self.ready(&bssid).await);
                return;
            }
            Ask::Roam { bssid, done } => {
                let _ = done.send(self.roam(&bssid, joining).await);
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
        // Each card is left whether or not one before it would, so a card
        // that won't leave doesn't keep the rest on, and the refusal says
        // which didn't.
        let mut unleft = Vec::new();
        for card in cards {
            if let Err(error) = self.leave_card(&card).await {
                unleft.push(format!("{card} didn't leave {name}: {error}"));
            } else {
                println!("<6>{card} left {name}, as asked");
            }
        }
        if unleft.is_empty() {
            return Ok(());
        }
        Err(Refused::Failed {
            reason: unleft.join("; "),
        })
    }

    /// Takes the card off its network. Left mid-move, a card isn't sent
    /// back to the network it left, and one that won't leave keeps its
    /// move.
    async fn leave_card(&self, card: &str) -> zbus::Result<()> {
        let at = self.supplicant.get_interface(card).await?;
        // Between one try and the next, as a way back is or a card that
        // fell off, wpa_supplicant refuses a disconnect and would join at
        // its next try, so the card is taken off that try as it begins
        // (`leaving`). networkd can still say it's on that network until
        // it hears it fell off.
        let between = match interface(self.bus, at).await?.disconnect().await {
            Ok(()) => false,
            Err(zbus::Error::MethodError(error, _, _)) if error.as_str() == NOT_CONNECTED => true,
            Err(error) => return Err(error),
        };
        self.arrivals.borrow_mut().forget(card);
        self.stopped.borrow_mut().insert(card.to_owned());
        if between {
            self.leaving
                .borrow_mut()
                .insert(card.to_owned(), Instant::now());
            println!("<6>{card} is between tries, so is taken off its next");
        } else {
            // What a move kept off is freed, as a card left between a way
            // back's tries says no step leaving one. Turned on, its
            // networks aren't joined after this, as wpa_supplicant keeps a
            // card it disconnected off until asked to join.
            self.leaving.borrow_mut().remove(card);
            self.together(card, true).await;
        }
        self.let_go(card, false).await;
        Ok(())
    }

    /// Takes a card Joel left between tries off the try it's begun, now
    /// wpa_supplicant holds a network to disconnect it from. A try that
    /// ended before the disconnect reached it leaves the card to be taken
    /// off its next. A card in a join the shell asked for isn't free, as
    /// the join turns its networks on as it ends.
    async fn take_off(&self, card: &str, free: bool) {
        let Some(asked) = self.leaving.borrow().get(card).copied() else {
            return;
        };
        let left = async {
            interface(self.bus, self.supplicant.get_interface(card).await?)
                .await?
                .disconnect()
                .await
        };
        match left.await {
            Ok(()) => {
                // Said with how long since Joel left it, so a try begun
                // long after, as one asked by hand, shows why it's undone.
                let since = asked.elapsed().as_secs();
                println!("<6>{card} was taken off its next try, {since} s after it was left");
                self.leaving.borrow_mut().remove(card);
                self.together(card, free).await;
            }
            Err(zbus::Error::MethodError(error, _, _)) if error.as_str() == NOT_CONNECTED => {
                println!("<7>{card} had no try under way to take off, so its next is");
            }
            // Tried again only at the try's end (`settled`), so said in
            // full: the card may stay on a network Joel left.
            Err(error) => println!(
                "<4>{card} wasn't taken off its next try, so may join the network Joel left: \
                 {error}"
            ),
        }
    }

    /// Whether wpa_supplicant says the card is on a network now, rather
    /// than in a signal it may have sent before Joel left it.
    async fn completed(&self, card: &str) -> bool {
        let step = async {
            interface(self.bus, self.supplicant.get_interface(card).await?)
                .await?
                .state()
                .await
        };
        match step.await {
            Ok(step) => step == "completed",
            // Kept left on a read that fails, as a stale join would, but
            // said, as it then keeps a stuck look from being nudged.
            Err(error) => {
                println!("<4>{card} stays marked left, as its step couldn't be read: {error}");
                false
            }
        }
    }

    /// Moves the card on an access point's network to it, for the chooser
    /// or for whoever compares one access point with another.
    ///
    /// The network is held to that point until the card comes off it.
    /// Left free, wpa_supplicant took the card back to the point it
    /// preferred within a minute, three times on 15 Nov 2026, and each
    /// move went silent for 200 to 800 ms. It's held before the move, so
    /// what the card hears meanwhile can't take it back, and let go if
    /// the move fails.
    ///
    /// Held, wpa_supplicant can take the card there itself on what it
    /// hears, and a card on its way or there already isn't asked again,
    /// which would join it over.
    ///
    /// A card on its way back isn't moved: the way back would follow the
    /// new point, and a card reaching the one it went back to would be
    /// counted elsewhere and let go. A card in the join the shell asked
    /// for, `joining`, has its networks turned on by the join as it ends.
    async fn roam(&self, bssid: &str, joining: Option<&str>) -> Result<(), Refused> {
        let card = self.state.borrow().roamer(bssid)?;
        if self.arrivals.borrow().going_back(&card) {
            return Err(Refused::Failed {
                reason: format!("{card} is on its way back to the point it left"),
            });
        }
        // A move already under way is put back as it stood if this ask
        // fails, held to its point again, so its timer still sends the card
        // back, other networks off, should it reach neither point.
        let before = self.arrivals.borrow().saved(&card);
        let moving = before.is_some();
        let roamed = async {
            let interface =
                interface(self.bus, self.supplicant.get_interface(&card).await?).await?;
            let network = interface.current_network().await?;
            self.hold(&network, bssid).await?;
            self.held.borrow_mut().insert(card.clone(), network.clone());
            let step = interface.state().await?;
            if self.on_its_way(&card, bssid, &step) {
                println!("<7>{card} is {step} on its own, held to {bssid}");
                // Held, wherever it joins is judged by this point. One
                // already in a move has it aimed here, keeping its way back.
                if step != "completed" {
                    self.arrivals
                        .borrow_mut()
                        .heading(&card, bssid, Instant::now());
                }
                Ok(())
            } else {
                // A card with no gateway yet is still on its point.
                let from = self.state.borrow().point_of(&card);
                if from.is_none() {
                    println!("<4>{card} moves to {bssid} with no point known to go back to");
                }
                // With nowhere to go back to, a failed move is let join
                // what it hears, so it keeps nothing off.
                if from.is_some() {
                    self.keep_alone(&card, &interface, &network).await;
                }
                // A move asked while the card looks waits for the look,
                // asked last, so one begun as its networks were turned off
                // is seen.
                let looking = interface.scanning().await.unwrap_or(false);
                {
                    let mut arrivals = self.arrivals.borrow_mut();
                    arrivals.asked(&card, from.as_deref(), bssid, Instant::now());
                    if looking {
                        arrivals.looking(&card);
                    }
                }
                let roamed = interface.roam(bssid).await;
                // Forgotten only here, since a failure before the ask is
                // none of a move already on its way.
                if roamed.is_err() && !moving {
                    self.arrivals.borrow_mut().forget(&card);
                }
                roamed
            }
        };
        if let Err(error) = roamed.await {
            if let Some(before) = before {
                let to = before.to().to_owned();
                println!("<6>{card} keeps on its move to {to}, as the move to {bssid} failed");
                // A move with nowhere to go back to kept nothing off, so
                // what this ask turned off comes on again.
                let alone = before.goes_back();
                self.resume(&card, before).await;
                if !alone {
                    self.together(&card, joining != Some(card.as_str())).await;
                }
            } else {
                self.together(&card, joining != Some(card.as_str())).await;
                self.let_go(&card, false).await;
            }
            return Err(Refused::Failed {
                reason: format!("{card} didn't move to {bssid}: {error}"),
            });
        }
        println!("<6>{card} moved to {bssid}, as asked");
        println!("<7>{card} keeps to {bssid} until it comes off its network");
        Ok(())
    }

    /// Sends a card back to the access point it left, held there, where
    /// its move hasn't reached the point it was asked to in time, so it's
    /// on its network again rather than looking for the one it missed, as
    /// it did for 18 s on 16 Nov 2026. It's moved there as any move is;
    /// where wpa_supplicant refuses that, as for a card off its network,
    /// that network alone is selected, the only point it may join being
    /// the one it left. wpa_supplicant carries on with whatever join it
    /// had queued, and took the Roadside Inn so when only asked to join
    /// again (2026-11-20), so the card's other networks stay off until it's
    /// joined. One that never left its point is held to it again and left
    /// there, and a card in a join Joel asked for is left to it.
    ///
    /// Says whether the way back was asked, or the card left where it got
    /// to, so a caller lets the card's network go otherwise, and it can
    /// join wherever it's heard.
    async fn go_back(
        &self,
        Unreached {
            card,
            from,
            to,
            why,
        }: Unreached,
        free: bool,
    ) -> bool {
        if !free {
            println!("<6>{card} is left to the join asked for, short of {to}");
            self.alone.borrow_mut().remove(&card);
            return false;
        }
        let held = self.held.borrow().get(&card).cloned();
        match why {
            Short::Late { after } => println!(
                "<4>{card} didn't reach {to} within {} ms, so goes back to {from}",
                after.as_millis()
            ),
            Short::Off => {
                println!("<4>{card} came off its network short of {to}, so goes back to {from}");
            }
            Short::Unstarted => {
                if self.under_way(&card, &from).await {
                    println!("<7>{card} has left {from} for {to} after all, so goes on");
                    return true;
                }
                if self.waits_on_look(&card, &from).await {
                    println!("<7>{card} hasn't left {from} for {to}, as it's looking, so waits");
                    return true;
                }
                return self.stays(&card, &from, &to, held, free).await;
            }
        }
        let gone_back = async {
            let interface =
                interface(self.bus, self.supplicant.get_interface(&card).await?).await?;
            let network = match held {
                Some(network) => network,
                None => interface.current_network().await?,
            };
            self.hold(&network, &from).await?;
            self.held.borrow_mut().insert(card.clone(), network.clone());
            self.arrivals
                .borrow_mut()
                .back(&card, &from, Instant::now());
            match interface.roam(&from).await {
                Ok(()) => Ok(()),
                Err(refused) => {
                    println!("<7>{card} joins {from} alone, as it couldn't move: {refused}");
                    self.alone.borrow_mut().insert(card.clone());
                    interface.select_network(&network.as_ref()).await
                }
            }
        };
        let Err(error) = gone_back.await else {
            return true;
        };
        self.arrivals.borrow_mut().forget(&card);
        println!("<4>{card} couldn't go back to {from}: {error}");
        self.together(&card, free).await;
        false
    }

    /// Whether a card whose move hasn't left its point, as wifid last
    /// heard, has left it after all, asked of wpa_supplicant, and so has
    /// its move carry on: the step saying so can be on its way across the
    /// bus as the move's time runs out, and ended there, the move would
    /// carry on unfollowed with the card's networks on. A card further on,
    /// in a handshake or done, is asked which point it's on too, as a
    /// rekey on its own point passes through the same steps; one whose
    /// point can't be named is said so and judged by its step alone
    /// (`arrive::has_left_for`). One whose step can't be read is said so
    /// and taken as never having left.
    async fn under_way(&self, card: &str, from: &str) -> bool {
        let read = async {
            let interface = interface(self.bus, self.supplicant.get_interface(card).await?).await?;
            let step = interface.state().await?;
            zbus::Result::Ok((interface, step))
        };
        let (interface, step) = match read.await {
            Ok(read) => read,
            Err(error) => {
                println!(
                    "<4>{card}'s step couldn't be read, so its move is taken as never begun: \
                     {error}"
                );
                return false;
            }
        };
        let mut on = None;
        if arrive::needs_point(&step) {
            let point = async {
                let point = interface.current_access_point().await?;
                zbus::Result::Ok(AccessPoint::new(described(self.bus, &point).await?).heard())
            };
            match point.await {
                Ok(Some(heard)) => on = Some(heard.bssid),
                Ok(None) => println!("<7>{card} is at {step} on a point wifid can't name"),
                Err(error) => {
                    println!("<7>{card} is at {step} on a point that couldn't be read: {error}");
                }
            }
        }
        arrive::has_left_for(&step, on.as_deref(), from)
            && self
                .arrivals
                .borrow_mut()
                .under_way(card, from, Instant::now())
    }

    /// Whether a card whose move hasn't left its point is looking, asked
    /// of wpa_supplicant, and so has its move wait on the look, once: one
    /// begun between the move's asking whether it looked and its ask has
    /// the move queued behind it, which ending the move wouldn't stop.
    async fn waits_on_look(&self, card: &str, from: &str) -> bool {
        let looking = async {
            let interface = interface(self.bus, self.supplicant.get_interface(card).await?).await?;
            interface.scanning().await
        };
        looking.await.unwrap_or(false) && self.arrivals.borrow_mut().waits_on_look(card, from)
    }

    /// Holds a card whose move never left the point it's on to that point
    /// again, and ends the move there: sent back, it would only rejoin the
    /// point it's on. Says whether it was held. A move still queued behind
    /// a look past `LOOK_ENDS` runs once it ends, unfollowed with the
    /// card's networks on, as every move ran before ways back: holding the
    /// network to its point doesn't stop it, as wpa_supplicant 2.12 checks
    /// only that the point and network are still there (sme.c,
    /// `sme_auth_start_cb`). A look over every channel takes 5 to 6 s.
    async fn stays(
        &self,
        card: &str,
        from: &str,
        to: &str,
        held: Option<OwnedObjectPath>,
        free: bool,
    ) -> bool {
        println!("<4>{card} never left {from} for {to}, so stays there");
        let kept = async {
            let network = match held {
                Some(network) => network,
                None => {
                    interface(self.bus, self.supplicant.get_interface(card).await?)
                        .await?
                        .current_network()
                        .await?
                }
            };
            self.hold(&network, from).await?;
            self.held.borrow_mut().insert(card.to_owned(), network);
            zbus::Result::Ok(())
        };
        let kept = kept.await;
        self.arrivals.borrow_mut().forget(card);
        self.together(card, free).await;
        if let Err(error) = &kept {
            println!("<4>{card} couldn't be kept to {from}: {error}");
        }
        kept.is_ok()
    }

    /// Forgets what a move kept of a card wpa_supplicant has let go of: a
    /// card handed over again holds new networks, which a move it no
    /// longer makes would keep off.
    fn gone(&self, card: &str) {
        self.astray.borrow_mut().remove(card);
        self.alone.borrow_mut().remove(card);
        self.held.borrow_mut().remove(card);
        self.picked.borrow_mut().remove(card);
        self.stopped.borrow_mut().remove(card);
        self.leaving.borrow_mut().remove(card);
    }

    /// Puts back a card's move an ask meant to replace failed, held to the
    /// point it was going to again, as the ask held it to its own first.
    async fn resume(&self, card: &str, before: arrive::Saved) {
        let network = self.held.borrow().get(card).cloned();
        if let Some(network) = network
            && let Err(error) = self.hold(&network, before.to()).await
        {
            println!(
                "<4>{card} couldn't be held to {} again: {error}",
                before.to()
            );
        }
        self.arrivals.borrow_mut().resume(card, before);
    }

    /// Forgets every card's move, hold and pick as wpa_supplicant stops or
    /// starts again, cards it never said a step for included: one that's
    /// been on its network since wifid started has said none. The paths
    /// they name were the old wpa_supplicant's, which a new one numbers
    /// again from nought, and it starts on no network, with every network
    /// on.
    fn forget_all(&self) {
        self.astray.borrow_mut().clear();
        self.alone.borrow_mut().clear();
        self.held.borrow_mut().clear();
        self.picked.borrow_mut().clear();
        self.stopped.borrow_mut().clear();
        self.leaving.borrow_mut().clear();
        self.arrivals.borrow_mut().clear();
    }

    /// Takes in a card come off its network. One that falls off partway
    /// into a move goes back, held, before wpa_supplicant takes another
    /// network. Otherwise it's let go, and its networks turned on where
    /// it's free: a join the shell asked for turns the others back on as
    /// it ends, however it ends, so a card in one is left to it.
    async fn came_off(&self, card: &str, free: bool) {
        let back = self.arrivals.borrow_mut().fell_off(card);
        if let Some(back) = back
            && self.go_back(back, free).await
        {
            return;
        }
        // Selected alone, wpa_supplicant starts its join over, saying the
        // card off its network meanwhile, so a way back is given its time.
        if free && self.arrivals.borrow().going_back(card) {
            return;
        }
        self.release(card, free).await;
    }

    /// Lets a card off its network join whatever it hears: no longer on
    /// the network Joel picked, its other networks on where it's free, and
    /// its network let go.
    async fn release(&self, card: &str, free: bool) {
        self.free(card, free).await;
        self.let_go(card, free).await;
    }

    /// Takes a card off the network Joel picked and out of a move's
    /// keeping alone, with its other networks on where it's free.
    async fn free(&self, card: &str, free: bool) {
        self.picked.borrow_mut().remove(card);
        // A card Joel left between tries keeps its networks as they are
        // until it's taken off its next (`leaving`).
        if self.leaving.borrow().contains_key(card) {
            return;
        }
        self.alone.borrow_mut().remove(card);
        if free {
            self.turn_on_networks(card).await;
        }
    }

    /// Sends each card whose move is late back to the point it left, and
    /// lets each whose way back is late join whatever it hears. A card in
    /// the join the shell asked for, `joining`, is left to it.
    async fn overdue(&self, joining: Option<&str>, now: Instant) {
        let free = |card: &str| joining != Some(card);
        let unreached = self.arrivals.borrow_mut().unreached(now);
        for late in unreached {
            let card = late.card.clone();
            if self
                .arrived(&card, &late.to, free(&card), late.why.left(), None)
                .await
            {
                continue;
            }
            if !self.go_back(late, free(&card)).await {
                self.let_go(&card, false).await;
            }
        }
        let stranded = self.arrivals.borrow_mut().stranded(now);
        // A way back is sent only from a move that left, so has left too.
        for (card, to, going) in stranded {
            if !self
                .arrived(&card, &to, free(&card), true, Some(going))
                .await
            {
                self.stranded(&card, &to, going, free(&card)).await;
            }
        }
    }

    /// Whether a card whose move's time has run out is on the point it was
    /// moving to after all, asked of wpa_supplicant: its reply comes after
    /// every signal it sent before, so a join completed as the time ran
    /// out, its signal not yet read, is seen. One that is ends its move as
    /// any arrival does. One done on a point wifid can't name is there
    /// only once its move has `left`, on the network the move holds
    /// (`arrive::reached`). Where wpa_supplicant can't say, it isn't,
    /// said so that a verdict from a failed read is told from a miss. A
    /// way back already handed back as stranded is said found there as
    /// its time, `going`, ran out, since when it got there isn't known.
    async fn arrived(
        &self,
        card: &str,
        to: &str,
        free: bool,
        left: bool,
        going: Option<Duration>,
    ) -> bool {
        let held = self.held.borrow().get(card).cloned();
        let there = async {
            let interface = interface(self.bus, self.supplicant.get_interface(card).await?).await?;
            if interface.state().await? != "completed" {
                return zbus::Result::Ok(None);
            }
            let point = interface.current_access_point().await?;
            let heard = AccessPoint::new(described(self.bus, &point).await?).heard();
            let on = heard.map(|heard| heard.bssid);
            // Only a point that can't be named, of a move that left, needs
            // its network asked.
            let on_its_network =
                on.is_none() && left && Some(interface.current_network().await?) == held;
            Ok(Some((on, on_its_network)))
        };
        let (on, on_its_network) = match there.await {
            Ok(Some(on)) => on,
            Ok(None) => return false,
            Err(error) => {
                println!("<7>{card}'s step or point couldn't be read, so isn't at {to}: {error}");
                return false;
            }
        };
        if !arrive::reached(on.as_deref(), to, left, on_its_network) {
            // One that never left is said so as it's judged where it is
            // (`under_way`).
            if on.is_none() && left {
                println!(
                    "<7>{card} is done on a point wifid can't name, not on the network its move \
                     holds, so isn't taken as at {to}"
                );
            }
            return false;
        }
        let arrived = self
            .arrivals
            .borrow_mut()
            .joined(card, on.as_deref(), Instant::now());
        let at = on.as_deref().unwrap_or("a point wifid can't name");
        match (arrived, going) {
            (Some(arrived), _) => println!("{}", arrived.said(card)),
            (None, Some(going)) => println!(
                "<6>{card} was found at {at} as its {} ms ran out, so isn't let go",
                going.as_millis()
            ),
            (None, None) => println!("<6>{card} reached {at} as its time ran out"),
        }
        self.settled(card, free).await;
        true
    }

    /// Lets a card that hasn't got back to the point it left within its
    /// time join whatever it hears, its other networks on again, so one
    /// whose point has gone too isn't held to it. It's off the network
    /// Joel picked, if that was the one, as a card come off it is.
    async fn stranded(&self, card: &str, to: &str, going: Duration, free: bool) {
        println!(
            "<4>{card} didn't get back to {to} within {} ms, so may join any network",
            going.as_millis()
        );
        self.free(card, free).await;
        // Not to look again once it's taken another network, as a card
        // let go of a hold does: that look had it leave the Roadside Inn
        // for the point that wouldn't answer, 0.2 s after joining (VM,
        // 2026-11-20).
        self.let_go(card, false).await;
    }

    /// Turns off the card's networks but the one it's moving on, so
    /// wpa_supplicant can't take another as the point it's moving to
    /// fails. Left on, it was joining the Roadside Inn as wifid heard
    /// the move had failed, and turning that join down as the card went
    /// back had it blame the point the card went back to, ignore it for
    /// ten seconds and look 800 ms for another (2026-11-20). The move
    /// doesn't need them off, and a way back still works with one on, so
    /// a network that won't turn off, as one forgotten meanwhile, is said
    /// and the move goes on.
    async fn keep_alone(
        &self,
        card: &str,
        interface: &InterfaceProxy<'_>,
        network: &OwnedObjectPath,
    ) {
        self.alone.borrow_mut().insert(card.to_owned());
        let others = match interface.networks().await {
            Ok(others) => others,
            Err(error) => {
                println!("<4>{card} moves with its networks on, unlisted: {error}");
                return;
            }
        };
        for other in others.iter().filter(|other| *other != network) {
            let off = async { self.joinable(other).await?.set_enabled(false).await };
            if let Err(error) = off.await {
                let other = other.as_str();
                println!("<4>{card} moves with {other} on, as it wouldn't turn off: {error}");
            }
        }
    }

    /// Turns the card's other networks on again where a move kept them
    /// off. A card on the network Joel picked keeps them off while it's
    /// on it, and one in a join the shell asked for has them turned on as
    /// the join ends, however it ends. One Joel left between tries keeps
    /// them as they are until it's taken off its next (`leaving`).
    async fn together(&self, card: &str, free: bool) {
        if self.leaving.borrow().contains_key(card) {
            return;
        }
        let alone = self.alone.borrow_mut().remove(card);
        if alone && free && !self.picked.borrow().contains(card) {
            println!("<7>{card}'s other networks are on again");
            self.turn_on_networks(card).await;
        }
    }

    /// Whether the card, at this step, is joining the access point it's
    /// held to or is on it already. Any join while held is to that point.
    fn on_its_way(&self, card: &str, bssid: &str, step: &str) -> bool {
        let there = self
            .state
            .borrow()
            .joined()
            .iter()
            .any(|joined| joined.card == card && joined.point.eq_ignore_ascii_case(bssid));
        supplicant::holds_a_network(step) && (step != "completed" || there)
    }

    /// Holds the card's network to the access point it's on, then has it
    /// look over the channel alone of the one it's about to move to. The
    /// kernel keeps what it hears 30 s, and looks for a point it's
    /// forgotten as the move begins: 145 ms of the 401 ms one move went
    /// silent (2026-11-15), the replies held for the card meanwhile lost
    /// as it left straight after. Looked for beforehand, they're given it
    /// as it comes back.
    ///
    /// Held where it is, what the look hears can't take the card anywhere,
    /// so the look is apart from the move. Held to the next point instead,
    /// wpa_supplicant moved on what the look heard, and the two went
    /// silent as one, 253 ms with 57 of them the look's (2026-11-15).
    async fn ready(&self, bssid: &str) -> Result<(), Refused> {
        let card = self.state.borrow().roamer(bssid)?;
        // Held anew, a way back couldn't reach the point it went back to.
        if self.arrivals.borrow().going_back(&card) {
            return Err(Refused::Failed {
                reason: format!("{card} is on its way back to the point it left"),
            });
        }
        let frequency = self
            .state
            .borrow()
            .around(&card)
            .iter()
            .find(|point| point.bssid.eq_ignore_ascii_case(bssid))
            .map(|point| point.frequency)
            .filter(|&frequency| frequency != 0);
        let held = async {
            let interface =
                interface(self.bus, self.supplicant.get_interface(&card).await?).await?;
            let network = interface.current_network().await?;
            let point = interface.current_access_point().await?;
            let on = AccessPoint::new(described(self.bus, &point).await?).heard();
            let on = on.map(|heard| heard.bssid);
            if let Some(on) = &on {
                self.hold(&network, on).await?;
                self.held.borrow_mut().insert(card.clone(), network);
            }
            zbus::Result::Ok((interface, on))
        };
        let (interface, on) = match held.await {
            Ok(held) => held,
            Err(error) => {
                self.let_go(&card, false).await;
                return Err(Refused::Failed {
                    reason: format!("{card} couldn't be held where it is: {error}"),
                });
            }
        };
        // Looking over every channel takes 5 to 6 s, longer than a move
        // waits, so one whose channel isn't known moves as it did.
        let Some(frequency) = frequency else {
            println!("<7>{card} moves to {bssid} without looking, its channel unknown");
            return Ok(());
        };
        let looking = HashMap::from([
            ("Type", zvariant::Value::from("active")),
            ("Channels", zvariant::Value::from(vec![(frequency, 20_u32)])),
        ]);
        let kept = on.map_or_else(|| "held nowhere".to_owned(), |on| format!("kept to {on}"));
        println!("<7>{card} looks over {frequency} MHz for {bssid}, {kept} until it moves");
        // A card already looking is refused, and the move waits for that
        // look instead.
        if let Err(error) = interface.scan(looking).await {
            println!("<7>{card} didn't look before moving: {error}");
        }
        Ok(())
    }

    /// Has wpa_supplicant join the network only at that access point, or at
    /// any, given `any`.
    async fn hold(&self, network: &OwnedObjectPath, bssid: &str) -> zbus::Result<()> {
        let fields = HashMap::from([("bssid", zvariant::Value::from(bssid))]);
        self.joinable(network).await?.set_properties(fields).await
    }

    /// Lets the network the card was held to join at any access point
    /// again, so it can rejoin wherever it's heard once the one it was
    /// held to goes.
    ///
    /// wpa_supplicant can pick again from what it heard before it's let
    /// go, and with that network held to a point that's gone it takes
    /// another network, as it did 110 ms ahead of wifid in a test
    /// (2026-11-15). So a card that's free, not in a join from the
    /// launcher, and on its way to another network by then is marked to
    /// look again once there, in `settled`.
    async fn let_go(&self, card: &str, free: bool) {
        let Some(network) = self.held.borrow_mut().remove(card) else {
            return;
        };
        // A network taken away with the card's others, as wpa_supplicant
        // starts again or the store changes, is gone with its hold.
        match self.hold(&network, "any").await {
            Ok(()) => println!("<7>{card} may join any access point of its network again"),
            Err(error) => println!("<7>{card}'s network wasn't let go: {error}"),
        }
        if free && self.elsewhere(card, &network, true).await {
            println!("<7>{card} took another network while held, and looks again once on it");
            self.astray.borrow_mut().insert(card.to_owned(), network);
        }
    }

    /// Has a card that went astray as it was let go pick again, now it's
    /// joined. Picking again partway into that join had wpa_supplicant's
    /// next try refused, and the network it was held on ignored for ten
    /// seconds (2026-11-15), so it waits until the join is done. A card
    /// that moved with its other networks off has them on again, unless a
    /// way back is still followed past a join elsewhere: it ends that, as
    /// the card reaches its point or is stranded.
    ///
    /// A card Joel left between tries is taken off the network it joined,
    /// as its first step may have come too soon for it. One he left is
    /// joined again only once wpa_supplicant says so now: a join it said
    /// before the Leave can be read after it.
    async fn settled(&self, card: &str, free: bool) {
        if self.leaving.borrow().contains_key(card) {
            return self.take_off(card, free).await;
        }
        let stopped = self.stopped.borrow().contains(card);
        if !stopped || self.completed(card).await {
            self.stopped.borrow_mut().remove(card);
        }
        if !self.arrivals.borrow().moving(card) {
            self.together(card, free).await;
        }
        let Some(held) = self.astray.borrow_mut().remove(card) else {
            return;
        };
        if !free || !self.elsewhere(card, &held, false).await {
            return;
        }
        let looked = async {
            interface(self.bus, self.supplicant.get_interface(card).await?)
                .await?
                .reassociate()
                .await
        };
        match looked.await {
            Ok(()) => println!("<6>{card} looks again, having taken another network while held"),
            Err(error) => eprintln!("<4>{card} can't look again once let go: {error}"),
        }
    }

    /// Whether the card is on, or on its way to, another network than
    /// that one; only on it, unless `on_its_way` will do.
    async fn elsewhere(&self, card: &str, network: &OwnedObjectPath, on_its_way: bool) -> bool {
        let asked = async {
            let interface = interface(self.bus, self.supplicant.get_interface(card).await?).await?;
            let step = interface.state().await?;
            let on = if on_its_way {
                supplicant::holds_a_network(&step)
            } else {
                step == "completed"
            };
            zbus::Result::Ok(on && interface.current_network().await? != *network)
        };
        asked.await.unwrap_or_else(|error| {
            eprintln!("<4>can't read which network {card} is on: {error}");
            false
        })
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

    #[test]
    fn a_card_is_gone_under_its_own_path_or_any_above_it_alone() {
        let card = "/fi/w1/wpa_supplicant1/Interfaces/1";
        assert!(under(card, ""), "a restart takes every card");
        assert!(under(card, &format!("{card}/")), "a card gone takes itself");
        assert!(
            under(card, "/fi/w1/wpa_supplicant1/Interfaces/"),
            "a parent's path takes every card under it, so only wpa_supplicant is heard on it"
        );
        assert!(
            !under("/fi/w1/wpa_supplicant1/Interfaces/10", &format!("{card}/")),
            "but not another card whose number starts the same"
        );
    }

    #[test]
    fn a_signal_is_taken_whole_only_from_the_wpa_supplicant_running_now() {
        assert!(
            said_by(Some(":1.7"), Some(":1.7")),
            "wpa_supplicant's own signal is taken"
        );
        assert!(
            !said_by(Some(":1.7"), Some(":1.9")),
            "one from a wpa_supplicant since replaced, or forged, isn't"
        );
        assert!(
            !said_by(Some(":1.7"), None),
            "nor one with nobody holding the name, which its going forgets anyway"
        );
        assert!(!said_by(None, Some(":1.7")), "nor one naming no sender");
    }

    #[test]
    fn a_join_is_credited_to_the_point_its_own_signal_names_before_the_last_said() {
        let named = ObjectPath::try_from("/fi/w1/wpa_supplicant1/Interfaces/1/BSSs/4")
            .expect("the path is one");
        let said = zvariant::Value::ObjectPath(named);
        let last = "/fi/w1/wpa_supplicant1/Interfaces/1/BSSs/2";

        assert_eq!(
            credited(Some(&said), Some(last)).as_deref(),
            Some("/fi/w1/wpa_supplicant1/Interfaces/1/BSSs/4"),
            "a point named with the join is the one joined"
        );
        assert_eq!(
            credited(None, Some(last)).as_deref(),
            Some(last),
            "a join naming none is credited to the last point said"
        );
        // The root, said of a card on no point, names nothing `joined`
        // knows, so it asks wpa_supplicant where the card is.
        let root = zvariant::Value::ObjectPath(ObjectPath::try_from("/").expect("the root is one"));
        assert_eq!(
            credited(Some(&root), Some(last)).as_deref(),
            Some("/"),
            "the root is passed on, never taken for the last point said"
        );
        assert_eq!(
            credited(None, None),
            None,
            "with none said either, wpa_supplicant is asked"
        );
    }

    #[test]
    fn another_account_is_heard_only_saying_a_card_begins_to_look() {
        let step = |step: &str| {
            HashMap::from([(
                "State".to_owned(),
                OwnedValue::from(zvariant::Str::from(step.to_owned())),
            )])
        };
        let interface = "fi.w1.wpa_supplicant1.Interface";

        assert!(
            begins_looking(interface, &step("scanning")),
            "a card beginning to look"
        );
        assert!(
            !begins_looking(interface, &step("completed")),
            "not a card said joined, which would end a move"
        );
        assert!(
            !begins_looking("fi.w1.wpa_supplicant1.BSS", &step("scanning")),
            "nor anything said of an access point"
        );
        assert!(
            !begins_looking(interface, &HashMap::new()),
            "nor a change saying no step"
        );
    }

    #[test]
    fn another_account_may_say_only_a_card_off_every_network_and_in_no_move_looks() {
        assert!(
            may_be_said_looking("disconnected", false),
            "a card stopped where it stands"
        );
        assert!(
            !may_be_said_looking("completed", false),
            "not one joined, which would be taken as leaving its network"
        );
        assert!(
            !may_be_said_looking("authenticating", false),
            "nor one joining"
        );
        assert!(
            !may_be_said_looking("disconnected", true),
            "nor one in a move, which would be sent back"
        );
    }

    #[test]
    fn another_accounts_look_is_taken_as_its_step_alone() {
        // A look said with a point beside it, which a forger could aim
        // the next join's credit at.
        let said = || {
            let point = ObjectPath::try_from("/fi/w1/wpa_supplicant1/Interfaces/1/BSSs/4")
                .expect("the path is one");
            HashMap::from([
                (
                    "State".to_owned(),
                    OwnedValue::from(zvariant::Str::from("scanning")),
                ),
                ("CurrentBSS".to_owned(), OwnedValue::from(point)),
            ])
        };
        let properties = |taken: Option<HashMap<String, OwnedValue>>| {
            taken.map(|taken| taken.into_keys().collect::<Vec<_>>())
        };

        let off = stranger_look(said(), "disconnected", false);
        let joined = stranger_look(said(), "completed", false);

        assert_eq!(
            properties(off),
            Some(vec!["State".to_owned()]),
            "of a card off every network, its step alone, never a point"
        );
        assert_eq!(properties(joined), None, "of one joined, nothing");
    }

    #[test]
    fn another_accounts_look_is_said_only_as_it_changes_something() {
        assert!(
            says_a_stranger_looked("disconnected", false),
            "said as it changes the card's step"
        );
        assert!(
            !says_a_stranger_looked("scanning", true),
            "unsaid while the card's look is counted already"
        );
        assert!(
            says_a_stranger_looked("scanning", false),
            "said as it starts the count of a card at rest"
        );
        // A count runs only while the stored step is "scanning", so this
        // row guards the step's half should the two ever drift apart.
        assert!(
            says_a_stranger_looked("disconnected", true),
            "said as it changes the card's step, whatever is counted"
        );
    }

    #[test]
    fn a_card_is_said_at_only_a_point_of_its_own_or_none() {
        let card = "/fi/w1/wpa_supplicant1/Interfaces/1";
        let at = |path: &'static str| {
            let path = ObjectPath::try_from(path).expect("the path is one");
            of_card(card, &zvariant::Value::ObjectPath(path))
        };

        assert!(
            at("/fi/w1/wpa_supplicant1/Interfaces/1/BSSs/4"),
            "its own point"
        );
        assert!(at("/"), "or none");
        assert!(
            !at("/fi/w1/wpa_supplicant1/Interfaces/2/BSSs/4"),
            "not another card's point"
        );
        assert!(
            !at("/fi/w1/wpa_supplicant1/Interfaces/12/BSSs/4"),
            "nor one whose card's number only starts as its own"
        );
        assert!(
            !of_card(card, &zvariant::Value::U32(4)),
            "nor anything but a path"
        );
    }

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
