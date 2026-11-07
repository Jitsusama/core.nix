//! Budgets: the money a request draws on, a Claude plan, an API key, a
//! proxy's cap or a team's token, as the config declares them. pi's logs
//! name one provider for everything a proxy carries, whoever pays, so
//! which budget a turn spends is worked out here: the budget claiming its
//! provider, since that's what's paid whatever the session's log says;
//! else the one its log's nearest budget entry names; else the default.
//! A feed's bill is its own budget's unless a record says another.
//!
//! A name no budget is declared by still counts, under a budget of that
//! name, shown as undeclared, so a sum over budgets is always the whole.

use crate::config::{BudgetConfig, Config};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Budgets {
    /// Every budget declared, or the implicit default, in the config's order.
    pub all: Vec<BudgetConfig>,
    /// The budget a turn draws on when nothing says otherwise.
    pub default: String,
    claims: HashMap<String, String>,
    feeds: HashMap<String, String>,
}

impl Budgets {
    pub fn of(config: &Config) -> Budgets {
        let all = config.budgets();
        let default = all.iter().find(|budget| budget.default).map_or_else(
            || crate::config::IMPLICIT.to_owned(),
            |budget| budget.name.clone(),
        );
        let claims = all
            .iter()
            .flat_map(|budget| {
                budget
                    .providers
                    .iter()
                    .map(|provider| (provider.clone(), budget.name.clone()))
            })
            .collect();
        let feeds = config
            .feeds
            .iter()
            .map(|feed| {
                let budget = feed.budget.clone().unwrap_or_else(|| default.clone());
                (feed.name.clone(), budget)
            })
            .collect();
        Budgets {
            all,
            default,
            claims,
            feeds,
        }
    }

    /// What a machine that declares nothing has: one budget, the default.
    pub fn implicit() -> Budgets {
        let config: Config = serde_json::from_value(serde_json::json!({
            "sessions": "/", "runs": "/", "registry": "/", "quests": "/", "state": "/",
        }))
        .expect("the least config");
        Budgets::of(&config)
    }

    /// The budget a turn draws on, from its provider and what its log
    /// said (none where nothing did, empty for the default), or none when
    /// neither says, for a run's agent to take its session's.
    pub fn of_turn(&self, provider: &str, logged: Option<&str>) -> Option<String> {
        self.claims
            .get(provider)
            .cloned()
            .or_else(|| logged.map(|name| self.named(name)))
    }

    /// The budget a log's word names: the default for none.
    pub fn named(&self, logged: &str) -> String {
        if logged.is_empty() {
            self.default.clone()
        } else {
            logged.to_owned()
        }
    }

    /// The budget a feed's bill and caps are for, unless a record says
    /// another.
    pub fn of_feed(&self, feed: &str) -> &str {
        self.feeds
            .get(feed)
            .map_or(self.default.as_str(), String::as_str)
    }

    /// The budgets the configured feeds bill.
    pub fn of_feeds(&self) -> Vec<String> {
        self.feeds.values().cloned().collect()
    }

    /// The feeds whose bill is a budget's unless a record says another.
    pub fn feeds_of(&self, budget: &str) -> Vec<String> {
        self.feeds
            .iter()
            .filter(|(_, of)| *of == budget)
            .map(|(feed, _)| feed.clone())
            .collect()
    }

    pub fn declared(&self, name: &str) -> Option<&BudgetConfig> {
        self.all.iter().find(|budget| budget.name == name)
    }

    /// Whether a budget is a plan, whose turns cost nothing past its
    /// price.
    pub fn is_plan(&self, name: &str) -> bool {
        self.declared(name)
            .is_some_and(|budget| budget.kind == crate::config::BudgetKind::Plan)
    }

    /// The budget a record names, or its feed's when it names none.
    pub fn of_record(&self, feed: &str, named: &str) -> String {
        if named.is_empty() {
            self.of_feed(feed).to_owned()
        } else {
            named.to_owned()
        }
    }
}

/// A window's name, its length in words in Title Case, as an allowance
/// no feed names is called: "Five Hours", "Day", "Week", "Ten Days".
pub fn window_name(hours: u32) -> String {
    match hours {
        1 => "Hour".into(),
        24 => "Day".into(),
        168 => "Week".into(),
        hours if hours % 168 == 0 => format!("{} Weeks", counted(hours / 168)),
        hours if hours % 24 == 0 => format!("{} Days", counted(hours / 24)),
        hours => format!("{} Hours", counted(hours)),
    }
}

/// A count in words in Title Case, since a name never opens with its
/// figure; past what a window is ever counted in, the figure itself.
fn counted(count: u32) -> String {
    const ONES: [&str; 20] = [
        "Zero",
        "One",
        "Two",
        "Three",
        "Four",
        "Five",
        "Six",
        "Seven",
        "Eight",
        "Nine",
        "Ten",
        "Eleven",
        "Twelve",
        "Thirteen",
        "Fourteen",
        "Fifteen",
        "Sixteen",
        "Seventeen",
        "Eighteen",
        "Nineteen",
    ];
    const TENS: [&str; 10] = [
        "", "", "Twenty", "Thirty", "Forty", "Fifty", "Sixty", "Seventy", "Eighty", "Ninety",
    ];
    let index = |value: u32| usize::try_from(value).unwrap_or(0);
    match count {
        0..20 => ONES[index(count)].into(),
        20..100 if count.is_multiple_of(10) => TENS[index(count / 10)].into(),
        20..100 => format!("{}-{}", TENS[index(count / 10)], ONES[index(count % 10)]),
        _ => count.to_string(),
    }
}

/// A window as a few words beside a share, as in "62% of 5 h".
pub fn window_label(hours: u32) -> String {
    match hours {
        24 => "the day".into(),
        168 => "the week".into(),
        hours if hours % 24 == 0 => format!("{} days", hours / 24),
        hours => format!("{hours} h"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declared() -> Budgets {
        let config: Config = serde_json::from_value(serde_json::json!({
            "sessions": "/", "runs": "/", "registry": "/", "quests": "/", "state": "/",
            "budgets": [
                {"name": "proxy", "label": "Your proxy cap", "kind": "cap", "whose": "employer",
                 "default": true},
                {"name": "claude", "label": "Claude Pro", "kind": "plan", "whose": "own",
                 "providers": ["claude-code"]},
                {"name": "delta", "label": "Delta team token", "kind": "team", "whose": "team"},
            ],
            "feeds": [{"name": "meter", "label": "the AI proxy", "command": ["f"]}],
        }))
        .unwrap();
        Budgets::of(&config)
    }

    // A provider's claim wins over the log, since the provider is what's
    // paid; the log names the rest, and a turn nothing names is left for
    // its run's session.
    #[test]
    fn a_turn_draws_on_its_provider_s_budget_then_its_log_s() {
        let budgets = declared();
        assert_eq!(
            budgets.of_turn("claude-code", Some("delta")).as_deref(),
            Some("claude")
        );
        assert_eq!(
            budgets.of_turn("anthropic", Some("delta")).as_deref(),
            Some("delta")
        );
        assert_eq!(
            budgets.of_turn("anthropic", Some("")).as_deref(),
            Some("proxy")
        );
        assert_eq!(budgets.of_turn("anthropic", None), None);
        // An undeclared name still counts, as itself.
        assert_eq!(
            budgets.of_turn("anthropic", Some("probe")).as_deref(),
            Some("probe")
        );
    }

    #[test]
    fn a_feed_without_a_budget_is_the_default_s() {
        assert_eq!(declared().of_feed("meter"), "proxy");
        assert_eq!(Budgets::implicit().default, "default");
    }

    #[test]
    fn a_window_is_named_for_its_length_in_words() {
        let named: Vec<String> = [1, 5, 24, 48, 168, 336, 720, 45]
            .into_iter()
            .map(window_name)
            .collect();
        assert_eq!(
            named,
            [
                "Hour",
                "Five Hours",
                "Day",
                "Two Days",
                "Week",
                "Two Weeks",
                "Thirty Days",
                "Forty-Five Hours"
            ]
        );
        // Its figure stays short.
        assert_eq!(window_label(5), "5 h");
    }
}
