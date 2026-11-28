//! The access point that last warned each card it drops the card, kept
//! past a restart, so a wifid started again, as a switch restarts it,
//! doesn't hold a card back to a point about to drop it
//! (`Wifid::holds_to`). Held in memory alone, a warning was forgotten
//! then, and the hand-over at start held the card to the point it was on.
//!
//! It's kept in wifid's runtime folder, which outlives a restart but not
//! a boot: no warning keeps its point off past an hour, so one from
//! before a boot would be long run out, and nothing need clear it. Nor
//! should anything keep it longer: the points that warned and when say
//! where the laptop has been, which a folder on the disk would hold on
//! to past every boot. Each is kept by the boot clock (`Boot`), which
//! the folder never outlives, as an `Instant` means nothing to the next
//! wifid, and is read back as the time it has left. A warning run out stays in the file until the
//! next is kept, but is never read back. A file that can't be read as
//! wifid starts is read again before a warning is first kept over it, as
//! the warning, written whole, would otherwise take every other card's.
//!
//! A warning the chooser forgot as its card came back to the point
//! (`State::warned_off`) is still kept here, so a wifid started again
//! meanwhile keeps the chooser off that point once the card leaves it,
//! until the keep-off ends, as the moves keep off it with no restart
//! (`Arrivals::drops`).

use std::collections::BTreeMap;
use std::fmt;
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
    /// Whether the file couldn't be read as wifid started, so is read
    /// again before a warning is first kept over it.
    unread: bool,
}

impl Warned {
    /// None kept yet, to be kept in this file.
    pub fn none(file: &Path) -> Warned {
        Warned {
            file: file.to_path_buf(),
            kept: BTreeMap::new(),
            unread: false,
        }
    }

    /// The warnings kept in this file, `now` on the boot clock, and what
    /// was forgotten of them, for the journal, each line ending with what
    /// follows: a file that can't be read or isn't as wifid keeps it is
    /// begun again empty, and a warning that would outlast any keep-off
    /// is dropped, as wifid wrote neither. No file says nothing, as no
    /// card was warned since the boot.
    ///
    /// What's left is written back over a file it read and forgot any of,
    /// as each start reads the file again: one left as it was said the
    /// same forgetting at every start until the next warning, as if it
    /// were new. A file it couldn't read is never written over, as it may
    /// read later with other cards' warnings still lasting in it; it's
    /// read again before the first warning is kept over it instead.
    pub fn kept_in(file: &Path, now: Boot) -> (Warned, Vec<String>) {
        let (warned, mut forgotten) = Warned::found_in(file, now);
        if !forgotten.is_empty()
            && !warned.unread
            && let Err(error) = warned.keep()
        {
            forgotten.push(format!(
                "can't write back the warnings it keeps, so says this again as it next starts: \
                 {error}"
            ));
        }
        (warned, forgotten)
    }

    /// The warnings kept in this file and what was forgotten of them, as
    /// `kept_in` says, the file left as it is.
    fn found_in(file: &Path, now: Boot) -> (Warned, Vec<String>) {
        let mut warned = Warned::none(file);
        let kept = match warnings_in(file) {
            Ok(kept) => kept,
            Err(unkept) => {
                // A file unreadable now may read later, as a failing disk
                // or a moment's lack of memory passes, and keeping a
                // warning over it unread would lose every other card's.
                let follows = match unkept {
                    Unkept::Unreadable(_) => {
                        warned.unread = true;
                        "it reads them again before it keeps a warning over them"
                    }
                    Unkept::Unlike(_) => "it clears them from the file",
                };
                let said = format!(
                    "forgets the warnings kept before, as {unkept}; no point warned before the \
                     restart is kept off, and {follows}"
                );
                return (warned, vec![said]);
            }
        };
        let forgotten = warned.take_in(kept, now);
        (warned, forgotten)
    }

    /// Takes in the warnings read from the file, `now` on the boot clock,
    /// but for any a card already has here, which is newer, and says each
    /// dropped as it would outlast any keep-off.
    fn take_in(&mut self, kept: BTreeMap<String, Kept>, now: Boot) -> Vec<String> {
        let most = now.millis().saturating_add(millis_of(LONGEST));
        let mut forgotten = Vec::new();
        for (card, kept) in kept {
            if kept.ends > most {
                forgotten.push(format!(
                    "forgets {card}'s warning from {}, as it would outlast any keep-off; \
                     {card} isn't kept off it",
                    kept.point
                ));
            } else {
                self.kept.entry(card).or_insert(kept);
            }
        }
        forgotten
    }

    /// Reads the file again, once, as it couldn't be read as wifid
    /// started, so the first warning kept over it keeps every other
    /// card's still lasting there, and says what it forgot. Those are
    /// kept for the next wifid alone: this one didn't keep their points
    /// off from its start, and their cards may have left them since.
    fn read_again(&mut self, now: Boot) -> Vec<String> {
        self.unread = false;
        match warnings_in(&self.file) {
            Ok(kept) => self.take_in(kept, now),
            Err(unkept) => vec![format!(
                "forgets the warnings kept before, as {unkept}; the warning it keeps now \
                 replaces them"
            )],
        }
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
    /// Says, for the journal, what it forgot of a file read again as it
    /// couldn't be read when wifid started.
    pub fn warned(
        &mut self,
        card: &str,
        point: &str,
        until: Instant,
        (now, boot): (Instant, Boot),
    ) -> io::Result<Vec<String>> {
        let said = if self.unread {
            self.read_again(boot)
        } else {
            Vec::new()
        };
        let since = boot.millis();
        self.kept.retain(|_, kept| kept.ends > since);
        let kept = Kept {
            point: point.to_owned(),
            ends: since.saturating_add(millis_of(until.saturating_duration_since(now))),
        };
        self.kept.insert(card.to_owned(), kept);
        self.keep()?;
        Ok(said)
    }

    /// Writes the warnings whole or not at all, readable by wifid alone.
    fn keep(&self) -> io::Result<()> {
        let partial = self.file.with_extension("partial");
        // Made readable by wifid alone, so a new partial is never readable
        // past it even for a moment; but opening one left behind, by a
        // wifid stopped as it wrote, keeps the mode it had, which is set
        // again below.
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

/// Why the warnings kept in a file weren't read back.
#[derive(Debug)]
enum Unkept {
    /// The file is there, but reading it failed.
    Unreadable(io::Error),
    /// The file was read, but isn't as wifid keeps the warnings.
    Unlike(serde_json::Error),
}

impl fmt::Display for Unkept {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Unkept::Unreadable(error) => write!(out, "it can't read them: {error}"),
            Unkept::Unlike(error) => write!(out, "they aren't as it keeps them: {error}"),
        }
    }
}

/// The warnings kept in a file, or why they can't be read back: none
/// when there's no file, as no card was warned since the boot.
fn warnings_in(file: &Path) -> Result<BTreeMap<String, Kept>, Unkept> {
    let read = match fs::read(file) {
        Ok(read) => read,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(error) => return Err(Unkept::Unreadable(error)),
    };
    serde_json::from_slice(&read).map_err(Unkept::Unlike)
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

    const DESK: &str = "02:00:00:00:01:00";
    const HALL: &str = "02:00:00:00:02:00";

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
    fn a_partial_file_left_readable_is_kept_readable_by_wifid_alone() {
        let folder = tempfile::tempdir().expect("a folder");
        let file = folder.path().join("warned");
        let partial = file.with_extension("partial");
        fs::write(&partial, "").expect("a partial file");
        fs::set_permissions(&partial, fs::Permissions::from_mode(0o644)).expect("readable");
        let (now, start) = (Instant::now(), boot(600));

        Warned::none(&file)
            .warned("wlan2", DESK, now + Duration::from_secs(600), (now, start))
            .expect("kept");

        assert_eq!(
            fs::metadata(&file).expect("a file").permissions().mode() & 0o777,
            0o600,
            "a partial file left readable past wifid was kept so"
        );
    }

    #[test]
    fn a_warning_run_out_is_forgotten() {
        let folder = tempfile::tempdir().expect("a folder");
        let file = folder.path().join("warned");
        let now = Instant::now();
        let mut warned = Warned::none(&file);
        warned
            .warned(
                "wlan2",
                DESK,
                now + Duration::from_secs(5),
                (now, boot(600)),
            )
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
                "forgets wlan2's warning from {DESK}, as it would outlast any keep-off; wlan2 \
                 isn't kept off it"
            )],
            "a warning dropped as past any keep-off wasn't said"
        );
        assert_eq!(
            instant(start.millis(), now, start),
            None,
            "a warning run out as it's read kept its point off"
        );
        let (again, forgotten) = Warned::kept_in(&file, start);
        assert_eq!(
            (again.lasting(now, start), forgotten),
            (
                vec![("wlan3".to_owned(), HALL.to_owned(), now + LONGEST)],
                Vec::new()
            ),
            "the next start said the same forgetting again, or lost what was kept with it"
        );
    }

    #[test]
    fn a_file_it_can_t_write_back_says_it_ll_say_so_again() {
        let folder = tempfile::tempdir().expect("a folder");
        let file = folder.path().join("warned");
        fs::write(&file, "not as wifid keeps them").expect("a file");
        // A folder where the warnings are first written whole, which no
        // file can be opened as, even by root.
        fs::create_dir(file.with_extension("partial")).expect("a folder in the way");

        let (_, forgotten) = Warned::kept_in(&file, boot(600));

        assert!(
            forgotten.len() == 2 && forgotten[1].starts_with("can't write back"),
            "a forgetting it couldn't write back didn't say it'll be said again: {forgotten:?}"
        );
    }

    #[test]
    fn a_file_it_can_t_read_as_it_starts_is_never_written_over() {
        let folder = tempfile::tempdir().expect("a folder");
        // A folder where the file goes can't be read as a file, and no
        // file can be renamed over it while it holds anything.
        let file = folder.path().join("warned");
        fs::create_dir_all(file.join("inside")).expect("a folder in the way");

        let (_, forgotten) = Warned::kept_in(&file, boot(600));

        assert!(
            forgotten.len() == 1 && !file.with_extension("partial").exists(),
            "a file it couldn't read was written over as it started: {forgotten:?}"
        );
    }

    #[test]
    fn a_file_kept_by_the_wall_clock_is_said_forgotten_once_and_cleared() {
        let folder = tempfile::tempdir().expect("a folder");
        let file = folder.path().join("warned");
        // As a wifid before this one kept it, in milliseconds since 1970.
        fs::write(
            &file,
            format!(r#"{{"wlan2":{{"point":"{DESK}","until":1795700000000}}}}"#),
        )
        .expect("a file");
        let (now, start) = (Instant::now(), boot(600));

        let (warned, forgotten) = Warned::kept_in(&file, start);

        assert!(
            warned.lasting(now, start).is_empty(),
            "a file wifid didn't write this way kept a point off"
        );
        assert!(
            forgotten.len() == 1
                && forgotten[0].starts_with(
                    "forgets the warnings kept before, as they aren't as it keeps them"
                )
                && forgotten[0].ends_with(
                    "; no point warned before the restart is kept off, and it clears them from \
                     the file"
                ),
            "the warnings forgotten with a file unlike wifid's, and what follows, weren't said: \
             {forgotten:?}"
        );
        assert!(
            !fs::read_to_string(&file).expect("a file").contains(DESK),
            "a file unlike wifid's stayed on the disk once said forgotten"
        );
        let (_, again) = Warned::kept_in(&file, start);
        assert!(
            again.is_empty(),
            "the next start said the same file forgotten again: {again:?}"
        );
    }

    #[test]
    fn a_kept_file_is_read_back_or_said_unreadable_or_unlike_what_wifid_keeps() {
        let folder = tempfile::tempdir().expect("a folder");
        let file = folder.path().join("warned");
        assert!(
            matches!(warnings_in(&file), Ok(kept) if kept.is_empty()),
            "no file wasn't read as no warning kept"
        );

        fs::create_dir(&file).expect("a folder where the file goes");
        let unreadable = warnings_in(&file);
        assert!(
            matches!(
                &unreadable,
                Err(Unkept::Unreadable(error)) if error.kind() == io::ErrorKind::IsADirectory
            ),
            "a file that couldn't be read wasn't said unreadable: {unreadable:?}"
        );

        fs::remove_dir(&file).expect("the folder gone");
        fs::write(&file, "not as wifid keeps them").expect("a file");
        let unlike = warnings_in(&file);
        assert!(
            matches!(&unlike, Err(Unkept::Unlike(_))),
            "a file not as wifid keeps it wasn't said unlike it: {unlike:?}"
        );
    }

    #[test]
    fn a_file_unreadable_as_wifid_starts_is_read_again_before_a_warning_is_kept_over_it() {
        let folder = tempfile::tempdir().expect("a folder");
        let file = folder.path().join("warned");
        fs::create_dir(&file).expect("a folder where the file goes");
        let (now, start) = (Instant::now(), boot(600));

        let (mut warned, forgotten) = Warned::kept_in(&file, start);

        assert!(
            forgotten.len() == 1
                && forgotten[0].starts_with("forgets the warnings kept before, as it can't read")
                && forgotten[0].ends_with(
                    "; no point warned before the restart is kept off, and it reads them again \
                     before it keeps a warning over them"
                ),
            "the warnings forgotten with a file it can't read, and what follows, weren't said: \
             {forgotten:?}"
        );
        // Readable again by the next warning, with another card's still
        // lasting in it.
        fs::remove_dir(&file).expect("the folder gone");
        let hall = now + Duration::from_secs(600);
        Warned::none(&file)
            .warned("wlan3", HALL, hall, (now, start))
            .expect("kept");
        let desk = now + Duration::from_secs(60);
        let said = warned
            .warned("wlan2", DESK, desk, (now, start))
            .expect("kept");

        assert!(
            said.is_empty(),
            "a file read again whole was said forgotten: {said:?}"
        );
        assert_eq!(
            Warned::kept_in(&file, start).0.lasting(now, start),
            vec![
                ("wlan2".to_owned(), DESK.to_owned(), desk),
                ("wlan3".to_owned(), HALL.to_owned(), hall)
            ],
            "a warning kept over a file unreadable at the start lost another card's"
        );
    }

    #[test]
    fn a_file_unreadable_as_wifid_starts_and_unlike_its_own_after_is_replaced() {
        let folder = tempfile::tempdir().expect("a folder");
        let file = folder.path().join("warned");
        fs::create_dir(&file).expect("a folder where the file goes");
        let (now, start) = (Instant::now(), boot(600));
        let (mut warned, _) = Warned::kept_in(&file, start);
        fs::remove_dir(&file).expect("the folder gone");
        fs::write(&file, "not as wifid keeps them").expect("a file");

        let until = now + Duration::from_secs(60);
        let said = warned
            .warned("wlan2", DESK, until, (now, start))
            .expect("kept");

        assert!(
            said.len() == 1
                && said[0].starts_with(
                    "forgets the warnings kept before, as they aren't as it keeps them"
                )
                && said[0].ends_with("; the warning it keeps now replaces them"),
            "a file read again and not as wifid keeps it wasn't said replaced: {said:?}"
        );
        assert_eq!(
            Warned::kept_in(&file, start).0.lasting(now, start),
            vec![("wlan2".to_owned(), DESK.to_owned(), until)],
            "the warning kept didn't replace a file unlike wifid's"
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
