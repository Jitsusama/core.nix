//! Whether WARP's network is degraded in a way worth saying, from the
//! status a listening warp-cli tells (Joel, 2026-11-15: "no real details,
//! nothing i can do"). WARP flips between `NetworkDegraded` and
//! `NetworkHealthy` as often as every 15 seconds, so each flip said as it
//! comes is a flicker with no meaning. Here spells of degraded less than a
//! minute apart are one episode, said only once a minute of degraded has
//! gathered in it, and over once it has been healthy for a minute. Each
//! episode carries how often it flipped, how much of the last hour was
//! degraded, and WARP's own evidence, the round trip and loss to its edge.

use std::collections::VecDeque;

use serde_json::Value;

/// How long degraded must gather in an episode before it's said, and how
/// long healthy must hold to end one, in milliseconds.
const HOLDS: u64 = 60_000;
const HEALS: u64 = 60_000;
/// How far back the hour's counts reach.
const HOUR: u64 = 3_600_000;

/// What WARP says of its network while degraded: the round trip and the
/// share lost on the way to its edge, as its status's reason carries them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Evidence {
    pub rtt_ms: Option<u64>,
    pub loss_percent: Option<f64>,
}

/// An episode of degraded worth saying.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Episode {
    /// When its first spell began.
    pub since_ms: u64,
    /// Whether WARP says degraded now, rather than in a healthy gap.
    pub now: bool,
    /// How many spells of degraded it holds.
    pub times: usize,
    /// How many spells began in the last hour, and how many whole minutes
    /// of the hour were degraded.
    pub hour_times: usize,
    pub hour_minutes: u64,
    /// WARP's evidence, as it last said it.
    pub evidence: Evidence,
}

/// The spells of degraded heard over the last hour, and WARP's evidence.
#[derive(Debug, Default)]
pub struct Spells {
    /// Each spell's start, and its end once it has healed.
    spells: VecDeque<(u64, Option<u64>)>,
    evidence: Evidence,
}

impl Spells {
    /// Hears one status line at `at_ms`: degraded, with its evidence, or not.
    pub fn heard(&mut self, degraded: Option<Evidence>, at_ms: u64) {
        let open = self.spells.back().is_some_and(|(_, end)| end.is_none());
        match degraded {
            Some(evidence) => {
                self.evidence = evidence;
                if !open {
                    self.spells.push_back((at_ms, None));
                }
            }
            None => {
                if let Some((_, end)) = self.spells.back_mut().filter(|_| open) {
                    *end = Some(at_ms);
                }
            }
        }
        while self
            .spells
            .front()
            .is_some_and(|&(_, end)| end.is_some_and(|end| at_ms.saturating_sub(end) > HOUR))
        {
            self.spells.pop_front();
        }
    }

    /// The episode worth saying at `now_ms`, if there is one.
    pub fn said(&self, now_ms: u64) -> Option<Episode> {
        let &(_, last_end) = self.spells.back()?;
        if last_end.is_some_and(|end| now_ms.saturating_sub(end) >= HEALS) {
            return None;
        }
        // Walk back while the gap before each spell is short of a healed
        // minute, gathering how long it was degraded.
        let mut first = self.spells.len() - 1;
        while first > 0 {
            let (start, _) = self.spells[first];
            let (_, end) = self.spells[first - 1];
            if end.is_some_and(|end| start.saturating_sub(end) >= HEALS) {
                break;
            }
            first -= 1;
        }
        let lasted =
            |&(start, end): &(u64, Option<u64>)| end.unwrap_or(now_ms).saturating_sub(start);
        let gathered: u64 = self.spells.range(first..).map(lasted).sum();
        if gathered < HOLDS {
            return None;
        }
        let from = now_ms.saturating_sub(HOUR);
        let hour_ms: u64 = self
            .spells
            .iter()
            .map(|&(start, end)| end.unwrap_or(now_ms).saturating_sub(start.max(from)))
            .sum();
        Some(Episode {
            since_ms: self.spells[first].0,
            now: last_end.is_none(),
            times: self.spells.len() - first,
            hour_times: self
                .spells
                .iter()
                .filter(|(start, _)| *start >= from)
                .count(),
            hour_minutes: hour_ms / 60_000,
            evidence: self.evidence,
        })
    }
}

/// How full the upload must be, in percent of what the shaper holds it
/// to, to be named behind an episode, and how much of it agents' bulk
/// must carry to be named as filling it.
const FULL_PCT: u64 = 80;
const AGENTS_PCT: u64 = 50;

/// What lies behind an episode, from the upload as waiting.rs reads it:
/// `agents` where the upload is full and agents' bulk carries at least
/// half of it; `upload` where it's full and agents' share isn't known or
/// is less; `beyond` where the line has room or there's no shaper to say,
/// so what WARP measures is lost or slow past this home, on the way to its
/// edge.
pub fn cause(upload: &Value) -> &'static str {
    let full = upload["full_pct"]
        .as_u64()
        .is_some_and(|pct| pct >= FULL_PCT);
    let agents = upload["shares"]["bulk"]
        .as_u64()
        .is_some_and(|pct| pct >= AGENTS_PCT);
    match (full, agents) {
        (true, true) => "agents",
        (true, false) => "upload",
        (false, _) => "beyond",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const S: u64 = 1000;

    /// WARP's figures on a degraded spell's line.
    fn bad(rtt: u64) -> Evidence {
        Evidence {
            rtt_ms: Some(rtt),
            loss_percent: Some(5.4),
        }
    }

    #[test]
    fn a_full_upload_is_named_and_agents_where_they_fill_it() {
        let full = |bulk: Value| json!({"full_pct": 95, "shares": {"bulk": bulk, "best": 20}});
        assert_eq!(
            cause(&full(json!(75))),
            "agents",
            "agents filling it went unnamed"
        );
        assert_eq!(
            cause(&full(json!(30))),
            "upload",
            "a full line was put on agents"
        );
        assert_eq!(cause(&json!({"full_pct": 85, "shares": null})), "upload");
        assert_eq!(cause(&json!({"full_pct": 40, "shares": null})), "beyond");
        assert_eq!(cause(&Value::Null), "beyond", "no shaper named the upload");
    }

    #[test]
    fn a_lone_blip_is_never_said() {
        let mut spells = Spells::default();
        spells.heard(Some(bad(90)), 0);
        spells.heard(None, 15 * S);
        for at in [10, 50, 65, 80, 200] {
            assert_eq!(spells.said(at * S), None, "a 15 s blip was said at {at} s");
        }
    }

    #[test]
    fn degraded_held_a_minute_is_said_from_when_it_began() {
        let mut spells = Spells::default();
        spells.heard(Some(bad(92)), 100 * S);
        assert_eq!(spells.said(159 * S), None, "said before a minute had held");
        let said = spells
            .said(160 * S)
            .expect("not said once a minute had held");
        assert_eq!(
            (said.since_ms, said.now, said.times),
            (100 * S, true, 1),
            "{said:?}"
        );
        assert_eq!(
            said.evidence.rtt_ms,
            Some(92),
            "WARP's round trip went unsaid"
        );
    }

    #[test]
    fn flips_under_a_minute_apart_are_one_episode_said_once_a_minute_gathers() {
        let mut spells = Spells::default();
        // Degraded 15 s, healthy 15 s, as WARP flipped at 20:20 on 15 November.
        let flip = |spells: &mut Spells, k: u64| {
            spells.heard(Some(bad(90 + k)), k * 30 * S);
            spells.heard(None, (k * 30 + 15) * S);
        };
        for k in 0..3 {
            flip(&mut spells, k);
        }
        assert_eq!(
            spells.said(80 * S),
            None,
            "said with only 45 s of degraded gathered"
        );
        for k in 3..6 {
            flip(&mut spells, k);
        }
        let said = spells.said(170 * S).expect("six 15 s spells went unsaid");
        assert_eq!(
            (said.since_ms, said.now, said.times),
            (0, false, 6),
            "{said:?}"
        );
        assert_eq!(said.evidence.rtt_ms, Some(95), "not WARP's latest evidence");
    }

    #[test]
    fn a_healthy_minute_ends_the_episode() {
        let mut spells = Spells::default();
        spells.heard(Some(bad(90)), 0);
        spells.heard(None, 120 * S);
        assert!(
            spells.said(179 * S).is_some(),
            "ended before a healthy minute"
        );
        assert_eq!(
            spells.said(180 * S),
            None,
            "still said after a healthy minute"
        );
        // A new spell after that starts a new episode, which must hold again.
        spells.heard(Some(bad(90)), 300 * S);
        assert_eq!(
            spells.said(330 * S),
            None,
            "the old episode carried into the new"
        );
    }

    #[test]
    fn the_hour_counts_its_spells_and_minutes_and_forgets_older_ones() {
        let mut spells = Spells::default();
        spells.heard(Some(bad(90)), 0);
        spells.heard(None, 300 * S);
        spells.heard(Some(bad(90)), 4000 * S);
        spells.heard(None, 4100 * S);
        spells.heard(Some(bad(90)), 4130 * S);
        let said = spells
            .said(4200 * S)
            .expect("the second episode went unsaid");
        assert_eq!(
            (said.times, said.hour_times, said.hour_minutes),
            (2, 2, 2),
            "{said:?}"
        );
        spells.heard(None, 4200 * S);
        spells.heard(Some(bad(90)), 8000 * S);
        assert_eq!(
            spells.spells.len(),
            1,
            "spells older than an hour were kept"
        );
    }
}
