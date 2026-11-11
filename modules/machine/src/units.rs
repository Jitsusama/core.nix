//! Restarting a user unit the shell asks for, as Spend's Start the Watch
//! does, through systemd's user bus rather than a spawned `systemctl`:
//! the shell spawns nothing to act (Y2YBTS, 2026-11-10). Only the units
//! named in [`RESTARTABLE`] are, so the socket can't be used to restart
//! anything else the session runs.
//!
//! And how a watcher's unit stands once it's fallen silent, asked of
//! systemd's bus as `systemctl show` would, on the session's for the
//! account's units and the system's for the machine's (status.rs).

use std::time::Duration;

use serde_json::{Value, json};

use crate::status::Unit;

/// The states a unit is running or on its way to running in, as
/// `systemctl` names them; any other is stopped.
const RUNNING: [&str; 5] = [
    "active",
    "activating",
    "deactivating",
    "reloading",
    "refreshing",
];

/// How long systemd is given to say how a unit stands. It answers from
/// what it holds, in a millisecond, and the status's reads wait on this.
const ASKING: Duration = Duration::from_secs(1);

/// The units the shell may restart, each the daemon behind a row that
/// offers to start it when it isn't answering: Spend's watch, and the
/// account's disk guard, which Disk's Start the Guard starts.
pub const RESTARTABLE: [&str; 2] = ["spend-watch.service", "disk-guard.service"];

/// How long systemd is given to queue the restart before the act gives up.
const QUEUEING: Duration = Duration::from_secs(10);

/// Why a unit wasn't restarted: no unit was named (`bad-act`), one not
/// in [`RESTARTABLE`] was (`not-allowed`), or systemd said no (`refused`),
/// with what to say of it.
#[derive(Debug, PartialEq, Eq)]
pub struct Unrestarted {
    pub code: &'static str,
    pub message: String,
}

fn unrestarted(code: &'static str, message: impl Into<String>) -> Unrestarted {
    Unrestarted {
        code,
        message: message.into(),
    }
}

/// Whether the shell may restart the unit named, refused before the bus
/// is reached where it may not.
pub fn allowed(unit: &str) -> Result<(), Unrestarted> {
    if unit.is_empty() {
        return Err(unrestarted("bad-act", "a restart needs its unit"));
    }
    if !RESTARTABLE.contains(&unit) {
        return Err(unrestarted(
            "not-allowed",
            format!("{unit} isn't a unit the shell may restart"),
        ));
    }
    Ok(())
}

/// Restarts the unit named, answering once systemd has queued the job,
/// as `systemctl --user restart --no-block` would, so a unit slow to
/// start holds nothing here.
pub fn restart(unit: &str) -> Result<(), Unrestarted> {
    allowed(unit)?;
    let refused = |error: zbus::Error| unrestarted("refused", error.to_string());
    let connection = zbus::blocking::connection::Builder::session()
        .and_then(|builder| builder.method_timeout(QUEUEING).build())
        .map_err(refused)?;
    let manager = zbus::blocking::Proxy::new(
        &connection,
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
    )
    .map_err(refused)?;
    manager
        .call::<_, _, zbus::zvariant::OwnedObjectPath>("RestartUnit", &(unit, "replace"))
        .map(|_| ())
        .map_err(refused)
}

/// How a unit stands, as `{state, result, stopped}`: its `ActiveState`
/// and `Result`, and whether that state is stopped. A unit that can't be
/// asked is taken as stopped, with nothing to say why, since its watcher
/// has fallen silent and nothing says it's coming back.
pub fn standing(unit: Unit) -> Value {
    let (state, result) = asked(unit).unwrap_or_default();
    json!({
        "stopped": !RUNNING.contains(&state.as_str()),
        "state": state,
        "result": result,
    })
}

fn asked(unit: Unit) -> zbus::Result<(String, String)> {
    let builder = if unit.account {
        zbus::blocking::connection::Builder::session()
    } else {
        zbus::blocking::connection::Builder::system()
    };
    let connection = builder?.method_timeout(ASKING).build()?;
    let path = format!("/org/freedesktop/systemd1/unit/{}", escaped(unit.name));
    // Each property is asked once, so none is kept and no match rule made.
    let property = |interface: &str, name: &str| -> zbus::Result<String> {
        let proxy: zbus::blocking::Proxy<'_> = zbus::blocking::proxy::Builder::new(&connection)
            .destination("org.freedesktop.systemd1")?
            .path(path.as_str())?
            .interface(interface)?
            .cache_properties(zbus::proxy::CacheProperties::No)
            .build()?;
        proxy.get_property::<String>(name)
    };
    Ok((
        property("org.freedesktop.systemd1.Unit", "ActiveState")?,
        property("org.freedesktop.systemd1.Service", "Result")?,
    ))
}

/// A unit's name as systemd's bus path holds it: every byte but a letter
/// or a digit as `_` and its two hex digits, and a leading digit too.
fn escaped(name: &str) -> String {
    let mut path = String::new();
    for (at, byte) in name.bytes().enumerate() {
        if byte.is_ascii_alphabetic() || (byte.is_ascii_digit() && at > 0) {
            path.push(char::from(byte));
        } else {
            path.push_str(&format!("_{byte:02x}"));
        }
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_units_bus_path_escapes_all_but_its_letters_and_digits() {
        assert_eq!(escaped("disk-guard.service"), "disk_2dguard_2eservice");
        assert_eq!(escaped("9p.mount"), "_39p_2emount");
    }

    #[test]
    fn a_unit_that_cant_be_asked_is_stopped_with_nothing_to_say_why() {
        // SAFETY: no other test in this binary reads the bus's address.
        unsafe { std::env::set_var("DBUS_SESSION_BUS_ADDRESS", "unix:path=/nowhere") };
        let unit = Unit {
            name: "disk-guard.service",
            account: true,
        };
        assert_eq!(
            standing(unit),
            json!({"stopped": true, "state": "", "result": ""})
        );
    }

    #[test]
    fn only_a_unit_named_may_be_restarted() {
        assert_eq!(allowed("spend-watch.service"), Ok(()));
        assert_eq!(allowed("").map_err(|no| no.code), Err("bad-act"));
        assert_eq!(
            allowed("sshd.service").map_err(|no| no.code),
            Err("not-allowed")
        );
        assert_eq!(
            restart("pipewire.service").map_err(|no| no.code),
            Err("not-allowed")
        );
    }
}
