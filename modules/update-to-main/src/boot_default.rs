//! What the boot loader starts by default, as update-to-main last left it,
//! so going back from a switch that moved it can put it back.
//!
//! Going back to a system whose boot check failed only activates it, as
//! its own switch did (`system::go_back`). A switch after it that made
//! itself the boot default and then failed would stay it, and the machine
//! would next boot the system that had just failed. So the system the
//! boot default was left on is kept, beside where the profile pointed
//! then. A profile pointing elsewhere since was moved by someone else, a
//! switch or boot by hand, whose boot default is that person's choice and
//! is left be.

use crate::config::{Config, Kind};
use crate::run::{self, Failure};
use std::fs;
use std::path::{Path, PathBuf};

/// The system update-to-main left the boot default on, or why there is
/// none to put back: never kept, unreadable, or since moved by someone
/// else.
pub fn kept(config: &Config) -> Result<PathBuf, String> {
    let file = config.boot_default_file();
    let kept = fs::read_to_string(&file)
        .map_err(|_| "update-to-main hasn't yet kept what it booted before".to_owned())?;
    let Some((system, profile)) = kept.split_once('\n') else {
        return Err(format!("{} won't read", file.display()));
    };
    if fs::read_link(&config.profile).ok().as_deref() != Some(Path::new(profile)) {
        return Err(
            "the profile moved outside update-to-main after it last switched, so the boot \
             default is whoever moved it's"
                .to_owned(),
        );
    }
    Ok(PathBuf::from(system))
}

/// Keeps `system` as the one update-to-main leaves the boot default on,
/// with the profile as it points now. One that can't be kept is said on
/// standard error, and the one kept before it goes, so going back says
/// there is none rather than putting back a stale one.
pub fn keep(config: &Config, system: &Path) {
    if !matches!(config.kind, Kind::Nixos) {
        return;
    }
    let file = config.boot_default_file();
    let new = file.with_extension("new");
    let kept = fs::canonicalize(system).and_then(|system| {
        let profile = fs::read_link(&config.profile)?;
        fs::write(&new, format!("{}\n{}", system.display(), profile.display()))?;
        fs::rename(&new, &file)
    });
    if let Err(e) = kept {
        eprintln!("{}: {e}", file.display());
        // A file that was never there is nothing left standing.
        let _ = fs::remove_file(&file);
    }
}

/// Makes `system` the boot default again, leaving what runs and the
/// profile as they are, as switch-to-configuration's `boot` does.
pub fn put_back(config: &Config, system: &Path) -> Result<(), Failure> {
    let switch = system.join("bin/switch-to-configuration");
    if !switch.exists() {
        return Err(Failure::new(format!(
            "{} is gone from the store",
            system.display()
        )));
    }
    run::said(
        run::command(&switch, &config.state),
        "making what the machine booted before its boot default again",
        ["boot"],
    )
    .map(|_| ())
}
