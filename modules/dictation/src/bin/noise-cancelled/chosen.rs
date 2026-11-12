//! The cleaner Joel picked by ear, kept in a file of its own: the trial
//! writes it when he picks a cleaned clip, and the service reads it each
//! time it loads the model, so his pick cleans his next call with nothing
//! restarted and no call cut short. The cleaner Nix names is the one used
//! until he's picked, or when the file says nothing a cleaner is named.

use std::path::{Path, PathBuf};

use crate::denoise::Cleaner;

/// Where the pick is kept, and the cleaner used without one.
#[derive(Clone, Debug)]
pub struct Chosen {
    file: Option<PathBuf>,
    configured: Cleaner,
}

impl Chosen {
    /// The pick kept in `file`, or none kept at all when it's `None`, as
    /// for a cleaner named on the command line alone.
    pub const fn new(file: Option<PathBuf>, configured: Cleaner) -> Self {
        Self { file, configured }
    }

    /// The cleaner to clean with now, one for every microphone.
    pub fn cleaner(&self, _microphone: Option<&str>) -> Cleaner {
        self.file
            .as_deref()
            .and_then(|file| std::fs::read_to_string(file).ok())
            .and_then(|said| Cleaner::named(said.trim()).ok())
            .unwrap_or(self.configured)
    }

    /// A file called `name` beside the pick's, for what's kept with it, or
    /// none when no pick is kept.
    pub fn beside(&self, name: &str) -> Option<PathBuf> {
        Some(self.file.as_ref()?.with_file_name(name))
    }

    /// Keeps `cleaner` as the pick, written whole beside the file and moved
    /// over it, so the service never reads half of one.
    pub fn choose(&self, _microphone: &str, cleaner: Cleaner) -> Result<(), String> {
        let Some(file) = &self.file else {
            return Ok(());
        };
        let written = file.with_extension("picking");
        if let Some(folder) = file.parent() {
            std::fs::create_dir_all(folder).map_err(|why| saying(folder, &why))?;
        }
        std::fs::write(&written, format!("{cleaner}\n")).map_err(|why| saying(&written, &why))?;
        std::fs::rename(&written, file).map_err(|why| saying(file, &why))
    }
}

fn saying(path: &Path, why: &std::io::Error) -> String {
    format!("keeping the pick in {}: {why}", path.display())
}

#[cfg(test)]
#[path = "chosen_tests.rs"]
mod tests;
