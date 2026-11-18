//! Whether a call's or a screen share's traffic crosses a link, from the
//! packets CAKE's video tins send, which `qdisc::video_packets` reads.

use crate::rate::float;

/// Packets a second in the video tins, both ways together, that say a call
/// or a screen share runs: a call's audio alone is about 50 a second each
/// way, where nothing else is classed there.
const LIVE_PACKETS: f64 = 20.0;
/// How long after its last such second real-time traffic still counts as
/// flowing, so a pause in talk doesn't put the floor back mid-call.
const LIVE_HOLD_S: f64 = 30.0;

/// Whether real-time traffic crosses a link, from its video tins' counts.
#[derive(Clone, Debug, Default)]
pub struct RealTime {
    /// The packets counted last, and when, in seconds.
    seen: Option<(u64, f64)>,
    /// Until when it counts as flowing.
    until: f64,
    /// What the last count gave.
    flows: bool,
}

impl RealTime {
    /// Takes the video tins' packets, sent and received together, at `now`
    /// seconds, and gives whether real-time traffic flows. A count that
    /// starts over, as a link's queues made again do, reads as none.
    pub fn count(&mut self, packets: u64, now: f64) -> bool {
        if let Some((before, at)) = self.seen {
            let seconds = (now - at).max(0.001);
            if float(packets.saturating_sub(before)) / seconds >= LIVE_PACKETS {
                self.until = now + LIVE_HOLD_S;
            }
        }
        self.seen = Some((packets, now));
        self.flows = now < self.until;
        self.flows
    }

    /// Whether real-time traffic flowed at the last count.
    pub fn flows(&self) -> bool {
        self.flows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_call_s_packets_count_as_real_time_and_last_through_a_pause() {
        let mut live = RealTime::default();
        assert!(!live.count(0, 0.0), "nothing counted was a call");
        // A call's audio, 50 packets a second each way.
        assert!(live.count(100, 1.0), "a call's audio wasn't a call");
        assert!(live.flows(), "the call counted wasn't kept");
        // Silence for a while keeps it, as talk pauses.
        assert!(live.count(100, 20.0), "a pause in talk ended the call");
        assert!(
            !live.count(100, 32.0),
            "half a minute's silence kept the call"
        );
    }

    #[test]
    fn a_trickle_isn_t_real_time_and_a_count_started_over_isn_t_a_burst() {
        let mut live = RealTime::default();
        live.count(1_000, 0.0);
        assert!(!live.count(1_010, 1.0), "a trickle was a call");
        assert!(!live.count(3, 2.0), "a count started over was a call");
    }
}
