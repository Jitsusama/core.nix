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

/// A multi-pack-index: one sorted table of names across many packs.
pub struct Midx {
    file: File,
    /// The `.idx` names of the packs it covers, in its order.
    pub packs: Vec<String>,
    fanout: u64,
    names: u64,
    offsets: u64,
    large: Option<u64>,
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
        let names_at = chunk(b"PNAM")?;
        let pack_count = u32::from_be_bytes(head[8..12].try_into().expect("4 bytes")) as usize;
        // The names run to the next chunk; read generously and split.
        let next = found.values().copied().filter(|&at| at > names_at).min();
        let len = next.map_or(65_536, |next| next - names_at);
        let raw = read_exact_at(&file, usize::try_from(len).unwrap_or(65_536), names_at)?;
        let packs: Vec<String> = raw
            .split(|&b| b == 0)
            .filter(|name| !name.is_empty())
            .take(pack_count)
            .map(|name| String::from_utf8_lossy(name).into_owned())
            .collect();
        Ok(Midx {
            file,
            packs,
            fanout: chunk(b"OIDF")?,
            names: chunk(b"OIDL")?,
            offsets: chunk(b"OOFF")?,
            large: found.get(b"LOFF").copied(),
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
            let large = self
                .large
                .ok_or_else(|| invalid("large offset without LOFF"))?;
            u64_at(&self.file, large + 8 * u64::from(small & 0x7fff_ffff))?
        };
        Ok(Some((pack, offset)))
    }
}
