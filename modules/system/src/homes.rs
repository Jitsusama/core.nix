//! Where each thing lives on the local sites: its home, as the site that is
//! home to it says in its `homes.json` (the web-design skill's One Home for
//! Every Thing). An address is only ever read from there, never built by
//! hand, so every surface opens the same page, and a home that moves moves
//! everywhere at once. A site may instead say a pattern in its row, as the
//! home lists it in `rows.json`,
//! `{"kind": "machine", "address": "#machine={name}"}`, when
//! every thing of a kind has its page there by construction, as the system
//! page's machines do: the pattern is the site's own word, so it is read, not
//! guessed, and no record listing the things has to be kept beside a source
//! that already knows them. A pattern holds its whole kind, so a row can't
//! give a kind a pattern once an earlier row has claimed it by name, and no
//! row's record is read for a kind a pattern holds. Which row holds a kind
//! is decided by the rows alone, never by whether a record read. A
//! thing whose home site says nothing of it has no address, and none is
//! guessed.
//!
//! Long because its tests stay beside it: every rule here keeps a link
//! from leaving the sites or from being taken by the wrong row, and each
//! is pinned by the case that would break it.

use std::collections::{HashMap, HashSet};
use std::io::ErrorKind;
use std::path::Path;

use serde_json::Value;

use crate::record::System;

/// Every home the local sites say they hold, by kind and by each name a
/// page may call the thing.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Homes {
    addresses: HashMap<(String, String), String>,
    /// Each kind a site gives every thing of a home, as its address from the
    /// sites' root with `{name}` where the thing's name goes.
    patterns: HashMap<String, String>,
}

/// Where a pattern's address puts the name the system knows a thing by.
const NAME: &str = "{name}";

impl Homes {
    /// The homes the sites under `root` say they hold, by the rows the home
    /// lists in its `rows.json`, in `home`, the folder of its own it's
    /// built into, where it's given: there alone, so rows not yet built
    /// there, as after a reboot, hold none rather than a session's, since
    /// what's under `root` only a session writes then. A home built into
    /// `root` itself keeps its rows file there, and one from before it
    /// listed rows apart keeps every row in its record, `sites.json`, read
    /// only where the rows file is missing. Rows that can't be read hold
    /// none, rather than an older place's, which no longer keeps the live
    /// rows. A site's record that can't be read holds none, and the other
    /// sites' still count.
    pub fn read(root: &Path, home: Option<&Path>) -> Self {
        let parsed = |said: String| serde_json::from_str::<Value>(&said).ok();
        let places = match home {
            Some(home) => vec![home.join("rows.json")],
            None => vec![root.join("rows.json"), root.join("sites.json")],
        };
        let rows = places
            .into_iter()
            .map(std::fs::read_to_string)
            .find(|read| !matches!(read, Err(error) if error.kind() == ErrorKind::NotFound))
            .and_then(Result::ok);
        let Some(sites) = rows.and_then(parsed) else {
            return Self::default();
        };
        Self::from_sites(&sites, |site| {
            std::fs::read_to_string(root.join(site).join("homes.json"))
                .ok()
                .and_then(parsed)
        })
    }

    /// The homes `sites`, the home's rows, lists: each row naming kinds
    /// it's home to, read from its `homes.json` as `homes` gives it by the
    /// row's path, or patterns, which give every thing of a kind a home and
    /// read no record. A row's path may start with one `./`, as the home
    /// reads it. Where two records give one thing a home, the first row's
    /// address stands, as the rows order them. A pattern holds its kind
    /// against every later row, and against its own row's record, but not
    /// against an earlier row that claimed that kind by name, whether or
    /// not its record could be read. A row or a pattern this can't read
    /// claims nothing, so the next row that serves its kind does: the rows
    /// are kept newest first, and a row just added above the site already
    /// serving a kind mustn't take that site's links. A record gives only
    /// the kinds its row claims by name, and only addresses that are a
    /// fragment on its row's own page, their bytes plain.
    pub fn from_sites(sites: &Value, homes: impl Fn(&str) -> Option<Value>) -> Self {
        let mut addresses = HashMap::new();
        let mut patterns: HashMap<String, String> = HashMap::new();
        let mut named = HashSet::new();
        let rows = sites.get("sites").and_then(Value::as_array);
        for row in rows.into_iter().flatten() {
            let claims = row.get("homes").and_then(Value::as_array);
            let Some(claims) = claims.filter(|claims| !claims.is_empty()) else {
                continue;
            };
            let Some(path) = row
                .get("path")
                .and_then(Value::as_str)
                .map(|path| path.strip_prefix("./").unwrap_or(path))
                .filter(|path| inside(path))
            else {
                continue;
            };
            for claim in claims.iter().filter(|claim| claim.is_object()) {
                let Some(kind) = claim.get("kind").and_then(Value::as_str) else {
                    continue;
                };
                if patterns.contains_key(kind) || named.contains(kind) {
                    continue;
                }
                if let Some(address) = pattern(claim, path) {
                    patterns.insert(kind.to_owned(), format!("{path}{address}"));
                }
            }
            let recorded: HashSet<&str> = claims
                .iter()
                .filter_map(Value::as_str)
                .filter(|kind| !patterns.contains_key(*kind))
                .collect();
            named.extend(recorded.iter().map(|kind| (*kind).to_owned()));
            if recorded.is_empty() {
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
                let on_its_page = address
                    .strip_prefix(path)
                    .is_some_and(|rest| rest.starts_with('#'));
                if !recorded.contains(kind) || !on_its_page || !plain(address) {
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
    /// A thing with no name has no address, since a pattern would give it a
    /// link that names nothing.
    pub fn address(&self, kind: &str, name: &str) -> Option<String> {
        if name.is_empty() {
            return None;
        }
        self.addresses
            .get(&(kind.to_owned(), name.to_owned()))
            .cloned()
            .or_else(|| {
                let pattern = self.patterns.get(kind)?;
                Some(pattern.replace(NAME, &encoded(name)))
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

/// A claim's address, from its row's path on, when it is a pattern whose
/// address names the thing and is a fragment on the row's own page, its
/// bytes plain. It may be written from the row's page, `#machine={name}`,
/// or from the sites' root as a record's is, `system/#machine={name}`. A
/// fragment never reaches a server and can't change which page a browser
/// opens, so neither the pattern nor a name put in it can step off the site.
fn pattern<'a>(claim: &'a Value, path: &str) -> Option<&'a str> {
    let address = claim.get("address").and_then(Value::as_str)?;
    let address = address.strip_prefix(path).unwrap_or(address);
    (address.starts_with('#') && plain(address) && address.contains(NAME)).then_some(address)
}

/// Whether an address's bytes are visible ASCII with no backslash, double
/// quote, angle bracket or backtick, so it can't break out of a terminal's
/// link or a page's markup. An apostrophe passes, since `encodeURIComponent`
/// leaves one raw and the page escapes it. A record's address and a pattern
/// are held to it alike.
fn plain(address: &str) -> bool {
    address
        .bytes()
        .all(|byte| byte.is_ascii_graphic() && !b"\\\"<>`".contains(&byte))
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
/// name a site's own record and an address it starts stays there: steps of
/// letters, digits and `-._~` only, none empty but the last and none
/// starting with a dot, so no scheme, host, backslash, encoded or dotted
/// step can ride in on it, and none the sites' server refuses to serve.
fn inside(path: &str) -> bool {
    let plain = |step: &str| {
        step.bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-._~".contains(&byte))
    };
    let steps = path.strip_suffix('/').unwrap_or(path);
    !steps.is_empty()
        && steps
            .split('/')
            .all(|step| !step.is_empty() && !step.starts_with('.') && plain(step))
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
    fn a_thing_with_an_empty_name_is_found_by_its_others_and_never_by_none() {
        let homes = Homes::from_sites(
            &sites(&json!([{"path": "code/", "homes": ["repository"]}])),
            |_| Some(home("repository", "", "core", &[], "code/#repo=core")),
        );
        assert_eq!(homes.address("repository", ""), None);
        assert_eq!(
            homes.address("repository", "core"),
            Some("code/#repo=core".into())
        );
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
        for path in [
            "../elsewhere/",
            "/etc/",
            "a/../../b/",
            "./",
            "",
            "https://x/",
            "javascript:x/",
            "..\\b/",
            "%2e%2e/",
            ".drafts/",
            "a/.b/",
            "a//b/",
            "code//",
        ] {
            let homes = Homes::from_sites(
                &sites(&json!([{"path": path, "homes": ["machine"]}])),
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
            assert_eq!(homes, Homes::default(), "{path:?} isn't read");
        }
    }

    #[test]
    fn a_pattern_gives_every_thing_of_its_kind_a_home_with_no_record_read() {
        let asked = std::cell::RefCell::new(Vec::new());
        let homes = Homes::from_sites(
            &sites(&json!([{"path": "mycelium/", "homes": [
                {"kind": "machine", "address": "#machine={name}"},
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
            "the name is encoded"
        );
        assert_eq!(
            homes.address("machine", ".."),
            Some("mycelium/#machine=..".into()),
            "a name stays in the fragment"
        );
        assert_eq!(homes.address("machine", ""), None, "no name, no link");
        assert_eq!(homes.address("repository", "osprey"), None);
    }

    #[test]
    fn a_row_may_say_kinds_and_patterns_together() {
        let homes = Homes::from_sites(
            &sites(&json!([{"path": "mycelium/", "homes": [
                "session",
                {"kind": "machine", "address": "#machine={name}"},
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
                {"path": "mycelium/", "homes": [
                    {"kind": "machine", "address": "#machine={name}"},
                ]},
                {"path": "health/", "homes": [
                    "machine",
                    {"kind": "machine", "address": "#m={name}"},
                ]},
            ])),
            |_| Some(home("machine", "osprey", "osprey", &[], "health/#osprey")),
        );
        assert_eq!(
            homes.address("machine", "osprey"),
            Some("mycelium/#machine=osprey".into())
        );
    }

    // The local sites' declaration refuses a row giving one kind two
    // patterns, but sites.json is read as it is, and nothing checks a row
    // there, so a row that gives two is read as Code's page and the skill's
    // reader read it: the first pattern that can be read keeps the kind.
    #[test]
    fn a_rows_first_readable_pattern_for_a_kind_holds_against_its_later_ones() {
        let homes = Homes::from_sites(
            &sites(&json!([{"path": "mycelium/", "homes": [
                {"kind": "machine", "address": "#machine={name}"},
                {"kind": "machine", "address": "#m={name}"},
            ]}])),
            |_| None,
        );
        assert_eq!(
            homes.address("machine", "osprey"),
            Some("mycelium/#machine=osprey".into())
        );
        let homes = Homes::from_sites(
            &sites(&json!([{"path": "mycelium/", "homes": [
                {"kind": "machine", "address": "../{name}"},
                {"kind": "machine", "address": "#m={name}"},
                {"kind": "machine", "address": "#machine={name}"},
            ]}])),
            |_| None,
        );
        assert_eq!(
            homes.address("machine", "osprey"),
            Some("mycelium/#m=osprey".into()),
            "one that can't be read claims nothing"
        );
    }

    #[test]
    fn the_first_row_to_claim_a_kind_by_record_keeps_all_of_it() {
        let homes = Homes::from_sites(
            &sites(&json!([
                {"path": "code/", "homes": ["repository"]},
                {"path": "health/", "homes": [
                    {"kind": "repository", "address": "#r={name}"},
                ]},
            ])),
            |_| Some(home("repository", "core", "core", &[], "code/#repo=core")),
        );
        assert_eq!(
            homes.address("repository", "core"),
            Some("code/#repo=core".into())
        );
        assert_eq!(
            homes.address("repository", "new"),
            None,
            "a later row's pattern fills no gap in the record"
        );
    }

    #[test]
    fn a_kind_claimed_by_name_holds_against_a_later_pattern_when_its_record_cant_be_read() {
        let homes = Homes::from_sites(
            &sites(&json!([
                {"path": "code/", "homes": ["repository"]},
                {"path": "health/", "homes": [
                    {"kind": "repository", "address": "#r={name}"},
                ]},
            ])),
            |_| None,
        );
        assert_eq!(homes.address("repository", "core"), None);
    }

    #[test]
    fn rows_claiming_a_kind_by_name_give_each_thing_its_records_home() {
        let homes = Homes::from_sites(
            &sites(&json!([
                {"path": "mycelium/", "homes": ["machine"]},
                {"path": "health/", "homes": ["machine"]},
            ])),
            |site| {
                let name = if site == "mycelium/" {
                    "laptop"
                } else {
                    "studio"
                };
                Some(home("machine", name, name, &[], &format!("{site}#{name}")))
            },
        );
        assert_eq!(
            homes.address("machine", "laptop"),
            Some("mycelium/#laptop".into())
        );
        assert_eq!(
            homes.address("machine", "studio"),
            Some("health/#studio".into())
        );
    }

    #[test]
    fn a_record_gives_only_the_kinds_its_row_claims() {
        let homes = Homes::from_sites(
            &sites(&json!([{"path": "code/", "homes": ["repository"]}])),
            |_| {
                Some(json!({"things": [
                    {"kind": "repository", "key": "core", "address": "code/#repo=core"},
                    {"kind": "tree", "key": "core", "address": "code/#tree=core"},
                ]}))
            },
        );
        assert_eq!(
            homes.address("repository", "core"),
            Some("code/#repo=core".into())
        );
        assert_eq!(homes.address("tree", "core"), None);
    }

    #[test]
    fn a_pattern_on_a_row_off_the_root_is_not_read() {
        for path in ["https://x/", "javascript:x/", "..\\b/", "%2e%2e/", "../"] {
            let homes = Homes::from_sites(
                &sites(&json!([{"path": path, "homes": [
                    {"kind": "machine", "address": "#machine={name}"},
                ]}])),
                |_| None,
            );
            assert_eq!(homes.address("machine", "osprey"), None, "{path:?}");
        }
    }

    #[test]
    fn a_rows_pattern_outranks_its_own_record_for_that_kind() {
        let homes = Homes::from_sites(
            &sites(&json!([{"path": "mycelium/", "homes": [
                "session",
                "machine",
                {"kind": "machine", "address": "#machine={name}"},
            ]}])),
            |_| {
                Some(json!({"things": [
                    {"kind": "session", "key": "abc", "address": "mycelium/#session=abc"},
                    {"kind": "machine", "key": "osprey", "address": "mycelium/#old=osprey"},
                ]}))
            },
        );
        assert_eq!(
            homes.address("machine", "osprey"),
            Some("mycelium/#machine=osprey".into()),
            "a leftover record doesn't outlast the pattern"
        );
        assert_eq!(
            homes.address("session", "abc"),
            Some("mycelium/#session=abc".into())
        );
    }

    #[test]
    fn a_pattern_off_its_page_or_naming_nothing_is_not_read() {
        for address in [
            "#machine",
            "#machine={key}",
            "machines/{name}",
            "/etc/{name}",
            "../{name}",
            "%2e%2e/{name}",
            "..\\code/#repo={name}",
            "https://x/{name}",
            "//x/{name}",
            "?machine={name}",
            "#machine={name}\u{1b}]8;;",
            "#machine=\"{name}\"",
            "#machine=\\{name}",
            "#machine=<{name}",
            "#machine={name}>",
            "#machine=`{name}`",
            "#machine={name}\u{202e}",
            "#machine={name} x",
            "code/#machine={name}",
            "mycelium/x#machine={name}",
            "mycelium/#machine",
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
    fn a_pattern_may_be_written_from_the_root_like_a_record() {
        let homes = Homes::from_sites(
            &sites(&json!([{"path": "mycelium/", "homes": [
                {"kind": "machine", "address": "mycelium/#machine={name}"},
            ]}])),
            |_| None,
        );
        assert_eq!(
            homes.address("machine", "osprey"),
            Some("mycelium/#machine=osprey".into())
        );
    }

    #[test]
    fn a_broken_pattern_leaves_its_own_rows_record_serving() {
        let broken = json!({"kind": "machine", "address": "#machine"});
        for claims in [json!(["machine", broken]), json!([broken, "machine"])] {
            let homes = Homes::from_sites(
                &sites(&json!([
                    {"path": "mycelium/", "homes": claims},
                    {"path": "later/", "homes": [
                        {"kind": "machine", "address": "#m={name}"},
                    ]},
                ])),
                |site| {
                    Some(home(
                        "machine",
                        "laptop",
                        "laptop",
                        &[],
                        &format!("{site}#laptop"),
                    ))
                },
            );
            assert_eq!(
                homes.address("machine", "laptop"),
                Some("mycelium/#laptop".into()),
                "{claims}"
            );
            assert_eq!(homes.address("machine", "osprey"), None, "{claims}");
        }
    }

    #[test]
    fn a_row_that_isnt_read_leaves_its_kind_to_the_next_row_that_serves_it() {
        // The rows are kept newest first, so a row this can't read is most
        // often one just added above the site that already serves the kind.
        for first in [
            json!({"path": "mycelium/", "homes": [
                {"kind": "machine", "address": "#machine"},
            ]}),
            json!({"path": "/mycelium/", "homes": ["machine"]}),
            json!({"path": "/mycelium/", "homes": [
                {"kind": "machine", "address": "#machine={name}"},
            ]}),
            json!({"path": "mycelium/", "homes": "machine"}),
            json!({"path": "mycelium/", "homes": [
                {"Kind": "machine", "address": "#machine={name}"},
            ]}),
        ] {
            // The next row serves by its record, and by a pattern, which an
            // earlier claim on the kind would stop.
            for next in [
                json!({"path": "health/", "homes": ["machine"]}),
                json!({"path": "health/", "homes": [
                    {"kind": "machine", "address": "#m={name}"},
                ]}),
            ] {
                let homes = Homes::from_sites(&sites(&json!([first, next])), |site| {
                    Some(home(
                        "machine",
                        "osprey",
                        "osprey",
                        &[],
                        &format!("{site}#m=osprey"),
                    ))
                });
                assert_eq!(
                    homes.address("machine", "osprey"),
                    Some("health/#m=osprey".into()),
                    "{first} then {next}"
                );
            }
        }
    }

    #[test]
    fn a_records_address_off_its_page_or_with_bytes_a_link_cant_hold_is_not_read() {
        let homes = Homes::from_sites(
            &sites(&json!([{"path": "code/", "homes": ["repository"]}])),
            |_| {
                Some(json!({"things": [
                    {"kind": "repository", "key": "core", "address": "code/#repo=core"},
                    {"kind": "repository", "key": "notes", "address": "code/#tree=joel's"},
                    {"kind": "repository", "key": "off", "address": "health/#r=off"},
                    {"kind": "repository", "key": "up", "address": "code/../health/#r=up"},
                    {"kind": "repository", "key": "coded", "address": "code/%2e%2e/h/#r=x"},
                    {"kind": "repository", "key": "deeper", "address": "code/x/#r=x"},
                    {"kind": "repository", "key": "esc", "address": "code/#x\u{1b}]8;;"},
                    {"kind": "repository", "key": "slash", "address": "code/#a\\b"},
                    {"kind": "repository", "key": "quote", "address": "code/#\"x"},
                    {"kind": "repository", "key": "angle", "address": "code/#<x"},
                    {"kind": "repository", "key": "tick", "address": "code/#`x"},
                    {"kind": "repository", "key": "space", "address": "code/#a b"},
                ]}))
            },
        );
        assert_eq!(
            homes.address("repository", "core"),
            Some("code/#repo=core".into())
        );
        assert_eq!(
            homes.address("repository", "notes"),
            Some("code/#tree=joel's".into()),
            "an apostrophe is left raw by encodeURIComponent"
        );
        for name in [
            "off", "up", "coded", "deeper", "esc", "slash", "quote", "angle", "tick", "space",
        ] {
            assert_eq!(homes.address("repository", name), None, "{name:?}");
        }
    }

    /// A home's folder holding the files `files` names, each with its text,
    /// fresh under the system's temporary folder for the test `name`.
    fn folder(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("homes-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (file, said) in files {
            let path = root.join(file);
            std::fs::create_dir_all(path.parent().expect("a file sits in a folder"))
                .expect("the home's folder is made");
            std::fs::write(path, said).expect("the home's file is written");
        }
        root
    }

    /// A row home to repositories at `path`, as the home writes it.
    fn repositories(path: &str) -> String {
        json!({"sites": [{"path": path, "homes": ["repository"]}]}).to_string()
    }

    /// A site's record at `site`, home to the repository core.
    fn core_at(site: &str) -> String {
        home(
            "repository",
            "core",
            "core",
            &[],
            &format!("{site}#repo=core"),
        )
        .to_string()
    }

    #[test]
    fn the_home_reads_its_rows_before_its_record() {
        let old = home("repository", "other", "other", &[], "old/#repo=other").to_string();
        let root = folder(
            "rows",
            &[
                ("rows.json", &repositories("code/")),
                ("sites.json", &repositories("old/")),
                ("code/homes.json", &core_at("code/")),
                ("old/homes.json", &old),
            ],
        );
        let homes = Homes::read(&root, None);
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(
            homes.address("repository", "core"),
            Some("code/#repo=core".into()),
            "the rows the home lists hold"
        );
        assert_eq!(
            homes.address("repository", "other"),
            None,
            "the record's rows aren't read beside the rows file"
        );
    }

    #[test]
    fn a_home_of_its_own_is_read_there_alone_and_the_sites_folder_only_without_one() {
        let old = home("repository", "other", "other", &[], "old/#repo=other").to_string();
        let root = folder(
            "own",
            &[
                ("home/rows.json", &repositories("code/")),
                ("rows.json", &repositories("old/")),
                ("code/homes.json", &core_at("code/")),
                ("old/homes.json", &old),
            ],
        );
        let homes = Homes::read(&root, Some(&root.join("home")));
        let unbuilt = Homes::read(&root, Some(&root.join("nowhere")));
        let before = Homes::read(&root, None);
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(
            homes.address("repository", "core"),
            Some("code/#repo=core".into()),
            "the rows the home lists in its own folder hold"
        );
        assert_eq!(
            homes.address("repository", "other"),
            None,
            "the rows a build before left in the sites' folder aren't read beside them"
        );
        assert_eq!(
            unbuilt,
            Homes::default(),
            "a session's rows in the sites' folder stood in for a home not yet built"
        );
        assert_eq!(
            before.address("repository", "other"),
            Some("old/#repo=other".into()),
            "a home built into the sites' folder is read there"
        );
    }

    #[test]
    fn rows_that_are_there_but_wont_open_hold_no_homes() {
        let root = folder(
            "unreadable",
            &[
                ("rows.json/kept", "a folder where the rows file goes"),
                ("sites.json", &repositories("old/")),
                ("old/homes.json", &core_at("old/")),
            ],
        );
        let homes = Homes::read(&root, None);
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(
            homes,
            Homes::default(),
            "only a missing rows file sends the reader to the record"
        );
    }

    #[test]
    fn a_home_with_rows_and_no_record_reads_its_rows() {
        let root = folder(
            "no-record",
            &[
                ("rows.json", &repositories("code/")),
                ("code/homes.json", &core_at("code/")),
            ],
        );
        let homes = Homes::read(&root, None);
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(
            homes.address("repository", "core"),
            Some("code/#repo=core".into()),
            "a home that has only ever listed rows apart needs no record"
        );
    }

    #[test]
    fn a_home_with_no_rows_reads_its_record() {
        let root = folder(
            "record",
            &[
                ("sites.json", &repositories("old/")),
                ("old/homes.json", &core_at("old/")),
            ],
        );
        let homes = Homes::read(&root, None);
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(
            homes.address("repository", "core"),
            Some("old/#repo=core".into()),
            "a home from before it listed its rows apart keeps them in its record"
        );
    }

    #[test]
    fn rows_that_wont_parse_hold_no_homes_rather_than_the_records() {
        let root = folder(
            "broken",
            &[
                ("rows.json", "{"),
                ("sites.json", &repositories("old/")),
                ("old/homes.json", &core_at("old/")),
            ],
        );
        let homes = Homes::read(&root, None);
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(
            homes,
            Homes::default(),
            "the record no longer keeps the live rows, so it isn't read"
        );
    }

    #[test]
    fn a_rows_path_may_start_at_its_own_folder_once() {
        let homes = Homes::from_sites(
            &sites(&json!([{"path": "./code/", "homes": ["repository"]}])),
            |site| {
                (site == "code/")
                    .then(|| home("repository", "core", "core", &[], "code/#repo=core"))
            },
        );
        assert_eq!(
            homes.address("repository", "core"),
            Some("code/#repo=core".into()),
            "a row at ./code/ is the site at code/"
        );
        // Only the row's path may start at its own folder: an address is
        // written from the sites' root, on a ./ row too, as code's reader
        // holds it (modules/repos/site/tests/homes.test.ts). The record and
        // the pattern beside each refused one show the row itself is read.
        let homes = Homes::from_sites(
            &sites(&json!([{"path": "./code/", "homes": ["repository"]}])),
            |_| {
                Some(json!({"things": [
                    {"kind": "repository", "key": "core", "address": "./code/#c=1"},
                    {"kind": "repository", "key": "kit", "address": "code/#c=2"},
                ]}))
            },
        );
        assert_eq!(
            homes.address("repository", "core"),
            None,
            "a record's address written from ./ isn't read, on a ./ row"
        );
        assert_eq!(
            homes.address("repository", "kit"),
            Some("code/#c=2".into()),
            "the ./ row itself was refused"
        );
        let homes = Homes::from_sites(
            &sites(&json!([{"path": "./mycelium/", "homes": [
                {"kind": "machine", "address": "./mycelium/#host={name}"},
                {"kind": "machine", "address": "#machine={name}"},
            ]}])),
            |_| None,
        );
        assert_eq!(
            homes.address("machine", "osprey"),
            Some("mycelium/#machine=osprey".into()),
            "a pattern written from ./ isn't read, on a ./ row"
        );
        let homes = Homes::from_sites(
            &sites(&json!([{"path": "./map/", "homes": [
                {"kind": "machine", "address": "#machine={name}"},
            ]}])),
            |_| None,
        );
        assert_eq!(
            homes.address("machine", "kestrel"),
            Some("map/#machine=kestrel".into()),
            "a pattern on a row at ./map/ is on the site at map/"
        );
        for path in ["././code/", "./", "./../code/", ".//code/"] {
            let homes = Homes::from_sites(
                &sites(&json!([{"path": path, "homes": ["repository"]}])),
                |site| {
                    Some(home(
                        "repository",
                        "core",
                        "core",
                        &[],
                        &format!("{site}#repo=core"),
                    ))
                },
            );
            assert_eq!(homes, Homes::default(), "{path:?} isn't read");
        }
    }

    #[test]
    fn a_record_on_a_path_with_no_closing_slash_stays_on_its_page() {
        let homes = Homes::from_sites(
            &sites(&json!([{"path": "code", "homes": ["repository"]}])),
            |_| {
                Some(json!({"things": [
                    {"kind": "repository", "key": "core", "address": "code#repo=core"},
                    {"kind": "repository", "key": "other", "address": "codex/#repo=other"},
                ]}))
            },
        );
        assert_eq!(
            homes.address("repository", "core"),
            Some("code#repo=core".into())
        );
        assert_eq!(homes.address("repository", "other"), None);
    }

    #[test]
    fn each_thing_is_given_its_homes_address_and_one_without_a_home_none() {
        let thing = |name: &str| Thing {
            kind: "machine",
            name: name.to_owned(),
            health: Health::Good,
            says: String::new(),
            side: Vec::new(),
            looks: Vec::new(),
            address: Some("stale/".to_owned()),
            ties: serde_json::Value::Null,
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
