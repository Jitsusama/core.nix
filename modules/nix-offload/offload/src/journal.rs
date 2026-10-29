//! What the builder did, one line of JSON an event, and what a day of it
//! cost.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

use serde_json::{Value, json};

/// Something that happened to a builder, as the journal keeps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The helper was asked to make it, before it ran, so a builder half
    /// made when this stops is still known to be this one's.
    Asked,
    /// It was made and Nix was given it, `seconds` after it was asked for,
    /// at the price the helper said.
    Started,
    /// The service was told to end while it was lent, having run
    /// `seconds`. It's given back by the next start rather than now, since
    /// a switch starts nothing until every stop has ended.
    Ended,
    /// It was given back, having run `seconds`.
    Stopped,
    /// It went on its own, having run `seconds`, and was given back.
    Lost,
    /// It couldn't be made ready in `seconds`, and was given back.
    Failed,
    /// It was found not given back, at start or later, and now is.
    Left,
}

/// The events that say a builder is gone: each is noted only once the
/// helper said so.
const GONE: [&str; 4] = ["stopped", "lost", "failed", "left"];

impl Event {
    fn name(self) -> &'static str {
        match self {
            Self::Asked => "asked",
            Self::Started => "started",
            Self::Ended => "ended",
            Self::Stopped => "stopped",
            Self::Lost => "lost",
            Self::Failed => "failed",
            Self::Left => "left",
        }
    }
}

/// How long a builder ran, and its price an hour in thousandths of a
/// dollar where the helper said one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ran {
    pub seconds: u64,
    pub rate: Option<u64>,
}

/// Add an event to the journal at `path`: when, what, which builder, and
/// how long it ran and at what price, where that's known.
pub fn note(path: &Path, at: u64, event: Event, name: &str, ran: Option<Ran>) -> io::Result<()> {
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    let mut line = json!({ "at": at, "event": event.name(), "name": name });
    if let Some(ran) = ran {
        line["seconds"] = ran.seconds.into();
        if let Some(rate) = ran.rate {
            line["rate"] = rate.into();
        }
    }
    let mut journal = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(journal, "{line}")
}

/// A builder this asked for and hasn't seen gone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unreturned {
    pub name: String,
    /// When it was asked for.
    pub asked: u64,
    /// Its price an hour, where the helper said one.
    pub rate: Option<u64>,
    /// When a stop left it for the next start to give back, and how long
    /// it had run then; none where it was lost track of instead.
    pub ended: Option<Ended>,
}

/// When a stop left a builder, and how long it had run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ended {
    pub at: u64,
    pub seconds: u64,
}

impl Unreturned {
    /// How it's journalled once given back at `now`. One a stop left ran on
    /// until now, or until its own delete took it `idle` seconds after Nix
    /// let it be, whichever came first. One lost track of is found left
    /// over, having run for a time nothing here knows.
    pub fn given_back(&self, now: u64, idle: u64) -> (Event, Option<Ran>) {
        match self.ended {
            Some(ended) => {
                let after = now.saturating_sub(ended.at).min(idle);
                let ran = Ran {
                    seconds: ended.seconds + after,
                    rate: self.rate,
                };
                (Event::Stopped, Some(ran))
            }
            None => (Event::Left, None),
        }
    }
}

/// The builders this asked for and hasn't seen gone, oldest first. They're
/// the only ones it may give back: whatever else shares their prefix is
/// someone else's.
pub fn unreturned(journal: &str) -> Vec<Unreturned> {
    let mut out: Vec<Unreturned> = Vec::new();
    for line in journal.lines() {
        let Ok(line) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let (Some(event), Some(name)) = (line["event"].as_str(), line["name"].as_str()) else {
            continue;
        };
        if GONE.contains(&event) {
            out.retain(|known| known.name != name);
            continue;
        }
        if !matches!(event, "asked" | "started" | "ended") {
            continue;
        }
        if !out.iter().any(|known| known.name == name) {
            out.push(Unreturned {
                name: name.to_owned(),
                asked: line["at"].as_u64().unwrap_or(0),
                rate: None,
                ended: None,
            });
        }
        if let Some(known) = out.iter_mut().find(|known| known.name == name) {
            known.rate = line["rate"].as_u64().or(known.rate);
            if event == "ended" {
                known.ended = Some(Ended {
                    at: line["at"].as_u64().unwrap_or(known.asked),
                    seconds: line["seconds"].as_u64().unwrap_or(0),
                });
            }
        }
    }
    out
}

/// A day of the journal: builders asked for, seconds they ran, what that
/// cost in thousandths of a dollar times seconds an hour, and builders
/// found left over.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Day {
    pub started: u64,
    pub seconds: u64,
    pub spent: u64,
    pub left: u64,
}

/// The day of the journal's text from `from`, in seconds since the epoch,
/// up to a day after it. Each builder's time is priced at the rate the
/// helper said for it, since a fallback machine costs more, or at `rate`
/// where it said none.
pub fn day(journal: &str, from: u64, rate: u64) -> Day {
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
            Some("asked") => day.started += 1,
            Some(event @ ("stopped" | "lost" | "failed" | "left")) => {
                let seconds = line["seconds"].as_u64().unwrap_or(0);
                day.seconds += seconds;
                day.spent += seconds * line["rate"].as_u64().unwrap_or(rate);
                day.left += u64::from(event == "left");
            }
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

/// The day said in a line: how many builders, how long, what it cost, and
/// what was left over.
pub fn said(day: &Day) -> String {
    let cost = day.spent.div_ceil(3600);
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
        let (cheap, dear) = (Some(680), Some(890));
        let ran = |seconds, rate| Some(Ran { seconds, rate });
        note(&path, 90, Event::Asked, "b-1", None).expect("noted");
        note(&path, 100, Event::Started, "b-1", None).expect("noted");
        note(&path, 2000, Event::Stopped, "b-1", ran(1910, cheap)).expect("noted");
        note(&path, 2900, Event::Asked, "b-2", None).expect("noted");
        note(&path, 3000, Event::Started, "b-2", None).expect("noted");
        note(&path, 4000, Event::Lost, "b-2", ran(1100, dear)).expect("noted");
        note(&path, 4100, Event::Asked, "b-3", None).expect("noted");
        note(&path, 4700, Event::Failed, "b-3", ran(600, None)).expect("noted");
        note(&path, 5000, Event::Left, "b-0", None).expect("noted");
        note(&path, 90_000, Event::Asked, "tomorrow", None).expect("noted");
        let journal = fs::read_to_string(&path).expect("read");
        assert_eq!(
            day(&journal, 0, 1000),
            Day {
                started: 3,
                seconds: 3610,
                spent: 1910 * 680 + 1100 * 890 + 600 * 1000,
                left: 1
            }
        );
    }

    #[test]
    fn a_line_it_cant_read_is_passed_over() {
        assert_eq!(
            day("not json\n{\"at\":5,\"event\":\"asked\"}\n", 0, 680).started,
            1
        );
    }

    #[test]
    fn only_what_it_asked_for_and_never_saw_gone_is_unreturned() {
        let journal = [
            r#"{"at":1,"event":"asked","name":"lend-1"}"#,
            r#"{"at":2,"event":"started","name":"lend-1"}"#,
            r#"{"at":3,"event":"stopped","name":"lend-1","seconds":2}"#,
            r#"{"at":4,"event":"asked","name":"lend-4"}"#,
            r#"{"at":5,"event":"asked","name":"lend-5"}"#,
            r#"{"at":6,"event":"started","name":"lend-5","seconds":1,"rate":890}"#,
            r#"{"at":9,"event":"ended","name":"lend-5","seconds":4,"rate":890}"#,
            r#"{"at":7,"event":"left","name":"someone-elses"}"#,
        ]
        .join("\n");
        assert_eq!(
            unreturned(&journal),
            [
                Unreturned {
                    name: "lend-4".to_owned(),
                    asked: 4,
                    rate: None,
                    ended: None,
                },
                Unreturned {
                    name: "lend-5".to_owned(),
                    asked: 5,
                    rate: Some(890),
                    ended: Some(Ended { at: 9, seconds: 4 }),
                },
            ]
        );
        assert!(unreturned("").is_empty());
    }

    #[test]
    fn one_a_stop_left_is_priced_to_its_return_or_its_own_delete() {
        let left = Unreturned {
            name: "lend-5".to_owned(),
            asked: 5,
            rate: Some(890),
            ended: Some(Ended {
                at: 100,
                seconds: 95,
            }),
        };
        let ran = |seconds| {
            Some(Ran {
                seconds,
                rate: Some(890),
            })
        };
        assert_eq!(left.given_back(130, 900), (Event::Stopped, ran(125)));
        assert_eq!(left.given_back(90_000, 900), (Event::Stopped, ran(995)));
        let lost = Unreturned {
            ended: None,
            ..left
        };
        assert_eq!(lost.given_back(130, 900), (Event::Left, None));
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
            spent: 5400 * 680,
            left: 0,
        };
        assert_eq!(said(&day), "2 builders, 1h 30m, $1.02, nothing left over");
        let one = Day {
            started: 1,
            seconds: 61,
            spent: 61 * 680,
            left: 1,
        };
        assert_eq!(
            said(&one),
            "1 builder, 0h 02m, $0.02, 1 found left over and given back"
        );
    }
}
