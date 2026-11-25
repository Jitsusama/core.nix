//! What the server is told by its module: where it listens, the names a
//! browser reaches it by, the name it serves the documents under, the folder of documents sessions write, the
//! folder the home's builder writes the home into, the
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
    /// The name the documents are served under, such as
    /// `documents.sites.localhost`: an origin of their own, so what a
    /// session's page runs is never one of the sites' own (http.rs). None
    /// of `names`, which would make the two one origin.
    pub documents_name: String,
    /// The folder of documents sessions write, served below every mount.
    pub documents: PathBuf,
    /// The folder the home's builder writes the home's own files into,
    /// absolute or under the user's runtime folder, each served at
    /// `/<name>` ahead of the documents, and its page at `/`. Sessions
    /// write none of it (a fenced one doesn't even see the runtime folder),
    /// so its pages are served as a mount's are rather than as a document
    /// (files.rs); none where the home is written among the documents.
    pub home: Option<PathBuf>,
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
}

impl Config {
    /// Reads the config from its JSON, a relative socket or home taken as
    /// under `runtime`, the user's runtime folder.
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
        let documents_name = text_of(&said, "documentsName")?.to_ascii_lowercase();
        if documents_name.is_empty() {
            return Err("documentsName is empty, so no browser could reach a document".to_owned());
        }
        if names
            .iter()
            .any(|name| name.eq_ignore_ascii_case(&documents_name))
        {
            return Err(format!(
                "documentsName {documents_name} is one of names, which would serve a session's \
                 page as one of the sites' own"
            ));
        }
        let documents = PathBuf::from(text_of(&said, "documents")?);
        let home = match said.get("home") {
            None | Some(Value::Null) => None,
            Some(home) => {
                let home = home.as_str().ok_or("home isn't a path")?;
                Some(
                    under(home, runtime)
                        .ok_or("home is under the runtime folder, and none is set")?,
                )
            }
        };
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
            documents_name,
            documents,
            home,
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
    let socket = under(socket, runtime).ok_or_else(|| {
        format!("feed {name}'s socket is under the runtime folder, and none is set")
    })?;
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
            Ok((
                kind.clone(),
                Kind {
                    fields: fields.transpose()?,
                    deep: deep.transpose()?,
                },
            ))
        })
        .collect::<Result<_, String>>()?;
    Ok(Feed { socket, kinds })
}

/// `path` as it stands where it's absolute, else under `runtime`, or none
/// where it's relative and no runtime folder is set.
fn under(path: &str, runtime: Option<&Path>) -> Option<PathBuf> {
    if Path::new(path).is_absolute() {
        Some(PathBuf::from(path))
    } else {
        runtime.map(|runtime| runtime.join(path))
    }
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
        "documentsName": "Documents.Sites.Localhost",
        "documents": "/home/joel/sites",
        "mounts": {"kit": "/nix/store/kit"},
        "feeds": {"node": {"socket": "mycelium/work.sock", "kinds": {
            "machine": {"fields": null, "deep": null},
            "session": {"fields": ["title", "presence"], "deep": ["presence"]}
        }}}
    }"#;

    #[test]
    fn a_whole_config_is_read_with_its_socket_under_the_runtime_folder() {
        let config = Config::from_json(WHOLE, Some(Path::new("/run/user/1000"))).expect("it reads");
        assert_eq!(config.listen.port(), 8484);
        assert_eq!(config.mounts["kit"], PathBuf::from("/nix/store/kit"));
        assert_eq!(config.home, None, "a home among the documents names none");
        assert_eq!(
            config.documents_name, "documents.sites.localhost",
            "a name's case is the browser's"
        );
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
    fn a_home_of_its_own_is_read_and_a_home_that_isnt_a_path_refused() {
        let homed = WHOLE.replace(
            r#""mounts""#,
            r#""home": "/home/joel/.local/state/local-sites/home", "mounts""#,
        );
        let config = Config::from_json(&homed, Some(Path::new("/run"))).expect("it reads");
        assert_eq!(
            config.home,
            Some(PathBuf::from("/home/joel/.local/state/local-sites/home"))
        );
        let unpathed = WHOLE.replace(r#""mounts""#, r#""home": 7, "mounts""#);
        let why = Config::from_json(&unpathed, Some(Path::new("/run"))).expect_err("refused");
        assert!(why.contains("home isn't a path"), "{why}");
        let running = WHOLE.replace(r#""mounts""#, r#""home": "local-sites/home", "mounts""#);
        let config =
            Config::from_json(&running, Some(Path::new("/run/user/1000"))).expect("it reads");
        assert_eq!(
            config.home,
            Some(PathBuf::from("/run/user/1000/local-sites/home")),
            "a relative home is under the runtime folder, as a socket is"
        );
        let absolute = WHOLE.replace("mycelium/work.sock", "/run/work.sock");
        let unrun = absolute.replace(r#""mounts""#, r#""home": "local-sites/home", "mounts""#);
        let why = Config::from_json(&unrun, None).expect_err("refused");
        assert!(
            why.contains("home is under the runtime folder, and none is set"),
            "{why}"
        );
    }

    #[test]
    fn documents_served_as_one_of_the_sites_own_names_or_none_are_refused() {
        let shared = WHOLE.replace("Documents.Sites.Localhost", "LOCALHOST");
        let why = Config::from_json(&shared, Some(Path::new("/run"))).expect_err("refused");
        assert!(why.contains("one of names"), "{why}");
        let empty = WHOLE.replace("Documents.Sites.Localhost", "");
        let why = Config::from_json(&empty, Some(Path::new("/run"))).expect_err("refused");
        assert!(why.contains("documentsName is empty"), "{why}");
        let unnamed = WHOLE.replace(r#""documentsName": "Documents.Sites.Localhost","#, "");
        let why = Config::from_json(&unnamed, Some(Path::new("/run"))).expect_err("refused");
        assert!(why.contains("documentsName is missing"), "{why}");
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
