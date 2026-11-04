//! The one-way controller (src/oneway.rs), shaping as the recorder writes.
//! The recorder (src/record.rs) writes each probe's one-way delays to a day
//! file; the shaper reads each line the file gains, makes seconds of them
//! as a replay does (src/replay.rs), and steps the controller on each, so
//! the rates that shape a link are the ones a replay of its lines scores.

use crate::oneway;
use crate::record::day_file;
use crate::replay::{Controller as _, Rates, Reading, Sample, parse};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// How long the recorder may write nothing, in ms, before its link is let
/// go unlimited: with no signal, the controller has nothing to hold a rate
/// to, and a rate held from a minute ago may be far under the line's now.
const SILENT_MS: u64 = 10_000;

/// What the recorder's lines have said: the link it records, and the
/// controller stepped on each second they finished.
pub struct Follow {
    link: Option<String>,
    reading: Reading,
    controller: oneway::Controller,
    rates: Rates,
    /// When the latest line was written, in ms since 1970.
    last_ms: u64,
}

impl Follow {
    pub fn new() -> Follow {
        Follow {
            link: None,
            reading: Reading::new("unknown".into()),
            controller: oneway::Controller::new(None, None),
            rates: Rates::default(),
            last_ms: 0,
        }
    }

    /// Takes one line's sample. A recording begun anew is a new stay, met
    /// as a network never measured, as a replay meets it.
    pub fn take(&mut self, sample: &Sample) {
        self.last_ms = self.last_ms.max(sample.at());
        if let Sample::Pool { network, link, .. } = sample {
            if link.is_some() {
                self.link.clone_from(link);
            }
            if let Some(network) = network {
                self.reading = Reading::new(network.clone());
                self.controller = oneway::Controller::new(None, None);
                self.rates = Rates::default();
                return;
            }
        }
        self.reading.take(sample);
        for second in self.reading.drain() {
            self.rates = self.controller.second(&second);
        }
    }

    /// The link the recorder follows and the rates to hold it to, at a
    /// moment in ms since 1970: no limit once the recorder has gone silent.
    pub fn now(&self, at_ms: u64) -> Option<(&str, Rates)> {
        let link = self.link.as_deref()?;
        if at_ms.saturating_sub(self.last_ms) > SILENT_MS {
            return Some((link, Rates::default()));
        }
        Some((link, self.rates))
    }
}

/// The recorder's day files, read as they grow.
pub struct Live {
    folder: PathBuf,
    /// The day file being read, and how far into it.
    file: (PathBuf, u64),
    /// A line the recorder is still writing.
    partial: Vec<u8>,
    follow: Follow,
}

impl Live {
    /// Starts at yesterday's file, so a stay begun before midnight is known
    /// from its first line.
    pub fn new(folder: &Path, now: SystemTime) -> Live {
        let yesterday = now.checked_sub(Duration::from_hours(24)).unwrap_or(now);
        Live {
            folder: folder.to_path_buf(),
            file: (day_file(folder, yesterday), 0),
            partial: Vec::new(),
            follow: Follow::new(),
        }
    }

    /// Reads what the recorder wrote since, and answers the link it
    /// follows and the rates to hold it to. A day's file is read to its end
    /// before the next day's is begun.
    pub fn poll(&mut self, now: SystemTime) -> Option<(&str, Rates)> {
        let today = day_file(&self.folder, now);
        self.read_on();
        if self.file.0 != today {
            self.file = (today, 0);
            self.partial.clear();
            self.read_on();
        }
        self.follow.now(ms(now))
    }

    /// Takes each whole line the file gained, keeping one half written.
    fn read_on(&mut self) {
        let Ok(mut file) = File::open(&self.file.0) else {
            return;
        };
        if file.seek(SeekFrom::Start(self.file.1)).is_err() {
            return;
        }
        let mut reader = BufReader::new(file);
        while let Ok(read) = reader.read_until(b'\n', &mut self.partial) {
            if read == 0 || self.partial.last() != Some(&b'\n') {
                self.file.1 += u64::try_from(read).unwrap_or(0);
                break;
            }
            self.file.1 += u64::try_from(read).unwrap_or(0);
            if let Some(sample) = parse(&String::from_utf8_lossy(&self.partial)) {
                self.follow.take(&sample);
            }
            self.partial.clear();
        }
    }
}

fn ms(at: SystemTime) -> u64 {
    u64::try_from(
        at.duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    )
    .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay;
    use serde_json::json;
    use std::fs::{self, OpenOptions};
    use std::io::Write;

    /// 3 November 2026, 12:00 UTC, in ms since 1970.
    const NOV3: u64 = 1_793_707_200_000;

    fn pool(at: u64, network: Option<&str>) -> String {
        let mut line = json!({"at": at, "pool": [["Quad9", "9.9.9.9"]], "operators": 1,
                              "enough": false, "link": "wlan0"});
        if let Some(network) = network {
            line["network"] = json!(network);
        }
        line.to_string()
    }

    fn stamp(at: u64, back: f64, rx: u64) -> String {
        json!({"at": at, "reflector": "9.9.9.9", "operator": "Quad9", "out": 10.0,
               "back": back, "rtt": 20.0, "rx": rx, "tx": at - NOV3})
        .to_string()
    }

    /// Six seconds of a 200 Mbit/s download, probed every 100 ms, whose
    /// own delay rises 40 ms after two.
    fn queued() -> Vec<String> {
        let mut lines = vec![pool(NOV3, Some("home"))];
        for i in 0..60_u64 {
            let back = if i < 20 { 10.0 } else { 50.0 };
            lines.push(stamp(NOV3 + 100 * (i + 1), back, 2_500_000 * (i + 1)));
        }
        lines
    }

    fn followed(lines: &[String]) -> Follow {
        let mut follow = Follow::new();
        for line in lines {
            follow.take(&parse(line).expect("a sample"));
        }
        follow
    }

    #[test]
    fn a_queue_standing_on_the_recorders_link_is_cut_live() {
        let follow = followed(&queued());
        let (link, rates) = follow.now(NOV3 + 6_000).expect("a link");
        assert_eq!(link, "wlan0");
        assert!(rates.down.is_some_and(|d| d < 200_000), "{rates:?}");
        assert_eq!(rates.up, None);
    }

    #[test]
    fn the_live_controller_answers_what_a_replay_of_the_same_lines_does() {
        let lines = queued();
        let stays = replay::stays(lines.iter().filter_map(|l| parse(l)));
        let mut controller = oneway::Controller::new(None, None);
        let mut replayed = Rates::default();
        for second in &stays[0].seconds {
            replayed = controller.second(second);
        }
        assert!(
            replayed.down.is_some(),
            "the replay cuts nothing to compare"
        );
        let follow = followed(&lines);
        assert_eq!(follow.now(NOV3 + 6_000).map(|(_, r)| r), Some(replayed));
    }

    #[test]
    fn a_silent_recorder_lets_its_link_go() {
        let follow = followed(&queued());
        assert_eq!(
            follow.now(NOV3 + 6_000 + 11_000),
            Some(("wlan0", Rates::default()))
        );
    }

    #[test]
    fn a_recording_begun_anew_starts_unlimited() {
        let mut lines = queued();
        lines.push(pool(NOV3 + 6_100, Some("hotel")));
        let follow = followed(&lines);
        assert_eq!(follow.now(NOV3 + 6_200), Some(("wlan0", Rates::default())));
    }

    #[test]
    fn an_hourly_pool_names_the_link_to_a_shaper_started_mid_stay() {
        let mut lines = vec![pool(NOV3, None)];
        lines.extend(queued().into_iter().skip(1));
        let follow = followed(&lines);
        assert!(
            follow
                .now(NOV3 + 6_000)
                .is_some_and(|(l, r)| l == "wlan0" && r.down.is_some())
        );
    }

    #[test]
    fn the_day_files_are_read_as_they_grow_and_across_midnight() {
        let folder = std::env::temp_dir().join(format!("shaper-live-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).expect("a folder");
        let lines = queued();
        let today = SystemTime::UNIX_EPOCH + Duration::from_millis(NOV3 + 6_000);
        let yesterday = today - Duration::from_hours(24);
        // The stay began yesterday; today's file holds its seconds, the
        // last line still half written.
        fs::write(
            crate::record::day_file(&folder, yesterday),
            format!("{}\n", lines[0]),
        )
        .expect("yesterday's file");
        let (last, rest) = lines[1..].split_last().expect("lines");
        let (half, other_half) = last.split_at(last.len() / 2);
        let path = crate::record::day_file(&folder, today);
        fs::write(&path, format!("{}\n{half}", rest.join("\n"))).expect("today's file");
        let mut live = Live::new(&folder, today);
        let first = live.poll(today).map(|(l, r)| (l.to_string(), r));
        assert!(
            first
                .as_ref()
                .is_some_and(|(l, r)| l == "wlan0" && r.down.is_some()),
            "{first:?}"
        );
        let mut file = OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("today's file");
        writeln!(file, "{other_half}").expect("the rest");
        let _ = live.poll(today);
        assert_eq!(
            live.follow.now(NOV3 + 6_000).map(|(_, r)| r),
            followed(&lines).now(NOV3 + 6_000).map(|(_, r)| r)
        );
        assert_eq!(live.follow.last_ms, NOV3 + 6_000);
        let _ = fs::remove_dir_all(&folder);
    }
}
