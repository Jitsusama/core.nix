//! The cleaner Joel picked by ear for each microphone, kept in a file of
//! its own: the trial writes it when he picks a cleaned clip, and the
//! service reads it as each microphone starts being recorded, so his pick
//! cleans his next call with nothing restarted and no call cut short.
//!
//! He moves between microphones, and a room that wants one cleaner on the
//! laptop's may want another on the MV7, so each keeps its own pick. A
//! microphone he's never picked for takes the cleaner he's picked for the
//! most of them, since that's the best guess at his taste, and the one Nix
//! names until he's picked at all.
//!
//! The file holds one line a microphone, its cleaner and then its node's
//! name, newest last. A line with a cleaner alone was written before picks
//! were kept apart; it counts towards the most picked and is no
//! microphone's own.

use std::path::{Path, PathBuf};

use crate::denoise::Cleaner;

/// Where the picks are kept, and the cleaner used without one.
#[derive(Clone, Debug)]
pub struct Chosen {
    file: Option<PathBuf>,
    configured: Cleaner,
}

/// One line of the file: a cleaner, and the microphone it was picked for.
struct Pick {
    cleaner: Cleaner,
    microphone: Option<String>,
}

impl Chosen {
    /// The picks kept in `file`, or none kept at all when it's `None`, as
    /// for a cleaner named on the command line alone.
    pub const fn new(file: Option<PathBuf>, configured: Cleaner) -> Self {
        Self { file, configured }
    }

    /// The cleaner to clean `microphone` with now, by its node's name; with
    /// none named, the one a microphone never picked for takes.
    pub fn cleaner(&self, microphone: Option<&str>) -> Cleaner {
        let picks = self.picks();
        microphone
            .and_then(|name| {
                picks
                    .iter()
                    .rev()
                    .find(|pick| pick.microphone.as_deref() == Some(name))
            })
            .map(|pick| pick.cleaner)
            .or_else(|| most_picked(&picks))
            .unwrap_or(self.configured)
    }

    /// A file called `name` beside the picks', for what's kept with them,
    /// or none when no pick is kept.
    pub fn beside(&self, name: &str) -> Option<PathBuf> {
        Some(self.file.as_ref()?.with_file_name(name))
    }

    /// Keeps `cleaner` as `microphone`'s pick, the file written whole
    /// beside itself and moved over, so the service never reads half.
    pub fn choose(&self, microphone: &str, cleaner: Cleaner) -> Result<(), String> {
        let Some(file) = &self.file else {
            return Ok(());
        };
        let mut picks = self.picks();
        picks.retain(|pick| pick.microphone.as_deref() != Some(microphone));
        picks.push(Pick {
            cleaner,
            microphone: Some(microphone.to_owned()),
        });
        let said: String = picks
            .iter()
            .map(|pick| match &pick.microphone {
                Some(name) => format!("{} {name}\n", pick.cleaner),
                None => format!("{}\n", pick.cleaner),
            })
            .collect();
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
                let (cleaner, microphone) = match line.trim().split_once(' ') {
                    Some((cleaner, microphone)) => (cleaner, Some(microphone.trim())),
                    None => (line.trim(), None),
                };
                Some(Pick {
                    cleaner: Cleaner::named(cleaner).ok()?,
                    microphone: microphone
                        .filter(|name| !name.is_empty())
                        .map(str::to_owned),
                })
            })
            .collect()
    }
}

/// The cleaner picked for the most microphones, the newest of those tied.
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
