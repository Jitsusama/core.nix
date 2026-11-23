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
//! minute. Each minute too it hands what it measured to moves.rs, which
//! asks the card's chooser whether another of the network's access points
//! would carry better and moves the card there. What each carried is
//! learned by the hour too (learn.rs), for those the card hasn't been on
//! in the span.
//!
//! Each second's echo goes twice, once unmarked and once in Wi-Fi's video
//! queue (echo.rs), and the chooser and the hours are handed the worse of
//! the two classes' p50, of their p95s and of their losses, each taken
//! apart, so a point holding a call's queue is passed over however fast
//! its best effort. A video echo answered past its second counts as slow
//! as it took and as lost, as a queue held for seconds answers late, in
//! order. One never answered isn't weighed, since a gateway that drops
//! marked echoes says nothing of the air. The minute's line says best
//! effort's own figures, and the video class's beside them.
//!
//! Long because its cases, near half of it, stay beside the private
//! figures they prove one by one, as every module in wifid keeps them.

use std::collections::{HashMap, VecDeque};
use std::io;
use std::net::Ipv4Addr;

use tokio::sync::{mpsc, watch};
use tokio::task::JoinSet;
use tokio::time::{Duration, Instant, MissedTickBehavior, interval};

use crate::air::Heard;
use crate::calling::Calling;
use crate::choose::{self, Chooser};
use crate::echo::{Class, Echoer, LOST, Late};
use crate::learn::{self, Here, Learned};
use crate::moves;
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
    /// Answered past [`LOST`], in what it took: a held queue's answer.
    Late(Duration),
    Lost,
}

/// What the kernel says of a card's station: counts since it joined.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Counts {
    pub sent: u32,
    pub retries: u32,
    pub failed: u32,
    pub signal: Option<i32>,
    /// Bytes received and sent since the card joined, where the kernel
    /// says both.
    pub moved: Option<u64>,
    /// What the link sends at, in kb/s, where the kernel says.
    pub link: Option<u32>,
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
    /// What the card moved both ways over the span, in Mbit/s, where the
    /// kernel says.
    pub moving: Option<u32>,
    /// The most the card's link sent at in the span, in Mbit/s, where the
    /// kernel says.
    pub room: Option<u32>,
}

/// What an access point's video echoes carried over the span.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Video {
    /// The echoes sent.
    pub echoes: usize,
    /// Of them, those answered, in time or late.
    pub answered: usize,
    /// The answers' middle and slow end, a late one's at what it took.
    pub p50: Option<Duration>,
    pub p95: Option<Duration>,
    /// The share of answers that came past [`LOST`], in percent.
    pub held: f64,
}

/// What's been measured of each access point, by its address.
#[derive(Default)]
pub struct Measures {
    points: HashMap<String, Point>,
}

#[derive(Default)]
struct Point {
    echoes: VecDeque<(Instant, Echo)>,
    video: VecDeque<(Instant, Echo)>,
    counts: VecDeque<(Instant, Counts)>,
}

impl Measures {
    /// Takes in a best-effort echo of the gateway, sent through the access
    /// point: the cases' shorthand, as most echo in best effort alone.
    #[cfg(test)]
    pub fn echoed(&mut self, point: &str, at: Instant, echo: Echo) {
        self.echoed_as(Class::BestEffort, point, at, echo);
    }

    /// Takes in an echo of the gateway sent in the class given.
    pub fn echoed_as(&mut self, class: Class, point: &str, at: Instant, echo: Echo) {
        let kept = self.points.entry(point.to_owned()).or_default();
        let echoes = match class {
            Class::BestEffort => &mut kept.echoes,
            Class::Video => &mut kept.video,
        };
        echoes.push_back((at, echo));
        forget_before(echoes, at);
    }

    /// Takes in a late answer to a video echo counted lost, on whichever
    /// access point it was sent through.
    pub fn answered_late(&mut self, late: Late) {
        for point in self.points.values_mut() {
            let sent = point
                .video
                .iter_mut()
                .find(|(at, echo)| *at == late.sent && *echo == Echo::Lost);
            if let Some((_, echo)) = sent {
                *echo = Echo::Late(late.took);
                return;
            }
        }
    }

    /// What the access point's video echoes carried over the span up to
    /// now, if any were sent in it.
    pub fn video_of(&self, point: &str, now: Instant) -> Option<Video> {
        let echoes: Vec<Echo> = within(&self.points.get(point)?.video, now).collect();
        video(&echoes)
    }

    /// Takes in the station's counts, read on the access point.
    pub fn counted(&mut self, point: &str, at: Instant, counts: Counts) {
        let kept = self.points.entry(point.to_owned()).or_default();
        kept.counts.push_back((at, counts));
        forget_before(&mut kept.counts, at);
    }

    /// The access point's figures over the span up to now, each the worse
    /// class's, if anything was measured of it in it.
    pub fn of(&self, point: &str, now: Instant) -> Option<Measured> {
        self.figures(point, now, true)
    }

    /// The access point's figures over the span up to now in best effort
    /// alone, as logged, if anything was measured of it in it.
    pub fn best_effort_of(&self, point: &str, now: Instant) -> Option<Measured> {
        self.figures(point, now, false)
    }

    fn figures(&self, point: &str, now: Instant, weighing_video: bool) -> Option<Measured> {
        let kept = self.points.get(point)?;
        let echoes: Vec<Echo> = within(&kept.echoes, now).collect();
        let counts: Vec<Counts> = within(&kept.counts, now).collect();
        if echoes.is_empty() && counts.is_empty() {
            return None;
        }
        let (times, lost) = timed(&echoes);
        let (mut p50, mut p95) = (percentile(&times, 50), percentile(&times, 95));
        let mut lost = per_hundred(lost, echoes.len());
        let video_echoes: Vec<Echo> = within(&kept.video, now).collect();
        let answered = video(&video_echoes).filter(|video| video.answered > 0);
        if let Some(video) = answered.filter(|_| weighing_video) {
            p50 = p50.max(video.p50);
            p95 = p95.max(video.p95);
            lost = lost.max(video.held);
        }
        let (sent, retries, failed) = sent_since(&counts);
        Some(Measured {
            echoes: echoes.len(),
            p50,
            p95,
            lost,
            retried: per_hundred(retries, sent),
            failed: per_hundred(failed, sent),
            signal: counts.last().and_then(|counts| counts.signal),
            moving: moving(&kept.counts, now),
            // The most it sent at, since rate control may drop a quiet
            // link's for a moment, which isn't the room it has.
            room: counts
                .iter()
                .filter_map(|counts| counts.link)
                .max()
                .map(|kbps| kbps / 1000),
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
/// learning from them what each carries at each hour. A card is chosen for
/// each minute, and again as soon as a look its chooser asked for ends, so
/// what the look heard is gone to without waiting out the minute.
pub async fn follow(
    state: watch::Receiver<State>,
    calling: watch::Receiver<Calling>,
    asks: mpsc::Sender<Ask>,
    mut learned: Learned,
) {
    let mut echoers: HashMap<String, Echoers> = HashMap::new();
    let mut choosers: HashMap<String, Chooser> = HashMap::new();
    let mut kernel: Option<Nl80211> = None;
    let mut measures = Measures::default();
    let (looked, mut ended) = mpsc::channel(1);
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
                measure(&joined, &mut echoers, &mut kernel, &mut measures).await;
            }
            _ = told.tick() => {
                let (now, hour) = (Instant::now(), learn::hour_here());
                let joined = state.borrow().joined();
                let call = calling.borrow().on(now);
                choosers.retain(|card, _| joined.iter().any(|on| on.card == *card));
                forget_the_forgotten(&mut learned, &state.borrow().saved_names());
                for on in joined {
                    let carried = measures.of(&on.point, now);
                    let best = measures.best_effort_of(&on.point, now);
                    let (Some(carried), Some(best)) = (carried, best) else {
                        continue;
                    };
                    let video = measures.video_of(&on.point, now);
                    println!("<6>{}", said(&on, (&best, &carried), video, call));
                    let around = state.borrow().around(&on.card);
                    learn_from(&mut learned, &on, &carried, &around, hour);
                    let chooser = choosers.entry(on.card.clone()).or_default();
                    moves::choose(
                        chooser,
                        (now, hour),
                        (&on, &carried),
                        &around,
                        (&measures, &learned),
                        (&asks, &state, &looked),
                        call,
                    );
                }
            }
            Some(card) = ended.recv() => {
                let (now, hour) = (Instant::now(), learn::hour_here());
                let call = calling.borrow().on(now);
                let joined = state.borrow().joined();
                let Some(on) = joined.into_iter().find(|on| on.card == card) else {
                    continue;
                };
                let Some(carried) = measures.of(&on.point, now) else {
                    continue;
                };
                let around = state.borrow().around(&on.card);
                let chooser = choosers.entry(on.card.clone()).or_default();
                chooser.looked_over(now);
                moves::choose(
                    chooser,
                    (now, hour),
                    (&on, &carried),
                    &around,
                    (&measures, &learned),
                    (&asks, &state, &looked),
                    call,
                );
            }
        }
    }
}

/// A card's echo sockets, one for each class, each opened again on its
/// own once it fails, so the other carries on meanwhile.
#[derive(Default)]
struct Echoers {
    best: Option<Echoer>,
    video: Option<Echoer>,
}

impl Echoers {
    /// Opens whichever of its sockets isn't open.
    fn open(&mut self, card: &str) {
        let sockets = [
            (Class::BestEffort, &mut self.best),
            (Class::Video, &mut self.video),
        ];
        for (class, socket) in sockets {
            if socket.is_some() {
                continue;
            }
            match Echoer::open(card, class) {
                Ok(opened) => *socket = Some(opened),
                Err(error) => {
                    eprintln!(
                        "<7>can't echo {card}'s gateway in {}: {error}",
                        class.name()
                    );
                }
            }
        }
    }

    /// Echoes the gateway once in each class whose socket is open, both at
    /// once, so a held class can't delay the other's.
    async fn echo(&mut self, gateway: Ipv4Addr) -> Vec<(Class, io::Result<(Instant, Echo)>)> {
        let (best, video) = tokio::join!(
            echo_on(self.best.as_mut(), gateway),
            echo_on(self.video.as_mut(), gateway)
        );
        [(Class::BestEffort, best), (Class::Video, video)]
            .into_iter()
            .filter_map(|(class, echoed)| Some((class, echoed?)))
            .collect()
    }

    /// Lets go of the class's socket, to be opened again next second.
    fn close(&mut self, class: Class) {
        match class {
            Class::BestEffort => self.best = None,
            Class::Video => self.video = None,
        }
    }

    /// The late answers to its video echoes. Best effort's are let go, as
    /// its echoes past [`LOST`] count lost however late they come.
    fn late(&mut self) -> Vec<Late> {
        if let Some(best) = self.best.as_mut() {
            best.late();
        }
        self.video.as_mut().map(Echoer::late).unwrap_or_default()
    }
}

/// Echoes the gateway on the socket, if it's open.
async fn echo_on(
    echoer: Option<&mut Echoer>,
    gateway: Ipv4Addr,
) -> Option<io::Result<(Instant, Echo)>> {
    Some(echoer?.echo(gateway).await)
}

/// Echoes each card's gateway once in each class, every card at once so
/// one whose queue is held can't thin another's echoes, then reads each
/// card's station. A socket that can't be opened, or errs, is passed over
/// until the next second, and opened again then.
async fn measure(
    joined: &[Joined],
    echoers: &mut HashMap<String, Echoers>,
    kernel: &mut Option<Nl80211>,
    measures: &mut Measures,
) {
    let mut echoing = JoinSet::new();
    for on in joined {
        let mut sockets = echoers.remove(&on.card).unwrap_or_default();
        sockets.open(&on.card);
        let on = on.clone();
        echoing.spawn(async move {
            let echoed = sockets.echo(on.gateway).await;
            (on, sockets, echoed)
        });
    }
    while let Some(done) = echoing.join_next().await {
        // A card whose echoing panicked loses its sockets, opened again
        // next second.
        let Ok((on, mut sockets, echoed)) = done else {
            continue;
        };
        for (class, echoed) in echoed {
            match echoed {
                Ok((at, echo)) => measures.echoed_as(class, &on.point, at, echo),
                Err(error) => {
                    eprintln!(
                        "<7>can't echo {}'s gateway in {}: {error}",
                        on.card,
                        class.name()
                    );
                    sockets.close(class);
                }
            }
        }
        for late in sockets.late() {
            measures.answered_late(late);
        }
        echoers.insert(on.card, sockets);
    }
    for on in joined {
        if let Some(counts) = station(kernel, &on.card).await.as_ref().and_then(counted) {
            measures.counted(&on.point, Instant::now(), counts);
        }
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
        moved: station
            .received
            .bytes
            .zip(station.sent.bytes)
            .map(|(received, sent)| received.saturating_add(sent)),
        link: station.send.kbps,
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

/// A span of time in milliseconds, as logged.
pub fn ms(time: Option<Duration>) -> String {
    time.map_or("none".into(), |time| {
        format!("{:.1} ms", time.as_secs_f64() * 1000.0)
    })
}

/// An access point's figures, as logged: its gateway's in best effort,
/// then in video where any video echo was sent, then the rest of what was
/// measured, and whether a call was on.
fn said(
    on: &Joined,
    (best, measured): (&Measured, &Measured),
    video: Option<Video>,
    call: bool,
) -> String {
    let video = video.map_or(String::new(), |video| {
        let late = if video.answered > 0 {
            format!(
                "{:.1}% of {} answers past {} s, ",
                video.held,
                video.answered,
                LOST.as_secs()
            )
        } else {
            String::new()
        };
        format!(
            "; video {} at p95, {late}{} of {} echoes unanswered",
            ms(video.p95),
            video.echoes - video.answered,
            video.echoes
        )
    });
    let signal = measured
        .signal
        .map_or("unread".into(), |signal| format!("{signal} dBm"));
    let moving = match (measured.moving, measured.room) {
        (Some(moving), Some(room)) => format!("; moving {moving} of {room} Mbit/s"),
        (Some(moving), None) => format!("; moving {moving} Mbit/s"),
        (None, _) => String::new(),
    };
    let own = if choose::slowed_by_its_own(measured) {
        ", slowed by what it moves"
    } else {
        ""
    };
    format!(
        "{} on {}: gateway {} at p50, {} at p95, {:.1}% of {} echoes lost{video}; \
         {:.2} in 100 frames sent again, {:.2} lost; signal {signal}{moving}{own}{}",
        on.card,
        on.point,
        ms(best.p50),
        ms(best.p95),
        best.lost,
        best.echoes,
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

/// How far apart two readings may be for what the card moved between them
/// to count: further, and the card may have been on another point between.
const APART: Duration = Duration::from_secs(5);

/// How long the readings that count must cover for what the card moved to
/// say anything, so a few seconds' burst isn't taken for the span's.
const COUNTED: Duration = Duration::from_secs(30);

/// What the card moved both ways together over the span, in Mbit/s: each
/// rise in its byte counts from one reading to the next, over the time
/// between them, where they're no more than `APART`. A reading lower than
/// the one before is a new join, and the second before it counts nothing.
fn moving(kept: &VecDeque<(Instant, Counts)>, now: Instant) -> Option<u32> {
    let said: Vec<(Instant, u64)> = kept
        .iter()
        .filter(|(at, _)| now.duration_since(*at) <= SPAN)
        .filter_map(|(at, counts)| Some((*at, counts.moved?)))
        .collect();
    let (bytes, counted) = said
        .windows(2)
        .filter_map(|pair| {
            let [(was, before), (at, after)] = pair else {
                return None;
            };
            let between = at.duration_since(*was);
            (between <= APART).then_some((after.checked_sub(*before)?, between))
        })
        .fold((0, Duration::ZERO), |(bytes, counted), (more, between)| {
            (u64::saturating_add(bytes, more), counted + between)
        });
    if counted < COUNTED {
        return None;
    }
    // Bits a microsecond are megabits a second, rounded to the nearest.
    let micros = counted.as_micros();
    let megabits = (u128::from(bytes) * 8 + micros / 2) / micros;
    Some(u32::try_from(megabits).unwrap_or(u32::MAX))
}

/// The answered echoes' times, quickest first, and how many were lost.
fn timed(echoes: &[Echo]) -> (Vec<Duration>, usize) {
    let mut times: Vec<Duration> = echoes
        .iter()
        .filter_map(|echo| match echo {
            Echo::Answered(time) => Some(*time),
            Echo::Late(_) | Echo::Lost => None,
        })
        .collect();
    times.sort_unstable();
    let lost = echoes.len() - times.len();
    (times, lost)
}

/// What video echoes carried, if any were sent: their answers' times, a
/// late one's at what it took, and the share that came late. One never
/// answered is left out.
fn video(echoes: &[Echo]) -> Option<Video> {
    if echoes.is_empty() {
        return None;
    }
    let mut times: Vec<Duration> = echoes
        .iter()
        .filter_map(|echo| match echo {
            Echo::Answered(time) | Echo::Late(time) => Some(*time),
            Echo::Lost => None,
        })
        .collect();
    times.sort_unstable();
    let held = echoes
        .iter()
        .filter(|echo| matches!(echo, Echo::Late(_)))
        .count();
    Some(Video {
        echoes: echoes.len(),
        answered: times.len(),
        p50: percentile(&times, 50),
        p95: percentile(&times, 95),
        held: per_hundred(held, times.len()),
    })
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
pub(crate) mod tests {
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
            moved: None,
            link: None,
        }
    }

    /// The station's counts at each second from `start` given, having
    /// moved the bytes beside it by then since it joined.
    fn moved(kept: &mut Measures, point: &str, start: Instant, moved: &[(u64, u64)]) {
        for &(second, moved) in moved {
            let counts = Counts {
                moved: Some(moved),
                ..counts(0, 0, 0)
            };
            kept.counted(point, start + Duration::from_secs(second), counts);
        }
    }

    /// 12.5 MB a second, 100 Mbit/s, over the seconds given, from `bytes`.
    fn steadily(seconds: std::ops::RangeInclusive<u64>, bytes: u64) -> Vec<(u64, u64)> {
        let first = *seconds.start();
        seconds
            .map(|second| (second, bytes + (second - first) * 12_500_000))
            .collect()
    }

    #[test]
    fn what_a_card_moves_is_measured_in_megabits_a_second() {
        let start = Instant::now();
        let mut kept = Measures::default();
        moved(&mut kept, DESK, start, &steadily(0..=30, 0));
        let measured = kept.of(DESK, start + Duration::from_secs(30));
        let measured = measured.expect("a station counted on is measured");
        assert_eq!(measured.moving, Some(100), "375 MB over 30 s");
    }

    #[test]
    fn what_a_card_moved_before_it_joined_again_counts_nothing() {
        let start = Instant::now();
        let mut kept = Measures::default();
        let mut readings = steadily(0..=15, 900_000_000);
        readings.extend(steadily(16..=40, 0));
        moved(&mut kept, DESK, start, &readings);
        let measured = kept.of(DESK, start + Duration::from_secs(40));
        let measured = measured.expect("a station counted on is measured");
        assert_eq!(measured.moving, Some(100), "the join's second says nothing");
    }

    #[test]
    fn what_a_card_moves_counts_only_the_time_it_was_counted() {
        let start = Instant::now();
        let mut kept = Measures::default();
        // Its counts carry on past the gap, so only the gap's length keeps
        // the 1.75 GB said to have moved over it from counting.
        let mut readings = steadily(0..=20, 0);
        readings.extend(steadily(80..=110, 2_000_000_000));
        moved(&mut kept, DESK, start, &readings);
        let measured = kept.of(DESK, start + Duration::from_secs(110));
        let measured = measured.expect("a station counted on is measured");
        assert_eq!(measured.moving, Some(100), "a minute away says nothing");
    }

    #[test]
    fn a_few_seconds_counted_say_nothing_of_what_a_card_moves() {
        let start = Instant::now();
        let mut kept = Measures::default();
        moved(&mut kept, DESK, start, &steadily(0..=29, 0));
        let measured = kept.of(DESK, start + Duration::from_secs(29));
        let measured = measured.expect("a station counted on is measured");
        assert_eq!(measured.moving, None, "29 s is short of 30");
    }

    #[test]
    fn readings_saying_no_bytes_leave_what_a_card_moves_to_the_rest() {
        let start = Instant::now();
        let mut kept = Measures::default();
        for (second, bytes) in steadily(0..=40, 0) {
            let moved = (second % 2 == 0).then_some(bytes);
            let counts = Counts {
                moved,
                ..counts(0, 0, 0)
            };
            kept.counted(DESK, start + Duration::from_secs(second), counts);
        }
        let measured = kept.of(DESK, start + Duration::from_secs(40));
        let measured = measured.expect("a station counted on is measured");
        assert_eq!(measured.moving, Some(100), "every other one says it");
    }

    #[test]
    fn what_a_link_has_room_for_is_the_most_it_sent_at_in_the_span() {
        let start = Instant::now();
        let mut kept = Measures::default();
        for (second, link) in [(0, 6_500), (1, 144_400), (2, 6_500)] {
            let counts = Counts {
                link: Some(link),
                ..counts(0, 0, 0)
            };
            kept.counted(DESK, start + Duration::from_secs(second), counts);
        }
        let measured = kept.of(DESK, start + Duration::from_secs(2));
        let measured = measured.expect("a station counted on is measured");
        assert_eq!(measured.room, Some(144), "144.4 Mbit/s, the most");
    }

    #[test]
    fn a_station_s_bytes_are_counted_both_ways_or_not_at_all() {
        use crate::nl80211::{Rate, Traffic};
        let traffic = |bytes| Traffic {
            bytes,
            packets: Some(1),
        };
        let rate = |kbps| Rate {
            kbps: Some(kbps),
            ..Rate::default()
        };
        let station = |sent| Station {
            received: traffic(Some(5)),
            sent: traffic(sent),
            receive: rate(6_500),
            send: rate(286_700),
            retries: Some(0),
            failed: Some(0),
            ..Station::default()
        };
        let both = counted(&station(Some(7))).expect("all its counts are said");
        let one = counted(&station(None)).expect("all its counts are said");
        assert_eq!(both.moved, Some(12), "5 in and 7 out");
        assert_eq!(one.moved, None, "one way unsaid");
        assert_eq!(both.link, Some(286_700), "the link is what it sends at");
    }

    #[test]
    fn what_a_card_moved_is_said_after_its_signal() {
        let measured = Measured {
            echoes: 0,
            p50: None,
            p95: None,
            lost: 0.0,
            retried: 0.0,
            failed: 0.0,
            signal: None,
            moving: Some(12),
            room: Some(286),
        };
        let line = said(&joined(DESK), (&measured, &measured), None, true);
        assert!(
            line.ends_with("unread; moving 12 of 286 Mbit/s, during a call"),
            "what it moved, and of what, is why it keeps to a band: {line}"
        );
        let unsaid = Measured {
            room: None,
            ..measured
        };
        let line = said(&joined(DESK), (&unsaid, &unsaid), None, false);
        let alone = "unread; moving 12 Mbit/s";
        assert!(line.ends_with(alone), "no room: {line}");
    }

    #[test]
    fn a_slow_end_from_what_a_card_moves_is_said_to_be_its_own() {
        let downloading = Measured {
            echoes: 121,
            p50: Some(ms(6)),
            p95: Some(ms(143)),
            lost: 0.0,
            retried: 0.0,
            failed: 0.0,
            signal: Some(-36),
            moving: Some(52),
            room: Some(1200),
        };
        let line = said(&joined(DESK), (&downloading, &downloading), None, false);
        assert!(
            line.ends_with("moving 52 of 1200 Mbit/s, slowed by what it moves"),
            "why it stays is said: {line}"
        );
        let idle = Measured {
            moving: Some(9),
            ..downloading
        };
        let line = said(&joined(DESK), (&idle, &idle), None, false);
        assert!(line.ends_with("moving 9 of 1200 Mbit/s"), "idle: {line}");
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
            moving: None,
            room: None,
        };

        let both = (&measured, &measured);
        assert_eq!(
            said(&on, both, None, false),
            "wlan2 on 6c:ae:f6:c1:84:26: gateway 5.1 ms at p50, 15.0 ms at p95, 0.0% of 120 \
             echoes lost; 1.25 in 100 frames sent again, 0.00 lost; signal -47 dBm",
            "one line says all of it"
        );
        assert!(
            said(&on, both, None, true).ends_with("signal -47 dBm, during a call"),
            "a call on is said, since the card stays put for it"
        );
        let video = Video {
            echoes: 120,
            answered: 118,
            p50: Some(ms(640)),
            p95: Some(ms(2400)),
            held: 38.3,
        };
        let folded = Measured {
            p50: Some(ms(640)),
            p95: Some(ms(2400)),
            lost: 38.3,
            ..measured.clone()
        };
        let line = said(&on, (&measured, &folded), Some(video), false);
        assert!(
            line.contains(
                "gateway 5.1 ms at p50, 15.0 ms at p95, 0.0% of 120 echoes lost; video \
                 2400.0 ms at p95, 38.3% of 118 answers past 1 s, 2 of 120 echoes unanswered; \
                 1.25 in 100"
            ),
            "best effort's own figures, then the video class's, for a call's path: {line}"
        );
        let unanswered = Video {
            answered: 0,
            p50: None,
            p95: None,
            held: 0.0,
            ..video
        };
        let line = said(&on, both, Some(unanswered), false);
        assert!(
            line.contains("; video none at p95, 120 of 120 echoes unanswered; 1.25 in 100"),
            "a video class never answered: {line}"
        );
    }

    /// One video echo a second for the times given, in milliseconds,
    /// starting at `start`: answered in time under a second, late past
    /// it, never with none.
    fn echoed_in_video(kept: &mut Measures, point: &str, start: Instant, times: &[Option<u64>]) {
        for (second, time) in (0..).zip(times) {
            let sent = start + Duration::from_secs(second);
            let echo = match time.map(ms) {
                Some(time) if time <= LOST => Echo::Answered(time),
                _ => Echo::Lost,
            };
            kept.echoed_as(Class::Video, point, sent, echo);
            if let Some(took) = time.map(ms).filter(|&time| time > LOST) {
                kept.answered_late(Late { sent, took });
            }
        }
    }

    #[test]
    fn an_access_point_holding_video_carries_as_its_video_does() {
        // As a mesh node's 5 GHz point carried a card on 23 Nov 2026: best
        // effort in 6 ms, video held near a second and past it, in order.
        let start = Instant::now();
        let mut kept = Measures::default();
        echoed(&mut kept, DESK, start, &[Some(6); 4]);
        echoed_in_video(
            &mut kept,
            DESK,
            start,
            &[Some(900), Some(1900), Some(800), Some(2400)],
        );
        let now = start + Duration::from_secs(4);
        let measured = kept.of(DESK, now);
        let measured = measured.expect("an access point echoed through is measured");
        assert_eq!(
            measured.p95,
            Some(ms(2400)),
            "the held class's slow end, late"
        );
        assert_eq!(measured.p50, Some(ms(900)), "and its middle");
        assert!(
            (measured.lost - 50.0).abs() < 1e-9,
            "the answers past a second count lost: {}",
            measured.lost
        );
        assert_eq!(measured.echoes, 4, "echoes count each second once");
        let best = kept
            .best_effort_of(DESK, now)
            .expect("best effort was echoed");
        assert_eq!(best.p95, Some(ms(6)), "best effort's own, as logged");
    }

    #[test]
    fn a_queue_held_past_every_echo_s_wait_is_weighed_as_held() {
        // Held longer than its second each time, every video echo first
        // counts lost, then answers late, as the held frames are let go.
        let start = Instant::now();
        let mut kept = Measures::default();
        echoed(&mut kept, DESK, start, &[Some(6); 3]);
        echoed_in_video(
            &mut kept,
            DESK,
            start,
            &[Some(4900), Some(3900), Some(2900)],
        );
        let measured = kept.of(DESK, start + Duration::from_secs(3));
        let measured = measured.expect("an access point echoed through is measured");
        assert_eq!(measured.p95, Some(ms(4900)), "as slow as it was held");
        assert!(
            (measured.lost - 100.0).abs() < 1e-9,
            "every answer late: {}",
            measured.lost
        );
    }

    #[test]
    fn a_gateway_dropping_most_marked_echoes_isn_t_weighed_by_them() {
        // One that polices marked echoes, at every access point alike,
        // answers a few in time and the rest never.
        let start = Instant::now();
        let mut kept = Measures::default();
        echoed(&mut kept, DESK, start, &[Some(5); 6]);
        echoed_in_video(
            &mut kept,
            DESK,
            start,
            &[Some(7), None, None, None, None, None],
        );
        let now = start + Duration::from_secs(6);
        let measured = kept.of(DESK, now);
        let measured = measured.expect("an access point echoed through is measured");
        assert!(
            measured.lost.abs() < f64::EPSILON,
            "loses nothing in the air"
        );
        assert_eq!(
            measured.p95,
            Some(ms(7)),
            "its answer is as quick as it was"
        );
        let video = kept.video_of(DESK, now).expect("video echoes were sent");
        assert_eq!(
            (video.echoes, video.answered),
            (6, 1),
            "one of six answered"
        );
    }

    #[test]
    fn a_late_answer_counts_on_the_access_point_its_echo_went_through() {
        let start = Instant::now();
        let mut kept = Measures::default();
        echoed_in_video(&mut kept, DESK, start, &[None]);
        echoed_in_video(&mut kept, HALL, start + Duration::from_secs(1), &[Some(5)]);
        kept.answered_late(Late {
            sent: start,
            took: ms(3100),
        });
        let now = start + Duration::from_secs(2);
        let desk = kept.video_of(DESK, now).expect("the desk's was echoed");
        let hall = kept.video_of(HALL, now).expect("the hall's was echoed");
        assert_eq!(desk.p95, Some(ms(3100)), "the desk held it");
        assert!((desk.held - 100.0).abs() < 1e-9, "late: {}", desk.held);
        assert_eq!(hall.p95, Some(ms(5)), "the hall's is its own");
    }

    #[test]
    fn an_access_point_slower_in_best_effort_carries_as_its_best_effort_does() {
        let start = Instant::now();
        let mut kept = Measures::default();
        echoed(&mut kept, DESK, start, &[Some(120), None]);
        echoed_in_video(&mut kept, DESK, start, &[Some(7), Some(7)]);
        let measured = kept.of(DESK, start + Duration::from_secs(2));
        let measured = measured.expect("an access point echoed through is measured");
        assert_eq!(
            measured.p95,
            Some(ms(120)),
            "the worse class is best effort"
        );
        assert!(
            (measured.lost - 50.0).abs() < 1e-9,
            "its losses: {}",
            measured.lost
        );
    }

    #[test]
    fn a_video_class_never_answered_isn_t_weighed_but_is_said() {
        let start = Instant::now();
        let mut kept = Measures::default();
        echoed(&mut kept, DESK, start, &[Some(5), Some(5)]);
        echoed_in_video(&mut kept, DESK, start, &[None, None]);
        let now = start + Duration::from_secs(2);
        let measured = kept
            .of(DESK, now)
            .expect("an access point echoed through is measured");
        assert_eq!(
            measured.p95,
            Some(ms(5)),
            "a gateway dropping marked echoes"
        );
        assert!(
            measured.lost.abs() < f64::EPSILON,
            "loses nothing in the air"
        );
        let video = kept.video_of(DESK, now).expect("video echoes were sent");
        assert_eq!(video.p95, None, "none answered");
        assert_eq!((video.echoes, video.answered), (2, 0), "none of two");
    }

    #[test]
    fn no_video_echo_sent_says_no_video_figures() {
        let start = Instant::now();
        let mut kept = Measures::default();
        echoed(&mut kept, DESK, start, &[Some(5)]);
        assert_eq!(kept.video_of(DESK, start + Duration::from_secs(1)), None);
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

    /// A card joined to Lantern through the access point given, which
    /// moves.rs's tests join through too.
    pub(crate) fn joined(point: &str) -> Joined {
        Joined {
            card: "wlan2".into(),
            point: point.into(),
            gateway: std::net::Ipv4Addr::new(10, 43, 0, 1),
            network: "Lantern".into(),
            trusted: true,
        }
    }
}
