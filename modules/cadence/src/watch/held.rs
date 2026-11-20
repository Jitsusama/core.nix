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
/// Its sender is kept as a bare address, as the mesh reads who answered,
/// and only when it reads as one, capitals and digits, so no sender can
/// share the key of a hold with none.
#[must_use]
pub fn read(held: &str) -> Option<NetHeld> {
    let held: serde_json::Value = serde_json::from_str(held).ok()?;
    Some(NetHeld {
        since: held["since"].as_i64()?,
        why: held["why"].as_str().unwrap_or_default().to_owned(),
        from: held["from"]
            .as_str()
            .map(|from| from.trim().trim_start_matches('@'))
            .filter(|from| {
                !from.is_empty()
                    && from
                        .bytes()
                        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
            })
            .map(str::to_owned),
    })
}

/// The key a hold's alarm goes by: when the net first found the stop, and
/// who holds it now, or `unnamed`.
fn key(hold: &NetHeld) -> String {
    format!(
        "netheld:{}:{}",
        hold.since,
        hold.from.as_deref().unwrap_or("unnamed")
    )
}

/// Whether the hold an alarm keyed `key` named is known to have ended: the
/// net is held no more, or held now under another key, as when the hold
/// passed to another sender. A `held.json` the net hasn't rewritten lately
/// says neither, so it hasn't.
#[must_use]
pub fn ended(s: &Snapshot, key_of: &str) -> bool {
    if s.net_held.is_none() {
        return true;
    }
    still_found(s) && s.net_held_since.as_ref().map(key).as_deref() != Some(key_of)
}

/// The net held by one hold past [`STALLED_MIN`] while main gained merges
/// since it began, said once for each holder, keyed by when the net first
/// found the stop and who holds it, and naming what it says. The net keeps
/// one `since` while one hold follows another, so without its holder a
/// second hold in the same stop would never be asked. A hold the net names
/// no sender for, as the pause file, is keyed `unnamed`, which no mesh
/// address is. Only a hold the net still finds counts.
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
    // The minutes are the stop's, which one hold may have passed to
    // another, so the hold is named as the one holding it now.
    Some(Breach {
        key: key(hold),
        text: format!(
            "the net has started nothing for {minutes} min while {merged} {noun} landed \
             untested, now held by: {why}"
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
    fn each_holder_in_one_stop_and_a_hold_with_no_sender_has_a_key_of_its_own() {
        // The net keeps `since` across holders while each write follows
        // the last, so only who holds it tells a second hold from the first.
        let since = 100_000 - 31 * 60;
        let mut s = held_for(31, &[since + 60]);
        let keys = |s: &Snapshot| stalls(s).into_iter().map(|b| b.key).collect::<Vec<_>>();
        let first = keys(&s);
        s.net_held_since.as_mut().expect("a hold").from = Some("OSPHT4".into());
        assert_eq!(
            [first, keys(&s)].concat(),
            [
                format!("netheld:{since}:P9R1KI"),
                format!("netheld:{since}:OSPHT4"),
            ],
            "a second holder in the same stop was keyed as the first"
        );
        s.net_held_since.as_mut().expect("a hold").from = None;
        assert_eq!(
            keys(&s),
            [format!("netheld:{since}:unnamed")],
            "a hold with no sender was keyed as a sender's"
        );
    }

    #[test]
    fn only_an_address_is_read_as_a_holds_sender() {
        let from = |from: &str| {
            read(&format!(r#"{{"since":1,"why":"w","from":"{from}"}}"#))
                .expect("a hold")
                .from
        };
        assert_eq!(
            from("@P9R1KI"),
            Some("P9R1KI".into()),
            "an address wasn't read"
        );
        for wrong in ["unnamed", "A:B", ""] {
            assert_eq!(from(wrong), None, "{wrong:?} was read as an address");
        }
    }

    #[test]
    fn a_hold_ends_when_the_net_is_free_or_held_by_another_never_when_unread() {
        let mut s = held_for(31, &[]);
        let since = 100_000 - 31 * 60;
        let own = format!("netheld:{since}:P9R1KI");
        assert!(!ended(&s, &own), "a hold still standing read as ended");
        assert!(
            ended(&s, &format!("netheld:{since}:OSPHT4")),
            "a hold passed to another still stood"
        );
        s.net_held = Some(s.now - HELD_FRESH_S - 60);
        assert!(
            !ended(&s, &own),
            "a held.json gone stale read as the hold's end"
        );
        s.net_held = None;
        assert!(ended(&s, &own), "a free net still held the hold");
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
