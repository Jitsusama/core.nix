//! Reading pi's logs into the history, from where each stopped. The first
//! pass reads every log there is, a slice at a time so the daemon keeps
//! answering while it does; after that, only what each log grew by.
//!
//! The logs are a person's own sessions, and the agents' that the config
//! finds by pattern: a swarm's, a review council's. Where an agent's log
//! was found says what kind of work it is. Another machine's, brought
//! here, are read the same way and are that machine's.
//!
//! Long because more than half of it is the tests beside it, each of which
//! writes logs to a folder and reads them through the same passes the
//! daemon runs.

use crate::config::Config;
use crate::history::History;
use crate::logs::{Ancestry, Carry, ancestry, read_after};
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

/// A log to read, the kind of work it is where an agent's it is, and the
/// machine it was written on where that's another.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Found {
    pub path: PathBuf,
    pub kind: Option<String>,
    pub machine: Option<String>,
    /// Where another machine's sessions are brought, so a log carried
    /// from a parent there finds it there; none for this machine's.
    pub brought: Option<PathBuf>,
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
/// pattern's in turn, then each other machine's the same way. A log two
/// patterns find is read once, as the first.
pub fn found(config: &Config) -> Vec<Found> {
    let mut seen = std::collections::HashSet::new();
    let here = machine_logs(&config.sessions, &config.agents, None);
    let elsewhere = config
        .machines
        .iter()
        .flat_map(|machine| machine_logs(&machine.sessions, &machine.agents, Some(&machine.name)));
    here.chain(elsewhere)
        .filter(|found| seen.insert(found.path.clone()))
        .collect()
}

/// One machine's logs: its own sessions', then each agents' pattern's.
fn machine_logs<'a>(
    sessions: &Path,
    agents: &'a [crate::config::AgentLogs],
    machine: Option<&'a String>,
) -> impl Iterator<Item = Found> + 'a {
    let brought = machine.map(|_| sessions.to_path_buf());
    let own_brought = brought.clone();
    let own = logs(sessions)
        .into_iter()
        .filter(move |path| machine.is_none() || !linked(path))
        .map(move |path| Found {
            path,
            kind: None,
            machine: machine.cloned(),
            brought: own_brought.clone(),
        });
    let agents = agents.iter().flat_map(move |agents| {
        let brought = brought.clone();
        matching(&agents.pattern)
            .into_iter()
            .map(move |path| Found {
                path,
                kind: Some(agents.kind.clone()),
                machine: machine.cloned(),
                brought: brought.clone(),
            })
    });
    own.chain(agents)
}

/// Whether a log, or the project folder it's in, is a link. Another
/// machine's logs are brought as files of their own, so a link there would
/// only find a log already counted, this machine's or another's, and count
/// it again under that machine's name.
fn linked(path: &Path) -> bool {
    let link = |path: &Path| {
        std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink())
    };
    link(path) || path.parent().is_some_and(link)
}

/// The directories logs are in beyond this machine's own sessions, to
/// watch: its agents', and every other machine's sessions' and agents'.
pub fn followed_directories(config: &Config) -> Vec<PathBuf> {
    let patterns = config
        .agents
        .iter()
        .chain(config.machines.iter().flat_map(|machine| &machine.agents));
    let machines = config
        .machines
        .iter()
        .flat_map(|machine| logs(&machine.sessions));
    let mut dirs: Vec<PathBuf> = patterns
        .flat_map(|agents| matching(&agents.pattern))
        .chain(machines)
        .filter_map(|path| path.parent().map(Path::to_path_buf))
        .chain(
            config
                .machines
                .iter()
                .map(|machine| machine.sessions.clone()),
        )
        .collect();
    dirs.sort();
    dirs.dedup();
    dirs
}

/// The files a pattern names, a `*` in a part standing for any run of
/// characters within one name, sorted as `logs` sorts. Only the
/// directories the pattern passes through are read, so a large tree beside
/// them costs nothing, and a link is followed only where it stays within
/// the pattern's root, the folder its parts name before the first `*`.
pub fn matching(pattern: &str) -> Vec<PathBuf> {
    let parts: Vec<&str> = pattern.split('/').filter(|part| !part.is_empty()).collect();
    let fixed: PathBuf = std::iter::once("/")
        .chain(parts.iter().copied().take_while(|part| !part.contains('*')))
        .collect();
    let Ok(root) = fixed.canonicalize() else {
        return Vec::new();
    };
    let mut at = vec![PathBuf::from("/")];
    for (index, part) in parts.iter().enumerate() {
        at = step(&at, part, index + 1 == parts.len(), &root);
    }
    let mut found: Vec<PathBuf> = at.into_iter().filter(|path| path.is_file()).collect();
    oldest_first(&mut found);
    found
}

/// What one part of a pattern reaches from each of `at`. Before the last
/// part only a folder can lead on, or a link to one within `root`, so
/// nothing else is kept: the entry's own type says which without a look at
/// the file, and scratch folders beside the logs hold millions of files.
/// A link leading out is left, since quests' scratch folders link to
/// shared caches and the Nix store, which hold no agent's log.
fn step(at: &[PathBuf], part: &str, last: bool, root: &Path) -> Vec<PathBuf> {
    if !part.contains('*') {
        return at.iter().map(|dir| dir.join(part)).collect();
    }
    at.iter()
        .flat_map(|dir| std::fs::read_dir(dir).into_iter().flatten().flatten())
        .filter(|entry| {
            last || entry.file_type().is_ok_and(|kind| {
                kind.is_dir()
                    || (kind.is_symlink()
                        && entry
                            .path()
                            .canonicalize()
                            .is_ok_and(|to| to.is_dir() && to.starts_with(root)))
            })
        })
        .filter(|entry| fits(part, &entry.file_name().to_string_lossy()))
        .map(|entry| entry.path())
        .collect()
}

/// pi names a log by when it began, so by name is oldest first. Two logs
/// can share a name in different directories, and then the whole path
/// decides, rather than the order a filesystem happens to list them in.
fn oldest_first(found: &mut [PathBuf]) {
    found.sort_by(|one, other| {
        one.file_name()
            .cmp(&other.file_name())
            .then_with(|| one.cmp(other))
    });
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
    oldest_first(&mut found);
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

/// A log's first line, read as its header.
fn header(path: &Path) -> Option<serde_json::Value> {
    let file = std::fs::File::open(path).ok()?;
    let mut line = Vec::new();
    std::io::BufRead::read_until(&mut std::io::BufReader::new(file), b'\n', &mut line).ok()?;
    serde_json::from_slice(&line).ok()
}

/// Whether a new log may take its session. Turns are kept by session, so
/// a second log naming one already held would count its turns twice, and
/// reading either again would forget the other's. A log moved, as when a
/// machine is renamed or its logs are brought somewhere new, takes the
/// session over from the path that's gone; one beside a log still there
/// is left unread and said, since a session copied to go on elsewhere is
/// counted where it began until it's carried into a session of its own.
fn claim(history: &History, path: &Path, name: &str) -> Result<bool, String> {
    let session = header(path).and_then(|header| {
        (header.get("type").and_then(serde_json::Value::as_str) == Some("session"))
            .then(|| header.get("id")?.as_str().map(str::to_owned))
            .flatten()
    });
    let Some(held) = session
        .as_deref()
        .filter(|session| !session.is_empty())
        .and_then(|session| history.held_by(session, name))
    else {
        return Ok(true);
    };
    if Path::new(&held).exists() {
        eprintln!("not reading {name}: its session's turns are {held}'s");
        return Ok(false);
    }
    history.forget_file(&held)?;
    Ok(true)
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
    let (mut offset, mut carry) = if let Some((offset, carry)) = history.file(&name, &identity) {
        (offset, serde_json::from_str(&carry).unwrap_or_default())
    } else {
        // A new log, or one replaced under the same name: start over.
        history.forget_file(&name)?;
        if !claim(history, path, &name)? {
            return Ok(Pass::default());
        }
        (0, Carry::default())
    };
    if metadata.len() < offset {
        history.forget_file(&name)?;
        offset = 0;
        carry = Carry::default();
    }
    if offset > 0 && !carry.copies_known {
        // Held from before copies were told apart, or brought before its
        // parent: a carried log is read again for its own turns, one still
        // waiting on its parent is left to be checked again, and any other
        // is marked as checked.
        let header = header(path);
        let brought = found.brought.as_deref();
        if header
            .as_ref()
            .is_some_and(|header| crate::logs::carried(header, brought))
        {
            history.forget_file(&name)?;
            offset = 0;
            carry = Carry::default();
        } else if !header
            .as_ref()
            .is_some_and(|header| crate::logs::awaits_parent(header, brought))
        {
            carry.copies_known = true;
            let carried = serde_json::to_string(&carry).map_err(|error| error.to_string())?;
            history.set_file(&name, &identity, offset, &carried, carry.session.as_deref())?;
        }
    }
    if metadata.len() == offset {
        return Ok(Pass::default());
    }
    let want = (metadata.len() - offset).min(slice);
    let mut file = std::fs::File::open(path).map_err(|error| error.to_string())?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|error| error.to_string())?;
    // Only a hint: a slice always fits in memory's addresses.
    let mut bytes = Vec::with_capacity(usize::try_from(want).unwrap_or(0));
    (&mut file)
        .take(want)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    carry.kind.clone_from(&found.kind);
    carry.machine.clone_from(&found.machine);
    carry.brought = found
        .brought
        .as_ref()
        .map(|brought| brought.to_string_lossy().into_owned());
    carry.workspaces = reading
        .workspaces
        .map(|workspaces| workspaces.to_string_lossy().into_owned());
    // A branch taken from further back is found in what the log held
    // before this read, which is read again only then.
    let earlier = || {
        std::fs::File::open(path).map_or_else(
            |_| Ancestry::default(),
            |file| ancestry(std::io::BufReader::new(file.take(offset))),
        )
    };
    let (events, used) = read_after(&bytes, &mut carry, reading.refusals, &earlier);
    // A slice that ends inside one enormous line, a pasted image say,
    // reads nothing: the line is stepped past, since no turn is that long,
    // or waited for while it's still being written.
    let used = if used == 0 && want == slice {
        match line_end(&mut file, offset, slice)? {
            Some(end) => {
                eprintln!("{name}: passed over a line of {} bytes", end - offset);
                usize::try_from(end - offset).map_err(|error| error.to_string())?
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

    // A log written anew keeps no budget it couldn't draw on from before,
    // and reading it again doesn't write one twice.
    #[test]
    fn a_replaced_log_tells_its_unresolved_budgets_once() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("project")).unwrap();
        let path = dir.path().join("project/2026_s1.jsonl");
        let missed = |budget: &str| {
            serde_json::json!({
                "type": "custom", "customType": "budget", "id": budget,
                "timestamp": "2026-10-14T10:02:00Z",
                "data": {"v": 1, "name": null,
                    "unresolved": {"budget": budget, "missing": "no key"}},
            })
            .to_string()
        };
        let unresolved = |history: &History| -> Vec<String> {
            let mut statement = history
                .db
                .prepare("SELECT budget FROM unresolved ORDER BY budget")
                .unwrap();
            statement
                .query_map([], |row| row.get(0))
                .unwrap()
                .map(Result::unwrap)
                .collect()
        };
        let teal = missed("teal");
        std::fs::write(&path, format!("{HEADER}\n{TURN}\n{teal}\n")).unwrap();
        let history = History::memory();
        scan(&history, &own(dir.path()), Reading::default()).unwrap();
        assert_eq!(unresolved(&history), ["teal"]);
        // Written anew, as a rewrite of the session would.
        std::fs::remove_file(&path).unwrap();
        let ochre = missed("ochre");
        std::fs::write(&path, format!("{HEADER}\n{ochre}\n")).unwrap();
        scan(&history, &own(dir.path()), Reading::default()).unwrap();
        assert_eq!(unresolved(&history), ["ochre"]);
    }

    // A log carried from a parent on another machine names that parent by
    // the other machine's path; brought here, the parent is found in the
    // same project folder, so what was copied is counted once.
    #[test]
    fn a_brought_copy_finds_its_parent_among_the_brought() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("teal/project");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(project.join("2026_s1.jsonl"), format!("{HEADER}\n{TURN}\n")).unwrap();
        let header = serde_json::json!({
            "type": "session", "id": "s2", "timestamp": "2026-10-14T10:30:00Z",
            "cwd": "/y", "parentSession": "/home/elsewhere/sessions/project/2026_s1.jsonl",
        });
        let later = TURN.replace("10:01:00", "10:31:00");
        std::fs::write(
            project.join("2026_s2.jsonl"),
            format!("{header}\n{TURN}\n{later}\n"),
        )
        .unwrap();
        let history = History::memory();
        let teal = "teal".to_owned();
        let logs: Vec<Found> = machine_logs(&dir.path().join("teal"), &[], Some(&teal)).collect();
        scan(&history, &logs, Reading::default()).unwrap();
        assert_eq!(
            turns(&history),
            2,
            "a brought copy's copied turn was counted beside its parent's"
        );
    }

    // Logs are brought in whatever order the transport keeps, so a copy can
    // land before its parent. It counts whole until then, its parent is
    // never taken from this machine's path, and once the parent lands the
    // copy is read again for its own turns alone.
    #[test]
    fn a_brought_copy_counts_once_when_its_parent_lands_after_it() {
        let dir = tempfile::tempdir().unwrap();
        let local = dir.path().join("here/project");
        std::fs::create_dir_all(&local).unwrap();
        std::fs::write(local.join("2026_s1.jsonl"), format!("{HEADER}\n{TURN}\n")).unwrap();
        let project = dir.path().join("teal/project");
        std::fs::create_dir_all(&project).unwrap();
        let header = serde_json::json!({
            "type": "session", "id": "s2", "timestamp": "2026-10-14T10:30:00Z",
            "cwd": "/y", "parentSession": local.join("2026_s1.jsonl"),
        });
        let later = TURN.replace("10:01:00", "10:31:00");
        std::fs::write(
            project.join("2026_s2.jsonl"),
            format!("{header}\n{TURN}\n{later}\n"),
        )
        .unwrap();
        let history = History::memory();
        let teal = "teal".to_owned();
        let logs =
            || -> Vec<Found> { machine_logs(&dir.path().join("teal"), &[], Some(&teal)).collect() };
        scan(&history, &logs(), Reading::default()).unwrap();
        assert_eq!(
            turns(&history),
            2,
            "a brought copy without its parent lost turns, or took this machine's log as it"
        );
        std::fs::write(project.join("2026_s1.jsonl"), format!("{HEADER}\n{TURN}\n")).unwrap();
        scan(&history, &logs(), Reading::default()).unwrap();
        assert_eq!(
            turns(&history),
            2,
            "a brought copy's copied turn was counted again once its parent landed"
        );
    }

    // A history from before copies were told apart holds a carried log's
    // copied turns beside its parent's; the log is read again once.
    #[test]
    fn a_copy_held_from_before_is_read_again_for_its_own() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("project")).unwrap();
        let parent = dir.path().join("project/2026_s1.jsonl");
        let child = dir.path().join("project/2026_s2.jsonl");
        let header = serde_json::json!({
            "type": "session", "id": "s2", "timestamp": "2026-10-14T10:30:00Z",
            "cwd": "/y", "parentSession": parent.to_string_lossy(),
        });
        let later = TURN.replace("10:01:00", "10:31:00");
        std::fs::write(&child, format!("{header}\n{TURN}\n{later}\n")).unwrap();
        let history = History::memory();
        // Before its parent's log is there, the copy is the only record.
        scan(&history, &own(dir.path()), Reading::default()).unwrap();
        assert_eq!(turns(&history), 2);
        history
            .db
            .execute("UPDATE files SET carry = '{\"session\":\"s2\"}'", [])
            .unwrap();
        std::fs::write(&parent, format!("{HEADER}\n{TURN}\n")).unwrap();
        scan(&history, &own(dir.path()), Reading::default()).unwrap();
        scan(&history, &own(dir.path()), Reading::default()).unwrap();
        assert_eq!(turns(&history), 2);
        let sessions: Vec<String> = history
            .db
            .prepare("SELECT session FROM turns ORDER BY session")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(sessions, ["s1", "s2"]);
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
        let found = Found {
            path,
            ..Found::default()
        };
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
            .map(|path| Found {
                path,
                ..Found::default()
            })
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
        // Made out of order, since a directory lists in whatever order its
        // filesystem keeps: some by when, some by a hash of the name.
        for agent in ["q2/demo/agents/b1", "q1/hub/agents/a1"] {
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

    // On 31 October one part of the agents' patterns reached 3,064,197
    // entries in quests' scratch folders, every 30 s, and the daemon grew
    // by up to 2.4 GB holding them. Only a folder, or a link that may be
    // one, can lead anywhere before the last part.
    #[test]
    fn a_part_before_the_last_keeps_only_what_could_be_a_folder() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir(root.join("folder")).unwrap();
        std::fs::write(root.join("file"), "").unwrap();
        std::os::unix::fs::symlink(root.join("folder"), root.join("link")).unwrap();
        let names = |last| {
            let within = root.canonicalize().unwrap();
            let mut names: Vec<String> = step(&[root.to_path_buf()], "*", last, &within)
                .iter()
                .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
                .collect();
            names.sort();
            names
        };
        assert_eq!(names(false), ["folder", "link"]);
        assert_eq!(names(true), ["file", "folder", "link"]);
    }

    // On 6 November 96 quests' scratch folders each linked to one shared
    // cache of 265,040 files, so one walk of the agents' patterns read
    // 26.7 million entries, about 20 s of a core, and the daemon ran one
    // every 30 s. A link out of the pattern's root is never followed.
    #[test]
    fn a_link_out_of_the_pattern_s_root_is_not_followed() {
        let dir = tempfile::tempdir().unwrap();
        let (root, outside) = (dir.path().join("root"), dir.path().join("cache"));
        for agent in ["q1/tmp/run/agents/a1", "q2/tmp/run/agents/b1"] {
            std::fs::create_dir_all(root.join(agent).join("session")).unwrap();
            std::fs::write(root.join(agent).join("session/2026_x.jsonl"), "").unwrap();
        }
        std::fs::create_dir_all(outside.join("run/agents/c1/session")).unwrap();
        std::fs::write(outside.join("run/agents/c1/session/2026_y.jsonl"), "").unwrap();
        std::os::unix::fs::symlink(&outside, root.join("q1/tmp/cache")).unwrap();
        std::os::unix::fs::symlink(root.join("q2/tmp/run"), root.join("q1/tmp/near")).unwrap();
        let pattern = format!("{}/*/tmp/*/*/agents/*/session/*.jsonl", root.display());
        assert_eq!(matching(&pattern), Vec::<PathBuf>::new());
        let pattern = format!("{}/*/tmp/*/agents/*/session/*.jsonl", root.display());
        let found: Vec<String> = matching(&pattern)
            .iter()
            .map(|path| path.strip_prefix(&root).unwrap().display().to_string())
            .collect();
        assert_eq!(
            found,
            [
                "q1/tmp/near/agents/b1/session/2026_x.jsonl",
                "q1/tmp/run/agents/a1/session/2026_x.jsonl",
                "q2/tmp/run/agents/b1/session/2026_x.jsonl",
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
            machine: None,
            brought: None,
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

    // Another machine's logs, brought here, are found as this one's are,
    // its agents' as their kind, and each session is kept as that
    // machine's; this machine's own are kept as nobody else's.
    #[test]
    fn another_machine_s_logs_are_that_machine_s() {
        let dir = tempfile::tempdir().unwrap();
        let write = |at: &str, id: &str| {
            let path = dir.path().join(at);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            let header = HEADER.replace("\"s1\"", &format!("{id:?}"));
            std::fs::write(&path, format!("{header}\n{TURN}\n")).unwrap();
        };
        write("here/project/2026_a.jsonl", "here");
        write("teal/sessions/project/2026_b.jsonl", "there");
        write("teal/runs/r1/session/2026_c.jsonl", "its-reviewer");
        let root = dir.path().display();
        let config: Config = serde_json::from_value(serde_json::json!({
            "sessions": format!("{root}/here"), "runs": "/r", "registry": "/g",
            "quests": "/q", "state": "/t", "machine": "slate",
            "machines": [{"name": "teal", "sessions": format!("{root}/teal/sessions"),
                "agents": [{"pattern": format!("{root}/teal/runs/*/session/*.jsonl"),
                    "kind": "review"}]}],
        }))
        .unwrap();
        let history = History::memory();
        scan(&history, &found(&config), Reading::default()).unwrap();
        let sessions: Vec<(String, Option<String>, Option<String>)> = history
            .db
            .prepare("SELECT id, kind, machine FROM sessions ORDER BY id")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(
            sessions,
            [
                ("here".into(), None, None),
                (
                    "its-reviewer".into(),
                    Some("review".into()),
                    Some("teal".into())
                ),
                ("there".into(), None, Some("teal".into())),
            ],
            "a machine's logs weren't found, or weren't kept as its own"
        );
        assert_eq!(turns(&history), 3);
        let watched = followed_directories(&config);
        for folder in [
            "teal/sessions",
            "teal/sessions/project",
            "teal/runs/r1/session",
        ] {
            assert!(
                watched.contains(&dir.path().join(folder)),
                "{folder} wasn't watched: {watched:?}"
            );
        }
    }

    // A link among another machine's logs finds a log already counted, so
    // it's passed over rather than counted again under that machine.
    #[test]
    fn a_link_among_another_machine_s_logs_is_passed_over() {
        let dir = tempfile::tempdir().unwrap();
        let here = dir.path().join("here/project");
        std::fs::create_dir_all(&here).unwrap();
        std::fs::write(here.join("2026_a.jsonl"), format!("{HEADER}\n{TURN}\n")).unwrap();
        let teal = dir.path().join("teal");
        std::fs::create_dir_all(teal.join("files")).unwrap();
        std::os::unix::fs::symlink(&here, teal.join("project")).unwrap();
        std::os::unix::fs::symlink(here.join("2026_a.jsonl"), teal.join("files/2026_b.jsonl"))
            .unwrap();
        let name = "teal".to_owned();
        let brought: Vec<Found> = machine_logs(&teal, &[], Some(&name)).collect();
        assert!(
            brought.is_empty(),
            "a link among another machine's logs was read: {brought:?}"
        );
    }

    // Turns are kept by session, so a second log naming a session already
    // held is left unread while the first is there: a session copied to go
    // on elsewhere counts where it began.
    #[test]
    fn a_session_held_by_a_log_still_there_is_read_once() {
        let dir = tempfile::tempdir().unwrap();
        let at = |folder: &str| {
            let project = dir.path().join(folder).join("project");
            std::fs::create_dir_all(&project).unwrap();
            std::fs::write(project.join("2026_s1.jsonl"), format!("{HEADER}\n{TURN}\n")).unwrap();
        };
        at("here");
        at("teal");
        let history = History::memory();
        let teal = "teal".to_owned();
        let mut logs = own(&dir.path().join("here"));
        logs.extend(machine_logs(&dir.path().join("teal"), &[], Some(&teal)));
        scan(&history, &logs, Reading::default()).unwrap();
        scan(&history, &logs, Reading::default()).unwrap();
        assert_eq!(
            turns(&history),
            1,
            "a session two logs name was counted twice"
        );
        let machine: Option<String> = history
            .db
            .query_row("SELECT machine FROM sessions WHERE id = 's1'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(
            machine, None,
            "a copy brought later took the session's machine"
        );
    }

    // A machine renamed, or its logs brought somewhere new, moves every
    // log's path; each session is taken over from the path that's gone
    // rather than counted again.
    #[test]
    fn logs_brought_somewhere_new_are_counted_once() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("teal/project");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(project.join("2026_s1.jsonl"), format!("{HEADER}\n{TURN}\n")).unwrap();
        let history = History::memory();
        let (teal, ochre) = ("teal".to_owned(), "ochre".to_owned());
        let logs: Vec<Found> = machine_logs(&dir.path().join("teal"), &[], Some(&teal)).collect();
        scan(&history, &logs, Reading::default()).unwrap();
        std::fs::rename(dir.path().join("teal"), dir.path().join("ochre")).unwrap();
        let logs: Vec<Found> = machine_logs(&dir.path().join("ochre"), &[], Some(&ochre)).collect();
        scan(&history, &logs, Reading::default()).unwrap();
        assert_eq!(
            turns(&history),
            1,
            "a log brought somewhere new was counted again"
        );
    }
}
