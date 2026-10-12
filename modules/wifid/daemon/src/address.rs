//! The address a card shows a network it joins.
//!
//! A network saved with `Address=network`, the default, sees the same
//! address every time the card joins it, so a hotel's sign-in page knows
//! you when you come back, and no two networks see the same one, so they
//! can't tell they've met the same machine. The address is a keyed hash of
//! the card's own address and the network's name, under a key that never
//! leaves the machine.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

/// The key every network's address is made from.
pub struct AddressKey([u8; 32]);

impl AddressKey {
    /// The key kept in this file, made the first time wifid starts. A file
    /// that isn't a key is refused rather than replaced: a new key would
    /// give every network a new address, and every sign-in page would
    /// forget this machine.
    pub fn kept_in(file: &Path) -> io::Result<Self> {
        match fs::read(file) {
            Ok(bytes) => bytes.try_into().map(Self).map_err(|bytes: Vec<u8>| {
                let reason = format!("{} holds {} bytes, not 32", file.display(), bytes.len());
                io::Error::new(io::ErrorKind::InvalidData, reason)
            }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Self::make(file),
            Err(error) => Err(error),
        }
    }

    fn make(file: &Path) -> io::Result<Self> {
        let mut key = [0; 32];
        File::open("/dev/urandom")?.read_exact(&mut key)?;
        let partial = file.with_extension("partial");
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&partial)?
            .write_all(&key)?;
        fs::rename(partial, file)?;
        Ok(Self(key))
    }

    /// The address this card shows the network of that name.
    pub fn address(&self, card: [u8; 6], network: &str) -> [u8; 6] {
        let mut hash = <Hmac<Sha256>>::new_from_slice(&self.0).expect("any key fits an HMAC");
        hash.update(&card);
        hash.update(network.as_bytes());
        let digest = hash.finalize().into_bytes();
        let mut address = [0; 6];
        address.copy_from_slice(&digest[..6]);
        // The first byte's lowest bit marks a group address, which no
        // card may take, and the next marks one a card chose for itself
        // rather than the one its maker gave it.
        address[0] = (address[0] & !0b01) | 0b10;
        address
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    const CARD: [u8; 6] = [0x8c, 0xf8, 0xc5, 0x12, 0x34, 0x56];
    const OTHER_CARD: [u8; 6] = [0x8c, 0xf8, 0xc5, 0xab, 0xcd, 0xef];

    #[test]
    fn the_key_is_made_once_and_kept() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("address-key");

        let made = AddressKey::kept_in(&file).unwrap();
        let read = AddressKey::kept_in(&file).unwrap();

        assert_eq!(made.address(CARD, "Lantern"), read.address(CARD, "Lantern"));
        assert_ne!(made.0, [0; 32]);
        let mode = fs::metadata(&file).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn a_key_file_of_the_wrong_size_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("address-key");
        fs::write(&file, b"short").unwrap();

        let refused = AddressKey::kept_in(&file);

        assert_eq!(
            refused.err().map(|e| e.kind()),
            Some(io::ErrorKind::InvalidData)
        );
        assert_eq!(fs::read(&file).unwrap(), b"short");
    }

    #[test]
    fn a_network_sees_the_same_address_every_time() {
        let key = AddressKey([7; 32]);

        let first = key.address(CARD, "Lantern");
        let again = key.address(CARD, "Lantern");

        assert_eq!(first, again);
        assert_ne!(first, CARD);
    }

    #[test]
    fn each_network_sees_an_address_of_its_own() {
        let key = AddressKey([7; 32]);

        let lantern = key.address(CARD, "Lantern");
        let inn = key.address(CARD, "Roadside Inn");

        assert_ne!(lantern, inn);
    }

    #[test]
    fn each_card_shows_a_network_an_address_of_its_own() {
        let key = AddressKey([7; 32]);

        let one = key.address(CARD, "Lantern");
        let other = key.address(OTHER_CARD, "Lantern");

        assert_ne!(one, other);
    }

    #[test]
    fn another_machine_shows_the_network_another_address() {
        let ours = AddressKey([7; 32]);
        let theirs = AddressKey([8; 32]);

        assert_ne!(
            ours.address(CARD, "Lantern"),
            theirs.address(CARD, "Lantern")
        );
    }

    #[test]
    fn the_address_is_one_a_card_may_take_for_itself() {
        let key = AddressKey([7; 32]);

        let names = ["Lantern", "Roadside Inn", "Hilton Honors", "Caf\u{e9}", ""];
        for name in names {
            let first = key.address(CARD, name)[0];
            assert_eq!(first & 0b10, 0b10, "{name}: not locally administered");
            assert_eq!(first & 0b01, 0, "{name}: a group address");
        }
    }
}
