//! What the system bus's services say, each asked on a thread of its
//! own only while its kind is watched: power-profiles-daemon, `UPower`,
//! logind and its sleeps, fwupd, and each disk's health of `UDisks`.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::hearing::heard;
use crate::making::hand_over;
use crate::reading::{due, due_while_watched};
use crate::{Daemon, Shared, lock};
use machine::interface::kind_of;
use machine::{disk, firmware, now_ms, rounded};
use serde_json::{Value, json};

/// How often each disk's health is asked while disks are watched.
const HEALTH_EVERY: Duration = Duration::from_secs(60);
/// The longest a call to a bus service may take, so one that hangs can't
/// hold its thread.
const BUS_WAIT: Duration = Duration::from_secs(10);
/// How often `UPower` is asked while supplies are watched; it works its
/// times out afresh no more often than this.
const UPOWER_EVERY: Duration = Duration::from_secs(5);
/// How often fwupd is asked while firmware is watched, unless it says
/// something changed first.
const FIRMWARE_EVERY: Duration = Duration::from_secs(60);
/// How often logind is asked while `awake` is watched.
const LOGIND_EVERY: Duration = Duration::from_secs(2);
const PROFILES: &str = "org.freedesktop.UPower.PowerProfiles";
const PROFILES_PATH: &str = "/org/freedesktop/UPower/PowerProfiles";

/// How long the sleep listener waits to try the bus again after it
/// couldn't reach it, or its connection ended.
const SLEEPS_AGAIN: Duration = Duration::from_secs(60);

/// A proxy to one interface of a bus service that keeps none of its
/// properties, since each is asked afresh on every read: one that kept
/// them would ask for all of them and add a match rule each time it's
/// made.
fn proxy<'a>(
    connection: &zbus::blocking::Connection,
    service: &'a str,
    path: impl TryInto<zbus::zvariant::ObjectPath<'a>, Error: Into<zbus::Error>>,
    interface: &'a str,
) -> zbus::Result<zbus::blocking::Proxy<'a>> {
    zbus::blocking::proxy::Builder::new(connection)
        .destination(service)?
        .path(path)?
        .interface(interface)?
        .cache_properties(zbus::proxy::CacheProperties::No)
        .build()
}

/// A connection to the system bus whose calls give up after `BUS_WAIT`,
/// so a service that hangs can't hold its loop.
fn system_bus() -> zbus::Result<zbus::blocking::Connection> {
    zbus::blocking::connection::Builder::system()
        .and_then(|builder| builder.method_timeout(BUS_WAIT).build())
}

/// Hears power-profiles-daemon from the daemon's start, as long as it
/// runs: what it's on, offers and is held to, asked again each time it
/// says one changed or it starts again, and told at once. It waits on the
/// bus alone, so it wakes for nothing else.
pub(crate) fn profile_loop(shared: &Shared) {
    loop {
        if let Ok(connection) = system_bus() {
            profiles_heard(&connection, shared);
        }
        std::thread::sleep(SLEEPS_AGAIN);
    }
}

/// Asks power-profiles-daemon what it's on now and each time it says
/// something changed, or a new one takes its name, until the connection
/// ends.
fn profiles_heard(connection: &zbus::blocking::Connection, shared: &Shared) {
    let Ok(daemon) = proxy(connection, PROFILES, PROFILES_PATH, PROFILES) else {
        return;
    };
    let Ok(properties) = zbus::blocking::fdo::PropertiesProxy::builder(connection)
        .destination(PROFILES)
        .and_then(|builder| builder.path(PROFILES_PATH))
        .and_then(zbus::blocking::proxy::Builder::build)
    else {
        return;
    };
    let (Ok(changes), Ok(owners)) = (
        properties.receive_properties_changed(),
        daemon.receive_owner_changed(),
    ) else {
        return;
    };
    let ask = || {
        let said = power_profiles(&daemon);
        lock(&shared.daemon).profile = said;
        heard(shared, &["profile"]);
    };
    ask();
    std::thread::scope(|scope| {
        scope.spawn(|| owners.for_each(|_| ask()));
        changes.for_each(|_| ask());
    });
}

/// What power-profiles-daemon says: the profile it's on, those it
/// offers, each hold a program has on one, with why, and why the
/// performance profile's slowed, if it is; null while it doesn't answer.
fn power_profiles(daemon: &zbus::blocking::Proxy<'_>) -> Value {
    type Dicts = Vec<HashMap<String, zbus::zvariant::OwnedValue>>;
    let Ok(active) = daemon.get_property::<String>("ActiveProfile") else {
        return Value::Null;
    };
    let field = |dict: &HashMap<String, zbus::zvariant::OwnedValue>, name: &str| {
        dict.get(name).map_or(Value::Null, |value| plain(value))
    };
    let profiles = daemon.get_property::<Dicts>("Profiles").ok().map(|all| {
        all.iter()
            .map(|profile| field(profile, "Profile"))
            .collect::<Vec<_>>()
    });
    let holds = daemon
        .get_property::<Dicts>("ActiveProfileHolds")
        .ok()
        .map(|all| {
            all.iter()
                .map(|hold| {
                    json!({
                        "profile": field(hold, "Profile"),
                        "app": field(hold, "ApplicationId"),
                        "why": field(hold, "Reason"),
                    })
                })
                .collect::<Vec<_>>()
        });
    // Empty while it isn't slowed.
    let degraded = daemon
        .get_property::<String>("PerformanceDegraded")
        .ok()
        .filter(|why| !why.is_empty());
    json!({"active": active, "profiles": profiles, "holds": holds, "degraded": degraded})
}

/// Lets go of what each light sensor last heard, keeping the sensor and
/// its scale, which don't change.
pub(crate) fn unheard(daemon: &mut Daemon) {
    let forget = |record: &mut Value| {
        record["raw"] = Value::Null;
        record["lux"] = Value::Null;
    };
    daemon.light.iter_mut().for_each(forget);
    daemon
        .held
        .iter_mut()
        .filter(|(key, _)| kind_of(key) == Some("light"))
        .for_each(|(_, record)| forget(record));
}

/// Asks `UPower` what it works out of each supply every few seconds while
/// supplies are watched: its state, and the battery's time to empty or
/// full and the rate it's changing at, by the supply's name.
pub(crate) fn upower_loop(shared: &Shared) {
    let mut bus: Option<zbus::blocking::Connection> = None;
    let mut last = None;
    loop {
        due(shared, "supply", UPOWER_EVERY, &mut last);
        if bus.is_none() {
            bus = system_bus().ok();
        }
        let said = bus.as_ref().map(upower).unwrap_or_default();
        let mut daemon = lock(&shared.daemon);
        daemon.upower = said;
    }
}

/// Asks logind every two seconds while `awake` is watched whether the
/// session is idle and what holds sleep off, with when the last sleep
/// began and ended as its signal said.
pub(crate) fn logind_loop(shared: &Shared) {
    let mut bus: Option<zbus::blocking::Connection> = None;
    let mut last = None;
    loop {
        due(shared, "awake", LOGIND_EVERY, &mut last);
        if bus.is_none() {
            bus = system_bus().ok();
        }
        let slept = lock(&lock(&shared.daemon).slept).clone();
        let said = bus
            .as_ref()
            .map_or(Value::Null, |connection| logind(connection, &slept));
        lock(&shared.daemon).logind = said;
    }
}

fn login1(connection: &zbus::blocking::Connection) -> zbus::Result<zbus::blocking::Proxy<'_>> {
    proxy(
        connection,
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
    )
}

/// Listens for logind's `PrepareForSleep` from the daemon's start, as long
/// as it runs, so the last sleep is known however soon after a wake it's
/// asked (Keep Awake's Last Slept): a sleep happens while nothing watches.
pub(crate) fn sleeps_loop(slept: &Mutex<Value>) {
    loop {
        if let Ok(connection) = system_bus() {
            sleeps(&connection, slept);
        }
        std::thread::sleep(SLEEPS_AGAIN);
    }
}

/// Keeps when each sleep began and ended, as logind's `PrepareForSleep`
/// says: true as it begins, false once it's woken. It waits on the bus
/// alone, so it wakes for nothing else.
fn sleeps(connection: &zbus::blocking::Connection, slept: &Mutex<Value>) {
    let Ok(manager) = login1(connection) else {
        return;
    };
    let Ok(signals) = manager.receive_signal("PrepareForSleep") else {
        return;
    };
    for signal in signals {
        let Ok(beginning) = signal.body().deserialize::<bool>() else {
            continue;
        };
        let mut slept = lock(slept);
        if beginning {
            *slept = json!({"began_at_ms": now_ms(), "ended_at_ms": null});
        } else {
            slept["ended_at_ms"] = json!(now_ms());
        }
    }
}

/// What logind says of idling and sleep: whether the session's idle and
/// since when, whether it's about to sleep, what's held off, and every
/// inhibitor, with the last sleep this daemon saw.
fn logind(connection: &zbus::blocking::Connection, slept: &Value) -> Value {
    let Ok(manager) = login1(connection) else {
        return Value::Null;
    };
    let inhibitors = manager
        .call::<_, _, Vec<(String, String, String, String, u32, u32)>>("ListInhibitors", &())
        .map(|all| {
            all.into_iter()
                .map(|(what, who, why, mode, uid, pid)| {
                    json!({
                        "what": what.split(':').collect::<Vec<_>>(),
                        "who": who, "why": why, "mode": mode, "uid": uid, "pid": pid,
                    })
                })
                .collect::<Vec<_>>()
        })
        .ok();
    let idle_since_ms = manager
        .get_property::<u64>("IdleSinceHint")
        .ok()
        .filter(|us| *us > 0)
        .map(|us| us / 1000);
    let listed = |property: &str| {
        manager.get_property::<String>(property).ok().map(|what| {
            what.split(':')
                .filter(|w| !w.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
    };
    json!({
        "idle": manager.get_property::<bool>("IdleHint").ok(),
        "idle_since_ms": idle_since_ms,
        "preparing": manager.get_property::<bool>("PreparingForSleep").ok(),
        // Read each time, since the machine's idle sleep reloads logind
        // with other settings on battery.
        "idle_action": manager.get_property::<String>("IdleAction").ok(),
        "idle_action_ms": manager.get_property::<u64>("IdleActionUSec").ok().map(|us| us / 1000),
        "blocked": listed("BlockInhibited"),
        "delayed": listed("DelayInhibited"),
        "inhibitors": inhibitors,
        "last_sleep": slept,
    })
}

/// Asks fwupd once a minute while firmware is watched, and at once when
/// it says something changed, what it knows of each device and what it
/// offers each. Its connection is closed with the last watch, which ends
/// the thread listening for its signals.
pub(crate) fn firmware_loop(shared: &Arc<Shared>) {
    let mut bus: Option<zbus::blocking::Connection> = None;
    let mut last = None;
    loop {
        if !due_while_watched(shared, "firmware", FIRMWARE_EVERY, &mut last) {
            if let Some(connection) = bus.take() {
                let _ = connection.close();
            }
            due(shared, "firmware", FIRMWARE_EVERY, &mut last);
        }
        if bus.is_none() {
            bus = system_bus().ok();
            if let Some(connection) = bus.clone() {
                let shared = Arc::clone(shared);
                std::thread::spawn(move || changes(&connection, &shared));
            }
        }
        // An fwupd that can't be reached answers a bare host, so the
        // shell is told Firmware has nothing rather than left waiting.
        let said = bus
            .as_ref()
            .map(fwupd)
            .filter(|said| !said.is_empty())
            .unwrap_or_else(|| vec![firmware::host(None, &[], None, None)]);
        lock(&shared.daemon).firmware = said;
    }
}

fn fwupd_proxy(connection: &zbus::blocking::Connection) -> zbus::Result<zbus::blocking::Proxy<'_>> {
    proxy(
        connection,
        "org.freedesktop.fwupd",
        "/",
        "org.freedesktop.fwupd",
    )
}

/// Says firmware changed each time fwupd says anything, until its
/// connection closes, waiting on the bus alone: `Changed` as its metadata does, and `DeviceAdded`,
/// `DeviceRemoved` and `DeviceChanged` as a device comes, goes, takes an
/// update or gains a problem, as one needing the charger does when it's
/// unplugged.
fn changes(connection: &zbus::blocking::Connection, shared: &Arc<Shared>) {
    let Ok(proxy) = fwupd_proxy(connection) else {
        return;
    };
    let Ok(signals) = proxy.receive_all_signals() else {
        return;
    };
    for _ in signals {
        lock(&shared.daemon).hurried.insert("firmware".to_owned());
        shared.woken.notify_all();
    }
}

/// Every record fwupd's answers make, the host's first.
fn fwupd(connection: &zbus::blocking::Connection) -> Vec<Value> {
    type Dicts = Vec<HashMap<String, zbus::zvariant::OwnedValue>>;
    let Ok(proxy) = fwupd_proxy(connection) else {
        return vec![];
    };
    let text = |property: &str| proxy.get_property::<String>(property).ok();
    let as_json = |dict: &HashMap<String, zbus::zvariant::OwnedValue>| {
        Value::Object(dict.iter().map(|(k, v)| (k.clone(), plain(v))).collect())
    };
    // What the level is made of; an fwupd that can't say has none, and
    // the journal says why.
    let attributes: Vec<Value> = match proxy.call::<_, _, Dicts>("GetHostSecurityAttrs", &()) {
        Ok(all) => all.iter().map(as_json).collect(),
        Err(why) => {
            eprintln!("<4>fwupd couldn't say its security attributes: {why}");
            vec![]
        }
    };
    let mut records = vec![firmware::host(
        text("HostSecurityId").as_deref(),
        &attributes,
        text("HostProduct").as_deref(),
        text("DaemonVersion").as_deref(),
    )];
    let Ok(devices) = proxy.call::<_, _, Dicts>("GetDevices", &()) else {
        return records;
    };
    let devices: Vec<Value> = devices.iter().map(as_json).collect();
    let mut offered = BTreeMap::new();
    for device in &devices {
        let Some(id) = device["DeviceId"].as_str() else {
            continue;
        };
        // fwupd offers only an updatable device anything.
        if device["Flags"]
            .as_u64()
            .is_none_or(|flags| flags & 0b10 == 0)
        {
            continue;
        }
        let said: firmware::Offered = match proxy.call::<_, _, Dicts>("GetUpgrades", &(id,)) {
            Ok(releases) => Ok(releases.first().map(as_json)),
            Err(zbus::Error::MethodError(_, Some(why), _)) => Err(why),
            Err(why) => Err(why.to_string()),
        };
        offered.insert(id.to_owned(), said);
    }
    // fwupd answers an empty history with an error, which is no history.
    let history = proxy
        .call::<_, _, Dicts>("GetHistory", &())
        .map(|entries| entries.iter().map(as_json).collect::<Vec<_>>())
        .unwrap_or_default();
    records.extend(firmware::records(
        &devices,
        &offered,
        &firmware::history(&history),
    ));
    records
}

/// What `UPower` says of each device it keeps, by its supply's name.
fn upower(connection: &zbus::blocking::Connection) -> BTreeMap<String, Value> {
    let service = "org.freedesktop.UPower";
    let Ok(manager) = proxy(
        connection,
        service,
        "/org/freedesktop/UPower",
        "org.freedesktop.UPower",
    ) else {
        return BTreeMap::new();
    };
    let Ok(paths) =
        manager.call::<_, _, Vec<zbus::zvariant::OwnedObjectPath>>("EnumerateDevices", &())
    else {
        return BTreeMap::new();
    };
    let mut said = BTreeMap::new();
    for path in paths {
        let Ok(device) = proxy(
            connection,
            service,
            path.as_str().to_owned(),
            "org.freedesktop.UPower.Device",
        ) else {
            continue;
        };
        let Ok(name) = device.get_property::<String>("NativePath") else {
            continue;
        };
        let seconds = |property: &str| device.get_property::<i64>(property).ok().filter(|s| *s > 0);
        // UPower's states, as its Device interface numbers them.
        let state = device.get_property::<u32>("State").ok().map(|s| match s {
            1 => "charging",
            2 => "discharging",
            3 => "empty",
            4 => "full",
            5 => "pending charge",
            6 => "pending discharge",
            _ => "unknown",
        });
        let warning = device
            .get_property::<u32>("WarningLevel")
            .ok()
            .map(|w| match w {
                2 => "discharging",
                3 => "low",
                4 => "critical",
                5 => "action",
                _ => "none",
            });
        said.insert(
            name.rsplit('/').next().unwrap_or(&name).to_owned(),
            json!({
                "state": state,
                "warning": warning,
                "time_to_empty_s": seconds("TimeToEmpty"),
                "time_to_full_s": seconds("TimeToFull"),
                "rate_w": device.get_property::<f64>("EnergyRate").ok().map(|w| rounded(w, 3)),
            }),
        );
    }
    said
}

/// Asks `UDisks` for each disk's health once a minute while disks are
/// watched, and when they start to be if it last asked a minute or more
/// ago, so a launcher opened and closed doesn't ask each time.
pub(crate) fn health_loop(shared: &Shared) {
    let mut bus: Option<zbus::blocking::Connection> = None;
    let mut last: Option<Instant> = None;
    loop {
        due(shared, "disk", HEALTH_EVERY, &mut last);
        let names = disk::disks(&lock(&shared.reader).root);
        if bus.is_none() {
            match system_bus() {
                Ok(connection) => bus = Some(connection),
                Err(why) => {
                    let mut daemon = lock(&shared.daemon);
                    let reason = format!("the system bus isn't there: {why}");
                    daemon.udisks = json!({"state": "refused", "reason": reason});
                    hand_over(&daemon);
                    continue;
                }
            }
        }
        let Some(connection) = bus.as_ref() else {
            continue;
        };
        let mut said = BTreeMap::new();
        let mut refused = None;
        for name in names {
            match smart(connection, &name) {
                Ok(health) => {
                    said.insert(name, health);
                }
                Err(why) => refused = Some(why.to_string()),
            }
        }
        // Asked again next time on a new connection, so a bus or a
        // `UDisks` that restarted is reached again.
        if refused.is_some() {
            bus = None;
        }
        let mut daemon = lock(&shared.daemon);
        daemon.health = said;
        daemon.udisks = match refused {
            Some(why) => json!({"state": "refused", "reason": why}),
            None => json!({"state": "answering"}),
        };
        hand_over(&daemon);
        shared.woken.notify_all();
    }
}

/// Whether a bus error says only that what was asked isn't there, as a
/// disk that isn't NVMe has no controller's figures, rather than that
/// `UDisks` is missing, refused or broken.
fn absent(why: &zbus::Error) -> bool {
    const ABSENT: [&str; 5] = [
        "org.freedesktop.DBus.Error.UnknownInterface",
        "org.freedesktop.DBus.Error.UnknownMethod",
        "org.freedesktop.DBus.Error.UnknownObject",
        "org.freedesktop.DBus.Error.UnknownProperty",
        "org.freedesktop.DBus.Error.InvalidArgs",
    ];
    match why {
        zbus::Error::MethodError(name, _, _) => ABSENT.contains(&name.as_str()),
        zbus::Error::FDO(fdo) => matches!(
            **fdo,
            zbus::fdo::Error::UnknownInterface(_)
                | zbus::fdo::Error::UnknownMethod(_)
                | zbus::fdo::Error::UnknownObject(_)
                | zbus::fdo::Error::UnknownProperty(_)
                | zbus::fdo::Error::InvalidArgs(_)
        ),
        _ => false,
    }
}

/// A disk's health as its controller tells `UDisks`, with no prompt: every
/// attribute of its SMART log, and its controller's own figures, or null
/// for a disk `UDisks` keeps none for, as one that isn't NVMe. Any other
/// error, `UDisks` missing or a refusal, is said as one.
fn smart(connection: &zbus::blocking::Connection, name: &str) -> zbus::Result<Value> {
    let udisks = "org.freedesktop.UDisks2";
    let block = proxy(
        connection,
        udisks,
        format!("/org/freedesktop/UDisks2/block_devices/{name}"),
        "org.freedesktop.UDisks2.Block",
    )?;
    let drive = match block.get_property::<zbus::zvariant::OwnedObjectPath>("Drive") {
        Ok(drive) => drive,
        Err(why) if absent(&why) => return Ok(Value::Null),
        Err(why) => return Err(why),
    };
    if drive.as_str() == "/" {
        return Ok(Value::Null);
    }
    let controller = proxy(
        connection,
        udisks,
        drive.as_str().to_owned(),
        "org.freedesktop.UDisks2.NVMe.Controller",
    )?;
    let mut options: HashMap<&str, zbus::zvariant::Value> = HashMap::new();
    options.insert(
        "auth.no_user_interaction",
        zbus::zvariant::Value::from(true),
    );
    let attributes = match controller.call::<_, _, HashMap<String, zbus::zvariant::OwnedValue>>(
        "SmartGetAttributes",
        &(options,),
    ) {
        Ok(attributes) => attributes,
        Err(why) if absent(&why) => return Ok(Value::Null),
        Err(why) => return Err(why),
    };
    let mut health: serde_json::Map<String, Value> = attributes
        .iter()
        .map(|(name, value)| (name.clone(), plain(value)))
        .collect();
    for (property, field) in [
        ("SmartCriticalWarning", "critical_warning"),
        ("SmartPowerOnHours", "power_on_hours"),
        ("SmartTemperature", "temperature_k"),
        ("SmartUpdated", "updated_s"),
        ("NVMeRevision", "nvme_revision"),
        ("UnallocatedCapacity", "unallocated_b"),
        ("State", "state"),
    ] {
        if let Ok(value) = controller.get_property::<zbus::zvariant::OwnedValue>(property) {
            health.insert(field.to_owned(), plain(&value));
        }
    }
    Ok(Value::Object(health))
}

/// A D-Bus value as plain JSON.
fn plain(value: &zbus::zvariant::Value) -> Value {
    use zbus::zvariant::Value as D;
    match value {
        D::U8(n) => json!(n),
        D::Bool(b) => json!(b),
        D::I16(n) => json!(n),
        D::U16(n) => json!(n),
        D::I32(n) => json!(n),
        D::U32(n) => json!(n),
        D::I64(n) => json!(n),
        D::U64(n) => json!(n),
        D::F64(n) => json!(n),
        D::Str(s) => json!(s.as_str()),
        D::ObjectPath(p) => json!(p.as_str()),
        D::Array(items) => Value::Array(items.iter().map(plain).collect()),
        D::Value(inner) => plain(inner),
        D::Dict(dict) => {
            let mut object = serde_json::Map::new();
            for (key, value) in dict.iter() {
                let key = match plain(key) {
                    Value::String(s) => s,
                    other => other.to_string(),
                };
                object.insert(key, plain(value));
            }
            Value::Object(object)
        }
        _ => Value::Null,
    }
}
