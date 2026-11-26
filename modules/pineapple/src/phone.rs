//! The safe word's alert on Joel's phone: the same alert the launcher
//! shows, as a message in his own note-to-self DM in Slack, which only he
//! sees.
//!
//! It is the one exception to never sending in Slack besides the replier,
//! and Joel's pick 5 (2026-11-13) holds it this narrow. It goes only to
//! his own IM: [`OwnIm`] is the one thing [`tell`] sends to, and nothing
//! makes one but [`own_im`], which asks Slack for the IM with Joel alone
//! and has Slack's conversations.info say it's an IM and his before it's
//! trusted, so no caller can name a channel. A reader over a tool gateway
//! sends by [`tell_self`], through the gateway's one tool that sends a
//! message from Joel to himself and can name nobody else. Either says only
//! the alert's fixed form, [`text`]: who stopped what, the words said, and
//! where. And it sends from the safe word's own reader, never from an
//! agent or a tool.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use serde_json::{Value, json};
use slack_people::{Account, Slack};
use ureq::Agent;

use crate::mcp::{Called, Gateway, never_connected};

/// How long Slack has to answer before it's given up on.
const PATIENCE: Duration = Duration::from_secs(10);

/// Slack's method that names the IM with the people given, here Joel
/// alone, which is his note-to-self. It's asked never to make one, so it
/// posts and makes nothing, but it is a write-scoped method all the same:
/// an IM Joel has closed it marks open again, back in his sidebar
/// (KP0L6A). It isn't one of slack-people's reads, so it's asked here.
/// Not users.conversations, which lists every IM: an Enterprise Grid
/// sign-in such as Joel's is refused it (`enterprise_is_restricted`, the
/// reader's first start against real Slack, 2026-11-14).
const OPEN: &str = "conversations.open";

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

/// How many characters of the words said are shown, so a long message
/// doesn't flood Joel's phone; the link has the rest. A row's words are
/// already its gist, cut shorter in tell.rs, so this guards only words
/// that came some other way, such as a line written before rows were cut.
const WORDS_SHOWN: usize = 300;

/// How every alert begins, so the reader knows its own when a search
/// finds one in Joel's IM.
const OPENING: &str = "pineapple: ";

/// Whether `text`, written as Joel, is one of these alerts: it quotes what
/// others said, so the reader must never judge it as his own saying, or
/// words made to slip out of its quotation marks would resume as him.
#[must_use]
pub fn is_alert(text: &str) -> bool {
    text.starts_with(OPENING)
}

/// The alert's one form, the same the launcher's daemon writes down as
/// what it would send. What others wrote is escaped as Slack's formatting
/// asks (&, < and >), so none of it is drawn as a masked link, a mention
/// or `<!here>` in Joel's own IM (KP0L6A), and the words are cut short.
#[must_use]
pub fn text(alert: Alert) -> String {
    let mut words: String = alert.words.chars().take(WORDS_SHOWN).collect();
    if alert.words.chars().nth(WORDS_SHOWN).is_some() {
        words.push_str("...");
    }
    format!(
        "{OPENING}{}\nSaid: \"{}\"\nWhere: {}",
        escaped(alert.title),
        escaped(&words),
        escaped(alert.link)
    )
}

/// Text as Slack draws it plainly: its three control characters escaped.
fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
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

/// Finds Joel's own IM: the one Slack names for Joel alone, never made
/// if it isn't there, which conversations.info then confirms is an IM
/// and his.
///
/// # Errors
///
/// Why it wasn't found or wasn't confirmed.
pub fn own_im(origin: &str, slack: &Slack, account: &Account) -> Result<OwnIm, String> {
    if account.user.is_empty() {
        return Err("the sign-in doesn't say who Joel is".to_owned());
    }
    let mut answer = agent()
        .get(format!("{origin}/api/{OPEN}"))
        .query("users", &account.user)
        .query("prevent_creation", "true")
        .query("return_im", "true")
        .header("Authorization", &format!("Bearer {}", account.token))
        .header("Cookie", &format!("d={}", account.cookie))
        .call()
        .map_err(|error| format!("couldn't reach Slack: {error}"))?;
    let named: Value = answer
        .body_mut()
        .read_json()
        .map_err(|error| format!("Slack's answer for Joel's own IM couldn't be read: {error}"))?;
    refused(&named)?;
    let id = picked(&named, &account.user).ok_or("Slack named no IM of Joel's own")?;
    let info = slack
        .read(account, "conversations.info", &[("channel", &id)])
        .map_err(|why| format!("Slack wouldn't say what {id} is: {why:?}"))?;
    if confirmed(&info, &account.user) {
        Ok(OwnIm(id))
    } else {
        Err(format!("{id} isn't Joel's own IM, as Slack tells it"))
    }
}

/// Why an alert didn't go, as far as can be told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Missed {
    /// It never went, or Slack said no: trying again can't say it twice.
    NotSent(String),
    /// It may have gone, as when Slack's answer timed out or couldn't be
    /// read: trying again may say it twice.
    Unsure(String),
}

/// Sends one alert to Joel's own IM, unfurling nothing under it, and
/// gives the text that went.
///
/// # Errors
///
/// Why it didn't go, and whether it may have all the same.
pub fn tell(origin: &str, account: &Account, own: &OwnIm, alert: Alert) -> Result<String, Missed> {
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
        .map_err(|error| {
            let why = format!("couldn't reach Slack: {error}");
            // Only a connection never made is sure not to have posted.
            if never_connected(&error) {
                Missed::NotSent(why)
            } else {
                Missed::Unsure(why)
            }
        })?;
    let status = answer.status().as_u16();
    let took: Value = answer
        .body_mut()
        .read_json()
        .map_err(|error| Missed::Unsure(format!("Slack answered {status}: {error}")))?;
    refused(&took).map_err(|why| Missed::NotSent(format!("Slack answered {status}: {why}")))?;
    Ok(said)
}

/// The gateway's tool that sends a message from Joel to himself, which
/// takes the message alone and can't be given anyone else to send to.
const TO_SELF: &str = "slack_send_message_to_self";

/// What's said in place of the words when the gateway won't take them.
const WORDS_NOT_TAKEN: &str = "(the words are at the link)";

/// Sends one alert to Joel from himself through `gateway`. It refuses a
/// mention, so `@here`, `@channel` and `@everyone` in what others wrote
/// are said as `(at)here` and so on; any other refusal of what they wrote
/// is met by sending it once more with the words left to the link, so
/// the title and where still reach his phone. A rate spent is no refusal
/// of the words, so it's tried again whole the next round.
///
/// The tool's failing promises nothing of whether Slack took the message,
/// so a second failing is counted as may have gone: two such rounds and
/// it's given up on, rather than sent twice a round for ten minutes.
/// It gives the text that went, its mentions said otherwise or its words
/// left to the link as they were.
///
/// # Errors
///
/// Why it didn't go, and whether it may have all the same.
pub fn tell_self(gateway: &mut Gateway, alert: Alert) -> Result<String, Missed> {
    let ask = |gateway: &mut Gateway, alert| {
        let said = unmentioned(&text(alert));
        gateway
            .call(TO_SELF, &json!({ "message": &said }))
            .map(|_| said)
    };
    let sent = match ask(gateway, alert) {
        Err(Called::Failed(why)) if spent(&why) => Err(Called::NotReached(why)),
        Err(Called::Failed(_)) if !alert.words.is_empty() => {
            let plain = Alert {
                words: WORDS_NOT_TAKEN,
                ..alert
            };
            ask(gateway, plain)
        }
        sent => sent,
    };
    sent.map_err(|called| match called {
        Called::Failed(why) if spent(&why) => Missed::NotSent(why),
        Called::Unreachable(why) | Called::Unreadable(why) | Called::Failed(why) => {
            Missed::Unsure(why)
        }
        Called::NotReached(why) => Missed::NotSent(why),
        Called::Refused => Missed::NotSent("the gateway refused Joel's sign-in".to_owned()),
        Called::NoSignIn(_) => Missed::NotSent("no sign-in to the gateway was given".to_owned()),
    })
}

/// Whether a tool's failing says Slack's rate was spent, which did nothing.
fn spent(why: &str) -> bool {
    let why = why.to_ascii_lowercase();
    why.contains("ratelimited") || why.contains("rate limit")
}

/// The words that mention everyone said so they don't, as the gateway
/// refuses to send a mention: `@` before `here`, `channel` or `everyone`
/// as a whole word, in any case, is said as `(at)`, while an address such
/// as `ops@channel4.com` is left as written.
fn unmentioned(text: &str) -> String {
    let mut said = String::with_capacity(text.len());
    for (at, character) in text.char_indices() {
        let everyone = || {
            let rest = text[at + character.len_utf8()..].to_ascii_lowercase();
            ["here", "channel", "everyone"].iter().any(|word| {
                rest.strip_prefix(word)
                    .is_some_and(|after| !after.starts_with(|next: char| next.is_alphanumeric()))
            })
        };
        if character == '@' && everyone() {
            said.push_str("(at)");
        } else {
            said.push(character);
        }
    }
    said
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

/// The IM Slack named, if it says it's an IM whose user is `joel`; a
/// missing `is_im` is let through, since conversations.info confirms it.
fn picked(named: &Value, joel: &str) -> Option<String> {
    let channel = &named["channel"];
    let ours = channel["is_im"].as_bool() != Some(false) && channel["user"].as_str() == Some(joel);
    channel["id"].as_str().filter(|_| ours).map(str::to_owned)
}

/// Whether conversations.info says it's an IM, and Joel's own.
fn confirmed(info: &Value, joel: &str) -> bool {
    let channel = &info["channel"];
    channel["is_im"].as_bool() == Some(true) && channel["user"].as_str() == Some(joel)
}

/// What phoned.jsonl keeps for an alert that may have gone, whose answer
/// was lost.
pub const UNSURE: &str = "unsure";

/// How many sends an alert gets whose answers were lost, so one that
/// landed unheard reaches Joel's phone twice at most (KP0L6A).
const UNSURE_TRIES: usize = 2;

/// The alerts done with, by [`told_as`], from phoned.jsonl's lines: told,
/// written down in shadow, or sent as often as an unsure one may be.
#[must_use]
pub fn done(phoned: &[Value]) -> HashSet<String> {
    let mut unsure: HashMap<String, usize> = HashMap::new();
    let mut done = HashSet::new();
    for line in phoned {
        if line["sent"] == UNSURE {
            *unsure.entry(told_as(line)).or_default() += 1;
        } else {
            done.insert(told_as(line));
        }
    }
    done.extend(
        unsure
            .into_iter()
            .filter(|(_, tries)| *tries >= UNSURE_TRIES)
            .map(|(told, _)| told),
    );
    done
}

/// What an alert is told as, its row and the saying it shows, as the
/// launcher's daemon tells it.
#[must_use]
pub fn told_as(line: &Value) -> String {
    format!(
        "{}@{}",
        line["key"].as_str().unwrap_or_default(),
        line["saying"].as_str().unwrap_or_default()
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const JOEL: &str = "U0JOEL0001";

    fn phoned(key: &str, sent: &Value) -> Value {
        json!({"key": key, "saying": "stopped", "sent": sent})
    }

    #[test]
    fn an_alert_whose_answer_was_lost_is_sent_once_more_and_no_more() {
        let once = [phoned("a", &json!(UNSURE))];
        assert!(done(&once).is_empty(), "one unsure send is tried again");
        let twice = [phoned("a", &json!(UNSURE)), phoned("a", &json!(UNSURE))];
        assert!(
            done(&twice).contains("a@stopped"),
            "a second unsure send is the last"
        );
        let told = [phoned("b", &json!(true)), phoned("c", &json!(false))];
        assert_eq!(
            done(&told).len(),
            2,
            "one told, and one written down in shadow, are done"
        );
    }

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
    fn what_others_wrote_is_never_drawn_as_slacks_markup() {
        let said = text(Alert {
            title: "Someone stopped your agents on <!here>",
            words: "pineapple override <https://evil.example|your PR> & <@U0OTHER001>",
            link: "https://example.slack.com/archives/C1/p1?thread_ts=1&cid=C1",
        });
        assert_eq!(
            said,
            "pineapple: Someone stopped your agents on &lt;!here&gt;\n\
             Said: \"pineapple override &lt;https://evil.example|your PR&gt; &amp; \
             &lt;@U0OTHER001&gt;\"\n\
             Where: https://example.slack.com/archives/C1/p1?thread_ts=1&amp;cid=C1",
            "a masked link, a mention and <!here> come out escaped"
        );
    }

    #[test]
    fn a_mention_of_everyone_is_said_as_at() {
        assert_eq!(
            unmentioned("pineapple override @here and @Channel, @everyone; mail joel@example.com"),
            "pineapple override (at)here and (at)Channel, (at)everyone; mail joel@example.com",
            "each mention of everyone, in any case, and nothing else"
        );
        assert_eq!(
            unmentioned("ends with @"),
            "ends with @",
            "an @ at the end is left"
        );
        assert_eq!(
            unmentioned("mail ops@channel4.com or @hereford, but @here."),
            "mail ops@channel4.com or @hereford, but (at)here.",
            "only the whole word, so an address or a longer name is left"
        );
    }

    #[test]
    fn only_a_rate_spent_is_said_as_spent() {
        assert!(spent("Slack said: ratelimited"), "Slack's own word");
        assert!(spent("Rate limit exceeded"), "in other words");
        assert!(!spent("Slack said: not_allowed"), "a refusal is no rate");
    }

    #[test]
    fn long_words_are_cut_short() {
        let long = "a".repeat(WORDS_SHOWN + 50);
        let said = text(Alert {
            title: "t",
            words: &long,
            link: "l",
        });
        let shown = format!("Said: \"{}...\"", "a".repeat(WORDS_SHOWN));
        assert!(
            said.contains(&shown),
            "cut to {WORDS_SHOWN} characters: {said}"
        );
    }

    #[test]
    fn only_an_im_whose_user_is_joel_is_picked() {
        let own = json!({"ok": true, "channel": {"id": "D0SELF0001", "is_im": true, "user": JOEL}});
        assert_eq!(
            picked(&own, JOEL).as_deref(),
            Some("D0SELF0001"),
            "his own IM is picked"
        );
        let theirs = json!({"ok": true, "channel": {"id": "D0OTHER001", "is_im": true,
            "user": "U0OTHER001"}});
        assert_eq!(picked(&theirs, JOEL), None, "nobody else's IM");
        let channel =
            json!({"ok": true, "channel": {"id": "C0ONE00001", "is_im": false, "user": JOEL}});
        assert_eq!(picked(&channel, JOEL), None, "nor a channel");
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
