//! The usage endpoint Claude Code's `/usage` asks, which Anthropic hasn't
//! documented: README.md says where its name and shape are known from. It
//! answers only a token holding the `user:profile` scope, and only with the
//! OAuth beta named.

use std::time::{Duration, Instant};

use ureq::Agent;

use claude_plan_feed::{Answer, Endpoint, Unreached};

const USAGE: &str = "https://api.anthropic.com/api/oauth/usage";
const BETA: &str = "oauth-2025-04-20";

/// Claude's own usage endpoint, over HTTPS.
pub struct Anthropic {
    agent: Agent,
}

impl Anthropic {
    pub fn new() -> Anthropic {
        let agent = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(30)))
            .http_status_as_error(false)
            .user_agent("claude-plan-feed (core.nix)")
            .build()
            .into();
        Anthropic { agent }
    }
}

impl Endpoint for Anthropic {
    fn usage(&mut self, token: &str) -> Result<Answer, Unreached> {
        let started = Instant::now();
        let millis = || u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let mut answer = self
            .agent
            .get(USAGE)
            .header("Authorization", &format!("Bearer {token}"))
            .header("anthropic-beta", BETA)
            .header("Accept", "application/json")
            .call()
            .map_err(|error| Unreached {
                why: error.to_string(),
                millis: millis(),
            })?;
        // Only a count of seconds is read; a date there is rare enough that
        // the feed's own pause serves.
        let retry_after = answer
            .headers()
            .get("retry-after")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse().ok())
            .map(Duration::from_secs);
        let body = answer
            .body_mut()
            .read_to_string()
            .map_err(|error| Unreached {
                why: error.to_string(),
                millis: millis(),
            })?;
        Ok(Answer {
            status: answer.status().as_u16(),
            body,
            retry_after,
            millis: millis(),
        })
    }
}
