//! A checkout's refs where git keeps them in a reftable, as a large monorepo's may be:
//! its `reftable/` folder's stack of tables, each a file of blocks, read
//! for its ref records alone (git's Documentation/reftable.txt). A
//! reftable checkout writes `ref: refs/heads/.invalid` in each HEAD file
//! and keeps the real HEAD here, so without this every tree of one reads
//! as at a branch called `.invalid`.
//!
//! Only what repos needs is read: each table's ref blocks, in order, until
//! the first block of another kind. Log blocks, compressed, and the index
//! and object blocks after them are never opened. Tables are read oldest
//! first, as `tables.list` names them, so a newer table's record of a ref,
//! or its deletion, stands.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// What a ref is: at a commit, or naming another ref.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ref {
    Commit(String),
    Symbolic(String),
}

/// Every ref a reftable folder's stack holds, or none where it can't be
/// read.
pub fn stack(folder: &Path) -> Option<HashMap<String, Ref>> {
    let list = fs::read_to_string(folder.join("tables.list")).ok()?;
    let mut refs = HashMap::new();
    for name in list.lines().map(str::trim).filter(|name| !name.is_empty()) {
        let table = fs::read(folder.join(name)).ok()?;
        records(&table, &mut refs)?;
    }
    Some(refs)
}

/// The bytes a file's header takes, by its version.
const HEADER_V1: usize = 24;
const HEADER_V2: usize = 28;

/// Reads one table's ref records into `refs`, a deletion taking its ref
/// out.
fn records(table: &[u8], refs: &mut HashMap<String, Ref>) -> Option<()> {
    if table.get(..4)? != b"REFT" {
        return None;
    }
    let (header, hash) = match table.get(4)? {
        1 => (HEADER_V1, 20),
        2 => match table.get(24..28)? {
            b"s256" => (HEADER_V2, 32),
            _ => (HEADER_V2, 20),
        },
        _ => return None,
    };
    let block_size = uint24(table.get(5..8)?);
    // The footer repeats the header and adds what's needed to find the
    // index; its first bytes say where blocks end.
    let footer = header + 44;
    let end = table.len().checked_sub(footer)?;
    let mut start = 0;
    let mut at = header;
    while at < end {
        match table[at] {
            b'r' => {}
            // Padding fills a block out to its size; the next is aligned.
            0 if block_size > 0 => {
                start = (at / block_size + 1) * block_size;
                at = start;
                continue;
            }
            _ => break,
        }
        let length = uint24(table.get(at + 1..at + 4)?);
        let block = table.get(start..start + length)?;
        let count = usize::from(u16::from_be_bytes([
            *block.get(length.checked_sub(2)?)?,
            *block.get(length - 1)?,
        ]));
        let records_end = length.checked_sub(2 + 3 * count)?;
        block_records(block, at - start + 4, records_end, hash, refs)?;
        start += length;
        at = start;
    }
    Some(())
}

/// The records of one ref block, from `at` to `end`, each sharing a prefix
/// of the name before it.
fn block_records(
    block: &[u8],
    mut at: usize,
    end: usize,
    hash: usize,
    refs: &mut HashMap<String, Ref>,
) -> Option<()> {
    let mut name: Vec<u8> = Vec::new();
    while at < end {
        let prefix = varint(block, &mut at)?;
        let both = varint(block, &mut at)?;
        let (suffix, kind) = (both >> 3, both & 7);
        name.truncate(prefix);
        name.extend_from_slice(block.get(at..at + suffix)?);
        at += suffix;
        let _update_index = varint(block, &mut at)?;
        let said = String::from_utf8_lossy(&name).into_owned();
        match kind {
            0 => {
                refs.remove(&said);
            }
            1 | 2 => {
                let commit = hex(block.get(at..at + hash)?);
                // A tag's record also holds what it peels to, which repos
                // never reads.
                at += hash * if kind == 2 { 2 } else { 1 };
                refs.insert(said, Ref::Commit(commit));
            }
            3 => {
                let length = varint(block, &mut at)?;
                let target = String::from_utf8_lossy(block.get(at..at + length)?).into_owned();
                at += length;
                refs.insert(said, Ref::Symbolic(target));
            }
            _ => return None,
        }
    }
    Some(())
}

fn uint24(bytes: &[u8]) -> usize {
    bytes
        .iter()
        .fold(0, |value, byte| (value << 8) | usize::from(*byte))
}

/// A number as git writes one in a reftable: seven bits a byte, the high
/// bit saying another follows, each continuation adding one before its
/// shift so no number has two spellings.
fn varint(bytes: &[u8], at: &mut usize) -> Option<usize> {
    let mut byte = *bytes.get(*at)?;
    *at += 1;
    let mut value = usize::from(byte & 0x7f);
    while byte & 0x80 != 0 {
        byte = *bytes.get(*at)?;
        *at += 1;
        value = (value.checked_add(1)?.checked_shl(7)?) | usize::from(byte & 0x7f);
    }
    Some(value)
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

#[cfg(test)]
pub(crate) mod tests;
