//! What the server is told by its module: where it listens, the names a
//! browser reaches it by, the folder of documents sessions write, the
//! folders Nix built that it serves at their own paths, and each feed a
//! page may read, with the kinds and fields that feed may show. One JSON
//! file written by Nix, so nothing a page asks can widen it.

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// Everything the server serves, as its module declared it.
#[derive(Debug, Clone)]
pub struct Config {
    /// The address it listens on, a loopback one.
    pub listen: SocketAddr,
    /// The names a browser may reach it by, such as `sites.localhost`; a
    /// request naming any other is refused, so a page elsewhere that
    /// rebinds its own name to this machine reads nothing.
    pub names: Vec<String>,
    /// The folder of documents sessions write, served below every mount.
    pub documents: PathBuf,
    /// Folders Nix built, each served at `/<name>/` ahead of the documents.
    pub mounts: BTreeMap<String, PathBuf>,
    /// The daemons a page may read, by the name in `/feed/<name>`.
    pub feeds: BTreeMap<String, Feed>,
}

/// One daemon a page may read, and how much of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feed {
    /// Its socket, absolute, or under the user's runtime folder.
    pub socket: PathBuf,
    /// The kinds a page may read, each with what of it may be shown.
    pub kinds: BTreeMap<String, Kind>,
}

/// What of a kind a page may read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Kind {
    /// The only fields shown, besides the key; every field where none are
    /// named.
    pub fields: Option<Vec<String>>,
    /// The fields asked of the daemon deep, so a change inside one says
    /// only what moved in it (the node's `deep`).
    pub deep: Option<Vec<String>>,
    /// How many of the kind's records, the latest heard, the daemon is
    /// asked to keep a page told of (the node's `latest`); every one where
    /// none is named.
    pub latest: Option<u64>,
}

impl Config {
    /// Reads the config from its JSON, a relative socket taken as under
    /// `runtime`, the user's runtime folder.
    pub fn from_json(text: &str, runtime: Option<&Path>) -> Result<Self, String> {
        let said: Value =
            serde_json::from_str(text).map_err(|why| format!("it isn't JSON: {why}"))?;
        let listen = text_of(&said, "listen")?
            .parse::<SocketAddr>()
            .map_err(|why| format!("listen isn't an address: {why}"))?;
        if !listen.ip().is_loopback() {
            return Err(format!(
                "listen is {listen}, which other machines reach; it serves loopback only"
            ));
        }
        let names = words(said.get("names")).ok_or("names isn't a list of names")?;
        if names.is_empty() {
            return Err("names is empty, so no browser could reach it".to_owned());
        }
        let documents = PathBuf::from(text_of(&said, "documents")?);
        let mounts = said
            .get("mounts")
            .and_then(Value::as_object)
            .map(|mounts| {
                mounts
                    .iter()
                    .map(|(name, path)| {
                        let path = path
                            .as_str()
                            .ok_or_else(|| format!("mount {name} isn't a path"))?;
                        Ok((name.clone(), PathBuf::from(path)))
                    })
                    .collect::<Result<_, String>>()
            })
            .transpose()?
            .unwrap_or_default();
        let feeds = said
            .get("feeds")
            .and_then(Value::as_object)
            .map(|feeds| {
                feeds
                    .iter()
                    .map(|(name, feed)| Ok((name.clone(), feed_of(name, feed, runtime)?)))
                    .collect::<Result<_, String>>()
            })
            .transpose()?
            .unwrap_or_default();
        Ok(Self {
            listen,
            names,
            documents,
            mounts,
            feeds,
        })
    }
}

fn feed_of(name: &str, said: &Value, runtime: Option<&Path>) -> Result<Feed, String> {
    let socket = said
        .get("socket")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("feed {name} names no socket"))?;
    let socket = if Path::new(socket).is_absolute() {
        PathBuf::from(socket)
    } else {
        runtime
            .ok_or_else(|| {
                format!("feed {name}'s socket is under the runtime folder, and none is set")
            })?
            .join(socket)
    };
    let kinds = said
        .get("kinds")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("feed {name} names no kinds"))?
        .iter()
        .map(|(kind, what)| {
            // Nix writes an option left unset as null, which says as much
            // as its absence.
            let said = |name| what.get(name).filter(|value| !value.is_null());
            let fields = said("fields").map(|fields| {
                words(Some(fields))
                    .ok_or_else(|| format!("feed {name}'s {kind} fields aren't names"))
            });
            let deep = said("deep").map(|deep| {
                words(Some(deep)).ok_or_else(|| format!("feed {name}'s {kind} deep isn't names"))
            });
            let latest = said("latest").map(|latest| {
                latest
                    .as_u64()
                    .filter(|count| *count > 0)
                    .ok_or_else(|| format!("feed {name}'s {kind} latest isn't a count"))
            });
            Ok((
                kind.clone(),
                Kind {
                    fields: fields.transpose()?,
                    deep: deep.transpose()?,
                    latest: latest.transpose()?,
                },
            ))
        })
        .collect::<Result<_, String>>()?;
    Ok(Feed { socket, kinds })
}

fn text_of<'a>(said: &'a Value, field: &str) -> Result<&'a str, String> {
    said.get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{field} is missing"))
}

fn words(said: Option<&Value>) -> Option<Vec<String>> {
    said?
        .as_array()?
        .iter()
        .map(|word| word.as_str().map(str::to_owned))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHOLE: &str = r#"{
        "listen": "127.0.0.1:8484",
        "names": ["sites.localhost", "localhost"],
        "documents": "/home/joel/sites",
        "mounts": {"kit": "/nix/store/kit"},
        "feeds": {"node": {"socket": "mycelium/work.sock", "kinds": {
            "machine": {"fields": null, "deep": null},
            "session": {"fields": ["title", "presence"], "deep": ["presence"]},
            "message": {"fields": ["at"], "latest": 20}
        }}}
    }"#;

    #[test]
    fn a_whole_config_is_read_with_its_socket_under_the_runtime_folder() {
        let config = Config::from_json(WHOLE, Some(Path::new("/run/user/1000"))).expect("it reads");
        assert_eq!(config.listen.port(), 8484);
        assert_eq!(config.mounts["kit"], PathBuf::from("/nix/store/kit"));
        let node = &config.feeds["node"];
        assert_eq!(
            node.socket,
            PathBuf::from("/run/user/1000/mycelium/work.sock")
        );
        assert_eq!(
            node.kinds["machine"],
            Kind::default(),
            "null says what absence does"
        );
        assert_eq!(
            node.kinds["session"].fields.as_deref(),
            Some(&["title".to_owned(), "presence".to_owned()][..])
        );
    }

    #[test]
    fn a_kind_asks_only_its_latest_records_where_it_names_how_many() {
        let config = Config::from_json(WHOLE, Some(Path::new("/run"))).expect("it reads");
        let kinds = &config.feeds["node"].kinds;
        assert_eq!(
            kinds["message"].latest,
            Some(20),
            "the message kind names 20"
        );
        assert_eq!(
            kinds["session"].latest, None,
            "a kind naming none asks for every one"
        );
        for wrong in ["0", "-1", "2.5", "\"20\""] {
            let said = WHOLE.replace(r#""latest": 20"#, &format!(r#""latest": {wrong}"#));
            let why = Config::from_json(&said, Some(Path::new("/run"))).expect_err("refused");
            assert!(why.contains("latest isn't a count"), "{wrong} gave {why}");
        }
    }

    #[test]
    fn an_address_other_machines_reach_is_refused() {
        let open = WHOLE.replace("127.0.0.1:8484", "0.0.0.0:8484");
        let why = Config::from_json(&open, Some(Path::new("/run"))).expect_err("refused");
        assert!(why.contains("loopback"), "{why}");
    }

    #[test]
    fn a_runtime_socket_without_a_runtime_folder_is_refused() {
        let why = Config::from_json(WHOLE, None).expect_err("refused");
        assert!(why.contains("runtime folder"), "{why}");
    }

    #[test]
    fn a_feed_naming_no_kinds_is_refused() {
        let none = WHOLE.replace(r#""kinds": {"#, r#""kinded": {"#);
        let why = Config::from_json(&none, Some(Path::new("/run"))).expect_err("refused");
        assert!(why.contains("names no kinds"), "{why}");
    }
}
