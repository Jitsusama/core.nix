//! Asking Google: the token endpoint, and the Calendar and Gmail APIs as
//! the account.
//!
//! Both origins are given rather than fixed, so the tests can ask a stand-in
//! on this machine instead (`tests/stand_in.rs`).

use std::time::Duration;

use jiff::Timestamp;
use serde_json::Value;
use ureq::Agent;

use crate::account::Account;
use crate::calendar::Answer;
use crate::mail;

/// Where Google answers.
pub struct Origins {
    /// The OAuth token endpoint.
    pub token: String,
    /// The APIs' root, before `/calendar/v3` and `/gmail/v1`.
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
    /// asking again later may work. An answer to a meeting Joel isn't
    /// invited to, or one gone, is said this way too.
    Unreachable(String),
}

/// What a status says of an answer from one of Google's APIs, beyond 200.
fn refused(status: u16, what: &str) -> Refused {
    // A 403 is as often a quota as a missing grant, so only a 401 says
    // the token is no good.
    if status == 401 {
        Refused::SignedOut
    } else {
        Refused::Unreachable(format!("Google answered {status} to {what}"))
    }
}

fn unreachable(error: &ureq::Error) -> Refused {
    Refused::Unreachable(format!("couldn't reach Google: {error}"))
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

    /// Asks one of the APIs, at `path` below its root, for a JSON answer.
    fn get(
        &self,
        token: &str,
        path: &str,
        query: &[(&str, &str)],
        what: &str,
    ) -> Result<Value, Refused> {
        let mut request = self
            .agent
            .get(format!("{}{path}", self.origins.api))
            .header("Authorization", &format!("Bearer {token}"));
        for (name, value) in query {
            request = request.query(*name, *value);
        }
        let mut answer = request.call().map_err(|error| unreachable(&error))?;
        let status = answer.status().as_u16();
        if status != 200 {
            return Err(refused(status, what));
        }
        answer
            .body_mut()
            .read_json()
            .map_err(|error| Refused::Unreachable(format!("Google's answer to {what}: {error}")))
    }

    /// The ids of the mail that wants Joel, newest first.
    pub fn wanting(&self, token: &str) -> Result<Value, Refused> {
        let shown = mail::SHOWN.to_string();
        self.get(
            token,
            "/gmail/v1/users/me/messages",
            &[("q", mail::WANTS), ("maxResults", &shown)],
            "the mail",
        )
    }

    /// One piece of mail: who sent it, its subject and when it came, and
    /// nothing of what it says.
    pub fn mail(&self, token: &str, id: &str) -> Result<Value, Refused> {
        self.get(
            token,
            &format!("/gmail/v1/users/me/messages/{id}"),
            &[
                ("format", "metadata"),
                ("metadataHeaders", "From"),
                ("metadataHeaders", "Subject"),
                ("fields", "id,threadId,internalDate,payload/headers"),
            ],
            "a piece of mail",
        )
    }

    /// Takes the label `label` off the mail `id`, as Gmail's own buttons
    /// do: UNREAD to mark it read, INBOX to archive it.
    pub fn unlabel(&self, token: &str, id: &str, label: &str) -> Result<(), Refused> {
        let written = self
            .agent
            .post(format!(
                "{}/gmail/v1/users/me/messages/{id}/modify",
                self.origins.api
            ))
            .header("Authorization", &format!("Bearer {token}"))
            .query("fields", "id")
            .send_json(serde_json::json!({ "removeLabelIds": [label] }))
            .map_err(|error| unreachable(&error))?;
        let status = written.status().as_u16();
        if status != 200 {
            return Err(refused(status, "marking mail"));
        }
        Ok(())
    }

    /// The inbox, for how many in it are unread.
    pub fn inbox(&self, token: &str) -> Result<Value, Refused> {
        self.get(
            token,
            "/gmail/v1/users/me/labels/INBOX",
            &[("fields", "messagesUnread")],
            "the inbox",
        )
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
            let mut answer = request.call().map_err(|error| unreachable(&error))?;
            let status = answer.status().as_u16();
            if status != 200 {
                return Err(refused(status, "the meetings"));
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

    /// Answers the invitation to the meeting `id`, one occurrence of a
    /// repeating one, as Joel: his own attendee's answer changes and
    /// everyone else's stays. The API takes the attendees whole, so they're
    /// read first. Nobody is sent mail about it, as when he answers in
    /// Google Calendar; the organizer sees it on the meeting.
    pub fn answer(&self, token: &str, id: &str, with: Answer) -> Result<(), Refused> {
        let url = format!(
            "{}/calendar/v3/calendars/primary/events/{id}",
            self.origins.api
        );
        let authorization = format!("Bearer {token}");
        let mut read = self
            .agent
            .get(&url)
            .header("Authorization", &authorization)
            .query("fields", "attendees")
            .call()
            .map_err(|error| unreachable(&error))?;
        let status = read.status().as_u16();
        if status != 200 {
            return Err(refused(status, "a meeting's attendees"));
        }
        let mut event: Value = read.body_mut().read_json().map_err(|error| {
            Refused::Unreachable(format!("Google Calendar's attendees: {error}"))
        })?;
        let mut attendees = event.get_mut("attendees").map_or(Value::Null, Value::take);
        let me = attendees.as_array_mut().and_then(|attendees| {
            attendees
                .iter_mut()
                .find(|attendee| attendee.get("self").and_then(Value::as_bool) == Some(true))
        });
        let Some(me) = me else {
            return Err(Refused::Unreachable(
                "Joel isn't invited to the meeting he answered".into(),
            ));
        };
        me["responseStatus"] = Value::from(with.status());
        let written = self
            .agent
            .patch(&url)
            .header("Authorization", &authorization)
            .query("sendUpdates", "none")
            .query("fields", "id")
            .send_json(serde_json::json!({ "attendees": attendees }))
            .map_err(|error| unreachable(&error))?;
        let status = written.status().as_u16();
        if status != 200 {
            return Err(refused(status, "an answer"));
        }
        Ok(())
    }

    /// Cancels the meeting `id`, one occurrence of a repeating one, and
    /// Google mails every guest that it's cancelled, as when Joel deletes
    /// one he holds in Google Calendar. One already gone is cancelled.
    pub fn cancel(&self, token: &str, id: &str) -> Result<(), Refused> {
        let written = self
            .agent
            .delete(format!(
                "{}/calendar/v3/calendars/primary/events/{id}",
                self.origins.api
            ))
            .header("Authorization", &format!("Bearer {token}"))
            .query("sendUpdates", "all")
            .call()
            .map_err(|error| unreachable(&error))?;
        match written.status().as_u16() {
            200 | 204 | 410 => Ok(()),
            status => Err(refused(status, "cancelling a meeting")),
        }
    }
}
