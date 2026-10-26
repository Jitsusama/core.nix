//! Which logs have settled, and the name each is copied under.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A settled log, and the object it is copied as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settled {
    pub path: PathBuf,
    /// `<host>/<folder>/<log>.<last write>.jsonl.zst.age`.
    pub name: String,
    pub bytes: u64,
    /// Its last write, in seconds since the epoch, so the oldest go first.
    pub written: u64,
}

/// Every log under `sessions` unwritten for `settled` as of `now`, oldest
/// first. A log is a `.jsonl` file in pi's sessions folder or a folder in
/// it; anything else is left alone.
pub fn settled(
    sessions: &Path,
    host: &str,
    settled: Duration,
    now: SystemTime,
) -> Result<Vec<Settled>, String> {
    let mut found = Vec::new();
    for (folder, path) in logs(sessions)? {
        // Removed since it was listed: there is nothing to copy.
        let Ok(meta) = fs::metadata(&path) else {
            continue;
        };
        let Ok(modified) = meta.modified() else {
            continue;
        };
        if now.duration_since(modified).unwrap_or_default() < settled {
            continue;
        }
        let written = modified
            .duration_since(UNIX_EPOCH)
            .map(|since| since.as_secs())
            .unwrap_or_default();
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let name = match &folder {
            Some(folder) => format!("{host}/{folder}/{stem}.{}.jsonl.zst.age", stamp(written)),
            None => format!("{host}/{stem}.{}.jsonl.zst.age", stamp(written)),
        };
        found.push(Settled {
            path,
            name,
            bytes: meta.len(),
            written,
        });
    }
    found.sort_by(|a, b| a.written.cmp(&b.written).then_with(|| a.name.cmp(&b.name)));
    Ok(found)
}

/// The logs in `sessions` and one folder down, each with its folder's name.
fn logs(sessions: &Path) -> Result<Vec<(Option<String>, PathBuf)>, String> {
    let entries = fs::read_dir(sessions)
        .map_err(|why| format!("can't read the sessions in {}: {why}", sessions.display()))?;
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let Some(folder) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let Ok(inner) = fs::read_dir(&path) else {
                // A folder that can't be read is skipped, not the run.
                continue;
            };
            for log in inner.flatten() {
                if is_log(&log.path()) {
                    found.push((Some(folder.clone()), log.path()));
                }
            }
        } else if is_log(&path) {
            found.push((None, path));
        }
    }
    Ok(found)
}

fn is_log(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .is_some_and(|extension| extension == "jsonl")
}

/// A time as UTC, `20261026T063412Z`, which sorts as it reads.
pub fn stamp(seconds: u64) -> String {
    let days = seconds / 86_400;
    let rest = seconds % 86_400;
    let (year, month, day) = civil(days);
    format!(
        "{year:04}{month:02}{day:02}T{:02}{:02}{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

/// The calendar date `days` after 1970-01-01, by Howard Hinnant's
/// days-from-civil inverse.
fn civil(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + u64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamps_in_utc() {
        assert_eq!(stamp(0), "19700101T000000Z");
        // 2026-10-26 06:34:12 UTC.
        assert_eq!(stamp(1_792_996_452), "20261026T063412Z");
        // A leap day.
        assert_eq!(stamp(951_782_400), "20000229T000000Z");
    }
}
