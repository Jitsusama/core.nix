//! A disk's partition table, entry by entry, as `sfdisk --json` gives it:
//! what a layout beside another system's partitions reads before it adds
//! its own, and again after, to prove it changed nothing else.

use serde::Deserialize;

use crate::{Fault, Machine, Tool};

/// The partition type GPT gives an EFI system partition.
pub const ESP: &str = "C12A7328-F81F-11D2-BA4B-00A0C93EC93B";

/// GPT has room for this many entries, the most sfdisk and sgdisk make.
const ENTRIES: u32 = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// The size of a sector, in bytes, which every start and size counts
    /// in: 4096 on an Apple silicon Mac's disk, 512 on most others.
    pub sector_size: u64,
    /// The first and last sectors a partition may use.
    pub first: u64,
    pub last: u64,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Entry {
    /// Its number in the table, which the kernel names its device by.
    #[serde(skip)]
    pub number: u32,
    pub node: String,
    pub start: u64,
    pub size: u64,
    #[serde(rename = "type")]
    pub kind: String,
    pub uuid: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub attrs: Option<String>,
}

/// A stretch of sectors no partition holds, first and last inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gap {
    pub first: u64,
    pub last: u64,
}

impl Gap {
    pub fn sectors(self) -> u64 {
        self.last + 1 - self.first
    }
}

impl Entry {
    /// Whether it's an EFI system partition.
    pub fn is_esp(&self) -> bool {
        self.kind.eq_ignore_ascii_case(ESP)
    }

    /// Whether it holds the same place, type, identity and attributes as
    /// another, its name aside.
    pub fn same_but_named(&self, other: &Entry) -> bool {
        self.number == other.number
            && self.start == other.start
            && self.size == other.size
            && self.kind.eq_ignore_ascii_case(&other.kind)
            && self.uuid.eq_ignore_ascii_case(&other.uuid)
            && self.attrs == other.attrs
    }

    fn last(&self) -> u64 {
        self.start + self.size - 1
    }
}

impl Table {
    /// The entry with this GPT name.
    pub fn named(&self, name: &str) -> Option<&Entry> {
        self.entries
            .iter()
            .find(|entry| entry.name.as_deref() == Some(name))
    }

    /// The entry with this unique partition GUID.
    pub fn with_uuid(&self, uuid: &str) -> Option<&Entry> {
        self.entries
            .iter()
            .find(|entry| entry.uuid.eq_ignore_ascii_case(uuid))
    }

    /// The lowest number no entry has, as the kernel names a new one.
    pub fn free_number(&self) -> Option<u32> {
        (1..=ENTRIES).find(|number| self.entries.iter().all(|entry| entry.number != *number))
    }

    /// One MiB in sectors, which a new partition starts on a multiple of.
    pub fn alignment(&self) -> u64 {
        (1 << 20) / self.sector_size
    }

    /// The largest stretch no partition holds, its start moved up to the
    /// alignment, or none when there's no room at all.
    pub fn largest_gap(&self) -> Option<Gap> {
        let mut held: Vec<(u64, u64)> = self
            .entries
            .iter()
            .map(|entry| (entry.start, entry.last()))
            .collect();
        held.sort_unstable();
        let align = self.alignment();
        let mut gaps = vec![];
        let mut next = self.first;
        for (start, last) in held {
            if start > next {
                gaps.push((next, start - 1));
            }
            next = next.max(last + 1);
        }
        if next <= self.last {
            gaps.push((next, self.last));
        }
        gaps.into_iter()
            .filter_map(|(first, last)| {
                let first = first.div_ceil(align) * align;
                (first <= last).then_some(Gap { first, last })
            })
            .max_by_key(|gap| gap.sectors())
    }

    /// Its size in bytes.
    pub fn bytes(&self, sectors: u64) -> u64 {
        sectors * self.sector_size
    }
}

pub fn observe(machine: &mut dyn Machine, disk: &str) -> Result<Table, Fault> {
    parse(&machine.run(Tool::Sfdisk, &["--json", disk])?.ok()?, disk)
}

#[derive(Deserialize)]
struct Listing {
    partitiontable: Raw,
}

#[derive(Deserialize)]
struct Raw {
    label: String,
    sectorsize: u64,
    firstlba: u64,
    lastlba: u64,
    #[serde(default)]
    partitions: Vec<Entry>,
}

/// The table of `disk`, from what sfdisk printed for it. Only GPT counts,
/// since a layout beside another system's partitions adds one by its name.
pub fn parse(json: &str, disk: &str) -> Result<Table, Fault> {
    let unreadable = |why: String| Fault::unreadable("sfdisk's partition table", why);
    let raw = serde_json::from_str::<Listing>(json)
        .map_err(|why| unreadable(why.to_string()))?
        .partitiontable;
    if raw.label != "gpt" {
        return Err(unreadable(format!(
            "{disk} has a {} table, not GPT",
            raw.label
        )));
    }
    let entries = raw
        .partitions
        .into_iter()
        .map(|mut entry| {
            entry.number = number(&entry.node, disk)
                .ok_or_else(|| unreadable(format!("{} isn't a partition of {disk}", entry.node)))?;
            Ok(entry)
        })
        .collect::<Result<Vec<Entry>, Fault>>()?;
    Ok(Table {
        sector_size: raw.sectorsize,
        first: raw.firstlba,
        last: raw.lastlba,
        entries,
    })
}

/// A partition's number from its device's name: the disk's name and a
/// number, with a `p` between them when the disk's name ends in a digit,
/// as `/dev/nvme0n1p5` and `/dev/sda5` have.
fn number(node: &str, disk: &str) -> Option<u32> {
    let rest = node.strip_prefix(disk)?;
    rest.strip_prefix('p').unwrap_or(rest).parse().ok()
}
