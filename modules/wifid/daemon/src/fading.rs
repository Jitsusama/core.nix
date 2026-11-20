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
//! The signal is the kernel's, read each second with the echo, rather than
//! from threshold events, which only a process holding `CAP_NET_ADMIN` may
//! set and whose one threshold per card wpa_supplicant may hold already.

use tokio::time::Duration;

use crate::air::Heard;

/// How far back a call's echoes are counted. The card echoes once a
/// second, so it holds about ten.
pub const WINDOW: Duration = Duration::from_secs(10);

/// How many echoes lost within `WINDOW` make a call's link fading. One
/// alone doesn't: from 14 to 20 Nov 2026, about half the two-minute spans
/// that lost any echo lost just one, nearly all heard louder than -50 dBm.
pub const DROPPED: usize = 2;

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

/// How long after giving way the card holds where it went, so the new
/// point's window fills before it's judged.
pub const BETWEEN: Duration = Duration::from_secs(30);

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

/// Where a card on `point` gives way to during a call, if its link is
/// fading: the loudest other access point of its network it hears, kept
/// off none (`kept_off`), loud enough to be worth the move. `lost` is how
/// many echoes it lost within `WINDOW`, and `signal` how loud the kernel
/// last heard its point, else how loud its look did.
pub fn giving_way(
    point: &str,
    (lost, signal): (usize, Option<i32>),
    around: &[Heard],
    kept_off: impl Fn(&str) -> bool,
) -> Option<Giving> {
    let signal = signal.or_else(|| {
        around
            .iter()
            .find(|heard| heard.bssid == point)
            .map(|heard| i32::from(heard.signal))
    })?;
    let loudest = around
        .iter()
        .filter(|heard| heard.bssid != point && !kept_off(&heard.bssid))
        .max_by_key(|heard| heard.signal)?;
    let heard = i32::from(loudest.signal);
    let why = if lost >= DROPPED && heard >= signal - FAINTER {
        Why::Lossy(lost)
    } else if signal < FADED && heard >= signal + STRONGER {
        Why::Faint(signal)
    } else {
        return None;
    };
    Some(Giving {
        to: loudest.bssid.clone(),
        heard: loudest.signal,
        why,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::radio::Radio;
    use crate::state::Security;

    const DESK: &str = "6c:ae:f6:c1:84:26";
    const HALL: &str = "6c:ae:f6:ba:85:86";
    const ROOM: &str = "6c:ae:f6:ba:85:87";

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

    fn none(_: &str) -> bool {
        false
    }

    #[test]
    fn a_call_gives_way_on_the_second_echo_lost_in_ten_seconds_not_the_first() {
        let around = [heard(DESK, -45), heard(HALL, -48)];

        let one = giving_way(DESK, (1, Some(-45)), &around, none);
        let two = giving_way(DESK, (2, Some(-45)), &around, none);

        assert_eq!(one, None, "a lone lost echo moves no call");
        assert_eq!(
            two,
            Some(Giving {
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

        let taken = giving_way(DESK, (3, Some(-45)), &within, none);
        let kept = giving_way(DESK, (3, Some(-45)), &past, none);

        assert_eq!(
            taken.map(|giving| giving.to),
            Some(HALL.into()),
            "six fainter is near enough"
        );
        assert_eq!(kept, None, "seven fainter gains nothing");
    }

    #[test]
    fn a_call_gives_way_past_minus_seventy_two_to_a_point_eight_db_stronger() {
        let around = [heard(DESK, -60), heard(HALL, -65)];

        let at = giving_way(DESK, (0, Some(-72)), &around, none);
        let past = giving_way(DESK, (0, Some(-73)), &around, none);

        assert_eq!(at, None, "at -72 dBm the link holds");
        assert_eq!(
            past,
            Some(Giving {
                to: HALL.into(),
                heard: -65,
                why: Why::Faint(-73)
            }),
            "past it, a point 8 dB stronger is given way to"
        );
    }

    #[test]
    fn a_faint_call_gives_way_to_no_point_less_than_eight_db_stronger() {
        let around = [heard(DESK, -60), heard(HALL, -67)];

        let held = giving_way(DESK, (0, Some(-74)), &around, none);

        assert_eq!(held, None, "7 dB stronger may be the same air read twice");
    }

    #[test]
    fn the_kernels_signal_outranks_what_a_look_heard_minutes_ago() {
        // The look heard the desk loud before the walk began; the kernel
        // hears it faded now.
        let around = [heard(DESK, -48), heard(HALL, -40)];

        let faded = giving_way(DESK, (0, Some(-78)), &around, none);
        let unread = giving_way(DESK, (0, None), &around, none);

        assert_eq!(
            faded.map(|giving| giving.why),
            Some(Why::Faint(-78)),
            "the kernel's reading counts"
        );
        assert_eq!(unread, None, "the look's, where the kernel hasn't said");
    }

    #[test]
    fn a_call_gives_way_to_the_loudest_point_not_kept_off() {
        let around = [heard(DESK, -78), heard(HALL, -40), heard(ROOM, -55)];

        let giving = giving_way(DESK, (0, Some(-78)), &around, |point| point == HALL);

        assert_eq!(
            giving.map(|giving| giving.to),
            Some(ROOM.into()),
            "the hall is kept off, so the room"
        );
    }

    #[test]
    fn a_card_hearing_no_other_point_gives_way_to_none() {
        let alone = [heard(DESK, -80)];

        assert_eq!(giving_way(DESK, (5, Some(-80)), &alone, none), None);
    }
}
