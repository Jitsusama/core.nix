//! The desktop's notifications server on the session bus, version 1.2 of
//! the specification (`org.freedesktop.Notifications`), as the shell's
//! Quickshell served it: what a sender sends is handed to [`State`], and
//! the signals owed are sent from the daemon's loop, in the order owed.
//!
//! It asks for the name without taking it from an owner, so it never
//! pushes the shell's own server aside; with another owner, it says so
//! and stops.

use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, PoisonError};

use serde_json::{Map, Value};
use zbus::fdo::{RequestNameFlags, RequestNameReply};
use zbus::zvariant::{OwnedValue, Value as Variant};

use crate::daemon::{Sent, Signal, State};
use crate::picture::Pixels;

pub const NAME: &str = "org.freedesktop.Notifications";
pub const PATH: &str = "/org/freedesktop/Notifications";

/// What it says it does, in the order Quickshell said it: words, with
/// markup and links; actions; a picture; and replies in place.
const CAPABILITIES: [&str; 6] = [
    "body",
    "body-markup",
    "body-hyperlinks",
    "actions",
    "icon-static",
    "inline-reply",
];

/// The milliseconds since 1970, as every time held is.
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_millis()).unwrap_or(i64::MAX)
        })
}

/// The server, holding what every arrival changes and a word to the loop
/// that something did.
pub struct Server {
    pub state: Arc<Mutex<State>>,
    pub changed: Sender<()>,
}

#[zbus::interface(name = "org.freedesktop.Notifications")]
impl Server {
    #[expect(
        clippy::too_many_arguments,
        reason = "Notify takes the specification's eight arguments"
    )]
    fn notify(
        &self,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        expire_timeout: i32,
    ) -> u32 {
        let sent = Sent {
            app: app_name,
            replaces: replaces_id,
            icon: app_icon,
            summary,
            body,
            actions,
            pixels: pixels(&hints),
            hints: worded(hints),
            expire: expire_timeout,
        };
        let id = self
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .notify(sent, now_ms());
        let _ = self.changed.send(());
        id
    }

    fn close_notification(&self, id: u32) {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .withdraw(id, now_ms());
        let _ = self.changed.send(());
    }

    #[expect(
        clippy::unused_self,
        reason = "the bus calls each method on the server"
    )]
    fn get_capabilities(&self) -> Vec<String> {
        CAPABILITIES.iter().map(|one| (*one).to_owned()).collect()
    }

    #[expect(
        clippy::unused_self,
        reason = "the bus calls each method on the server"
    )]
    fn get_server_information(&self) -> (String, String, String, String) {
        (
            "notices".to_owned(),
            "jitsusama".to_owned(),
            env!("CARGO_PKG_VERSION").to_owned(),
            "1.2".to_owned(),
        )
    }
}

/// Serves the server on the session bus, once the name is its own.
pub fn serve(server: Server) -> Result<zbus::blocking::Connection, String> {
    let connection = zbus::blocking::connection::Builder::session()
        .and_then(|builder| builder.serve_at(PATH, server))
        .and_then(zbus::blocking::connection::Builder::build)
        .map_err(|why| format!("can't reach the session bus: {why}"))?;
    // Neither replacing an owner nor queueing behind one, so the name is
    // only ever had by one server.
    match connection.request_name_with_flags(NAME, RequestNameFlags::DoNotQueue.into()) {
        Ok(RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner) => Ok(connection),
        // zbus says an owner's refusal as an error of its own.
        Ok(_) | Err(zbus::Error::NameTaken) => Err(format!("{NAME} is another's")),
        Err(why) => Err(format!("can't ask for {NAME}: {why}")),
    }
}

/// Sends a signal owed, to every sender listening.
pub fn send(connection: &zbus::blocking::Connection, signal: &Signal) -> zbus::Result<()> {
    let emit = |member: &str, body: &(u32, &str)| {
        connection.emit_signal(None::<&str>, PATH, NAME, member, body)
    };
    match signal {
        Signal::Closed(id, why) => {
            connection.emit_signal(None::<&str>, PATH, NAME, "NotificationClosed", &(*id, *why))
        }
        Signal::Token(id, token) => emit("ActivationToken", &(*id, token.as_str())),
        Signal::Invoked(id, action) => emit("ActionInvoked", &(*id, action.as_str())),
        Signal::Replied(id, text) => emit("NotificationReplied", &(*id, text.as_str())),
    }
}

/// The hints that are words, numbers or a yes or no, as JSON; a picture
/// sent as pixels is read apart, and nothing else is a hint a surface
/// reads.
fn worded(hints: HashMap<String, OwnedValue>) -> Map<String, Value> {
    hints
        .into_iter()
        .filter_map(|(name, value)| Some((name, json_of(&value)?)))
        .collect()
}

fn json_of(value: &Variant) -> Option<Value> {
    Some(match value {
        Variant::Str(text) => Value::String(String::from(&**text)),
        Variant::Bool(yes) => Value::Bool(*yes),
        Variant::U8(n) => Value::from(*n),
        Variant::I16(n) => Value::from(*n),
        Variant::U16(n) => Value::from(*n),
        Variant::I32(n) => Value::from(*n),
        Variant::U32(n) => Value::from(*n),
        Variant::I64(n) => Value::from(*n),
        Variant::U64(n) => Value::from(*n),
        Variant::F64(n) => Value::from(*n),
        Variant::Value(inner) => json_of(inner)?,
        _ => return None,
    })
}

/// A picture sent as pixels, by any of the three names the specification
/// has given its hint.
fn pixels(hints: &HashMap<String, OwnedValue>) -> Option<Pixels> {
    let value = ["image-data", "image_data", "icon_data"]
        .iter()
        .find_map(|name| hints.get(*name))?;
    let Variant::Structure(structure) = &**value else {
        return None;
    };
    let fields = structure.fields();
    let number = |at: usize| match fields.get(at) {
        Some(Variant::I32(n)) => Some(*n),
        _ => None,
    };
    let alpha = match fields.get(3) {
        Some(Variant::Bool(yes)) => *yes,
        _ => return None,
    };
    let Some(Variant::Array(bytes)) = fields.get(6) else {
        return None;
    };
    let data = bytes
        .inner()
        .iter()
        .map(|byte| match byte {
            Variant::U8(byte) => Some(*byte),
            _ => None,
        })
        .collect::<Option<Vec<u8>>>()?;
    Some(Pixels {
        width: number(0)?,
        height: number(1)?,
        stride: number(2)?,
        alpha,
        bits: number(4)?,
        channels: number(5)?,
        data,
    })
}
