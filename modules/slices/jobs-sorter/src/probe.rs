//! Wakes on a steady beat and does nothing else, so the kernel's processor
//! pressure for its slice counts only the waiting others make an app wait.
//!
//! app.slice's own `cpu.pressure` counts every wait inside it, his programs
//! making each other wait among them: on 11 November nearly all of it came
//! from Slack's and Chrome's own bursts, which no agent caused. A watcher
//! that stopped agents' work on it stopped them for his own Slack. This
//! sits in a slice of its own at app.slice's weight and uses next to no
//! processor, so a wait in its slice's `cpu.pressure` is what a newly woken
//! app would have waited for everything else. In the lab, at 10 wakes a
//! second, it waited 17 µs a second on average against app.slice's 1,345,
//! and nothing at app.slice's worst second, which was Slack's alone.

use std::thread;
use std::time::Duration;

/// How often it wakes when not told: ten times a second, often enough that
/// a second's pressure reads a wait a woken app would feel, and rare enough
/// to cost nothing.
const EVERY: Duration = Duration::from_millis(100);

/// Parses `probe [--every MS]`.
pub fn every(args: &[String]) -> Result<Duration, String> {
    match args {
        [] => Ok(EVERY),
        [flag, value] if flag == "--every" => match value.parse::<u64>() {
            Ok(ms) if ms > 0 => Ok(Duration::from_millis(ms)),
            _ => Err(format!("--every takes a positive count of ms, not {value}")),
        },
        _ => Err("it takes `probe [--every MS]`".to_owned()),
    }
}

/// Wakes every `beat` for as long as it runs.
pub fn run(beat: Duration) -> ! {
    loop {
        // The wake itself is the measure: each one waits for a processor
        // as an app's would, and the kernel counts that wait.
        thread::sleep(beat);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| (*word).to_owned()).collect()
    }

    #[test]
    fn it_wakes_ten_times_a_second_unless_told() {
        assert_eq!(every(&args(&[])), Ok(EVERY));
        assert_eq!(EVERY, Duration::from_millis(100));
    }

    #[test]
    fn it_wakes_as_often_as_it_is_told() {
        assert_eq!(
            every(&args(&["--every", "20"])),
            Ok(Duration::from_millis(20))
        );
    }

    #[test]
    fn it_refuses_a_beat_it_cannot_keep() {
        for words in [
            &["--every", "0"][..],
            &["--every", "soon"],
            &["--every"],
            &["--often", "20"],
        ] {
            assert!(every(&args(words)).is_err(), "{words:?} was taken");
        }
    }
}
