//! How far a long step has got, said while it runs: the step feeds a meter
//! as it moves bytes, and the screen reads it once a second, from another
//! thread, to say how much is done, how fast, and how long is left. A step
//! that moves nothing it can count is still said to be running, and for
//! how long, so a wait never looks like a hang.

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
    /// Whether the stage is counted by what the program it runs reads,
    /// for work done by a program the guide only waits on.
    by_reads: bool,
    /// The program running now, by its process id.
    program: Option<u32>,
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
        reading.by_reads = false;
    }

    /// Begins a part of the work a program does, counted by how many bytes
    /// it has read, which for one that copies files is how far along it
    /// is.
    pub fn stage_by_reads(&self, name: &str, total: Option<u64>) {
        self.stage(name, total);
        self.reading().by_reads = true;
    }

    /// The program running now, or that none is.
    pub fn running(&self, program: Option<u32>) {
        self.reading().program = program;
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
            if let (true, Some(program)) = (reading.by_reads, reading.program)
                && let Some(read) = read_by(program)
            {
                reading.done = read;
            }
            reading.clone()
        };
        let elapsed =
            |since: Option<Instant>| since.map(|since| since.elapsed()).unwrap_or_default();
        match reading.stage {
            Some((stage, since)) => {
                let pace = said(reading.done, reading.total, elapsed(Some(since)));
                // Before a byte of it is counted, how long the whole step has
                // been running says more than how long this part has.
                let pace = if reading.done == 0 {
                    said(0, None, elapsed(reading.started))
                } else {
                    pace
                };
                format!("{stage}: {}", lowered(&pace))
            }
            None => said(reading.done, reading.total, elapsed(reading.started)),
        }
    }

    fn reading(&self) -> std::sync::MutexGuard<'_, Reading> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// How many bytes a running program has read, every read it made counted
/// whether or not it reached a disk, as Linux says in /proc; none where
/// it doesn't.
fn read_by(program: u32) -> Option<u64> {
    let io = std::fs::read_to_string(format!("/proc/{program}/io")).ok()?;
    io.lines()
        .find_map(|line| line.strip_prefix("rchar:"))
        .and_then(|count| count.trim().parse().ok())
}

fn lowered(said: &str) -> String {
    let mut chars = said.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Something read or written through, every byte that passes counted on
/// a meter.
pub struct Counted<T> {
    inner: T,
    meter: Meter,
}

impl<T> Counted<T> {
    pub fn new(inner: T, meter: &Meter) -> Counted<T> {
        Counted {
            inner,
            meter: meter.clone(),
        }
    }
}

impl<T: Read> Read for Counted<T> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        let count = self.inner.read(into)?;
        self.meter.advance(count as u64);
        Ok(count)
    }
}

impl<T: Write> Write for Counted<T> {
    fn write(&mut self, from: &[u8]) -> io::Result<usize> {
        let count = self.inner.write(from)?;
        self.meter.advance(count as u64);
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
    fn what_passes_through_is_counted_both_ways() {
        let meter = Meter::new();
        meter.start();
        let mut counted = Counted::new(io::Cursor::new(vec![0u8; 100]), &meter);
        counted.write_all(&[1; 30]).unwrap();
        let mut back = vec![];
        counted.read_to_end(&mut back).unwrap();
        assert_eq!(back.len(), 70);
        assert!(
            meter.said().starts_with("100 bytes so far"),
            "{}",
            meter.said()
        );
    }
}
