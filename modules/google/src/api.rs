//! Asking Google: the token endpoint, and the Calendar, Gmail, Drive and
//! Meet APIs as the account.
//!
//! The origins are given rather than fixed, so the tests can ask a stand-in
//! on this machine instead (`tests/stand_in.rs`).

use std::time::Duration;

use jiff::Timestamp;
use serde_json::Value;
use ureq::Agent;

use crate::account::Account;
use crate::calendar::Answer;
use crate::drive;
use crate::mail;

/// The most a file's answer may be: Gmail takes a file of 25 MB, which
/// its base64 makes a third again.
const ATTACHMENT_MOST: u64 = 36 * 1024 * 1024;

/// The scope that lets this program make a Meet meeting, and manage only
/// those it made, which a sign-in from before pi asked for it lacks.
pub const MAKES_MEETINGS: &str = "https://www.googleapis.com/auth/meetings.space.created";

/// Where Google answers.
pub struct Origins {
    /// The OAuth token endpoint.
    pub token: String,
    /// The APIs' root, before `/calendar/v3`, `/gmail/v1` and `/drive/v3`.
    pub api: String,
    /// Meet's API, which answers only on a host of its own.
    pub meet: String,
}

impl Default for Origins {
    fn default() -> Origins {
        Origins {
            token: "https://oauth2.googleapis.com/token".into(),
            api: "https://www.googleapis.com".into(),
            meet: "https://meet.googleapis.com".into(),
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

    /// The lines a series of meetings on the primary calendar repeats by,
    /// its `recurrence`, as RFC 5545 writes them (RRULE, EXDATE, RDATE),
    /// which an occurrence doesn't carry. A series gone since says none.
    pub fn recurrence(&self, token: &str, series: &str) -> Result<Vec<String>, Refused> {
        let mut answer = self
            .agent
            .get(format!(
                "{}/calendar/v3/calendars/primary/events/{series}",
                self.origins.api
            ))
            .header("Authorization", &format!("Bearer {token}"))
            .query("fields", "recurrence")
            .call()
            .map_err(|error| unreachable(&error))?;
        let status = answer.status().as_u16();
        if status == 404 || status == 410 {
            return Ok(Vec::new());
        }
        if status != 200 {
            return Err(refused(status, "a series of meetings"));
        }
        let body: Value = answer.body_mut().read_json().map_err(|error| {
            Refused::Unreachable(format!("Google's answer to a series of meetings: {error}"))
        })?;
        Ok(body
            .get("recurrence")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect())
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
                ("format", "full"),
                (
                    "fields",
                    "id,threadId,internalDate,payload(headers,mimeType,filename,body,\
                     parts(mimeType,filename,body,parts(mimeType,filename,body,\
                     parts(mimeType,filename,body))))",
                ),
            ],
            "a piece of mail",
        )
    }

    /// The bytes of the file Gmail keeps as `file` on the mail `id`, as
    /// its URL-safe base64.
    pub fn attachment(&self, token: &str, id: &str, file: &str) -> Result<String, Refused> {
        let mut answer = self
            .agent
            .get(format!(
                "{}/gmail/v1/users/me/messages/{id}/attachments/{file}",
                self.origins.api
            ))
            .header("Authorization", &format!("Bearer {token}"))
            .query("fields", "data")
            .call()
            .map_err(|error| unreachable(&error))?;
        let status = answer.status().as_u16();
        if status != 200 {
            return Err(refused(status, "a file"));
        }
        let answer: Value = answer
            .body_mut()
            .with_config()
            .limit(ATTACHMENT_MOST)
            .read_json()
            .map_err(|error| Refused::Unreachable(format!("Google's answer to a file: {error}")))?;
        answer
            .get("data")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| Refused::Unreachable("Google answered a file without it".into()))
    }

    /// A new Meet meeting, made as Joel's own, and the link that joins it.
    /// It's a meeting space, not an event, so it's on no calendar and
    /// nobody is told of it: whoever has the link may join.
    pub fn space(&self, token: &str) -> Result<String, Refused> {
        let mut answer = self
            .agent
            .post(format!("{}/v2/spaces", self.origins.meet))
            .header("Authorization", &format!("Bearer {token}"))
            .send_json(serde_json::json!({}))
            .map_err(|error| unreachable(&error))?;
        let status = answer.status().as_u16();
        if status != 200 {
            return Err(refused(status, "making a meeting"));
        }
        let answer: Value = answer.body_mut().read_json().map_err(|error| {
            Refused::Unreachable(format!("Google's answer to making a meeting: {error}"))
        })?;
        answer
            .get("meetingUri")
            .and_then(Value::as_str)
            .filter(|link| link.starts_with("https://meet.google.com/"))
            .map(str::to_owned)
            .ok_or_else(|| Refused::Unreachable("Google made a meeting without a link".into()))
    }

    /// Takes the label `label` off the mail `id`, as Gmail's own buttons
    /// do: UNREAD to mark it read, INBOX to archive it.
    pub fn unlabel(&self, token: &str, id: &str, label: &str) -> Result<(), Refused> {
        self.relabel(token, id, "removeLabelIds", label)
    }

    /// Puts the label `label` on the mail `id`: UNREAD to mark it unread
    /// again.
    pub fn label(&self, token: &str, id: &str, label: &str) -> Result<(), Refused> {
        self.relabel(token, id, "addLabelIds", label)
    }

    /// Changes one label on the mail `id`, adding or removing it as `how`
    /// names Gmail's field.
    fn relabel(&self, token: &str, id: &str, how: &str, label: &str) -> Result<(), Refused> {
        let written = self
            .agent
            .post(format!(
                "{}/gmail/v1/users/me/messages/{id}/modify",
                self.origins.api
            ))
            .header("Authorization", &format!("Bearer {token}"))
            .query("fields", "id")
            .send_json(serde_json::json!({ how: [label] }))
            .map_err(|error| unreachable(&error))?;
        let status = written.status().as_u16();
        if status != 200 {
            return Err(refused(status, "marking mail"));
        }
        Ok(())
    }

    /// The documents of one kind Joel looked at lately, the last first,
    /// from his own drive, what's shared with him and shared drives alike,
    /// with only what Recent says of each.
    pub fn documents(&self, token: &str, kind: drive::Kind) -> Result<Value, Refused> {
        let shown = drive::SHOWN.to_string();
        self.get(
            token,
            "/drive/v3/files",
            &[
                ("q", &kind.search()),
                ("orderBy", "viewedByMeTime desc"),
                ("pageSize", &shown),
                ("corpora", "allDrives"),
                ("includeItemsFromAllDrives", "true"),
                ("supportsAllDrives", "true"),
                (
                    "fields",
                    "files(id,name,webViewLink,viewedByMeTime,modifiedTime,owners(displayName))",
                ),
            ],
            "the documents",
        )
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
    /// one answer's `items`, beside the calendar's `defaultReminders`,
    /// which the first page says.
    pub fn events(&self, token: &str, from: Timestamp, to: Timestamp) -> Result<Value, Refused> {
        let url = format!("{}/calendar/v3/calendars/primary/events", self.origins.api);
        let (from, to) = (from.to_string(), to.to_string());
        let mut items = Vec::new();
        let mut defaults: Option<Value> = None;
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
            if defaults.is_none() {
                defaults = body.get_mut("defaultReminders").map(Value::take);
            }
            page = body
                .get("nextPageToken")
                .and_then(Value::as_str)
                .map(str::to_owned);
            if page.is_none() {
                return Ok(serde_json::json!({
                    "items": items,
                    "defaultReminders": defaults.unwrap_or(Value::Array(Vec::new())),
                }));
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

    /// The ids of the calendars Joel may change events on, every page of
    /// his calendar list: those he owns or was given writer access to.
    pub fn writable(&self, token: &str) -> Result<Vec<String>, Refused> {
        let mut ids = Vec::new();
        let mut page: Option<String> = None;
        loop {
            let mut query = vec![
                ("minAccessRole", "writer"),
                ("fields", "items(id),nextPageToken"),
            ];
            if let Some(page) = &page {
                query.push(("pageToken", page));
            }
            let answer = self.get(
                token,
                "/calendar/v3/users/me/calendarList",
                &query,
                "the calendars",
            )?;
            ids.extend(
                answer
                    .get("items")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|calendar| calendar.get("id").and_then(Value::as_str))
                    .map(str::to_owned),
            );
            page = answer
                .get("nextPageToken")
                .and_then(Value::as_str)
                .map(str::to_owned);
            if page.is_none() {
                return Ok(ids);
            }
        }
    }

    /// Moves the meeting `id`, one occurrence of a repeating one, on the
    /// calendar `through`, to run from `start` to `end`, and Google mails
    /// everyone on it the new time, as when it's moved in Google Calendar.
    /// Only its times change, given as instants; its time zone and all
    /// else stay as they were.
    pub fn reschedule(
        &self,
        token: &str,
        through: &str,
        id: &str,
        start: Timestamp,
        end: Timestamp,
    ) -> Result<(), Refused> {
        let written = self
            .agent
            .patch(format!(
                "{}/calendar/v3/calendars/{}/events/{id}",
                self.origins.api,
                segment(through)
            ))
            .header("Authorization", &format!("Bearer {token}"))
            .query("sendUpdates", "all")
            .query("fields", "id")
            .send_json(serde_json::json!({
                "start": { "dateTime": start.to_string() },
                "end": { "dateTime": end.to_string() },
            }))
            .map_err(|error| unreachable(&error))?;
        let status = written.status().as_u16();
        if status != 200 {
            return Err(refused(status, "moving a meeting"));
        }
        Ok(())
    }
}

/// A calendar's id as one part of a path: an address, or a group's, with
/// anything but its letters, digits and `@.-_` written as `%XX`, so an id
/// holding a `#` or a `/` stays one part.
fn segment(id: &str) -> String {
    id.bytes()
        .map(|byte| match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'@' | b'.' | b'-' | b'_' => {
                char::from(byte).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_calendar_id_stays_one_part_of_a_path() {
        assert_eq!(segment("joel@example.com"), "joel@example.com");
        assert_eq!(
            segment("en.canadian#holiday@group.v.calendar.google.com"),
            "en.canadian%23holiday@group.v.calendar.google.com"
        );
        assert_eq!(segment("a/b c"), "a%2Fb%20c");
    }
}
