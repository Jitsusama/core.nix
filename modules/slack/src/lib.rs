//! Who Slack knows by an email address, as a library the program
//! (main.rs) and its tests share: the harness's Slack sign-in, read and
//! never written ([`Account`]), and one look-up by address ([`Slack`]),
//! said to the shell as one JSON line ([`Found`]).
//!
//! It only ever reads: `users.lookupByEmail`, and where a workspace
//! refuses that to the sign-in, `search.messages` and `users.info`, then
//! `users.info` and `users.getPresence` for how they are, are the methods
//! it calls, and nothing it does sends a message or changes anything in
//! Slack.

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
/// Slack and its workspace, which open them there, their name, and the
/// time zone their profile keeps, by tzdata's name; or that Slack knows
/// nobody by it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Found {
    pub email: String,
    /// Their id in Slack, or none for an address Slack knows nobody by.
    pub id: Option<String>,
    pub team: String,
    pub name: String,
    /// Their zone, as "America/Toronto", or "" when Slack keeps none.
    pub zone: String,
    /// "active", "away" or "in a huddle", or "" until Slack says.
    pub presence: String,
    /// Their status's words while it's set, or "".
    pub status: String,
    /// Their status's emoji as Unicode while it's set, or "" for none or
    /// for a workspace's own picture, which has no Unicode form to show.
    pub emoji: String,
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
    ///
    /// An enterprise grid refuses the look-up by address to the browser's
    /// sign-in the harness keeps, so there they're found by a message from
    /// the handle their address starts with, and taken only when Slack's
    /// profile for whoever that is gives the same address. Someone who has
    /// never said anything Joel can see isn't found that way.
    pub fn look_up(&self, account: &Account, email: &str) -> Result<Found, Refused> {
        let said = self.ask(account, "users.lookupByEmail", &[("email", email)])?;
        if !refused_to_the_sign_in(&said) {
            return found(&said, email, &account.team);
        }
        let handle = email.split_once('@').map_or(email, |(handle, _)| handle);
        let searched = self.ask(
            account,
            "search.messages",
            &[("query", &format!("from:@{handle}")), ("count", "1")],
        )?;
        let Some(id) = speaker(&searched, handle)? else {
            return found(&nobody(), email, &account.team);
        };
        let user = self.ask(account, "users.info", &[("user", &id)])?;
        if addressed(&user) == Some(email.to_lowercase()) {
            found(&user, email, &account.team)
        } else {
            found(&nobody(), email, &account.team)
        }
    }

    /// Who they are now, for someone already found: Slack asked again by
    /// their id, so a name, a zone or a status changed since is taken.
    pub fn again(&self, account: &Account, known: &Found) -> Result<Found, Refused> {
        let Some(id) = &known.id else {
            return Ok(known.clone());
        };
        let user = self.ask(account, "users.info", &[("user", id)])?;
        found(&user, &known.email, &known.team)
    }

    /// Whether someone is active in Slack now or away, as
    /// `users.getPresence` says, or "" when it says neither.
    pub fn presence(&self, account: &Account, id: &str) -> Result<String, Refused> {
        let said = self.ask(account, "users.getPresence", &[("user", id)])?;
        if said.get("ok").and_then(Value::as_bool) != Some(true) {
            return found(&said, "", "").map(|_| String::new());
        }
        Ok(match said.get("presence").and_then(Value::as_str) {
            Some(word @ ("active" | "away")) => word.to_owned(),
            _ => String::new(),
        })
    }

    /// What Slack answers to one method, asked with the sign-in.
    fn ask(
        &self,
        account: &Account,
        method: &str,
        query: &[(&str, &str)],
    ) -> Result<Value, Refused> {
        let mut asked = self
            .agent
            .get(format!("{}/api/{method}", self.origin))
            .header("Authorization", &format!("Bearer {}", account.token))
            .header("Cookie", &format!("d={}", account.cookie));
        for (name, value) in query {
            asked = asked.query(*name, *value);
        }
        let mut answer = asked
            .call()
            .map_err(|error| Refused::Unreachable(format!("couldn't reach Slack: {error}")))?;
        let status = answer.status().as_u16();
        if status != 200 {
            return Err(Refused::Unreachable(format!("Slack answered {status}")));
        }
        answer
            .body_mut()
            .read_json()
            .map_err(|error| Refused::Unreachable(format!("Slack's answer: {error}")))
    }
}

/// Slack's answer for an address it knows nobody by.
fn nobody() -> Value {
    serde_json::json!({ "ok": false, "error": "users_not_found" })
}

/// Whether Slack refused a method to this kind of sign-in, as an
/// enterprise grid refuses the look-up by address to a browser's.
pub fn refused_to_the_sign_in(said: &Value) -> bool {
    matches!(
        said.get("error").and_then(Value::as_str),
        Some("not_allowed_token_type" | "enterprise_is_restricted" | "missing_scope")
    )
}

/// Who said the message a search for `from:@handle` found, when it was
/// that handle who said it, or none when the search found nothing of
/// theirs.
pub fn speaker(said: &Value, handle: &str) -> Result<Option<String>, Refused> {
    if said.get("ok").and_then(Value::as_bool) != Some(true) {
        // Refusals read as a look-up's do: signed out, or try later.
        return found(said, "", "").map(|_| None);
    }
    let matches = said.pointer("/messages/matches").and_then(Value::as_array);
    Ok(matches.into_iter().flatten().find_map(|message| {
        let by = message.get("username").and_then(Value::as_str)?;
        let id = message.get("user").and_then(Value::as_str)?;
        by.eq_ignore_ascii_case(handle).then(|| id.to_owned())
    }))
}

/// The address a profile Slack gave keeps, as written in lower case.
pub fn addressed(said: &Value) -> Option<String> {
    said.pointer("/user/profile/email")
        .and_then(Value::as_str)
        .map(str::to_lowercase)
}

/// Their status as a profile Slack gave keeps it, while it's set: its
/// words, and its emoji as Unicode (VKCF3Z, 67f6cbac). `now` is in seconds
/// since the epoch, as Slack says when a status ends, 0 for never.
pub fn status(profile: &Value, now: u64) -> (String, String) {
    let until = profile
        .get("status_expiration")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if until != 0 && until <= now {
        return (String::new(), String::new());
    }
    let words = profile
        .get("status_text")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    let emoji = profile
        .pointer("/status_emoji_display_info/0/unicode")
        .and_then(Value::as_str)
        .and_then(unicode)
        .unwrap_or_default();
    (words.to_owned(), emoji)
}

/// An emoji as Slack writes its code points, in hex joined by `-`, or
/// none when one isn't a character.
fn unicode(points: &str) -> Option<String> {
    points
        .split('-')
        .map(|point| u32::from_str_radix(point, 16).ok().and_then(char::from_u32))
        .collect()
}

/// Seconds since the epoch, as Slack keeps when a status ends.
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// What Slack's answer to a look-up says.
pub fn found(said: &Value, email: &str, team: &str) -> Result<Found, Refused> {
    let nobody = || Found {
        email: email.to_owned(),
        id: None,
        team: team.to_owned(),
        name: String::new(),
        zone: String::new(),
        presence: String::new(),
        status: String::new(),
        emoji: String::new(),
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
        let (status, emoji) = status(user.get("profile").unwrap_or(&Value::Null), now());
        // In a huddle says more than active, so it's said in its place.
        let huddle = text("/profile/huddle_state") == Some("in_a_huddle");
        return Ok(Found {
            email: email.to_owned(),
            id: Some(id.to_owned()),
            team: text("/team_id").unwrap_or(team).to_owned(),
            name: name.to_owned(),
            zone: text("/tz").unwrap_or_default().to_owned(),
            presence: if huddle { "in a huddle" } else { "" }.to_owned(),
            status,
            emoji,
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
            "real_name": "Ada L", "tz": "Europe/London",
            "profile": { "real_name": "Ada Lovelace" } } });
        assert_eq!(
            found(&said, "ada@example.com", "T1"),
            Ok(Found {
                email: "ada@example.com".into(),
                id: Some("U1".into()),
                team: "T9".into(),
                name: "Ada Lovelace".into(),
                zone: "Europe/London".into(),
                presence: String::new(),
                status: String::new(),
                emoji: String::new(),
            })
        );
    }

    #[test]
    fn a_status_is_its_words_and_its_emoji_while_it_lasts() {
        let palm = json!({ "status_text": "Away on the coast ", "status_expiration": 2000,
            "status_emoji_display_info": [{ "emoji_name": "palm_tree", "unicode": "1f334" }] });
        assert_eq!(
            status(&palm, 1000),
            ("Away on the coast".into(), "🌴".into())
        );
        assert_eq!(status(&palm, 2000), (String::new(), String::new()));
        let joined = json!({ "status_text": "Cooking", "status_expiration": 0,
            "status_emoji_display_info": [{ "unicode": "1f469-200d-1f373" }] });
        assert_eq!(
            status(&joined, u64::MAX),
            ("Cooking".into(), "👩\u{200d}🍳".into())
        );
        // A workspace's own picture has no Unicode form, so only the words.
        let own = json!({ "status_text": "Shipping", "status_emoji": ":ship-it:",
            "status_emoji_display_info": [{ "emoji_name": "ship-it",
                "display_url": "https://emoji.example/ship-it.png" }] });
        assert_eq!(status(&own, 0), ("Shipping".into(), String::new()));
        assert_eq!(status(&json!({}), 0), (String::new(), String::new()));
    }

    #[test]
    fn someone_in_a_huddle_is_said_so_in_place_of_active() {
        let said = json!({ "ok": true, "user": { "id": "U1",
            "profile": { "huddle_state": "in_a_huddle" } } });
        assert_eq!(
            found(&said, "a@example.com", "T1").unwrap().presence,
            "in a huddle"
        );
        let not = json!({ "ok": true, "user": { "id": "U1",
            "profile": { "huddle_state": "default_unset" } } });
        assert_eq!(found(&not, "a@example.com", "T1").unwrap().presence, "");
    }

    #[test]
    fn an_address_slack_knows_nobody_by_is_said_so() {
        let said = json!({ "ok": false, "error": "users_not_found" });
        let nobody = found(&said, "x@example.com", "T1").unwrap();
        assert_eq!(nobody.id, None);
        assert_eq!(nobody.team, "T1");
        assert_eq!(nobody.zone, "");
    }

    #[test]
    fn a_sign_in_slack_refuses_is_signed_out() {
        for error in ["invalid_auth", "token_revoked"] {
            let said = json!({ "ok": false, "error": error });
            assert_eq!(found(&said, "x@example.com", "T1"), Err(Refused::SignedOut));
        }
    }

    #[test]
    fn a_search_names_whoever_said_it_only_by_their_own_handle() {
        let said = json!({ "ok": true, "messages": { "matches": [
            { "user": "U7", "username": "someone.else" },
            { "user": "U2", "username": "Grace.Hopper" }] } });
        assert_eq!(speaker(&said, "grace.hopper"), Ok(Some("U2".into())));
        let none = json!({ "ok": true, "messages": { "matches": [] } });
        assert_eq!(speaker(&none, "grace.hopper"), Ok(None));
        let refused = json!({ "ok": false, "error": "invalid_auth" });
        assert_eq!(speaker(&refused, "grace.hopper"), Err(Refused::SignedOut));
    }

    #[test]
    fn only_a_grids_refusal_of_the_sign_in_turns_to_searching() {
        for error in ["not_allowed_token_type", "enterprise_is_restricted"] {
            assert!(refused_to_the_sign_in(
                &json!({ "ok": false, "error": error })
            ));
        }
        for error in ["users_not_found", "invalid_auth", "ratelimited"] {
            assert!(!refused_to_the_sign_in(
                &json!({ "ok": false, "error": error })
            ));
        }
        assert_eq!(
            addressed(
                &json!({ "ok": true, "user": { "profile": { "email": "Grace@Example.com" } } })
            ),
            Some("grace@example.com".into())
        );
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
