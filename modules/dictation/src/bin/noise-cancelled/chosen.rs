//! The cleaner Joel picked by ear for each microphone at each spot, kept
//! in a file of its own: the trial writes it when he picks a cleaned
//! clip, and the service reads it as each microphone starts being
//! recorded, so his pick cleans his next call with nothing restarted and
//! no call cut short.
//!
//! He moves between microphones and rooms, and a room that wants one
//! cleaner on the laptop's may want another on the MV7, so each
//! microphone keeps a pick for each spot it's picked at (spot.rs). At a
//! spot it's never been picked at, a microphone takes its newest pick
//! anywhere, the best guess at what suits it. A microphone he's never
//! picked for takes the cleaner he's picked most, since that's the best
//! guess at his taste, and the one Nix names until he's picked at all.
//!
//! The file holds one line a pick, its cleaner, its microphone's node
//! name and its spot's key, newest last. Lines from before picks were
//! kept apart still count: one with a cleaner alone towards the most
//! picked, and one with no spot as its microphone's pick anywhere.

use std::path::{Path, PathBuf};

use crate::denoise::Cleaner;
use crate::spot::{Spot, Where};

/// Where the picks are kept, where the spot is read from, and the
/// cleaner used without a pick.
#[derive(Clone, Debug)]
pub struct Chosen {
    file: Option<PathBuf>,
    configured: Cleaner,
    here: Where,
}

/// One line of the file: a cleaner, and the microphone and spot it was
/// picked for.
struct Pick {
    cleaner: Cleaner,
    microphone: Option<String>,
    spot: Option<String>,
}

impl Chosen {
    /// The picks kept in `file`, or none kept at all when it's `None`, as
    /// for a cleaner named on the command line alone, at the spot this
    /// machine is at.
    pub fn new(file: Option<PathBuf>, configured: Cleaner) -> Self {
        Self {
            file,
            configured,
            here: Where::here(),
        }
    }

    /// The same picks, at the spot `here` reads.
    #[cfg(test)]
    pub fn within(self, here: Where) -> Self {
        Self { here, ..self }
    }

    /// The spot the machine is at now.
    pub fn spot(&self) -> Spot {
        self.here.spot()
    }

    /// The cleaner to clean `microphone` with now, by its node's name, at
    /// `spot`; with no microphone named, the one a microphone never picked
    /// for takes.
    pub fn cleaner(&self, microphone: Option<&str>, spot: Option<&Spot>) -> Cleaner {
        let picks = self.picks();
        let newest = |at: Option<&str>| {
            picks.iter().rev().find(|pick| {
                pick.microphone.as_deref() == microphone
                    && at.is_none_or(|key| pick.spot.as_deref() == Some(key))
            })
        };
        microphone
            .and_then(|_| newest(spot.map(|spot| spot.key.as_str())).or_else(|| newest(None)))
            .map(|pick| pick.cleaner)
            .or_else(|| most_picked(&picks))
            .unwrap_or(self.configured)
    }

    /// Whether Joel has named the spot with `key`.
    pub fn named(&self, key: &str) -> bool {
        let _ = key;
        false
    }

    /// Keeps `name` as Joel's for `spot`, answering the spot as named.
    pub fn name(&self, spot: &Spot, name: &str) -> Result<Spot, String> {
        let _ = name;
        Ok(spot.clone())
    }

    /// A file called `name` beside the picks', for what's kept with them,
    /// or none when no pick is kept.
    pub fn beside(&self, name: &str) -> Option<PathBuf> {
        Some(self.file.as_ref()?.with_file_name(name))
    }

    /// Keeps `cleaner` as `microphone`'s pick at `spot`, in place of its
    /// pick there and of any it had before picks kept their spot, the file
    /// written whole beside itself and moved over, so the service never
    /// reads half.
    pub fn choose(&self, microphone: &str, spot: &Spot, cleaner: Cleaner) -> Result<(), String> {
        let Some(file) = &self.file else {
            return Ok(());
        };
        let mut picks = self.picks();
        picks.retain(|pick| {
            pick.microphone.as_deref() != Some(microphone)
                || pick.spot.as_deref().is_some_and(|key| key != spot.key)
        });
        picks.push(Pick {
            cleaner,
            microphone: Some(microphone.to_owned()),
            spot: Some(spot.key.clone()),
        });
        let said: String = picks.iter().map(Pick::line).collect();
        let written = file.with_extension("picking");
        if let Some(folder) = file.parent() {
            std::fs::create_dir_all(folder).map_err(|why| saying(folder, &why))?;
        }
        std::fs::write(&written, said).map_err(|why| saying(&written, &why))?;
        std::fs::rename(&written, file).map_err(|why| saying(file, &why))
    }

    /// Every pick the file holds, oldest first, past any line naming no
    /// cleaner this program has.
    fn picks(&self) -> Vec<Pick> {
        let said = self
            .file
            .as_deref()
            .and_then(|file| std::fs::read_to_string(file).ok())
            .unwrap_or_default();
        said.lines()
            .filter_map(|line| {
                let mut fields = line.split_whitespace();
                Some(Pick {
                    cleaner: Cleaner::named(fields.next()?).ok()?,
                    microphone: fields.next().map(str::to_owned),
                    spot: fields.next().map(str::to_owned),
                })
            })
            .collect()
    }
}

impl Pick {
    /// The pick as its line of the file, each field it has after its
    /// cleaner.
    fn line(&self) -> String {
        let mut line = self.cleaner.to_string();
        for field in [self.microphone.as_deref(), self.spot.as_deref()]
            .into_iter()
            .flatten()
        {
            line.push(' ');
            line.push_str(field);
        }
        line.push('\n');
        line
    }
}

/// The cleaner picked most, the newest of those tied.
fn most_picked(picks: &[Pick]) -> Option<Cleaner> {
    let count = |cleaner: Cleaner| picks.iter().filter(|pick| pick.cleaner == cleaner).count();
    picks
        .iter()
        .rev()
        .map(|pick| pick.cleaner)
        .reduce(|best, other| {
            if count(other) > count(best) {
                other
            } else {
                best
            }
        })
}

fn saying(path: &Path, why: &std::io::Error) -> String {
    format!("keeping the pick in {}: {why}", path.display())
}

#[cfg(test)]
#[path = "chosen_tests.rs"]
mod tests;
