//! Reading pi's logs into the history, from where each stopped. The first
//! pass reads every log there is, a slice at a time so the daemon keeps
//! answering while it does; after that, only what each log grew by.

use crate::history::History;
use crate::logs::{Carry, read};
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// The most read from one log in one pass.
const SLICE: u64 = 32 << 20;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Pass {
    pub bytes: u64,
    pub turns: u64,
    /// Whether any log has more left to read than this pass took.
    pub behind: bool,
}

/// Every log, oldest directory entry first, so a first pass fills the
/// history in roughly the order it happened.
pub fn logs(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(projects) = std::fs::read_dir(dir) else {
        return found;
    };
    for project in projects.flatten() {
        let Ok(files) = std::fs::read_dir(project.path()) else {
            continue;
        };
        for file in files.flatten() {
            let path = file.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "jsonl")
            {
                found.push(path);
            }
        }
    }
    // Named by when they started, so the names sort by time.
    found.sort_by(|one, other| one.file_name().cmp(&other.file_name()));
    found
}

/// Reads every log's new lines. One that can't be read is said and passed
/// over, so the logs after it are still read; it's tried again next pass.
pub fn scan(history: &History, dir: &Path, refusals: &[String]) -> Result<Pass, String> {
    let mut total = Pass::default();
    for path in logs(dir) {
        match scan_one(history, &path, refusals) {
            Ok(one) => {
                total.bytes += one.bytes;
                total.turns += one.turns;
                total.behind |= one.behind;
            }
            Err(error) => eprintln!("reading {}: {error}", path.display()),
        }
    }
    Ok(total)
}

pub fn scan_one(history: &History, path: &Path, refusals: &[String]) -> Result<Pass, String> {
    scan_slice(history, path, refusals, SLICE)
}

/// Where the line under way at `from` ends, just past its newline, if it
/// has ended yet.
fn line_end(file: &mut std::fs::File, from: u64, slice: u64) -> Result<Option<u64>, String> {
    file.seek(SeekFrom::Start(from))
        .map_err(|error| error.to_string())?;
    let mut at = from;
    let mut chunk = vec![0u8; slice.min(1 << 20) as usize];
    loop {
        let got = file.read(&mut chunk).map_err(|error| error.to_string())?;
        if got == 0 {
            return Ok(None);
        }
        if let Some(newline) = chunk[..got].iter().position(|&byte| byte == b'\n') {
            return Ok(Some(at + newline as u64 + 1));
        }
        at += got as u64;
    }
}

fn scan_slice(
    history: &History,
    path: &Path,
    refusals: &[String],
    slice: u64,
) -> Result<Pass, String> {
    let Ok(metadata) = std::fs::metadata(path) else {
        return Ok(Pass::default());
    };
    let identity = format!("{}:{}", metadata.dev(), metadata.ino());
    let name = path.to_string_lossy().into_owned();
    let (mut offset, mut carry) = match history.file(&name, &identity) {
        Some((offset, carry)) => (offset, serde_json::from_str(&carry).unwrap_or_default()),
        None => {
            // A new log, or one replaced under the same name: start over.
            history.forget_file(&name)?;
            (0, Carry::default())
        }
    };
    if metadata.len() < offset {
        history.forget_file(&name)?;
        offset = 0;
        carry = Carry::default();
    }
    if metadata.len() == offset {
        return Ok(Pass::default());
    }
    let want = (metadata.len() - offset).min(slice);
    let mut file = std::fs::File::open(path).map_err(|error| error.to_string())?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|error| error.to_string())?;
    let mut bytes = Vec::with_capacity(want as usize);
    (&mut file)
        .take(want)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    let (events, used) = read(&bytes, &mut carry, refusals);
    // A slice that ends inside one enormous line, a pasted image say,
    // reads nothing: the line is stepped past, since no turn is that long,
    // or waited for while it's still being written.
    let used = if used == 0 && want == slice {
        match line_end(&mut file, offset, slice)? {
            Some(end) => {
                eprintln!("{name}: passed over a line of {} bytes", end - offset);
                (end - offset) as usize
            }
            None => return Ok(Pass::default()),
        }
    } else {
        used
    };
    let carried = serde_json::to_string(&carry).map_err(|error| error.to_string())?;
    let turns = history.within(|| {
        let mut turns = 0;
        for event in &events {
            history.add_event(event)?;
            turns += u64::from(matches!(event, crate::logs::Event::Turn(_)));
        }
        history.set_file(
            &name,
            &identity,
            offset + used as u64,
            &carried,
            carry.session.as_deref(),
        )?;
        Ok(turns)
    })?;
    Ok(Pass {
        bytes: used as u64,
        turns,
        behind: offset + (used as u64) < metadata.len() && want == slice,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn turns(history: &History) -> i64 {
        history
            .db
            .query_row("SELECT COUNT(*) FROM turns", [], |row| row.get(0))
            .unwrap()
    }

    const HEADER: &str =
        r#"{"type":"session","id":"s1","timestamp":"2026-10-14T10:00:00Z","cwd":"/x"}"#;
    const TURN: &str = concat!(
        r#"{"type":"message","timestamp":"2026-10-14T10:01:00Z","#,
        r#""message":{"role":"assistant","model":"m","#,
        r#""usage":{"input":1,"output":1,"cacheRead":0,"cacheWrite":0,"cost":{"total":0.5}}}}"#
    );

    #[test]
    fn a_growing_log_is_read_once() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("project")).unwrap();
        let path = dir.path().join("project/2026_s1.jsonl");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, "{HEADER}\n{TURN}").unwrap();
        let history = History::memory();
        scan(&history, dir.path(), &[]).unwrap();
        scan(&history, dir.path(), &[]).unwrap();
        assert_eq!(turns(&history), 1);
        writeln!(file, "{TURN}").unwrap();
        scan(&history, dir.path(), &[]).unwrap();
        assert_eq!(turns(&history), 2);
    }

    #[test]
    fn a_replaced_log_is_read_again() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("project")).unwrap();
        let path = dir.path().join("project/2026_s1.jsonl");
        std::fs::write(&path, format!("{HEADER}\n{TURN}\n{TURN}\n")).unwrap();
        let history = History::memory();
        scan(&history, dir.path(), &[]).unwrap();
        assert_eq!(turns(&history), 2);
        // Written anew, as a rewrite of the session would.
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, format!("{HEADER}\n{TURN}\n")).unwrap();
        scan(&history, dir.path(), &[]).unwrap();
        assert_eq!(turns(&history), 1);
    }

    // A line longer than a slice is stepped past, and the turns after it
    // are read; one still being written is waited for.
    #[test]
    fn a_line_longer_than_a_slice_is_passed_over() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("2026_s1.jsonl");
        let long = format!(r#"{{"type":"custom","data":"{}"}}"#, "x".repeat(2000));
        std::fs::write(&path, format!("{HEADER}\n{TURN}\n{long}\n{TURN}\n{long}")).unwrap();
        let history = History::memory();
        let slice = 512;
        for _ in 0..20 {
            scan_slice(&history, &path, &[], slice).unwrap();
        }
        assert_eq!(turns(&history), 2);
        assert_eq!(
            scan_slice(&history, &path, &[], slice).unwrap(),
            Pass::default()
        );
    }
}
