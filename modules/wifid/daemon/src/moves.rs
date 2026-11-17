//! Where a card is moved each minute, and why it's said: what's known of
//! each access point it hears, from what was measured lately
//! (measure.rs) or learned at this hour (learn.rs), handed to the card's
//! chooser (choose.rs), with each move logged with what it went by.

use tokio::sync::{mpsc, watch};
use tokio::time::Instant;

use crate::acts;
use crate::air::Heard;
use crate::choose::{self, Choice, Chooser, Other};
use crate::learn::Learned;
use crate::measure::{self, Measured, Measures};
use crate::service::Ask;
use crate::state::{Joined, State};

/// Where what's known of an access point comes from, said with each move
/// so a move made on what was learned can be told from one made on what
/// was just measured (15 Nov 2026).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Whence {
    /// Measured in the span.
    Lately,
    /// Learned at this hour, heard about as loud.
    Learned,
}

/// What's known of how an access point carries from here: what it carried
/// in the span, else what it's carried at this hour, heard about as loud.
fn known<'a>(
    lately: &'a Measures,
    learned: &'a Learned,
    now: Instant,
    hour: u8,
) -> impl Fn(&str, i16) -> Option<(Measured, Whence)> + 'a {
    move |point, signal| {
        let lately = lately
            .of(point, now)
            .map(|measured| (measured, Whence::Lately));
        lately.or_else(|| {
            learned
                .of(point, hour, signal)
                .map(|measured| (measured, Whence::Learned))
        })
    }
}

/// What's known of an access point heard this loud, and whence.
type Known<'a> = dyn Fn(&str, i16) -> Option<(Measured, Whence)> + 'a;

/// Asks a card's chooser where it should be, from what the access point
/// it's on carried, what it hears of its network and what's known of each,
/// and moves it there as the roam act does.
pub fn choose(
    chooser: &mut Chooser,
    (now, hour): (Instant, u8),
    (on, measured): (&Joined, &Measured),
    around: &[Heard],
    (lately, learned): (&Measures, &Learned),
    (asks, air, looked): (
        &mpsc::Sender<Ask>,
        &watch::Receiver<State>,
        &mpsc::Sender<String>,
    ),
    call: bool,
) {
    let known = known(lately, learned, now, hour);
    match chosen(chooser, now, (on, measured), around, &known, call) {
        Some(Next::Move(to)) => {
            tokio::spawn(roam(asks.clone(), air.clone(), on.card.clone(), to));
        }
        Some(Next::Look) => {
            let card = on.card.clone();
            tokio::spawn(look(asks.clone(), air.clone(), card, looked.clone()));
        }
        None => {}
    }
}

/// What a card does next, where it doesn't stay as it is.
#[derive(Debug, PartialEq)]
enum Next {
    /// Moves to this access point.
    Move(String),
    /// Looks for another of its network.
    Look,
}

/// Asks a card's chooser where it should be, from what it's on carried and
/// what it hears of its network, and says why where it moves.
fn chosen(
    chooser: &mut Chooser,
    now: Instant,
    (on, measured): (&Joined, &Measured),
    around: &[Heard],
    known: &Known<'_>,
    call: bool,
) -> Option<Next> {
    let here = around.iter().find(|heard| heard.bssid == on.point)?;
    // The card's own point goes along too, since how busy it says its
    // channel is tells the chooser whether there's more room elsewhere.
    let (others, whence): (Vec<Other>, Vec<Option<Whence>>) = around
        .iter()
        .map(|heard| {
            let known = known(&heard.bssid, heard.signal);
            let whence = known.as_ref().map(|(_, whence)| *whence);
            let other = Other {
                point: heard.bssid.clone(),
                signal: heard.signal,
                busy: heard.radio.load.map(|load| load.busy),
                frequency: heard.frequency,
                measured: known.map(|(measured, _)| measured),
            };
            (other, whence)
        })
        .unzip();
    let card = (&*on.point, measured, here.signal, here.frequency);
    let busy = here.radio.load.map(|load| load.busy);
    let from = (measured, here.frequency, busy);
    let choice = chooser.choose(now, card, &others, call);
    if let Some(unreached) = chooser.unreached() {
        println!("<6>{}", kept_off(on, &unreached));
    }
    match choice {
        Choice::Stay => None,
        Choice::Try(to) => {
            let at = others.iter().position(|other| other.point == to)?;
            let why = tried(on, from, &others[at], whence[at]);
            println!("<6>{why}");
            Some(Next::Move(to))
        }
        Choice::Back(to) => {
            println!(
                "<6>{} goes back to {to}, as {} carried worse",
                on.card, on.point
            );
            Some(Next::Move(to))
        }
        Choice::Look => {
            println!(
                "<6>{} looks for another access point, as {} carried {} at p95 with {:.1}% \
                 lost, and it hears no other of its network",
                on.card,
                on.point,
                measure::ms(measured.p95),
                measured.lost
            );
            Some(Next::Look)
        }
        Choice::LookWider => {
            println!(
                "<6>{} looks for an access point on 5 GHz, as {} carried {} at p95 on {} MHz, \
                 and it hears none of its network there within reach",
                on.card,
                on.point,
                measure::ms(measured.p95),
                here.frequency
            );
            Some(Next::Look)
        }
        Choice::LookQuieter => {
            println!(
                "<6>{} looks for a quieter channel, as {} says its own is {}% busy, and it \
                 hears none of its network quieter within reach",
                on.card,
                on.point,
                busy.unwrap_or_default()
            );
            Some(Next::Look)
        }
    }
}

/// Has every card look, and hands this one back to be chosen for as soon
/// as the look ends: on 16 Nov 2026 a card on a point carrying 77 to 106
/// ms at p95 heard another within 12 s of looking, and waited out the
/// rest of the minute there. wpa_supplicant forgets an access point it
/// hasn't heard for 3 minutes, and nothing else looks while a card is on
/// its network, so on 15 Nov 2026 a card on one carrying 44 to 52 ms at
/// p95 heard no other, though one 15 minutes before had carried 15.
async fn look(
    asks: mpsc::Sender<Ask>,
    air: watch::Receiver<State>,
    card: String,
    looked: mpsc::Sender<String>,
) {
    match acts::look(&asks, air).await {
        // Wifid stopping is the only way the channel closes, so a card
        // not handed back then has nothing to be chosen for.
        Ok(true) => {
            let _ = looked.send(card).await;
        }
        Ok(false) => println!("<6>{card}'s look didn't end in time to choose again at once"),
        Err(refused) => {
            eprintln!("<4>{card} couldn't look for another access point: {refused:?}");
        }
    }
}

/// Moves a card to the access point chosen as the roam act does, looking
/// for it first where it's gone unheard.
async fn roam(asks: mpsc::Sender<Ask>, air: watch::Receiver<State>, card: String, to: String) {
    if let Err(refused) = acts::roam(&asks, air, &to).await {
        eprintln!("<4>{card} couldn't move to {to}: {refused:?}");
    }
}

/// Why a card keeps off an access point it was moved to: it's somewhere
/// else since, whether the move failed or something moved it on, and the
/// chooser can't tell which.
fn kept_off(on: &Joined, unreached: &str) -> String {
    format!(
        "{} keeps off {unreached} for {} minutes, as it's on {} rather than there since \
         it was moved",
        on.card,
        choose::SPURNED.as_secs() / 60,
        on.point
    )
}

/// Why a card tries another access point: what the one it's on carried,
/// what was known of the other, and from where, and whether it's to a
/// wider or narrower band or a quieter channel.
fn tried(
    on: &Joined,
    (measured, frequency, busy): (&Measured, u32, Option<u8>),
    to: &Other,
    whence: Option<Whence>,
) -> String {
    let known = match (&to.measured, whence) {
        (Some(known), Some(Whence::Learned)) => {
            format!(
                "carried {} at p95 at this hour before",
                measure::ms(known.p95)
            )
        }
        (Some(known), _) => format!(
            "carried {} at p95 in the last two minutes",
            measure::ms(known.p95)
        ),
        (None, _) => format!("is untried, heard at {} dBm", to.signal),
    };
    let room = match (busy, to.busy) {
        _ if choose::wider(to.frequency, frequency) => ", on a wider band".to_owned(),
        _ if choose::wider(frequency, to.frequency)
            && choose::resting(measured)
            && !choose::badly(measured) =>
        {
            ", on a narrower band, as the card moves little".to_owned()
        }
        (Some(busy), Some(theirs))
            if busy > choose::BUSIEST
                && choose::quieter(theirs, busy)
                && to.measured.is_none()
                && !choose::badly(measured) =>
        {
            format!(", on a channel {theirs}% busy rather than {busy}%")
        }
        _ => String::new(),
    };
    format!(
        "{} tries {}, as {} carried {} at p95 with {:.1}% lost, and {} {known}{room}",
        on.card,
        to.point,
        on.point,
        measure::ms(measured.p95),
        measured.lost,
        to.point
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use tokio::time::Duration;

    use super::*;
    use crate::learn::Here;
    use crate::measure::Echo;
    use crate::measure::tests::joined;
    use crate::radio::{Load, Radio};
    use crate::state::Security;

    const DESK: &str = "6c:ae:f6:c1:84:26";
    const HALL: &str = "6c:ae:f6:c1:84:28";

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    fn heard(bssid: &str, signal: i16) -> Heard {
        Heard {
            name: "Lantern".into(),
            security: Security::Personal,
            signal,
            bssid: bssid.into(),
            frequency: 2437,
            age: 3,
            radio: Radio::default(),
        }
    }

    fn carried(p95: u64) -> Measured {
        Measured {
            echoes: 120,
            p50: None,
            p95: Some(ms(p95)),
            lost: 0.0,
            retried: 0.0,
            failed: 0.0,
            signal: None,
            moving: Some(200),
            room: None,
        }
    }

    /// What the hall carried for ten minutes at eight in the evening.
    fn the_hall_at_eight(dir: &Path, p95: u64) -> Learned {
        let mut learned = Learned::kept_in(&dir.join("carried"));
        let here = Here {
            network: "Lantern",
            point: HALL,
            hour: 20,
            signal: -50,
            trusted: true,
        };
        for _ in 0..10 {
            learned.carried(&here, &carried(p95)).unwrap();
        }
        learned
    }

    /// Has a card look, the main loop answering that it's looking and
    /// ending the look or not, and says which card was handed back to be
    /// chosen for again before the look's end, and which after, if any.
    async fn looked_with(ending: bool) -> (Option<String>, Option<String>) {
        let (asks, mut taken) = mpsc::channel(1);
        let (looked, mut ended) = mpsc::channel(1);
        let (state, air) = watch::channel(State::default());
        let looking = tokio::spawn(look(asks, air, "wlan0".into(), looked));
        let Some(Ask::Scan { done }) = taken.recv().await else {
            panic!("asked for something other than a look");
        };
        done.send(Ok(())).expect("answered");
        tokio::task::yield_now().await;
        let before = ended.try_recv().ok();
        if ending {
            state.send_modify(State::look_ended);
        }
        looking.await.expect("looked");
        (before, ended.try_recv().ok())
    }

    #[tokio::test(start_paused = true)]
    async fn a_card_is_chosen_for_again_once_its_look_ends() {
        let (before, after) = looked_with(true).await;

        assert_eq!(before, None, "not as the look began");
        assert_eq!(after.as_deref(), Some("wlan0"), "once it ended");
    }

    #[tokio::test(start_paused = true)]
    async fn a_card_whose_look_never_ends_waits_for_its_next_minute() {
        let (_, again) = looked_with(false).await;

        assert_eq!(again, None, "its look never ended");
    }

    #[test]
    fn an_access_point_not_measured_lately_is_known_by_what_it_carried_at_this_hour() {
        let dir = tempfile::tempdir().unwrap();
        let learned = the_hall_at_eight(dir.path(), 15);
        let lately = Measures::default();

        let at_eight = known(&lately, &learned, Instant::now(), 20);
        let at_nine = known(&lately, &learned, Instant::now(), 21);

        let (hall, whence) = at_eight(HALL, -50).expect("the hall was learned at eight");
        assert_eq!(hall.p95, Some(ms(15)), "what it carried then is known");
        assert_eq!(whence, Whence::Learned, "and known as learned");
        assert_eq!(at_nine(HALL, -50), None, "nothing was learned at nine");
    }

    #[test]
    fn what_an_access_point_carried_lately_outweighs_what_it_learned() {
        let dir = tempfile::tempdir().unwrap();
        let learned = the_hall_at_eight(dir.path(), 15);
        let mut lately = Measures::default();
        let start = Instant::now();
        for second in 0..100 {
            let at = start + Duration::from_secs(second);
            lately.echoed(HALL, at, Echo::Answered(ms(40)));
        }

        let known = known(&lately, &learned, start + Duration::from_secs(99), 20);

        let (hall, whence) = known(HALL, -50).expect("the hall was measured lately");
        assert_eq!(hall.p95, Some(ms(40)), "what it carried lately is known");
        assert_eq!(whence, Whence::Lately, "and known as measured lately");
    }

    fn other(point: &str, measured: Option<Measured>) -> Other {
        Other {
            point: point.into(),
            signal: -50,
            busy: None,
            frequency: 2437,
            measured,
        }
    }

    #[test]
    fn a_move_says_what_it_went_by_and_whence() {
        let on = joined(DESK);
        let why = "wlan2 tries 6c:ae:f6:c1:84:28, as 6c:ae:f6:c1:84:26 carried 30.0 ms at p95 \
                   with 0.0% lost, and 6c:ae:f6:c1:84:28";
        let quick = Some(carried(10));

        assert_eq!(
            tried(
                &on,
                (&carried(30), 2437, None),
                &other(HALL, quick.clone()),
                Some(Whence::Learned)
            ),
            format!("{why} carried 10.0 ms at p95 at this hour before"),
            "a move made on what was learned says so"
        );
        assert_eq!(
            tried(
                &on,
                (&carried(30), 2437, None),
                &other(HALL, quick),
                Some(Whence::Lately)
            ),
            format!("{why} carried 10.0 ms at p95 in the last two minutes"),
            "a move made on what was just measured says so"
        );
        assert_eq!(
            tried(&on, (&carried(30), 2437, None), &other(HALL, None), None),
            format!("{why} is untried, heard at -50 dBm"),
            "a move made on signal alone says so"
        );
    }

    #[test]
    fn a_move_to_a_wider_band_says_so() {
        let on = joined(DESK);
        let wide = Other {
            frequency: 5260,
            ..other(HALL, None)
        };

        let why = tried(&on, (&carried(9), 2437, None), &wide, None);

        assert!(
            why.ends_with("is untried, heard at -50 dBm, on a wider band"),
            "a move from 2.4 GHz to 5 GHz says it's for the band: {why}"
        );
    }

    #[test]
    fn a_move_to_a_narrower_band_says_so() {
        let on = joined(DESK);
        let narrow = other(HALL, Some(carried(9)));
        let resting = Measured {
            moving: Some(24),
            ..carried(30)
        };

        let why = tried(&on, (&resting, 5260, None), &narrow, None);
        let failing = tried(&on, (&carried(52), 5260, None), &narrow, None);

        let alone = "two minutes, on a narrower band, as the card moves little";
        assert!(why.ends_with(alone), "says why it left: {why}");
        assert!(!failing.contains("band"), "it moves as bad: {failing}");
    }

    #[test]
    fn a_card_keeping_off_a_point_it_was_moved_to_says_where_it_is() {
        let why = kept_off(&joined(HALL), DESK);

        assert!(
            why.ends_with(&format!(
                "keeps off {DESK} for 60 minutes, as it's on {HALL} rather \
                 than there since it was moved"
            )),
            "a point kept off says how long and where the card is: {why}"
        );
    }

    #[test]
    fn a_move_to_a_quieter_channel_says_so() {
        let on = joined(HALL);
        let quieter = Other {
            busy: Some(20),
            frequency: 5260,
            ..other(DESK, None)
        };

        let why = tried(&on, (&carried(38), 5745, Some(56)), &quieter, None);

        assert!(
            why.ends_with("heard at -50 dBm, on a channel 20% busy rather than 56%"),
            "a move off a busy channel says it's for room: {why}"
        );
    }

    #[test]
    fn a_move_off_a_busy_channel_for_another_reason_says_nothing_of_room() {
        let on = joined(HALL);
        let quieter = Other {
            busy: Some(20),
            frequency: 5260,
            ..other(DESK, None)
        };
        let known = Other {
            measured: Some(carried(10)),
            ..quieter.clone()
        };

        let badly = tried(&on, (&carried(52), 5745, Some(56)), &quieter, None);
        let quicker = tried(&on, (&carried(38), 5745, Some(56)), &known, None);
        let little = tried(&on, (&carried(38), 5745, Some(34)), &quieter, None);

        assert!(!badly.contains("busy"), "it moves as bad: {badly}");
        assert!(!quicker.contains("busy"), "it moves for speed: {quicker}");
        assert!(!little.contains("busy"), "34% isn't past the bar: {little}");
    }

    #[test]
    fn a_card_on_2_4_ghz_moves_to_5_ghz_heard_well_enough() {
        let dir = tempfile::tempdir().unwrap();
        let learned = Learned::kept_in(&dir.path().join("carried"));
        let (lately, now) = (Measures::default(), Instant::now());
        let wide = Heard {
            frequency: 5260,
            ..heard(HALL, -45)
        };
        let around = [heard(DESK, -34), wide];
        let known = known(&lately, &learned, now, 20);

        let to = chosen(
            &mut Chooser::default(),
            now,
            (&joined(DESK), &carried(9)),
            &around,
            &known,
            false,
        );

        assert_eq!(
            to,
            Some(Next::Move(HALL.into())),
            "the hall's 5 GHz is heard well"
        );
    }

    #[test]
    fn a_card_moves_to_an_access_point_learned_far_quicker_at_this_hour() {
        let dir = tempfile::tempdir().unwrap();
        let learned = the_hall_at_eight(dir.path(), 10);
        let (lately, now) = (Measures::default(), Instant::now());
        let around = [heard(DESK, -50), heard(HALL, -50)];
        let known = known(&lately, &learned, now, 20);

        let to = chosen(
            &mut Chooser::default(),
            now,
            (&joined(DESK), &carried(30)),
            &around,
            &known,
            false,
        );

        assert_eq!(
            to,
            Some(Next::Move(HALL.into())),
            "10 ms learned beats 30 ms now"
        );
    }

    #[test]
    fn a_card_carrying_badly_and_hearing_no_other_looks() {
        let dir = tempfile::tempdir().unwrap();
        let learned = Learned::kept_in(&dir.path().join("carried"));
        let (lately, now) = (Measures::default(), Instant::now());
        let around = [heard(DESK, -50)];
        let known = known(&lately, &learned, now, 20);

        let next = chosen(
            &mut choose::tests::after_a_bad_minute(DESK),
            now,
            (&joined(DESK), &carried(52)),
            &around,
            &known,
            false,
        );

        assert_eq!(next, Some(Next::Look), "52 ms with nothing else heard");
    }

    #[test]
    fn a_card_carrying_well_stays_where_nothing_is_known_to_carry_better() {
        let dir = tempfile::tempdir().unwrap();
        let learned = Learned::kept_in(&dir.path().join("carried"));
        let (lately, now) = (Measures::default(), Instant::now());
        let on_5_ghz = |bssid| Heard {
            frequency: 5260,
            ..heard(bssid, -50)
        };
        let around = [on_5_ghz(DESK), on_5_ghz(HALL)];
        let known = known(&lately, &learned, now, 20);

        let to = chosen(
            &mut Chooser::default(),
            now,
            (&joined(DESK), &carried(30)),
            &around,
            &known,
            false,
        );

        assert_eq!(to, None, "30 ms is carrying well, and the hall is unknown");
    }

    #[test]
    fn a_card_hears_how_busy_its_own_channel_is() {
        let dir = tempfile::tempdir().unwrap();
        let learned = Learned::kept_in(&dir.path().join("carried"));
        let (lately, now) = (Measures::default(), Instant::now());
        let on_5_ghz = |bssid, frequency, busy| Heard {
            frequency,
            radio: Radio {
                load: Some(Load { stations: 4, busy }),
                ..Radio::default()
            },
            ..heard(bssid, -42)
        };
        let around = [on_5_ghz(HALL, 5745, 56), on_5_ghz(DESK, 5260, 20)];
        let known = known(&lately, &learned, now, 20);

        let to = chosen(
            &mut Chooser::default(),
            now,
            (&joined(HALL), &carried(38)),
            &around,
            &known,
            false,
        );

        assert_eq!(to, Some(Next::Move(DESK.into())), "the hall said 56% busy");
    }

    #[test]
    fn a_card_on_a_busy_channel_hearing_nothing_quieter_looks() {
        let dir = tempfile::tempdir().unwrap();
        let learned = Learned::kept_in(&dir.path().join("carried"));
        let (lately, now) = (Measures::default(), Instant::now());
        let crowded = Heard {
            frequency: 5745,
            radio: Radio {
                load: Some(Load {
                    stations: 9,
                    busy: 56,
                }),
                ..Radio::default()
            },
            ..heard(HALL, -41)
        };
        let known = known(&lately, &learned, now, 20);

        let next = chosen(
            &mut Chooser::default(),
            now,
            (&joined(HALL), &carried(38)),
            &[crowded],
            &known,
            false,
        );

        assert_eq!(next, Some(Next::Look), "56% busy with nothing else heard");
    }
}
