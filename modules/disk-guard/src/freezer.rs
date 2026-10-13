//! Freezing a runaway writer through systemd. The writer and every
//! process descended from it are moved into a transient scope of their
//! own, which is then frozen, so whatever started it, a terminal or an
//! agent, keeps running and can be used to see why. The cgroup freezer
//! stops the scope's processes where they stand, and thawing lets them
//! carry on as if nothing happened, which killing never could.

use std::time::{Duration, Instant};
use zbus::blocking::Connection;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

const SYSTEMD: &str = "org.freedesktop.systemd1";
const MANAGER_PATH: &str = "/org/freedesktop/systemd1";
const MANAGER: &str = "org.freedesktop.systemd1.Manager";
const UNIT: &str = "org.freedesktop.systemd1.Unit";

/// The scopes the guard makes, which are the only ones it ever thaws.
pub const PATTERN: &str = "disk-guard-freeze-*.scope";

/// The scope a writer is frozen in, named for it.
pub fn unit_for(pid: u32) -> String {
    format!("disk-guard-freeze-{pid}.scope")
}

/// How long a new scope may take to start before it's frozen.
const STARTING: Duration = Duration::from_secs(2);

/// A unit as ListUnitsByPatterns lists it.
type Listed = (
    String,
    String,
    String,
    String,
    String,
    String,
    OwnedObjectPath,
    u32,
    String,
    OwnedObjectPath,
);

/// A service manager, the account's or the machine's.
pub struct Freezer {
    connection: Connection,
}

impl Freezer {
    pub fn new(connection: Connection) -> Freezer {
        Freezer { connection }
    }

    /// Moves `pids` into a new scope named `unit` and freezes it.
    pub fn freeze(&self, unit: &str, pids: &[u32], description: &str) -> zbus::Result<()> {
        let properties: Vec<(&str, Value)> = vec![
            ("PIDs", Value::from(pids.to_vec())),
            ("Description", Value::from(description)),
            // Gone once its processes are, frozen or not.
            ("CollectMode", Value::from("inactive-or-failed")),
        ];
        let aux: Vec<(&str, Vec<(&str, Value)>)> = Vec::new();
        self.manager("StartTransientUnit", &(unit, "fail", properties, aux))?;
        let started = Instant::now();
        while self.property(unit, "ActiveState")? != "active" {
            if started.elapsed() > STARTING {
                return Err(zbus::Error::Failure(format!("{unit} never started")));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        self.manager("FreezeUnit", &unit)
    }

    pub fn thaw(&self, unit: &str) -> zbus::Result<()> {
        self.manager("ThawUnit", &unit)
    }

    /// Whether `unit` is frozen, or on its way there.
    pub fn frozen(&self, unit: &str) -> zbus::Result<bool> {
        let state = self.property(unit, "FreezerState")?;
        Ok(state == "frozen" || state == "freezing")
    }

    /// The scopes the guard made that the manager still has.
    pub fn ours(&self) -> zbus::Result<Vec<String>> {
        let reply = self.connection.call_method(
            Some(SYSTEMD),
            MANAGER_PATH,
            Some(MANAGER),
            "ListUnitsByPatterns",
            &(Vec::<&str>::new(), vec![PATTERN]),
        )?;
        let units: Vec<Listed> = reply.body().deserialize()?;
        Ok(units.into_iter().map(|unit| unit.0).collect())
    }

    fn manager<B>(&self, method: &str, body: &B) -> zbus::Result<()>
    where
        B: serde::Serialize + zbus::zvariant::DynamicType,
    {
        self.connection
            .call_method(Some(SYSTEMD), MANAGER_PATH, Some(MANAGER), method, body)?;
        Ok(())
    }

    fn property(&self, unit: &str, name: &str) -> zbus::Result<String> {
        let reply = self.connection.call_method(
            Some(SYSTEMD),
            MANAGER_PATH,
            Some(MANAGER),
            "GetUnit",
            &unit,
        )?;
        let path: OwnedObjectPath = reply.body().deserialize()?;
        let reply = self.connection.call_method(
            Some(SYSTEMD),
            path.as_str(),
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &(UNIT, name),
        )?;
        let value: OwnedValue = reply.body().deserialize()?;
        String::try_from(value).map_err(zbus::Error::from)
    }
}
