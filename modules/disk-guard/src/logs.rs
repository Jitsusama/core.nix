//! Logs that grow without bound, each kept to its last part.

use crate::anchor::Dir;
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// A log past this is trimmed.
pub const LIMIT: u64 = 64 << 20;

/// What a trimmed log keeps of its end.
pub const KEEP: u64 = 16 << 20;

/// How deep logs are looked for, and how many entries are read.
const DEPTH: usize = 8;
const BUDGET: usize = 50_000;

/// How much is copied at a time, so a trim never holds the whole tail.
const CHUNK: usize = 1 << 20;

/// How many times a trim catches up with a writer before letting it go.
const CATCH_UPS: usize = 16;

/// Every `.log` file beneath `root` past `limit`, never through a symlink
/// and never onto another filesystem.
pub fn oversized(root: &Path, limit: u64) -> Vec<PathBuf> {
    let Ok(top) = fs::symlink_metadata(root) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    let mut budget = BUDGET;
    let mut pending = vec![(root.to_path_buf(), 0)];
    while let Some((dir, depth)) = pending.pop() {
        let Ok(listing) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in listing.filter_map(Result::ok) {
            if budget == 0 {
                found.sort();
                return found;
            }
            budget -= 1;
            let path = entry.path();
            let Ok(meta) = fs::symlink_metadata(&path) else {
                continue;
            };
            if meta.dev() != top.dev() {
                continue;
            }
            if meta.is_dir() && depth + 1 < DEPTH {
                pending.push((path, depth + 1));
            } else if meta.is_file()
                && meta.len() > limit
                && path.extension().is_some_and(|extension| extension == "log")
            {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Keeps the last `keep` bytes of a log from its first whole line, written
/// over its start in place, so a writer holding it open carries on
/// appending to it. Whatever is appended while the tail is copied is
/// carried over too; what lands between the last look and the truncation
/// is lost, a window of a system call. Returns the bytes dropped from the
/// front.
pub fn trim(root: &Dir, relative: &Path, keep: u64) -> io::Result<u64> {
    let (Some(parent), Some(name)) = (relative.parent(), relative.file_name()) else {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "no file named"));
    };
    let file = root.child(parent)?.file(name, true)?;
    let mut size = rustix::fs::fstat(&file)?.st_size as u64;
    if size <= keep {
        return Ok(0);
    }
    // The tail starts after the last line break before it, read from the
    // byte before, so a tail that starts a line keeps that line.
    let mut start = size - keep;
    let mut head = vec![0; (keep as usize).min(CHUNK)];
    let read = rustix::io::pread(&file, &mut head, start - 1)?;
    if let Some(newline) = head[..read].iter().position(|&byte| byte == b'\n') {
        start += newline as u64;
    }
    let mut written = 0;
    let mut from = start;
    let mut buffer = vec![0; CHUNK];
    for _ in 0..CATCH_UPS {
        while from < size {
            let want = ((size - from) as usize).min(CHUNK);
            let read = rustix::io::pread(&file, &mut buffer[..want], from)?;
            if read == 0 {
                break;
            }
            write_all_at(&file, &buffer[..read], written)?;
            from += read as u64;
            written += read as u64;
        }
        let now = rustix::fs::fstat(&file)?.st_size as u64;
        if now <= size {
            break;
        }
        size = now;
    }
    rustix::fs::ftruncate(&file, written)?;
    Ok(start)
}

fn write_all_at(file: &std::os::fd::OwnedFd, mut bytes: &[u8], mut at: u64) -> io::Result<()> {
    while !bytes.is_empty() {
        let wrote = rustix::io::pwrite(file, bytes, at)?;
        bytes = &bytes[wrote..];
        at += wrote as u64;
    }
    Ok(())
}
