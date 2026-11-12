//! What an opening of the launcher took from Joel, in the seconds he lost to
//! it, and which of a day's interactions slowed him most.
//!
//! Each kind of loss comes from how product research tells a struggle
//! from a visit that went well: the wait past what feels immediate (about
//! 400 ms to usable, the Doherty threshold; 100 ms for a frame, where a
//! response stops feeling instant), the time to his pick past what that
//! pick usually takes him where he searched for it by typing (time on
//! task against his own baseline; time spent reading isn't), the
//! whole of an opening he typed in and left with nothing (an abandoned
//! search), and an opening he came straight back to (a u-turn: the first
//! didn't get him there).

use crate::trial::Trial;
use serde::Serialize;
use std::collections::HashMap;

/// Usable this long after the chord feels immediate.
pub const USABLE_MS: f64 = 400.0;
/// A gap before a frame longer than this is a stall he sees.
pub const STALL_MS: f64 = 100.0;
/// A key whose frame came later than this lags, the launcher's own bar.
pub const FELT_MS: f64 = 50.0;
/// An opening this soon after one that failed him is a second go at it.
pub const AGAIN_MS: i64 = 30_000;
/// The most an abandoned opening is counted for, since one left open
/// while he looked away would otherwise outweigh every real struggle.
pub const GAVE_UP_MOST_MS: f64 = 120_000.0;
/// What a pick is expected to take before he has made it often enough to
/// have his own: about what the keystroke-level model gives a chord, a
/// few letters, a look and Return.
pub const SPEC_MS: f64 = 3_000.0;
/// How many of one kind of pick make his own usual time for it.
pub const USUAL_LEAST: usize = 3;
/// An interaction that lost him less than this isn't named.
pub const PAINFUL_MS: f64 = 1_000.0;
/// How many of a day's interactions the day file keeps.
pub const KEPT: usize = 10;

/// How long each kind of pick usually takes Joel: the median of his
/// picks of that kind over the trials it's given.
pub struct Usual {
    by_kind: HashMap<String, Vec<i64>>,
}

impl Usual {
    pub fn of(trials: &[Trial]) -> Usual {
        let mut by_kind: HashMap<String, Vec<i64>> = HashMap::new();
        for trial in trials.iter().filter(|t| t.picked_one() && t.ms >= 0) {
            by_kind.entry(trial.trying()).or_default().push(trial.ms);
        }
        for times in by_kind.values_mut() {
            times.sort_unstable();
        }
        Usual { by_kind }
    }

    /// His usual time for this kind of pick, in milliseconds, or the spec
    /// where he hasn't made it often enough.
    pub fn ms(&self, kind: &str) -> f64 {
        match self.by_kind.get(kind) {
            Some(times) if times.len() >= USUAL_LEAST => float(times[times.len() / 2]),
            _ => SPEC_MS,
        }
    }
}

/// When each opening became usable, from the launcher's journal: when the
/// line was logged, in milliseconds since the epoch, and how long after
/// the chord.
pub struct Usable(Vec<(i64, f64)>);

impl Usable {
    pub fn new(lines: Vec<(i64, f64)>) -> Usable {
        Usable(lines)
    }

    /// How long the opening at `at` took to be usable, where a line for it
    /// was logged.
    pub fn near(&self, at: i64) -> Option<f64> {
        // A line is logged once the opening is usable, so it follows the
        // opening by about its own time; a little either side allows for
        // when each is stamped.
        self.0
            .iter()
            .find(|&&(logged, ms)| logged >= at - 500 && float(logged) <= float(at) + ms + 3000.0)
            .map(|&(_, ms)| ms)
    }
}

/// How long after the chord an opening was usable, from its journal line
/// ("launcher: opening usable 422 ms from the chord, ...").
pub fn usable_of(message: &str) -> Option<f64> {
    let (_, rest) = message.split_once("opening usable ")?;
    let (ms, _) = rest.split_once(" ms from the chord")?;
    ms.trim().parse().ok()
}

/// What one opening lost him, in milliseconds, by where it went.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Lost {
    /// Waiting for the launcher to be usable.
    pub opening: f64,
    /// Waiting on a key's search.
    pub search: f64,
    /// Waiting on frames: a key's drawing or a stall.
    pub drawing: f64,
    /// Finding his pick past his usual time for it.
    pub hunting: f64,
    /// An opening he typed in and left with nothing.
    pub gave_up: f64,
    /// A pick he made again at once, since the first didn't do it.
    pub again: f64,
}

impl Lost {
    pub fn waiting(&self) -> f64 {
        self.opening + self.search + self.drawing
    }

    pub fn total(&self) -> f64 {
        self.waiting() + self.hunting + self.gave_up + self.again
    }
}

/// What one opening lost him.
pub fn lost(trial: &Trial, usable: Option<f64>, usual: &Usual) -> Lost {
    let mut it = Lost {
        opening: usable.map_or(0.0, |ms| (ms - USABLE_MS).max(0.0)),
        ..Lost::default()
    };
    for key in trial.timed.iter().flat_map(|t| &t.felt) {
        let over = (key.drawn - FELT_MS).max(0.0);
        let searching = if key.drawn > 0.0 {
            (key.searched / key.drawn).clamp(0.0, 1.0)
        } else {
            0.0
        };
        it.search += over * searching;
        it.drawing += over * (1.0 - searching);
    }
    if let Some(frames) = trial.frames.as_ref().filter(|f| f.late > 0) {
        it.drawing += (frames.longest - STALL_MS).max(0.0);
    }
    // Time to a pick he searched for by typing is finding it; time to one
    // he arrowed his way to is reading what each row says, as a long
    // dwell on a page is, and isn't counted against him.
    if trial.picked_one() && trial.keys.letters > 0 {
        let past = float(trial.ms) - usual.ms(&trial.trying()) - it.waiting();
        it.hunting = past.max(0.0);
    } else if trial.missed {
        it.gave_up = float(trial.open.max(0)).min(GAVE_UP_MOST_MS);
    }
    it
}

/// Whether `next` was a second go at what `was` failed to do: it came
/// within `AGAIN_MS` of `was` closing, and `was` was a search he left
/// with nothing or the very row he picks again, which didn't do what he
/// wanted. A different row of the same kind is working down a list, not
/// a second go. The rows are compared here and never kept.
fn again(was: &Trial, next: &Trial) -> bool {
    next.at - was.closed() <= AGAIN_MS
        && (was.missed || (was.picked_one() && was.picked == next.picked))
}

/// What an interaction showed, by kind and timing alone.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Signs {
    pub usable_ms: u32,
    pub stall_ms: u32,
    pub slow_keys: u32,
    pub letters: u32,
    pub erased: u32,
    pub arrows: u32,
    pub backs: u32,
}

/// One interaction that slowed him, as the day file says it: when it began
/// (seconds since the epoch), what he was doing as a kind, how long it
/// took against his usual, what it lost him, and what slowed it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Painful {
    pub at: i64,
    pub place: &'static str,
    pub trying: String,
    pub seconds: f64,
    pub usual_seconds: f64,
    pub lost_seconds: f64,
    pub rank: usize,
    pub slowed: &'static str,
    pub part: &'static str,
    pub openings: usize,
    pub signs: Signs,
}

/// The day's interactions that lost him at least a second, worst first
/// and at most `KEPT`: each an opening, or openings in a row where each
/// came straight back to one that failed him.
pub fn painful(day: &[Trial], usable: &Usable, usual: &Usual) -> Vec<Painful> {
    let mut day = day.to_vec();
    day.sort_by_key(|t| t.at);
    let mut goes: Vec<Vec<Trial>> = Vec::new();
    for trial in day {
        match goes.last_mut() {
            Some(go) if go.last().is_some_and(|was| again(was, &trial)) => go.push(trial),
            _ => goes.push(vec![trial]),
        }
    }
    let mut named: Vec<Painful> = goes
        .iter()
        .map(|go| interaction(go, usable, usual))
        .filter(|p| p.lost_seconds * 1000.0 >= PAINFUL_MS)
        .collect();
    named.sort_by(|a, b| {
        b.lost_seconds
            .total_cmp(&a.lost_seconds)
            .then(a.at.cmp(&b.at))
    });
    named.truncate(KEPT);
    for (n, p) in named.iter_mut().enumerate() {
        p.rank = n + 1;
    }
    named
}

/// One go at something, an opening or openings each straight after one
/// that failed him, as the day file says it.
fn interaction(go: &[Trial], usable: &Usable, usual: &Usual) -> Painful {
    let first = &go[0];
    let last = &go[go.len() - 1];
    let mut sum = Lost::default();
    let mut signs = Signs::default();
    for (n, trial) in go.iter().enumerate() {
        let ready = usable.near(trial.at);
        let it = lost(trial, ready, usual);
        sum.opening += it.opening;
        sum.search += it.search;
        sum.drawing += it.drawing;
        sum.hunting += it.hunting;
        sum.gave_up += it.gave_up;
        // A pick made again at once didn't do it the first time, so the
        // first's whole time went for nothing.
        if n + 1 < go.len() && trial.picked_one() {
            sum.again += float(trial.ms.max(0));
        }
        signs.usable_ms = signs.usable_ms.max(whole(ready.unwrap_or(0.0)));
        if let Some(frames) = trial.frames.as_ref().filter(|f| f.late > 0) {
            signs.stall_ms = signs.stall_ms.max(whole(frames.longest));
        }
        let slow = trial
            .timed
            .iter()
            .flat_map(|t| &t.felt)
            .filter(|k| k.drawn > FELT_MS)
            .count();
        signs.slow_keys += u32::try_from(slow).unwrap_or(u32::MAX);
        signs.letters += trial.keys.letters;
        signs.erased += trial.keys.erased;
        signs.arrows += trial.keys.arrows;
        signs.backs += trial.keys.backs;
    }
    let trying = go
        .iter()
        .rev()
        .map(Trial::trying)
        .find(|kind| kind != "glanced")
        .unwrap_or_else(|| "glanced".to_string());
    let finding = sum.hunting + sum.gave_up + sum.again;
    let slowed = if go.len() > 1 {
        "tried again"
    } else if sum.waiting() >= sum.hunting.max(sum.gave_up) {
        "waiting"
    } else if sum.hunting >= sum.gave_up {
        "hunting"
    } else {
        "gave up"
    };
    let part = [
        (sum.opening, "opening"),
        (sum.search, "search"),
        (sum.drawing, "drawing"),
        (finding, "finding"),
    ]
    .into_iter()
    .fold(
        (f64::MIN, "finding"),
        |best, it| if it.0 > best.0 { it } else { best },
    )
    .1;
    Painful {
        at: first.at / 1000,
        place: "launcher",
        usual_seconds: tenths(usual.ms(&trying)),
        trying,
        seconds: tenths(float(last.closed() - first.at)),
        lost_seconds: tenths(sum.total()),
        rank: 0,
        slowed,
        part,
        openings: go.len(),
        signs,
    }
}

/// Milliseconds as seconds, to a tenth.
fn tenths(ms: f64) -> f64 {
    (ms / 100.0).round() / 10.0
}

/// Milliseconds as a whole number of them.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "clamped to what a u32 holds first"
)]
fn whole(ms: f64) -> u32 {
    ms.round().clamp(0.0, f64::from(u32::MAX)) as u32
}

/// Milliseconds, or a time in them, as a float to reckon with.
#[expect(
    clippy::cast_precision_loss,
    reason = "epoch milliseconds fit well inside f64's 52 bits until the year 144,000"
)]
fn float(ms: i64) -> f64 {
    ms as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trial::{Felt, Frames, Keys, Timed};

    const AT: i64 = 1_794_455_000_000;

    fn pick(at: i64, picked: &str, ms: i64) -> Trial {
        Trial {
            at,
            keys: Keys {
                letters: 3,
                returns: 1,
                ..Keys::default()
            },
            picked: picked.to_string(),
            ms,
            open: ms,
            ..Trial::default()
        }
    }

    fn miss(at: i64, open: i64) -> Trial {
        Trial {
            at,
            keys: Keys {
                letters: 5,
                erased: 2,
                ..Keys::default()
            },
            open,
            missed: true,
            ..Trial::default()
        }
    }

    fn usual_of(kind: &str, ms: &[i64]) -> Usual {
        let trials: Vec<Trial> = ms
            .iter()
            .map(|&ms| pick(AT - 86_400_000, kind, ms))
            .collect();
        Usual::of(&trials)
    }

    #[test]
    fn an_opening_s_usable_time_reads_from_its_journal_line() {
        let line = "launcher: opening usable 422 ms from the chord, its longest stall 198 ms";
        assert_eq!(usable_of(line), Some(422.0));
        assert_eq!(
            usable_of("launcher: opening presented 261 ms from the chord"),
            None
        );
    }

    #[test]
    fn an_opening_takes_the_usable_line_logged_just_after_it() {
        let usable = Usable::new(vec![
            (AT - 60_000, 300.0),
            (AT + 450, 422.0),
            (AT + 90_000, 500.0),
        ]);
        assert_eq!(usable.near(AT), Some(422.0));
        assert_eq!(
            usable.near(AT + 30_000),
            None,
            "no line within its time is none"
        );
    }

    #[test]
    fn his_usual_is_the_median_of_his_picks_of_that_kind() {
        let usual = usual_of("program:slack", &[1000, 9000, 2000]);
        assert_eq!(usual.ms("program:slack"), 2000.0);
        assert_eq!(
            usual_of("program:slack", &[1000, 9000]).ms("program:slack"),
            SPEC_MS
        );
        assert_eq!(usual.ms("capture:window"), SPEC_MS);
    }

    #[test]
    fn a_quick_pick_with_nothing_slow_loses_nothing() {
        let usual = usual_of("program:slack", &[2000, 2000, 2000]);
        let quick = pick(AT, "program:slack", 1500);
        assert_eq!(lost(&quick, Some(120.0), &usual).total(), 0.0);
        assert!(
            painful(&[quick], &Usable::new(vec![]), &usual).is_empty(),
            "nothing is named"
        );
    }

    #[test]
    fn a_slow_opening_loses_what_it_took_past_usable() {
        let usual = usual_of("program:slack", &[2000, 2000, 2000]);
        let slow = pick(AT, "program:slack", 2000);
        let it = lost(&slow, Some(1400.0), &usual);
        assert_eq!(it.opening, 1000.0);
        let named = painful(&[slow], &Usable::new(vec![(AT + 1400, 1400.0)]), &usual);
        assert_eq!(named.len(), 1, "a second lost is named");
        assert_eq!((named[0].slowed, named[0].part), ("waiting", "opening"));
        assert_eq!(named[0].signs.usable_ms, 1400);
    }

    #[test]
    fn a_pick_past_his_usual_loses_the_difference_less_its_waits() {
        let usual = usual_of("program:slack", &[4000, 4000, 4000]);
        let hunted = pick(AT, "program:slack", 20_000);
        let it = lost(&hunted, Some(1400.0), &usual);
        assert_eq!(
            it.hunting, 15_000.0,
            "20 s against 4 s, less the 1 s waited"
        );
        let named = painful(&[hunted], &Usable::new(vec![(AT + 1400, 1400.0)]), &usual);
        assert_eq!((named[0].slowed, named[0].part), ("hunting", "finding"));
        assert_eq!((named[0].seconds, named[0].usual_seconds), (20.0, 4.0));
    }

    #[test]
    fn a_pick_read_his_way_to_without_typing_is_reading_not_hunting() {
        let usual = usual_of("on:look", &[4000, 4000, 4000]);
        let read = Trial {
            keys: Keys {
                arrows: 15,
                returns: 3,
                ..Keys::default()
            },
            ..pick(AT, "on:look", 58_000)
        };
        assert_eq!(lost(&read, None, &usual).hunting, 0.0);
    }

    #[test]
    fn an_abandoned_search_loses_all_its_time_up_to_two_minutes() {
        let usual = Usual::of(&[]);
        assert_eq!(lost(&miss(AT, 9000), None, &usual).gave_up, 9000.0);
        assert_eq!(
            lost(&miss(AT, 600_000), None, &usual).gave_up,
            GAVE_UP_MOST_MS
        );
        let named = painful(&[miss(AT, 9000)], &Usable::new(vec![]), &usual);
        assert_eq!(named[0].slowed, "gave up");
        assert_eq!(named[0].trying, "looked for something");
    }

    #[test]
    fn a_glance_pains_only_when_the_launcher_was_slow() {
        let usual = Usual::of(&[]);
        let glance = Trial {
            at: AT,
            open: 20_000,
            ..Trial::default()
        };
        assert_eq!(lost(&glance, Some(300.0), &usual).total(), 0.0);
        assert_eq!(lost(&glance, Some(2400.0), &usual).total(), 2000.0);
    }

    #[test]
    fn a_miss_then_a_pick_straight_after_is_one_interaction_tried_again() {
        let usual = usual_of("program:slack", &[2000, 2000, 2000]);
        let day = [
            miss(AT, 9000),
            pick(AT + 9000 + 5000, "program:slack", 2000),
        ];
        let named = painful(&day, &Usable::new(vec![]), &usual);
        assert_eq!(named.len(), 1, "both openings are one go at it");
        assert_eq!(named[0].openings, 2);
        assert_eq!(named[0].slowed, "tried again");
        assert_eq!(named[0].trying, "program:slack");
        assert_eq!(named[0].lost_seconds, 9.0);
        assert_eq!(named[0].seconds, 16.0);
    }

    #[test]
    fn the_same_pick_again_at_once_counts_the_first_as_lost() {
        let usual = usual_of("verb:agents", &[2000, 2000, 2000]);
        let day = [
            pick(AT, "verb:agents", 2000),
            pick(AT + 4000, "verb:agents", 2000),
        ];
        let named = painful(&day, &Usable::new(vec![]), &usual);
        assert_eq!(named[0].openings, 2);
        assert_eq!(named[0].slowed, "tried again");
        assert_eq!(named[0].lost_seconds, 2.0);
    }

    #[test]
    fn working_down_a_list_one_row_at_a_time_is_not_trying_again() {
        let usual = usual_of("on:look", &[2000, 2000, 2000]);
        let day = [
            pick(AT, "on:look:546a", 2000),
            pick(AT + 4000, "on:look:686d", 2000),
        ];
        let named = painful(&day, &Usable::new(vec![]), &usual);
        assert!(
            named.is_empty(),
            "two different rows, each at his pace: {named:?}"
        );
    }

    #[test]
    fn openings_apart_are_apart() {
        let usual = Usual::of(&[]);
        let day = [miss(AT, 9000), miss(AT + 9000 + AGAIN_MS + 1, 4000)];
        assert_eq!(painful(&day, &Usable::new(vec![]), &usual).len(), 2);
    }

    #[test]
    fn a_slow_key_s_wait_splits_between_its_search_and_its_drawing() {
        let usual = usual_of("program:slack", &[2000, 2000, 2000]);
        let trial = Trial {
            timed: Some(Timed {
                felt: vec![Felt {
                    searched: 150.0,
                    drawn: 250.0,
                }],
            }),
            frames: Some(Frames {
                drawn: 100,
                late: 2,
                longest: 300.0,
            }),
            ..pick(AT, "program:slack", 1000)
        };
        let it = lost(&trial, None, &usual);
        assert_eq!(it.search, 120.0, "three fifths of the 200 ms past 50");
        assert_eq!(
            it.drawing,
            80.0 + 200.0,
            "the rest, and the stall past 100 ms"
        );
    }

    #[test]
    fn the_worst_come_first_and_no_more_than_are_kept() {
        let usual = Usual::of(&[]);
        let day: Vec<Trial> = (0..i64::try_from(KEPT).expect("KEPT is small") + 3)
            .map(|n| miss(AT + n * 200_000, 2000 + n * 1000))
            .collect();
        let named = painful(&day, &Usable::new(vec![]), &usual);
        assert_eq!(named.len(), KEPT);
        assert_eq!(named[0].rank, 1);
        assert_eq!(
            named[0].lost_seconds,
            2.0 + f64::from(u32::try_from(KEPT).expect("KEPT is small") + 2)
        );
        assert!(
            named
                .windows(2)
                .all(|w| w[0].lost_seconds >= w[1].lost_seconds),
            "worst first"
        );
    }
}
