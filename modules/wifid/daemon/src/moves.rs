//! Where a card is moved each minute, and why it's said: what's known of
//! each access point it hears, from what was measured lately
//! (measure.rs) or learned at this hour (learn.rs), handed to the card's
//! chooser (choose.rs), with each move logged with what it went by.
//!
//! Long because its cases, over half of it, stay beside the private
//! rules they prove, as every module in wifid keeps them.

use tokio::sync::{mpsc, watch};
use tokio::time::{Duration, Instant};

use crate::acts;
use crate::air::Heard;
use crate::choose::{self, Against, Choice, Chooser, Other};
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
    /// Learned while the card brought a download down, heard about as
    /// loud.
    Loaded,
}

/// Whose figures are asked for: the card's own point's or another's, as
/// loaded as the card is.
#[derive(Clone, Copy, Debug)]
struct Asked {
    /// The card's own point, whose last minutes are the card's own.
    own: bool,
    /// The card brings a download down.
    loaded: bool,
}

/// What's known of how an access point carries from here, as loaded as
/// the card is now, so the chooser weighs like with like: what it carried
/// in the span, else what it's learned to carry under load, loaded, or at
/// this hour, not. For the card's own point, what's learned alone, as
/// what it usually carries. Under load, one learned to carry badly idle
/// and never learned under load is known by that, since a radio slow
/// idle is no quicker bringing a download down, and isn't tried blind.
/// The card's own is too, where slow, so a download queued on it weighs
/// another against that floor rather than its minute, lest it leave as at
/// 04:48 on 19 Nov 2026: a floor is the least it carries loaded, so
/// another beating it by the quicker margin beats it loaded too. One
/// quick idle but lossy is no floor, and would hold the download however
/// slow. The chooser leaves a floor out of a trial's bar, where it would
/// send a download back for carrying worse than no download could.
fn known<'a>(
    lately: &'a Measures,
    learned: &'a Learned,
    (now, hour): (Instant, u8),
) -> impl Fn(&str, i16, Asked) -> Option<(Measured, Whence)> + 'a {
    move |point, signal, Asked { own, loaded }| {
        let lately = lately
            .of(point, now)
            .filter(|measured| !own && choose::loaded(measured) == loaded)
            .map(|measured| (measured, Whence::Lately));
        let idle = || {
            learned
                .of(point, hour, signal)
                .map(|measured| (measured, Whence::Learned))
        };
        lately.or_else(|| {
            if loaded {
                learned
                    .loaded(point, signal)
                    .map(|measured| (measured, Whence::Loaded))
                    .or_else(|| {
                        idle().filter(|(measured, _)| {
                            if own {
                                choose::slow(measured)
                            } else {
                                choose::badly(measured)
                            }
                        })
                    })
            } else {
                idle()
            }
        })
    }
}

/// What's known of an access point heard this loud, as asked, and whence.
type Known<'a> = dyn Fn(&str, i16, Asked) -> Option<(Measured, Whence)> + 'a;

/// Asks a card's chooser where it should be, from what the access point
/// it's on carried, what it hears of its network and what's known of each,
/// and moves it there as the roam act does. What it hears is read here,
/// less a point that warned it drops the card (`State::around_to_choose`),
/// so no caller can hand the chooser a point that just shed the card.
pub fn choose(
    chooser: &mut Chooser,
    (now, hour): (Instant, u8),
    (on, measured): (&Joined, &Measured),
    (lately, learned): (&Measures, &Learned),
    (asks, air, looked): (
        &mpsc::Sender<Ask>,
        &watch::Receiver<State>,
        &mpsc::Sender<String>,
    ),
    call: bool,
) {
    let known = known(lately, learned, (now, hour));
    let (around, warned) = {
        let air = air.borrow();
        let warned = air.warned_from(&on.card, now).map(str::to_owned);
        (air.around_to_choose(&on.card, now), warned)
    };
    let choosable = (&*around, warned.as_deref());
    match chosen(chooser, now, (on, measured), choosable, &known, call) {
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
    (around, warned): (&[Heard], Option<&str>),
    known: &Known<'_>,
    call: bool,
) -> Option<Next> {
    let here = around.iter().find(|heard| heard.bssid == on.point)?;
    let loaded = choose::loaded(measured);
    // The card's own point goes along too, since how busy it says its
    // channel is tells the chooser whether there's more room elsewhere.
    let (others, whence): (Vec<Other>, Vec<Option<Whence>>) = around
        .iter()
        .map(|heard| {
            let own = heard.bssid == on.point;
            let known = known(&heard.bssid, heard.signal, Asked { own, loaded });
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
    for aside in asides(chooser, on, measured, here.signal) {
        println!("<6>{aside}");
    }
    match choice {
        Choice::Stay => None,
        Choice::Try(to) => {
            let at = others.iter().position(|other| other.point == to)?;
            let why = tried(on, from, &others[at], whence[at]);
            println!("<6>{why}");
            Some(Next::Move(to))
        }
        // A trial's way back is to the point it began on, which the
        // chooser keeps itself and may since have warned it drops the
        // card, so it isn't taken to that one. One merely unheard is gone
        // back to all the same, as the roam looks for it: by a trial's end
        // wpa_supplicant has often forgotten the point left. Why it would
        // have gone is taken all the same, so it isn't said of a later way
        // back.
        Choice::Back(to) if warned.is_some_and(|point| point.eq_ignore_ascii_case(&to)) => {
            chooser.went_back();
            println!(
                "<6>{} stays on {} rather than go back to {to}, which warned it drops the card",
                on.card, on.point
            );
            None
        }
        Choice::Back(to) => {
            println!("<6>{}", said_back(chooser, now, (on, measured), &to));
            Some(Next::Move(to))
        }
        Choice::Look => {
            // A card held on its wider band looks while it hears narrower
            // radios, which it may not take.
            let hears = if others.iter().any(|other| other.point != on.point) {
                "none of its network it may take off its wider band"
            } else {
                "no other of its network"
            };
            println!(
                "<6>{} looks for another access point, as {} carried {} at p95 with {:.1}% \
                 lost, and it hears {hears}",
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

/// What the journal says beside the choice itself, each said once: an
/// access point kept off as a move never reached it, a narrower one a
/// wider band holding well keeps off, a failing link held as the card was
/// just sent back there, a keep-off giving way to a failing link, and one
/// a trial standing as the usual ended early.
fn asides(chooser: &mut Chooser, on: &Joined, measured: &Measured, signal: i16) -> Vec<String> {
    let mut said = Vec::new();
    if let Some((unreached, lasts)) = chooser.unreached() {
        said.push(kept_off(on, &unreached, lasts));
    }
    if let Some(narrower) = chooser.began_keeping_wide() {
        said.push(kept_wide(on, measured, signal, &narrower));
    }
    if let Some(point) = chooser.held() {
        said.push(format!(
            "{} stays on {point} though it loses {:.1}%, as it was sent back there under {} s \
             ago and echoes lost on the way may be among them",
            on.card,
            measured.lost,
            measure::SPAN.as_secs()
        ));
    }
    if let Some(point) = chooser.yielded() {
        said.push(format!(
            "{} may try {point} though it kept it off, as its link on {} has failed a whole \
             span",
            on.card, on.point
        ));
    }
    if let Some((point, from)) = chooser.freed() {
        said.push(format!(
            "{} no longer keeps off {point}, as it carried as {from} usually does",
            on.card
        ));
    }
    said
}

/// Why a card keeps off an access point it was moved to: it's somewhere
/// else since, whether the move failed or something moved it on, and the
/// chooser can't tell which. A keep-off longer than the hour a miss holds
/// was there before it, and runs on, but holds against a failing link for
/// that hour alone (`choose::keep_off_unreached`), which the line says.
fn kept_off(on: &Joined, unreached: &str, lasts: Duration) -> String {
    let firmly = firm_for(lasts, &on.point);
    format!(
        "{} keeps off {unreached} for {}, as it's on {} rather than there since it was \
         moved{firmly}",
        on.card,
        how_long(lasts),
        on.point
    )
}

/// What a keep-off longer than the hour it holds against a failing link
/// says of that hour (`choose::SPURNED`), the same wherever it's said, so
/// one search finds when a failing link `on` may next leave for the point.
/// It's judged to the minute the journal says lengths in, so one with 60
/// minutes and some seconds left says nothing it would read twice.
fn firm_for(lasts: Duration, on: &str) -> String {
    let minutes = |lasts: Duration| lasts.as_secs() / 60;
    if minutes(lasts) <= minutes(choose::SPURNED) {
        return String::new();
    }
    format!(
        ", or for {} should {on} fail a whole span",
        how_long(choose::SPURNED)
    )
}

/// A keep-off's length as the journal says it: in minutes under two
/// hours, and in hours from there, with the minutes past them, as one set
/// at a back is a whole number of hours and what's left of it when a
/// move misses its point is not.
fn how_long(lasts: Duration) -> String {
    let minutes = lasts.as_secs() / 60;
    if minutes < 120 {
        return format!("{minutes} minutes");
    }
    let (hours, past) = (minutes / 60, minutes % 60);
    match past {
        0 => format!("{hours} hours"),
        1 => format!("{hours} hours and 1 minute"),
        _ => format!("{hours} hours and {past} minutes"),
    }
}

/// Why a card keeps to its wider band where it would once have tried a
/// narrower one, heard as the hold heard it, and its middle echo, which
/// past `STEADY` at rest would have let it go.
fn kept_wide(on: &Joined, measured: &Measured, signal: i16, narrower: &str) -> String {
    format!(
        "{} keeps to {} on its wider band rather than try {narrower}, as it loses {:.1}% \
         heard at {} dBm, its middle {}",
        on.card,
        on.point,
        measured.lost,
        choose::heard(measured, signal),
        measure::ms(measured.p50)
    )
}

/// What the journal says of a card the chooser has just sent back from the
/// point it tried: why, and whether it now keeps that point off.
fn said_back(
    chooser: &mut Chooser,
    now: Instant,
    (on, measured): (&Joined, &Measured),
    to: &str,
) -> String {
    let why = chooser.went_back();
    let kept = chooser.kept_for(&on.point, now);
    went_back(on, measured, to, (why.as_ref(), kept))
}

/// Why a card goes back from a trial: what the point tried carried, and
/// what it was judged against, so a trial judged on what the point left
/// usually carries can be read from the journal, and whether the card
/// keeps off the point tried.
fn went_back(
    on: &Joined,
    measured: &Measured,
    to: &str,
    (why, kept): (Option<&Against>, Option<Duration>),
) -> String {
    let carried = format!(
        "{} carried {} at p95 with {:.1}% lost",
        on.point,
        measure::ms(measured.p95),
        measured.lost
    );
    let spurned = kept.map_or_else(String::new, |lasts| {
        format!("; it keeps off {} for {}", on.point, how_long(lasts))
    });
    // A keep-off for carrying worse holds against a failing link for its
    // first hour alone (choose.rs), which a longer one says.
    let firmly = format!(
        "{spurned}{}",
        kept.map_or_else(String::new, |lasts| firm_for(lasts, to))
    );
    let because = match why {
        Some(Against::Worse(than)) => format!(
            "{carried}, worse than the {} with {:.1}% lost {to} carried before{firmly}",
            measure::ms(than.p95),
            than.lost
        ),
        Some(Against::Usually(than)) => {
            // The usual is as loaded as the minute: under load what the
            // point carried loaded at any hour, else idle at this hour.
            let when = if choose::loaded(measured) {
                "under load"
            } else {
                "at this hour"
            };
            // A failing link leaves for the point tried once a whole span
            // on whatever point the card is on says so (choose.rs).
            let after = if kept.is_some() {
                format!(
                    "{spurned} unless the card's link fails a whole span, having gone \
                     back from it once already"
                )
            } else {
                format!(
                    "; it may try {} again once settled, or sooner should {to} fail a whole span",
                    on.point
                )
            };
            format!(
                "{carried}, worse than the {} {to} usually carries {when} at p95, with the \
                 {:.1}% lost it carried before{after}",
                measure::ms(than.p95),
                than.lost
            )
        }
        Some(Against::Slowed(before)) => format!(
            "{} moved {} Mbit/s under load, where {to} moved {before}{firmly}",
            on.point,
            measured.moving.unwrap_or(0)
        ),
        Some(Against::Badly) => format!("{carried}, badly for a wider band's room{firmly}"),
        None => carried,
    };
    format!("{} goes back to {to}, as {because}", on.card)
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
        (Some(known), Some(Whence::Loaded)) => {
            format!(
                "carried {} at p95 under load before",
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
            // The slow air that let it leave: a middle past `STEADY` or a
            // channel past `BUSIEST`.
            let channel =
                busy.map_or_else(String::new, |busy| format!(", its channel {busy}% busy"));
            format!(
                ", on a narrower band, as the card moves little, its middle {}{channel}",
                measure::ms(measured.p50)
            )
        }
        _ if choose::wider(frequency, to.frequency) && choose::loaded(measured) => {
            ", on a narrower band, during a download".to_owned()
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

    use super::*;
    use crate::learn::Here;
    use crate::measure::tests::joined;
    use crate::measure::{Counts, Echo};
    use crate::radio::{Load, Radio};
    use crate::state::Security;

    const DESK: &str = "6c:ae:f6:c1:84:26";
    const HALL: &str = "6c:ae:f6:c1:84:28";
    const NEAR: &str = "6c:ae:f6:c1:84:27";

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

    /// What a card carried bringing little down.
    fn idle(p95: u64) -> Measured {
        Measured {
            moving: Some(10),
            ..carried(p95)
        }
    }

    /// What the hall carried for ten minutes at eight in the evening,
    /// idle.
    fn the_hall_at_eight(dir: &Path, p95: u64) -> Learned {
        let mut learned = Learned::kept_in(&dir.join("carried"));
        hall_carried(&mut learned, 20, &idle(p95));
        learned
    }

    /// Figures asked for another point, idle or under load, and for the
    /// card's own.
    const IDLE: Asked = Asked {
        own: false,
        loaded: false,
    };
    const LOADED: Asked = Asked {
        own: false,
        loaded: true,
    };
    const OWN: Asked = Asked {
        own: true,
        loaded: false,
    };

    /// What the hall carried for ten minutes at this hour.
    fn hall_carried(learned: &mut Learned, hour: u8, measured: &Measured) {
        point_carried(learned, HALL, hour, measured);
    }

    /// What an access point carried for ten minutes at this hour.
    fn point_carried(learned: &mut Learned, point: &str, hour: u8, measured: &Measured) {
        let here = Here {
            network: "Lantern",
            point,
            hour,
            signal: -50,
            trusted: true,
        };
        for _ in 0..10 {
            learned.carried(&here, measured).unwrap();
        }
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

        let at_eight = known(&lately, &learned, (Instant::now(), 20));
        let at_nine = known(&lately, &learned, (Instant::now(), 21));

        let (hall, whence) = at_eight(HALL, -50, IDLE).expect("the hall was learned at eight");
        assert_eq!(hall.p95, Some(ms(15)), "what it carried then is known");
        assert_eq!(whence, Whence::Learned, "and known as learned");
        assert_eq!(
            at_nine(HALL, -50, IDLE),
            None,
            "nothing was learned at nine"
        );
    }

    #[test]
    fn under_load_an_access_point_is_known_by_what_it_carried_under_load() {
        let dir = tempfile::tempdir().unwrap();
        let mut learned = the_hall_at_eight(dir.path(), 15);
        hall_carried(&mut learned, 3, &carried(80));
        let lately = Measures::default();

        let known = known(&lately, &learned, (Instant::now(), 20));

        let (hall, whence) = known(HALL, -50, LOADED).expect("the hall was learned under load");
        assert_eq!(
            hall.p95,
            Some(ms(80)),
            "what it carried under load, at any hour"
        );
        assert_eq!(whence, Whence::Loaded, "and known as learned under load");
        let (hall, _) = known(HALL, -50, IDLE).expect("the hall was learned idle");
        assert_eq!(hall.p95, Some(ms(15)), "idle, what it carried idle");
        let own = Asked {
            own: true,
            ..LOADED
        };
        let (hall, whence) = known(HALL, -50, own).expect("the card's own was learned");
        assert_eq!(hall.p95, Some(ms(80)), "the card's own under load too");
        assert_eq!(whence, Whence::Loaded);
    }

    #[test]
    fn under_load_an_access_point_learned_to_carry_badly_idle_is_known_by_it() {
        let dir = tempfile::tempdir().unwrap();
        let learned = the_hall_at_eight(dir.path(), 119);
        let lately = Measures::default();

        let known = known(&lately, &learned, (Instant::now(), 20));

        let (hall, whence) = known(HALL, -50, LOADED).expect("119 ms idle is slow loaded");
        assert_eq!(hall.p95, Some(ms(119)), "so it isn't tried as untried");
        assert_eq!(whence, Whence::Learned);
        let own = Asked {
            own: true,
            ..LOADED
        };
        let (own, _) = known(HALL, -50, own).expect("the card's own is known by it too");
        assert_eq!(
            own.p95,
            Some(ms(119)),
            "so a download queued on it weighs another by its floor, not the minute"
        );
    }

    #[test]
    fn under_load_a_point_quick_idle_but_lossy_is_kept_off_yet_no_floor_of_its_own() {
        let dir = tempfile::tempdir().unwrap();
        let mut learned = Learned::kept_in(&dir.path().join("carried"));
        let lossy = Measured {
            lost: 3.0,
            ..idle(15)
        };
        hall_carried(&mut learned, 20, &lossy);
        let lately = Measures::default();

        let known = known(&lately, &learned, (Instant::now(), 20));

        let (hall, _) = known(HALL, -50, LOADED).expect("a lossy point isn't tried blind");
        assert_eq!(hall.p95, Some(ms(15)), "so it's known by what it lost");
        let own = Asked {
            own: true,
            ..LOADED
        };
        assert_eq!(
            known(HALL, -50, own),
            None,
            "but 15 ms is no floor on a download, which it would hold however slow"
        );
    }

    #[test]
    fn under_load_the_cards_own_idle_figure_is_a_floor_only_past_slow() {
        let floor = |p95| {
            let dir = tempfile::tempdir().unwrap();
            let learned = the_hall_at_eight(dir.path(), p95);
            let lately = Measures::default();
            let known = known(&lately, &learned, (Instant::now(), 20));
            let own = Asked {
                own: true,
                ..LOADED
            };
            known(HALL, -50, own).and_then(|(measured, _)| measured.p95)
        };

        assert_eq!(floor(40), None, "40 ms idle is no floor");
        assert_eq!(floor(41), Some(ms(41)), "41 is");
    }

    #[test]
    fn under_load_nothing_learned_idle_is_gone_by() {
        let dir = tempfile::tempdir().unwrap();
        let learned = the_hall_at_eight(dir.path(), 15);
        let lately = Measures::default();

        let known = known(&lately, &learned, (Instant::now(), 20));

        assert_eq!(
            known(HALL, -50, LOADED),
            None,
            "15 ms idle says nothing of a download"
        );
    }

    #[test]
    fn what_an_access_point_carried_lately_under_another_load_is_not_gone_by() {
        let dir = tempfile::tempdir().unwrap();
        let learned = the_hall_at_eight(dir.path(), 15);
        let mut lately = Measures::default();
        let start = Instant::now();
        for second in 0..100 {
            let at = start + Duration::from_secs(second);
            lately.echoed(HALL, at, Echo::Answered(ms(40)));
            let counts = Counts {
                sent: 0,
                retries: 0,
                failed: 0,
                signal: Some(-50),
                moved: Some(second * 12_500_000),
                link: None,
            };
            lately.counted(HALL, at, counts);
        }

        let known = known(&lately, &learned, (start + Duration::from_secs(99), 20));

        let (hall, whence) = known(HALL, -50, IDLE).expect("the hall was learned idle");
        assert_eq!(hall.p95, Some(ms(15)), "40 ms moving 100 Mbit/s isn't idle");
        assert_eq!(whence, Whence::Learned);
        let (hall, whence) = known(HALL, -50, LOADED).expect("the hall was measured lately");
        assert_eq!(
            hall.p95,
            Some(ms(40)),
            "under load it's what it carried lately"
        );
        assert_eq!(whence, Whence::Lately);
    }

    #[test]
    fn the_card_s_own_access_point_is_known_by_what_it_usually_carries() {
        let dir = tempfile::tempdir().unwrap();
        let learned = the_hall_at_eight(dir.path(), 15);
        let mut lately = Measures::default();
        let start = Instant::now();
        for second in 0..100 {
            let at = start + Duration::from_secs(second);
            lately.echoed(HALL, at, Echo::Answered(ms(40)));
        }

        let known = known(&lately, &learned, (start + Duration::from_secs(99), 20));

        let (hall, whence) = known(HALL, -50, OWN).expect("the hall was learned");
        assert_eq!(
            hall.p95,
            Some(ms(15)),
            "its last two minutes are the card's own"
        );
        assert_eq!(whence, Whence::Learned);
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

        let known = known(&lately, &learned, (start + Duration::from_secs(99), 20));

        let (hall, whence) = known(HALL, -50, IDLE).expect("the hall was measured lately");
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
                &other(HALL, quick.clone()),
                Some(Whence::Loaded)
            ),
            format!("{why} carried 10.0 ms at p95 under load before"),
            "a move made on what was learned under load says so"
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
            p50: Some(ms(15)),
            moving: Some(24),
            ..carried(30)
        };
        let bad = Measured {
            moving: Some(24),
            ..carried(52)
        };

        let downloading = Measured {
            moving: Some(52),
            ..carried(136)
        };

        let why = tried(&on, (&resting, 5260, None), &narrow, None);
        let busy = tried(&on, (&resting, 5260, Some(54)), &narrow, None);
        let failing = tried(&on, (&bad, 5260, None), &narrow, None);
        let loaded = tried(
            &on,
            (&downloading, 5260, None),
            &narrow,
            Some(Whence::Loaded),
        );

        let alone = "two minutes, on a narrower band, as the card moves little, its middle 15.0 ms";
        assert!(why.ends_with(alone), "says why it left: {why}");
        assert!(
            busy.ends_with("its middle 15.0 ms, its channel 54% busy"),
            "says how busy its channel was: {busy}"
        );
        assert!(!failing.contains("band"), "it moves as bad: {failing}");
        let queued = "under load before, on a narrower band, during a download";
        assert!(
            loaded.ends_with(queued),
            "says it left mid-download: {loaded}"
        );
    }

    #[test]
    fn a_card_keeping_to_its_wider_band_says_what_it_kept_off_and_why() {
        let quick_middle = Measured {
            p50: Some(ms(5)),
            ..carried(34)
        };
        let why = kept_wide(&joined(DESK), &quick_middle, -38, HALL);

        assert!(
            why.ends_with(&format!(
                "keeps to {DESK} on its wider band rather than try {HALL}, as it loses 0.0% \
                 heard at -38 dBm, its middle 5.0 ms"
            )),
            "a wider band kept to says what and why: {why}"
        );
        let fainter = Measured {
            signal: Some(-61),
            ..carried(34)
        };
        let why = kept_wide(&joined(DESK), &fainter, -38, HALL);
        assert!(
            why.contains("heard at -61 dBm,"),
            "it says the station's own signal where fainter than the scan's: {why}"
        );
    }

    #[test]
    fn a_card_going_back_says_what_judged_its_trial_and_how_long_it_keeps_off() {
        let on = joined(HALL);
        let before = Measured {
            lost: 1.5,
            ..carried(76)
        };
        let hour = Some(choose::SPURNED);
        let line = |why: &Against| went_back(&on, &idle(50), DESK, (Some(why), hour));
        let usual = Measured {
            p95: Some(ms(14)),
            ..before.clone()
        };

        let worse = line(&Against::Worse(before));
        let usually_again = line(&Against::Usually(usual.clone()));
        let loaded_usually = went_back(
            &on,
            &carried(50),
            DESK,
            (Some(&Against::Usually(usual.clone())), None),
        );
        let usually = went_back(&on, &idle(50), DESK, (Some(&Against::Usually(usual)), None));
        let cut = Measured {
            moving: Some(30),
            ..carried(50)
        };
        let slowed = went_back(&on, &cut, DESK, (Some(&Against::Slowed(52)), hour));
        let badly = line(&Against::Badly);

        let carried =
            format!("goes back to {DESK}, as {HALL} carried 50.0 ms at p95 with 0.0% lost");
        let kept = format!("; it keeps off {HALL} for 60 minutes");
        assert!(
            worse.ends_with(&format!(
                "{carried}, worse than the 76.0 ms with 1.5% lost {DESK} carried before{kept}"
            )),
            "a worse trial says what it left carried, and its hour off: {worse}"
        );
        let usual_line = |when: &str| {
            format!(
                "{carried}, worse than the 14.0 ms {DESK} usually carries {when} at p95, with \
                 the 1.5% lost it carried before"
            )
        };
        assert!(
            usually.ends_with(&format!(
                "{}; it may try {HALL} again once settled, or sooner should {DESK} fail a whole \
                 span",
                usual_line("at this hour")
            )),
            "a trial judged on the usual says so, and that it isn't kept off: {usually}"
        );
        assert!(
            loaded_usually.contains(&usual_line("under load")),
            "a download's usual says it was learned under load: {loaded_usually}"
        );
        assert!(
            usually_again.ends_with(&format!(
                "{}{kept} unless the card's link fails a whole span, having gone back from it \
                 once already",
                usual_line("at this hour")
            )),
            "a second back on the usual says it keeps off the point: {usually_again}"
        );
        assert!(
            slowed.ends_with(&format!(
                "goes back to {DESK}, as {HALL} moved 30 Mbit/s under load, where {DESK} \
                 moved 52{kept}"
            )),
            "a download cut says what each moved: {slowed}"
        );
        assert!(
            badly.ends_with(&format!("{carried}, badly for a wider band's room{kept}")),
            "a trial for a wider band's room says it carried badly there: {badly}"
        );
    }

    #[test]
    fn a_card_going_back_from_a_point_found_worse_again_says_its_longer_keep_off() {
        let (mut chooser, again) = choose::tests::found_worse_twice(Instant::now());

        let line = said_back(&mut chooser, again, (&joined(DESK), &idle(775)), NEAR);

        assert!(
            line.ends_with(&format!(
                "; it keeps off {DESK} for 2 hours, or for 60 minutes should {NEAR} fail a \
                 whole span"
            )),
            "a second keep-off running isn't said as the chooser keeps it: {line}"
        );
    }

    #[test]
    fn a_keep_off_is_said_in_minutes_under_two_hours_and_hours_from_there() {
        let said = [3600, 5400, 7199, 7200, 7260, 25_080, 43_200]
            .map(|secs| how_long(Duration::from_secs(secs)));

        assert_eq!(
            said,
            [
                "60 minutes",
                "90 minutes",
                "119 minutes",
                "2 hours",
                "2 hours and 1 minute",
                "6 hours and 58 minutes",
                "12 hours"
            ],
            "a keep-off's length is misread"
        );
    }

    #[test]
    fn a_card_sent_back_says_why_and_keeps_off_the_point_it_tried() {
        let (mut chooser, again) = choose::tests::back_twice_on_the_usual(Instant::now());
        let line = said_back(&mut chooser, again, (&joined(HALL), &idle(50)), DESK);

        assert!(
            line.contains("usually carries at this hour")
                && line.contains(&format!("; it keeps off {HALL} for 60 minutes unless")),
            "a second back on the usual says so, and keeps the hall it tried off: {line}"
        );
        assert!(chooser.went_back().is_none(), "why is said once");
    }

    #[test]
    fn a_failing_link_held_after_a_back_and_a_keep_off_giving_way_are_each_said_once() {
        let (mut chooser, again) = choose::tests::back_twice_on_the_usual(Instant::now());
        chooser.went_back();
        let failing = Measured {
            lost: 15.0,
            ..idle(400)
        };
        let around = [
            Other {
                frequency: 5260,
                ..other(DESK, None)
            },
            Other {
                frequency: 5260,
                ..other(HALL, None)
            },
        ];
        let on = joined(DESK);
        let at = (DESK, &failing, -50, 5260);

        let held = chooser.choose(again + Duration::from_secs(60), at, &around, false);
        let held_said = asides(&mut chooser, &on, &failing, -50);
        let held_again = asides(&mut chooser, &on, &failing, -50);
        let yielded = chooser.choose(again + measure::SPAN, at, &around, false);
        let yielded_said = asides(&mut chooser, &on, &failing, -50);
        let yielded_again = asides(&mut chooser, &on, &failing, -50);

        assert_eq!(
            held,
            Choice::Stay,
            "a desk failing just after the back holds"
        );
        assert_eq!(
            held_said,
            [format!(
                "wlan2 stays on {DESK} though it loses 15.0%, as it was sent back there under \
                 120 s ago and echoes lost on the way may be among them"
            )],
            "and says only why"
        );
        assert_eq!(held_again, Vec::<String>::new(), "once");
        assert_eq!(
            yielded,
            Choice::Try(HALL.into()),
            "a whole span on, it tries the hall"
        );
        assert_eq!(
            yielded_said,
            [format!(
                "wlan2 may try {HALL} though it kept it off, as its link on {DESK} has failed \
                 a whole span"
            )],
            "and says only that the keep-off gave way"
        );
        assert_eq!(yielded_again, Vec::<String>::new(), "once");
    }

    #[test]
    fn a_keep_off_a_trial_ends_is_said_once_naming_the_point_freed_and_the_one_left() {
        let mut chooser = choose::tests::hall_freed(Instant::now());
        let quick = idle(20);
        let on = joined(HALL);

        let said = asides(&mut chooser, &on, &quick, -45);
        let again = asides(&mut chooser, &on, &quick, -45);

        assert_eq!(
            said,
            [format!(
                "wlan2 no longer keeps off {HALL}, as it carried as {DESK} usually does"
            )],
            "a trial standing as the desk usually carries frees the hall, and says only that"
        );
        assert_eq!(again, Vec::<String>::new(), "once");
    }

    #[test]
    fn a_card_keeping_off_a_point_it_was_moved_to_says_where_it_is() {
        let why = kept_off(&joined(HALL), DESK, choose::SPURNED);
        // What's left of an eight-hour keep-off when a move made a span
        // after its first hour misses, as the chooser gives it, from the
        // desk's 2.4 GHz radio its card was on.
        let (mut chooser, _, _) = choose::tests::missed_ch52_through_its_keep_off(Instant::now());
        let (missed, left) = chooser.unreached().expect("the chooser said no miss");
        let longer = kept_off(&joined(NEAR), &missed, left);
        // A few seconds past the hour reads as the hour itself, so says it
        // once.
        let hour_and_a_bit = choose::SPURNED + Duration::from_secs(30);
        let barely = kept_off(&joined(HALL), DESK, hour_and_a_bit);
        // A minute past the hour is past it, though not yet two hours.
        let past = kept_off(
            &joined(HALL),
            DESK,
            choose::SPURNED + Duration::from_secs(60),
        );

        assert!(
            why.ends_with(&format!(
                "keeps off {DESK} for 60 minutes, as it's on {HALL} rather \
                 than there since it was moved"
            )),
            "a point kept off says how long and where the card is: {why}"
        );
        assert!(
            longer.ends_with(&format!(
                "keeps off {DESK} for 6 hours and 58 minutes, as it's on {NEAR} rather than \
                 there since it was moved, or for 60 minutes should {NEAR} fail a whole span"
            )),
            "a miss through a longer keep-off didn't say what runs on, and what may \
             leave for it after an hour: {longer}"
        );
        assert!(
            barely.ends_with(&format!(
                "keeps off {DESK} for 60 minutes, as it's on {HALL} rather than there \
                 since it was moved"
            )),
            "a keep-off of 60 minutes and some seconds said the hour twice: {barely}"
        );
        assert!(
            past.ends_with(&format!(
                "keeps off {DESK} for 61 minutes, as it's on {HALL} rather than there \
                 since it was moved, or for 60 minutes should {HALL} fail a whole span"
            )),
            "a keep-off of 61 minutes didn't say what may leave for it after an hour: {past}"
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
        let known = known(&lately, &learned, (now, 20));

        let to = chosen(
            &mut Chooser::default(),
            now,
            (&joined(DESK), &carried(9)),
            (&around, None),
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
        let known = known(&lately, &learned, (now, 20));

        let to = chosen(
            &mut Chooser::default(),
            now,
            (&joined(DESK), &idle(30)),
            (&around, None),
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
    fn a_card_bringing_a_download_down_goes_by_nothing_learned_idle() {
        let dir = tempfile::tempdir().unwrap();
        let learned = the_hall_at_eight(dir.path(), 10);
        let (lately, now) = (Measures::default(), Instant::now());
        let around = [heard(DESK, -50), heard(HALL, -50)];
        let known = known(&lately, &learned, (now, 20));
        let downloading = Measured {
            moving: Some(50),
            ..carried(30)
        };

        let to = chosen(
            &mut Chooser::default(),
            now,
            (&joined(DESK), &downloading),
            (&around, None),
            &known,
            false,
        );

        assert_eq!(
            to, None,
            "10 ms idle says nothing of 30 ms moving 50 Mbit/s"
        );
    }

    #[test]
    fn a_card_weighs_5_ghz_against_what_its_own_point_usually_carries() {
        let dir = tempfile::tempdir().unwrap();
        let mut learned = Learned::kept_in(&dir.path().join("carried"));
        point_carried(&mut learned, DESK, 20, &idle(11));
        hall_carried(&mut learned, 20, &idle(20));
        let mut lately = Measures::default();
        let start = Instant::now();
        for second in 0..100 {
            let at = start + Duration::from_secs(second);
            lately.echoed(DESK, at, Echo::Answered(ms(26)));
        }
        let now = start + Duration::from_secs(99);
        let wide = Heard {
            frequency: 5260,
            ..heard(HALL, -50)
        };
        let around = [heard(DESK, -50), wide];
        let known = known(&lately, &learned, (now, 20));

        let to = chosen(
            &mut Chooser::default(),
            now,
            (&joined(DESK), &idle(26)),
            (&around, None),
            &known,
            false,
        );

        assert_eq!(to, None, "the desk usually carries 11 ms, and 5 GHz 20");
    }

    #[test]
    fn a_card_carrying_badly_and_hearing_no_other_looks() {
        let dir = tempfile::tempdir().unwrap();
        let learned = Learned::kept_in(&dir.path().join("carried"));
        let (lately, now) = (Measures::default(), Instant::now());
        let around = [heard(DESK, -50)];
        let known = known(&lately, &learned, (now, 20));

        let next = chosen(
            &mut choose::tests::after_a_bad_minute(DESK),
            now,
            (&joined(DESK), &carried(52)),
            (&around, None),
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
        let known = known(&lately, &learned, (now, 20));

        let to = chosen(
            &mut Chooser::default(),
            now,
            (&joined(DESK), &carried(30)),
            (&around, None),
            &known,
            false,
        );

        assert_eq!(to, None, "30 ms is carrying well, and the hall is unknown");
    }

    #[test]
    fn a_trial_goes_back_to_a_point_unheard_but_not_one_that_warned() {
        let dir = tempfile::tempdir().unwrap();
        let learned = the_hall_at_eight(dir.path(), 10);
        let (lately, start) = (Measures::default(), Instant::now());
        let known = known(&lately, &learned, (start, 20));
        // A trial of the hall, learned far quicker, carrying far worse
        // than the desk it left once there, which the card no longer
        // hears, as wpa_supplicant forgets a point unheard a while, or
        // which warned it drops the card (`State::warned_from`).
        let judged = |warned: Option<&str>| {
            let mut chooser = Chooser::default();
            let tried = chosen(
                &mut chooser,
                start,
                (&joined(DESK), &idle(30)),
                (&[heard(DESK, -50), heard(HALL, -50)], None),
                &known,
                false,
            );
            assert_eq!(tried, Some(Next::Move(HALL.into())), "the hall is tried");
            let at = start + choose::TRIAL;
            let next = chosen(
                &mut chooser,
                at,
                (&joined(HALL), &idle(200)),
                (&[heard(HALL, -50)], warned),
                &known,
                false,
            );
            (next, chooser.went_back())
        };

        let (back, _) = judged(None);
        let (kept, why) = judged(Some(&DESK.to_uppercase()));

        assert_eq!(
            back,
            Some(Next::Move(DESK.into())),
            "a trial carrying 200 ms against the 30 it left didn't go back to it unheard"
        );
        assert_eq!(
            kept, None,
            "a trial went back to a point that warned it drops the card"
        );
        assert!(
            why.is_none(),
            "why a way back not taken was left to be said of a later one"
        );
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
        let known = known(&lately, &learned, (now, 20));

        let to = chosen(
            &mut Chooser::default(),
            now,
            (&joined(HALL), &carried(38)),
            (&around, None),
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
        let known = known(&lately, &learned, (now, 20));

        let next = chosen(
            &mut Chooser::default(),
            now,
            (&joined(HALL), &carried(38)),
            (&[crowded], None),
            &known,
            false,
        );

        assert_eq!(next, Some(Next::Look), "56% busy with nothing else heard");
    }
}
