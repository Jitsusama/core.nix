//! How far a long step has got, said while it runs: the step feeds a meter
//! as it moves bytes, and the screen reads it once a second, from another
//! thread, to say how much is done, how fast, and how long is left. A step
//! that moves nothing it can count is still said to be running, and for
//! how long, so a wait never looks like a hang.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// What a meter has seen since it was last started.
#[derive(Clone, Debug, Default)]
struct Reading {
    started: Option<Instant>,
    done: u64,
    total: Option<u64>,
}

/// A step's meter, shared between the step feeding it and the screen
/// reading it.
#[derive(Clone, Debug, Default)]
pub struct Meter(Arc<Mutex<Reading>>);

impl Meter {
    pub fn new() -> Meter {
        Meter::default()
    }

    /// Starts counting afresh, for a step about to run.
    pub fn start(&self) {
        *self.reading() = Reading {
            started: Some(Instant::now()),
            ..Reading::default()
        };
    }

    /// How many bytes the work comes to, once it's known.
    pub fn total(&self, bytes: u64) {
        self.reading().total = Some(bytes);
    }

    /// Adds bytes done.
    pub fn advance(&self, bytes: u64) {
        self.reading().done += bytes;
    }

    /// Sets the bytes done, for work read off as a running count.
    pub fn done(&self, bytes: u64) {
        self.reading().done = bytes;
    }

    /// What the screen says about it now.
    pub fn said(&self) -> String {
        let reading = self.reading().clone();
        let elapsed = reading.started.map(|started| started.elapsed());
        said(reading.done, reading.total, elapsed.unwrap_or_default())
    }

    fn reading(&self) -> std::sync::MutexGuard<'_, Reading> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// How far along, how fast and how long is left, in words; or, before any
/// byte is counted, how long it's been running.
pub fn said(done: u64, total: Option<u64>, elapsed: Duration) -> String {
    if done == 0 {
        return format!("Running for {}", duration(elapsed));
    }
    let rate = (elapsed.as_secs_f64() >= 1.0).then(|| done as f64 / elapsed.as_secs_f64());
    let mut parts = vec![match total {
        Some(total) => format!("{} of {}", bytes(done), bytes(total.max(done))),
        None => format!("{} so far", bytes(done)),
    }];
    if let Some(rate) = rate {
        parts.push(format!("{} a second", bytes(rate as u64)));
        if let Some(total) = total.filter(|total| *total > done) {
            let left = Duration::from_secs_f64((total - done) as f64 / rate);
            parts.push(format!("about {} left", rough(left)));
        }
    }
    parts.join(", ")
}

/// Bytes as a person reads them: decimal units, one place once it's past
/// a megabyte.
pub fn bytes(count: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if count < 1000 {
        return format!("{count} bytes");
    }
    let mut value = count as f64 / 1000.0;
    let mut unit = 0;
    while value >= 1000.0 && unit + 1 < UNITS.len() {
        value /= 1000.0;
        unit += 1;
    }
    if unit == 0 || value >= 100.0 {
        format!("{value:.0} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// A running time, to the second.
fn duration(elapsed: Duration) -> String {
    let seconds = elapsed.as_secs();
    let plural =
        |count: u64, unit: &str| format!("{count} {unit}{}", if count == 1 { "" } else { "s" });
    match (seconds / 60, seconds % 60) {
        (0, seconds) => plural(seconds, "second"),
        (minutes, 0) => plural(minutes, "minute"),
        (minutes, seconds) => format!(
            "{} {}",
            plural(minutes, "minute"),
            plural(seconds, "second")
        ),
    }
}

/// A time left, rounded as a person would say it: never to the second
/// past the first minute, since an estimate isn't that sure.
fn rough(left: Duration) -> String {
    let seconds = left.as_secs();
    if seconds < 90 {
        return "a minute".into();
    }
    let minutes = (seconds + 30) / 60;
    if minutes < 60 {
        return format!("{minutes} minutes");
    }
    let hours = minutes / 60;
    let minutes = minutes % 60;
    let hours = format!("{hours} hour{}", if hours == 1 { "" } else { "s" });
    if minutes == 0 {
        hours
    } else {
        format!("{hours} {minutes} minutes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_read_as_a_person_says_them() {
        assert_eq!(bytes(512), "512 bytes");
        assert_eq!(bytes(14_200_000), "14.2 MB");
        assert_eq!(bytes(9_512_000_000), "9.5 GB");
        assert_eq!(bytes(145_000_000), "145 MB");
    }

    #[test]
    fn before_any_byte_it_says_how_long_its_been_running() {
        assert_eq!(
            said(0, None, Duration::from_secs(5)),
            "Running for 5 seconds"
        );
        assert_eq!(
            said(0, Some(10), Duration::from_secs(130)),
            "Running for 2 minutes 10 seconds"
        );
        assert_eq!(
            said(0, None, Duration::from_secs(60)),
            "Running for 1 minute"
        );
    }

    #[test]
    fn with_a_total_it_says_how_far_how_fast_and_how_long_is_left() {
        assert_eq!(
            said(3_100_000_000, Some(9_500_000_000), Duration::from_secs(200)),
            "3.1 GB of 9.5 GB, 15.5 MB a second, about 7 minutes left"
        );
    }

    #[test]
    fn without_a_total_it_says_how_much_and_how_fast() {
        assert_eq!(
            said(1_200_000_000, None, Duration::from_secs(100)),
            "1.2 GB so far, 12.0 MB a second"
        );
    }

    #[test]
    fn the_time_left_is_rounded_as_a_person_would_say_it() {
        assert_eq!(rough(Duration::from_secs(20)), "a minute");
        assert_eq!(rough(Duration::from_secs(89)), "a minute");
        assert_eq!(rough(Duration::from_secs(150)), "3 minutes");
        assert_eq!(rough(Duration::from_secs(3900)), "1 hour 5 minutes");
    }

    #[test]
    fn a_meter_shared_with_another_thread_says_what_was_fed() {
        let meter = Meter::new();
        meter.start();
        meter.total(1000);
        let fed = meter.clone();
        std::thread::spawn(move || fed.advance(400)).join().unwrap();
        assert!(
            meter.said().starts_with("400 bytes of 1 KB"),
            "{}",
            meter.said()
        );
    }
}
