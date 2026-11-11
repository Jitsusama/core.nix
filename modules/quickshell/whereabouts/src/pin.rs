//! Joel's own word on where the laptop is, kept as the pin in the state
//! folder, which the daemon says in place of anything GeoClue says until
//! he asks it to find the laptop again. Each is asked over the socket, as
//! Location's Set It by Hand and Find It Again do, or run by hand
//! (`whereabouts set PIN WORDS`, `whereabouts again PIN`); the daemon
//! hears the pin change either way, by inotify, and says it at once.

use std::fs;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::fix::Fix;
use crate::named::{self, Unfound};

/// How near, in metres, a town Joel names is taken to be: the width of a
/// town, which is as sure as its name makes it.
pub const BY_HAND: f64 = 1_000.0;

/// Why a town Joel named wasn't pinned.
#[derive(Debug, PartialEq)]
pub enum Unpinned {
    /// No place is called what he typed.
    Nowhere,
    /// The place couldn't be asked about, as with no network.
    Unasked,
    /// The place found, or the pin, couldn't be kept.
    Unkept(String),
}

/// Finds the place Joel named and pins it, waiting on the web to find it.
pub fn set(pin: &Path, words: &str) -> Result<(), Unpinned> {
    let found = named::find(words).map_err(|unfound| match unfound {
        Unfound::Nowhere => Unpinned::Nowhere,
        Unfound::Unasked => Unpinned::Unasked,
    })?;
    let when = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let fix = Fix::new(found.latitude, found.longitude, BY_HAND, when)
        .ok_or_else(|| Unpinned::Unkept("the place found is nowhere on the Earth".into()))?
        .named(Some(found.place), found.zone)
        .countried(found.country)
        .by_hand();
    pin.parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|()| fix.write(pin))
        .map_err(|error| Unpinned::Unkept(error.to_string()))
}

/// Takes the pin away, so the daemon says only what GeoClue finds.
pub fn again(pin: &Path) -> io::Result<()> {
    match fs::remove_file(pin) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}
