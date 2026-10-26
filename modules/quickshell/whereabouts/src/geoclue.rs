//! GeoClue, on the system bus: a client of its own, asking where the laptop
//! is to a city's accuracy, and what each position it finds says.

use std::future;

use futures_util::{Stream, StreamExt};
use zbus::fdo::DBusProxy;
use zbus::proxy::CacheProperties;
use zbus::zvariant::OwnedObjectPath;
use zbus::{Connection, proxy};

use crate::fix::{FAR_ENOUGH, Fix};

const GEOCLUE: &str = "org.freedesktop.GeoClue2";

/// The name GeoClue knows this program by, which the machine's GeoClue
/// settings allow (the shell's nixos.nix), so no agent is asked.
const DESKTOP_ID: &str = "whereabouts";

/// GeoClue's accuracy for a city, the least the weather needs, so the
/// laptop is never placed closer than that.
const CITY: u32 = 4;

#[proxy(
    interface = "org.freedesktop.GeoClue2.Manager",
    default_service = "org.freedesktop.GeoClue2",
    default_path = "/org/freedesktop/GeoClue2/Manager"
)]
trait Manager {
    fn create_client(&self) -> zbus::Result<OwnedObjectPath>;
}

#[proxy(
    interface = "org.freedesktop.GeoClue2.Client",
    default_service = "org.freedesktop.GeoClue2"
)]
pub trait Client {
    fn start(&self) -> zbus::Result<()>;

    /// The latest position, or `/` before there is one.
    #[zbus(property)]
    fn location(&self) -> zbus::Result<OwnedObjectPath>;

    #[zbus(property)]
    fn set_desktop_id(&self, id: &str) -> zbus::Result<()>;

    #[zbus(property)]
    fn set_requested_accuracy_level(&self, level: u32) -> zbus::Result<()>;

    /// The least move, in metres, GeoClue says anything of.
    #[zbus(property)]
    fn set_distance_threshold(&self, metres: u32) -> zbus::Result<()>;

    /// A new position, at `now`, replacing the one at `was`.
    #[zbus(signal)]
    fn location_updated(&self, was: OwnedObjectPath, now: OwnedObjectPath) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.freedesktop.GeoClue2.Location",
    default_service = "org.freedesktop.GeoClue2"
)]
trait Location {
    #[zbus(property)]
    fn latitude(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn longitude(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn accuracy(&self) -> zbus::Result<f64>;

    /// Seconds and microseconds since the epoch.
    #[zbus(property)]
    fn timestamp(&self) -> zbus::Result<(u64, u64)>;
}

/// A client of GeoClue's, set up and started. GeoClue says nothing of a
/// move smaller than the least that matters, so the laptop sitting still
/// wakes nothing here.
pub async fn client(bus: &Connection) -> zbus::Result<ClientProxy<'static>> {
    let path = ManagerProxy::new(bus).await?.create_client().await?;
    let client = ClientProxy::builder(bus)
        .path(path)?
        .cache_properties(CacheProperties::No)
        .build()
        .await?;
    client.set_desktop_id(DESKTOP_ID).await?;
    client.set_requested_accuracy_level(CITY).await?;
    client.set_distance_threshold(FAR_ENOUGH).await?;
    Ok(client)
}

/// Ends when GeoClue leaves the bus, taking every client with it, so the
/// program stops and systemd starts it again with a client anew.
pub async fn gone(bus: &Connection) -> zbus::Result<impl Stream<Item = ()> + Unpin> {
    let owners = DBusProxy::new(bus).await?;
    let changes = owners
        .receive_name_owner_changed_with_args(&[(0, GEOCLUE)])
        .await?;
    Ok(changes
        .filter(|change| future::ready(change.args().is_ok_and(|args| args.new_owner().is_none())))
        .map(drop))
}

/// What the position at `path` says, or nothing for none yet, or one
/// GeoClue no longer has or that names no place on the Earth.
pub async fn fix(bus: &Connection, path: OwnedObjectPath) -> Option<Fix> {
    if path.as_str() == "/" {
        return None;
    }
    let location = LocationProxy::builder(bus)
        .path(path)
        .ok()?
        .cache_properties(CacheProperties::No)
        .build()
        .await
        .ok()?;
    Fix::new(
        location.latitude().await.ok()?,
        location.longitude().await.ok()?,
        location.accuracy().await.ok()?,
        location.timestamp().await.ok()?.0,
    )
}
