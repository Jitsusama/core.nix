//! The safe word's alert on Joel's phone: the same alert the launcher
//! shows, as a message in his own note-to-self DM in Slack, which only he
//! sees.
//!
//! It is the one exception to never sending in Slack besides the replier,
//! and Joel's pick 5 (2026-11-13) holds it this narrow. It goes only to
//! his own IM: [`OwnIm`] is the one thing [`tell`] sends to, and nothing
//! makes one but [`own_im`], which finds the IM whose user is Joel among
//! his IMs and has Slack's conversations.info say so before it's trusted,
//! so no caller can name a channel. It says only the alert's fixed form,
//! [`text`]: who stopped what, the words said, and where. And it sends
//! from the safe word's own reader, never from an agent or a tool.

use std::time::Duration;

use serde_json::Value;
use slack_people::{Account, Slack};
use ureq::Agent;

/// How long Slack has to answer before it's given up on.
const PATIENCE: Duration = Duration::from_secs(10);

/// Slack's method that lists a person's conversations, here only IMs.
/// It only reads, but isn't one of slack-people's, so it's asked here.
const LIST: &str = "users.conversations";

/// How many IMs one page of the list holds, at most.
const PAGE: &str = "200";

/// How many pages are read before giving up on finding Joel's own IM:
/// four thousand IMs, far past anybody's.
const PAGES: usize = 20;

/// Slack's method that posts a message.
const POST: &str = "chat.postMessage";

/// One alert, as the launcher titles it.
#[derive(Debug, Clone, Copy)]
pub struct Alert<'a> {
    /// What the launcher's row says, as "Someone stopped your agents on
    /// pull request example/shop#12".
    pub title: &'a str,
    /// The words that were said.
    pub words: &'a str,
    /// Where they were said.
    pub link: &'a str,
}

/// The alert's one form, the same the launcher's daemon writes down as
/// what it would send.
#[must_use]
pub fn text(alert: Alert) -> String {
    format!(
        "pineapple: {}\nSaid: \"{}\"\nWhere: {}",
        alert.title, alert.words, alert.link
    )
}

/// Joel's own note-to-self IM, as Slack confirmed it: made only by
/// [`own_im`], and the only place [`tell`] sends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnIm(String);

impl OwnIm {
    /// The IM's id, to write down where an alert went.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.0
    }
}

/// Finds Joel's own IM: the one among his IMs whose user is him, which
/// conversations.info then confirms is an IM and his.
///
/// # Errors
///
/// Why it wasn't found or wasn't confirmed.
pub fn own_im(origin: &str, slack: &Slack, account: &Account) -> Result<OwnIm, String> {
    if account.user.is_empty() {
        return Err("the sign-in doesn't say who Joel is".to_owned());
    }
    let agent = agent();
    let mut cursor = String::new();
    for _ in 0..PAGES {
        let mut asked = vec![
            ("types", "im"),
            ("exclude_archived", "true"),
            ("limit", PAGE),
        ];
        if !cursor.is_empty() {
            asked.push(("cursor", cursor.as_str()));
        }
        let mut request = agent.get(format!("{origin}/api/{LIST}"));
        for (name, value) in asked {
            request = request.query(name, value);
        }
        let mut answer = request
            .header("Authorization", &format!("Bearer {}", account.token))
            .header("Cookie", &format!("d={}", account.cookie))
            .call()
            .map_err(|error| format!("couldn't reach Slack: {error}"))?;
        let listed: Value = answer
            .body_mut()
            .read_json()
            .map_err(|error| format!("Slack's list of IMs couldn't be read: {error}"))?;
        refused(&listed)?;
        if let Some(id) = picked(&listed, &account.user) {
            let info = slack
                .read(account, "conversations.info", &[("channel", &id)])
                .map_err(|why| format!("Slack wouldn't say what {id} is: {why:?}"))?;
            return if confirmed(&info, &account.user) {
                Ok(OwnIm(id))
            } else {
                Err(format!("{id} isn't Joel's own IM, as Slack tells it"))
            };
        }
        listed["response_metadata"]["next_cursor"]
            .as_str()
            .unwrap_or_default()
            .clone_into(&mut cursor);
        if cursor.is_empty() {
            break;
        }
    }
    Err("none of Joel's IMs is his own".to_owned())
}

/// Sends one alert to Joel's own IM, unfurling nothing under it.
///
/// # Errors
///
/// Why Slack didn't take it.
pub fn tell(origin: &str, account: &Account, own: &OwnIm, alert: Alert) -> Result<(), String> {
    let said = text(alert);
    let mut answer = agent()
        .post(format!("{origin}/api/{POST}"))
        .header("Authorization", &format!("Bearer {}", account.token))
        .header("Cookie", &format!("d={}", account.cookie))
        .send_form([
            ("channel", own.id()),
            ("text", said.as_str()),
            ("unfurl_links", "false"),
            ("unfurl_media", "false"),
        ])
        .map_err(|error| format!("couldn't reach Slack: {error}"))?;
    let status = answer.status().as_u16();
    let took: Value = answer
        .body_mut()
        .read_json()
        .map_err(|error| format!("Slack answered {status}: {error}"))?;
    refused(&took).map_err(|why| format!("Slack answered {status}: {why}"))
}

fn agent() -> Agent {
    Agent::config_builder()
        .timeout_global(Some(PATIENCE))
        .http_status_as_error(false)
        .build()
        .into()
}

/// Why Slack said no, where it did.
fn refused(said: &Value) -> Result<(), String> {
    if said["ok"].as_bool() == Some(true) {
        Ok(())
    } else {
        Err(said["error"].as_str().unwrap_or("no reason").to_owned())
    }
}

/// The IM in a page of the list whose user is `joel`, if there is one.
fn picked(listed: &Value, joel: &str) -> Option<String> {
    listed["channels"]
        .as_array()?
        .iter()
        .find(|one| one["is_im"].as_bool() != Some(false) && one["user"].as_str() == Some(joel))
        .and_then(|one| one["id"].as_str())
        .map(str::to_owned)
}

/// Whether conversations.info says it's an IM, and Joel's own.
fn confirmed(info: &Value, joel: &str) -> bool {
    let channel = &info["channel"];
    channel["is_im"].as_bool() == Some(true) && channel["user"].as_str() == Some(joel)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const JOEL: &str = "U0JOEL0001";

    #[test]
    fn the_alert_says_who_stopped_what_the_words_and_where() {
        let said = text(Alert {
            title: "Someone stopped your agents on pull request example/shop#12",
            words: "pineapple override",
            link: "https://example.slack.com/archives/C1/p1",
        });
        assert_eq!(
            said,
            "pineapple: Someone stopped your agents on pull request example/shop#12\n\
             Said: \"pineapple override\"\n\
             Where: https://example.slack.com/archives/C1/p1"
        );
    }

    #[test]
    fn only_the_im_whose_user_is_joel_is_picked() {
        let listed = json!({"ok": true, "channels": [
            {"id": "D0OTHER001", "is_im": true, "user": "U0OTHER001"},
            {"id": "D0SELF0001", "is_im": true, "user": JOEL},
        ]});
        assert_eq!(picked(&listed, JOEL).as_deref(), Some("D0SELF0001"));
        let others = json!({"ok": true, "channels": [
            {"id": "D0OTHER001", "is_im": true, "user": "U0OTHER001"},
            {"id": "C0ONE00001", "is_im": false, "user": JOEL},
        ]});
        assert_eq!(
            picked(&others, JOEL),
            None,
            "nobody else's IM, nor a channel"
        );
    }

    #[test]
    fn slack_must_say_it_is_an_im_and_joels() {
        let own = json!({"ok": true, "channel": {"id": "D0SELF0001", "is_im": true, "user": JOEL}});
        assert!(confirmed(&own, JOEL), "Joel's own IM is confirmed");
        let theirs = json!({"ok": true, "channel": {"id": "D0OTHER001", "is_im": true,
            "user": "U0OTHER001"}});
        assert!(!confirmed(&theirs, JOEL), "someone else's IM isn't");
        let channel = json!({"ok": true, "channel": {"id": "C0ONE00001", "is_channel": true}});
        assert!(!confirmed(&channel, JOEL), "a channel isn't");
        let refused = json!({"ok": false, "error": "channel_not_found"});
        assert!(!confirmed(&refused, JOEL), "nor is what Slack won't say");
    }
}
