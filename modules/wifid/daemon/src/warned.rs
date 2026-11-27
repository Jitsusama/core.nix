//! The access point that last warned each card it drops the card, kept
//! past a restart, so a wifid started again, as a switch restarts it,
//! doesn't hold a card back to a point about to drop it
//! (`Wifid::holds_to`). Held in memory alone, a warning was forgotten
//! then, and the hand-over at start held the card to the point it was on.
//!
//! It's kept in wifid's runtime folder, which outlives a restart but not
//! a boot: no warning keeps its point off past an hour, so one from
//! before a boot would be long run out, and nothing need clear it. Each
//! is kept by the boot clock (`Boot`), which the folder never outlives,
//! as an `Instant` means nothing to the next wifid, and is read back as
//! the time it has left. A warning run out stays in the file until the
//! next is kept, but is never read back.
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

use serde::{Deserialize, Serialize};
use tokio::time::{Duration, Instant};

/// The longest a warning keeps its point off, so a kept one lasting
/// longer wasn't kept by wifid.
const LONGEST: Duration = crate::transition::KEEPS_OFF_AT_MOST;

/// A moment on the boot clock, which counts from the machine's start,
/// its suspends included, and which nothing sets. The wall clock could
/// be set back or on between a warning kept and read, and a step either
/// way took a warning as lasting longer or shorter than it said.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Boot(Duration);

impl Boot {
    /// Now, on the boot clock.
    pub fn now() -> io::Result<Boot> {
        let mut now = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // SAFETY: the call writes only the time it's handed, which
        // outlives it.
        let read = unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &raw mut now) };
        if read != 0 {
            return Err(io::Error::last_os_error());
        }
        let seconds = u64::try_from(now.tv_sec).map_err(io::Error::other)?;
        let nanos = u32::try_from(now.tv_nsec).map_err(io::Error::other)?;
        Ok(Boot(Duration::new(seconds, nanos)))
    }

    /// Milliseconds since the machine started.
    fn millis(self) -> u64 {
        millis_of(self.0)
    }
}

/// A point's warning, as kept: the point, and until when it's kept off,
/// in milliseconds on the boot clock. Named apart from the wall-clock
/// time a wifid before kept, so a file of those is read as not wifid's.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct Kept {
    point: String,
    ends: u64,
}

/// The last warning each card was given, by the card's name.
pub struct Warned {
    file: PathBuf,
    kept: BTreeMap<String, Kept>,
}

impl Warned {
    /// None kept yet, to be kept in this file.
    pub fn none(file: &Path) -> Warned {
        Warned {
            file: file.to_path_buf(),
            kept: BTreeMap::new(),
        }
    }

    /// The warnings kept in this file, `now` on the boot clock, and what
    /// was forgotten of them, for the journal: a file that can't be read
    /// is begun again empty, and a warning that would outlast any
    /// keep-off is dropped, as wifid wrote neither. No file says nothing,
    /// as no card was warned since the boot.
    pub fn kept_in(file: &Path, now: Boot) -> (Warned, Vec<String>) {
        let mut warned = Warned::none(file);
        let read = match fs::read(file) {
            Ok(read) => read,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return (warned, Vec::new()),
            Err(error) => {
                let said = format!("forgets the warnings kept before, as it can't read them: {error}");
                return (warned, vec![said]);
            }
        };
        let kept: BTreeMap<String, Kept> = match serde_json::from_slice(&read) {
            Ok(kept) => kept,
            Err(error) => {
                let said = format!(
                    "forgets the warnings kept before, as they aren't as it keeps them: {error}"
                );
                return (warned, vec![said]);
            }
        };
        let most = now.millis().saturating_add(millis_of(LONGEST));
        let mut forgotten = Vec::new();
        for (card, kept) in kept {
            if kept.ends > most {
                forgotten.push(format!(
                    "forgets {card}'s warning from {}, as it would outlast any keep-off",
                    kept.point
                ));
            } else {
                warned.kept.insert(card, kept);
            }
        }
        (warned, forgotten)
    }

    /// Each card's warning still keeping its point off, as the card, the
    /// point and until when, `now` being `boot` on the boot clock.
    pub fn lasting(&self, now: Instant, boot: Boot) -> Vec<(String, String, Instant)> {
        self.kept
            .iter()
            .filter_map(|(card, kept)| {
                let until = instant(kept.ends, now, boot)?;
                Some((card.clone(), kept.point.clone(), until))
            })
            .collect()
    }

    /// Keeps that the point warned the card it drops it, kept off until
    /// then, in place of the card's warning before, and forgets every
    /// warning that has run out, `now` being `boot` on the boot clock.
    pub fn warned(
        &mut self,
        card: &str,
        point: &str,
        until: Instant,
        (now, boot): (Instant, Boot),
    ) -> io::Result<()> {
        let since = boot.millis();
        self.kept.retain(|_, kept| kept.ends > since);
        let kept = Kept {
            point: point.to_owned(),
            ends: since.saturating_add(millis_of(until.saturating_duration_since(now))),
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

/// A duration in whole milliseconds, as many as a `u64` holds.
fn millis_of(lasts: Duration) -> u64 {
    u64::try_from(lasts.as_millis()).unwrap_or(u64::MAX)
}

/// The `Instant` a time kept in milliseconds on the boot clock falls on,
/// `now` being `boot`, if it's still to come.
fn instant(ends: u64, now: Instant, boot: Boot) -> Option<Instant> {
    let left = ends.checked_sub(boot.millis()).filter(|&left| left > 0)?;
    Some(now + Duration::from_millis(left))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DESK: &str = "6c:ae:f6:c1:84:26";
    const HALL: &str = "6c:ae:f6:c1:84:28";

    fn boot(seconds: u64) -> Boot {
        Boot(Duration::from_secs(seconds))
    }

    #[test]
    fn a_warning_is_kept_for_the_next_wifid_as_the_time_it_has_left() {
        let folder = tempfile::tempdir().expect("a folder");
        let file = folder.path().join("warned");
        let (warned_at, read_at) = (Instant::now(), Instant::now() + Duration::from_secs(3));
        let start = boot(86_400);
        let ten = Duration::from_secs(600);
        Warned::none(&file)
            .warned("wlan2", DESK, warned_at + ten, (warned_at, start))
            .expect("kept");

        // The next wifid starts 4 s on by the boot clock, its own clock
        // begun anew.
        let (warned, forgotten) = Warned::kept_in(&file, start);
        let lasting = warned.lasting(read_at, boot(86_404));

        assert_eq!(
            lasting,
            vec![(
                "wlan2".to_owned(),
                DESK.to_owned(),
                read_at + ten - Duration::from_secs(4)
            )],
            "a warning wasn't read back as the time it had left"
        );
        assert!(
            forgotten.is_empty(),
            "a warning kept was said forgotten: {forgotten:?}"
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
        let mut warned = Warned::none(&file);
        warned
            .warned("wlan2", DESK, now + Duration::from_secs(5), (now, boot(600)))
            .expect("kept");

        let later = boot(605);
        warned
            .warned("wlan3", HALL, now + Duration::from_secs(60), (now, later))
            .expect("kept");

        assert_eq!(
            Warned::kept_in(&file, later).0.lasting(now, later),
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
        let mut warned = Warned::none(&folder.path().join("warned"));
        let (now, start) = (Instant::now(), boot(600));
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
    fn a_kept_warning_past_any_keep_off_is_said_and_forgotten() {
        let folder = tempfile::tempdir().expect("a folder");
        let file = folder.path().join("warned");
        let (now, start) = (Instant::now(), boot(600));
        let most = start.millis() + millis_of(LONGEST);
        let kept = BTreeMap::from([
            (
                "wlan2",
                Kept {
                    point: DESK.to_owned(),
                    ends: most + 1,
                },
            ),
            (
                "wlan3",
                Kept {
                    point: HALL.to_owned(),
                    ends: most,
                },
            ),
        ]);
        fs::write(&file, serde_json::to_vec(&kept).expect("json")).expect("a file");

        let (warned, forgotten) = Warned::kept_in(&file, start);

        assert_eq!(
            warned.lasting(now, start),
            vec![("wlan3".to_owned(), HALL.to_owned(), now + LONGEST)],
            "a warning lasting as long as one may was dropped, or one past it kept"
        );
        assert_eq!(
            forgotten,
            vec![format!(
                "forgets wlan2's warning from {DESK}, as it would outlast any keep-off"
            )],
            "a warning dropped as past any keep-off wasn't said"
        );
        assert_eq!(
            instant(start.millis(), now, start),
            None,
            "a warning run out as it's read kept its point off"
        );
    }

    #[test]
    fn a_file_kept_by_the_wall_clock_is_said_forgotten_and_replaced() {
        let folder = tempfile::tempdir().expect("a folder");
        let file = folder.path().join("warned");
        // As a wifid before this one kept it, in milliseconds since 1970.
        fs::write(
            &file,
            format!(r#"{{"wlan2":{{"point":"{DESK}","until":1795700000000}}}}"#),
        )
        .expect("a file");
        let (now, start) = (Instant::now(), boot(600));

        let (mut warned, forgotten) = Warned::kept_in(&file, start);

        assert!(
            warned.lasting(now, start).is_empty(),
            "a file wifid didn't write this way kept a point off"
        );
        assert!(
            forgotten.len() == 1 && forgotten[0].starts_with("forgets the warnings kept before"),
            "the warnings forgotten with their file weren't said: {forgotten:?}"
        );
        let until = now + Duration::from_secs(60);
        warned
            .warned("wlan3", HALL, until, (now, start))
            .expect("kept");
        let (again, forgotten) = Warned::kept_in(&file, start);
        assert_eq!(
            again.lasting(now, start),
            vec![("wlan3".to_owned(), HALL.to_owned(), until)],
            "the next warning didn't replace a file that couldn't be read"
        );
        assert!(
            forgotten.is_empty(),
            "a file the next warning replaced was still said forgotten: {forgotten:?}"
        );
    }

    #[test]
    fn no_file_says_nothing_forgotten() {
        let folder = tempfile::tempdir().expect("a folder");

        let (warned, forgotten) = Warned::kept_in(&folder.path().join("warned"), boot(600));

        assert!(
            warned.kept.is_empty() && forgotten.is_empty(),
            "a boot with no warning yet said one was forgotten: {forgotten:?}"
        );
    }

    #[test]
    fn the_boot_clock_only_goes_on() {
        let first = Boot::now().expect("the boot clock");
        let second = Boot::now().expect("the boot clock");

        assert!(
            second.0 >= first.0 && first.0 > Duration::ZERO,
            "the boot clock read {first:?} then {second:?}"
        );
    }
}
