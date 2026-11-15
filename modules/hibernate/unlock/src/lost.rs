//! A hibernation whose image never came back, said to whoever signs in at
//! the next start.
//!
//! When a start doesn't read the image back, whatever was open is gone, and
//! until now nothing said so: the machine simply came up afresh, and Joel
//! learned it from his windows. Before an image is written, the hook keeps
//! a mark on the disk naming this start's boot id, and after any return,
//! resumed or rolled back, it takes the mark away. The image keeps the
//! running kernel's boot id, so a resume comes back to the same one. A
//! start that finds the mark under another boot id was never the image
//! coming back: this start's log says why, as near as it can, and once
//! someone is signed in to a graphical session they're told once, after
//! which the mark goes.

use std::fs::{self, File};
use std::io::{self, ErrorKind, Write};
use std::path::Path;

/// Where the mark is kept, on a disk that outlives the start.
pub const MARK: &str = "/var/lib/hibernate-unlock/writing";

/// This start's boot id, which the image keeps.
pub const BOOT_ID: &str = "/proc/sys/kernel/random/boot_id";

/// Keeps the mark at `mark`, naming `boot`, and makes sure it's on the disk
/// before the image is written.
pub fn keep(mark: &Path, boot: &str) -> io::Result<()> {
    if let Some(folder) = mark.parent() {
        fs::create_dir_all(folder)?;
    }
    let mut file = File::create(mark)?;
    file.write_all(boot.trim().as_bytes())?;
    file.sync_all()
}

/// Takes the mark away, as nothing is lost once the machine is back.
pub fn forget(mark: &Path) -> io::Result<()> {
    match fs::remove_file(mark) {
        Err(error) if error.kind() != ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

/// Whether the mark at `mark` was kept by a start other than `boot`'s: an
/// image written then never came back.
pub fn lost(mark: &Path, boot: &str) -> bool {
    fs::read_to_string(mark).is_ok_and(|kept| kept.trim() != boot.trim())
}

/// Why this start didn't read the image back, from what it logged.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Why {
    /// Linux found the image and refused it or couldn't read it.
    Refused,
    /// This start never looked for it: another kernel or boot entry
    /// started, and systemd cleared where the image was.
    Elsewhere,
    /// Nothing said: the power went, or it was forced off, before the
    /// image was read.
    Unsaid,
}

impl Why {
    /// Reads this start's log, a line each.
    #[must_use]
    pub fn from_log(said: &[String]) -> Self {
        let any = |words: &[&str]| {
            said.iter()
                .any(|line| words.iter().any(|word| line.contains(word)))
        };
        if any(&[
            "Failed to load image",
            "Image mismatch",
            "Could not resume from",
        ]) {
            Self::Refused
        } else if any(&["Finished Clear Stale Hibernate Storage Info"]) {
            Self::Elsewhere
        } else {
            Self::Unsaid
        }
    }

    /// The words the log line gives it.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Refused => "refused",
            Self::Elsewhere => "elsewhere",
            Self::Unsaid => "unsaid",
        }
    }
}

/// The notice's title and body: what happened, why, and where to look.
#[must_use]
pub fn said(why: Why) -> (String, String) {
    let opening = match why {
        Why::Refused => {
            "The laptop found the image it hibernated to but couldn't read it back, so it \
             started afresh."
        }
        Why::Elsewhere => {
            "The laptop started another system than the one that hibernated, which couldn't \
             read its image, so it started afresh."
        }
        Why::Unsaid => {
            "The laptop started afresh before reading back the image it hibernated to: the \
             power may have gone, or it was held off."
        }
    };
    (
        "Your work couldn't come back".to_owned(),
        format!(
            "{opening} What was open when it hibernated is gone. \
             journalctl -b -u hibernate-unlock-lost says more."
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(said: &[&str]) -> Vec<String> {
        said.iter().map(|line| (*line).to_owned()).collect()
    }

    #[test]
    fn a_mark_from_another_start_is_an_image_lost() {
        let folder = std::env::temp_dir().join(format!("lost-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        let mark = folder.join("hibernate-unlock").join("writing");
        keep(&mark, "aaaa\n").expect("the mark is kept, its folder made");
        assert!(
            !lost(&mark, "aaaa\n"),
            "the start that kept the mark, back from its image, counted it lost"
        );
        assert!(
            lost(&mark, "bbbb\n"),
            "a start under another boot id didn't count the mark as an image lost"
        );
        forget(&mark).expect("the mark goes");
        assert!(!lost(&mark, "bbbb\n"), "a mark taken away still counted");
        forget(&mark).expect("a mark already gone is no error");
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn the_start_says_why_it_didnt_read_the_image_back() {
        let cases = [
            (
                vec!["PM: hibernation: Failed to load image, recovering."],
                Why::Refused,
            ),
            (
                vec!["PM: Image mismatch: architecture specific data"],
                Why::Refused,
            ),
            (
                vec!["Could not resume from '/dev/dm-0' (254:0:533760)."],
                Why::Refused,
            ),
            (
                vec!["Finished Clear Stale Hibernate Storage Info."],
                Why::Elsewhere,
            ),
            (
                vec!["Clear Stale Hibernate Storage Info skipped, unmet condition check"],
                Why::Unsaid,
            ),
            (vec![], Why::Unsaid),
        ];
        for (said, why) in cases {
            assert_eq!(
                Why::from_log(&lines(&said)),
                why,
                "{said:?} read as the wrong why"
            );
        }
    }

    #[test]
    fn each_notice_says_its_work_is_gone_why_and_where_to_look() {
        let cases = [
            (
                Why::Refused,
                "The laptop found the image it hibernated to but couldn't read it back, \
                 so it started afresh.",
            ),
            (
                Why::Elsewhere,
                "The laptop started another system than the one that hibernated, which \
                 couldn't read its image, so it started afresh.",
            ),
            (
                Why::Unsaid,
                "The laptop started afresh before reading back the image it hibernated to: \
                 the power may have gone, or it was held off.",
            ),
        ];
        for (why, opening) in cases {
            assert_eq!(
                said(why),
                (
                    "Your work couldn't come back".to_owned(),
                    format!(
                        "{opening} What was open when it hibernated is gone. \
                         journalctl -b -u hibernate-unlock-lost says more."
                    )
                ),
                "the notice for {why:?} isn't word for word"
            );
        }
    }
}
