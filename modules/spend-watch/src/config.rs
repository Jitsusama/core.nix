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
    /// The default budget's allowance in dollars, for a machine no feed gives
    /// that budget a cap.
    #[serde(default)]
    pub cap: Option<CapConfig>,
    /// Where pi's requests draw their money from, as declared; none means
    /// one budget, the implicit default.
    #[serde(default)]
    budgets: Vec<BudgetConfig>,
    /// What each of pi's providers is called as it names itself, by its
    /// id: "Claude Code" for `claude-code`. One not named is said by its
    /// id.
    #[serde(default)]
    pub provider_names: BTreeMap<String, String>,
    /// Whether a report is written each Monday morning.
    #[serde(default = "yes")]
    pub weekly_report: bool,
    /// What a provider's error says when it refuses for being over a cap,
    /// matched without case against the errors in pi's logs.
    #[serde(default = "default_refusals")]
    pub refusals: Vec<String>,
    /// The launcher's vocabulary, its words.json, which words what the
    /// rows served ready say beyond the collection's own words: their
    /// drawers' fact names and their acts (`ready.rs`). With none, each
    /// says its id, as the launcher says a word it doesn't hold.
    #[serde(default)]
    pub vocabulary: Option<PathBuf>,
}

fn default_lead() -> Vec<u32> {
    vec![3, 1]
}

fn default_refusals() -> Vec<String> {
    [
        "over the allowed cap",
        "over your weekly cap",
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
    /// The budget its bill and caps are for, unless a record says another;
    /// the default budget when none is named.
    #[serde(default)]
    pub budget: Option<String>,
}

/// A named source of money a request draws on: a Claude plan, an API
/// key, a proxy's cap, a team's token.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BudgetConfig {
    pub name: String,
    pub label: String,
    pub kind: BudgetKind,
    pub whose: Whose,
    #[serde(default)]
    pub default: bool,
    /// pi's providers whose turns are always this budget's, whatever the
    /// session's log says, since the provider is what's paid.
    #[serde(default)]
    pub providers: Vec<String>,
    /// What a plan costs, which is billed spend whether or not
    /// it's used.
    #[serde(default)]
    pub price: Option<Price>,
    #[serde(default)]
    pub allowances: Vec<AllowanceConfig>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BudgetKind {
    Plan,
    Key,
    Cap,
    Team,
}

impl BudgetKind {
    pub fn name(self) -> &'static str {
        match self {
            BudgetKind::Plan => "plan",
            BudgetKind::Key => "key",
            BudgetKind::Cap => "cap",
            BudgetKind::Team => "team",
        }
    }
}

/// Whose money a budget is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Whose {
    Own,
    Employer,
    Team,
}

impl Whose {
    pub fn name(self) -> &'static str {
        match self {
            Whose::Own => "own",
            Whose::Employer => "employer",
            Whose::Team => "team",
        }
    }
}

/// A plan's price: so many dollars a month.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Price {
    pub dollars: f64,
    pub per: Per,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Per {
    Month,
}

/// What a budget may spend in a period: dollars, or a share of a
/// plan's window.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AllowanceConfig {
    pub name: String,
    #[serde(default)]
    pub unit: crate::caps::Unit,
    /// Dollars allowed in the window, for an allowance in dollars.
    #[serde(default)]
    pub allowance: Option<f64>,
    pub window_hours: u32,
    /// When it next resets, for one counted over periods rather than a
    /// rolling window.
    #[serde(default)]
    pub resets: Option<String>,
    pub warn: f64,
    /// The machine's own cap, which stands in only while no feed gives
    /// its budget one, as it always has.
    #[serde(skip)]
    pub stands_in: bool,
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

/// The budget a machine that declares none has, so it behaves as it
/// always did.
pub const IMPLICIT: &str = "default";

impl Config {
    /// Every budget, the default among them: those declared, or the one
    /// a machine that declares none has. The machine's `cap` is the
    /// default budget's allowance in dollars.
    pub fn budgets(&self) -> Vec<BudgetConfig> {
        let mut budgets = if self.budgets.is_empty() {
            vec![BudgetConfig {
                name: IMPLICIT.into(),
                label: "pi".into(),
                kind: BudgetKind::Key,
                whose: Whose::Own,
                default: true,
                providers: Vec::new(),
                price: None,
                allowances: Vec::new(),
            }]
        } else {
            self.budgets.clone()
        };
        if let Some(cap) = &self.cap
            && let Some(default) = budgets.iter_mut().find(|budget| budget.default)
        {
            default.allowances.push(AllowanceConfig {
                name: "own".into(),
                unit: crate::caps::Unit::Dollars,
                allowance: Some(cap.allowance),
                window_hours: cap.window_hours,
                resets: None,
                warn: cap.warn,
                stands_in: true,
            });
        }
        budgets
    }

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
        self.check_budgets()
    }

    /// What budgets can't mean: none or two to fall back on, a provider
    /// two budgets claim, a feed's bill for a budget nobody declared, or a
    /// allowance that can't be counted.
    fn check_budgets(&self) -> Result<(), String> {
        let budgets = self.budgets();
        let defaults = budgets.iter().filter(|budget| budget.default).count();
        if defaults != 1 {
            return Err(format!("exactly one budget is the default, not {defaults}"));
        }
        let mut names: Vec<&str> = budgets.iter().map(|budget| budget.name.as_str()).collect();
        names.sort_unstable();
        if names.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err("two budgets share a name".into());
        }
        let mut claimed: BTreeMap<&str, &str> = BTreeMap::new();
        for budget in &budgets {
            for provider in &budget.providers {
                if let Some(other) = claimed.insert(provider, &budget.name) {
                    return Err(format!(
                        "the provider {provider} is claimed by both {other} and {}",
                        budget.name
                    ));
                }
            }
            if budget.price.is_some() && budget.kind != BudgetKind::Plan {
                return Err(format!("{} has a price, but only a plan does", budget.name));
            }
            if budget.price.is_some_and(|price| price.dollars < 0.0) {
                return Err(format!(
                    "{}'s price is a figure of none or more",
                    budget.name
                ));
            }
            for allowance in &budget.allowances {
                allowance_holds(&budget.name, allowance)?;
            }
        }
        for feed in &self.feeds {
            if let Some(budget) = &feed.budget
                && !names.contains(&budget.as_str())
            {
                return Err(format!(
                    "the feed {} is for the budget {budget}, which isn't declared",
                    feed.name
                ));
            }
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

/// Whether a budget's allowance can be counted: so many dollars,
/// or a share, each over a window of hours and warned at a fraction.
fn allowance_holds(budget: &str, allowance: &AllowanceConfig) -> Result<(), String> {
    let named = format!("{budget}'s allowance {}", allowance.name);
    match (allowance.unit, allowance.allowance) {
        (crate::caps::Unit::Dollars, None) => {
            return Err(format!("{named} is in dollars, so it says how many"));
        }
        (crate::caps::Unit::Dollars, Some(allowance)) if allowance <= 0.0 => {
            return Err(format!("{named} allows more than nothing"));
        }
        (crate::caps::Unit::Share, Some(_)) => {
            return Err(format!("{named} is a share, so it says no dollars"));
        }
        _ => {}
    }
    if allowance.window_hours == 0 || !(0.0..=1.0).contains(&allowance.warn) {
        return Err(format!(
            "{named}'s window is hours and its warning a fraction"
        ));
    }
    if let Some(resets) = &allowance.resets
        && crate::time::parse(resets).is_none()
    {
        return Err(format!(
            "{named} resets at {resets}, which isn't an instant"
        ));
    }
    Ok(())
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

    fn with(budgets: serde_json::Value, feeds: serde_json::Value) -> Result<Config, String> {
        let mut config = serde_json::json!({
            "sessions": "/s", "runs": "/r", "registry": "/g", "quests": "/q", "state": "/t",
        });
        config["budgets"] = budgets;
        config["feeds"] = feeds;
        let config: Config = serde_json::from_value(config).map_err(|error| error.to_string())?;
        config.check().map(|()| config)
    }

    fn budget(name: &str, default: bool, providers: &[&str]) -> serde_json::Value {
        serde_json::json!({
            "name": name, "label": name, "kind": "key", "whose": "own",
            "default": default, "providers": providers,
        })
    }

    // A machine that declares no budget has one, the default, so it
    // behaves as it did; its cap is that budget's allowance in dollars.
    #[test]
    fn no_budget_declared_is_one_default() {
        let mut config = with(serde_json::json!([]), serde_json::json!([])).unwrap();
        config.cap = Some(CapConfig {
            allowance: 4000.0,
            window_hours: 168,
            warn: 0.9,
        });
        let budgets = config.budgets();
        assert_eq!(budgets.len(), 1);
        assert_eq!(
            (budgets[0].name.as_str(), budgets[0].label.as_str()),
            (IMPLICIT, "pi")
        );
        assert!(budgets[0].default);
        let own = &budgets[0].allowances[0];
        assert_eq!(
            (own.allowance, own.window_hours, own.stands_in),
            (Some(4000.0), 168, true)
        );
    }

    #[test]
    fn budgets_are_declared_with_their_allowances_and_feeds() {
        let claude = serde_json::json!({
            "name": "claude", "label": "Claude Pro", "kind": "plan",
            "whose": "own", "providers": ["claude-code"],
            "price": {"dollars": 20, "per": "month"},
            "allowances": [{"name": "week", "unit": "share", "windowHours": 168, "warn": 0.8}],
        });
        let proxy = serde_json::json!({
            "name": "proxy", "label": "Your proxy cap", "kind": "cap",
            "whose": "employer", "default": true,
            "allowances": [{"name": "weekly", "allowance": 40000, "windowHours": 168,
                        "warn": 0.9}],
        });
        let feed = serde_json::json!([
            {"name": "proxy", "label": "the AI proxy", "command": ["f"], "budget": "proxy"},
        ]);
        let config = with(serde_json::json!([claude, proxy]), feed).unwrap();
        let budgets = config.budgets();
        assert_eq!(budgets[0].allowances[0].unit, crate::caps::Unit::Share);
        assert_eq!(budgets[1].allowances[0].unit, crate::caps::Unit::Dollars);
        assert_eq!(config.feeds[0].budget.as_deref(), Some("proxy"));
    }

    #[test]
    fn exactly_one_budget_is_the_default() {
        let none = serde_json::json!([budget("a", false, &[]), budget("b", false, &[])]);
        assert!(
            with(none, serde_json::json!([]))
                .unwrap_err()
                .contains("exactly one")
        );
        let two = serde_json::json!([budget("a", true, &[]), budget("b", true, &[])]);
        assert!(
            with(two, serde_json::json!([]))
                .unwrap_err()
                .contains("exactly one")
        );
    }

    #[test]
    fn a_provider_is_claimed_by_one_budget_at_most() {
        let budgets = serde_json::json!([
            budget("a", true, &["claude-code"]),
            budget("b", false, &["claude-code"]),
        ]);
        let refused = with(budgets, serde_json::json!([])).unwrap_err();
        assert!(refused.contains("claude-code"), "{refused}");
    }

    #[test]
    fn a_feed_names_a_declared_budget() {
        let feed = serde_json::json!([
            {"name": "proxy", "label": "p", "command": ["f"], "budget": "nowhere"},
        ]);
        let refused = with(serde_json::json!([budget("a", true, &[])]), feed.clone());
        assert!(refused.unwrap_err().contains("nowhere"));
        // With none declared, there's only the implicit one to name.
        assert!(with(serde_json::json!([]), feed).is_err());
    }

    #[test]
    fn an_allowance_in_dollars_says_how_many() {
        let declared = |allowance: serde_json::Value| {
            let mut budget = budget("a", true, &[]);
            budget["allowances"] = serde_json::json!([allowance]);
            with(serde_json::json!([budget]), serde_json::json!([]))
        };
        let refused = declared(serde_json::json!({"name": "w", "windowHours": 168, "warn": 0.9}));
        assert!(refused.unwrap_err().contains("says how many"));
        let share = serde_json::json!({
            "name": "w", "unit": "share", "allowance": 5, "windowHours": 5, "warn": 0.9,
        });
        assert!(declared(share).is_err());
        let held = serde_json::json!({
            "name": "w", "allowance": 400, "windowHours": 168, "warn": 0.9,
            "resets": "2026-11-09T05:00:00Z",
        });
        assert!(declared(held).is_ok());
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
