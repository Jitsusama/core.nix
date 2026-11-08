//! When the daemon asks eero for each part of the home network: each as
//! often as its figures move, so as few requests as possible go over the
//! internet, and the fast ones only while someone is listening.

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
    /// since the others hang from its nodes.
    pub const ALL: [Read; 4] = [Read::Network, Read::Connections, Read::Usage, Read::Rest];

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

/// When each read was last done, and whether a listener has asked for one.
#[derive(Debug, Clone, Default)]
pub struct Schedule {
    last: [Option<Instant>; 4],
    asked: bool,
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

    /// Forgets every read but the network's, so all are due at once: once
    /// eero answers again after it couldn't, what was missed is read
    /// rather than waiting its turn.
    pub fn catch_up(&mut self) {
        for read in [Read::Connections, Read::Usage, Read::Rest] {
            self.last[read.index()] = None;
        }
    }

    fn until(&self, read: Read, now: Instant, listening: bool) -> Duration {
        let Some(last) = self.last[read.index()] else {
            return Duration::ZERO;
        };
        let good_for = if read == Read::Network && self.asked {
            ASKED_AT_MOST_EVERY
        } else {
            read.every(listening)
        };
        (last + good_for).saturating_duration_since(now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(s: u64) -> Duration {
        Duration::from_secs(s)
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

        assert_eq!(schedule.due(start + secs(29), true), [] as [schedule::Read; 0]);
        assert_eq!(schedule.due(start + secs(30), true), [Read::Network]);
        assert_eq!(schedule.wait(start + secs(10), true), secs(20));
    }

    #[test]
    fn the_network_is_read_every_5_minutes_while_nobody_listens() {
        let start = Instant::now();
        let schedule = all_done(start);

        assert_eq!(schedule.due(start + secs(299), false), [] as [schedule::Read; 0]);
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

        assert_eq!(schedule.due(start + secs(60), false), [] as [schedule::Read; 0]);
        assert_eq!(schedule.due(start + secs(60), true), [Read::Network]);
    }

    #[test]
    fn a_listener_asking_reads_the_network_now_but_not_twice_in_5_seconds() {
        let start = Instant::now();
        let mut schedule = all_done(start);

        schedule.ask();

        assert_eq!(schedule.due(start + secs(2), true), [] as [schedule::Read; 0]);
        assert_eq!(schedule.wait(start + secs(2), true), secs(3));
        assert_eq!(schedule.due(start + secs(5), true), [Read::Network]);
        schedule.done(Read::Network, start + secs(5));
        assert_eq!(schedule.wait(start + secs(5), true), secs(30));
    }

    #[test]
    fn catching_up_makes_every_read_but_the_network_due() {
        let start = Instant::now();
        let mut schedule = all_done(start);

        schedule.catch_up();

        assert_eq!(
            schedule.due(start + secs(1), true),
            [Read::Connections, Read::Usage, Read::Rest]
        );
    }
}
