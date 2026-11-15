//! A day of the network's figures, one line a minute, kept beside the
//! record in `$XDG_STATE_HOME/net/day.jsonl` so net can say what changed
//! over the day and not only what is. Only figures that climb or swing
//! are kept: what each of the modem's channels lost beyond repair, and
//! each radio's busy share. A line that doesn't parse is skipped, so a
//! half-written last line after a crash costs one minute, not the day.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::model::{Channel, Modem, Network};
use crate::sources::modem;

/// How far back the day reaches.
pub const DAY: u64 = 24 * 60 * 60;

/// How far past a day the file may run before it's rewritten without its
/// oldest lines: rewriting each minute would write the whole day each
/// time, so it's trimmed about once an hour instead.
const SLACK: u64 = 60 * 60;

/// The network's swinging figures at one moment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Moment {
    pub at: u64,
    pub lost: Vec<Lost>,
    pub busy: Vec<Busy>,
}

/// What one of the modem's downstream channels had lost beyond repair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lost {
    pub kind: String,
    pub channel: Option<u32>,
    pub uncorrectable: u64,
}

impl Lost {
    /// Whether this is the count of that channel: channels are told apart
    /// by kind and number, since an OFDM channel has no number.
    pub fn of(&self, channel: &Channel) -> bool {
        self.kind == channel.kind && self.channel == channel.channel
    }
}

/// How busy one radio was.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Busy {
    pub device: String,
    pub radio: String,
    pub pct: u32,
}

impl Moment {
    /// What a modem's downstream channels say they've lost.
    pub fn lost_of(modem: &Modem) -> Vec<Lost> {
        modem
            .downstream
            .iter()
            .filter_map(|channel| {
                Some(Lost {
                    kind: channel.kind.clone(),
                    channel: channel.channel,
                    uncorrectable: channel.uncorrectable?,
                })
            })
            .collect()
    }

    /// The figures of a record, as it was collected.
    pub fn of(network: &Network) -> Self {
        let lost = network
            .devices
            .iter()
            .filter(|device| device.id == modem::ID)
            .filter_map(|device| device.modem.as_ref())
            .flat_map(Self::lost_of)
            .collect();
        let busy = network
            .devices
            .iter()
            .flat_map(|device| {
                device.radios.iter().filter_map(|radio| {
                    Some(Busy {
                        device: device.id.clone(),
                        radio: radio.name.clone(),
                        pct: radio.busy_pct?,
                    })
                })
            })
            .collect();
        Self {
            at: network.collected_at,
            lost,
            busy,
        }
    }
}

/// The day's moments up to now, oldest first.
pub fn read(file: &Path, now: u64) -> Vec<Moment> {
    let Ok(text) = fs::read_to_string(file) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| serde_json::from_str::<Moment>(line).ok())
        .filter(|moment| moment.at + DAY >= now && moment.at <= now)
        .collect()
}

/// Adds a moment to the day's file, trimming the file to the day once its
/// oldest line is more than an hour past it.
pub fn keep(file: &Path, moment: &Moment) -> io::Result<()> {
    let line = serde_json::to_string(moment).map_err(io::Error::other)? + "\n";
    let oldest = fs::read_to_string(file)
        .ok()
        .and_then(|text| {
            text.lines()
                .find_map(|line| serde_json::from_str::<Moment>(line).ok())
        })
        .map(|first| first.at);
    if oldest.is_some_and(|at| at + DAY + SLACK < moment.at) {
        let mut kept: String = read(file, moment.at)
            .iter()
            .filter_map(|each| serde_json::to_string(each).ok())
            .map(|each| each + "\n")
            .collect();
        kept.push_str(&line);
        let fresh = file.with_extension("jsonl.new");
        fs::write(&fresh, kept)?;
        return fs::rename(fresh, file);
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)?
        .write_all(line.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;

    /// A day file of the test's own, in a folder no other test uses.
    fn day_file(test: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("net-history-{}-{test}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("the test's folder can be made");
        folder.join("day.jsonl")
    }

    fn moment(at: u64, lost: u64) -> Moment {
        Moment {
            at,
            lost: vec![Lost {
                kind: "sc-qam".into(),
                channel: Some(16),
                uncorrectable: lost,
            }],
            busy: vec![Busy {
                device: "eero:2003".into(),
                radio: "5 GHz high".into(),
                pct: 40,
            }],
        }
    }

    #[test]
    fn what_is_kept_is_read_back_in_order() {
        let file = day_file("kept");
        for at in [100, 160, 220] {
            keep(&file, &moment(at, at)).expect("a moment is kept");
        }
        let day = read(&file, 220);
        let ats: Vec<u64> = day.iter().map(|moment| moment.at).collect();
        assert_eq!(ats, [100, 160, 220], "read back: {day:?}");
        assert_eq!(day[0], moment(100, 100), "a moment reads back whole");
    }

    #[test]
    fn only_the_last_day_is_read() {
        let file = day_file("day");
        keep(&file, &moment(1_000, 1)).expect("a moment is kept");
        keep(&file, &moment(1_000 + DAY, 2)).expect("a moment is kept");
        let day = read(&file, 1_001 + DAY);
        assert_eq!(day.len(), 1, "the first is past a day: {day:?}");
        assert_eq!(day[0].at, 1_000 + DAY);
    }

    #[test]
    fn the_file_is_trimmed_once_its_oldest_line_is_an_hour_past_the_day() {
        let file = day_file("trim");
        keep(&file, &moment(1_000, 1)).expect("a moment is kept");
        keep(&file, &moment(1_000 + DAY + SLACK, 2)).expect("a moment is kept");
        let lines = fs::read_to_string(&file)
            .expect("the file reads")
            .lines()
            .count();
        assert_eq!(lines, 2, "an hour past the day isn't trimmed yet");
        keep(&file, &moment(1_001 + DAY + SLACK, 3)).expect("a moment is kept");
        let text = fs::read_to_string(&file).expect("the file reads");
        assert_eq!(text.lines().count(), 2, "the oldest is dropped: {text}");
        assert!(
            !file.with_extension("jsonl.new").exists(),
            "the rewrite leaves nothing beside the file"
        );
    }

    #[test]
    fn a_line_that_does_not_parse_costs_only_itself() {
        let file = day_file("torn");
        keep(&file, &moment(100, 1)).expect("a moment is kept");
        fs::write(
            &file,
            fs::read_to_string(&file).expect("the file reads") + "{\"at\": 16\n",
        )
        .expect("a torn line is written");
        keep(&file, &moment(220, 3)).expect("a moment is kept after it");
        let ats: Vec<u64> = read(&file, 220).iter().map(|moment| moment.at).collect();
        assert_eq!(ats, [100, 220], "the torn line alone is skipped");
    }

    #[test]
    fn no_file_is_an_empty_day() {
        let file = day_file("none");
        assert!(read(&file, 100).is_empty(), "nothing kept yet");
    }
}
