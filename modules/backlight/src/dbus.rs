//! The daemon's side of D-Bus: logind's session, which sets the backlight,
//! and its manager, which says where the lid is and when the machine
//! sleeps; wl-gammarelay-rs, which sets the screens' warmth; and the waits
//! on what each of them says.

use std::pin::Pin;

use tokio::sync::watch;
use zbus::export::futures_core::Stream;
use zbus::fdo::DBusProxy;
use zbus::proxy::{CacheProperties, OwnerChangedStream, PropertyStream};
use zbus::zvariant::Value;
use zbus::{Connection, Message, proxy};

use crate::ask;
use crate::screen::Screen;
use crate::warmth::{WHITE, Warmth};

/// The session this daemon runs in, which logind lets set its backlight.
#[proxy(
    interface = "org.freedesktop.login1.Session",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1/session/auto"
)]
pub(crate) trait Session {
    fn set_brightness(&self, subsystem: &str, name: &str, brightness: u32) -> zbus::Result<()>;
}

/// logind itself, which says whether the lid is shut and when that changes.
#[proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
pub(crate) trait Manager {
    #[zbus(property)]
    fn lid_closed(&self) -> zbus::Result<bool>;

    /// Said as the machine goes to sleep, `start` true, and as it wakes.
    #[zbus(signal)]
    fn prepare_for_sleep(&self, start: bool) -> zbus::Result<()>;
}

/// wl-gammarelay-rs, which sets every screen's warmth through niri.
#[proxy(
    interface = "rs.wl.gammarelay",
    default_service = "rs.wl-gammarelay",
    default_path = "/"
)]
pub(crate) trait Relay {
    #[zbus(property)]
    fn temperature(&self) -> zbus::Result<u16>;
}

/// Why the panel's warmth wasn't set.
#[derive(Debug, PartialEq)]
pub(crate) enum Refused {
    /// The relay holds no object for the panel, since niri isn't showing
    /// it, as while its lid is shut beside another screen.
    NoPanel,
    /// Anything else, in the words it came in.
    Else(String),
}

/// Sets the panel's warmth, or says why it wasn't.
pub(crate) async fn set_warmth(relay: &RelayProxy<'_>, kelvin: u16) -> Result<(), Refused> {
    let relay = relay.inner();
    let reply = relay
        .connection()
        .call_method(
            Some(relay.destination().clone()),
            relay.path().clone(),
            Some("org.freedesktop.DBus.Properties"),
            "Set",
            &(
                relay.interface().clone(),
                "Temperature",
                Value::from(kelvin),
            ),
        )
        .await
        .map_err(|error| refusal(&error.to_string()))?;
    answered(&reply)
}

/// What a reply to a set of the warmth says. wl-gammarelay-rs's D-Bus
/// crate, rustbus, sends a refusal as an ordinary reply holding its reason
/// rather than as an error, and a set's own reply holds nothing, so any
/// words in it are the refusal. Read as a set's reply, zbus would call it
/// a signature mismatch and lose why. Newer rustbus sends an error, which
/// zbus gives as one, so `refusal` reads both.
fn answered(reply: &Message) -> Result<(), Refused> {
    match reply.body().deserialize::<&str>() {
        Ok(reason) => Err(refusal(reason)),
        Err(_) => Ok(()),
    }
}

/// Why a set was refused, from the relay's words, which name the object it
/// has none of when niri isn't showing the panel.
fn refusal(reason: &str) -> Refused {
    if reason.contains("are accepted for object") {
        Refused::NoPanel
    } else {
        Refused::Else(reason.to_owned())
    }
}

/// Waits for the screens' warmth to be set by a program that just started,
/// or forever once the bus stops saying.
pub(crate) async fn relay_started(owners: &mut Option<Pin<Box<OwnerChangedStream<'_>>>>) -> bool {
    let Some(stream) = owners else {
        return std::future::pending().await;
    };
    if let Some(owner) = std::future::poll_fn(|cx| stream.as_mut().poll_next(cx)).await {
        owner.is_some()
    } else {
        *owners = None;
        std::future::pending().await
    }
}

/// Waits for Joel to say whether the warmth follows the room, or forever
/// once nothing can.
pub(crate) async fn switched(said: &mut Option<watch::Receiver<bool>>) -> bool {
    let Some(heard) = said else {
        return std::future::pending().await;
    };
    if heard.changed().await.is_ok() {
        *heard.borrow_and_update()
    } else {
        *said = None;
        std::future::pending().await
    }
}

/// Waits for the machine to wake from sleep, however it slept: an idle
/// suspend or a suspend asked for wakes with no lid moving. Once logind
/// stops saying, as when the system bus goes, it fails, so the daemon
/// stops and systemd starts it again hearing it afresh.
pub(crate) async fn woke(sleeps: &mut Pin<Box<PrepareForSleepStream>>) -> zbus::Result<()> {
    loop {
        let Some(said) = std::future::poll_fn(|cx| sleeps.as_mut().poll_next(cx)).await else {
            return Err(zbus::Error::Failure(
                "logind stopped saying when it sleeps".into(),
            ));
        };
        if said.args().is_ok_and(|args| !*args.start()) {
            return Ok(());
        }
    }
}

/// Waits for the lid to open or shut. Once logind stops saying, as when
/// the system bus goes, it fails, as `woke` does.
pub(crate) async fn lid_moved(lid: &mut Pin<Box<PropertyStream<'_, bool>>>) -> zbus::Result<bool> {
    match std::future::poll_fn(|cx| lid.as_mut().poll_next(cx)).await {
        Some(changed) => changed.get().await,
        None => Err(zbus::Error::Failure(
            "logind stopped saying where the lid is".into(),
        )),
    }
}

/// Each time what sets the screens' warmth starts.
pub(crate) type Started = Option<Pin<Box<OwnerChangedStream<'static>>>>;

/// What sets the built-in panel's warmth, on the session's bus, with the
/// warmth it has and its starts heard. Only that panel is warmed, as True
/// Tone does: the sensor sits beside it, and another screen's white is its
/// own. None when the panel can't be named or the bus can't be reached.
pub(crate) async fn relay(screen: &Screen) -> (Option<(Warmth, RelayProxy<'static>)>, Started) {
    let Some(connector) = screen.connector() else {
        eprintln!("<5>{} names no panel, so no screen is warmed", screen.name);
        return (None, None);
    };
    // wl-gammarelay-rs names each screen's object for its connector, with
    // a dash turned into an underscore.
    let path = format!("/outputs/{}", connector.replace('-', "_"));
    let bus = match Connection::session().await {
        Ok(bus) => bus,
        Err(error) => {
            eprintln!("<4>can't reach the session's bus to warm the screens: {error}");
            return (None, None);
        }
    };
    // Uncached, since the program may start after the daemon, or again.
    let built = match RelayProxy::builder(&bus).path(path) {
        Ok(builder) => builder.cache_properties(CacheProperties::No).build().await,
        Err(error) => Err(error),
    };
    let relay = match built {
        Ok(relay) => relay,
        Err(error) => {
            eprintln!("<4>can't ask for the screens' warmth: {error}");
            return (None, None);
        }
    };
    // Heard before it's read, so a start between the two isn't missed.
    let started = relay
        .inner()
        .receive_owner_changed()
        .await
        .ok()
        .map(Box::pin);
    // Read only from a relay already running: asking would start one, which
    // the bus does on the first warmth set, and one starts at the screen's
    // own white.
    let running = match DBusProxy::new(&bus).await {
        Ok(owners) => owners
            .name_has_owner(relay.inner().destination().clone())
            .await
            .unwrap_or(false),
        Err(_) => false,
    };
    let shown = if running {
        ask::twice(ask::MOMENT, || relay.temperature())
            .await
            .and_then(Result::ok)
            .unwrap_or(WHITE)
    } else {
        WHITE
    };
    (Some((Warmth::new(shown), relay)), started)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reply to a set on the panel's object, holding what's given.
    fn reply<B>(body: &B) -> Message
    where
        B: zbus::export::serde::Serialize + zbus::zvariant::DynamicType,
    {
        let set = Message::method_call("/outputs/eDP_1", "Set")
            .and_then(|call| call.destination("rs.wl-gammarelay"))
            .and_then(|call| call.build(&()))
            .expect("a set is built");
        Message::method_return(&set.header())
            .and_then(|answer| answer.build(body))
            .expect("its reply is built")
    }

    #[test]
    fn an_empty_reply_is_the_warmth_set() {
        assert_eq!(answered(&reply(&())), Ok(()), "a set's reply holds nothing");
    }

    #[test]
    fn a_reply_holding_words_is_the_relay_refusing() {
        let said = "Invalid arguments for calls to org.freedesktop.DBus.Properties.Set";
        assert_eq!(
            answered(&reply(&(said,))),
            Err(Refused::Else(said.to_owned())),
            "wl-gammarelay-rs refuses with an ordinary reply holding why"
        );
    }

    #[test]
    fn a_refusal_naming_no_object_is_the_panel_not_showing() {
        let said = "No calls to org.freedesktop.DBus.Properties.Set are accepted for object \
                    /outputs/eDP_1";
        assert_eq!(
            answered(&reply(&(said,))),
            Err(Refused::NoPanel),
            "a panel niri doesn't show has no object to set"
        );
    }

    #[test]
    fn an_error_naming_no_object_is_the_panel_not_showing_too() {
        let said = "org.freedesktop.DBus.Error.UnknownMethod: No calls to \
                    org.freedesktop.DBus.Properties.Set are accepted for object /outputs/eDP_1";
        assert_eq!(
            refusal(said),
            Refused::NoPanel,
            "a relay on a newer rustbus refuses with an error, read the same"
        );
    }
}
