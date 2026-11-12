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

    /// The cleaner to clean with now.
    pub fn cleaner(&self) -> Cleaner {
        self.file
            .as_deref()
            .and_then(|file| std::fs::read_to_string(file).ok())
            .and_then(|said| Cleaner::named(said.trim()).ok())
            .unwrap_or(self.configured)
    }

    /// Keeps `cleaner` as the pick, written whole beside the file and moved
    /// over it, so the service never reads half of one.
    pub fn choose(&self, cleaner: Cleaner) -> Result<(), String> {
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
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("chosen-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        folder
    }

    #[test]
    fn nothing_picked_cleans_as_nix_says() {
        let chosen = Chosen::new(Some(folder("none").join("cleaner")), Cleaner::DeepFilterNet);
        assert_eq!(chosen.cleaner(), Cleaner::DeepFilterNet);
    }

    #[test]
    fn a_pick_is_kept_and_cleans_from_then_on() {
        let file = folder("kept").join("noise-cancelled").join("cleaner");
        let chosen = Chosen::new(Some(file.clone()), Cleaner::DeepFilterNet);
        chosen.choose(Cleaner::RNNoise).expect("the pick is kept");
        assert_eq!(chosen.cleaner(), Cleaner::RNNoise);
        assert_eq!(
            std::fs::read_to_string(&file).expect("the file is there"),
            "rnnoise\n"
        );
        let other = Chosen::new(Some(file), Cleaner::DeepFilterNetGentle);
        assert_eq!(
            other.cleaner(),
            Cleaner::RNNoise,
            "another process reads the pick"
        );
    }

    #[test]
    fn a_file_naming_no_cleaner_cleans_as_nix_says() {
        let folder = folder("garbled");
        std::fs::create_dir_all(&folder).expect("the folder is made");
        std::fs::write(folder.join("cleaner"), "loud\n").expect("the file is written");
        let chosen = Chosen::new(Some(folder.join("cleaner")), Cleaner::DeepFilterNetGentle);
        assert_eq!(chosen.cleaner(), Cleaner::DeepFilterNetGentle);
    }

    #[test]
    fn nowhere_to_keep_it_keeps_nothing() {
        let chosen = Chosen::new(None, Cleaner::DeepFilterNet);
        chosen
            .choose(Cleaner::RNNoise)
            .expect("nothing to write is no failure");
        assert_eq!(chosen.cleaner(), Cleaner::DeepFilterNet);
    }
}
