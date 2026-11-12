//! What the broker asks of the account's systemd, over the session bus:
//! a scope started around a process, and a unit's standing, a fixed set
//! of its properties and nothing that holds a command line.

use std::os::fd::BorrowedFd;
use std::time::Duration;

use serde_json::{Map, Value, json};
use zbus::blocking::{Connection, Proxy, connection, proxy};
use zbus::zvariant::{self, Fd, OwnedObjectPath};

const DESTINATION: &str = "org.freedesktop.systemd1";
const MANAGER_PATH: &str = "/org/freedesktop/systemd1";
const MANAGER: &str = "org.freedesktop.systemd1.Manager";
const UNIT: &str = "org.freedesktop.systemd1.Unit";

/// How long one call to systemd may take before the ask is refused.
const ASKING: Duration = Duration::from_secs(5);

/// What a status says of every unit, from systemd's Unit interface.
const UNIT_PROPERTIES: &[&str] = &[
    "Id",
    "Description",
    "LoadState",
    "ActiveState",
    "SubState",
    "StateChangeTimestamp",
    "ActiveEnterTimestamp",
    "InactiveEnterTimestamp",
];

/// What a status says of a unit of each kind, from that kind's own
/// interface. None holds a command line, an environment or a credential.
const KIND_PROPERTIES: &[(&str, &str, &[&str])] = &[
    (
        ".service",
        "org.freedesktop.systemd1.Service",
        &["Slice", "Result", "MainPID", "ExecMainStatus", "NRestarts"],
    ),
    (
        ".scope",
        "org.freedesktop.systemd1.Scope",
        &["Slice", "Result"],
    ),
    (".slice", "org.freedesktop.systemd1.Slice", &["Slice"]),
    (
        ".socket",
        "org.freedesktop.systemd1.Socket",
        &["Slice", "Result"],
    ),
    (
        ".timer",
        "org.freedesktop.systemd1.Timer",
        &["Result", "NextElapseUSecRealtime", "LastTriggerUSec"],
    ),
    (
        ".mount",
        "org.freedesktop.systemd1.Mount",
        &["Slice", "Result"],
    ),
];

/// The session bus, where the account's systemd answers.
pub fn session() -> zbus::Result<Connection> {
    connection::Builder::session()?
        .method_timeout(ASKING)
        .build()
}

/// Starts the scope `name` in `slice` around the process `pidfd` stands
/// for, handed over as the descriptor itself (`PIDFDs`) so systemd moves
/// that process and no other given its pid. The broker sets every property
/// the scope has; the process asking sets none.
pub fn start_scope(
    bus: &Connection,
    name: &str,
    slice: &str,
    pidfd: BorrowedFd<'_>,
) -> zbus::Result<()> {
    let manager = Proxy::new(bus, DESTINATION, MANAGER_PATH, MANAGER)?;
    let properties: Vec<(&str, zvariant::Value<'_>)> = vec![
        (
            "Description",
            zvariant::Value::from("A fenced pi command's own work"),
        ),
        ("Slice", zvariant::Value::from(slice)),
        // Gone once it ends, failed or not, so its name is free for the
        // next process given the same pid.
        ("CollectMode", zvariant::Value::from("inactive-or-failed")),
        ("PIDFDs", zvariant::Value::from(vec![Fd::from(pidfd)])),
    ];
    let auxiliary: Vec<(&str, Vec<(&str, zvariant::Value<'_>)>)> = Vec::new();
    manager
        .call::<_, _, OwnedObjectPath>("StartTransientUnit", &(name, "fail", properties, auxiliary))
        .map(|_| ())
}

/// How `unit` stands: the properties every unit has, and its kind's own,
/// each as JSON, or why it couldn't be asked.
pub fn status(bus: &Connection, unit: &str) -> zbus::Result<Value> {
    let manager = Proxy::new(bus, DESTINATION, MANAGER_PATH, MANAGER)?;
    // LoadUnit, as systemctl show does, so a unit not running is told as
    // it stands on disk rather than as missing.
    let path: OwnedObjectPath = manager.call("LoadUnit", &(unit,))?;
    let mut said = Map::new();
    let mut read = |interface: &str, names: &[&str]| -> zbus::Result<()> {
        let asked: Proxy<'_> = proxy::Builder::new(bus)
            .destination(DESTINATION)?
            .path(path.as_str())?
            .interface(interface)?
            .cache_properties(proxy::CacheProperties::No)
            .build()?;
        for name in names {
            let value = asked.get_property::<zvariant::OwnedValue>(name)?;
            said.insert((*name).to_owned(), plain(&value));
        }
        Ok(())
    };
    read(UNIT, UNIT_PROPERTIES)?;
    if let Some((_, interface, names)) =
        KIND_PROPERTIES.iter().find(|(end, ..)| unit.ends_with(end))
    {
        read(interface, names)?;
    }
    Ok(Value::Object(said))
}

/// A property as JSON: its string, number or truth, and null for any
/// other shape, which none of those asked has.
fn plain(value: &zvariant::Value<'_>) -> Value {
    match value {
        zvariant::Value::Str(text) => json!(text.as_str()),
        zvariant::Value::U64(number) => json!(number),
        zvariant::Value::U32(number) => json!(number),
        zvariant::Value::I32(number) => json!(number),
        zvariant::Value::Bool(truth) => json!(truth),
        _ => Value::Null,
    }
}
