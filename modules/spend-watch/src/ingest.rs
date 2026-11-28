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
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// The most read from one log in one pass.
const SLICE: u64 = 32 << 20;

/// How long a folder at a pattern's first `*` has gone unwritten, every
/// log in it read to its end, before the walks stop going into it. A
/// council's round takes minutes and its retries come within the hour.
const QUIET: Duration = Duration::from_hours(24);

/// How often every folder is walked again, settled or not, for what
/// neither a watch nor a folder's time shows: a log made in a folder that
/// held none, which no watch follows and whose time no pass reads.
const SWEEP: Duration = Duration::from_hours(6);

/// What the walks know can't have moved: each folder at a pattern's first
/// `*`, a council's run or a quest's workspace, whose logs were all read
/// to their ends and which nothing has written for a day, with the logs
/// it holds. On 22 November a walk read 36,295 folders to find 1,283
/// reviewers' logs, nearly all in runs weeks over, and took 42 s under
/// load: a pass walked at least once and the watch again every 30 s, so
/// the watch read 3 to 5 MB a second of the filesystem's records all
/// day. A settled folder is walked again when a folder the walk read in
/// it changes (a reviewer asked again makes one), when the watch hears a
/// write under it (`touched`), and at least every six hours.
#[derive(Debug, Default)]
pub struct Walks {
    settled: HashMap<(String, PathBuf), Walked>,
    walked: HashMap<(String, PathBuf), Walked>,
    /// On the wall's clock, as `QUIET` is, so a machine asleep still
    /// sweeps on waking.
    swept: Option<SystemTime>,
}

/// What one walk under a folder at a pattern's first `*` found: the logs,
/// and each folder it read to reach a part past the next, with when that
/// was last written. Those folders are a run's own and its reviewers', two
/// a run, so looking at their times each pass costs a few hundred looks,
/// not the walk's tens of thousands; a log's own folder is the watch's.
#[derive(Debug)]
struct Walked {
    logs: Vec<PathBuf>,
    read: Vec<(PathBuf, Option<SystemTime>)>,
}

impl Walks {
    /// Starts a walk, forgetting what's settled once a sweep is due, and
    /// each settled folder one of whose folders has changed since.
    fn begin(&mut self) {
        self.walked.clear();
        let now = SystemTime::now();
        // A clock set back sweeps too, rather than waiting for it.
        if !self
            .swept
            .is_some_and(|at| now.duration_since(at).is_ok_and(|since| since < SWEEP))
        {
            if !self.settled.is_empty() {
                eprintln!(
                    "walking again the {} folders settled, as each six hours",
                    self.settled.len()
                );
            }
            self.settled.clear();
            self.swept = Some(now);
        }
        self.settled
            .retain(|_, walked| walked.read.iter().all(|(dir, at)| written(dir) == *at));
    }

    /// Walks again each settled folder holding `dir`, or held by it, as
    /// the watch hears something written there.
    pub fn touched(&mut self, dir: &Path) {
        self.settled
            .retain(|(_, top), _| !dir.starts_with(top) && !top.starts_with(dir));
    }

    /// Settles each folder the last walk went into whose logs the history
    /// has read to their ends, none of it written since a day before `now`.
    pub fn settle(&mut self, history: &History, now: SystemTime) {
        for (key, walked) in self.walked.drain() {
            let still = walked
                .read
                .iter()
                .all(|(_, at)| at.is_none_or(|at| aged(at, now)));
            if still
                && walked
                    .logs
                    .iter()
                    .all(|log| read_through(history, log, now))
            {
                self.settled.insert(key, walked);
            }
        }
    }
}

#[cfg(test)]
impl Walks {
    /// Walks with each of `tops` settled for `pattern`, holding no log,
    /// for the daemon's tests.
    pub fn settled_at(pattern: &str, tops: &[&str]) -> Walks {
        let mut walks = Walks::default();
        for top in tops {
            let walked = Walked {
                logs: Vec::new(),
                read: Vec::new(),
            };
            walks
                .settled
                .insert((pattern.to_owned(), PathBuf::from(top)), walked);
        }
        walks
    }

    /// The folders settled, in order.
    pub fn settled_tops(&self) -> Vec<PathBuf> {
        let mut tops: Vec<PathBuf> = self.settled.keys().map(|(_, top)| top.clone()).collect();
        tops.sort();
        tops
    }
}

/// When `path` was last written, none where it isn't there.
fn written(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
}

/// Whether nothing has written `path` for a day before `now`.
fn quiet(path: &Path, now: SystemTime) -> bool {
    written(path).is_some_and(|modified| aged(modified, now))
}

/// Whether `at` is a day or more before `now`. A time so far ahead that a
/// day past it can't be held is not, rather than a panic any agent could
/// set off with `touch` on its own folder.
fn aged(at: SystemTime, now: SystemTime) -> bool {
    at.checked_add(QUIET).is_some_and(|then| then <= now)
}

/// What tells a log from another written under its name since: its inode.
/// Not its device too, as a file's identity usually takes, since btrfs gives
/// each subvolume a device number as it mounts, and the number moves from
/// boot to boot. A kept history held eleven of them on 25 November, and
/// each boot took every log for a new one, forgot it and read it again
/// from its start: 29 GB in the walk a start makes before its first pass.
/// A log written beside and moved over the old one always has another
/// inode. One deleted and made again could be handed the freed number,
/// which the device never told apart within a boot either: it's taken for
/// the old log unless it's shorter than where reading stopped.
fn identity(metadata: &std::fs::Metadata) -> String {
    metadata.ino().to_string()
}

/// Whether a log is quiet and read to its end, its copies known, so a
/// pass would read nothing more of it.
fn read_through(history: &History, log: &Path, now: SystemTime) -> bool {
    let Ok(metadata) = std::fs::metadata(log) else {
        return false;
    };
    let identity = identity(&metadata);
    quiet(log, now)
        && history
            .file(&log.to_string_lossy(), &identity)
            .is_some_and(|(offset, carry)| {
                offset == metadata.len()
                    && serde_json::from_str::<Carry>(&carry).is_ok_and(|carry| carry.copies_known)
            })
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Pass {
    pub bytes: u64,
    pub turns: u64,
    /// Whether any log has more left to read than this pass took.
    pub behind: bool,
    /// Whether a log read is still waiting on the parent it names.
    pub waiting: bool,
}

impl Pass {
    /// Adds what one log's read took, saying whether that log still waits
    /// on its parent.
    fn add(&mut self, found: &Found, one: Result<Pass, String>) -> bool {
        match one {
            Ok(one) => {
                self.bytes += one.bytes;
                self.turns += one.turns;
                self.behind |= one.behind;
                one.waiting
            }
            Err(error) => {
                eprintln!("reading {}: {error}", found.path.display());
                false
            }
        }
    }
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
    found_kept(config, &mut Walks::default())
}

/// Every log the config names, as `found`, going into no folder `walks`
/// has settled.
pub fn found_kept(config: &Config, walks: &mut Walks) -> Vec<Found> {
    walks.begin();
    let mut seen = std::collections::HashSet::new();
    let mut all = machine_logs_kept(&config.sessions, &config.agents, None, walks);
    for machine in &config.machines {
        all.extend(machine_logs_kept(
            &machine.sessions,
            &machine.agents,
            Some(&machine.name),
            walks,
        ));
    }
    all.into_iter()
        .filter(|found| seen.insert(found.path.clone()))
        .collect()
}

/// One machine's logs, as `machine_logs_kept` finds them afresh.
#[cfg(test)]
fn machine_logs(
    sessions: &Path,
    agents: &[crate::config::AgentLogs],
    machine: Option<&String>,
) -> impl Iterator<Item = Found> {
    machine_logs_kept(sessions, agents, machine, &mut Walks::default()).into_iter()
}

/// One machine's logs: its own sessions', then each agents' pattern's.
fn machine_logs_kept(
    sessions: &Path,
    agents: &[crate::config::AgentLogs],
    machine: Option<&String>,
    walks: &mut Walks,
) -> Vec<Found> {
    let brought = machine.map(|_| sessions.to_path_buf());
    let mut all: Vec<Found> = logs(sessions)
        .into_iter()
        .filter(|path| machine.is_none() || !linked(path))
        .map(|path| Found {
            path,
            kind: None,
            machine: machine.cloned(),
            brought: brought.clone(),
        })
        .collect();
    for agents in agents {
        all.extend(
            matching_kept(&agents.pattern, walks)
                .into_iter()
                .filter(|path| machine.is_none() || !linked(path))
                .map(|path| Found {
                    path,
                    kind: Some(agents.kind.clone()),
                    machine: machine.cloned(),
                    brought: brought.clone(),
                }),
        );
    }
    all
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

/// The directories `logs` are in beyond this machine's own sessions, to
/// watch: its agents', and every other machine's sessions' and agents'.
/// They're taken from what a pass found rather than walked for, since a
/// walk is what a pass costs.
pub fn followed_directories(config: &Config, logs: &[Found]) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = logs
        .iter()
        .filter(|found| found.kind.is_some() || found.machine.is_some())
        .filter_map(|found| found.path.parent().map(Path::to_path_buf))
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
    matching_kept(pattern, &mut Walks::default())
}

/// The files a pattern names, as `matching`, going into no folder at its
/// first `*` that `walks` has settled, and noting what's under each other.
fn matching_kept(pattern: &str, walks: &mut Walks) -> Vec<PathBuf> {
    let parts: Vec<&str> = pattern.split('/').filter(|part| !part.is_empty()).collect();
    let fixed: PathBuf = std::iter::once("/")
        .chain(parts.iter().copied().take_while(|part| !part.contains('*')))
        .collect();
    let Ok(root) = fixed.canonicalize() else {
        return Vec::new();
    };
    let Some(last) = parts.len().checked_sub(1) else {
        return Vec::new();
    };
    let first = parts
        .iter()
        .position(|part| part.contains('*'))
        .unwrap_or(last);
    // Walks `from` through `some` of the parts, the first of them the
    // pattern's part number `skipped`.
    let walk = |from: Vec<PathBuf>, some: &[&str], skipped: usize| {
        some.iter().enumerate().fold(from, |at, (index, part)| {
            step(&at, part, skipped + index == last, &root)
        })
    };
    let mut found = Vec::new();
    for top in walk(vec![PathBuf::from("/")], &parts[..=first], 0) {
        // Keyed by the pattern too, so two patterns meeting in one folder
        // keep their own logs and kinds.
        let key = (pattern.to_owned(), top);
        if let Some(walked) = walks.settled.get(&key) {
            found.extend(walked.logs.iter().cloned());
            continue;
        }
        let walked = walk_under(&key.1, &parts, first + 1, &root);
        found.extend(walked.logs.iter().cloned());
        walks.walked.insert(key, walked);
    }
    oldest_first(&mut found);
    found
}

/// Walks `top` through the parts from number `from` to the last, noting
/// the folders read on the way short of the last part's, each with its
/// time taken before it's read, so a folder made between the two shows
/// as a change.
fn walk_under(top: &Path, parts: &[&str], from: usize, root: &Path) -> Walked {
    let last = parts.len() - 1;
    let mut read = vec![(top.to_path_buf(), written(top))];
    let mut at = vec![top.to_path_buf()];
    for (number, part) in parts.iter().enumerate().skip(from) {
        if part.contains('*') && number != last {
            read.extend(at.iter().map(|dir| (dir.clone(), written(dir))));
        }
        at = step(&at, part, number == last, root);
    }
    let logs = at.into_iter().filter(|path| path.is_file()).collect();
    Walked { logs, read }
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
    let mut waiting = Vec::new();
    for found in logs {
        if total.add(found, scan_one(history, found, reading)) {
            waiting.push(found);
        }
    }
    // A copy passed over as waiting on its parent is checked again once
    // every log has been read, so a parent read later in the same scan
    // never leaves this pass counting its turns under the copy as well.
    for found in waiting {
        total.add(found, scan_one(history, found, reading));
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

/// Checks a log held from before copies were told apart, or brought before
/// its parent, and says whether it's to be read again from its start: a
/// carried log is, for its own turns, once what was counted before its
/// header is dropped as its parent's. One still waiting on its parent is
/// left to be checked again, and any other is marked as checked. A brought
/// log's header is the other machine's to write, so for a session of this
/// machine's it drops what was counted only when it began when this
/// machine's own log says, as a copy of it does: a header naming another
/// time can't take what this machine counted. Brought machines are
/// trusted among themselves, as their folders are: a session only
/// elsewhere takes its start from whichever log was last read from its
/// start, so no start held could guard one from another.
fn check_copies(
    history: &History,
    found: &Found,
    identity: &str,
    carry: &mut Carry,
    offset: u64,
) -> Result<bool, String> {
    let name = found.path.to_string_lossy().into_owned();
    let header = header(&found.path);
    let brought = found.brought.as_deref();
    let held = held_parent(history, header.as_ref()).is_some();
    if let Some(header) = header
        .as_ref()
        .filter(|header| crate::logs::copies::carried(header, brought, held))
    {
        let session = header.get("id").and_then(serde_json::Value::as_str);
        let until = header
            .get("timestamp")
            .and_then(serde_json::Value::as_str)
            .and_then(crate::time::parse);
        if let (Some(session), Some(until)) = (session, until)
            && (brought.is_none()
                || history
                    .this_machines_start(session)
                    .is_none_or(|started| started.total_cmp(&until).is_eq()))
        {
            history.forget_copied(session, until)?;
        }
        history.forget_file(&name)?;
        return Ok(true);
    }
    if !header
        .as_ref()
        .is_some_and(|header| crate::logs::copies::awaits_parent(header, brought, held))
    {
        carry.copies_known = true;
        let carried = serde_json::to_string(&*carry).map_err(|error| error.to_string())?;
        history.set_file(&name, identity, offset, &carried, carry.session.as_deref())?;
    }
    Ok(false)
}

/// The session a header names as its parent, when the history counts its
/// turns from before the fork. A header alone, brought from anywhere,
/// names a session without saying anything was spent, so it can't take
/// what a fork was counted for.
fn held_parent(history: &History, header: Option<&serde_json::Value>) -> Option<String> {
    let header = header?;
    let until = header
        .get("timestamp")
        .and_then(serde_json::Value::as_str)
        .and_then(crate::time::parse)?;
    crate::logs::copies::parent_id(header).filter(|id| history.holds_turns(id, until))
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
    let identity = identity(&metadata);
    let name = path.to_string_lossy().into_owned();
    let (mut offset, mut carry) = if let Some((offset, carry)) = history.file(&name, &identity) {
        (offset, serde_json::from_str(&carry).unwrap_or_default())
    } else {
        // A new log, or one replaced under the same name: start over.
        history.forget_file(&name)?;
        (0, Carry::default())
    };
    if metadata.len() < offset {
        history.forget_file(&name)?;
        offset = 0;
        carry = Carry::default();
    }
    if offset > 0
        && !carry.copies_known
        && check_copies(history, found, &identity, &mut carry, offset)?
    {
        offset = 0;
        carry = Carry::default();
    }
    if metadata.len() == offset {
        return Ok(Pass {
            waiting: !carry.copies_known,
            ..Pass::default()
        });
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
    // Only a read from its start meets the header.
    carry.held_parent = if offset == 0 {
        held_parent(history, header(path).as_ref())
    } else {
        None
    };
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
        waiting: !carry.copies_known,
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
        writeln!(file, "{}", turn_at(2)).unwrap();
        scan(&history, &own(dir.path()), Reading::default()).unwrap();
        assert_eq!(turns(&history), 2);
    }

    #[test]
    fn a_replaced_log_is_read_again() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("project")).unwrap();
        let path = dir.path().join("project/2026_s1.jsonl");
        let later = turn_at(2);
        std::fs::write(&path, format!("{HEADER}\n{TURN}\n{later}\n")).unwrap();
        let history = History::memory();
        scan(&history, &own(dir.path()), Reading::default()).unwrap();
        assert_eq!(turns(&history), 2);
        // Written anew, as a rewrite of the session would, and longer, so
        // only noticing it's another file reads it from its start. It's
        // written beside the old and moved over it, so the two can't share
        // a freed inode's number.
        let anew = [turn_at(3), turn_at(4), turn_at(5)].join("\n");
        let beside = dir.path().join("project/2026_s1.jsonl.new");
        std::fs::write(&beside, format!("{HEADER}\n{anew}\n")).unwrap();
        std::fs::rename(&beside, &path).unwrap();
        scan(&history, &own(dir.path()), Reading::default()).unwrap();
        let first: f64 = history
            .db
            .query_row("SELECT MIN(at) FROM turns", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            (turns(&history), Some(first)),
            (3, crate::time::parse("2026-10-14T10:03:00Z")),
            "a log written anew wasn't read again from its start alone"
        );
    }

    // A boot gives a btrfs subvolume another device number, so a log kept
    // under the number it had is the same log, read on from where it was.
    #[test]
    fn a_log_kept_under_another_boot_s_device_is_read_on_from_where_it_was() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("project")).unwrap();
        let path = dir.path().join("project/2026_s1.jsonl");
        std::fs::write(&path, format!("{HEADER}\n{TURN}\n")).unwrap();
        let history = History::memory();
        scan(&history, &own(dir.path()), Reading::default()).unwrap();
        let metadata = std::fs::metadata(&path).unwrap();
        // Another number than the one the log is under now, whatever that is.
        let kept = format!("{}:{}", metadata.dev() + 1, metadata.ino());
        history
            .db
            .execute("UPDATE files SET identity = ?1", [kept])
            .unwrap();
        let later = turn_at(2);
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        writeln!(file, "{later}").unwrap();
        let read = scan(&history, &own(dir.path()), Reading::default()).unwrap();
        assert_eq!(
            (read.bytes, turns(&history)),
            (u64::try_from(later.len() + 1).unwrap(), 2),
            "a log kept under another boot's device number was read again from its start"
        );
    }

    // A row kept as device and inode still tells a log replaced since: its
    // inode isn't the one kept, so it's read again from its start.
    #[test]
    fn a_log_kept_as_device_and_another_inode_is_read_again_from_its_start() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("project")).unwrap();
        let path = dir.path().join("project/2026_s1.jsonl");
        std::fs::write(&path, format!("{HEADER}\n{TURN}\n")).unwrap();
        let history = History::memory();
        scan(&history, &own(dir.path()), Reading::default()).unwrap();
        let metadata = std::fs::metadata(&path).unwrap();
        let kept = format!("{}:{}", metadata.dev(), metadata.ino() + 1);
        history
            .db
            .execute("UPDATE files SET identity = ?1", [kept])
            .unwrap();
        let read = scan(&history, &own(dir.path()), Reading::default()).unwrap();
        assert_eq!(
            (read.bytes, turns(&history)),
            (metadata.len(), 1),
            "a log under another inode than its kept row's was read on rather than from its start"
        );
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

    /// A fork, s2, of s1 at 10:30, holding s1's turn at 10:01, its own at
    /// 10:31 and whatever `own` adds, written beside and moved over.
    fn write_fork(project: &Path, parent: &str, own: &[String]) {
        let header = serde_json::json!({
            "type": "session", "id": "s2", "timestamp": "2026-10-14T10:30:00Z",
            "cwd": "/y", "parentSession": parent,
        });
        let body = [TURN.to_owned(), turn_at(31)]
            .into_iter()
            .chain(own.iter().cloned())
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::create_dir_all(project).unwrap();
        let beside = project.join("2026_s2.jsonl.new");
        std::fs::write(&beside, format!("{header}\n{body}\n")).unwrap();
        std::fs::rename(beside, project.join("2026_s2.jsonl")).unwrap();
    }

    const ELSEWHERE: &str = "/home/elsewhere/sessions/project/2026_s1.jsonl";

    /// Teal's and ochre's brought logs, teal's first.
    fn teal_and_ochre(dir: &Path) -> Vec<Found> {
        let (teal, ochre) = ("teal".to_owned(), "ochre".to_owned());
        machine_logs(&dir.join("teal"), &[], Some(&teal))
            .chain(machine_logs(&dir.join("ochre"), &[], Some(&ochre)))
            .collect()
    }

    // A copy brought to two machines before its parent is held by two logs,
    // so its rows stay when one is read again; what it was counted for
    // before its header still goes once the parent lands beside either,
    // and only that: ochre's copy went further than teal's.
    #[test]
    fn a_copy_two_machines_hold_counts_once_when_its_parent_lands() {
        let dir = tempfile::tempdir().unwrap();
        write_fork(&dir.path().join("teal/project"), ELSEWHERE, &[]);
        write_fork(&dir.path().join("ochre/project"), ELSEWHERE, &[turn_at(40)]);
        let history = History::memory();
        scan(&history, &teal_and_ochre(dir.path()), Reading::default()).unwrap();
        assert_eq!(
            turns(&history),
            3,
            "a copy two machines hold was counted twice"
        );
        let parent = dir.path().join("teal/project/2026_s1.jsonl");
        std::fs::write(parent, format!("{HEADER}\n{TURN}\n")).unwrap();
        scan(&history, &teal_and_ochre(dir.path()), Reading::default()).unwrap();
        scan(&history, &teal_and_ochre(dir.path()), Reading::default()).unwrap();
        assert_eq!(
            turns(&history),
            3,
            "a copy two machines hold kept its parent's turn, or lost its own, once the \
             parent landed"
        );
        // Brought again, as a transport replacing each copy would.
        write_fork(&dir.path().join("ochre/project"), ELSEWHERE, &[turn_at(40)]);
        scan(&history, &teal_and_ochre(dir.path()), Reading::default()).unwrap();
        assert_eq!(
            turns(&history),
            3,
            "a copy brought again after its parent landed counted its parent's turn"
        );
    }

    // The parent beside one copy is the parent of the other's too, since
    // what was copied is decided for the session.
    #[test]
    fn a_copy_two_machines_hold_counts_once_beside_one_parent() {
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().join("teal/project/2026_s1.jsonl");
        std::fs::create_dir_all(parent.parent().unwrap()).unwrap();
        std::fs::write(parent, format!("{HEADER}\n{TURN}\n")).unwrap();
        write_fork(&dir.path().join("teal/project"), ELSEWHERE, &[]);
        write_fork(&dir.path().join("ochre/project"), ELSEWHERE, &[turn_at(40)]);
        let history = History::memory();
        for _ in 0..3 {
            scan(&history, &teal_and_ochre(dir.path()), Reading::default()).unwrap();
        }
        assert_eq!(
            turns(&history),
            3,
            "a copy whose parent landed beside the other copy counted its parent's turn"
        );
    }

    // A fork made here and carried on elsewhere names its parent by this
    // machine's path, which the other machine never had: the parent's turn
    // is still counted once, and only what came before the fork is taken
    // as the parent's, since this machine's copy, read once, holds a turn
    // the other's doesn't.
    #[test]
    fn a_fork_gone_on_elsewhere_counts_its_parent_s_turns_once() {
        let dir = tempfile::tempdir().unwrap();
        let here = dir.path().join("here/project");
        std::fs::create_dir_all(&here).unwrap();
        let parent = here.join("2026_s1.jsonl");
        std::fs::write(&parent, format!("{HEADER}\n{TURN}\n")).unwrap();
        let named = parent.to_string_lossy().into_owned();
        write_fork(&here, &named, &[turn_at(35)]);
        write_fork(&dir.path().join("teal/project"), &named, &[turn_at(40)]);
        let teal = "teal".to_owned();
        for teal_first in [false, true] {
            let history = History::memory();
            let mine = own(&dir.path().join("here"));
            let there: Vec<Found> =
                machine_logs(&dir.path().join("teal"), &[], Some(&teal)).collect();
            let logs: Vec<Found> = if teal_first {
                there.into_iter().chain(mine).collect()
            } else {
                mine.into_iter().chain(there).collect()
            };
            for _ in 0..3 {
                scan(&history, &logs, Reading::default()).unwrap();
            }
            assert_eq!(
                turns(&history),
                4,
                "a fork gone on elsewhere counted its parent's turn again, or lost its own \
                 (teal first: {teal_first})"
            );
        }
    }

    // A fork made here of a session whose log is only ever brought waits
    // for it, and once it lands counts what came before the fork once.
    #[test]
    fn a_fork_of_a_brought_session_counts_its_parent_s_turns_once() {
        let dir = tempfile::tempdir().unwrap();
        write_fork(&dir.path().join("here/project"), ELSEWHERE, &[]);
        let project = dir.path().join("teal/project");
        std::fs::create_dir_all(&project).unwrap();
        let teal = "teal".to_owned();
        let logs = || -> Vec<Found> {
            own(&dir.path().join("here"))
                .into_iter()
                .chain(machine_logs(&dir.path().join("teal"), &[], Some(&teal)))
                .collect()
        };
        let history = History::memory();
        scan(&history, &logs(), Reading::default()).unwrap();
        std::fs::write(project.join("2026_s1.jsonl"), format!("{HEADER}\n{TURN}\n")).unwrap();
        // The parent is read after the fork in the same scan, which has
        // to count it once by its end, not a scan later.
        for scans in 1..=3 {
            scan(&history, &logs(), Reading::default()).unwrap();
            assert_eq!(
                turns(&history),
                2,
                "a fork of a brought session counted its parent's turn again after {scans} \
                 scans"
            );
        }
    }

    // A header is all a brought log needs to name a session, and it says
    // nothing was spent, so a parent known only by one can't take the
    // turns a fork of this machine's was counted for before it began.
    #[test]
    fn a_parent_s_header_alone_takes_nothing_from_a_fork() {
        let dir = tempfile::tempdir().unwrap();
        write_fork(&dir.path().join("here/project"), ELSEWHERE, &[]);
        let project = dir.path().join("teal/project");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(project.join("2026_s1.jsonl"), format!("{HEADER}\n")).unwrap();
        let teal = "teal".to_owned();
        let logs = own(&dir.path().join("here"))
            .into_iter()
            .chain(machine_logs(&dir.path().join("teal"), &[], Some(&teal)))
            .collect::<Vec<_>>();
        let history = History::memory();
        for scans in 1..=2 {
            scan(&history, &logs, Reading::default()).unwrap();
            assert_eq!(
                turns(&history),
                2,
                "a fork lost its turns to a parent known only by its header after {scans} scans"
            );
        }
    }

    // A brought log's header is the other machine's to write, so naming a
    // session of this machine's and a time to come can't take what this
    // machine counted for it, even once this machine's log is gone.
    #[test]
    fn a_brought_log_never_forgets_this_machine_s_session() {
        let dir = tempfile::tempdir().unwrap();
        let here = dir.path().join("here/project");
        std::fs::create_dir_all(&here).unwrap();
        std::fs::write(here.join("2026_s1.jsonl"), format!("{HEADER}\n{TURN}\n")).unwrap();
        let project = dir.path().join("teal/project");
        std::fs::create_dir_all(&project).unwrap();
        let header = serde_json::json!({
            "type": "session", "id": "s1", "timestamp": "2027-01-01T00:00:00Z",
            "cwd": "/x", "parentSession": "/home/elsewhere/sessions/project/2026_p.jsonl",
        });
        std::fs::write(project.join("2026_s1.jsonl"), format!("{header}\n")).unwrap();
        let teal = "teal".to_owned();
        let logs = || -> Vec<Found> {
            own(&dir.path().join("here"))
                .into_iter()
                .chain(machine_logs(&dir.path().join("teal"), &[], Some(&teal)))
                .collect()
        };
        let history = History::memory();
        scan(&history, &logs(), Reading::default()).unwrap();
        let parent = r#"{"type":"session","id":"p","timestamp":"2026-10-14T09:00:00Z","cwd":"/x"}"#;
        std::fs::write(project.join("2026_p.jsonl"), format!("{parent}\n")).unwrap();
        scan(&history, &logs(), Reading::default()).unwrap();
        scan(&history, &logs(), Reading::default()).unwrap();
        assert_eq!(
            turns(&history),
            1,
            "a brought log's header took what this machine counted for its own session"
        );
        std::fs::remove_file(here.join("2026_s1.jsonl")).unwrap();
        let beside = project.join("2026_s1.jsonl.new");
        std::fs::write(&beside, format!("{header}\n")).unwrap();
        std::fs::rename(beside, project.join("2026_s1.jsonl")).unwrap();
        scan(&history, &logs(), Reading::default()).unwrap();
        assert_eq!(
            turns(&history),
            1,
            "a brought log replaced once this machine's was gone took what this machine \
             counted"
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
        let later = turn_at(2);
        std::fs::write(&path, format!("{HEADER}\n{TURN}\n{long}\n{later}\n{long}")).unwrap();
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

    /// An agent's log of one turn in `run`'s `group`, as `name`, under
    /// `root`'s runs, it and every folder above it to the run written a day
    /// and a minute before now when `aged`.
    fn agent(root: &Path, run: &str, group: &str, name: &str, aged: bool) -> PathBuf {
        let session = empty(root, run, group, name, false);
        let path = session.join(format!("2026_{name}.jsonl"));
        let header = HEADER.replace("\"s1\"", &format!("\"{run}-{group}-{name}\""));
        std::fs::write(&path, format!("{header}\n{TURN}\n")).unwrap();
        if aged {
            age(&path);
            empty(root, run, group, name, true);
        }
        path
    }

    /// A reviewer's log of one turn, as `agent`'s.
    fn reviewer(root: &Path, run: &str, name: &str, aged: bool) -> PathBuf {
        agent(root, run, "reviewers", name, aged)
    }

    /// A reviewer's session folder holding no log yet, it and every folder
    /// above it to the run aged as `agent` ages them.
    fn empty(root: &Path, run: &str, group: &str, name: &str, aged: bool) -> PathBuf {
        let run = root.join("runs").join(run);
        let session = run.join(group).join(name).join("session");
        std::fs::create_dir_all(&session).unwrap();
        if aged {
            for folder in session.ancestors().take(4) {
                age(folder);
            }
        }
        session
    }

    /// Sets `path`'s time a day and a minute before now.
    fn age(path: &Path) {
        let then = SystemTime::now() - QUIET - Duration::from_secs(60);
        std::fs::File::open(path)
            .unwrap()
            .set_modified(then)
            .unwrap();
    }

    /// A log of one turn written into a session folder `empty` made, its
    /// session named for the run.
    fn written_in(session: &Path) {
        let run = session.ancestors().nth(3).unwrap().file_name().unwrap();
        let header = HEADER.replace("\"s1\"", &format!("\"late-{}\"", run.display()));
        std::fs::write(
            session.join("2026_late.jsonl"),
            format!("{header}\n{TURN}\n"),
        )
        .unwrap();
    }

    /// A config whose only logs are reviewers' under `root`'s runs.
    fn reviewers(root: &Path) -> Config {
        let root = root.display();
        serde_json::from_value(serde_json::json!({
            "sessions": format!("{root}/none"), "runs": "/r", "registry": "/g",
            "quests": "/q", "state": "/t",
            "agents": [{"pattern": format!("{root}/runs/*/reviewers/*/session/*.jsonl"),
                "kind": "review"}],
        }))
        .unwrap()
    }

    /// One pass's reading, as the daemon's: walk, read, then settle.
    fn read(history: &History, config: &Config, walks: &mut Walks) {
        let logs = found_kept(config, walks);
        scan(history, &logs, Reading::default()).unwrap();
        walks.settle(history, SystemTime::now());
    }

    // A run whose logs were all read and which nothing has written for a
    // day is walked no more: a log made in a session folder that held none
    // is found only by the next sweep.
    #[test]
    fn a_quiet_run_is_walked_no_more_until_the_sweep() {
        let dir = tempfile::tempdir().unwrap();
        let config = reviewers(dir.path());
        let history = History::memory();
        let mut walks = Walks::default();
        reviewer(dir.path(), "r1", "a", true);
        let late = empty(dir.path(), "r1", "reviewers", "c", true);
        let later = empty(dir.path(), "r1", "reviewers", "d", true);
        read(&history, &config, &mut walks);
        assert_eq!(turns(&history), 1);
        written_in(&late);
        read(&history, &config, &mut walks);
        assert_eq!(turns(&history), 1, "a settled run was walked again");
        walks.swept = SystemTime::now().checked_sub(SWEEP);
        read(&history, &config, &mut walks);
        assert_eq!(turns(&history), 2, "the sweep didn't walk the run again");
        age(&late.join("2026_late.jsonl"));
        read(&history, &config, &mut walks);
        assert_eq!(walks.settled.len(), 1, "the swept run didn't settle again");
        // A clock set back sweeps at once rather than waiting to catch up.
        let header = HEADER.replace("\"s1\"", "\"later\"");
        std::fs::write(
            later.join("2026_later.jsonl"),
            format!("{header}\n{TURN}\n"),
        )
        .unwrap();
        walks.swept = SystemTime::now().checked_add(Duration::from_hours(1));
        read(&history, &config, &mut walks);
        assert_eq!(turns(&history), 3, "a clock set back didn't sweep");
    }

    // A log written lately keeps its run walked though every folder above
    // it is a day old, as a reviewer's appending leaves them.
    #[test]
    fn a_run_with_a_log_written_today_is_walked_each_pass() {
        let dir = tempfile::tempdir().unwrap();
        let config = reviewers(dir.path());
        let history = History::memory();
        let mut walks = Walks::default();
        let log = reviewer(dir.path(), "r1", "a", true);
        std::fs::File::open(&log)
            .unwrap()
            .set_modified(SystemTime::now())
            .unwrap();
        read(&history, &config, &mut walks);
        assert_eq!(turns(&history), 1);
        assert!(walks.settled.is_empty(), "a run with a fresh log settled");
    }

    // A settled run's logs are still read each pass, so a turn written to
    // one is counted without a walk.
    #[test]
    fn a_settled_run_s_logs_are_still_read() {
        let dir = tempfile::tempdir().unwrap();
        let config = reviewers(dir.path());
        let history = History::memory();
        let mut walks = Walks::default();
        let log = reviewer(dir.path(), "r1", "a", true);
        read(&history, &config, &mut walks);
        assert_eq!(walks.settled.len(), 1, "the run didn't settle");
        let later = TURN.replace("10:01:00Z", "10:02:00Z");
        std::fs::OpenOptions::new()
            .append(true)
            .open(&log)
            .and_then(|mut file| {
                std::io::Write::write_all(&mut file, format!("{later}\n").as_bytes())
            })
            .unwrap();
        read(&history, &config, &mut walks);
        assert_eq!(turns(&history), 2, "a settled run's log went unread");
    }

    // A reviewer asked again in a settled run makes a folder of its own,
    // which changes its reviewers' folder, and that walks the run again
    // with no watch and no sweep, as in the daemon's poll.
    #[test]
    fn a_folder_made_in_a_settled_run_walks_it_again() {
        let dir = tempfile::tempdir().unwrap();
        let config = reviewers(dir.path());
        let history = History::memory();
        let mut walks = Walks::default();
        reviewer(dir.path(), "r1", "a", true);
        read(&history, &config, &mut walks);
        assert_eq!(walks.settled.len(), 1, "the run didn't settle");
        reviewer(dir.path(), "r1", "b", false);
        read(&history, &config, &mut walks);
        assert_eq!(turns(&history), 2, "the new reviewer wasn't found");
        assert!(walks.settled.is_empty(), "the changed run stayed settled");
    }

    // A reviewer's folder made today in an old run, its log not yet
    // written, keeps the run walked: the folder its log lands in is one no
    // watch follows and no pass looks at.
    #[test]
    fn a_run_with_a_folder_made_today_is_walked_each_pass() {
        let dir = tempfile::tempdir().unwrap();
        let config = reviewers(dir.path());
        let history = History::memory();
        let mut walks = Walks::default();
        reviewer(dir.path(), "r1", "a", true);
        let late = empty(dir.path(), "r1", "reviewers", "c", false);
        read(&history, &config, &mut walks);
        assert!(walks.settled.is_empty(), "a run written today settled");
        written_in(&late);
        read(&history, &config, &mut walks);
        assert_eq!(turns(&history), 2);
    }

    // A log read only to a line still being written keeps its run walked.
    #[test]
    fn a_run_with_a_log_read_partway_is_walked_each_pass() {
        let dir = tempfile::tempdir().unwrap();
        let config = reviewers(dir.path());
        let history = History::memory();
        let mut walks = Walks::default();
        let log = reviewer(dir.path(), "r1", "a", true);
        std::fs::OpenOptions::new()
            .append(true)
            .open(&log)
            .and_then(|mut file| file.write_all(br#"{"type":"mess"#))
            .unwrap();
        age(&log);
        read(&history, &config, &mut walks);
        assert_eq!(turns(&history), 1);
        assert!(
            walks.settled.is_empty(),
            "a run with a log read partway settled"
        );
    }

    // A write heard in one run leaves every other settled, even one whose
    // name the first's starts with, and so does one in pi's own sessions.
    #[test]
    fn a_write_heard_elsewhere_leaves_a_run_settled() {
        let dir = tempfile::tempdir().unwrap();
        let config = reviewers(dir.path());
        let history = History::memory();
        let mut walks = Walks::default();
        reviewer(dir.path(), "r1", "a", true);
        let other = reviewer(dir.path(), "r10", "a", true);
        read(&history, &config, &mut walks);
        assert_eq!(walks.settled.len(), 2);
        walks.touched(other.parent().unwrap());
        walks.touched(&config.sessions);
        let left: Vec<&Path> = walks.settled.keys().map(|(_, top)| top.as_path()).collect();
        assert_eq!(left, [dir.path().join("runs").join("r1")]);
    }

    // A time too far ahead to add a day to is never quiet, and asking
    // doesn't panic.
    #[test]
    fn a_time_past_the_clock_s_reach_is_never_quiet() {
        let far = SystemTime::UNIX_EPOCH + Duration::from_secs(i64::MAX.unsigned_abs());
        assert!(
            !aged(far, SystemTime::now()),
            "a time past reach counted as quiet"
        );
        assert!(
            aged(SystemTime::now() - QUIET, SystemTime::now()),
            "a day ago wasn't quiet"
        );
    }

    // A write the watch hears in a settled run walks it again, as does an
    // overflow, which may have missed any.
    #[test]
    fn a_settled_run_is_walked_again_once_it_s_written_in() {
        for overflow in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let config = reviewers(dir.path());
            let history = History::memory();
            let mut walks = Walks::default();
            let first = reviewer(dir.path(), "r1", "a", true);
            let late = empty(dir.path(), "r1", "reviewers", "c", true);
            read(&history, &config, &mut walks);
            assert_eq!(walks.settled.len(), 1, "the run didn't settle");
            written_in(&late);
            let heard = if overflow {
                Path::new("/")
            } else {
                first.parent().unwrap()
            };
            walks.touched(heard);
            read(&history, &config, &mut walks);
            assert_eq!(
                turns(&history),
                2,
                "hearing {} didn't walk the run",
                heard.display()
            );
        }
    }

    // A run written within the day, or with a log not yet read to its end,
    // is walked each pass, so a reviewer starting in it is read at once.
    #[test]
    fn a_run_not_yet_quiet_is_walked_each_pass() {
        let dir = tempfile::tempdir().unwrap();
        let config = reviewers(dir.path());
        let history = History::memory();
        let mut walks = Walks::default();
        reviewer(dir.path(), "r1", "a", false);
        let fresh = empty(dir.path(), "r1", "reviewers", "c", false);
        read(&history, &config, &mut walks);
        reviewer(dir.path(), "r2", "a", true);
        let unread = empty(dir.path(), "r2", "reviewers", "c", true);
        // Walked but not yet read, as a log is between a walk and its read.
        found_kept(&config, &mut walks);
        walks.settle(&history, SystemTime::now());
        written_in(&fresh);
        written_in(&unread);
        read(&history, &config, &mut walks);
        assert_eq!(turns(&history), 4);
    }

    // Two patterns meeting in one run's folder keep their own logs and
    // kinds once it's settled.
    #[test]
    fn patterns_meeting_in_a_folder_keep_their_own_logs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().display();
        let config: Config = serde_json::from_value(serde_json::json!({
            "sessions": format!("{root}/none"), "runs": "/r", "registry": "/g",
            "quests": "/q", "state": "/t",
            "agents": [
                {"pattern": format!("{root}/runs/*/reviewers/*/session/*.jsonl"), "kind": "review"},
                {"pattern": format!("{root}/runs/*/agents/*/session/*.jsonl"), "kind": "swarm"},
            ],
        }))
        .unwrap();
        let history = History::memory();
        let mut walks = Walks::default();
        agent(dir.path(), "r1", "agents", "a", true);
        reviewer(dir.path(), "r1", "b", true);
        read(&history, &config, &mut walks);
        assert_eq!(walks.settled.len(), 2, "each pattern's run didn't settle");
        for found in found_kept(&config, &mut walks) {
            let group = found.path.ancestors().nth(3).unwrap().file_name().unwrap();
            let kind = if group == "agents" { "swarm" } else { "review" };
            assert_eq!(
                found.kind.as_deref(),
                Some(kind),
                "{}",
                found.path.display()
            );
        }
        assert_eq!(found_kept(&config, &mut walks).len(), 2);
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
        let watched = followed_directories(&config, &found(&config));
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

    // A link among another machine's logs, its sessions' or its agents',
    // finds a log already counted, so it's passed over rather than counted
    // again under that machine.
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
        let runs = dir.path().join("teal-runs/run/session");
        std::fs::create_dir_all(&runs).unwrap();
        std::os::unix::fs::symlink(here.join("2026_a.jsonl"), runs.join("2026_c.jsonl")).unwrap();
        let agents = [crate::config::AgentLogs {
            pattern: format!("{}/teal-runs/*/session/*.jsonl", dir.path().display()),
            kind: "review".into(),
        }];
        let name = "teal".to_owned();
        let brought: Vec<Found> = machine_logs(&teal, &agents, Some(&name)).collect();
        assert!(
            brought.is_empty(),
            "a link among another machine's logs was read: {brought:?}"
        );
    }

    /// The machine a session is kept under, `None` for this one.
    fn machine_of(history: &History, session: &str) -> Option<String> {
        history
            .db
            .query_row(
                "SELECT machine FROM sessions WHERE id = ?1",
                [session],
                |row| row.get(0),
            )
            .unwrap()
    }

    /// A turn at `minute` past ten, as `TURN` is at one past.
    fn turn_at(minute: u32) -> String {
        TURN.replace("10:01:00", &format!("10:{minute:02}:00"))
    }

    // A session copied to another machine and gone on there keeps its id,
    // so both logs hold the turns from before the copy and each its own
    // after. Read together, each turn counts once, whichever log was read
    // first, and the session stays this machine's, as this machine says.
    #[test]
    fn a_session_gone_on_elsewhere_counts_each_turn_once() {
        let dir = tempfile::tempdir().unwrap();
        let write = |folder: &str, lines: &[String]| {
            let project = dir.path().join(folder).join("project");
            std::fs::create_dir_all(&project).unwrap();
            let body = lines.join("\n");
            // The copy says it ran in another folder.
            let header = if folder == "teal" {
                HEADER.replace(r#""cwd":"/x""#, r#""cwd":"/y""#)
            } else {
                HEADER.to_owned()
            };
            std::fs::write(project.join("2026_s1.jsonl"), format!("{header}\n{body}\n")).unwrap();
        };
        for teal_first in [false, true] {
            write("here", &[turn_at(1), turn_at(2)]);
            write("teal", &[turn_at(1), turn_at(3)]);
            let history = History::memory();
            let teal = "teal".to_owned();
            let here = own(&dir.path().join("here"));
            let there: Vec<Found> =
                machine_logs(&dir.path().join("teal"), &[], Some(&teal)).collect();
            let logs: Vec<Found> = if teal_first {
                there.into_iter().chain(here).collect()
            } else {
                here.into_iter().chain(there).collect()
            };
            scan(&history, &logs, Reading::default()).unwrap();
            scan(&history, &logs, Reading::default()).unwrap();
            assert_eq!(
                turns(&history),
                3,
                "a session two logs hold didn't count each turn once (teal first: {teal_first})"
            );
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(dir.path().join("teal/project/2026_s1.jsonl"))
                .unwrap();
            writeln!(file, "{}", turn_at(4)).unwrap();
            scan(&history, &logs, Reading::default()).unwrap();
            assert_eq!(
                turns(&history),
                4,
                "a turn the copy took after was lost (teal first: {teal_first})"
            );
            assert_eq!(
                machine_of(&history, "s1"),
                None,
                "a session with a log here wasn't this machine's (teal first: {teal_first})"
            );
            let cwd: String = history
                .db
                .query_row("SELECT cwd FROM sessions WHERE id = 's1'", [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(
                cwd, "/x",
                "a brought log rewrote what this machine says of its session (teal first: {teal_first})"
            );
        }
    }

    // A session's turns stay when one of its several logs is replaced,
    // since which log said what isn't kept: nothing the others hold is
    // lost, and the replacement counts only the turns not held already.
    #[test]
    fn a_log_replaced_keeps_the_turns_its_session_s_other_logs_hold() {
        let dir = tempfile::tempdir().unwrap();
        let write = |folder: &str, lines: &[String]| {
            let project = dir.path().join(folder).join("project");
            std::fs::create_dir_all(&project).unwrap();
            // Written beside and moved over, so a log written anew is
            // always another file, never one reusing a freed inode.
            let beside = project.join("2026_s1.jsonl.new");
            let body = lines.join("\n");
            std::fs::write(&beside, format!("{HEADER}\n{body}\n")).unwrap();
            std::fs::rename(beside, project.join("2026_s1.jsonl")).unwrap();
        };
        write("here", &[turn_at(1), turn_at(2)]);
        write("teal", &[turn_at(1), turn_at(3)]);
        let history = History::memory();
        let teal = "teal".to_owned();
        let mut logs = own(&dir.path().join("here"));
        logs.extend(machine_logs(&dir.path().join("teal"), &[], Some(&teal)));
        scan(&history, &logs, Reading::default()).unwrap();
        // Written anew, as a rewrite of the session would.
        write("here", &[turn_at(1), turn_at(5)]);
        scan(&history, &logs, Reading::default()).unwrap();
        scan(&history, &logs, Reading::default()).unwrap();
        assert_eq!(
            turns(&history),
            4,
            "a log replaced lost turns, or counted again the ones already held"
        );
    }

    // A log first seen before its header is whole still counts each of its
    // session's turns once, once the rest of it lands.
    #[test]
    fn a_copy_seen_half_written_counts_each_turn_once() {
        let dir = tempfile::tempdir().unwrap();
        let here = dir.path().join("here/project");
        std::fs::create_dir_all(&here).unwrap();
        std::fs::write(here.join("2026_s1.jsonl"), format!("{HEADER}\n{TURN}\n")).unwrap();
        let teal = dir.path().join("teal/project");
        std::fs::create_dir_all(&teal).unwrap();
        let (start, end) = HEADER.split_at(HEADER.len() / 2);
        std::fs::write(teal.join("2026_s1.jsonl"), start).unwrap();
        let history = History::memory();
        let name = "teal".to_owned();
        let mut logs = own(&dir.path().join("here"));
        logs.extend(machine_logs(&dir.path().join("teal"), &[], Some(&name)));
        scan(&history, &logs, Reading::default()).unwrap();
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(teal.join("2026_s1.jsonl"))
            .unwrap();
        writeln!(file, "{end}\n{TURN}").unwrap();
        scan(&history, &logs, Reading::default()).unwrap();
        assert_eq!(
            turns(&history),
            1,
            "a copy seen half written counted its session's turn twice"
        );
    }

    // A machine renamed, or its logs brought somewhere new, finds every log
    // again under a new path while the old ones stay on disk. Each turn
    // counts once, the session goes to the new name, and what the session
    // takes on after is counted.
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
        let moved = dir.path().join("ochre/project");
        std::fs::create_dir_all(&moved).unwrap();
        std::fs::copy(project.join("2026_s1.jsonl"), moved.join("2026_s1.jsonl")).unwrap();
        let logs: Vec<Found> = machine_logs(&dir.path().join("ochre"), &[], Some(&ochre)).collect();
        scan(&history, &logs, Reading::default()).unwrap();
        assert_eq!(
            turns(&history),
            1,
            "a log brought somewhere new was counted again"
        );
        assert_eq!(
            machine_of(&history, "s1").as_deref(),
            Some("ochre"),
            "a session brought somewhere new kept the old machine's name"
        );
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(moved.join("2026_s1.jsonl"))
            .unwrap();
        writeln!(file, "{}", turn_at(2)).unwrap();
        scan(&history, &logs, Reading::default()).unwrap();
        assert_eq!(
            turns(&history),
            2,
            "a turn taken after the logs were brought somewhere new was lost"
        );
        // The old folder cleared away, what its log said stays counted:
        // written anew, the log left can't take what only the old one held.
        std::fs::remove_dir_all(dir.path().join("teal")).unwrap();
        let beside = moved.join("2026_s1.jsonl.new");
        std::fs::write(&beside, format!("{HEADER}\n{}\n", turn_at(2))).unwrap();
        std::fs::rename(beside, moved.join("2026_s1.jsonl")).unwrap();
        scan(&history, &logs, Reading::default()).unwrap();
        assert_eq!(
            turns(&history),
            2,
            "a log brought somewhere new, written anew once the old was gone, took the old \
             log's turns with it"
        );
    }
}
