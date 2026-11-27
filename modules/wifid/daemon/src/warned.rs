//! The access point that last warned each card it drops the card, kept
//! while the warning keeps it off, so a wifid started again, as a switch
//! restarts it, doesn't hold a card back to a point about to drop it
//! (`Wifid::holds_to`). Held in memory alone, a warning was forgotten
//! then, and the hand-over at start held the card to the point it was on.
//!
//! It's kept in wifid's runtime folder, which outlives a restart but not
//! the machine's: which point a card was on says where the machine was,
//! which wifid never keeps on its disk (seen.rs). Each warning is kept
//! by the wall clock, as an `Instant` means nothing to the next wifid,
//! and is read back as the time it has left.
//!
//! A warning the chooser forgot as its card came back to the point
//! (`State::warned_off`) is still kept here, so a wifid started again
//! meanwhile keeps the chooser off that point once the card leaves it,
//! until the keep-off ends, as the moves keep off it with no restart
//! (`Arrivals::drops`).

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::time::{Duration, Instant};

/// The longest a warning keeps its point off, so a kept one lasting
/// longer was kept by a clock since set back.
const LONGEST: Duration = crate::transition::KEEPS_OFF_AT_MOST;

/// A point's warning, as kept: the point, and until when it's kept off,
/// in milliseconds since 1970.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct Kept {
    point: String,
    until: u64,
}

/// The last warning each card was given, by the card's name.
pub struct Warned {
    file: PathBuf,
    kept: BTreeMap<String, Kept>,
}

impl Warned {
    /// The warnings kept in this file. One that can't be read is begun
    /// again empty, which forgets what a restart forgot before.
    pub fn kept_in(file: &Path) -> Warned {
        let kept = fs::read(file).ok();
        let kept = kept.and_then(|kept| serde_json::from_slice(&kept).ok());
        Warned {
            file: file.to_path_buf(),
            kept: kept.unwrap_or_default(),
        }
    }

    /// Each card's warning still keeping its point off, as the card, the
    /// point and until when, `now` being `wall` on the wall clock.
    pub fn lasting(&self, now: Instant, wall: SystemTime) -> Vec<(String, String, Instant)> {
        self.kept
            .iter()
            .filter_map(|(card, kept)| {
                let until = instant(kept.until, now, wall)?;
                Some((card.clone(), kept.point.clone(), until))
            })
            .collect()
    }

    /// Keeps that the point warned the card it drops it, kept off until
    /// then, in place of the card's warning before, and forgets every
    /// warning that has run out, `now` being `wall` on the wall clock.
    pub fn warned(
        &mut self,
        card: &str,
        point: &str,
        until: Instant,
        (now, wall): (Instant, SystemTime),
    ) -> io::Result<()> {
        let since = millis(wall);
        self.kept.retain(|_, kept| kept.until > since);
        let kept = Kept {
            point: point.to_owned(),
            until: since.saturating_add(millis_of(until.saturating_duration_since(now))),
        };
        self.kept.insert(card.to_owned(), kept);
        self.keep()
    }

    /// Writes the warnings whole or not at all, readable by wifid alone.
    fn keep(&self) -> io::Result<()> {
        let partial = self.file.with_extension("partial");
        let mut written = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&partial)?;
        written.write_all(&serde_json::to_vec(&self.kept)?)?;
        fs::set_permissions(&partial, fs::Permissions::from_mode(0o600))?;
        fs::rename(partial, &self.file)
    }
}

/// Milliseconds since 1970 at `wall`, none for a clock set before it.
fn millis(wall: SystemTime) -> u64 {
    millis_of(wall.duration_since(UNIX_EPOCH).unwrap_or_default())
}

/// A duration in whole milliseconds, as many as a `u64` holds.
fn millis_of(lasts: Duration) -> u64 {
    u64::try_from(lasts.as_millis()).unwrap_or(u64::MAX)
}

/// The `Instant` a time kept in milliseconds since 1970 falls on, `now`
/// being `wall`, if it's still to come. A kept time far past any keep-off
/// is a clock set back since, and is taken as run out rather than kept
/// for its whole jump.
fn instant(until: u64, now: Instant, wall: SystemTime) -> Option<Instant> {
    let left = until.checked_sub(millis(wall)).filter(|&left| left > 0)?;
    let left = Duration::from_millis(left);
    (left <= LONGEST).then(|| now + left)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DESK: &str = "6c:ae:f6:c1:84:26";
    const HALL: &str = "6c:ae:f6:c1:84:28";

    fn wall(seconds: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(seconds)
    }

    #[test]
    fn a_warning_is_kept_for_the_next_wifid_as_the_time_it_has_left() {
        let folder = tempfile::tempdir().expect("a folder");
        let file = folder.path().join("warned");
        let (then, now) = (Instant::now(), Instant::now() + Duration::from_secs(3));
        let start = wall(1_795_700_000);
        let ten = Duration::from_secs(600);
        Warned::kept_in(&file)
            .warned("wlan2", DESK, then + ten, (then, start))
            .expect("kept");

        // The next wifid starts 4 s on by the wall clock, its own clock
        // begun anew.
        let lasting = Warned::kept_in(&file).lasting(now, start + Duration::from_secs(4));

        assert_eq!(
            lasting,
            vec![(
                "wlan2".to_owned(),
                DESK.to_owned(),
                now + ten - Duration::from_secs(4)
            )],
            "a warning wasn't read back as the time it had left"
        );
        assert_eq!(
            fs::metadata(&file).expect("a file").permissions().mode() & 0o777,
            0o600,
            "the points a card was warned by were readable past wifid"
        );
    }

    #[test]
    fn a_warning_run_out_is_forgotten() {
        let folder = tempfile::tempdir().expect("a folder");
        let file = folder.path().join("warned");
        let now = Instant::now();
        let start = wall(1_795_700_000);
        let mut warned = Warned::kept_in(&file);
        warned
            .warned("wlan2", DESK, now + Duration::from_secs(5), (now, start))
            .expect("kept");

        let later = start + Duration::from_secs(5);
        warned
            .warned("wlan3", HALL, now + Duration::from_secs(60), (now, later))
            .expect("kept");

        assert_eq!(
            Warned::kept_in(&file).lasting(now, later),
            vec![(
                "wlan3".to_owned(),
                HALL.to_owned(),
                now + Duration::from_secs(60)
            )],
            "a warning that ran out was kept, or one lasting was lost"
        );
        assert!(
            !fs::read_to_string(&file).expect("a file").contains(DESK),
            "a point whose warning ran out stayed on the disk"
        );
    }

    #[test]
    fn a_card_keeps_its_last_warning_alone() {
        let folder = tempfile::tempdir().expect("a folder");
        let mut warned = Warned::kept_in(&folder.path().join("warned"));
        let (now, start) = (Instant::now(), wall(1_795_700_000));
        let until = now + Duration::from_secs(600);
        warned
            .warned("wlan2", DESK, until, (now, start))
            .expect("kept");
        warned
            .warned("wlan2", HALL, until, (now, start))
            .expect("kept");

        assert_eq!(
            warned.lasting(now, start),
            vec![("wlan2".to_owned(), HALL.to_owned(), until)],
            "a card kept a warning another had replaced"
        );
    }

    #[test]
    fn a_kept_warning_past_any_keep_off_is_a_clock_set_back() {
        let now = Instant::now();
        let start = wall(1_795_700_000);
        let most = millis(start) + millis_of(LONGEST);

        assert_eq!(
            instant(most, now, start),
            Some(now + LONGEST),
            "a warning lasting as long as one may was dropped"
        );
        assert_eq!(
            instant(most + 1, now, start),
            None,
            "a warning kept by a clock since set back kept its point off past any keep-off"
        );
        assert_eq!(
            instant(millis(start), now, start),
            None,
            "a warning run out as it's read kept its point off"
        );
    }

    #[test]
    fn a_file_that_cant_be_read_keeps_nothing() {
        let folder = tempfile::tempdir().expect("a folder");
        let file = folder.path().join("warned");
        fs::write(&file, "not what wifid writes").expect("a file");

        let warned = Warned::kept_in(&file);

        assert!(
            warned
                .lasting(Instant::now(), wall(1_795_700_000))
                .is_empty(),
            "a file wifid didn't write kept a point off"
        );
    }
}
