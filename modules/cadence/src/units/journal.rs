//! The journals as a tick reads them: each once, with no matches, from
//! where the tick before stopped, its lines sorted here among the readers
//! that want them and kept in cadence's folder for as long as each looks
//! back. On 27 November a tick's four matched reads (20 minutes of
//! restarts, a day of each update unit, a week of the kernel's) read about
//! 1 GB from disk every five minutes while Joel's apps stalled waiting on
//! it. A match walks every journal file's index, which the rest of the
//! machine had pushed out of the page cache since the tick before: the
//! restarts' read took 1 GB cold and 71 MB straight after, and the same 20
//! minutes unmatched took 148 MB. A reader with nothing kept, or none read
//! for longer than it looks back, is filled once by its own matched read.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// How far before the last read the next one starts, so a line the
/// journal stamped a little before it took it isn't missed; a line read
/// twice is kept once, by its cursor.
const OVERLAP: i64 = 60;

/// The fields kept of a line: those `units::restarts` reads, those
/// journalctl's short-unix output writes, and the boot it was written in.
const KEPT: [&str; 14] = [
    "MESSAGE_ID",
    "USER_UNIT",
    "UNIT",
    "USER_INVOCATION_ID",
    "INVOCATION_ID",
    "EXIT_STATUS",
    "EXIT_CODE",
    "__REALTIME_TIMESTAMP",
    "_HOSTNAME",
    "SYSLOG_IDENTIFIER",
    "_COMM",
    "_PID",
    "MESSAGE",
    "_BOOT_ID",
];

/// Which journal: the account's own, or the system's.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Journal {
    User,
    System,
}

impl Journal {
    fn args(self) -> &'static [&'static str] {
        match self {
            Self::User => &["--user"],
            Self::System => &[],
        }
    }
}

/// A reader of a journal: its name, which names its file, the lines it
/// takes, how far back it looks in seconds, and the matches of the read
/// that fills it when it has nothing to go on.
pub struct Want {
    pub name: &'static str,
    pub journal: Journal,
    pub keep: i64,
    pub fill: &'static [&'static str],
    pub takes: fn(&Map<String, Value>) -> bool,
}

/// Where Linux says which boot this is.
const BOOT_ID: &str = "/proc/sys/kernel/random/boot_id";

/// A day, as far back as the card reads the updates.
const DAY: i64 = 86_400;

/// The follower's lines, which the card reads its asks and looks from:
/// its own, or its user manager's about it, as `journalctl --user -u`
/// matches them.
pub const FOLLOWS: Want = Want {
    name: "follows",
    journal: Journal::User,
    keep: DAY,
    fill: &["-u", "update-to-main-follows"],
    takes: |fields| {
        is(
            fields,
            "_SYSTEMD_USER_UNIT",
            "update-to-main-follows.service",
        ) || is(fields, "USER_UNIT", "update-to-main-follows.service")
    },
};

/// The update service's lines, which the card reads every update's look
/// from, whoever asked for it: its own, or systemd's about it.
pub const UPDATES: Want = Want {
    name: "updates",
    journal: Journal::System,
    keep: DAY,
    fill: &["-u", "update-to-main.service"],
    takes: |fields| {
        is(fields, "_SYSTEMD_UNIT", "update-to-main.service")
            || is(fields, "UNIT", "update-to-main.service")
    },
};

/// The kernel's out-of-memory kills over a week, which the card reads the
/// last machine-wide one from; [`short_unix`] given this boot leaves out
/// those of a boot before, as `journalctl -k` does.
pub const KERNEL: Want = Want {
    name: "kernel",
    journal: Journal::System,
    keep: 7 * DAY,
    fill: &["-k", "--grep=oom-kill:"],
    takes: |fields| {
        is(fields, "_TRANSPORT", "kernel")
            && text(fields, "MESSAGE").is_some_and(|m| m.contains("oom-kill:"))
    },
};

/// One line kept: its cursor, when it was written in microseconds, and
/// the fields of it a reader reads.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Line {
    pub cursor: String,
    pub micros: i64,
    pub fields: Map<String, Value>,
}

/// What a reader keeps: when it was last read up to, in seconds, and its
/// lines.
#[derive(Default, Serialize, Deserialize)]
struct Kept {
    read: i64,
    lines: Vec<Line>,
}

/// Each reader's lines in the `keep` before `now`, reading each journal
/// with no matches once, from the oldest of its readers' last reads on.
/// A reader with nothing kept, or none read for longer than it looks
/// back, is filled by its own matched read instead. `run` runs journalctl
/// with the arguments given, answering what it wrote or why it failed. A
/// read that fails leaves what its readers kept as it was, and says so on
/// stderr, so the next tick reads again from the same place.
#[must_use]
pub fn forward(
    folder: &Path,
    wants: &[Want],
    now: i64,
    run: impl Fn(&[String]) -> Result<String, String>,
) -> BTreeMap<&'static str, Vec<Line>> {
    let folder = folder.join("journals");
    let mut kept: BTreeMap<&str, Kept> = wants
        .iter()
        .map(|want| (want.name, load(&folder, want.name)))
        .collect();
    for journal in [Journal::User, Journal::System] {
        let mut fresh = Vec::new();
        for want in wants.iter().filter(|w| w.journal == journal) {
            let read = kept.get(want.name).map_or(0, |k| k.read);
            if read > 0 && now - read <= want.keep {
                fresh.push(want);
                continue;
            }
            let mut args = asked(journal, now - want.keep, now);
            args.extend(want.fill.iter().map(|a| (*a).to_owned()));
            match run(&args) {
                Ok(said) => {
                    let lines = taken(&said, want.takes);
                    kept.insert(want.name, Kept { read: now, lines });
                }
                Err(why) => eprintln!("cadence: filling the {} lines: {why}", want.name),
            }
        }
        let Some(from) = fresh
            .iter()
            .filter_map(|w| kept.get(w.name).map(|k| k.read))
            .min()
        else {
            continue;
        };
        let said = match run(&asked(journal, from - OVERLAP, now)) {
            Ok(said) => said,
            Err(why) => {
                eprintln!("cadence: reading the journal forward: {why}");
                continue;
            }
        };
        for want in fresh {
            let Some(k) = kept.get_mut(want.name) else {
                continue;
            };
            let seen: BTreeSet<String> = k.lines.iter().map(|l| l.cursor.clone()).collect();
            k.lines.extend(
                taken(&said, want.takes)
                    .into_iter()
                    .filter(|l| !seen.contains(&l.cursor)),
            );
            k.read = now;
        }
    }
    if let Err(err) = fs::create_dir_all(&folder) {
        eprintln!(
            "cadence: keeping the journal's lines: {}: {err}",
            folder.display()
        );
    }
    wants
        .iter()
        .map(|want| {
            let mut k = kept.remove(want.name).unwrap_or_default();
            k.lines
                .retain(|l| l.micros >= (now - want.keep) * 1_000_000);
            k.lines.sort_by_key(|l| l.micros);
            keep(&folder, want.name, &k);
            (want.name, k.lines)
        })
        .collect()
}

/// The journals' lines a tick reads, as journalctl writes them: the
/// restarts as JSON, the rest as short-unix, the kernel's of this boot
/// alone.
pub struct Journals {
    pub restarts: String,
    pub follows: String,
    pub updates: String,
    pub kernel: String,
}

/// What a tick at `now` reads of the journals, each read once by `run`,
/// from where the tick before stopped, through the readers kept in
/// `folder`.
#[must_use]
pub fn tick(
    folder: &Path,
    now: i64,
    run: impl Fn(&[String]) -> Result<String, String>,
) -> Journals {
    let wants = [super::RESTARTS, FOLLOWS, UPDATES, KERNEL];
    let kept = forward(folder, &wants, now, run);
    // A boot that can't be told keeps every boot's kills, rather than
    // none of them.
    let boot = match fs::read_to_string(BOOT_ID) {
        Ok(id) => Some(id.trim().replace('-', "")),
        Err(err) => {
            eprintln!("cadence: reading {BOOT_ID}: {err}, so every boot's kills are read");
            None
        }
    };
    // forward answers every reader it's given, so each is there.
    Journals {
        restarts: json(&kept[super::RESTARTS.name]),
        follows: short_unix(&kept[FOLLOWS.name], None),
        updates: short_unix(&kept[UPDATES.name], None),
        kernel: short_unix(&kept[KERNEL.name], boot.as_deref()),
    }
}

/// journalctl's arguments for `journal`'s lines from `since` to `until`,
/// as JSON.
fn asked(journal: Journal, since: i64, until: i64) -> Vec<String> {
    let mut args: Vec<String> = journal.args().iter().map(|a| (*a).to_owned()).collect();
    args.extend([
        "-o".to_owned(),
        "json".to_owned(),
        "--no-pager".to_owned(),
        format!("--since=@{since}"),
        format!("--until=@{until}"),
    ]);
    args
}

/// The lines of journalctl's JSON that `takes` takes, each with only the
/// fields kept.
fn taken(said: &str, takes: fn(&Map<String, Value>) -> bool) -> Vec<Line> {
    said.lines()
        .filter_map(|line| serde_json::from_str::<Map<String, Value>>(line).ok())
        .filter(|entry| takes(entry))
        .filter_map(|entry| {
            let cursor = entry.get("__CURSOR")?.as_str()?.to_owned();
            let micros = entry.get("__REALTIME_TIMESTAMP")?.as_str()?.parse().ok()?;
            let fields = KEPT
                .iter()
                .filter_map(|name| Some(((*name).to_owned(), entry.get(*name)?.clone())))
                .collect();
            Some(Line {
                cursor,
                micros,
                fields,
            })
        })
        .collect()
}

/// What `name` kept, or nothing when it kept none or it can't be read,
/// which a fill then reads again.
fn load(folder: &Path, name: &str) -> Kept {
    let path = folder.join(format!("{name}.json"));
    match fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|err| {
            eprintln!(
                "cadence: {} can't be read, so it's read again: {err}",
                path.display()
            );
            Kept::default()
        }),
        // None kept yet: a first read, which the fill reads.
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Kept::default(),
        Err(err) => {
            eprintln!(
                "cadence: {} can't be read, so it's read again: {err}",
                path.display()
            );
            Kept::default()
        }
    }
}

/// Keeps what `name` read, whole, through a file beside it, so a tick cut
/// short leaves the last one standing.
fn keep(folder: &Path, name: &str, kept: &Kept) {
    let path = folder.join(format!("{name}.json"));
    let beside = folder.join(format!(".{name}.json.new"));
    let written = serde_json::to_vec(kept)
        .map_err(std::io::Error::other)
        .and_then(|bytes| fs::write(&beside, bytes))
        .and_then(|()| fs::rename(&beside, &path));
    if let Err(err) = written {
        eprintln!("cadence: keeping {}: {err}", path.display());
    }
}

/// A field as text: journalctl writes one that isn't valid text as its
/// bytes, which are read as lossy text here.
fn text(fields: &Map<String, Value>, name: &str) -> Option<String> {
    match fields.get(name)? {
        Value::String(s) => Some(s.clone()),
        Value::Array(bytes) => {
            let bytes: Vec<u8> = bytes
                .iter()
                .filter_map(|b| u8::try_from(b.as_u64()?).ok())
                .collect();
            Some(String::from_utf8_lossy(&bytes).into_owned())
        }
        _ => None,
    }
}

/// Whether `fields` has `name` as `value`.
#[must_use]
pub fn is(fields: &Map<String, Value>, name: &str, value: &str) -> bool {
    text(fields, name).is_some_and(|said| said == value)
}

/// The lines as journalctl's JSON writes them, one object a line.
#[must_use]
pub fn json(lines: &[Line]) -> String {
    lines
        .iter()
        .filter_map(|l| serde_json::to_string(&l.fields).ok())
        .map(|l| l + "\n")
        .collect()
}

/// The lines as journalctl's short-unix output writes them, `<seconds>.<
/// microseconds> <host> <identifier>[<pid>]: <message>`, those of
/// `boot` alone when it's given.
#[must_use]
pub fn short_unix(lines: &[Line], boot: Option<&str>) -> String {
    let mut out = String::new();
    for line in lines {
        let f = &line.fields;
        if let Some(boot) = boot
            && !is(f, "_BOOT_ID", boot)
        {
            continue;
        }
        let host = text(f, "_HOSTNAME").unwrap_or_default();
        let ident = text(f, "SYSLOG_IDENTIFIER")
            .or_else(|| text(f, "_COMM"))
            .unwrap_or_default();
        let pid = text(f, "_PID")
            .map(|p| format!("[{p}]"))
            .unwrap_or_default();
        let message = text(f, "MESSAGE").unwrap_or_default();
        // Writing to a String can't fail.
        let _ = writeln!(
            out,
            "{}.{:06} {host} {ident}{pid}: {message}",
            line.micros.div_euclid(1_000_000),
            line.micros.rem_euclid(1_000_000),
        );
    }
    out
}

#[cfg(test)]
mod tests;
