//! The reflectors the recorder (src/record.rs) probes, and how they're
//! chosen: on each new network from public resolvers that answer
//! timestamps, one per operator first, since one operator's outage or a
//! queue near its servers would otherwise speak for all of them. With
//! fewer than three operators answering, the samples still go down, marked
//! so, since a controller has nothing to vote with there.
//!
//! A choice is asked again each hour, or when one of the pool keeps
//! missing, or in five minutes while it reaches fewer than three
//! operators and fewer than the candidates do. One that too few answer,
//! while the pool there is reaches more, keeps that pool for one choice,
//! asked again in five minutes: on
//! 17 November a choice asked inside half a minute of a dark network heard
//! nobody, and replaced six reflectors that answered again a moment later
//! with none.

use std::collections::{BTreeMap, BTreeSet};
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

/// Public resolvers, by operator, that a timestamp may be asked of. On
/// 3 November four of eleven tried answered from a home connection.
pub const CANDIDATES: &[(&str, [u8; 4])] = &[
    ("Quad9", [9, 9, 9, 9]),
    ("Quad9", [149, 112, 112, 112]),
    ("Quad9", [9, 9, 9, 11]),
    ("AdGuard", [94, 140, 14, 14]),
    ("AdGuard", [94, 140, 15, 15]),
    ("CleanBrowsing", [185, 228, 168, 9]),
    ("CleanBrowsing", [185, 228, 169, 9]),
    ("OpenDNS", [208, 67, 222, 222]),
    ("OpenDNS", [208, 67, 220, 220]),
    ("Level 3", [4, 2, 2, 1]),
    ("Level 3", [4, 2, 2, 2]),
    ("Control D", [76, 76, 2, 0]),
    ("Control D", [76, 76, 10, 0]),
    ("Verisign", [64, 6, 64, 6]),
    ("Verisign", [64, 6, 65, 6]),
    ("SafeDNS", [195, 46, 39, 39]),
    ("DNS.Watch", [84, 200, 69, 80]),
    ("Hurricane Electric", [74, 82, 42, 42]),
    ("Yandex", [77, 88, 8, 8]),
    ("UltraDNS", [156, 154, 70, 1]),
    ("Comodo", [8, 26, 56, 26]),
];

/// The reflectors to ask: `SHAPER_REFLECTORS`, comma-separated, when a
/// network of its own names them, a test's among them, and otherwise the
/// public resolvers.
pub fn candidates() -> Vec<(&'static str, Ipv4Addr)> {
    match std::env::var("SHAPER_REFLECTORS") {
        Ok(named) => named
            .split(',')
            .filter_map(|a| a.trim().parse().ok())
            .map(|a| ("named", a))
            .collect(),
        Err(_) => CANDIDATES
            .iter()
            .map(|&(operator, octets)| (operator, Ipv4Addr::from(octets)))
            .collect(),
    }
}

/// At most this many reflectors are probed, in turn.
const POOL: usize = 6;
/// Fewer operators than this answering leaves nothing to vote with.
pub const ENOUGH: usize = 3;
/// A pool member lost this many times running is replaced.
pub const MISSES: u32 = 3;
const REPOOL: Duration = Duration::from_secs(3_600);
/// How soon a pool too small to vote with, or kept, is asked for again.
const SHORT_REPOOL: Duration = Duration::from_secs(300);
/// How long the candidates asked for a pool are waited on.
pub const ASKING_FOR: Duration = Duration::from_millis(1_500);

/// The reflectors to probe from those that answered: one per operator in
/// the order they answered, then more of the same operators, up to the
/// pool's size.
pub fn pool(answered: &[(&'static str, Ipv4Addr)]) -> Vec<(&'static str, Ipv4Addr)> {
    let mut chosen: Vec<(&'static str, Ipv4Addr)> = Vec::new();
    let mut seen = BTreeSet::new();
    for &(operator, address) in answered {
        if chosen.len() < POOL && seen.insert(operator) {
            chosen.push((operator, address));
        }
    }
    for &(operator, address) in answered {
        if chosen.len() < POOL && !chosen.contains(&(operator, address)) {
            chosen.push((operator, address));
        }
    }
    chosen
}

/// How many operators a pool reaches.
pub fn operators(pool: &[(&'static str, Ipv4Addr)]) -> usize {
    pool.iter().map(|(o, _)| *o).collect::<BTreeSet<_>>().len()
}

/// The candidates asked for a pool, and those that answered so far. It's
/// heard in the loop beside the probes rather than waited on, since the
/// 1.5 s it takes left the gateway unsampled at each start and each hour:
/// a switch on 16 November restarted the shaper inside one.
pub struct Asking {
    asked: BTreeMap<u16, (&'static str, Ipv4Addr)>,
    heard: Vec<(&'static str, Ipv4Addr)>,
    until: Instant,
}

impl Asking {
    /// The candidates `asked`, by the number each ask went with, waited on
    /// from `now`.
    pub fn new(asked: BTreeMap<u16, (&'static str, Ipv4Addr)>, now: Instant) -> Asking {
        Asking {
            asked,
            heard: Vec::new(),
            until: now + ASKING_FOR,
        }
    }

    /// Take a timestamp reply numbered `seq`, from `from`, whose clock is
    /// `standard` or not, and say whether it answers an ask, so it's kept
    /// out of the record. A candidate is heard once, from its own address,
    /// and only with a clock one-way delays can be read from.
    pub fn answered(&mut self, seq: u16, from: Option<Ipv4Addr>, standard: bool) -> bool {
        let Some(&candidate) = self.asked.get(&seq) else {
            return false;
        };
        if standard && from == Some(candidate.1) && !self.heard.contains(&candidate) {
            self.heard.push(candidate);
        }
        true
    }

    /// Whether the asks have been waited on long enough at `now`.
    pub fn done(&self, now: Instant) -> bool {
        now >= self.until
    }

    /// The pool chosen from those heard, or none to keep `current`: a
    /// choice too few answer, when the pool there is has more, says the
    /// network went quiet while it was asked, not that its pool went.
    /// Once `kept` already, the answer is taken, since the network
    /// answering the same five minutes on is the network as it is.
    fn chosen(
        &self,
        current: &[(&'static str, Ipv4Addr)],
        kept: bool,
    ) -> Option<Vec<(&'static str, Ipv4Addr)>> {
        let chosen = pool(&self.heard);
        let few = !kept && operators(&chosen) < ENOUGH && operators(&chosen) < operators(current);
        (!few).then_some(chosen)
    }
}

/// Whether to ask for a new pool, `since` the last was chosen or kept:
/// when one of it keeps missing, `failing`, or when it's due, an hour on,
/// or five minutes on for a pool `short` of the operators it could reach
/// or `kept`.
fn asks_again(short: bool, kept: bool, failing: bool, since: Duration) -> bool {
    let due = if short || kept { SHORT_REPOOL } else { REPOOL };
    // A kept pool's misses don't ask again before then, or a network
    // that stays dark would be asked every few seconds.
    (failing && !kept) || since > due
}

/// The pool a recording probes, and how it came to be.
pub struct Pool {
    pub reflectors: Vec<(&'static str, Ipv4Addr)>,
    /// When it was chosen or kept, so only the misses since count, and
    /// it's asked for again on time.
    pub at: Instant,
    /// Whether a choice has settled yet, so the first is said.
    settled: bool,
    /// Whether the last choice kept it, so the next is asked within five
    /// minutes rather than the hour, its misses wait for it, and it's
    /// taken whatever it hears.
    pub kept: bool,
    /// How many operators the candidates reach, so a pool that has them
    /// all isn't short however few they are: every `SHAPER_REFLECTORS`
    /// address is one operator's.
    reachable: usize,
    /// Each of it lost this many times running since it was chosen.
    misses: BTreeMap<Ipv4Addr, u32>,
}

impl Pool {
    /// No pool yet, at `now`, from candidates reaching `reachable`
    /// operators.
    pub fn new(now: Instant, reachable: usize) -> Pool {
        Pool {
            reflectors: Vec::new(),
            at: now,
            settled: false,
            kept: false,
            reachable,
            misses: BTreeMap::new(),
        }
    }

    /// A probe to `address` came back, so its misses start again.
    pub fn heard(&mut self, address: Ipv4Addr) {
        self.misses.remove(&address);
    }

    /// A probe to `address`, sent at `sent`, was lost. It counts only if
    /// it went to this pool: one sent to the last, still out as this was
    /// chosen, would set off another choice at once.
    pub fn missed(&mut self, address: Ipv4Addr, sent: Instant) {
        if sent >= self.at {
            *self.misses.entry(address).or_default() += 1;
        }
    }

    /// Takes a choice that's done at `now`, on `link`: the pool it heard,
    /// or the pool there is, kept for one choice. Answers what to say in
    /// the service's log when the pool changes, is first chosen, is kept,
    /// or a keep ends, and not every five minutes on a network that
    /// answers no timestamps.
    pub fn settle(&mut self, link: &str, asking: &Asking, now: Instant) -> Option<String> {
        self.at = now;
        self.misses.clear();
        let Some(chosen) = asking.chosen(&self.reflectors, self.kept) else {
            self.kept = true;
            return Some(format!(
                "recording {link} with the {} reflectors it had, since {} operators answered \
                 a new choice; asking again in five minutes",
                self.reflectors.len(),
                operators(&asking.heard)
            ));
        };
        let said = (chosen != self.reflectors || !self.settled || self.kept).then(|| {
            format!(
                "recording {link} with {} reflectors from {} operators",
                chosen.len(),
                operators(&chosen)
            )
        });
        self.reflectors = chosen;
        self.settled = true;
        self.kept = false;
        said
    }

    /// Whether to ask for a new pool at `now`.
    pub fn asks_again(&self, now: Instant) -> bool {
        let failing = self.misses.values().any(|m| *m >= MISSES);
        let short = operators(&self.reflectors) < ENOUGH.min(self.reachable);
        asks_again(
            short,
            self.kept,
            failing,
            now.saturating_duration_since(self.at),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(n: u8) -> Ipv4Addr {
        Ipv4Addr::new(10, 0, 0, n)
    }

    const MINUTE: Duration = Duration::from_secs(60);

    #[test]
    fn a_pool_takes_one_of_each_operator_before_seconds() {
        let answered = [
            ("Quad9", at(1)),
            ("Quad9", at(2)),
            ("AdGuard", at(3)),
            ("CleanBrowsing", at(4)),
        ];
        let chosen = pool(&answered);
        assert_eq!(
            chosen,
            vec![
                ("Quad9", at(1)),
                ("AdGuard", at(3)),
                ("CleanBrowsing", at(4)),
                ("Quad9", at(2))
            ]
        );
        assert_eq!(operators(&chosen), 3);
    }

    #[test]
    fn a_pool_stops_at_six() {
        let answered: Vec<(&'static str, Ipv4Addr)> = (1..=9).map(|n| ("Quad9", at(n))).collect();
        assert_eq!(pool(&answered).len(), 6);
        assert_eq!(
            operators(&pool(&answered)),
            1,
            "one operator is still one vote"
        );
    }

    /// Asks of one operator twice and another once, waited on until `until`.
    fn asking(until: Instant) -> Asking {
        let quad9 = ("Quad9", at(9));
        Asking {
            asked: BTreeMap::from([(1, quad9), (2, ("AdGuard", at(1))), (3, quad9)]),
            heard: Vec::new(),
            until,
        }
    }

    /// A choice that heard `heard`.
    fn heard(heard: &[(&'static str, Ipv4Addr)]) -> Asking {
        let mut asked = asking(Instant::now());
        asked.heard = heard.to_vec();
        asked
    }

    fn three() -> Vec<(&'static str, Ipv4Addr)> {
        vec![("Quad9", at(9)), ("AdGuard", at(1)), ("Google", at(8))]
    }

    #[test]
    fn a_candidate_asked_twice_is_heard_once() {
        let mut asked = asking(Instant::now());
        assert!(
            asked.answered(1, Some(at(9)), true),
            "the first ask's reply went to the record"
        );
        assert!(
            asked.answered(3, Some(at(9)), true),
            "the second ask's reply went to the record"
        );
        assert_eq!(asked.heard, vec![("Quad9", at(9))]);
    }

    #[test]
    fn an_answer_from_another_address_is_kept_out_but_not_heard() {
        let mut asked = asking(Instant::now());
        assert!(
            asked.answered(2, Some(at(66)), true),
            "an ask's reply went to the record"
        );
        assert!(asked.heard.is_empty(), "heard {:?}", asked.heard);
    }

    #[test]
    fn an_answer_whose_clock_is_not_standard_is_not_heard() {
        let mut asked = asking(Instant::now());
        assert!(
            asked.answered(2, Some(at(1)), false),
            "an ask's reply went to the record"
        );
        assert!(asked.heard.is_empty(), "heard {:?}", asked.heard);
    }

    #[test]
    fn a_probe_reply_is_no_answer_to_the_asks() {
        let mut asked = asking(Instant::now());
        assert!(
            !asked.answered(4, Some(at(9)), true),
            "a probe's reply was taken as an ask's"
        );
        assert!(asked.heard.is_empty(), "heard {:?}", asked.heard);
    }

    #[test]
    fn the_asks_are_waited_on_until_their_time() {
        let now = Instant::now();
        let asked = Asking::new(BTreeMap::new(), now);
        assert!(!asked.done(now), "the asks were done as they were sent");
        assert!(
            asked.done(now + ASKING_FOR),
            "the asks were waited on past their time"
        );
    }

    // On 17 November the network went dark for half a minute from 08:55:05,
    // a choice asked inside it heard nobody, and six reflectors that
    // answered again once it was back were replaced by none until 09:00.
    #[test]
    fn a_choice_nobody_answers_keeps_the_pool() {
        assert_eq!(heard(&[]).chosen(&three(), false), None);
    }

    #[test]
    fn a_choice_too_few_answer_keeps_a_pool_with_enough() {
        assert_eq!(heard(&[("Quad9", at(9))]).chosen(&three(), false), None);
    }

    // Two operators can't vote, but one is fewer still: a choice that
    // reaches fewer than the pool there is keeps it, enough or not.
    #[test]
    fn a_choice_too_few_answer_keeps_a_pool_short_of_enough_with_more() {
        let current = vec![("Quad9", at(9)), ("AdGuard", at(1))];
        assert_eq!(heard(&[("Quad9", at(9))]).chosen(&current, false), None);
    }

    #[test]
    fn a_choice_with_enough_answering_replaces_the_pool() {
        let answer = vec![("Quad9", at(9)), ("AdGuard", at(1)), ("Cloud", at(66))];
        assert_eq!(heard(&answer).chosen(&three(), false), Some(answer));
    }

    // Fewer operators than the pool had, but enough to vote with, is a
    // pool that changed, not a network gone quiet.
    #[test]
    fn a_choice_enough_answer_replaces_a_larger_pool() {
        let mut current = three();
        current.push(("Cloud", at(66)));
        assert_eq!(heard(&three()).chosen(&current, false), Some(three()));
    }

    // A network that answers no timestamps has no pool to keep, and one
    // or two answering is still better than none.
    #[test]
    fn a_choice_with_no_pool_takes_whoever_answered() {
        assert_eq!(heard(&[]).chosen(&[], false), Some(vec![]));
        let one = vec![("Quad9", at(9))];
        assert_eq!(heard(&one).chosen(&[], false), Some(one));
    }

    #[test]
    fn a_pool_is_asked_for_again_when_it_s_due_or_failing() {
        assert!(
            !asks_again(false, true, true, MINUTE),
            "a kept pool's misses asked again within the minute"
        );
        assert!(
            asks_again(false, true, false, SHORT_REPOOL + MINUTE),
            "a kept pool waited past five minutes"
        );
        assert!(
            asks_again(false, false, true, MINUTE),
            "a chosen pool's misses didn't ask again"
        );
        assert!(
            !asks_again(false, false, false, SHORT_REPOOL + MINUTE),
            "a chosen pool was asked again before the hour"
        );
        assert!(
            asks_again(false, false, false, REPOOL + MINUTE),
            "a chosen pool waited past the hour"
        );
        assert!(
            asks_again(true, false, false, SHORT_REPOOL + MINUTE),
            "a short pool waited past five minutes"
        );
    }

    /// Operators the public resolvers reach, so three is never all.
    const PUBLIC: usize = 14;

    /// A pool chosen from three operators at `now`.
    fn chosen(now: Instant) -> Pool {
        let mut pool = Pool::new(now, PUBLIC);
        pool.settle("wlp0s20f3", &heard(&three()), now);
        pool
    }

    /// `address` lost `MISSES` times, each sent at `sent`.
    fn failing(pool: &mut Pool, address: Ipv4Addr, sent: Instant) {
        for _ in 0..MISSES {
            pool.missed(address, sent);
        }
    }

    #[test]
    fn a_reflector_missing_its_probes_asks_again() {
        let start = Instant::now();
        let mut pool = chosen(start);
        failing(&mut pool, at(9), start);
        assert!(pool.asks_again(start + MINUTE), "misses didn't ask again");
        pool.heard(at(9));
        assert!(
            !pool.asks_again(start + MINUTE),
            "an answer didn't start its misses again"
        );
    }

    #[test]
    fn a_probe_sent_to_the_last_pool_is_no_miss() {
        let start = Instant::now();
        let mut pool = chosen(start + MINUTE);
        failing(&mut pool, at(9), start);
        assert!(!pool.asks_again(start + 2 * MINUTE));
    }

    // Misses counted against a pool would otherwise set off the next
    // choice the moment one is settled.
    #[test]
    fn a_choice_starts_the_misses_again() {
        let start = Instant::now();
        let mut pool = chosen(start);
        failing(&mut pool, at(9), start);
        let next = start + MINUTE;
        pool.settle("wlp0s20f3", &heard(&three()), next);
        assert!(
            !pool.asks_again(next + MINUTE),
            "the last pool's misses asked again"
        );
    }

    #[test]
    fn a_first_choice_nobody_answers_is_said() {
        let start = Instant::now();
        let mut pool = Pool::new(start, PUBLIC);
        assert_eq!(
            pool.settle("wlp0s20f3", &heard(&[]), start).as_deref(),
            Some("recording wlp0s20f3 with 0 reflectors from 0 operators")
        );
    }

    // A pool that really lost its operators would otherwise be kept for
    // the rest of the stay, its dead reflectors never replaced.
    #[test]
    fn a_keep_lasts_one_choice() {
        let start = Instant::now();
        let mut pool = chosen(start);
        let dark = start + REPOOL;
        assert_eq!(
            pool.settle("wlp0s20f3", &heard(&[("Quad9", at(9))]), dark)
                .as_deref(),
            Some(concat!(
                "recording wlp0s20f3 with the 3 reflectors it had, ",
                "since 1 operators answered a new choice; asking again in five minutes"
            ))
        );
        assert_eq!(pool.reflectors, three(), "the pool wasn't kept");
        failing(&mut pool, at(9), dark);
        assert!(
            !pool.asks_again(dark + MINUTE),
            "a kept pool's misses asked again"
        );
        let again = dark + SHORT_REPOOL + MINUTE;
        assert!(
            pool.asks_again(again),
            "a kept pool waited past five minutes"
        );
        assert_eq!(
            pool.settle("wlp0s20f3", &heard(&[("Quad9", at(9))]), again)
                .as_deref(),
            Some("recording wlp0s20f3 with 1 reflectors from 1 operators")
        );
        assert_eq!(
            pool.reflectors,
            vec![("Quad9", at(9))],
            "a second keep in a row"
        );
        failing(&mut pool, at(9), again);
        assert!(
            pool.asks_again(again + MINUTE),
            "the pool taken after a keep didn't act on its misses"
        );
    }

    #[test]
    fn a_choice_ending_a_keep_is_said_though_the_pool_is_the_same() {
        let start = Instant::now();
        let mut pool = chosen(start);
        pool.settle("wlp0s20f3", &heard(&[]), start + REPOOL);
        let back = start + REPOOL + SHORT_REPOOL + MINUTE;
        assert_eq!(
            pool.settle("wlp0s20f3", &heard(&three()), back).as_deref(),
            Some("recording wlp0s20f3 with 3 reflectors from 3 operators")
        );
        assert!(!pool.kept);
        assert!(
            !pool.asks_again(back + SHORT_REPOOL + MINUTE),
            "a pool with enough was asked for again before the hour"
        );
    }

    #[test]
    fn the_same_pool_chosen_again_is_not_said() {
        let start = Instant::now();
        let mut pool = chosen(start);
        assert_eq!(
            pool.settle("wlp0s20f3", &heard(&three()), start + REPOOL),
            None
        );
    }

    #[test]
    fn a_pool_too_small_to_vote_with_is_asked_for_again_in_five_minutes() {
        let start = Instant::now();
        let mut pool = Pool::new(start, PUBLIC);
        pool.settle("wlp0s20f3", &heard(&[("Quad9", at(9))]), start);
        assert!(
            !pool.asks_again(start + SHORT_REPOOL),
            "a short pool was asked for again inside five minutes"
        );
        assert!(
            pool.asks_again(start + SHORT_REPOOL + Duration::from_secs(1)),
            "a short pool waited past five minutes"
        );
    }

    // Every `SHAPER_REFLECTORS` address is one operator's, so a pool of
    // them would otherwise be asked for again every five minutes for good.
    #[test]
    fn a_pool_with_every_operator_it_could_reach_waits_the_hour() {
        let start = Instant::now();
        let mut pool = Pool::new(start, 1);
        pool.settle("wlp0s20f3", &heard(&[("named", at(9))]), start);
        assert!(
            !pool.asks_again(start + SHORT_REPOOL + MINUTE),
            "a pool of every operator there is was asked for again"
        );
        assert!(pool.asks_again(start + REPOOL + MINUTE));
    }

    #[test]
    fn an_empty_pool_is_asked_for_again_in_five_minutes_wherever_it_is() {
        let start = Instant::now();
        let mut pool = Pool::new(start, 1);
        pool.settle("wlp0s20f3", &heard(&[]), start);
        assert!(pool.asks_again(start + SHORT_REPOOL + MINUTE));
    }
}
