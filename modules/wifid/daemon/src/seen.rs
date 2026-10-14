//! Where each saved network was last heard, so a card looks there first.
//!
//! A card looking on every channel takes seconds, most of them on channels
//! where nothing it knows has ever been. wifid keeps, for each saved
//! network, the channels it was last joined on, newest first, and a card
//! with no network looks on those alone. Should nothing it knows answer,
//! wpa_supplicant looks everywhere next, as it would have anyway.
//!
//! Only the channels are kept, by the network's name, and never which
//! access point was joined: that would say where the machine has been.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

/// How many channels are kept for each network.
const KEPT: usize = 4;

/// The channels each saved network was last joined on, by frequency in
/// MHz.
pub struct Seen {
    file: PathBuf,
    channels: BTreeMap<String, Vec<u32>>,
}

impl Seen {
    /// The channels kept in this file. One that can't be read is begun
    /// again empty, which costs no more than a look everywhere.
    pub fn kept_in(file: &Path) -> Seen {
        let kept = fs::read(file).ok();
        let channels = kept.and_then(|kept| serde_json::from_slice(&kept).ok());
        Seen {
            file: file.to_path_buf(),
            channels: channels.unwrap_or_default(),
        }
    }

    /// Every channel a saved network was last joined on, lowest first.
    pub fn channels(&self) -> Vec<u32> {
        let mut channels: Vec<u32> = self.channels.values().flatten().copied().collect();
        channels.sort_unstable();
        channels.dedup();
        channels
    }

    /// Notes the network was joined on this channel, keeping it on the
    /// disk.
    pub fn joined(&mut self, name: &str, frequency: u32) -> io::Result<()> {
        let channels = self.channels.entry(name.to_string()).or_default();
        if channels.first() == Some(&frequency) {
            return Ok(());
        }
        channels.retain(|kept| *kept != frequency);
        channels.insert(0, frequency);
        channels.truncate(KEPT);
        self.keep()
    }

    /// Forgets the channels of every network but these.
    pub fn keep_only<'a>(&mut self, names: impl IntoIterator<Item = &'a str>) -> io::Result<()> {
        let names: Vec<&str> = names.into_iter().collect();
        let before = self.channels.len();
        self.channels
            .retain(|name, _| names.contains(&name.as_str()));
        if self.channels.len() == before {
            return Ok(());
        }
        self.keep()
    }

    /// Writes the channels whole or not at all, readable by wifid alone.
    fn keep(&self) -> io::Result<()> {
        let partial = self.file.with_extension("partial");
        let mut written = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&partial)?;
        written.write_all(&serde_json::to_vec(&self.channels)?)?;
        fs::set_permissions(&partial, fs::Permissions::from_mode(0o600))?;
        fs::rename(partial, &self.file)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    #[test]
    fn a_network_s_channels_are_those_it_was_joined_on() {
        let dir = tempfile::tempdir().unwrap();
        let mut seen = Seen::kept_in(&dir.path().join("seen"));

        seen.joined("Lantern", 5180).unwrap();
        seen.joined("Lantern", 2437).unwrap();
        seen.joined("Roadside Inn", 2412).unwrap();
        seen.joined("Roadside Inn", 5180).unwrap();

        assert_eq!(seen.channels(), vec![2412, 2437, 5180]);
    }

    #[test]
    fn the_channels_are_kept_for_when_wifid_next_starts() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("seen");
        Seen::kept_in(&file).joined("Lantern", 5180).unwrap();

        let seen = Seen::kept_in(&file);

        assert_eq!(seen.channels(), vec![5180]);
        assert_eq!(
            fs::metadata(&file).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn only_the_latest_few_channels_are_kept() {
        let dir = tempfile::tempdir().unwrap();
        let mut seen = Seen::kept_in(&dir.path().join("seen"));

        for frequency in [2412, 2437, 2462, 5180, 5745, 2437] {
            seen.joined("Lantern", frequency).unwrap();
        }

        assert_eq!(seen.channels(), vec![2437, 2462, 5180, 5745]);
    }

    #[test]
    fn a_network_no_longer_saved_is_forgotten() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("seen");
        let mut seen = Seen::kept_in(&file);
        seen.joined("Lantern", 5180).unwrap();
        seen.joined("Roadside Inn", 2412).unwrap();

        seen.keep_only(["Roadside Inn"]).unwrap();

        assert_eq!(seen.channels(), vec![2412]);
        assert_eq!(Seen::kept_in(&file).channels(), vec![2412]);
    }

    #[test]
    fn nothing_joined_has_no_channels() {
        let dir = tempfile::tempdir().unwrap();

        let seen = Seen::kept_in(&dir.path().join("seen"));

        assert_eq!(seen.channels(), Vec::<u32>::new());
    }

    #[test]
    fn a_file_that_cant_be_read_is_begun_again() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("seen");
        fs::write(&file, "not what wifid writes").unwrap();
        let mut seen = Seen::kept_in(&file);

        seen.joined("Lantern", 5180).unwrap();

        assert_eq!(Seen::kept_in(&file).channels(), vec![5180]);
    }
}
