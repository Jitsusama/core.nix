//! Replays shaper-record's day files through a controller, a second at a
//! time, and scores the rates it would have set, so a new controller is
//! proved on a real week of the connection before it shapes anything.
//!
//! A replay can't show what a rate would have done to the delay: the
//! connection carried what it carried under the rates of the day. So the
//! score is of the controller's own state against what the connection
//! showed: a rate held low while nothing needed it to be, a cut with no
//! queue of that direction's own to cause it, and a rate held under half
//! the connection while traffic wanted more.

use crate::oneway;
use crate::rate::{float, whole};
use crate::record;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, VecDeque};
use std::fmt::Write as _;
use std::fs::File;
use std::io::{BufRead, BufReader, Write as _};
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::process::ExitCode;

/// Counters further apart than this, in ms, are of a machine asleep or a
/// recorder stopped, and the seconds between them aren't the
/// connection's.
const GAP_MS: u64 = 60_000;
/// How long a reflector's least delay stands, in ms: long enough for a
/// queue to drain under it, short enough to follow a clock that drifts.
const WINDOW_MS: u64 = 600_000;
/// A rate under this share of the base is held low.
const LOW: f64 = 0.8;
/// A direction carrying under this share of its rate needs no less.
const IDLE: f64 = 0.5;
/// A spell held low counts once it lasts longer than this, in seconds.
const SPELL_S: u64 = 60;
/// A rate under this share of the base is starved, when the connection,
/// recorded unshaped, carried more than it with no queue of its own.
const STARVED: f64 = 0.5;
/// A direction's own delay over this, in ms, is cause for a cut.
const CAUSE_MS: f64 = 15.0;

/// What one second of the connection showed, as a controller is handed it.
#[derive(Clone, Debug, PartialEq)]
pub struct Second {
    /// Seconds since the stay began.
    pub t_s: f64,
    /// What the link carried each way over the second, in kbit/s.
    pub up_kbit: f64,
    pub down_kbit: f64,
    /// Each direction's own delay, in ms: what it took over the least it
    /// took in the ten minutes before, voted on by the reflectors heard in
    /// the second. None when no reflector was heard.
    pub up_delay_ms: Option<f64>,
    pub down_delay_ms: Option<f64>,
    /// The gateway's round trip, in ms, when it answered in the second.
    pub gateway_ms: Option<f64>,
    /// Whose the upload's queue was, as only the live shaper sees it; the
    /// recorder's lines don't say, so a replay leaves it None.
    pub up_ours: Option<crate::oneway::Ours>,
    /// A call's round trip through the VPN riding the link, in ms, as only
    /// the live shaper hears it (src/vpndelay.rs); a replay leaves it None.
    pub vpn_ms: Option<f64>,
}

/// The rates a controller sets each way, in kbit/s; None holds no limit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rates {
    pub up: Option<u64>,
    pub down: Option<u64>,
}

/// A controller under replay: handed each second, it answers the rates
/// that hold through the next.
pub trait Controller {
    fn second(&mut self, s: &Second) -> Rates;
}

/// The controller that limits nothing, which stands in until a real one
/// is replayed.
pub struct Unlimited;

impl Controller for Unlimited {
    fn second(&mut self, _: &Second) -> Rates {
        Rates::default()
    }
}

/// One line of a day file.
#[derive(Clone, Debug, PartialEq)]
pub enum Sample {
    /// The reflectors chosen; the first of a recording names its network,
    /// and each names the link it records, when the recorder wrote it.
    Pool {
        at: u64,
        network: Option<String>,
        link: Option<String>,
    },
    /// A timestamp answered: each way's delay, in ms, with the reflector's
    /// clock offset in it, and the link's byte counters.
    Stamp {
        at: u64,
        reflector: Ipv4Addr,
        out: f64,
        back: f64,
        rx: u64,
        tx: u64,
    },
    /// A timestamp answered by a clock that isn't counted from midnight in
    /// UTC, so it gives no delay, only the counters.
    Clock { at: u64, rx: u64, tx: u64 },
    /// The gateway's echo, its round trip in ms.
    Gateway { at: u64, rtt: f64, rx: u64, tx: u64 },
    /// A probe that didn't come back.
    Lost { at: u64 },
}

/// A line read as a sample, or None for one this doesn't know.
pub fn parse(line: &str) -> Option<Sample> {
    let v: Value = serde_json::from_str(line).ok()?;
    let at = v.get("at")?.as_u64()?;
    let counters = || Some((v.get("rx")?.as_u64()?, v.get("tx")?.as_u64()?));
    let number = |key: &str| v.get(key).and_then(Value::as_f64);
    if v.get("lost").and_then(Value::as_bool) == Some(true) {
        return Some(Sample::Lost { at });
    }
    if v.get("pool").is_some() {
        let named = |key: &str| v.get(key).and_then(Value::as_str).map(String::from);
        return Some(Sample::Pool {
            at,
            network: named("network"),
            link: named("link"),
        });
    }
    if v.get("gateway").is_some() {
        let (rx, tx) = counters()?;
        let rtt = number("gateway")?;
        return Some(Sample::Gateway { at, rtt, rx, tx });
    }
    let reflector = v.get("reflector")?.as_str()?.parse().ok()?;
    let (rx, tx) = counters()?;
    if v.get("clock").is_some() {
        return Some(Sample::Clock { at, rx, tx });
    }
    Some(Sample::Stamp {
        at,
        reflector,
        out: number("out")?,
        back: number("back")?,
        rx,
        tx,
    })
}

impl Sample {
    pub(crate) fn at(&self) -> u64 {
        match self {
            Sample::Pool { at, .. }
            | Sample::Stamp { at, .. }
            | Sample::Clock { at, .. }
            | Sample::Gateway { at, .. }
            | Sample::Lost { at } => *at,
        }
    }
}

/// The least a reflector's delay one way took over the window before each
/// sample, kept as the samples that could still be the least.
#[derive(Default)]
struct Least {
    kept: VecDeque<(u64, f64)>,
}

impl Least {
    /// What a sample took over the least of the window it closes.
    fn over(&mut self, at: u64, delay: f64) -> f64 {
        while self.kept.back().is_some_and(|&(_, d)| d >= delay) {
            self.kept.pop_back();
        }
        self.kept.push_back((at, delay));
        while self.kept.front().is_some_and(|&(t, _)| t + WINDOW_MS < at) {
            self.kept.pop_front();
        }
        delay - self.kept.front().map_or(delay, |&(_, d)| d)
    }
}

/// What a second is gathering while its samples come in.
#[derive(Default)]
struct Tick {
    /// kbit carried each way, and over how many ms of the second.
    up_kbit: f64,
    down_kbit: f64,
    covered_ms: f64,
    /// Each reflector's latest own delay each way.
    up: BTreeMap<Ipv4Addr, f64>,
    down: BTreeMap<Ipv4Addr, f64>,
    gateway: Vec<f64>,
}

/// The middle of some values, or the mean of the middle two.
pub fn median(values: impl Iterator<Item = f64>) -> Option<f64> {
    let mut all: Vec<f64> = values.collect();
    all.sort_by(f64::total_cmp);
    let half = all.len() / 2;
    match all.len() {
        0 => None,
        n if n % 2 == 1 => Some(all[half]),
        _ => Some(f64::midpoint(all[half - 1], all[half])),
    }
}

/// A stay being read, a second at a time: a second is done once a counter
/// read after it has come, since the samples come in order.
pub(crate) struct Reading {
    network: String,
    first: Option<u64>,
    ticks: BTreeMap<u64, Tick>,
    /// Seconds before this one are done, and a sample for one is late.
    done: u64,
    counted: Option<(u64, u64, u64)>,
    out: BTreeMap<Ipv4Addr, Least>,
    back: BTreeMap<Ipv4Addr, Least>,
    /// Each reflector's round trip, out and back together, which a clock
    /// stepping can't move, since it moves the two by opposite amounts.
    round: BTreeMap<Ipv4Addr, Least>,
    seconds: Vec<Second>,
}

impl Reading {
    pub(crate) fn new(network: String) -> Self {
        Reading {
            network,
            first: None,
            ticks: BTreeMap::new(),
            done: 0,
            counted: None,
            out: BTreeMap::new(),
            back: BTreeMap::new(),
            round: BTreeMap::new(),
            seconds: Vec::new(),
        }
    }

    /// The seconds finished since this was last asked, for a reader that
    /// steps a controller as the samples come rather than at a stay's end.
    pub(crate) fn drain(&mut self) -> Vec<Second> {
        std::mem::take(&mut self.seconds)
    }

    pub(crate) fn take(&mut self, sample: &Sample) {
        let at = sample.at();
        let second = at / 1_000;
        // A clock set back writes a moment already past; it's passed over.
        // A new pool says nothing of the connection.
        if second < self.done || matches!(sample, Sample::Pool { .. }) {
            return;
        }
        self.first.get_or_insert(second);
        let tick = self.ticks.entry(second).or_default();
        match *sample {
            Sample::Stamp {
                reflector,
                out,
                back,
                rx,
                tx,
                ..
            } => {
                // No way's own delay is more than the round trip rose: a
                // queue raises both, a clock stepping only the one.
                let rose = self
                    .round
                    .entry(reflector)
                    .or_default()
                    .over(at, out + back);
                let up = self
                    .out
                    .entry(reflector)
                    .or_default()
                    .over(at, out)
                    .min(rose);
                let down = self
                    .back
                    .entry(reflector)
                    .or_default()
                    .over(at, back)
                    .min(rose);
                tick.up.insert(reflector, up);
                tick.down.insert(reflector, down);
                self.count(at, rx, tx);
            }
            Sample::Gateway { rtt, rx, tx, .. } => {
                tick.gateway.push(rtt);
                self.count(at, rx, tx);
            }
            Sample::Clock { rx, tx, .. } => self.count(at, rx, tx),
            // A probe lost still shows the recorder was listening.
            Sample::Pool { .. } | Sample::Lost { .. } => {}
        }
    }

    /// Spreads what the link carried since its last counters over the
    /// seconds between, as evenly as the counters can tell, and finishes
    /// the seconds before this one.
    fn count(&mut self, at: u64, received: u64, sent: u64) {
        if let Some((then, received_then, sent_then)) = self.counted {
            if at <= then {
                return;
            }
            // Counters that went back are of a link made anew.
            if at - then <= GAP_MS && received >= received_then && sent >= sent_then {
                let kbit = |bytes: u64| float(bytes) * 8.0 / 1_000.0;
                self.spread(
                    then,
                    at,
                    kbit(sent - sent_then),
                    kbit(received - received_then),
                );
            }
        }
        self.counted = Some((at, received, sent));
        self.finish_before(at / 1_000);
    }

    fn spread(&mut self, from: u64, to: u64, up_kbit: f64, down_kbit: f64) {
        let span = float(to - from);
        for second in from / 1_000..=(to - 1) / 1_000 {
            let start = (second * 1_000).max(from);
            let end = ((second + 1) * 1_000).min(to);
            let share = float(end - start) / span;
            let tick = self.ticks.entry(second).or_default();
            tick.up_kbit += up_kbit * share;
            tick.down_kbit += down_kbit * share;
            tick.covered_ms += float(end - start);
        }
    }

    fn finish_before(&mut self, second: u64) {
        while let Some(entry) = self.ticks.first_entry()
            && *entry.key() < second
        {
            let (at, tick) = entry.remove_entry();
            let per_second = |kbit: f64| {
                if tick.covered_ms > 0.0 {
                    kbit * 1_000.0 / tick.covered_ms
                } else {
                    0.0
                }
            };
            self.seconds.push(Second {
                t_s: float(at - self.first.unwrap_or(at)),
                up_kbit: per_second(tick.up_kbit),
                down_kbit: per_second(tick.down_kbit),
                up_delay_ms: median(tick.up.values().copied()),
                down_delay_ms: median(tick.down.values().copied()),
                gateway_ms: median(tick.gateway.iter().copied()),
                up_ours: None,
                vpn_ms: None,
            });
        }
        self.done = self.done.max(second);
    }

    fn finish(mut self) -> Option<Stay> {
        self.finish_before(u64::MAX);
        Some(Stay {
            network: self.network,
            began_ms: self.first? * 1_000,
            seconds: self.seconds,
        })
    }
}

/// A stay on one network, second by second.
#[derive(Debug)]
pub struct Stay {
    pub network: String,
    /// When its first second began, in ms since 1970.
    pub began_ms: u64,
    pub seconds: Vec<Second>,
}

/// Samples in the order they were written, as stays.
pub fn stays(samples: impl IntoIterator<Item = Sample>) -> Vec<Stay> {
    let mut all = Vec::new();
    let mut reading: Option<Reading> = None;
    for sample in samples {
        if let Sample::Pool {
            network: Some(network),
            ..
        } = &sample
        {
            all.extend(reading.take().and_then(Reading::finish));
            reading = Some(Reading::new(network.clone()));
            continue;
        }
        reading
            .get_or_insert_with(|| Reading::new("unknown".into()))
            .take(&sample);
    }
    all.extend(reading.and_then(Reading::finish));
    all
}

/// The stays in day files, read in the order of their names, which is the
/// order of their days.
pub fn read(files: &[PathBuf]) -> Result<Vec<Stay>, String> {
    let mut sorted = files.to_vec();
    sorted.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    let opened = sorted
        .iter()
        .map(|path| {
            File::open(path)
                .map(BufReader::new)
                .map_err(|e| format!("{}: {e}", path.display()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    // A line that isn't text ends its file, and the next file is read on.
    let lines = opened
        .into_iter()
        .flat_map(|file| file.lines().map_while(Result::ok));
    Ok(stays(lines.filter_map(|line| parse(&line))))
}

/// What a direction is held against, in kbit/s.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Base {
    pub up: u64,
    pub down: u64,
}

/// How one direction's rates scored.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Score {
    /// Seconds in spells of over a minute with the rate under 0.8 of the
    /// base while the direction carried under half the rate.
    pub held_idle_s: u64,
    pub longest_held_idle_s: u64,
    /// Every fall of over 5% in a second.
    pub cuts: u64,
    /// Those made with the direction's own delay not over 15 ms.
    pub unfounded: u64,
    /// Seconds with the rate under half the base while the direction,
    /// recorded unshaped, carried more than the rate with its own delay
    /// not over 15 ms: what holding it there would have held back for no
    /// queue. Past a queue, holding it is the cut's work, which a replay
    /// can't weigh, since it can't show the queue a rate would drain.
    pub starved_s: u64,
}

/// A cut made with no queue of the direction's own to cause it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Cut {
    pub way: &'static str,
    pub t_s: f64,
    pub from: Option<u64>,
    pub to: u64,
    pub delay_ms: Option<f64>,
}

/// How a stay scored.
#[derive(Debug, Serialize)]
pub struct Scored {
    pub network: String,
    pub began_ms: u64,
    pub seconds: u64,
    pub base: Base,
    pub up: Score,
    pub down: Score,
    pub unfounded_cuts: Vec<Cut>,
}

/// How every stay scored, and the whole of them.
#[derive(Debug, Serialize)]
pub struct Report {
    pub stays: Vec<Scored>,
    pub up: Score,
    pub down: Score,
}

/// Replays a stay through a controller, held against `base`, or against
/// the most the stay carried each way in a second when none is given.
///
/// The rates a controller answers for a second hold through the next, so
/// each second is judged by the rates its predecessor answered, and a cut
/// by the second the controller was handed when it made it.
pub fn score(stay: &Stay, controller: &mut impl Controller, base: Option<Base>) -> Scored {
    let base = base.unwrap_or_else(|| Base {
        up: whole(sustained(&stay.seconds, |s| s.up_kbit)).max(1),
        down: whole(sustained(&stay.seconds, |s| s.down_kbit)).max(1),
    });
    let (mut up, mut down) = (Tally::new("up", base.up), Tally::new("down", base.down));
    let mut holding = Rates::default();
    let mut answered: Option<Rates> = None;
    let mut last: Option<f64> = None;
    let mut unfounded_cuts = Vec::new();
    for s in &stay.seconds {
        let joined = last.is_some_and(|t| s.t_s - t < 1.5);
        last = Some(s.t_s);
        up.carry(joined, s.up_kbit, holding.up, s.up_delay_ms);
        down.carry(joined, s.down_kbit, holding.down, s.down_delay_ms);
        let now = controller.second(s);
        if let Some(before) = answered {
            unfounded_cuts.extend(up.cut(s.t_s, before.up, now.up, s.up_delay_ms));
            unfounded_cuts.extend(down.cut(s.t_s, before.down, now.down, s.down_delay_ms));
        }
        answered = Some(now);
        holding = now;
    }
    Scored {
        network: stay.network.clone(),
        began_ms: stay.began_ms,
        seconds: u64::try_from(stay.seconds.len()).unwrap_or(u64::MAX),
        base,
        up: up.finish(),
        down: down.finish(),
        unfounded_cuts,
    }
}

/// How long a rate must be carried to stand for the connection, in
/// seconds.
const SUSTAINED_S: usize = 5;

/// The most a direction carried for `SUSTAINED_S` seconds running, or
/// in any one second of a shorter stay. A burst out of a buffer upstream
/// can carry a second over the connection; it can't carry five. A percentile
/// would be steadier still on a busy week, but on an idle one it lands on
/// idle seconds.
fn sustained(seconds: &[Second], kbit: fn(&Second) -> f64) -> f64 {
    let joined = |w: &[Second]| w[w.len() - 1].t_s - w[0].t_s < float(SUSTAINED_S as u64);
    seconds
        .windows(SUSTAINED_S)
        .filter(|w| joined(w))
        .map(|w| w.iter().map(kbit).fold(f64::INFINITY, f64::min))
        .reduce(f64::max)
        .unwrap_or_else(|| seconds.iter().map(kbit).fold(0.0, f64::max))
}

/// One direction's score as its seconds go by.
struct Tally {
    way: &'static str,
    base: f64,
    score: Score,
    /// How long the rate has been held low on an idle connection, in
    /// seconds.
    spell: u64,
}

impl Tally {
    fn new(way: &'static str, base: u64) -> Self {
        Tally {
            way,
            base: float(base),
            score: Score::default(),
            spell: 0,
        }
    }

    /// A second the direction carried `carried` kbit/s under `rate` with
    /// its own delay `delay_ms`, the one before it `joined` to it or not.
    fn carry(&mut self, joined: bool, carried: f64, rate: Option<u64>, delay_ms: Option<f64>) {
        if !joined {
            self.end_spell();
        }
        let Some(rate) = rate.map(float) else {
            return self.end_spell();
        };
        if rate < LOW * self.base && carried < IDLE * rate {
            self.spell += 1;
        } else {
            self.end_spell();
        }
        if rate < STARVED * self.base
            && carried > rate
            && delay_ms.is_none_or(|delay| delay <= CAUSE_MS)
        {
            self.score.starved_s += 1;
        }
    }

    fn end_spell(&mut self) {
        if self.spell > SPELL_S {
            self.score.held_idle_s += self.spell;
            self.score.longest_held_idle_s = self.score.longest_held_idle_s.max(self.spell);
        }
        self.spell = 0;
    }

    /// Counts a fall of over 5% from `before` to `after`, a limit set
    /// where there was none among them, and gives it back when the
    /// direction's own delay gave no cause for it.
    fn cut(
        &mut self,
        t_s: f64,
        before: Option<u64>,
        after: Option<u64>,
        delay_ms: Option<f64>,
    ) -> Option<Cut> {
        let to = after?;
        if before.is_some_and(|from| to * 100 >= from * 95) {
            return None;
        }
        self.score.cuts += 1;
        if delay_ms.is_some_and(|d| d > CAUSE_MS) {
            return None;
        }
        self.score.unfounded += 1;
        Some(Cut {
            way: self.way,
            t_s,
            from: before,
            to,
            delay_ms,
        })
    }

    fn finish(mut self) -> Score {
        self.end_spell();
        self.score
    }
}

impl Score {
    fn add(&mut self, other: &Score) {
        self.held_idle_s += other.held_idle_s;
        self.longest_held_idle_s = self.longest_held_idle_s.max(other.longest_held_idle_s);
        self.cuts += other.cuts;
        self.unfounded += other.unfounded;
        self.starved_s += other.starved_s;
    }
}

/// Replays each stay through a controller of its own, as the shaper starts
/// one afresh on each network.
pub fn replay<C: Controller>(
    stays: &[Stay],
    base: Option<Base>,
    mut make: impl FnMut() -> C,
) -> Report {
    let scored: Vec<Scored> = stays
        .iter()
        .map(|stay| score(stay, &mut make(), base))
        .collect();
    let (mut up, mut down) = (Score::default(), Score::default());
    for s in &scored {
        up.add(&s.up);
        down.add(&s.down);
    }
    Report {
        stays: scored,
        up,
        down,
    }
}

/// A length of time as a person reads it, to the minute past an hour.
fn span(seconds: u64) -> String {
    let (days, hours, minutes) = (seconds / 86_400, seconds / 3_600 % 24, seconds / 60 % 60);
    match seconds {
        0..60 => format!("{seconds} s"),
        60..3_600 => format!("{minutes} min {} s", seconds % 60),
        3_600..86_400 => format!("{hours} h {minutes} min"),
        _ => format!("{days} d {hours} h"),
    }
}

fn mbit(kbit: u64) -> String {
    format!("{:.1}", float(kbit) / 1_000.0)
}

/// When a stay began, in UTC, to the minute.
fn moment(ms: u64) -> String {
    let seconds = ms / 1_000;
    let (y, m, d) = record::civil(i64::try_from(seconds / 86_400).unwrap_or(0));
    let (h, min) = (seconds / 3_600 % 24, seconds / 60 % 60);
    format!("{y:04}-{m:02}-{d:02} {h:02}:{min:02} UTC")
}

/// A report as a few lines for a person: the whole, what each count means,
/// then each stay.
pub fn summary(report: &Report) -> String {
    let mut out = String::new();
    let stays = report.stays.len();
    let seconds = report.stays.iter().map(|s| s.seconds).sum();
    let plural = if stays == 1 { "" } else { "s" };
    let _ = writeln!(
        out,
        "{stays} stay{plural}, {} of the connection\n",
        span(seconds)
    );
    let _ = writeln!(out, "{:<16}{:>12}{:>12}", "", "up", "down");
    let (up, down) = (&report.up, &report.down);
    let rows: [(&str, String, String); 5] = [
        ("held low", span(up.held_idle_s), span(down.held_idle_s)),
        (
            "  longest",
            span(up.longest_held_idle_s),
            span(down.longest_held_idle_s),
        ),
        ("cuts", up.cuts.to_string(), down.cuts.to_string()),
        (
            "  without cause",
            up.unfounded.to_string(),
            down.unfounded.to_string(),
        ),
        ("starved", span(up.starved_s), span(down.starved_s)),
    ];
    for (name, up, down) in rows {
        let _ = writeln!(out, "{name:<16}{up:>12}{down:>12}");
    }
    let _ = writeln!(
        out,
        "\nheld low: under {LOW} of the base for over a minute, carrying under {IDLE} of \
         the rate\nwithout cause: cut with the direction's own delay not over {CAUSE_MS} ms\n\
         starved: under {STARVED} of the base, carrying more than the rate with the \
         direction's own delay not over {CAUSE_MS} ms"
    );
    for s in &report.stays {
        let _ = writeln!(
            out,
            "\n{}, from {} for {}, against {} up and {} down Mbit/s",
            s.network,
            moment(s.began_ms),
            span(s.seconds),
            mbit(s.base.up),
            mbit(s.base.down)
        );
        for (way, score) in [("up", &s.up), ("down", &s.down)] {
            let _ = writeln!(
                out,
                "  {way:<6}held low {}, longest {}; {} cuts, {} without cause; starved {}",
                span(score.held_idle_s),
                span(score.longest_held_idle_s),
                score.cuts,
                score.unfounded,
                span(score.starved_s)
            );
        }
    }
    out
}

const USAGE: &str = "shaper replay [--base UP,DOWN] [--controller one-way|none] [--json] FILES...";

/// Which controller a replay runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// The rebuilt controller, from each direction's own delay
    /// (src/oneway.rs), starting each stay as on a network it never met.
    OneWay,
    /// The stand-in that limits nothing, for what the connection alone
    /// scores.
    Unlimited,
}

/// What `shaper replay` was asked.
#[derive(Debug, PartialEq, Eq)]
pub struct Asked {
    pub base: Option<Base>,
    pub controller: Choice,
    pub json: bool,
    pub files: Vec<PathBuf>,
}

pub fn asked(args: &[String]) -> Result<Asked, String> {
    let mut asked = Asked {
        base: None,
        controller: Choice::OneWay,
        json: false,
        files: Vec::new(),
    };
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--base" => {
                let value = it.next().map_or("", String::as_str);
                let (up, down) = value
                    .split_once(',')
                    .and_then(|(u, d)| Some((u.trim().parse().ok()?, d.trim().parse().ok()?)))
                    .ok_or_else(|| format!("--base takes UP,DOWN in kbit/s, not {value:?}"))?;
                asked.base = Some(Base { up, down });
            }
            "--controller" => {
                asked.controller = match it.next().map_or("", String::as_str) {
                    "one-way" => Choice::OneWay,
                    "none" => Choice::Unlimited,
                    other => return Err(format!("--controller is one-way or none, not {other:?}")),
                };
            }
            "--json" => asked.json = true,
            file => asked.files.push(file.into()),
        }
    }
    if asked.files.is_empty() {
        return Err("name the day files to replay".into());
    }
    Ok(asked)
}

/// `shaper replay [--base UP,DOWN] [--controller one-way|none] [--json]
/// FILES...`: replays the files through the controller chosen, the rebuilt
/// one unless told otherwise, and prints the summary, or the report as
/// JSON.
pub fn command(args: &[String]) -> ExitCode {
    let asked = match asked(args) {
        Ok(asked) => asked,
        Err(e) => {
            eprintln!("{e}\nusage: {USAGE}");
            return ExitCode::from(2);
        }
    };
    let stays = match read(&asked.files) {
        Ok(stays) => stays,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let report = match asked.controller {
        Choice::OneWay => replay(&stays, asked.base, || oneway::Controller::new(None, None)),
        Choice::Unlimited => replay(&stays, asked.base, || Unlimited),
    };
    let text = if asked.json {
        serde_json::to_string(&report).expect("a report serialises") + "\n"
    } else {
        summary(&report)
    };
    // Written whole, so a reader that stops early ends it without a panic.
    let _ = std::io::stdout().write_all(text.as_bytes());
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    /// Midnight in UTC on 3 November 2026, in ms.
    const NOV3: u64 = 1_793_664_000_000;
    /// Where every test's byte counters count from.
    const COUNTED_FROM: u64 = NOV3 - 7 * 86_400_000;
    /// Three reflectors and their clocks' offsets, in ms.
    const REFLECTORS: [(&str, f64); 3] = [
        ("9.9.9.9", 500.0),
        ("94.140.14.14", -300.0),
        ("208.67.222.222", 20.0),
    ];

    fn pool(at: u64, network: &str) -> String {
        json!({"at": at, "pool": [["Quad9", "9.9.9.9"]], "operators": 1, "enough": false,
               "link": "wlan0", "network": network})
        .to_string()
    }

    fn repool(at: u64) -> String {
        json!({"at": at, "pool": [], "operators": 0, "enough": false}).to_string()
    }

    fn stamp(at: u64, reflector: &str, (out, back): (f64, f64), (rx, tx): (u64, u64)) -> String {
        json!({"at": at, "reflector": reflector, "operator": "Quad9", "out": out, "back": back,
               "rtt": 20.0, "rx": rx, "tx": tx})
        .to_string()
    }

    fn clock(at: u64, (rx, tx): (u64, u64)) -> String {
        json!({"at": at, "reflector": "9.9.9.9", "operator": "Quad9", "clock": "not standard",
               "rtt": 20.0, "rx": rx, "tx": tx})
        .to_string()
    }

    fn gateway(at: u64, rtt: f64, (rx, tx): (u64, u64)) -> String {
        json!({"at": at, "gateway": rtt, "rx": rx, "tx": tx}).to_string()
    }

    fn lost(at: u64) -> String {
        json!({"at": at, "reflector": "9.9.9.9", "operator": "Quad9", "lost": true}).to_string()
    }

    fn lost_gateway(at: u64) -> String {
        json!({"at": at, "gateway": null, "lost": true}).to_string()
    }

    /// The counters of a connection carrying `up` and `down` kbit/s,
    /// received and sent, at a moment.
    fn constant(up: u64, down: u64) -> impl Fn(u64) -> (u64, u64) {
        move |at| ((at - COUNTED_FROM) * down / 8, (at - COUNTED_FROM) * up / 8)
    }

    /// The recorded lines of a connection heard each second from `from`
    /// for `seconds`: each reflector at its turn, its delays 10 ms out and
    /// 12 back over its clock's offset, plus what `queue` adds for that
    /// second and reflector, and the gateway at 4 ms.
    fn connection(
        from: u64,
        seconds: u64,
        counters: impl Fn(u64) -> (u64, u64),
        queue: impl Fn(u64, usize) -> (f64, f64),
    ) -> Vec<String> {
        let mut lines = Vec::new();
        for t in 0..seconds {
            for (i, (address, offset)) in REFLECTORS.iter().enumerate() {
                let at = from + t * 1_000 + 100 + 300 * i as u64;
                let (out, back) = queue(t, i);
                lines.push(stamp(
                    at,
                    address,
                    (10.0 + offset + out, 12.0 - offset + back),
                    counters(at),
                ));
            }
            let at = from + t * 1_000 + 950;
            lines.push(gateway(at, 4.0, counters(at)));
        }
        lines
    }

    fn quiet(_: u64, _: usize) -> (f64, f64) {
        (0.0, 0.0)
    }

    fn read_lines(lines: &[String]) -> Vec<Stay> {
        stays(lines.iter().filter_map(|l| parse(l)))
    }

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 0.01
    }

    fn near_some(a: Option<f64>, b: f64) -> bool {
        a.is_some_and(|a| near(a, b))
    }

    #[test]
    fn a_line_it_cannot_read_is_passed_over() {
        assert_eq!(parse("not json"), None);
        assert_eq!(parse("[1, 2]"), None);
        assert_eq!(parse(r#"{"at": 1, "something": "else"}"#), None);
        assert_eq!(
            parse(r#"{"reflector": "9.9.9.9", "out": 1}"#),
            None,
            "no time"
        );
        assert!(matches!(
            parse(&stamp(5, "9.9.9.9", (1.0, 2.0), (3, 4))),
            Some(Sample::Stamp {
                at: 5,
                rx: 3,
                tx: 4,
                ..
            })
        ));
        assert_eq!(
            parse(&clock(5, (3, 4))),
            Some(Sample::Clock {
                at: 5,
                rx: 3,
                tx: 4
            })
        );
        assert_eq!(
            parse(&gateway(5, 4.0, (3, 4))),
            Some(Sample::Gateway {
                at: 5,
                rtt: 4.0,
                rx: 3,
                tx: 4
            })
        );
        assert_eq!(parse(&lost(5)), Some(Sample::Lost { at: 5 }));
        assert_eq!(parse(&lost_gateway(5)), Some(Sample::Lost { at: 5 }));
        assert_eq!(
            parse(&pool(5, "aa:bb")),
            Some(Sample::Pool {
                at: 5,
                network: Some("aa:bb".into()),
                link: Some("wlan0".into())
            })
        );
        assert_eq!(
            parse(&repool(5)),
            Some(Sample::Pool {
                at: 5,
                network: None,
                link: None
            })
        );
    }

    #[test]
    fn what_the_link_carried_is_read_from_its_counters() {
        let mut lines = vec![pool(NOV3, "home")];
        lines.extend(connection(NOV3, 120, constant(8_000, 40_000), quiet));
        let stays = read_lines(&lines);
        assert_eq!(stays.len(), 1);
        let seconds = &stays[0].seconds;
        assert_eq!(seconds.len(), 120);
        for s in seconds {
            assert!(near(s.up_kbit, 8_000.0), "{s:?}");
            assert!(near(s.down_kbit, 40_000.0), "{s:?}");
            assert!(near_some(s.gateway_ms, 4.0), "{s:?}");
        }
    }

    #[test]
    fn each_direction_has_its_own_delay_over_each_reflector_s_least() {
        // Uploads queue 30 ms for a minute; the clocks' offsets differ by
        // 800 ms between reflectors and never show.
        let queue = |t: u64, _| {
            if (60..120).contains(&t) {
                (30.0, 0.0)
            } else {
                (0.0, 0.0)
            }
        };
        let stays = read_lines(&connection(NOV3, 180, constant(8_000, 40_000), queue));
        let seconds = &stays[0].seconds;
        for s in seconds {
            let queued = if (60.0..120.0).contains(&s.t_s) {
                30.0
            } else {
                0.0
            };
            assert!(near_some(s.up_delay_ms, queued), "{s:?}");
            assert!(near_some(s.down_delay_ms, 0.0), "{s:?}");
        }
    }

    #[test]
    fn a_clock_stepping_is_no_queue_either_way() {
        // As the replay of 5 November found: a clock stepped 5.2 s a
        // minute in, out shorter by it and back longer, the round trip
        // unmoved. Read over the least before it, the download stood
        // 5,213 ms late until the shaper gave up.
        let queue = |t: u64, _| {
            if t >= 60 {
                (-5_200.0, 5_200.0)
            } else {
                (0.0, 0.0)
            }
        };
        let seconds = &read_lines(&connection(NOV3, 180, constant(0, 0), queue))[0].seconds;
        for s in seconds {
            assert!(near_some(s.up_delay_ms, 0.0), "{s:?}");
            assert!(near_some(s.down_delay_ms, 0.0), "{s:?}");
        }
    }

    #[test]
    fn one_reflector_s_spike_is_outvoted() {
        let queue = |_, reflector| {
            if reflector == 0 {
                (0.0, 80.0)
            } else {
                (0.0, 0.0)
            }
        };
        let mut lines = connection(NOV3, 10, constant(0, 0), quiet);
        lines.extend(connection(NOV3 + 10_000, 10, constant(0, 0), queue));
        let seconds = &read_lines(&lines)[0].seconds;
        assert!(seconds.iter().all(|s| near_some(s.down_delay_ms, 0.0)));
    }

    #[test]
    fn a_reflector_s_least_is_forgotten_after_ten_minutes() {
        // Every reflector's clock steps 50 ms at five minutes in.
        let queue = |t: u64, _| if t >= 300 { (50.0, 0.0) } else { (0.0, 0.0) };
        let seconds = &read_lines(&connection(NOV3, 1_000, constant(0, 0), queue))[0].seconds;
        assert!(near_some(seconds[360].up_delay_ms, 50.0));
        assert!(near_some(seconds[899].up_delay_ms, 50.0));
        assert!(near_some(seconds[950].up_delay_ms, 0.0));
    }

    #[test]
    fn a_stay_runs_on_across_midnight() {
        let folder = std::env::temp_dir().join(format!("shaper-replay-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let mut before = vec![pool(NOV3 - 120_000, "home")];
        before.extend(connection(
            NOV3 - 120_000,
            120,
            constant(8_000, 40_000),
            quiet,
        ));
        let after = connection(NOV3, 120, constant(8_000, 40_000), quiet);
        let (first, second) = (
            folder.join("2026-11-02.jsonl"),
            folder.join("2026-11-03.jsonl"),
        );
        fs::write(&first, before.join("\n") + "\n").unwrap();
        fs::write(&second, after.join("\n") + "\n").unwrap();
        // Named out of order, as a shell's glob never does but a person might.
        let stays = read(&[second, first]).unwrap();
        fs::remove_dir_all(&folder).unwrap();
        assert_eq!(stays.len(), 1);
        let seconds = &stays[0].seconds;
        assert_eq!(seconds.len(), 240);
        assert!(seconds.windows(2).all(|w| near(w[1].t_s - w[0].t_s, 1.0)));
        assert!(seconds.iter().all(|s| near(s.up_kbit, 8_000.0)));
    }

    #[test]
    fn a_file_that_is_not_there_is_said() {
        let missing = std::env::temp_dir().join("shaper-replay-nothing-here.jsonl");
        let error = read(&[missing]).unwrap_err();
        assert!(
            error.contains("shaper-replay-nothing-here.jsonl"),
            "{error}"
        );
    }

    #[test]
    fn a_new_network_starts_a_stay_and_a_new_pool_does_not() {
        let mut lines = vec![pool(NOV3, "home")];
        lines.extend(connection(NOV3, 60, constant(8_000, 40_000), quiet));
        lines.push(repool(NOV3 + 60_000));
        lines.extend(connection(
            NOV3 + 60_000,
            60,
            constant(8_000, 40_000),
            quiet,
        ));
        lines.push(pool(NOV3 + 120_000, "cafe"));
        // The new network's link counts from nothing.
        let counters = |at: u64| constant(800, 4_000)(at - (NOV3 + 120_000) + COUNTED_FROM);
        lines.extend(connection(NOV3 + 120_000, 60, counters, quiet));
        let stays = read_lines(&lines);
        let named: Vec<(&str, usize)> = stays
            .iter()
            .map(|s| (s.network.as_str(), s.seconds.len()))
            .collect();
        assert_eq!(named, [("home", 120), ("cafe", 60)]);
        assert_eq!(stays[1].began_ms, NOV3 + 120_000);
        assert!(stays[1].seconds.iter().all(|s| near(s.up_kbit, 800.0)));
    }

    #[test]
    fn samples_before_any_network_are_a_stay_of_their_own() {
        let stays = read_lines(&connection(NOV3, 5, constant(0, 0), quiet));
        assert_eq!(stays[0].network, "unknown");
    }

    #[test]
    fn lost_and_clock_lines_keep_the_seconds_without_a_delay() {
        let counters = constant(8_000, 40_000);
        let mut lines = vec![pool(NOV3, "home")];
        lines.extend(
            (0..10).map(|t| clock(NOV3 + t * 1_000 + 100, counters(NOV3 + t * 1_000 + 100))),
        );
        lines.extend(
            (10..15).flat_map(|t| [lost(NOV3 + t * 1_000), lost_gateway(NOV3 + t * 1_000 + 500)]),
        );
        lines.extend(connection(NOV3 + 15_000, 10, &counters, quiet));
        let seconds = &read_lines(&lines)[0].seconds;
        assert_eq!(seconds.len(), 25);
        for s in &seconds[..15] {
            assert_eq!((s.up_delay_ms, s.gateway_ms), (None, None), "{s:?}");
        }
        // A second is carried across from the counters either side of it.
        assert!(seconds[1..].iter().all(|s| near(s.up_kbit, 8_000.0)));
        assert!(seconds[15..].iter().all(|s| near_some(s.up_delay_ms, 0.0)));
    }

    #[test]
    fn a_machine_asleep_leaves_no_seconds() {
        let mut lines = connection(NOV3, 60, constant(8_000, 40_000), quiet);
        lines.extend(connection(
            NOV3 + 7_200_000,
            60,
            constant(8_000, 40_000),
            quiet,
        ));
        let seconds = &read_lines(&lines)[0].seconds;
        assert_eq!(seconds.len(), 120);
        assert!(near(seconds[60].t_s, 7_200.0));
        assert!(near(seconds[60].up_kbit, 8_000.0), "{:?}", seconds[60]);
    }

    /// A controller that holds the same rates whatever it's handed.
    struct Fixed(Rates);

    impl Controller for Fixed {
        fn second(&mut self, _: &Second) -> Rates {
            self.0
        }
    }

    /// A controller that cuts its upload a tenth every ten seconds, queue
    /// or none.
    struct Cutter {
        rate: u64,
        heard: u64,
    }

    impl Controller for Cutter {
        fn second(&mut self, _: &Second) -> Rates {
            self.heard += 1;
            if self.heard.is_multiple_of(10) {
                self.rate = self.rate * 9 / 10;
            }
            Rates {
                up: Some(self.rate),
                down: None,
            }
        }
    }

    const BASE: Base = Base {
        up: 10_000,
        down: 50_000,
    };

    fn fixed(up: u64) -> Fixed {
        Fixed(Rates {
            up: Some(up),
            down: None,
        })
    }

    #[test]
    fn no_limit_scores_nothing() {
        let stays = read_lines(&connection(NOV3, 300, constant(0, 0), quiet));
        let report = replay(&stays, Some(BASE), || Unlimited);
        assert_eq!(
            (&report.up, &report.down),
            (&Score::default(), &Score::default())
        );
        assert_eq!(
            report.stays[0].unfounded_cuts,
            [] as [crate::replay::Cut; 0]
        );
    }

    #[test]
    fn a_cut_without_its_own_delay_is_named() {
        // Uploads queue 30 ms from 25 s to 45 s, so of the cuts at 9, 19,
        // 29, 39, 49 and 59 s, those at 29 and 39 have a cause.
        let queue = |t: u64, _| {
            if (25..45).contains(&t) {
                (30.0, 0.0)
            } else {
                (0.0, 0.0)
            }
        };
        let stays = read_lines(&connection(NOV3, 60, constant(8_000, 40_000), queue));
        let scored = score(
            &stays[0],
            &mut Cutter {
                rate: 10_000,
                heard: 0,
            },
            Some(BASE),
        );
        assert_eq!((scored.up.cuts, scored.up.unfounded), (6, 4));
        assert_eq!(scored.down, Score::default());
        let when: Vec<f64> = scored.unfounded_cuts.iter().map(|c| c.t_s).collect();
        assert_eq!(when, [9.0, 19.0, 49.0, 59.0]);
        assert_eq!(
            scored.unfounded_cuts[0],
            Cut {
                way: "up",
                t_s: 9.0,
                from: Some(10_000),
                to: 9_000,
                delay_ms: Some(0.0)
            }
        );
    }

    #[test]
    fn a_limit_set_where_there_was_none_is_a_cut() {
        struct Late(u64);
        impl Controller for Late {
            fn second(&mut self, _: &Second) -> Rates {
                self.0 += 1;
                Rates {
                    up: None,
                    down: (self.0 > 5).then_some(40_000),
                }
            }
        }
        let stays = read_lines(&connection(NOV3, 10, constant(0, 0), quiet));
        let scored = score(&stays[0], &mut Late(0), Some(BASE));
        assert_eq!((scored.down.cuts, scored.down.unfounded), (1, 1));
    }

    #[test]
    fn a_rate_held_low_on_an_idle_connection_counts_after_a_minute() {
        // The rate holds from the second after it's first answered.
        let stays = read_lines(&connection(NOV3, 150, constant(0, 0), quiet));
        let scored = score(&stays[0], &mut fixed(4_000), Some(BASE));
        assert_eq!(
            (scored.up.held_idle_s, scored.up.longest_held_idle_s),
            (149, 149)
        );
        assert_eq!(scored.down.held_idle_s, 0, "no limit is never held low");

        // Idle for 40 s, busy for 30 and idle for 40 more: no spell lasts.
        let busy = |at: u64| {
            let from = NOV3 + 40_000;
            (0, (at.clamp(from, from + 30_000) - from) * 8_000 / 8)
        };
        let stays = read_lines(&connection(NOV3, 110, busy, quiet));
        let scored = score(&stays[0], &mut fixed(4_000), Some(BASE));
        assert_eq!(scored.up.held_idle_s, 0);

        // Nor across a machine asleep.
        let mut lines = connection(NOV3, 50, constant(0, 0), quiet);
        lines.extend(connection(NOV3 + 3_600_000, 50, constant(0, 0), quiet));
        let scored = score(&read_lines(&lines)[0], &mut fixed(4_000), Some(BASE));
        assert_eq!(scored.up.held_idle_s, 0);
    }

    #[test]
    fn a_rate_under_half_the_base_the_connection_carried_past_is_starved() {
        let stays = read_lines(&connection(NOV3, 60, constant(5_000, 0), quiet));
        let scored = score(&stays[0], &mut fixed(4_000), Some(BASE));
        assert_eq!(scored.up.starved_s, 59);
        assert_eq!(scored.up.held_idle_s, 0);
        let scored = score(&stays[0], &mut fixed(6_000), Some(BASE));
        assert_eq!(scored.up.starved_s, 0, "over half the base");

        // As the replay of 3 to 7 November found, a rate the connection
        // never reached held nothing back, recorded unshaped.
        let stays = read_lines(&connection(NOV3, 60, constant(4_000, 0), quiet));
        let scored = score(&stays[0], &mut fixed(4_000), Some(BASE));
        assert_eq!(scored.up.starved_s, 0, "carried no more than the rate");

        // Nor past a queue of its own, which holding it would drain.
        let queue = |t: u64, _| if t >= 30 { (30.0, 0.0) } else { (0.0, 0.0) };
        let stays = read_lines(&connection(NOV3, 60, constant(5_000, 0), queue));
        let scored = score(&stays[0], &mut fixed(4_000), Some(BASE));
        assert_eq!(scored.up.starved_s, 29, "only before the queue");
    }

    #[test]
    fn the_base_is_the_most_carried_unless_given() {
        let stays = read_lines(&connection(NOV3, 60, constant(8_000, 40_000), quiet));
        assert_eq!(
            score(&stays[0], &mut Unlimited, None).base,
            Base {
                up: 8_000,
                down: 40_000
            }
        );
        assert_eq!(score(&stays[0], &mut Unlimited, Some(BASE)).base, BASE);

        // Two seconds of a burst from a buffer don't stand for the
        // connection.
        let burst = |at: u64| {
            let (received, sent) = constant(8_000, 40_000)(at);
            let from = NOV3 + 30_000;
            (
                received,
                sent + (at.clamp(from, from + 2_000) - from) * 12_000 / 8,
            )
        };
        let stays = read_lines(&connection(NOV3, 60, burst, quiet));
        assert!(stays[0].seconds.iter().any(|s| s.up_kbit > 19_000.0));
        assert_eq!(score(&stays[0], &mut Unlimited, None).base.up, 8_000);
        let short = read_lines(&connection(NOV3, 3, constant(8_000, 40_000), quiet));
        assert_eq!(score(&short[0], &mut Unlimited, None).base.up, 8_000);
    }

    #[test]
    fn the_whole_adds_up_its_stays() {
        let mut lines = vec![pool(NOV3, "home")];
        lines.extend(connection(NOV3, 150, constant(0, 0), quiet));
        lines.push(pool(NOV3 + 150_000, "cafe"));
        lines.extend(connection(NOV3 + 150_000, 100, constant(0, 0), quiet));
        let report = replay(&read_lines(&lines), Some(BASE), || fixed(4_000));
        assert_eq!(
            (report.up.held_idle_s, report.up.longest_held_idle_s),
            (149 + 99, 149)
        );
        let text = summary(&report);
        for words in [
            "2 stays",
            "home",
            "cafe",
            "held low",
            "without cause",
            "4 min 8 s",
        ] {
            assert!(text.contains(words), "{words} in:\n{text}");
        }
    }

    #[test]
    fn the_command_takes_a_base_and_files() {
        let args = |words: &str| words.split(' ').map(String::from).collect::<Vec<_>>();
        assert_eq!(
            asked(&args("--base 10000,50000 --json a.jsonl b.jsonl")),
            Ok(Asked {
                base: Some(BASE),
                controller: Choice::OneWay,
                json: true,
                files: vec!["a.jsonl".into(), "b.jsonl".into()],
            })
        );
        assert_eq!(
            asked(&args("a.jsonl")).map(|a| (a.base, a.controller, a.json)),
            Ok((None, Choice::OneWay, false))
        );
        assert_eq!(
            asked(&args("--controller none a.jsonl")).map(|a| a.controller),
            Ok(Choice::Unlimited)
        );
        assert!(asked(&args("--controller cake a.jsonl")).is_err());
        assert!(asked(&args("--base 10000 a.jsonl")).is_err());
        assert!(asked(&args("--json")).is_err(), "no files");
    }
}
