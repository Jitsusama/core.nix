//! Each day's look at the home network, written for a review to read:
//! `day/<date>.json` in eero's state folder, the date in the machine's own
//! zone, rewritten whole after each of the probe's looks, so the file holds
//! the day so far and a reader never sees half of it. The figures are the
//! history's (history/day.rs); the files go back as far as the history
//! does.

use std::fs;
use std::path::Path;

use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;

use crate::history::{History, KEPT_S};

const DAY_S: i64 = 24 * 60 * 60;

/// Writes the day `now` falls in, in `zone`, to `folder`, and drops the
/// files of days the history no longer keeps.
pub fn write(history: &History, now: i64, zone: &TimeZone, folder: &Path) -> Result<(), String> {
    let date = date_of(now, zone)?;
    let start = |date: Date| {
        date.to_zoned(zone.clone())
            .map(|start| start.timestamp().as_second())
            .map_err(|e| e.to_string())
    };
    let from = start(date)?;
    let to = start(date.tomorrow().map_err(|e| e.to_string())?)?;
    let said = date.to_string();
    let day = history.day(&said, from, to)?;
    let text = serde_json::to_string_pretty(&day).map_err(|e| e.to_string())? + "\n";
    fs::create_dir_all(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let file = folder.join(format!("{said}.json"));
    let writing = folder.join(format!(".{said}.json.writing"));
    fs::write(&writing, text).map_err(|e| format!("{}: {e}", writing.display()))?;
    fs::rename(&writing, &file).map_err(|e| format!("{}: {e}", file.display()))?;
    prune(folder, &date_of(now - KEPT_S - DAY_S, zone)?.to_string())
}

fn date_of(at: i64, zone: &TimeZone) -> Result<Date, String> {
    Timestamp::from_second(at)
        .map(|at| at.to_zoned(zone.clone()).date())
        .map_err(|e| e.to_string())
}

/// Drops each day's file from before `oldest`, a date said as its file
/// names it, which sorts as the days do.
fn prune(folder: &Path, oldest: &str) -> Result<(), String> {
    let listed = fs::read_dir(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    for entry in listed.flatten() {
        let name = entry.file_name();
        let Some(date) = name.to_str().and_then(|name| name.strip_suffix(".json")) else {
            continue;
        };
        if date.len() == oldest.len() && date < oldest {
            fs::remove_file(entry.path()).map_err(|e| format!("{date}: {e}"))?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "day_tests.rs"]
mod tests;
