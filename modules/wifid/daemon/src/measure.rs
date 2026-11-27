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
//! in the span. During a call it's asked each second too whether the
//! card's link is fading, and moves.rs has it give way at once.
//!
//! Each second's echo goes twice, once unmarked and once in Wi-Fi's video
//! queue (echo.rs), and the chooser and the hours are handed the worse of
//! the two classes' p50, of their p95s and of their losses, each taken
//! apart, so a point holding a call's queue is passed over however fast
//! its best effort. A video echo answered past its second counts as slow
//! as it took and as lost, as a queue held for seconds answers late, in
//! order, and only a held queue answers late. One never answered isn't
//! weighed, and video's times aren't unless half its echoes were
//! answered: a gateway that polices marked echoes does so through every
//! access point alike, says nothing of the air, and leaves few answers to
//! time. Which leaves a point dropping a call's queue in the air, rather
//! than holding it, unseen by video, as by best effort wherever its own
//! queue is spared; and a hold still on unseen until its answers come.
//! The minute's line says best effort's own figures, and the video
//! class's beside them, or that video went unmeasured.
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
use crate::fading;
use crate::learn::{self, Here, Learned};
use crate::moves;
use crate::nl80211::{Nl80211, Station};
use crate::service::Ask;
use crate::state::{Joined, State};

/// How far back an access point's figures reach: long enough for a p95 of
/// one echo a second to mean something, short enough to follow a move to
/// another room.
pub const SPAN: Duration = Duration::from_secs(120);

/// How far back the minute a trial is judged by reaches: a minute, less
/// half a second, so a minute read on time shares no echo with the one
/// before. A minute's tick waits behind a measure, up to about 2 s where
/// echoes go unanswered, so one echo of a minute's sixty may be read in
/// both, or in neither, which can tip only a minute already at the bar.
const MINUTE: Duration = Duration::from_millis(59_500);

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
    /// The kernel's average of the signal, steadier than one frame's.
    pub average: Option<i32>,
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
    /// The share of echoes sent that were answered past [`LOST`], in
    /// percent, as best effort's loss is a share of its echoes sent.
    pub held: f64,
}

impl Video {
    /// Whether its times say anything of the air: at least half its echoes
    /// were answered, in time or late. A queue held for seconds answers
    /// them all, late; a gateway that polices marked echoes, at every
    /// access point alike, answers few, and the few it answers say nothing
    /// of the rest.
    pub fn timed(&self) -> bool {
        self.answered > 0 && self.answered * 2 >= self.echoes
    }
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
    /// When each echo taken within `fading::LOOK` of its card beginning a
    /// look for networks ended, so a call's fading is judged without them.
    looking: VecDeque<(Instant, ())>,
    /// The look the card was last seen in, by how many it had begun, and
    /// when it was first seen in it.
    look: Option<(u32, Instant)>,
    /// When each echo couldn't be sent as the card's link was down, kept
    /// apart from the echoes: lost to a call's link, they say nothing of
    /// how the point carries, and right after a resume, a join or a roam
    /// they'd read as a failing minute, learned for its hour.
    unsent: VecDeque<(Instant, ())>,
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

    /// Takes in what a card heard in a second on the access point it's on:
    /// the late answers first, as each is to an echo of an earlier second,
    /// already kept, then this second's echoes, though only if best
    /// effort's went, so video's share is of the seconds best effort's is.
    /// Says whether it went, as only then is the card's station read.
    pub fn took_in(
        &mut self,
        point: &str,
        late: Vec<Late>,
        echoed: &[(Class, io::Result<(Instant, Echo)>)],
    ) -> bool {
        for late in late {
            self.answered_late(late);
        }
        let went = best_effort_went(echoed);
        if went {
            for (class, echoed) in echoed {
                if let Ok((at, echo)) = echoed {
                    self.echoed_as(*class, point, *at, *echo);
                }
            }
        }
        went
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
        let echoes: Vec<Echo> = within(&self.points.get(point)?.video, now, SPAN).collect();
        video(&echoes)
    }

    /// Takes in that a best-effort echo through the access point couldn't
    /// be sent then, as the card's link was down.
    pub fn unsent(&mut self, point: &str, at: Instant) {
        let kept = self.points.entry(point.to_owned()).or_default();
        kept.unsent.push_back((at, ()));
        forget_before(&mut kept.unsent, at);
    }

    /// Takes in that the echo sent then was taken while its card
    /// was in this look for networks, by how many it had begun. It's left
    /// out of a call's fading only within `fading::LOOK` of the look.
    pub fn looked(&mut self, point: &str, at: Instant, look: u32) {
        let kept = self.points.entry(point.to_owned()).or_default();
        let began = match kept.look {
            Some((seen, began)) if seen == look => began,
            _ => at,
        };
        kept.look = Some((look, began));
        if at.duration_since(began) <= fading::LOOK {
            kept.looking.push_back((at, ()));
            forget_before(&mut kept.looking, at);
        }
    }

    /// Takes in the station's counts, read on the access point.
    pub fn counted(&mut self, point: &str, at: Instant, counts: Counts) {
        let kept = self.points.entry(point.to_owned()).or_default();
        kept.counts.push_back((at, counts));
        forget_before(&mut kept.counts, at);
    }

    /// How many best-effort echoes through the access point were lost
    /// within `within` of now, those taken while its card looked for
    /// networks aside and those that couldn't be sent as its link was down
    /// among them, and its signal as the kernel last said it within then,
    /// its average where it says one, for whether a call's link is fading
    /// (fading.rs). A reading older than that says nothing of the link
    /// now.
    pub fn lately(&self, point: &str, now: Instant, within: Duration) -> (usize, Option<i32>) {
        let Some(kept) = self.points.get(point) else {
            return (0, None);
        };
        let looking = |at: &Instant| kept.looking.iter().any(|(looked, ())| looked == at);
        let lost = kept
            .echoes
            .iter()
            .filter(|(at, echo)| now.duration_since(*at) <= within && *echo == Echo::Lost)
            .filter(|(at, _)| !looking(at))
            .count();
        let unsent = kept
            .unsent
            .iter()
            .filter(|(at, ())| now.duration_since(*at) <= within)
            .count();
        let lost = lost + unsent;
        let signal = kept
            .counts
            .back()
            .filter(|(at, _)| now.duration_since(*at) <= within)
            .and_then(|(_, counts)| counts.average.or(counts.signal));
        (lost, signal)
    }

    /// The access point's figures over the span up to now, each the worse
    /// class's, if anything was measured of it in it.
    pub fn of(&self, point: &str, now: Instant) -> Option<Measured> {
        self.figures(point, now, true, SPAN)
    }

    /// The access point's figures over the minute up to now alone, each
    /// the worse class's, as a trial is judged early by single minutes
    /// rather than spans of two.
    pub fn minute_of(&self, point: &str, now: Instant) -> Option<Measured> {
        self.figures(point, now, true, MINUTE)
    }

    /// The access point's figures over the span up to now in best effort
    /// alone, as logged, if anything was measured of it in it.
    pub fn best_effort_of(&self, point: &str, now: Instant) -> Option<Measured> {
        self.figures(point, now, false, SPAN)
    }

    fn figures(
        &self,
        point: &str,
        now: Instant,
        weighing_video: bool,
        over: Duration,
    ) -> Option<Measured> {
        let kept = self.points.get(point)?;
        let echoes: Vec<Echo> = within(&kept.echoes, now, over).collect();
        let counts: Vec<Counts> = within(&kept.counts, now, over).collect();
        if echoes.is_empty() && counts.is_empty() {
            return None;
        }
        let (times, lost) = timed(&echoes);
        let (mut p50, mut p95) = (percentile(&times, 50), percentile(&times, 95));
        let mut lost = per_hundred(lost, echoes.len());
        let video_echoes: Vec<Echo> = within(&kept.video, now, over).collect();
        if let Some(video) = video(&video_echoes).filter(|_| weighing_video) {
            lost = lost.max(video.held);
            if video.timed() {
                p50 = worse(p50, video.p50);
                p95 = worse(p95, video.p95);
            }
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
                measure(&joined, &mut echoers, &mut kernel, &mut measures, &state).await;
                let now = Instant::now();
                if calling.borrow().on(now) {
                    for on in &joined {
                        let chooser = choosers.entry(on.card.clone()).or_default();
                        let with = (&asks, &state, &looked);
                        moves::give_way(chooser, now, on, &measures, with);
                    }
                }
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
                    if let Some(minute) = measures.minute_of(&on.point, now) {
                        chooser.took_minute(now, &on.point, &minute);
                    }
                    moves::choose(
                        chooser,
                        (now, hour),
                        (&on, &carried),
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
                let chooser = choosers.entry(on.card.clone()).or_default();
                chooser.looked_over(now);
                moves::choose(
                    chooser,
                    (now, hour),
                    (&on, &carried),
                    (&measures, &learned),
                    (&asks, &state, &looked),
                    call,
                );
            }
        }
    }
}

/// A card's echo sockets, one for each class, each opened again on its
/// own once it fails. Video's carries on while best effort's is down, its
/// late answers still counted, though its echoes then count for nothing.
#[derive(Default)]
struct Echoers {
    best: Option<Echoer>,
    video: Option<Echoer>,
    failing: Failing,
}

/// The classes whose echoes have failed, to open or to go, since each last
/// went: the first failure is said as a warning, as such a class goes
/// unmeasured, and the rest quietly.
#[derive(Default)]
struct Failing(Vec<Class>);

impl Failing {
    /// The journal level to say the class's failure at.
    fn failed(&mut self, class: Class) -> u8 {
        if self.0.contains(&class) {
            7
        } else {
            self.0.push(class);
            4
        }
    }

    /// Takes note that the class's echo went.
    fn went(&mut self, class: Class) {
        self.0.retain(|failing| *failing != class);
    }
}

/// Whether best effort's echo went this second, as only then is the card's
/// station read: its counts alone would read as never answered, and send
/// it from one good access point to the next.
fn best_effort_went(echoed: &[(Class, io::Result<(Instant, Echo)>)]) -> bool {
    echoed
        .iter()
        .any(|(class, echoed)| *class == Class::BestEffort && echoed.is_ok())
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
                    let level = self.failing.failed(class);
                    eprintln!(
                        "<{level}>can't echo {card}'s gateway in {}: {error}",
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
/// until the next second, and opened again then; a card best effort's echo
/// couldn't be sent on goes unmeasured that second, its video echo and its
/// station too, though late answers to earlier echoes still count. A
/// best-effort echo sent while its card looked for networks, or across a
/// look begun and ended between the reads either side of it, is marked as
/// taken in that look, for a call's fading.
async fn measure(
    joined: &[Joined],
    echoers: &mut HashMap<String, Echoers>,
    kernel: &mut Option<Nl80211>,
    measures: &mut Measures,
    state: &watch::Receiver<State>,
) {
    let mut echoing = JoinSet::new();
    let mut cards = HashMap::new();
    let mut before = HashMap::new();
    let mut echoed_on = Vec::new();
    for on in joined {
        let mut sockets = echoers.remove(&on.card).unwrap_or_default();
        sockets.open(&on.card);
        before.insert(on.card.clone(), state.borrow().looking(&on.card));
        let on = on.clone();
        let card = on.card.clone();
        let task = echoing.spawn(async move {
            let echoed = sockets.echo(on.gateway).await;
            (on, sockets, echoed)
        });
        cards.insert(task.id(), card);
    }
    while let Some(done) = echoing.join_next().await {
        // A card whose echoing panicked loses its sockets, opened again
        // next second, and its late answers on the way with them.
        let (on, mut sockets, echoed) = match done {
            Ok(done) => done,
            Err(error) => {
                let card = cards.get(&error.id()).map_or("a card", String::as_str);
                eprintln!("<4>can't echo {card}'s gateway this second: {error}");
                continue;
            }
        };
        // The late answers are taken before a failing socket is let go, as
        // what it heard late before failing is heard all the same.
        if measures.took_in(&on.point, sockets.late(), &echoed) {
            let after = state.borrow().looking(&on.card);
            let looked = before
                .get(&on.card)
                .is_some_and(|before| before.looked_until(after));
            let sent = echoed.iter().find_map(|(class, echoed)| match echoed {
                Ok((at, _)) if *class == Class::BestEffort => Some(*at),
                _ => None,
            });
            if let (true, Some(at)) = (looked, sent) {
                measures.looked(&on.point, at, after.begun);
            }
            echoed_on.push(on.clone());
        }
        for (class, echoed) in echoed {
            match echoed {
                Ok(_) => sockets.failing.went(class),
                Err(error) => {
                    let level = sockets.failing.failed(class);
                    eprintln!(
                        "<{level}>can't echo {}'s gateway in {}: {error}",
                        on.card,
                        class.name()
                    );
                    // A best-effort echo that couldn't be sent as the link
                    // is down is lost to a call as surely as one
                    // unanswered, so a call's fading counts it, and nothing
                    // else does. Refused for another reason, as by a
                    // firewall, it says nothing of the link.
                    if class == Class::BestEffort && link_down(&error) {
                        measures.unsent(&on.point, Instant::now());
                    }
                    sockets.close(class);
                }
            }
        }
        echoers.insert(on.card, sockets);
    }
    for on in echoed_on {
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
        average: station.average,
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
/// then in video, or that video went unmeasured where no video echo went,
/// then the rest of what was measured, and whether a call was on.
fn said(
    on: &Joined,
    (best, measured): (&Measured, &Measured),
    video: Option<Video>,
    call: bool,
) -> String {
    // A card whose video echoes don't go is judged by best effort alone,
    // which is said each minute rather than once, hours back.
    let video = video.map_or("; video unmeasured".into(), |video| {
        format!(
            "; video {} at p95, {:.1}% of {} echoes answered past {} s, {} unanswered{}",
            ms(video.p95),
            video.held,
            video.echoes,
            LOST.as_secs(),
            video.echoes - video.answered,
            if video.timed() {
                ""
            } else {
                ", too few to time"
            },
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

/// Whether an echo couldn't be sent as the card's link was down, rather
/// than refused for another reason.
fn link_down(error: &std::io::Error) -> bool {
    matches!(
        error.raw_os_error(),
        Some(libc::ENETDOWN | libc::ENETUNREACH | libc::EHOSTUNREACH | libc::ENODEV)
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

/// What was measured in the stretch this long ending at `now`.
fn within<T: Copy>(
    kept: &VecDeque<(Instant, T)>,
    now: Instant,
    over: Duration,
) -> impl Iterator<Item = T> {
    kept.iter()
        .filter(move |(at, _)| now.duration_since(*at) <= over)
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

/// The slower of best effort's figure and video's. Best effort never
/// answered stays unanswered, as slow as can be, however quick video.
fn worse(best: Option<Duration>, video: Option<Duration>) -> Option<Duration> {
    match (best, video) {
        (Some(best), Some(video)) => Some(best.max(video)),
        (best, _) => best,
    }
}

/// What video echoes carried, if any were sent: their answers' times, a
/// late one's at what it took, and the share of those sent that came
/// late. One never answered is in no figure but the count of answers.
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
        held: per_hundred(held, echoes.len()),
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

    const DESK: &str = "02:00:00:00:a4:00";
    const HALL: &str = "02:00:00:00:a6:00";

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    fn counts(sent: u32, retries: u32, failed: u32) -> Counts {
        Counts {
            sent,
            retries,
            failed,
            signal: Some(-47),
            average: None,
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
    fn lately_counts_the_echoes_lost_within_its_window_and_the_last_signal() {
        let start = Instant::now();
        let mut kept = Measures::default();
        let lost_at = [0, 5, 12, 14];
        for second in 0..=15 {
            let echo = if lost_at.contains(&second) {
                Echo::Lost
            } else {
                Echo::Answered(Duration::from_millis(5))
            };
            kept.echoed(DESK, start + Duration::from_secs(second), echo);
        }
        kept.counted(DESK, start + Duration::from_secs(15), counts(10, 0, 0));
        let now = start + Duration::from_secs(15);

        let lately = kept.lately(DESK, now, Duration::from_secs(10));
        let unmeasured = kept.lately("02:00:00:00:a1:00", now, Duration::from_secs(10));

        assert_eq!(
            lately,
            (3, Some(-47)),
            "those from 5 s on, and the kernel's signal"
        );
        assert_eq!(unmeasured, (0, None), "nothing of a point not measured");
    }

    #[test]
    fn lately_says_the_kernel_s_average_signal_and_none_from_an_old_reading() {
        let start = Instant::now();
        let mut kept = Measures::default();
        let averaged = Counts {
            average: Some(-70),
            ..counts(10, 0, 0)
        };
        kept.counted(DESK, start, averaged);
        let within = Duration::from_secs(10);

        let fresh = kept.lately(DESK, start + Duration::from_secs(10), within);
        let old = kept.lately(DESK, start + Duration::from_secs(11), within);

        assert_eq!(fresh, (0, Some(-70)), "the average, not one frame's -47");
        assert_eq!(old, (0, None), "a reading 11 s old says nothing of now");
    }

    #[test]
    fn lately_leaves_out_the_echoes_lost_while_the_card_looked() {
        let start = Instant::now();
        let mut kept = Measures::default();
        for second in 0..10 {
            let at = start + Duration::from_secs(second);
            kept.echoed(DESK, at, Echo::Lost);
            if second != 4 {
                kept.looked(DESK, at, 1);
            }
        }
        let now = start + Duration::from_secs(9);

        let (lost, _) = kept.lately(DESK, now, Duration::from_secs(10));

        // Seconds 0 to 3 and 5 and 6 are the look's first 6 s; 7 to 9 run
        // past them, as a fading link's look does.
        assert_eq!(
            lost, 4,
            "the one lost between and those past the look's first 6 s"
        );
    }

    #[test]
    fn each_look_has_its_own_first_seconds_left_out() {
        let start = Instant::now();
        let mut kept = Measures::default();
        for second in 0..10 {
            let at = start + Duration::from_secs(second);
            kept.echoed(DESK, at, Echo::Lost);
            kept.looked(DESK, at, if second < 5 { 1 } else { 2 });
        }
        let now = start + Duration::from_secs(9);

        let (lost, _) = kept.lately(DESK, now, Duration::from_secs(10));

        assert_eq!(lost, 0, "two looks of 5 s each are both left out whole");
    }

    #[test]
    fn an_echo_unsent_as_the_link_was_down_is_lost_to_a_call_alone() {
        let start = Instant::now();
        let mut kept = Measures::default();
        for second in 0..10 {
            let at = start + Duration::from_secs(second);
            if second < 3 {
                kept.unsent(DESK, at);
            } else {
                kept.echoed(DESK, at, Echo::Answered(Duration::from_millis(5)));
            }
        }
        let now = start + Duration::from_secs(9);

        let (lost, _) = kept.lately(DESK, now, Duration::from_secs(10));
        let minute = kept.of(DESK, now).expect("answered echoes were measured");

        assert_eq!(lost, 3, "a call's link lost all three");
        assert_eq!(minute.echoes, 7, "the minute holds only the echoes sent");
        assert!(minute.lost < f64::EPSILON, "and lost none of them");
    }

    #[test]
    fn only_an_echo_refused_for_a_link_down_counts_as_unsent() {
        let down = std::io::Error::from_raw_os_error(libc::ENETUNREACH);
        let refused = std::io::Error::from_raw_os_error(libc::EPERM);

        assert!(link_down(&down), "no route is the link down");
        assert!(!link_down(&refused), "a firewall's refusal isn't");
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
    fn a_minute_alone_holds_none_of_the_minute_before() {
        let start = Instant::now();
        let mut kept = Measures::default();
        echoed(&mut kept, DESK, start, &[Some(5)]);
        echoed_in_video(&mut kept, DESK, start, &[Some(500)]);
        let minute = start + Duration::from_secs(60);
        echoed(&mut kept, DESK, minute, &[Some(5)]);
        echoed_in_video(&mut kept, DESK, minute, &[Some(5)]);
        let alone = kept
            .minute_of(DESK, minute)
            .expect("the minute was measured");
        let span = kept.of(DESK, minute).expect("the span was measured");
        assert_eq!(
            alone.echoes, 1,
            "the echo a minute back is the minute before's"
        );
        assert_eq!(
            alone.p95,
            Some(ms(5)),
            "and its held video says nothing of this one"
        );
        assert_eq!(
            (span.echoes, span.p95),
            (2, Some(ms(500))),
            "the span holds both, video's too"
        );
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
            "wlan2 on 02:00:00:00:a4:00: gateway 5.1 ms at p50, 15.0 ms at p95, 0.0% of 120 \
             echoes lost; video unmeasured; 1.25 in 100 frames sent again, 0.00 lost; \
             signal -47 dBm",
            "one line says all of it, and that video went unmeasured"
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
            held: 37.5,
        };
        let folded = Measured {
            p50: Some(ms(640)),
            p95: Some(ms(2400)),
            lost: 37.5,
            ..measured.clone()
        };
        let line = said(&on, (&measured, &folded), Some(video), false);
        assert!(
            line.contains(
                "gateway 5.1 ms at p50, 15.0 ms at p95, 0.0% of 120 echoes lost; video \
                 2400.0 ms at p95, 37.5% of 120 echoes answered past 1 s, 2 unanswered; \
                 1.25 in 100"
            ),
            "best effort's own figures, then the video class's, for a call's path: {line}"
        );
        let policed = Video {
            answered: 10,
            p95: Some(ms(1500)),
            held: 1.7,
            ..video
        };
        let line = said(&on, both, Some(policed), false);
        assert!(
            line.contains(
                "; video 1500.0 ms at p95, 1.7% of 120 echoes answered past 1 s, \
                 110 unanswered, too few to time; 1.25 in 100"
            ),
            "a video class answered too seldom to say anything: {line}"
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
            line.contains(
                "; video none at p95, 0.0% of 120 echoes answered past 1 s, \
                 120 unanswered, too few to time; 1.25 in 100"
            ),
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
        assert!(best.lost.abs() < 1e-9, "its loss its own: {}", best.lost);
    }

    #[test]
    fn a_class_failing_is_warned_of_once_until_its_echo_goes_again() {
        let mut failing = Failing::default();
        let levels = [
            failing.failed(Class::Video),
            failing.failed(Class::Video),
            failing.failed(Class::BestEffort),
        ];
        assert_eq!(levels, [4, 7, 4], "each class warns once on its own");
        failing.went(Class::Video);
        assert_eq!(failing.failed(Class::Video), 4, "and again once it went");
        assert_eq!(
            failing.failed(Class::BestEffort),
            7,
            "the other still quiet"
        );
    }

    #[test]
    fn a_second_best_effort_s_echo_didn_t_go_in_keeps_only_late_answers() {
        let start = Instant::now();
        let then = start + Duration::from_secs(1);
        let failed = || Err(io::Error::other("unreachable"));
        let mut kept = Measures::default();
        let went = kept.took_in(
            DESK,
            Vec::new(),
            &[
                (Class::BestEffort, Ok((start, Echo::Lost))),
                (Class::Video, Ok((start, Echo::Lost))),
            ],
        );
        assert!(went, "best effort went, lost or not");

        let went = kept.took_in(
            DESK,
            vec![Late {
                sent: start,
                took: ms(1500),
            }],
            &[
                (Class::BestEffort, failed()),
                (Class::Video, Ok((then, Echo::Lost))),
            ],
        );
        assert!(!went, "only video went");
        assert!(!kept.took_in(DESK, Vec::new(), &[]), "neither socket open");

        let video = kept.video_of(DESK, then).expect("video was echoed");
        assert_eq!(video.echoes, 1, "only the second best effort went in");
        assert_eq!(video.answered, 1, "its late answer kept");
        let measured = kept.of(DESK, then).expect("best effort was echoed");
        assert_eq!(measured.echoes, 1, "best effort alike");
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
        // answers a few and the rest never, its few slower than best
        // effort's.
        let start = Instant::now();
        let mut kept = Measures::default();
        echoed(&mut kept, DESK, start, &[Some(5); 6]);
        echoed_in_video(
            &mut kept,
            DESK,
            start,
            &[Some(70), None, None, None, None, None],
        );
        let now = start + Duration::from_secs(6);
        let measured = kept.of(DESK, now);
        let measured = measured.expect("an access point echoed through is measured");
        assert!(
            measured.lost.abs() < f64::EPSILON,
            "loses nothing in the air"
        );
        assert_eq!(measured.p95, Some(ms(5)), "best effort's own slow end");
        let video = kept.video_of(DESK, now).expect("video echoes were sent");
        assert_eq!(
            (video.echoes, video.answered, video.timed()),
            (6, 1, false),
            "one of six answered, too few to time"
        );
    }

    #[test]
    fn a_point_holding_video_then_dropping_it_loses_what_it_held() {
        // Held past a second and then dropped, most of it: too few
        // answered to time, but each late answer is a held one.
        let start = Instant::now();
        let mut kept = Measures::default();
        echoed(&mut kept, DESK, start, &[Some(5); 6]);
        echoed_in_video(
            &mut kept,
            DESK,
            start,
            &[Some(2500), Some(1500), None, None, None, None],
        );
        let measured = kept.of(DESK, start + Duration::from_secs(6));
        let measured = measured.expect("an access point echoed through is measured");
        assert!(
            (measured.lost - 100.0 / 3.0).abs() < 1e-9,
            "two held of six sent: {}",
            measured.lost
        );
        assert_eq!(measured.p95, Some(ms(5)), "best effort's own slow end");
    }

    #[test]
    fn video_is_timed_once_half_its_echoes_are_answered() {
        let answered = |echoes, answered| Video {
            echoes,
            answered,
            p50: None,
            p95: None,
            held: 0.0,
        };
        assert!(answered(4, 2).timed(), "half");
        assert!(answered(5, 3).timed(), "more than half");
        assert!(!answered(5, 2).timed(), "less than half");
        assert!(!answered(0, 0).timed(), "none sent");
    }

    #[test]
    fn a_held_video_class_s_loss_is_a_share_of_its_echoes_sent() {
        // Half answered, one of them late: a quarter of what was sent,
        // as best effort's loss is of its own.
        let start = Instant::now();
        let mut kept = Measures::default();
        echoed(&mut kept, DESK, start, &[Some(5); 4]);
        echoed_in_video(&mut kept, DESK, start, &[Some(1500), Some(7), None, None]);
        let measured = kept.of(DESK, start + Duration::from_secs(4));
        let measured = measured.expect("an access point echoed through is measured");
        assert!(
            (measured.lost - 25.0).abs() < 1e-9,
            "one late of four sent: {}",
            measured.lost
        );
        assert_eq!(measured.p95, Some(ms(1500)), "held as long as it was");
    }

    #[test]
    fn each_figure_is_the_worse_class_s_taken_apart() {
        // Best effort steadily 20 ms, video quicker at its middle but
        // slower at its end: neither class is worse at both.
        let start = Instant::now();
        let mut kept = Measures::default();
        echoed(&mut kept, DESK, start, &[Some(20); 4]);
        echoed_in_video(
            &mut kept,
            DESK,
            start,
            &[Some(5), Some(5), Some(5), Some(90)],
        );
        let measured = kept.of(DESK, start + Duration::from_secs(4));
        let measured = measured.expect("an access point echoed through is measured");
        assert_eq!(
            (measured.p50, measured.p95),
            (Some(ms(20)), Some(ms(90))),
            "best effort's middle, video's slow end"
        );
    }

    #[test]
    fn best_effort_never_answered_stays_as_slow_as_can_be_however_quick_video() {
        let start = Instant::now();
        let mut kept = Measures::default();
        echoed(&mut kept, DESK, start, &[None, None]);
        echoed_in_video(&mut kept, DESK, start, &[Some(7), Some(7)]);
        let measured = kept.of(DESK, start + Duration::from_secs(2));
        let measured = measured.expect("an access point echoed through is measured");
        assert_eq!((measured.p50, measured.p95), (None, None), "unanswered");
        assert!((measured.lost - 100.0).abs() < 1e-9, "{}", measured.lost);
    }

    #[test]
    fn a_late_answer_counts_on_the_access_point_its_echo_went_through() {
        let start = Instant::now();
        let mut kept = Measures::default();
        echoed_in_video(&mut kept, DESK, start, &[None]);
        echoed_in_video(&mut kept, HALL, start + Duration::from_secs(1), &[None]);
        kept.answered_late(Late {
            sent: start,
            took: ms(3100),
        });
        let now = start + Duration::from_secs(2);
        let desk = kept.video_of(DESK, now).expect("the desk's was echoed");
        let hall = kept.video_of(HALL, now).expect("the hall's was echoed");
        assert_eq!(desk.p95, Some(ms(3100)), "the desk held it");
        assert!((desk.held - 100.0).abs() < 1e-9, "late: {}", desk.held);
        assert_eq!(hall.answered, 0, "the hall's lost echo waits still");
    }

    #[test]
    fn a_late_answer_counts_for_the_echo_sent_when_it_was() {
        let start = Instant::now();
        let mut kept = Measures::default();
        echoed_in_video(&mut kept, DESK, start, &[None, None]);
        kept.answered_late(Late {
            sent: start + Duration::from_secs(1),
            took: ms(2100),
        });
        let now = start + Duration::from_secs(2);
        let video = kept.video_of(DESK, now).expect("video echoes were sent");
        assert_eq!(
            (video.answered, video.p95),
            (1, Some(ms(2100))),
            "one answered, late"
        );
        let first = kept.points[DESK].video.front().map(|(_, echo)| *echo);
        assert_eq!(first, Some(Echo::Lost), "the first still waits");
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
