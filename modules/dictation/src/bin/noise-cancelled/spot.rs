//! Where the machine sits now, its spot (the glossary's), so a microphone
//! keeps a pick for each: the MV7 at the desk in a quiet room may want a
//! gentler cleaner than the laptop's own microphone on the couch beside
//! the television. A spot is told apart by what the machine knows of
//! itself, never by where on a map it is:
//!
//! - the home network's node it's on, at home, from the file the eero
//!   daemon keeps while its read is fresh (modules/eero);
//! - otherwise the network networkd joined it to, as its link's network
//!   file names it, since WARP runs everywhere and tells no spots apart;
//! - each screen plugged in, the laptop's own aside, by the bytes of its
//!   EDID that say who made it, which one it is and its serial, as the
//!   backlight daemon tells them apart (modules/backlight's facts.rs);
//! - each dock on Thunderbolt, past the machine's own controllers.
//!
//! All of it is read from files the kernel, networkd and the eero daemon
//! keep, in a few milliseconds and with no program started, since the
//! denoiser's thread reads it as a microphone starts being recorded.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde_json::Value;

/// How old the eero daemon's file may be and still say which node this
/// machine is on; it reads every few minutes while eero answers.
const FRESH: Duration = Duration::from_mins(30);

/// The connectors of the laptop's own screen, by the part of a
/// connector's name after its card, which say nothing about a spot.
const BUILT_IN: [&str; 3] = ["eDP-", "LVDS-", "DSI-"];

/// A spot: the key a pick is kept under, with no spaces so it ends a
/// line of the picks' file, and a name for it until Joel gives his own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spot {
    pub key: String,
    pub name: String,
}

/// Where the files a spot is read from are, so a test can stand in.
#[derive(Clone, Debug)]
pub struct Where {
    /// The user's runtime folder, holding the eero daemon's file.
    runtime: Option<PathBuf>,
    sys: PathBuf,
    /// networkd's state, holding each link's.
    netif: PathBuf,
}

impl Where {
    /// This machine's own files.
    pub fn here() -> Self {
        Self {
            runtime: std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from),
            sys: PathBuf::from("/sys"),
            netif: PathBuf::from("/run/systemd/netif"),
        }
    }

    /// The same files under `root`: `run/user`, `sys` and
    /// `run/systemd/netif`.
    #[cfg(test)]
    pub fn at(root: &Path) -> Self {
        Self {
            runtime: Some(root.join("run/user")),
            sys: root.join("sys"),
            netif: root.join("run/systemd/netif"),
        }
    }

    /// The spot the machine is at now.
    pub fn spot(&self) -> Spot {
        let (mut key, mut name) = match self.node() {
            Some((node, location)) => (
                format!("home={}", keyed(&node)),
                location.unwrap_or_else(|| "home".to_owned()),
            ),
            None => {
                let networks = self.networks();
                let joined = if networks.is_empty() {
                    "none".to_owned()
                } else {
                    let parts: Vec<String> =
                        networks.iter().map(|network| keyed(network)).collect();
                    parts.join(",")
                };
                (format!("network={joined}"), away(&networks))
            }
        };
        let screens = self.screens();
        for (identity, _) in &screens {
            key.push_str(&format!("+screen={identity}"));
        }
        let docks = self.docks();
        for identity in &docks {
            key.push_str(&format!("+dock={}", keyed(identity)));
        }
        let named: Vec<String> = screens
            .iter()
            .map(|(_, screen)| format!("the {screen}"))
            .collect();
        if !named.is_empty() {
            name.push_str(&format!(", with {}", named.join(" and ")));
        }
        if named.is_empty() && !docks.is_empty() {
            name.push_str(", docked");
        }
        Spot { key, name }
    }

    /// The home network's node this machine is on, and where the node is
    /// as eero names it, while the eero daemon's read is fresh.
    fn node(&self) -> Option<(String, Option<String>)> {
        let file = self.runtime.as_ref()?.join("eero/home-network.json");
        let modified = std::fs::metadata(&file).ok()?.modified().ok()?;
        if SystemTime::now()
            .duration_since(modified)
            .unwrap_or_default()
            > FRESH
        {
            return None;
        }
        let network: Value = serde_json::from_str(&std::fs::read_to_string(file).ok()?).ok()?;
        let node = network["devices"]
            .as_array()?
            .iter()
            .find(|device| device["this_machine"] == true)?["node"]
            .as_str()?
            .to_owned();
        let location = network["nodes"]
            .as_array()
            .and_then(|nodes| nodes.iter().find(|each| each["id"] == node.as_str()))
            .and_then(|each| each["location"].as_str())
            .map(str::to_owned);
        Some((node, location))
    }

    /// The network file each link out is joined by, sorted, as networkd
    /// keeps them: a Wi-Fi network's own, or a cable's.
    fn networks(&self) -> Vec<String> {
        let mut networks: Vec<String> = listed(&self.netif.join("links"))
            .iter()
            .filter_map(|link| {
                let state = std::fs::read_to_string(link).ok()?;
                let said = |name: &str| {
                    state
                        .lines()
                        .find_map(|line| line.strip_prefix(name)?.strip_prefix('='))
                };
                if said("OPER_STATE") != Some("routable") {
                    return None;
                }
                let file = said("NETWORK_FILE").filter(|file| !file.is_empty())?;
                Some(Path::new(file).file_stem()?.to_string_lossy().into_owned())
            })
            .collect();
        networks.sort();
        networks.dedup();
        networks
    }

    /// Each screen plugged in past the laptop's own, by its EDID's
    /// identity, sorted, with the name it gives itself.
    fn screens(&self) -> Vec<(String, String)> {
        let mut screens: Vec<(String, String)> = listed(&self.sys.join("class/drm"))
            .iter()
            .filter_map(|connector| {
                let named = connector.file_name()?.to_string_lossy().into_owned();
                let (_, kind) = named.split_once('-')?;
                if BUILT_IN.iter().any(|built_in| kind.starts_with(built_in)) {
                    return None;
                }
                let status = std::fs::read_to_string(connector.join("status")).ok()?;
                if status.trim() != "connected" {
                    return None;
                }
                edid_said(&std::fs::read(connector.join("edid")).ok()?)
            })
            .collect();
        screens.sort();
        screens.dedup();
        screens
    }

    /// Each Thunderbolt device past the machine's own controllers, as
    /// its vendor's and device's numbers, sorted.
    fn docks(&self) -> Vec<String> {
        let mut docks: Vec<String> = listed(&self.sys.join("bus/thunderbolt/devices"))
            .iter()
            .filter_map(|device| {
                let named = device.file_name()?.to_string_lossy().into_owned();
                // A controller is N-0, a port's retimer has a colon, and
                // a domain has no dash.
                if !named.contains('-') || named.ends_with("-0") || named.contains(':') {
                    return None;
                }
                let number = |file: &str| {
                    std::fs::read_to_string(device.join(file))
                        .ok()
                        .map(|said| said.trim().to_owned())
                };
                Some(format!("{}.{}", number("vendor")?, number("device")?))
            })
            .collect();
        docks.sort();
        docks.dedup();
        docks
    }
}

/// A spot away from home's name: the Wi-Fi it's on, where its network
/// file is named for it as wifid names them ("50-wifi-‹network›").
fn away(networks: &[String]) -> String {
    networks
        .iter()
        .find_map(|network| network.split_once("-wifi-"))
        .map_or_else(|| "away".to_owned(), |(_, wifi)| format!("away, on {wifi}"))
}

/// A screen's identity and name from its EDID: the hex of bytes 8 to
/// 17, its maker, product, serial and when it was made, and the name its
/// descriptor gives, else "screen", or nothing for no EDID at all.
fn edid_said(edid: &[u8]) -> Option<(String, String)> {
    const HEADER: [u8; 8] = [0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0];
    if edid.len() < 128 || edid[..8] != HEADER {
        return None;
    }
    let identity: String = edid[8..18]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    // The four 18-byte descriptors; the one tagged 0xFC is the name,
    // ended by a line feed when it's short.
    let name = [54, 72, 90, 108]
        .iter()
        .map(|start| &edid[*start..start + 18])
        .find(|descriptor| descriptor[..4] == [0, 0, 0, 0xfc])
        .map(|descriptor| {
            let text = &descriptor[5..];
            let end = text
                .iter()
                .position(|byte| *byte == b'\n')
                .unwrap_or(text.len());
            String::from_utf8_lossy(&text[..end]).trim().to_owned()
        })
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "screen".to_owned());
    Some((identity, name))
}

/// `said` as a part of a spot's key: a space, and each mark the key is
/// put together with, written as a percent sign and its hex, so a Wi-Fi
/// named "Roadside Inn" keeps its line of the picks' file whole.
fn keyed(said: &str) -> String {
    said.chars()
        .map(|mark| {
            if mark.is_whitespace() || "%+,=".contains(mark) {
                let mut bytes = [0; 4];
                mark.encode_utf8(&mut bytes)
                    .bytes()
                    .map(|byte| format!("%{byte:02X}"))
                    .collect()
            } else {
                mark.to_string()
            }
        })
        .collect()
}

/// What a folder holds, or nothing when it isn't there.
fn listed(folder: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(folder)
        .map(|entries| {
            entries
                .filter_map(|entry| Some(entry.ok()?.path()))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "spot_tests.rs"]
mod tests;
