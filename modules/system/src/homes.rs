//! Where each thing lives on the local sites: its home, as the site that is
//! home to it says in its `homes.json` (the web-design skill's One Home for
//! Every Thing). An address is only ever read from there, never built by
//! hand, so every surface opens the same page, and a home that moves moves
//! everywhere at once. A thing whose home site says nothing of it has no
//! address, and none is guessed.

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;

use crate::record::System;

/// Every home the local sites say they hold, by kind and by each name a
/// page may call the thing.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Homes {
    addresses: HashMap<(String, String), String>,
}

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
    /// it's home to, its `homes.json` as `homes` gives it by the row's path.
    /// Where two sites claim one thing, the first row's address stands, as
    /// the record orders them.
    pub fn from_sites(sites: &Value, homes: impl Fn(&str) -> Option<Value>) -> Self {
        let mut addresses = HashMap::new();
        let rows = sites.get("sites").and_then(Value::as_array);
        for row in rows.into_iter().flatten() {
            let claims = row.get("homes").and_then(Value::as_array);
            if claims.is_none_or(Vec::is_empty) {
                continue;
            }
            let Some(path) = row
                .get("path")
                .and_then(Value::as_str)
                .filter(|path| inside(path))
            else {
                continue;
            };
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
                for name in names(thing) {
                    addresses
                        .entry((kind.to_owned(), name.to_owned()))
                        .or_insert_with(|| address.to_owned());
                }
            }
        }
        Self { addresses }
    }

    /// The address of a thing's home, from the sites' root, when a site is
    /// home to it.
    pub fn address(&self, kind: &str, name: &str) -> Option<&str> {
        self.addresses
            .get(&(kind.to_owned(), name.to_owned()))
            .map(String::as_str)
    }

    /// Gives each thing in `system` the address of its home, where it has
    /// one.
    pub fn place(&self, system: &mut System) {
        for thing in system.parts.iter_mut().flat_map(|part| &mut part.things) {
            thing.address = self.address(thing.kind, &thing.name).map(str::to_owned);
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
                Some("code/#repo=Jitsusama%2Fcore.nix"),
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
            Some("mycelium/#machine=laptop")
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
            Some("mycelium/#machine=laptop")
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
            Some("mycelium/#machine=laptop")
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
    fn each_thing_is_given_its_homes_address_and_one_without_a_home_none() {
        let thing = |name: &str| Thing {
            kind: "machine",
            name: name.to_owned(),
            health: Health::Good,
            says: String::new(),
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
