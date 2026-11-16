//! Where each thing lives on the local sites: its home, as the site that is
//! home to it says in its `homes.json` (the web-design skill's One Home for
//! Every Thing). An address is only ever read from there, never built by
//! hand, so every surface opens the same page, and a home that moves moves
//! everywhere at once. A site may instead say a pattern in its row of
//! `sites.json`, `{"kind": "machine", "address": "#machine={key}"}`, when
//! every thing of a kind has its page there by construction, as the mycelium
//! map's machines do: the pattern is the site's own word, so it is read, not
//! guessed, and no record listing the things has to be kept beside a source
//! that already knows them. A thing whose home site says nothing of it has
//! no address, and none is guessed.

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;

use crate::record::System;

/// Every home the local sites say they hold, by kind and by each name a
/// page may call the thing.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Homes {
    addresses: HashMap<(String, String), String>,
    /// Each kind a site gives every thing of a home, as its address from the
    /// sites' root with `{key}` where the thing's name goes.
    patterns: HashMap<String, String>,
}

/// Where a pattern's address puts the thing's name.
const KEY: &str = "{key}";

impl Homes {
    /// The homes the sites under `root` say they hold. A record that can't
    /// be read holds none, and the other sites' still count.
    pub fn read(root: &Path) -> Self {
        let read = |path: &Path| -> Option<Value> {
            let said = std::fs::read_to_string(path).ok()?;
            serde_json::from_str(&said).ok()
        };
        let Some(sites) = read(&root.join("sites.json")) else {
            return Self::default();
        };
        Self::from_sites(&sites, |site| read(&root.join(site).join("homes.json")))
    }

    /// The homes `sites`, the sites' record, lists: each row naming kinds
    /// it's home to, its `homes.json` as `homes` gives it by the row's path,
    /// or patterns, which give every thing of a kind a home and read no
    /// record. Where two sites claim one thing, the first row's address
    /// stands, as the record orders them.
    pub fn from_sites(sites: &Value, homes: impl Fn(&str) -> Option<Value>) -> Self {
        let mut addresses = HashMap::new();
        let mut patterns = HashMap::new();
        let rows = sites.get("sites").and_then(Value::as_array);
        for row in rows.into_iter().flatten() {
            let claims = row.get("homes").and_then(Value::as_array);
            let Some(claims) = claims.filter(|claims| !claims.is_empty()) else {
                continue;
            };
            let Some(path) = row
                .get("path")
                .and_then(Value::as_str)
                .filter(|path| inside(path))
            else {
                continue;
            };
            // A kind an earlier row's pattern holds stays that row's, so
            // this row's record can't take one of its things.
            let held: Vec<String> = patterns.keys().cloned().collect();
            for (kind, address) in claims.iter().filter_map(pattern) {
                patterns
                    .entry(kind.to_owned())
                    .or_insert_with(|| format!("{path}{address}"));
            }
            if !claims.iter().any(Value::is_string) {
                continue;
            }
            let Some(said) = homes(path) else {
                continue;
            };
            let things = said.get("things").and_then(Value::as_array);
            for thing in things.into_iter().flatten() {
                let (Some(kind), Some(address)) = (
                    thing.get("kind").and_then(Value::as_str),
                    thing.get("address").and_then(Value::as_str),
                ) else {
                    continue;
                };
                if held.iter().any(|held| held == kind) {
                    continue;
                }
                for name in names(thing) {
                    addresses
                        .entry((kind.to_owned(), name.to_owned()))
                        .or_insert_with(|| address.to_owned());
                }
            }
        }
        Self {
            addresses,
            patterns,
        }
    }

    /// The address of a thing's home, from the sites' root, when a site is
    /// home to it: the one its record gives it, or else its kind's pattern
    /// with its name in place.
    pub fn address(&self, kind: &str, name: &str) -> Option<String> {
        self.addresses
            .get(&(kind.to_owned(), name.to_owned()))
            .cloned()
            .or_else(|| {
                let pattern = self.patterns.get(kind)?;
                Some(pattern.replace(KEY, &encoded(name)))
            })
    }

    /// Gives each thing in `system` the address of its home, where it has
    /// one.
    pub fn place(&self, system: &mut System) {
        for thing in system.parts.iter_mut().flat_map(|part| &mut part.things) {
            thing.address = self.address(thing.kind, &thing.name);
        }
    }
}

/// Every name a thing's home is found by: its key, its name, and any other
/// name a page may call it.
fn names(thing: &Value) -> impl Iterator<Item = &str> {
    let key = thing.get("key").and_then(Value::as_str);
    let name = thing.get("name").and_then(Value::as_str);
    let others = thing
        .get("names")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str);
    key.into_iter().chain(name).chain(others)
}

/// A claim's kind and address when it is a pattern whose address names the
/// thing's key and stays on the row's own site: a fragment, query or path
/// below it, never a root, a parent or another host.
fn pattern(claim: &Value) -> Option<(&str, &str)> {
    let kind = claim.get("kind").and_then(Value::as_str)?;
    let address = claim.get("address").and_then(Value::as_str)?;
    let stays = !address.starts_with('/')
        && !address.contains("://")
        && address.split(['/', '#', '?']).all(|step| step != "..");
    (address.contains(KEY) && stays).then_some((kind, address))
}

/// A name as it may sit in an address: every byte but the unreserved ones
/// percent-encoded, so a name with a slash, a space or an ampersand still
/// reaches the page whole.
fn encoded(name: &str) -> String {
    name.bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

/// Whether a site's path stays under the sites' root, so a row can only
/// name a site's own record.
fn inside(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && path.split('/').all(|step| step != ".." && step != ".")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::{Deeper, Health, Part, Thing};
    use serde_json::json;

    fn sites(rows: &Value) -> Value {
        json!({"about": "every site", "sites": rows})
    }

    /// A site's homes.json, home to one thing.
    fn home(kind: &str, key: &str, name: &str, names: &[&str], address: &str) -> Value {
        json!({"things": [
            {"kind": kind, "key": key, "name": name, "names": names, "address": address},
        ]})
    }

    #[test]
    fn a_thing_is_found_by_its_key_its_name_and_its_other_names() {
        let homes = Homes::from_sites(
            &sites(&json!([{"path": "code/", "homes": ["repository"]}])),
            |_| {
                Some(home(
                    "repository",
                    "github:Jitsusama/core.nix",
                    "Jitsusama/core.nix",
                    &["core.nix"],
                    "code/#repo=Jitsusama%2Fcore.nix",
                ))
            },
        );
        for name in [
            "github:Jitsusama/core.nix",
            "Jitsusama/core.nix",
            "core.nix",
        ] {
            assert_eq!(
                homes.address("repository", name),
                Some("code/#repo=Jitsusama%2Fcore.nix".into()),
                "{name} finds its home"
            );
        }
        assert_eq!(homes.address("tree", "core.nix"), None, "a kind is its own");
    }

    #[test]
    fn only_a_site_that_says_it_is_a_home_is_read() {
        let asked = std::cell::RefCell::new(Vec::new());
        let homes = Homes::from_sites(
            &sites(&json!([
                {"path": "system/"},
                {"path": "network/", "homes": []},
                {"path": "mycelium/", "homes": ["machine"]},
            ])),
            |site| {
                asked.borrow_mut().push(site.to_owned());
                Some(home(
                    "machine",
                    "laptop",
                    "laptop",
                    &[],
                    "mycelium/#machine=laptop",
                ))
            },
        );
        assert_eq!(asked.into_inner(), vec!["mycelium/"]);
        assert_eq!(
            homes.address("machine", "laptop"),
            Some("mycelium/#machine=laptop".into())
        );
    }

    #[test]
    fn a_record_that_cant_be_read_holds_no_homes_and_the_rest_still_count() {
        let homes = Homes::from_sites(
            &sites(&json!([
                {"path": "broken/", "homes": ["machine"]},
                {"path": "half/", "homes": ["machine"]},
                {"path": "mycelium/", "homes": ["machine"]},
            ])),
            |site| match site {
                "broken/" => None,
                "half/" => Some(json!({"things": [{"kind": "machine", "name": "studio"}]})),
                _ => Some(home(
                    "machine",
                    "laptop",
                    "laptop",
                    &[],
                    "mycelium/#machine=laptop",
                )),
            },
        );
        assert_eq!(
            homes.address("machine", "studio"),
            None,
            "a home with no address"
        );
        assert_eq!(
            homes.address("machine", "laptop"),
            Some("mycelium/#machine=laptop".into())
        );
        assert_eq!(
            Homes::from_sites(&json!({"sites": "none"}), |_| None),
            Homes::default()
        );
    }

    #[test]
    fn the_first_site_to_claim_a_thing_keeps_it() {
        let homes = Homes::from_sites(
            &sites(&json!([
                {"path": "mycelium/", "homes": ["machine"]},
                {"path": "health/", "homes": ["machine"]},
            ])),
            |site| {
                Some(home(
                    "machine",
                    "laptop",
                    "laptop",
                    &[],
                    &format!("{site}#machine=laptop"),
                ))
            },
        );
        assert_eq!(
            homes.address("machine", "laptop"),
            Some("mycelium/#machine=laptop".into())
        );
    }

    #[test]
    fn a_row_can_only_name_a_site_under_the_root() {
        for path in ["../elsewhere/", "/etc/", "a/../../b/", "./", ""] {
            let homes = Homes::from_sites(
                &sites(&json!([{"path": path, "homes": ["machine"]}])),
                |_| {
                    Some(home(
                        "machine",
                        "laptop",
                        "laptop",
                        &[],
                        "x/#machine=laptop",
                    ))
                },
            );
            assert_eq!(homes, Homes::default(), "{path:?} isn't read");
        }
    }

    #[test]
    fn a_pattern_gives_every_thing_of_its_kind_a_home_with_no_record_read() {
        let asked = std::cell::RefCell::new(Vec::new());
        let homes = Homes::from_sites(
            &sites(&json!([{"path": "mycelium/", "homes": [
                {"kind": "machine", "address": "#machine={key}"},
            ]}])),
            |site| {
                asked.borrow_mut().push(site.to_owned());
                None
            },
        );
        assert_eq!(asked.into_inner(), Vec::<String>::new(), "no homes.json");
        assert_eq!(
            homes.address("machine", "osprey"),
            Some("mycelium/#machine=osprey".into())
        );
        assert_eq!(
            homes.address("machine", "a b/c&d"),
            Some("mycelium/#machine=a%20b%2Fc%26d".into()),
            "the key is encoded"
        );
        assert_eq!(homes.address("repository", "osprey"), None);
    }

    #[test]
    fn a_row_may_say_kinds_and_patterns_together() {
        let homes = Homes::from_sites(
            &sites(&json!([{"path": "mycelium/", "homes": [
                "session",
                {"kind": "machine", "address": "#machine={key}"},
            ]}])),
            |_| Some(home("session", "abc", "abc", &[], "mycelium/#session=abc")),
        );
        assert_eq!(
            homes.address("session", "abc"),
            Some("mycelium/#session=abc".into())
        );
        assert_eq!(
            homes.address("machine", "osprey"),
            Some("mycelium/#machine=osprey".into())
        );
    }

    #[test]
    fn the_first_row_to_claim_a_kind_by_pattern_keeps_it() {
        let homes = Homes::from_sites(
            &sites(&json!([
                {"path": "mycelium/", "homes": [{"kind": "machine", "address": "#machine={key}"}]},
                {"path": "health/", "homes": ["machine", {"kind": "machine", "address": "#m={key}"}]},
            ])),
            |_| Some(home("machine", "osprey", "osprey", &[], "health/#osprey")),
        );
        assert_eq!(
            homes.address("machine", "osprey"),
            Some("mycelium/#machine=osprey".into())
        );
    }

    #[test]
    fn a_pattern_that_names_no_key_or_leaves_the_site_is_not_read() {
        for address in [
            "#machine",
            "/etc/{key}",
            "../{key}",
            "https://x/{key}",
            "//x/{key}",
        ] {
            let homes = Homes::from_sites(
                &sites(&json!([{"path": "mycelium/", "homes": [
                    {"kind": "machine", "address": address},
                ]}])),
                |_| None,
            );
            assert_eq!(homes, Homes::default(), "{address:?} isn't read");
        }
    }

    #[test]
    fn each_thing_is_given_its_homes_address_and_one_without_a_home_none() {
        let thing = |name: &str| Thing {
            kind: "machine",
            name: name.to_owned(),
            health: Health::Good,
            says: String::new(),
            side: Vec::new(),
            address: Some("stale/".to_owned()),
        };
        let mut system = System {
            machine: "laptop".to_owned(),
            read_at: "2026-11-16T00:00:00Z".to_owned(),
            parts: vec![Part {
                name: "machines",
                health: Health::Good,
                says: String::new(),
                side: Vec::new(),
                holds: None,
                things: vec![thing("laptop"), thing("studio")],
                deeper: Deeper {
                    command: "mycelium machines",
                    site: "mycelium/",
                    launcher: "Machine",
                },
            }],
        };
        let homes = Homes::from_sites(
            &sites(&json!([{"path": "mycelium/", "homes": ["machine"]}])),
            |_| {
                Some(home(
                    "machine",
                    "laptop",
                    "laptop",
                    &[],
                    "mycelium/#machine=laptop",
                ))
            },
        );
        homes.place(&mut system);
        let addresses: Vec<Option<&str>> = system.parts[0]
            .things
            .iter()
            .map(|thing| thing.address.as_deref())
            .collect();
        assert_eq!(addresses, vec![Some("mycelium/#machine=laptop"), None]);
    }
}
