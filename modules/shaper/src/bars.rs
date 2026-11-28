//! The bars a rate is held to against the line a network is known to
//! carry, its anchor, scored over a replay's seconds: whether the upload
//! was held far under the anchor with no cut of its own to show for it,
//! and what share of the anchor it held while traffic wanted it.
//!
//! The replay's own score (src/replay.rs) holds a rate against the most a
//! stay carried, which a burst to the home network can put at many times
//! the line, so a rate held at a third of the line all evening passes it.
//! An anchor is the line's own rate, given by whoever knows it, keyed by
//! the network's name as the recorder names it.
//!
//! A replay can't show whether a cut drained the delay it answered, since
//! the day carried what the rates of the day let through. So a cut counts
//! as earned when the upload's own delay was over `CAUSE_MS` as it was
//! made: the nearest to proof a record allows, and said as such.
//!
//! A trial (src/live.rs) may hold the upload under a ceiling of its own,
//! as the ceiling arms at 01:12 on 17 November held it at 9.5 and then
//! 8.0 Mb/s, which read as two breaches of the controller's, and a
//! network's own ceiling holds it so for good. A second held at a
//! ceiling, a trial's, its network's or its house's, and a fall to it, are the
//! ceiling's: counted apart and judged no further. A cut under the
//! ceiling is still the controller's.

use crate::oneway::ANCHOR_HOLD;
use crate::rate::{float, whole};
use crate::replay::{Rates, Second, Stay};
use serde::Serialize;
use std::collections::BTreeMap;

/// A rate under this share of the anchor is held under the line.
pub const UNDER: f64 = 0.8;
/// How long, in seconds, a rate may be held under the line before it
/// breaches the bar, and how long an earned cut excuses it.
pub const GRACE_S: f64 = 60.0;
/// The upload's own delay over this, in ms, earns a cut.
const CAUSE_MS: f64 = 15.0;
/// A fall of more than this share in a second is a cut.
const CUT: f64 = 0.95;
/// A second carrying at least this share of its rate wanted the rate.
const BUSY: f64 = 0.5;
/// Seconds further apart than this are of a machine asleep or a recorder
/// stopped, and a spell doesn't run across them.
const JOINED_S: f64 = 1.5;

/// The line each network is known to carry up, in kbit/s, by its name.
pub type Anchors = BTreeMap<String, u64>;

/// A stretch of time to be scored apart, in seconds since 1970.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Window {
    pub name: String,
    pub from_s: u64,
    pub to_s: u64,
}

/// A spell held under the line past its grace.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Breach {
    /// When the spell began, in seconds since 1970, and how long it ran.
    pub from_s: u64,
    pub seconds: u64,
    /// The least the upload was held to in it, in kbit/s.
    pub least: u64,
}

/// How the upload did against its anchor.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Bar {
    /// The line it was judged against, in kbit/s: the anchor, or the
    /// line a network's ceiling leaves under it where a replay says so.
    pub anchor: u64,
    /// Seconds judged: held to a rate or not.
    pub seconds: u64,
    /// Seconds in spells under `UNDER` of the anchor that outlasted
    /// `GRACE_S` with no earned cut in the grace before, and the spells.
    pub breached_s: u64,
    pub longest_breach_s: u64,
    pub breaches: Vec<Breach>,
    /// Every cut, and those made with the upload's own delay over
    /// `CAUSE_MS`.
    pub cuts: u64,
    pub earned: u64,
    /// Seconds the upload carried at least half its rate, and the median
    /// share of the anchor it was held to in them; a second with no limit
    /// holds the whole anchor.
    pub busy_s: u64,
    pub busy_hold: Option<f64>,
    /// Seconds the upload was held at a ceiling, a trial's, its network's
    /// or its house's: the ceiling's doing, so none of the rest counts them.
    pub ceiling_s: u64,
}

/// The bar over every stay with an anchor, and over each window.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Judged {
    /// By network, every stay's seconds together.
    pub networks: BTreeMap<String, Bar>,
    /// By window, the seconds in it of every network with an anchor.
    pub windows: BTreeMap<String, Bar>,
}

/// Judges each stay's rates, `rates[i]` answered at the stay's second
/// `i`, against its network's anchor, and apart within each window.
/// Stays on a network with no anchor aren't judged.
pub fn judge(stays: &[(&Stay, &[Rates])], anchors: &Anchors, windows: &[Window]) -> Judged {
    let mut judged = Judged::default();
    for (stay, rates) in stays {
        let Some(&anchor) = anchors.get(&stay.network) else {
            continue;
        };
        let whole = judged
            .networks
            .entry(stay.network.clone())
            .or_insert_with(|| Bar::new(anchor));
        whole.add(&Spells::over(stay, rates, anchor, None).finish());
        for w in windows {
            let bar = judged
                .windows
                .entry(w.name.clone())
                .or_insert_with(|| Bar::new(anchor));
            bar.add(&Spells::over(stay, rates, anchor, Some(w)).finish());
        }
    }
    judged
}

impl Bar {
    fn new(anchor: u64) -> Bar {
        Bar {
            anchor,
            ..Bar::default()
        }
    }

    /// Takes in another's seconds; the median of two medians is no
    /// median, so the busy hold is weighted by each one's busy seconds,
    /// which reads the same as long as the two are alike.
    fn add(&mut self, other: &Bar) {
        self.busy_hold = match (self.busy_hold, other.busy_hold) {
            (Some(a), Some(b)) => {
                let (n, m) = (float(self.busy_s), float(other.busy_s));
                Some((a * n + b * m) / (n + m))
            }
            (a, b) => a.or(b),
        };
        self.seconds += other.seconds;
        self.breached_s += other.breached_s;
        self.longest_breach_s = self.longest_breach_s.max(other.longest_breach_s);
        self.breaches.extend(other.breaches.iter().cloned());
        self.cuts += other.cuts;
        self.earned += other.earned;
        self.busy_s += other.busy_s;
        self.ceiling_s += other.ceiling_s;
    }
}

/// One stay's seconds as they go by.
struct Spells {
    bar: Bar,
    /// The spell under the line: when it began, how long it has run past
    /// any excuse, and the least rate in it.
    spell: Option<(u64, u64, u64)>,
    /// When the latest earned cut was made, in seconds since 1970.
    earned_at: Option<f64>,
    holds: Vec<f64>,
}

impl Spells {
    /// Walks a stay's seconds, each judged by the rate its predecessor
    /// answered, as the replay's score judges them, within `window` only
    /// when one is given.
    fn over(stay: &Stay, rates: &[Rates], anchor: u64, window: Option<&Window>) -> Spells {
        let mut spells = Spells {
            bar: Bar::new(anchor),
            spell: None,
            earned_at: None,
            holds: Vec::new(),
        };
        let began = float(stay.began_ms) / 1_000.0;
        let mut before: Option<(&Second, Option<u64>)> = None;
        for (s, answered) in stay.seconds.iter().zip(rates) {
            let at = began + s.t_s;
            let inside = window.is_none_or(|w| at >= float(w.from_s) && at < float(w.to_s));
            match before {
                Some((last, holding)) if inside => {
                    if s.t_s - last.t_s >= JOINED_S {
                        spells.end();
                    }
                    // The rate a second is held to was answered at the
                    // second before, under that second's trial.
                    if at_the_ceiling(holding, last.ceiling) {
                        spells.bar.ceiling_s += 1;
                        spells.end();
                    } else {
                        spells.second(at, s, holding, anchor);
                    }
                    if !at_the_ceiling(answered.up, s.ceiling) {
                        spells.cut(at, (holding, answered.up), s.up_delay_ms, anchor);
                    }
                }
                _ => spells.end(),
            }
            before = Some((s, answered.up));
        }
        spells
    }

    fn second(&mut self, at: f64, s: &Second, holding: Option<u64>, anchor: u64) {
        self.bar.seconds += 1;
        let held = holding.map_or(float(anchor), float);
        if s.up_kbit >= BUSY * held {
            self.bar.busy_s += 1;
            self.holds.push(held / float(anchor));
        }
        if held >= UNDER * float(anchor) {
            return self.end();
        }
        let excused = self.earned_at.is_some_and(|e| at - e <= GRACE_S);
        let least = holding.unwrap_or(anchor);
        let (from, run, low) = self.spell.get_or_insert((whole(at), 0, least));
        if excused {
            *from = whole(at);
            *run = 0;
        } else {
            *run += 1;
        }
        *low = (*low).min(least);
    }

    /// Counts a fall from `before` to `after` as a cut, unless it lands at
    /// or over the anchor's hold: a trial over the line falling back to
    /// the hold holds nothing back.
    fn cut(
        &mut self,
        at: f64,
        rates: (Option<u64>, Option<u64>),
        delay_ms: Option<f64>,
        anchor: u64,
    ) {
        let (before, after) = rates;
        let Some(to) = after else { return };
        if before.is_some_and(|from| float(to) >= CUT * float(from))
            || float(to) >= ANCHOR_HOLD * float(anchor)
        {
            return;
        }
        self.bar.cuts += 1;
        if delay_ms.is_some_and(|d| d > CAUSE_MS) {
            self.bar.earned += 1;
            self.earned_at = Some(at);
        }
    }

    fn end(&mut self) {
        if let Some((from_s, run, least)) = self.spell.take()
            && float(run) > GRACE_S
        {
            self.bar.breached_s += run;
            self.bar.longest_breach_s = self.bar.longest_breach_s.max(run);
            self.bar.breaches.push(Breach {
                from_s,
                seconds: run,
                least,
            });
        }
    }

    fn finish(mut self) -> Bar {
        self.end();
        self.holds.sort_by(f64::total_cmp);
        self.bar.busy_hold = self.holds.get(self.holds.len() / 2).copied();
        self.bar
    }
}

/// Whether `rate` is held at a `ceiling`, or over it, which a ceiling
/// never lets: either way the ceiling holds it there.
fn at_the_ceiling(rate: Option<u64>, ceiling: Option<u64>) -> bool {
    ceiling.is_some_and(|most| rate.is_none_or(|rate| rate >= most))
}

/// A moment written `2026-11-16T00:30Z`, in UTC, as seconds since 1970.
pub fn moment(text: &str) -> Option<u64> {
    let text = text.strip_suffix('Z')?;
    let (date, time) = text.split_once('T')?;
    let mut ymd = date.splitn(3, '-').map(str::parse::<i64>);
    let (y, m, d) = (ymd.next()?.ok()?, ymd.next()?.ok()?, ymd.next()?.ok()?);
    let mut hm = time.splitn(2, ':').map(str::parse::<u64>);
    let (h, min) = (hm.next()?.ok()?, hm.next()?.ok()?);
    if h > 23 || min > 59 {
        return None;
    }
    // A date that doesn't come back the same, 30 February say, is none.
    let days = days_from_civil(y, m, d);
    let (cy, cm, cd) = crate::record::civil(days);
    if (cy, i64::from(cm), i64::from(cd)) != (y, m, d) {
        return None;
    }
    let days = u64::try_from(days).ok()?;
    Some(days * 86_400 + h * 3_600 + min * 60)
}

/// The count of days since 1970 a date falls on (Howard Hinnant's
/// days-from-civil, the inverse of src/record.rs's `civil`).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Windows written one a line, a name then its start and end as
/// `moment` reads them, split by whitespace; a line starting `#` is a
/// note. A line that can't be read is named, so a slip isn't scored as
/// an empty window.
pub fn windows(text: &str) -> Result<Vec<Window>, String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            let [name, from, to] = words.as_slice() else {
                return Err(format!("a window is NAME FROM TO, not {line:?}"));
            };
            let (Some(from_s), Some(to_s)) = (moment(from), moment(to)) else {
                return Err(format!(
                    "a window's times are as 2026-11-16T00:30Z: {line:?}"
                ));
            };
            if to_s <= from_s {
                return Err(format!("a window ends after it starts: {line:?}"));
            }
            Ok(Window {
                name: (*name).to_owned(),
                from_s,
                to_s,
            })
        })
        .collect()
}

/// Anchors written `NETWORK=UP`, UP in kbit/s.
pub fn anchor(text: &str) -> Result<(String, u64), String> {
    text.rsplit_once('=')
        .and_then(|(network, up)| Some((network.to_lowercase(), up.trim().parse().ok()?)))
        .filter(|(network, up)| !network.is_empty() && *up > 0)
        .ok_or_else(|| format!("--anchor takes NETWORK=UP in kbit/s, not {text:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Midnight in UTC on 16 November 2026, in seconds.
    const NOV16: u64 = 1_794_787_200;
    const HOME: &str = "02:00:00:00:aa:00";

    /// A stay of `n` seconds from `NOV16` carrying `up` kbit/s with the
    /// upload's own delay at `delay(t)`.
    fn stay(n: usize, up: f64, delay: impl Fn(usize) -> f64) -> Stay {
        Stay {
            network: HOME.into(),
            began_ms: NOV16 * 1_000,
            seconds: (0..n)
                .map(|t| Second {
                    t_s: float(t as u64),
                    up_kbit: up,
                    down_kbit: 0.0,
                    up_delay_ms: Some(delay(t)),
                    down_delay_ms: None,
                    gateway_ms: None,
                    gateway_most_ms: None,
                    up_ours: None,
                    vpn_ms: None,
                    ceiling: None,
                })
                .collect(),
        }
    }

    /// `s` with a trial's ceiling of `most` kbit/s over its `seconds`.
    fn under_a_trial(mut s: Stay, most: u64, seconds: std::ops::Range<usize>) -> Stay {
        for second in &mut s.seconds[seconds] {
            second.ceiling = Some(most);
        }
        s
    }

    fn held(rates: impl Fn(usize) -> Option<u64>, n: usize) -> Vec<Rates> {
        (0..n)
            .map(|t| Rates {
                up: rates(t),
                down: None,
            })
            .collect()
    }

    fn home(anchor: u64) -> Anchors {
        Anchors::from([(HOME.to_owned(), anchor)])
    }

    fn at_home(stay: &Stay, rates: &[Rates], anchor: u64) -> Bar {
        judge(&[(stay, rates)], &home(anchor), &[])
            .networks
            .remove(HOME)
            .expect("the home network is judged")
    }

    #[test]
    fn a_rate_held_far_under_the_line_with_no_cause_breaches() {
        let s = stay(600, 4_500.0, |_| 2.0);
        let rates = held(|t| Some(if t < 10 { 10_800 } else { 4_500 }), 600);
        let bar = at_home(&s, &rates, 12_000);
        assert_eq!(bar.cuts, 1, "one cut, from 10.8 to 4.5: {bar:?}");
        assert_eq!(bar.earned, 0, "the delay never rose: {bar:?}");
        assert_eq!(bar.breaches.len(), 1, "one spell: {bar:?}");
        assert_eq!(bar.breached_s, 589, "every second held under: {bar:?}");
        assert_eq!(bar.breaches[0].least, 4_500, "the least it held: {bar:?}");
        assert_eq!(bar.breaches[0].from_s, NOV16 + 11, "when it began: {bar:?}");
        let hold = bar.busy_hold.expect("it was busy");
        assert!((hold - 0.375).abs() < 1e-9, "held at 4.5 of 12: {hold}");
    }

    #[test]
    fn a_trial_falling_back_to_the_hold_is_no_cut() {
        let s = stay(200, 9_000.0, |_| 2.0);
        let rates = held(
            |t| match t {
                20..40 => Some(15_300),
                0..100 => Some(10_800),
                _ => Some(9_500),
            },
            200,
        );
        let bar = at_home(&s, &rates, 12_000);
        assert_eq!(
            bar.cuts, 1,
            "15.3 back to the 10.8 hold holds nothing back; 10.8 to 9.5 does: {bar:?}"
        );
    }

    #[test]
    fn a_cut_made_on_the_lines_own_delay_excuses_its_minute() {
        let s = stay(100, 5_000.0, |t| if t == 10 { 40.0 } else { 2.0 });
        let rates = held(|t| Some(if t < 10 { 10_800 } else { 5_000 }), 100);
        let bar = at_home(&s, &rates, 12_000);
        assert_eq!(bar.earned, 1, "the cut met 40 ms: {bar:?}");
        assert_eq!(
            bar.breached_s, 0,
            "excused for its minute, then 29 s: {bar:?}"
        );
        let s = stay(200, 5_000.0, |t| if t == 10 { 40.0 } else { 2.0 });
        let rates = held(|t| Some(if t < 10 { 10_800 } else { 5_000 }), 200);
        let bar = at_home(&s, &rates, 12_000);
        assert_eq!(bar.breaches.len(), 1, "once its minute is out: {bar:?}");
        assert_eq!(
            bar.breaches[0].from_s,
            NOV16 + 70,
            "from when the excuse ran out: {bar:?}"
        );
    }

    /// At 01:12 on 17 November a trial held the upload at 9.5 Mb/s for
    /// 240 s, then at 8.0, under a plateau of 11.4: the trial's arms, not
    /// the controller's doing, which read as two breaches.
    #[test]
    fn a_trial_s_ceiling_is_set_aside_rather_than_breached() {
        let s = under_a_trial(stay(400, 7_000.0, |_| 2.0), 8_000, 10..300);
        let rates = held(
            |t| {
                Some(if (10..300).contains(&t) {
                    8_000
                } else {
                    10_800
                })
            },
            400,
        );
        let bar = at_home(&s, &rates, 12_000);
        assert_eq!(
            bar.ceiling_s, 290,
            "each second at the trial's ceiling: {bar:?}"
        );
        assert_eq!(bar.breached_s, 0, "none is the controller's: {bar:?}");
        assert_eq!(bar.cuts, 0, "a fall to the ceiling is the trial's: {bar:?}");
        assert_eq!(bar.seconds, 109, "only the controller's seconds: {bar:?}");
        assert_eq!(bar.busy_hold, Some(0.9), "its 10.8 of 12: {bar:?}");
    }

    #[test]
    fn a_trial_is_read_under_the_ceiling_its_rate_was_answered_in() {
        // A rate answered in one second holds through the next, so the
        // ceiling it's judged by is that of the second it was answered in:
        // here a second's alone, at 0 with no limit and at 5 over it.
        let mut s = under_a_trial(stay(10, 7_000.0, |_| 2.0), 8_000, 0..1);
        s.seconds[5].ceiling = Some(8_000);
        let rates = held(
            |t| match t {
                0 => None,
                5 => Some(9_000),
                _ => Some(10_800),
            },
            10,
        );
        let bar = at_home(&s, &rates, 12_000);
        assert_eq!(
            bar.ceiling_s, 2,
            "seconds 1 and 6 were the trial's: {bar:?}"
        );
        assert_eq!(bar.seconds, 7, "the rest the controller's: {bar:?}");
    }

    #[test]
    fn a_trial_ends_a_spell_rather_than_bridging_two() {
        // Held low with no cause for 50 s either side of a trial: neither
        // stretch is past the grace alone, and the trial joins neither.
        let s = under_a_trial(stay(111, 4_500.0, |_| 2.0), 8_000, 50..60);
        let rates = held(
            |t| Some(if (50..60).contains(&t) { 8_000 } else { 4_500 }),
            111,
        );
        let bar = at_home(&s, &rates, 12_000);
        assert_eq!(bar.ceiling_s, 10, "the trial's seconds: {bar:?}");
        assert_eq!(bar.breached_s, 0, "two short spells, not one: {bar:?}");
    }

    #[test]
    fn a_cut_under_a_trial_s_ceiling_is_still_the_controller_s() {
        let s = under_a_trial(stay(400, 4_500.0, |_| 2.0), 8_000, 0..400);
        let rates = held(|t| Some(if t < 10 { 8_000 } else { 4_500 }), 400);
        let bar = at_home(&s, &rates, 12_000);
        assert_eq!(bar.cuts, 1, "8.0 to 4.5, under the ceiling: {bar:?}");
        assert_eq!(
            bar.ceiling_s, 10,
            "held at the ceiling until the cut: {bar:?}"
        );
        assert_eq!(
            bar.breaches.len(),
            1,
            "held under it with no cause: {bar:?}"
        );
        assert_eq!(bar.breached_s, 389, "from the cut on: {bar:?}");
    }

    #[test]
    fn no_limit_holds_the_whole_line_and_a_gap_ends_a_spell() {
        let s = stay(300, 9_000.0, |_| 2.0);
        let bar = at_home(&s, &held(|_| None, 300), 12_000);
        assert_eq!(bar.breached_s, 0, "nothing held: {bar:?}");
        assert_eq!(bar.busy_hold, Some(1.0), "the whole anchor: {bar:?}");
        let mut s = stay(100, 4_000.0, |_| 2.0);
        for (i, second) in s.seconds.iter_mut().enumerate().skip(50) {
            second.t_s = float(i as u64) + 600.0;
        }
        let bar = at_home(&s, &held(|_| Some(4_000), 100), 12_000);
        assert_eq!(
            bar.breached_s, 0,
            "two spells of 50 s, the sleep between: {bar:?}"
        );
    }

    #[test]
    fn a_window_scores_only_its_own_seconds_and_other_networks_go_unjudged() {
        let s = stay(600, 4_500.0, |_| 2.0);
        let rates = held(|_| Some(4_500), 600);
        let w = Window {
            name: "evening".into(),
            from_s: NOV16 + 100,
            to_s: NOV16 + 200,
        };
        let judged = judge(&[(&s, &rates)], &home(12_000), std::slice::from_ref(&w));
        let bar = &judged.windows["evening"];
        assert_eq!(bar.seconds, 100, "the window's seconds alone: {bar:?}");
        assert_eq!(bar.breached_s, 100, "from its first second: {bar:?}");
        let mut away = stay(600, 4_500.0, |_| 2.0);
        away.network = "link wlp0s20f3".into();
        let judged = judge(&[(&away, &rates)], &home(12_000), &[w]);
        assert!(judged.networks.is_empty(), "no anchor, no bar: {judged:?}");
    }

    #[test]
    fn windows_and_anchors_are_read_as_written_or_refused() {
        assert_eq!(moment("2026-11-16T00:00Z"), Some(NOV16), "midnight");
        assert_eq!(
            moment("2026-11-16T00:30Z"),
            Some(NOV16 + 1_800),
            "half past"
        );
        let leap = moment("2028-02-29T00:00Z").expect("a leap day") / 86_400;
        assert_eq!(
            crate::record::civil(i64::try_from(leap).expect("a day count")),
            (2028, 2, 29),
            "the inverse of the recorder's dates"
        );
        assert_eq!(moment("2026-11-16 00:30"), None, "no T or Z");
        assert_eq!(moment("2026-13-01T00:00Z"), None, "no thirteenth month");
        assert_eq!(moment("2026-02-29T00:00Z"), None, "2026 is no leap year");
        let read = windows("# bites\nevening 2026-11-16T00:30Z 2026-11-16T05:00Z\n\n");
        assert_eq!(
            read,
            Ok(vec![Window {
                name: "evening".into(),
                from_s: NOV16 + 1_800,
                to_s: NOV16 + 18_000
            }]),
            "one window, the note passed over"
        );
        assert!(
            windows("evening 2026-11-16T05:00Z 2026-11-16T00:30Z").is_err(),
            "backwards"
        );
        assert!(windows("evening 2026-11-16T05:00Z").is_err(), "no end");
        assert_eq!(
            anchor("02:00:00:00:AA:00=12000"),
            Ok((HOME.to_owned(), 12_000)),
            "named by its router, as the recorder names it"
        );
        assert!(anchor("12000").is_err(), "no network");
        assert!(anchor("home=0").is_err(), "no rate");
    }
}
