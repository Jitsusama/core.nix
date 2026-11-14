//! The two daemons: the account's guard and the machine's, each run until
//! it's told to stop, holding its state so a second can't share it.

use super::reads::running;
use super::{Env, failed, read_config};
use crate::config::Scope;
use crate::daemon::{Guard, Places};
use crate::serve::{answering, hold, leave_when_held_up, steps, stopping};
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::Duration;
use terminal_kit::{Exit, Failure};

/// How often a daemon steps.
const STEP_EVERY: Duration = Duration::from_secs(5);

/// Runs a daemon until it's told to stop.
pub(super) fn daemon(
    env: &Env,
    scope: Scope,
    config: Option<PathBuf>,
    state: Option<PathBuf>,
    err: &mut dyn Write,
) -> Result<(), Failure> {
    // Before any thread starts, so every thread blocks them too.
    let mut wait = stopping(&[libc::SIGTERM, libc::SIGINT, libc::SIGHUP]).map_err(failed)?;
    let (default_config, default_state, name) = match scope {
        Scope::User => (&env.config, &env.state, "user"),
        Scope::System => (&env.system_config, &env.system_state, "system"),
    };
    let config_path = config.unwrap_or_else(|| default_config.clone());
    let config = read_config(&config_path)?;
    if config.scope != scope {
        return Err(Failure::new(
            Exit::Config,
            format!(
                "{} is a config for the other daemon, not the {name} daemon",
                config_path.display()
            ),
        ));
    }
    let state = state.unwrap_or_else(|| default_state.clone());
    make_state(&state, scope).map_err(|error| {
        Failure::new(
            Exit::Failed,
            format!("{} can't hold the daemon's state", state.display()),
        )
        .because(error.to_string())
    })?;
    let Some(_held) = hold(&state).map_err(failed)? else {
        return Err(Failure::new(
            Exit::TryAgain,
            format!("a {name} daemon already runs with {}", state.display()),
        ));
    };
    let mut hearing = None;
    let places = match scope {
        Scope::User => Places {
            state: state.clone(),
            status: Some(env.status.clone()),
            processes: env.processes.clone(),
            system: Some(env.system_socket.clone()),
            running: running(),
        },
        Scope::System => {
            if let Some(dir) = env.system_socket.parent() {
                fs::create_dir_all(dir).map_err(failed)?;
            }
            let store = crate::pool::pool(&config.nix.store).ok();
            hearing = Some(
                answering(
                    &env.system_socket,
                    &env.processes,
                    crate::socket::ASK_EVERY,
                    store,
                )
                .map_err(failed)?,
            );
            Places {
                state: state.clone(),
                status: Some(env.system_status.clone()),
                processes: env.processes.clone(),
                system: None,
                running: running(),
            }
        }
    };
    // Only the user daemon has a session to show a notice in.
    let mut desk = crate::desktop::desk(config.notify && scope == Scope::User);
    let builds = config
        .sources
        .values()
        .any(|source| matches!(source, crate::config::Source::Cargo { .. }));
    let freezes = config.freeze;
    // Written before the first survey, so nothing the configuration keeps
    // is ever offered, and under the daemon's hold on its state.
    crate::keeps::declare(&places.state, &config.keep, super::wall()).map_err(failed)?;
    let mut guard = Guard::new(config, &places).map_err(failed)?;
    guard.beat_always().map_err(failed)?;
    // A stop waits, blocked, until a step ends, and a survey's walks once
    // held one for 35 s; the survey asks as it goes and ends early.
    guard.stop_when(Box::new(crate::commands::stop_waiting));
    // A step that doesn't ask, as some walks don't, is left mid-way.
    leave_when_held_up(io::stderr()).map_err(failed)?;
    if let Some(hearing) = hearing {
        guard.hear(hearing);
    }
    // Only the account's guard freezes, in its own manager, unless its
    // config says never. A Mac has no manager to freeze in yet.
    #[cfg(target_os = "linux")]
    if scope == Scope::User && freezes {
        guard.freeze_with(Box::new(crate::freezer::Session::default()));
    }
    #[cfg(not(target_os = "linux"))]
    let _ = freezes;
    // Without a watch nothing dates an older build, so it stays.
    if builds {
        match crate::opens::Watch::new() {
            Ok(watch) => guard.listen(std::sync::Arc::new(watch)),
            Err(error) => {
                let _ = writeln!(
                    err,
                    "disk-guard: no watch on builds, so older ones stay: {error}"
                );
            }
        }
    }
    steps(&mut guard, STEP_EVERY, &mut wait, &mut desk, err);
    Ok(())
}

/// Makes a daemon's state folder. The system daemon's lets any account
/// through to its journal, but not list it: every other file there is
/// saved as root's alone, and what an earlier guard saved readable is made
/// so first. It's set on a folder that's there too, as a Mac's earlier
/// guard left it root's alone. On NixOS systemd opens the folder as the
/// unit starts, so what an earlier guard left is closed a moment after.
fn make_state(state: &Path, scope: Scope) -> io::Result<()> {
    let mode = match scope {
        Scope::User => 0o700,
        Scope::System => 0o711,
    };
    fs::DirBuilder::new()
        .recursive(true)
        .mode(mode)
        .create(state)?;
    match scope {
        Scope::User => Ok(()),
        Scope::System => {
            crate::state::private(state)?;
            fs::set_permissions(state, fs::Permissions::from_mode(mode))
        }
    }
}
