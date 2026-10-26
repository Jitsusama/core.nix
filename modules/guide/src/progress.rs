//! How far a long step has got, said while it runs: the step feeds a meter
//! as it moves bytes, and the screen reads it once a second, from another
//! thread, to say how much is done, how fast, and how long is left. A step
//! that moves nothing it can count is still said to be running, and for
//! how long, so a wait never looks like a hang. How fast, and so how long
//! is left, is over the last minute: long enough to span a disk taking
//! what's written in bursts as its cache fills and empties, and short
//! enough that a step that slows down says so rather than showing an
//! average that drifts. A step that stops for ten seconds says so at once.

use std::collections::VecDeque;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// What a meter has seen since it was last started.
#[derive(Clone, Debug, Default)]
struct Reading {
    started: Option<Instant>,
    /// The part of the work under way, said before how far along it is,
    /// and when it began, so its pace is its own.
    stage: Option<(String, Instant)>,
    done: u64,
    total: Option<u64>,
    /// Whether the stage is counted by what the program it runs reads or
    /// writes, for work done by a program the guide only waits on.
    by: Option<Io>,
    /// The program running now, by its process id.
    program: Option<u32>,
    /// How much was done when the screen last looked, and when.
    looked: VecDeque<(Instant, u64)>,
}

/// How far back a step is seen to have stopped.
const PACE: Duration = Duration::from_secs(10);

/// How far back how fast it goes, and how long is left, is reckoned from.
/// A copy under load reckoned over only `PACE` said 2 hours 20 minutes
/// left, then 21 minutes, then about a minute, within 30 seconds.
const STEADY: Duration = Duration::from_secs(60);

/// Bytes a second, over the last `PACE` and the last `STEADY`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Pace {
    lately: f64,
    steadily: f64,
}

impl Pace {
    /// One rate, for a reckoning with nothing but an average to go by.
    fn even(rate: f64) -> Pace {
        Pace {
            lately: rate,
            steadily: rate,
        }
    }
}

/// The pace from what was done each time it was looked at, which keeps
/// what it needs; none until there's a second to go by.
fn pace(looked: &mut VecDeque<(Instant, u64)>, now: Instant, done: u64) -> Option<Pace> {
    looked.push_back((now, done));
    while looked
        .get(1)
        .is_some_and(|(then, _)| now.duration_since(*then) >= STEADY)
    {
        looked.pop_front();
    }
    let over = |(then, was): (Instant, u64)| {
        let over = now.duration_since(then).as_secs_f64();
        (over >= 1.0).then(|| approximately(done.saturating_sub(was)) / over)
    };
    // The newest look at least `PACE` back, or the oldest kept.
    let lately = looked
        .iter()
        .rev()
        .find(|(then, _)| now.duration_since(*then) >= PACE)
        .or(looked.front())
        .copied()?;
    Some(Pace {
        lately: over(lately)?,
        steadily: over(*looked.front()?)?,
    })
}

/// A count of bytes as a float, for the rates and units a person reads
/// it in.
#[expect(
    clippy::cast_precision_loss,
    reason = "only a count past 2^53 bytes loses a bit, and it's read rounded to a place"
)]
fn approximately(count: u64) -> f64 {
    count as f64
}

/// A rate in whole bytes a second, its fraction dropped.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the rate shown is finite and at least 1, so the cast drops its fraction and no sign"
)]
fn whole(rate: f64) -> u64 {
    rate as u64
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

    /// Begins a part of the work, counted afresh: what it is, as the
    /// person would say it, and how many bytes it comes to where that's
    /// known.
    pub fn stage(&self, name: &str, total: Option<u64>) {
        let mut reading = self.reading();
        reading.stage = Some((name.to_string(), Instant::now()));
        reading.done = 0;
        reading.total = total;
        reading.by = None;
        reading.looked.clear();
    }

    /// Begins a part of the work a program does, counted by how many bytes
    /// it has read, which for one that copies files is how far along it
    /// is.
    pub fn stage_by_reads(&self, name: &str, total: Option<u64>) {
        self.stage(name, total);
        self.reading().by = Some(Io::Reads);
    }

    /// Begins a part of the work a program does, counted by how many bytes
    /// it has written, for one whose reads Linux can't count, as Nix maps
    /// the files it copies from rather than reading them.
    pub fn stage_by_writes(&self, name: &str, total: Option<u64>) {
        self.stage(name, total);
        self.reading().by = Some(Io::Writes);
    }

    /// The program running now, or that none is.
    pub fn running(&self, program: Option<u32>) {
        self.reading().program = program;
    }

    /// The program running now, if one is.
    pub fn program(&self) -> Option<u32> {
        self.reading().program
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

    /// The bytes counted in the stage under way.
    pub fn counted(&self) -> u64 {
        self.reading().done
    }

    /// What the screen says about it now.
    pub fn said(&self) -> String {
        let reading = {
            let mut reading = self.reading();
            if let (Some(by), Some(program)) = (reading.by, reading.program)
                && let Some(count) = counted_by(program, by)
            {
                reading.done = count;
            }
            let done = reading.done;
            let pace = pace(&mut reading.looked, Instant::now(), done);
            (reading.clone(), pace)
        };
        let (reading, pace) = reading;
        let elapsed =
            |since: Option<Instant>| since.map(|since| since.elapsed()).unwrap_or_default();
        match reading.stage {
            Some((stage, since)) => {
                let pace = match pace {
                    Some(pace) => told(reading.done, reading.total, Some(pace)),
                    None => said(reading.done, reading.total, elapsed(Some(since))),
                };
                // Before a byte of it is counted, how long the whole step has
                // been running says more than how long this part has.
                let pace = if reading.done == 0 {
                    said(0, None, elapsed(reading.started))
                } else {
                    pace
                };
                format!("{stage}: {}", lowered(&pace))
            }
            None => match pace {
                Some(pace) if reading.done > 0 => told(reading.done, reading.total, Some(pace)),
                _ => said(reading.done, reading.total, elapsed(reading.started)),
            },
        }
    }

    fn reading(&self) -> std::sync::MutexGuard<'_, Reading> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// Which of a program's counts a stage goes by.
#[derive(Clone, Copy, Debug)]
enum Io {
    Reads,
    Writes,
}

/// How many bytes a running program has read or written, every call it
/// made counted whether or not it reached a disk, as Linux says in /proc;
/// none where it doesn't.
fn counted_by(program: u32, by: Io) -> Option<u64> {
    let io = std::fs::read_to_string(format!("/proc/{program}/io")).ok()?;
    let field = match by {
        Io::Reads => "rchar:",
        Io::Writes => "wchar:",
    };
    io.lines()
        .find_map(|line| line.strip_prefix(field))
        .and_then(|count| count.trim().parse().ok())
}

fn lowered(said: &str) -> String {
    let mut chars = said.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Something read or written through, the bytes that pass one way
/// counted on a meter: a write counted by what's written, so a file
/// system's own lookups along the way don't count as progress, and a read
/// by what's read.
pub struct Counted<T> {
    inner: T,
    meter: Meter,
    reads: bool,
}

impl<T> Counted<T> {
    /// Counting what's written through it.
    pub fn writes(inner: T, meter: &Meter) -> Counted<T> {
        Counted {
            inner,
            meter: meter.clone(),
            reads: false,
        }
    }

    /// Counting what's read through it.
    pub fn reads(inner: T, meter: &Meter) -> Counted<T> {
        Counted {
            inner,
            meter: meter.clone(),
            reads: true,
        }
    }
}

impl<T: Read> Read for Counted<T> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        let count = self.inner.read(into)?;
        if self.reads {
            self.meter.advance(count as u64);
        }
        Ok(count)
    }
}

impl<T: Write> Write for Counted<T> {
    fn write(&mut self, from: &[u8]) -> io::Result<usize> {
        let count = self.inner.write(from)?;
        if !self.reads {
            self.meter.advance(count as u64);
        }
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<T: Seek> Seek for Counted<T> {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        self.inner.seek(to)
    }
}

/// How far along, how fast and how long is left, in words; or, before any
/// byte is counted, how long it's been running.
pub fn said(done: u64, total: Option<u64>, elapsed: Duration) -> String {
    if done == 0 {
        return format!("Running for {}", duration(elapsed));
    }
    let rate = (elapsed.as_secs_f64() >= 1.0).then(|| approximately(done) / elapsed.as_secs_f64());
    told(done, total, rate.map(Pace::even))
}

/// How far along, and at its pace how fast and how long is left.
fn told(done: u64, total: Option<u64>, pace: Option<Pace>) -> String {
    let mut parts = vec![match total {
        Some(total) => format!("{} of {}", bytes(done), bytes(total.max(done))),
        None => format!("{} so far", bytes(done)),
    }];
    match pace.filter(|pace| pace.lately.is_finite() && pace.steadily.is_finite()) {
        // Nothing moved over the last `PACE`, as while a step closes what
        // it wrote: how long is left can't be reckoned from that.
        Some(pace) if pace.lately < 1.0 => {
            parts.push(format!("nothing more in the last {}", duration(PACE)));
        }
        // A pace from under a hundredth of the work says nothing of the
        // rest, as a copy's slow start said 72 hours for five minutes, and
        // its speed read as a copy gone wrong, so both wait for that much.
        Some(_) if total.is_some_and(|total| total > done && done < total / 100) => {
            parts.push("just started: how fast and how long show once 1% is done".into());
        }
        Some(Pace { steadily: rate, .. }) => {
            parts.push(format!("{} a second", bytes(whole(rate))));
            let reckoned = |total: &u64| *total > done && done >= total / 100;
            let left = total.filter(reckoned).and_then(|total| {
                Duration::try_from_secs_f64(approximately(total - done) / rate).ok()
            });
            if let Some(left) = left {
                parts.push(rough(left));
            }
        }
        None => {}
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
    let mut value = approximately(count) / 1000.0;
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

/// How long is left, rounded as a person would say it: never to the
/// second, since an estimate isn't that sure, and under half a minute only
/// that it's nearly done.
fn rough(left: Duration) -> String {
    let seconds = left.as_secs();
    if seconds < 30 {
        return "a few seconds left".into();
    }
    if seconds < 90 {
        return "about a minute left".into();
    }
    let minutes = (seconds + 30) / 60;
    if minutes < 60 {
        return format!("about {minutes} minutes left");
    }
    let hours = minutes / 60;
    let minutes = minutes % 60;
    let hours = format!("{hours} hour{}", if hours == 1 { "" } else { "s" });
    if minutes == 0 {
        format!("about {hours} left")
    } else {
        format!("about {hours} {minutes} minutes left")
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
    fn under_a_hundredth_done_it_says_its_starting_rather_than_how_fast() {
        // A copy off a stick starts slowly while nix lists what it has to
        // copy: 575 KB in 20 seconds of 7.8 GB said 72 hours left, and
        // "25 KB a second" read as a copy gone wrong.
        assert_eq!(
            told(575_000, Some(7_800_000_000), Some(Pace::even(30_000.0))),
            "575 KB of 7.8 GB, just started: how fast and how long show once 1% is done"
        );
        assert_eq!(
            told(
                78_000_000,
                Some(7_800_000_000),
                Some(Pace::even(22_000_000.0))
            ),
            "78.0 MB of 7.8 GB, 22.0 MB a second, about 6 minutes left"
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
    fn the_pace_is_the_last_minute_not_the_whole_stage() {
        let start = Instant::now();
        let mut looked = VecDeque::new();
        let mut paced = None;
        // 100 MB a second for five seconds, then 1 MB a second.
        for second in 0..=80u64 {
            let done = if second <= 5 {
                second * 100_000_000
            } else {
                500_000_000 + (second - 5) * 1_000_000
            };
            paced = pace(&mut looked, start + Duration::from_secs(second), done);
        }
        let paced = paced.unwrap();
        for rate in [paced.lately, paced.steadily] {
            assert!(
                (900_000.0..1_100_000.0).contains(&rate),
                "{rate} bytes a second"
            );
        }
        assert!(looked.len() <= 61, "{} kept", looked.len());
    }

    #[test]
    fn a_disk_taking_writes_in_bursts_keeps_a_steady_time_left() {
        // As a copy under load did: a disk takes what's written in bursts
        // as its cache fills and empties, here 300 MB a second for 3
        // seconds in every 30, and 1 MB a second between.
        let start = Instant::now();
        let mut looked = VecDeque::new();
        let mut done = 0u64;
        let mut lately = vec![];
        let mut steadily = vec![];
        for second in 0..=240u64 {
            done += if second % 30 < 3 {
                300_000_000
            } else {
                1_000_000
            };
            let paced = pace(&mut looked, start + Duration::from_secs(second), done);
            if let Some(paced) = paced.filter(|_| second >= 60) {
                lately.push(paced.lately);
                steadily.push(paced.steadily);
            }
        }
        let spread = |rates: &[f64]| {
            let most = rates.iter().copied().fold(f64::MIN, f64::max);
            let least = rates.iter().copied().fold(f64::MAX, f64::min);
            most / least
        };
        assert_eq!(
            steadily.len(),
            181,
            "a pace every second from the first minute"
        );
        assert!(spread(&lately) > 10.0, "the last seconds swing");
        assert!(
            spread(&steadily) < 1.2,
            "the last minute swings by {}",
            spread(&steadily)
        );
    }

    #[test]
    fn how_fast_and_how_long_go_by_the_last_minute() {
        let pace = Pace {
            lately: 100_000_000.0,
            steadily: 15_500_000.0,
        };
        assert_eq!(
            told(3_100_000_000, Some(9_500_000_000), Some(pace)),
            "3.1 GB of 9.5 GB, 15.5 MB a second, about 7 minutes left"
        );
    }

    #[test]
    fn a_pace_that_stopped_says_so_rather_than_a_time_left() {
        // Stopped for the last ten seconds, though the last minute moved.
        let stopped = |lately| Pace {
            lately,
            steadily: 20_000_000.0,
        };
        assert_eq!(
            told(10_000_000_000, Some(10_100_000_000), Some(stopped(0.0))),
            "10.0 GB of 10.1 GB, nothing more in the last 10 seconds"
        );
        assert_eq!(
            told(10_000_000_000, Some(10_100_000_000), Some(stopped(1e-300))),
            "10.0 GB of 10.1 GB, nothing more in the last 10 seconds"
        );
    }

    #[test]
    fn the_time_left_is_rounded_as_a_person_would_say_it() {
        assert_eq!(rough(Duration::from_secs(3)), "a few seconds left");
        assert_eq!(rough(Duration::from_secs(29)), "a few seconds left");
        assert_eq!(rough(Duration::from_secs(30)), "about a minute left");
        assert_eq!(rough(Duration::from_secs(89)), "about a minute left");
        assert_eq!(rough(Duration::from_secs(150)), "about 3 minutes left");
        assert_eq!(rough(Duration::from_mins(60)), "about 1 hour left");
        assert_eq!(
            rough(Duration::from_mins(65)),
            "about 1 hour 5 minutes left"
        );
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

    #[test]
    fn each_stage_is_named_and_counted_afresh() {
        let meter = Meter::new();
        meter.start();
        meter.stage("Writing the stick", Some(2000));
        assert!(
            meter.said().starts_with("Writing the stick: running for "),
            "{}",
            meter.said()
        );
        meter.advance(2000);
        meter.stage("Reading it back", Some(1000));
        meter.advance(250);
        assert!(
            meter
                .said()
                .starts_with("Reading it back: 250 bytes of 1 KB"),
            "{}",
            meter.said()
        );
    }

    #[test]
    fn a_stage_a_program_does_is_counted_by_what_it_has_read() {
        let meter = Meter::new();
        meter.start();
        meter.stage_by_reads("Copying", Some(1 << 30));
        // sha256sum reads as Nix does, a read at a time; cat would splice,
        // which isn't counted as reading.
        let mut program = std::process::Command::new("sha256sum")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        meter.running(Some(program.id()));
        let mut stdin = program.stdin.take().unwrap();
        stdin.write_all(&vec![0u8; 300_000]).unwrap();
        stdin.flush().unwrap();
        let mut waited = 0;
        while meter.counted() < 300_000 && waited < 50 {
            let _ = meter.said();
            std::thread::sleep(Duration::from_millis(20));
            waited += 1;
        }
        assert!(meter.counted() >= 300_000, "{}", meter.said());
        drop(stdin);
        program.wait().unwrap();
        meter.running(None);
    }

    #[test]
    fn a_stage_a_program_does_can_be_counted_by_what_it_has_written() {
        let meter = Meter::new();
        meter.start();
        meter.stage_by_writes("Copying", Some(1 << 30));
        // head writes what it reads; with nothing read by the meter, only
        // its writes can have counted.
        let mut program = std::process::Command::new("head")
            .args(["-c", "300000", "/dev/zero"])
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        meter.running(Some(program.id()));
        let mut out = program.stdout.take().unwrap();
        let mut taken = vec![0u8; 100_000];
        std::io::Read::read_exact(&mut out, &mut taken).unwrap();
        let mut waited = 0;
        while meter.counted() < 100_000 && waited < 50 {
            let _ = meter.said();
            std::thread::sleep(Duration::from_millis(20));
            waited += 1;
        }
        assert!(meter.counted() >= 100_000, "{}", meter.said());
        drop(out);
        program.wait().unwrap();
        meter.running(None);
    }

    #[test]
    fn only_what_passes_the_counted_way_is_counted() {
        let meter = Meter::new();
        meter.start();
        let mut writing = Counted::writes(io::Cursor::new(vec![0u8; 100]), &meter);
        writing.write_all(&[1; 30]).unwrap();
        let mut back = vec![];
        writing.read_to_end(&mut back).unwrap();
        assert_eq!(back.len(), 70);
        assert!(
            meter.said().starts_with("30 bytes so far"),
            "a write's reads aren't counted: {}",
            meter.said()
        );

        let meter = Meter::new();
        meter.start();
        let mut reading = Counted::reads(io::Cursor::new(vec![0u8; 100]), &meter);
        reading.write_all(&[1; 30]).unwrap();
        reading.read_to_end(&mut back).unwrap();
        assert!(
            meter.said().starts_with("70 bytes so far"),
            "a read's writes aren't counted: {}",
            meter.said()
        );
    }
}
