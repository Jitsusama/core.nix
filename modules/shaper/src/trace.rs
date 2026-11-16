//! What the live shaper heard and held each second that the recorder's
//! lines can't say, kept as day files of its own: whose the upload's queue
//! was, a call's round trip through the VPN, and the rate it held the
//! upload to. A replay (src/replay.rs) puts these back into the recorded
//! seconds, so a bite that came through our own queues or the VPN is
//! replayed rather than passed as clean, and the rates the shaper really
//! held are scored beside any a controller would set.
//!
//! One short line a second, about 20 MB a day, kept for as many days as
//! the recorder keeps.

use crate::oneway::Ours;
use crate::record::{day_file, expired};
use crate::replay::{Rates, Stay};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

/// How long, in seconds, a held rate stands for the seconds after it
/// that have no line, as a step that missed a write leaves them.
const STANDS_S: u64 = 5;

/// One second as the live shaper took it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Line {
    /// The recorded second it was stepped on, in seconds since 1970.
    pub at: u64,
    /// The link it holds.
    pub link: String,
    /// What the upload was held to, in kbit/s; none held no limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub up: Option<u64>,
    /// Whose the upload's queue was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ours: Option<Ours>,
    /// A call's round trip through the VPN, and the least it was taken to
    /// be, in ms, to a tenth.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vpn_ms: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vpn_base_ms: Option<f64>,
    /// The plateau the upload holds under, in kbit/s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plateau: Option<u64>,
}

impl Line {
    /// The line for what the controller heard of `link` at the recorded
    /// second `at`.
    pub fn of(at: u64, link: &str, heard: &crate::live::Heard) -> Line {
        let tenth = |ms: f64| (ms * 10.0).round() / 10.0;
        Line {
            at,
            link: link.to_owned(),
            up: heard.up_limit,
            ours: heard.ours,
            vpn_ms: heard.vpn_ms.map(tenth),
            vpn_base_ms: heard.vpn_base_ms.map(tenth),
            plateau: heard.plateau.map(crate::rate::whole),
        }
    }
}

/// Writes a line for each recorded second the shaper steps on, once.
pub struct Writer {
    folder: PathBuf,
    last: Option<u64>,
    day: Option<PathBuf>,
}

impl Writer {
    pub fn new(folder: &Path) -> Writer {
        Writer {
            folder: folder.to_path_buf(),
            last: None,
            day: None,
        }
    }

    /// Writes `line` unless its second was written already. A write that
    /// fails is let go: the trace is for a replay later, and the shaping
    /// mustn't wait on a disk.
    pub fn write(&mut self, line: &Line) {
        if self.last.is_some_and(|last| line.at <= last) {
            return;
        }
        self.last = Some(line.at);
        let path = day_file(&self.folder, UNIX_EPOCH + Duration::from_secs(line.at));
        if self.day.as_ref() != Some(&path) {
            let _ = fs::create_dir_all(&self.folder);
            self.tidy();
            self.day = Some(path.clone());
        }
        let text = serde_json::to_string(line).expect("a trace line serialises");
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(file, "{text}");
        }
    }

    fn tidy(&self) {
        let names: Vec<String> = fs::read_dir(&self.folder)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        for name in expired(&names) {
            let _ = fs::remove_file(self.folder.join(name));
        }
    }
}

/// Every line in a folder's day files, by its second. A line that can't
/// be read is passed over, as the recorder's are.
pub fn read(folder: &Path) -> Result<BTreeMap<u64, Line>, String> {
    let entries = fs::read_dir(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let mut lines = BTreeMap::new();
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "jsonl") {
            continue;
        }
        let file = File::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        for text in BufReader::new(file).lines().map_while(Result::ok) {
            if let Ok(line) = serde_json::from_str::<Line>(&text) {
                lines.insert(line.at, line);
            }
        }
    }
    Ok(lines)
}

/// Puts what the shaper heard back into each stay's seconds, and gives
/// the rates it held at each, in the stays' order: a second with no line
/// holds the last rate for up to `STANDS_S`, and no limit after.
pub fn merge(stays: &mut [Stay], lines: &BTreeMap<u64, Line>) -> Vec<Vec<Rates>> {
    stays
        .iter_mut()
        .map(|stay| {
            let began = stay.began_ms / 1_000;
            let mut last: Option<(u64, Option<u64>)> = None;
            stay.seconds
                .iter_mut()
                .map(|second| {
                    let at = began + crate::rate::whole(second.t_s);
                    let up = match lines.get(&at) {
                        Some(line) => {
                            second.up_ours = line.ours;
                            second.vpn_ms = line.vpn_ms;
                            last = Some((at, line.up));
                            line.up
                        }
                        None => last
                            .filter(|(then, _)| at - then <= STANDS_S)
                            .and_then(|(_, up)| up),
                    };
                    Rates { up, down: None }
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::Second;

    /// Midnight in UTC on 16 November 2026, in seconds.
    const NOV16: u64 = 1_794_787_200;

    fn second(t: u64) -> Second {
        Second {
            t_s: crate::rate::float(t),
            up_kbit: 1_000.0,
            down_kbit: 0.0,
            up_delay_ms: Some(1.0),
            down_delay_ms: None,
            gateway_ms: None,
            up_ours: None,
            vpn_ms: None,
        }
    }

    fn line(at: u64, up: Option<u64>, vpn_ms: Option<f64>) -> Line {
        Line {
            at,
            link: "wlan0".into(),
            up,
            ours: Some(Ours {
                queued: true,
                ..Ours::default()
            }),
            vpn_ms,
            ..Line::default()
        }
    }

    #[test]
    fn a_trace_is_written_once_a_second_and_read_back() {
        let folder = std::env::temp_dir().join(format!("shaper-trace-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        let mut writer = Writer::new(&folder);
        writer.write(&line(NOV16 + 5, Some(10_800), Some(87.0)));
        writer.write(&line(NOV16 + 5, Some(1), None));
        writer.write(&line(NOV16 + 6, None, None));
        let read = read(&folder).expect("the folder reads");
        let _ = fs::remove_dir_all(&folder);
        assert_eq!(
            read.len(),
            2,
            "a second written twice is kept once: {read:?}"
        );
        assert_eq!(
            read[&(NOV16 + 5)].up,
            Some(10_800),
            "the first write stands: {read:?}"
        );
        assert_eq!(
            read[&(NOV16 + 5)].vpn_ms,
            Some(87.0),
            "with the VPN's: {read:?}"
        );
        assert_eq!(
            read[&(NOV16 + 6)].up,
            None,
            "no limit reads back as none: {read:?}"
        );
    }

    #[test]
    fn a_stays_seconds_take_what_the_shaper_heard_and_held() {
        let mut stays = vec![Stay {
            network: "home".into(),
            began_ms: NOV16 * 1_000,
            seconds: (0..12).map(second).collect(),
        }];
        let lines = BTreeMap::from([
            (NOV16 + 1, line(NOV16 + 1, Some(5_000), Some(120.0))),
            (NOV16 + 2, line(NOV16 + 2, Some(4_800), None)),
        ]);
        let held = merge(&mut stays, &lines);
        let ups: Vec<Option<u64>> = held[0].iter().map(|r| r.up).collect();
        let mut want = vec![None, Some(5_000)];
        want.extend([Some(4_800); 6]);
        want.extend([None; 4]);
        assert_eq!(ups, want, "held, then standing 5 s past the last line");
        let s = &stays[0].seconds;
        assert_eq!(s[1].vpn_ms, Some(120.0), "the VPN's round trip: {:?}", s[1]);
        assert!(
            s[2].up_ours.is_some_and(|o| o.queued),
            "our queue: {:?}",
            s[2]
        );
        assert_eq!(
            s[3].up_ours, None,
            "nothing heard, nothing put in: {:?}",
            s[3]
        );
    }
}
