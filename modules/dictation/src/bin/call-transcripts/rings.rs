//! The Tuple calls Joel missed, as the calls receptor counts them: when
//! each rang, and nothing else. tuple-call keeps a line a call in
//! `tuple-call/recent.jsonl` in the state folder (modules/tuple/call's
//! recent.rs), naming everyone in it; only whether it was `missed` and
//! when it rang, `since`, are read here, and the rest of each line is
//! passed over as it's parsed, never kept, so who rang never reaches the
//! receptor, its memory past the parse, or mycelium.
//!
//! Nothing is written to Tuple or its files, and no tuple command is run.
//! A file that's missing, can't be opened as a plain file of its own, is
//! larger than tuple-call ever writes, or holds a line that isn't one,
//! counts no missed call at all rather than a guess.

use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// The most a file of tuple-call's nine kept calls could hold, with
/// room to spare; one larger is no file tuple-call wrote.
const LARGEST_FILE: u64 = 64 * 1024;

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
    /// When each missed call began to ring, in milliseconds.
    Missed(Vec<u64>),
    /// No file: Tuple has kept no calls here, or isn't here.
    Missing,
    /// A file that couldn't be trusted, and why, in fixed words.
    Unsound(&'static str),
}

impl Rings {
    /// When each missed call rang, none unless the file read soundly.
    pub fn missed(&self) -> &[u64] {
        match self {
            Self::Missed(at) => at,
            Self::Missing | Self::Unsound(_) => &[],
        }
    }
}

/// Where tuple-call keeps its calls: `tuple-call/recent.jsonl` in the
/// state folder, as tuple-call itself finds it; none without an absolute
/// home to find it in.
pub fn file() -> Option<PathBuf> {
    let absolute = |name| {
        std::env::var_os(name)
            .map(PathBuf::from)
            .filter(|folder| folder.is_absolute())
    };
    let state =
        absolute("XDG_STATE_HOME").or_else(|| Some(absolute("HOME")?.join(".local/state")))?;
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
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Rings::Missing,
        Err(_) => return Rings::Unsound("it couldn't be opened as a file of its own"),
    };
    if !file.metadata().is_ok_and(|meta| meta.is_file()) {
        return Rings::Unsound("it isn't a plain file");
    }
    let mut text = String::new();
    if file
        .take(LARGEST_FILE + 1)
        .read_to_string(&mut text)
        .is_err()
    {
        return Rings::Unsound("it isn't text");
    }
    if u64::try_from(text.len()).map_or(true, |length| length > LARGEST_FILE) {
        return Rings::Unsound("it's larger than tuple-call writes");
    }
    of(&text)
}

/// The missed calls `text` names, a call a line.
fn of(text: &str) -> Rings {
    let mut missed = Vec::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(ring) = serde_json::from_str::<Ring>(line) else {
            return Rings::Unsound("a line isn't a call");
        };
        if ring.missed {
            let Ok(at) = u64::try_from(ring.since) else {
                return Rings::Unsound("a call rang before 1970");
            };
            missed.push(at);
        }
    }
    Rings::Missed(missed)
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

    #[test]
    fn only_the_missed_calls_are_read_and_only_when_each_rang() {
        let text = [line(true, 1_000), line(false, 2_000), line(true, 3_000)].join("\n");
        assert_eq!(of(&text), Rings::Missed(vec![1_000, 3_000]));
        assert_eq!(
            of(""),
            Rings::Missed(Vec::new()),
            "no calls is no missed call"
        );
    }

    #[test]
    fn who_rang_is_never_kept() {
        let read = of(&line(true, 1_000));
        let kept = format!("{read:?}");
        for word in ["Ada", "Brightwater", "example", "7,"] {
            assert!(!kept.contains(word), "{word} was kept: {kept}");
        }
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
            let read = of(&text);
            assert!(
                matches!(read, Rings::Unsound(_)),
                "{bad:?} was read past: {read:?}"
            );
            assert!(read.missed().is_empty(), "{bad:?} still counted a call");
        }
    }

    #[test]
    fn a_file_missing_linked_or_too_large_counts_none() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("recent.jsonl");
        assert_eq!(read(&path), Rings::Missing);

        let sound = folder.path().join("sound.jsonl");
        std::fs::write(&sound, line(true, 1_000)).unwrap();
        assert_eq!(
            read(&sound),
            Rings::Missed(vec![1_000]),
            "the file linked to doesn't read soundly"
        );
        std::os::unix::fs::symlink(&sound, &path).unwrap();
        assert!(
            matches!(read(&path), Rings::Unsound(_)),
            "a link was read through"
        );
        std::fs::remove_file(&path).unwrap();

        let mut large = line(true, 1_000).into_bytes();
        large.resize(usize::try_from(LARGEST_FILE).unwrap() + 1, b'\n');
        std::fs::write(&path, large).unwrap();
        assert!(
            matches!(read(&path), Rings::Unsound(_)),
            "a file past the bound was read"
        );

        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        assert!(
            matches!(read(&path), Rings::Unsound(_)),
            "a folder was read as a file"
        );

        // A pipe opens and reads as empty, no missed call, so only the
        // plain-file check tells it from a file of none.
        std::fs::remove_dir(&path).unwrap();
        let made = std::process::Command::new("mkfifo")
            .arg(&path)
            .status()
            .unwrap();
        assert!(made.success(), "mkfifo failed");
        assert!(
            matches!(read(&path), Rings::Unsound(_)),
            "a pipe was read as a file of no calls"
        );
    }
}
