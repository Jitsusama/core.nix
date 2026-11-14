//! How each access point does from where the machine is now, over the last
//! two minutes: how fast and how often its network's gateway answers, and
//! how many of the card's frames it had to send again or lost.
//!
//! An access point's signal says how loud it is, not how well it carries:
//! on 14 Nov 2026 two radios of one access point, at one desk, heard within
//! 4 dB of each other, answered the gateway at a p95 of 15 ms and 119 ms.
//! So wifid measures the one it's on, and keeps what it measured of each,
//! to choose between them.

use std::collections::{HashMap, VecDeque};

use tokio::time::{Duration, Instant};

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
        let _ = (point, at, echo);
    }

    /// Takes in the station's counts, read on the access point.
    pub fn counted(&mut self, point: &str, at: Instant, counts: Counts) {
        let _ = (point, at, counts);
    }

    /// The access point's figures over the span up to now, if anything was
    /// measured of it in it.
    pub fn of(&self, point: &str, now: Instant) -> Option<Measured> {
        let _ = (point, now, &self.points);
        None
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
    fn echoed(measures: &mut Measures, point: &str, start: Instant, times: &[Option<u64>]) {
        for (second, time) in (0..).zip(times) {
            let echo = time.map_or(Echo::Lost, |time| Echo::Answered(ms(time)));
            measures.echoed(point, start + Duration::from_secs(second), echo);
        }
    }

    #[test]
    fn an_access_point_s_echoes_say_its_middle_and_its_slow_end() {
        let start = Instant::now();
        let mut measures = Measures::default();
        let times: Vec<Option<u64>> = (1..=100).map(Some).collect();
        echoed(&mut measures, DESK, start, &times);
        let measured = measures.of(DESK, start + Duration::from_secs(100));
        let measured = measured.expect("an access point echoed through is measured");
        assert_eq!(measured.echoes, 100, "every echo in the span counts");
        assert_eq!(measured.p50, Some(ms(50)), "half answered by 50 ms");
        assert_eq!(measured.p95, Some(ms(95)), "95 of 100 answered by 95 ms");
        assert!(measured.lost.abs() < f64::EPSILON, "none lost: {}", measured.lost);
    }

    #[test]
    fn a_lost_echo_counts_as_lost_and_not_as_slow() {
        let start = Instant::now();
        let mut measures = Measures::default();
        echoed(&mut measures, DESK, start, &[Some(5), None, Some(5), None]);
        let measured = measures.of(DESK, start + Duration::from_secs(4));
        let measured = measured.expect("an access point echoed through is measured");
        assert!((measured.lost - 50.0).abs() < 1e-9, "two of four lost: {}", measured.lost);
        assert_eq!(measured.p95, Some(ms(5)), "the answered ones say how fast");
    }

    #[test]
    fn only_the_span_s_echoes_are_counted() {
        let start = Instant::now();
        let mut measures = Measures::default();
        echoed(&mut measures, DESK, start, &[Some(500)]);
        let later = start + SPAN + Duration::from_secs(10);
        echoed(&mut measures, DESK, later, &[Some(5)]);
        let measured = measures.of(DESK, later + Duration::from_secs(1));
        let measured = measured.expect("an access point echoed through is measured");
        assert_eq!(measured.echoes, 1, "the echo before the span is let go");
        assert_eq!(measured.p95, Some(ms(5)), "and says nothing of now");
    }

    #[test]
    fn each_access_point_is_measured_on_its_own() {
        let start = Instant::now();
        let mut measures = Measures::default();
        echoed(&mut measures, DESK, start, &[Some(5)]);
        echoed(&mut measures, HALL, start, &[Some(120)]);
        let now = start + Duration::from_secs(1);
        let desk = measures.of(DESK, now).expect("the desk's was measured");
        let hall = measures.of(HALL, now).expect("the hall's was measured");
        assert_eq!(desk.p95, Some(ms(5)), "the desk's echo is the desk's");
        assert_eq!(hall.p95, Some(ms(120)), "the hall's echo is the hall's");
    }

    #[test]
    fn an_access_point_never_measured_says_nothing() {
        let measures = Measures::default();
        assert_eq!(measures.of(DESK, Instant::now()), None, "nothing measured, nothing said");
    }

    #[test]
    fn retries_and_failures_are_per_hundred_frames_sent_in_the_span() {
        let start = Instant::now();
        let mut measures = Measures::default();
        measures.counted(DESK, start, counts(1_000, 10, 1));
        measures.counted(DESK, start + Duration::from_secs(60), counts(3_000, 110, 5));
        let measured = measures.of(DESK, start + Duration::from_secs(60));
        let measured = measured.expect("an access point counted on is measured");
        assert!((measured.retried - 5.0).abs() < 1e-9, "100 again of 2,000: {}", measured.retried);
        assert!((measured.failed - 0.2).abs() < 1e-9, "4 lost of 2,000: {}", measured.failed);
        assert_eq!(measured.signal, Some(-47), "the latest signal is said");
    }

    #[test]
    fn counts_that_start_again_on_a_new_join_are_counted_from_there() {
        let start = Instant::now();
        let mut measures = Measures::default();
        measures.counted(DESK, start, counts(9_000, 900, 90));
        measures.counted(DESK, start + Duration::from_secs(10), counts(100, 1, 0));
        measures.counted(DESK, start + Duration::from_secs(20), counts(1_100, 21, 2));
        let measured = measures.of(DESK, start + Duration::from_secs(20));
        let measured = measured.expect("an access point counted on is measured");
        assert!((measured.retried - 2.0).abs() < 1e-9, "20 again of 1,000: {}", measured.retried);
        assert!((measured.failed - 0.2).abs() < 1e-9, "2 lost of 1,000: {}", measured.failed);
    }
}
