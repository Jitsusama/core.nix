//! Reading the records the queues are measured from, each capped by
//! construction: the end of a record that grows a line a job, and when
//! each rider aboard the merge train boarded. A record that isn't there,
//! isn't a plain file, or whose end holds no whole line reads as unread,
//! never as empty: empty says nothing passed, where these say the hour
//! can't tell.

use std::fs;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

/// How much of each queue's record the hour reads: nix-offload keeps every
/// send it ever made (38 MB on 21 November), about 2.6 MB a day, so the end
/// holds days of them, and the others are smaller.
pub const RECORD_READ: u64 = 16 << 20;

/// The most of a rider's file read: merge-net writes its head, branch and
/// boarding time, some 150 bytes.
const RIDER_READ: u64 = 4096;

/// The file at `path` opened only where it is a plain one, since opening
/// a pipe left at a record's path would wait on it for ever.
fn plain(path: &Path) -> io::Result<fs::File> {
    if !fs::metadata(path)?.is_file() {
        return Err(io::Error::other("not a plain file"));
    }
    fs::File::open(path)
}

/// The last `bytes` of the file at `path`, from its first whole line, or
/// none where it can't be read: not there, not a plain file, failing, or
/// cut with no break left after the cut, which holds no whole line.
pub fn tail(path: &Path, bytes: u64) -> Option<String> {
    let read = || -> io::Result<Option<String>> {
        let mut file = plain(path)?;
        let from = file.metadata()?.len().saturating_sub(bytes);
        // One byte early, so a cut falling on a line's start keeps it:
        // the break before it is what's dropped.
        let early = from.saturating_sub(1);
        file.seek(SeekFrom::Start(early))?;
        let mut raw = Vec::new();
        // Taken to the bound, so a file that grew since can't be read
        // past it.
        file.take(bytes + from - early).read_to_end(&mut raw)?;
        let text = String::from_utf8_lossy(&raw);
        // A cut line isn't one, so it starts after the first break.
        if from == 0 {
            return Ok(Some(text.into_owned()));
        }
        Ok(text.split_once('\n').map(|(_, rest)| rest.to_owned()))
    };
    read().ok().flatten()
}

/// When each rider aboard the train boarded, from its file in `riders`:
/// none aboard where the folder isn't there, since merge-net makes it at
/// the first boarding, and none read where it can't be listed.
pub fn aboard(riders: &Path) -> Option<Vec<i64>> {
    let listed = match fs::read_dir(riders) {
        Ok(listed) => listed,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Some(Vec::new()),
        Err(_) => return None,
    };
    let boarded = |path: &Path| -> Option<i64> {
        let mut said = String::new();
        plain(path)
            .ok()?
            .take(RIDER_READ)
            .read_to_string(&mut said)
            .ok()?;
        serde_json::from_str::<serde_json::Value>(&said).ok()?["at"].as_i64()
    };
    Some(
        listed
            .flatten()
            .filter_map(|entry| boarded(&entry.path()))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_records_tail_starts_at_its_first_whole_line() {
        let root = tempfile::tempdir().expect("a scratch folder");
        let path = root.path().join("there.jsonl");
        // Each line is 9 bytes.
        fs::write(&path, "{\"at\":1}\n{\"at\":2}\n{\"at\":3}\n").expect("written");
        assert_eq!(
            tail(&path, 12).as_deref(),
            Some("{\"at\":3}\n"),
            "the cut line is left out"
        );
        assert_eq!(
            tail(&path, 18).as_deref(),
            Some("{\"at\":2}\n{\"at\":3}\n"),
            "a cut on a line's start dropped that whole line"
        );
        assert_eq!(
            tail(&path, 1 << 20),
            Some(fs::read_to_string(&path).expect("read")),
            "all of a short one"
        );
        assert_eq!(
            tail(&root.path().join("none"), 12),
            None,
            "a missing record read as an empty one, which closes its queue's ask"
        );
        assert_eq!(tail(root.path(), 12), None, "a folder can't be read");
        let unbroken = root.path().join("unbroken.jsonl");
        fs::write(&unbroken, "x".repeat(40)).expect("written");
        assert_eq!(
            tail(&unbroken, 12),
            None,
            "an end holding no whole line read as an empty record"
        );
        // A pipe at a record's path: opened, it would wait for a writer.
        let pipe = root.path().join("pipe.jsonl");
        let made = std::process::Command::new("mkfifo")
            .arg(&pipe)
            .status()
            .expect("mkfifo ran");
        assert!(made.success(), "the pipe wasn't made");
        assert!(plain(&pipe).is_err(), "a pipe was opened as a record");
        assert_eq!(tail(&pipe, 12), None, "a pipe read as a record");
    }

    #[test]
    fn each_rider_aboard_says_when_it_boarded() {
        let root = tempfile::tempdir().expect("a scratch folder");
        let riders = root.path().join("riders");
        assert_eq!(
            aboard(&riders),
            Some(Vec::new()),
            "no folder is no rider yet"
        );
        fs::create_dir(&riders).expect("made");
        // As merge-net's board() writes one.
        fs::write(
            riders.join("a1.json"),
            r#"{"head":"a1","branch":"jitsusama/a","at":1795262400}"#,
        )
        .expect("written");
        fs::write(riders.join("b2.json"), "not a rider").expect("written");
        fs::write(
            riders.join("c3.json"),
            format!(r#"{{"at":1,"pad":"{}"}}"#, "x".repeat(5000)),
        )
        .expect("written");
        fs::create_dir(riders.join("d4.json")).expect("made");
        assert_eq!(
            aboard(&riders),
            Some(vec![1_795_262_400]),
            "a rider's boarding wasn't read, or a file that isn't one, read whole, was"
        );
        assert_eq!(
            aboard(&riders.join("a1.json")),
            None,
            "riders that can't be listed read as none aboard"
        );
    }
}
