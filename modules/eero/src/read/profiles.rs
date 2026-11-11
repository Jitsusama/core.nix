//! The profiles a household groups its devices under, each with its
//! filters, and the profile a device's record names.

use super::{flag, id_of, list, text, texts, usage};
use crate::model::{Filters, Member, Profile};
use serde_json::Value;
use std::collections::BTreeMap;

/// A device's profile, which eero gives as the profile's record or url.
pub(super) fn profile_id(profile: Option<&Value>) -> Option<String> {
    match profile? {
        Value::String(url) => id_of(url),
        profile => text(profile, "/url").and_then(|url| id_of(&url)),
    }
}

pub(super) fn profiles(answer: &Value, usages: &BTreeMap<String, Value>) -> Vec<Profile> {
    list(answer, "")
        .filter_map(|profile| {
            let id = id_of(&text(profile, "/url")?)?;
            Some(Profile {
                name: text(profile, "/name"),
                state: text(profile, "/state/value"),
                paused: flag(profile, "/paused"),
                devices: list(profile, "/devices")
                    .filter_map(|d| text(d, "/mac"))
                    .collect(),
                members: list(profile, "/devices")
                    .filter_map(|d| {
                        Some(Member {
                            mac: text(d, "/mac")?,
                            name: text(d, "/display_name"),
                            kind: text(d, "/device_type"),
                        })
                    })
                    .collect(),
                default: flag(profile, "/default"),
                schedules: profile
                    .pointer("/schedule")
                    .and_then(Value::as_array)
                    .map(Vec::len),
                filters: filters(profile),
                usage: usages.get(&id).and_then(usage),
                id,
            })
        })
        .collect()
}

/// What a profile blocks. eero gives its content filters twice: the
/// unified ones, every kind, and the older Secure ones, four of them, so
/// a kind the unified ones don't say is taken from the older.
pub(super) fn filters(profile: &Value) -> Filters {
    let policy = |kind: &str| {
        flag(
            profile,
            &format!("/unified_content_filters/dns_policies/{kind}"),
        )
        .or_else(|| flag(profile, &format!("/premium_dns/dns_policies/{kind}")))
    };
    let premium = |at: &str| format!("/premium_dns/{at}");
    Filters {
        set: flag(profile, "/unified_content_filters/is_content_filters_set"),
        gaming: policy("block_gaming_content"),
        illegal: policy("block_illegal_content"),
        messaging: policy("block_messaging_content"),
        pornographic: policy("block_pornographic_content"),
        social: policy("block_social_content"),
        shopping: policy("block_shopping_content"),
        streaming: policy("block_streaming_content"),
        violent: policy("block_violent_content"),
        safe_search: policy("safe_search_enabled"),
        youtube_restricted: policy("youtube_restricted"),
        ad_block: flag(profile, &premium("ad_block_settings/enabled")),
        ad_block_network: flag(profile, &premium("ad_block_settings/enabled_for_network")),
        blocked_sites: texts(profile, &premium("advanced_content_filters/blocked_list")),
        allowed_sites: texts(profile, &premium("advanced_content_filters/allowed_list")),
        blocked_apps: texts(profile, &premium("blocked_applications")),
    }
}
