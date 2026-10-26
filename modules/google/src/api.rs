//! Asking Google: the token endpoint, and the Calendar API as the account.
//!
//! Both origins are given rather than fixed, so the tests can ask a stand-in
//! on this machine instead (`tests/stand_in.rs`).

use std::time::Duration;

use jiff::Timestamp;
use serde_json::Value;
use ureq::Agent;

use crate::account::Account;

/// Where Google answers.
pub struct Origins {
    /// The OAuth token endpoint.
    pub token: String,
    /// The Calendar API's root, before `/calendar/v3`.
    pub api: String,
}

impl Default for Origins {
    fn default() -> Origins {
        Origins {
            token: "https://oauth2.googleapis.com/token".into(),
            api: "https://www.googleapis.com".into(),
        }
    }
}

/// Why Google didn't answer with what was asked.
#[derive(Debug, PartialEq, Eq)]
pub enum Refused {
    /// Google won't take the account's refresh token any more, or its
    /// access is refused: Joel has to sign in again.
    SignedOut,
    /// Google couldn't be reached, or answered with an error of its own;
    /// asking again later may work.
    Unreachable(String),
}

/// A fresh access token and when it stops working.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Access {
    pub token: String,
    /// Milliseconds since the epoch.
    pub expires: i64,
}

pub struct Google {
    agent: Agent,
    origins: Origins,
}

impl Google {
    pub fn new(origins: Origins) -> Google {
        let agent = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(20)))
            .http_status_as_error(false)
            .user_agent("google (core.nix)")
            .build()
            .into();
        Google { agent, origins }
    }

    /// A new access token for the account, from its refresh token.
    pub fn refresh(&self, account: &Account) -> Result<Access, Refused> {
        let mut answer = self
            .agent
            .post(&self.origins.token)
            .send_form([
                ("client_id", account.client_id.as_str()),
                ("client_secret", account.client_secret.as_str()),
                ("refresh_token", account.refresh_token.as_str()),
                ("grant_type", "refresh_token"),
            ])
            .map_err(|error| Refused::Unreachable(format!("couldn't reach Google: {error}")))?;
        let status = answer.status().as_u16();
        let body: Value = answer.body_mut().read_json().unwrap_or(Value::Null);
        // An unknown, revoked or expired refresh token, and a client Google
        // no longer knows, are said as 400 or 401 with invalid_grant or
        // invalid_client: only signing in again mends either.
        let error = body.get("error").and_then(Value::as_str).unwrap_or("");
        if matches!(status, 400 | 401) && matches!(error, "invalid_grant" | "invalid_client") {
            return Err(Refused::SignedOut);
        }
        if status != 200 {
            return Err(Refused::Unreachable(format!(
                "Google answered {status} to a refresh"
            )));
        }
        let token = body.get("access_token").and_then(Value::as_str);
        let lasts = body.get("expires_in").and_then(Value::as_i64).unwrap_or(0);
        Ok(Access {
            token: token
                .ok_or_else(|| Refused::Unreachable("Google refreshed without a token".into()))?
                .to_owned(),
            expires: Timestamp::now().as_millisecond() + lasts * 1000,
        })
    }

    /// Every event on the primary calendar between two times, each
    /// repeating one as its occurrences, by start, every page joined into
    /// one answer's `items`.
    pub fn events(&self, token: &str, from: Timestamp, to: Timestamp) -> Result<Value, Refused> {
        let url = format!("{}/calendar/v3/calendars/primary/events", self.origins.api);
        let (from, to) = (from.to_string(), to.to_string());
        let mut items = Vec::new();
        let mut page: Option<String> = None;
        loop {
            let mut request = self
                .agent
                .get(&url)
                .header("Authorization", &format!("Bearer {token}"))
                .query("timeMin", &from)
                .query("timeMax", &to)
                .query("singleEvents", "true")
                .query("orderBy", "startTime")
                .query("maxResults", "250");
            if let Some(page) = &page {
                request = request.query("pageToken", page);
            }
            let mut answer = request
                .call()
                .map_err(|error| Refused::Unreachable(format!("couldn't reach Google: {error}")))?;
            let status = answer.status().as_u16();
            // A 403 is as often a quota as a missing grant, so only a 401
            // says the token is no good.
            if status == 401 {
                return Err(Refused::SignedOut);
            }
            if status != 200 {
                return Err(Refused::Unreachable(format!(
                    "Google Calendar answered {status}"
                )));
            }
            let mut body: Value = answer.body_mut().read_json().map_err(|error| {
                Refused::Unreachable(format!("Google Calendar's answer: {error}"))
            })?;
            if let Some(Value::Array(more)) = body.get_mut("items").map(Value::take) {
                items.extend(more);
            }
            page = body
                .get("nextPageToken")
                .and_then(Value::as_str)
                .map(str::to_owned);
            if page.is_none() {
                return Ok(serde_json::json!({ "items": items }));
            }
        }
    }
}
