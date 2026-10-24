//! The best rates each network held, so a network met before starts right.
//! A network is named by its router's hardware address, kept in one JSON
//! file of name to rates.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Held {
    pub up: u64,
    pub down: u64,
}

pub struct Memory {
    path: PathBuf,
    networks: BTreeMap<String, Held>,
}

impl Memory {
    pub fn open(dir: &Path) -> Memory {
        let path = dir.join("networks.json");
        let networks = fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Memory { path, networks }
    }

    pub fn get(&self, network: &str) -> Option<Held> {
        self.networks.get(network).copied()
    }

    /// Remembers a network's rates, writing the file only when they changed
    /// by more than a twentieth, so a rate that wanders doesn't wear the disk.
    pub fn put(&mut self, network: &str, held: Held) {
        let near = |a: u64, b: u64| a.abs_diff(b) * 20 <= a.max(b);
        if let Some(old) = self.networks.get(network)
            && near(old.up, held.up)
            && near(old.down, held.down)
        {
            return;
        }
        self.networks.insert(network.to_string(), held);
        let text = serde_json::to_string_pretty(&self.networks).expect("rates serialise");
        let fresh = self.path.with_extension("json.new");
        if fs::write(&fresh, text)
            .and_then(|()| fs::rename(&fresh, &self.path))
            .is_err()
        {
            eprintln!("couldn't write {}", self.path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("shaper-memory-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn a_network_met_before_is_remembered_across_starts() {
        let d = dir("again");
        Memory::open(&d).put(
            "6c:ae:f6:ba:85:86",
            Held {
                up: 10_400,
                down: 85_000,
            },
        );
        assert_eq!(
            Memory::open(&d).get("6c:ae:f6:ba:85:86"),
            Some(Held {
                up: 10_400,
                down: 85_000
            })
        );
        assert_eq!(Memory::open(&d).get("00:11:22:33:44:55"), None);
    }

    #[test]
    fn a_small_wander_isnt_written() {
        let d = dir("wander");
        let mut m = Memory::open(&d);
        m.put(
            "net",
            Held {
                up: 10_000,
                down: 80_000,
            },
        );
        m.put(
            "net",
            Held {
                up: 10_300,
                down: 81_000,
            },
        );
        assert_eq!(
            Memory::open(&d).get("net"),
            Some(Held {
                up: 10_000,
                down: 80_000
            })
        );
        m.put(
            "net",
            Held {
                up: 12_000,
                down: 81_000,
            },
        );
        assert_eq!(
            Memory::open(&d).get("net"),
            Some(Held {
                up: 12_000,
                down: 81_000
            })
        );
    }
}
