//! Bulk inside a VPN and bulk beside it, as peers on the connection.
//!
//! A VPN's packets go as best effort on the connection, whatever they
//! carry, so its queue forms in its own CAKE, where its flows can be told
//! apart. Its bulk then outranks the connection's own: on 13 November
//! WARP's bulk tin had sent 4.39 GB since the boot and the connection's,
//! where every upload to a lent builder goes, 0.90 GB with 81,337 drops,
//! held to diffserv4's sixteenth, about 426 kbit/s of 6.8 Mbit/s, so a
//! 3 MB copy took over 110 s. Both are bulk, the work of agents nobody is
//! waiting on, so neither should starve the other. While both bulk tins
//! hold a queue and the VPN's sends more, the VPN gives up a share of its
//! rate, which the connection's bulk takes, until the two send about
//! alike. The VPN's CAKE still sends its best effort before its bulk, so
//! what Joel sends through it keeps its room, and what the VPN gives up is
//! never more than its bulk was sending.

use crate::qdisc::Tin;
use crate::rate::{float, whole};

/// How much of the gap between the two bulk tins' rates a second closes:
/// half, so a step overshoots nothing the next can't take back.
const CLOSE: f64 = 0.5;

/// How much more than it uses the connection's bulk is left once it holds
/// no queue, so a little more of it finds room before it queues again.
const ROOMY: f64 = 1.1;

/// Under this, in kbit/s, what the VPN gives up is nothing.
const NOTHING: f64 = 10.0;

/// What the two bulk tins had sent, in bytes, and when, in seconds.
#[derive(Clone, Copy, Debug)]
struct Seen {
    inside: u64,
    beside: u64,
    at: f64,
}

/// What a VPN gives up for the connection's own bulk, in kbit/s, from the
/// two bulk tins read about once a second.
#[derive(Clone, Debug, Default)]
pub struct Room {
    seen: Option<Seen>,
    give: f64,
}

impl Room {
    /// What the VPN gives up now, in kbit/s.
    pub fn given(&self) -> u64 {
        whole(self.give)
    }

    /// Takes the VPN's bulk tin and the connection's, the connection's
    /// rate in kbit/s, and the time in seconds, and answers what the VPN
    /// gives up of its rate.
    pub fn sample(&mut self, inside: &Tin, beside: &Tin, line: u64, t: f64) -> u64 {
        let now = Seen {
            inside: inside.bytes,
            beside: beside.bytes,
            at: t,
        };
        let Some(was) = self.seen.replace(now) else {
            return whole(self.give);
        };
        let dt = t - was.at;
        // A counter that went back belongs to a CAKE made again: counting
        // starts over from it.
        if dt <= 0.0 || now.inside < was.inside || now.beside < was.beside {
            return whole(self.give);
        }
        let kbit = |bytes: u64| float(bytes) * 8.0 / 1_000.0 / dt;
        let sent_inside = kbit(now.inside - was.inside);
        let sent_beside = kbit(now.beside - was.beside);
        self.give = if inside.backlog == 0 {
            // The VPN's bulk wants no more than it has, so it takes no
            // room from anything beside it.
            0.0
        } else if beside.backlog > 0 {
            // What the VPN's bulk sent was already less what it gave, and
            // the connection's more, so the gap closes from where it stands.
            let gap = (sent_inside - sent_beside) / 2.0;
            (self.give + gap * CLOSE * dt.min(1.0)).clamp(0.0, self.give + sent_inside)
        } else {
            // The connection's bulk has room to spare: it keeps what it
            // uses past its own sixteenth, and the VPN takes back the rest,
            // half of it a second. A copy's queue empties between its
            // bursts, so on 13 November taking it all back at once made
            // room and took it back every one to four seconds.
            let used = (sent_beside - float(line) / 16.0).max(0.0) * ROOMY;
            self.give - (self.give - used).max(0.0) * CLOSE * dt.min(1.0)
        };
        if self.give < NOTHING {
            self.give = 0.0;
        }
        whole(self.give)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINE: f64 = 6_800.0;

    fn tin(bytes: u64, waiting: bool) -> Tin {
        Tin {
            bytes,
            backlog: u64::from(waiting),
            ..Tin::default()
        }
    }

    fn vpn() -> f64 {
        LINE * crate::qdisc::VPN_SHARE
    }

    /// The two bulk tins sharing the line for `seconds`, the VPN's wanting
    /// `inside` kbit/s and the connection's `beside`. The VPN's bulk gets
    /// what it wants up to its rate, less what it gave; the connection's
    /// the rest of the line, or diffserv4's sixteenth, as CAKE on the
    /// connection gives it. A tin holds a queue while it wanted more than
    /// it got. Answers what each sent over the last second, in kbit/s.
    fn share(room: &mut Room, (inside, beside): (f64, f64), seconds: u32) -> (f64, f64) {
        let (mut sent, mut waiting, mut last) =
            ((0_u64, 0_u64), (inside > 0.0, beside > 0.0), (0.0, 0.0));
        for second in 0..=seconds {
            let given = float(room.sample(
                &tin(sent.0, waiting.0),
                &tin(sent.1, waiting.1),
                whole(LINE),
                f64::from(second),
            ));
            let to_inside = inside.min(vpn() - given);
            let to_beside = beside.min((LINE - to_inside).max(LINE / 16.0));
            sent.0 += whole(to_inside * 1_000.0 / 8.0);
            sent.1 += whole(to_beside * 1_000.0 / 8.0);
            waiting = (inside > to_inside + 1.0, beside > to_beside + 1.0);
            last = (to_inside, to_beside);
        }
        last
    }

    #[test]
    fn bulk_beside_a_busy_vpn_comes_to_share_the_line_with_its_bulk() {
        let mut room = Room::default();
        let (inside, beside) = share(&mut room, (10_000.0, 10_000.0), 30);
        assert!(
            (inside - beside).abs() < 100.0,
            "the VPN's bulk sent {inside:.0} kbit/s and the connection's {beside:.0}"
        );
    }

    #[test]
    fn a_vpn_gives_nothing_while_nothing_beside_it_waits() {
        let mut room = Room::default();
        let (inside, _) = share(&mut room, (10_000.0, 0.0), 30);
        assert!(
            (inside - vpn()).abs() < 1.0,
            "the VPN's bulk sent {inside:.0} of {:.0}",
            vpn()
        );
    }

    #[test]
    fn a_vpn_gives_no_more_than_the_bulk_beside_it_uses() {
        let mut room = Room::default();
        let (inside, beside) = share(&mut room, (10_000.0, 1_500.0), 30);
        assert!(
            (beside - 1_500.0).abs() < 1.0,
            "the connection's bulk wanted 1,500 kbit/s and sent {beside:.0}"
        );
        assert!(
            inside > vpn() - 1_500.0,
            "the VPN's bulk kept only {inside:.0} of {:.0}",
            vpn()
        );
    }

    #[test]
    fn what_a_vpn_gave_comes_back_once_the_bulk_beside_it_is_done() {
        let mut room = Room::default();
        share(&mut room, (10_000.0, 10_000.0), 30);
        assert!(
            room.give > 1_000.0,
            "the VPN gave only {:.0} kbit/s",
            room.give
        );
        let (inside, beside) = room.seen.map_or((0, 0), |s| (s.inside, s.beside));
        let mut given = 0;
        for second in 1..=10 {
            given = room.sample(
                &tin(inside + second * 600_000, true),
                &tin(beside, false),
                whole(LINE),
                31.0 + float(second),
            );
        }
        assert_eq!(given, 0, "the VPN still gives {given} kbit/s");
    }

    #[test]
    fn a_moment_the_bulk_beside_holds_no_queue_keeps_most_of_what_was_given() {
        let mut room = Room::default();
        share(&mut room, (10_000.0, 10_000.0), 30);
        let before = room.give;
        let (inside, beside) = room.seen.map_or((0, 0), |s| (s.inside, s.beside));
        let given = float(room.sample(
            &tin(inside + 600_000, true),
            &tin(beside, false),
            whole(LINE),
            32.0,
        ));
        assert!(
            given >= before / 2.0 - 1.0,
            "a second's empty queue beside took {given:.0} of {before:.0} kbit/s back"
        );
    }

    #[test]
    fn a_vpn_with_no_bulk_waiting_gives_nothing() {
        let mut room = Room::default();
        share(&mut room, (10_000.0, 10_000.0), 30);
        let (inside, beside) = room.seen.map_or((0, 0), |s| (s.inside, s.beside));
        let given = room.sample(
            &tin(inside + 1_000, false),
            &tin(beside + 400_000, true),
            whole(LINE),
            32.0,
        );
        assert_eq!(
            given, 0,
            "a VPN whose bulk holds no queue gave {given} kbit/s"
        );
    }

    #[test]
    fn a_cake_made_again_keeps_what_was_given() {
        let mut room = Room::default();
        share(&mut room, (10_000.0, 10_000.0), 30);
        let before = whole(room.give);
        let given = room.sample(&tin(10, true), &tin(10, true), whole(LINE), 32.0);
        assert_eq!(
            given, before,
            "a counter that went back moved what was given"
        );
    }
}
