//! Reading pi's logs into the history, from where each stopped. The first
//! pass reads every log there is, a slice at a time so the daemon keeps
//! answering while it does; after that, only what each log grew by.
//!
//! The logs are a person's own sessions, and the agents' that the config
//! finds by pattern: a swarm's, a review council's. Where an agent's log
//! was found says what kind of work it is.

use crate::config::Config;
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

/// A log to read, and the kind of work it is where an agent's it is.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub path: PathBuf,
    pub kind: Option<String>,
}

/// What every log is read with.
#[derive(Debug, Clone, Copy, Default)]
pub struct Reading<'a> {
    pub refusals: &'a [String],
    pub workspaces: Option<&'a Path>,
}

impl<'a> Reading<'a> {
    pub fn of(config: &'a Config) -> Reading<'a> {
        Reading {
            refusals: &config.refusals,
            workspaces: config.workspaces.as_deref(),
        }
    }
}

/// Every log the config names: the person's own, then each agents'
/// pattern's in turn. A log two patterns find is read once, as the first.
pub fn found(config: &Config) -> Vec<Found> {
    let mut seen = std::collections::HashSet::new();
    let own = logs(&config.sessions)
        .into_iter()
        .map(|path| Found { path, kind: None });
    let agents = config.agents.iter().flat_map(|agents| {
        matching(&agents.pattern).into_iter().map(|path| Found {
            path,
            kind: Some(agents.kind.clone()),
        })
    });
    own.chain(agents)
        .filter(|found| seen.insert(found.path.clone()))
        .collect()
}

/// The directories agents' logs are in, to watch.
pub fn agents_directories(config: &Config) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = config
        .agents
        .iter()
        .flat_map(|agents| matching(&agents.pattern))
        .filter_map(|path| path.parent().map(Path::to_path_buf))
        .collect();
    dirs.sort();
    dirs.dedup();
    dirs
}

/// The files a pattern names, a `*` in a part standing for any run of
/// characters within one name, sorted as `logs` sorts. Only the
/// directories the pattern passes through are read, so a large tree beside
/// them costs nothing.
pub fn matching(pattern: &str) -> Vec<PathBuf> {
    let mut at = vec![PathBuf::from("/")];
    for part in pattern.split('/').filter(|part| !part.is_empty()) {
        at = if part.contains('*') {
            at.iter()
                .flat_map(|dir| std::fs::read_dir(dir).into_iter().flatten().flatten())
                .filter(|entry| fits(part, &entry.file_name().to_string_lossy()))
                .map(|entry| entry.path())
                .collect()
        } else {
            at.iter().map(|dir| dir.join(part)).collect()
        };
    }
    let mut found: Vec<PathBuf> = at.into_iter().filter(|path| path.is_file()).collect();
    found.sort_by(|one, other| one.file_name().cmp(&other.file_name()));
    found
}

/// Whether `name` fits `part`, each `*` in it any run of characters.
fn fits(part: &str, name: &str) -> bool {
    let mut pieces = part.split('*');
    let first = pieces.next().unwrap_or("");
    let Some(mut rest) = name.strip_prefix(first) else {
        return false;
    };
    let pieces: Vec<&str> = pieces.collect();
    for (index, piece) in pieces.iter().enumerate() {
        if index + 1 == pieces.len() {
            return rest.len() >= piece.len() && rest.ends_with(piece);
        }
        match rest.find(piece) {
            Some(at) => rest = &rest[at + piece.len()..],
            None => return false,
        }
    }
    rest.is_empty()
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
pub fn scan(history: &History, logs: &[Found], reading: Reading) -> Result<Pass, String> {
    let mut total = Pass::default();
    for found in logs {
        match scan_one(history, found, reading) {
            Ok(one) => {
                total.bytes += one.bytes;
                total.turns += one.turns;
                total.behind |= one.behind;
            }
            Err(error) => eprintln!("reading {}: {error}", found.path.display()),
        }
    }
    Ok(total)
}

pub fn scan_one(history: &History, found: &Found, reading: Reading) -> Result<Pass, String> {
    scan_slice(history, found, reading, SLICE)
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
    found: &Found,
    reading: Reading,
    slice: u64,
) -> Result<Pass, String> {
    let path = found.path.as_path();
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
    carry.kind = found.kind.clone();
    carry.workspaces = reading
        .workspaces
        .map(|workspaces| workspaces.to_string_lossy().into_owned());
    let (events, used) = read(&bytes, &mut carry, reading.refusals);
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
        scan(&history, &own(dir.path()), Reading::default()).unwrap();
        scan(&history, &own(dir.path()), Reading::default()).unwrap();
        assert_eq!(turns(&history), 1);
        writeln!(file, "{TURN}").unwrap();
        scan(&history, &own(dir.path()), Reading::default()).unwrap();
        assert_eq!(turns(&history), 2);
    }

    #[test]
    fn a_replaced_log_is_read_again() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("project")).unwrap();
        let path = dir.path().join("project/2026_s1.jsonl");
        std::fs::write(&path, format!("{HEADER}\n{TURN}\n{TURN}\n")).unwrap();
        let history = History::memory();
        scan(&history, &own(dir.path()), Reading::default()).unwrap();
        assert_eq!(turns(&history), 2);
        // Written anew, as a rewrite of the session would.
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, format!("{HEADER}\n{TURN}\n")).unwrap();
        scan(&history, &own(dir.path()), Reading::default()).unwrap();
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
        let found = Found { path, kind: None };
        for _ in 0..20 {
            scan_slice(&history, &found, Reading::default(), slice).unwrap();
        }
        assert_eq!(turns(&history), 2);
        assert_eq!(
            scan_slice(&history, &found, Reading::default(), slice).unwrap(),
            Pass::default()
        );
    }

    fn own(dir: &Path) -> Vec<Found> {
        logs(dir)
            .into_iter()
            .map(|path| Found { path, kind: None })
            .collect()
    }

    #[test]
    fn a_part_with_stars_fits_what_it_says() {
        assert!(fits("*", "anything"));
        assert!(fits("*.jsonl", "2026_s1.jsonl"));
        assert!(!fits("*.jsonl", "agent.log"));
        assert!(fits("council-*", "council-20261014"));
        assert!(fits("a*b*c", "a-b-c"));
        assert!(!fits("a*b*c", "a-c-b"));
        assert!(!fits("a*a", "a"));
    }

    // A pattern reads only the directories it passes through, and finds
    // each agent's log under them.
    #[test]
    fn a_pattern_finds_agents_logs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for agent in ["q1/hub/agents/a1", "q2/demo/agents/b1"] {
            std::fs::create_dir_all(root.join(agent).join("session")).unwrap();
            std::fs::write(root.join(agent).join("session/2026_x.jsonl"), "").unwrap();
            std::fs::write(root.join(agent).join("agent.log"), "").unwrap();
        }
        std::fs::create_dir_all(root.join("q1/src/deep/agents/c/session")).unwrap();
        std::fs::write(root.join("q1/src/deep/agents/c/session/2026_y.jsonl"), "").unwrap();
        let pattern = format!("{}/*/*/agents/*/session/*.jsonl", root.display());
        let found: Vec<String> = matching(&pattern)
            .iter()
            .map(|path| path.strip_prefix(root).unwrap().display().to_string())
            .collect();
        assert_eq!(
            found,
            [
                "q1/hub/agents/a1/session/2026_x.jsonl",
                "q2/demo/agents/b1/session/2026_x.jsonl",
            ]
        );
    }

    // An agent's log is read as its kind, in its workspace's quest.
    #[test]
    fn an_agent_s_log_is_its_kind_s() {
        let dir = tempfile::tempdir().unwrap();
        let session = dir.path().join("QEST-2/hub/agents/a1/session");
        std::fs::create_dir_all(&session).unwrap();
        let header = HEADER.replace("\"/x\"", &format!("{:?}", session.display().to_string()));
        std::fs::write(session.join("2026_s1.jsonl"), format!("{header}\n{TURN}\n")).unwrap();
        let history = History::memory();
        let logs = [Found {
            path: session.join("2026_s1.jsonl"),
            kind: Some("swarm".into()),
        }];
        let reading = Reading {
            refusals: &[],
            workspaces: Some(dir.path()),
        };
        scan(&history, &logs, reading).unwrap();
        let kind: String = history
            .db
            .query_row("SELECT kind FROM sessions WHERE id = 's1'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(kind, "swarm");
        let quest: String = history
            .db
            .query_row("SELECT quest FROM turns", [], |row| row.get(0))
            .unwrap();
        assert_eq!(quest, "QEST-2");
    }
}
