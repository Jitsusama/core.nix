//! Joel's phone, told each alert the launcher's daemon writes down, in its
//! one form, in his own note-to-self IM in Slack and nowhere else
//! (src/phone.rs), once each, kept in phoned.jsonl. Only the reader over
//! Slack's own API tells it, as only that one holds a sign-in that writes.

use std::path::Path;

use pineapple::phone::{self, Alert, Missed, OwnIm, UNSURE, done, told_as};
use pineapple::record;
use serde_json::{Value, json};
use slack_people::{Account, Slack};

use crate::{LOOK_BACK, MILLIS, millis};

/// Where Joel's phone is told: his own IM, once it's found.
pub struct Phone {
    pub origin: String,
    pub own: Option<OwnIm>,
    /// Whether it has looked for the IM yet: once as it starts, so the
    /// finding is tried against Slack whether or not an alert comes, and
    /// then again only when an alert finds none.
    pub looked: bool,
    /// Whether alerts are only written down, as the reader is in shadow.
    pub shadow: bool,
}

/// Tells Joel's phone each alert the launcher's daemon wrote down in the
/// last ten minutes and his phone wasn't told yet. One Slack didn't take
/// is tried again next round, until it's ten minutes old.
pub fn tell(slack: &Slack, phone: &mut Phone, account: &Account, state: &Path) {
    let phoned = state.join("phoned.jsonl");
    let told = done(&lines(&phoned));
    let since = millis().saturating_sub(LOOK_BACK * MILLIS);
    let new: Vec<Value> = lines(&state.join("would-tell.jsonl"))
        .into_iter()
        .filter(|line| line["at"].as_u64().unwrap_or(0) >= since && !told.contains(&told_as(line)))
        .collect();
    if phone.own.is_none() && (!phone.looked || !new.is_empty()) {
        phone.looked = true;
        match phone::own_im(&phone.origin, slack, account) {
            Ok(own) => {
                eprintln!("<6>Joel's own IM is {}", own.id());
                phone.own = Some(own);
            }
            Err(why) => eprintln!("<4>Joel's own IM wasn't found, so his phone isn't told: {why}"),
        }
    }
    if new.is_empty() {
        return;
    }
    for line in new {
        let field = |name: &str| line[name].as_str().unwrap_or_default().to_owned();
        let (title, words, link) = (field("title"), field("words"), field("link"));
        let alert = Alert {
            title: &title,
            words: &words,
            link: &link,
        };
        let shadow = phone.shadow || line["shadow"] != false;
        let sent = match (&phone.own, shadow) {
            (_, true) => {
                eprintln!("<5>Joel's phone, in shadow: {link}");
                json!(false)
            }
            (Some(own), false) => match phone::tell(&phone.origin, account, own, alert) {
                Ok(()) => {
                    eprintln!("<5>Joel's phone was told: {link}");
                    json!(true)
                }
                Err(Missed::NotSent(why)) => {
                    eprintln!("<3>Slack didn't take Joel's phone alert: {why}");
                    continue;
                }
                Err(Missed::Unsure(why)) => {
                    eprintln!("<4>Joel's phone alert may have gone: {why}");
                    json!(UNSURE)
                }
            },
            (None, false) => continue,
        };
        let one = json!({
            "at": millis(),
            "key": line["key"],
            "saying": line["saying"],
            "shadow": shadow,
            "sent": sent,
            "im": phone.own.as_ref().map(OwnIm::id),
            "text": phone::text(alert),
        });
        if let Err(error) = record::append(&phoned, &one) {
            eprintln!("<3>couldn't keep a phone alert's line: {error}");
        }
    }
}

/// The lines of a log kept as JSON lines, those that read.
fn lines(path: &Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}
