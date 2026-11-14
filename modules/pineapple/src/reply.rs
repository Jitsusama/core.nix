//! The replier: what the safe word's reader says back in Slack, where,
//! and the one way it says it.
//!
//! Joel's rules (the plan's picks 6 and 7) hold it narrow. A reply goes
//! from Joel's own account, opening with a line saying it is automated
//! and not him, so no bot or app stands in for him. It says one of two
//! fixed things, what stopped or that Joel was asked, in words he said
//! yes to, and only in the thread of the message that asked: a [`Post`]
//! takes its conversation and thread from that message and from nothing
//! else. Its words are replies.json's, beside this crate, and a reply
//! with any of them missing is never said, so nothing goes out until his
//! yes is written there. Which replies go at all is src/gate.rs's call.

use std::time::Duration;

use serde_json::Value;
use slack_people::Account;
use ureq::Agent;

use crate::Reply;
use crate::slack::Message;
use crate::tell::said_reaches;
use crate::vocabulary::Words;

/// How long Slack has to answer a reply before it's given up on.
const PATIENCE: Duration = Duration::from_secs(10);

/// Slack's one method the replier asks, which posts a message.
pub const METHOD: &str = "chat.postMessage";

/// One reply, ready to say.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Post {
    /// The conversation the asking message is in.
    pub channel: String,
    /// The thread it's in, or the one it starts.
    pub thread_ts: String,
    /// What it says, its first line saying it is automated and not Joel.
    pub text: String,
}

/// The reply to a message that asked for one, or none for a reply that
/// says nothing or whose words aren't all written yet.
#[must_use]
pub fn post(message: &Message, reply: Reply, reaches: &[String], words: &Words) -> Option<Post> {
    let body = match reply {
        Reply::WhatStopped => "what-stopped",
        Reply::JoelAsked => "joel-asked",
        Reply::Nothing => return None,
    };
    let opening = words.fill("opening", &[]);
    let said = words.fill(body, &[("reach", &said_reaches(reaches, words))]);
    if opening.trim().is_empty() || said.trim().is_empty() {
        return None;
    }
    Some(Post {
        channel: message.channel.clone(),
        thread_ts: message.thread.clone().unwrap_or_else(|| message.ts.clone()),
        text: format!("{opening}\n{said}"),
    })
}

/// A reply as its line in the log keeps it.
#[must_use]
pub fn kept(post: Option<&Post>) -> Value {
    post.map_or(Value::Null, |post| {
        serde_json::json!({
            "channel": post.channel,
            "thread_ts": post.thread_ts,
            "text": post.text,
        })
    })
}

/// Say one reply in Slack, from the account given, at Slack's origin or a
/// test's stand-in. Nothing in it is unfurled, so a link it names draws
/// nothing more under it.
///
/// # Errors
///
/// Why Slack didn't take it.
pub fn send(origin: &str, account: &Account, post: &Post) -> Result<(), String> {
    let agent: Agent = Agent::config_builder()
        .timeout_global(Some(PATIENCE))
        .http_status_as_error(false)
        .build()
        .into();
    let mut answer = agent
        .post(format!("{origin}/api/{METHOD}"))
        .header("Authorization", &format!("Bearer {}", account.token))
        .header("Cookie", &format!("d={}", account.cookie))
        .send_form([
            ("channel", post.channel.as_str()),
            ("thread_ts", post.thread_ts.as_str()),
            ("text", post.text.as_str()),
            ("unfurl_links", "false"),
            ("unfurl_media", "false"),
        ])
        .map_err(|error| format!("couldn't reach Slack: {error}"))?;
    let status = answer.status().as_u16();
    let said: Value = answer
        .body_mut()
        .read_json()
        .map_err(|error| format!("Slack answered {status}: {error}"))?;
    if said.get("ok").and_then(Value::as_bool) == Some(true) {
        Ok(())
    } else {
        let why = said
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("no reason");
        Err(format!("Slack answered {status}: {why}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asking(thread: Option<&str>) -> Message {
        Message {
            channel: "C0ONE00001".to_owned(),
            ts: "1700000100.000100".to_owned(),
            thread: thread.map(str::to_owned),
            link: "https://example.slack.com/archives/C0ONE00001/p1700000100000100".to_owned(),
            author: "U0OTHER001".to_owned(),
            by_bot: false,
            edited: false,
            text: "<@U0JOEL0001> pineapple override".to_owned(),
        }
    }

    fn words() -> Words {
        Words::of(&[
            ("opening", "Automated, not Joel."),
            ("what-stopped", "Stopped: {reach}."),
            ("joel-asked", "Joel was asked."),
            ("reach:pr", "pull request {repo}#{number}"),
        ])
    }

    #[test]
    fn a_reply_goes_in_the_asking_messages_own_thread() {
        let post = post(
            &asking(Some("1700000000.000001")),
            Reply::WhatStopped,
            &["pr:example/shop#12".to_owned()],
            &words(),
        );
        assert_eq!(
            post,
            Some(Post {
                channel: "C0ONE00001".to_owned(),
                thread_ts: "1700000000.000001".to_owned(),
                text: "Automated, not Joel.\nStopped: pull request example/shop#12.".to_owned(),
            }),
            "it says what stopped, under its opening, where it was asked"
        );
    }

    #[test]
    fn a_message_in_no_thread_is_answered_in_the_one_it_starts() {
        let post = post(&asking(None), Reply::JoelAsked, &[], &words());
        assert_eq!(
            post.map(|post| post.thread_ts),
            Some("1700000100.000100".to_owned()),
            "the reply starts a thread under the message"
        );
    }

    #[test]
    fn nothing_is_said_without_every_word_written() {
        let message = asking(None);
        let reaches = ["everything".to_owned()];
        let none = Words::of(&[]);
        let no_opening = Words::of(&[("what-stopped", "Stopped.")]);
        assert_eq!(
            post(&message, Reply::WhatStopped, &reaches, &none),
            None,
            "no words, no reply"
        );
        assert_eq!(
            post(&message, Reply::WhatStopped, &reaches, &no_opening),
            None,
            "a reply never goes without the line saying it isn't Joel"
        );
        assert_eq!(
            post(&message, Reply::Nothing, &reaches, &words()),
            None,
            "a reply that says nothing isn't said"
        );
    }

    #[test]
    fn the_words_kept_beside_the_crate_say_nothing_until_joel_says_yes() {
        let kept = Words::from_object(include_str!("../replies.json")).expect("replies.json");
        let reaches = ["everything".to_owned()];
        assert_eq!(
            post(&asking(None), Reply::WhatStopped, &reaches, &kept),
            None,
            "until his yes is written in replies.json, no reply has words"
        );
    }
}
