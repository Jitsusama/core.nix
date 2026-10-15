//! Reading a reftable stack, the refs format World uses: `tables.list` names
//! the tables oldest first, each a run of blocks of prefix-compressed
//! records. The engine reads every ref record once per change to the stack
//! and keeps the merged answer; the logs are read only to count stashes.

use crate::hash::Oid;
use crate::stamp::{Stamp, Stamps};
use std::collections::HashMap;
use std::io;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Oid(Oid),
    Symbolic(String),
}

pub struct Stack {
    pub refs: HashMap<String, Value>,
    /// How many entries the stash's log holds.
    pub stashes: usize,
    pub stamps: Stamps,
}

fn invalid(what: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("reftable: {what}"))
}

/// reftable's variable-length integer, which is git's offset encoding.
fn varint(data: &[u8], at: &mut usize) -> io::Result<u64> {
    let mut byte = *data.get(*at).ok_or_else(|| invalid("varint"))?;
    *at += 1;
    let mut value = u64::from(byte & 0x7f);
    while byte & 0x80 != 0 {
        byte = *data.get(*at).ok_or_else(|| invalid("varint"))?;
        *at += 1;
        value = ((value + 1) << 7) | u64::from(byte & 0x7f);
    }
    Ok(value)
}

fn u24(data: &[u8], at: usize) -> io::Result<usize> {
    let bytes = data.get(at..at + 3).ok_or_else(|| invalid("u24"))?;
    Ok(usize::from(bytes[0]) << 16 | usize::from(bytes[1]) << 8 | usize::from(bytes[2]))
}

struct Header {
    size: usize,
    block_size: usize,
}

fn header(data: &[u8]) -> io::Result<Header> {
    if data.get(..4) != Some(b"REFT") {
        return Err(invalid("magic"));
    }
    let size = match data.get(4) {
        Some(1) => 24,
        Some(2) => {
            if data.get(24..28) != Some(b"sha1") {
                return Err(invalid("only SHA-1 tables are read"));
            }
            28
        }
        _ => return Err(invalid("version")),
    };
    Ok(Header {
        size,
        block_size: u24(data, 5)?,
    })
}

/// The blocks of a table, as (where the block's own header is, where it
/// ends, counted as reftable counts them from the block's start).
fn next_block(data: &[u8], header: &Header, end: usize) -> usize {
    if header.block_size > 0 && data.get(end) == Some(&0) {
        end.div_ceil(header.block_size) * header.block_size
    } else {
        end
    }
}

/// What reads one record: its key, its value type, the block from where its
/// value starts, and how far that value runs, to be moved past it.
type Record<'a> = dyn FnMut(&[u8], u8, &[u8], &mut usize) -> io::Result<()> + 'a;

/// Each record in one block: its key and the bytes after it, handed to `each`.
fn records(block: &[u8], start: usize, each: &mut Record) -> io::Result<()> {
    let count = usize::from(u16::from_be_bytes(
        block
            .get(block.len().saturating_sub(2)..)
            .and_then(|b| b.try_into().ok())
            .ok_or_else(|| invalid("restarts"))?,
    ));
    let end = block
        .len()
        .checked_sub(2 + 3 * count)
        .ok_or_else(|| invalid("restart table"))?;
    let mut at = start;
    let mut key: Vec<u8> = Vec::new();
    while at < end {
        let prefix = usize::try_from(varint(block, &mut at)?).map_err(|_| invalid("prefix"))?;
        let suffix_and_type = varint(block, &mut at)?;
        let kind = (suffix_and_type & 7) as u8;
        let suffix = usize::try_from(suffix_and_type >> 3).map_err(|_| invalid("suffix"))?;
        if prefix > key.len() {
            return Err(invalid("prefix past the last key"));
        }
        key.truncate(prefix);
        key.extend_from_slice(block.get(at..at + suffix).ok_or_else(|| invalid("key"))?);
        at += suffix;
        each(&key, kind, block, &mut at)?;
    }
    Ok(())
}

/// A log entry's key, and whether it's there or deleted.
type LogEntry = (Vec<u8>, bool);

/// Every ref one table records and every stash log entry, deletions
/// included.
/// A table's refs, by name, `None` where it deletes one.
type Refs = Vec<(String, Option<Value>)>;

fn read_table(data: &[u8]) -> io::Result<(Refs, Vec<LogEntry>)> {
    let header = header(data)?;
    let footer = if header.size == 24 { 68 } else { 72 };
    let body_end = data
        .len()
        .checked_sub(footer)
        .ok_or_else(|| invalid("footer"))?;
    let mut refs = Vec::new();
    let mut block_start = 0;
    let mut first = true;
    loop {
        let at = if first { header.size } else { block_start };
        if at >= body_end || data[at] != b'r' {
            break;
        }
        let len = u24(data, at + 1)?;
        let end = block_start + len;
        let block = data
            .get(block_start..end)
            .ok_or_else(|| invalid("ref block"))?;
        records(block, at + 4 - block_start, &mut |key, kind, block, at| {
            let _update = varint(block, at)?;
            let name = String::from_utf8_lossy(key).into_owned();
            let oid = |at: &mut usize| -> io::Result<Oid> {
                let oid = Oid::from_slice(block.get(*at..*at + 20).ok_or_else(|| invalid("oid"))?)
                    .ok_or_else(|| invalid("oid"))?;
                *at += 20;
                Ok(oid)
            };
            let value = match kind {
                0 => None,
                1 => Some(Value::Oid(oid(at)?)),
                2 => {
                    let value = oid(at)?;
                    let _peeled = oid(at)?;
                    Some(Value::Oid(value))
                }
                3 => {
                    let len = usize::try_from(varint(block, at)?).map_err(|_| invalid("symref"))?;
                    let target = block.get(*at..*at + len).ok_or_else(|| invalid("symref"))?;
                    *at += len;
                    Some(Value::Symbolic(
                        String::from_utf8_lossy(target).into_owned(),
                    ))
                }
                _ => return Err(invalid("ref value type")),
            };
            refs.push((name, value));
            Ok(())
        })?;
        block_start = next_block(data, &header, end);
        first = false;
    }
    let log_at = usize::try_from(u64::from_be_bytes(
        data[body_end + header.size + 24..body_end + header.size + 32]
            .try_into()
            .expect("8 bytes"),
    ))
    .map_err(|_| invalid("log position"))?;
    let stashes = if log_at > 0 && log_at < body_end {
        stash_log(data, log_at, body_end)?
    } else {
        Vec::new()
    };
    Ok((refs, stashes))
}

/// The entries this table records in `refs/stash`'s log.
fn stash_log(data: &[u8], mut at: usize, body_end: usize) -> io::Result<Vec<LogEntry>> {
    let mut entries = Vec::new();
    while at < body_end && data[at] == b'g' {
        let len = u24(data, at + 1)?;
        let mut state =
            miniz_oxide::inflate::stream::InflateState::new_boxed(miniz_oxide::DataFormat::Zlib);
        let mut out = vec![0; len.saturating_sub(4)];
        let mut input = &data[at + 4..body_end];
        let mut written = 0;
        let mut used = 0;
        while written < out.len() {
            let result = miniz_oxide::inflate::stream::inflate(
                &mut state,
                input,
                &mut out[written..],
                miniz_oxide::MZFlush::Finish,
            );
            written += result.bytes_written;
            used += result.bytes_consumed;
            input = &input[result.bytes_consumed..];
            if result.status.is_err() || (result.bytes_written == 0 && result.bytes_consumed == 0) {
                break;
            }
        }
        let mut block = vec![b'g', 0, 0, 0];
        block.extend_from_slice(&out[..written]);
        records(&block, 4, &mut |key, kind, block, at| {
            let name = key.split(|&b| b == 0).next().unwrap_or_default();
            if name == b"refs/stash" {
                entries.push((key.to_vec(), kind == 1));
            }
            if kind == 1 {
                *at += 40;
                for _ in 0..2 {
                    let len =
                        usize::try_from(varint(block, at)?).map_err(|_| invalid("log name"))?;
                    *at += len;
                }
                let _time = varint(block, at)?;
                *at += 2;
                let len =
                    usize::try_from(varint(block, at)?).map_err(|_| invalid("log message"))?;
                *at += len;
            }
            Ok(())
        })?;
        // The compressed stream's end is where the next block begins.
        at += 4 + used;
    }
    Ok(entries)
}

impl Stack {
    pub fn read(dir: &Path) -> io::Result<Stack> {
        let list = dir.join("tables.list");
        let stamps = vec![(list.clone(), Stamp::of(&list))];
        let names = std::fs::read_to_string(&list)?;
        let mut refs: HashMap<String, Option<Value>> = HashMap::new();
        let mut log: HashMap<Vec<u8>, bool> = HashMap::new();
        // Newest last in the list; a newer table's record wins, a deletion
        // included.
        for name in names.lines().rev().filter(|line| !line.is_empty()) {
            let data = std::fs::read(dir.join(name))?;
            let (records, entries) = read_table(&data)?;
            for (name, value) in records {
                refs.entry(name).or_insert(value);
            }
            for (key, present) in entries {
                log.entry(key).or_insert(present);
            }
        }
        let refs = refs
            .into_iter()
            .filter_map(|(name, value)| Some((name, value?)))
            .collect();
        let stashes = log.values().filter(|&&present| present).count();
        Ok(Stack {
            refs,
            stashes,
            stamps,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varints_read_as_reftable_writes_them() {
        let mut at = 0;
        assert_eq!(varint(&[0x7f], &mut at).unwrap(), 127);
        at = 0;
        assert_eq!(varint(&[0x80, 0x00], &mut at).unwrap(), 128);
        at = 0;
        assert_eq!(varint(&[0xff, 0x7f], &mut at).unwrap(), 16_511);
    }
}
