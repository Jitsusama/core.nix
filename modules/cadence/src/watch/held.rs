//! The net held by one hold on the machine while merges keep landing
//! untested. The net's lag counts from a hold's end, so a hold that stands
//! for hours says nothing of its own: on 19 November holds about
//! concluding quests and Slack kept the net starting nothing from 15:26 to
//! past 18:35 while 21 merges landed, and nobody was told.

use super::{Breach, Snapshot, held};

/// How long one hold may keep the net starting nothing while merges land
/// untested, in minutes.
pub const STALLED_MIN: i64 = 30;

/// A hold on the machine as the net keeps it in `held.json`: when it first
/// found this hold standing, which it keeps while the same hold stands,
/// what the hold says, and who sent it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetHeld {
    pub since: i64,
    pub why: String,
    /// The mesh address of the session that sent the hold, when the net
    /// names one; the watcher is asked when it doesn't.
    pub from: Option<String>,
}

/// The hold in the net's `held.json`, once the net says since when. A file
/// written before the net kept `since` gives none, so it's read as it was.
#[must_use]
pub fn read(held: &str) -> Option<NetHeld> {
    let held: serde_json::Value = serde_json::from_str(held).ok()?;
    Some(NetHeld {
        since: held["since"].as_i64()?,
        why: held["why"].as_str().unwrap_or_default().to_owned(),
        from: held["from"]
            .as_str()
            .filter(|from| !from.is_empty())
            .map(str::to_owned),
    })
}

/// The net held by one hold past [`STALLED_MIN`] while main gained merges
/// since it began, said once for the hold, keyed by when the net first
/// found it and naming what it says. Only a hold the net still finds
/// counts.
pub fn stalled(s: &Snapshot) -> Option<Breach> {
    let hold = s.net_held_since.as_ref().filter(|_| held(s))?;
    let minutes = (s.now - hold.since) / 60;
    let merged = s
        .landings
        .iter()
        .filter(|landing| landing.merge.at > hold.since)
        .count();
    if minutes < STALLED_MIN || merged == 0 {
        return None;
    }
    let why: String = hold.why.chars().take(240).collect();
    let noun = if merged == 1 { "merge" } else { "merges" };
    Some(Breach {
        key: format!("netheld:{}", hold.since),
        text: format!(
            "the net has started nothing for {minutes} min while {merged} {noun} landed \
             untested, held by: {why}"
        ),
    })
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::watch::tests::{calm, merge};
    use crate::watch::{HELD_FRESH_S, Landing, breaches};

    #[test]
    fn a_hold_is_read_with_when_the_net_first_saw_it_and_who_sent_it() {
        assert_eq!(
            read("{\"at\":9,\"since\":1,\"why\":\"a reboot\",\"from\":\"P9R1KI\"}"),
            Some(NetHeld {
                since: 1,
                why: "a reboot".into(),
                from: Some("P9R1KI".into()),
            })
        );
        assert_eq!(
            read("{\"at\":9,\"since\":1,\"why\":\"a reboot\",\"from\":\"\"}").and_then(|h| h.from),
            None,
            "a sender left empty was read as one named"
        );
        assert_eq!(
            read("{\"at\":1794602830,\"why\":\"a reboot\"}"),
            None,
            "a hold written without since was read as one the net counts from"
        );
    }

    /// A snapshot of the net held by one hold first seen `minutes` ago and
    /// last seen a minute ago, with a merge landing at each of `merged`.
    pub fn held_for(minutes: i64, merged: &[i64]) -> Snapshot {
        let mut s = calm(100_000);
        s.net_held = Some(100_000 - 60);
        s.net_held_since = Some(NetHeld {
            since: 100_000 - minutes * 60,
            why: "P9R1KI, a hold about Slack".into(),
            from: Some("P9R1KI".into()),
        });
        s.landings = merged
            .iter()
            .map(|&at| Landing {
                merge: merge("abc", at),
                on: None,
            })
            .collect();
        s
    }

    fn stalls(s: &Snapshot) -> Vec<Breach> {
        breaches(s)
            .into_iter()
            .filter(|b| b.key.starts_with("netheld:"))
            .collect()
    }

    #[test]
    fn a_net_held_half_an_hour_while_merges_land_is_said_once_for_the_hold() {
        let since = 100_000 - 31 * 60;
        let said = stalls(&held_for(31, &[since + 60, since + 120]));
        assert_eq!(said.len(), 1, "the stalled net wasn't said: {said:?}");
        assert_eq!(said[0].key, format!("netheld:{since}"), "keyed by the hold");
        assert!(
            said[0].text.contains("31 min")
                && said[0].text.contains("2 merges")
                && said[0].text.contains("a hold about Slack"),
            "its time, its merges or its why went unsaid: {}",
            said[0].text
        );
        let later = stalls(&held_for(90, &[since + 60]));
        assert_eq!(
            later[0].key, "netheld:94600",
            "the same hold is said under one key however long it stands"
        );
    }

    #[test]
    fn a_held_net_is_quiet_with_nothing_landed_under_half_an_hour_or_once_let_go() {
        let since = 100_000 - 31 * 60;
        assert!(
            stalls(&held_for(31, &[since - 60])).is_empty(),
            "a merge from before the hold was counted as waiting on it"
        );
        assert!(
            stalls(&held_for(29, &[100_000 - 60])).is_empty(),
            "said before half an hour"
        );
        let mut s = held_for(31, &[since + 60]);
        s.net_held = Some(100_000 - HELD_FRESH_S - 60);
        assert!(
            stalls(&s).is_empty(),
            "a hold the net no longer finds was said as standing"
        );
        s.net_held = Some(100_000 - 60);
        s.net_held_since = None;
        assert!(
            stalls(&s).is_empty(),
            "a hold written without since was said"
        );
    }
}
