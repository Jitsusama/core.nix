//! The configuration home.nix renders from `jitsusama.spend-watch`, read
//! once at start. Every path is given whole, so nothing here guesses where
//! the harness keeps its files.
//!
//! Long because every setting is read and refused here, in one place both
//! the build and the daemon check, and a third of it is the tests pinning
//! each refusal.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The host name, read once: a machine renamed while the daemon runs
/// would otherwise show its work under two names, and the name checked
/// against other machines' at load would no longer be the one shown.
fn host_name() -> &'static str {
    static NAME: OnceLock<String> = OnceLock::new();
    NAME.get_or_init(|| {
        let name = rustix::system::uname()
            .nodename()
            .to_string_lossy()
            .trim()
            .to_owned();
        if name.is_empty() {
            "this machine".into()
        } else {
            name
        }
    })
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    /// pi's session logs, a directory of directories of `.jsonl` files.
    pub sessions: PathBuf,
    /// The logs pi keeps for agents nobody types into, wherever the tools
    /// that run them put them.
    #[serde(default)]
    pub agents: Vec<AgentLogs>,
    /// What this machine is called, which its own logs' work is shown
    /// under beside other machines'; by default its host name.
    #[serde(default)]
    machine: Option<String>,
    /// Other machines whose logs are brought here, each read as this
    /// machine's are and its work shown under its name, rather than as a
    /// bill nobody here explains. Only its logs: its runs and workspaces
    /// stay there.
    #[serde(default)]
    pub machines: Vec<MachineLogs>,
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

/// Another machine's logs, where they're brought to here: its own
/// sessions, laid out as pi lays out this machine's, and its agents'.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MachineLogs {
    pub name: String,
    pub sessions: PathBuf,
    #[serde(default)]
    pub agents: Vec<AgentLogs>,
}

/// What a machine's work is shown under when no log says which machine
/// it was, as a bill no log here explains.
pub const ELSEWHERE: &str = "elsewhere";

/// How another tool's spend is named in the machine cut, after the tool,
/// since a bill names no machine.
pub const NO_MACHINE: &str = " (no machine)";

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
        let agents = self
            .agents
            .iter()
            .chain(self.machines.iter().flat_map(|machine| &machine.agents));
        for agents in agents {
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
        self.check_machines()?;
        if !(0.0..1.0).contains(&self.detectors.false_discovery) {
            return Err("the false discovery rate is a fraction below one".into());
        }
        if self.detectors.burn_run_length_days <= 0.0 {
            return Err("the burn's run length is a positive number of days".into());
        }
        self.check_budgets()
    }

    /// What machines can't mean: two of one name, one named as this
    /// machine, as elsewhere or as another tool's row, whose work it would
    /// be mistaken for, or logs found by anything but a whole path.
    fn check_machines(&self) -> Result<(), String> {
        let here = self.machine();
        // Shown as written, so "teal " here would stand beside a teal.
        if here.trim() != here {
            return Err(format!(
                "this machine's name has no spaces around it: {here:?}"
            ));
        }
        if here == ELSEWHERE || here.ends_with(NO_MACHINE) {
            return Err(format!(
                "this machine's name is its own, not elsewhere or a tool's: {here:?}"
            ));
        }
        let mut names: Vec<&str> = vec![here.as_str(), ELSEWHERE];
        // A folder inside another's would have its logs found twice, under
        // two machines, and so counted twice. Folders are compared where
        // they really are, so a link can't stand one inside another, and
        // one not made yet where its nearest folder that is really is.
        let real = |path: &Path| {
            let mut rest = Vec::new();
            let mut at = path;
            loop {
                if let Ok(real) = std::fs::canonicalize(at) {
                    return rest.iter().rev().fold(real, |real, part| real.join(part));
                }
                match (at.parent(), at.file_name()) {
                    (Some(parent), Some(part)) => {
                        rest.push(part.to_owned());
                        at = parent;
                    }
                    _ => return path.to_owned(),
                }
            }
        };
        let mut folders: Vec<PathBuf> = vec![real(&self.sessions)];
        for machine in &self.machines {
            let name = machine.name.trim();
            if name.is_empty() {
                return Err("a machine has no name".into());
            }
            // Shown as written, so "teal " would be a second teal.
            if name != machine.name {
                return Err(format!(
                    "a machine's name has no spaces around it: {:?}",
                    machine.name
                ));
            }
            if names.contains(&name) || name.ends_with(NO_MACHINE) {
                return Err(format!(
                    "a machine's name is its own, not this machine's, elsewhere or another's: \
                     {:?}",
                    machine.name
                ));
            }
            if !machine.sessions.is_absolute() {
                return Err(format!(
                    "a machine's logs are found by a whole path: {}",
                    machine.sessions.display()
                ));
            }
            let sessions = real(&machine.sessions);
            if let Some(taken) = folders
                .iter()
                .find(|folder| folder.starts_with(&sessions) || sessions.starts_with(folder))
            {
                return Err(format!(
                    "a machine's logs are in a folder of their own, apart from {}: {}",
                    taken.display(),
                    machine.sessions.display()
                ));
            }
            names.push(name);
            folders.push(sessions);
        }
        Ok(())
    }

    /// What this machine is called: as the config names it, else its host
    /// name, else "this machine".
    pub fn machine(&self) -> String {
        self.machine
            .clone()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| host_name().to_owned())
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
        assert!(budgets[0].default, "the one budget isn't the default");
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
        let refused = with(none, serde_json::json!([])).unwrap_err();
        assert!(refused.contains("exactly one"), "no default: {refused}");
        let two = serde_json::json!([budget("a", true, &[]), budget("b", true, &[])]);
        let refused = with(two, serde_json::json!([])).unwrap_err();
        assert!(refused.contains("exactly one"), "two defaults: {refused}");
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
        let refused = with(serde_json::json!([budget("a", true, &[])]), feed.clone()).unwrap_err();
        assert!(refused.contains("nowhere"), "{refused}");
        // With none declared, there's only the implicit one to name.
        with(serde_json::json!([]), feed)
            .expect_err("a feed named a budget though none was declared");
    }

    #[test]
    fn an_allowance_in_dollars_says_how_many() {
        let declared = |allowance: serde_json::Value| {
            let mut budget = budget("a", true, &[]);
            budget["allowances"] = serde_json::json!([allowance]);
            with(serde_json::json!([budget]), serde_json::json!([]))
        };
        let refused = declared(serde_json::json!({"name": "w", "windowHours": 168, "warn": 0.9}))
            .unwrap_err();
        assert!(refused.contains("says how many"), "{refused}");
        let share = serde_json::json!({
            "name": "w", "unit": "share", "allowance": 5, "windowHours": 5, "warn": 0.9,
        });
        declared(share).expect_err("a share's allowance named dollars and was taken");
        let held = serde_json::json!({
            "name": "w", "allowance": 400, "windowHours": 168, "warn": 0.9,
            "resets": "2026-11-09T05:00:00Z",
        });
        declared(held).expect("an allowance with its own reset was refused");
    }

    fn with_machines(machines: &serde_json::Value) -> Result<(), String> {
        let config = serde_json::json!({
            "sessions": "/s", "runs": "/r", "registry": "/g", "quests": "/q",
            "state": "/t", "machine": "slate", "machines": machines,
        });
        serde_json::from_value::<Config>(config)
            .map_err(|error| error.to_string())?
            .check()
    }

    // Another machine's work is shown under its name, so a name it
    // shares with this machine, with elsewhere or with another would
    // count one's work as the other's.
    #[test]
    fn each_machine_has_a_name_of_its_own() {
        let machine =
            |name: &str| serde_json::json!({"name": name, "sessions": format!("/m/{name}")});
        let two = serde_json::json!([machine("teal"), machine("ochre")]);
        assert_eq!(
            with_machines(&two),
            Ok(()),
            "two machines of their own names were refused"
        );
        for named in [
            "slate",
            "elsewhere",
            "",
            "teal ",
            " slate",
            "codex (no machine)",
        ] {
            assert!(
                with_machines(&serde_json::json!([machine(named)])).is_err(),
                "a machine named {named:?} was taken"
            );
        }
        assert!(
            with_machines(&serde_json::json!([machine("teal"), machine("teal")])).is_err(),
            "two machines of one name were taken"
        );
        for here in ["elsewhere", "codex (no machine)", "slate ", " slate"] {
            let config = serde_json::json!({
                "sessions": "/s", "runs": "/r", "registry": "/g", "quests": "/q",
                "state": "/t", "machine": here,
            });
            assert!(
                serde_json::from_value::<Config>(config)
                    .unwrap()
                    .check()
                    .is_err(),
                "this machine was taken as {here:?}"
            );
        }
        let relative = serde_json::json!([{"name": "teal", "sessions": "m/s"}]);
        assert!(
            with_machines(&relative).is_err(),
            "a machine's logs were found by part of a path"
        );
        let walked = serde_json::json!([{"name": "teal", "sessions": "/m/s",
            "agents": [{"pattern": "/m/**/*.jsonl", "kind": "review"}]}]);
        assert!(
            with_machines(&walked).is_err(),
            "a machine's agents' logs walked a tree"
        );
    }

    // A machine's logs found inside another's folder, or this machine's,
    // would be read under both, and their spend counted twice.
    #[test]
    fn each_machine_s_logs_are_in_a_folder_of_their_own() {
        let at =
            |name: &str, sessions: &str| serde_json::json!({"name": name, "sessions": sessions});
        for (sessions, over) in [
            ("/s", "this machine's"),
            ("/s/teal", "inside this machine's"),
            ("/", "over this machine's"),
        ] {
            assert!(
                with_machines(&serde_json::json!([at("teal", sessions)])).is_err(),
                "a machine's logs {over} folder were taken"
            );
        }
        for (first, second) in [("/m/a", "/m/a"), ("/m/a", "/m/a/b"), ("/m/a/b", "/m/a")] {
            assert!(
                with_machines(&serde_json::json!([at("teal", first), at("ochre", second)]))
                    .is_err(),
                "logs at {second} were taken beside logs at {first}"
            );
        }
        let apart = serde_json::json!([at("teal", "/m/a"), at("ochre", "/m/ab")]);
        assert_eq!(
            with_machines(&apart),
            Ok(()),
            "folders sharing only a prefix were refused"
        );
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("a")).unwrap();
        std::os::unix::fs::symlink(dir.path().join("a"), dir.path().join("b")).unwrap();
        let linked = serde_json::json!([
            at("teal", &dir.path().join("a").to_string_lossy()),
            at("ochre", &dir.path().join("b").to_string_lossy()),
        ]);
        assert!(
            with_machines(&linked).is_err(),
            "a link to another machine's folder was taken as a folder of its own"
        );
        // Not made until the daemon starts, so compared where they'd be.
        let unmade = serde_json::json!([
            at("teal", &dir.path().join("a/x").to_string_lossy()),
            at("ochre", &dir.path().join("b/x/y").to_string_lossy()),
        ]);
        assert!(
            with_machines(&unmade).is_err(),
            "a folder not made yet, through a link into another's, was taken as its own"
        );
    }

    // Named nowhere, this machine is its host name, read as the daemon
    // reads it, and no other machine may take that name.
    #[test]
    fn a_machine_named_nowhere_is_its_host() {
        let host = rustix::system::uname()
            .nodename()
            .to_string_lossy()
            .trim()
            .to_owned();
        let host = if host.is_empty() {
            "this machine".to_owned()
        } else {
            host
        };
        for named in [serde_json::Value::Null, serde_json::json!("  ")] {
            let config = serde_json::json!({
                "sessions": "/s", "runs": "/r", "registry": "/g", "quests": "/q",
                "state": "/t", "machine": named,
                "machines": [{"name": host, "sessions": "/m/host"}],
            });
            let config: Config = serde_json::from_value(config).unwrap();
            assert_eq!(
                config.machine(),
                host,
                "a machine named {named} wasn't its host"
            );
            assert!(
                config.check().is_err(),
                "another machine took the host's name while this one was named {named}"
            );
        }
    }

    // A pattern is a whole path whose each star is one name, so finding
    // agents' logs never walks a tree, and a kind is one word.
    #[test]
    fn agents_logs_are_named_by_a_whole_path() {
        with_agents("/w/*/agents/*/session/*.jsonl", "swarm")
            .expect("a whole path with one name to each star was refused");
        with_agents("w/*/session/*.jsonl", "swarm").expect_err("a relative pattern was taken");
        with_agents("/w/**/*.jsonl", "swarm").expect_err("a pattern that walks a tree was taken");
        with_agents("/w/*.jsonl", "").expect_err("an empty kind was taken");
        with_agents("/w/*.jsonl", "two words").expect_err("a kind of two words was taken");
    }
}
