//! The sign-in Claude Code keeps in its config folder's `.credentials.json`,
//! as `claudeAiOauth`: the access token, when it expires (in milliseconds),
//! its scopes, and the plan it's for. Only Linux keeps it there; on macOS
//! it's in the keychain, which this doesn't read. The refresh token is never
//! read at all.

use std::io::ErrorKind;
use std::path::Path;

use jiff::Timestamp;
use serde::Deserialize;

/// The scope a token needs for the usage endpoint to answer it.
const READS_USAGE: &str = "user:profile";

/// How the plan's sign-in stands, as of a moment.
#[derive(Debug, PartialEq)]
pub enum Login {
    /// Nothing is signed in to a plan there: no file, or one holding no
    /// plan's sign-in, as when only an MCP server's is kept.
    Missing,
    /// The token has expired, and Claude Code renews it next time it runs.
    Expired,
    /// The token can't read usage: it lacks `user:profile`, as a long-lived
    /// token made for model requests alone does.
    Unscoped,
    /// A token to ask with, and the plan's name.
    Signed { token: String, plan: Plan },
}

/// The plan's name, as Claude Code's sign-in says it: `pro`, `max`,
/// `ultra`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan(Option<String>);

impl Plan {
    /// What the plan is called where its windows are shown: "Claude Pro",
    /// or "Claude" when the sign-in doesn't say.
    pub fn title(&self) -> String {
        let mut letters = self.0.as_deref().unwrap_or_default().chars();
        match letters.next() {
            Some(first) => format!("Claude {}{}", first.to_uppercase(), letters.as_str()),
            None => "Claude".into(),
        }
    }
}

/// The sign-in in `file` as of `now`, or why it can't be read.
pub fn read(file: &Path, now: Timestamp) -> Result<Login, String> {
    let written = match std::fs::read_to_string(file) {
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Login::Missing),
        written => written.map_err(|error| format!("can't read Claude Code's sign-in: {error}"))?,
    };
    let kept: Kept = serde_json::from_str(&written)
        .map_err(|error| format!("Claude Code's sign-in doesn't read: {error}"))?;
    let Some(oauth) = kept
        .claude_ai_oauth
        .filter(|oauth| !oauth.access_token.is_empty())
    else {
        return Ok(Login::Missing);
    };
    if oauth
        .expires_at
        .is_some_and(|expires| expires <= milliseconds(now))
    {
        return Ok(Login::Expired);
    }
    if oauth
        .scopes
        .is_some_and(|scopes| !scopes.iter().any(|scope| scope == READS_USAGE))
    {
        return Ok(Login::Unscoped);
    }
    Ok(Login::Signed {
        token: oauth.access_token,
        plan: Plan(oauth.subscription_type.filter(|name| !name.is_empty())),
    })
}

#[expect(
    clippy::cast_precision_loss,
    reason = "milliseconds since the epoch stay below 2^53 for 285,000 years, so f64 holds \
              them exactly"
)]
fn milliseconds(at: Timestamp) -> f64 {
    at.as_millisecond() as f64
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Kept {
    claude_ai_oauth: Option<OAuth>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OAuth {
    #[serde(default)]
    access_token: String,
    expires_at: Option<f64>,
    scopes: Option<Vec<String>>,
    subscription_type: Option<String>,
}
