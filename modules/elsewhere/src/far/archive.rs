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
use std::io::{self, BufWriter, Write};
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

/// What an open archive gathers before it writes, so a tree of small
/// files takes a few writes, not several for each.
const GATHERS: usize = 1 << 16;

/// An archive being written, and the bytes in it so far.
type Open = (BufWriter<fs::File>, u64);

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
/// them in order. Each is written under a hidden name and takes its own
/// once closed and on the disk, so a share, which carries no hidden file,
/// never carries one half written. Each file is read once, whole, so it
/// goes in at the size it was read at even when it changed after it was
/// gathered, and is written from that one read, never copied again, so
/// it costs its own size in memory as it goes in. It isn't copied across
/// a piece at a time instead, since its header, written first, gives its
/// size: a file rewritten in place as a session works would then end
/// short of it or run past it, and its job be refused. A file whose own
/// entry is past `most` is refused, as no share carries it, and so is
/// one that would take the archives past `all` bytes in all, before it
/// is written: a tree that grew after it was gathered leaves this job's
/// archives no larger than `all` all the same. A refusal leaves the
/// archives written so far unlisted, and the one still open under its
/// hidden name, for the caller to clear. Each is made new, so nothing
/// standing at its name, a link included, is written through.
pub fn write<'a>(
    root: &Path,
    paths: impl IntoIterator<Item = &'a str>,
    into: &Path,
    most: u64,
    all: u64,
) -> io::Result<Vec<Part>> {
    let mut parts = Vec::new();
    let mut open: Option<Open> = None;
    let mut done = 0;
    let close = CLOSED;
    for path in paths {
        let (data, meta) = read(root, path, most)?;
        let length = taken(path, data.len() as u64);
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
            let part = closed(open.take(), into, parts.len())?;
            done += part.size;
            parts.push(part);
        }
        let open_size = open.as_ref().map_or(0, |(_, size)| *size);
        if done + open_size + length + close > all {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{path} would take the archives past the {all} bytes a share carries"),
            ));
        }
        let (file, size) = match &mut open {
            Some(open) => open,
            slot @ None => {
                let made = made_new(&into.join(writing(parts.len())))?;
                slot.insert((BufWriter::with_capacity(GATHERS, made), 0))
            }
        };
        put(file, path, &data, &meta)?;
        *size += length;
    }
    if open.is_some() {
        parts.push(closed(open, into, parts.len())?);
    }
    Ok(parts)
}

/// The plain file at `name` in the tree at `root`, read whole through one
/// opening, with what that opening says of it. A link or a pipe where the
/// file was gathered from is refused before any of it is read: one there
/// at the look is never opened, a link swapped in after it is never
/// followed, and a pipe swapped in is opened without waiting and refused
/// as not the file looked at. A folder on its way that is a link once it
/// is open is refused too, as the file may lie outside the tree. So
/// nothing outside the tree goes in, unless a link is put in a folder's
/// place and taken out again while the file is opened, which something
/// swapping it over and over could win, as only opening each folder from
/// the one before would close it and std can't; and nothing holds the
/// job. A file past `most` bytes, as it is opened or
/// as it is read, is refused without being read whole, as no share
/// carries it.
fn read(root: &Path, name: &str, most: u64) -> io::Result<(Vec<u8>, fs::Metadata)> {
    let path = root.join(name);
    let refused =
        |error: &dyn std::fmt::Display| format!("{} can't go in: {error}", path.display());
    let mut file = super::landing::opened_plain(&path)
        .map_err(|error| io::Error::new(error.kind(), refused(&error)))?;
    if !super::names::Folders::default().own(root, name) {
        return Err(io::Error::other(refused(&"a folder on its way is a link")));
    }
    let meta = file.metadata()?;
    let past = |how: &str| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{} {how} past the {most} bytes one shared file may be",
                path.display()
            ),
        )
    };
    if meta.len() > most {
        return Err(past(&format!("is {} bytes,", meta.len())));
    }
    let Some(data) = at_most(&mut file, most)? else {
        return Err(past("grew as it was read,"));
    };
    Ok((data, meta))
}

/// All of `from` when it holds at most `most` bytes, and nothing when it
/// holds more, found by reading no further than the byte past them.
fn at_most(from: impl io::Read, most: u64) -> io::Result<Option<Vec<u8>>> {
    let mut data = Vec::new();
    io::Read::read_to_end(&mut from.take(most.saturating_add(1)), &mut data)?;
    Ok((data.len() as u64 <= most).then_some(data))
}

/// The archive numbered `number`'s name.
fn name(number: usize) -> String {
    format!("{number}.tar")
}

/// A file at `path` made new, whatever stood there taken away first and
/// never followed, as a link the job left at the name would be.
fn made_new(path: &Path) -> io::Result<fs::File> {
    match fs::remove_file(path) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
        _ => {}
    }
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
}

/// The archive numbered `number`'s name while it is written: hidden, so
/// no share carries it.
fn writing(number: usize) -> String {
    format!(".{number}.tar")
}

/// Closes the archive `open`, numbered `number` in `into`, gives it its
/// own name once it is on the disk, and answers it.
fn closed(open: Option<Open>, into: &Path, number: usize) -> io::Result<Part> {
    let Some((mut file, size)) = open else {
        return Err(io::Error::other("no archive was open to close"));
    };
    file.write_all(&[0; CLOSE])?;
    let file = file.into_inner().map_err(io::IntoInnerError::into_error)?;
    file.sync_all()?;
    fs::rename(into.join(writing(number)), into.join(name(number)))?;
    Ok(Part {
        name: name(number),
        size: size + CLOSE as u64,
    })
}

/// Puts the file at `path`, holding `data`, in the archive `into` as its
/// entry, of just the bytes `taken` counts: a pax header first where
/// ustar can't hold its name, then its own header and data.
fn put(into: &mut impl Write, path: &str, data: &[u8], meta: &fs::Metadata) -> io::Result<()> {
    let mode = meta.permissions().mode() & 0o777;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |since| since.as_secs());
    let (prefix, name) = if let Some(split) = split(path) {
        split
    } else {
        let record = record("path", path);
        into.write_all(&header(
            "",
            "PaxHeader",
            0o644,
            record.len() as u64,
            0,
            b'x',
        ))?;
        padded(into, record.as_bytes())?;
        // A name the pax header stands in for, kept short for any reader
        // that skips the header.
        ("", truncated(path))
    };
    into.write_all(&header(prefix, name, mode, data.len() as u64, mtime, b'0'))?;
    padded(into, data)
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

/// Writes `data` to `into`, padded with zeros to a whole number of
/// blocks.
fn padded(into: &mut impl Write, data: &[u8]) -> io::Result<()> {
    into.write_all(data)?;
    let short = data.len().next_multiple_of(BLOCK) - data.len();
    into.write_all(&[0; BLOCK][..short])
}

#[cfg(test)]
mod tests;
