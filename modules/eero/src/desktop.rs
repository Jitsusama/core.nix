//! The desktop, over D-Bus: the notices the daemon tells Joel what changed
//! on the home network with (notices.rs), raised as disk-guard raises its
//! own, on the session bus's `org.freedesktop.Notifications`.

use std::collections::HashMap;
use std::time::Duration;

use zbus::blocking::Connection;
use zbus::blocking::connection::Builder;
use zbus::zvariant::Value;

use crate::notices::{Notice, Urgency};

/// How long a call waits for its answer. D-Bus calls otherwise wait for
/// ever, and a wedged notice server would stop the daemon's reads with it.
const PATIENCE: Duration = Duration::from_secs(2);

const NOTIFICATIONS: &str = "org.freedesktop.Notifications";

/// The name a notice is signed with, the glossary's for the home network,
/// and the icon the freedesktop names give a network of machines.
const SENDER: &str = "Home network";
const ICON: &str = "network-workgroup";

/// Raises notices, each in place of the last one about the same thing, so
/// an outage's notice is replaced by its end rather than left beside it.
/// The session bus is connected when first needed, and afresh after a
/// call fails.
#[derive(Default)]
pub struct Notifier {
    connection: Option<Connection>,
    /// The notice showing about each thing, by the id the server gave it.
    shown: HashMap<String, u32>,
}

impl Notifier {
    pub fn tell(&mut self, notice: &Notice) -> Result<(), String> {
        let connection = match self.connection.take() {
            Some(connection) => connection,
            None => Builder::session()
                .and_then(|builder| builder.method_timeout(PATIENCE).build())
                .map_err(|error| error.to_string())?,
        };
        let urgency: u8 = match notice.urgency {
            Urgency::Normal => 1,
            Urgency::Critical => 2,
        };
        let hints = HashMap::from([("urgency", Value::U8(urgency))]);
        let replaces = notice
            .about
            .as_ref()
            .and_then(|about| self.shown.get(about))
            .copied()
            .unwrap_or(0);
        let reply = connection
            .call_method(
                Some(NOTIFICATIONS),
                "/org/freedesktop/Notifications",
                Some(NOTIFICATIONS),
                "Notify",
                &(
                    SENDER,
                    replaces,
                    ICON,
                    notice.summary.as_str(),
                    notice.body.as_str(),
                    Vec::<&str>::new(),
                    hints,
                    -1i32,
                ),
            )
            .map_err(|error| error.to_string())?;
        let id: u32 = reply
            .body()
            .deserialize()
            .map_err(|error| error.to_string())?;
        self.connection = Some(connection);
        if let Some(about) = &notice.about {
            self.shown.insert(about.clone(), id);
        }
        Ok(())
    }
}
