//! timekeeping says how the clock is kept, for the launcher's Time: the
//! server systemd-timesyncd asks, how far the clock is from it, the round
//! trip's delay and jitter, the server's stratum, what it keeps its time
//! by and how far it is from that, how often it's asked and between what
//! bounds, the leap second announced, when it was last heard, the servers
//! timesyncd is given, and whether the clock counts as synchronised
//! (VKCF3Z's design, landing 6, and its Server, 2026-10-29).
//!
//! It hears timesyncd over D-Bus, so nothing runs `timedatectl` and
//! nothing asks on a clock: timesyncd says each new exchange as a change of
//! its `NTPMessage`, and the rest is read with it, since timesyncd says no
//! change of its server's name or address on its own. Each reading is
//! written whole to the file it's given, a small JSON object the launcher
//! watches, replaced through a rename so it's never seen half written.
//! Before its first exchange the file says how it keeps the clock alone.
//! While timesyncd isn't on the bus the file says nothing, as `{}`, and is
//! never removed, since the shell stops watching a file once it's gone.

mod reading;

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use futures_util::StreamExt;
use zbus::fdo::{DBusProxy, PropertiesProxy};
use zbus::names::{BusName, InterfaceName};
use zbus::proxy::CacheProperties;
use zbus::{Connection, proxy};

use reading::{Message, Said, Servers};

const TIMESYNC: &str = "org.freedesktop.timesync1";
const MANAGER: &str = "org.freedesktop.timesync1.Manager";

/// The file timesyncd keeps while the clock counts as synchronised.
const SYNCHRONISED: &str = "/run/systemd/timesync/synchronized";

/// timesyncd's last exchange, as D-Bus gives it: leap, version, mode,
/// stratum, precision, root delay, root dispersion, reference, then the
/// origin, receive, transmit and destination times, whether it was a
/// spike, the packets counted and the jitter.
type Exchange = (
    u32,
    u32,
    u32,
    u32,
    i32,
    u64,
    u64,
    Vec<u8>,
    u64,
    u64,
    u64,
    u64,
    bool,
    u64,
    u64,
);

#[proxy(
    interface = "org.freedesktop.timesync1.Manager",
    default_service = "org.freedesktop.timesync1",
    default_path = "/org/freedesktop/timesync1"
)]
trait Manager {
    #[zbus(property, name = "NTPMessage")]
    fn ntp_message(&self) -> zbus::Result<Exchange>;

    #[zbus(property, name = "ServerName")]
    fn server_name(&self) -> zbus::Result<String>;

    #[zbus(property, name = "ServerAddress")]
    fn server_address(&self) -> zbus::Result<(i32, Vec<u8>)>;

    #[zbus(property, name = "PollIntervalUSec")]
    fn poll_interval(&self) -> zbus::Result<u64>;

    #[zbus(property, name = "PollIntervalMinUSec")]
    fn poll_interval_min(&self) -> zbus::Result<u64>;

    #[zbus(property, name = "PollIntervalMaxUSec")]
    fn poll_interval_max(&self) -> zbus::Result<u64>;

    #[zbus(property, name = "SystemNTPServers")]
    fn system_servers(&self) -> zbus::Result<Vec<String>>;

    #[zbus(property, name = "LinkNTPServers")]
    fn link_servers(&self) -> zbus::Result<Vec<String>>;

    #[zbus(property, name = "RuntimeNTPServers")]
    fn runtime_servers(&self) -> zbus::Result<Vec<String>>;

    #[zbus(property, name = "FallbackNTPServers")]
    fn fallback_servers(&self) -> zbus::Result<Vec<String>>;

    #[zbus(property, name = "Frequency")]
    fn frequency(&self) -> zbus::Result<i64>;
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let Some(path) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("<3>usage: timekeeping FILE");
        return ExitCode::from(2);
    };
    match follow(&path).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("<3>{error}");
            ExitCode::FAILURE
        }
    }
}

async fn follow(path: &Path) -> Result<(), Box<dyn Error>> {
    let bus = Connection::system().await?;
    // Heard before anything is read, so no change and no leaving is missed.
    let owners = DBusProxy::new(&bus).await?;
    let mut comings = owners
        .receive_name_owner_changed_with_args(&[(0, TIMESYNC)])
        .await?;
    let properties = PropertiesProxy::builder(&bus)
        .destination(TIMESYNC)?
        .path("/org/freedesktop/timesync1")?
        .cache_properties(CacheProperties::No)
        .build()
        .await?;
    let mut changes = properties.receive_properties_changed().await?;
    // Read afresh each time, since what timesyncd says no change of would
    // otherwise be kept from when it was first read.
    let manager = ManagerProxy::builder(&bus)
        .cache_properties(CacheProperties::No)
        .build()
        .await?;
    let there = owners.name_has_owner(BusName::try_from(TIMESYNC)?).await?;
    said(
        path,
        if there {
            Some(read(&manager).await)
        } else {
            None
        },
    )?;
    let mine = InterfaceName::try_from(MANAGER)?;
    loop {
        tokio::select! {
            change = changes.next() => match change {
                Some(change) if change.args()?.interface_name() == &mine => {
                    said(path, Some(read(&manager).await))?;
                }
                Some(_) => {}
                None => return Err("the bus stopped saying timesyncd's changes".into()),
            },
            coming = comings.next() => match coming {
                Some(coming) => {
                    let back = coming.args()?.new_owner().is_some();
                    said(path, if back { Some(read(&manager).await) } else { None })?;
                }
                None => return Err("the bus stopped saying who owns timesyncd".into()),
            },
        }
    }
}

/// What timesyncd says now, its last exchange left out before its first.
async fn read(manager: &ManagerProxy<'_>) -> reading::Reading {
    let (family, bytes) = manager.server_address().await.unwrap_or_default();
    let said = Said {
        synchronised: Path::new(SYNCHRONISED).exists(),
        server: manager.server_name().await.unwrap_or_default(),
        address: reading::address(family, &bytes),
        poll: manager.poll_interval().await.unwrap_or_default(),
        poll_least: manager.poll_interval_min().await.unwrap_or_default(),
        poll_most: manager.poll_interval_max().await.unwrap_or_default(),
        frequency: manager.frequency().await.unwrap_or_default(),
        servers: Servers {
            system: manager.system_servers().await.unwrap_or_default(),
            link: manager.link_servers().await.unwrap_or_default(),
            runtime: manager.runtime_servers().await.unwrap_or_default(),
            fallback: manager.fallback_servers().await.unwrap_or_default(),
        },
    };
    let message = manager.ntp_message().await.ok().map(message_of);
    // An exchange not yet made has no times.
    said.reading(message.as_ref().filter(|message| message.destination != 0))
}

/// An exchange as D-Bus gives it, by its fields' names.
fn message_of(exchange: Exchange) -> Message {
    Message {
        leap: exchange.0,
        version: exchange.1,
        stratum: exchange.3,
        precision: exchange.4,
        root_delay: exchange.5,
        root_dispersion: exchange.6,
        reference: exchange.7,
        origin: exchange.8,
        receive: exchange.9,
        transmit: exchange.10,
        destination: exchange.11,
        packets: exchange.13,
        jitter: exchange.14,
    }
}

/// Writes what's read, or `{}` for nothing, whole, through a rename.
fn said(path: &Path, reading: Option<reading::Reading>) -> Result<(), Box<dyn Error>> {
    let text = match reading {
        Some(reading) => serde_json::to_string(&reading)?,
        None => "{}".to_owned(),
    };
    let new = path.with_extension("new");
    fs::write(&new, text)?;
    fs::rename(&new, path)?;
    Ok(())
}
