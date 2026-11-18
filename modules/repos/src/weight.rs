//! How much room a checkout's git store takes, and whether it's due
//! upkeep, read the way git's own auto-gc reads it before every command
//! that may gather. Git spreads loose objects over 256 folders by their
//! first two hex digits, so where one (`17`, git's own choice) holds
//! enough to go by, its count and room times 256 are the estimate, its
//! largest object counted once, so one large file doesn't swell it, and
//! where it holds few, every folder is counted, which is then cheap. Packs are
//! counted by the index git loads each from, and their folder's room
//! whole, a killed repack's leavings included, since they take room too.
//! Room is what the disk gives each file, as `git count-objects` says it,
//! not its length. No git runs and no object is opened, where `git
//! count-objects` lists every loose object, which for a checkout of two
//! hundred thousand took a minute of disk.
//!
//! Whether it's due is git's own rule for its `gc` task, at the checkout's
//! own config or git's defaults, as no machine of Joel's sets them
//! anywhere else; includes aren't followed. A failed gather's `gc.log`
//! doesn't change it: only a detached `git gc --auto` reads one, and git's
//! own upkeep, `maintenance run --auto`, runs gc not detached, so it still
//! gathers a store whatever log a gather left. It's weighed again when
//! the checkout is read again: as a ref moves, and at the daemon's whole
//! reading each minute, which is when a gather is seen, since git packs
//! the refs before it gathers objects.

use std::fs::{self, Metadata};
use std::io::ErrorKind;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use crate::model::Weight;

/// The folder of 256 git's auto-gc counts, as its own `too_many_loose_objects`.
const SAMPLED: &str = "17";

/// How many folders loose objects are spread over, one per first byte.
const FOLDERS: u64 = 256;

/// Fewer than this in the sampled folder, and every folder is counted,
/// since an estimate from so few swings by hundreds of times on one object.
const ENOUGH: u64 = 16;

/// `gc.auto`'s default: past this many loose objects git gathers them.
const LOOSE_LIMIT: i64 = 6700;

/// `gc.autoPackLimit`'s default: past this many packs not kept, git
/// gathers them into one.
const PACK_LIMIT: i64 = 50;

/// What a checkout's config says of git's automatic upkeep, each key's
/// last value kept, as git reads them, and judged once all are read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Limits {
    auto: Option<i64>,
    pack_limit: Option<i64>,
    maintenance: Option<bool>,
    gc_task: Option<bool>,
    strategy: Option<String>,
}

impl Limits {
    /// Takes one key of the config, by its section, the section's name
    /// (empty for none) and the key. Anything else is passed over, as is a
    /// value this can't read, which leaves git's default.
    pub fn set(&mut self, section: &str, name: &str, key: &str, value: &str) {
        let section = section.to_ascii_lowercase();
        let key = key.to_ascii_lowercase();
        match (section.as_str(), name, key.as_str()) {
            ("gc", "", "auto") => self.auto = number(value).or(self.auto),
            ("gc", "", "autopacklimit") => self.pack_limit = number(value).or(self.pack_limit),
            ("maintenance", "", "auto") => self.maintenance = boolean(value).or(self.maintenance),
            ("maintenance", "", "strategy") => self.strategy = Some(value.to_ascii_lowercase()),
            ("maintenance", "gc", "enabled") => self.gc_task = boolean(value).or(self.gc_task),
            _ => {}
        }
    }

    /// The loose objects and the packs not kept past which git gathers,
    /// none where it never does. `gc.auto` at zero or less turns both off,
    /// as `maintenance.auto` false does, and so does the `gc` task off, or
    /// a strategy that doesn't run it (none, or geometric, which replaces
    /// it) unless the task is turned on by name; `gc.autoPackLimit` at zero
    /// or less turns off only its own half.
    fn judged(&self) -> (Option<u64>, Option<u64>) {
        let replaced = matches!(self.strategy.as_deref(), Some("none" | "geometric"));
        let task = self.gc_task.unwrap_or(!replaced);
        let auto = self.auto.unwrap_or(LOOSE_LIMIT);
        if auto <= 0 || self.maintenance == Some(false) || !task {
            return (None, None);
        }
        let positive = |limit: i64| u64::try_from(limit).ok().filter(|limit| *limit > 0);
        (
            positive(auto),
            positive(self.pack_limit.unwrap_or(PACK_LIMIT)),
        )
    }
}

/// The store under the git folder `common`, a checkout's own, judged by
/// `limits`; none where its objects can't be read.
pub fn weigh(common: &Path, limits: &Limits) -> Option<Weight> {
    let objects = common.join("objects");
    let rooms = loose_in(&objects.join(SAMPLED))?;
    let sampled = rooms.len() as u64;
    let estimated = sampled >= ENOUGH;
    let (loose, loose_bytes) = if estimated {
        // The sample's largest object is counted once rather than 256 times,
        // so one large file there doesn't swell the whole, while the rest,
        // large ones among them, stand for every folder's.
        let sample = rooms
            .iter()
            .fold(0_u64, |sum, room| sum.saturating_add(*room));
        let largest = rooms.iter().copied().max().unwrap_or_default();
        (
            sampled.saturating_mul(FOLDERS),
            (sample - largest)
                .saturating_mul(FOLDERS)
                .saturating_add(largest),
        )
    } else {
        let mut every = (0_u64, 0_u64);
        for first in 0..FOLDERS {
            let rooms = loose_in(&objects.join(format!("{first:02x}")))?;
            let bytes = rooms
                .iter()
                .fold(0_u64, |sum, room| sum.saturating_add(*room));
            every = (
                every.0.saturating_add(rooms.len() as u64),
                every.1.saturating_add(bytes),
            );
        }
        every
    };
    let (mut packs, mut gathered, mut packed_bytes) = (0_u64, 0_u64, 0_u64);
    for entry in fs::read_dir(objects.join("pack")).ok()?.flatten() {
        packed_bytes = packed_bytes.saturating_add(entry.metadata().map_or(0, |data| room(&data)));
        let index = entry.path();
        // Git loads a pack from its index, and only with the pack beside it.
        if index.extension().is_none_or(|extension| extension != "idx")
            || !index.with_extension("pack").exists()
        {
            continue;
        }
        packs += 1;
        if !index.with_extension("keep").exists() {
            gathered += 1;
        }
    }
    let (loose_limit, pack_limit) = limits.judged();
    let over = |limit: Option<u64>, count: u64| limit.is_some_and(|limit| count > limit);
    Some(Weight {
        loose,
        estimated,
        loose_bytes,
        packs,
        packed_bytes,
        due: over(loose_limit.map(|limit| limit.div_ceil(FOLDERS)), sampled)
            || over(pack_limit, gathered),
    })
}

/// The room each loose object in one folder takes: none where it's there
/// but can't be read, and nothing where git hasn't made it.
fn loose_in(folder: &Path) -> Option<Vec<u64>> {
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Some(Vec::new()),
        Err(_) => return None,
    };
    Some(
        entries
            .flatten()
            .filter(|entry| is_object(&entry.file_name().to_string_lossy()))
            .map(|entry| entry.metadata().map_or(0, |data| room(&data)))
            .collect(),
    )
}

/// The room the disk gives a file, in its blocks of 512 bytes, as `git
/// count-objects` counts it: a loose object of a hundred bytes takes a
/// whole block of the disk's.
fn room(data: &Metadata) -> u64 {
    data.blocks().saturating_mul(512)
}

/// Whether a name in a loose objects folder is an object's, as git names
/// one: the rest of its hex name, 38 digits for SHA-1 or 62 for SHA-256.
fn is_object(name: &str) -> bool {
    matches!(name.len(), 38 | 62)
        && name
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

/// A config's integer as git reads one, with an optional k, m or g.
fn number(value: &str) -> Option<i64> {
    let (digits, scale) = match value.chars().last()?.to_ascii_lowercase() {
        'k' => (&value[..value.len() - 1], 1 << 10),
        'm' => (&value[..value.len() - 1], 1 << 20),
        'g' => (&value[..value.len() - 1], 1 << 30),
        _ => (value, 1),
    };
    digits.parse::<i64>().ok()?.checked_mul(scale)
}

/// A config's boolean in any of git's words for one; an empty value is
/// false, as git reads `key =`.
fn boolean(value: &str) -> Option<bool> {
    let said = |words: &[&str]| words.iter().any(|word| value.eq_ignore_ascii_case(word));
    if value.is_empty() || said(&["false", "no", "off", "0"]) {
        Some(false)
    } else if said(&["true", "yes", "on", "1"]) {
        Some(true)
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
