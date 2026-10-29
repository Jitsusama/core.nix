//! What the builder did, one line of JSON an event, and what a day of it
//! cost.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

use serde_json::{Value, json};

/// Something that happened to a builder, as the journal keeps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// It was made and Nix was given it.
    Started,
    /// It was given back, having run `seconds`.
    Stopped,
    /// It went on its own, having run `seconds`, and was given back.
    Lost,
    /// It couldn't be made ready, and was given back.
    Failed,
    /// It was found left over, by the daily check or at start, and given back.
    Left,
}

impl Event {
    fn name(self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Stopped => "stopped",
            Self::Lost => "lost",
            Self::Failed => "failed",
            Self::Left => "left",
        }
    }
}

/// Add an event to the journal at `path`: when, what, which builder, and
/// how long it ran, where that's known.
pub fn note(
    path: &Path,
    at: u64,
    event: Event,
    name: &str,
    seconds: Option<u64>,
) -> io::Result<()> {
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    let mut line = json!({ "at": at, "event": event.name(), "name": name });
    if let Some(seconds) = seconds {
        line["seconds"] = seconds.into();
    }
    let mut journal = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(journal, "{line}")
}

/// A day of the journal: builders started, seconds they ran, and builders
/// found left over.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Day {
    pub started: u64,
    pub seconds: u64,
    pub left: u64,
}

/// The day of the journal's text from `from`, in seconds since the epoch,
/// up to a day after it.
pub fn day(journal: &str, from: u64) -> Day {
    let mut day = Day::default();
    for line in journal.lines() {
        let Ok(line) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if !line["at"]
            .as_u64()
            .is_some_and(|at| (from..from + 86_400).contains(&at))
        {
            continue;
        }
        match line["event"].as_str() {
            Some("started") => day.started += 1,
            Some("stopped" | "lost") => day.seconds += line["seconds"].as_u64().unwrap_or(0),
            Some("left") => day.left += 1,
            _ => {}
        }
    }
    day
}

/// A price in thousandths of a dollar, from how it's written, `0.68`.
pub fn price(written: &str) -> Option<u64> {
    let (dollars, cents) = written.split_once('.').unwrap_or((written, ""));
    if cents.len() > 3 || !cents.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let dollars: u64 = dollars.parse().ok()?;
    let thousandths: u64 = format!("{cents:0<3}").parse().ok()?;
    Some(dollars * 1000 + thousandths)
}

/// The day said in a line: how many builders, how long, what it cost at
/// `rate` thousandths of a dollar an hour, and what was left over.
pub fn said(day: &Day, rate: u64) -> String {
    let cost = (day.seconds * rate).div_ceil(3600);
    let minutes = day.seconds.div_ceil(60);
    let builders = if day.started == 1 {
        "builder"
    } else {
        "builders"
    };
    let left = match day.left {
        0 => "nothing left over".to_owned(),
        n => format!("{n} found left over and given back"),
    };
    format!(
        "{} {builders}, {}h {:02}m, ${}.{:02}, {left}",
        day.started,
        minutes / 60,
        minutes % 60,
        cost.div_ceil(10) / 100,
        cost.div_ceil(10) % 100
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_day_counts_its_builders_time_and_what_was_left() {
        let dir = tempfile::tempdir().expect("a folder");
        let path = dir.path().join("journal.jsonl");
        note(&path, 100, Event::Started, "b-1", None).expect("noted");
        note(&path, 2000, Event::Stopped, "b-1", Some(1900)).expect("noted");
        note(&path, 3000, Event::Started, "b-2", None).expect("noted");
        note(&path, 4000, Event::Lost, "b-2", Some(1000)).expect("noted");
        note(&path, 5000, Event::Left, "b-0", None).expect("noted");
        note(&path, 90_000, Event::Started, "tomorrow", None).expect("noted");
        let journal = fs::read_to_string(&path).expect("read");
        assert_eq!(
            day(&journal, 0),
            Day {
                started: 2,
                seconds: 2900,
                left: 1
            }
        );
    }

    #[test]
    fn a_line_it_cant_read_is_passed_over() {
        assert_eq!(
            day("not json\n{\"at\":5,\"event\":\"started\"}\n", 0).started,
            1
        );
    }

    #[test]
    fn prices_are_read_to_the_tenth_of_a_cent() {
        assert_eq!(price("0.68"), Some(680));
        assert_eq!(price("1"), Some(1000));
        assert_eq!(price("0.345"), Some(345));
        assert_eq!(price("0.3456"), None);
        assert_eq!(price("a.b"), None);
    }

    #[test]
    fn a_day_is_said_in_hours_and_dollars_rounded_up() {
        let day = Day {
            started: 2,
            seconds: 5400,
            left: 0,
        };
        assert_eq!(
            said(&day, 680),
            "2 builders, 1h 30m, $1.02, nothing left over"
        );
        let one = Day {
            started: 1,
            seconds: 61,
            left: 1,
        };
        assert_eq!(
            said(&one, 680),
            "1 builder, 0h 02m, $0.02, 1 found left over and given back"
        );
    }
}
