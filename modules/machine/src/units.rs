//! Restarting a user unit the shell asks for, as Spend's Start the Watch
//! does, through systemd's user bus rather than a spawned `systemctl`:
//! the shell spawns nothing to act (Y2YBTS, 2026-11-10). Only the units
//! named in [`RESTARTABLE`] are, so the socket can't be used to restart
//! anything else the session runs.

use std::time::Duration;

/// The units the shell may restart, each the daemon behind a row that
/// offers to start it when it isn't answering.
pub const RESTARTABLE: [&str; 1] = ["spend-watch.service"];

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

#[cfg(test)]
mod tests {
    use super::*;

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
