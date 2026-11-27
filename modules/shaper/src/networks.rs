//! What each network's line is known to carry, as nixos.nix writes
//! `jitsusama.shaper.networks` for `shaper run --networks`: the one-way
//! controller guards the upload near it (src/oneway.rs), and under the
//! network's ceiling where one is set, and a call is given room under the
//! download's line where one is declared, for the network or for one of
//! its links alone (src/room.rs).

use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

/// Each network's declared upload rate, its anchor, in kbit/s, by its
/// router's hardware address, as `jitsusama.shaper.networks` sets them.
pub type Anchors = BTreeMap<String, f64>;

/// Each network's ceiling, the most its upload is held to, in kbit/s, by
/// its router's hardware address, where `jitsusama.shaper.networks` sets
/// one: under a second shaper past ours, so the queue stands in ours.
pub type Ceilings = BTreeMap<String, f64>;

/// A network's house: the most the upload this machine and another on the
/// same line send between them may come to, in kbit/s, and the addresses
/// the other reports what it sends from, one for each interface it may
/// send by (src/house.rs), or none for a keyed house heard by its key
/// alone, and the file holding the key its reports are signed with, where
/// it names one.
#[derive(Clone, Debug, PartialEq)]
pub struct House {
    pub most: u64,
    pub from: Vec<IpAddr>,
    pub key: Option<PathBuf>,
}

/// A house as written, read loosely so one written wrongly loses only
/// itself, not every network's anchor with it.
#[derive(Deserialize)]
struct Written {
    most: f64,
    from: Addresses,
    #[serde(default, rename = "keyFile")]
    key: Option<String>,
}

/// A house's addresses as written: a list, as nixos.nix writes them, or
/// one alone.
#[derive(Deserialize)]
#[serde(untagged)]
enum Addresses {
    One(String),
    Many(Vec<String>),
}

/// Each network's house, by its router's hardware address, where
/// `jitsusama.shaper.networks` sets one.
pub type Houses = BTreeMap<String, House>;

/// A network's declared download lines, in kbit/s: the network's own,
/// which a link naming none takes, and each link's own, by the link's
/// name, where the network names one. A link reaching the line slower
/// than the line carries, as Wi-Fi through an access point whose own
/// queue holds a call behind a download, is held under its own, and the
/// network's other links keep the network's.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Line {
    pub network: Option<u64>,
    pub links: BTreeMap<String, u64>,
}

impl Line {
    /// The line link `name` takes: its own, or the network's where it has
    /// none.
    pub fn of(&self, name: &str) -> Option<u64> {
        self.links.get(name).copied().or(self.network)
    }
}

/// Each network's declared download lines, by its router's hardware
/// address, where `jitsusama.shaper.networks` sets any.
pub type Lines = BTreeMap<String, Line>;

/// The line link `name` takes on `network`, its own or the network's, or
/// none where neither is declared.
pub fn line_of(lines: &Lines, network: &str, name: &str) -> Option<u64> {
    lines.get(network).and_then(|line| line.of(name))
}

#[derive(Deserialize)]
struct Network {
    #[serde(default)]
    up: Option<f64>,
    #[serde(default)]
    down: Option<f64>,
    /// Read as any value, then link by link, so one written wrongly loses
    /// only itself, not the whole file's parse.
    #[serde(default)]
    links: Option<serde_json::Value>,
    #[serde(default)]
    most: Option<f64>,
    /// Read as any value, then as a house, so one of the wrong shape
    /// loses only itself, not the whole file's parse.
    #[serde(default)]
    house: Option<serde_json::Value>,
}

/// The networks written at `path`, by router in lower case, or none where
/// there is no such file or it can't be read as one.
fn networks(path: &Path) -> BTreeMap<String, Network> {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<BTreeMap<String, Network>>(&text).ok())
        .map(|networks| {
            networks
                .into_iter()
                .map(|(router, network)| (router.to_lowercase(), network))
                .collect()
        })
        .unwrap_or_default()
}

/// The anchors written at `path`, or none where there is no such file or it
/// can't be read as one, which leaves every network unguarded.
pub fn anchors(path: &Path) -> Anchors {
    networks(path)
        .into_iter()
        .filter_map(|(router, network)| Some((router, network.up?)))
        .collect()
}

/// The most a line's download may be declared at, in kbit/s, as nixos.nix
/// says, and the least: a call's room under the least is still above the
/// download's floor.
const DOWN_KBIT: std::ops::RangeInclusive<f64> = 6_000.0..=10_000_000.0;

/// The lines written at `path`, of the networks that declare one within
/// what nixos.nix takes, for the network or one of its links, or none
/// where there is no such file or it can't be read as one, which gives no
/// call room on the download anywhere.
pub fn lines(path: &Path) -> Lines {
    networks(path)
        .into_iter()
        .filter_map(|(router, network)| {
            let line = Line {
                network: network.down.and_then(declared),
                links: links_lines(network.links),
            };
            (line.network.is_some() || !line.links.is_empty()).then_some((router, line))
        })
        .collect()
}

/// A download's line as declared, in kbit/s, where it's within what
/// nixos.nix takes.
fn declared(down: f64) -> Option<u64> {
    DOWN_KBIT.contains(&down).then(|| crate::rate::whole(down))
}

/// Each link's own download line, by its name, of a network's `links` as
/// written, leaving out one written wrongly, null or outside what
/// nixos.nix takes.
fn links_lines(links: Option<serde_json::Value>) -> BTreeMap<String, u64> {
    let Some(serde_json::Value::Object(links)) = links else {
        return BTreeMap::new();
    };
    links
        .into_iter()
        .filter_map(|(name, link)| Some((name, declared(link.get("down")?.as_f64()?)?)))
        .collect()
}

/// The ceilings written at `path`, of the networks that set one the
/// controller would hold: none under the least a trial may set, which
/// nixos.nix refuses too, so the start and the steps agree on one.
pub fn ceilings(path: &Path) -> Ceilings {
    networks(path)
        .into_iter()
        .filter_map(|(router, network)| Some((router, network.most?)))
        .filter(|(_, most)| most.is_finite() && *most >= crate::oneway::MOST_KBIT_LEAST)
        .collect()
}

/// The most a house's ceiling may name, in kbit/s, as nixos.nix says.
const HOUSE_MOST_KBIT: f64 = 10_000_000.0;

/// The houses written at `path`, or none where there is no such file or it
/// can't be read as one, leaving out one naming neither an address nor a
/// key, or an address that isn't one, or is IPv4's written as IPv6, mapped or compatible
/// (`Ipv6Addr::to_ipv4`, as nixos.nix refuses), which the firewall and the
/// ear would read apart, or whose ceiling is under the least a trial may
/// set or past the most, as nixos.nix refuses too. One naming a key file
/// that isn't an absolute path is left out too, so a house meant to take
/// only signed reports never takes unsigned ones.
pub fn houses(path: &Path) -> Houses {
    networks(path)
        .into_iter()
        .filter_map(|(router, network)| {
            let house: Written = serde_json::from_value(network.house?).ok()?;
            let written = match house.from {
                Addresses::One(address) => vec![address],
                Addresses::Many(addresses) => addresses,
            };
            let from = written
                .iter()
                .map(|address| address.parse().ok())
                .collect::<Option<Vec<IpAddr>>>()?;
            let mapped = from
                .iter()
                .any(|from| matches!(from, IpAddr::V6(v6) if v6.to_ipv4().is_some()));
            let key = house.key.map(PathBuf::from);
            let keyed = key.as_deref().is_none_or(Path::is_absolute);
            let least = crate::oneway::MOST_KBIT_LEAST;
            let bounded = (least..=HOUSE_MOST_KBIT).contains(&house.most);
            // A house naming no address is heard by its key alone, so one
            // with neither would hear nothing (src/house.rs).
            let heard = !from.is_empty() || key.is_some();
            (bounded && keyed && !mapped && heard).then(|| {
                let most = crate::rate::whole(house.most);
                (router, House { most, from, key })
            })
        })
        .collect()
}

/// What a link joining `network` starts its upload at, in kbit/s, where
/// the network has an anchor: the anchor's hold, or its ceiling where that
/// is lower, as the one-way controller starts its stay there.
pub fn hold(anchors: &Anchors, ceilings: &Ceilings, network: &str) -> Option<u64> {
    let most = ceilings.get(network).copied().unwrap_or(f64::INFINITY);
    anchors
        .get(network)
        .map(|&up| crate::rate::whole((up * crate::oneway::ANCHOR_HOLD).min(most)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_network_with_an_anchor_starts_at_its_hold() {
        let anchors = Anchors::from([("00:00:5e:00:53:01".to_string(), 12_000.0)]);
        let none = Ceilings::new();
        assert_eq!(hold(&anchors, &none, "00:00:5e:00:53:01"), Some(10_800));
        assert_eq!(
            hold(&anchors, &none, "link wlan0"),
            None,
            "a network with none"
        );
        let under = Ceilings::from([("00:00:5e:00:53:01".to_string(), 9_500.0)]);
        assert_eq!(
            hold(&anchors, &under, "00:00:5e:00:53:01"),
            Some(9_500),
            "a ceiling under the hold starts it there"
        );
    }

    #[test]
    fn each_network_s_anchor_and_ceiling_are_read_by_its_router_in_lower_case() {
        let folder = std::env::temp_dir().join(format!("shaper-anchors-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let file = folder.join("networks.json");
        assert!(anchors(&file).is_empty(), "with no file");
        fs::write(&file, r#"{"00:00:5E:00:53:01": {"up": 12000}}"#).expect("the networks");
        assert_eq!(
            anchors(&file).get("00:00:5e:00:53:01"),
            Some(&12_000.0),
            "as nixos.nix writes it"
        );
        assert!(ceilings(&file).is_empty(), "a network with no ceiling");
        fs::write(
            &file,
            concat!(
                r#"{"00:00:5E:00:53:01": {"up": 12000, "most": 9500}, "#,
                r#""00:00:5e:00:53:02": {"up": 9000, "most": null}}"#,
            ),
        )
        .expect("the networks");
        assert_eq!(
            ceilings(&file),
            Ceilings::from([("00:00:5e:00:53:01".to_string(), 9_500.0)]),
            "as nixos.nix writes them, null where none is set"
        );
        assert_eq!(
            anchors(&file).len(),
            2,
            "a ceiling leaves the anchors alone"
        );
        fs::write(
            &file,
            r#"{"00:00:5e:00:53:01": {"up": 12000, "most": 9.5}}"#,
        )
        .expect("a ceiling in Mb/s");
        assert!(
            ceilings(&file).is_empty(),
            "one under a trial's least, as the controller would refuse"
        );
        fs::write(
            &file,
            r#"{"00:00:5e:00:53:01": {"up": 12000, "most": 4000}}"#,
        )
        .expect("the least ceiling");
        assert_eq!(ceilings(&file).len(), 1, "the least nixos.nix takes");
        assert!(houses(&file).is_empty(), "a network with no house");
        fs::write(&file, "not json").expect("a broken file");
        assert!(anchors(&file).is_empty(), "a broken file anchors nothing");
        assert!(ceilings(&file).is_empty(), "nor sets a ceiling");
        assert!(houses(&file).is_empty(), "nor a house");
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn each_network_s_house_is_read_by_its_router_and_one_written_wrongly_loses_only_itself() {
        let folder = std::env::temp_dir().join(format!("shaper-houses-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let file = folder.join("networks.json");
        fs::write(
            &file,
            concat!(
                r#"{"00:00:5E:00:53:01": {"up": 12000, "most": null, "#,
                r#""house": {"most": 7000, "from": ["192.0.2.7", "192.0.2.8"]}}, "#,
                r#""00:00:5e:00:53:02": {"up": 9000, "house": null}}"#,
            ),
        )
        .expect("a house");
        let address = |text: &str| text.parse::<IpAddr>().expect("an address");
        assert_eq!(
            houses(&file),
            Houses::from([(
                "00:00:5e:00:53:01".to_string(),
                House {
                    most: 7_000,
                    from: vec![address("192.0.2.7"), address("192.0.2.8")],
                    key: None,
                },
            )]),
            "as nixos.nix writes them, each address in a list, null where none is set"
        );
        assert!(
            ceilings(&file).is_empty(),
            "a house is no ceiling of its own"
        );
        fs::write(
            &file,
            concat!(
                r#"{"00:00:5e:00:53:01": {"up": 12000, "#,
                r#""house": {"most": 7000, "from": "the mac"}}, "#,
                r#""00:00:5e:00:53:02": {"up": 9000, "#,
                r#""house": {"most": 7, "from": "192.0.2.7"}}, "#,
                r#""00:00:5e:00:53:03": {"up": 9000, "#,
                r#""house": {"most": 8000, "from": "2001:db8::7"}}, "#,
                r#""00:00:5e:00:53:04": {"up": 9000, "#,
                r#""house": {"most": 8000, "from": []}}, "#,
                r#""00:00:5e:00:53:05": {"up": 9000, "#,
                r#""house": {"most": 8000, "from": ["192.0.2.7", "the mac"]}}}"#,
            ),
        )
        .expect("four houses written wrongly beside one written well");
        let well = houses(&file);
        assert_eq!(
            houses(&file),
            Houses::from([(
                "00:00:5e:00:53:03".to_string(),
                House {
                    most: 8_000,
                    from: vec![address("2001:db8::7")],
                    key: None,
                },
            )]),
            "one with no address, an empty list, a list holding a name, or a ceiling in Mb/s, is \
             left out alone, and one address written alone is a list of one"
        );
        assert_eq!(anchors(&file).len(), 5, "which loses no anchor");
        fs::write(
            &file,
            concat!(
                r#"{"00:00:5e:00:53:01": {"up": 12000, "most": 9500, "#,
                r#""house": {"most": "7000", "from": "192.0.2.7"}}, "#,
                r#""00:00:5e:00:53:02": {"up": 9000, "house": {"most": 7000}}, "#,
                r#""00:00:5e:00:53:03": {"up": 9000, "#,
                r#""house": {"most": 8000, "from": "2001:db8::7"}}}"#,
            ),
        )
        .expect("houses of the wrong shape");
        assert_eq!(
            houses(&file),
            well,
            "a most in quotes, or no address, is left out alone"
        );
        assert_eq!(anchors(&file).len(), 3, "which loses no anchor");
        assert_eq!(ceilings(&file).len(), 1, "nor a ceiling");
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_house_s_key_is_read_where_it_names_an_absolute_file_and_the_house_left_out_where_not() {
        let folder = std::env::temp_dir().join(format!("shaper-keys-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let file = folder.join("networks.json");
        fs::write(
            &file,
            concat!(
                r#"{"00:00:5e:00:53:01": {"up": 12000, "#,
                r#""house": {"most": 7000, "from": "192.0.2.7", "keyFile": "/etc/house.key"}}, "#,
                r#""00:00:5e:00:53:02": {"up": 9000, "#,
                r#""house": {"most": 7000, "from": "192.0.2.7", "keyFile": null}}, "#,
                r#""00:00:5e:00:53:03": {"up": 9000, "#,
                r#""house": {"most": 7000, "from": "192.0.2.7", "keyFile": "house.key"}}, "#,
                r#""00:00:5e:00:53:04": {"up": 9000, "#,
                r#""house": {"most": 7000, "from": "192.0.2.7", "keyFile": 7}}, "#,
                r#""00:00:5e:00:53:05": {"up": 9000, "#,
                r#""house": {"most": 7000, "from": [], "keyFile": "/etc/house.key"}}}"#,
            ),
        )
        .expect("houses with keys");
        let houses = houses(&file);
        let _ = fs::remove_dir_all(&folder);
        let keys: Vec<(&str, Option<&Path>)> = houses
            .iter()
            .map(|(router, house)| (router.as_str(), house.key.as_deref()))
            .collect();
        assert_eq!(
            keys,
            [
                ("00:00:5e:00:53:01", Some(Path::new("/etc/house.key"))),
                ("00:00:5e:00:53:02", None),
                ("00:00:5e:00:53:05", Some(Path::new("/etc/house.key"))),
            ],
            "an absolute key file read, a null one none; a relative one, or one that isn't a \
             path, left the house out rather than taking unsigned reports; a keyed house \
             naming no address kept, heard by its key alone"
        );
        assert!(
            houses["00:00:5e:00:53:05"].from.is_empty(),
            "a keyed house naming no address was given one"
        );
    }

    #[test]
    fn a_house_at_its_bounds_is_kept_and_one_past_them_left_out() {
        let folder = std::env::temp_dir().join(format!("shaper-bounds-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let file = folder.join("networks.json");
        fs::write(
            &file,
            concat!(
                r#"{"00:00:5e:00:53:01": {"up": 12000, "#,
                r#""house": {"most": 4000, "from": "192.0.2.7"}}, "#,
                r#""00:00:5e:00:53:02": {"up": 9000, "#,
                r#""house": {"most": 10000001, "from": "192.0.2.7"}}, "#,
                r#""00:00:5e:00:53:03": {"up": 9000, "#,
                r#""house": {"most": 8000, "from": "::ffff:c000:207"}}, "#,
                r#""00:00:5e:00:53:04": {"up": 9000, "#,
                r#""house": {"most": 8000, "from": "0:0:0:0:0:ffff:c000::"}}, "#,
                r#""00:00:5e:00:53:05": {"up": 9000, "#,
                r#""house": {"most": 8000, "from": "::c000:207"}}, "#,
                r#""00:00:5e:00:53:06": {"up": 9000, "#,
                r#""house": {"most": 8000, "from": "1::ffff:a:b"}}, "#,
                r#""00:00:5e:00:53:07": {"up": 9000, "#,
                r#""house": {"most": 8000, "from": ["192.0.2.7", "::ffff:c000:207"]}}}"#,
            ),
        )
        .expect("houses at the bounds");
        assert_eq!(
            houses(&file).into_keys().collect::<Vec<_>>(),
            ["00:00:5e:00:53:01", "00:00:5e:00:53:06"],
            "the least, and IPv6 merely holding ffff, kept; past the most, or IPv4's written \
             as IPv6 in any form, alone or in a list, left out"
        );
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn each_network_s_line_is_read_by_its_router_within_what_nixos_takes() {
        let folder = std::env::temp_dir().join(format!("shaper-lines-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let file = folder.join("networks.json");
        assert!(lines(&file).is_empty(), "with no file");
        fs::write(
            &file,
            concat!(
                r#"{"00:00:5E:00:53:01": {"up": null, "down": 90000, "links": {}}, "#,
                r#""00:00:5e:00:53:02": {"up": 9000, "down": null}, "#,
                r#""00:00:5e:00:53:03": {"up": 9000, "down": 6000}, "#,
                r#""00:00:5e:00:53:04": {"up": 9000, "down": 90}, "#,
                r#""00:00:5e:00:53:05": {"up": 9000, "down": 10000001}}"#,
            ),
        )
        .expect("the lines");
        let network = |down| Line {
            network: Some(down),
            links: BTreeMap::new(),
        };
        assert_eq!(
            lines(&file),
            Lines::from([
                ("00:00:5e:00:53:01".to_string(), network(90_000)),
                ("00:00:5e:00:53:03".to_string(), network(6_000)),
            ]),
            "as nixos.nix writes them, null where none is set, the least kept, one in Mb/s \
             or past the most left out"
        );
        assert_eq!(
            anchors(&file).into_keys().collect::<Vec<_>>(),
            [
                "00:00:5e:00:53:02",
                "00:00:5e:00:53:03",
                "00:00:5e:00:53:04",
                "00:00:5e:00:53:05"
            ],
            "a network declaring only its download anchors no upload"
        );
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_link_s_own_line_holds_it_alone_and_the_network_s_holds_the_rest() {
        let folder = std::env::temp_dir().join(format!("shaper-links-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let file = folder.join("networks.json");
        fs::write(
            &file,
            concat!(
                r#"{"00:00:5e:00:53:01": {"up": 12000, "down": null, "links": "#,
                r#"{"wlan0": {"down": 20000}, "eth1": {"down": null}}}, "#,
                r#""00:00:5e:00:53:02": {"up": 9000, "down": 90000, "links": "#,
                r#"{"wlan0": {"down": 20000}, "eth1": {"down": null}, "eth2": {"down": 90}, "#,
                r#""eth3": "fast"}}, "#,
                r#""00:00:5e:00:53:03": {"up": 9000, "down": null, "links": {}}}"#,
            ),
        )
        .expect("the lines");
        let lines = lines(&file);
        let wifi_only = |name| line_of(&lines, "00:00:5e:00:53:01", name);
        assert_eq!(wifi_only("wlan0"), Some(20_000), "the link naming its own");
        assert_eq!(
            wifi_only("eth1"),
            None,
            "a link whose own is null, on a network declaring none, is held by none"
        );
        assert_eq!(wifi_only("eth0"), None, "a link the network doesn't name");
        let both = |name| line_of(&lines, "00:00:5e:00:53:02", name);
        assert_eq!(both("wlan0"), Some(20_000), "its own over the network's");
        assert_eq!(
            both("eth1"),
            Some(90_000),
            "a link whose own is null, as nixos.nix writes one left at its default, \
             takes the network's"
        );
        assert_eq!(
            both("eth2"),
            Some(90_000),
            "one in Mb/s is left out, so the link takes the network's"
        );
        assert_eq!(
            both("eth3"),
            Some(90_000),
            "one written wrongly loses only itself"
        );
        assert_eq!(both("eth0"), Some(90_000), "the network's");
        assert_eq!(
            line_of(&lines, "00:00:5e:00:53:03", "wlan0"),
            None,
            "a network declaring no line, for itself or a link, has none"
        );
        assert_eq!(
            line_of(&lines, "00:00:5e:00:53:09", "wlan0"),
            None,
            "a network not declared, though a link of its name is elsewhere"
        );
        assert_eq!(
            anchors(&file).len(),
            3,
            "links written beside the anchors lose none of them"
        );
        let _ = fs::remove_dir_all(&folder);
    }
}
