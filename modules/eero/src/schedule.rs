//! When the daemon asks eero for each part of the home network: each as
//! often as its figures move, so as few requests as possible go over the
//! internet, and the fast ones only while someone is listening. And when
//! it takes its own look at the internet (probe.rs).

use std::time::{Duration, Instant};

/// A group of reads that move at one pace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Read {
    /// The network and its devices, two requests: the nodes' state, each
    /// radio's channel and how busy it is, each device's signal, rate and
    /// speed, which move by the second.
    Network,
    /// Each node's connections, a request a node: its ports' best speeds
    /// and which carries its backhaul, which move when a cable does.
    Connections,
    /// The day's usage by hour, the network's and each node's, which grows
    /// by the hour.
    Usage,
    /// Speed tests, profiles, Thread, outages and events, five requests,
    /// which change a few times a day.
    Rest,
}

impl Read {
    /// Every read, in the order a cycle takes them: the network first,
    /// since the others hang from its nodes, and the rest before usage,
    /// since each profile's traffic is asked by the profiles the rest
    /// reads.
    pub const ALL: [Read; 4] = [Read::Network, Read::Connections, Read::Rest, Read::Usage];

    /// How long its answer is good for.
    pub fn every(self, listening: bool) -> Duration {
        let minutes = |m: u64| Duration::from_secs(m * 60);
        match self {
            Read::Network if listening => Duration::from_secs(30),
            Read::Network | Read::Connections => minutes(5),
            Read::Usage | Read::Rest => minutes(15),
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

/// The soonest a listener's asking reads the network again after the last
/// read of it, so a listener asking over and over can't make the daemon
/// hammer eero.
pub const ASKED_AT_MOST_EVERY: Duration = Duration::from_secs(5);

/// How soon the network is read again after eero couldn't be reached. A
/// machine starting reads before its network is up, and waiting the five
/// minutes a read is good for would leave the shell with nothing that long.
pub const UNREACHED_AGAIN_AFTER: Duration = Duration::from_secs(10);

/// When each read was last done, whether a listener has asked for one, and
/// whether eero couldn't be reached at the last.
#[derive(Debug, Clone, Default)]
pub struct Schedule {
    last: [Option<Instant>; 4],
    asked: bool,
    unreached: bool,
}

impl Schedule {
    /// The reads due at `now`, in the order to take them.
    pub fn due(&self, now: Instant, listening: bool) -> Vec<Read> {
        Read::ALL
            .into_iter()
            .filter(|read| self.until(*read, now, listening).is_zero())
            .collect()
    }

    /// How long to sleep from `now` until the next read is due.
    pub fn wait(&self, now: Instant, listening: bool) -> Duration {
        Read::ALL
            .into_iter()
            .map(|read| self.until(read, now, listening))
            .min()
            .unwrap_or_default()
    }

    /// Marks a read done at `at`, whether eero answered it or not, so a
    /// read that fails waits its turn rather than being asked again at once.
    pub fn done(&mut self, read: Read, at: Instant) {
        self.last[read.index()] = Some(at);
        if read == Read::Network {
            self.asked = false;
        }
    }

    /// A listener asks for the network to be read now.
    pub fn ask(&mut self) {
        self.asked = true;
    }

    /// Whether eero could be reached at the last read. Only a failure to
    /// reach it reads sooner: a refusal, or being signed out, isn't helped
    /// by asking again.
    pub fn reached(&mut self, reached: bool) {
        self.unreached = !reached;
    }

    /// Forgets every read but the network's, so all are due at once: once
    /// eero answers again after it couldn't, what was missed is read
    /// rather than waiting its turn.
    pub fn catch_up(&mut self) {
        for read in [Read::Connections, Read::Rest, Read::Usage] {
            self.last[read.index()] = None;
        }
    }

    fn until(&self, read: Read, now: Instant, listening: bool) -> Duration {
        let Some(last) = self.last[read.index()] else {
            return Duration::ZERO;
        };
        let good_for = match read {
            Read::Network if self.asked => ASKED_AT_MOST_EVERY,
            Read::Network if self.unreached => UNREACHED_AGAIN_AFTER,
            _ => read.every(listening),
        };
        (last + good_for).saturating_duration_since(now)
    }
}

/// The soonest the daemon looks at the internet again after its last look.
pub const LOOK_AT_MOST_EVERY: Duration = Duration::from_secs(10);

/// When the daemon last looked at the internet itself.
#[derive(Debug, Clone, Default)]
pub struct Looks {
    last: Option<Instant>,
}

impl Looks {
    /// Whether to look at `now`: with each read of the network, and while
    /// eero's cloud can't be reached, every `LOOK_AT_MOST_EVERY` between,
    /// since then the look is all that says how the internet is. Never
    /// twice within it, so a listener asking for reads can't make the
    /// daemon look faster. Signed out, the internet is fine and looks ride
    /// on the reads.
    pub fn due(&self, now: Instant, network_read: bool, unreachable: bool) -> bool {
        let rested = self
            .last
            .is_none_or(|last| now.duration_since(last) >= LOOK_AT_MOST_EVERY);
        rested && (network_read || unreachable)
    }

    /// How long from `now` until a look is due of itself: never while eero
    /// can be reached, since then the looks ride on the network's reads.
    pub fn wait(&self, now: Instant, unreachable: bool) -> Option<Duration> {
        unreachable.then(|| {
            self.last.map_or(Duration::ZERO, |last| {
                (last + LOOK_AT_MOST_EVERY).saturating_duration_since(now)
            })
        })
    }

    pub fn done(&mut self, at: Instant) {
        self.last = Some(at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(s: u64) -> Duration {
        Duration::from_secs(s)
    }

    #[test]
    fn the_internet_is_looked_at_with_each_network_read_but_not_twice_in_10_seconds() {
        let start = Instant::now();
        let mut looks = Looks::default();

        assert!(looks.due(start, true, false));
        looks.done(start);

        assert!(!looks.due(start + secs(5), true, false), "an asked read");
        assert!(!looks.due(start + secs(30), false, false));
        assert!(looks.due(start + secs(30), true, false));
        assert_eq!(looks.wait(start + secs(30), false), None);
    }

    #[test]
    fn the_internet_is_looked_at_every_10_seconds_while_eero_cant_be_reached() {
        let start = Instant::now();
        let mut looks = Looks::default();
        looks.done(start);

        assert_eq!(looks.wait(start + secs(4), true), Some(secs(6)));
        assert!(!looks.due(start + secs(9), false, true));
        assert!(looks.due(start + secs(10), false, true));
    }

    /// A schedule with every read done at `at`.
    fn all_done(at: Instant) -> Schedule {
        let mut schedule = Schedule::default();
        for read in Read::ALL {
            schedule.done(read, at);
        }
        schedule
    }

    #[test]
    fn everything_is_due_before_anything_is_read() {
        assert_eq!(
            Schedule::default().due(Instant::now(), false),
            Read::ALL.to_vec()
        );
    }

    #[test]
    fn the_network_is_read_every_30_seconds_while_someone_listens() {
        let start = Instant::now();
        let schedule = all_done(start);

        assert_eq!(
            schedule.due(start + secs(29), true),
            [] as [crate::schedule::Read; 0]
        );
        assert_eq!(schedule.due(start + secs(30), true), [Read::Network]);
        assert_eq!(schedule.wait(start + secs(10), true), secs(20));
    }

    #[test]
    fn the_network_is_read_every_5_minutes_while_nobody_listens() {
        let start = Instant::now();
        let schedule = all_done(start);

        assert_eq!(
            schedule.due(start + secs(299), false),
            [] as [crate::schedule::Read; 0]
        );
        assert_eq!(
            schedule.due(start + secs(300), false),
            [Read::Network, Read::Connections]
        );
        assert_eq!(schedule.wait(start + secs(10), false), secs(290));
    }

    #[test]
    fn usage_and_the_rest_are_read_every_15_minutes_listened_to_or_not() {
        let start = Instant::now();
        let schedule = all_done(start);
        let slow = |due: Vec<Read>| {
            due.into_iter()
                .filter(|r| matches!(r, Read::Usage | Read::Rest))
        };

        for listening in [true, false] {
            assert_eq!(slow(schedule.due(start + secs(899), listening)).count(), 0);
            assert_eq!(slow(schedule.due(start + secs(900), listening)).count(), 2);
        }
    }

    #[test]
    fn a_listener_joining_makes_a_stale_network_read_due_at_once() {
        let start = Instant::now();
        let schedule = all_done(start);

        assert_eq!(
            schedule.due(start + secs(60), false),
            [] as [crate::schedule::Read; 0]
        );
        assert_eq!(schedule.due(start + secs(60), true), [Read::Network]);
    }

    #[test]
    fn a_listener_asking_reads_the_network_now_but_not_twice_in_5_seconds() {
        let start = Instant::now();
        let mut schedule = all_done(start);

        schedule.ask();

        assert_eq!(
            schedule.due(start + secs(2), true),
            [] as [crate::schedule::Read; 0]
        );
        assert_eq!(schedule.wait(start + secs(2), true), secs(3));
        assert_eq!(schedule.due(start + secs(5), true), [Read::Network]);
        schedule.done(Read::Network, start + secs(5));
        assert_eq!(schedule.wait(start + secs(5), true), secs(30));
    }

    #[test]
    fn the_network_is_read_every_10_seconds_while_eero_cant_be_reached() {
        let start = Instant::now();
        let mut schedule = all_done(start);

        schedule.reached(false);

        assert_eq!(schedule.wait(start + secs(4), false), secs(6));
        assert_eq!(schedule.due(start + secs(10), false), [Read::Network]);
        schedule.reached(true);
        assert_eq!(schedule.wait(start + secs(10), false), secs(290));
    }

    #[test]
    fn catching_up_makes_every_read_but_the_network_due() {
        let start = Instant::now();
        let mut schedule = all_done(start);

        schedule.catch_up();

        assert_eq!(
            schedule.due(start + secs(1), true),
            [Read::Connections, Read::Rest, Read::Usage]
        );
    }
}
