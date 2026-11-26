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
//! The download fills its line when it carries nine tenths of it, or when
//! the controller's hold of it says the line is full: it cut the download
//! by its delay in the last few seconds, or the download fills the rate
//! it's held to (`oneway::Way::judged_full`). A cut kept on over a quiet
//! download says nothing of the line, so a call starting then is given no
//! room. The controller can cut it within a second of a load starting,
//! before a second carrying nine tenths is recorded, and a hold that
//! waited on one would never begin: the download, under the cut, never
//! carries that much again. Its queue stands in CAKE while the cut lasts,
//! but once the controller let go it would go straight in, to the
//! router's queue, until a full second was recorded and turned it back.
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

/// Whether a link's download is held for a call, at what rate, and on
/// which network.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Room {
    held: Option<(String, u64)>,
}

impl Room {
    /// The rate link `name`'s download is held to for a call this second,
    /// in kbit/s, and the journal's line as a hold begins or ends. A hold
    /// begins while a call is on, `call`, and the download fills the line
    /// its network, `network`, declares, `line`: it carried, `carried` in
    /// kbit/s, at least `ROOM` of it, or the controller's hold of it says
    /// the line is full, `cut`.
    /// It holds at `ROOM` of the line until the call ends or the network
    /// declares none. A hold is kept only on the network it began on and
    /// at the line it filled: a link that moves to another network, even
    /// one declaring the same line, or whose line changes, lets it go
    /// unless the download fills the new one, so a hold is never carried
    /// to a line it didn't fill (download hold round 2's F7 and F24, round
    /// 3's F5). A link the recorder doesn't follow carries nothing known
    /// and isn't cut, so it starts no hold, but keeps one it has, as it
    /// does through a second the recorder missed: a call held through a
    /// spell costs the download a tenth, where one let go a second costs
    /// the call its drops.
    pub fn step(
        &mut self,
        name: &str,
        network: &str,
        call: bool,
        (carried, cut): (Option<f64>, bool),
        line: Option<u64>,
    ) -> (Option<u64>, Option<String>) {
        let room = line.filter(|_| call).map(|line| whole(float(line) * ROOM));
        let fills = carried
            .zip(line)
            .is_some_and(|(carried, line)| carried >= float(line) * ROOM);
        let full = cut || fills;
        let was = self.held.take();
        let kept = |room: u64| {
            was.as_ref()
                .is_some_and(|(on, held)| on == network && *held == room)
        };
        self.held = room
            .filter(|&room| kept(room) || full)
            .map(|room| (network.to_owned(), room));
        let said = match (&was, self.held()) {
            (_, Some(room)) if !kept(room) => Some(format!(
                "{name}: a call is on and {}, so it's held to {room} kbit/s, where its own \
                 queue puts the call first",
                if fills {
                    "the download fills the line"
                } else {
                    "the controller holds the download under a full line"
                }
            )),
            (Some(_), None) => Some(format!("{name}: the download is no longer held for a call")),
            _ => None,
        };
        (self.held(), said)
    }

    /// The rate the download is held to for a call, in kbit/s, if it is.
    pub fn held(&self) -> Option<u64> {
        self.held.as_ref().map(|&(_, room)| room)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINE: u64 = 90_000;
    const HOME: &str = "home";

    /// What the download carried, in kbit/s, the controller not cutting it.
    fn carried(kbit: f64) -> (Option<f64>, bool) {
        (Some(kbit), false)
    }

    /// A second the recorder missed, the controller not cutting it.
    const UNKNOWN: (Option<f64>, bool) = (None, false);

    #[test]
    fn a_call_holds_a_full_download_under_its_line() {
        let mut room = Room::default();
        let (held, said) = room.step("eth1", HOME, true, carried(85_000.0), Some(LINE));
        assert_eq!(held, Some(81_000), "nine tenths of the line: {said:?}");
        assert!(
            said.as_deref()
                .is_some_and(|s| s.contains("held to 81000 kbit/s")),
            "the journal says the hold begins: {said:?}"
        );
        let (held, said) = room.step("eth1", HOME, true, carried(80_000.0), Some(LINE));
        assert_eq!(
            (held, said),
            (Some(81_000), None),
            "a held download carries under the line, and stays held, said once"
        );
    }

    #[test]
    fn a_hold_begins_where_the_download_carries_nine_tenths_of_its_line() {
        let mut room = Room::default();
        let (held, said) = room.step("eth1", HOME, true, carried(81_000.0), Some(LINE));
        assert_eq!(held, Some(81_000), "a download at nine tenths wasn't held");
        assert!(
            said.is_some_and(|said| said.contains("the download fills the line")),
            "the journal didn't say the download filled the line"
        );
    }

    #[test]
    fn a_hold_begins_where_the_controller_already_cuts_the_download() {
        // On 23 November the controller cut the call test's download to
        // 44.7 Mbit/s a second after the call began, before a second at
        // nine tenths was recorded, and the hold never began.
        let mut room = Room::default();
        let (held, said) = room.step("eth1", HOME, true, (Some(44_700.0), true), Some(LINE));
        assert_eq!(
            held,
            Some(81_000),
            "a cut download wasn't held for the call"
        );
        assert!(
            said.is_some_and(|said| said.contains("the controller holds the download")),
            "the journal didn't say the controller's hold began the call's"
        );
        let (held, said) = room.step("eth1", HOME, true, carried(60_000.0), Some(LINE));
        assert_eq!(
            (held, said),
            (Some(81_000), None),
            "the controller letting go let the call's hold go too"
        );
        // A second the recorder missed says nothing carried and no cut
        // (main's room_heard), which keeps the hold.
        let (held, said) = room.step("eth1", HOME, true, UNKNOWN, Some(LINE));
        assert_eq!(
            (held, said),
            (Some(81_000), None),
            "a second the recorder missed under the cut"
        );
    }

    #[test]
    fn a_download_short_of_its_line_is_left_alone() {
        let mut room = Room::default();
        let cases = [
            (
                "a call, the download under nine tenths",
                true,
                carried(80_999.0),
                Some(LINE),
            ),
            (
                "a full download, no call",
                false,
                carried(90_000.0),
                Some(LINE),
            ),
            (
                "a cut download, no call",
                false,
                (Some(44_700.0), true),
                Some(LINE),
            ),
            ("a call, nothing known carried", true, UNKNOWN, Some(LINE)),
            ("a call, no line declared", true, carried(90_000.0), None),
            (
                "a call, a cut, no line declared",
                true,
                (Some(44_700.0), true),
                None,
            ),
        ];
        for (case, call, carried, line) in cases {
            assert_eq!(
                room.step("eth1", HOME, call, carried, line),
                (None, None),
                "{case}"
            );
        }
    }

    #[test]
    fn the_hold_ends_with_the_call_and_not_before() {
        let mut room = Room::default();
        room.step("eth1", HOME, true, carried(90_000.0), Some(LINE));
        assert_eq!(
            room.step("eth1", HOME, true, UNKNOWN, Some(LINE)),
            (Some(81_000), None),
            "a second the recorder missed keeps the hold"
        );
        let (held, said) = room.step("eth1", HOME, false, carried(70_000.0), Some(LINE));
        assert_eq!(held, None, "the call ended");
        assert_eq!(
            said.as_deref(),
            Some("eth1: the download is no longer held for a call"),
            "the journal says the hold ends"
        );
        assert_eq!(room.held(), None, "and holds nothing after");
    }

    #[test]
    fn a_hold_is_let_go_on_a_new_line_the_download_doesnt_fill() {
        let mut room = Room::default();
        room.step("eth1", HOME, true, carried(90_000.0), Some(LINE));
        let (held, said) = room.step("eth1", HOME, true, carried(190_000.0), Some(200_000));
        assert_eq!(held, Some(180_000), "a new line filled at once wasn't held");
        assert!(
            said.as_deref()
                .is_some_and(|s| s.contains("held to 180000")),
            "the journal didn't say the hold moved: {said:?}"
        );
        let (held, said) = room.step("eth1", HOME, true, carried(70_000.0), Some(LINE));
        assert_eq!(held, None, "a hold was carried to a line it didn't fill");
        assert_eq!(
            said.as_deref(),
            Some("eth1: the download is no longer held for a call"),
            "the journal didn't say the hold ended"
        );
        let (held, said) = room.step("eth1", HOME, true, carried(180_000.0), Some(200_000));
        assert_eq!(
            held,
            Some(180_000),
            "a new line the download fills wasn't held"
        );
        assert!(said.is_some(), "the journal didn't say the new hold began");
    }

    #[test]
    fn a_hold_is_let_go_on_another_network_declaring_the_same_line() {
        let mut room = Room::default();
        room.step("eth1", HOME, true, carried(90_000.0), Some(LINE));
        let (held, said) = room.step("eth1", "office", true, carried(40_000.0), Some(LINE));
        assert_eq!(
            held, None,
            "a hold was carried to another network it didn't fill"
        );
        assert_eq!(
            said.as_deref(),
            Some("eth1: the download is no longer held for a call"),
            "the journal didn't say the hold ended"
        );
        let (held, said) = room.step("eth1", "office", true, carried(85_000.0), Some(LINE));
        assert_eq!(
            held,
            Some(81_000),
            "the other network's line, filled, wasn't held"
        );
        assert!(said.is_some(), "the journal didn't say the new hold began");
        let (held, said) = room.step("eth1", HOME, true, carried(85_000.0), Some(LINE));
        assert_eq!(held, Some(81_000), "a filled line on the way back");
        assert!(
            said.is_some(),
            "a hold moved to another network unsaid, at the same rate"
        );
    }

    #[test]
    fn a_network_declaring_no_line_lets_a_hold_go() {
        let mut room = Room::default();
        room.step("eth1", HOME, true, carried(90_000.0), Some(LINE));
        let (held, said) = room.step("eth1", HOME, true, carried(90_000.0), None);
        assert_eq!(held, None, "a network with no line holds nothing");
        assert!(said.is_some(), "and the journal says so");
    }

    #[test]
    fn a_call_holds_the_wireless_link_under_its_own_line_and_not_the_cable() {
        // On 25 November an unpaced download through a 2.4 GHz access
        // point raised a call's pings from 12.5 to 556 ms at p95, in the
        // access point's queue, while the cable to the same router met
        // its bars that day with no line declared: one line for the
        // network would hold the cable too.
        let line = crate::networks::Line {
            any: None,
            links: std::collections::BTreeMap::from([("wlan0".to_string(), 20_000)]),
        };
        let (mut wireless, mut cable) = (Room::default(), Room::default());
        let (held, _) = wireless.step("wlan0", HOME, true, carried(19_000.0), line.of("wlan0"));
        assert_eq!(
            held,
            Some(18_000),
            "the wireless link at nine tenths of its own"
        );
        let (held, said) = cable.step("eth1", HOME, true, carried(90_000.0), line.of("eth1"));
        assert_eq!(
            (held, said),
            (None, None),
            "the cable naming none is left alone"
        );
        let (held, said) = wireless.step("wlan0", HOME, false, carried(19_000.0), line.of("wlan0"));
        assert_eq!(held, None, "the hold goes with the call");
        assert!(said.is_some(), "and the journal says so");
        assert_eq!(
            Room::default().step("wlan0", HOME, false, carried(19_000.0), line.of("wlan0")),
            (None, None),
            "outside a call a full wireless link isn't held"
        );
    }
}
