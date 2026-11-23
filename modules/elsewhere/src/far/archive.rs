//! The job's files as tar archives, so a tree of thousands of files
//! crosses as a few: a share's cost grows with each file it carries, and
//! a repository's copy sent file by file took minutes to cross a link
//! where one archive takes seconds.
//!
//! Each archive is a POSIX ustar one, which every far machine's `tar`
//! reads: GNU tar on Linux and bsdtar on a Mac. A path too long for
//! ustar's two name fields goes in a pax header before it. Only regular
//! files go in, each at its own mode, so the far side's copy is the
//! near tree's tracked files and nothing else. Each archive stays within
//! what a share carries in one file, and the job's files are split over
//! as many as that takes.

use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::UNIX_EPOCH;

/// A tar archive's block, which every header and every file's data is
/// padded to.
const BLOCK: usize = 512;

/// What closes an archive: two empty blocks.
const CLOSE: usize = 2 * BLOCK;

/// What closes an archive, in bytes, as a size is counted.
pub const CLOSED: u64 = CLOSE as u64;

/// What a file at `path` of `size` bytes takes in an archive, just as
/// `write` puts it in: a pax header and its record where ustar can't hold
/// the path, then its own header and its data, each padded.
pub fn taken(path: &str, size: u64) -> u64 {
    let block = BLOCK as u64;
    let pax = if split(path).is_some() {
        0
    } else {
        let record = record("path", path).len() as u64;
        block + record.div_ceil(block) * block
    };
    pax + block + size.div_ceil(block) * block
}

/// The most the archives of entries taking `taken` bytes in all come to,
/// each at most `most` bytes, with their closes. An archive is closed
/// only when the next entry won't fit in it, so any two archives in a
/// row hold more than one's room, and there are never more than twice
/// as many as the entries fill, and one.
pub fn archived(taken: u64, most: u64) -> u64 {
    let room = most.saturating_sub(CLOSED).max(1);
    taken + CLOSED * (2 * taken.div_ceil(room) + 1)
}

/// The longest name, and the longest prefix before it, a ustar header
/// holds.
const NAME: usize = 100;
const PREFIX: usize = 155;

/// One archive's name and size, as it crosses.
#[derive(Debug, PartialEq, Eq)]
pub struct Part {
    pub name: String,
    pub size: u64,
}

/// Writes `paths`, files of the tree at `root`, as archives in `into`,
/// each at most `most` bytes, named `0.tar`, `1.tar` and on, and answers
/// them in order. Each file is read once, whole, so it goes in at the
/// size it was read at even when it changed after it was gathered. A
/// file whose own entry is past `most` is refused, as no share carries it.
pub fn write<'a>(
    root: &Path,
    paths: impl IntoIterator<Item = &'a str>,
    into: &Path,
    most: u64,
) -> io::Result<Vec<Part>> {
    let mut parts = Vec::new();
    let mut open: Option<(fs::File, u64)> = None;
    let close = CLOSED;
    for path in paths {
        let (data, meta) = read(&root.join(path))?;
        let entry = entry(path, &data, &meta);
        let length = entry.len() as u64;
        if length + close > most {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{path} is past the {most} bytes one shared file may be"),
            ));
        }
        if open
            .as_ref()
            .is_some_and(|(_, size)| size + length + close > most)
        {
            parts.push(closed(open.take(), parts.len())?);
        }
        let (file, size) = match &mut open {
            Some(open) => open,
            slot @ None => slot.insert((fs::File::create(into.join(name(parts.len())))?, 0)),
        };
        file.write_all(&entry)?;
        *size += length;
    }
    if open.is_some() {
        parts.push(closed(open, parts.len())?);
    }
    Ok(parts)
}

/// The plain file at `path`, read whole through one opening, with what
/// that opening says of it. A link or a pipe put where the file was
/// gathered from is refused, neither followed nor waited on, so nothing
/// outside the tree goes in and nothing holds the job.
fn read(path: &Path) -> io::Result<(Vec<u8>, fs::Metadata)> {
    let mut file = super::landing::opened_plain(path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("{} can't go in: {error}", path.display()),
        )
    })?;
    let meta = file.metadata()?;
    let mut data = Vec::new();
    io::Read::read_to_end(&mut file, &mut data)?;
    Ok((data, meta))
}

/// The archive numbered `number`'s name.
fn name(number: usize) -> String {
    format!("{number}.tar")
}

/// Closes the archive `open`, numbered `number`, and answers it.
fn closed(open: Option<(fs::File, u64)>, number: usize) -> io::Result<Part> {
    let Some((mut file, size)) = open else {
        return Err(io::Error::other("no archive was open to close"));
    };
    file.write_all(&[0; CLOSE])?;
    file.sync_all()?;
    Ok(Part {
        name: name(number),
        size: size + CLOSE as u64,
    })
}

/// The file at `path`, holding `data`, as an archive entry: a pax header
/// first where ustar can't hold its name, then its own header and data.
fn entry(path: &str, data: &[u8], meta: &fs::Metadata) -> Vec<u8> {
    let mode = meta.permissions().mode() & 0o777;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |since| since.as_secs());
    let mut entry = Vec::new();
    let (prefix, name) = split(path).unwrap_or_else(|| {
        let record = record("path", path);
        entry.extend(header("", "PaxHeader", 0o644, record.len() as u64, 0, b'x'));
        entry.extend(padded(record.as_bytes()));
        // A name the pax header stands in for, kept short for any reader
        // that skips the header.
        ("", truncated(path))
    });
    entry.extend(header(prefix, name, mode, data.len() as u64, mtime, b'0'));
    entry.extend(padded(data));
    entry
}

/// `path` split into ustar's prefix and name at a folder, or none where
/// neither fits.
fn split(path: &str) -> Option<(&str, &str)> {
    if path.len() <= NAME {
        return Some(("", path));
    }
    path.match_indices('/')
        .map(|(at, _)| (&path[..at], &path[at + 1..]))
        .find(|(prefix, name)| prefix.len() <= PREFIX && !name.is_empty() && name.len() <= NAME)
}

/// The first of `path`'s bytes that fit in ustar's name, cut at a
/// character.
fn truncated(path: &str) -> &str {
    let mut end = NAME.min(path.len());
    while !path.is_char_boundary(end) {
        end -= 1;
    }
    &path[..end]
}

/// A pax record, `LENGTH KEY=VALUE\n`, whose length counts itself.
fn record(key: &str, value: &str) -> String {
    let rest = key.len() + value.len() + 3;
    let mut length = rest + 1;
    while length != rest + length.to_string().len() {
        length = rest + length.to_string().len();
    }
    format!("{length} {key}={value}\n")
}

/// A ustar header block.
fn header(prefix: &str, name: &str, mode: u32, size: u64, mtime: u64, kind: u8) -> [u8; 512] {
    let mut block = [0u8; 512];
    block[..name.len()].copy_from_slice(name.as_bytes());
    octal(&mut block[100..108], u64::from(mode));
    octal(&mut block[108..116], 0);
    octal(&mut block[116..124], 0);
    octal(&mut block[124..136], size);
    octal(&mut block[136..148], mtime);
    block[156] = kind;
    block[257..263].copy_from_slice(b"ustar\0");
    block[263..265].copy_from_slice(b"00");
    block[345..345 + prefix.len()].copy_from_slice(prefix.as_bytes());
    // The checksum is taken with its own field as spaces.
    block[148..156].fill(b' ');
    let sum: u64 = block.iter().map(|&byte| u64::from(byte)).sum();
    octal(&mut block[148..155], sum);
    block
}

/// `value` in octal, zero-filled to the field's width less its closing
/// NUL; a value past the field's width keeps its lowest digits, which
/// only a time past the year 2242 or a file past 8 GiB would meet.
fn octal(field: &mut [u8], value: u64) {
    let width = field.len() - 1;
    let digits = format!("{value:0width$o}");
    let digits = &digits.as_bytes()[digits.len() - width..];
    field[..width].copy_from_slice(digits);
    field[width] = 0;
}

/// `data` padded with zeros to a whole number of blocks.
fn padded(data: &[u8]) -> Vec<u8> {
    let mut padded = data.to_vec();
    padded.resize(data.len().div_ceil(BLOCK) * BLOCK, 0);
    padded
}

#[cfg(test)]
mod tests;
