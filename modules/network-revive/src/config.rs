//! The units to look after, as the home module writes them.

use std::fs;
use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub units: Vec<Unit>,
    /// Whether it says what it found each time it looked, as a test needs
    /// to know when it has.
    #[serde(default)]
    pub debug: bool,
}

/// A unit the network can let down.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unit {
    pub name: String,
    /// The arguments that make the unit's own program say whether it could
    /// start, added to its ExecStart, as nginx's -t; with none, it's started
    /// unchecked.
    #[serde(default)]
    pub check: Option<Vec<String>>,
    /// Whether it's reloaded when the network changes while it runs, for a
    /// program that looks its names up only as it reads its configuration.
    #[serde(default)]
    pub reload: bool,
}

impl Config {
    pub fn read(path: &Path) -> Result<Config, String> {
        let text =
            fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
        Config::parsed(&text).map_err(|error| format!("{}: {error}", path.display()))
    }

    fn parsed(text: &str) -> serde_json::Result<Config> {
        serde_json::from_str(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_unit_says_its_check_and_whether_it_reloads() {
        let text = r#"{"units": [{"name": "proxy.service", "check": ["-t"], "reload": true}]}"#;

        let config = Config::parsed(text).unwrap();

        let unit = Unit {
            name: "proxy.service".into(),
            check: Some(vec!["-t".into()]),
            reload: true,
        };
        assert_eq!(
            config,
            Config {
                units: vec![unit],
                debug: false,
            }
        );
    }

    #[test]
    fn it_can_be_asked_to_say_what_it_finds() {
        let text = r#"{"units": [], "debug": true}"#;

        assert_eq!(
            Config::parsed(text).ok().map(|config| config.debug),
            Some(true)
        );
    }

    #[test]
    fn a_unit_naming_only_itself_is_started_unchecked_and_never_reloaded() {
        let config = Config::parsed(r#"{"units": [{"name": "backup.service"}]}"#).unwrap();

        let unit = Unit {
            name: "backup.service".into(),
            check: None,
            reload: false,
        };
        assert_eq!(config.units, [unit]);
    }

    #[test]
    fn a_setting_it_doesnt_know_is_refused() {
        let text = r#"{"units": [{"name": "proxy.service", "reloads": true}]}"#;

        assert!(Config::parsed(text).is_err());
    }
}
