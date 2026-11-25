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

/// A multi-pack-index: one sorted table of names across many packs.
pub struct Midx {
    file: File,
    /// The `.idx` names of the packs it covers, in its order, each a bare
    /// file name of the pack folder.
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
        // The names run to the next chunk, and never past the file's end, so
        // a chunk table that says otherwise costs no more than the file.
        let size = file.metadata()?.len();
        let end = found
            .values()
            .copied()
            .filter(|&at| at > names_at)
            .min()
            .map_or(size, |next| next.min(size));
        let len = end
            .checked_sub(names_at)
            .and_then(|len| usize::try_from(len).ok())
            .ok_or_else(|| invalid("multi-pack-index names past its end"))?;
        let raw = read_exact_at(&file, len, names_at)?;
        let packs: Vec<String> = raw
            .split(|&b| b == 0)
            .filter(|name| !name.is_empty())
            .take(pack_count)
            .map(pack_name)
            .collect::<io::Result<_>>()?;
        if packs.len() < pack_count {
            return Err(invalid("multi-pack-index names fewer packs than it holds"));
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    // Names spread over first bytes, one of them sharing a first byte with
    // another, as the fan-out must count.
    const NAMES: [[u8; 20]; 4] = [[0x01; 20], [0x7f; 20], [0x80; 20], [0xfe; 20]];
    const SAME_FIRST: [u8; 20] = {
        let mut name = [0x7f; 20];
        name[19] = 0x80;
        name
    };

    fn sorted() -> Vec<[u8; 20]> {
        let mut names = NAMES.to_vec();
        names.push(SAME_FIRST);
        names.sort_unstable();
        names
    }

    fn be32(value: u32) -> [u8; 4] {
        value.to_be_bytes()
    }

    fn count(names: &[[u8; 20]]) -> u32 {
        u32::try_from(names.len()).expect("a handful of names")
    }

    fn fanout(names: &[[u8; 20]]) -> Vec<u8> {
        (0..=255u8)
            .flat_map(|byte| {
                be32(count(
                    &names
                        .iter()
                        .copied()
                        .filter(|name| name[0] <= byte)
                        .collect::<Vec<_>>(),
                ))
            })
            .collect()
    }

    /// Writes `bytes` where a test alone reads them.
    fn file(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("git-engine-{}-{name}", std::process::id()));
        std::fs::write(&path, bytes).expect("the temp folder takes a file");
        path
    }

    /// Git ends each index with the checksums of its pack and of itself,
    /// which nothing here reads but a window of names may run into.
    fn trailed(mut bytes: Vec<u8>) -> Vec<u8> {
        bytes.extend([0; 40]);
        bytes
    }

    /// Where each name lies in its pack, the last past what four bytes hold.
    fn offsets(names: &[[u8; 20]]) -> Vec<u64> {
        (0..names.len())
            .map(|at| {
                if at + 1 == names.len() {
                    0x1_2345_6789
                } else {
                    12 + 100 * at as u64
                }
            })
            .collect()
    }

    fn version_1(names: &[[u8; 20]]) -> Vec<u8> {
        let mut bytes = fanout(names);
        for (name, offset) in names.iter().zip(offsets(names)) {
            bytes.extend(be32(u32::try_from(offset).unwrap_or(0)));
            bytes.extend(name);
        }
        trailed(bytes)
    }

    fn version_2(names: &[[u8; 20]]) -> Vec<u8> {
        let mut bytes = b"\xfftOc".to_vec();
        bytes.extend(be32(2));
        bytes.extend(fanout(names));
        bytes.extend(names.concat());
        bytes.extend(vec![0; 4 * names.len()]);
        let mut large: Vec<u64> = Vec::new();
        for offset in offsets(names) {
            if let Ok(small) = u32::try_from(offset)
                && small & 0x8000_0000 == 0
            {
                bytes.extend(be32(small));
            } else {
                let at = u32::try_from(large.len()).expect("few large offsets");
                bytes.extend(be32(0x8000_0000 | at));
                large.push(offset);
            }
        }
        bytes.extend(large.iter().flat_map(|offset| offset.to_be_bytes()));
        trailed(bytes)
    }

    /// A multi-pack-index over `packs`, its names spread across them in
    /// turn, claiming `pack_count` packs, with its pack names' chunk at
    /// `names_at` when given and where they lie otherwise.
    fn midx(packs: &[&str], pack_count: u32, names_at: Option<u64>) -> Vec<u8> {
        let names = sorted();
        let mut pnam = packs.join("\0").into_bytes();
        pnam.push(0);
        while !pnam.len().is_multiple_of(4) {
            pnam.push(0);
        }
        let mut ooff = Vec::new();
        let mut loff = Vec::new();
        for (at, offset) in offsets(&names).into_iter().enumerate() {
            ooff.extend(be32(
                u32::try_from(at % packs.len().max(1)).expect("few packs"),
            ));
            if offset > 0x7fff_ffff {
                ooff.extend(be32(
                    0x8000_0000 | u32::try_from(loff.len() / 8).expect("few"),
                ));
                loff.extend(offset.to_be_bytes());
            } else {
                ooff.extend(be32(u32::try_from(offset).expect("small")));
            }
        }
        let chunks: [(&[u8; 4], Vec<u8>); 5] = [
            (b"PNAM", pnam),
            (b"OIDF", fanout(&names)),
            (b"OIDL", names.concat()),
            (b"OOFF", ooff),
            (b"LOFF", loff),
        ];
        let mut at = 12 + 12 * (chunks.len() as u64 + 1);
        let mut bytes = b"MIDX".to_vec();
        bytes.extend([1, 1, u8::try_from(chunks.len()).expect("five"), 0]);
        bytes.extend(be32(pack_count));
        for (id, chunk) in &chunks {
            bytes.extend(*id);
            let start = if *id == b"PNAM" {
                names_at.unwrap_or(at)
            } else {
                at
            };
            bytes.extend(start.to_be_bytes());
            at += chunk.len() as u64;
        }
        bytes.extend([0; 4]);
        bytes.extend(at.to_be_bytes());
        for (_, chunk) in chunks {
            bytes.extend(chunk);
        }
        // Its own checksum, as git's ends with.
        bytes.extend([0; 20]);
        bytes
    }

    fn found(index: &Index) -> Vec<Option<u64>> {
        let mut absent = [0x7f; 20];
        absent[19] = 0x01;
        sorted()
            .into_iter()
            .chain([absent])
            .map(|name| index.offset(&Oid(name)).expect("the index reads"))
            .collect()
    }

    #[test]
    fn a_version_1_index_finds_each_name_where_its_pack_holds_it() {
        let names = sorted();
        let index = Index::open(&file("v1.idx", &version_1(&names))).expect("it opens");
        let mut want: Vec<Option<u64>> = offsets(&names)
            .into_iter()
            .map(|offset| Some(offset & 0xffff_ffff))
            .collect();
        // Version 1 holds four bytes of offset, so the last reads as written.
        *want.last_mut().expect("names") = Some(0);
        want.push(None);
        assert_eq!(found(&index), want);
    }

    #[test]
    fn a_version_2_index_finds_an_offset_past_four_bytes_in_its_large_table() {
        let names = sorted();
        let index = Index::open(&file("v2.idx", &version_2(&names))).expect("it opens");
        let mut want: Vec<Option<u64>> = offsets(&names).into_iter().map(Some).collect();
        want.push(None);
        assert_eq!(found(&index), want);
    }

    #[test]
    fn a_multi_pack_index_says_which_pack_holds_each_name_and_where() {
        let packs = ["pack-a.idx", "pack-b.idx"];
        let midx = Midx::open(&file("multi-pack-index", &midx(&packs, 2, None))).expect("it opens");
        assert_eq!(midx.packs, packs);
        let names = sorted();
        let located: Vec<_> = names
            .iter()
            .map(|name| midx.locate(&Oid(*name)).expect("the index reads"))
            .collect();
        let want: Vec<_> = offsets(&names)
            .into_iter()
            .enumerate()
            .map(|(at, offset)| Some((at % 2, offset)))
            .collect();
        assert_eq!(located, want);
        assert_eq!(
            midx.locate(&Oid([0x02; 20])).expect("the index reads"),
            None
        );
    }

    #[test]
    fn a_multi_pack_index_naming_a_file_outside_the_pack_folder_is_refused() {
        for name in [
            "../objects.idx",
            "sub/pack-a.idx",
            ".hidden.idx",
            "pack-a.pack",
            "pack-a\\b.idx",
        ] {
            let bytes = midx(&["pack-a.idx", name], 2, None);
            let opened = Midx::open(&file("named-outside", &bytes));
            assert!(opened.is_err(), "a pack named {name:?} was taken");
        }
    }

    #[test]
    fn a_multi_pack_index_naming_fewer_packs_than_it_holds_is_refused() {
        let bytes = midx(&["pack-a.idx"], 2, None);
        assert!(
            Midx::open(&file("named-fewer", &bytes)).is_err(),
            "one name was taken for two packs"
        );
    }

    #[test]
    fn a_multi_pack_index_reads_no_further_than_its_own_end() {
        // The names are said to start past the file's end.
        let past = midx(&["pack-a.idx"], 1, Some(1 << 40));
        assert!(
            Midx::open(&file("names-past", &past)).is_err(),
            "names past the end were read"
        );
        // Every chunk after the names is said to lie a terabyte on, so the
        // file's end is all that bounds them.
        let mut far = midx(&["pack-a.idx"], 1, None);
        for entry in 1..5 {
            let at = 12 + 12 * entry + 4;
            far[at..at + 8].copy_from_slice(&((1u64 << 40) + entry as u64).to_be_bytes());
        }
        let midx = Midx::open(&file("chunk-far", &far)).expect("the names it has are read");
        assert_eq!(midx.packs, ["pack-a.idx"]);
    }
}
