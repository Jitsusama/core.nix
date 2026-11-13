//! Every key's time to the screen, from the launcher's trials. The shell
//! keeps each opening as a line of launcher-trials.jsonl, in a folder of
//! its state named for its config, and since 12 November with `presented`:
//! each key's milliseconds from the press to niri showing it. Pooled over
//! the day they give its exact 99th, where the journal's batches give only
//! a ceiling (`launcher.rs` says why).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use jiff::tz::TimeZone;
use serde::Deserialize;

use crate::follow::date_of;
use crate::launcher::Timed;

/// Where the shells keep a folder for each config, under the account's
/// state.
pub const SHELLS: &str = "quickshell/by-shell";

/// The file each keeps its trials in.
const TRIALS: &str = "launcher-trials.jsonl";

/// As much of an opening as the pool takes: when it opened, in
/// milliseconds since the epoch, and each key's time to the screen.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Trial {
    at: i64,
    presented: Vec<u64>,
}

/// Reads the shells' trials as they grow.
#[derive(Debug)]
pub struct Reader {
    shells: PathBuf,
    /// The day the files were last read on.
    date: String,
    /// Each file's length and last change as it was read then.
    seen: BTreeMap<PathBuf, (u64, Option<SystemTime>)>,
    /// When the last opening pooled opened, so each is pooled once though
    /// the shell writes its file again as it starts, to let the oldest go.
    pooled_to: i64,
}

impl Reader {
    /// A reader of the trials in each folder of `shells`.
    #[must_use]
    pub fn new(shells: &Path) -> Self {
        Self {
            shells: shells.to_owned(),
            date: String::new(),
            seen: BTreeMap::new(),
            pooled_to: 0,
        }
    }

    /// Pool into `keys` each key of every opening on `date` in `zone` not
    /// pooled yet, reading a file again only once it changed, and saying
    /// whether any was. A file last changed before the day holds none.
    pub fn pool(&mut self, keys: &mut Timed, date: &str, zone: &TimeZone) -> bool {
        if self.date != date {
            date.clone_into(&mut self.date);
            self.seen.clear();
        }
        let Ok(folders) = fs::read_dir(&self.shells) else {
            return false;
        };
        let mut trials = Vec::new();
        for path in folders
            .filter_map(Result::ok)
            .map(|folder| folder.path().join(TRIALS))
        {
            let Ok(file) = fs::metadata(&path) else {
                continue;
            };
            let looked = (file.len(), file.modified().ok());
            if self.seen.get(&path) == Some(&looked) {
                continue;
            }
            let before = looked
                .1
                .and_then(|changed| changed.duration_since(UNIX_EPOCH).ok())
                .and_then(|since| i64::try_from(since.as_micros()).ok())
                .is_some_and(|changed| date_of(changed, zone).as_str() < date);
            if !before && let Ok(text) = fs::read_to_string(&path) {
                trials.extend(
                    text.lines()
                        .filter_map(|line| serde_json::from_str::<Trial>(line).ok())
                        .filter(|trial| date_of(trial.at * 1000, zone) == date),
                );
            }
            self.seen.insert(path, looked);
        }
        trials.sort_by_key(|trial| trial.at);
        let mut any = false;
        for trial in trials {
            if trial.at <= self.pooled_to {
                continue;
            }
            self.pooled_to = trial.at;
            for took in trial.presented {
                keys.pool(took);
                any = true;
            }
        }
        any
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A trial's line as the shell writes it, with more than the pool takes.
    fn line(at: i64, presented: &[u64]) -> String {
        format!(
            "{{\"at\":{at},\"home\":\"today\",\"keys\":{{\"letters\":3}},\"presented\":{},\
             \"unpresented\":0}}\n",
            serde_json::to_string(presented).expect("times as JSON")
        )
    }

    /// Noon on 12 November in UTC, in milliseconds.
    const NOON: i64 = 1_794_484_800_000;

    #[test]
    fn every_key_of_the_days_openings_is_pooled_once() {
        let shells = tempfile::tempdir().expect("a folder for the shells");
        let (old, new) = (shells.path().join("old"), shells.path().join("new"));
        fs::create_dir_all(&old).expect("the old shell's folder");
        fs::create_dir_all(&new).expect("the new shell's folder");
        let day = 86_400_000;
        let before = line(NOON - day, &[900]);
        let morning = line(NOON, &[38, 41]) + &line(NOON + 1, &[]);
        fs::write(old.join(TRIALS), before.clone() + &morning).expect("the old shell's trials");
        fs::write(new.join(TRIALS), line(NOON + 2, &[52, 36]) + "{\"at\":").expect("its trials");
        let mut reader = Reader::new(shells.path());
        let mut keys = Timed::default();
        assert!(
            reader.pool(&mut keys, "2026-11-12", &TimeZone::UTC),
            "the day's keys weren't pooled"
        );
        assert_eq!(
            keys.pooled.keys().copied().collect::<Vec<_>>(),
            [36, 38, 41, 52],
            "a key of another day, or a line half written, was pooled, or one was missed"
        );
        fs::write(old.join(TRIALS), morning + &line(NOON + 3, &[60])).expect("a rewrite");
        assert!(
            reader.pool(&mut keys, "2026-11-12", &TimeZone::UTC),
            "an opening added as the file was written again wasn't pooled"
        );
        assert_eq!(
            keys.pooled_count(),
            5,
            "an opening pooled before was pooled again when its file was written again"
        );
        assert!(
            !reader.pool(&mut keys, "2026-11-12", &TimeZone::UTC),
            "a file that hadn't changed was read again"
        );
        let mut tomorrow = Timed::default();
        fs::write(new.join(TRIALS), line(NOON + day, &[70])).expect("tomorrow's trials");
        reader.pool(&mut tomorrow, "2026-11-13", &TimeZone::UTC);
        assert_eq!(
            tomorrow.pooled_count(),
            1,
            "the next day's keys weren't pooled into it"
        );
    }
}
