//! How each access point does from where the machine is now, over the last
//! two minutes: how fast and how often its network's gateway answers, and
//! how many of the card's frames it had to send again or lost.
//!
//! An access point's signal says how loud it is, not how well it carries:
//! on 14 Nov 2026 two radios of one access point, at one desk, heard within
//! 4 dB of each other, answered the gateway at a p95 of 15 ms and 119 ms.
//! So wifid measures the one it's on, and keeps what it measured of each,
//! to choose between them. Each card echoes its gateway once a second, as
//! a sampler did to find that, and logs its access point's figures each
//! minute. Each minute too it asks the card's chooser (choose.rs) whether
//! another of the network's access points would carry better, and moves
//! the card as the roam act does. What each carried is learned by the hour
//! too (learn.rs), for those the card hasn't been on in the span.

use std::collections::{HashMap, VecDeque};

use tokio::sync::{mpsc, watch};
use tokio::time::{Duration, Instant, MissedTickBehavior, interval};

use crate::acts;
use crate::air::Heard;
use crate::calling::Calling;
use crate::choose::{Choice, Chooser, Other};
use crate::echo::Echoer;
use crate::learn::{self, Here, Learned};
use crate::nl80211::{Nl80211, Station};
use crate::service::Ask;
use crate::state::{Joined, State};

/// How far back an access point's figures reach: long enough for a p95 of
/// one echo a second to mean something, short enough to follow a move to
/// another room.
pub const SPAN: Duration = Duration::from_secs(120);

/// What the gateway said to one echo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Echo {
    Answered(Duration),
    Lost,
}

/// What the kernel says of a card's station: counts since it joined.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Counts {
    pub sent: u32,
    pub retries: u32,
    pub failed: u32,
    pub signal: Option<i32>,
}

/// An access point's figures over the span.
#[derive(Clone, Debug, PartialEq)]
pub struct Measured {
    pub echoes: usize,
    pub p50: Option<Duration>,
    pub p95: Option<Duration>,
    /// The share of echoes lost, in percent.
    pub lost: f64,
    /// Frames sent again and frames lost, per hundred sent.
    pub retried: f64,
    pub failed: f64,
    pub signal: Option<i32>,
}

/// What's been measured of each access point, by its address.
#[derive(Default)]
pub struct Measures {
    points: HashMap<String, Point>,
}

#[derive(Default)]
struct Point {
    echoes: VecDeque<(Instant, Echo)>,
    counts: VecDeque<(Instant, Counts)>,
}

impl Measures {
    /// Takes in an echo of the gateway, sent through the access point.
    pub fn echoed(&mut self, point: &str, at: Instant, echo: Echo) {
        let kept = self.points.entry(point.to_owned()).or_default();
        kept.echoes.push_back((at, echo));
        forget_before(&mut kept.echoes, at);
    }

    /// Takes in the station's counts, read on the access point.
    pub fn counted(&mut self, point: &str, at: Instant, counts: Counts) {
        let kept = self.points.entry(point.to_owned()).or_default();
        kept.counts.push_back((at, counts));
        forget_before(&mut kept.counts, at);
    }

    /// The access point's figures over the span up to now, if anything was
    /// measured of it in it.
    pub fn of(&self, point: &str, now: Instant) -> Option<Measured> {
        let kept = self.points.get(point)?;
        let echoes: Vec<Echo> = within(&kept.echoes, now).collect();
        let counts: Vec<Counts> = within(&kept.counts, now).collect();
        if echoes.is_empty() && counts.is_empty() {
            return None;
        }
        let mut times: Vec<Duration> = echoes
            .iter()
            .filter_map(|echo| match echo {
                Echo::Answered(time) => Some(*time),
                Echo::Lost => None,
            })
            .collect();
        times.sort_unstable();
        let lost = echoes.len() - times.len();
        let (sent, retries, failed) = sent_since(&counts);
        Some(Measured {
            echoes: echoes.len(),
            p50: percentile(&times, 50),
            p95: percentile(&times, 95),
            lost: per_hundred(lost, echoes.len()),
            retried: per_hundred(retries, sent),
            failed: per_hundred(failed, sent),
            signal: counts.last().and_then(|counts| counts.signal),
        })
    }
}

/// How often each card on a network echoes its gateway and reads its
/// station.
const EACH: Duration = Duration::from_secs(1);

/// How often what's measured is logged.
const TOLD: Duration = Duration::from_secs(60);

/// Measures the access point each card is on, for as long as wifid runs,
/// and logs each one's figures every minute, saying when a call was on,
/// learning from them what each carries at each hour.
pub async fn follow(
    state: watch::Receiver<State>,
    calling: watch::Receiver<Calling>,
    asks: mpsc::Sender<Ask>,
    mut learned: Learned,
) {
    let mut echoers: HashMap<String, Echoer> = HashMap::new();
    let mut choosers: HashMap<String, Chooser> = HashMap::new();
    let mut kernel: Option<Nl80211> = None;
    let mut measures = Measures::default();
    let mut each = interval(EACH);
    each.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut told = interval(TOLD);
    told.set_missed_tick_behavior(MissedTickBehavior::Skip);
    told.tick().await;
    loop {
        tokio::select! {
            _ = each.tick() => {
                let joined = state.borrow().joined();
                echoers.retain(|card, _| joined.iter().any(|on| on.card == *card));
                for on in &joined {
                    measure(on, &mut echoers, &mut kernel, &mut measures).await;
                }
            }
            _ = told.tick() => {
                let (now, hour) = (Instant::now(), learn::hour_here());
                let joined = state.borrow().joined();
                let call = calling.borrow().on(now);
                choosers.retain(|card, _| joined.iter().any(|on| on.card == *card));
                forget_the_forgotten(&mut learned, &state.borrow().saved_names());
                for on in joined {
                    let Some(carried) = measures.of(&on.point, now) else {
                        continue;
                    };
                    println!("<6>{}", said(&on, &carried, call));
                    let around = state.borrow().around(&on.card);
                    learn_from(&mut learned, &on, &carried, &around, hour);
                    let chooser = choosers.entry(on.card.clone()).or_default();
                    let figures = (&on, &carried);
                    let known = known(&measures, &learned, now, hour);
                    if let Some(to) = chosen(chooser, now, figures, &around, &known, call) {
                        tokio::spawn(roam(asks.clone(), state.clone(), on.card, to));
                    }
                }
            }
        }
    }
}

/// Echoes a card's gateway once and reads its station's counts. A card
/// whose socket can't be opened, or errs, is passed over until the next
/// second, with its socket opened again then.
async fn measure(
    on: &Joined,
    echoers: &mut HashMap<String, Echoer>,
    kernel: &mut Option<Nl80211>,
    measures: &mut Measures,
) {
    if !echoers.contains_key(&on.card) {
        match Echoer::open(&on.card) {
            Ok(echoer) => echoers.insert(on.card.clone(), echoer),
            Err(error) => {
                eprintln!("<7>can't echo {}'s gateway: {error}", on.card);
                return;
            }
        };
    }
    if let Some(echoer) = echoers.get_mut(&on.card) {
        match echoer.echo(on.gateway).await {
            Ok(echo) => measures.echoed(&on.point, Instant::now(), echo),
            Err(error) => {
                eprintln!("<7>can't echo {}'s gateway: {error}", on.card);
                echoers.remove(&on.card);
            }
        }
    }
    if let Some(counts) = station(kernel, &on.card).await.as_ref().and_then(counted) {
        measures.counted(&on.point, Instant::now(), counts);
    }
}

/// What the kernel says of the card's station, opening its socket first
/// if none is open; one that errs is let go, to be opened again.
async fn station(kernel: &mut Option<Nl80211>, card: &str) -> Option<Station> {
    if kernel.is_none() {
        *kernel = Nl80211::open().await.ok();
    }
    let reading = kernel.as_mut()?.station(card);
    if let Ok(Ok(station)) = tokio::time::timeout(Duration::from_secs(1), reading).await {
        return station;
    }
    *kernel = None;
    None
}

/// A station's counts, where the kernel says them all.
fn counted(station: &Station) -> Option<Counts> {
    Some(Counts {
        sent: station.sent.packets?,
        retries: station.retries?,
        failed: station.failed?,
        signal: station.signal,
    })
}

/// Folds what the access point a card is on carried into what it's learned
/// to carry at this hour, heard as loud as it's heard now.
fn learn_from(learned: &mut Learned, on: &Joined, carried: &Measured, around: &[Heard], hour: u8) {
    let Some(heard) = around.iter().find(|heard| heard.bssid == on.point) else {
        return;
    };
    let here = Here {
        network: &on.network,
        point: &on.point,
        hour,
        signal: heard.signal,
        trusted: on.trusted,
    };
    if let Err(error) = learned.carried(&here, carried) {
        eprintln!("<4>can't keep what {} carried: {error}", on.point);
    }
}

/// Forgets what the access points of networks no longer saved carried.
/// None saved is taken as none read yet, since nothing can be learned of a
/// network that isn't.
fn forget_the_forgotten(learned: &mut Learned, saved: &[String]) {
    if saved.is_empty() {
        return;
    }
    if let Err(error) = learned.keep_only(saved) {
        eprintln!("<4>can't forget what forgotten networks carried: {error}");
    }
}

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

/// Asks a card's chooser where it should be, from what it's on carried and
/// what it hears of its network, and says why where it moves.
fn chosen(
    chooser: &mut Chooser,
    now: Instant,
    (on, measured): (&Joined, &Measured),
    around: &[Heard],
    known: &Known<'_>,
    call: bool,
) -> Option<String> {
    let signal = around.iter().find(|heard| heard.bssid == on.point)?.signal;
    let (others, whence): (Vec<Other>, Vec<Option<Whence>>) = around
        .iter()
        .filter(|heard| heard.bssid != on.point)
        .map(|heard| {
            let known = known(&heard.bssid, heard.signal);
            let whence = known.as_ref().map(|(_, whence)| *whence);
            let other = Other {
                point: heard.bssid.clone(),
                signal: heard.signal,
                busy: heard.radio.load.map(|load| load.busy),
                measured: known.map(|(measured, _)| measured),
            };
            (other, whence)
        })
        .unzip();
    match chooser.choose(now, (&on.point, measured, signal), &others, call) {
        Choice::Stay => None,
        Choice::Try(to) => {
            let at = others.iter().position(|other| other.point == to)?;
            println!("<6>{}", tried(on, measured, &others[at], whence[at]));
            Some(to)
        }
        Choice::Back(to) => {
            println!(
                "<6>{} goes back to {to}, as {} carried worse",
                on.card, on.point
            );
            Some(to)
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

/// Why a card tries another access point: what the one it's on carried,
/// and what was known of the other, and from where.
fn tried(on: &Joined, measured: &Measured, to: &Other, whence: Option<Whence>) -> String {
    let known = match (&to.measured, whence) {
        (Some(known), Some(Whence::Learned)) => {
            format!("carried {} at p95 at this hour before", ms(known.p95))
        }
        (Some(known), _) => format!("carried {} at p95 in the last two minutes", ms(known.p95)),
        (None, _) => format!("is untried, heard at {} dBm", to.signal),
    };
    format!(
        "{} tries {}, as {} carried {} at p95 with {:.1}% lost, and {} {known}",
        on.card,
        to.point,
        on.point,
        ms(measured.p95),
        measured.lost,
        to.point
    )
}

/// A span of time in milliseconds, as logged.
fn ms(time: Option<Duration>) -> String {
    time.map_or("none".into(), |time| {
        format!("{:.1} ms", time.as_secs_f64() * 1000.0)
    })
}

/// An access point's figures, as logged, and whether a call was on.
fn said(on: &Joined, measured: &Measured, call: bool) -> String {
    let signal = measured
        .signal
        .map_or("unread".into(), |signal| format!("{signal} dBm"));
    format!(
        "{} on {}: gateway {} at p50, {} at p95, {:.1}% of {} echoes lost; \
         {:.2} in 100 frames sent again, {:.2} lost; signal {signal}{}",
        on.card,
        on.point,
        ms(measured.p50),
        ms(measured.p95),
        measured.lost,
        measured.echoes,
        measured.retried,
        measured.failed,
        if call { ", during a call" } else { "" },
    )
}

/// Lets go of what was measured before the span ending at `now`.
fn forget_before<T>(kept: &mut VecDeque<(Instant, T)>, now: Instant) {
    while kept
        .front()
        .is_some_and(|(at, _)| now.duration_since(*at) > SPAN)
    {
        kept.pop_front();
    }
}

/// What was measured in the span ending at `now`.
fn within<T: Copy>(kept: &VecDeque<(Instant, T)>, now: Instant) -> impl Iterator<Item = T> {
    kept.iter()
        .filter(move |(at, _)| now.duration_since(*at) <= SPAN)
        .map(|(_, measured)| *measured)
}

/// The time by which `percent` of the answered echoes had answered, by
/// nearest rank.
fn percentile(sorted: &[Duration], percent: usize) -> Option<Duration> {
    let rank = (sorted.len() * percent).div_ceil(100);
    sorted.get(rank.checked_sub(1)?).copied()
}

/// The frames sent, sent again and lost between each reading and the next.
/// A reading lower than the one before is a new join, whose counts start
/// again, so the pair across it says nothing.
fn sent_since(counts: &[Counts]) -> (usize, usize, usize) {
    let mut totals = (0, 0, 0);
    for pair in counts.windows(2) {
        let [before, after] = pair else { continue };
        let (Some(sent), Some(retries), Some(failed)) = (
            after.sent.checked_sub(before.sent),
            after.retries.checked_sub(before.retries),
            after.failed.checked_sub(before.failed),
        ) else {
            continue;
        };
        totals.0 += usize::try_from(sent).unwrap_or(usize::MAX);
        totals.1 += usize::try_from(retries).unwrap_or(usize::MAX);
        totals.2 += usize::try_from(failed).unwrap_or(usize::MAX);
    }
    totals
}

/// `part` of `whole`, per hundred; none of nothing is none.
fn per_hundred(part: usize, whole: usize) -> f64 {
    let figure = |count: usize| f64::from(u32::try_from(count).unwrap_or(u32::MAX));
    if whole == 0 {
        0.0
    } else {
        figure(part) * 100.0 / figure(whole)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::radio::Radio;
    use crate::state::Security;

    const DESK: &str = "6c:ae:f6:c1:84:26";
    const HALL: &str = "6c:ae:f6:c1:84:28";

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    fn counts(sent: u32, retries: u32, failed: u32) -> Counts {
        Counts {
            sent,
            retries,
            failed,
            signal: Some(-47),
        }
    }

    /// One echo a second for the times given, in milliseconds, starting
    /// at `start`.
    fn echoed(kept: &mut Measures, point: &str, start: Instant, times: &[Option<u64>]) {
        for (second, time) in (0..).zip(times) {
            let echo = time.map_or(Echo::Lost, |time| Echo::Answered(ms(time)));
            kept.echoed(point, start + Duration::from_secs(second), echo);
        }
    }

    #[test]
    fn an_access_point_s_echoes_say_its_middle_and_its_slow_end() {
        let start = Instant::now();
        let mut kept = Measures::default();
        let times: Vec<Option<u64>> = (1..=100).map(Some).collect();
        echoed(&mut kept, DESK, start, &times);
        let measured = kept.of(DESK, start + Duration::from_secs(100));
        let measured = measured.expect("an access point echoed through is measured");
        assert_eq!(measured.echoes, 100, "every echo in the span counts");
        assert_eq!(measured.p50, Some(ms(50)), "half answered by 50 ms");
        assert_eq!(measured.p95, Some(ms(95)), "95 of 100 answered by 95 ms");
        assert!(
            measured.lost.abs() < f64::EPSILON,
            "none lost: {}",
            measured.lost
        );
    }

    #[test]
    fn a_lost_echo_counts_as_lost_and_not_as_slow() {
        let start = Instant::now();
        let mut kept = Measures::default();
        echoed(&mut kept, DESK, start, &[Some(5), None, Some(5), None]);
        let measured = kept.of(DESK, start + Duration::from_secs(4));
        let measured = measured.expect("an access point echoed through is measured");
        assert!(
            (measured.lost - 50.0).abs() < 1e-9,
            "two of four lost: {}",
            measured.lost
        );
        assert_eq!(measured.p95, Some(ms(5)), "the answered ones say how fast");
    }

    #[test]
    fn only_the_span_s_echoes_are_counted() {
        let start = Instant::now();
        let mut kept = Measures::default();
        echoed(&mut kept, DESK, start, &[Some(500)]);
        let later = start + SPAN + Duration::from_secs(10);
        echoed(&mut kept, DESK, later, &[Some(5)]);
        let measured = kept.of(DESK, later + Duration::from_secs(1));
        let measured = measured.expect("an access point echoed through is measured");
        assert_eq!(measured.echoes, 1, "the echo before the span is let go");
        assert_eq!(measured.p95, Some(ms(5)), "and says nothing of now");
    }

    #[test]
    fn each_access_point_is_measured_on_its_own() {
        let start = Instant::now();
        let mut kept = Measures::default();
        echoed(&mut kept, DESK, start, &[Some(5)]);
        echoed(&mut kept, HALL, start, &[Some(120)]);
        let now = start + Duration::from_secs(1);
        let desk = kept.of(DESK, now).expect("the desk's was measured");
        let hall = kept.of(HALL, now).expect("the hall's was measured");
        assert_eq!(desk.p95, Some(ms(5)), "the desk's echo is the desk's");
        assert_eq!(hall.p95, Some(ms(120)), "the hall's echo is the hall's");
    }

    #[test]
    fn an_access_point_s_figures_are_logged_with_its_card_and_address() {
        let on = joined(DESK);
        let measured = Measured {
            echoes: 120,
            p50: Some(Duration::from_micros(5_100)),
            p95: Some(ms(15)),
            lost: 0.0,
            retried: 1.25,
            failed: 0.0,
            signal: Some(-47),
        };

        assert_eq!(
            said(&on, &measured, false),
            "wlan2 on 6c:ae:f6:c1:84:26: gateway 5.1 ms at p50, 15.0 ms at p95, 0.0% of 120 \
             echoes lost; 1.25 in 100 frames sent again, 0.00 lost; signal -47 dBm",
            "one line says all of it"
        );
        assert!(
            said(&on, &measured, true).ends_with("signal -47 dBm, during a call"),
            "a call on is said, since the card stays put for it"
        );
    }

    #[test]
    fn an_access_point_never_measured_says_nothing() {
        let kept = Measures::default();
        assert_eq!(
            kept.of(DESK, Instant::now()),
            None,
            "nothing measured, nothing said"
        );
    }

    #[test]
    fn retries_and_failures_are_per_hundred_frames_sent_in_the_span() {
        let start = Instant::now();
        let mut kept = Measures::default();
        kept.counted(DESK, start, counts(1_000, 10, 1));
        kept.counted(DESK, start + Duration::from_secs(60), counts(3_000, 110, 5));
        let measured = kept.of(DESK, start + Duration::from_secs(60));
        let measured = measured.expect("an access point counted on is measured");
        assert!(
            (measured.retried - 5.0).abs() < 1e-9,
            "100 again of 2,000: {}",
            measured.retried
        );
        assert!(
            (measured.failed - 0.2).abs() < 1e-9,
            "4 lost of 2,000: {}",
            measured.failed
        );
        assert_eq!(measured.signal, Some(-47), "the latest signal is said");
    }

    #[test]
    fn counts_that_start_again_on_a_new_join_are_counted_from_there() {
        let start = Instant::now();
        let mut kept = Measures::default();
        kept.counted(DESK, start, counts(9_000, 900, 90));
        kept.counted(DESK, start + Duration::from_secs(10), counts(100, 1, 0));
        kept.counted(DESK, start + Duration::from_secs(20), counts(1_100, 21, 2));
        let measured = kept.of(DESK, start + Duration::from_secs(20));
        let measured = measured.expect("an access point counted on is measured");
        assert!(
            (measured.retried - 2.0).abs() < 1e-9,
            "20 again of 1,000: {}",
            measured.retried
        );
        assert!(
            (measured.failed - 0.2).abs() < 1e-9,
            "2 lost of 1,000: {}",
            measured.failed
        );
    }

    fn joined(point: &str) -> Joined {
        Joined {
            card: "wlan2".into(),
            point: point.into(),
            gateway: std::net::Ipv4Addr::new(10, 43, 0, 1),
            network: "Lantern".into(),
            trusted: true,
        }
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
        echoed(&mut lately, HALL, start, &[Some(40); 100]);

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
                &carried(30),
                &other(HALL, quick.clone()),
                Some(Whence::Learned)
            ),
            format!("{why} carried 10.0 ms at p95 at this hour before"),
            "a move made on what was learned says so"
        );
        assert_eq!(
            tried(&on, &carried(30), &other(HALL, quick), Some(Whence::Lately)),
            format!("{why} carried 10.0 ms at p95 in the last two minutes"),
            "a move made on what was just measured says so"
        );
        assert_eq!(
            tried(&on, &carried(30), &other(HALL, None), None),
            format!("{why} is untried, heard at -50 dBm"),
            "a move made on signal alone says so"
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

        assert_eq!(to.as_deref(), Some(HALL), "10 ms learned beats 30 ms now");
    }

    #[test]
    fn a_card_carrying_well_stays_where_nothing_is_known_to_carry_better() {
        let dir = tempfile::tempdir().unwrap();
        let learned = Learned::kept_in(&dir.path().join("carried"));
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

        assert_eq!(to, None, "30 ms is carrying well, and the hall is unknown");
    }
}
