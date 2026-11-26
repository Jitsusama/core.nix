//! Where an object lies in a pack, found through the pack's own `.idx` or
//! one multi-pack-index across many, and the search of sorted names they
//! share with the commit-graph. Apart from the object store, since what
//! changes here is how git lays out its indexes, not how objects are read.

use crate::hash::Oid;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::os::unix::fs::FileExt;
use std::path::Path;

// Its own, as the commit-graph's and the index's are, so this module needs
// nothing from the object store that reads through it.
fn invalid(what: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, what.to_string())
}

fn read_exact_at(file: &File, len: usize, offset: u64) -> io::Result<Vec<u8>> {
    let mut buf = vec![0; len];
    file.read_exact_at(&mut buf, offset)?;
    Ok(buf)
}

fn u32_at(file: &File, offset: u64) -> io::Result<u32> {
    let mut buf = [0; 4];
    file.read_exact_at(&mut buf, offset)?;
    Ok(u32::from_be_bytes(buf))
}

fn u64_at(file: &File, offset: u64) -> io::Result<u64> {
    let mut buf = [0; 8];
    file.read_exact_at(&mut buf, offset)?;
    Ok(u64::from_be_bytes(buf))
}

/// The first eight bytes of a name, as a number, for guessing where among
/// sorted names it falls.
fn leading(name: &[u8]) -> u64 {
    u64::from_be_bytes(name[..8].try_into().expect("8 bytes"))
}

/// How many names one read takes in, around a guess.
const WINDOW: u32 = 128;

/// Finds an object name among sorted names starting at `names` in a file,
/// `stride` bytes apart, narrowed first by the 256-entry fan-out at `fanout`.
/// Names are uniformly random, so the guess from interpolating between the
/// ends lands within a window of the answer: one or two reads, where a
/// binary search through millions would take two dozen.
pub fn search(
    file: &File,
    fanout: u64,
    names: u64,
    stride: u64,
    oid: &Oid,
) -> io::Result<Option<u32>> {
    let first = usize::from(oid.0[0]);
    let mut low = if first == 0 {
        0
    } else {
        u32_at(file, fanout + 4 * (first as u64 - 1))?
    };
    let mut high = u32_at(file, fanout + 4 * first as u64)?;
    // Within one first byte, names span this range of leading values.
    let (mut low_value, mut high_value) = (
        (first as u64) << 56,
        ((first as u64) << 56) | ((1 << 56) - 1),
    );
    let want = leading(&oid.0);
    let mut buf = Vec::new();
    while low < high {
        let span = high - low;
        if span <= WINDOW * 2 {
            return scan(file, names, stride, low, high, oid, &mut buf);
        }
        let guess = low + interpolate(span, want, low_value, high_value);
        let start = guess.saturating_sub(WINDOW / 2).max(low);
        let end = (start + WINDOW).min(high);
        read_names(file, names, stride, start, end, &mut buf)?;
        let first_name = &buf[..20];
        let last_at = usize::try_from(u64::from(end - start - 1) * stride)
            .map_err(|_| invalid("a pack index past what memory holds"))?;
        let last_name = &buf[last_at..][..20];
        if oid.0.as_slice() < first_name {
            high = start;
            high_value = leading(first_name);
        } else if oid.0.as_slice() > last_name {
            low = end;
            low_value = leading(last_name);
        } else {
            return Ok(find_in(&buf, stride, oid).map(|at| start + at));
        }
    }
    Ok(None)
}

/// How far into a span of names the one wanted likely sits, by where its
/// leading value lies between the span's ends.
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a guess needs no more than a float's precision, and is a share of the span"
)]
fn interpolate(span: u32, want: u64, low_value: u64, high_value: u64) -> u32 {
    let fraction =
        (want.saturating_sub(low_value) as f64) / ((high_value - low_value).max(1) as f64);
    ((f64::from(span) * fraction) as u32).min(span - 1)
}

fn read_names(
    file: &File,
    names: u64,
    stride: u64,
    start: u32,
    end: u32,
    buf: &mut Vec<u8>,
) -> io::Result<()> {
    let len = usize::try_from(u64::from(end - start) * stride)
        .map_err(|_| invalid("a pack index past what memory holds"))?;
    buf.resize(len, 0);
    file.read_exact_at(buf, names + stride * u64::from(start))
}

fn find_in(buf: &[u8], stride: u64, oid: &Oid) -> Option<u32> {
    let stride = usize::try_from(stride).ok()?;
    let count = buf.len() / stride;
    let (mut low, mut high) = (0, count);
    while low < high {
        let mid = low + (high - low) / 2;
        match buf[mid * stride..mid * stride + 20].cmp(&oid.0) {
            std::cmp::Ordering::Equal => return u32::try_from(mid).ok(),
            std::cmp::Ordering::Less => low = mid + 1,
            std::cmp::Ordering::Greater => high = mid,
        }
    }
    None
}

fn scan(
    file: &File,
    names: u64,
    stride: u64,
    low: u32,
    high: u32,
    oid: &Oid,
    buf: &mut Vec<u8>,
) -> io::Result<Option<u32>> {
    read_names(file, names, stride, low, high, buf)?;
    Ok(find_in(buf, stride, oid).map(|at| low + at))
}

/// One pack's `.idx`, version 1 or 2.
pub struct Index {
    file: File,
    version: u32,
    count: u32,
}

impl Index {
    pub fn open(path: &Path) -> io::Result<Index> {
        let file = File::open(path)?;
        let mut head = [0; 8];
        file.read_exact_at(&mut head, 0)?;
        if head[..4] == *b"\xfftOc" {
            let version = u32::from_be_bytes(head[4..8].try_into().expect("4 bytes"));
            if version != 2 {
                return Err(invalid("pack index version"));
            }
            let count = u32_at(&file, 8 + 255 * 4)?;
            Ok(Index {
                file,
                version,
                count,
            })
        } else {
            let count = u32_at(&file, 255 * 4)?;
            Ok(Index {
                file,
                version: 1,
                count,
            })
        }
    }

    pub fn offset(&self, oid: &Oid) -> io::Result<Option<u64>> {
        if self.version == 1 {
            let Some(at) = search(&self.file, 0, 1024 + 4, 24, oid)? else {
                return Ok(None);
            };
            return Ok(Some(u64::from(u32_at(
                &self.file,
                1024 + 24 * u64::from(at),
            )?)));
        }
        let Some(at) = search(&self.file, 8, 8 + 1024, 20, oid)? else {
            return Ok(None);
        };
        let count = u64::from(self.count);
        let offsets = 8 + 1024 + 20 * count + 4 * count;
        let small = u32_at(&self.file, offsets + 4 * u64::from(at))?;
        if small & 0x8000_0000 == 0 {
            return Ok(Some(u64::from(small)));
        }
        let large = offsets + 4 * count + 8 * u64::from(small & 0x7fff_ffff);
        Ok(Some(u64_at(&self.file, large)?))
    }
}

/// One pack's `.idx` name as a multi-pack-index gives it, taken only as a
/// file of the pack folder, since the store joins it onto that folder: a
/// name that climbs out or into a subfolder is a crafted or broken index.
#[expect(
    clippy::case_sensitive_file_extension_comparisons,
    reason = "git names a pack's index .idx, in lowercase"
)]
fn pack_name(raw: &[u8]) -> io::Result<String> {
    let name = std::str::from_utf8(raw).map_err(|_| invalid("multi-pack-index pack name"))?;
    let bare = !name.starts_with('.') && !name.contains(['/', '\\']);
    if bare && name.ends_with(".idx") {
        Ok(name.to_owned())
    } else {
        Err(invalid("multi-pack-index pack name"))
    }
}

/// How many bytes of pack names one read takes in: dozens of git's names.
const NAMES_READ: u64 = 4096;

/// The first `count` pack names of a names chunk running from `at` to
/// `end`, read a window at a time. Git ends each name with a NUL and pads
/// only after the last, so an empty name before the last is an index
/// naming fewer packs than it holds, refused at the first window rather
/// than once a sparse file's whole apparent tail is read.
fn pack_names(file: &File, mut at: u64, end: u64, count: usize) -> io::Result<Vec<String>> {
    let fewer = || invalid("multi-pack-index names fewer packs than it holds");
    let mut packs = Vec::new();
    let mut name = Vec::new();
    while packs.len() < count {
        let len = NAMES_READ.min(end - at);
        if len == 0 {
            return Err(fewer());
        }
        let window = read_exact_at(file, usize::try_from(len).expect("a window"), at)?;
        at += len;
        for byte in window {
            if byte != 0 {
                // No file name passes 255 bytes.
                if name.len() == 255 {
                    return Err(invalid("multi-pack-index pack name"));
                }
                name.push(byte);
                continue;
            }
            if name.is_empty() {
                return Err(fewer());
            }
            packs.push(pack_name(&name)?);
            name.clear();
            if packs.len() == count {
                break;
            }
        }
    }
    Ok(packs)
}

/// A multi-pack-index: one sorted table of names across many packs.
pub struct Midx {
    file: File,
    /// The `.idx` names of the packs it covers, in its order, each a bare
    /// file name of the pack folder.
    pub packs: Vec<String>,
    fanout: u64,
    names: u64,
    offsets: u64,
    /// Where its large offsets lie, and how many its chunk holds.
    large: Option<(u64, u64)>,
}

impl Midx {
    pub fn open(path: &Path) -> io::Result<Midx> {
        let file = File::open(path)?;
        let head = read_exact_at(&file, 12, 0)?;
        if head[..4] != *b"MIDX" || head[5] != 1 || head[7] != 0 {
            return Err(invalid("multi-pack-index header"));
        }
        let chunks = usize::from(head[6]);
        let table = read_exact_at(&file, 12 * (chunks + 1), 12)?;
        let mut found = HashMap::new();
        for entry in table.as_chunks::<12>().0.iter().take(chunks) {
            let id: [u8; 4] = entry[..4].try_into().expect("4 bytes");
            let at = u64::from_be_bytes(entry[4..].try_into().expect("8 bytes"));
            found.insert(id, at);
        }
        let chunk = |id: &[u8; 4]| {
            found
                .get(id)
                .copied()
                .ok_or_else(|| invalid("missing chunk"))
        };
        // Every chunk lies before the checksum git ends the file with. One
        // said to lie further is a crafted or broken index, refused here
        // rather than at each lookup, so the store reads each pack's own
        // index instead. The table just read already holds more than the
        // checksum's 20 bytes.
        let body = file.metadata()?.len().saturating_sub(20);
        let within = |at: u64, len: u64, what: &str| {
            at.checked_add(len)
                .filter(|&end| end <= body)
                .map(|_| at)
                .ok_or_else(|| invalid(what))
        };
        let fanout = within(
            chunk(b"OIDF")?,
            1024,
            "multi-pack-index fan-out past its end",
        )?;
        // Sorted names give counts that never fall. One that does would
        // send a lookup past the names, so it's refused here, where the
        // store reads each pack's own index instead.
        let counts: Vec<u32> = read_exact_at(&file, 1024, fanout)?
            .as_chunks::<4>()
            .0
            .iter()
            .map(|count| u32::from_be_bytes(*count))
            .collect();
        if counts.windows(2).any(|pair| pair[0] > pair[1]) {
            return Err(invalid("multi-pack-index fan-out out of order"));
        }
        let objects = u64::from(counts[255]);
        let names = within(
            chunk(b"OIDL")?,
            20 * objects,
            "multi-pack-index object names past its end",
        )?;
        let offsets = within(
            chunk(b"OOFF")?,
            8 * objects,
            "multi-pack-index offsets past its end",
        )?;
        // A chunk whose length the table alone gives runs to the next
        // chunk, or to the checksum.
        let end_of = |at: u64| {
            found
                .values()
                .copied()
                .filter(|&next| next > at)
                .min()
                .map_or(body, |next| next.min(body))
        };
        let large = found
            .get(b"LOFF")
            .map(|&at| within(at, 0, "multi-pack-index large offsets past its end"))
            .transpose()?
            .map(|at| (at, (end_of(at) - at) / 8));
        let names_at = within(chunk(b"PNAM")?, 0, "multi-pack-index names past its end")?;
        let pack_count = u32::from_be_bytes(head[8..12].try_into().expect("4 bytes")) as usize;
        let packs = pack_names(&file, names_at, end_of(names_at), pack_count)?;
        Ok(Midx {
            file,
            packs,
            fanout,
            names,
            offsets,
            large,
        })
    }

    /// Which of its packs holds `oid`, by place in `packs`, and where in it.
    pub fn locate(&self, oid: &Oid) -> io::Result<Option<(usize, u64)>> {
        let Some(at) = search(&self.file, self.fanout, self.names, 20, oid)? else {
            return Ok(None);
        };
        let entry = read_exact_at(&self.file, 8, self.offsets + 8 * u64::from(at))?;
        let pack = u32::from_be_bytes(entry[..4].try_into().expect("4 bytes")) as usize;
        let small = u32::from_be_bytes(entry[4..].try_into().expect("4 bytes"));
        let offset = if small & 0x8000_0000 == 0 {
            u64::from(small)
        } else {
            let (large, held) = self
                .large
                .ok_or_else(|| invalid("large offset without LOFF"))?;
            // One past what its chunk holds would read the next chunk, or
            // the checksum, as an offset.
            let entry = u64::from(small & 0x7fff_ffff);
            if entry >= held {
                return Err(invalid("multi-pack-index large offset past its chunk"));
            }
            u64_at(&self.file, large + 8 * entry)?
        };
        Ok(Some((pack, offset)))
    }
}

// The store's tests build their indexes with these, so they're the crate's.
#[cfg(test)]
#[path = "pack_index_tests.rs"]
pub(crate) mod tests;
