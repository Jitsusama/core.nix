//! The desktop, over D-Bus: whether the person has asked to save power,
//! and the one notice the user daemon keeps them told with.

use crate::notices::{Change, Urgency};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use zbus::blocking::Connection;
use zbus::blocking::connection::Builder;
use zbus::zvariant::{OwnedValue, Value};

/// How long any call waits for its answer. D-Bus calls otherwise wait for
/// ever, and a wedged desktop service would stop the daemon's loop, the
/// urgent passes with it.
const PATIENCE: Duration = Duration::from_secs(2);

/// Connects as `builder` says, every call giving up after `PATIENCE`.
/// Every connection the guard makes goes through here.
pub fn patient(builder: Builder<'_>) -> zbus::Result<Connection> {
    builder.method_timeout(PATIENCE).build()
}

/// power-profiles-daemon's names for itself: the present one, then the
/// one it had before joining UPower, which older systems still use.
const PROFILES: [(&str, &str, &str); 2] = [
    (
        "org.freedesktop.UPower.PowerProfiles",
        "/org/freedesktop/UPower/PowerProfiles",
        "org.freedesktop.UPower.PowerProfiles",
    ),
    (
        "net.hadess.PowerProfiles",
        "/net/hadess/PowerProfiles",
        "net.hadess.PowerProfiles",
    ),
];

/// Whether power-profiles-daemon's profile is power-saver. The firmware's
/// own platform profile isn't read, since it can disagree with what the
/// person chose. No daemon to ask means no choice made, so nothing saved.
pub fn saver(system: &Connection) -> bool {
    PROFILES.iter().find_map(|&(name, path, interface)| {
        let reply = system
            .call_method(
                Some(name),
                path,
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &(interface, "ActiveProfile"),
            )
            .ok()?;
        let value: OwnedValue = reply.body().deserialize().ok()?;
        String::try_from(value).ok()
    }) == Some("power-saver".to_owned())
}

/// How long a read power profile is trusted, since it changes only when
/// the person changes it and each step would otherwise ask.
const PROFILE_FRESH: Duration = Duration::from_secs(30);

/// The desktop over the real buses: the profile from the system bus, the
/// notice on the session bus when the config asks for notices. A bus
/// that can't be reached, or stops answering, is connected afresh when
/// next needed.
pub struct Buses {
    system: Option<Connection>,
    profile: Option<(Instant, bool)>,
    notify: bool,
    notifier: Option<Notifier>,
}

/// What the desktop is to its daemon, telling only when `notify`.
pub fn desk(notify: bool) -> Buses {
    Buses::new(notify)
}

impl Buses {
    pub fn new(notify: bool) -> Buses {
        Buses {
            system: None,
            profile: None,
            notify,
            notifier: None,
        }
    }
}

impl crate::serve::Desk for Buses {
    fn saving(&mut self) -> bool {
        if let Some((at, saving)) = self.profile
            && at.elapsed() < PROFILE_FRESH
        {
            return saving;
        }
        if self.system.is_none() {
            self.system = Builder::system().and_then(patient).ok();
        }
        let saving = self.system.as_ref().is_some_and(saver);
        self.profile = Some((Instant::now(), saving));
        saving
    }

    fn tell(&mut self, change: &Change) -> Result<(), String> {
        if !self.notify {
            return Ok(());
        }
        if self.notifier.is_none() {
            let session = Builder::session().and_then(patient);
            self.notifier = Some(Notifier::new(session.map_err(|error| error.to_string())?));
        }
        let told = self.notifier.as_mut().map(|notifier| notifier.tell(change));
        if let Some(Err(error)) = told {
            self.notifier = None;
            return Err(error.to_string());
        }
        Ok(())
    }
}

const NOTIFICATIONS: &str = "org.freedesktop.Notifications";

/// Shows, replaces and withdraws one notice, so the person is never left
/// a pile of stale ones.
pub struct Notifier {
    connection: Connection,
    /// The notice showing, by the id the server gave it.
    shown: Option<u32>,
}

impl Notifier {
    pub fn new(connection: Connection) -> Notifier {
        Notifier {
            connection,
            shown: None,
        }
    }

    pub fn tell(&mut self, change: &Change) -> zbus::Result<()> {
        match change {
            Change::Show(notice) => {
                let urgency: u8 = match notice.urgency {
                    Urgency::Normal => 1,
                    Urgency::Critical => 2,
                };
                let hints = HashMap::from([("urgency", Value::U8(urgency))]);
                let reply = self.connection.call_method(
                    Some(NOTIFICATIONS),
                    "/org/freedesktop/Notifications",
                    Some(NOTIFICATIONS),
                    "Notify",
                    &(
                        "Disk guard",
                        self.shown.unwrap_or(0),
                        "drive-harddisk",
                        notice.summary.as_str(),
                        notice.body.as_str(),
                        Vec::<&str>::new(),
                        hints,
                        -1i32,
                    ),
                )?;
                self.shown = Some(reply.body().deserialize()?);
            }
            Change::Withdraw => {
                if let Some(id) = self.shown.take() {
                    self.connection.call_method(
                        Some(NOTIFICATIONS),
                        "/org/freedesktop/Notifications",
                        Some(NOTIFICATIONS),
                        "CloseNotification",
                        &id,
                    )?;
                }
            }
        }
        Ok(())
    }
}
