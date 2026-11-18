//! The Tuple calls Joel missed, as the calls receptor counts them: when
//! each rang, and nothing else. tuple-call keeps a line a call in
//! `tuple-call/recent.jsonl` in the state folder (modules/tuple/call's
//! recent.rs), naming everyone in it; only whether it was `missed` and
//! when it rang, `since`, are read here. The rest of each line is passed
//! over as it's parsed, and the file's bytes are zeroed once read, so who
//! rang never reaches the receptor, its memory past the read, or mycelium.
//!
//! Nothing is written to Tuple or its files, and no tuple command is run.
//! A file that's missing, can't be opened as a plain file of its own, is
//! larger than tuple-call could fill, or holds a line that isn't a call
//! counts no missed call at all rather than a guess, and says why in fixed
//! words for the daemon's log.

use std::collections::BTreeSet;
use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// More than tuple-call's nine kept calls could fill, even each with a
/// room of a hundred people; one larger is no file tuple-call wrote.
const LARGEST_FILE: u64 = 1024 * 1024;

/// One call tuple-call kept, as far as the receptor reads it.
#[derive(Deserialize)]
struct Ring {
    /// Whether it rang and wasn't answered.
    missed: bool,
    /// When it began to ring, or connected, in milliseconds.
    since: i64,
}

/// What reading the rings found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rings {
    /// When each missed call in the file began to ring, in milliseconds.
    Missed(Vec<u64>),
    /// Nothing could be counted, and why, in fixed words.
    Unread(&'static str),
}

/// Where tuple-call keeps its calls: `tuple-call/recent.jsonl` in the
/// state folder, found as tuple-call finds it, from `XDG_STATE_HOME` or
/// else `HOME`, but only when that's an absolute folder, so a relative one
/// never reads a file wherever the daemon happens to run.
pub fn file() -> Option<PathBuf> {
    let named = |name| std::env::var_os(name).filter(|value| !value.is_empty());
    let state = named("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| named("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .filter(|folder| folder.is_absolute())?;
    Some(state.join("tuple-call/recent.jsonl"))
}

/// The missed calls in the file at `path`.
pub fn read(path: &Path) -> Rings {
    let opened = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(path);
    let file = match opened {
        Ok(file) => file,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
            return Rings::Unread("tuple-call has kept no calls");
        }
        Err(_) => return Rings::Unread("its file couldn't be opened as a file of its own"),
    };
    if !file.metadata().is_ok_and(|meta| meta.is_file()) {
        return Rings::Unread("its file isn't a plain file");
    }
    let mut bytes = Vec::new();
    let rings = match file.take(LARGEST_FILE + 1).read_to_end(&mut bytes) {
        Err(_) => Rings::Unread("its file couldn't be read"),
        Ok(length) if u64::try_from(length).map_or(true, |length| length > LARGEST_FILE) => {
            Rings::Unread("its file is larger than tuple-call writes")
        }
        Ok(_) => of(&bytes),
    };
    forget(bytes);
    rings
}

/// Zeroes what the file said before letting it go, so the names in it
/// don't sit in freed memory.
fn forget(mut bytes: Vec<u8>) {
    bytes.fill(0);
    // So the zeroing isn't taken out as a write nothing reads.
    std::hint::black_box(&bytes);
}

/// The missed calls `bytes` names, a call a line.
fn of(bytes: &[u8]) -> Rings {
    let mut missed = Vec::new();
    for line in bytes.split(|&byte| byte == b'\n') {
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let Ok(ring) = serde_json::from_slice::<Ring>(line) else {
            return Rings::Unread("a line of its file isn't a call");
        };
        if ring.missed {
            let Ok(at) = u64::try_from(ring.since) else {
                return Rings::Unread("a call in its file rang before 1970");
            };
            missed.push(at);
        }
    }
    Rings::Missed(missed)
}

/// The missed calls heard today, kept between reads: tuple-call keeps
/// only its newest nine calls, so a ring missed this morning leaves its
/// file once Joel takes enough calls, and would stop being counted while
/// it's still today's. Only when each rang is kept.
#[derive(Clone, Debug, Default)]
pub struct Heard {
    rang: BTreeSet<u64>,
}

impl Heard {
    /// When each missed call heard since `midnight` rang, this read's
    /// among them; none while the file can't be read, since what it holds
    /// then isn't known, though what was heard is kept for when it reads
    /// again.
    pub fn hear(&mut self, rings: &Rings, midnight: u64) -> Vec<u64> {
        self.rang.retain(|&at| at >= midnight);
        match rings {
            Rings::Missed(rang) => {
                self.rang
                    .extend(rang.iter().copied().filter(|&at| at >= midnight));
                self.rang.iter().copied().collect()
            }
            Rings::Unread(_) => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(missed: bool, since: i64) -> String {
        serde_json::json!({
            "people": [{"id": "7", "name": "Ada Brightwater", "email": "ada@example.com"}],
            "since": since,
            "ended": since + 60_000,
            "missed": missed,
            "caller": null,
        })
        .to_string()
    }

    fn unread(rings: &Rings) -> bool {
        matches!(rings, Rings::Unread(_))
    }

    #[test]
    fn only_the_missed_calls_are_read_and_only_when_each_rang() {
        let text = [line(true, 1_000), line(false, 2_000), line(true, 3_000)].join("\n");
        assert_eq!(of(text.as_bytes()), Rings::Missed(vec![1_000, 3_000]));
        assert_eq!(
            of(b""),
            Rings::Missed(Vec::new()),
            "no calls is no missed call"
        );
    }

    #[test]
    fn one_line_that_isnt_a_call_counts_none_of_them() {
        for bad in [
            "not json".to_owned(),
            r#"{"since": 1000}"#.to_owned(),
            r#"{"missed": "yes", "since": 1000}"#.to_owned(),
            line(true, -1),
        ] {
            let text = [line(true, 1_000), bad.clone()].join("\n");
            assert!(unread(&of(text.as_bytes())), "{bad:?} was read past");
        }
    }

    #[test]
    fn a_file_missing_linked_or_too_large_counts_none() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("recent.jsonl");
        assert!(unread(&read(&path)), "a missing file was counted");

        let sound = folder.path().join("sound.jsonl");
        std::fs::write(&sound, line(true, 1_000)).unwrap();
        assert_eq!(
            read(&sound),
            Rings::Missed(vec![1_000]),
            "the file linked to doesn't read soundly"
        );
        std::os::unix::fs::symlink(&sound, &path).unwrap();
        assert!(unread(&read(&path)), "a link was read through");
        std::fs::remove_file(&path).unwrap();

        let size = usize::try_from(LARGEST_FILE).unwrap();
        let mut largest = line(true, 1_000).into_bytes();
        largest.resize(size, b'\n');
        std::fs::write(&path, &largest).unwrap();
        assert_eq!(
            read(&path),
            Rings::Missed(vec![1_000]),
            "a file at the bound wasn't read"
        );
        largest.push(b'\n');
        std::fs::write(&path, largest).unwrap();
        assert!(unread(&read(&path)), "a file past the bound was read");

        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        assert!(unread(&read(&path)), "a folder was read as a file");

        // A pipe opens and reads as empty, no missed call, so only the
        // plain-file check tells it from a file of none.
        std::fs::remove_dir(&path).unwrap();
        let made = std::process::Command::new("mkfifo")
            .arg(&path)
            .status()
            .unwrap();
        assert!(made.success(), "mkfifo failed");
        assert!(
            unread(&read(&path)),
            "a pipe was read as a file of no calls"
        );
    }

    #[test]
    fn a_ring_missed_today_is_still_heard_once_it_leaves_the_file() {
        let midnight = 10_000;
        let mut heard = Heard::default();
        let first = Rings::Missed(vec![midnight - 1, midnight + 5]);
        assert_eq!(
            heard.hear(&first, midnight),
            [midnight + 5],
            "yesterday's ring was heard as today's"
        );
        let later = Rings::Missed(vec![midnight + 9]);
        assert_eq!(
            heard.hear(&later, midnight),
            [midnight + 5, midnight + 9],
            "a ring that left the file stopped being heard today"
        );
        assert!(
            heard
                .hear(&Rings::Unread("its file isn't a plain file"), midnight)
                .is_empty(),
            "rings were counted while the file couldn't be read"
        );
        assert_eq!(
            heard.hear(&Rings::Missed(Vec::new()), midnight),
            [midnight + 5, midnight + 9],
            "what was heard was lost while the file couldn't be read"
        );
        assert!(
            heard
                .hear(&Rings::Missed(Vec::new()), midnight + 10)
                .is_empty(),
            "a ring was still heard the next day"
        );
    }
}
