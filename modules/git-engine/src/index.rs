//! git's index, versions 2 to 4: each entry's path, mode, object and the
//! file's stat as it was when git last looked, plus the cache-tree, which
//! names the tree each folder of the index would make. A sparse index's
//! folders outside the checkout are single entries holding their tree.

use crate::hash::Oid;
use std::io;

pub const ASSUME_VALID: u16 = 0x8000;
pub const SKIP_WORKTREE: u16 = 0x4000;
pub const INTENT_TO_ADD: u16 = 0x2000;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StatData {
    pub ctime: (u32, u32),
    pub mtime: (u32, u32),
    // The device isn't kept: git leaves it out of its own check too.
    pub ino: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: u32,
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub path: Vec<u8>,
    pub mode: u32,
    pub oid: Oid,
    pub stage: u8,
    pub flags: u16,
    pub extended: u16,
    pub stat: StatData,
}

impl Entry {
    pub fn assume_valid(&self) -> bool {
        self.flags & ASSUME_VALID != 0
    }

    pub fn skip_worktree(&self) -> bool {
        self.extended & SKIP_WORKTREE != 0
    }

    pub fn intent_to_add(&self) -> bool {
        self.extended & INTENT_TO_ADD != 0
    }

    /// A sparse index's folder: its whole tree, outside the checkout.
    pub fn is_sparse_dir(&self) -> bool {
        self.mode == 0o040_000
    }
}

/// One folder's cached tree: how many entries it covers (negative when a
/// change since made it stale) and the tree it makes.
#[derive(Clone, Debug, Default)]
pub struct CacheTree {
    pub name: Vec<u8>,
    pub entries: i64,
    pub oid: Option<Oid>,
    pub children: Vec<CacheTree>,
}

impl CacheTree {
    pub fn child(&self, name: &[u8]) -> Option<&CacheTree> {
        self.children.iter().find(|child| child.name == name)
    }
}

pub struct Index {
    pub entries: Vec<Entry>,
    pub tree: Option<CacheTree>,
    /// When the index was written, for telling a racily clean file from a
    /// clean one.
    pub written: (i64, i64),
}

fn invalid(what: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("index: {what}"))
}

fn u32_at(data: &[u8], at: usize) -> io::Result<u32> {
    data.get(at..at + 4)
        .map(|b| u32::from_be_bytes(b.try_into().expect("4 bytes")))
        .ok_or_else(|| invalid("truncated"))
}

fn u16_at(data: &[u8], at: usize) -> io::Result<u16> {
    data.get(at..at + 2)
        .map(|b| u16::from_be_bytes(b.try_into().expect("2 bytes")))
        .ok_or_else(|| invalid("truncated"))
}

/// git's offset varint, which index version 4 uses for its name prefixes.
fn varint(data: &[u8], at: &mut usize) -> io::Result<usize> {
    let mut byte = *data.get(*at).ok_or_else(|| invalid("varint"))?;
    *at += 1;
    let mut value = usize::from(byte & 0x7f);
    while byte & 0x80 != 0 {
        byte = *data.get(*at).ok_or_else(|| invalid("varint"))?;
        *at += 1;
        value = ((value + 1) << 7) | usize::from(byte & 0x7f);
    }
    Ok(value)
}

impl Index {
    /// A repository with no index yet.
    pub fn empty() -> Index {
        Index {
            entries: Vec::new(),
            tree: None,
            written: (0, 0),
        }
    }

    pub fn parse(data: &[u8], written: (i64, i64)) -> io::Result<Index> {
        if data.len() < 32 || data[..4] != *b"DIRC" {
            return Err(invalid("header"));
        }
        let version = u32_at(data, 4)?;
        if !(2..=4).contains(&version) {
            return Err(invalid("version"));
        }
        let count = u32_at(data, 8)? as usize;
        let end = data.len() - 20;
        let mut entries = Vec::with_capacity(count);
        let mut at = 12;
        let mut last: Vec<u8> = Vec::new();
        for _ in 0..count {
            let start = at;
            let word = |n: usize| u32_at(data, start + 4 * n);
            let stat = StatData {
                ctime: (word(0)?, word(1)?),
                mtime: (word(2)?, word(3)?),
                ino: word(5)?,
                uid: word(7)?,
                gid: word(8)?,
                size: word(9)?,
            };
            let mode = word(6)?;
            let oid = Oid::from_slice(
                data.get(start + 40..start + 60)
                    .ok_or_else(|| invalid("truncated"))?,
            )
            .ok_or_else(|| invalid("oid"))?;
            let flags = u16_at(data, start + 60)?;
            at = start + 62;
            let extended = if flags & 0x4000 != 0 {
                if version < 3 {
                    return Err(invalid("extended flags before version 3"));
                }
                at += 2;
                u16_at(data, start + 62)?
            } else {
                0
            };
            let path = if version == 4 {
                let strip = varint(data, &mut at)?;
                let nul = data[at..end]
                    .iter()
                    .position(|&b| b == 0)
                    .ok_or_else(|| invalid("name"))?;
                let keep = last
                    .len()
                    .checked_sub(strip)
                    .ok_or_else(|| invalid("name prefix"))?;
                let mut path = last[..keep].to_vec();
                path.extend_from_slice(&data[at..at + nul]);
                at += nul + 1;
                path
            } else {
                let nul = data[at..end]
                    .iter()
                    .position(|&b| b == 0)
                    .ok_or_else(|| invalid("name"))?;
                let path = data[at..at + nul].to_vec();
                // Padded with one to eight NULs to a multiple of eight.
                at = start + ((at - start + nul + 8) & !7);
                path
            };
            last.clone_from(&path);
            entries.push(Entry {
                path,
                mode,
                oid,
                stage: ((flags >> 12) & 3) as u8,
                flags,
                extended,
                stat,
            });
        }
        let mut tree = None;
        while at + 8 <= end {
            let signature = &data[at..at + 4];
            let size = u32_at(data, at + 4)? as usize;
            let body = data
                .get(at + 8..at + 8 + size)
                .ok_or_else(|| invalid("extension"))?;
            match signature {
                b"TREE" => {
                    let mut pos = 0;
                    tree = Some(parse_tree(body, &mut pos)?);
                }
                b"link" => return Err(invalid("a split index isn't read yet")),
                // Required extensions start in lower case; sparse is the one
                // understood by what's already done.
                b"sdir" => {}
                s if s[0].is_ascii_lowercase() => {
                    return Err(invalid("unknown required extension"));
                }
                _ => {}
            }
            at += 8 + size;
        }
        Ok(Index {
            entries,
            tree,
            written,
        })
    }

    /// Entries in `lo..hi` whose paths start with `prefix`, which is either
    /// empty or ends in a slash.
    pub fn under(&self, prefix: &[u8]) -> std::ops::Range<usize> {
        let lo = self.entries.partition_point(|e| e.path.as_slice() < prefix);
        let hi = lo + self.entries[lo..].partition_point(|e| e.path.starts_with(prefix));
        lo..hi
    }

    pub fn find(&self, path: &[u8]) -> Option<&Entry> {
        let at = self.entries.partition_point(|e| e.path.as_slice() < path);
        self.entries.get(at).filter(|e| e.path == path)
    }
}

fn parse_tree(data: &[u8], at: &mut usize) -> io::Result<CacheTree> {
    let nul = data[*at..]
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| invalid("cache-tree name"))?;
    let name = data[*at..*at + nul].to_vec();
    *at += nul + 1;
    let newline = data[*at..]
        .iter()
        .position(|&b| b == b'\n')
        .ok_or_else(|| invalid("cache-tree counts"))?;
    let counts =
        std::str::from_utf8(&data[*at..*at + newline]).map_err(|_| invalid("cache-tree counts"))?;
    *at += newline + 1;
    let mut parts = counts.split(' ');
    let entries: i64 = parts
        .next()
        .and_then(|n| n.parse().ok())
        .ok_or_else(|| invalid("cache-tree entries"))?;
    let subtrees: usize = parts
        .next()
        .and_then(|n| n.parse().ok())
        .ok_or_else(|| invalid("cache-tree subtrees"))?;
    let oid = if entries >= 0 {
        let oid = Oid::from_slice(
            data.get(*at..*at + 20)
                .ok_or_else(|| invalid("cache-tree oid"))?,
        );
        *at += 20;
        oid
    } else {
        None
    };
    let mut children = Vec::with_capacity(subtrees);
    for _ in 0..subtrees {
        children.push(parse_tree(data, at)?);
    }
    Ok(CacheTree {
        name,
        entries,
        oid,
        children,
    })
}
