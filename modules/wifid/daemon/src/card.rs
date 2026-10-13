//! The machine's Wi-Fi cards.

use std::fs;
use std::io;
use std::path::Path;

/// A Wi-Fi card, by the name of its link.
#[derive(Clone, Debug, PartialEq)]
pub struct Card {
    pub name: String,
    /// The address its maker gave it, which the card's link shows only
    /// until wpa_supplicant gives it another.
    pub address: [u8; 6],
}

/// Every Wi-Fi card under this sysfs, sorted by name.
pub fn all(sys: &Path) -> io::Result<Vec<Card>> {
    let mut cards = Vec::new();
    for link in fs::read_dir(sys.join("class/net"))? {
        let link = link?;
        // Only a Wi-Fi card's link has a radio.
        let radio = link.path().join("phy80211/macaddress");
        let Ok(address) = fs::read_to_string(&radio) else {
            continue;
        };
        let address = parsed(address.trim()).ok_or_else(|| {
            let reason = format!("{} isn't an address", radio.display());
            io::Error::new(io::ErrorKind::InvalidData, reason)
        })?;
        let name = link.file_name().to_string_lossy().into_owned();
        cards.push(Card { name, address });
    }
    cards.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(cards)
}

fn parsed(address: &str) -> Option<[u8; 6]> {
    let bytes: Vec<u8> = address
        .split(':')
        .map(|byte| {
            u8::from_str_radix(byte, 16)
                .ok()
                .filter(|_| byte.len() == 2)
        })
        .collect::<Option<_>>()?;
    bytes.try_into().ok()
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::symlink;

    use super::*;

    fn link(sys: &Path, name: &str, radio: Option<&str>) {
        let dir = sys.join("class/net").join(name);
        fs::create_dir_all(&dir).unwrap();
        if let Some(radio) = radio {
            let phy = sys.join("class/ieee80211").join(radio);
            fs::create_dir_all(&phy).unwrap();
            fs::write(phy.join("macaddress"), "8c:f8:c5:12:34:5f\n").unwrap();
            symlink(&phy, dir.join("phy80211")).unwrap();
        }
    }

    #[test]
    fn the_wifi_cards_are_listed_with_their_radio_s_address() {
        let sys = tempfile::tempdir().unwrap();
        link(sys.path(), "wlp0s20f3", Some("phy0"));
        link(sys.path(), "enp0s31f6", None);
        link(sys.path(), "lo", None);
        link(sys.path(), "wg0", None);

        let cards = all(sys.path()).unwrap();

        let card = Card {
            name: "wlp0s20f3".to_string(),
            address: [0x8c, 0xf8, 0xc5, 0x12, 0x34, 0x5f],
        };
        assert_eq!(cards, vec![card]);
    }
}
