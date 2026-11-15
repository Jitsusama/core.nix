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
//! minute.

use std::collections::{HashMap, VecDeque};

use tokio::sync::watch;
use tokio::time::{Duration, Instant, MissedTickBehavior, interval};

use crate::echo::Echoer;
use crate::nl80211::{Nl80211, Station};
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
/// and logs each one's figures every minute.
pub async fn follow(state: watch::Receiver<State>) {
    let mut echoers: HashMap<String, Echoer> = HashMap::new();
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
                let (now, joined) = (Instant::now(), state.borrow().joined());
                for on in joined {
                    if let Some(measured) = measures.of(&on.point, now) {
                        println!("<6>{}", said(&on, &measured));
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

/// An access point's figures, as logged.
fn said(on: &Joined, measured: &Measured) -> String {
    let ms = |time: Option<Duration>| {
        time.map_or("none".into(), |time| {
            format!("{:.1} ms", time.as_secs_f64() * 1000.0)
        })
    };
    let signal = measured
        .signal
        .map_or("unread".into(), |signal| format!("{signal} dBm"));
    format!(
        "{} on {}: gateway {} at p50, {} at p95, {:.1}% of {} echoes lost; \
         {:.2} in 100 frames sent again, {:.2} lost; signal {signal}",
        on.card,
        on.point,
        ms(measured.p50),
        ms(measured.p95),
        measured.lost,
        measured.echoes,
        measured.retried,
        measured.failed,
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
    use super::*;

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
        let on = Joined {
            card: "wlan2".into(),
            point: DESK.into(),
            gateway: std::net::Ipv4Addr::new(10, 43, 0, 1),
        };
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
            said(&on, &measured),
            "wlan2 on 6c:ae:f6:c1:84:26: gateway 5.1 ms at p50, 15.0 ms at p95, 0.0% of 120 \
             echoes lost; 1.25 in 100 frames sent again, 0.00 lost; signal -47 dBm",
            "one line says all of it"
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
}
