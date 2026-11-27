//! Joel's phone, told each alert the launcher's daemon writes down, in its
//! one form, in his own note-to-self IM in Slack and nowhere else
//! (src/phone.rs), once each, kept in phoned.jsonl: by the reader over
//! Slack's own API in the IM it finds for him alone, or by the reader over
//! a tool gateway through its one tool that sends to Joel himself.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use pineapple::mcp::Gateway;
use pineapple::phone::{self, Alert, Missed, OwnIm, UNSURE, done, told_as};
use pineapple::record;
use serde_json::{Value, json};
use slack_people::{Account, Slack};

use crate::{LOOK_BACK, MILLIS, millis};

/// What phoned.jsonl names as where an alert went through a gateway: his
/// own IM, which the gateway finds and never names.
const TO_SELF: &str = "self";

/// What phoned.jsonl names an alert as when it went unsent for its ten
/// minutes and is let go.
const GIVEN_UP: &str = "given-up";

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
    let new = untold(state);
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
    let Phone {
        origin,
        own,
        shadow,
        ..
    } = &*phone;
    let im = own.as_ref().map(OwnIm::id);
    tell_each(*shadow, state, new, im, |alert| {
        own.as_ref()
            .map(|own| phone::tell(origin, account, own, alert))
    });
}

/// Tells Joel's phone, as [`tell`] does out of shadow, through `gateway`,
/// whose tool sends to him alone, so no IM is looked for. A reader over a
/// gateway in shadow tells no phone, so never calls it.
pub fn tell_through(gateway: &mut Gateway, state: &Path) {
    tell_each(false, state, untold(state), Some(TO_SELF), |alert| {
        Some(phone::tell_self(gateway, alert))
    });
}

/// The alerts the launcher's daemon wrote down in the last ten minutes
/// that Joel's phone wasn't told yet.
fn untold(state: &Path) -> Vec<Value> {
    let told = done(&lines(&state.join("phoned.jsonl")));
    let since = millis().saturating_sub(LOOK_BACK * MILLIS);
    lines(&state.join("would-tell.jsonl"))
        .into_iter()
        .filter(|line| line["at"].as_u64().unwrap_or(0) >= since && !told.contains(&told_as(line)))
        .collect()
}

impl Phone {
    /// Says each live alert let go, as [`give_up`] does, naming Joel's own
    /// IM where it's found; in shadow, none, as none is tried.
    pub fn give_up(&self, state: &Path, from: u64) {
        if !self.shadow {
            give_up(state, self.own.as_ref().map(OwnIm::id), from);
        }
    }
}

/// Says each live alert let go, as [`give_up`] does, for a reader telling
/// Joel's phone through a gateway.
pub fn give_up_through(state: &Path, from: u64) {
    give_up(state, Some(TO_SELF), from);
}

/// The oldest an alert may be and still be said let go, taken once as a
/// reader starts: twenty minutes before phoned.jsonl's newest line, kept
/// as a reader told Joel's phone, wrote an alert down or let one go, so
/// one written while no reader ran, or aging out in the last round before
/// one stopped, is said too, while one from before alerts were ever let
/// go with a word is left as it was. A reader that never kept a line
/// takes twenty minutes back, and the earlier of the two is taken, so a
/// clock set back can't move it ahead.
pub fn let_go_from(state: &Path) -> u64 {
    let back = 2 * LOOK_BACK * MILLIS;
    let fresh = millis().saturating_sub(back);
    lines(&state.join("phoned.jsonl"))
        .iter()
        .filter_map(|line| line["at"].as_u64())
        .max()
        .map_or(fresh, |last| fresh.min(last.saturating_sub(back)))
}

/// Says each live alert that went ten minutes untold is let go, as it
/// won't be tried again, and keeps a line of it in phoned.jsonl so it's
/// said once. It's asked each round before any is told, whether or not
/// Slack or the gateway answers, so one aging out in an outage is said
/// too; only those written before `from`, as [`let_go_from`] gives it,
/// go unsaid. One whose sends may have gone is said so, those left after
/// their last such send included. An alert the daemon wrote down twice is
/// said once.
fn give_up(state: &Path, im: Option<&str>, from: u64) {
    let phoned = state.join("phoned.jsonl");
    let kept = lines(&phoned);
    let mut unsure: HashMap<String, usize> = HashMap::new();
    let mut told = HashSet::new();
    for line in &kept {
        if line["sent"] == UNSURE {
            *unsure.entry(told_as(line)).or_default() += 1;
        } else {
            told.insert(told_as(line));
        }
    }
    let now = millis();
    let since = now.saturating_sub(LOOK_BACK * MILLIS);
    for line in lines(&state.join("would-tell.jsonl")) {
        let at = line["at"].as_u64().unwrap_or(0);
        let named = told_as(&line);
        if at < from || at >= since || line["shadow"] != false || told.contains(&named) {
            continue;
        }
        let link = line["link"].as_str().unwrap_or_default();
        if let Some(sends) = unsure.get(&named) {
            let sends = if *sends == 1 {
                "its one send"
            } else {
                "each send"
            };
            eprintln!(
                "<3>Joel's phone alert is let go after ten minutes, {sends} that may have gone \
                 unanswered: {link}"
            );
        } else {
            eprintln!("<3>Joel's phone alert is let go, untold after ten minutes: {link}");
        }
        let one = json!({
            "at": now,
            "key": line["key"],
            "saying": line["saying"],
            "shadow": false,
            "sent": GIVEN_UP,
            "im": im,
        });
        if let Err(error) = record::append(&phoned, &one) {
            eprintln!("<3>couldn't keep a phone alert's line: {error}");
        }
        told.insert(named);
    }
}

/// Tells each of `new` by `tell`, which says nothing where there's
/// nowhere yet to send, and keeps a line in phoned.jsonl for each told,
/// written down in shadow, or that may have gone, as `im` names where:
/// with the text that went, or for one unsent or unsure, its own form.
/// An alert the daemon wrote down twice is tried once a round, so it's
/// sent once and its sends that may have gone are counted by round. Its
/// first line stands for both: the daemon writes each from the row its
/// saying names, shadow and all, so lines of one key and saying differ
/// at most in a title whose words were changed between them.
fn tell_each(
    shadow: bool,
    state: &Path,
    new: Vec<Value>,
    im: Option<&str>,
    mut tell: impl FnMut(Alert) -> Option<Result<String, Missed>>,
) {
    let phoned = state.join("phoned.jsonl");
    let mut tried = HashSet::new();
    for line in new {
        if !tried.insert(told_as(&line)) {
            continue;
        }
        let field = |name: &str| line[name].as_str().unwrap_or_default().to_owned();
        let (title, words, link) = (field("title"), field("words"), field("link"));
        let alert = Alert {
            title: &title,
            words: &words,
            link: &link,
        };
        let shadow = shadow || line["shadow"] != false;
        let (sent, text) = if shadow {
            eprintln!("<5>Joel's phone, in shadow: {link}");
            (json!(false), phone::text(alert))
        } else {
            match tell(alert) {
                Some(Ok(said)) => {
                    eprintln!("<5>Joel's phone was told: {link}");
                    (json!(true), said)
                }
                Some(Err(Missed::NotSent(why))) => {
                    eprintln!(
                        "<3>Joel's phone alert didn't go; it's tried each round while \
                         under ten minutes old: {why}"
                    );
                    continue;
                }
                Some(Err(Missed::Unsure(why))) => {
                    eprintln!("<4>Joel's phone alert may have gone: {why}");
                    (json!(UNSURE), phone::text(alert))
                }
                None => continue,
            }
        };
        let one = json!({
            "at": millis(),
            "key": line["key"],
            "saying": line["saying"],
            "shadow": shadow,
            "sent": sent,
            "im": im,
            "text": text,
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
