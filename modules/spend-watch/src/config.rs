//! The configuration home.nix renders from `jitsusama.spend-watch`, read
//! once at start. Every path is given whole, so nothing here guesses where
//! the harness keeps its files.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    /// pi's session logs, a directory of directories of `.jsonl` files.
    pub sessions: PathBuf,
    /// The logs pi keeps for agents nobody types into, wherever the tools
    /// that run them put them.
    #[serde(default)]
    pub agents: Vec<AgentLogs>,
    /// Where the harness gives each quest a workspace, a directory named
    /// by its id: a session working in one serves that quest.
    #[serde(default)]
    pub workspaces: Option<PathBuf>,
    /// The harness's record of subagent runs, read only.
    pub runs: PathBuf,
    /// The harness's session registry, a file per open session.
    pub registry: PathBuf,
    /// The harness's quests, a directory per quest.
    pub quests: PathBuf,
    /// Where the history and the reports are kept.
    pub state: PathBuf,
    /// Where the status files and the socket go; by default the session's
    /// runtime directory's `spend-watch`, which a config can't know ahead.
    #[serde(default)]
    runtime: Option<PathBuf>,
    /// The zone days and hours are local to, as IANA names it; by default
    /// the machine's own.
    #[serde(default)]
    zone: Option<String>,
    #[serde(default)]
    pub feeds: Vec<FeedConfig>,
    #[serde(default)]
    pub notices: NoticeConfig,
    #[serde(default)]
    pub detectors: DetectorConfig,
    /// Models whose name matches any of these count as premium.
    #[serde(default)]
    pub premium: Vec<String>,
    /// How many days before an override ends to warn, earliest first.
    #[serde(default = "default_lead")]
    pub override_lead: Vec<u32>,
    /// A cap for a machine no feed gives one to.
    #[serde(default)]
    pub cap: Option<CapConfig>,
    /// Whether a report is written each Monday morning.
    #[serde(default = "yes")]
    pub weekly_report: bool,
    /// What a provider's error says when it refuses for being over a cap,
    /// matched without case against the errors in pi's logs.
    #[serde(default = "default_refusals")]
    pub refusals: Vec<String>,
}

fn default_lead() -> Vec<u32> {
    vec![3, 1]
}

fn default_refusals() -> Vec<String> {
    [
        "over the allowed cap",
        "credit balance is too low",
        "usage limit",
    ]
    .map(str::to_owned)
    .to_vec()
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FeedConfig {
    pub name: String,
    pub label: String,
    pub command: Vec<String>,
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
}

/// One place agents' logs are kept: a pattern of their paths, each `*`
/// standing for any one name, and the kind of work their sessions are.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentLogs {
    pub pattern: String,
    pub kind: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct NoticeConfig {
    pub enable: bool,
    /// Notices a week outside cap events.
    pub per_week: u32,
    /// One quiet line at the day's first unlock.
    pub daily: bool,
}

impl Default for NoticeConfig {
    fn default() -> Self {
        NoticeConfig {
            enable: true,
            per_week: 3,
            daily: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct DetectorConfig {
    /// Days between false alarms the burn's threshold is set for.
    pub burn_run_length_days: f64,
    /// The false discovery rate held across every cut tested at once.
    pub false_discovery: f64,
    /// The least excess, in dollars, a finding is made for.
    pub floor: f64,
    /// Detectors whose findings are only ever notes, by name.
    pub notes_only: Vec<String>,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        DetectorConfig {
            burn_run_length_days: 30.0,
            false_discovery: 0.05,
            floor: 25.0,
            notes_only: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapConfig {
    pub allowance: f64,
    pub window_hours: u32,
    pub warn: f64,
}

impl Config {
    pub fn read(path: &Path) -> Result<Config, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("reading {}: {error}", path.display()))?;
        let config: Config = serde_json::from_str(&text)
            .map_err(|error| format!("reading {}: {error}", path.display()))?;
        config.check()?;
        Ok(config)
    }

    /// What a config can't mean, refused before the daemon starts.
    pub fn check(&self) -> Result<(), String> {
        self.zone()?;
        if self.feeds.iter().any(|feed| feed.command.is_empty()) {
            return Err("a feed has no command".into());
        }
        let mut names: Vec<&str> = self.feeds.iter().map(|feed| feed.name.as_str()).collect();
        names.sort_unstable();
        if names.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err("two feeds share a name".into());
        }
        for agents in &self.agents {
            if !agents.pattern.starts_with('/') || agents.pattern.contains("**") {
                return Err(format!(
                    "agents' logs are found by a whole path, each `*` one name: {}",
                    agents.pattern
                ));
            }
            let word = |c: char| c.is_ascii_alphanumeric() || c == '-';
            if agents.kind.is_empty() || !agents.kind.chars().all(word) {
                return Err(format!("an agent's kind is one word: {:?}", agents.kind));
            }
        }
        if !(0.0..1.0).contains(&self.detectors.false_discovery) {
            return Err("the false discovery rate is a fraction below one".into());
        }
        if self.detectors.burn_run_length_days <= 0.0 {
            return Err("the burn's run length is a positive number of days".into());
        }
        Ok(())
    }

    pub fn history(&self) -> PathBuf {
        self.state.join("history.db")
    }

    pub fn reports(&self) -> PathBuf {
        self.state.join("reports")
    }

    pub fn socket(&self) -> PathBuf {
        self.runtime().unwrap_or_default().join("socket")
    }

    pub fn runtime(&self) -> Result<PathBuf, String> {
        match &self.runtime {
            Some(path) => Ok(path.clone()),
            None => std::env::var_os("XDG_RUNTIME_DIR")
                .map(|dir| PathBuf::from(dir).join("spend-watch"))
                .ok_or_else(|| {
                    "no runtime directory: XDG_RUNTIME_DIR isn't set and none is configured".into()
                }),
        }
    }

    pub fn zone(&self) -> Result<crate::time::Zone, String> {
        match &self.zone {
            Some(name) => crate::time::Zone::named(name),
            None => Ok(crate::time::Zone::system()),
        }
    }

    pub fn is_premium(&self, model: &str) -> bool {
        self.premium
            .iter()
            .any(|pattern| model.contains(pattern.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_agents(pattern: &str, kind: &str) -> Result<(), String> {
        let config = serde_json::json!({
            "sessions": "/s", "runs": "/r", "registry": "/g", "quests": "/q",
            "state": "/t", "agents": [{ "pattern": pattern, "kind": kind }],
        });
        serde_json::from_value::<Config>(config)
            .map_err(|error| error.to_string())?
            .check()
    }

    // A pattern is a whole path whose each star is one name, so finding
    // agents' logs never walks a tree, and a kind is one word.
    #[test]
    fn agents_logs_are_named_by_a_whole_path() {
        assert!(with_agents("/w/*/agents/*/session/*.jsonl", "swarm").is_ok());
        assert!(with_agents("w/*/session/*.jsonl", "swarm").is_err());
        assert!(with_agents("/w/**/*.jsonl", "swarm").is_err());
        assert!(with_agents("/w/*.jsonl", "").is_err());
        assert!(with_agents("/w/*.jsonl", "two words").is_err());
    }
}
