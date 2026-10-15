//! The account's units, through its systemd manager on the session bus.

use std::process::Stdio;

use tokio::process::Command;
use tokio::time::{Duration, timeout};
use zbus::proxy::CacheProperties;
use zbus::zvariant::OwnedObjectPath;
use zbus::{Connection, proxy};

use crate::revive::{self, State, Unanswered};

#[proxy(
    interface = "org.freedesktop.systemd1.Manager",
    default_service = "org.freedesktop.systemd1",
    default_path = "/org/freedesktop/systemd1"
)]
trait Manager {
    fn load_unit(&self, name: &str) -> zbus::Result<OwnedObjectPath>;
    fn start_unit(&self, name: &str, mode: &str) -> zbus::Result<OwnedObjectPath>;
    fn reload_unit(&self, name: &str, mode: &str) -> zbus::Result<OwnedObjectPath>;
    fn reset_failed_unit(&self, name: &str) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.freedesktop.systemd1.Unit",
    default_service = "org.freedesktop.systemd1"
)]
trait Unit {
    #[zbus(property)]
    fn load_state(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn active_state(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn sub_state(&self) -> zbus::Result<String>;
}

/// One of a service's commands: its path, its arguments from the program's
/// own name on, and what systemd keeps of its last run.
type Exec = (String, Vec<String>, bool, u64, u64, u64, u64, u32, i32, i32);

#[proxy(
    interface = "org.freedesktop.systemd1.Service",
    default_service = "org.freedesktop.systemd1"
)]
trait Service {
    #[zbus(property)]
    fn exec_start(&self) -> zbus::Result<Vec<Exec>>;
    #[zbus(property)]
    fn environment(&self) -> zbus::Result<Vec<String>>;
    #[zbus(property)]
    fn working_directory(&self) -> zbus::Result<String>;
}

/// The longest a check may take.
const CHECKING: Duration = Duration::from_secs(10);

pub struct Units {
    bus: Connection,
    manager: ManagerProxy<'static>,
}

impl Units {
    pub async fn new(bus: &Connection) -> zbus::Result<Units> {
        Ok(Units {
            bus: bus.clone(),
            manager: ManagerProxy::new(bus).await?,
        })
    }

    async fn path(&self, name: &str) -> Result<OwnedObjectPath, Unanswered> {
        self.manager.load_unit(name).await.map_err(said)
    }

    /// The service's first command, as systemd would run it: the program,
    /// its arguments, its environment and its directory.
    async fn program(&self, name: &str) -> Result<Command, Unanswered> {
        let service = ServiceProxy::builder(&self.bus)
            .path(self.path(name).await?)
            .map_err(said)?
            .cache_properties(CacheProperties::No)
            .build()
            .await
            .map_err(said)?;
        let commands = service.exec_start().await.map_err(said)?;
        let Some((path, arguments, ..)) = commands.into_iter().next() else {
            return Err(Unanswered::Said("it has no ExecStart to check with".into()));
        };
        let mut program = Command::new(path);
        program.args(arguments.iter().skip(1));
        let environment = service.environment().await.map_err(said)?;
        program.envs(
            environment
                .iter()
                .filter_map(|setting| setting.split_once('=')),
        );
        // A directory systemd may do without is said with a leading !, and
        // the account's home as ~, which is where the daemon runs anyway.
        let directory = service.working_directory().await.map_err(said)?;
        let directory = directory.trim_start_matches('!');
        if !directory.is_empty() && directory != "~" {
            program.current_dir(directory);
        }
        Ok(program)
    }
}

impl revive::Units for Units {
    async fn state(&self, name: &str) -> Result<State, Unanswered> {
        let unit = UnitProxy::builder(&self.bus)
            .path(self.path(name).await?)
            .map_err(said)?
            .cache_properties(CacheProperties::No)
            .build()
            .await
            .map_err(said)?;
        Ok(State {
            load: unit.load_state().await.map_err(said)?,
            active: unit.active_state().await.map_err(said)?,
            sub: unit.sub_state().await.map_err(said)?,
        })
    }

    async fn check(&self, name: &str, arguments: &[String]) -> Result<(), Unanswered> {
        let mut program = self.program(name).await?;
        program
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let child = program
            .spawn()
            .map_err(|error| Unanswered::Said(format!("its program won't run: {error}")))?;
        let Ok(ran) = timeout(CHECKING, child.wait_with_output()).await else {
            return Err(Unanswered::Said(format!(
                "its check took longer than {} s",
                CHECKING.as_secs()
            )));
        };
        let ran =
            ran.map_err(|error| Unanswered::Said(format!("its check went wrong: {error}")))?;
        if ran.status.success() {
            return Ok(());
        }
        let said = String::from_utf8_lossy(&ran.stderr);
        let first = said.lines().map(str::trim).find(|line| !line.is_empty());
        Err(Unanswered::Said(first.map_or_else(
            || format!("its check ended with {}", ran.status),
            str::to_string,
        )))
    }

    async fn start(&self, name: &str) -> Result<(), Unanswered> {
        // Spawned, so the start is asked for even if the network changes
        // again between the two calls and this look is given up.
        let manager = self.manager.clone();
        let name = name.to_string();
        let starting = tokio::spawn(async move {
            // systemd unloads a unit nothing needs, which it says can't have
            // failed, so there's nothing to reset; starting loads it again.
            match manager.reset_failed_unit(&name).await {
                Err(zbus::Error::MethodError(error, _, _)) if error.as_str() == NO_SUCH_UNIT => {}
                reset => reset?,
            }
            manager.start_unit(&name, "replace").await
        });
        match starting.await {
            Ok(started) => started.map(drop).map_err(said),
            Err(error) => Err(Unanswered::Said(error.to_string())),
        }
    }

    async fn reload(&self, name: &str) -> Result<(), Unanswered> {
        self.manager
            .reload_unit(name, "replace")
            .await
            .map(drop)
            .map_err(said)
    }
}

/// What systemd says of a unit it has unloaded.
const NO_SUCH_UNIT: &str = "org.freedesktop.systemd1.NoSuchUnit";

/// What the bus says in systemd's place while it executes itself again: that
/// it left before answering, then that nobody has its name until it's back.
const AWAY: [&str; 3] = [
    "org.freedesktop.DBus.Error.NoReply",
    "org.freedesktop.DBus.Error.NameHasNoOwner",
    "org.freedesktop.DBus.Error.ServiceUnknown",
];

/// What D-Bus said went wrong, without the name of its error, and whether it
/// said so because systemd was away.
fn said(error: zbus::Error) -> Unanswered {
    let away =
        matches!(&error, zbus::Error::MethodError(name, _, _) if AWAY.contains(&name.as_str()));
    let message = match error {
        zbus::Error::MethodError(_, Some(message), _) => message,
        error => error.to_string(),
    };
    if away {
        Unanswered::Away(message)
    } else {
        Unanswered::Said(message)
    }
}
