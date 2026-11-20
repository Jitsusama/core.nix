//! The net held by one hold on the machine while merges keep landing
//! untested. The net's lag counts from a hold's end, so a hold that stands
//! for hours says nothing of its own: on 19 November holds about
//! concluding quests and Slack kept the net starting nothing from 15:26 to
//! past 18:35 while 21 merges landed, and nobody was told.

use super::{Breach, Snapshot, held as still_found};

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
/// Its sender is kept as a bare address, as the mesh reads who answered.
#[must_use]
pub fn read(held: &str) -> Option<NetHeld> {
    let held: serde_json::Value = serde_json::from_str(held).ok()?;
    Some(NetHeld {
        since: held["since"].as_i64()?,
        why: held["why"].as_str().unwrap_or_default().to_owned(),
        from: held["from"]
            .as_str()
            .map(|from| from.trim().trim_start_matches('@'))
            .filter(|from| !from.is_empty())
            .map(str::to_owned),
    })
}

/// The net held by one hold past [`STALLED_MIN`] while main gained merges
/// since it began, said once for each holder, keyed by when the net first
/// found the stop and who holds it, and naming what it says. The net keeps
/// one `since` while one hold follows another, so without its holder a
/// second hold in the same stop would never be asked. Only a hold the net
/// still finds counts.
pub fn stalled(s: &Snapshot) -> Option<Breach> {
    let hold = s.net_held_since.as_ref().filter(|_| still_found(s))?;
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
        key: format!(
            "netheld:{}:{}",
            hold.since,
            hold.from.as_deref().unwrap_or("pause")
        ),
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
            read("{\"since\":1,\"why\":\"x\",\"from\":\"@P9R1KI\"}").and_then(|h| h.from),
            Some("P9R1KI".into()),
            "a sender written as a mesh mention wasn't read as its address"
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
        assert_eq!(
            said[0].key,
            format!("netheld:{since}:P9R1KI"),
            "keyed by the stop and its holder"
        );
        assert!(
            said[0].text.contains("31 min")
                && said[0].text.contains("2 merges")
                && said[0].text.contains("a hold about Slack"),
            "its time, its merges or its why went unsaid: {}",
            said[0].text
        );
        // The same hold an hour on keeps its key, so asking's seen-state
        // says it once.
        let mut later = held_for(31, &[since + 60]);
        later.now += 60 * 60;
        later.net_held = Some(later.now - 60);
        let later = stalls(&later);
        assert_eq!(
            later.iter().map(|b| b.key.as_str()).collect::<Vec<_>>(),
            [format!("netheld:{since}:P9R1KI")],
            "the same hold was said under another key as it stood longer"
        );
    }

    #[test]
    fn a_new_holder_in_the_same_stop_is_asked_in_its_turn() {
        // The net keeps `since` across holders while each write follows
        // the last, so only who holds it tells a second hold from the first.
        let since = 100_000 - 31 * 60;
        let mut s = held_for(31, &[since + 60]);
        let first = stalls(&s);
        s.net_held_since.as_mut().expect("a hold").from = Some("OSPHT4".into());
        let second = stalls(&s);
        s.net_held_since.as_mut().expect("a hold").from = None;
        let paused = stalls(&s);
        let keys: Vec<&str> = [&first, &second, &paused]
            .iter()
            .flat_map(|said| said.iter().map(|b| b.key.as_str()))
            .collect();
        assert_eq!(
            keys,
            [
                format!("netheld:{since}:P9R1KI"),
                format!("netheld:{since}:OSPHT4"),
                format!("netheld:{since}:pause"),
            ],
            "a second holder in the same stop was keyed as the first, so never asked"
        );
    }

    #[test]
    fn a_hold_counts_from_its_thirtieth_minute_and_merges_after_it_began() {
        let since = 100_000 - STALLED_MIN * 60;
        assert_eq!(
            stalls(&held_for(STALLED_MIN, &[since + 1])).len(),
            1,
            "a hold of exactly half an hour wasn't said"
        );
        assert!(
            stalls(&held_for(STALLED_MIN, &[since])).is_empty(),
            "a merge landing the second the hold began was counted as waiting on it"
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
