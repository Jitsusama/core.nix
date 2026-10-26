//! What an update is told, from the JSON file its module writes.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::Value;

/// How long a release `current` has left is kept, at least.
const KEEP_DAYS: u64 = 2;

/// One package pi loads, and where it comes from.
#[derive(Debug, Clone)]
pub struct Package {
    /// Where its git repository is, as git clones it.
    pub url: String,
    /// The branch whose head is installed.
    pub branch: String,
    /// Its folder under the root: the url's host and path, as pi names a
    /// git package's folder.
    pub key: PathBuf,
}

impl Package {
    /// A package from its url, on `branch`.
    ///
    /// # Errors
    ///
    /// Says so when the url names no host and path to keep it under.
    pub fn new(url: &str, branch: &str) -> Result<Self, String> {
        let bare = url
            .split_once("://")
            .map_or(url, |(_, rest)| rest)
            .trim_end_matches('/')
            .trim_end_matches(".git");
        let key: PathBuf = bare
            .split('/')
            .filter(|part| !part.is_empty() && *part != "." && *part != "..")
            .collect();
        if key.components().count() < 2 {
            return Err(format!("{url} names no host and path to keep it under"));
        }
        Ok(Self {
            url: url.to_owned(),
            branch: branch.to_owned(),
            key,
        })
    }
}

/// One update's settings.
#[derive(Debug, Clone)]
pub struct Config {
    /// Where every package's mirror, releases and `current` live.
    pub root: PathBuf,
    pub packages: Vec<Package>,
    /// The programs a build runs, by absolute path.
    pub git: PathBuf,
    pub node: PathBuf,
    pub pnpm: PathBuf,
    /// The folders a build's PATH holds, which its install scripts run
    /// with: node, pnpm and git's own, and whatever the module adds.
    pub path: Vec<PathBuf>,
    /// Variables a build is given, over those it keeps from where it
    /// was started.
    pub env: Vec<(String, String)>,
    /// How long a release `current` has left is kept, at least.
    pub keep: Duration,
}

impl Config {
    /// Read the settings the module wrote.
    ///
    /// # Errors
    ///
    /// Says which file could not be read, or which field is missing.
    pub fn read(file: &Path) -> Result<Self, String> {
        let text =
            fs::read_to_string(file).map_err(|error| format!("{}: {error}", file.display()))?;
        let value: Value =
            serde_json::from_str(&text).map_err(|error| format!("{}: {error}", file.display()))?;
        Self::from_json(&value).map_err(|error| format!("{}: {error}", file.display()))
    }

    /// The settings a JSON value holds.
    ///
    /// # Errors
    ///
    /// Names the field that is missing or of the wrong kind.
    pub fn from_json(value: &Value) -> Result<Self, String> {
        let text = |name: &str| {
            value[name]
                .as_str()
                .map(PathBuf::from)
                .ok_or_else(|| format!("{name} is missing"))
        };
        let packages = value["packages"]
            .as_array()
            .ok_or("packages is missing")?
            .iter()
            .map(|package| {
                let url = package["url"].as_str().ok_or("a package has no url")?;
                Package::new(url, package["branch"].as_str().unwrap_or("main"))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let path = value["path"]
            .as_array()
            .map(|dirs| {
                dirs.iter()
                    .filter_map(Value::as_str)
                    .map(PathBuf::from)
                    .collect()
            })
            .unwrap_or_default();
        let env = value["env"]
            .as_object()
            .map(|vars| {
                vars.iter()
                    .filter_map(|(key, value)| {
                        value.as_str().map(|value| (key.clone(), value.to_owned()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let days = value["keepDays"].as_u64().unwrap_or(KEEP_DAYS);
        Ok(Self {
            root: text("root")?,
            packages,
            git: text("git")?,
            node: text("node")?,
            pnpm: text("pnpm")?,
            path,
            env,
            keep: Duration::from_secs(days * 86_400),
        })
    }
}
