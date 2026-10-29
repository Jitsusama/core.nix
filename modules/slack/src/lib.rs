//! Who Slack knows by an email address, as a library the program
//! (main.rs) and its tests share: the harness's Slack sign-in, read and
//! never written ([`Account`]), and one look-up by address ([`Slack`]),
//! said to the shell as one JSON line ([`Found`]).
//!
//! It only ever reads: `users.lookupByEmail` is the one method it calls,
//! and nothing it does sends a message or changes anything in Slack.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ureq::Agent;

/// The harness's Slack sign-in, as its credentials file keeps it: the
/// token Slack's own client uses, with the cookie it goes with, and the
/// workspace it's for.
#[derive(Debug, Deserialize)]
pub struct Account {
    #[serde(rename = "accessToken")]
    pub token: String,
    pub cookie: String,
    #[serde(rename = "teamId")]
    pub team: String,
}

#[derive(Deserialize)]
struct File {
    token: Account,
}

impl Account {
    /// The sign-in in the harness's file, or none when the file is
    /// missing or isn't one: then the program says nothing of anyone.
    pub fn read(path: &Path) -> Option<Account> {
        let text = std::fs::read_to_string(path).ok()?;
        serde_json::from_str::<File>(&text)
            .ok()
            .map(|file| file.token)
    }
}

/// What Slack said of an address, as the shell is told it: who it is in
/// Slack and its workspace, which open them there, and their name; or
/// that Slack knows nobody by it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Found {
    pub email: String,
    /// Their id in Slack, or none for an address Slack knows nobody by.
    pub id: Option<String>,
    pub team: String,
    pub name: String,
}

/// Why Slack said nothing of an address.
#[derive(Debug, PartialEq, Eq)]
pub enum Refused {
    /// Slack won't take the sign-in any more: signing in again from pi
    /// mends it, and until then nobody is asked about.
    SignedOut,
    /// Slack couldn't be reached, or answered with an error of its own;
    /// asking again later may work.
    Unreachable(String),
}

/// Slack's API, at an origin given rather than fixed, so the tests can
/// ask a stand-in on this machine.
pub struct Slack {
    agent: Agent,
    origin: String,
}

impl Slack {
    pub fn new(origin: impl Into<String>) -> Slack {
        let agent = Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(10)))
            .http_status_as_error(false)
            .build()
            .into();
        Slack {
            agent,
            origin: origin.into(),
        }
    }

    /// Who Slack knows by `email` in the account's workspace.
    pub fn look_up(&self, account: &Account, email: &str) -> Result<Found, Refused> {
        let mut answer = self
            .agent
            .get(format!("{}/api/users.lookupByEmail", self.origin))
            .header("Authorization", &format!("Bearer {}", account.token))
            .header("Cookie", &format!("d={}", account.cookie))
            .query("email", email)
            .call()
            .map_err(|error| Refused::Unreachable(format!("couldn't reach Slack: {error}")))?;
        let status = answer.status().as_u16();
        if status != 200 {
            return Err(Refused::Unreachable(format!("Slack answered {status}")));
        }
        let said: Value = answer
            .body_mut()
            .read_json()
            .map_err(|error| Refused::Unreachable(format!("Slack's answer: {error}")))?;
        found(&said, email, &account.team)
    }
}

/// What Slack's answer to a look-up says.
pub fn found(said: &Value, email: &str, team: &str) -> Result<Found, Refused> {
    let nobody = || Found {
        email: email.to_owned(),
        id: None,
        team: team.to_owned(),
        name: String::new(),
    };
    if said.get("ok").and_then(Value::as_bool) == Some(true) {
        let user = said.get("user").unwrap_or(&Value::Null);
        let text = |pointer: &str| {
            user.pointer(pointer)
                .and_then(Value::as_str)
                .filter(|said| !said.is_empty())
        };
        let Some(id) = text("/id") else {
            return Ok(nobody());
        };
        let name = text("/profile/real_name")
            .or_else(|| text("/real_name"))
            .or_else(|| text("/profile/display_name"))
            .unwrap_or_default();
        return Ok(Found {
            email: email.to_owned(),
            id: Some(id.to_owned()),
            team: text("/team_id").unwrap_or(team).to_owned(),
            name: name.to_owned(),
        });
    }
    match said.get("error").and_then(Value::as_str).unwrap_or("") {
        "users_not_found" => Ok(nobody()),
        "invalid_auth" | "not_authed" | "account_inactive" | "token_revoked" | "token_expired" => {
            Err(Refused::SignedOut)
        }
        other => Err(Refused::Unreachable(format!(
            "Slack refused the look-up: {other}"
        ))),
    }
}

/// Whether an address is worth asking Slack about: one `@` with
/// something either side, and nothing that would read as more than one.
pub fn asked(email: &str) -> bool {
    let Some((name, host)) = email.split_once('@') else {
        return false;
    };
    !name.is_empty()
        && host.contains('.')
        && !host.contains('@')
        && !email.chars().any(|c| c.is_whitespace() || c == ',')
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn someone_found_is_said_by_their_id_and_real_name() {
        let said = json!({ "ok": true, "user": { "id": "U1", "team_id": "T9",
            "real_name": "Ada L", "profile": { "real_name": "Ada Lovelace" } } });
        assert_eq!(
            found(&said, "ada@example.com", "T1"),
            Ok(Found {
                email: "ada@example.com".into(),
                id: Some("U1".into()),
                team: "T9".into(),
                name: "Ada Lovelace".into(),
            })
        );
    }

    #[test]
    fn an_address_slack_knows_nobody_by_is_said_so() {
        let said = json!({ "ok": false, "error": "users_not_found" });
        let nobody = found(&said, "x@example.com", "T1").unwrap();
        assert_eq!(nobody.id, None);
        assert_eq!(nobody.team, "T1");
    }

    #[test]
    fn a_sign_in_slack_refuses_is_signed_out() {
        for error in ["invalid_auth", "token_revoked"] {
            let said = json!({ "ok": false, "error": error });
            assert_eq!(found(&said, "x@example.com", "T1"), Err(Refused::SignedOut));
        }
    }

    #[test]
    fn a_rate_limit_is_unreachable_for_now() {
        let said = json!({ "ok": false, "error": "ratelimited" });
        assert!(matches!(
            found(&said, "x@example.com", "T1"),
            Err(Refused::Unreachable(_))
        ));
    }

    #[test]
    fn only_a_plain_address_is_asked_about() {
        assert!(asked("ada@example.com"));
        for bad in [
            "",
            "ada",
            "@example.com",
            "ada@host",
            "a@b@c.com",
            "a b@c.com",
            "a@b.com,c@d.com",
        ] {
            assert!(!asked(bad), "{bad}");
        }
    }
}
