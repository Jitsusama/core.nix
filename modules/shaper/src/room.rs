//! Room for a call on the download. While a call is on and the download
//! fills its line, the download is held a little under the line, so its
//! queue stands in this machine's CAKE rather than the router's. CAKE's
//! tins put the call ahead of bulk flows, and its set-associative hash all
//! but rules out the call sharing a queue with one. A router's `fq_codel`
//! hashes into 1,024 queues, and a call that lands in a bulk flow's shares
//! its drops: on 23 November the call test's media call lost 332 of 5,213
//! datagrams so, in the router's download queue, while nothing here held
//! the download (vm-a-call-keeps-its-packets).
//!
//! Only a network that declares its line's `down` is held. A rate learned
//! from what the download carried would read a burst past the line as the
//! line, and never hold, or a line seldom filled as a slower one, and hold
//! Joel's download under what it could carry. Held, the download can't
//! show a line faster than the one declared, so a line made faster is
//! declared again, or each call holds it at nine tenths of the old one.
//!
//! What the link carried is all it received, the network's own traffic
//! too, so a copy from a machine beside it at the line's rate starts a
//! hold as a download would. The hold itself keeps to the internet: the
//! IFB sends what's local past its CAKE (`qdisc::LOCAL_BIT`), so the copy
//! goes at its whole rate, and the download a tenth under its line until
//! the call ends.

use crate::rate::{float, whole};

/// The share of its line the download is held to while a call is on, and
/// the share it must carry for the hold to start: under the line by more
/// than a second's spread of what it carries, so the router's queue stays
/// empty.
pub const ROOM: f64 = 0.9;

/// Whether a link's download is held for a call, and at what rate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Room {
    held: Option<u64>,
}

impl Room {
    /// The rate link `name`'s download is held to for a call this second,
    /// in kbit/s, and the journal's line as a hold begins or ends. A hold
    /// begins while a call is on, `call`, and the download carried,
    /// `carried` in kbit/s, at least `ROOM` of the network's declared
    /// `line`, and holds at `ROOM` of it until the call ends or the
    /// network declares none. A link the recorder doesn't follow carries
    /// nothing known, so it starts no hold, but keeps one it has, as it
    /// does through a second the recorder missed: a call held through a
    /// spell costs the download a tenth, where one let go a second costs
    /// the call its drops.
    pub fn step(
        &mut self,
        name: &str,
        call: bool,
        carried: Option<f64>,
        line: Option<u64>,
    ) -> (Option<u64>, Option<String>) {
        let room = line.filter(|_| call).map(|line| whole(float(line) * ROOM));
        let full = carried
            .zip(line)
            .is_some_and(|(carried, line)| carried >= float(line) * ROOM);
        let was = self.held;
        self.held = room.filter(|_| was.is_some() || full);
        let said = match (was, self.held) {
            (None, Some(room)) => Some(format!(
                "{name}: a call is on and the download fills the line, so it's held to \
                 {room} kbit/s, where its own queue puts the call first"
            )),
            (Some(_), None) => Some(format!("{name}: the download is no longer held for a call")),
            _ => None,
        };
        (self.held, said)
    }

    /// The rate the download is held to for a call, in kbit/s, if it is.
    pub fn held(self) -> Option<u64> {
        self.held
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINE: u64 = 90_000;

    #[test]
    fn a_call_holds_a_full_download_under_its_line() {
        let mut room = Room::default();
        let (held, said) = room.step("eth1", true, Some(85_000.0), Some(LINE));
        assert_eq!(held, Some(81_000), "nine tenths of the line: {said:?}");
        assert!(
            said.as_deref()
                .is_some_and(|s| s.contains("held to 81000 kbit/s")),
            "the journal says the hold begins: {said:?}"
        );
        let (held, said) = room.step("eth1", true, Some(80_000.0), Some(LINE));
        assert_eq!(
            (held, said),
            (Some(81_000), None),
            "a held download carries under the line, and stays held, said once"
        );
    }

    #[test]
    fn a_hold_begins_where_the_download_carries_nine_tenths_of_its_line() {
        let mut room = Room::default();
        let (held, _) = room.step("eth1", true, Some(81_000.0), Some(LINE));
        assert_eq!(held, Some(81_000), "a download at nine tenths wasn't held");
    }

    #[test]
    fn a_download_short_of_its_line_is_left_alone() {
        let mut room = Room::default();
        let cases = [
            (
                "a call, the download under nine tenths",
                true,
                Some(80_999.0),
                Some(LINE),
            ),
            (
                "a full download, no call",
                false,
                Some(90_000.0),
                Some(LINE),
            ),
            ("a call, nothing known carried", true, None, Some(LINE)),
            ("a call, no line declared", true, Some(90_000.0), None),
        ];
        for (case, call, carried, line) in cases {
            assert_eq!(
                room.step("eth1", call, carried, line),
                (None, None),
                "{case}"
            );
        }
    }

    #[test]
    fn the_hold_ends_with_the_call_and_not_before() {
        let mut room = Room::default();
        room.step("eth1", true, Some(90_000.0), Some(LINE));
        assert_eq!(
            room.step("eth1", true, None, Some(LINE)),
            (Some(81_000), None),
            "a second the recorder missed keeps the hold"
        );
        let (held, said) = room.step("eth1", false, Some(70_000.0), Some(LINE));
        assert_eq!(held, None, "the call ended");
        assert_eq!(
            said.as_deref(),
            Some("eth1: the download is no longer held for a call"),
            "the journal says the hold ends"
        );
        assert_eq!(room.held(), None, "and holds nothing after");
    }

    #[test]
    fn a_network_declaring_no_line_lets_a_hold_go() {
        let mut room = Room::default();
        room.step("eth1", true, Some(90_000.0), Some(LINE));
        let (held, said) = room.step("eth1", true, Some(90_000.0), None);
        assert_eq!(held, None, "a network with no line holds nothing");
        assert!(said.is_some(), "and the journal says so");
    }
}
