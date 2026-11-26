//! What each build reads and writes on the disks themselves, and who
//! asked for it, said in the journal as the build goes, so the builds
//! that wear the disk most can be found.

use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use crate::{QUEST_SLICE, client_of_build, quest_slice};

/// Who asked for a build: the client's command, and its quest if it sits
/// in one's slice.
struct Asker {
    command: String,
    quest: Option<String>,
}

/// What one build has read and written so far. Nix names a build's cgroup
/// by its build user, and the next build of that user makes it again, so a
/// build is known by its cgroup's inode, which the kernel never reuses
/// while vm-first runs.
///
/// A disk counts a write only once writeback sends it, which waits until
/// enough is dirty or 30 s have passed, so a build often ends with some of
/// its writes still in memory. `unwritten` keeps what was dirty or being
/// written at the last look; a file the build deleted before then never
/// reaches the disk, so it's the most still to come.
///
/// Each time the daemon restarts, systemd takes the builds' controllers
/// back to start it, and vm-first hands them on again: a build's io.stat
/// goes, and comes back counting from nothing. `counter` keeps what the
/// counter that's there now said at the last look, so `read` and `written`
/// add up every counter the build has had.
pub struct Tally {
    cgroup: u64,
    read: u64,
    written: u64,
    counter: (u64, u64),
    unwritten: u64,
    seen: bool,
    asked: Option<Asker>,
}

impl Tally {
    fn new(cgroup: u64) -> Self {
        Self {
            cgroup,
            read: 0,
            written: 0,
            counter: (0, 0),
            unwritten: 0,
            seen: false,
            asked: None,
        }
    }

    /// Adds what a build's io.stat has counted since the last look, given
    /// what it reads now, or `None` while it's gone. A counter that reads
    /// less than it did started again between two looks, from nothing.
    fn count(&mut self, now: Option<(u64, u64)>) {
        let (read, written) = now.unwrap_or((0, 0));
        let (was_read, was_written) = self.counter;
        let (from_read, from_written) = if read < was_read || written < was_written {
            (0, 0)
        } else {
            (was_read, was_written)
        };
        self.read += read - from_read;
        self.written += written - from_written;
        self.counter = (read, written);
    }
}

/// The bytes of a cgroup's file pages dirty or being written back, from its
/// `memory.stat`.
fn parse_unwritten(stat: &str) -> u64 {
    stat.lines()
        .filter_map(|line| line.split_once(' '))
        .filter(|(key, _)| matches!(*key, "file_dirty" | "file_writeback"))
        .filter_map(|(_, value)| value.trim().parse::<u64>().ok())
        .sum()
}

/// The bytes a cgroup has read and written, from its `io.stat`, on the
/// devices `counts` takes by their `major:minor`. `io.stat` lists a write
/// on every layer it passes through, so a disk under LUKS shows each byte
/// twice, on the mapper and on the disk beneath it, and zram shows swap
/// that never leaves memory; counting only the disks themselves counts
/// each byte the hardware takes once.
fn parse_io_stat(stat: &str, counts: impl Fn(&str) -> bool) -> (u64, u64) {
    let lines: Vec<&str> = stat
        .lines()
        .filter(|line| line.split_whitespace().next().is_some_and(&counts))
        .collect();
    let count = |key: &str| -> u64 {
        lines
            .iter()
            .flat_map(|line| line.split_whitespace())
            .filter_map(|field| field.strip_prefix(key))
            .filter_map(|value| value.parse::<u64>().ok())
            .sum()
    };
    (count("rbytes="), count("wbytes="))
}

/// Whether a block device, by its `major:minor`, is hardware: a disk the
/// kernel found on a bus, rather than a layer such as device-mapper's or a
/// device kept in memory such as zram, which have no device behind them.
fn is_a_disk(device: &str) -> bool {
    Path::new("/sys/dev/block")
        .join(device)
        .join("device")
        .exists()
}

/// A count of bytes as a person reads it, in binary units.
fn size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KiB", "MiB", "GiB", "TiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "a tenth of a unit is all it says"
    )]
    let mut scaled = bytes as f64 / 1024.0;
    let mut unit = 0;
    while scaled >= 1024.0 && unit < UNITS.len() - 1 {
        scaled /= 1024.0;
        unit += 1;
    }
    format!("{scaled:.1} {}", UNITS[unit])
}

/// The longest a client's command is said at, in characters.
const COMMAND_LENGTH: usize = 120;

/// A client's command as a person would type it, from its command line:
/// the program by its name alone, and cut short past `COMMAND_LENGTH`.
fn command_of(cmdline: &[u8]) -> String {
    let mut args = cmdline
        .split(|&byte| byte == 0)
        .filter(|arg| !arg.is_empty())
        .map(String::from_utf8_lossy);
    let Some(program) = args.next() else {
        return String::new();
    };
    let name = program.rsplit('/').next().unwrap_or(&program).to_string();
    let said = std::iter::once(name)
        .chain(args.map(std::borrow::Cow::into_owned))
        .collect::<Vec<_>>()
        .join(" ");
    if said.chars().count() <= COMMAND_LENGTH {
        return said;
    }
    let mut cut: String = said.chars().take(COMMAND_LENGTH - 3).collect();
    cut.push_str("...");
    cut
}

/// The line vm-first says when a build has gone.
/// A build that wrote nothing says nothing: what's said is for finding the
/// builds that wear the disk, and on 7 November 180 of one `nix build`'s
/// 186 lines were builds that wrote 0 B.
fn told(tally: &Tally) -> Option<String> {
    if tally.written == 0 && tally.unwritten == 0 {
        return None;
    }
    let left = if tally.unwritten == 0 {
        String::new()
    } else {
        format!(
            ", leaving up to {} more in memory to write",
            size(tally.unwritten)
        )
    };
    let did = format!(
        "wrote {} and read {}{left}.",
        size(tally.written),
        size(tally.read)
    );
    Some(match &tally.asked {
        Some(asker) => {
            let quest = asker
                .quest
                .as_ref()
                .map_or("no quest".to_string(), |quest| format!("quest {quest}"));
            format!("A build for `{}`, for {quest}, {did}", asker.command)
        }
        // A build that ended between two looks had no process to follow
        // to its client, which says nothing about the client.
        None if !tally.seen => format!("A build too short to see who asked for it {did}"),
        None => format!("A build whose client couldn't be found {did}"),
    })
}

/// The quest a quest's slice is for, from the slice's path.
fn quest_of(slice: &str) -> Option<&str> {
    slice
        .rsplit('/')
        .next()?
        .strip_prefix(QUEST_SLICE)?
        .strip_suffix(".slice")
        .filter(|quest| !quest.is_empty())
}

/// Whether a build's cgroup holds a process to follow to its client.
fn has_a_process(build: &Path) -> bool {
    fs::read_to_string(build.join("cgroup.procs")).is_ok_and(|procs| !procs.trim().is_empty())
}

/// Who asked for a build, while its client is still there to ask.
fn asker_of(build: &Path) -> Option<Asker> {
    let client = client_of_build(build)?;
    let command = command_of(&fs::read(format!("/proc/{client}/cmdline")).ok()?);
    let cgroup = fs::read_to_string(format!("/proc/{client}/cgroup")).ok()?;
    let quest = quest_slice(&cgroup).and_then(quest_of).map(str::to_string);
    Some(Asker { command, quest })
}

/// Brings each build's tally up to date, and says what each build that has
/// gone since the last look read and wrote.
pub fn tally(tallies: &mut HashMap<PathBuf, Tally>, builds: &[PathBuf]) {
    let mut seen = Vec::with_capacity(builds.len());
    for build in builds {
        let Ok(cgroup) = fs::metadata(build).map(|meta| meta.ino()) else {
            continue;
        };
        if tallies
            .get(build)
            .is_some_and(|known| known.cgroup != cgroup)
            && let Some(gone) = tallies.remove(build)
            && let Some(said) = told(&gone)
        {
            eprintln!("{said}");
        }
        let known = tallies
            .entry(build.clone())
            .or_insert_with(|| Tally::new(cgroup));
        if known.asked.is_none() {
            known.seen |= has_a_process(build);
            known.asked = asker_of(build);
        }
        known.count(
            fs::read_to_string(build.join("io.stat"))
                .ok()
                .map(|stat| parse_io_stat(&stat, is_a_disk)),
        );
        if let Ok(stat) = fs::read_to_string(build.join("memory.stat")) {
            known.unwritten = parse_unwritten(&stat);
        }
        seen.push(build);
    }
    tallies.retain(|build, gone| {
        let here = seen.contains(&build);
        if !here && let Some(said) = told(gone) {
            eprintln!("{said}");
        }
        here
    });
}

#[cfg(test)]
#[path = "tally_tests.rs"]
mod tests;
