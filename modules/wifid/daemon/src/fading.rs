//! Whether a card's link is fading under a call, and the access point of
//! its network it gives way to, judged each second rather than each
//! minute.
//!
//! A call keeps the card where it is (choose.rs) unless the last two
//! minutes lost a tenth of their echoes, which on a walk comes a minute or
//! more after the call began to break up: on 17 Nov 2026 a card heard its
//! ch52 radio fall from -74 to -80 dBm over four minutes while another
//! eero was heard at -40, and moved only once a minute had lost 27.7% of
//! its echoes at 572 ms p95. So during a call the card gives way as soon as
//! its last ten seconds lose `DROPPED` echoes, or its signal falls past
//! `FADED` with another heard `STRONGER` than it.
//!
//! The signal is the kernel's average over the frames it hears, read each
//! second with the echo, rather than one frame's, which a moment's shadow
//! dips, or threshold events, which only a process holding `CAP_NET_ADMIN`
//! may set and whose one threshold per card wpa_supplicant may hold
//! already.

use tokio::time::Duration;

use crate::air::Heard;
use crate::choose;

/// How far back a call's echoes are counted. The card echoes once a
/// second, so it holds about ten.
pub const WINDOW: Duration = Duration::from_secs(10);

/// How many echoes lost within `WINDOW` make a call's link fading. One
/// alone doesn't: from 14 to 20 Nov 2026, about half the two-minute spans
/// that lost any echo lost just one, nearly all heard louder than -50 dBm.
pub const DROPPED: usize = 2;

/// How many echoes lost within `WINDOW` make a call's link gone, given way
/// from to any other point heard however faint: half of them. A point
/// that stops answering may leave the kernel's signal at the last frame
/// it heard, loud.
pub const GONE: usize = 5;

/// How faint, in dBm, a call's link may grow before another heard
/// `STRONGER` is given way to. From 14 to 20 Nov 2026 the card's two
/// spells fainter than this that lost echoes were at -75 to -77 dBm, and
/// none at -72 or louder; past it, it rarely stayed loss-free for long.
pub const FADED: i32 = -72;

/// How much louder, in dB, another access point must be heard than a
/// fading link for the card to give way to it: enough to be another room's
/// rather than the same air read twice.
pub const STRONGER: i32 = 8;

/// How much fainter than a lossy link another may be heard and still be
/// given way to, as the chooser tries one (choose.rs): losing echoes at a
/// strong signal, the card gains nothing from a faint point.
pub const FAINTER: i32 = 6;

/// How long after any move the card holds where it went, so the move isn't
/// raced and the new point's window fills before it's judged. A give-way
/// not there by then never got there (choose.rs).
pub const BETWEEN: Duration = Duration::from_secs(30);

/// How long a call keeps off each point it gave way from, and holds where
/// it lost again after giving way for loss: loss every point shares, as
/// the gateway's own, would otherwise move it from point to point every
/// `BETWEEN`.
pub const AWAY: Duration = Duration::from_secs(120);

/// How often a fading call with nowhere to give way to has the card look
/// for somewhere: wpa_supplicant forgets a point 180 s after it last heard
/// it, and nothing else looks while a card is on a call, but a look takes
/// the card off its channel for 5 to 6 s.
pub const LOOKS: Duration = Duration::from_secs(120);

/// How long into a look the echoes it loses are left out, as the look's
/// rather than the link's. A look takes the card off its channel, and
/// looks asked once a second in a test lost 8 of the 10 echoes before a
/// call (2026-11-20). One over every channel takes 5 to 6 s (main.rs), but
/// a card can't leave a channel whose point no longer answers, so a fading
/// link's look runs long: one begun as a test's link lost every frame
/// took 19 s, and left out whole, held the card there throughout.
pub const LOOK: Duration = Duration::from_secs(6);

/// Why a call's link is fading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// It lost this many echoes within `WINDOW`.
    Lossy(usize),
    /// It's heard this faint, in dBm.
    Faint(i32),
}

/// Where a card on a call gives way to, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Giving {
    pub to: String,
    /// How loud it's heard, in dBm.
    pub heard: i16,
    pub why: Why,
}

/// Whether a call's link holds, or where it gives way to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Way {
    /// It isn't fading.
    Holds,
    /// It's fading, and gives way there.
    To(Giving),
    /// It's fading, and hears nowhere worth giving way to.
    Nowhere(Why),
}

/// Where a card on `point` gives way to during a call, if its link is
/// fading: the loudest other access point of its network it hears, kept
/// off none (`kept_off`), loud enough to be worth the move. `lost` is how
/// many echoes it lost within `WINDOW`, and `signal` how loud the kernel
/// lately heard its point, else how loud a look heard it within `WINDOW`:
/// a look's level older than that says nothing of a link fading now. A
/// faint link on a wider band gives way to a narrower one only
/// `choose::ACROSS` louder still, as the same access point is heard
/// louder on its narrower band; a lossy one gives way to a narrower band
/// only where it hears none as wide within reach, as 2.4 GHz brought the
/// 17 Nov 2026 downloads down to 54 to 76% (choose.rs), and a give-way
/// settles where it lands. A point that says another security than the
/// card's own is never given way to: the card's network can't be joined
/// there, and a radio in range naming itself the network could otherwise
/// pull a call off its point with a loud beacon alone.
pub fn giving_way(
    point: &str,
    (lost, signal): (usize, Option<i32>),
    around: &[Heard],
    kept_off: impl Fn(&str) -> bool,
) -> Way {
    let own = around.iter().find(|heard| heard.bssid == point);
    let looked = own.filter(|heard| u64::from(heard.age) <= WINDOW.as_secs());
    let signal = signal.or_else(|| looked.map(|heard| i32::from(heard.signal)));
    let frequency = own.map_or(0, |heard| heard.frequency);
    let lossy = lost >= DROPPED;
    let faint = signal.filter(|signal| *signal < FADED);
    if !lossy && faint.is_none() {
        return Way::Holds;
    }
    let why = |heard: &Heard| {
        let loud = i32::from(heard.signal);
        let near = lost >= GONE || signal.is_some_and(|signal| loud >= signal - FAINTER);
        if lossy && near {
            return Some(Why::Lossy(lost));
        }
        let across = if choose::wider(frequency, heard.frequency) {
            i32::from(choose::ACROSS)
        } else {
            0
        };
        faint
            .filter(|signal| loud >= signal + STRONGER + across)
            .map(Why::Faint)
    };
    let best = around
        .iter()
        .filter(|heard| heard.bssid != point && !kept_off(&heard.bssid))
        .filter(|heard| own.is_none_or(|own| heard.security == own.security))
        .filter_map(|heard| Some((why(heard)?, heard)))
        .max_by_key(|(_, heard)| (!choose::wider(frequency, heard.frequency), heard.signal));
    match (best, faint) {
        (Some((why, heard)), _) => Way::To(Giving {
            to: heard.bssid.clone(),
            heard: heard.signal,
            why,
        }),
        (None, Some(signal)) if !lossy => Way::Nowhere(Why::Faint(signal)),
        (None, _) => Way::Nowhere(Why::Lossy(lost)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::radio::Radio;
    use crate::state::Security;

    const DESK: &str = "02:00:00:00:a4:00";
    const HALL: &str = "02:00:00:00:a1:00";
    const ROOM: &str = "02:00:00:00:a2:00";

    fn heard(bssid: &str, signal: i16) -> Heard {
        Heard {
            name: "Lantern".into(),
            security: Security::Personal,
            signal,
            bssid: bssid.into(),
            frequency: 5260,
            age: 0,
            radio: Radio::default(),
        }
    }

    fn heard_on(bssid: &str, signal: i16, frequency: u32) -> Heard {
        Heard {
            frequency,
            ..heard(bssid, signal)
        }
    }

    fn none(_: &str) -> bool {
        false
    }

    /// Where a call gives way to, if anywhere.
    fn to(way: Way) -> Option<Giving> {
        match way {
            Way::To(giving) => Some(giving),
            Way::Holds | Way::Nowhere(_) => None,
        }
    }

    #[test]
    fn a_call_gives_way_on_the_second_echo_lost_in_ten_seconds_not_the_first() {
        let around = [heard(DESK, -45), heard(HALL, -48)];

        let one = giving_way(DESK, (1, Some(-45)), &around, none);
        let two = giving_way(DESK, (2, Some(-45)), &around, none);

        assert_eq!(one, Way::Holds, "a lone lost echo moves no call");
        assert_eq!(
            two,
            Way::To(Giving {
                to: HALL.into(),
                heard: -48,
                why: Why::Lossy(2)
            }),
            "two lost give way to the loudest other"
        );
    }

    #[test]
    fn a_lossy_call_gives_way_to_no_point_heard_past_six_db_fainter() {
        let within = [heard(DESK, -45), heard(HALL, -51)];
        let past = [heard(DESK, -45), heard(HALL, -52)];

        let taken = to(giving_way(DESK, (3, Some(-45)), &within, none));
        let kept = giving_way(DESK, (3, Some(-45)), &past, none);

        assert_eq!(
            taken.map(|giving| giving.to),
            Some(HALL.into()),
            "six fainter is near enough"
        );
        assert_eq!(
            kept,
            Way::Nowhere(Why::Lossy(3)),
            "seven fainter gains nothing"
        );
    }

    #[test]
    fn a_call_losing_half_its_echoes_gives_way_however_loud_its_signal_last_read() {
        // A point that stops answering may leave the kernel's signal at
        // the last frame it heard.
        let around = [heard(DESK, -42), heard(HALL, -60)];

        let four = giving_way(DESK, (4, Some(-42)), &around, none);
        let five = to(giving_way(DESK, (5, Some(-42)), &around, none));

        assert_eq!(four, Way::Nowhere(Why::Lossy(4)), "four lost may be air");
        assert_eq!(
            five.map(|giving| giving.why),
            Some(Why::Lossy(5)),
            "half lost is a link gone"
        );
    }

    #[test]
    fn a_call_gives_way_past_minus_seventy_two_to_a_point_eight_db_stronger() {
        // The hall is heard loud enough at both readings, so only how faint
        // the link is tells them apart.
        let around = [heard(DESK, -60), heard(HALL, -60)];

        let at = giving_way(DESK, (0, Some(-72)), &around, none);
        let past = giving_way(DESK, (0, Some(-73)), &around, none);

        assert_eq!(at, Way::Holds, "at -72 dBm the link holds");
        assert_eq!(
            past,
            Way::To(Giving {
                to: HALL.into(),
                heard: -60,
                why: Why::Faint(-73)
            }),
            "past it, a point 8 dB stronger is given way to"
        );
    }

    #[test]
    fn a_faint_call_gives_way_to_no_point_less_than_eight_db_stronger() {
        let around = [heard(DESK, -60), heard(HALL, -67)];

        let held = giving_way(DESK, (0, Some(-74)), &around, none);

        assert_eq!(
            held,
            Way::Nowhere(Why::Faint(-74)),
            "7 dB stronger may be the same air read twice"
        );
    }

    #[test]
    fn a_faint_wide_link_gives_way_to_a_narrower_band_only_ten_db_louder_still() {
        // The desk's own 2.4 GHz radio is heard louder than its 5 GHz one
        // from the same place.
        const NEAR: &str = "02:00:00:00:a5:00";
        let short = [heard(DESK, -74), heard_on(NEAR, -57, 2437)];
        let enough = [heard(DESK, -74), heard_on(NEAR, -56, 2437)];
        let wide = [heard(DESK, -74), heard(HALL, -66)];

        let held = giving_way(DESK, (0, Some(-74)), &short, none);
        let narrowed = to(giving_way(DESK, (0, Some(-74)), &enough, none));
        let widened = to(giving_way(DESK, (0, Some(-74)), &wide, none));

        assert_eq!(held, Way::Nowhere(Why::Faint(-74)), "17 dB isn't enough");
        assert_eq!(narrowed.map(|giving| giving.to), Some(NEAR.into()), "18 is");
        assert_eq!(
            widened.map(|giving| giving.to),
            Some(HALL.into()),
            "a point on its own band needs 8"
        );
    }

    #[test]
    fn the_kernels_signal_outranks_what_a_look_heard_minutes_ago() {
        // The look heard the desk loud before the walk began; the kernel
        // hears it faded now.
        let around = [heard(DESK, -48), heard(HALL, -40)];

        let faded = to(giving_way(DESK, (0, Some(-78)), &around, none));
        let unread = giving_way(DESK, (0, None), &around, none);

        assert_eq!(
            faded.map(|giving| giving.why),
            Some(Why::Faint(-78)),
            "the kernel's reading counts"
        );
        assert_eq!(
            unread,
            Way::Holds,
            "the look's, where the kernel hasn't said"
        );
    }

    #[test]
    fn a_call_gives_way_to_the_loudest_point_not_kept_off() {
        let around = [heard(DESK, -78), heard(HALL, -40), heard(ROOM, -55)];

        let giving = to(giving_way(DESK, (0, Some(-78)), &around, |point| {
            point == HALL
        }));

        assert_eq!(
            giving.map(|giving| giving.to),
            Some(ROOM.into()),
            "the hall is kept off, so the room"
        );
    }

    #[test]
    fn a_look_s_level_older_than_the_window_says_nothing_of_a_fading_link() {
        let fresh = [heard(DESK, -78), heard(HALL, -60)];
        let stale = [
            Heard {
                age: 11,
                ..heard(DESK, -78)
            },
            heard(HALL, -60),
        ];

        let faded = to(giving_way(DESK, (0, None), &fresh, none));
        let unknown = giving_way(DESK, (0, None), &stale, none);

        assert_eq!(
            faded.map(|giving| giving.why),
            Some(Why::Faint(-78)),
            "heard faint within 10 s"
        );
        assert_eq!(unknown, Way::Holds, "heard faint 11 s ago may be loud now");
    }

    #[test]
    fn a_lossy_wide_link_gives_way_to_its_own_band_before_a_louder_narrower_one() {
        const NEAR: &str = "02:00:00:00:a5:00";
        let both = [
            heard(DESK, -50),
            heard_on(NEAR, -40, 2437),
            heard(HALL, -54),
        ];
        let narrower = [heard(DESK, -50), heard_on(NEAR, -40, 2437)];

        let wide = to(giving_way(DESK, (2, Some(-50)), &both, none));
        let only = to(giving_way(DESK, (2, Some(-50)), &narrower, none));

        assert_eq!(
            wide.map(|giving| giving.to),
            Some(HALL.into()),
            "the hall's 5 GHz, though the desk's 2.4 GHz is louder"
        );
        assert_eq!(
            only.map(|giving| giving.to),
            Some(NEAR.into()),
            "2.4 GHz where it hears nothing as wide"
        );
    }

    #[test]
    fn a_call_gives_way_to_no_point_saying_another_security() {
        let around = [
            heard(DESK, -78),
            Heard {
                security: Security::Open,
                ..heard(HALL, -40)
            },
        ];

        let way = giving_way(DESK, (5, Some(-78)), &around, none);

        assert_eq!(
            way,
            Way::Nowhere(Why::Lossy(5)),
            "an open point naming itself the network isn't it"
        );
    }

    #[test]
    fn a_card_hearing_no_other_point_gives_way_to_none() {
        let alone = [heard(DESK, -80)];

        assert_eq!(
            giving_way(DESK, (5, Some(-80)), &alone, none),
            Way::Nowhere(Why::Lossy(5))
        );
    }
}
