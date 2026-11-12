//! Whether a chat program's notice is to Joel himself, so it's heard, or
//! one of the many he'd only like to know are there, so it's counted
//! without a card or a sound. Joel, 2026-11-12: "the slack sound is the
//! only way i notice, i don't like it, but I get major FOMO if I don't
//! know". Slack's desktop app says a direct message as "[<workspace>]
//! from <person>" and a channel's as "[<workspace>] in <channel>", its
//! words as "<person>: <text>", and the same for a post and a reply in a
//! thread. It sends a channel's notice only for a mention, a thread Joel
//! follows, or a channel he set to tell him every message, so outside
//! those channels each one is to him. The names, bots and channels are
//! the machine's to give, since core names no one.

use serde::Deserialize;

/// What's to Joel, for one sender, as the machine gives it.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Addressed {
    /// The names he's mentioned by, without the `@`, as his display name
    /// and his handle: a mention reads as one of them.
    pub names: Vec<String>,
    /// Those whose direct messages are a program's, not a person's.
    pub bots: Vec<String>,
    /// The channels he set to tell him every message, without the `#`.
    pub every_message: Vec<String>,
}

/// How a notice stands to him.
#[derive(Debug, PartialEq, Eq)]
pub enum To {
    /// It's to him: a person's direct message, a mention, or what a
    /// channel tells only for a thread he follows.
    Him,
    /// One of a channel's every message, or a program's direct one.
    Batched,
}

impl Addressed {
    /// How a notice from this sender stands, by its summary and words.
    /// One in a shape this doesn't know is to him, as staying silent on
    /// one he'd want costs more than hearing one he wouldn't.
    pub fn to(&self, summary: &str, body: &str) -> To {
        let Some((_, said)) = summary
            .strip_prefix('[')
            .and_then(|rest| rest.split_once("] "))
        else {
            return To::Him;
        };
        if let Some(from) = said.strip_prefix("from ") {
            return if self.bots.iter().any(|bot| bot == from) {
                To::Batched
            } else {
                To::Him
            };
        }
        let Some(channel) = said.strip_prefix("in ") else {
            return To::Him;
        };
        let channel = channel.trim_start_matches('#');
        if self.mentions(body) || !self.every_message.iter().any(|every| every == channel) {
            To::Him
        } else {
            To::Batched
        }
    }

    /// Whether its words name him, or everyone there with him.
    fn mentions(&self, body: &str) -> bool {
        let named = |name: &str| {
            body.match_indices(&format!("@{name}")).any(|(at, found)| {
                body[at + found.len()..]
                    .chars()
                    .next()
                    .is_none_or(|next| !(next.is_alphanumeric() || next == '.' || next == '_'))
            })
        };
        self.names.iter().any(|name| named(name)) || named("here") || named("channel")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use To::{Batched, Him};

    /// How a notice stands to Ada, who set two channels to tell her every
    /// message.
    fn to(summary: &str, body: &str) -> To {
        let ada = Addressed {
            names: vec!["ada".into(), "ada.l".into()],
            bots: vec!["Merge Bot".into()],
            every_message: vec!["busy".into(), "team chat".into()],
        };
        ada.to(summary, body)
    }

    #[test]
    fn a_persons_direct_message_is_to_him_and_a_bots_is_batched() {
        assert_eq!(to("[work] from Grace Hopper", "hi"), Him);
        assert_eq!(to("[work] from Merge Bot", "merged"), Batched);
    }

    #[test]
    fn a_channel_that_tells_every_message_is_batched_unless_it_names_him() {
        assert_eq!(to("[work] in #busy", "Grace: lunch?"), Batched);
        assert_eq!(to("[work] in team chat", "Grace: lunch?"), Batched);
        assert_eq!(to("[work] in #busy", "Grace: @ada lunch?"), Him);
        assert_eq!(to("[work] in #busy", "Grace: @ada.l, lunch?"), Him);
        assert_eq!(to("[work] in #busy", "Grace: @here lunch"), Him);
        assert_eq!(to("[work] in #busy", "Grace: @channel"), Him);
    }

    #[test]
    fn a_longer_name_beginning_with_his_is_not_his() {
        assert_eq!(to("[work] in #busy", "Grace: @adam lunch?"), Batched);
        assert_eq!(to("[work] in #busy", "Grace: @ada.lovelace"), Batched);
        assert_eq!(to("[work] in #busy", "Grace: @hereford"), Batched);
    }

    #[test]
    fn any_other_channels_notice_is_a_thread_he_follows_so_to_him() {
        assert_eq!(to("[work] in #quiet", "Grace: done"), Him);
    }

    #[test]
    fn a_shape_it_doesnt_know_is_to_him() {
        assert_eq!(to("Slack", "You have a new message"), Him);
        assert_eq!(to("[work] said something", "Grace: hi"), Him);
    }

    #[test]
    fn with_nothing_given_only_a_bots_message_could_be_batched_and_none_is() {
        let none = Addressed::default();
        assert_eq!(none.to("[work] in #busy", "Grace: lunch?"), Him);
        assert_eq!(none.to("[work] from Merge Bot", "merged"), Him);
    }
}
