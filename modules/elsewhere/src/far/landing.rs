//! Landing what a far job made in the person's folder: each file under a
//! name of its own first, locked while it lands, then moved to its path,
//! so nothing already there is written through and nothing half made is
//! ever at the path. A run killed as it landed leaves its name behind,
//! which the next landing in that folder sweeps.

use super::MARK;
use std::collections::HashSet;
use std::ffi::c_int;
use std::fs;
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

/// The mode bits a file brought back may land with: its own read, write
/// and run bits, but never setuid, setgid, sticky or a write for anyone,
/// and then only those the person's umask lets a new file have.
pub(super) const LANDS_WITH: u32 = 0o775;

/// How old a landing name is before a sweep may take it: far past the
/// moment between its run making it and locking it.
pub(super) const SWEEPS_AFTER: Duration = Duration::from_secs(60);

/// `O_NONBLOCK`, so opening a pipe that took a file's place returns at
/// once rather than waiting for a writer. Its value is the platform's, so
/// a platform this doesn't name doesn't build rather than guessing.
#[cfg(all(
    target_os = "linux",
    any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64"
    )
))]
const OPEN_NONBLOCKING: i32 = 0o4000;
#[cfg(target_os = "macos")]
const OPEN_NONBLOCKING: i32 = 0x4;

/// The number the next landing name takes, so no two in this process
/// share one.
static NEXT_LANDING: AtomicU32 = AtomicU32::new(0);

/// How many landing names a landing tries before it gives up: more than
/// any run killed in the last minute under the same process id leaves.
const LANDING_TRIES: usize = 64;

/// How much of a file brought back is read before it is written.
const COPIES_AT_ONCE: usize = 64 << 10;

/// Puts `from` at `into` under a name of its own first and then moves it
/// there, so whatever is already at `into`, a hard link to a file
/// elsewhere or a symbolic link, is replaced rather than written through.
/// `source`, opened as listed, lands as every one of its `size` bytes. A
/// folder named in `swept` was swept already by this bring-back.
pub(super) fn land(
    mut source: fs::File,
    size: u64,
    into: &Path,
    swept: &mut HashSet<PathBuf>,
) -> Result<(), Fault> {
    let parent = into.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(Fault::Folder)?;
    if swept.insert(parent.to_path_buf()) {
        sweep(parent);
    }
    let opened = source.metadata().map_err(Fault::Share)?;
    let bits = landing_bits(opened.permissions().mode());
    let (landing, mut to) = landing_name(parent, bits).map_err(Fault::Folder)?;
    let landed = held(&to)
        .map_err(Fault::Folder)
        .and_then(|()| copied(&mut source, size, &mut to))
        .and_then(|()| synced(&to).map_err(Fault::Folder))
        .and_then(|()| fs::rename(&landing, into).map_err(Fault::Folder));
    landed.inspect_err(|_| {
        // What's left would be in the person's folder; it's spent anyway.
        let _ = fs::remove_file(&landing);
    })
}

/// What kept a file brought back from landing: the share it came home in,
/// which another try may not meet, or the folder it lands in.
#[derive(Debug)]
pub(super) enum Fault {
    /// It changed, or couldn't be read, after it crossed whole.
    Share(io::Error),
    /// The folder couldn't take it.
    Folder(io::Error),
}

/// Copies `size` bytes of `source` to `to`, so a read that fails, or ends
/// short of them, is the share's and a write that fails the folder's.
fn copied(source: &mut fs::File, size: u64, to: &mut fs::File) -> Result<(), Fault> {
    let mut buffer = vec![0; COPIES_AT_ONCE];
    let mut left = size;
    while left > 0 {
        let most = usize::try_from(left).map_or(buffer.len(), |left| left.min(buffer.len()));
        let read = match source.read(&mut buffer[..most]) {
            Ok(0) => {
                return Err(Fault::Share(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    format!("only {} of its {size} bytes could be read", size - left),
                )));
            }
            Ok(read) => read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(Fault::Share(error)),
        };
        to.write_all(&buffer[..read]).map_err(Fault::Folder)?;
        left -= u64::try_from(read).unwrap_or(left);
    }
    Ok(())
}

/// Asks the folder to keep what was written to `to`, since a network
/// folder may say it is full, or past a quota, only then, and a close that
/// is dropped never hears it. `fsync` itself rather than std's sync, which
/// on Apple's systems is `F_FULLFSYNC`, a flush of the drive's own cache
/// besides: one a file makes a large bring-back crawl, and some network
/// folders refuse it outright, which would fail every landing.
fn synced(to: &fs::File) -> io::Result<()> {
    unsafe extern "C" {
        fn fsync(fd: c_int) -> c_int;
    }
    loop {
        // Safe: the descriptor is `to`'s own, open for as long as it's lent.
        if unsafe { fsync(to.as_raw_fd()) } == 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
}

/// The bits a file of `mode` lands with, before the umask: never setuid,
/// setgid, sticky or writable by anyone.
fn landing_bits(mode: u32) -> u32 {
    mode & LANDS_WITH
}

/// Makes a landing name in `parent` with `bits`, passing over any a run
/// killed under the same process id left too young to sweep.
fn landing_name(parent: &Path, bits: u32) -> io::Result<(PathBuf, fs::File)> {
    for _ in 0..LANDING_TRIES {
        // Made new, so nothing already at the name, a link perhaps, is
        // opened, and short, so any name that fits its folder lands.
        let landing = parent.join(format!(
            "{MARK}-{}-{}",
            std::process::id(),
            NEXT_LANDING.fetch_add(1, Ordering::Relaxed)
        ));
        // Made with its bits, so the umask takes what it would from any
        // new file. They always let its owner read it, as it was read to
        // land, so a sweep can open it if its run is killed.
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(bits)
            .open(&landing)
        {
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            opened => return opened.map(|file| (landing, file)),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!("{LANDING_TRIES} landing names in a row were taken"),
    ))
}

/// Locks a landing name while it lands, so a sweep knows it is no
/// leftover. Where its folder's filesystem takes no locks, it lands
/// unlocked, as a sweep removes only what it can lock.
fn held(file: &fs::File) -> io::Result<()> {
    match file.try_lock() {
        Ok(()) | Err(fs::TryLockError::Error(_)) => Ok(()),
        Err(held) => Err(io::Error::other(held)),
    }
}

/// Removes from `folder` each landing name a run killed as it landed left:
/// one no live run holds locked, as the kernel lets a dead one's lock go,
/// and older than [`SWEEPS_AFTER`], so never one a live run has made and
/// not yet locked.
fn sweep(folder: &Path) {
    let Ok(names) = fs::read_dir(folder) else {
        // Nothing to sweep in a folder that can't be read; the landing
        // then says why it can't be written either.
        return;
    };
    for name in names.flatten() {
        let path = name.path();
        let landing = name
            .file_name()
            .to_str()
            .and_then(|name| name.strip_prefix(&format!("{MARK}-")))
            .and_then(|rest| rest.split_once('-'))
            .is_some_and(|(pid, n)| {
                [pid, n]
                    .iter()
                    .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
            });
        let Some(file) = landing.then(|| opened_plain(&path).ok()).flatten() else {
            continue;
        };
        let old = file
            .metadata()
            .and_then(|meta| meta.modified())
            .is_ok_and(|at| at.elapsed().is_ok_and(|age| age >= SWEEPS_AFTER));
        // A shared lock, which a file opened only to read can take, as an
        // NFS client's emulated lock asks; a live run's own lock refuses it.
        if !old || file.try_lock_shared().is_err() {
            continue;
        }
        // Still the file locked at the name, so one made there since by a
        // run whose pid came round again is never the one removed.
        let same = file.metadata().is_ok_and(|locked| {
            fs::symlink_metadata(&path)
                .is_ok_and(|named| named.dev() == locked.dev() && named.ino() == locked.ino())
        });
        if same {
            let _ = fs::remove_file(&path);
        }
    }
}

/// The file brought back at `from`, opened as the plain file of the
/// `size` its list says: the size is the open file's own, so another put
/// at its name since the list was checked is never taken for it.
pub(super) fn opened_as_listed(from: &Path, size: u64) -> io::Result<fs::File> {
    let source = opened_plain(from)?;
    let opened = source.metadata()?.len();
    if opened != size {
        return Err(io::Error::other(format!(
            "it is {opened} bytes, not the {size} listed"
        )));
    }
    Ok(source)
}

/// Opens `path` only when it is a plain file, and the same file once open,
/// so a link put there after the look is never followed.
pub(super) fn opened_plain(path: &Path) -> io::Result<fs::File> {
    let looked = fs::symlink_metadata(path)?;
    if !looked.is_file() {
        return Err(io::Error::other("it isn't a plain file"));
    }
    // Without blocking, so a pipe put there after the look can't hold the
    // run; it is then refused as not the file looked at.
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(OPEN_NONBLOCKING)
        .open(path)?;
    let opened = file.metadata()?;
    if opened.dev() != looked.dev() || opened.ino() != looked.ino() {
        return Err(io::Error::other("it changed as it was opened"));
    }
    Ok(file)
}

/// The first folder on the way to `path` under `folder` that is a symbolic
/// link a file brought back mustn't be written through, and where it
/// leads: out of `folder`, nowhere, which would make what it names, or
/// into a hidden folder, where nothing brought back may go. Any other link
/// within the folder is the person's own. A link at the file's own name is
/// replaced as it lands, never followed, so it isn't looked at.
pub(super) fn linked(folder: &Path, path: &str) -> Option<(PathBuf, &'static str)> {
    let home = fs::canonicalize(folder).unwrap_or_else(|_| folder.to_path_buf());
    let mut at = folder.to_path_buf();
    let parts: Vec<_> = Path::new(path).components().collect();
    let folders = parts.len().saturating_sub(1);
    parts[..folders].iter().find_map(|part| {
        at.push(part);
        if !fs::symlink_metadata(&at).is_ok_and(|meta| meta.file_type().is_symlink()) {
            return None;
        }
        let Ok(to) = fs::canonicalize(&at) else {
            return Some((at.clone(), "leads nowhere"));
        };
        let Ok(within) = to.strip_prefix(&home) else {
            return Some((at.clone(), "leads out of its folder"));
        };
        within
            .components()
            .any(|part| part.as_os_str().to_string_lossy().starts_with('.'))
            .then(|| (at.clone(), "leads into a hidden folder"))
    })
}

#[cfg(test)]
mod tests;
